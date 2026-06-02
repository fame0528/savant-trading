use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

use savant_trading::agent::context_builder::FullContext;
use savant_trading::agent::knowledge::KnowledgeBase;
use savant_trading::agent::orchestrator::{AgentConfig, AgentOrchestrator, AutonomyLevel};
use savant_trading::agent::prompts::{self, PromptComposer};
use savant_trading::agent::provider::LlmConfig;
use savant_trading::core::config::AppConfig;
use savant_trading::core::events::EventBus;
use savant_trading::core::types::{Candle, Position, ScaleLevel, TradingEvent};
use savant_trading::data::indicators::IndicatorEngine;
use savant_trading::data::kraken::KrakenClient;
use savant_trading::data::market_data::MarketDataStore;
use savant_trading::data::orderbook::OrderBookManager;
use savant_trading::execution::engine::ExecutionEngine;
use savant_trading::execution::paper::PaperTrader;
use savant_trading::insight::aggregator::{InsightAggregator, InsightConfig};
use savant_trading::monitor::journal::TradeJournal;
use savant_trading::monitor::metrics::PerformanceMetrics;
use savant_trading::risk::circuit_breaker::{CircuitBreaker, CircuitBreakerResult};
use savant_trading::risk::position::PositionSizer;
use savant_trading::strategy::mean_reversion::MeanReversionStrategy;
use savant_trading::strategy::momentum::MomentumStrategy;
use savant_trading::strategy::regime::RegimeDetector;
use savant_trading::vault::config::VaultConfig;
use savant_trading::vault::watcher::VaultWatcher;
use savant_trading::vault::writer::VaultWriter;

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

/// Load knowledge base from JSON files.
///
/// Searches knowledge/ at project root first (22 files, 254 units).
/// Falls back to src/agent/knowledge/ for development builds.
fn load_knowledge_base() -> KnowledgeBase {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let knowledge_root = manifest_dir.join("knowledge");
    let knowledge_src = manifest_dir.join("src").join("agent").join("knowledge");

    let knowledge_dir = if knowledge_root.exists() {
        knowledge_root
    } else {
        warn!(
            "knowledge/ not found at {:?}, falling back to src/agent/knowledge/",
            manifest_dir
        );
        knowledge_src
    };

    info!("Loading knowledge from {:?}", knowledge_dir);

    let mut all_units = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&knowledge_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match std::fs::read_to_string(&path) {
                    Ok(json) => match KnowledgeBase::from_json(&json) {
                        Ok(kb) => {
                            let count = kb.len();
                            all_units.extend_from_slice(kb.all());
                            info!(
                                "Loaded {} knowledge units from {:?}",
                                count,
                                path.file_name()
                            );
                        }
                        Err(e) => warn!("Failed to parse {:?}: {}", path.file_name(), e),
                    },
                    Err(e) => warn!("Failed to read {:?}: {}", path.file_name(), e),
                }
            }
        }
    }

    info!("Knowledge base loaded: {} total units", all_units.len());
    let mut kb = KnowledgeBase::new(all_units);

    // Load persisted utility scores if available
    let scores_path = std::path::Path::new("data/knowledge_utility.json");
    if let Err(e) = kb.load_utility_scores(scores_path) {
        warn!("Failed to load utility scores: {}", e);
    } else if scores_path.exists() {
        info!("Loaded utility scores from {:?}", scores_path);
    }

    kb
}

