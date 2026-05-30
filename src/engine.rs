use std::collections::HashMap;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use savant_trading::core::config::AppConfig;
use savant_trading::core::types::{Candle, Position, ScaleLevel, Side};
use savant_trading::data::indicators::IndicatorEngine;
use savant_trading::data::kraken::KrakenClient;
use savant_trading::data::market_data::MarketDataStore;
use savant_trading::execution::engine::ExecutionEngine;
use savant_trading::execution::paper::PaperTrader;
use savant_trading::monitor::journal::TradeJournal;
use savant_trading::monitor::metrics::PerformanceMetrics;
use savant_trading::risk::circuit_breaker::{CircuitBreaker, CircuitBreakerResult};
use savant_trading::risk::position::PositionSizer;
use savant_trading::strategy::base::Strategy;
use savant_trading::strategy::mean_reversion::MeanReversionStrategy;
use savant_trading::strategy::momentum::MomentumStrategy;
use savant_trading::strategy::regime::RegimeDetector;

pub fn parse_timeframe(tf: &str) -> u64 {
    match tf {
        "1m" => 60,
        "5m" => 300,
        "15m" => 900,
        "1h" => 3600,
        "4h" => 14400,
        "1d" => 86400,
        _ => 300,
    }
}

pub fn parse_timeframe_minutes(tf: &str) -> u32 {
    match tf {
        "1m" => 1,
        "5m" => 5,
        "15m" => 15,
        "1h" => 60,
        "4h" => 240,
        "1d" => 1440,
        _ => 5,
    }
}

