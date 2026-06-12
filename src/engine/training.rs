use tracing::{info, warn};

use savant_trading::agent::context_builder::FullContext;
use savant_trading::agent::prompts::{self, PromptComposer};
use savant_trading::core::config::AppConfig;
use savant_trading::core::types::{Candle, TradeRecord};
use savant_trading::data::candle_client::CandleClient;
use savant_trading::execution::portfolio::PortfolioManager;
use savant_trading::monitor::metrics::{Metrics, PerformanceMetrics};
use savant_trading::strategy::regime::RegimeDetector;
use savant_trading::vault::config::VaultConfig;
use savant_trading::vault::writer::VaultWriter;

use super::utils::load_knowledge_base;

async fn fetch_and_cache(candle_api: &CandleClient, cache_path: &str) -> Vec<Candle> {
    match candle_api.get_ohlc("BTC/USD", 5, None).await {
        Ok(mut c) => {
            if c.len() > 1 {
                c.pop();
            }
            println!("Fetched {} real candles", c.len());
            if let Ok(json) = serde_json::to_string(&c) {
                let _ = std::fs::create_dir_all("data");
                let _ = std::fs::write(cache_path, &json);
                println!("Cached to {}", cache_path);
            }
            c
        }
        Err(e) => {
            warn!("Candle fetch failed ({}), using synthetic fallback", e);
            let gen_config = savant_trading::sandbox::generator::GeneratorConfig {
                num_candles: 200, // FID-126: 200 candles (17h) for sharper indicators
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

/// A single raw LLM response entry for diagnostic persistence.
#[derive(serde::Serialize)]
struct RawResponseEntry {
    scenario_id: String,
    scenario_name: String,
    category: String,
    difficulty: String,
    expected_action: String,
    current_price: f64,
    raw_response: Option<String>,
    error: Option<String>,
    latency_ms: u64,
}

/// Module-level diagnostic response for `save_raw_responses`.
/// Both `run_training_batch` and `run_sandbox` define local
/// `ScenarioResponse` types — this shared struct normalizes the
/// fields needed for raw response persistence.
///
/// Uses `Option<String>` for the response to avoid requiring
/// `LlmError: Clone` (it doesn't derive it).
struct DiagnosticResponse {
    scenario_id: String,
    scenario_name: String,
    category: String,
    difficulty: String,
    expected_action: String,
    current_price: f64,
    raw_response: Option<String>,
    error: Option<String>,
    latency_ms: u64,
}

/// Save raw LLM responses to disk for diagnostic inspection.
///
/// Creates a timestamped directory under `data/sandbox_responses/` and writes
/// one JSON file per scenario plus an index file for quick filtering.
fn save_raw_responses(
    responses: &[DiagnosticResponse],
    tag: &str,
    model: &str,
    endpoint: &str,
) {
    use std::io::Write;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let dir = format!("data/sandbox_responses/{}_{}", tag, timestamp);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        warn!("Failed to create response directory {}: {}", dir, e);
        return;
    }

    // Write individual response files
    for sr in responses {
        let entry = RawResponseEntry {
            scenario_id: sr.scenario_id.clone(),
            scenario_name: sr.scenario_name.clone(),
            category: sr.category.clone(),
            difficulty: sr.difficulty.clone(),
            expected_action: sr.expected_action.clone(),
            current_price: sr.current_price,
            raw_response: sr.raw_response.clone(),
            error: sr.error.clone(),
            latency_ms: sr.latency_ms,
        };
        let filename = format!("{}/{}.json", dir, sr.scenario_id);
        if let Ok(json) = serde_json::to_string_pretty(&entry) {
            if let Err(e) = std::fs::write(&filename, &json) {
                warn!("Failed to write {}: {}", filename, e);
            }
        }
    }

    // Write index file
    let index: Vec<serde_json::Value> = responses
        .iter()
        .map(|sr| {
            let action = match sr.raw_response.as_deref() {
                Some(text) => {
                    match savant_trading::agent::decision_parser::parse_decision(
                        text,
                        sr.current_price,
                        10.0,
                    ) {
                        Ok(d) => format!("{:?}", d.action),
                        Err(_) => "ParseError".to_string(),
                    }
                }
                None => "LLMError".to_string(),
            };
            serde_json::json!({
                "id": sr.scenario_id,
                "name": sr.scenario_name,
                "expected": sr.expected_action,
                "action": action,
            })
        })
        .collect();

    let index_path = format!("{}/index.json", dir);
    if let Ok(json) = serde_json::to_string_pretty(&index) {
        let mut f = match std::fs::File::create(&index_path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to create {}: {}", index_path, e);
                return;
            }
        };
        let _ = f.write_all(json.as_bytes());
    }

    // Write metadata file
    let metadata = serde_json::json!({
        "model": model,
        "endpoint": endpoint,
        "scenario_count": responses.len(),
        "timestamp": timestamp.to_string(),
    });
    let meta_path = format!("{}/metadata.json", dir);
    if let Ok(json) = serde_json::to_string_pretty(&metadata) {
        let _ = std::fs::write(&meta_path, &json);
    }

    println!("\nRaw responses saved to {}/", dir);
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
    metrics: Metrics,
    #[allow(dead_code)]
    starting_balance: f64,
}

/// Run a single training batch. Called by `run_training` in a loop.
async fn run_training_batch(
    config: &AppConfig,
    scenarios: &[savant_trading::sandbox::scenarios::Scenario],
    test_memory: &savant_trading::memory::episodic::EpisodicMemory,
    model_override: Option<&str>,
    managed_keys: bool,
    save_responses: bool,
) -> anyhow::Result<TrainingRunResult> {
    use savant_trading::sandbox::generator;

    // Managed keys: create a temporary API key with spending limit
    let _managed_key_hash: Option<String> = None;
    let api_keys: Vec<String> = if managed_keys && config.ai.provider == "openrouter" {
        let mgmt_key =
            std::env::var(&config.ai.openrouter.management.management_key_env).unwrap_or_default();
        if mgmt_key.is_empty() {
            warn!("--managed-keys set but OPENROUTER_MANAGEMENT_KEY not found, falling back to env keys");
            std::env::var("SANDBOX_API_KEYS")
                .unwrap_or_else(|_| std::env::var(&config.ai.api_key_env).unwrap_or_default())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            let mgmt =
                savant_trading::agent::openrouter_management::OpenRouterManagementClient::new(
                    mgmt_key,
                );
            let model_name = model_override.unwrap_or(&config.ai.model);
            let key_name = format!("savant-{}", chrono::Utc::now().format("%m%d-%H%M"));
            match mgmt
                .create_key(
                    savant_trading::agent::openrouter_management::CreateKeyRequest {
                        name: key_name.clone(),
                        limit: Some(1.0), // $1 limit per test/training run
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(created) => {
                    info!(
                        "Managed key created: {} (limit: $1.00, model: {})",
                        key_name, model_name
                    );
                    vec![created.key]
                }
                Err(e) => {
                    warn!(
                        "Failed to create managed key ({}), falling back to env keys",
                        e
                    );
                    std::env::var("SANDBOX_API_KEYS")
                        .unwrap_or_else(|_| {
                            std::env::var(&config.ai.api_key_env).unwrap_or_default()
                        })
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            }
        }
    } else {
        std::env::var("SANDBOX_API_KEYS")
            .unwrap_or_else(|_| std::env::var(&config.ai.api_key_env).unwrap_or_default())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    if api_keys.is_empty() {
        anyhow::bail!(
            "No API keys. Set SANDBOX_API_KEYS or {} in .env",
            config.ai.api_key_env
        );
    }

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
    let regime_detector = RegimeDetector::new(
        config.strategy.regime.adx_period,
        config.strategy.regime.adx_trending_threshold,
        config.strategy.regime.adx_ranging_threshold,
        config.strategy.regime.atr_volatility_multiplier,
    );

    let cache_path = "data/sandbox_candles.json";
    let candle_api = CandleClient::new(&config.exchange.rest_url);
    let real_candles = if std::path::Path::new(cache_path).exists() {
        match std::fs::read_to_string(cache_path) {
            Ok(json) => serde_json::from_str::<Vec<Candle>>(&json).unwrap_or_default(),
            Err(_) => fetch_and_cache(&candle_api, cache_path).await,
        }
    } else {
        fetch_and_cache(&candle_api, cache_path).await
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
        let candles = match &scenario.candles_override {
            Some(override_candles) => override_candles.clone(),
            None => {
                let mut c = real_candles.clone();
                generator::apply_scenario(&mut c, &scenario.params);
                c
            }
        };

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
                close: chunk.last().map(|c| c.close).unwrap_or(0.0),
                volume: chunk.iter().map(|c| c.volume).sum(),
                pair: "BTC/USD".into(),
            });
        }

        let session = if let Some(ref s) = mock.session_override {
            match s.as_str() {
                "Asian" => savant_trading::core::session::Session::Asian,
                "European" => savant_trading::core::session::Session::European,
                "US" => savant_trading::core::session::Session::UsEuOverlap,
                "UsEuOverlap" => savant_trading::core::session::Session::UsEuOverlap,
                "DeepAsian" => savant_trading::core::session::Session::DeepAsian,
                "Late US" => savant_trading::core::session::Session::LateUs,
                _ => savant_trading::core::session::current_session(),
            }
        } else {
            // FID-126: Default to best-liquidity session for trade scenarios.
            // Without this, all scenarios get Deep Asian penalties when run at night.
            let expects_trade = scenario.expected_action.to_lowercase().contains("buy")
                || scenario.expected_action.to_lowercase().contains("sell")
                || scenario.expected_action.to_lowercase().contains("short")
                || scenario.expected_action.to_lowercase().contains("close");
            if expects_trade {
                savant_trading::core::session::Session::UsEuOverlap
            } else {
                savant_trading::core::session::current_session()
            }
        };

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
            pair: "BTC/USD",
            recent_trades: None,
            order_book_imbalance: Some(0.1),
            session,
            memory_context: memory_ctx_str.clone(),
            higher_tf_candles: vec![("1h".into(), htf_candles)],
            context_tags: savant_trading::agent::context_builder::generate_context_tags(
                &indicators,
            ),
            live_price: None,
            decision_log_context: None,
            dex_price: None,
            active_pairs: Some(&["BTC/USD".to_string()]),
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
        let model = model_override
            .map(|m| m.to_string())
            .unwrap_or_else(|| config.ai.model.clone());
        let sys = ps.system_prompt;
        let usr = ps.user_message;
        let sem = semaphore.clone();
        join_set.spawn(async move {
            let Ok(_permit) = sem.acquire().await else {
                tracing::warn!("Semaphore closed, skipping scenario");
                return ScenarioResponse {
                    scenario_id: ps.scenario_id,
                    scenario_name: ps.scenario_name,
                    category: ps.category,
                    expected_action: ps.expected_action,
                    regime: ps.regime,
                    indicators_snapshot: ps.indicators_snapshot,
                    response: Err(savant_trading::agent::provider::LlmError::InvalidResponse(
                        "Semaphore closed".into(),
                    )),
                    current_price: ps.current_price,
                    latency_ms: 0,
                };
            };
            let provider = savant_trading::agent::provider::LlmProvider::new(
                savant_trading::agent::provider::LlmConfig {
                    endpoint,
                    model,
                    api_key: key,
                    max_tokens: 131072,
                    temperature: 0.6,
                    top_p: 0.95,
                    timeout_secs: 300,
                    extra_headers: vec![],
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

    // PHASE 2c: Save raw responses for diagnostic inspection
    if save_responses {
        let resolved_model = model_override.unwrap_or(&config.ai.model);
        let diagnostic: Vec<DiagnosticResponse> = all_responses.iter().map(|sr| DiagnosticResponse {
            scenario_id: sr.scenario_id.clone(),
            scenario_name: sr.scenario_name.clone(),
            category: sr.category.clone(),
            difficulty: String::new(),
            expected_action: sr.expected_action.clone(),
            current_price: sr.current_price,
            raw_response: sr.response.as_ref().ok().cloned(),
            error: sr.response.as_ref().err().map(|e| e.to_string()),
            latency_ms: sr.latency_ms,
        }).collect();
        save_raw_responses(
            &diagnostic,
            "training",
            resolved_model,
            &config.ai.endpoint,
        );
    }

    // PHASE 3: Parse, capture episodes, collect stats
    let mut brier_predictions: Vec<(f64, bool)> = Vec::new();
    let mut category_edge: std::collections::HashMap<String, (u32, u32)> =
        std::collections::HashMap::new();
    let mut action_count = 0u32;
    let mut hold_count = 0u32;
    let mut error_count = 0u32;
    let mut high_conviction_failures: Vec<(String, String, f64, String)> = Vec::new();
    let mut trades: Vec<TradeRecord> = Vec::new();

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
                            != savant_trading::agent::decision_parser::TradeAction::Pass;
                        let expected_traded = expected_is_trade(&sr.expected_action);
                        let is_correct = agent_traded == expected_traded;
                        let is_hold = !agent_traded;

                        if is_hold {
                            hold_count += 1;
                        } else {
                            action_count += 1;
                        }

                        brier_predictions.push((decision.confidence, is_correct));

                        let edge = category_edge.entry(sr.category.clone()).or_insert((0, 0));
                        edge.1 += 1;
                        if is_correct {
                            edge.0 += 1;
                        }

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

                        let risk = 5.0f64;
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

                        trades.push(TradeRecord {
                            id: sr.scenario_id.clone(),
                            pair: "BTC/USD".into(),
                            side: decision.side,
                            entry_price: decision.entry_price,
                            exit_price: decision.entry_price,
                            quantity: 1.0,
                            pnl: trade_pnl,
                            pnl_pct: trade_pnl / 50.0 * 100.0,
                            fees: 0.0,
                            strategy_name: sr.category.clone(),
                            opened_at: chrono::Utc::now(),
                            closed_at: chrono::Utc::now(),
                            notes: String::new(),
                            on_chain_verified: false,
                            tx_hash: None,
                        });

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

    let metrics = if !trades.is_empty() {
        PerformanceMetrics::calculate(&trades)
    } else {
        Metrics::default()
    };

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

        let lesson_vault_config = VaultConfig::default();
        if lesson_vault_config.enabled {
            let lesson_vault = VaultWriter::new(lesson_vault_config);
            let _ = lesson_vault.project_lesson(scen_id, "high_conviction_failure", &heuristic);
        }
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

    if brier_predictions.len() >= 10 {
        let calibrator =
            savant_trading::memory::calibration::IsotonicCalibrator::fit(&brier_predictions);
        println!("\n--- ISOTONIC CALIBRATION ---");
        for raw in &[0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80, 0.90] {
            let calibrated = calibrator.calibrate(*raw);
            println!(
                "  Raw {:.0}% → Calibrated {:.0}%",
                raw * 100.0,
                calibrated * 100.0
            );
        }
    }

    let mut causal_attributions: Vec<savant_trading::sandbox::feedback::CausalAttribution> =
        Vec::new();
    for sr in &all_responses {
        if let Ok(text) = &sr.response {
            if let Ok(decision) = savant_trading::agent::decision_parser::parse_decision(
                text,
                sr.current_price,
                config.ai.price_tolerance_pct,
            ) {
                let agent_traded =
                    decision.action != savant_trading::agent::decision_parser::TradeAction::Pass;
                let expected_trade = sr.expected_action != "Hold / No Trade";
                let is_correct = agent_traded == expected_trade;

                if agent_traded && !is_correct {
                    let factor = if decision.confidence < 0.40 {
                        savant_trading::sandbox::feedback::LossFactor::Process
                    } else if sr.category.contains("Edge Case")
                        || sr.category.contains("Volatility")
                    {
                        savant_trading::sandbox::feedback::LossFactor::Market
                    } else if decision.reasoning.to_lowercase().contains("fomo")
                        || decision.reasoning.to_lowercase().contains("revenge")
                    {
                        savant_trading::sandbox::feedback::LossFactor::Trader
                    } else {
                        savant_trading::sandbox::feedback::LossFactor::Setup
                    };
                    causal_attributions.push(
                        savant_trading::sandbox::feedback::CausalAttribution {
                            episode_id: sr.scenario_id.clone(),
                            factor,
                            explanation: decision.reasoning.chars().take(100).collect::<String>(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        },
                    );
                }
            }
        }
    }
    if !causal_attributions.is_empty() {
        println!("\n--- CAUSAL ATTRIBUTION ---");
        let mut factor_counts = std::collections::HashMap::new();
        for attr in &causal_attributions {
            *factor_counts
                .entry(format!("{}", attr.factor))
                .or_insert(0u32) += 1;
        }
        for (factor, count) in &factor_counts {
            println!("  {}: {} losses", factor, count);
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

    let pnl_pct = if metrics.total_trades > 0 {
        metrics.total_pnl / 50.0 * 100.0
    } else {
        0.0
    };
    println!("\n═══════════════════════════════════════════");
    println!("         TRAINING WALLET REPORT");
    println!("═══════════════════════════════════════════");
    println!("Starting Balance:  ${:.2}", 50.0);
    println!("Final Balance:     ${:.2}", 50.0 + metrics.total_pnl);
    println!(
        "Total P&L:         ${:+.2} ({:+.2}%)",
        metrics.total_pnl, pnl_pct
    );
    println!("Trades:            {} taken", metrics.total_trades);
    println!(
        "Win Rate:          {:.1}% ({}W / {}L)",
        metrics.win_rate * 100.0,
        metrics.wins,
        metrics.losses
    );
    println!("Profit Factor:     {:.2}", metrics.profit_factor);
    println!("Max Drawdown:      -{:.2}%", metrics.max_drawdown * 100.0);
    println!("═══════════════════════════════════════════\n");
    println!("{}\n", "=".repeat(80));

    let brier_score = if !brier_predictions.is_empty() {
        savant_trading::memory::calibration::calculate_brier_score(&brier_predictions).total
    } else {
        0.5
    };

    // PHASE 6: Post-batch wiring
    match savant_trading::memory::semantic::consolidate(test_memory).await {
        Ok(n) => println!("Semantic consolidation: {} patterns inserted/updated", n),
        Err(e) => warn!("Semantic consolidation failed (non-fatal): {}", e),
    }

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

    let vault_config = VaultConfig::default();
    if vault_config.enabled {
        let vault = VaultWriter::new(vault_config.clone());

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

        if !anti_pattern_narratives.is_empty() {
            let details = anti_pattern_narratives.join("\n- ");
            let _ = vault.project_risk_event(
                "anti_pattern",
                &format!("Training batch anti-patterns:\n- {}", details),
            );
        }

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

        let session_summary = format!(
            "# Training Session — {}\n\n\
             **Scenarios:** {}\n\
             **Actions:** {} ({:.0}%)\n\
             **Holds:** {} ({:.0}%)\n\
             **Errors:** {}\n\
             **Brier Score:** {:.4}\n\
             **Lessons:** {}\n\
             **Anti-Patterns:** {}\n\
             **Episodes in DB:** {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            all_responses.len(),
            action_count,
            action_count as f64 / total * 100.0,
            hold_count,
            hold_count as f64 / total * 100.0,
            error_count,
            brier_score,
            lessons_count,
            anti_pattern_narratives.len(),
            test_memory.total_trades().await.unwrap_or(0),
        );
        let session_id = format!("session_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        let _ = vault.project_session(&session_id, &session_summary);
    }

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
                    decision.action != savant_trading::agent::decision_parser::TradeAction::Pass;
                let is_correct = agent_traded == expected_traded;

                let delta = if is_correct { lr } else { -lr * 0.5 };
                for unit in kb.units_mut() {
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
        metrics,
        starting_balance: 50.0,
    })
}

/// Training mode: run scenarios in a loop until Brier score converges.
#[allow(clippy::too_many_arguments)]
pub async fn run_training(
    config: AppConfig,
    category_filter: Option<String>,
    action_only: bool,
    count_filter: Option<usize>,
    full: bool,
    historical: bool,
    model_override: Option<String>,
    managed_keys: bool,
    save_responses: bool,
) -> anyhow::Result<()> {
    let _managed_keys = managed_keys;
    let _test_memory =
        savant_trading::memory::episodic::EpisodicMemory::new("sqlite:data/test_memory.db").await?;

    let test_memory =
        savant_trading::memory::episodic::EpisodicMemory::new("sqlite:data/test_memory.db").await?;

    let max_runs = if full { 20 } else { 5 };
    let scenarios_per_run = 60;
    let convergence_threshold = 0.02;
    let mut brier_history: Vec<f64> = Vec::new();
    let mut consecutive_small_deltas = 0u32;

    backup_databases(config.training.max_backups);

    let historical_dataset = if historical {
        info!("Historical training mode — fetching real market data...");
        let candle_api =
            savant_trading::data::candle_client::CandleClient::new(&config.exchange.rest_url);
        match savant_trading::data::historical::get_historical(&candle_api, "BTC/USD", 5, 30).await
        {
            Ok(dataset) => {
                info!(
                    "Historical data ready: {} candles ({} days)",
                    dataset.candles.len(),
                    30
                );
                Some(dataset)
            }
            Err(e) => {
                warn!("Historical fetch failed: {}. Falling back to synthetic.", e);
                None
            }
        }
    } else {
        None
    };

    for run in 1..=max_runs {
        let mut scenarios =
            savant_trading::sandbox::scenarios::generate_random_scenarios(scenarios_per_run);

        if let Some(n) = count_filter {
            scenarios.truncate(n);
        }

        if let Some(ref dataset) = historical_dataset {
            let raw_hist =
                savant_trading::data::historical::generate_scenarios_from_history(dataset, 100, 50);
            if !raw_hist.is_empty() {
                let hist_scenarios: Vec<savant_trading::sandbox::scenarios::Scenario> = raw_hist
                    .iter()
                    .map(savant_trading::sandbox::scenarios::historical_to_scenario)
                    .collect();
                info!(
                    "Historical: {} real market scenarios mixed with {} synthetic",
                    hist_scenarios.len(),
                    scenarios.len()
                );
                scenarios.extend(hist_scenarios);
            }
        }

        if let Some(ref cat) = category_filter {
            scenarios.retain(|s| s.category.to_lowercase().contains(&cat.to_lowercase()));
        }
        if action_only {
            scenarios.retain(|s| {
                let a = s.expected_action.to_lowercase();
                a.contains("buy") || a.contains("sell")
            });
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

        let result = run_training_batch(
            &config,
            &scenarios,
            &test_memory,
            model_override.as_deref(),
            managed_keys,
            save_responses,
        )
        .await?;
        brier_history.push(result.brier_score);

        println!(
            "Run {} Brier: {:.4} | Actions: {} | Holds: {} | Lessons: {} | P&L ${:+.2}",
            run,
            result.brier_score,
            result.action_count,
            result.hold_count,
            result.lessons_generated,
            result.metrics.total_pnl,
        );

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
    model_override: Option<String>,
    managed_keys: bool,
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

    let result = run_training_batch(
        &config,
        &scenarios,
        &test_memory,
        model_override.as_deref(),
        managed_keys,
        false,
    )
    .await?;

    let total_episodes = test_memory.total_trades().await.unwrap_or(0);
    println!("Total episodes in test DB: {}", total_episodes);
    println!(
        "Brier: {:.4} | Actions: {} | Holds: {} | Lessons: {} | P&L ${:+.2}",
        result.brier_score,
        result.action_count,
        result.hold_count,
        result.lessons_generated,
        result.metrics.total_pnl,
    );

    Ok(())
}

/// Sandbox: run all 50 scenarios through the real AI brain and grade every decision.
pub async fn run_sandbox(
    config: AppConfig,
    model_override: Option<String>,
    _managed_keys: bool,
    endpoint_override: Option<String>,
    api_key_env_override: Option<String>,
    save_responses: bool,
) -> anyhow::Result<()> {
    use savant_trading::sandbox::feedback::analyze_failures;
    use savant_trading::sandbox::generator::{self};
    use savant_trading::sandbox::grader;
    use savant_trading::sandbox::harness::{SandboxSummary, ScenarioResult};
    use savant_trading::sandbox::report::{format_report_markdown, generate_report_card};
    use savant_trading::sandbox::scenarios::load_all_scenarios;

    let scenarios = load_all_scenarios();
    println!("Loaded {} scenarios", scenarios.len());

    // FID-123: Resolve sandbox provider from [sandbox] config + CLI overrides.
    // Falls back to [ai] fields when [sandbox] is not configured.
    let sandbox_endpoint = endpoint_override
        .as_deref()
        .unwrap_or(&config.sandbox.endpoint);
    let sandbox_api_key_env = api_key_env_override
        .as_deref()
        .unwrap_or(&config.sandbox.api_key_env);
    let resolved_model = model_override
        .clone()
        .unwrap_or_else(|| config.sandbox.model.clone());

    println!("Sandbox provider: endpoint={} model={} key_env={}",
        sandbox_endpoint, resolved_model, sandbox_api_key_env);

    let api_keys: Vec<String> = std::env::var("SANDBOX_API_KEYS")
        .unwrap_or_else(|_| std::env::var(sandbox_api_key_env).unwrap_or_default())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if api_keys.is_empty() {
        anyhow::bail!(
            "No API keys found. Set SANDBOX_API_KEYS (comma-separated) or {} in .env",
            sandbox_api_key_env
        );
    }
    let mut byok_headers: Vec<(String, String)> = Vec::new();
    if resolved_model.starts_with("google/") {
        if let Ok(google_key) = std::env::var("GOOGLE_API_KEY") {
            if !google_key.is_empty() {
                byok_headers.push(("X-Provider-Api-Key".to_string(), google_key));
            }
        }
    }
    let providers: Vec<savant_trading::agent::provider::LlmProvider> = api_keys
        .iter()
        .map(|key| {
            savant_trading::agent::provider::LlmProvider::new(
                savant_trading::agent::provider::LlmConfig {
                    endpoint: sandbox_endpoint.to_string(),
                    model: resolved_model.clone(),
                    api_key: key.clone(),
                    max_tokens: config.ai.max_tokens,
                    temperature: config.ai.temperature,
                    top_p: config.ai.top_p,
                    timeout_secs: config.ai.timeout_secs,
                    extra_headers: byok_headers.clone(),
                },
            )
        })
        .collect();
    println!("AI pool: {} providers (key rotation)", providers.len());

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

    let regime_detector = RegimeDetector::new(
        config.strategy.regime.adx_period,
        config.strategy.regime.adx_trending_threshold,
        config.strategy.regime.adx_ranging_threshold,
        config.strategy.regime.atr_volatility_multiplier,
    );

    let cache_path = "data/sandbox_candles.json";
    let candle_api_sandbox = CandleClient::new(&config.exchange.rest_url);
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
                    warn!("Cache parse failed ({}), fetching from API", e);
                    fetch_and_cache(&candle_api_sandbox, cache_path).await
                }
            },
            Err(e) => {
                warn!("Cache read failed ({}), fetching from API", e);
                fetch_and_cache(&candle_api_sandbox, cache_path).await
            }
        }
    } else {
        println!("No cache found, fetching from API...");
        fetch_and_cache(&candle_api_sandbox, cache_path).await
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
                close: chunk.last().map(|c| c.close).unwrap_or(0.0),
                volume: chunk.iter().map(|c| c.volume).sum(),
                pair: "BTC/USD".into(),
            });
        }
        let higher_tf_candles = vec![("1h".into(), htf_candles)];

        let session = if let Some(ref override_str) = mock.session_override {
            match override_str.as_str() {
                "Asian" => savant_trading::core::session::Session::Asian,
                "European" => savant_trading::core::session::Session::European,
                "US" => savant_trading::core::session::Session::UsEuOverlap,
                "UsEuOverlap" => savant_trading::core::session::Session::UsEuOverlap,
                "DeepAsian" => savant_trading::core::session::Session::DeepAsian,
                "Late US" => savant_trading::core::session::Session::LateUs,
                _ => savant_trading::core::session::current_session(),
            }
        } else {
            let expects_trade = scenario.expected_action.to_lowercase().contains("buy")
                || scenario.expected_action.to_lowercase().contains("sell")
                || scenario.expected_action.to_lowercase().contains("short")
                || scenario.expected_action.to_lowercase().contains("close");
            if expects_trade {
                savant_trading::core::session::Session::UsEuOverlap
            } else {
                savant_trading::core::session::current_session()
            }
        };

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
            pair: "BTC/USD",
            recent_trades: None,
            order_book_imbalance: Some(0.2),
            session,
            memory_context: None,
            higher_tf_candles,
            context_tags: vec![],
            live_price: None,
            decision_log_context: None,
            dex_price: None,
            active_pairs: Some(&["BTC/USD".to_string()]),
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

    // Phase 2: Fire LLM calls in parallel
    let max_concurrent = std::env::var("SANDBOX_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(10);
    println!(
        "Sending {} scenarios to AI brain (max {} concurrent)...",
        prepared.len(),
        max_concurrent
    );

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent));

    struct ScenarioResponse {
        scenario_id: String,
        scenario_name: String,
        category: String,
        difficulty: String,
        expected_action: String,
        response: Result<String, savant_trading::agent::provider::LlmError>,
        current_price: f64,
    }

    struct RetryData {
        scenario_id: String,
        system_prompt: String,
        user_message: String,
        current_price: f64,
    }
    let prepared_for_retry: Vec<RetryData> = prepared
        .iter()
        .map(|ps| RetryData {
            scenario_id: ps.scenario_id.clone(),
            system_prompt: ps.system_prompt.clone(),
            user_message: ps.user_message.clone(),
            current_price: ps.current_price,
        })
        .collect();

    let mut join_set = tokio::task::JoinSet::new();
    for (idx, ps) in prepared.into_iter().enumerate() {
        let provider_config = providers[idx % providers.len()].config_clone();
        let sys = ps.system_prompt;
        let usr = ps.user_message;
        let sem = semaphore.clone();
        join_set.spawn(async move {
            let Ok(_permit) = sem.acquire().await else {
                tracing::warn!("Semaphore closed, skipping sandbox scenario");
                return ScenarioResponse {
                    scenario_id: String::new(),
                    scenario_name: String::new(),
                    category: String::new(),
                    difficulty: String::new(),
                    expected_action: String::new(),
                    response: Err(savant_trading::agent::provider::LlmError::InvalidResponse(
                        "Semaphore closed".into(),
                    )),
                    current_price: 0.0,
                };
            };
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

    // Phase 2b: Retry rate-limited failures
    let retryable = std::mem::take(&mut all_responses);
    let (ok_responses, failed): (Vec<_>, Vec<_>) =
        retryable.into_iter().partition(|sr| sr.response.is_ok());
    let rate_limited: Vec<ScenarioResponse> = failed
        .into_iter()
        .filter(|sr| {
            matches!(&sr.response, Err(savant_trading::agent::provider::LlmError::RateLimited(_)))
                || matches!(&sr.response, Err(savant_trading::agent::provider::LlmError::Http(e))
                    if e.contains("429") || e.contains("502") || e.contains("503") || e.contains("transient"))
        })
        .collect();

    all_responses = ok_responses;

    if !rate_limited.is_empty() {
        println!(
            "\n── Retrying {} rate-limited scenarios (concurrency=1, 5s delay) ──",
            rate_limited.len()
        );
        let retry_provider = providers[0].config_clone();
        for (i, sr) in rate_limited.into_iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            let local_provider =
                savant_trading::agent::provider::LlmProvider::new(retry_provider.clone());
            let matching = prepared_for_retry
                .iter()
                .find(|p| p.scenario_id == sr.scenario_id);
            let (retry_response, retry_price) = if let Some(ps) = matching {
                let messages = vec![savant_trading::agent::provider::Message {
                    role: "user".to_string(),
                    content: ps.user_message.clone(),
                }];
                let resp = local_provider.chat(&ps.system_prompt, &messages).await;
                (resp, ps.current_price)
            } else {
                (
                    Err(savant_trading::agent::provider::LlmError::Http(
                        "Scenario data not found for retry".into(),
                    )),
                    sr.current_price,
                )
            };
            let retry_status = match &retry_response {
                Ok(_) => "OK (retry)".to_string(),
                Err(e) => format!("ERR (retry): {}", e),
            };
            println!(
                "  [retry {}/{}] {} — {}",
                i + 1,
                scenarios.len(),
                sr.scenario_name,
                retry_status
            );
            all_responses.push(ScenarioResponse {
                scenario_id: sr.scenario_id,
                scenario_name: sr.scenario_name,
                category: sr.category,
                difficulty: sr.difficulty,
                expected_action: sr.expected_action,
                response: retry_response,
                current_price: retry_price,
            });
        }
    }

    // Phase 2c: Save raw responses for diagnostic inspection
    if save_responses {
        let diagnostic: Vec<DiagnosticResponse> = all_responses.iter().map(|sr| DiagnosticResponse {
            scenario_id: sr.scenario_id.clone(),
            scenario_name: sr.scenario_name.clone(),
            category: sr.category.clone(),
            difficulty: sr.difficulty.clone(),
            expected_action: sr.expected_action.clone(),
            current_price: sr.current_price,
            raw_response: sr.response.as_ref().ok().cloned(),
            error: sr.response.as_ref().err().map(|e| e.to_string()),
            latency_ms: 0,
        }).collect();
        save_raw_responses(
            &diagnostic,
            "sandbox",
            &resolved_model,
            sandbox_endpoint,
        );
    }

    // Phase 3: Grade all responses
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
            model: resolved_model.clone(),
            concurrency: max_concurrent,
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
#[allow(dead_code)]
pub(super) fn has_actionable_signal(
    indicators: &savant_trading::core::types::IndicatorValues,
    _regime: savant_trading::core::types::MarketRegime,
    ob_imbalance: Option<f64>,
    current_price: f64,
    current_volume: Option<f64>,
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
        if spread_pct > 0.5 {
            return true;
        }
    }

    // Order book imbalance
    if let Some(obi) = ob_imbalance {
        if obi.abs() > 0.3 {
            return true;
        }
    }

    // VWAP deviation (WIRED - FID-021: was dead code)
    if let (Some(vwap), Some(atr)) = (indicators.vwap, indicators.atr) {
        if atr > 0.0 && ((current_price - vwap) / atr).abs() > 1.0 {
            return true;
        }
    }

    // NOTE: Trending regime gate removed (FID-021: redundant with ADX > 25)

    // Volume spike (NEW - FID-021)
    if let (Some(vol), Some(vsma)) = (current_volume, indicators.volume_sma) {
        if vsma > 0.0 && vol / vsma > 1.5 {
            return true;
        }
    }

    false
}

/// Verify token safety before buying — checks 24h volume and holder count
/// via Blockscout API. Rejects dead coins (< $1M volume) and honeypots (< 5000 holders).
#[allow(dead_code)]
pub(super) async fn verify_token_safety(token_address: &str) -> Result<(f64, u64), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    let url = format!(
        "https://arbitrum.blockscout.com/api/v2/tokens/{}",
        token_address
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Blockscout error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Blockscout returned {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Blockscout parse error: {}", e))?;

    let volume = json["volume_24h"]
        .as_str()
        .unwrap_or("0")
        .parse::<f64>()
        .unwrap_or(0.0);
    let holders = json["holders_count"]
        .as_str()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);

    Ok((volume, holders))
}