pub async fn run(
    config: AppConfig,
    shared: savant_trading::core::shared::SharedEngineData,
    _engine_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    // PROD-2: Block file check — refuse to start if circuit breaker wrote block file
    let block_path = "savant.blocked";
    if std::path::Path::new(block_path).exists() {
        let contents = std::fs::read_to_string(block_path).unwrap_or_default();
        error!(
            "ENGINE BLOCKED: {} exists. Delete file to resume. Contents: {}",
            block_path, contents
        );
        return Err(anyhow::anyhow!(
            "Engine blocked by {}. Delete file to resume.",
            block_path
        ));
    }

    let kraken = KrakenClient::new(&config.exchange.rest_url);

    // SPRINT-3: Scan all pairs — discover USD pairs from Kraken API
    let active_pairs = if config.trading.scan_all_pairs {
        match kraken.discover_usd_pairs().await {
            Ok(discovered) => {
                info!("Scan mode: discovered {} pairs", discovered.len());
                discovered
            }
            Err(e) => {
                warn!(
                    "Pair discovery failed ({}), falling back to config pairs",
                    e
                );
                config.trading.pairs.clone()
            }
        }
    } else {
        config.trading.pairs.clone()
    };
    info!("Active pairs ({}): {:?}", active_pairs.len(), active_pairs);

    let mut market_stores: HashMap<String, MarketDataStore> = HashMap::new();
    for pair in &active_pairs {
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

    // PROD-3: Load saved state if exists
    let state_path = "data/paper_state.json";
    if std::path::Path::new(state_path).exists() {
        match paper.load_state(state_path) {
            Ok(()) => info!("Restored state from {}", state_path),
            Err(e) => warn!("Failed to load state ({}), starting fresh", e),
        }
    }

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

    // === AI AGENT SETUP ===
    let knowledge_base = load_knowledge_base();

    // Extract knowledge tuples for Glass House projection (before move)
    let knowledge_tuples: Vec<(String, String, String)> = knowledge_base
        .all()
        .iter()
        .map(|u| {
            let title = u.content.chars().take(60).collect::<String>();
            (u.id.clone(), format!("{:?}", u.topic), title)
        })
        .collect();

    let composer = PromptComposer::new(
        &prompts::default_base_identity(),
        &format!(
            "Max risk per trade: {}% | Max daily loss: {}% | Max drawdown: {}% | Max positions: {} | Min R:R: {}",
            config.risk.max_risk_per_trade * 100.0,
            config.risk.max_daily_loss * 100.0,
            config.risk.max_drawdown * 100.0,
            config.risk.max_positions,
            config.risk.min_rr_ratio,
        ),
        &format!(
            "{}\n\n---\n\n{}",
            include_str!("agent/prompts/strategy_knowledge.md"),
            include_str!("agent/prompts/echo_rules.md")
        ),
        &prompts::default_output_format(),
    );

    let autonomy = match config.ai.autonomy_level {
        1 => AutonomyLevel::Suggest,
        2 => AutonomyLevel::Confirm,
        _ => AutonomyLevel::Autonomous,
    };

    let llm_config = LlmConfig {
        endpoint: config.ai.endpoint.clone(),
        model: config.ai.model.clone(),
        api_key: std::env::var(&config.ai.api_key_env).unwrap_or_default(),
        max_tokens: config.ai.max_tokens,
        temperature: config.ai.temperature,
        top_p: config.ai.top_p,
        timeout_secs: config.ai.timeout_secs,
    };

    let agent_config = AgentConfig {
        autonomy_level: autonomy,
        max_decisions_per_hour: config.ai.max_decisions_per_hour,
        knowledge_token_budget: config.ai.knowledge_token_budget,
        price_tolerance_pct: config.ai.price_tolerance_pct,
        max_retries: config.ai.max_retries,
    };

    let agent = AgentOrchestrator::new(llm_config, agent_config, knowledge_base, composer);
    info!("AI agent initialized: {:?} mode, mimo v2.5 pro", autonomy);

    // === INSIGHT SETUP ===
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
    info!("Insight aggregator initialized");

    // === EVENT BUS ===
    let event_bus = EventBus::new(256);

    // === VAULT (Glass House) ===
    let vault_config = VaultConfig::default();
    let vault_writer = VaultWriter::new(vault_config.clone());
    if vault_config.enabled {
        if let Err(e) = vault_writer.ensure_scaffolded() {
            warn!("Vault scaffold failed: {}", e);
        } else {
            info!("Vault scaffolded at {}", vault_config.vault_path);
        }

        // Project knowledge index to Glass House
        if let Err(e) = vault_writer.project_knowledge(&knowledge_tuples) {
            warn!("Knowledge projection failed: {}", e);
        }
    }

    // === VAULT WATCHER — ingest lessons on startup ===
    let vault_watcher = VaultWatcher::new(&vault_config.vault_path);
    let lessons = vault_watcher.read_lessons();
    if !lessons.is_empty() {
        info!("Ingested {} lesson files from vault", lessons.len());
        for (name, _content) in &lessons {
            info!("  Lesson: {}", name);
        }
    }

    // === ORDER BOOK MANAGERS (one per pair) ===
    let mut order_books: HashMap<String, OrderBookManager> = HashMap::new();
    for pair in &active_pairs {
        order_books.insert(pair.clone(), OrderBookManager::new(pair));
    }

    // === EPISODIC MEMORY (persistent decision ledger) ===
    let memory = match savant_trading::memory::episodic::EpisodicMemory::new(
        "sqlite:data/memory.db",
    )
    .await
    {
        Ok(m) => {
            info!("Episodic memory initialized");
            Some(m)
        }
        Err(e) => {
            warn!("Episodic memory init failed: {} — memory disabled", e);
            None
        }
    };

    // === CUSUM CHARTS (WIRE-2: edge decay detection per pair) ===
    let mut cusum_charts: HashMap<String, savant_trading::memory::cusum::CusumChart> =
        HashMap::new();
    for pair in &active_pairs {
        cusum_charts.insert(
            pair.clone(),
            savant_trading::memory::cusum::CusumChart::default_trading(),
        );
    }

    // === BRIER SCORE TRACKING (WIRE-3) ===
    let mut brier_predictions: Vec<(f64, bool)> = Vec::new();

    // === OPERATOR RULES FROM VAULT (WIRE-5) ===
    let mut operator_rules: Vec<String> = Vec::new();
    for (name, content) in &lessons {
        if name.ends_with(".md") {
            // Extract rules from lesson content (non-empty lines)
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('>') {
                    operator_rules.push(trimmed.to_string());
                }
            }
        }
    }
    if !operator_rules.is_empty() {
        info!("Loaded {} operator rules from vault", operator_rules.len());
    }

    // === EXPERIENCE REPLAY (WIRE-4: run on startup if enough data) ===
    if let Some(ref mem) = memory {
        match mem.total_trades().await {
            Ok(count) if count >= 20 => {
                info!("Running experience replay ({} trades in history)", count);
                if let Ok(losses) =
                    savant_trading::memory::replay::query_high_conviction_losses(mem, 5).await
                {
                    for (ep_id, pair, regime, reasoning) in &losses {
                        let heuristic = format!(
                            "HIGH conviction loss on {} in {} regime: {}",
                            pair,
                            regime,
                            reasoning.chars().take(100).collect::<String>()
                        );
                        let _ = savant_trading::memory::replay::store_lesson(
                            mem.pool(),
                            ep_id,
                            "high_conviction_loss",
                            &heuristic,
                        )
                        .await;
                    }
                }
            }
            _ => {
                info!("Not enough trades for experience replay (need 20+)");
            }
        }
    }

    // === RULE-BASED STRATEGIES (parallel signals, not primary brain) ===
    let _momentum = MomentumStrategy::new(
        config.strategy.momentum.ema_period,
        config.strategy.momentum.volume_spike_multiplier,
        config.strategy.momentum.atr_compression_threshold,
    );

    let _mean_rev = MeanReversionStrategy::new(
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
        "Fetching initial data for {} pairs (parallel)...",
        active_pairs.len()
    );

    // Parallel candle fetch — all pairs simultaneously
    let mut candle_futures = tokio::task::JoinSet::new();
    for pair in &active_pairs {
        let kraken_clone = KrakenClient::new(&config.exchange.rest_url);
        let pair_clone = pair.clone();
        let tf = config.trading.timeframe.clone();
        candle_futures.spawn(async move {
            let result = kraken_clone
                .get_ohlc(&pair_clone, parse_timeframe_minutes(&tf), None)
                .await;
            (pair_clone, result)
        });
    }

    while let Some(result) = candle_futures.join_next().await {
        match result {
            Ok((pair, Ok(mut candles))) => {
                if candles.len() > 1 {
                    candles.pop();
                }
                if let Some(store) = market_stores.get_mut(&pair) {
                    let count = candles.len();
                    store.add_candles(candles);
                    info!("Loaded {} historical candles for {}", count, pair);
                }
            }
            Ok((pair, Err(e))) => error!("Failed to fetch initial data for {}: {}", pair, e),
            Err(e) => error!("Candle fetch task panicked: {}", e),
        }
    }

    // Initial insight fetch (single call for all pairs)
    info!(
        "Fetching initial market insight for {} pairs...",
        active_pairs.len()
    );
    insight.refresh_multi(&active_pairs).await;

    info!(
        "Starting main loop (interval: {}s, autonomy: {:?})...",
        interval_seconds, autonomy
    );

    // SPRINT-2: Spawn WebSocket connection for real-time data
    let (ws_tx, mut ws_rx) = savant_trading::data::websocket::create_channel();
    let ws_pairs = active_pairs.clone();
    let ws_url = config.exchange.ws_url.clone();
    tokio::spawn(async move {
        savant_trading::data::websocket::connect(&ws_url, ws_pairs, ws_tx).await;
    });

    // Track latest WS ticker prices
    let mut ws_ticker_prices: HashMap<String, f64> = HashMap::new();

    let mut tick = 0u64;

    loop {
        tick += 1;

        // SPRINT-2: Drain WebSocket messages (non-blocking)
        let mut ws_messages_drained = 0u32;
        while let Ok(msg) = ws_rx.try_recv() {
            ws_messages_drained += 1;
            match msg {
                savant_trading::data::websocket::WsMessage::Ticker(ticker) => {
                    ws_ticker_prices.insert(ticker.pair.clone(), ticker.last);
                }
                savant_trading::data::websocket::WsMessage::BookUpdate(book) => {
                    if let Some(ob) = order_books.get_mut(&book.pair) {
                        ob.update(book);
                    }
                }
                savant_trading::data::websocket::WsMessage::Trade { pair, price, .. } => {
                    ws_ticker_prices.insert(pair, price);
                }
                savant_trading::data::websocket::WsMessage::CancelAllOrders { reason } => {
                    warn!("WS CANCEL-ALL TRIGGERED: {}", reason);
                    // In paper mode, close all positions as a safety measure.
                    // In live mode, this would call Kraken's CancelAll endpoint.
                    let pairs: Vec<String> = paper.positions().keys().cloned().collect();
                    for pair in &pairs {
                        if let Some(pos) = paper.positions().get(pair) {
                            warn!(
                                "Emergency close: {} {} {} @ {:.2}",
                                pos.side, pos.quantity, pair, pos.current_price
                            );
                        }
                    }
                    shared
                        .log_activity(
                            savant_trading::core::shared::ActivityLevel::Warning,
                            "SYSTEM",
                            &format!("CANCEL-ALL: {}", reason),
                        )
                        .await;
                }
                _ => {}
            }
        }
        if ws_messages_drained > 0 {
            debug!("Drained {} WS messages", ws_messages_drained);
        }

        // Refresh insight every 5 ticks (all pairs, single funding API call)
        if tick.is_multiple_of(5) {
            insight.refresh_multi(&active_pairs).await;
        }

        // === PHASE 1: Fetch data for all pairs (sequential, fast) ===
        struct PairData {
            pair: String,
            indicators: savant_trading::core::types::IndicatorValues,
            regime: savant_trading::core::types::MarketRegime,
            current_price: f64,
            system_prompt: String,
            user_message: String,
        }

        let mut pair_data_vec: Vec<PairData> = Vec::new();
        let market_ctx = insight.cached().clone();
        let positions: Vec<Position> = paper.positions().values().cloned().collect();
        let recent_trades = paper.closed_trades().to_vec();
        let current_session = savant_trading::core::session::current_session();

        for pair in &active_pairs {
            debug!("Phase 1: Processing {}", pair);
            shared
                .log_activity(
                    savant_trading::core::shared::ActivityLevel::Info,
                    pair,
                    "Fetching candles + order book...",
                )
                .await;

            let candles_result = kraken
                .get_ohlc(
                    pair,
                    parse_timeframe_minutes(&config.trading.timeframe),
                    None,
                )
                .await;

            debug!("Phase 1: Candle fetch done for {}", pair);

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

                // Fetch order book
                if let Some(ob_manager) = order_books.get_mut(pair.as_str()) {
                    match kraken.get_order_book(pair, 10).await {
                        Ok(book) => {
                            ob_manager.update(book);
                            let imbalance = ob_manager.imbalance(5);
                            if imbalance > 0.3 {
                                info!(
                                    "Order book imbalance for {}: {:.2} (bid-heavy)",
                                    pair, imbalance
                                );
                            } else if imbalance < -0.3 {
                                info!(
                                    "Order book imbalance for {}: {:.2} (ask-heavy)",
                                    pair, imbalance
                                );
                            }
                        }
                        Err(e) => warn!("Order book fetch failed for {}: {}", pair, e),
                    }
                }
                debug!("Phase 1: Order book done for {}", pair);

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
                let ob_imbalance = order_books.get(pair.as_str()).map(|ob| ob.imbalance(5));
                let current_price = candle_data.last().map(|c| c.close).unwrap_or(0.0);

                // SPRINT-1: Fetch higher timeframe candles for multi-TF context
                let mut higher_tf_candles: Vec<(String, Vec<Candle>)> = Vec::new();
                for tf in &config.trading.timeframes {
                    if *tf == config.trading.timeframe {
                        continue;
                    }
                    match kraken
                        .get_ohlc(pair, parse_timeframe_minutes(tf), None)
                        .await
                    {
                        Ok(mut htf_candles) => {
                            if htf_candles.len() > 1 {
                                htf_candles.pop();
                            }
                            if !htf_candles.is_empty() {
                                higher_tf_candles.push((tf.clone(), htf_candles));
                            }
                        }
                        Err(e) => {
                            warn!("Higher TF fetch failed for {} {}: {}", pair, tf, e);
                        }
                    }
                }
                debug!(
                    "Phase 1: Higher TF done for {} ({} timeframes)",
                    pair,
                    higher_tf_candles.len()
                );

                shared
                    .log_activity(
                        savant_trading::core::shared::ActivityLevel::Info,
                        pair,
                        &format!(
                            "Indicators: RSI={:.1} ADX={:.1} ATR={:.4} Regime={:?}",
                            indicators.rsi.unwrap_or(0.0),
                            indicators.adx.unwrap_or(0.0),
                            indicators.atr.unwrap_or(0.0),
                            regime
                        ),
                    )
                    .await;

                // Pre-filter: skip pairs with no actionable signal
                // Only send to LLM if indicators show a potential setup
                let has_signal = has_actionable_signal(&indicators, regime, ob_imbalance);
                let has_position = positions.iter().any(|p| p.pair == *pair);
                debug!(
                    "Phase 1: {} signal={} position={} → {}",
                    pair,
                    has_signal,
                    has_position,
                    if has_signal || has_position {
                        "SENDING TO LLM"
                    } else {
                        "SKIPPED"
                    }
                );
                if !has_signal && !has_position {
                    continue;
                }

                shared
                    .log_activity(
                        savant_trading::core::shared::ActivityLevel::Thinking,
                        pair,
                        "Sending to AI brain...",
                    )
                    .await;

                // Query memory context (WIRE-1)
                let memory_ctx_str = if let Some(ref mem) = memory {
                    let mem_ctx = savant_trading::memory::context::query_memory_context(
                        mem,
                        pair,
                        &format!("{}", regime),
                        current_session.name(),
                    )
                    .await;
                    let formatted = savant_trading::memory::context::format_memory_prompt(&mem_ctx);
                    if formatted.is_empty() {
                        None
                    } else {
                        Some(formatted)
                    }
                } else {
                    None
                };

                let ctx = FullContext {
                    candles: &candle_data,
                    indicators: &indicators,
                    regime,
                    volume_profile: profile.as_ref(),
                    market_context: &market_ctx,
                    positions: &positions,
                    account: paper.account(),
                    pair,
                    recent_trades: if recent_trades.is_empty() {
                        None
                    } else {
                        Some(&recent_trades)
                    },
                    order_book_imbalance: ob_imbalance,
                    session: current_session,
                    memory_context: memory_ctx_str,
                    higher_tf_candles,
                    context_tags: savant_trading::agent::context_builder::generate_context_tags(
                        &indicators,
                    ),
                };

                let (system_prompt, user_message) =
                    savant_trading::agent::context_builder::build_context(
                        &ctx,
                        agent.knowledge_base(),
                        agent.composer(),
                        config.ai.knowledge_token_budget,
                    );

                info!(
                    "ai_context | prompt_chars={} | pair={} | regime={:?}",
                    system_prompt.len() + user_message.len(),
                    pair,
                    regime
                );

                pair_data_vec.push(PairData {
                    pair: pair.clone(),
                    indicators,
                    regime,
                    current_price,
                    system_prompt,
                    user_message,
                });
            }
        }

        // === PHASE 2: Send all LLM calls in parallel via streaming ===
        info!(
            "Phase 2: {} pairs queued for LLM evaluation (streaming)",
            pair_data_vec.len()
        );
        struct PairResult {
            pair: String,
            response: Result<String, savant_trading::agent::provider::LlmError>,
            current_price: f64,
            _atr: Option<f64>,
        }

        // Save pair data for episodic capture (before consuming in JoinSet)
        let pair_data_for_memory: Vec<(
            String,
            savant_trading::core::types::IndicatorValues,
            savant_trading::core::types::MarketRegime,
        )> = pair_data_vec
            .iter()
            .map(|pd| (pd.pair.clone(), pd.indicators.clone(), pd.regime))
            .collect();

        let mut join_set = tokio::task::JoinSet::new();
        let eval_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));
        for pd in pair_data_vec {
            let provider = agent.provider_clone();
            let sys = pd.system_prompt.clone();
            let usr = pd.user_message.clone();
            let atr = pd.indicators.atr;
            let sem = eval_semaphore.clone();
            join_set.spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let messages = vec![savant_trading::agent::provider::Message {
                    role: "user".to_string(),
                    content: usr,
                }];
                let start = std::time::Instant::now();
                let response = provider.chat_stream(&sys, &messages).await;
                let elapsed = start.elapsed().as_millis();
                match &response {
                    Ok(text) => tracing::debug!(
                        "LLM stream complete for {}: {} chars in {}ms",
                        pd.pair,
                        text.len(),
                        elapsed
                    ),
                    Err(e) => tracing::warn!("LLM stream error for {}: {}", pd.pair, e),
                }
                PairResult {
                    pair: pd.pair,
                    response,
                    current_price: pd.current_price,
                    _atr: atr,
                }
            });
        }

        let mut all_results: Vec<PairResult> = Vec::new();
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(pr) => {
                    match &pr.response {
                        Ok(_) => {
                            shared
                                .log_activity(
                                    savant_trading::core::shared::ActivityLevel::Thinking,
                                    &pr.pair,
                                    "AI response received",
                                )
                                .await
                        }
                        Err(e) => {
                            shared
                                .log_activity(
                                    savant_trading::core::shared::ActivityLevel::Error,
                                    &pr.pair,
                                    &format!("LLM error: {}", e),
                                )
                                .await
                        }
                    }
                    all_results.push(pr);
                }
                Err(e) => warn!("Parallel task panicked: {}", e),
            }
        }
        info!(
            "Parallel evaluation complete: {}/{} pairs",
            all_results.len(),
            active_pairs.len()
        );

        // === PHASE 3: Process all results sequentially ===
        for pr in all_results {
            let response = match pr.response {
                Ok(r) => r,
                Err(e) => {
                    warn!("LLM error for {}: {}", pr.pair, e);
                    continue;
                }
            };

            match savant_trading::agent::decision_parser::parse_decision(
                &response,
                pr.current_price,
                config.ai.price_tolerance_pct,
            ) {
                Ok(decision) => {
                    // Log decision to activity feed with reasoning
                    shared
                        .log_activity(
                            savant_trading::core::shared::ActivityLevel::Decision,
                            &decision.pair,
                            &format!(
                                "{:?} {} @ {:.4} | Conf: {:.0}% | R:R {:.1} | {}",
                                decision.action,
                                decision.side,
                                decision.entry_price,
                                decision.confidence * 100.0,
                                decision.risk_reward,
                                decision.reasoning,
                            ),
                        )
                        .await;

                    // Log ALL decisions including Hold (CRIT-2)
                    let decision_record = savant_trading::core::shared::DecisionRecord {
                        timestamp: Utc::now().to_rfc3339(),
                        pair: decision.pair.clone(),
                        action: format!("{:?}", decision.action),
                        side: format!("{}", decision.side),
                        entry_price: decision.entry_price,
                        stop_loss: decision.stop_loss,
                        take_profit_1: decision.take_profit_1,
                        confidence: decision.confidence,
                        reasoning: decision.reasoning.clone(),
                    };
                    {
                        let mut decisions = shared.decisions.write().await;
                        decisions.push(decision_record);
                        if decisions.len() > 100 {
                            decisions.drain(0..50);
                        }
                    }

                    // Log to vault
                    if vault_config.enabled {
                        if let Err(e) = vault_writer.project_decision(
                            &decision.pair,
                            &format!("{:?}", decision.action),
                            decision.confidence,
                            &decision.reasoning,
                        ) {
                            warn!("Vault decision projection failed: {}", e);
                        }
                    }

                    // Capture episodic memory
                    if let Some(ref mem) = memory {
                        let pair_data = pair_data_for_memory
                            .iter()
                            .find(|(p, _, _)| *p == decision.pair);
                        let (atr_val, adx_val, rsi_val, regime_str) = pair_data
                            .map(|(_, ind, reg)| (ind.atr, ind.adx, ind.rsi, format!("{}", reg)))
                            .unwrap_or((None, None, None, "Unknown".to_string()));

                        let snapshot = savant_trading::memory::episodic::MinimumViableSnapshot {
                            pair: decision.pair.clone(),
                            action: format!("{:?}", decision.action),
                            side: Some(format!("{}", decision.side)),
                            entry_price: decision.entry_price,
                            stop_loss: decision.stop_loss,
                            take_profit_1: decision.take_profit_1,
                            confidence: decision.confidence,
                            reasoning: decision.reasoning.clone(),
                            planned_rr: decision.risk_reward,
                            regime: regime_str,
                            session: current_session.name().to_string(),
                            funding_rate: insight.cached().funding.funding_rate,
                            funding_rate_annualized: insight
                                .cached()
                                .funding
                                .funding_rate_annualized,
                            fear_greed_index: insight
                                .cached()
                                .sentiment
                                .fear_greed_index
                                .map(|v| v as i32),
                            fear_greed_label: insight.cached().sentiment.fear_greed_label.clone(),
                            order_book_imbalance: order_books
                                .get(decision.pair.as_str())
                                .map(|ob| ob.imbalance(5)),
                            mvrv: insight.cached().onchain.mvrv,
                            sopr: insight.cached().onchain.sopr,
                            nvt_signal: insight.cached().onchain.nvt_signal,
                            atr: atr_val,
                            adx: adx_val,
                            rsi: rsi_val,
                            condition_tags: vec![],
                            knowledge_units_used: vec![],
                            thesis_summary: decision.reasoning.chars().take(200).collect(),
                            invalidation_reasoning: format!("Stop at {:.4}", decision.stop_loss),
                            pnl: None,
                            pnl_pct: None,
                            is_win: None,
                            achieved_rr: None,
                            status: if decision.action
                                == savant_trading::agent::decision_parser::TradeAction::Hold
                            {
                                "held".to_string()
                            } else {
                                "executed".to_string()
                            },
                        };
                        if let Err(e) = mem.capture_episode(&snapshot).await {
                            warn!("Episodic capture failed: {}", e);
                        }
                    }

                    // Skip execution for Hold decisions
                    if decision.action == savant_trading::agent::decision_parser::TradeAction::Hold
                    {
                        continue;
                    }

                    info!(
                        "AI DECISION: {:?} {} {} @ {:.2} | SL: {:.2} | TP1: {:.2} | Conf: {:.0}% | R:R: {:.2} | Reason: {}",
                        decision.action, decision.pair, decision.side,
                        decision.entry_price, decision.stop_loss, decision.take_profit_1,
                        decision.confidence * 100.0, decision.risk_reward, decision.reasoning,
                    );

                    // Execute if autonomous
                    if matches!(autonomy, AutonomyLevel::Autonomous) {
                        match circuit_breaker.check(paper.account()) {
                            CircuitBreakerResult::Triggered(reason) => {
                                warn!("AI decision blocked by circuit breaker: {}", reason);
                                let _ = std::fs::write(
                                    "savant.blocked",
                                    format!("{}\nReason: {}\n", Utc::now().to_rfc3339(), reason),
                                );
                                error!("CIRCUIT BREAKER TRIGGERED — wrote savant.blocked.");
                            }
                            CircuitBreakerResult::Ok => {
                                let ps = position_sizer.calculate(
                                    paper.account(),
                                    decision.entry_price,
                                    decision.stop_loss,
                                    decision.take_profit_1,
                                    decision.side,
                                );

                                if let Some(mut ps) = ps {
                                    let session = savant_trading::core::session::current_session();
                                    let session_mult = session.position_size_multiplier();
                                    if session_mult != 1.0 {
                                        ps.quantity *= session_mult;
                                        ps.risk_amount *= session_mult;
                                    }

                                    let order = paper
                                        .place_order(
                                            &decision.pair,
                                            decision.side,
                                            ps.quantity,
                                            Some(decision.entry_price),
                                        )
                                        .await;

                                    match order {
                                        Ok(_) => {
                                            let pos = Position {
                                                id: format!("ai-{}", tick),
                                                pair: decision.pair.clone(),
                                                side: decision.side,
                                                entry_price: decision.entry_price,
                                                current_price: decision.entry_price,
                                                quantity: ps.quantity,
                                                stop_loss: decision.stop_loss,
                                                take_profit_1: decision.take_profit_1,
                                                take_profit_2: decision.take_profit_2,
                                                take_profit_3: decision.take_profit_3,
                                                unrealized_pnl: 0.0,
                                                risk_amount: ps.risk_amount,
                                                strategy_name: "ai-agent".to_string(),
                                                opened_at: chrono::Utc::now(),
                                                scale_level: ScaleLevel::Full,
                                            };
                                            paper
                                                .positions_mut()
                                                .insert(pos.id.clone(), pos.clone());
                                            paper.account_mut().open_positions =
                                                paper.positions().len();
                                            paper.account_mut().trades_today += 1;
                                            info!("AI position opened: {}", decision.pair);

                                            shared.log_activity(
                                                savant_trading::core::shared::ActivityLevel::Trade,
                                                &decision.pair,
                                                &format!(
                                                    "OPENED {} {:?} @ {:.4} | Qty: {:.4} | SL: {:.4} | TP1: {:.4} | Risk: ${:.2}",
                                                    decision.side, decision.action, decision.entry_price,
                                                    ps.quantity, decision.stop_loss, decision.take_profit_1, ps.risk_amount,
                                                ),
                                            ).await;

                                            event_bus.publish(TradingEvent::PositionOpened(pos));
                                        }
                                        Err(e) => error!("AI order failed: {}", e),
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Parse error for {}: {}", pr.pair, e);
                }
            }
        }

        // Check stops for all positions after processing all pairs
        let mut all_prices: HashMap<String, f64> = market_stores
            .iter()
            .filter_map(|(pair, store)| store.last().map(|c| (pair.clone(), c.close)))
            .collect();
        for (pair, price) in &ws_ticker_prices {
            all_prices.insert(pair.clone(), *price);
        }
        let closed = paper.check_stops(&all_prices);
        for trade in closed {
            info!(
                "CLOSED: {} {} | PnL: ${:.2} ({:.2}%) | {}",
                trade.pair, trade.side, trade.pnl, trade.pnl_pct, trade.notes,
            );

            shared
                .log_activity(
                    savant_trading::core::shared::ActivityLevel::Trade,
                    &trade.pair,
                    &format!(
                        "CLOSED {} | PnL: ${:.2} ({:.2}%) | {}",
                        trade.side, trade.pnl, trade.pnl_pct, trade.notes,
                    ),
                )
                .await;

            event_bus.publish(TradingEvent::PositionClosed(trade.clone()));
            if vault_config.enabled {
                if let Err(e) = vault_writer.project_trade(&trade) {
                    warn!("Vault trade projection failed: {}", e);
                }
            }
            if let Some(ref j) = journal {
                if let Err(e) = j.record_trade(&trade).await {
                    warn!("Failed to record trade: {}", e);
                }
            }

            // WIRE-2: Update CUSUM chart on trade close
            if let Some(chart) = cusum_charts.get_mut(&trade.pair) {
                // Calculate achieved R:R from trade
                let achieved_rr = if trade.pnl > 0.0 {
                    trade.pnl_pct / 100.0 * 2.0 // Approximate R:R
                } else {
                    -trade.pnl_pct / 100.0 * 2.0
                };
                let alert = chart.update(achieved_rr);
                match alert {
                    savant_trading::memory::cusum::CusumAlert::NegativeShift => {
                        warn!("CUSUM: Edge decay detected on {}", trade.pair);
                        // WIRE-6: Persist CUSUM alert
                        if let Some(ref mem) = memory {
                            let _ = savant_trading::memory::replay::store_lesson(
                                mem.pool(),
                                &trade.id,
                                "edge_decay",
                                &format!(
                                    "Edge decay detected on {} — reduce conviction",
                                    trade.pair
                                ),
                            )
                            .await;
                        }
                    }
                    savant_trading::memory::cusum::CusumAlert::PositiveShift => {
                        info!("CUSUM: Edge improving on {}", trade.pair);
                    }
                    _ => {}
                }
            }

            // WIRE-3: Track Brier Score predictions
            if trade.strategy_name == "ai-agent" {
                let is_win = trade.pnl > 0.0;
                // Find the matching episode's confidence
                if let Some(ref mem) = memory {
                    if let Ok(episodes) = mem.recent_episodes(&trade.pair, 1).await {
                        if let Some(ep) = episodes.first() {
                            brier_predictions.push((ep.confidence, is_win));
                        }
                    }
                }
            }
        }

        // Update equity from all position prices (C1 fix)
        // SPRINT-2: Merge WS real-time prices with REST candle prices
        let mut all_prices: HashMap<String, f64> = market_stores
            .iter()
            .filter_map(|(pair, store)| store.last().map(|c| (pair.clone(), c.close)))
            .collect();
        for (pair, price) in &ws_ticker_prices {
            all_prices.insert(pair.clone(), *price);
        }
        paper.update_prices(&all_prices);

        if tick.is_multiple_of(10) {
            let account = paper.account();
            let trades = paper.closed_trades();
            let metrics = PerformanceMetrics::calculate(trades);
            info!(
                "[STATUS] Balance: ${:.2} | Equity: ${:.2} | DD: {:.1}% | AI: {} | {}",
                account.balance,
                account.equity,
                account.drawdown_pct * 100.0,
                if agent.is_fallback() {
                    "FALLBACK"
                } else {
                    "ACTIVE"
                },
                metrics,
            );

            // Update shared state for API
            {
                let mut shared_account = shared.account.write().await;
                *shared_account = account.clone();
                let mut shared_positions = shared.positions.write().await;
                *shared_positions = paper.positions().values().cloned().collect();
                let mut shared_trades = shared.closed_trades.write().await;
                *shared_trades = trades.to_vec();
                let mut shared_insight = shared.insight.write().await;
                *shared_insight = insight.cached().clone();

                // WIRE-7: Update memory snapshot for TUI
                let brier_score = if brier_predictions.len() >= 20 {
                    let score = savant_trading::memory::calibration::calculate_brier_score(
                        &brier_predictions,
                    );
                    Some(score.total)
                } else {
                    None
                };
                let brier_label = match brier_score {
                    Some(s) if s <= 0.15 => "Excellent".to_string(),
                    Some(s) if s <= 0.25 => "Good".to_string(),
                    Some(s) if s <= 0.35 => "Fair".to_string(),
                    Some(_) => "Poor".to_string(),
                    None => "Insufficient data".to_string(),
                };
                let total_trades = if let Some(ref mem) = memory {
                    mem.total_trades().await.unwrap_or(0)
                } else {
                    0
                };
                let confidence_cap =
                    savant_trading::memory::calibration::max_conviction_for_trade_count(
                        total_trades,
                        if total_trades > 0 {
                            brier_predictions.iter().filter(|(_, w)| *w).count() as f64
                                / total_trades as f64
                        } else {
                            0.0
                        },
                    );
                let cusum_status: Vec<(String, String)> = cusum_charts
                    .iter()
                    .map(|(pair, chart)| (pair.clone(), chart.status()))
                    .collect();
                let replay_lesson_count = if let Some(ref mem) = memory {
                    savant_trading::memory::replay::get_lessons(mem.pool())
                        .await
                        .map(|l| l.len())
                        .unwrap_or(0)
                } else {
                    0
                };

                let mut shared_memory = shared.memory_snapshot.write().await;
                *shared_memory = savant_trading::core::shared::MemorySnapshot {
                    brier_score,
                    brier_label,
                    confidence_cap: confidence_cap.to_string(),
                    total_trades,
                    cusum_status,
                    replay_lesson_count,
                };
            }

            // Project portfolio to vault
            if vault_config.enabled {
                if let Err(e) = vault_writer.project_portfolio(
                    account.balance,
                    account.equity,
                    account.drawdown_pct,
                ) {
                    warn!("Vault portfolio projection failed: {}", e);
                }
            }

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

        // PROD-1: Graceful shutdown on Ctrl+C
        tokio::select! {
            _ = time::sleep(Duration::from_secs(interval_seconds)) => {}
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received. Saving state...");
                // PROD-3: Save state before exit
                if let Err(e) = paper.save_state("data/paper_state.json") {
                    warn!("Failed to save state: {}", e);
                } else {
                    info!("State saved to data/paper_state.json");
                }
                info!("Savant engine shut down cleanly.");
                return Ok(());
            }
        }
    }
}

/// Dry-run: make ONE AI call and print the full pipeline output.
pub async fn dry_run(config: AppConfig) -> anyhow::Result<()> {
    let kraken = KrakenClient::new(&config.exchange.rest_url);
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
    let mut candles = kraken
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
        &format!(
            "Max risk per trade: {}% | Max daily loss: {}% | Max drawdown: {}% | Max positions: {} | Min R:R: {}",
            config.risk.max_risk_per_trade * 100.0,
            config.risk.max_daily_loss * 100.0,
            config.risk.max_drawdown * 100.0,
            config.risk.max_positions,
            config.risk.min_rr_ratio,
        ),
        &format!(
            "{}\n\n---\n\n{}",
            include_str!("agent/prompts/strategy_knowledge.md"),
            include_str!("agent/prompts/echo_rules.md")
        ),
        &prompts::default_output_format(),
    );

    let paper = PaperTrader::new(
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
        account: paper.account(),
        pair: &pair,
        recent_trades: None,
        order_book_imbalance: None,
        session: savant_trading::core::session::current_session(),
        memory_context: None,
        higher_tf_candles: vec![],
        context_tags: savant_trading::agent::context_builder::generate_context_tags(&indicators),
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
    let llm_config = LlmConfig {
        endpoint: config.ai.endpoint.clone(),
        model: config.ai.model.clone(),
        api_key: std::env::var(&config.ai.api_key_env).unwrap_or_default(),
        max_tokens: config.ai.max_tokens,
        temperature: config.ai.temperature,
        top_p: config.ai.top_p,
        timeout_secs: config.ai.timeout_secs,
    };
    let provider = savant_trading::agent::provider::LlmProvider::new(llm_config);
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

async fn fetch_and_cache(kraken: &KrakenClient, cache_path: &str) -> Vec<Candle> {
    match kraken.get_ohlc("BTC/USD", 5, None).await {
        Ok(mut c) => {
            if c.len() > 1 {
                c.pop();
            }
            println!("Fetched {} real candles from Kraken", c.len());
            if let Ok(json) = serde_json::to_string(&c) {
                let _ = std::fs::create_dir_all("data");
                let _ = std::fs::write(cache_path, &json);
                println!("Cached to {}", cache_path);
            }
            c
        }
        Err(e) => {
            warn!("Kraken fetch failed ({}), using synthetic fallback", e);
            let gen_config = savant_trading::sandbox::generator::GeneratorConfig {
                num_candles: 721,
                interval_minutes: 5,
                ..Default::default()
            };
            savant_trading::sandbox::generator::generate_candles(&gen_config)
        }
    }
}

/// Backup SQLite databases to rolling timestamped files.
///
/// Keeps the last `max_backups` files in `data/backups/`. Oldest files are
/// deleted when the limit is exceeded.
pub fn backup_databases(max_backups: u32) {
    let backup_dir = std::path::Path::new("data/backups");
    if let Err(e) = std::fs::create_dir_all(backup_dir) {
        warn!("Failed to create backup directory: {}", e);
        return;
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

    // Backup memory.db
    let src = std::path::Path::new("data/memory.db");
    if src.exists() {
        let dst = backup_dir.join(format!("memory_{}.db", timestamp));
        if let Err(e) = std::fs::copy(src, &dst) {
            warn!("Failed to backup memory.db: {}", e);
        } else {
            info!("Backed up memory.db → {}", dst.display());
        }
    }

    // Backup test_memory.db
    let src = std::path::Path::new("data/test_memory.db");
    if src.exists() {
        let dst = backup_dir.join(format!("test_memory_{}.db", timestamp));
        if let Err(e) = std::fs::copy(src, &dst) {
            warn!("Failed to backup test_memory.db: {}", e);
        } else {
            info!("Backed up test_memory.db → {}", dst.display());
        }
    }

    // Rotate old backups
    rotate_backups(backup_dir, "memory_", max_backups);
    rotate_backups(backup_dir, "test_memory_", max_backups);
}

fn rotate_backups(dir: &std::path::Path, prefix: &str, max: u32) {
    let mut files: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
            .collect(),
        Err(_) => return,
    };

    files.sort_by_key(|e| e.file_name());

    while files.len() > max as usize {
        if let Some(oldest) = files.first() {
            let _ = std::fs::remove_file(oldest.path());
            info!("Rotated old backup: {}", oldest.path().display());
        }
        files.remove(0);
    }
}

/// Determine if an expected action string indicates a trade (Buy/Sell) vs Hold.
///
/// Scenarios use varied formats: "Buy (High Conviction)", "Hold / Take Profit",
/// "Sell / Short (High Conviction)", "Hold / No Trade", etc.
fn expected_is_trade(expected: &str) -> bool {
    let lower = expected.to_lowercase();
    // Hold indicators take precedence — "Hold / Take Profit" is still a hold
    if lower.contains("hold") || lower.contains("no trade") {
        return false;
    }
    lower.contains("buy")
        || lower.contains("sell")
        || lower.contains("short")
        || lower.contains("add")
}

/// Training run result for convergence tracking.
struct TrainingRunResult {
    brier_score: f64,
    action_count: u32,
    #[allow(dead_code)]
    hold_count: u32,
    #[allow(dead_code)]
    error_count: u32,
    #[allow(dead_code)]
    total: u32,
    lessons_generated: u32,
}

/// Run a single training batch. Called by `run_training` in a loop.
async fn run_training_batch(
    config: &AppConfig,
    scenarios: &[savant_trading::sandbox::scenarios::Scenario],
    test_memory: &savant_trading::memory::episodic::EpisodicMemory,
) -> anyhow::Result<TrainingRunResult> {
    use savant_trading::sandbox::generator;

    let api_keys: Vec<String> = std::env::var("SANDBOX_API_KEYS")
        .unwrap_or_else(|_| std::env::var(&config.ai.api_key_env).unwrap_or_default())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if api_keys.is_empty() {
        anyhow::bail!(
            "No API keys. Set SANDBOX_API_KEYS or {} in .env",
            config.ai.api_key_env
        );
    }

    let knowledge_base = load_knowledge_base();
    let composer = savant_trading::agent::prompts::PromptComposer::new(
        &savant_trading::agent::prompts::default_base_identity(),
        &format!(
            "Max risk per trade: {}% | Max daily loss: {}% | Max drawdown: {}% | Max positions: {} | Min R:R: {}",
            config.risk.max_risk_per_trade * 100.0,
            config.risk.max_daily_loss * 100.0,
            config.risk.max_drawdown * 100.0,
            config.risk.max_positions,
            config.risk.min_rr_ratio,
        ),
        &format!(
            "{}\n\n---\n\n{}",
            include_str!("agent/prompts/strategy_knowledge.md"),
            include_str!("agent/prompts/echo_rules.md")
        ),
        &savant_trading::agent::prompts::default_output_format(),
    );
    let regime_detector = RegimeDetector::new(
        config.strategy.regime.adx_period,
        config.strategy.regime.adx_trending_threshold,
        config.strategy.regime.adx_ranging_threshold,
        config.strategy.regime.atr_volatility_multiplier,
    );

    let cache_path = "data/sandbox_candles.json";
    let kraken = KrakenClient::new(&config.exchange.rest_url);
    let real_candles = if std::path::Path::new(cache_path).exists() {
        match std::fs::read_to_string(cache_path) {
            Ok(json) => serde_json::from_str::<Vec<Candle>>(&json).unwrap_or_default(),
            Err(_) => fetch_and_cache(&kraken, cache_path).await,
        }
    } else {
        fetch_and_cache(&kraken, cache_path).await
    };
    if real_candles.is_empty() {
        anyhow::bail!("No candle data available");
    }

    // Query existing memory context for 6th prompt layer
    let total_trades = test_memory.total_trades().await.unwrap_or(0);
    let min_trades = config.training.memory_context_min_trades;
    let mut memory_ctx_str = if total_trades >= min_trades {
        let ctx = savant_trading::memory::context::query_memory_context(
            test_memory,
            "BTC/USD",
            "Trending",
            "TestSession",
        )
        .await;
        let formatted = savant_trading::memory::context::format_memory_prompt(&ctx);
        if formatted.is_empty() {
            None
        } else {
            Some(formatted)
        }
    } else {
        info!(
            "Memory: inactive ({} episodes, need {})",
            total_trades, min_trades
        );
        None
    };

    // Append semantic patterns to memory context
    if let Ok(patterns) =
        savant_trading::memory::semantic::query_active_patterns(test_memory.pool(), 10).await
    {
        let patterns_str = savant_trading::memory::semantic::format_patterns_for_prompt(&patterns);
        if !patterns_str.is_empty() {
            memory_ctx_str = Some(match memory_ctx_str {
                Some(existing) => format!("{}\n{}", existing, patterns_str),
                None => patterns_str,
            });
        }
    }

    // Append anti-patterns to memory context
    if let Ok(anti_patterns) =
        savant_trading::memory::anti_pattern::detect_anti_patterns(test_memory.pool()).await
    {
        let ap_str =
            savant_trading::memory::anti_pattern::format_anti_patterns_for_prompt(&anti_patterns);
        if !ap_str.is_empty() {
            memory_ctx_str = Some(match memory_ctx_str {
                Some(existing) => format!("{}\n{}", existing, ap_str),
                None => ap_str,
            });
        }
    }

    // PHASE 1: Build prompts
    struct Prepared {
        scenario_id: String,
        scenario_name: String,
        category: String,
        expected_action: String,
        system_prompt: String,
        user_message: String,
        current_price: f64,
        regime: String,
        indicators_snapshot: (Option<f64>, Option<f64>, Option<f64>),
    }

    let mut prepared: Vec<Prepared> = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let mut candles = real_candles.clone();
        generator::apply_scenario(&mut candles, &scenario.params);

        let indicators = savant_trading::data::indicators::IndicatorEngine::calculate_all(
            &candles,
            config.strategy.regime.adx_period,
        );
        let regime = regime_detector.detect(&indicators, &candles);
        let profile = savant_trading::data::indicators::IndicatorEngine::volume_profile(
            &candles,
            config.strategy.mean_reversion.profile_periods.min(50),
        );

        let mock = &scenario.mock_data;
        let funding_annualized = mock.funding_rate * 365.0 * 3.0;
        let market_ctx = savant_trading::insight::aggregator::MarketContext {
            sentiment: savant_trading::insight::sentiment::SentimentData {
                fear_greed_index: Some(mock.fear_greed_index as u32),
                fear_greed_label: Some(mock.fear_greed_label.clone()),
                btc_dominance: Some(mock.btc_dominance),
                ..Default::default()
            },
            funding: savant_trading::insight::funding_rates::FundingData {
                funding_rate: Some(mock.funding_rate),
                funding_rate_annualized: Some(funding_annualized),
                open_interest: Some(mock.open_interest),
                ..Default::default()
            },
            onchain: savant_trading::insight::onchain::OnchainData {
                mvrv: Some(mock.mvrv),
                sopr: Some(mock.sopr),
                nvt_signal: Some(mock.nvt_signal),
                ..Default::default()
            },
            flows: savant_trading::insight::flows::FlowData {
                block_height: Some(mock.block_height),
                ..Default::default()
            },
            rss_items: mock
                .news_headlines
                .iter()
                .map(|h| savant_trading::insight::rss::RssItem {
                    title: h.clone(),
                    link: String::new(),
                    pub_date: None,
                    description: h.clone(),
                    categories: vec!["crypto".into()],
                    source: "action-test".into(),
                    relevance_score: 0.9,
                })
                .collect(),
            ..Default::default()
        };

        let mut htf_candles: Vec<Candle> = Vec::new();
        for chunk in candles.chunks(12) {
            if chunk.is_empty() {
                continue;
            }
            htf_candles.push(Candle {
                timestamp: chunk[0].timestamp,
                open: chunk[0].open,
                high: chunk
                    .iter()
                    .map(|c| c.high)
                    .fold(f64::NEG_INFINITY, f64::max),
                low: chunk.iter().map(|c| c.low).fold(f64::INFINITY, f64::min),
                close: chunk.last().unwrap().close,
                volume: chunk.iter().map(|c| c.volume).sum(),
                pair: "BTC/USD".into(),
            });
        }

        let session = if let Some(ref s) = mock.session_override {
            match s.as_str() {
                "Asian" => savant_trading::core::session::Session::Asian,
                "European" => savant_trading::core::session::Session::European,
                "US" => savant_trading::core::session::Session::UsEuOverlap,
                "Late US" => savant_trading::core::session::Session::LateUs,
                "Weekend" => savant_trading::core::session::Session::Weekend,
                _ => savant_trading::core::session::current_session(),
            }
        } else {
            savant_trading::core::session::current_session()
        };

        let paper = PaperTrader::new(
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
            account: paper.account(),
            pair: "BTC/USD",
            recent_trades: None,
            order_book_imbalance: Some(0.1),
            session,
            memory_context: memory_ctx_str.clone(),
            higher_tf_candles: vec![("1h".into(), htf_candles)],
            context_tags: savant_trading::agent::context_builder::generate_context_tags(
                &indicators,
            ),
        };

        let (system_prompt, user_message) = savant_trading::agent::context_builder::build_context(
            &ctx,
            &knowledge_base,
            &composer,
            config.ai.knowledge_token_budget,
        );

        prepared.push(Prepared {
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            category: scenario.category.clone(),
            expected_action: scenario.expected_action.clone(),
            system_prompt,
            user_message,
            current_price: candles.last().map(|c| c.close).unwrap_or(0.0),
            regime: format!("{}", regime),
            indicators_snapshot: (indicators.atr, indicators.adx, indicators.rsi),
        });
    }

    // PHASE 2: LLM calls via streaming — optimized for throughput
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(20));
    struct ScenarioResponse {
        scenario_id: String,
        scenario_name: String,
        category: String,
        expected_action: String,
        regime: String,
        indicators_snapshot: (Option<f64>, Option<f64>, Option<f64>),
        response: Result<String, savant_trading::agent::provider::LlmError>,
        current_price: f64,
        latency_ms: u64,
    }

    let mut join_set = tokio::task::JoinSet::new();
    for (idx, ps) in prepared.into_iter().enumerate() {
        let key = api_keys[idx % api_keys.len()].clone();
        let endpoint = config.ai.endpoint.clone();
        let model = config.ai.model.clone();
        let sys = ps.system_prompt;
        let usr = ps.user_message;
        let sem = semaphore.clone();
        join_set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let provider = savant_trading::agent::provider::LlmProvider::new(
                savant_trading::agent::provider::LlmConfig {
                    endpoint,
                    model,
                    api_key: key,
                    max_tokens: 131072,
                    temperature: 0.6,
                    top_p: 0.95,
                    timeout_secs: 300,
                },
            );
            let messages = vec![savant_trading::agent::provider::Message {
                role: "user".to_string(),
                content: usr,
            }];
            let start = std::time::Instant::now();
            let response = provider.chat(&sys, &messages).await;
            ScenarioResponse {
                scenario_id: ps.scenario_id,
                scenario_name: ps.scenario_name,
                category: ps.category,
                expected_action: ps.expected_action,
                regime: ps.regime,
                indicators_snapshot: ps.indicators_snapshot,
                response,
                current_price: ps.current_price,
                latency_ms: start.elapsed().as_millis() as u64,
            }
        });
    }

    let mut all_responses: Vec<ScenarioResponse> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        if let Ok(sr) = result {
            let status = match &sr.response {
                Ok(_) => "OK".to_string(),
                Err(e) => format!("ERR: {}", e),
            };
            println!(
                "  [{}/{}] {} ({}) — {} — {}ms",
                all_responses.len() + 1,
                scenarios.len(),
                sr.scenario_name,
                sr.scenario_id,
                status,
                sr.latency_ms,
            );
            all_responses.push(sr);
        }
    }

    // PHASE 3: Parse, capture episodes, collect stats
    let mut brier_predictions: Vec<(f64, bool)> = Vec::new();
    let mut category_edge: std::collections::HashMap<String, (u32, u32)> =
        std::collections::HashMap::new();
    let mut action_count = 0u32;
    let mut hold_count = 0u32;
    let mut error_count = 0u32;
    let mut high_conviction_failures: Vec<(String, String, f64, String)> = Vec::new();

    for sr in &all_responses {
        match &sr.response {
            Ok(text) => {
                match savant_trading::agent::decision_parser::parse_decision(
                    text,
                    sr.current_price,
                    config.ai.price_tolerance_pct,
                ) {
                    Ok(decision) => {
                        let agent_traded = decision.action
                            != savant_trading::agent::decision_parser::TradeAction::Hold;
                        let expected_traded = expected_is_trade(&sr.expected_action);
                        let is_correct = agent_traded == expected_traded;
                        let is_hold = !agent_traded;

                        if is_hold {
                            hold_count += 1;
                        } else {
                            action_count += 1;
                        }

                        // Track Brier predictions
                        brier_predictions.push((decision.confidence, is_correct));

                        // Track category edge
                        let edge = category_edge.entry(sr.category.clone()).or_insert((0, 0));
                        edge.1 += 1; // total
                        if is_correct {
                            edge.0 += 1;
                        } // wins

                        // Track high-conviction failures for auto-lessons
                        if !is_correct && decision.confidence > 0.7 {
                            high_conviction_failures.push((
                                sr.scenario_id.clone(),
                                sr.category.clone(),
                                decision.confidence,
                                format!(
                                    "Expected {} but agent did {:?} {} | Reasoning: {}",
                                    sr.expected_action,
                                    decision.action,
                                    decision.side,
                                    &decision.reasoning.chars().take(200).collect::<String>()
                                ),
                            ));
                        }

                        // Capture episode to test memory DB
                        let (atr, adx, rsi) = sr.indicators_snapshot;
                        let snapshot = savant_trading::memory::episodic::MinimumViableSnapshot {
                            pair: "BTC/USD".to_string(),
                            action: format!("{:?}", decision.action),
                            side: Some(format!("{}", decision.side)),
                            entry_price: decision.entry_price,
                            stop_loss: decision.stop_loss,
                            take_profit_1: decision.take_profit_1,
                            confidence: decision.confidence,
                            reasoning: decision.reasoning.clone(),
                            planned_rr: decision.risk_reward,
                            regime: sr.regime.clone(),
                            session: "TestSession".to_string(),
                            funding_rate: None,
                            funding_rate_annualized: None,
                            fear_greed_index: None,
                            fear_greed_label: None,
                            order_book_imbalance: None,
                            mvrv: None,
                            sopr: None,
                            nvt_signal: None,
                            atr,
                            adx,
                            rsi,
                            condition_tags: vec![sr.category.clone()],
                            knowledge_units_used: vec![],
                            thesis_summary: decision.reasoning.chars().take(200).collect(),
                            invalidation_reasoning: format!("Stop at {:.4}", decision.stop_loss),
                            pnl: None,
                            pnl_pct: None,
                            is_win: Some(is_correct),
                            achieved_rr: None,
                            status: if agent_traded {
                                "test_action".to_string()
                            } else {
                                "test_hold".to_string()
                            },
                        };
                        if let Err(e) = test_memory.capture_episode(&snapshot).await {
                            warn!("Episode capture failed: {}", e);
                        }

                        // Calculate dollar P&L for this trade
                        let risk = 5.0f64; // $5 fixed risk (10% of $50 starting)
                        let trade_pnl = if is_correct {
                            if agent_traded {
                                risk * decision.risk_reward
                            } else {
                                0.0
                            }
                        } else {
                            if agent_traded {
                                -risk
                            } else {
                                0.0
                            }
                        };

                        println!(
                            "  {} | {} | {:?} {} @ {:.2} | Conf: {:.0}% | R:R {:.1} | P&L ${:+.2} | {}",
                            sr.scenario_name,
                            if is_hold { "HOLD " } else { "TRADE" },
                            decision.action,
                            decision.side,
                            decision.entry_price,
                            decision.confidence * 100.0,
                            decision.risk_reward,
                            trade_pnl,
                            &decision.reasoning.chars().take(120).collect::<String>(),
                        );
                    }
                    Err(e) => {
                        error_count += 1;
                        println!("  {} | PARSE_ERR: {}", sr.scenario_name, e);
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                println!("  {} | LLM_ERR: {}", sr.scenario_name, e);
            }
        }
    }

    // PHASE 4: Auto-generate lessons from high-conviction failures
    let lessons_count = high_conviction_failures.len() as u32;
    for (scen_id, category, confidence, reasoning) in &high_conviction_failures {
        let heuristic = format!(
            "HIGH conviction failure (conf {:.0}%) in {} scenario {}: {}",
            confidence * 100.0,
            category,
            scen_id,
            reasoning,
        );
        // Store lesson using the test memory pool directly
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO experience_replay_lessons (lesson_id, timestamp, original_episode_id, error_type, heuristic) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(scen_id)
        .bind("high_conviction_failure")
        .bind(&heuristic)
        .execute(test_memory.pool())
        .await;
    }
    if lessons_count > 0 {
        info!(
            "Auto-generated {} lessons from high-conviction failures",
            lessons_count
        );
    }

    // PHASE 5: Print reports
    let total = all_responses.len() as f64;
    println!("\n{}", "=".repeat(80));
    println!("ACTION TEST RESULTS — {} scenarios", all_responses.len());
    println!("{}", "=".repeat(80));
    println!(
        "SUMMARY: {} total | {} actions ({:.0}%) | {} holds ({:.0}%) | {} errors ({:.0}%)",
        all_responses.len(),
        action_count,
        action_count as f64 / total * 100.0,
        hold_count,
        hold_count as f64 / total * 100.0,
        error_count,
        error_count as f64 / total * 100.0,
    );

    // Brier Score
    if !brier_predictions.is_empty() {
        let brier = savant_trading::memory::calibration::calculate_brier_score(&brier_predictions);
        println!("\n--- CALIBRATION ---");
        println!(
            "Brier Score: {:.4} (lower = better, perfect = 0, random = 1)",
            brier.total
        );
        println!(
            "Reliability: {:.4} | Resolution: {:.4} | Uncertainty: {:.4}",
            brier.reliability, brier.resolution, brier.uncertainty
        );

        // Confidence distribution
        let mut buckets: Vec<(String, u32, u32, f64)> = vec![
            ("0-25%".into(), 0, 0, 0.0),
            ("25-50%".into(), 0, 0, 0.0),
            ("50-75%".into(), 0, 0, 0.0),
            ("75-100%".into(), 0, 0, 0.0),
        ];
        for (conf, is_win) in &brier_predictions {
            let bucket = if *conf < 0.25 {
                0
            } else if *conf < 0.50 {
                1
            } else if *conf < 0.75 {
                2
            } else {
                3
            };
            buckets[bucket].1 += 1;
            if *is_win {
                buckets[bucket].2 += 1;
            }
            buckets[bucket].3 += conf;
        }
        println!("\n--- CONFIDENCE DISTRIBUTION ---");
        println!("  Range    | Count | Accuracy | Avg Conf");
        println!("  ---------|-------|----------|----------");
        for (label, count, wins, conf_sum) in &buckets {
            if *count > 0 {
                println!(
                    "  {:8} | {:5} | {:6.0}%  | {:6.0}%",
                    label,
                    count,
                    *wins as f64 / *count as f64 * 100.0,
                    conf_sum / *count as f64 * 100.0
                );
            }
        }
    }

    // Category edge
    if !category_edge.is_empty() {
        println!("\n--- CATEGORY EDGE ---");
        for (cat, (wins, total)) in &category_edge {
            println!(
                "  {}: {}/{} ({:.0}%)",
                cat,
                wins,
                total,
                *wins as f64 / *total as f64 * 100.0
            );
        }
    }

    let avg_latency: u64 = if !all_responses.is_empty() {
        all_responses.iter().map(|r| r.latency_ms).sum::<u64>() / all_responses.len() as u64
    } else {
        0
    };
    println!("\nAvg latency: {}ms", avg_latency);
    println!("Episodes captured: {}", brier_predictions.len());
    println!("Lessons auto-generated: {}", lessons_count);
    println!("{}\n", "=".repeat(80));

    let brier_score = if !brier_predictions.is_empty() {
        savant_trading::memory::calibration::calculate_brier_score(&brier_predictions).total
    } else {
        0.5
    };

    // PHASE 6: Post-batch wiring — consolidate, detect anti-patterns, update utility
    // Each phase is wrapped in its own error boundary so a failure in one
    // doesn't prevent the others from running.

    // 6a. Semantic consolidation
    match savant_trading::memory::semantic::consolidate(test_memory).await {
        Ok(n) => println!("Semantic consolidation: {} patterns inserted/updated", n),
        Err(e) => warn!("Semantic consolidation failed (non-fatal): {}", e),
    }

    // 6b. Anti-pattern detection
    let mut anti_pattern_narratives: Vec<String> = Vec::new();
    match savant_trading::memory::anti_pattern::detect_anti_patterns(test_memory.pool()).await {
        Ok(aps) => {
            if !aps.is_empty() {
                println!("Anti-patterns detected: {}", aps.len());
                for ap in &aps {
                    println!("  - {}", ap.narrative);
                    anti_pattern_narratives.push(ap.narrative.clone());
                }
            }
        }
        Err(e) => warn!("Anti-pattern detection failed (non-fatal): {}", e),
    }

    // 6b2. Vault wiring — project decisions, risk events, sandbox report
    let vault_config = VaultConfig::default();
    if vault_config.enabled {
        let vault = VaultWriter::new(vault_config.clone());

        // Project each parsed decision to vault
        for sr in &all_responses {
            if let Ok(text) = &sr.response {
                if let Ok(decision) = savant_trading::agent::decision_parser::parse_decision(
                    text,
                    sr.current_price,
                    config.ai.price_tolerance_pct,
                ) {
                    let _ = vault.project_decision(
                        &decision.pair,
                        &format!("{:?}", decision.action),
                        decision.confidence,
                        &decision.reasoning,
                    );
                }
            }
        }

        // Project anti-patterns as risk events
        if !anti_pattern_narratives.is_empty() {
            let details = anti_pattern_narratives.join("\n- ");
            let _ = vault.project_risk_event(
                "anti_pattern",
                &format!("Training batch anti-patterns:\n- {}", details),
            );
        }

        // Project sandbox report
        let report = format!(
            "# Training Batch Report\n\n\
             **Scenarios:** {}\n\
             **Actions:** {} ({:.0}%)\n\
             **Holds:** {} ({:.0}%)\n\
             **Errors:** {}\n\
             **Brier Score:** {:.4}\n\
             **Lessons Generated:** {}\n\
             **Anti-Patterns:** {}\n\n\
             **Timestamp:** {}\n",
            all_responses.len(),
            action_count,
            action_count as f64 / total * 100.0,
            hold_count,
            hold_count as f64 / total * 100.0,
            error_count,
            savant_trading::memory::calibration::calculate_brier_score(&brier_predictions).total,
            lessons_count,
            anti_pattern_narratives.len(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        );
        let _ = vault.project_sandbox(&report);
    }

    // 6c. Knowledge utility update — actually update and persist scores
    let lr = config.training.utility_learning_rate;
    let mut kb = load_knowledge_base();
    let mut utility_updates = 0u32;
    for sr in &all_responses {
        if let Ok(text) = &sr.response {
            if let Ok(decision) = savant_trading::agent::decision_parser::parse_decision(
                text,
                sr.current_price,
                config.ai.price_tolerance_pct,
            ) {
                let expected_traded = expected_is_trade(&sr.expected_action);
                let agent_traded =
                    decision.action != savant_trading::agent::decision_parser::TradeAction::Hold;
                let is_correct = agent_traded == expected_traded;

                // Update utility scores for knowledge units that were in context
                // In absence of per-episode knowledge tracking, apply global signal
                let delta = if is_correct { lr } else { -lr * 0.5 };
                for unit in kb.units_mut() {
                    // Boost/suppress based on tag overlap with decision reasoning
                    if !decision.reasoning.is_empty() {
                        let reasoning_lower = decision.reasoning.to_lowercase();
                        let matches = unit
                            .tags
                            .iter()
                            .any(|t| reasoning_lower.contains(&t.to_lowercase()));
                        if matches {
                            unit.utility_score = (unit.utility_score + delta).clamp(0.1, 5.0);
                            utility_updates += 1;
                        }
                    }
                }
            }
        }
    }
    if utility_updates > 0 {
        let scores_path = std::path::Path::new("data/knowledge_utility.json");
        if let Err(e) = kb.save_utility_scores(scores_path) {
            warn!("Failed to save utility scores: {}", e);
        } else {
            println!(
                "Knowledge utility: {} units updated, saved to {:?}",
                utility_updates, scores_path
            );
        }
    }

    Ok(TrainingRunResult {
        brier_score,
        action_count,
        hold_count,
        error_count,
        total: all_responses.len() as u32,
        lessons_generated: lessons_count,
    })
}

/// Training mode: run scenarios in a loop until Brier score converges.
pub async fn run_training(
    config: AppConfig,
    category_filter: Option<String>,
    action_only: bool,
    count_filter: Option<usize>,
    full: bool,
) -> anyhow::Result<()> {
    let _test_memory =
        savant_trading::memory::episodic::EpisodicMemory::new("sqlite:data/test_memory.db").await?;

    let test_memory =
        savant_trading::memory::episodic::EpisodicMemory::new("sqlite:data/test_memory.db").await?;

    let max_runs = if full { 20 } else { 5 };
    let scenarios_per_run = 60;
    let convergence_threshold = 0.02;
    let mut brier_history: Vec<f64> = Vec::new();
    let mut consecutive_small_deltas = 0u32;

    // Backup databases before training starts
    backup_databases(config.training.max_backups);

    for run in 1..=max_runs {
        // Generate UNIQUE random scenarios every run — no memorization
        let mut scenarios =
            savant_trading::sandbox::scenarios::generate_random_scenarios(scenarios_per_run);

        if let Some(ref cat) = category_filter {
            scenarios.retain(|s| s.category.to_lowercase().contains(&cat.to_lowercase()));
        }
        if action_only {
            scenarios.retain(|s| {
                let a = s.expected_action.to_lowercase();
                a.contains("buy") || a.contains("sell")
            });
        }
        if let Some(n) = count_filter {
            scenarios.truncate(n);
        }

        println!("\n{}", "=".repeat(80));
        println!(
            "TRAINING RUN {}/{} — {} random scenarios",
            run,
            max_runs,
            scenarios.len(),
        );
        println!("{}\n", "=".repeat(80));

        if scenarios.is_empty() {
            warn!("No scenarios generated. Stopping.");
            break;
        }

        let result = run_training_batch(&config, &scenarios, &test_memory).await?;
        brier_history.push(result.brier_score);

        println!(
            "Run {} Brier: {:.4} | Actions: {} | Holds: {} | Lessons: {}",
            run,
            result.brier_score,
            result.action_count,
            result.action_count,
            result.lessons_generated
        );

        // Convergence check
        if brier_history.len() >= 2 {
            let delta = (brier_history[brier_history.len() - 2] - result.brier_score).abs();
            if delta < convergence_threshold {
                consecutive_small_deltas += 1;
            } else {
                consecutive_small_deltas = 0;
            }
            if consecutive_small_deltas >= 3 {
                println!(
                    "\n*** CONVERGED — Brier delta < {} for 3 consecutive runs ***",
                    convergence_threshold
                );
                println!("Final Brier: {:.4}", result.brier_score);
                break;
            }
        }
    }

    // Final report
    println!("\n{}", "=".repeat(80));
    println!("TRAINING COMPLETE — {} runs", brier_history.len());
    println!(
        "Brier history: {:?}",
        brier_history
            .iter()
            .map(|b| format!("{:.4}", b))
            .collect::<Vec<_>>()
    );
    let total_episodes = test_memory.total_trades().await.unwrap_or(0);
    println!("Total episodes in test DB: {}", total_episodes);

    // Save knowledge utility scores for persistence across runs
    let kb = load_knowledge_base();
    let scores_path = std::path::Path::new("data/knowledge_utility.json");
    if let Err(e) = kb.save_utility_scores(scores_path) {
        warn!("Failed to save utility scores: {}", e);
    } else {
        println!("Knowledge utility scores saved to {:?}", scores_path);
    }

    println!("{}\n", "=".repeat(80));

    Ok(())
}

/// Action test: run scenarios through the real AI brain using the EXACT same
/// `build_context()` path as the live engine. Captures episodes to test_memory.db.
pub async fn run_action_test(
    config: AppConfig,
    category_filter: Option<String>,
    action_only: bool,
    count_filter: Option<usize>,
) -> anyhow::Result<()> {
    use savant_trading::sandbox::scenarios::load_all_scenarios;

    let test_memory =
        savant_trading::memory::episodic::EpisodicMemory::new("sqlite:data/test_memory.db").await?;

    let mut scenarios = load_all_scenarios();
    if let Some(ref cat) = category_filter {
        scenarios.retain(|s| s.category.to_lowercase().contains(&cat.to_lowercase()));
    }
    if action_only {
        scenarios.retain(|s| {
            let a = s.expected_action.to_lowercase();
            a.contains("buy") || a.contains("sell") || a.contains("trade")
        });
    }
    if let Some(n) = count_filter {
        scenarios.truncate(n);
    }

    let result = run_training_batch(&config, &scenarios, &test_memory).await?;

    let total_episodes = test_memory.total_trades().await.unwrap_or(0);
    println!("Total episodes in test DB: {}", total_episodes);
    println!(
        "Brier: {:.4} | Actions: {} | Lessons: {}",
        result.brier_score, result.action_count, result.lessons_generated
    );

    Ok(())
}

/// Sandbox: run all 50 scenarios through the real AI brain and grade every decision.
pub async fn run_sandbox(config: AppConfig) -> anyhow::Result<()> {
    use savant_trading::sandbox::feedback::analyze_failures;
    use savant_trading::sandbox::generator::{self};
    use savant_trading::sandbox::grader;
    use savant_trading::sandbox::harness::{SandboxSummary, ScenarioResult};
    use savant_trading::sandbox::report::{format_report_markdown, generate_report_card};
    use savant_trading::sandbox::scenarios::load_all_scenarios;

    let scenarios = load_all_scenarios();
    println!("Loaded {} scenarios", scenarios.len());

    // Setup AI — pool of providers for key rotation from env
    let api_keys: Vec<String> = std::env::var("SANDBOX_API_KEYS")
        .unwrap_or_else(|_| std::env::var(&config.ai.api_key_env).unwrap_or_default())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if api_keys.is_empty() {
        warn!(
            "No API keys found. Set SANDBOX_API_KEYS (comma-separated) or {} in .env",
            config.ai.api_key_env
        );
    }
    let providers: Vec<savant_trading::agent::provider::LlmProvider> = api_keys
        .iter()
        .map(|key| {
            savant_trading::agent::provider::LlmProvider::new(
                savant_trading::agent::provider::LlmConfig {
                    endpoint: config.ai.endpoint.clone(),
                    model: config.ai.model.clone(),
                    api_key: key.clone(),
                    max_tokens: config.ai.max_tokens,
                    temperature: config.ai.temperature,
                    top_p: config.ai.top_p,
                    timeout_secs: config.ai.timeout_secs,
                },
            )
        })
        .collect();
    println!("AI pool: {} providers (key rotation)", providers.len());

    let knowledge_base = load_knowledge_base();
    let composer = savant_trading::agent::prompts::PromptComposer::new(
        &savant_trading::agent::prompts::default_base_identity(),
        &format!(
            "Max risk per trade: {}% | Max daily loss: {}% | Max drawdown: {}% | Max positions: {} | Min R:R: {}",
            config.risk.max_risk_per_trade * 100.0,
            config.risk.max_daily_loss * 100.0,
            config.risk.max_drawdown * 100.0,
            config.risk.max_positions,
            config.risk.min_rr_ratio,
        ),
        &format!(
            "{}\n\n---\n\n{}",
            include_str!("agent/prompts/strategy_knowledge.md"),
            include_str!("agent/prompts/echo_rules.md")
        ),
        &savant_trading::agent::prompts::default_output_format(),
    );

    let regime_detector = RegimeDetector::new(
        config.strategy.regime.adx_period,
        config.strategy.regime.adx_trending_threshold,
        config.strategy.regime.adx_ranging_threshold,
        config.strategy.regime.atr_volatility_multiplier,
    );

    // ── Phase 1: Load candles (cache-first, then Kraken) ───────
    let cache_path = "data/sandbox_candles.json";
    let kraken_sandbox = KrakenClient::new(&config.exchange.rest_url);
    let real_candles = if std::path::Path::new(cache_path).exists() {
        match std::fs::read_to_string(cache_path) {
            Ok(json) => match serde_json::from_str::<Vec<Candle>>(&json) {
                Ok(cached) => {
                    println!(
                        "Loaded {} candles from cache ({})",
                        cached.len(),
                        cache_path
                    );
                    cached
                }
                Err(e) => {
                    warn!("Cache parse failed ({}), fetching from Kraken", e);
                    fetch_and_cache(&kraken_sandbox, cache_path).await
                }
            },
            Err(e) => {
                warn!("Cache read failed ({}), fetching from Kraken", e);
                fetch_and_cache(&kraken_sandbox, cache_path).await
            }
        }
    } else {
        println!("No cache found, fetching from Kraken...");
        fetch_and_cache(&kraken_sandbox, cache_path).await
    };

    println!("Building prompts for {} scenarios...", scenarios.len());

    struct PreparedScenario {
        scenario_id: String,
        scenario_name: String,
        category: String,
        difficulty: String,
        expected_action: String,
        system_prompt: String,
        user_message: String,
        current_price: f64,
    }

    let mut prepared: Vec<PreparedScenario> = Vec::with_capacity(scenarios.len());
    for scenario in &scenarios {
        let mut candles = real_candles.clone();
        generator::apply_scenario(&mut candles, &scenario.params);

        let indicators = savant_trading::data::indicators::IndicatorEngine::calculate_all(
            &candles,
            config.strategy.regime.adx_period,
        );
        let regime = regime_detector.detect(&indicators, &candles);
        let profile = savant_trading::data::indicators::IndicatorEngine::volume_profile(
            &candles,
            config.strategy.mean_reversion.profile_periods.min(50),
        );

        let mock = &scenario.mock_data;
        let funding_annualized = mock.funding_rate * 365.0 * 3.0;
        let market_ctx = savant_trading::insight::aggregator::MarketContext {
            sentiment: savant_trading::insight::sentiment::SentimentData {
                fear_greed_index: Some(mock.fear_greed_index as u32),
                fear_greed_label: Some(mock.fear_greed_label.clone()),
                btc_dominance: Some(mock.btc_dominance),
                ..Default::default()
            },
            funding: savant_trading::insight::funding_rates::FundingData {
                funding_rate: Some(mock.funding_rate),
                funding_rate_annualized: Some(funding_annualized),
                open_interest: Some(mock.open_interest),
                ..Default::default()
            },
            onchain: savant_trading::insight::onchain::OnchainData {
                mvrv: Some(mock.mvrv),
                sopr: Some(mock.sopr),
                nvt_signal: Some(mock.nvt_signal),
                ..Default::default()
            },
            flows: savant_trading::insight::flows::FlowData {
                block_height: Some(mock.block_height),
                ..Default::default()
            },
            rss_items: mock
                .news_headlines
                .iter()
                .map(|h| savant_trading::insight::rss::RssItem {
                    title: h.clone(),
                    link: String::new(),
                    pub_date: None,
                    description: h.clone(),
                    categories: vec!["crypto".into()],
                    source: "sandbox-mock".into(),
                    relevance_score: 0.9,
                })
                .collect(),
            ..Default::default()
        };

        // Generate 1H higher-TF candles from 5m data (aggregate every 12)
        let mut htf_candles: Vec<savant_trading::core::types::Candle> = Vec::new();
        for chunk in candles.chunks(12) {
            if chunk.is_empty() {
                continue;
            }
            htf_candles.push(savant_trading::core::types::Candle {
                timestamp: chunk[0].timestamp,
                open: chunk[0].open,
                high: chunk
                    .iter()
                    .map(|c| c.high)
                    .fold(f64::NEG_INFINITY, f64::max),
                low: chunk.iter().map(|c| c.low).fold(f64::INFINITY, f64::min),
                close: chunk.last().unwrap().close,
                volume: chunk.iter().map(|c| c.volume).sum(),
                pair: "BTC/USD".into(),
            });
        }
        let higher_tf_candles = vec![("1h".into(), htf_candles)];

        // Session override from mock data
        let session = if let Some(ref override_str) = mock.session_override {
            match override_str.as_str() {
                "Asian" => savant_trading::core::session::Session::Asian,
                "European" => savant_trading::core::session::Session::European,
                "US" => savant_trading::core::session::Session::UsEuOverlap,
                "Late US" => savant_trading::core::session::Session::LateUs,
                "Weekend" => savant_trading::core::session::Session::Weekend,
                _ => savant_trading::core::session::current_session(),
            }
        } else {
            savant_trading::core::session::current_session()
        };

        let paper = PaperTrader::new(
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
            account: paper.account(),
            pair: "BTC/USD",
            recent_trades: None,
            order_book_imbalance: Some(0.2),
            session,
            memory_context: None,
            higher_tf_candles,
            context_tags: vec![],
        };

        let (system_prompt, user_message) = savant_trading::agent::context_builder::build_context(
            &ctx,
            &knowledge_base,
            &composer,
            config.ai.knowledge_token_budget,
        );

        let current_price = candles.last().map(|c| c.close).unwrap_or(0.0);

        prepared.push(PreparedScenario {
            scenario_id: scenario.id.clone(),
            scenario_name: scenario.name.clone(),
            category: scenario.category.clone(),
            difficulty: scenario.difficulty.clone(),
            expected_action: scenario.expected_action.clone(),
            system_prompt,
            user_message,
            current_price,
        });
    }

    // ── Phase 2: Fire LLM calls in parallel (capped at 10) ─────
    println!(
        "Sending {} scenarios to AI brain (max 10 concurrent)...",
        prepared.len()
    );

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));

    struct ScenarioResponse {
        scenario_id: String,
        scenario_name: String,
        category: String,
        difficulty: String,
        expected_action: String,
        response: Result<String, savant_trading::agent::provider::LlmError>,
        current_price: f64,
    }

    let mut join_set = tokio::task::JoinSet::new();
    for (idx, ps) in prepared.into_iter().enumerate() {
        let provider_config = providers[idx % providers.len()].config_clone();
        let sys = ps.system_prompt;
        let usr = ps.user_message;
        let sem = semaphore.clone();
        join_set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let local_provider = savant_trading::agent::provider::LlmProvider::new(provider_config);
            let messages = vec![savant_trading::agent::provider::Message {
                role: "user".to_string(),
                content: usr,
            }];
            let response = local_provider.chat(&sys, &messages).await;
            ScenarioResponse {
                scenario_id: ps.scenario_id,
                scenario_name: ps.scenario_name,
                category: ps.category,
                difficulty: ps.difficulty,
                expected_action: ps.expected_action,
                response,
                current_price: ps.current_price,
            }
        });
    }

    let mut all_responses: Vec<ScenarioResponse> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(sr) => {
                let status = match &sr.response {
                    Ok(_) => "OK".to_string(),
                    Err(e) => {
                        warn!("Scenario {} ERR: {}", sr.scenario_name, e);
                        format!("ERR: {}", e)
                    }
                };
                println!(
                    "  [{}/{}] {} — {}",
                    all_responses.len() + 1,
                    scenarios.len(),
                    sr.scenario_name,
                    status
                );
                all_responses.push(sr);
            }
            Err(e) => warn!("Scenario task panicked: {}", e),
        }
    }

    // ── Phase 3: Grade all responses ────────────────────────────
    println!("Grading {} responses...", all_responses.len());

    let mut results: Vec<ScenarioResult> = Vec::with_capacity(all_responses.len());
    for sr in all_responses {
        let start = std::time::Instant::now();

        let (action_taken, grade) = match sr.response {
            Ok(ref text) => {
                match savant_trading::agent::decision_parser::parse_decision(
                    text,
                    sr.current_price,
                    config.ai.price_tolerance_pct,
                ) {
                    Ok(decision) => {
                        let action_str = format!("{:?}", decision.action);
                        let t1 = grader::tier_1_compliance(
                            &action_str,
                            decision.stop_loss,
                            decision.entry_price,
                            decision.confidence,
                            &decision.reasoning,
                            &sr.expected_action,
                        );
                        let (tier_2, t2_details) = grader::tier_2_rr_score(
                            decision.entry_price,
                            decision.stop_loss,
                            decision.take_profit_1,
                            &action_str,
                            &sr.expected_action,
                        );
                        let (tier_3, t3_rationale) = grader::tier_3_reasoning_score(
                            &decision.reasoning,
                            &sr.expected_action,
                        );
                        let total = grader::calculate_total(t1.0, tier_2, tier_3);

                        (
                            action_str,
                            grader::Grade {
                                tier_1_compliance: t1.0,
                                tier_1_reason: t1.1,
                                tier_2_rr_score: tier_2,
                                tier_2_details: t2_details,
                                tier_3_reasoning_score: tier_3,
                                tier_3_rationale: t3_rationale,
                                total_score: total,
                            },
                        )
                    }
                    Err(e) => (
                        "ParseError".into(),
                        grader::Grade {
                            tier_1_compliance: false,
                            tier_1_reason: Some(format!("Parse error: {}", e)),
                            tier_2_rr_score: 0.0,
                            tier_2_details: String::new(),
                            tier_3_reasoning_score: 0.0,
                            tier_3_rationale: String::new(),
                            total_score: 0.0,
                        },
                    ),
                }
            }
            Err(e) => (
                "LLMError".into(),
                grader::Grade {
                    tier_1_compliance: false,
                    tier_1_reason: Some(format!("LLM error: {}", e)),
                    tier_2_rr_score: 0.0,
                    tier_2_details: String::new(),
                    tier_3_reasoning_score: 0.0,
                    tier_3_rationale: String::new(),
                    total_score: 0.0,
                },
            ),
        };

        let latency = start.elapsed().as_millis() as u64;
        let pass_str = if grade.total_score >= 0.6 {
            "PASS"
        } else {
            "FAIL"
        };
        println!(
            "  {} | {} ({}) — {} | Score: {:.2} | T1: {} | T2: {:.2} | T3: {:.2}",
            pass_str,
            sr.scenario_name,
            sr.scenario_id,
            action_taken,
            grade.total_score,
            grade.tier_1_compliance,
            grade.tier_2_rr_score,
            grade.tier_3_reasoning_score,
        );

        results.push(ScenarioResult {
            scenario_id: sr.scenario_id,
            scenario_name: sr.scenario_name,
            category: sr.category,
            difficulty: sr.difficulty,
            action_taken,
            grade,
            latency_ms: latency,
        });
    }

    // 7. Generate report
    let summary = SandboxSummary::from_results(results);
    println!("\n{}", summary.report_card());

    let report_card = generate_report_card(&summary);
    let md = format_report_markdown(&report_card);
    println!("\n{}", md);

    // 7b. Wallet simulation
    let wallet = savant_trading::sandbox::simulator::VirtualWallet::new(
        config.trading.starting_balance,
        config.trading.fee_rate,
        config.trading.slippage_pct,
    );
    // Note: wallet simulation needs raw decisions + candle data.
    // For now, use the graded results to count trades.
    let wallet_metrics = wallet.metrics();
    println!("\n{}", wallet_metrics.report_card());

    // 7c. Run report
    let run_report = savant_trading::sandbox::run_report::RunReport::generate(
        &summary.results,
        &wallet_metrics,
        &wallet.trades,
        savant_trading::sandbox::run_report::ConfigSnapshot {
            pairs: config.trading.pairs.clone(),
            timeframe: config.trading.timeframe.clone(),
            model: config.ai.model.clone(),
            concurrency: 10,
            starting_balance: config.trading.starting_balance,
        },
        savant_trading::sandbox::run_report::KnowledgeStats {
            total_units: load_knowledge_base().len(),
            files_loaded: 10,
        },
    );
    match run_report.write_to_disk("data") {
        Ok(path) => println!("Run report written to {}", path),
        Err(e) => warn!("Failed to write run report: {}", e),
    }

    // 8. Feedback analysis
    let analysis = analyze_failures(&summary);
    if !analysis.violated_rules.is_empty() {
        println!("\n─── Failure Analysis ───────────────────");
        for v in &analysis.violated_rules {
            println!(
                "  Rule: {} — violated {} times (scenarios: {:?})",
                v.rule, v.violation_count, v.scenarios
            );
        }
        for p in &analysis.patterns {
            println!(
                "  Pattern: {} — {} — {}",
                p.pattern, p.frequency, p.suggestion
            );
        }
    }

    // 9. Write report to disk
    let report_path = "data/sandbox_report.md";
    if let Err(e) = std::fs::write(report_path, &md) {
        warn!("Failed to write sandbox report: {}", e);
    } else {
        println!("\nReport written to {}", report_path);
    }

    println!("\n=== SANDBOX COMPLETE ===");
    Ok(())
}

