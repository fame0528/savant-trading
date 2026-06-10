use savant_trading::agent::context_builder::FullContext;
use savant_trading::agent::prompts::{self, PromptComposer};
use savant_trading::core::config::AppConfig;
use savant_trading::core::types::Candle;
use savant_trading::data::candle_client::CandleClient;
use savant_trading::execution::portfolio::PortfolioManager;
use savant_trading::insight::aggregator::{InsightAggregator, InsightConfig};
use savant_trading::strategy::regime::RegimeDetector;

use super::utils::{load_knowledge_base, parse_timeframe_minutes};

/// Dry-run: make ONE AI call and print the full pipeline output.
pub async fn dry_run(config: AppConfig) -> anyhow::Result<()> {
    let candle_api = CandleClient::new(&config.exchange.rest_url);
    let pair = config
        .trading
        .pairs
        .first()
        .cloned()
        .unwrap_or_else(|| "BTC/USD".to_string());

    // 1. Fetch market data
    println!("\n=== SAVANT DRY RUN ===");
    println!("Pair: {}", pair);
    println!(
        "Time: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    println!("\n--- MARKET DATA ---");
    let mut candles = candle_api
        .get_ohlc(
            &pair,
            parse_timeframe_minutes(&config.trading.timeframe),
            None,
        )
        .await
        .unwrap_or_default();
    if candles.len() > 1 {
        candles.pop();
    }

    if candles.is_empty() {
        println!("ERROR: No candle data available");
        return Ok(());
    }

    let indicators = savant_trading::data::indicators::IndicatorEngine::calculate_all(
        &candles,
        config.strategy.regime.adx_period,
    );

    let regime_detector = RegimeDetector::new(
        config.strategy.regime.adx_period,
        config.strategy.regime.adx_trending_threshold,
        config.strategy.regime.adx_ranging_threshold,
        config.strategy.regime.atr_volatility_multiplier,
    );
    let regime = regime_detector.detect(&indicators, &candles);

    let profile = savant_trading::data::indicators::IndicatorEngine::volume_profile(
        &candles,
        config.strategy.mean_reversion.profile_periods.min(50),
    );

    if let Some(last) = candles.last() {
        println!(
            "Candle: O={:.2} H={:.2} L={:.2} C={:.2} V={:.2}",
            last.open, last.high, last.low, last.close, last.volume
        );
    }
    println!(
        "Indicators: EMA_FAST={:?} EMA_SLOW={:?} RSI={:?} ATR={:?} ADX={:?} VWAP={:?}",
        indicators.ema_fast,
        indicators.ema_slow,
        indicators.rsi,
        indicators.atr,
        indicators.adx,
        indicators.vwap
    );
    println!("Regime: {:?}", regime);

    // 2. Fetch insight
    println!("\n--- INSIGHT ---");
    let insight_config = InsightConfig {
        funding_rate_enabled: config.insight.funding_rate_enabled,
        liquidation_enabled: config.insight.liquidation_enabled,
        fear_greed_enabled: config.insight.fear_greed_enabled,
        btc_dominance_enabled: config.insight.btc_dominance_enabled,
        exchange_flows_enabled: config.insight.exchange_flows_enabled,
        news_sentiment_enabled: config.insight.news_sentiment_enabled,
        rss_enabled: config.insight.rss_enabled,
        rss_max_items: config.insight.rss_max_items,
        onchain_enabled: config.insight.onchain_enabled,
    };
    let mut insight = InsightAggregator::new(insight_config);
    let market_ctx = insight.refresh(&pair).await.clone();
    println!("{}", market_ctx.summary());

    // 3. Build context using the SAME path as the live engine
    println!("\n--- KNOWLEDGE SELECTION ---");
    let knowledge_base = load_knowledge_base();

    let composer = PromptComposer::new(
        &prompts::default_base_identity(),
        include_str!("../agent/prompts/risk_constraints.md"),
        &format!(
            "{}\n\n---\n\n{}",
            include_str!("../agent/prompts/strategy_knowledge.md"),
            include_str!("../agent/prompts/echo_rules.md")
        ),
        include_str!("../agent/prompts/output_format.md"),
    );

    let portfolio = PortfolioManager::new(
        config.trading.starting_balance,
        config.trading.fee_rate,
        config.trading.slippage_pct,
    );
    let ctx = FullContext {
        candles: &candles,
        indicators: &indicators,
        regime,
        volume_profile: Some(&profile),
        market_context: &market_ctx,
        positions: &[],
        account: portfolio.account(),
        pair: &pair,
        recent_trades: None,
        order_book_imbalance: None,
        session: savant_trading::core::session::current_session(),
        memory_context: None,
        higher_tf_candles: vec![],
        context_tags: savant_trading::agent::context_builder::generate_context_tags(&indicators),
        live_price: None,
        decision_log_context: None,
        dex_price: None,
    };

    let (system_prompt, user_message) = savant_trading::agent::context_builder::build_context(
        &ctx,
        &knowledge_base,
        &composer,
        3000, // Reduced for training speed — full 8000 used in live engine
    );

    println!(
        "Conditions: {:?}",
        savant_trading::agent::context_builder::determine_conditions_static(
            regime,
            market_ctx.sentiment.fear_greed_index,
            market_ctx.funding.funding_rate,
        )
    );
    println!("Context tags: {:?}", ctx.context_tags);
    println!("System prompt: {} chars", system_prompt.len());
    println!("User message: {} chars", user_message.len());

    // Call LLM
    println!("\n--- LLM CALL ---");
    let provider = savant_trading::agent::provider::create_provider(&config.ai);
    let messages = vec![savant_trading::agent::provider::Message {
        role: "user".to_string(),
        content: user_message,
    }];

    match provider.chat(&system_prompt, &messages).await {
        Ok(response) => {
            println!("\n--- LLM RESPONSE ---");
            println!("{}", response);

            // Parse decision
            let current_price = candles.last().map(|c| c.close).unwrap_or(0.0);
            match savant_trading::agent::decision_parser::parse_decision(
                &response,
                current_price,
                config.ai.price_tolerance_pct,
            ) {
                Ok(decision) => {
                    println!("\n--- PARSED DECISION ---");
                    println!("Action: {:?}", decision.action);
                    println!("Pair: {}", decision.pair);
                    println!("Side: {:?}", decision.side);
                    println!("Entry: {:.2}", decision.entry_price);
                    println!("Stop Loss: {:.2}", decision.stop_loss);
                    println!(
                        "TP1: {:.2} | TP2: {:.2} | TP3: {:.2}",
                        decision.take_profit_1, decision.take_profit_2, decision.take_profit_3
                    );
                    println!("Position Size: {:.1}%", decision.position_size_pct);
                    println!("Confidence: {:.0}%", decision.confidence * 100.0);
                    println!("R:R: {:.2}", decision.risk_reward);
                    println!("Reasoning: {}", decision.reasoning);
                    println!("Knowledge Sources: {:?}", decision.knowledge_sources);
                }
                Err(e) => {
                    println!("\n--- PARSE ERROR ---");
                    println!("Failed to parse LLM response: {}", e);
                }
            }
        }
        Err(e) => {
            println!("\n--- LLM ERROR ---");
            println!("Failed to call LLM: {}", e);
        }
    }

    println!("\n=== DRY RUN COMPLETE ===");
    Ok(())
}

/// FID-084: Live Situation Sandbox — test any model against current market data.
/// Fetches live candles, insight, and positions — same prompt pipeline as live engine.
/// Read-only: no state mutation, no API server, can run alongside active engine.
pub async fn run_live_test(
    config: AppConfig,
    model_override: Option<String>,
    pairs_override: Vec<String>,
    show_prompt: bool,
) -> anyhow::Result<()> {
    let candle_api = CandleClient::new(&config.exchange.rest_url);
    let pairs = if pairs_override.is_empty() {
        config.trading.pairs.clone()
    } else {
        pairs_override
    };

    // Apply model override if specified
    let mut config = config;
    if let Some(ref model) = model_override {
        config.ai.model = model.clone();
    }

    let est = savant_trading::core::console::est_now();
    println!("\n=== LIVE SITUATION TEST ===");
    println!("Model: {}", config.ai.model);
    println!("Time: {}", est);
    println!("Pairs: {}", pairs.join(", "));

    // Load positions from dex_state.json (read-only)
    let positions_path = std::path::Path::new("data/dex_state.json");
    let positions: Vec<savant_trading::core::types::Position> = if positions_path.exists() {
        let data = std::fs::read_to_string(positions_path).unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&data)
            .ok()
            .and_then(|v| v.get("positions").cloned())
            .and_then(|p| serde_json::from_value(p).ok())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let account_balance: f64 = if positions_path.exists() {
        let data = std::fs::read_to_string(positions_path).unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&data)
            .ok()
            .and_then(|v| v.get("balance").and_then(|b| b.as_f64()))
            .unwrap_or(0.0)
    } else {
        0.0
    };

    println!("Positions: {} open", positions.len());
    for pos in &positions {
        println!("  {} {} @ {:.2} | SL: {:.2} | Qty: {:.6}", pos.pair, pos.side, pos.entry_price, pos.stop_loss, pos.quantity);
    }
    println!("Balance: ${:.2}", account_balance);

    // Fetch candles for all pairs in parallel
    println!("\n--- FETCHING LIVE DATA ---");
    let interval = parse_timeframe_minutes(&config.trading.timeframe);

    let candle_futures: Vec<_> = pairs
        .iter()
        .map(|pair| {
            let api = candle_api.clone();
            let p = pair.clone();
            async move {
                let mut candles = api.get_ohlc(&p, interval, None).await.unwrap_or_default();
                if candles.len() > 1 { candles.pop(); }
                (p, candles)
            }
        })
        .collect();

    let candle_results = futures_util::future::join_all(candle_futures).await;
    let mut market_stores: std::collections::HashMap<String, Vec<Candle>> = std::collections::HashMap::new();
    for (pair, candles) in candle_results {
        if !candles.is_empty() {
            println!("  {}: {} candles", pair, candles.len());
            market_stores.insert(pair, candles);
        }
    }

    // Fetch insight
    let insight_config = InsightConfig {
        funding_rate_enabled: config.insight.funding_rate_enabled,
        liquidation_enabled: config.insight.liquidation_enabled,
        fear_greed_enabled: config.insight.fear_greed_enabled,
        btc_dominance_enabled: config.insight.btc_dominance_enabled,
        exchange_flows_enabled: config.insight.exchange_flows_enabled,
        news_sentiment_enabled: config.insight.news_sentiment_enabled,
        rss_enabled: config.insight.rss_enabled,
        rss_max_items: config.insight.rss_max_items,
        onchain_enabled: config.insight.onchain_enabled,
    };
    let mut insight = InsightAggregator::new(insight_config);

    // Build knowledge base + prompt composer (same as live engine)
    let knowledge_base = load_knowledge_base();
    let composer = PromptComposer::new(
        &prompts::default_base_identity(),
        include_str!("../agent/prompts/risk_constraints.md"),
        &format!(
            "{}\n\n---\n\n{}",
            include_str!("../agent/prompts/strategy_knowledge.md"),
            include_str!("../agent/prompts/echo_rules.md")
        ),
        include_str!("../agent/prompts/output_format.md"),
    );

    let portfolio = PortfolioManager::new(
        config.trading.starting_balance,
        config.trading.fee_rate,
        config.trading.slippage_pct,
    );

    // Evaluate each pair — same prompt pipeline as live engine
    let mut all_decisions = Vec::new();
    let provider = savant_trading::agent::provider::create_provider(&config.ai);

    for pair in &pairs {
        let candles = match market_stores.get(pair) {
            Some(c) if !c.is_empty() => c,
            _ => {
                println!("\n  {}: No candle data — skipping", pair);
                continue;
            }
        };

        let indicators = savant_trading::data::indicators::IndicatorEngine::calculate_all(
            candles,
            config.strategy.regime.adx_period,
        );

        let regime_detector = RegimeDetector::new(
            config.strategy.regime.adx_period,
            config.strategy.regime.adx_trending_threshold,
            config.strategy.regime.adx_ranging_threshold,
            config.strategy.regime.atr_volatility_multiplier,
        );
        let regime = regime_detector.detect(&indicators, candles);

        let profile = savant_trading::data::indicators::IndicatorEngine::volume_profile(
            candles,
            config.strategy.mean_reversion.profile_periods.min(50),
        );

        let market_ctx = insight.refresh(pair).await.clone();

        // Check if this pair has an open position
        let pos_ref: Vec<savant_trading::core::types::Position> = positions
            .iter()
            .filter(|p| p.pair == *pair)
            .cloned()
            .collect();

        let ctx = FullContext {
            candles,
            indicators: &indicators,
            regime,
            volume_profile: Some(&profile),
            market_context: &market_ctx,
            positions: &pos_ref,
            account: portfolio.account(),
            pair,
            recent_trades: None,
            order_book_imbalance: None,
            session: savant_trading::core::session::current_session(),
            memory_context: None,
            higher_tf_candles: vec![],
            context_tags: savant_trading::agent::context_builder::generate_context_tags(&indicators),
            live_price: None,
            decision_log_context: None,
            dex_price: None,
        };

        let (system_prompt, user_message) = savant_trading::agent::context_builder::build_context(
            &ctx,
            &knowledge_base,
            &composer,
            3000,
        );

        if show_prompt {
            println!("\n  === PROMPT FOR {} ===", pair);
            println!("  System: {} chars", system_prompt.len());
            println!("  User: {}", user_message);
        }

        // Call LLM
        let start = std::time::Instant::now();
        let messages = vec![savant_trading::agent::provider::Message {
            role: "user".to_string(),
            content: user_message,
        }];

        match provider.chat(&system_prompt, &messages).await {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("\n  === {} ({:.1}s) ===", pair, elapsed.as_secs_f64());
                println!("  Raw: {}", response);

                let current_price = candles.last().map(|c| c.close).unwrap_or(0.0);
                match savant_trading::agent::decision_parser::parse_decision(
                    &response,
                    current_price,
                    config.ai.price_tolerance_pct,
                ) {
                    Ok(decision) => {
                        println!("  Action: {:?}", decision.action);
                        println!("  Side: {:?}", decision.side);
                        println!("  Confidence: {:.0}%", decision.confidence * 100.0);
                        println!("  R:R: {:.2}", decision.risk_reward);
                        println!("  Entry: {:.2} | Stop: {:.2}", decision.entry_price, decision.stop_loss);
                        println!("  TP1: {:.2} | TP2: {:.2} | TP3: {:.2}", decision.take_profit_1, decision.take_profit_2, decision.take_profit_3);
                        println!("  Reasoning: {}", decision.reasoning);
                        all_decisions.push(decision);
                    }
                    Err(e) => {
                        println!("  PARSE ERROR: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("\n  === {} ===", pair);
                println!("  LLM ERROR: {}", e);
            }
        }
    }

    // Summary
    println!("\n=== SUMMARY ===");
    println!("Model: {}", config.ai.model);
    println!("Pairs evaluated: {}", pairs.len());
    println!("Decisions: {}", all_decisions.len());
    for d in &all_decisions {
        println!("  {} {:?} {:.0}% R:{:.2} — {}", d.pair, d.action, d.confidence * 100.0, d.risk_reward, &d.reasoning[..d.reasoning.len().min(80)]);
    }

    println!("\n=== LIVE TEST COMPLETE ===");
    Ok(())
}
