//! Forward simulator + virtual wallet for sandbox trade simulation.
//!
//! Takes a trade decision and remaining candles, simulates the trade forward,
//! and calculates P&L, R-multiple, MFE/MAE, and portfolio metrics.

use crate::core::types::Candle;

/// A simulated trade executed through candle data.
#[derive(Debug, Clone)]
pub struct SimTrade {
    pub scenario_id: String,
    pub action: String,
    pub side: String,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    pub take_profit_3: f64,
    pub exit_price: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub r_multiple: f64,
    pub hold_candles: usize,
    pub fees_paid: f64,
    pub slippage_applied: f64,
    pub mfe: f64,
    pub mae: f64,
    pub exit_reason: String,
}

/// Virtual wallet tracking portfolio state through simulated trades.
#[derive(Debug, Clone)]
pub struct VirtualWallet {
    pub starting_balance: f64,
    pub balance: f64,
    pub trades: Vec<SimTrade>,
    pub equity_curve: Vec<(usize, f64)>,
    pub fee_rate: f64,
    pub slippage_pct: f64,
}

/// Parameters for simulating a trade.
pub struct TradeParams<'a> {
    pub scenario_id: &'a str,
    pub action: &'a str,
    pub side: &'a str,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    pub take_profit_3: f64,
    pub position_size: f64,
    pub remaining_candles: &'a [Candle],
    pub tick_index: usize,
}

impl VirtualWallet {
    pub fn new(starting_balance: f64, fee_rate: f64, slippage_pct: f64) -> Self {
        Self {
            starting_balance,
            balance: starting_balance,
            trades: Vec::new(),
            equity_curve: vec![(0, starting_balance)],
            fee_rate,
            slippage_pct,
        }
    }

    /// Simulate a trade through remaining candles.
    /// Returns Some(SimTrade) if the trade was executed, None if Hold.
    #[allow(clippy::too_many_arguments)]
    pub fn simulate_trade(
        &mut self,
        scenario_id: &str,
        action: &str,
        side: &str,
        entry_price: f64,
        stop_loss: f64,
        take_profit_1: f64,
        take_profit_2: f64,
        take_profit_3: f64,
        position_size: f64,
        remaining_candles: &[Candle],
        tick_index: usize,
    ) -> Option<SimTrade> {
        let params = TradeParams {
            scenario_id,
            action,
            side,
            entry_price,
            stop_loss,
            take_profit_1,
            take_profit_2,
            take_profit_3,
            position_size,
            remaining_candles,
            tick_index,
        };
        self.simulate(params)
    }