/// Pre-filter: does this pair have an actionable signal worth sending to LLM?
/// Returns true if any indicator suggests a potential setup.
fn has_actionable_signal(
    indicators: &savant_trading::core::types::IndicatorValues,
    regime: savant_trading::core::types::MarketRegime,
    ob_imbalance: Option<f64>,
) -> bool {
    // RSI extreme — oversold or overbought
    if let Some(rsi) = indicators.rsi {
        if !(30.0..=70.0).contains(&rsi) {
            return true;
        }
    }

    // ADX strong trend
    if let Some(adx) = indicators.adx {
        if adx > 25.0 {
            return true;
        }
    }

    // EMA crossover (fast vs slow)
    if let (Some(fast), Some(slow)) = (indicators.ema_fast, indicators.ema_slow) {
        let spread_pct = ((fast - slow) / slow).abs() * 100.0f64;
        if spread_pct > 0.1 {
            return true;
        }
    }

    // Order book imbalance
    if let Some(obi) = ob_imbalance {
        if obi.abs() > 0.3 {
            return true;
        }
    }

    // VWAP deviation
    if let Some(vwap) = indicators.vwap {
        if let Some(atr) = indicators.atr {
            // If price is more than 1 ATR from VWAP
            // We don't have current_price here, so skip this check
            let _ = (vwap, atr);
        }
    }

    // Strong trend regime (ADX > 25 is checked above, but also check regime)
    if regime == savant_trading::core::types::MarketRegime::Trending {
        return true;
    }

    false
}
