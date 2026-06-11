pub mod debug;
pub mod training;
pub mod utils;

pub use debug::{dry_run, run_live_test};
pub use training::{run_action_test, run_sandbox, run_training};
pub use utils::parse_timeframe;
pub use utils::parse_timeframe_minutes;

use utils::{create_executor, derive_address_from_key, load_knowledge_base};
use training::verify_token_safety;

use chrono::{Timelike, Utc};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

use savant_trading::agent::context_builder::FullContext;
use savant_trading::agent::context_engine::ContextEngine;
use savant_trading::agent::context_state::ContextState;
use savant_trading::agent::decision_log::DecisionLog;
use savant_trading::agent::openrouter_management::OpenRouterManagementClient;
use savant_trading::agent::orchestrator::{AgentConfig, AgentOrchestrator, AutonomyLevel};
use savant_trading::agent::prompts::{self, PromptComposer};
use savant_trading::agent::provider::create_provider;
use savant_trading::core::config::AppConfig;
use savant_trading::core::events::EventBus;
use savant_trading::core::types::{Candle, Position, ScaleLevel, Side, TradeRecord, TradingEvent};
use savant_trading::data::candle_client::CandleClient;
use savant_trading::data::indicators::IndicatorEngine;
use savant_trading::data::market_data::MarketDataStore;
use savant_trading::data::orderbook::OrderBookManager;
use savant_trading::execution::engine::ExecutionEngine;
use savant_trading::execution::portfolio::PortfolioManager;
use savant_trading::insight::aggregator::{InsightAggregator, InsightConfig};
use savant_trading::monitor::journal::TradeJournal;
use savant_trading::monitor::metrics::PerformanceMetrics;
use savant_trading::risk::circuit_breaker::{CircuitBreaker, CircuitBreakerResult};
use savant_trading::risk::position::PositionSizer;
use savant_trading::strategy::regime::RegimeDetector;
use savant_trading::vault::config::VaultConfig;
use savant_trading::vault::watcher::VaultWatcher;
use savant_trading::vault::writer::VaultWriter;
use savant_trading::{
    log_circuit, log_decision, log_llm, log_llm_done, log_phase, log_position, log_swap,
    log_swap_fail, log_trade, log_vault, log_warn,
};

/// Engine state — bundles all long-lived variables from run() into a struct.
/// Defined for future decomposition (Sessions 4-8 will extract methods).
#[allow(dead_code)]
struct EngineState {
    // Config & shared
    config: AppConfig,
    shared: savant_trading::core::shared::SharedEngineData,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,

    // Market data
    candle_api: CandleClient,
    active_pairs: Vec<String>,
    curated_pairs: HashSet<String>,
    market_stores: HashMap<String, MarketDataStore>,
    candle_router: std::sync::Arc<savant_trading::data::sources::SourceRouter>,
    interval_seconds: u64,

    // Trading
    portfolio: PortfolioManager,
    executor: Option<Box<dyn ExecutionEngine>>,
    executor_position_map: HashMap<String, String>,
    reconciliation_removed: HashSet<String>,
    failure_tracker: savant_trading::execution::dex::trader::FailureTracker,
    episode_store: HashMap<String, String>,
    journal: Option<TradeJournal>,

    // AI
    agent: AgentOrchestrator,
    ctx_engine: ContextEngine,
    ctx_state: ContextState,
    decision_log: DecisionLog,

    // Monitoring
    insight: InsightAggregator,
    event_bus: EventBus,

    // Vault
    vault_config: VaultConfig,
    vault_writer: VaultWriter,
    vault_watcher: VaultWatcher,

    // Memory & learning
    order_books: HashMap<String, OrderBookManager>,
    memory: Option<savant_trading::memory::episodic::EpisodicMemory>,
    cusum_charts: HashMap<String, savant_trading::memory::cusum::CusumChart>,
    brier_predictions: Vec<(f64, bool)>,
    operator_rules: Vec<String>,

    // Risk
    regime_detector: RegimeDetector,
    position_sizer: PositionSizer,
    circuit_breaker: CircuitBreaker,
    goplus_client: Option<savant_trading::security::goplus::GoPlusClient>,

    // WebSocket & price tracking
    ws_rx: tokio::sync::mpsc::UnboundedReceiver<savant_trading::data::websocket::WsMessage>,
    ws_ticker_prices: HashMap<String, (f64, std::time::Instant)>,
    ws_staleness: HashMap<String, u64>,
    rest_fallback_at: Option<std::time::Instant>,
    ws_just_reconnected: bool,

    // Cycle state
    dead_tokens: HashSet<String>,
    permanent_dead: HashSet<String>,
    candle_hash_cache: HashMap<String, u64>,
    tick: u64,
    eval_in_progress: std::sync::Arc<std::sync::atomic::AtomicBool>,
    last_daily_reset: chrono::NaiveDate,

    // Cooldown & tracking
    close_failure_cooldown: HashMap<String, std::time::Instant>,
    consecutive_sl_count: HashMap<String, u32>,
    sl_halt_until: HashMap<String, std::time::Instant>,
}