    fn simulate(&mut self, p: TradeParams<'_>) -> Option<SimTrade> {
        if p.action == "Hold" || p.position_size <= 0.0 || p.remaining_candles.is_empty() {
            return None;
        }

        // Apply slippage to entry
        let slippage = p.entry_price * self.slippage_pct;
        let actual_entry = if p.side == "Long" {
            p.entry_price + slippage
        } else {
            p.entry_price - slippage
        };

        // Entry fee
        let entry_fee = p.position_size * actual_entry * self.fee_rate;
        let quantity = p.position_size / actual_entry;

        let mut mfe = 0.0f64;
        let mut mae = 0.0f64;
        let mut exit_price = actual_entry;
        let mut exit_reason = "end_of_data".to_string();
        let mut hold_candles = 0usize;

        let initial_risk = if p.side == "Long" {
            actual_entry - p.stop_loss
        } else {
            p.stop_loss - actual_entry
        }
        .abs()
        .max(0.0001);

        // Track partial fills
        let mut remaining_qty = quantity;
        let mut total_exit_value = 0.0f64;
        let tp1_qty = quantity * 0.30;
        let tp2_qty = quantity * 0.30;

        for (i, candle) in p.remaining_candles.iter().enumerate() {
            hold_candles = i + 1;
            let high = candle.high;
            let low = candle.low;

            // Track MFE/MAE
            if p.side == "Long" {
                mfe = mfe.max(high - actual_entry);
                mae = mae.max(actual_entry - low);
            } else {
                mfe = mfe.max(actual_entry - low);
                mae = mae.max(high - actual_entry);
            }

            // Check stop loss first (pessimistic: stop before TP on same candle)
            let stop_hit = if p.side == "Long" {
                low <= p.stop_loss
            } else {
                high >= p.stop_loss
            };

            if stop_hit && remaining_qty > 0.0 {
                let slippage_exit = p.stop_loss * self.slippage_pct;
                let stop_exit = if p.side == "Long" {
                    p.stop_loss - slippage_exit
                } else {
                    p.stop_loss + slippage_exit
                };
                total_exit_value += remaining_qty * stop_exit;
                exit_price = stop_exit;
                exit_reason = "stop_loss".to_string();
                remaining_qty = 0.0;
                break;
            }

            // Check TP levels
            if p.take_profit_1 > 0.0 && remaining_qty >= tp1_qty {
                let tp1_hit = if p.side == "Long" {
                    high >= p.take_profit_1
                } else {
                    low <= p.take_profit_1
                };
                if tp1_hit {
                    total_exit_value += tp1_qty * p.take_profit_1;
                    remaining_qty -= tp1_qty;
                }
            }

            if p.take_profit_2 > 0.0 && remaining_qty >= tp2_qty {
                let tp2_hit = if p.side == "Long" {
                    high >= p.take_profit_2
                } else {
                    low <= p.take_profit_2
                };
                if tp2_hit {
                    total_exit_value += tp2_qty * p.take_profit_2;
                    remaining_qty -= tp2_qty;
                }
            }

            if p.take_profit_3 > 0.0 && remaining_qty > 0.0 {
                let tp3_hit = if p.side == "Long" {
                    high >= p.take_profit_3
                } else {
                    low <= p.take_profit_3
                };
                if tp3_hit {
                    total_exit_value += remaining_qty * p.take_profit_3;
                    exit_price = p.take_profit_3;
                    exit_reason = "take_profit_3".to_string();
                    remaining_qty = 0.0;
                    break;
                }
            }

            // If all TPs hit, close remaining at last TP
            if remaining_qty <= 0.0 {
                exit_reason = "take_profits".to_string();
                break;
            }
        }

        // Close any remaining position at last candle close
        if remaining_qty > 0.0 {
            if let Some(last) = p.remaining_candles.last() {
                total_exit_value += remaining_qty * last.close;
                exit_price = last.close;
                exit_reason = "end_of_data".to_string();
            }
        }

        // Calculate P&L
        let exit_fee = total_exit_value * self.fee_rate;
        let total_fees = entry_fee + exit_fee;
        let gross_pnl = if p.side == "Long" {
            total_exit_value - (quantity * actual_entry)
        } else {
            (quantity * actual_entry) - total_exit_value
        };
        let net_pnl = gross_pnl - total_fees;
        let pnl_pct = if p.position_size > 0.0 {
            (net_pnl / p.position_size) * 100.0
        } else {
            0.0
        };

        // R-multiple
        let r_multiple = if initial_risk > 0.0 {
            net_pnl / (quantity * initial_risk)
        } else {
            0.0
        };

        self.balance += net_pnl;
        self.equity_curve
            .push((p.tick_index + hold_candles, self.balance));

        let trade = SimTrade {
            scenario_id: p.scenario_id.to_string(),
            action: p.action.to_string(),
            side: p.side.to_string(),
            entry_price: actual_entry,
            stop_loss: p.stop_loss,
            take_profit_1: p.take_profit_1,
            take_profit_2: p.take_profit_2,
            take_profit_3: p.take_profit_3,
            exit_price,
            pnl: net_pnl,
            pnl_pct,
            r_multiple,
            hold_candles,
            fees_paid: total_fees,
            slippage_applied: slippage * 2.0,
            mfe,
            mae,
            exit_reason,
        };

        self.trades.push(trade.clone());
        Some(trade)
    }

