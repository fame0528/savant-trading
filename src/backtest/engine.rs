//! Backtest engine — replays historical candles through strategies.

use chrono::{DateTime, Utc};
use tracing::info;

use crate::backtest::metrics::{BacktestMetrics, BacktestTrade};
use crate::core::types::{Candle, Side};
use crate::data::indicators::IndicatorEngine;
use crate::strategy::base::Strategy;
use crate::strategy::regime::RegimeDetector;

/// Configuration for a backtest run.
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    /// Starting balance
    pub starting_balance: f64,
    /// Risk per trade (fraction of balance)
    pub risk_per_trade: f64,
    /// Minimum R:R ratio
    pub min_rr_ratio: f64,
    /// Fee rate (per side)
    pub fee_rate: f64,
    /// Slippage percentage
    pub slippage_pct: f64,
    /// ADX period for regime detection
    pub adx_period: usize,
    /// ADX trending threshold
    pub adx_trending: f64,
    /// ADX ranging threshold
    pub adx_ranging: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            starting_balance: 10000.0,
            risk_per_trade: 0.01,
            min_rr_ratio: 1.5,
            fee_rate: 0.0026,
            slippage_pct: 0.0005,
            adx_period: 14,
            adx_trending: 25.0,
            adx_ranging: 20.0,
        }
    }
}

/// Result of a single backtest run.
#[derive(Debug)]
pub struct BacktestResult {
    pub metrics: BacktestMetrics,
    pub trades: Vec<BacktestTrade>,
    pub equity_curve: Vec<(DateTime<Utc>, f64)>,
}

/// Run a backtest on historical candles using the given strategy.
pub fn run_backtest(
    candles: &[Candle],
    strategy: &dyn Strategy,
    config: &BacktestConfig,
) -> BacktestResult {
    let mut balance = config.starting_balance;
    let mut peak_balance = balance;
    let mut trades: Vec<BacktestTrade> = Vec::new();
    let mut equity_curve: Vec<(DateTime<Utc>, f64)> = Vec::new();
    let mut open_position: Option<OpenPosition> = None;

    let regime_detector = RegimeDetector::new(
        config.adx_period,
        config.adx_trending,
        config.adx_ranging,
        1.5,
    );

    let min_candles = 50;

    for i in min_candles..candles.len() {
        let window = &candles[..=i];
        let indicators = IndicatorEngine::calculate_all(window, config.adx_period);
        let regime = regime_detector.detect(&indicators, window);
        let profile = Some(IndicatorEngine::volume_profile(window, 50));
        let last = &candles[i];

        // Check if open position should be closed
        if let Some(ref mut pos) = open_position {
            let hit_stop = match pos.side {
                Side::Long => last.close <= pos.stop_loss,
                Side::Short => last.close >= pos.stop_loss,
            };
            let hit_tp = match pos.side {
                Side::Long => last.close >= pos.take_profit_1,
                Side::Short => last.close <= pos.take_profit_1,
            };

            if hit_stop || hit_tp {
                let exit_price = if hit_stop {
                    pos.stop_loss
                } else {
                    pos.take_profit_1
                };
                let slippage = exit_price * config.slippage_pct;
                let actual_exit = match pos.side {
                    Side::Long => exit_price - slippage,
                    Side::Short => exit_price + slippage,
                };
                let exit_fee = actual_exit * pos.quantity * config.fee_rate;
                let pnl = match pos.side {
                    Side::Long => (actual_exit - pos.entry_price) * pos.quantity - exit_fee,
                    Side::Short => (pos.entry_price - actual_exit) * pos.quantity - exit_fee,
                };

                balance += pnl;
                if balance > peak_balance {
                    peak_balance = balance;
                }

                trades.push(BacktestTrade {
                    pair: last.pair.clone(),
                    side: pos.side,
                    entry_price: pos.entry_price,
                    exit_price: actual_exit,
                    quantity: pos.quantity,
                    pnl,
                    pnl_pct: pnl / (pos.entry_price * pos.quantity) * 100.0,
                    entry_time: pos.entry_time,
                    exit_time: last.timestamp,
                    is_win: pnl > 0.0,
                });

                open_position = None;
            }
        }

        // Look for new entry if no position
        if open_position.is_none() {
            let signal = strategy.evaluate_sync(window, &indicators, regime, profile.as_ref());

            if let Some(sig) = signal {
                // Check R:R
                let risk = match sig.side {
                    Side::Long => sig.entry_price - sig.stop_loss,
                    Side::Short => sig.stop_loss - sig.entry_price,
                };
                let reward = match sig.side {
                    Side::Long => sig.take_profit_1 - sig.entry_price,
                    Side::Short => sig.entry_price - sig.take_profit_1,
                };

                if risk > 0.0 && reward > 0.0 {
                    let rr = reward / risk;
                    if rr >= config.min_rr_ratio {
                        let risk_amount = balance * config.risk_per_trade;
                        let quantity = risk_amount / risk;
                        let entry_fee = sig.entry_price * quantity * config.fee_rate;

                        if entry_fee < balance {
                            balance -= entry_fee;
                            open_position = Some(OpenPosition {
                                side: sig.side,
                                entry_price: sig.entry_price,
                                stop_loss: sig.stop_loss,
                                take_profit_1: sig.take_profit_1,
                                quantity,
                                entry_time: last.timestamp,
                            });
                        }
                    }
                }
            }
        }

        equity_curve.push((last.timestamp, balance));
    }

    // Close any remaining position at last price
    if let Some(pos) = open_position {
        if let Some(last) = candles.last() {
            let pnl = match pos.side {
                Side::Long => (last.close - pos.entry_price) * pos.quantity,
                Side::Short => (pos.entry_price - last.close) * pos.quantity,
            };
            balance += pnl;
            equity_curve.push((last.timestamp, balance));
            trades.push(BacktestTrade {
                pair: last.pair.clone(),
                side: pos.side,
                entry_price: pos.entry_price,
                exit_price: last.close,
                quantity: pos.quantity,
                pnl,
                pnl_pct: pnl / (pos.entry_price * pos.quantity) * 100.0,
                entry_time: pos.entry_time,
                exit_time: last.timestamp,
                is_win: pnl > 0.0,
            });
        }
    }

    let metrics = BacktestMetrics::calculate(&trades, config.starting_balance, &equity_curve);

    info!(
        "Backtest complete: {} trades, WR: {:.1}%, PnL: ${:.2}, MaxDD: {:.1}%",
        metrics.total_trades,
        metrics.win_rate * 100.0,
        metrics.total_pnl,
        metrics.max_drawdown_pct * 100.0
    );

    BacktestResult {
        metrics,
        trades,
        equity_curve,
    }
}

struct OpenPosition {
    side: Side,
    entry_price: f64,
    stop_loss: f64,
    take_profit_1: f64,
    quantity: f64,
    entry_time: DateTime<Utc>,
}