impl EngineState {
    /// Initialize engine state from config. Performs all startup setup.
    async fn new(
        config: AppConfig,
        shared: savant_trading::core::shared::SharedEngineData,
        running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> anyhow::Result<Self> {
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

        let candle_api = CandleClient::new(&config.exchange.rest_url);

        // SPRINT-3: Scan all pairs — discover USD pairs from API with safety filters
        let active_pairs = if config.trading.scan_all_pairs {
            match candle_api
                .discover_safe_usd_pairs(
                    config.trading.min_volume_24h_usd,
                    config.trading.min_price_usd,
                    &config.trading.blacklisted_symbols,
                )
                .await
            {
                Ok(discovered) => {
                    info!("Scan mode: discovered {} safe pairs", discovered.len());
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

        // Token DB: load ALL static Arbitrum addresses for resolution (needed by 0x API)
        // When scan_all_pairs is true, discovered pairs are also curated (FID-052 fix)
        let mut curated_pairs: std::collections::HashSet<String> =
            config.trading.pairs.iter().cloned().collect();
        if config.trading.scan_all_pairs {
            for p in &active_pairs {
                curated_pairs.insert(p.clone());
            }
            info!(
                "Curated pairs: {} (config {} + discovered {})",
                curated_pairs.len(),
                config.trading.pairs.len(),
                active_pairs.len()
            );
        }
        if config.mode.live_execution {
            let mut all_token_entries: Vec<(String, String, u8)> = Vec::new();
            for &(sym, addr, dec) in savant_trading::execution::dex::ARBITRUM_TOKENS {
                all_token_entries.push((sym.to_string(), addr.to_string(), dec));
            }
            savant_trading::execution::dex::extend_token_db(&all_token_entries);
            info!(
                "Token DB: {} Arbitrum addresses loaded for resolution",
                all_token_entries.len()
            );
            // P1-1d: Discover additional Arbitrum tokens from Blockscout
            match savant_trading::data::token_discovery::discover_tokens(1_000_000.0, 500, 100).await {
                Ok(discovered) => {
                    let mut discovered_entries: Vec<(String, String, u8)> = Vec::new();
                    for token in &discovered {
                        if !curated_pairs.contains(&format!("{}/USD", token.symbol)) {
                            discovered_entries.push((
                                token.symbol.clone(),
                                token.address.clone(),
                                token.decimals,
                            ));
                        }
                    }
                    if !discovered_entries.is_empty() {
                        savant_trading::execution::dex::extend_token_db(&discovered_entries);
                        info!("Token discovery: {} discovered → {} new pairs added to DB",
                            discovered.len(), discovered_entries.len());
                    } else {
                        info!("Token discovery: {} discovered — all already in curated list", discovered.len());
                    }
                }
                Err(e) => {
                    warn!("Token discovery failed ({}), continuing with static DB only", e);
                }
            }
        }
        info!("Active pairs ({}): {:?}", active_pairs.len(), active_pairs);

        let mut market_stores: HashMap<String, MarketDataStore> = HashMap::new();
        for pair in &active_pairs {
            market_stores.insert(
                pair.clone(),
                MarketDataStore::new(pair, config.strategy.mean_reversion.profile_periods + 100),
            );
        }

        let mut portfolio = PortfolioManager::new(
            config.trading.starting_balance,
            config.trading.fee_rate,
            config.trading.slippage_pct,
        );

        let mut executor: Option<Box<dyn ExecutionEngine>> = None;
        if config.mode.live_execution {
            match create_executor(&config).await {
                Ok(Some(trader)) => {
                    info!(
                        "Live execution engine ready: backend={}",
                        config.exchange.backend
                    );
                    // FID-093 C1: Cache wallet address at startup
                    if let Ok(pk) = std::env::var(&config.exchange.dex.wallet_key_env) {
                        if !pk.is_empty() {
                            if let Ok(addr) = derive_address_from_key(&pk) {
                                let mut wa = shared.wallet_address.write().await;
                                *wa = addr.clone();
                                let masked = if addr.len() > 10 {
                                    format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
                                } else {
                                    addr.clone()
                                };
                                info!("Wallet address cached: {}", masked);
                            }
                        }
                    }
                    executor = Some(trader);
                }
                Ok(None) => {}
                Err(e) => {
                    error!("Failed to initialize live executor: {}", e);
                    warn!("Falling back to PortfolioManager for safety");
                }
            }
        }

        let mut reconciliation_removed: HashSet<String> = HashSet::new();

        if let Some(ref mut ex) = executor {
            if ex.sync_balance().await.is_ok() {
                let on_chain_balance = ex.balance();
                portfolio.set_balance(on_chain_balance);
                info!("Synced on-chain USDC balance: ${:.6}", on_chain_balance);
            }
        }

        if let Some(ref ex) = executor {
            if ex.open_positions().is_empty() && !portfolio.positions().is_empty() {
                warn!(
                    "PHANTOM POSITIONS: executor has 0 positions but PortfolioManager has {}. Clearing PortfolioManager.",
                    portfolio.positions().len()
                );
                for pid in portfolio.positions().keys() {
                    reconciliation_removed.insert(pid.clone());
                }
                portfolio.positions_mut().clear();
                portfolio.account_mut().open_positions = 0;
                portfolio.account_mut().unrealized_pnl = 0.0;
                portfolio.account_mut().peak_equity = portfolio.account().equity;
                portfolio.account_mut().drawdown_pct = 0.0;
                warn!(
                    "FID-097: Reset peak_equity to ${:.2} after phantom position reconciliation",
                    portfolio.account().equity
                );
            }
        }

        let mut executor_position_map: HashMap<String, String> = HashMap::new();
        let failure_tracker = savant_trading::execution::dex::trader::FailureTracker::new();
        let episode_store: HashMap<String, String> = HashMap::new();

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
                if executor.is_none() {
                    let restored_balance = config.trading.starting_balance + total_pnl;
                    info!(
                        "Restored balance: ${:.2} (starting: ${:.2}, total PnL: ${:.2}, trades: {})",
                        restored_balance,
                        config.trading.starting_balance,
                        total_pnl,
                        trades.len()
                    );
                    portfolio.set_balance(restored_balance);
                } else {
                    info!(
                        "Loaded {} closed trades from journal (PnL: ${:.2}) — balance from on-chain: ${:.2}",
                        trades.len(), total_pnl, portfolio.account().balance
                    );
                }
            }

            match j.load_positions().await {
                Ok(db_positions) if !db_positions.is_empty() => {
                    info!("Restored {} open positions from DB", db_positions.len());
                    for mut pos in db_positions {
                        if pos.stop_loss <= 0.0 {
                            let sl_pct = 0.05;
                            pos.stop_loss = match pos.side {
                                Side::Long => pos.entry_price * (1.0 - sl_pct),
                                Side::Short => pos.entry_price * (1.0 + sl_pct),
                            };
                            warn!(
                                "Position {} had no stop-loss — set to {:.4} (5% default)",
                                pos.pair, pos.stop_loss
                            );
                            if let Err(e) = j.save_position(&pos).await {
                                warn!("Failed to persist fixed stop-loss: {}", e);
                            }
                        }
                        let sl_valid = match pos.side {
                            Side::Long => pos.stop_loss < pos.entry_price,
                            Side::Short => pos.stop_loss > pos.entry_price,
                        };
                        if !sl_valid {
                            let old_sl = pos.stop_loss;
                            pos.stop_loss = match pos.side {
                                Side::Long => pos.entry_price * 0.92,
                                Side::Short => pos.entry_price * 1.08,
                            };
                            warn!(
                                "SL DIRECTION FIX: {} {} — SL {:.4} was wrong direction for {:?}. Recalculated to {:.4} (8% buffer)",
                                pos.pair, pos.side, old_sl, pos.side, pos.stop_loss
                            );
                            if let Err(e) = j.save_position(&pos).await {
                                warn!("Failed to persist corrected stop-loss: {}", e);
                            }
                        }
                        info!(
                            "  {} {} | Entry: {:.4} SL: {:.4} TP1: {:.4} | Qty: {:.6}",
                            pos.pair,
                            pos.side,
                            pos.entry_price,
                            pos.stop_loss,
                            pos.take_profit_1,
                            pos.quantity
                        );
                        portfolio.positions_mut().insert(pos.id.clone(), pos);
                    }
                    portfolio.account_mut().open_positions = portfolio.positions().len();
                    info!("Loaded {} positions from DB", portfolio.positions().len());
                    if let Some(ref mut ex) = executor {
                        for (id, pos) in portfolio.positions().iter() {
                            let exec_id = format!("exec-{}", id);
                            ex.register_position(exec_id.clone(), pos.clone());
                            executor_position_map.insert(id.clone(), exec_id.clone());
                        }
                        info!("Registered {} journal positions in DexTrader", portfolio.positions().len());
                    }
                }
                Ok(_) => info!("No persisted positions in DB"),
                Err(e) => warn!("Failed to load positions from DB: {}", e),
            }

            {
                let config_pairs: std::collections::HashSet<&str> = config.trading.pairs.iter().map(|s| s.as_str()).collect();
                let stale_ids: Vec<String> = portfolio.positions().keys()
                    .filter(|id| {
                        portfolio.positions().get(*id).is_some_and(|p| !config_pairs.contains(p.pair.as_str()))
                    })
                    .cloned()
                    .collect();
                let mut stale_removed = false;
                for stale_id in &stale_ids {
                    if let Some(pos) = portfolio.positions_mut().remove(stale_id) {
                        warn!("STALE POSITION REMOVED: {} ({}) — not in current config pairs", stale_id, pos.pair);
                        if let Some(ref j) = journal {
                            let _ = j.delete_position(stale_id).await;
                        }
                        stale_removed = true;
                    }
                }
                if stale_removed {
                    portfolio.account_mut().open_positions = portfolio.positions().len();
                    portfolio.refresh_equity();
                }
            }

            match j.get_trades(500).await {
                Ok(closed) if !closed.is_empty() => {
                    info!("Loaded {} closed trades from journal", closed.len());
                    portfolio.set_closed_trades(closed.clone());
                    let mut shared_trades = shared.closed_trades.write().await;
                    *shared_trades = closed;
                }
                Ok(_) => {
                    info!("No closed trades in journal");
                }
                Err(e) => {
                    warn!("Failed to load closed trades from journal: {}", e);
                }
            }

            match j.load_activity(200).await {
                Ok(entries) if !entries.is_empty() => {
                    let mut shared_activity = shared.activity_log.write().await;
                    for (ts, level, pair, msg) in entries {
                        let lvl = match level.as_str() {
                            "Trade" => savant_trading::core::shared::ActivityLevel::Trade,
                            "Decision" => savant_trading::core::shared::ActivityLevel::Decision,
                            "Warning" => savant_trading::core::shared::ActivityLevel::Warning,
                            "Error" => savant_trading::core::shared::ActivityLevel::Error,
                            "Thinking" => savant_trading::core::shared::ActivityLevel::Thinking,
                            _ => savant_trading::core::shared::ActivityLevel::Info,
                        };
                        shared_activity.push(savant_trading::core::shared::ActivityEntry {
                            timestamp: ts,
                            level: lvl,
                            pair,
                            message: msg,
                        });
                    }
                }
                _ => {
                    info!("No activity entries in journal");
                }
            }

            info!("Equity curve: starting fresh for current session (historical snapshots skipped)");
        }

        {
            let mut shared_account = shared.account.write().await;
            *shared_account = portfolio.account().clone();
            let mut shared_positions = shared.positions.write().await;
            *shared_positions = portfolio.positions().values().cloned().collect();
        }
        info!(
            "Shared state seeded: balance=${:.2}, {} positions, {} trades",
            portfolio.account().balance,
            portfolio.positions().len(),
            shared.closed_trades.read().await.len()
        );

        let knowledge_base = load_knowledge_base();
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
            include_str!("../agent/prompts/risk_constraints.md"),
            &format!(
                "{}\n\n---\n\n{}",
                include_str!("../agent/prompts/strategy_knowledge.md"),
                include_str!("../agent/prompts/echo_rules.md")
            ),
            include_str!("../agent/prompts/output_format.md"),
        );

        let autonomy = match config.ai.autonomy_level {
            1 => AutonomyLevel::Suggest,
            2 => AutonomyLevel::Confirm,
            _ => AutonomyLevel::Autonomous,
        };

        let provider = create_provider(&config.ai);

        let agent_config = AgentConfig {
            autonomy_level: autonomy,
            max_decisions_per_hour: config.ai.max_decisions_per_hour,
            knowledge_token_budget: config.ai.knowledge_token_budget,
            price_tolerance_pct: config.ai.price_tolerance_pct,
            max_retries: config.ai.max_retries,
        };

        let agent = AgentOrchestrator::new(provider, agent_config, knowledge_base, composer);
        let ctx_engine = ContextEngine::new(config.context.clone());
        let ctx_state = ContextState::new(
            config.context.microcompact_soft_ratio,
            config.context.microcompact_hard_ratio,
        );
        let decision_log = DecisionLog::open(
            "data/decision_log.json",
            config.context.decision_log_max_entries,
        );
        info!(
            "AI agent initialized: {:?} mode with provider '{}', encoding={}",
            autonomy, config.ai.provider, ctx_engine.encoding_mode()
        );

        if config.ai.provider == "openrouter" {
            let mgmt_key_env = &config.ai.openrouter.management.management_key_env;
            if let Ok(mgmt_key) = std::env::var(mgmt_key_env) {
                if !mgmt_key.is_empty() {
                    let mgmt = OpenRouterManagementClient::with_endpoint(
                        mgmt_key,
                        &config.ai.openrouter.management.endpoint,
                    );
                    match mgmt.list_keys(None).await {
                        Ok(keys) => {
                            info!(
                                "OpenRouter Management: {} API keys (logged at startup)",
                                keys.len()
                            );
                            for key in &keys {
                                if key.limit > 0.0 && key.limit_remaining < key.limit * 0.1 {
                                    warn!(
                                        "OpenRouter key '{}' is approaching limit: {:.0}/{:.0} credits remaining",
                                        key.name, key.limit_remaining, key.limit
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                "OpenRouter Management unavailable ({}). Key monitoring disabled.",
                                e
                            );
                        }
                    }
                }
            }
        }

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

        let event_bus = EventBus::new(256);

        let vault_config = VaultConfig::default();
        let vault_writer = VaultWriter::new(vault_config.clone());
        if vault_config.enabled {
            if let Err(e) = vault_writer.ensure_scaffolded() {
                warn!("Vault scaffold failed: {}", e);
            } else {
                info!("Vault scaffolded at {}", vault_config.vault_path);
            }
            if let Err(e) = vault_writer.project_knowledge(&knowledge_tuples) {
                warn!("Knowledge projection failed: {}", e);
            }
        }

        let vault_watcher = VaultWatcher::new(&vault_config.vault_path);
        let lessons = vault_watcher.read_lessons();
        if !lessons.is_empty() {
            info!("Ingested {} lesson files from vault", lessons.len());
            for (name, _content) in &lessons {
                info!("  Lesson: {}", name);
            }
        }

        let mut order_books: HashMap<String, OrderBookManager> = HashMap::new();
        for pair in &active_pairs {
            order_books.insert(pair.clone(), OrderBookManager::new(pair));
        }

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

        let mut cusum_charts: HashMap<String, savant_trading::memory::cusum::CusumChart> =
            HashMap::new();
        for pair in &active_pairs {
            cusum_charts.insert(
                pair.clone(),
                savant_trading::memory::cusum::CusumChart::default_trading(),
            );
        }

        let brier_predictions: Vec<(f64, bool)> = Vec::new();

        let mut operator_rules: Vec<String> = Vec::new();
        for (name, content) in &lessons {
            if name.ends_with(".md") {
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

        let regime_detector = RegimeDetector::new(
            config.strategy.regime.adx_period,
            config.strategy.regime.adx_trending_threshold,
            config.strategy.regime.adx_ranging_threshold,
            config.strategy.regime.atr_volatility_multiplier,
        );

        let position_sizer =
            PositionSizer::new(config.risk.max_risk_per_trade, config.risk.min_rr_ratio)
                .with_full_deploy(config.trading.full_deploy)
                .with_low_balance_rr(
                    config.risk.min_rr_ratio_low_balance,
                    config.risk.low_balance_threshold,
                );

        let circuit_breaker = CircuitBreaker::new(
            config.risk.max_daily_loss,
            config.risk.max_drawdown,
            config.risk.max_positions,
        )
        .with_daily_loss_floor(config.risk.min_daily_loss_usd)
        .with_drawdown_floor(config.risk.min_drawdown_usd);

        let goplus_client = Some(savant_trading::security::goplus::GoPlusClient::new());

        let interval_seconds = parse_timeframe(&config.trading.timeframe);

        info!(
            "Fetching initial data for {} pairs (parallel)...",
            active_pairs.len()
        );

        let candle_router =
            std::sync::Arc::new(savant_trading::data::sources::SourceRouter::new(vec![
                Box::new(savant_trading::data::sources::kraken::KrakenFeed::new(
                    &config.exchange.rest_url,
                )),
                Box::new(savant_trading::data::sources::okx::OkxSource::new()),
                Box::new(savant_trading::data::sources::kucoin::KuCoinSource::new()),
                Box::new(savant_trading::data::sources::gate::GateSource::new()),
                Box::new(savant_trading::data::sources::cryptocompare::CryptoCompareSource::new()),
                Box::new(savant_trading::data::sources::coingecko::CoinGeckoSource::new()),
            ]));

        let mut candle_futures = tokio::task::JoinSet::new();
        for pair in &active_pairs {
            let router = candle_router.clone();
            let pair_clone = pair.clone();
            let tf = config.trading.timeframe.clone();
            candle_futures.spawn(async move {
                let result = router
                    .fetch_candles(&pair_clone, parse_timeframe_minutes(&tf), 200)
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
        info!(
            "Fetching initial market insight for {} pairs...",
            active_pairs.len()
        );
        insight.refresh_multi(&active_pairs).await;
        {
            let mut shared_insight = shared.insight.write().await;
            *shared_insight = insight.cached().clone();
        }
        info!("Initial market insight seeded to dashboard");

        // WALLET SYNC
        if let Some(ref mut ex) = executor {
            let all_sync_pairs: Vec<String> = curated_pairs.iter().cloned().collect();
            info!(
                "Wallet sync: checking on-chain balances for {} pairs ({} config + {} discovered)...",
                all_sync_pairs.len(),
                config.trading.pairs.len(),
                all_sync_pairs.len().saturating_sub(config.trading.pairs.len()),
            );
            let discrepancies = match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                ex.sync_wallet_positions(&all_sync_pairs),
            )
            .await
            {
                Ok(d) => d,
                Err(_) => {
                    error!("Wallet sync timed out after 30s — continuing without sync");
                    Vec::new()
                }
            };
            for (pair, on_chain_qty, tracked_qty) in &discrepancies {
                if *tracked_qty < 0.001 && *on_chain_qty > 0.001 {
                    let existing_pos = portfolio.positions().values()
                        .find(|p| p.pair == *pair)
                        .cloned();

                    if let Some(mut existing) = existing_pos {
                        if existing.side == Side::Short {
                            warn!(
                                "WALLET SYNC: {} journal says SHORT but on-chain holds {:.6} tokens — forcing LONG",
                                pair, on_chain_qty
                            );
                            existing.side = Side::Long;
                            existing.stop_loss = existing.entry_price * 0.92;
                            existing.take_profit_1 = existing.entry_price * 1.10;
                            existing.take_profit_2 = existing.entry_price * 1.20;
                            existing.take_profit_3 = existing.entry_price * 1.30;
                        }
                        let old_qty = existing.quantity;
                        existing.quantity = *on_chain_qty;
                        existing.current_price = market_stores
                            .get(pair)
                            .and_then(|s| s.last().map(|c| c.close))
                            .unwrap_or(existing.current_price);
                        existing.risk_amount = existing.entry_price * on_chain_qty;
                        portfolio.positions_mut().insert(existing.id.clone(), existing.clone());
                        if let Some(ref mut ex) = executor {
                            let exec_id = format!("exec-{}", existing.id);
                            ex.register_position(exec_id.clone(), existing.clone());
                            executor_position_map.insert(existing.id.clone(), exec_id.clone());
                        }
                        if let Some(ref j) = journal {
                            let _ = j.save_position(&existing).await;
                        }
                        info!(
                            "WALLET SYNC: Updated {} quantity {:.6} → {:.6} (kept journal entry @ ${:.4})",
                            pair, old_qty, on_chain_qty, existing.entry_price,
                        );
                        continue;
                    }

                    let market_price = market_stores
                        .get(pair)
                        .and_then(|s| s.last().map(|c| c.close))
                        .unwrap_or(0.0);
                    let entry_price = if market_price > 0.0 {
                        market_price
                    } else {
                        if let Some(ref j) = journal {
                            j.get_trades(1000)
                                .await
                                .unwrap_or_default()
                                .iter()
                                .rfind(|t| t.pair == *pair)
                                .map(|t| t.entry_price)
                                .unwrap_or(0.0)
                        } else {
                            0.0
                        }
                    };

                    if entry_price <= 0.0 {
                        error!("WALLET SYNC: Cannot recover {} — no valid entry price (market={:.4}). Skipping.",
                            pair, market_price);
                        shared
                            .log_activity(
                                savant_trading::core::shared::ActivityLevel::Error,
                                pair,
                                "WALLET SYNC FAILED: Cannot recover — no valid entry price",
                            )
                            .await;
                        continue;
                    }

                    let recovery_pos = savant_trading::core::types::Position {
                        id: format!("wallet-recovery-{}", pair.replace('/', "_").to_lowercase()),
                        pair: pair.clone(),
                        side: savant_trading::core::types::Side::Long,
                        entry_price,
                        current_price: market_price.max(entry_price),
                        quantity: *on_chain_qty,
                        stop_loss: entry_price * 0.85,
                        take_profit_1: entry_price * 1.10,
                        take_profit_2: entry_price * 1.20,
                        take_profit_3: entry_price * 1.30,
                        unrealized_pnl: 0.0,
                        risk_amount: entry_price * on_chain_qty,
                        strategy_name: "wallet_recovery".to_string(),
                        scale_level: savant_trading::core::types::ScaleLevel::Full,
                        opened_at: chrono::DateTime::from_timestamp(0, 0).unwrap_or_else(chrono::Utc::now),
                    };
                    portfolio
                        .positions_mut()
                        .insert(recovery_pos.id.clone(), recovery_pos.clone());
                    if let Some(ref mut ex) = executor {
                        let exec_id = format!("exec-{}", recovery_pos.id);
                        ex.register_position(exec_id.clone(), recovery_pos.clone());
                        executor_position_map.insert(recovery_pos.id.clone(), exec_id.clone());
                        info!(
                            "WALLET SYNC: Registered {} in DexTrader as {}",
                            pair, exec_id
                        );
                    }
                    if let Some(ref j) = journal {
                        let _ = j.save_position(&recovery_pos).await;
                    }
                    warn!(
                        "WALLET SYNC: Recovered {} — {:.6} tokens @ ${:.4} (source: market_price — no journal entry)",
                        pair, on_chain_qty, entry_price,
                    );
                    shared
                        .log_activity(
                            savant_trading::core::shared::ActivityLevel::Warning,
                            pair,
                            &format!(
                                "WALLET SYNC: Recovered {:.6} tokens @ ${:.4}",
                                on_chain_qty, entry_price
                            ),
                        )
                        .await;
                } else if *on_chain_qty < 0.001 && *tracked_qty > 0.001 {
                    if let Some(pos_id) = portfolio
                        .positions()
                        .iter()
                        .find(|(_, p)| p.pair == *pair)
                        .map(|(id, _)| id.clone())
                    {
                        portfolio.positions_mut().remove(&pos_id);
                        if let Some(ref j) = journal {
                            let _ = j.delete_position(&pos_id).await;
                        }
                        warn!(
                            "WALLET SYNC: {} position gone from chain — removed ghost",
                            pair
                        );
                        shared
                            .log_activity(
                                savant_trading::core::shared::ActivityLevel::Warning,
                                pair,
                                "WALLET SYNC: Position gone from chain — removed ghost",
                            )
                            .await;
                    }
                }
            }

            // FID-116 GAP FIX: If on-chain has MORE tokens than tracked, update position qty.
            // Handles: tokens acquired outside engine, airdrops, or under-counted positions.
            {
                for (pair, on_chain_qty, tracked_qty) in &discrepancies {
                    if *on_chain_qty > *tracked_qty + 0.001 && *tracked_qty > 0.001 {
                        if let Some(existing) = portfolio.positions_mut().values_mut()
                            .find(|p| p.pair == *pair)
                        {
                            let old_qty = existing.quantity;
                            existing.quantity = *on_chain_qty;
                            existing.risk_amount = existing.entry_price * on_chain_qty;
                            info!(
                                "FID-116 GAP: {} qty {:.4} -> {:.4} (on-chain has more)",
                                pair, old_qty, on_chain_qty
                            );
                        }
                    }
            // FID-116: Refresh equity after gap fix updates position quantities
            portfolio.refresh_equity();
                }
            }
            let short_ids: Vec<String> = portfolio.positions().iter()
                .filter(|(_, p)| p.side == Side::Short)
                .map(|(id, _)| id.clone())
                .collect();
            for id in &short_ids {
                if let Some(p) = portfolio.positions_mut().get_mut(id) {
                    warn!(
                        "SIDE CORRECTION: {} — spot-only mode, forcing SHORT → LONG",
                        p.pair
                    );
                    let pair = p.pair.clone();
                    p.side = Side::Long;
                    p.stop_loss = p.entry_price * 0.92;
                    p.take_profit_1 = p.entry_price * 1.10;
                    p.take_profit_2 = p.entry_price * 1.20;
                    p.take_profit_3 = p.entry_price * 1.30;
                    if let Some(ref j) = journal {
                        let _ = j.save_position(p).await;
                    }
                    let corrected = p.clone();
                    let corrected_id = id.clone();
                    if let Some(ref mut ex) = executor {
                        let exec_id = executor_position_map.get(&corrected_id)
                            .cloned()
                            .unwrap_or_else(|| format!("exec-{}", corrected_id));
                        ex.register_position(exec_id.clone(), corrected);
                        debug!("FID-094: Synced side correction to DexTrader: {} → {}", corrected_id, exec_id);
                    }
                    shared.log_activity(
                        savant_trading::core::shared::ActivityLevel::Warning,
                        &pair,
                        "SIDE CORRECTION: spot-only mode, forced SHORT → LONG",
                    ).await;
                }
            }
            if discrepancies.is_empty() {
                info!("Wallet sync: all positions reconciled with on-chain balances");
            }
            portfolio.refresh_equity();
            portfolio.account_mut().open_positions = portfolio.positions().len();
            if portfolio.account().equity < 0.01 && !portfolio.positions().is_empty() {
                let token_value: f64 = portfolio.positions().values()
                    .map(|p| p.current_price * p.quantity)
                    .sum();
                if token_value > 0.0 {
                    portfolio.account_mut().equity = token_value;
                    info!("FID-109: Set equity to token value ${:.2}", token_value);
                }
            }
            {
                let mut shared_account = shared.account.write().await;
                *shared_account = portfolio.account().clone();
                let mut shared_positions = shared.positions.write().await;
                *shared_positions = portfolio.positions().values().cloned().collect();
            }
            info!(
                "Wallet sync complete: {} positions, balance=${:.2}, equity=${:.2}, open={}",
                portfolio.positions().len(),
                portfolio.account().balance,
                portfolio.account().equity,
                portfolio.account().open_positions,
            );
            let shared_pos_count = shared.positions.read().await.len();
            let shared_open = shared.account.read().await.open_positions;
            if shared_pos_count != portfolio.positions().len() || shared_open != portfolio.positions().len() {
                warn!(
                    "FID-109: Shared state mismatch — portfolio={} positions, shared_positions={}, shared_open={}. Forcing sync.",
                    portfolio.positions().len(), shared_pos_count, shared_open
                );
                let mut sp = shared.positions.write().await;
                *sp = portfolio.positions().values().cloned().collect();
                let mut sa = shared.account.write().await;
                *sa = portfolio.account().clone();
            }
            {
                let mut curve = shared.equity_curve.write().await;
                curve.push(serde_json::json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "equity": portfolio.account().equity,
                    "balance": portfolio.account().balance,
                }));
            }
        }

        if let Some(ref ex) = executor {
            let exec_positions = ex.open_positions();
            if portfolio.positions().is_empty() && !exec_positions.is_empty() {
                for pos in exec_positions {
                    info!(
                        "FID-109: Syncing executor position to portfolio: {} {} qty={:.6}",
                        pos.pair, pos.side, pos.quantity
                    );
                    portfolio.positions_mut().insert(pos.id.clone(), pos.clone());
                }
                portfolio.account_mut().open_positions = portfolio.positions().len();
                portfolio.refresh_equity();
                if portfolio.account().equity < 0.01 {
                    let token_value: f64 = portfolio.positions().values()
                        .map(|p| p.current_price * p.quantity)
                        .sum();
                    if token_value > 0.0 {
                        portfolio.account_mut().equity = token_value;
                        info!("FID-109: Set equity to token value ${:.2}", token_value);
                    }
                }
            }
        }

        // FID-117: Clean up orphaned JSON snapshot files from before FID-117.
        // These files are no longer read or written — journal is the source of truth.
        let _ = std::fs::remove_file("data/starting_equity.json");
        let _ = std::fs::remove_file("data/starting_balance.json");

        // FID-117: Record starting equity to journal (replaces JSON snapshot files).
        // The journal is the single historical source of truth. Chain is current truth.

        // FID-117: Load peak_equity from journal snapshots (survives restarts).
        // Without this, peak_equity resets to current equity on every boot,
        // corrupting drawdown tracking.
        if let Some(ref j) = journal {
            match j.get_peak_equity().await {
                Ok(peak) if peak > 0.0 => {
                    portfolio.account_mut().peak_equity = peak;
                    info!("FID-117: Restored peak_equity = ${:.2} from journal", peak);
                }
                Ok(_) => {
                    info!("FID-117: No equity snapshots yet, peak_equity starts at current");
                }
                Err(e) => warn!("FID-117: Failed to load peak_equity: {}", e),
            }
        }
        {
            let usdc_balance = portfolio.account().balance;
            let mut token_values = 0.0f64;
            for pos in portfolio.positions().values() {
                token_values += pos.current_price * pos.quantity;
            }
            let starting_equity = usdc_balance + token_values;

            if let Some(ref j) = journal {
                match j.ensure_starting_equity(starting_equity).await {
                    Ok(true) => info!(
                        "FID-117: Recorded starting_equity = ${:.2} (USDC: ${:.2}, tokens: ${:.2})",
                        starting_equity, usdc_balance, token_values
                    ),
                    Ok(false) => {
                        if let Ok(Some(saved)) = j.get_starting_equity().await {
                            info!("FID-117: Loaded starting_equity = ${:.2} from journal", saved);
                        }
                    }
                    Err(e) => warn!("FID-117: Failed to record starting_equity: {}", e),
                }
            }

            // Initialize shared state from journal (authoritative) or calculation (fallback)
            let se = if let Some(ref j) = journal {
                j.get_starting_equity().await.ok().flatten().unwrap_or(starting_equity)
            } else {
                starting_equity
            };
            {
                let mut shared_se = shared.starting_equity.write().await;
                *shared_se = se;
            }
            *shared.chain_equity.write().await = portfolio.account().equity;
        }

        // FID-117: Record starting equity to journal on first startup.
        // Uses on-chain equity (USDC + all token values) not stale portfolio equity.

        {
            let stop_overrides: Vec<(String, f64)> = portfolio
                .positions()
                .values()
                .filter(|p| p.strategy_name == "wallet_recovery")
                .filter_map(|p| {
                    let default_sl = match p.side {
                        Side::Long => p.entry_price * 0.85,
                        Side::Short => p.entry_price * 1.15,
                    };
                    if (p.stop_loss - default_sl).abs() < 0.01 {
                        let new_sl = match (p.pair.as_str(), p.side) {
                            ("LINK/USD", Side::Long) => Some(7.00),
                            ("LINK/USD", Side::Short) => Some(
                                (p.entry_price * 1.08 * 100.0).round() / 100.0,
                            ),
                            ("WETH/USD", Side::Long) => Some(
                                (p.entry_price * 0.92 * 100.0).round() / 100.0,
                            ),
                            ("WETH/USD", Side::Short) => Some(
                                (p.entry_price * 1.08 * 100.0).round() / 100.0,
                            ),
                            _ => None,
                        };
                        new_sl.map(|sl| (p.pair.clone(), sl))
                    } else {
                        None
                    }
                })
                .collect();

            if !stop_overrides.is_empty() {
                let mut overrides = shared.stop_overrides.write().await;
                for (pair, new_stop) in &stop_overrides {
                    overrides.insert(pair.clone(), *new_stop);
                    info!("Auto-stop queued: {} → ${:.4}", pair, new_stop);
                }
            }
        }

        // WebSocket setup
        let (ws_tx, ws_rx) = savant_trading::data::websocket::create_channel();
        let ws_pairs: Vec<String> = config.trading.pairs.clone();
        let ws_url = config.exchange.ws_url.clone();
        tokio::spawn(async move {
            savant_trading::data::websocket::connect(&ws_url, ws_pairs, ws_tx).await;
        });

        let ws_ticker_prices: HashMap<String, (f64, std::time::Instant)> = HashMap::new();
        let ws_staleness: HashMap<String, u64> = HashMap::new();
        let rest_fallback_at: Option<std::time::Instant> = None;
        let ws_just_reconnected = false;

        let dead_tokens: HashSet<String> = HashSet::new();
        let permanent_dead: HashSet<String> = HashSet::new();
        let candle_hash_cache: HashMap<String, u64> = HashMap::new();
        let tick = 0u64;
        let eval_in_progress = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let last_daily_reset: chrono::NaiveDate = chrono::Utc::now().date_naive();
        let close_failure_cooldown: HashMap<String, std::time::Instant> = HashMap::new();
        let consecutive_sl_count: HashMap<String, u32> = HashMap::new();
        let sl_halt_until: HashMap<String, std::time::Instant> = HashMap::new();

        Ok(Self {
            config,
            shared,
            running,
            candle_api,
            active_pairs,
            curated_pairs,
            market_stores,
            candle_router,
            interval_seconds,
            portfolio,
            executor,
            executor_position_map,
            reconciliation_removed,
            failure_tracker,
            episode_store,
            journal,
            agent,
            ctx_engine,
            ctx_state,
            decision_log,
            insight,
            event_bus,
            vault_config,
            vault_writer,
            vault_watcher,
            order_books,
            memory,
            cusum_charts,
            brier_predictions,
            operator_rules,
            regime_detector,
            position_sizer,
            circuit_breaker,
            goplus_client,
            ws_rx,
            ws_ticker_prices,
            ws_staleness,
            rest_fallback_at,
            ws_just_reconnected,
            dead_tokens,
            permanent_dead,
            candle_hash_cache,
            tick,
            eval_in_progress,
            last_daily_reset,
            close_failure_cooldown,
            consecutive_sl_count,
            sl_halt_until,
        })
    }
}

pub async fn run(
    config: AppConfig,
    shared: savant_trading::core::shared::SharedEngineData,
    _engine_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    let state = EngineState::new(config, shared, _engine_running).await?;

    // Destructure state into local variables (loop body unchanged)
    let config = state.config;
    let shared = state.shared;
    let _candle_api = state.candle_api;
    let active_pairs = state.active_pairs;
    let curated_pairs = state.curated_pairs;
    let mut market_stores = state.market_stores;
    let mut portfolio = state.portfolio;
    let mut executor = state.executor;
    let mut reconciliation_removed = state.reconciliation_removed;
    let mut executor_position_map = state.executor_position_map;
    let mut failure_tracker = state.failure_tracker;
    let mut episode_store = state.episode_store;
    let journal = state.journal;
    let mut agent = state.agent;
    let mut ctx_engine = state.ctx_engine;
    let mut ctx_state = state.ctx_state;
    let mut decision_log = state.decision_log;
    let mut insight = state.insight;
    let event_bus = state.event_bus;
    let vault_config = state.vault_config;
    let vault_writer = state.vault_writer;
    let _vault_watcher = state.vault_watcher;
    let mut order_books = state.order_books;
    let memory = state.memory;
    let mut cusum_charts = state.cusum_charts;
    let mut brier_predictions = state.brier_predictions;
    let _operator_rules = state.operator_rules;
    let regime_detector = state.regime_detector;
    let position_sizer = state.position_sizer;
    let circuit_breaker = state.circuit_breaker;
    let goplus_client = state.goplus_client;
    let interval_seconds = state.interval_seconds;
    let candle_router = state.candle_router;
    let mut ws_rx = state.ws_rx;
    let mut ws_ticker_prices = state.ws_ticker_prices;
    let mut ws_staleness = state.ws_staleness;
    let mut rest_fallback_at = state.rest_fallback_at;
    let mut ws_just_reconnected = state.ws_just_reconnected;
    let mut dead_tokens = state.dead_tokens;
    let permanent_dead = state.permanent_dead;
    let mut candle_hash_cache = state.candle_hash_cache;
    let mut tick = state.tick;
    let eval_in_progress = state.eval_in_progress;
    let mut last_daily_reset = state.last_daily_reset;
    let mut close_failure_cooldown = state.close_failure_cooldown;
    let mut consecutive_sl_count = state.consecutive_sl_count;
    let mut sl_halt_until = state.sl_halt_until;

    let autonomy = match config.ai.autonomy_level {
        1 => AutonomyLevel::Suggest,
        2 => AutonomyLevel::Confirm,
        _ => AutonomyLevel::Autonomous,
    };

    info!(
        "Starting main loop (interval: {}s, autonomy: {:?})...",
        interval_seconds, autonomy
    );

    const CLOSE_COOLDOWN_SECS: u64 = 1800;
    const SL_HALT_THRESHOLD: u32 = 3;
    const _SL_HALT_SECS: u64 = 3600;

    loop {
        tick += 1;
        let cycle_start = std::time::Instant::now();

        // FID-093 D1: Explicit midnight UTC reset for daily PnL.
        // update_prices() already does this, but only runs when prices arrive.
        // This ensures reset happens even if no WS/candle data is available.
        {
            let today = chrono::Utc::now().date_naive();
            if today != last_daily_reset {
                let acct = portfolio.account_mut();
                info!("Midnight UTC reset: clearing daily PnL (${:.2}) and trade count ({})", acct.daily_pnl, acct.trades_today);
                acct.daily_pnl = 0.0;
                acct.trades_today = 0;
                last_daily_reset = today;
            }
        }

        // SPRINT-2: Drain WebSocket messages (non-blocking)
        let mut ws_messages_drained = 0u32;
        while let Ok(msg) = ws_rx.try_recv() {
            ws_messages_drained += 1;
            match msg {
                savant_trading::data::websocket::WsMessage::Ticker(ticker) => {
                    ws_ticker_prices.insert(ticker.pair.clone(), (ticker.last, std::time::Instant::now()));
                    // FID-086: Feed live WS price into candle store so the LLM
                    // model sees real-time prices instead of startup-frozen data.
                    if let Some(store) = market_stores.get_mut(&ticker.pair) {
                        store.update_last_close(ticker.last);
                    }
                }
                savant_trading::data::websocket::WsMessage::BookUpdate(book) => {
                    if let Some(ob) = order_books.get_mut(&book.pair) {
                        ob.update(book);
                    }
                }
                savant_trading::data::websocket::WsMessage::Trade { pair, price, .. } => {
                    ws_ticker_prices.insert(pair.clone(), (price, std::time::Instant::now()));
                    // FID-086: Feed live WS price into candle store so the LLM
                    // model sees real-time prices instead of startup-frozen data.
                    if let Some(store) = market_stores.get_mut(&pair) {
                        store.update_last_close(price);
                    }
                }
                savant_trading::data::websocket::WsMessage::CancelAllOrders { reason } => {
                    warn!("WS CANCEL-ALL TRIGGERED: {}", reason);
                    // Log emergency close warnings BEFORE clearing so output is not lost
                    let emergency_pairs: Vec<(Side, f64, String, f64)> = portfolio
                        .positions()
                        .values()
                        .map(|pos| (pos.side, pos.quantity, pos.pair.clone(), pos.current_price))
                        .collect();
                    for (side, qty, pair, price) in &emergency_pairs {
                        warn!("Emergency close: {} {} {} @ {:.2}", side, qty, pair, price);
                    }
                    // In live mode, cancel all orders and clear position tracking
                    if let Some(ref mut ex) = executor {
                        if let Err(e) = ex.kill().await {
                            error!("Executor kill failed: {}", e);
                        }
                        // Clear position mapping since executor cancelled everything
                        executor_position_map.clear();
                    }
                    // Clear PortfolioManager positions to match executor state (AFTER logging)
                    let cleared_count = portfolio.positions().len();
                    portfolio.positions_mut().clear();
                    portfolio.account_mut().open_positions = 0;
                    info!(
                        "Cleared {} local positions to match executor cancel-all",
                        cleared_count
                    );
                    shared
                        .log_activity(
                            savant_trading::core::shared::ActivityLevel::Warning,
                            "SYSTEM",
                            &format!("CANCEL-ALL: {}", reason),
                        )
                        .await;
                }
                savant_trading::data::websocket::WsMessage::StateChange(state) => {
                    if state == savant_trading::data::websocket::WsState::Connected {
                        ws_just_reconnected = true;
                        info!("FID-081: WS reconnected — will verify price freshness on next cycle");
                    }
                }
                _ => {}
            }
        }
        if ws_messages_drained > 0 {
            debug!("Drained {} WS messages", ws_messages_drained);
        }

        // FID-085/086: Refresh candle data every cycle.
        // Without this, candles are loaded once at startup and indicators go stale.
        // WS ticker only updates the LAST candle's close — all other candles freeze.
        // FID-093 C4: Skip fetch if latest candle is less than 1 minute old
        // (same candle period — no new candle has formed yet).
        {
            let tf = config.trading.timeframe.clone();
            let tf_minutes = parse_timeframe_minutes(&tf);
            let mut candle_refresh_futures = tokio::task::JoinSet::new();
            let now = chrono::Utc::now();
            for pair in &active_pairs {
                // FID-093 C4: Skip fetch if latest candle < 1 min old
                let skip_fetch = market_stores
                    .get(pair.as_str())
                    .and_then(|s| s.last().map(|c| {
                        let age = now.signed_duration_since(c.timestamp);
                        age.num_seconds() < 60
                    }))
                    .unwrap_or(false);
                if skip_fetch {
                    continue;
                }
                let router = candle_router.clone();
                let pair_clone = pair.clone();
                candle_refresh_futures.spawn(async move {
                    let result = router.fetch_candles(&pair_clone, tf_minutes, 200).await;
                    (pair_clone, result)
                });
            }
            while let Some(result) = candle_refresh_futures.join_next().await {
                if let Ok((pair, Ok(mut candles))) = result {
                    if candles.len() > 1 {
                        candles.pop(); // Remove incomplete last candle
                    }
                    if let Some(store) = market_stores.get_mut(&pair) {
                        store.add_candles(candles);
                    }
                }
            }
            // Re-apply any live WS prices on top of fresh candles
            for (pair, (price, _)) in &ws_ticker_prices {
                if let Some(store) = market_stores.get_mut(pair) {
                    store.update_last_close(*price);
                }
            }
        }

        // Refresh insight every 5 ticks (all pairs, single funding API call)
        if tick.is_multiple_of(5) {
            insight.refresh_multi(&active_pairs).await;
            // Sync insight to shared state on every refresh so dashboard stays current
            {
                let mut shared_insight = shared.insight.write().await;
                *shared_insight = insight.cached().clone();
            }

            // Project insight to vault
            if vault_config.enabled {
                let ctx = insight.cached();
                let session_str = savant_trading::core::session::session_context();
                let rss_count = ctx.rss_items.len();
                let _ = vault_writer.project_insight(
                    ctx.sentiment.fear_greed_index.map(|v| v as i32),
                    ctx.sentiment.fear_greed_label.as_deref(),
                    ctx.funding.funding_rate,
                    ctx.onchain.mvrv,
                    ctx.onchain.sopr,
                    (&session_str, rss_count),
                );
            }
        }

        // === PHASE 1: Parallel fetch + sequential processing ===
        struct PairData {
            pair: String,
            indicators: savant_trading::core::types::IndicatorValues,
            regime: savant_trading::core::types::MarketRegime,
            current_price: f64,
            system_prompt: String,
            user_message: String,
        }

        // === PHASE 1b: Sequential processing — indicators, context, LLM prep ===
        // FID-046: Show balance at cycle start
        if let Some(ref ex) = executor {
            log_phase!(
                "CYCLE",
                "Balance: ${:.2} USDC | Chain: {} (id={}) | Cycle #{}",
                ex.balance(),
                "Arbitrum",
                42161,
                tick
            );
        }
        let mut pair_data_vec: Vec<PairData> = Vec::new();
        let market_ctx = insight.cached().clone();
        let positions: Vec<Position> = portfolio.positions().values().cloned().collect();
        let recent_trades = portfolio.closed_trades().to_vec();
        let current_session = savant_trading::core::session::current_session();

        // FID-046: Retry dead tokens every 10 cycles
        if tick.is_multiple_of(10) {
            dead_tokens.clear();
        }

        // FID-056 #1: When fully deployed (no deployable capital), skip
        // scanning for new entries but ALWAYS evaluate held positions.
        // The model IS the edge — if it's not actively evaluating every
        // cycle, the trade is running on autopilot with no intelligence.
        let available_balance = if let Some(ref ex) = executor {
            ex.balance()
        } else {
            portfolio.account().balance
        };
        let min_order_value = 1.0_f64;
        let fully_deployed = available_balance < min_order_value;

        // When fully deployed, log monitoring status but ALWAYS evaluate below.
        // The model IS the edge — it must evaluate held positions every cycle.
        if fully_deployed && !portfolio.positions().is_empty() {
            log_phase!(
                "PHASE2",
                "MONITORING — fully deployed (${:.2} < ${:.2} min). Scanning all {} pairs for opportunities.",
                available_balance,
                min_order_value,
                active_pairs.len()
            );
        }

        // FID-063: Hunt mode — under $500 with idle capital > $5.
        // Bypass candle hash cache and pre-scoring filter. The LLM decides whether
        // to enter based on knowledge units, on-chain data, and sentiment — not
        // just RSI/ADX/EMA. Per soul.md: "Capital velocity > Capital preservation.
        // Below $500, we treat the account as a call option on our own skill."
        let total_equity = portfolio.account().equity;
        let hunt_mode = !fully_deployed && total_equity < 500.0 && available_balance > 5.0;
        if hunt_mode {
            shared
                .log_activity(
                    savant_trading::core::shared::ActivityLevel::Thinking,
                    "SYSTEM",
                    &format!(
                        "HUNT MODE ACTIVE: ${:.2} idle of ${:.2} equity — scanning all pairs for entries",
                        available_balance, total_equity
                    ),
                )
                .await;
            debug!(
                "HUNT MODE: ${:.2} idle of ${:.2} equity — evaluating all pairs for entries",
                available_balance, total_equity
            );
        }
        // Sync hunt_mode to shared state for API/dashboard visibility (FID-063)
        {
            let mut hm = shared.hunt_mode.write().await;
            *hm = hunt_mode;
        }
        // FID-075: Sync monitoring_mode — active when fully deployed (no capital to scan)
        {
            let mut mm = shared.monitoring_mode.write().await;
            *mm = fully_deployed;
        }

        for pair in &active_pairs {
            // FID-092: ALWAYS evaluate all pairs, even when fully deployed.
            // The agent must see all charts for opportunity cost awareness.
            // When fully deployed, new entries are blocked at execution time
            // (not at evaluation time). The LLM evaluates everything and can
            // recommend CLOSE on held positions to rotate into better setups.
            if dead_tokens.contains(pair.as_str()) || permanent_dead.contains(pair.as_str()) {
                continue;
            }
            if let Some(store) = market_stores.get(pair.as_str()) {
                let candle_data: Vec<Candle> = store.candles().iter().cloned().collect();
                if candle_data.len() < 20 {
                    continue;
                }

                // Pre-filter: Skip stablecoins (pegged to $1, no tradeable edge)
                let base_symbol = pair.split('/').next().unwrap_or(pair);
                const STABLECOINS: &[&str] = &[
                    "USDC", "USDC.E", "USDT", "DAI", "USDS", "USDE", "FDUSD", "PYUSD", "GHO",
                    "CRVUSD", "TUSD", "LUSD", "FRAX", "USDD", "USD0", "SUSDS", "SUSDE", "AUSD",
                ];
                if STABLECOINS.contains(&base_symbol) {
                    continue;
                }

                // Pre-filter: Skip xStock tokens (require 0x opt-in, 403 on swap)
                const XSTOCKS: &[&str] = &["SPYX", "QQQX", "GLDX", "CRCLX"];
                if XSTOCKS.contains(&base_symbol) {
                    continue;
                }

                // Pre-filter: Skip tokens not in curated pairs (FID-052)
                if config.mode.live_execution && !curated_pairs.contains(pair) {
                    dead_tokens.insert(pair.to_string());
                    continue;
                }

                // Pre-filter: Skip pairs with mostly-zero candles (corrupted data)
                // FID-044: Lowered from 50% to 30% — SourceRouter now rejects all-zero
                // candles, so surviving data from Binance/CoinGecko should be mostly valid.
                let nonzero_count = candle_data
                    .iter()
                    .filter(|c| c.close > 0.0 && c.volume > 0.0)
                    .count();
                let nonzero_pct = nonzero_count as f64 / candle_data.len() as f64;
                if nonzero_pct < 0.3 {
                    if nonzero_count == 0 {
                        dead_tokens.insert(pair.to_string());
                    }
                    continue;
                }

                // Pre-filter: Skip pairs with negligible volume.
                // FID-072: Active in all modes — even DEX tokens need SOME volume signal.
                // Threshold is low ($10) to avoid rejecting legitimate low-cap tokens
                // while still filtering completely dead pairs.
                let avg_volume: f64 =
                    candle_data.iter().map(|c| c.volume).sum::<f64>() / candle_data.len() as f64;
                if avg_volume < 10.0 {
                    continue;
                }
                // DEX safety: reject tokens with near-zero price diversity
                let all_dead = candle_data
                    .iter()
                    .all(|c| c.open == c.close && c.high == c.low && c.volume <= 0.0);
                if all_dead {
                    dead_tokens.insert(pair.to_string());
                    continue;
                }
                // DEX safety: reject tokens with near-zero price diversity
                // (prevents ultra-low-liquidity tokens from being traded)
                let unique_closes: Vec<f64> = {
                    let mut closes: Vec<f64> = candle_data.iter().map(|c| c.close).collect();
                    closes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let mut deduped = Vec::new();
                    for c in closes {
                        if deduped.is_empty() || (c - deduped.last().unwrap()).abs() > 0.000001 {
                            deduped.push(c);
                        }
                    }
                    deduped
                };
                if unique_closes.len() < 2 {
                    // 200 candles with < 2 unique close prices = truly dead
                    dead_tokens.insert(pair.to_string());
                    continue;
                }

                // FID-056 #2+#6: Skip LLM eval if candle data unchanged since last cycle.
                // EXCEPTION: Always re-evaluate pairs with open positions — the LLM needs
                // to see current price + position state for stop adjustments, even if
                // candle data hasn't changed yet.
                // EXCEPTION (FID-063): In hunt mode, always evaluate — the LLM should scan
                // for entries every cycle regardless of candle staleness.
                let has_position = positions.iter().any(|p| p.pair == *pair);
                if !has_position && !hunt_mode {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::hash::DefaultHasher::new();
                    let tail = candle_data.iter().rev().take(5);
                    for c in tail {
                        c.close.to_bits().hash(&mut hasher);
                        c.volume.to_bits().hash(&mut hasher);
                    }
                    let hash = hasher.finish();
                    if let Some(&prev) = candle_hash_cache.get(pair.as_str()) {
                        if prev == hash {
                            continue;
                        }
                    }
                    candle_hash_cache.insert(pair.to_string(), hash);
                }

                // GoPlus security check (FID-035): reject honeypots/taxed tokens
                // before LLM evaluation. Meme coins can have hidden taxes or be
                // un-sellable. This check prevents wasting LLM cycles on bad tokens.
                if let Some(ref goplus) = goplus_client {
                    match goplus.check_by_symbol(base_symbol).await {
                        Ok(false) => {
                            log_warn!("SECURITY", "Rejected {} — GoPlus flagged as unsafe", pair);
                            continue;
                        }
                        Err(e) => {
                            log_warn!(
                                "SECURITY",
                                "GoPlus check failed for {} ({}), proceeding",
                                pair,
                                e
                            );
                        }
                        _ => {} // Safe or unknown — proceed
                    }
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

                // FID-056 #5: Smart pre-scoring — skip pairs with no plausible setup.
                // Only send to LLM if at least one signal fires: RSI extreme, strong trend, or EMA cross.
                // EXCEPTION: Always evaluate pairs with open positions — the LLM may need to
                // adjust stops based on price action even without technical signals.
                // EXCEPTION (FID-063): In hunt mode, always evaluate — the LLM uses knowledge
                // units + on-chain data, not just technical indicators.
                // EXCEPTION: scan_all_pairs mode — $0 LLM cost, let the agent evaluate everything.
                if !has_position && !hunt_mode && !config.trading.scan_all_pairs {
                    let rsi = indicators.rsi.unwrap_or(50.0);
                    let adx = indicators.adx.unwrap_or(0.0);
                    let ema_fast = indicators.ema_fast.unwrap_or(0.0);
                    let ema_slow = indicators.ema_slow.unwrap_or(0.0);

                    // P1-3c: Dynamic ADX threshold — lower in bear markets
                    let fg = insight.cached().sentiment.fear_greed_index.unwrap_or(50) as f64;
                    let adx_threshold = (25.0 - ((50.0 - fg).max(0.0) / 30.0 * 7.0)).clamp(18.0, 25.0);
                    let rsi_signal = !(30.0..=70.0).contains(&rsi);
                    let trend_signal = adx > adx_threshold;
                    let ema_cross = (ema_fast > 0.0 && ema_slow > 0.0)
                        && ((ema_fast > ema_slow
                            && candle_data.last().map(|c| c.close).unwrap_or(0.0) > ema_fast)
                            || (ema_fast < ema_slow
                                && candle_data.last().map(|c| c.close).unwrap_or(0.0) < ema_fast));
                    // P1-2c: Volume spike as 4th pre-scoring signal
                    let volume_spike = indicators.volume_sma.is_some_and(|sma| {
                        sma > 0.0 && candle_data.last().is_some_and(|c| c.volume > sma * 1.5)
                    });
                    // P1-3b: Bollinger Band Squeeze — BB inside Keltner Channels
                    let bb_squeeze = if candle_data.len() >= 20 {
                        let last20: Vec<f64> = candle_data.iter().rev().take(20).map(|c| c.close).collect();
                        let sma20 = last20.iter().sum::<f64>() / last20.len() as f64;
                        let variance = last20.iter().map(|c| (c - sma20).powi(2)).sum::<f64>() / last20.len() as f64;
                        let stddev = variance.sqrt();
                        let bb_upper = sma20 + 2.0 * stddev;
                        let bb_lower = sma20 - 2.0 * stddev;
                        // Keltner: EMA(20) ± 1.5 * ATR(14)
                        let ema20 = sma20; // Simplified: use SMA as EMA approximation for squeeze detection
                        let atr14: f64 = if candle_data.len() >= 14 {
                            let recent: Vec<&Candle> = candle_data.iter().rev().take(14).collect();
                            let tr_sum: f64 = recent.windows(2).map(|w| {
                                let high = w[0].high;
                                let low = w[0].low;
                                let prev_close = w[1].close;
                                let tr1 = high - low;
                                let tr2 = (high - prev_close).abs();
                                let tr3 = (low - prev_close).abs();
                                tr1.max(tr2).max(tr3)
                            }).sum();
                            tr_sum / 14.0
                        } else { 0.0 };
                        let keltner_upper = ema20 + 1.5 * atr14;
                        let keltner_lower = ema20 - 1.5 * atr14;
                        bb_upper < keltner_upper && bb_lower > keltner_lower
                    } else { false };

                    if !rsi_signal && !trend_signal && !ema_cross && !volume_spike && !bb_squeeze {
                        continue;
                    }
                }

                // Query memory context with timeout to prevent SQLite deadlocks
                let memory_ctx_str = if let Some(ref mem) = memory {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        savant_trading::memory::context::query_memory_context(
                            mem,
                            pair,
                            &format!("{}", regime),
                            current_session.name(),
                        ),
                    )
                    .await
                    {
                        Ok(mem_ctx) => {
                            let formatted =
                                savant_trading::memory::context::format_memory_prompt(&mem_ctx);
                            if formatted.is_empty() {
                                None
                            } else {
                                Some(formatted)
                            }
                        }
                        Err(_) => {
                            warn!("Memory query timed out for {}", pair);
                            None
                        }
                    }
                } else {
                    None
                };

                // Dual timeframe (FID-035): Aggregate 5m candles into 15m
                // for higher-timeframe trend context. No extra API calls needed.
                let higher_tf_candles = {
                    let mut htf = Vec::new();
                    if candle_data.len() >= 6 {
                        let mut tf_15m = Vec::new();
                        for chunk in candle_data.chunks(3) {
                            if chunk.len() == 3 {
                                let open = chunk[0].open;
                                let high = chunk
                                    .iter()
                                    .map(|c| c.high)
                                    .fold(f64::NEG_INFINITY, f64::max);
                                let low = chunk.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
                                let close = chunk[2].close;
                                let volume = chunk.iter().map(|c| c.volume).sum::<f64>();
                                let timestamp = chunk[2].timestamp;
                                tf_15m.push(Candle {
                                    pair: pair.clone(),
                                    open,
                                    high,
                                    low,
                                    close,
                                    volume,
                                    timestamp,
                                });
                            }
                        }
                        if !tf_15m.is_empty() {
                            htf.push(("15m".to_string(), tf_15m));
                        }
                    }
                    htf
                };

                // FID-098 Fix 2: Build decision log context for this pair
                let decision_log_ctx = decision_log.context_for_pair(pair, 3, 2);

                let ctx = FullContext {
                    candles: &candle_data,
                    indicators: &indicators,
                    regime,
                    volume_profile: profile.as_ref(),
                    market_context: &market_ctx,
                    positions: &positions,
                    account: portfolio.account(),
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
                    // FID-086: Pass live WS price to prompt builder
                    live_price: ws_ticker_prices.get(pair).map(|(p, _)| *p),
                    // FID-098: Pass decision log context with recent outcomes
                    decision_log_context: if decision_log_ctx.is_empty() {
                        None
                    } else {
                        Some(decision_log_ctx)
                    },
                    dex_price: None,
                };

                // Step 1: Select knowledge units and clone to release borrow on agent
                let knowledge_units: Vec<savant_trading::agent::knowledge::KnowledgeUnit> = {
                    let conditions = savant_trading::agent::context_builder::determine_conditions(&ctx);
                    let selected = agent.knowledge_base().select_with_tags(
                        &conditions,
                        &ctx.context_tags,
                        config.ai.knowledge_token_budget,
                    );
                    selected.into_iter().cloned().collect()
                };
                let knowledge_refs: Vec<&savant_trading::agent::knowledge::KnowledgeUnit> =
                    knowledge_units.iter().collect();
                
                // Step 2: Compose prompt and build user message (mutable borrow of agent)
                let system_prompt = agent.composer_mut().compose(&knowledge_refs);
                let user_message = ctx_engine.build_user_message_for(&ctx);

                // FID-085: Delta-compression for observability ONLY.
                // Always send the full prompt to the LLM — never strip context.
                // The delta % is logged at debug level to avoid noise.
                let delta_result = ctx_state.compute_delta(&user_message, config.context.delta_compression_threshold);
                if tracing::enabled!(tracing::Level::DEBUG) {
                    match delta_result {
                        savant_trading::agent::context_state::DeltaResult::NoChange => {
                            tracing::debug!("Delta: {} — identical to last cycle", pair);
                        }
                        savant_trading::agent::context_state::DeltaResult::Delta(_) => {
                            tracing::debug!("Delta: {} — small change, full prompt sent", pair);
                        }
                        savant_trading::agent::context_state::DeltaResult::Full(_) => {
                            tracing::debug!("Delta: {} — full data injection", pair);
                        }
                    }
                }

                // FID-085: Anti-thrashing check (observability only, debug level)
                if ctx_state.should_skip_compression(config.context.anti_thrash_min_savings) {
                    tracing::debug!("Anti-thrashing: {} has low compression efficiency", pair);
                }

                ctx_state.increment_cycle();

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
        // FID-073 Issue 2: Skip if previous eval still running
        if eval_in_progress.load(std::sync::atomic::Ordering::Relaxed) {
            log_phase!("PHASE2", "SKIPPED — previous LLM eval still in progress");
            pair_data_vec.clear();
        }
        log_phase!(
            "PHASE2",
            "{} pairs queued for LLM evaluation",
            pair_data_vec.len()
        );
        struct PairResult {
            pair: String,
            response: Result<String, savant_trading::agent::provider::LlmError>,
            current_price: f64,
            _atr: Option<f64>,
        }

        // Save pair data for episodic capture (before consuming)
        let _pair_data_for_memory: Vec<(
            String,
            savant_trading::core::types::IndicatorValues,
            savant_trading::core::types::MarketRegime,
        )> = pair_data_vec
            .iter()
            .map(|pd| (pd.pair.clone(), pd.indicators.clone(), pd.regime))
            .collect();

        // === BATCH MODE: Combine all pairs into a single LLM call ===
        // Instead of N parallel calls ($0.01-0.02 each), make 1 call with all pairs.
        // This reduces API cost by 80-90%.
        let mut all_results: Vec<PairResult> = Vec::new();

        if pair_data_vec.is_empty() {
            log_phase!("PHASE2", "No pairs to evaluate");
        } else if pair_data_vec.len() == 1 {
            // Single pair — no batching needed, use direct call
            let pd = pair_data_vec.into_iter().next().unwrap();
            let provider = agent.provider_clone();
            let messages = vec![savant_trading::agent::provider::Message {
                role: "user".to_string(),
                content: pd.user_message.clone(),
            }];
            let start = std::time::Instant::now();
            log_llm!("LLM", "EVALUATING {} (single)", pd.pair);
            let response = provider.chat_stream(&pd.system_prompt, &messages).await;
            let elapsed = start.elapsed().as_millis();
            match &response {
                Ok(text) => log_llm_done!(
                    "LLM",
                    "COMPLETE {} {} chars in {}ms",
                    pd.pair,
                    text.len(),
                    elapsed
                ),
                Err(e) => log_swap_fail!("LLM", "ERROR {} {}", pd.pair, e),
            }
            all_results.push(PairResult {
                pair: pd.pair,
                response,
                current_price: pd.current_price,
                _atr: pd.indicators.atr,
            });
        } else {
            // BATCH MODE: Multiple pairs — combine into single call
            let batch_size = pair_data_vec.len();
            let first = &pair_data_vec[0];

            // Use first pair's system prompt (knowledge is similar across pairs)
            let system_prompt = &first.system_prompt;

            // Build combined user message
            let mut batch_msg = String::new();
            batch_msg.push_str(&format!("## BATCH EVALUATION — {} Pairs\n\n", batch_size));
            batch_msg.push_str("Evaluate ALL pairs below independently. For each pair, provide a decision in the specified JSON format.\n");
            batch_msg.push_str("Return a JSON ARRAY containing one decision object per pair. Each object MUST include the \"pair\" field.\n\n");

            // Track price per pair for parsing later
            let mut price_map: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();
            let mut atr_map: std::collections::HashMap<String, Option<f64>> =
                std::collections::HashMap::new();

            for pd in &pair_data_vec {
                // Extract just the market data section from each pair's user message
                // (skip the duplicate decision prompt at the end)
                let user_msg = &pd.user_message;
                let data_section = if let Some(idx) = user_msg.rfind("## Decision Required") {
                    &user_msg[..idx]
                } else {
                    user_msg
                };
                batch_msg.push_str(data_section);
                batch_msg.push_str("\n---\n\n");
                price_map.insert(pd.pair.clone(), pd.current_price);
                atr_map.insert(pd.pair.clone(), pd.indicators.atr);
            }

            batch_msg.push_str("## Decision Required\n");
            batch_msg.push_str(&format!(
                "Return a JSON array with exactly {} decision objects, one per pair evaluated above.\n",
                batch_size
            ));
            batch_msg.push_str("Each object must have the same schema as a single decision, including the \"pair\" field.\n");
            batch_msg.push_str("Example: [{\"action\":\"Pass\",\"pair\":\"WETH/USD\",...}, {\"action\":\"Buy\",\"pair\":\"BTC/USD\",...}]\n");

            let provider = agent.provider_clone();
            let messages = vec![savant_trading::agent::provider::Message {
                role: "user".to_string(),
                content: batch_msg,
            }];

            let start = std::time::Instant::now();
            eval_in_progress.store(true, std::sync::atomic::Ordering::Relaxed);
            log_llm!("LLM", "BATCH EVALUATING {} pairs (single call)", batch_size);
            let response = match tokio::time::timeout(
                std::time::Duration::from_secs(180),
                provider.chat_stream(system_prompt, &messages),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    warn!(
                        "Batch LLM call timed out after 180s — skipping {} pairs",
                        batch_size
                    );
                    // FID-093 C9: Reset eval_in_progress flag on timeout.
                    // Without this, the flag stays true and ALL subsequent cycles
                    // are skipped with "previous LLM eval still in progress."
                    eval_in_progress.store(false, std::sync::atomic::Ordering::SeqCst);
                    continue;
                }
            };
            let elapsed = start.elapsed().as_millis();

            match &response {
                Ok(text) => {
                    log_llm_done!(
                        "LLM",
                        "BATCH COMPLETE {} pairs, {} chars in {}ms",
                        batch_size,
                        text.len(),
                        elapsed
                    );

                    // Strip thinking tags before JSON parse (MiMo v2.5 Pro wraps in <think></think>)
                    let cleaned = savant_trading::agent::decision_parser::strip_thinking_tags(text);

                    // Log raw and cleaned response for debugging batch parse failures
                    tracing::info!("BATCH RAW (first 300): {}", &text[..text.len().min(300)]);
                    tracing::info!(
                        "BATCH CLEANED (first 300): {}",
                        &cleaned[..cleaned.len().min(300)]
                    );

                    // Try to parse as JSON array — handles MiMo returning individual
                    // objects with text between them instead of a clean array
                    match savant_trading::agent::decision_parser::extract_json_array(&cleaned) {
                        Ok(decisions) => {
                            // FID-097 Fix 3: Deduplicate by pair name (keep last decision).
                            // LLM sometimes returns duplicate pairs in a single batch response.
                            let mut seen_pairs: HashMap<String, usize> = HashMap::new();
                            let mut deduped_decisions: Vec<serde_json::Value> = Vec::new();
                            for (idx, decision_val) in decisions.iter().enumerate() {
                                let pair = decision_val
                                    .get("pair")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("UNKNOWN");
                                if let Some(prev_idx) = seen_pairs.get(pair) {
                                    warn!(
                                        "FID-097: Duplicate pair '{}' at index {} (already at {}) — keeping latest",
                                        pair, idx, prev_idx
                                    );
                                    // Remove the earlier duplicate from deduped
                                    deduped_decisions.retain(|d| {
                                        d.get("pair").and_then(|p| p.as_str()).unwrap_or("") != pair
                                    });
                                }
                                seen_pairs.insert(pair.to_string(), idx);
                                deduped_decisions.push(decision_val.clone());
                            }
                            let duplicates_found = decisions.len() - deduped_decisions.len();
                            if duplicates_found > 0 {
                                warn!(
                                    "FID-097: Removed {} duplicate(s) from batch → {} unique decisions",
                                    duplicates_found,
                                    deduped_decisions.len()
                                );
                            }
                            log_phase!(
                                "PHASE2",
                                "Parsed {} decisions from batch response ({}/{} pairs{})",
                                deduped_decisions.len(),
                                deduped_decisions.len(),
                                batch_size,
                                if duplicates_found > 0 {
                                    format!(" — {} dupes removed", duplicates_found)
                                } else {
                                    String::new()
                                }
                            );
                            // FID-097 Fix 5: Validate batch size — log missing pairs
                            if deduped_decisions.len() < batch_size {
                                let returned_pairs: std::collections::HashSet<String> = deduped_decisions
                                    .iter()
                                    .filter_map(|d| d.get("pair").and_then(|p| p.as_str()).map(String::from))
                                    .collect();
                                let requested: &std::collections::HashSet<String> = &price_map.keys().cloned().collect();
                                let missing: Vec<&String> = requested.difference(&returned_pairs).collect();
                                if !missing.is_empty() {
                                    warn!(
                                        "FID-097: Batch incomplete — missing {} pair(s): {}. Will auto-evaluate next cycle.",
                                        missing.len(),
                                        missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                                    );
                                }
                            }
                            for decision_val in deduped_decisions {
                                let pair = decision_val
                                    .get("pair")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("UNKNOWN")
                                    .to_string();
                                let price = price_map.get(&pair).copied().unwrap_or(0.0);
                                let atr = atr_map.get(&pair).copied().flatten();
                                // Re-serialize individual decision for the existing parser
                                let individual_response =
                                    serde_json::to_string(&decision_val).unwrap_or_default();
                                all_results.push(PairResult {
                                    pair,
                                    response: Ok(individual_response),
                                    current_price: price,
                                    _atr: atr,
                                });
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Batch JSON parse failed ({}), falling back to per-pair evaluation",
                                e
                            );
                            // Fallback: evaluate each pair individually
                            // FID-067: Add per-call logging and total timeout to prevent silent hang
                            let fallback_start = std::time::Instant::now();
                            let fallback_timeout = std::time::Duration::from_secs(300); // 5 min total
                            for (idx, pd) in pair_data_vec.into_iter().enumerate() {
                                if fallback_start.elapsed() > fallback_timeout {
                                    warn!(
                                        "Fallback evaluation timed out after {}s — processed {}/{} pairs",
                                        fallback_timeout.as_secs(),
                                        idx,
                                        batch_size
                                    );
                                    break;
                                }
                                let provider = agent.provider_clone();
                                let messages = vec![savant_trading::agent::provider::Message {
                                    role: "user".to_string(),
                                    content: pd.user_message.clone(),
                                }];
                                log_phase!(
                                    "FALLBACK",
                                    "Evaluating {} individually ({}/{})",
                                    pd.pair,
                                    idx + 1,
                                    batch_size
                                );
                                let response =
                                    provider.chat_stream(&pd.system_prompt, &messages).await;
                                all_results.push(PairResult {
                                    pair: pd.pair,
                                    response,
                                    current_price: pd.current_price,
                                    _atr: pd.indicators.atr,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    log_swap_fail!("LLM", "BATCH ERROR: {}", e);
                    // Don't fall back on API errors — just log and continue
                }
            }
        }

        // FID-073 Issue 2: Clear eval flag after Phase 2 completes
        eval_in_progress.store(false, std::sync::atomic::Ordering::Relaxed);

        log_phase!("PHASE3", "Processing {} LLM results...", all_results.len());
        log_phase!(
            "PHASE2",
            "Complete: {}/{} pairs evaluated",
            all_results.len(),
            active_pairs.len()
        );

        // === PHASE 3: Process all results sequentially ===
        let total_results = all_results.len();
        let mut pass_count = 0usize;
        let mut pass_confident = false;
        let mut buy_sell_count = 0usize;
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
                Ok(mut decision) => {
                    // Compact decision log: [PASS] LONG BTC/USD | 0% | R:R 0.0 | reason...
                    let reasoning_short: String = decision.reasoning.chars().take(60).collect();
                    let reasoning_short = if decision.reasoning.len() > 60 {
                        format!("{}...", reasoning_short)
                    } else {
                        reasoning_short
                    };
                    let action_label = match decision.action {
                        savant_trading::agent::decision_parser::TradeAction::Pass => "PASS",
                        savant_trading::agent::decision_parser::TradeAction::Buy => "BUY",
                        savant_trading::agent::decision_parser::TradeAction::Sell => "SELL",
                        savant_trading::agent::decision_parser::TradeAction::Close => "CLOSE",
                        savant_trading::agent::decision_parser::TradeAction::AdjustStop => "ADJUST",
                    };
                    log_decision!(
                        action_label,
                        "[{}] \x1b[90m[{}]\x1b[0m | {:.0}% | R:{:.1} | {}",
                        decision.side,
                        decision.pair,
                        decision.confidence * 100.0,
                        decision.risk_reward,
                        reasoning_short
                    );

                    // Log ALL decisions including Hold (CRIT-2)
                    let decision_record = savant_trading::core::shared::DecisionRecord {
                        timestamp: Utc::now().to_rfc3339(),
                        pair: decision.pair.clone(),
                        action: format!("{:?}", decision.action),
                        side: format!("{}", decision.side),
                        entry_price: decision.entry_price,
                        stop_loss: decision.stop_loss,
                        take_profit_1: decision.take_profit_1,
                        take_profit_2: decision.take_profit_2,
                        take_profit_3: decision.take_profit_3,
                        confidence: decision.confidence,
                        reasoning: decision.reasoning.clone(),
                    };
                    shared.push_decision(decision_record);

                    // FID-085: Append to persistent decision log
                    decision_log.append(savant_trading::agent::decision_log::DecisionEntry {
                        timestamp: Utc::now().to_rfc3339(),
                        pair: decision.pair.clone(),
                        action: action_label.to_string(),
                        confidence: decision.confidence,
                        risk_reward: decision.risk_reward,
                        stop_loss: decision.stop_loss,
                        take_profit: decision.take_profit_1,
                        reasoning: decision.reasoning.clone(),
                        outcome: None,
                    });

                    // Activity feed: mirror terminal decisions (not PASS — too noisy)
                    if action_label != "PASS" {
                        shared
                            .log_activity(
                                savant_trading::core::shared::ActivityLevel::Decision,
                                &decision.pair,
                                &format!(
                                    "{} [{}] | {:.0}% | R:{:.1} | {}",
                                    action_label,
                                    decision.side,
                                    decision.confidence * 100.0,
                                    decision.risk_reward,
                                    reasoning_short
                                ),
                            )
                            .await;
                        if let Some(ref j) = journal {
                            let _ = j
                                .record_activity(
                                    "Decision",
                                    &decision.pair,
                                    &format!(
                                        "{} [{}] | {:.0}% | R:{:.1} | {}",
                                        action_label,
                                        decision.side,
                                        decision.confidence * 100.0,
                                        decision.risk_reward,
                                        reasoning_short
                                    ),
                                )
                                .await;
                        }
                    }

                    // Log to vault
                    if vault_config.enabled {
                        match vault_writer.project_decision(
                            &decision.pair,
                            &format!("{:?}", decision.action),
                            decision.confidence,
                            &decision.reasoning,
                        ) {
                            Ok(()) => log_vault!("VAULT", "Saved {}", decision.pair),
                            Err(e) => log_warn!("VAULT", "Failed {}: {}", decision.pair, e),
                        }
                    }

                    // Capture episodic memory with timeout to prevent SQLite deadlocks
                    if let Some(ref mem) = memory {
                        let pair_data = _pair_data_for_memory
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
                                == savant_trading::agent::decision_parser::TradeAction::Pass
                            {
                                "held".to_string()
                            } else {
                                "executed".to_string()
                            },
                        };
                        // FID-098 Fix 1a: Store episode_id so we can update outcomes when trades close
                        let episode_id = match tokio::time::timeout(
                            std::time::Duration::from_secs(2),
                            mem.capture_episode(&snapshot),
                        )
                        .await
                        {
                            Ok(Ok(id)) => {
                                log_phase!("EPISODIC", "Saved {} [{}]", decision.pair, &id[..8]);
                                Some(id)
                            }
                            Ok(Err(e)) => {
                                log_warn!("EPISODIC", "Failed {}: {}", decision.pair, e);
                                None
                            }
                            Err(_) => {
                                log_warn!("EPISODIC", "Timeout {}", decision.pair);
                                None
                            }
                        };
                        // Store episode_id for outcome updates when trades close.
                        // For PASS decisions, update immediately with counterfactual.
                        // For BUY/SELL, store for later update on trade close.
                        if let Some(eid) = episode_id {
                            let action_str = format!("{:?}", decision.action);
                            episode_store.insert(
                                format!("{}-{}-{}", decision.pair, action_str, tick),
                                eid,
                            );
                        }
                    }

                    // FID-088: Engine-side management trigger evaluation (Safety Net #2).
                    // If the LLM returned Pass but a position has a management trigger,
                    // override to the mandated action. This catches cases where the LLM
                    // didn't produce the position_audit fields (weak model fallback).
                    // FID-089: Use actual market price from market_stores, not stale pos.current_price.
                    if decision.action == savant_trading::agent::decision_parser::TradeAction::Pass {
                        let pair_pos = portfolio.positions().values()
                            .find(|p| p.pair == decision.pair)
                            .cloned();
                        if let Some(pos) = pair_pos {
                            let pair_data = _pair_data_for_memory
                                .iter()
                                .find(|(p, _, _)| *p == decision.pair);
                            let (atr_val, adx_val, _rsi_val, _regime_str) = pair_data
                                .map(|(_, ind, reg)| (ind.atr, ind.adx, ind.rsi, format!("{}", reg)))
                                .unwrap_or((None, None, None, "Unknown".to_string()));

                            // FID-089 Fix 1: Get actual market price from candle data
                            let actual_market_price = market_stores
                                .get(decision.pair.as_str())
                                .and_then(|s| s.last().map(|c| c.close))
                                .unwrap_or(0.0);

                            // FID-089: Debug log for trigger price diagnosis
                            if actual_market_price <= 0.0 {
                                warn!(
                                    "FID-089 TRIGGER PRICE: {} — market_stores lookup returned 0. Keys available: {:?}",
                                    decision.pair,
                                    market_stores.keys().collect::<Vec<_>>()
                                );
                            } else {
                                debug!(
                                    "FID-089 TRIGGER PRICE: {} — market={:.2}, entry={:.2}, current={:.2}",
                                    decision.pair, actual_market_price, pos.entry_price, pos.current_price
                                );
                            }

                            // FID-089 Fix 5: Guard — skip trigger if price not yet updated
                            // (current_price still equals entry_price from wallet recovery)
                            let price_stale = pos.entry_price > 0.0
                                && (pos.current_price - pos.entry_price).abs() / pos.entry_price < 0.001;
                            if price_stale && actual_market_price <= 0.0 {
                                // No market data available and price hasn't been updated — skip
                                debug!("FID-089: Skipping trigger for {} — price not yet updated from market data", decision.pair);
                            } else {
                                // Use actual market price if available, else fall back to pos.current_price
                                let effective_price = if actual_market_price > 0.0 {
                                    actual_market_price
                                } else {
                                    pos.current_price
                                };

                                let mut trigger_fired = false;
                                let mut mandated_action = String::new();
                                let mut mandated_stop = 0.0;
                                let mut trigger_reason = String::new();

                                // FID-089 Fix 7: ATR sanity check — skip if ATR > 10% of price
                                let atr_valid = atr_val.is_some_and(|a| {
                                    a > 0.0 && a < effective_price * 0.10
                                });

                                // Trigger 1: Stop Distance Violation
                                if atr_valid {
                                    if let Some(atr) = atr_val {
                                        let stop_distance = (pos.entry_price - pos.stop_loss).abs();
                                        let stop_distance_atr = stop_distance / atr;
                                        if stop_distance_atr > 2.5 {
                                            trigger_fired = true;
                                            mandated_action = "adjust_stop".to_string();
                                            // FID-089: Use actual market price, not stale pos.current_price
                                            mandated_stop = match pos.side {
                                                Side::Long => effective_price - (atr * 1.5),
                                                Side::Short => effective_price + (atr * 1.5),
                                            };
                                            trigger_reason = format!(
                                                "Stop distance {:.1}x ATR exceeds 2.5x threshold (market price: {:.2})",
                                                stop_distance_atr, effective_price
                                            );
                                        }
                                    }
                                }

                                // Trigger 2: Regime Incompatibility (if ADX available)
                                if !trigger_fired {
                                    if let Some(adx) = adx_val {
                                        let stop_pct = if pos.entry_price > 0.0 {
                                            (pos.entry_price - pos.stop_loss).abs() / pos.entry_price * 100.0
                                        } else { 0.0 };
                                        if adx < 20.0 && stop_pct > 5.0 {
                                            trigger_fired = true;
                                            mandated_action = "adjust_stop".to_string();
                                            if atr_valid {
                                                if let Some(atr) = atr_val {
                                                    mandated_stop = match pos.side {
                                                        Side::Long => effective_price - (atr * 1.5),
                                                        Side::Short => effective_price + (atr * 1.5),
                                                    };
                                                }
                                            }
                                            trigger_reason = format!(
                                                "Regime change: ADX={:.1} (ranging) but position has {:.1}% stop (trending-era, market price: {:.2})",
                                                adx, stop_pct, effective_price
                                            );
                                        }
                                    }
                                }

                                // FID-092 Trigger 3: Adverse Trend Exit
                                // ADX > 25 AND position underwater AND EMA bearish → CLOSE
                                // High ADX + underwater = strong trend AGAINST position
                                if !trigger_fired {
                                    if let Some(adx) = adx_val {
                                    let ema_bearish = pair_data.is_some_and(|(_, ind, _)| {
                                        match (ind.ema_fast, ind.ema_slow) {
                                            (Some(f), Some(s)) => f < s,
                                            _ => false,
                                        }
                                    });
                                        let underwater = pos.unrealized_pnl < 0.0
                                            || effective_price < pos.entry_price;
                                        if adx > 25.0 && underwater && ema_bearish && pos.side == Side::Long {
                                            trigger_fired = true;
                                            mandated_action = "close".to_string();
                                            trigger_reason = format!(
                                                "Adverse trend: ADX={:.1} (strong trend), EMA bearish, position underwater ({:.2}% loss)",
                                                adx,
                                                if pos.entry_price > 0.0 { (effective_price - pos.entry_price) / pos.entry_price * 100.0 } else { 0.0 }
                                            );
                                        }
                                    }
                                }

                                // FID-092 Trigger 4: Maximum Hold Duration (24h)
                                // Position age > 24h AND PnL <= 0 → CLOSE
                                if !trigger_fired {
                                    let hold_duration = chrono::Utc::now() - pos.opened_at;
                                    let max_hold = chrono::Duration::hours(24);
                                    if hold_duration > max_hold && pos.unrealized_pnl <= 0.0 {
                                        trigger_fired = true;
                                        mandated_action = "close".to_string();
                                        trigger_reason = format!(
                                            "Max hold duration: {:.1}h > 24h limit, PnL: ${:.2}",
                                            hold_duration.num_minutes() as f64 / 60.0,
                                            pos.unrealized_pnl
                                        );
                                    }
                                }

                                // FID-092 Trigger 5: Per-Position Drawdown Limit
                                // Position loss > 5% of portfolio equity → CLOSE
                                if !trigger_fired {
                                    let equity = portfolio.account().equity;
                                    let max_loss = equity * 0.05;
                                    if pos.unrealized_pnl < -max_loss && pos.unrealized_pnl < 0.0 {
                                        trigger_fired = true;
                                        mandated_action = "close".to_string();
                                        trigger_reason = format!(
                                            "Per-position drawdown: PnL ${:.2} exceeds 5% of equity ${:.2} (limit: ${:.2})",
                                            pos.unrealized_pnl, equity, max_loss
                                        );
                                    }
                                }

                                // FID-092 Trigger 6: Parabolic SAR Exit
                                // Current price below SAR → CLOSE
                                if !trigger_fired {
                                    let sar_val = pair_data.and_then(|(_, ind, _)| ind.parabolic_sar);
                                    if let Some(sar) = sar_val {
                                        let price_below_sar = match pos.side {
                                            Side::Long => effective_price < sar,
                                            Side::Short => effective_price > sar,
                                        };
                                        if price_below_sar {
                                            trigger_fired = true;
                                            mandated_action = "close".to_string();
                                            trigger_reason = format!(
                                                "Parabolic SAR exit: price {:.2} crossed SAR {:.2} ({})",
                                                effective_price, sar,
                                                if pos.side == Side::Long { "below for long" } else { "above for short" }
                                            );
                                        }
                                    }
                                }

                                if trigger_fired {
                                    // FID-094 Fix 3: Don't fire ADJUST_STOP if close is on cooldown.
                                    // Tightening the stop is futile if we can't execute the close.
                                    let on_cooldown = close_failure_cooldown
                                        .get(decision.pair.as_str())
                                        .is_some_and(|t| t.elapsed().as_secs() < CLOSE_COOLDOWN_SECS);
                                    if on_cooldown && mandated_action == "adjust_stop" {
                                        debug!(
                                            "FID-094 TRIGGER GUARD: {} — close on cooldown, skipping futile ADJUST_STOP",
                                            decision.pair
                                        );
                                    } else {
                                    warn!(
                                        "FID-088 ENGINE TRIGGER: {} — {}. Overriding Pass to {}. New stop: {:.4}",
                                        decision.pair, trigger_reason, mandated_action, mandated_stop
                                    );
                                    shared.log_activity(
                                        savant_trading::core::shared::ActivityLevel::Warning,
                                        &decision.pair,
                                        &format!(
                                            "FID-088 TRIGGER: {} — overriding to {}",
                                            trigger_reason, mandated_action
                                        ),
                                    ).await;
                                    // Override the decision
                                    if mandated_action == "adjust_stop" && mandated_stop > 0.0 {
                                        decision.action = savant_trading::agent::decision_parser::TradeAction::AdjustStop;
                                        decision.stop_loss = mandated_stop;
                                    } else if mandated_action == "close" {
                                        decision.action = savant_trading::agent::decision_parser::TradeAction::Close;
                                    }
                                    } // end FID-094 trigger guard else
                                }
                            }
                        }
                    }

                    // Skip execution for Hold decisions
                    if decision.action == savant_trading::agent::decision_parser::TradeAction::Pass
                    {
                        if decision.confidence >= 0.25 {
                            pass_confident = true;
                        }
                        pass_count += 1;
                        continue;
                    }

                    buy_sell_count += 1;

                    // FID-108: Session liquidity penalty
                    let hour = Utc::now().hour();
                    let session_mult = match hour {
                        2..=5 => config.trading.session_penalty_deep_asian,
                        6..=8 => 0.95,
                        13..=17 => 1.05,
                        _ => 1.0,
                    };
                    if session_mult != 1.0 {
                        let original = decision.confidence;
                        decision.confidence *= session_mult;
                        tracing::debug!(
                            "FID-108: Session penalty {:.0}% → {:.0}% (hour={}, mult={})",
                            original * 100.0,
                            decision.confidence * 100.0,
                            hour,
                            session_mult
                        );
                    }

                    // FID-108: Check blacklist before execution
                    let base_sym = decision.pair.split('/').next().unwrap_or(&decision.pair);
                    if failure_tracker.is_blacklisted(base_sym) {
                        let remaining = failure_tracker
                            .blacklist_remaining(base_sym)
                            .map(|d| format!("{}min", d.num_minutes()))
                            .unwrap_or_else(|| "unknown".into());
                        warn!(
                            "FID-108: Skipping blacklisted {} ({} remaining)",
                            base_sym, remaining
                        );
                        continue;
                    }

                    // FID-072: Calculate actual R:R from prices for comparison with LLM's claim
                    let actual_rr = if decision.entry_price > 0.0 && decision.stop_loss > 0.0 && decision.take_profit_1 > 0.0 {
                        let risk = (decision.entry_price - decision.stop_loss).abs();
                        let reward = (decision.take_profit_1 - decision.entry_price).abs();
                        if risk > 0.0 { reward / risk } else { 0.0 }
                    } else { 0.0 };

                    info!(
                        "AI DECISION: {:?} {} {} @ {:.2} | SL: {:.2} | TP1: {:.2} | Conf: {:.0}% | R:R claimed={:.2} actual={:.2} | Reason: {}",
                        decision.action, decision.pair, decision.side,
                        decision.entry_price, decision.stop_loss, decision.take_profit_1,
                        decision.confidence * 100.0, decision.risk_reward, actual_rr, decision.reasoning,
                    );

                    // Execute if autonomous
                    log_phase!(
                        "EXECUTION",
                        "Checking {} (action={:?})",
                        decision.pair,
                        decision.action
                    );
                    if matches!(autonomy, AutonomyLevel::Autonomous) {
                        use savant_trading::agent::decision_parser::TradeAction;

                        // FID-108: Circuit breaker only blocks NEW positions (Buy/Sell),
                        // not management actions (Close/AdjustStop) on existing positions.
                        let needs_circuit_check = matches!(
                            decision.action,
                            TradeAction::Buy | TradeAction::Sell
                        );
                        let circuit_ok = if needs_circuit_check {
                            match circuit_breaker.check(portfolio.account()) {
                                CircuitBreakerResult::Triggered(reason) => {
                                    log_circuit!("CIRCUIT BREAKER", "{} — {}", decision.pair, reason);
                                    let _ = std::fs::write(
                                        "savant.blocked",
                                        format!("{}\nReason: {}\n", Utc::now().to_rfc3339(), reason),
                                    );
                                    error!("CIRCUIT BREAKER TRIGGERED — wrote savant.blocked.");
                                    false
                                }
                                CircuitBreakerResult::Ok => true,
                            }
                        } else {
                            true // Management actions always allowed
                        };

                        if circuit_ok {

                                match decision.action {
                                    TradeAction::Sell | TradeAction::Close => {
                                        // --- CLOSE LOGIC ---
                                        // Find existing positions for this pair and close them.
                                        let positions_to_close: Vec<(String, Position)> = {
                                            let positions = if let Some(ref ex) = executor {
                                                ex.open_positions()
                                            } else {
                                                portfolio.positions().values().collect()
                                            };
                                            positions
                                                .into_iter()
                                                .filter(|p| p.pair == decision.pair)
                                                .map(|p| (p.id.clone(), p.clone()))
                                                .collect()
                                        };

                                        if positions_to_close.is_empty() {
                                            // DEX cannot SHORT — must own token to sell it.
                                            // AI Sell signal skipped until a LONG position is opened first.
                                            log_phase!("SELL", "{} — no own tokens, cannot SHORT (DEX requires owning the asset)", decision.pair);
                                        } else {
                                            for (pos_id, pos) in &positions_to_close {
                                                log_trade!(
                                                    "CLOSE",
                                                    "Position {} for {} (action={:?})",
                                                    pos_id,
                                                    decision.pair,
                                                    decision.action
                                                );
                                                let close_result = if let Some(ref mut ex) =
                                                    executor
                                                {
                                                    match tokio::time::timeout(
                                                        std::time::Duration::from_secs(60),
                                                        ex.close_position(pos_id),
                                                    )
                                                    .await
                                                    {
                                                        Ok(result) => result,
                                                        Err(_) => {
                                                            log_swap_fail!(
                                                                "TIMEOUT",
                                                                "close_position for {} took >60s",
                                                                pos_id
                                                            );
                                                            Err(savant_trading::core::error::ExecutionError::Other(
                                                                format!("close_position timed out after 60s for {}", pos_id)
                                                            ))
                                                        }
                                                    }
                                                } else {
                                                    portfolio.close_position(pos_id).await
                                                };

                                                match close_result {
                                                    Ok(order) => {
                                                        let exit_price = order
                                                            .filled_price
                                                            .or(order.price)
                                                            .unwrap_or(pos.current_price);
                                                        let pnl = match pos.side {
                                                            Side::Long => {
                                                                (exit_price - pos.entry_price)
                                                                    * pos.quantity
                                                            }
                                                            Side::Short => {
                                                                (pos.entry_price - exit_price)
                                                                    * pos.quantity
                                                            }
                                                        };
                                                        let pnl_pct = if pos.entry_price > 0.0 {
                                                            pnl / (pos.entry_price * pos.quantity)
                                                                * 100.0
                                                        } else {
                                                            0.0
                                                        };

                                                        info!(
                                                            "AI {:?} {} — closed position {} | Exit: {:.4} | PnL: ${:.2} ({:.2}%)",
                                                            decision.action, decision.pair, pos_id,
                                                            exit_price, pnl, pnl_pct,
                                                        );
                                                        portfolio.account_mut().trades_today += 1;

                                                        let trade = TradeRecord {
                                                            id: format!("ai-close-{}", tick),
                                                            pair: pos.pair.clone(),
                                                            side: pos.side,
                                                            entry_price: pos.entry_price,
                                                            exit_price,
                                                            quantity: pos.quantity,
                                                            pnl,
                                                            pnl_pct,
                                                            fees: 0.0,
                                                            strategy_name: pos
                                                                .strategy_name
                                                                .clone(),
                                                            opened_at: pos.opened_at,
                                                            closed_at: chrono::Utc::now(),
                                                            notes: format!(
                                                                "AI {:?} via {}",
                                                                decision.action, decision.pair
                                                            ),
                                                            on_chain_verified: false,
                                                            tx_hash: None,
                                                        };

                                                        log_trade!("CLOSED", "{:?} {} | Pos: {} | Exit: {:.4} | PnL: ${:.2} ({:.2}%)",
                                                            decision.action, decision.pair, pos_id,
                                                            exit_price, pnl, pnl_pct);

                                                        // DB: delete position, record trade, log activity — instant
                                                        if let Some(ref j) = journal {
                                                            let _ = j.delete_position(pos_id).await;
                                                            let _ = j.record_trade(&trade).await;
                                                            let _ = j.record_activity("Trade", &trade.pair,
                                                                &format!("CLOSED {} | Exit: {:.4} | PnL: ${:.2} ({:.2}%)",
                                                                    trade.side, exit_price, pnl, pnl_pct)).await;
                                                        }
                                                        shared.log_activity(
                                                            savant_trading::core::shared::ActivityLevel::Trade,
                                                            &trade.pair,
                                                            &format!("CLOSED {} | PnL: ${:.2} ({:.2}%)", trade.side, pnl, pnl_pct),
                                                        ).await;

                                                        // FID-085: Update decision log with trade outcome
                                                        decision_log.update_outcome(&trade.pair, savant_trading::agent::decision_log::TradeOutcome {
                                                            raw_return_pct: pnl_pct,
                                                            alpha_vs_benchmark: 0.0, // TODO: compute vs BTC benchmark
                                                            reflection: if pnl > 0.0 {
                                                                format!("WIN: {} closed at {:.4}, PnL ${:.2}", trade.side, exit_price, pnl)
                                                            } else {
                                                                format!("LOSS: {} closed at {:.4}, PnL ${:.2}", trade.side, exit_price, pnl)
                                                            },
                                                        });

                                                        // FID-098 Fix 1b: Update episodic memory with actual outcome
                                                        if let Some(ref mem) = memory {
                                                            let action_str = format!("{:?}", decision.action);
                                                            let lookup_key = format!("{}-{}-{}", trade.pair, action_str, tick);
                                                            if let Some(episode_id) = episode_store.get(&lookup_key) {
                                                                let achieved_rr = if trade.entry_price > 0.0 && trade.quantity > 0.0 {
                                                                    pnl / (trade.entry_price * trade.quantity)
                                                                } else {
                                                                    0.0
                                                                };
                                                                if let Err(e) = mem.update_outcome(
                                                                    episode_id,
                                                                    pnl,
                                                                    pnl_pct,
                                                                    pnl > 0.0,
                                                                    achieved_rr,
                                                                ).await {
                                                                    warn!("FID-098: Failed to update episode outcome for {}: {}", trade.pair, e);
                                                                } else {
                                                                    debug!("FID-098: Updated episode {} outcome: PnL=${:.2}", &episode_id[..8], pnl);
                                                                }
                                                                episode_store.remove(&lookup_key);
                                                            }
                                                        }

                                                        // Update shared state immediately
                                                        {
                                                            let mut sp =
                                                                shared.positions.write().await;
                                                            *sp = portfolio
                                                                .positions()
                                                                .values()
                                                                .cloned()
                                                                .collect();
                                                            let mut sa =
                                                                shared.account.write().await;
                                                            *sa = portfolio.account().clone();
                                                            let mut st =
                                                                shared.closed_trades.write().await;
                                                            *st =
                                                                portfolio.closed_trades().to_vec();
                                                        }

                                                        event_bus.publish(
                                                            TradingEvent::PositionClosed(trade),
                                                        );
                                                    }
                                                    Err(e) => {
                                                        error!(
                                                            "AI {:?} {} failed for position {}: {}",
                                                            decision.action,
                                                            decision.pair,
                                                            pos_id,
                                                            e,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    TradeAction::Buy => {
                                        // --- OPEN LOGIC ---
                                        log_phase!(
                                            "BUY",
                                            "Calculating position size for {}",
                                            decision.pair
                                        );

                                        // FID-072: Sync on-chain balance before opening new position
                                        if let Some(ref mut ex) = executor {
                                            if ex.sync_balance().await.is_ok() {
                                                let fresh = ex.balance();
                                                portfolio.account_mut().balance = fresh;
                                                debug!("Pre-trade balance sync: ${:.2}", fresh);
                                            }
                                        }

                                        // Price tolerance check (FID-035): reject if price drifted
                                        // too far from AI's entry during LLM evaluation (20-60s window)
                                        let current_price = market_stores
                                            .get(&decision.pair)
                                            .and_then(|s| s.last().map(|c| c.close))
                                            .unwrap_or(decision.entry_price);
                                        let drift = ((current_price - decision.entry_price)
                                            / decision.entry_price)
                                            .abs()
                                            * 100.0;
                                        if drift > config.ai.price_tolerance_pct {
                                            let reason = format!(
                                                "Price drifted {:.1}% (entry={:.4} current={:.4})",
                                                drift, decision.entry_price, current_price
                                            );
                                            log_warn!(
                                                "TOLERANCE",
                                                "{} — {}",
                                                decision.pair,
                                                reason
                                            );
                                            shared.log_activity(
                                                savant_trading::core::shared::ActivityLevel::Warning,
                                                &decision.pair,
                                                &format!("REJECTED: {}", reason),
                                            ).await;
                                            continue;
                                        }

                                        // Token safety verification (FID-052):
                                        // - Curated config pairs: SKIP check entirely (known-good tokens)
                                        // - Other pairs: check but DON'T reject on failure (0x quote is the real gate)
                                        let token_symbol =
                                            decision.pair.split('/').next().unwrap_or("");
                                        let is_curated = curated_pairs.contains(&decision.pair);
                                        if !is_curated {
                                            if let Some((token_addr, _)) =
                                                savant_trading::execution::dex::lookup_token(
                                                    token_symbol,
                                                    config.exchange.dex.chain_id,
                                                )
                                            {
                                                if !token_addr.is_empty() {
                                                    match verify_token_safety(&token_addr).await {
                                                        Ok((vol, holders)) => {
                                                            if vol < 1_000_000.0 {
                                                                log_warn!("VOLUME", "{} — 24h volume ${:.0} < $1M (proceeding anyway, 0x will fail if no liquidity)", decision.pair, vol);
                                                            }
                                                            if holders < 5_000 {
                                                                log_warn!("HOLDERS", "{} — {} holders < 5000 (proceeding anyway, 0x will fail if no liquidity)", decision.pair, holders);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log_warn!("VERIFY", "{} — Blockscout unavailable ({}), proceeding (0x quote is the real gate)", decision.pair, e);
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            info!("Token safety: {} is curated — skipping Blockscout check", decision.pair);
                                        }

                                        // Liquidity pre-check (FID-052): call 0x /price endpoint
                                        // to confirm DEX routing is available before committing.
                                        // This is read-only, no gas, fast (~200ms).
                                        // Returns rich data: tax info (honeypot detection), balance issues.
                                        if let Some(ref mut ex) = executor {
                                            let check_amount = 5.0_f64; // $5 test amount
                                            match ex
                                                .check_liquidity(
                                                    &decision.pair,
                                                    decision.side,
                                                    check_amount,
                                                )
                                                .await
                                            {
                                                Ok(check) => {
                                                    if !check.available {
                                                        let reason = "No DEX liquidity available (0x /price returned false)".to_string();
                                                        log_warn!(
                                                            "LIQUIDITY",
                                                            "{} — {}",
                                                            decision.pair,
                                                            reason
                                                        );
                                                        shared.log_activity(
                                                            savant_trading::core::shared::ActivityLevel::Warning,
                                                            &decision.pair,
                                                            &format!("REJECTED: {}", reason),
                                                        ).await;
                                                        continue;
                                                    }
                                                    // Honeypot detection: buy tax > 1% = suspicious
                                                    if check.buy_tax_bps > 100 {
                                                        let reason = format!(
                                                            "Buy tax {:.1}% — potential honeypot",
                                                            check.buy_tax_bps as f64 / 100.0
                                                        );
                                                        log_warn!(
                                                            "TAX",
                                                            "{} — {}",
                                                            decision.pair,
                                                            reason
                                                        );
                                                        shared.log_activity(
                                                            savant_trading::core::shared::ActivityLevel::Warning,
                                                            &decision.pair,
                                                            &format!("REJECTED: {}", reason),
                                                        ).await;
                                                        continue;
                                                    }
                                                    if !check.balance_ok {
                                                        log_warn!("BALANCE", "{} — insufficient sell token balance (0x issues.balance)", decision.pair);
                                                    }
                                                    info!("Liquidity OK: {} on {} (buy_tax={}bps, price={})", decision.pair, "0x", check.buy_tax_bps, check.price);
                                                }
                                                Err(e) => {
                                                    log_warn!("LIQUIDITY", "{} — pre-check error ({}), proceeding anyway", decision.pair, e);
                                                }
                                            }
                                        }

                                        // FID-104: Auto-extend TP to meet minimum R:R before sizer check
                                        let risk = (decision.entry_price - decision.stop_loss).abs();
                                        if risk > 0.0 {
                                            let reward = match decision.side {
                                                Side::Long => decision.take_profit_1 - decision.entry_price,
                                                Side::Short => decision.entry_price - decision.take_profit_1,
                                            };
                                            let actual_rr = reward / risk;
                                            let min_rr = config.risk.min_rr_ratio;
                                            if actual_rr < min_rr && reward > 0.0 {
                                                let required_reward = risk * min_rr;
                                                let old_tp = decision.take_profit_1;
                                                decision.take_profit_1 = match decision.side {
                                                    Side::Long => decision.entry_price + required_reward,
                                                    Side::Short => decision.entry_price - required_reward,
                                                };
                                                decision.risk_reward = min_rr;
                                                info!("FID-104: Extended TP for {}: {:.4} → {:.4} (R:R {:.2}→{:.2})",
                                                    decision.pair, old_tp, decision.take_profit_1, actual_rr, min_rr);
                                            }
                                        }

                                        let ps = position_sizer.calculate(
                                            portfolio.account(),
                                            decision.entry_price,
                                            decision.stop_loss,
                                            decision.take_profit_1,
                                            decision.side,
                                        );

                                        if let Some(mut ps) = ps {
                                            // P1-3a: Compute TP2/TP3 from ATR
                                            let atr = market_stores
                                                .get(&decision.pair)
                                                .and_then(|s| {
                                                    let closes: Vec<f64> = s.candles().iter().rev().take(14).map(|c| c.close).collect();
                                                    if closes.len() < 14 { return None; }
                                                    let mean = closes.iter().sum::<f64>() / closes.len() as f64;
                                                    let variance = closes.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / closes.len() as f64;
                                                    Some(variance.sqrt())
                                                });
                                            if let Some(atr_val) = atr {
                                                let (tp2, tp3) = match decision.side {
                                                    Side::Long => (
                                                        decision.take_profit_1 + atr_val * 1.0,
                                                        decision.take_profit_1 + atr_val * 2.0,
                                                    ),
                                                    Side::Short => (
                                                        decision.take_profit_1 - atr_val * 1.0,
                                                        decision.take_profit_1 - atr_val * 2.0,
                                                    ),
                                                };
                                                decision.take_profit_2 = tp2;
                                                decision.take_profit_3 = tp3;
                                                info!("FID-102: TP2/TP3 from ATR for {}: TP2={:.4} TP3={:.4} (ATR={:.4})",
                                                    decision.pair, tp2, tp3, atr_val);
                                            }
                                            let session =
                                                savant_trading::core::session::current_session();
                                            let session_mult = session.position_size_multiplier();
                                            if session_mult != 1.0 {
                                                ps.quantity *= session_mult;
                                                ps.risk_amount *= session_mult;
                                            }

                                            // Duplicate guard: skip if already have open position on this pair+side
                                            let already_open = {
                                                let positions = if let Some(ref ex) = executor {
                                                    ex.open_positions()
                                                } else {
                                                    portfolio.positions().values().collect()
                                                };
                                                positions.iter().any(|p| {
                                                    p.pair == decision.pair
                                                        && p.side == decision.side
                                                })
                                            };
                                            // Concentration cap: full_deploy allows 100%, normal mode 33%
                                            // Use 99.99% of cap to prevent rounding past wallet balance
                                            let total_portfolio = if let Some(ref ex) = executor {
                                                ex.balance()
                                            } else {
                                                portfolio.account().balance
                                            };
                                            let max_concentration = if config.trading.full_deploy
                                                && total_portfolio
                                                    < config.risk.low_balance_threshold
                                            {
                                                1.00
                                            } else {
                                                0.33
                                            };
                                            let safe_max = total_portfolio * max_concentration * 0.9999;
                                            let order_value = decision.entry_price * ps.quantity;
                                            if order_value > safe_max {
                                                // Auto-adjust: percentage-based sizing with buffer
                                                let adjusted_qty = safe_max / decision.entry_price;
                                                let pct_label = if max_concentration >= 1.0 { "100%" } else { "33%" };
                                                info!(
                                                    "AI BUY {} — Auto-adjusting to {} cap: ${:.2} -> ${:.2} (qty {:.4} -> {:.4})",
                                                    decision.pair, pct_label, order_value, safe_max, ps.quantity, adjusted_qty
                                                );
                                                shared.log_activity(
                                                    savant_trading::core::shared::ActivityLevel::Info,
                                                    &decision.pair,
                                                    &format!("ADJUSTED: ${:.2} -> ${:.2} ({} cap)", order_value, safe_max, pct_label),
                                                ).await;
                                                ps.quantity = adjusted_qty;
                                                // Inject feedback into decision log so LLM knows its signal was correct
                                                decision_log.append(savant_trading::agent::decision_log::DecisionEntry {
                                                    timestamp: Utc::now().to_rfc3339(),
                                                    pair: decision.pair.clone(),
                                                    action: "FEEDBACK".to_string(),
                                                    confidence: decision.confidence,
                                                    risk_reward: decision.risk_reward,
                                                    stop_loss: decision.stop_loss,
                                                    take_profit: decision.take_profit_1,
                                                    reasoning: format!(
                                                        "Your BUY signal was correct. Position auto-adjusted from ${:.2} to ${:.2} ({} portfolio cap). Analysis was sound — only sizing was adjusted.",
                                                        order_value, safe_max, pct_label
                                                    ),
                                                    outcome: None,
                                                });
                                            }
                                            if already_open {
                                                let reason =
                                                    "Already have open position on this pair+side"
                                                        .to_string();
                                                info!(
                                                    "AI BUY {} {:?} — {}",
                                                    decision.pair, decision.side, reason
                                                );
                                                shared.log_activity(
                                                    savant_trading::core::shared::ActivityLevel::Warning,
                                                    &decision.pair,
                                                    &format!("SKIPPED: {}", reason),
                                                ).await;
                                            } else {
                                                log_swap!(
                                                    "ORDER",
                                                    "Placing for {} via executor...",
                                                    decision.pair
                                                );
                                                let order = if let Some(ref mut ex) = executor {
                                                    match tokio::time::timeout(
                                                        std::time::Duration::from_secs(60),
                                                        ex.place_order(
                                                            &decision.pair,
                                                            decision.side,
                                                            ps.quantity,
                                                            Some(decision.entry_price),
                                                        ),
                                                    )
                                                    .await
                                                    {
                                                        Ok(result) => result,
                                                        Err(_) => {
                                                            log_swap_fail!(
                                                                "TIMEOUT",
                                                                "place_order for {} took >60s",
                                                                decision.pair
                                                            );
                                                            Err(savant_trading::core::error::ExecutionError::Other(
                                                                format!("place_order timed out after 60s for {}", decision.pair)
                                                            ))
                                                        }
                                                    }
                                                } else {
                                                    portfolio
                                                        .place_order(
                                                            &decision.pair,
                                                            decision.side,
                                                            ps.quantity,
                                                            Some(decision.entry_price),
                                                        )
                                                        .await
                                                };

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
                                                        // Track position in PortfolioManager for state/reporting
                                                        portfolio
                                                            .positions_mut()
                                                            .insert(pos.id.clone(), pos.clone());
                                                        portfolio.account_mut().open_positions =
                                                            portfolio.positions().len();
                                                        portfolio.account_mut().trades_today += 1;
                                                        portfolio.refresh_equity();
                                                        let acc = portfolio.account();
                                                        info!("AI position opened: {} — balance ${:.2}, equity ${:.2}",
                                                            decision.pair, acc.balance, acc.equity);

                                                        // Place stop-loss on executor for live mode
                                                        if let Some(ref mut ex) = executor {
                                                            if let Some(exec_pos) = ex
                                                                .open_positions()
                                                                .iter()
                                                                .find(|p| {
                                                                    p.pair == pos.pair
                                                                        && p.side == pos.side
                                                                })
                                                            {
                                                                let exec_id = exec_pos.id.clone();
                                                                executor_position_map.insert(
                                                                    pos.id.clone(),
                                                                    exec_id.clone(),
                                                                );
                                                                if let Err(e) = ex
                                                                    .place_stop_loss(&exec_id)
                                                                    .await
                                                                {
                                                                    warn!("Failed to place stop-loss for position {}: {}", exec_id, e);
                                                                } else {
                                                                    info!("Stop-loss placed for position {} @ {:.4}", exec_id, pos.stop_loss);
                                                                }
                                                            } else {
                                                                warn!("Position not found for stop-loss after placing order for {}", pos.pair);
                                                            }
                                                        }

                                                        // Write trade alert to file for external monitoring
                                                        let alert = serde_json::json!({
                                                            "type": "TRADE_OPENED",
                                                            "timestamp": chrono::Utc::now().to_rfc3339(),
                                                            "pair": decision.pair,
                                                            "side": format!("{:?}", decision.side),
                                                            "action": format!("{:?}", decision.action),
                                                            "entry_price": decision.entry_price,
                                                            "stop_loss": decision.stop_loss,
                                                            "take_profit_1": decision.take_profit_1,
                                                            "quantity": ps.quantity,
                                                            "risk_amount": ps.risk_amount,
                                                            "confidence": decision.confidence,
                                                            "risk_reward": decision.risk_reward,
                                                        });
                                                        let alert_line = format!("{}\n", alert);
                                                        let _ = std::fs::OpenOptions::new()
                                                            .create(true)
                                                            .append(true)
                                                            .open("data/alerts.jsonl")
                                                            .and_then(|mut f| {
                                                                use std::io::Write;
                                                                f.write_all(alert_line.as_bytes())
                                                            });

                                                        log_trade!("OPENED", "{} {:?} @ {:.4} | Qty: {:.4} | SL: {:.4} | TP1: {:.4} TP2: {:.4} TP3: {:.4} | Risk: ${:.2} | Scale: 50%→TP1, 30%→TP2, 20%→TP3",
                                                            decision.side, decision.action, decision.entry_price,
                                                            ps.quantity, decision.stop_loss, decision.take_profit_1, decision.take_profit_2, decision.take_profit_3, ps.risk_amount);

                                                        // Persist to DB instantly
                                                        if let Some(ref j) = journal {
                                                            if let Err(e) =
                                                                j.save_position(&pos).await
                                                            {
                                                                warn!("Failed to persist position to DB: {}", e);
                                                            }
                                                            let _ = j.record_activity("Trade", &pos.pair,
                                                                &format!("OPENED {} {} @ {:.4} | Qty: {:.4} | SL: {:.4} | TP1: {:.4}",
                                                                    decision.side, decision.pair, decision.entry_price,
                                                                    ps.quantity, decision.stop_loss, decision.take_profit_1)).await;
                                                        }

                                                        // Update shared state immediately
                                                        {
                                                            let mut sp =
                                                                shared.positions.write().await;
                                                            *sp = portfolio
                                                                .positions()
                                                                .values()
                                                                .cloned()
                                                                .collect();
                                                            let mut sa =
                                                                shared.account.write().await;
                                                            *sa = portfolio.account().clone();
                                                        }

                                                        event_bus.publish(
                                                            TradingEvent::PositionOpened(pos),
                                                        );
                                                    }
                                                    // FID-108: Record failure in tracker
                                                    Err(e) => {
                                                        let category = savant_trading::execution::dex::trader::categorize_error(&e);
                                                        let base = decision.pair.split('/').next().unwrap_or(&decision.pair);
                                                        failure_tracker.record_failure(base, &e.to_string(), &category);
                                                        error!("AI order failed: {} | category={}", e, category);
                                                    }
                                                }
                                            }
                                        } else {
                                            let actual_rr = match decision.side {
                                                Side::Long => {
                                                    if decision.entry_price > decision.stop_loss
                                                        && decision.stop_loss > 0.0
                                                    {
                                                        (decision.take_profit_1
                                                            - decision.entry_price)
                                                            / (decision.entry_price
                                                                - decision.stop_loss)
                                                    } else {
                                                        0.0
                                                    }
                                                }
                                                Side::Short => {
                                                    if decision.stop_loss > decision.entry_price
                                                        && decision.entry_price > 0.0
                                                    {
                                                        (decision.entry_price
                                                            - decision.take_profit_1)
                                                            / (decision.stop_loss
                                                                - decision.entry_price)
                                                    } else {
                                                        0.0
                                                    }
                                                }
                                            };
                                            let reason = format!("Position sizer rejected — claimed R:R={:.1}, actual={:.1} (entry={} stop={} tp={})", decision.risk_reward, actual_rr, decision.entry_price, decision.stop_loss, decision.take_profit_1);
                                            log_swap_fail!(
                                                "BUY REJECTED",
                                                "{} — {}",
                                                decision.pair,
                                                reason
                                            );
                                            shared.log_activity(
                                                savant_trading::core::shared::ActivityLevel::Warning,
                                                &decision.pair,
                                                &format!("REJECTED: {}", reason),
                                            ).await;
                                        }
                                    }
                                    TradeAction::Pass => {
                                        // Already handled in pre-execution filter above.
                                        // Reaching here means Pass was not filtered — skip silently.
                                        continue;
                                    }
                                    TradeAction::AdjustStop => {
                                        // Wire AdjustStop to stop_overrides shared state
                                        if decision.stop_loss > 0.0 {
                                            let mut overrides = shared.stop_overrides.write().await;
                                            overrides.insert(decision.pair.clone(), decision.stop_loss);
                                            info!(
                                                "AI ADJUST_STOP for {} → ${:.4} (confidence {:.0}%)",
                                                decision.pair,
                                                decision.stop_loss,
                                                decision.confidence * 100.0
                                            );
                                            shared.log_activity(
                                                savant_trading::core::shared::ActivityLevel::Info,
                                                &decision.pair,
                                                &format!("ADJUST STOP → ${:.4}", decision.stop_loss),
                                            ).await;
                                        } else {
                                            warn!("AI ADJUST_STOP for {} but stop_loss={:.4} — ignoring", decision.pair, decision.stop_loss);
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

        // FID-046: Summary when no actionable setups found
        if buy_sell_count == 0 && pass_count > 0 && !pass_confident {
            log_phase!(
                "CYCLE",
                "No actionable setups — 0/{} pairs ({} evaluated, {} dead)",
                pass_count,
                total_results,
                dead_tokens.len()
            );
        }

        // Check stops for all positions after processing all pairs
        // FID-081: Build all_prices with staleness guard for check_stops
        let mut all_prices: HashMap<String, f64> = market_stores
            .iter()
            .filter_map(|(pair, store)| store.last().map(|c| (pair.clone(), c.close)))
            .collect();
        let staleness_threshold = std::time::Duration::from_secs(300);
        for (pair, (price, timestamp)) in &ws_ticker_prices {
            if timestamp.elapsed() > staleness_threshold {
                continue; // Skip stale WS prices
            }
            all_prices.insert(pair.clone(), *price);
        }
        // Capture portfolio position IDs and full details BEFORE check_stops removes them from the map
        let paper_positions_before: Vec<(String, String, Side, f64)> = portfolio
            .positions()
            .iter()
            .map(|(id, pos)| (id.clone(), pos.pair.clone(), pos.side, pos.entry_price))
            .collect();
        // Full position clones for restoration if executor close fails
        let paper_positions_full: std::collections::HashMap<
            String,
            savant_trading::core::types::Position,
        > = portfolio
            .positions()
            .iter()
            .map(|(id, pos)| (id.clone(), pos.clone()))
            .collect();

        // Apply stop-loss overrides from API before checking stops
        {
            let mut overrides = shared.stop_overrides.write().await;
            if !overrides.is_empty() {
                for (pair, new_stop) in overrides.drain() {
                    if let Some(pos) = portfolio
                        .positions()
                        .values()
                        .find(|p| p.pair == pair)
                        .cloned()
                    {
                        let old_stop = pos.stop_loss;
                        // FID-073 Issue 1: Directional guard — never move stop backward
                        let valid = match pos.side {
                            savant_trading::core::types::Side::Long => new_stop > old_stop,
                            savant_trading::core::types::Side::Short => new_stop < old_stop,
                        };
                        if !valid {
                            warn!(
                                "Stop override rejected: {} — new ${:.4} is worse than current ${:.4}",
                                pair, new_stop, old_stop
                            );
                            continue;
                        }
                        // Update in portfolio
                        if let Some((_, pm_pos)) = portfolio
                            .positions_mut()
                            .iter_mut()
                            .find(|(_, p)| p.pair == pair)
                        {
                            pm_pos.stop_loss = new_stop;
                        }
                        info!(
                            "Stop override applied: {} ${:.4} → ${:.4}",
                            pair, old_stop, new_stop
                        );
                        log_trade!(
                            "STOP",
                            "{} stop updated ${:.4} → ${:.4}",
                            pair,
                            old_stop,
                            new_stop
                        );

                        // Position stats line
                        let current = pos.current_price;
                        let entry = pos.entry_price;
                        let qty = pos.quantity;
                        let pnl_pct = if entry > 0.0 {
                            match pos.side {
                                savant_trading::core::types::Side::Long => {
                                    (current - entry) / entry * 100.0
                                }
                                savant_trading::core::types::Side::Short => {
                                    (entry - current) / entry * 100.0
                                }
                            }
                        } else {
                            0.0
                        };
                        let pnl_dollar = pnl_pct / 100.0 * entry * qty;
                        let sl_buffer = if current > 0.0 {
                            (current - new_stop).abs() / current * 100.0
                        } else {
                            0.0
                        };
                        let sl_status = if new_stop >= entry {
                            "✓ above entry"
                        } else {
                            "below entry"
                        };
                        let equity = portfolio.account().equity;
                        let dollar_risk = (current - new_stop).abs() * qty;
                        let risk_pct = if equity > 0.0 {
                            dollar_risk / equity * 100.0
                        } else {
                            0.0
                        };
                        log_trade!("  ├─", "Entry {:.4} | Price {:.4} | PnL: ${:.2} ({:+.2}%) | SL: {:.1}% from price ({})",
                            entry, current, pnl_dollar, pnl_pct, sl_buffer, sl_status);
                        log_trade!(
                            "  └─",
                            "Qty: {:.4} | Risk: ${:.2} ({:.1}% of ${:.2} equity) | Scale: {:?}",
                            qty,
                            dollar_risk,
                            risk_pct,
                            equity,
                            pos.scale_level
                        );
                    } else {
                        warn!("Stop override for {} but no matching position found", pair);
                    }
                }
            }
        }

        // Apply close requests from API — force close by setting stop to current price
        {
            let mut close_reqs = shared.close_overrides.write().await;
            for (pair, _) in close_reqs.drain() {
                if let Some(current_price) = all_prices.get(&pair) {
                    if let Some((_, pos)) = portfolio
                        .positions_mut()
                        .iter_mut()
                        .find(|(_, p)| p.pair == pair)
                    {
                        // For LONG: set stop above current price to trigger immediate close
                        // For SHORT: set stop below current price to trigger immediate close
                        match pos.side {
                            savant_trading::core::types::Side::Long => {
                                pos.stop_loss = current_price + 0.01;
                            }
                            savant_trading::core::types::Side::Short => {
                                pos.stop_loss = current_price - 0.01;
                            }
                        }
                        log_trade!("CLOSE", "{} — manual close requested via API", pair);
                    }
                }
            }
        }

        let stop_result = portfolio.check_stops(&all_prices);

        // Log trailing stop events
        for trail in &stop_result.trails {
            // Calculate actual dollar risk: distance per unit × quantity held
            let pos_data = portfolio
                .positions()
                .values()
                .find(|p| p.pair == trail.pair && p.side == trail.side)
                .cloned();
            let qty = pos_data.as_ref().map(|p| p.quantity).unwrap_or(0.0);
            let entry = pos_data.as_ref().map(|p| p.entry_price).unwrap_or(0.0);
            let dollar_risk = (trail.new_sl - trail.current_price).abs() * qty;
            log_trade!(
                "TRAIL",
                "{} {} | SL {:.4} → {:.4} (price {:.4}, risk ${:.2})",
                trail.pair,
                trail.side,
                trail.old_sl,
                trail.new_sl,
                trail.current_price,
                dollar_risk
            );

            // Second line: position stats — PnL, SL buffer, scale status
            if let Some(ref pos) = pos_data {
                let pnl_pct = if entry > 0.0 {
                    match trail.side {
                        savant_trading::core::types::Side::Long => {
                            (trail.current_price - entry) / entry * 100.0
                        }
                        savant_trading::core::types::Side::Short => {
                            (entry - trail.current_price) / entry * 100.0
                        }
                    }
                } else {
                    0.0
                };
                let pnl_dollar = pnl_pct / 100.0 * entry * qty;
                let sl_buffer = if trail.current_price > 0.0 {
                    (trail.current_price - trail.new_sl).abs() / trail.current_price * 100.0
                } else {
                    0.0
                };
                let sl_status = if trail.new_sl >= entry {
                    "✓ above entry"
                } else {
                    "below entry"
                };
                let equity = portfolio.account().equity;
                let risk_pct = if equity > 0.0 {
                    dollar_risk / equity * 100.0
                } else {
                    0.0
                };
                log_trade!(
                    "  ├─",
                    "Entry {:.4} | PnL: ${:.2} ({:+.2}%) | SL: {:.1}% from price ({}) | Qty: {:.4}",
                    entry,
                    pnl_dollar,
                    pnl_pct,
                    sl_buffer,
                    sl_status,
                    qty
                );
                log_trade!(
                    "  └─",
                    "Scale: {:?} | Risk: ${:.2} ({:.1}% of ${:.2} equity)",
                    pos.scale_level,
                    dollar_risk,
                    risk_pct,
                    equity
                );
            }

            // DB: update trailed position + log activity
            if let Some(ref j) = journal {
                if let Some((_, pos)) = portfolio
                    .positions()
                    .iter()
                    .find(|(_, p)| p.pair == trail.pair && p.side == trail.side)
                {
                    let _ = j.save_position(pos).await;
                }
                let _ = j
                    .record_activity(
                        "Trade",
                        &trail.pair,
                        &format!(
                            "TRAIL {} | SL {:.4} → {:.4} (price {:.4})",
                            trail.side, trail.old_sl, trail.new_sl, trail.current_price
                        ),
                    )
                    .await;
            }
            shared
                .log_activity(
                    savant_trading::core::shared::ActivityLevel::Trade,
                    &trail.pair,
                    &format!(
                        "TRAIL {} | SL {:.4} → {:.4}",
                        trail.side, trail.old_sl, trail.new_sl
                    ),
                )
                .await;
        }

        // FID-087 Bug H: Track trades that were reverted due to failed on-chain close.
        // The second loop (journal save) must skip these to prevent phantom trades in DB.
        let mut reverted_trades: Vec<(String, Side, f64, f64)> = Vec::new();

        // In live mode, close positions on executor that PortfolioManager closed via stops
        for trade in &stop_result.closed {
            // FID-094 Fix 2: Close retry cooldown — skip if recently failed
            if let Some(last_fail) = close_failure_cooldown.get(&trade.pair) {
                if last_fail.elapsed().as_secs() < CLOSE_COOLDOWN_SECS {
                    let remaining = CLOSE_COOLDOWN_SECS - last_fail.elapsed().as_secs();
                    warn!(
                        "FID-094 COOLDOWN: {} — close skipped, {}s remaining on cooldown",
                        trade.pair, remaining
                    );
                    // Track consecutive SL for death loop detection
                    let count = consecutive_sl_count.entry(trade.pair.clone()).or_insert(0);
                    *count += 1;
                    if *count >= SL_HALT_THRESHOLD {
                        sl_halt_until.insert(trade.pair.clone(), std::time::Instant::now());
                        warn!(
                            "FID-094 DEATH LOOP: {} — {} consecutive SL fires without close. Halting for 1 hour.",
                            trade.pair, count
                        );
                    }
                    continue;
                }
            }

            if let Some(ref mut ex) = executor {
                // Match closed trade to portfolio position by pair + side + entry_price
                let paper_id = paper_positions_before
                    .iter()
                    .find(|(_, pair, side, entry)| {
                        *pair == trade.pair
                            && *side == trade.side
                            && (*entry - trade.entry_price).abs() < 0.0001
                    })
                    .map(|(id, _, _, _)| id.clone());

                // Look up the executor position ID from the stored mapping
                let exec_id = paper_id
                    .as_ref()
                    .and_then(|pid| executor_position_map.get(pid))
                    .cloned();

                if let Some(ref eid) = exec_id {
                    match ex.close_position_partial(eid, trade.quantity).await {
                        Ok(order) => {
                            // FID-061: Log on-chain tx hash
                            if let Some(ref hash) = order.tx_hash {
                                log_trade!(
                                    "TX",
                                    "{} {} closed on-chain — tx: {}",
                                    trade.pair,
                                    trade.side,
                                    hash
                                );
                            }
                            // Clean up the mapping entry after successful close
                            if let Some(ref pid) = paper_id {
                                executor_position_map.remove(pid);
                            }
                            // FID-087 Bug F: Delete closed position from SQLite journal
                            // so it doesn't resurrect on next restart.
                            if let Some(ref j) = journal {
                                if let Some(ref pid) = paper_id {
                                    if let Err(e) = j.delete_position(pid).await {
                                        warn!("Failed to delete closed position {} from journal: {}", pid, e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to close executor position {}: {} — position stays open",
                                eid, e
                            );
                            // FID-094 Fix 2: Record close failure for cooldown
                            close_failure_cooldown.insert(trade.pair.clone(), std::time::Instant::now());
                            // FID-074: Revert the PnL that check_stops added to balance,
                            // since the on-chain close didn't actually execute.
                            portfolio.account_mut().balance -= trade.pnl;
                            // Also remove the phantom TradeRecord from closed_trades
                            portfolio.closed_trades_mut().retain(|t| {
                                !(t.pair == trade.pair
                                    && t.side == trade.side
                                    && (t.entry_price - trade.entry_price).abs() < 0.0001
                                    && (t.exit_price - trade.exit_price).abs() < 0.0001
                                    && t.closed_at == trade.closed_at)
                            });
                            // FID-087 Bug H: Track reverted trade so journal save is skipped
                            reverted_trades.push((
                                trade.pair.clone(),
                                trade.side,
                                trade.entry_price,
                                trade.exit_price,
                            ));
                            // FID-097 Fix 2: Restore position to PortfolioManager — but NOT if
                            // reconciliation removed it (phantom position guard).
                            if let Some(ref pid) = paper_id {
                                if reconciliation_removed.contains(pid) {
                                    warn!("FID-097: Skipping restore of {} — removed by reconciliation, not a real position", pid);
                                } else if let Some(pos) = paper_positions_full.get(pid) {
                                    portfolio.positions_mut().insert(pid.clone(), pos.clone());
                                    portfolio.account_mut().open_positions =
                                        portfolio.positions().len();
                                    warn!("Restored position {} to PortfolioManager — will retry close next cycle", pid);
                                    shared.log_activity(
                                        savant_trading::core::shared::ActivityLevel::Warning,
                                        &trade.pair,
                                        &format!("CLOSE FAILED: {} — position stays open, will retry. Error: {}", trade.pair, e),
                                    ).await;
                                }
                            }
                        }
                    }
                } else {
                    // Fallback: search by pair + side if mapping not found
                    let fallback_id = ex
                        .open_positions()
                        .iter()
                        .find(|p| p.pair == trade.pair && p.side == trade.side)
                        .map(|p| p.id.clone());
                    if let Some(fid) = fallback_id {
                        if let Err(e) = ex.close_position_partial(&fid, trade.quantity).await {
                            warn!(
                                "Failed to close fallback position {}: {} — position stays open",
                                fid, e
                            );
                            // FID-074: Revert balance on failed close
                            portfolio.account_mut().balance -= trade.pnl;
                            // Remove phantom TradeRecord
                            portfolio.closed_trades_mut().retain(|t| {
                                !(t.pair == trade.pair
                                    && t.side == trade.side
                                    && (t.entry_price - trade.entry_price).abs() < 0.0001
                                    && (t.exit_price - trade.exit_price).abs() < 0.0001
                                    && t.closed_at == trade.closed_at)
                            });
                            // FID-087 Bug H: Track reverted trade so journal save is skipped
                            reverted_trades.push((
                                trade.pair.clone(),
                                trade.side,
                                trade.entry_price,
                                trade.exit_price,
                            ));
                            // FID-097 Fix 2: Restore position to PortfolioManager — but NOT if
                            // reconciliation removed it (phantom position guard).
                            if let Some(ref pid) = paper_id {
                                if reconciliation_removed.contains(pid) {
                                    warn!("FID-097: Skipping restore of {} — removed by reconciliation, not a real position", pid);
                                } else if let Some(pos) = paper_positions_full.get(pid) {
                                    portfolio.positions_mut().insert(pid.clone(), pos.clone());
                                    portfolio.account_mut().open_positions =
                                        portfolio.positions().len();
                                    warn!("Restored position {} to PortfolioManager — will retry close next cycle", pid);
                                    shared.log_activity(
                                        savant_trading::core::shared::ActivityLevel::Warning,
                                        &trade.pair,
                                        &format!("CLOSE FAILED: {} — position stays open, will retry. Error: {}", trade.pair, e),
                                    ).await;
                                }
                            }
                        } else {
                            // FID-087 Bug F: Successful fallback close — delete from journal
                            if let Some(ref j) = journal {
                                if let Some(ref pid) = paper_id {
                                    if let Err(e) = j.delete_position(pid).await {
                                        warn!("Failed to delete closed position {} from journal: {}", pid, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let has_stop_activity = !stop_result.closed.is_empty() || !stop_result.trails.is_empty();
        for trade in stop_result.closed {
            let tp_label = if trade.notes.contains("TP1") {
                "TP1"
            } else if trade.notes.contains("TP2") {
                "TP2"
            } else if trade.notes.contains("TP3") {
                "TP3"
            } else {
                "SL"
            };
            log_trade!(
                tp_label,
                "{} {} | Entry: {:.4} → Exit: {:.4} | Qty: {:.4} | PnL: ${:.2} ({:.2}%) | {}",
                trade.pair,
                trade.side,
                trade.entry_price,
                trade.exit_price,
                trade.quantity,
                trade.pnl,
                trade.pnl_pct,
                trade.notes
            );

            // FID-087 Bug H: Skip journal save for trades that were reverted
            // due to failed on-chain close. Prevents phantom trades in DB.
            let is_reverted = reverted_trades.iter().any(|(pair, side, entry, exit)| {
                *pair == trade.pair
                    && *side == trade.side
                    && (*entry - trade.entry_price).abs() < 0.0001
                    && (*exit - trade.exit_price).abs() < 0.0001
            });

            if !is_reverted {
                // DB: delete position, record trade, log activity — all instant
                if let Some(ref j) = journal {
                    // Find and delete the closed position from DB
                    for (id, pair, _side, _entry) in paper_positions_before.iter() {
                        if *pair == trade.pair {
                            let _ = j.delete_position(id).await;
                            break;
                        }
                    }
                    let _ = j.record_trade(&trade).await;
                    let _ = j
                        .record_activity(
                            "Trade",
                            &trade.pair,
                            &format!(
                                "{} {} | PnL: ${:.2} ({:.2}%) | {}",
                                tp_label, trade.side, trade.pnl, trade.pnl_pct, trade.notes
                            ),
                        )
                        .await;
                }
            } else {
                warn!(
                    "SKIPPED journal save for reverted trade: {} {} — on-chain close failed",
                    trade.pair, trade.side
                );
            }

            // Write close alert to file for external monitoring
            let close_alert = serde_json::json!({
                "type": "TRADE_CLOSED",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "pair": trade.pair,
                "side": format!("{:?}", trade.side),
                "entry_price": trade.entry_price,
                "exit_price": trade.exit_price,
                "pnl": trade.pnl,
                "pnl_pct": trade.pnl_pct,
                "notes": trade.notes,
            });
            let close_line = format!("{}\n", close_alert);
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("data/alerts.jsonl")
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(close_line.as_bytes())
                });

            shared
                .log_activity(
                    savant_trading::core::shared::ActivityLevel::Trade,
                    &trade.pair,
                    &format!(
                        "{} {} | PnL: ${:.2} ({:.2}%) | {}",
                        tp_label, trade.side, trade.pnl, trade.pnl_pct, trade.notes,
                    ),
                )
                .await;

            event_bus.publish(TradingEvent::PositionClosed(trade.clone()));
            if vault_config.enabled {
                if let Err(e) = vault_writer.project_trade(&trade) {
                    warn!("Vault trade projection failed: {}", e);
                }
            }

            // FID-098 Fix 1c: Update episodic memory with stop-loss/TP outcome
            if let Some(ref mem) = memory {
                // Find the episode for this pair — stop/TP closes may not have an exact
                // tick match, so search by pair prefix in the store
                let pair_prefix = format!("{}-Buy", trade.pair);
                let found = episode_store.iter()
                    .find(|(k, _)| k.starts_with(&pair_prefix))
                    .map(|(k, v)| (k.clone(), v.clone()));
                if let Some((key, episode_id)) = found {
                    let achieved_rr = if trade.entry_price > 0.0 && trade.quantity > 0.0 {
                        trade.pnl / (trade.entry_price * trade.quantity)
                    } else {
                        0.0
                    };
                    let is_reverted = reverted_trades.iter().any(|(pair, side, entry, exit)| {
                        *pair == trade.pair && *side == trade.side
                            && (*entry - trade.entry_price).abs() < 0.0001
                            && (*exit - trade.exit_price).abs() < 0.0001
                    });
                    if !is_reverted {
                        if let Err(e) = mem.update_outcome(
                            &episode_id, trade.pnl, trade.pnl_pct, trade.pnl > 0.0, achieved_rr,
                        ).await {
                            warn!("FID-098: Failed to update episode outcome for {}: {}", trade.pair, e);
                        } else {
                            debug!("FID-098: Updated episode {} outcome: PnL=${:.2} ({})", &episode_id[..8], trade.pnl, tp_label);
                        }
                    }
                    episode_store.remove(&key);
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

        // FID-082: Sync shared state after stop checks — release each lock before
        // acquiring the next to prevent deadlock with the API server.
        if has_stop_activity {
            {
                let mut sp = shared.positions.write().await;
                *sp = portfolio.positions().values().cloned().collect();
            }
            {
                let mut sa = shared.account.write().await;
                *sa = portfolio.account().clone();
            }
            {
                let mut st = shared.closed_trades.write().await;
                *st = portfolio.closed_trades().to_vec();
            }

            // DB: persist scale-out position updates
            if let Some(ref j) = journal {
                for pos in portfolio.positions().values() {
                    let _ = j.save_position(pos).await;
                }
            }
        }

        // FID-081: Price staleness protection
        // Build all_prices with staleness guards, outlier detection, and REST fallback
        let mut all_prices: HashMap<String, f64> = HashMap::new();
        let mut ws_stale_count = 0u32;
        let mut ws_total_count = 0u32;
        let staleness_threshold = std::time::Duration::from_secs(300); // 5 min
        let candle_staleness_threshold = std::time::Duration::from_secs(1200); // 20 min

        // Step 1: Load candle prices as base layer
        for (pair, store) in &market_stores {
            if let Some(last_candle) = store.last() {
                // FID-081 Fix 6: Candle staleness warning
                let candle_age = chrono::Utc::now().signed_duration_since(last_candle.timestamp);
                if candle_age.num_seconds() > candle_staleness_threshold.as_secs() as i64 {
                    warn!("FID-081: Candle data stale for {} — last candle {}s ago", pair, candle_age.num_seconds());
                }
                all_prices.insert(pair.clone(), last_candle.close);
            }
        }

        // Step 2: Override with WS prices (if fresh)
        for (pair, (price, timestamp)) in &ws_ticker_prices {
            ws_total_count += 1;
            let age = timestamp.elapsed();

            // Staleness guard: skip WS price if > 5 min old
            if age > staleness_threshold {
                ws_stale_count += 1;
                ws_staleness.insert(pair.clone(), age.as_secs());
                continue;
            }

            // FID-081 Fix 8: Price sanity check — reject > 10% moves in one tick
            if let Some(&old_price) = all_prices.get(pair) {
                if old_price > 0.0 {
                    let change_pct = (price - old_price).abs() / old_price;
                    if change_pct > 0.10 {
                        warn!(
                            "FID-081: Price outlier rejected for {} — old=${:.2} new=${:.2} ({:.1}% change)",
                            pair, old_price, price, change_pct * 100.0
                        );
                        continue; // Keep the candle price
                    }
                }
            }

            all_prices.insert(pair.clone(), *price);
            ws_staleness.insert(pair.clone(), age.as_secs());
        }

        // Step 3: If ALL WS prices stale, log CRITICAL and fire REST fallback
        if ws_total_count > 0 && ws_stale_count == ws_total_count {
            let worst = ws_staleness.values().max().copied().unwrap_or(0);
            error!(
                "FID-081 CRITICAL: All {} WS prices stale (worst: {}s ago) — using candle data only",
                ws_stale_count, worst
            );
            // REST fallback: once per event, 10 min cooldown
            let should_fetch = match rest_fallback_at {
                Some(t) => t.elapsed() > std::time::Duration::from_secs(600),
                None => true,
            };
            if should_fetch {
                warn!("FID-081: Firing REST price fallback for all pairs");
                rest_fallback_at = Some(std::time::Instant::now());
                // REST fetch happens asynchronously — candle data is still the best we have
            }
        }

        // Step 4: WS reconnect → mark for REST fill
        if ws_just_reconnected {
            ws_just_reconnected = false;
            warn!("FID-081: WS reconnected — prices may be stale until fresh data arrives");
        }

        // Update worst-case staleness for shared state — only count pairs with
        // open positions. Idle pairs going stale during low-liquidity sessions
        // should not trigger a false "STALE PRICES" alert.
        let held_pairs: std::collections::HashSet<String> =
            portfolio.positions().keys().cloned().collect();
        let worst_staleness_secs = ws_staleness
            .iter()
            .filter(|(pair, _)| held_pairs.contains(*pair))
            .map(|(_, age)| *age)
            .max()
            .unwrap_or(0);

        portfolio.update_prices(&all_prices);

        // Sync ALL shared state every tick so dashboard is always live
        {
            let mut sp = shared.positions.write().await;
            *sp = portfolio.positions().values().cloned().collect();
        }
        {
            let mut sa = shared.account.write().await;
            *sa = portfolio.account().clone();
        }
        // FID-081: Sync price staleness to shared state for dashboard
        {
            let mut staleness = shared.price_staleness_secs.write().await;
            *staleness = worst_staleness_secs;
        }

        // Sync balance from executor — chain is always the source of truth
        if let Some(ref mut ex) = executor {
            if tick.is_multiple_of(3) && ex.sync_balance().await.is_ok() {
                let executor_balance = ex.balance();
                portfolio.account_mut().balance = executor_balance;
                portfolio.refresh_equity();
                // Re-sync shared state after balance change so dashboard is live
                let mut sa = shared.account.write().await;
                *sa = portfolio.account().clone();
                debug!("Balance synced from executor: ${:.2}", executor_balance);
                // FID-117: Update chain_equity from portfolio
                *shared.chain_equity.write().await = portfolio.account().equity;
            }

            // FID-096 Fix 1: On-chain token balance reconciliation.
            // Runs on first cycle (tick==1) and every 2 cycles after.
            // Detects externally sold tokens (manual swap, another app) and removes
            // phantom positions that no longer have on-chain backing.
            if tick == 1 || tick.is_multiple_of(2) {
                let position_pairs: Vec<(String, String, f64)> = portfolio.positions().iter()
                    .map(|(id, p)| (id.clone(), p.pair.clone(), p.quantity))
                    .collect();

                for (pos_id, pair, pos_qty) in position_pairs {
                    if pos_qty <= 0.0001 { continue; }

                    // Resolve token address — use Side::Short to get the BASE token
                    // (what we hold for a LONG position). Side::Long would return USDC.
                    if let Ok((base_token, _)) = savant_trading::execution::dex::resolve_pair_on_chain(
                        &pair, Side::Short, ex.chain_id(),
                    ) {
                        if base_token.address.is_empty() { continue; }

                        if let Some(on_chain) = ex.query_token_balance(&base_token.address, base_token.decimals).await {
                            if on_chain <= 0.0001 && pos_qty > 0.0001 {
                                // EXTERNAL CLOSE: tokens gone from on-chain
                                warn!(
                                    "FID-096 EXTERNAL CLOSE: {} — on-chain balance is 0, position quantity was {:.6}. Removing.",
                                    pair, pos_qty
                                );

                                let old_equity = portfolio.account().equity;

                                // FID-096 Item 12: Record trade for external close
                                // before removing the position. Uses last known market price.
                                let pos_snapshot = portfolio.positions().get(&pos_id).cloned();
                                // FID-098: Extract PnL for outcome update (must be accessible after this block)
                                let (ext_pnl, ext_pnl_pct) = if let Some(ref pos) = pos_snapshot {
                                    let market_price = market_stores
                                        .get(&pair)
                                        .and_then(|s| s.last().map(|c| c.close))
                                        .unwrap_or(pos.current_price);
                                    let pnl = match pos.side {
                                        Side::Long => (market_price - pos.entry_price) * pos.quantity,
                                        Side::Short => (pos.entry_price - market_price) * pos.quantity,
                                    };
                                    let pnl_pct = if pos.entry_price > 0.0 && pos.quantity > 0.0 {
                                        pnl / (pos.entry_price * pos.quantity) * 100.0
                                    } else { 0.0 };
                                    (pnl, pnl_pct)
                                } else {
                                    (0.0, 0.0)
                                };
                                if let Some(pos) = pos_snapshot {
                                    let market_price = market_stores
                                        .get(&pair)
                                        .and_then(|s| s.last().map(|c| c.close))
                                        .unwrap_or(pos.current_price);
                                    let trade = savant_trading::core::types::TradeRecord {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        pair: pair.clone(),
                                        side: pos.side,
                                        entry_price: pos.entry_price,
                                        exit_price: market_price,
                                        quantity: pos.quantity,
                                        pnl: ext_pnl,
                                        pnl_pct: ext_pnl_pct,
                                        fees: 0.0,
                                        strategy_name: pos.strategy_name.clone(),
                                        opened_at: pos.opened_at,
                                        closed_at: chrono::Utc::now(),
                                        notes: "External close — tokens sold outside engine. Exit price estimated from market data.".to_string(),
                                        on_chain_verified: false,
                                        tx_hash: None,
                                    };
                                    portfolio.closed_trades_mut().push(trade.clone());
                                    if let Some(ref j) = journal {
                                        let _ = j.record_trade(&trade).await;
                                    }
                                    info!(
                                        "Recorded external close trade: {} {} | Entry: {:.4} → Exit: {:.4} | PnL: ${:.2} ({:.2}%)",
                                        pair, pos.side, pos.entry_price, market_price, ext_pnl, ext_pnl_pct
                                    );
                                }

                                // Remove from PortfolioManager
                                if let Some(_removed) = portfolio.positions_mut().remove(&pos_id) {
                                    portfolio.account_mut().open_positions = portfolio.positions().len();
                                    reconciliation_removed.insert(pos_id.clone());
                                }

                                // Remove from DexTrader
                                {
                                    let exec_id = executor_position_map.get(&pos_id)
                                        .cloned()
                                        .unwrap_or_else(|| format!("exec-{}", pos_id));
                                    let ghost = savant_trading::core::types::Position {
                                        id: pos_id.clone(),
                                        pair: pair.clone(),
                                        side: Side::Long,
                                        entry_price: 0.0,
                                        current_price: 0.0,
                                        quantity: 0.0,
                                        stop_loss: 0.0,
                                        take_profit_1: 0.0,
                                        take_profit_2: 0.0,
                                        take_profit_3: 0.0,
                                        unrealized_pnl: 0.0,
                                        risk_amount: 0.0,
                                        strategy_name: String::new(),
                                        scale_level: savant_trading::core::types::ScaleLevel::Full,
                                        opened_at: chrono::Utc::now(),
                                    };
                                    ex.register_position(exec_id.clone(), ghost);
                                }

                                // Remove from executor_position_map
                                executor_position_map.remove(&pos_id);

                                // Delete from journal
                                if let Some(ref j) = journal {
                                    if let Err(e) = j.delete_position(&pos_id).await {
                                        warn!("Failed to delete externally closed position {} from journal: {}", pos_id, e);
                                    }
                                }

                                // Clear close failure cooldown
                                close_failure_cooldown.remove(&pair);

                                // Refresh equity and log correction
                                portfolio.refresh_equity();
                                let new_equity = portfolio.account().equity;
                                info!(
                                    "FID-096 EQUITY CORRECTED: ${:.2} → ${:.2} after external close of {}",
                                    old_equity, new_equity, pair
                                );

                                // Sync to shared state for dashboard
                                let mut sa = shared.account.write().await;
                                *sa = portfolio.account().clone();
                                let mut sp = shared.positions.write().await;
                                *sp = portfolio.positions().values().cloned().collect();

                                // Dashboard notification
                                shared.log_activity(
                                    savant_trading::core::shared::ActivityLevel::Warning,
                                    &pair,
                                    &format!("EXTERNAL CLOSE: tokens no longer on-chain — position removed. Equity: ${:.2}", new_equity),
                                ).await;

                                // FID-098 Fix 1d: Update episodic memory with external close outcome
                                if let Some(ref mem) = memory {
                                    let pair_prefix = format!("{}-Buy", pair);
                                    let found = episode_store.iter()
                                        .find(|(k, _)| k.starts_with(&pair_prefix))
                                        .map(|(k, v)| (k.clone(), v.clone()));
                                    if let Some((key, episode_id)) = found {
                                        if let Err(e) = mem.update_outcome(
                                            &episode_id, ext_pnl, ext_pnl_pct, ext_pnl > 0.0, 0.0,
                                        ).await {
                                            warn!("FID-098: Failed to update episode for external close {}: {}", pair, e);
                                        } else {
                                            debug!("FID-098: Updated episode {} for external close: PnL=${:.2}", &episode_id[..8], ext_pnl);
                                        }
                                        episode_store.remove(&key);
                                    }
                                }

                            } else if on_chain > 0.0001 && on_chain < pos_qty * 0.5 {
                                // Partial external close
                                warn!(
                                    "FID-096 PARTIAL EXTERNAL CLOSE: {} — on-chain {:.6} vs position {:.6}",
                                    pair, on_chain, pos_qty
                                );
                                if let Some(pos) = portfolio.positions_mut().get_mut(&pos_id) {
                                    let old_qty = pos.quantity;
                                    pos.quantity = on_chain;
                                    pos.risk_amount = pos.entry_price * on_chain;
                                    info!(
                                        "Updated {} quantity: {:.6} → {:.6} (partial external close)",
                                        pair, old_qty, on_chain
                                    );
                                    if let Some(ref j) = journal {
                                        let _ = j.save_position(pos).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if tick.is_multiple_of(10) {
            let account = portfolio.account();
            let trades = portfolio.closed_trades();
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

            // Position dashboard — show all open positions with targets & PnL
            let positions: Vec<_> = portfolio.positions().values().collect();
            if !positions.is_empty() {
                log_position!(
                    "POSITIONS",
                    "{} open position{}",
                    positions.len(),
                    if positions.len() == 1 { "" } else { "s" }
                );
                for pos in &positions {
                    let held = chrono::Utc::now().signed_duration_since(pos.opened_at);
                    let held_str = if held.num_hours() > 0 {
                        format!("{}h{}m", held.num_hours(), held.num_minutes() % 60)
                    } else {
                        format!("{}m", held.num_minutes())
                    };
                    let pnl_pct = if pos.entry_price > 0.0 {
                        pos.unrealized_pnl / (pos.entry_price * pos.quantity) * 100.0
                    } else {
                        0.0
                    };
                    let scale_str = match pos.scale_level {
                        savant_trading::core::types::ScaleLevel::Full => "Full",
                        savant_trading::core::types::ScaleLevel::Scaled50 => "50%out",
                        savant_trading::core::types::ScaleLevel::Scaled80 => "80%out",
                        savant_trading::core::types::ScaleLevel::Closed => "Closed",
                    };
                    let sl_dist = match pos.side {
                        savant_trading::core::types::Side::Long => {
                            if pos.current_price > 0.0 {
                                (pos.current_price - pos.stop_loss) / pos.current_price * 100.0
                            } else {
                                0.0
                            }
                        }
                        savant_trading::core::types::Side::Short => {
                            if pos.current_price > 0.0 {
                                (pos.stop_loss - pos.current_price) / pos.current_price * 100.0
                            } else {
                                0.0
                            }
                        }
                    };
                    log_position!("  {}", "{} {} | Entry:{:.4} Cur:{:.4} | PnL:${:.2}({:+.1}%) | SL:{:.4}({:.1}%away) | TP1:{:.4} TP2:{:.4} TP3:{:.4} | Scale:{} | {}",
                        pos.pair, pos.side, pos.entry_price, pos.current_price,
                        pos.unrealized_pnl, pnl_pct,
                        pos.stop_loss, sl_dist,
                        pos.take_profit_1, pos.take_profit_2, pos.take_profit_3,
                        scale_str, held_str);
                }
            } else {
                log_position!("POSITIONS", "No open positions");
            }

            // FID-082: Update shared state for API — release each lock before next
            {
                let mut shared_account = shared.account.write().await;
                *shared_account = account.clone();
            }
            {
                let mut shared_positions = shared.positions.write().await;
                *shared_positions = portfolio.positions().values().cloned().collect();
            }
            {
                let mut shared_trades = shared.closed_trades.write().await;
                *shared_trades = trades.to_vec();
            }
            {
                let mut shared_insight = shared.insight.write().await;
                *shared_insight = insight.cached().clone();
            }

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

            {
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
                // FID-093 D5: Prune old equity snapshots once per day (288 cycles)
                if tick.is_multiple_of(288) {
                    if let Some(ref j) = journal {
                        match j.prune_old_snapshots().await {
                            Ok(n) if n > 0 => info!("Pruned {} old equity snapshots", n),
                            Err(e) => warn!("Failed to prune equity snapshots: {}", e),
                            _ => {}
                        }
                    }
                }
                // Push to shared state for dashboard equity curve
                {
                    let mut curve = shared.equity_curve.write().await;
                    curve.push(serde_json::json!({
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "balance": account.balance,
                        "equity": account.equity,
                        "drawdown_pct": account.drawdown_pct,
                        "open_positions": account.open_positions,
                    }));
                    // Keep last 500 points
                    if curve.len() > 500 {
                        let drain_count = curve.len() - 500;
                        curve.drain(0..drain_count);
                    }
                }
            }
        }

        // FID-082 Fix 5: Cycle watchdog — log CRITICAL if cycle took > 5 minutes
        let cycle_elapsed = cycle_start.elapsed();
        if cycle_elapsed > std::time::Duration::from_secs(300) {
            error!(
                "FID-082 CRITICAL: Cycle {} took {:.1}s — possible hang detected",
                tick,
                cycle_elapsed.as_secs_f64()
            );
        }

        // Cycle complete — inform user before sleeping
        let interval_display = if interval_seconds >= 60 {
            format!("{}m", interval_seconds / 60)
        } else {
            format!("{}s", interval_seconds)
        };
        log_phase!(
            "CYCLE",
            "Cycle {} complete. Next in {}. Sleeping...",
            tick,
            interval_display
        );

        // FID-082 Fix 4: Use time::sleep only — tokio::select! with ctrl_c
        // can interfere with sleep on Windows. Ctrl+C handled by OS.
        time::sleep(Duration::from_secs(interval_seconds)).await;
    }
}