    /// Portfolio-level metrics.
    pub fn metrics(&self) -> WalletMetrics {
        let total_trades = self.trades.len();
        if total_trades == 0 {
            return WalletMetrics::default();
        }

        let wins: Vec<&SimTrade> = self.trades.iter().filter(|t| t.pnl > 0.0).collect();
        let losses: Vec<&SimTrade> = self.trades.iter().filter(|t| t.pnl <= 0.0).collect();

        let win_rate = (wins.len() as f64 / total_trades as f64) * 100.0;
        let gross_profit: f64 = wins.iter().map(|t| t.pnl).sum();
        let gross_loss: f64 = losses.iter().map(|t| t.pnl.abs()).sum();
        let net_pnl = self.balance - self.starting_balance;
        let pnl_pct = (net_pnl / self.starting_balance) * 100.0;
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        let avg_r: f64 =
            self.trades.iter().map(|t| t.r_multiple).sum::<f64>() / total_trades as f64;

        // Max drawdown
        let mut peak = self.starting_balance;
        let mut max_dd = 0.0f64;
        for &(_, equity) in &self.equity_curve {
            if equity > peak {
                peak = equity;
            }
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        let best = self
            .trades
            .iter()
            .max_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap())
            .cloned();
        let worst = self
            .trades
            .iter()
            .min_by(|a, b| a.pnl.partial_cmp(&b.pnl).unwrap())
            .cloned();

        WalletMetrics {
            starting_balance: self.starting_balance,
            final_balance: self.balance,
            net_pnl,
            pnl_pct,
            total_trades,
            wins: wins.len(),
            losses: losses.len(),
            win_rate,
            avg_r_multiple: avg_r,
            max_drawdown_pct: max_dd * 100.0,
            profit_factor,
            gross_profit,
            gross_loss,
            total_fees: self.trades.iter().map(|t| t.fees_paid).sum(),
            best_trade: best,
            worst_trade: worst,
            equity_curve: self.equity_curve.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalletMetrics {
    pub starting_balance: f64,
    pub final_balance: f64,
    pub net_pnl: f64,
    pub pnl_pct: f64,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub avg_r_multiple: f64,
    pub max_drawdown_pct: f64,
    pub profit_factor: f64,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub total_fees: f64,
    pub best_trade: Option<SimTrade>,
    pub worst_trade: Option<SimTrade>,
    pub equity_curve: Vec<(usize, f64)>,
}

impl Default for WalletMetrics {
    fn default() -> Self {
        Self {
            starting_balance: 0.0,
            final_balance: 0.0,
            net_pnl: 0.0,
            pnl_pct: 0.0,
            total_trades: 0,
            wins: 0,
            losses: 0,
            win_rate: 0.0,
            avg_r_multiple: 0.0,
            max_drawdown_pct: 0.0,
            profit_factor: 0.0,
            gross_profit: 0.0,
            gross_loss: 0.0,
            total_fees: 0.0,
            best_trade: None,
            worst_trade: None,
            equity_curve: vec![],
        }
    }
}

impl WalletMetrics {
    pub fn report_card(&self) -> String {
        let best_str = self
            .best_trade
            .as_ref()
            .map(|t| format!("{} {} +${:.2}", t.scenario_id, t.action, t.pnl))
            .unwrap_or_else(|| "N/A".into());
        let worst_str = self
            .worst_trade
            .as_ref()
            .map(|t| format!("{} {} -${:.2}", t.scenario_id, t.action, t.pnl.abs()))
            .unwrap_or_else(|| "N/A".into());

        format!(
            r#"═══ SANDBOX WALLET REPORT ═══
Starting Balance:  ${:.2}
Final Balance:     ${:.2}
Total P&L:         ${:+.2} ({:+.2}%)
Trades:            {} taken
Win Rate:          {:.1}% ({}W / {}L)
Avg R-Multiple:    {:.2}
Max Drawdown:      -{:.2}%
Profit Factor:     {:.2}
Total Fees:        ${:.2}
Best Trade:        {}
Worst Trade:       {}
════════════════════════════"#,
            self.starting_balance,
            self.final_balance,
            self.net_pnl,
            self.pnl_pct,
            self.total_trades,
            self.win_rate,
            self.wins,
            self.losses,
            self.avg_r_multiple,
            self.max_drawdown_pct,
            self.profit_factor,
            self.total_fees,
            best_str,
            worst_str,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_candles(count: usize, start_price: f64, direction: f64) -> Vec<Candle> {
        (0..count)
            .map(|i| {
                let p = start_price + direction * i as f64;
                Candle {
                    timestamp: Utc::now(),
                    open: p,
                    high: p + 2.0,
                    low: p - 2.0,
                    close: p + direction,
                    volume: 1000.0,
                    pair: "BTC/USD".into(),
                }
            })
            .collect()
    }

    #[test]
    fn wallet_hold_returns_none() {
        let mut wallet = VirtualWallet::new(50.0, 0.0026, 0.0005);
        let candles = make_candles(10, 100.0, 1.0);
        let result = wallet.simulate_trade(
            "T1", "Hold", "Long", 100.0, 95.0, 110.0, 0.0, 0.0, 10.0, &candles, 0,
        );
        assert!(result.is_none());
        assert_eq!(wallet.trades.len(), 0);
    }

    #[test]
    fn wallet_buy_tp1_hit() {
        let mut wallet = VirtualWallet::new(50.0, 0.0026, 0.0005);
        // Price goes up to hit TP1 at 110
        let candles = make_candles(20, 100.0, 2.0);
        let trade = wallet.simulate_trade(
            "T1", "Buy", "Long", 100.0, 90.0, 110.0, 120.0, 130.0, 10.0, &candles, 0,
        );
        assert!(trade.is_some());
        let t = trade.unwrap();
        assert!(t.pnl != 0.0);
        assert!(t.hold_candles > 0);
    }

    #[test]
    fn wallet_buy_stop_hit() {
        let mut wallet = VirtualWallet::new(50.0, 0.0026, 0.0005);
        // Price drops to stop at 90
        let candles = make_candles(20, 100.0, -2.0);
        let trade = wallet.simulate_trade(
            "T1", "Buy", "Long", 100.0, 90.0, 110.0, 120.0, 130.0, 10.0, &candles, 0,
        );
        assert!(trade.is_some());
        let t = trade.unwrap();
        assert!(t.pnl < 0.0);
        assert_eq!(t.exit_reason, "stop_loss");
    }

    #[test]
    fn wallet_metrics_empty() {
        let wallet = VirtualWallet::new(50.0, 0.0026, 0.0005);
        let m = wallet.metrics();
        assert_eq!(m.total_trades, 0);
        assert_eq!(m.win_rate, 0.0);
    }

    #[test]
    fn wallet_metrics_after_trades() {
        let mut wallet = VirtualWallet::new(50.0, 0.0026, 0.0005);
        // Big up move — clear win
        let candles_up = make_candles(50, 100.0, 5.0);
        // Big down move — clear loss (stop hit)
        let candles_down = make_candles(50, 100.0, -5.0);
        wallet.simulate_trade(
            "T1",
            "Buy",
            "Long",
            100.0,
            90.0,
            130.0,
            160.0,
            200.0,
            10.0,
            &candles_up,
            0,
        );
        wallet.simulate_trade(
            "T2",
            "Buy",
            "Long",
            100.0,
            95.0,
            110.0,
            120.0,
            130.0,
            10.0,
            &candles_down,
            50,
        );
        let m = wallet.metrics();
        assert_eq!(m.total_trades, 2);
        // T1 should be a win (big up move), T2 a loss (stop hit)
        assert!(m.wins >= 1);
    }
}