pub async fn run(config: AppConfig) -> anyhow::Result<()> {
    let kraken = KrakenClient::new(&config.exchange.rest_url);

    let mut market_stores: HashMap<String, MarketDataStore> = HashMap::new();
    for pair in &config.trading.pairs {
        market_stores.insert(
            pair.clone(),
            MarketDataStore::new(pair, config.strategy.mean_reversion.profile_periods + 100),
        );
    }

    let mut paper = PaperTrader::new(
        config.trading.starting_balance,
        config.trading.fee_rate,
        config.trading.slippage_pct,
    );

    let journal = match TradeJournal::new(&config.trading.database_url).await {
        Ok(j) => {
            info!("Trade journal connected: {}", config.trading.database_url);
            Some(j)
        }
        Err(e) => {
            warn!(
                "Trade journal unavailable ({}), running without persistence",
                e
            );
            None
        }
    };

    if let Some(ref j) = journal {
        let trades = j.get_trades(10000).await.unwrap_or_default();
        if !trades.is_empty() {
            let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
            let restored_balance = config.trading.starting_balance + total_pnl;
            info!(
                "Restored balance: ${:.2} (starting: ${:.2}, total PnL: ${:.2}, trades: {})",
                restored_balance,
                config.trading.starting_balance,
                total_pnl,
                trades.len()
            );
            paper.set_balance(restored_balance);
        }
    }

    let momentum = MomentumStrategy::new(
        config.strategy.momentum.ema_period,
        config.strategy.momentum.volume_spike_multiplier,
        config.strategy.momentum.atr_compression_threshold,
    );

    let mean_rev = MeanReversionStrategy::new(
        config.strategy.mean_reversion.profile_periods,
        config.strategy.mean_reversion.value_area_pct,
        config.strategy.mean_reversion.volume_spike_multiplier,
    );

    let regime_detector = RegimeDetector::new(
        config.strategy.regime.adx_period,
        config.strategy.regime.adx_trending_threshold,
        config.strategy.regime.adx_ranging_threshold,
        config.strategy.regime.atr_volatility_multiplier,
    );

    let position_sizer =
        PositionSizer::new(config.risk.max_risk_per_trade, config.risk.min_rr_ratio);

    let circuit_breaker = CircuitBreaker::new(
        config.risk.max_daily_loss,
        config.risk.max_drawdown,
        config.risk.max_positions,
    );

    let interval_seconds = parse_timeframe(&config.trading.timeframe);

    info!(
        "Fetching initial data for {} pairs...",
        config.trading.pairs.len()
    );

    for pair in &config.trading.pairs {
        match kraken
            .get_ohlc(
                pair,
                parse_timeframe_minutes(&config.trading.timeframe),
                None,
            )
            .await
        {
            Ok(mut candles) => {
                if candles.len() > 1 {
                    candles.pop();
                }
                if let Some(store) = market_stores.get_mut(pair) {
                    let count = candles.len();
                    store.add_candles(candles);
                    info!("Loaded {} historical candles for {}", count, pair);
                }
            }
            Err(e) => error!("Failed to fetch initial data for {}: {}", pair, e),
        }
    }

    info!("Starting main loop (interval: {}s)...", interval_seconds);
    let mut tick = 0u64;

    loop {
        tick += 1;

        for pair in &config.trading.pairs {
            let candles_result = kraken
                .get_ohlc(
                    pair,
                    parse_timeframe_minutes(&config.trading.timeframe),
                    None,
                )
                .await;

            let mut candles = match candles_result {
                Ok(c) => c,
                Err(e) => {
                    error!("Data fetch error for {}: {}", pair, e);
                    continue;
                }
            };

            if candles.len() > 1 {
                candles.pop();
            }

            if let Some(store) = market_stores.get_mut(pair.as_str()) {
                if let Some(last) = candles.last() {
                    store.add_candle(last.clone());
                }

                let candle_data: Vec<Candle> = store.candles().iter().cloned().collect();
                if candle_data.len() < 50 {
                    continue;
                }

                let indicators =
                    IndicatorEngine::calculate_all(&candle_data, config.strategy.regime.adx_period);
                let regime = regime_detector.detect(&indicators, &candle_data);
                let profile = Some(IndicatorEngine::volume_profile(
                    &candle_data,
                    config.strategy.mean_reversion.profile_periods.min(50),
                ));

                let mut signals = Vec::new();

                if let Some(sig) = momentum
                    .evaluate(&candle_data, &indicators, regime, profile.as_ref())
                    .await
                {
                    signals.push(sig);
                }
                if let Some(sig) = mean_rev
                    .evaluate(&candle_data, &indicators, regime, profile.as_ref())
                    .await
                {
                    signals.push(sig);
                }

                for signal in signals {
                    match circuit_breaker.check(paper.account()) {
                        CircuitBreakerResult::Triggered(reason) => {
                            warn!("Signal blocked: {}", reason);
                            continue;
                        }
                        CircuitBreakerResult::Ok => {}
                    }

                    let ps = position_sizer.calculate(
                        paper.account(),
                        signal.entry_price,
                        signal.stop_loss,
                        signal.take_profit_1,
                        signal.side,
                    );

                    let ps = match ps {
                        Some(p) => p,
                        None => {
                            info!("Signal rejected: {} {}", signal.pair, signal.side as u8);
                            continue;
                        }
                    };

                    info!(
                        "SIGNAL: {} {} @ {:.2} | SL: {:.2} | TP1: {:.2} | Qty: {:.8} | R:R: {:.2} | {}",
                        signal.pair,
                        if matches!(signal.side, Side::Long) { "LONG" } else { "SHORT" },
                        signal.entry_price,
                        signal.stop_loss,
                        signal.take_profit_1,
                        ps.quantity,
                        ps.rr_ratio,
                        signal.strategy_name,
                    );

                    let order = paper
                        .place_order(
                            &signal.pair,
                            signal.side,
                            ps.quantity,
                            Some(signal.entry_price),
                        )
                        .await;

                    match order {
                        Ok(_) => {
                            let pos = Position {
                                id: format!("pos-{}", tick),
                                pair: signal.pair.clone(),
                                side: signal.side,
                                entry_price: signal.entry_price,
                                current_price: signal.entry_price,
                                quantity: ps.quantity,
                                stop_loss: signal.stop_loss,
                                take_profit_1: signal.take_profit_1,
                                take_profit_2: signal.take_profit_2,
                                take_profit_3: signal.take_profit_3,
                                unrealized_pnl: 0.0,
                                risk_amount: ps.risk_amount,
                                strategy_name: signal.strategy_name.clone(),
                                opened_at: chrono::Utc::now(),
                                scale_level: ScaleLevel::Full,
                            };
                            paper.positions_mut().insert(pos.id.clone(), pos);
                            paper.account_mut().open_positions = paper.positions().len();
                            paper.account_mut().trades_today += 1;
                            info!("Position opened: {}", signal.pair);
                        }
                        Err(e) => error!("Order failed: {}", e),
                    }
                }

                let mut prices = HashMap::new();
                if let Some(last) = store.last() {
                    prices.insert(pair.clone(), last.close);
                }
                let closed = paper.check_stops(&prices);
                for trade in closed {
                    info!(
                        "CLOSED: {} {} | PnL: ${:.2} ({:.2}%) | {}",
                        trade.pair,
                        if matches!(trade.side, Side::Long) {
                            "LONG"
                        } else {
                            "SHORT"
                        },
                        trade.pnl,
                        trade.pnl_pct,
                        trade.notes,
                    );
                    if let Some(ref j) = journal {
                        if let Err(e) = j.record_trade(&trade).await {
                            warn!("Failed to record trade: {}", e);
                        }
                    }
                }
            }
        }

        if tick.is_multiple_of(10) {
            let account = paper.account();
            let trades = paper.closed_trades();
            let metrics = PerformanceMetrics::calculate(trades);
            info!(
                "[STATUS] Balance: ${:.2} | Equity: ${:.2} | DD: {:.1}% | {}",
                account.balance,
                account.equity,
                account.drawdown_pct * 100.0,
                metrics
            );
            if let Some(ref j) = journal {
                if let Err(e) = j
                    .record_snapshot(
                        account.balance,
                        account.equity,
                        account.drawdown_pct,
                        account.open_positions as i32,
                    )
                    .await
                {
                    warn!("Failed to record equity snapshot: {}", e);
                }
            }
        }

        time::sleep(Duration::from_secs(interval_seconds)).await;
    }
}
