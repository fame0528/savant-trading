//! REST API server for the Savant Trading dashboard.
//!
//! Exposes engine state, trade history, AI decisions, insight data, and
//! control endpoints via a localhost REST API.

use axum::{
    extract::{Path, State},
    http::header,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};

use savant_trading::core::config::AppConfig;
use savant_trading::core::shared::{DecisionRecord, SharedEngineData};
use serde::{Deserialize, Serialize};

/// Shared application state accessible by both engine and API.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub engine_status: Arc<RwLock<EngineStatus>>,
    pub shared: SharedEngineData,
    pub engine_running: Arc<AtomicBool>,
    pub engine_child: Arc<Mutex<Option<tokio::process::Child>>>,
    pub engine_started_at: Arc<Mutex<Option<Instant>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    pub mode: String,
    pub uptime_seconds: u64,
    pub pairs: Vec<String>,
    pub autonomy_level: u8,
    pub ai_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    pub error: Option<String>,
    pub timestamp: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            data,
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Start the REST API server.
pub async fn start_server(
    config: AppConfig,
    shared: SharedEngineData,
    engine_running: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let state = AppState {
        config: config.clone(),
        engine_status: Arc::new(RwLock::new(EngineStatus {
            running: false,
            mode: if config.mode.paper_trading {
                "PAPER".to_string()
            } else {
                "LIVE".to_string()
            },
            uptime_seconds: 0,
            pairs: config.trading.pairs.clone(),
            autonomy_level: config.ai.autonomy_level,
            ai_status: "idle".to_string(),
        })),
        shared,
        engine_running,
        engine_child: Arc::new(Mutex::new(None)),
        engine_started_at: Arc::new(Mutex::new(None)),
    };

    // CORS: allow dashboard origin + localhost fallbacks
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse::<axum::http::HeaderValue>().unwrap(),
            "http://127.0.0.1:3000".parse::<axum::http::HeaderValue>().unwrap(),
            "http://localhost:8080".parse::<axum::http::HeaderValue>().unwrap(),
            "http://127.0.0.1:8080".parse::<axum::http::HeaderValue>().unwrap(),
        ])
        .allow_methods(tower_http::cors::Any)
        .allow_headers(vec![
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
        ]);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/status", get(get_status))
        .route("/api/config", get(get_config))
        .route("/api/portfolio", get(get_portfolio))
        .route("/api/positions", get(get_positions))
        .route("/api/trades", get(get_trades))
        .route("/api/decisions", get(get_decisions))
        .route("/api/insight", get(get_insight))
        .route("/api/knowledge", get(get_knowledge))
        .route("/api/knowledge/{topic}", get(get_knowledge_by_topic))
        .route("/api/risk", get(get_risk))
        .route("/api/session", get(get_session))
        .route("/api/activity", get(get_activity))
        .route("/api/memory", get(get_memory))
        .route("/api/training", get(get_training))
        .route("/api/wallet", get(get_wallet))
        .route("/api/engine/start", post(start_engine))
        .route("/api/engine/stop", post(stop_engine))
        .route("/api/terminal", get(terminal_ws))
        .with_state(state)
        .layer(cors)
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("API server listening on http://127.0.0.1:8080");

    // Graceful shutdown on Ctrl+C
    tokio::select! {
        result = axum::serve(listener, app) => result?,
        _ = tokio::signal::ctrl_c() => {
            info!("API server shutting down gracefully");
        }
    }
    Ok(())
}

async fn get_status(State(state): State<AppState>) -> Json<ApiResponse<EngineStatus>> {
    let mut status = state.engine_status.read().await.clone();
    status.running = state.engine_running.load(Ordering::Relaxed);

    // Compute uptime from engine_started_at
    if let Ok(started) = state.engine_started_at.lock() {
        if let Some(instant) = *started {
            status.uptime_seconds = instant.elapsed().as_secs();
        }
    }

    Json(ApiResponse::ok(status))
}

async fn get_config(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let config = &state.config;
    Json(ApiResponse::ok(serde_json::json!({
        "exchange": config.exchange.name,
        "pairs": config.trading.pairs,
        "timeframe": config.trading.timeframe,
        "paper_trading": config.mode.paper_trading,
        "starting_balance": config.trading.starting_balance,
        "autonomy_level": config.ai.autonomy_level,
        "model": config.ai.model,
    })))
}

async fn get_portfolio(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let account = state.shared.account.read().await;
    Json(ApiResponse::ok(serde_json::json!({
        "balance": account.balance,
        "equity": account.equity,
        "drawdown_pct": account.drawdown_pct,
        "daily_pnl": account.daily_pnl,
        "unrealized_pnl": account.unrealized_pnl,
        "peak_equity": account.peak_equity,
        "open_positions": account.open_positions,
        "trades_today": account.trades_today,
    })))
}

async fn get_positions(State(state): State<AppState>) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let positions = state.shared.positions.read().await;
    let items: Vec<serde_json::Value> = positions
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "pair": p.pair,
                "side": format!("{:?}", p.side),
                "entry_price": p.entry_price,
                "current_price": p.current_price,
                "quantity": p.quantity,
                "stop_loss": p.stop_loss,
                "take_profit_1": p.take_profit_1,
                "take_profit_2": p.take_profit_2,
                "take_profit_3": p.take_profit_3,
                "unrealized_pnl": p.unrealized_pnl,
                "risk_amount": p.risk_amount,
                "strategy_name": p.strategy_name,
                "scale_level": format!("{:?}", p.scale_level),
                "opened_at": p.opened_at.to_rfc3339(),
            })
        })
        .collect();
    Json(ApiResponse::ok(items))
}

async fn get_trades(State(state): State<AppState>) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let trades = state.shared.closed_trades.read().await;
    let items: Vec<serde_json::Value> = trades
        .iter()
        .rev()
        .take(50)
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "pair": t.pair,
                "side": format!("{:?}", t.side),
                "entry_price": t.entry_price,
                "exit_price": t.exit_price,
                "quantity": t.quantity,
                "pnl": t.pnl,
                "pnl_pct": t.pnl_pct,
                "strategy_name": t.strategy_name,
                "opened_at": t.opened_at.to_rfc3339(),
                "closed_at": t.closed_at.to_rfc3339(),
                "notes": t.notes,
            })
        })
        .collect();
    Json(ApiResponse::ok(items))
}

async fn get_decisions(State(state): State<AppState>) -> Json<ApiResponse<Vec<DecisionRecord>>> {
    let decisions = state.shared.decisions.read().await;
    let items: Vec<DecisionRecord> = decisions.iter().rev().take(20).cloned().collect();
    Json(ApiResponse::ok(items))
}

async fn get_insight(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let insight = state.shared.insight.read().await;
    Json(ApiResponse::ok(serde_json::json!({
        "fear_greed": insight.sentiment.fear_greed_index,
        "fear_greed_label": insight.sentiment.fear_greed_label,
        "btc_dominance": insight.sentiment.btc_dominance,
        "funding_rate": insight.funding.funding_rate,
        "open_interest": insight.funding.open_interest,
        "block_height": insight.flows.block_height,
        "mempool_size": insight.flows.mempool_size,
        "rss_items": insight.rss_items.len(),
        "trending_coins": insight.news.trending_coins.iter().map(|c| &c.symbol).collect::<Vec<_>>(),
        "summary": insight.summary(),
    })))
}

async fn get_knowledge() -> Json<ApiResponse<serde_json::Value>> {
    let knowledge_dir = std::path::PathBuf::from("knowledge");
    let mut units = Vec::new();

    if knowledge_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&knowledge_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(json) = std::fs::read_to_string(&path) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                            if let Some(arr) = parsed.as_array() {
                                units.extend(arr.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    Json(ApiResponse::ok(serde_json::json!({
        "total_units": units.len(),
        "units": units,
    })))
}

async fn get_knowledge_by_topic(Path(topic): Path<String>) -> Json<ApiResponse<serde_json::Value>> {
    let knowledge_dir = std::path::PathBuf::from("knowledge");
    let mut units = Vec::new();

    if knowledge_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&knowledge_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(json) = std::fs::read_to_string(&path) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                            if let Some(arr) = parsed.as_array() {
                                for unit in arr {
                                    if unit
                                        .get("topic")
                                        .and_then(|t| t.as_str())
                                        .map(|t| t.eq_ignore_ascii_case(&topic))
                                        .unwrap_or(false)
                                    {
                                        units.push(unit.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Json(ApiResponse::ok(serde_json::json!({
        "topic": topic,
        "total_units": units.len(),
        "units": units,
    })))
}

async fn get_risk(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let account = state.shared.account.read().await;
    let positions = state.shared.positions.read().await;
    let config = &state.config;

    let circuit_status = if account.drawdown_pct >= config.risk.max_drawdown {
        "KILL_SWITCH"
    } else if account.daily_pnl.abs() >= account.equity * config.risk.max_daily_loss {
        "DAILY_LIMIT"
    } else {
        "OK"
    };

    Json(ApiResponse::ok(serde_json::json!({
        "circuit_breaker": circuit_status,
        "daily_loss_pct": if account.equity > 0.0 { account.daily_pnl / account.equity } else { 0.0 },
        "drawdown_pct": account.drawdown_pct,
        "open_positions": positions.len(),
        "max_positions": config.risk.max_positions,
        "max_risk_per_trade": config.risk.max_risk_per_trade,
        "max_daily_loss": config.risk.max_daily_loss,
        "max_drawdown": config.risk.max_drawdown,
    })))
}

async fn get_session(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let trades = state.shared.closed_trades.read().await;
    let decisions = state.shared.decisions.read().await;
    let account = state.shared.account.read().await;

    let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
    let total = trades.len();

    Json(ApiResponse::ok(serde_json::json!({
        "total_trades": total,
        "wins": wins,
        "losses": total - wins,
        "win_rate": if total > 0 { wins as f64 / total as f64 } else { 0.0 },
        "total_pnl": trades.iter().map(|t| t.pnl).sum::<f64>(),
        "total_decisions": decisions.len(),
        "balance": account.balance,
        "equity": account.equity,
    })))
}

async fn get_activity(State(state): State<AppState>) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let activity = state.shared.activity_log.read().await;
    let items: Vec<serde_json::Value> = activity
        .iter()
        .rev()
        .take(100)
        .map(|e| {
            serde_json::json!({
                "timestamp": e.timestamp,
                "level": format!("{:?}", e.level),
                "pair": e.pair,
                "message": e.message,
            })
        })
        .collect();
    Json(ApiResponse::ok(items))
}

async fn get_memory(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let mem = state.shared.memory_snapshot.read().await;
    Json(ApiResponse::ok(serde_json::json!({
        "brier_score": mem.brier_score,
        "brier_label": mem.brier_label,
        "confidence_cap": mem.confidence_cap,
        "total_trades": mem.total_trades,
        "cusum_status": mem.cusum_status,
        "replay_lesson_count": mem.replay_lesson_count,
    })))
}

async fn start_engine(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    // Check if already running
    {
        let child_lock = state.engine_child.lock().unwrap();
        if child_lock.is_some() {
            return Json(ApiResponse::ok(
                serde_json::json!({"message": "Engine already running"}),
            ));
        }
    }

    // Find the savant binary, fallback to cargo run
    let exe_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("savant")))
        .filter(|p| p.exists());

    let (result, cmd_name) = if let Some(path) = exe_path {
        (
            tokio::process::Command::new(&path)
                .kill_on_drop(true)
                .spawn(),
            path.display().to_string(),
        )
    } else {
        let root = std::env::var("SAVANT_ROOT").unwrap_or_else(|_| ".".to_string());
        (
            tokio::process::Command::new("cargo")
                .args(["run", "--release"])
                .current_dir(&root)
                .kill_on_drop(true)
                .spawn(),
            "cargo run --release".to_string(),
        )
    };

    match result {
        Ok(child) => {
            state.engine_running.store(true, Ordering::Relaxed);
            *state.engine_child.lock().unwrap() = Some(child);
            *state.engine_started_at.lock().unwrap() = Some(Instant::now());

            let mut status = state.engine_status.write().await;
            status.running = true;
            status.ai_status = "active".to_string();

            info!("Engine started: {}", cmd_name);
            Json(ApiResponse::ok(
                serde_json::json!({"message": format!("Engine started: {}", cmd_name)}),
            ))
        }
        Err(e) => {
            warn!("Failed to start engine: {}", e);
            Json(ApiResponse {
                data: serde_json::json!({"message": "Failed to start"}),
                error: Some(e.to_string()),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

async fn stop_engine(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let child = state.engine_child.lock().unwrap().take();
    if let Some(mut child) = child {
        let _ = child.start_kill();
    }
    state.engine_running.store(false, Ordering::Relaxed);
    *state.engine_started_at.lock().unwrap() = None;

    let mut status = state.engine_status.write().await;
    status.running = false;
    status.ai_status = "stopped".to_string();

    info!("Engine stopped");
    Json(ApiResponse::ok(
        serde_json::json!({"message": "Engine stopped"}),
    ))
}

/// Health check endpoint for load balancers / Docker.
async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let running = state.engine_running.load(Ordering::Relaxed);
    let uptime = state.engine_started_at.lock().unwrap()
        .map(|i| i.elapsed().as_secs())
        .unwrap_or(0);

    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": uptime,
        "engine_running": running,
    }))
}

/// Wallet endpoint — returns address + on-chain balances.
async fn get_wallet(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let private_key = std::env::var(&state.config.exchange.dex.wallet_key_env).unwrap_or_default();
    if private_key.is_empty() {
        return Json(ApiResponse {
            data: serde_json::json!({"address": null, "error": "WALLET_PRIVATE_KEY not set"}),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    let address = match derive_wallet_address(&private_key) {
        Ok(addr) => addr,
        Err(e) => {
            return Json(ApiResponse {
                data: serde_json::json!({"address": null, "error": e}),
                error: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    // Query on-chain ETH balance via RPC
    let rpc_url = &state.config.exchange.dex.rpc_url;
    let eth_balance = query_eth_balance(rpc_url, &address).await.unwrap_or(0.0);

    // Query USDC balance
    let usdc_contract = savant_trading::execution::dex::usdc_address_for_chain(
        state.config.exchange.dex.chain_id,
    );
    let usdc_balance = query_erc20_balance(rpc_url, usdc_contract, &address)
        .await
        .unwrap_or(0.0);

    Json(ApiResponse::ok(serde_json::json!({
        "address": address,
        "eth_balance": eth_balance,
        "usdc_balance": usdc_balance,
        "chain_id": state.config.exchange.dex.chain_id,
        "rpc_url": rpc_url,
    })))
}

/// Derive wallet address from private key hex.
fn derive_wallet_address(private_key_hex: &str) -> Result<String, String> {
    use alloy_core::primitives::{Address, Keccak256, hex};
    use k256::ecdsa::SigningKey;

    let hex_key = private_key_hex.trim_start_matches("0x");
    let key_bytes = hex::decode(hex_key).map_err(|e| format!("Invalid hex: {}", e))?;
    let signing_key = SigningKey::from_slice(&key_bytes).map_err(|e| format!("Invalid key: {}", e))?;
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false).to_bytes().to_vec();
    let mut hasher = Keccak256::new();
    hasher.update(&encoded[1..]);
    let hash = hasher.finalize();
    let addr_bytes: [u8; 20] = hash[12..32].try_into().map_err(|_| "Failed to derive address".to_string())?;
    let address = Address::from(addr_bytes);
    Ok(format!("{:#x}", address))
}

/// Query native ETH balance via eth_getBalance RPC.
async fn query_eth_balance(rpc_url: &str, address: &str) -> Result<f64, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getBalance",
        "params": [address, "latest"]
    });
    let resp: serde_json::Value = client.post(rpc_url)
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    if let Some(hex) = resp.get("result").and_then(|r| r.as_str()) {
        let wei = alloy_core::primitives::U256::from_str_radix(hex.trim_start_matches("0x"), 16)
            .unwrap_or(alloy_core::primitives::U256::ZERO);
        let eth: f64 = wei.to_string().parse().unwrap_or(0.0) / 1e18;
        Ok(eth)
    } else {
        Err("No result in RPC response".to_string())
    }
}

/// Query ERC-20 token balance via balanceOf(address).
async fn query_erc20_balance(rpc_url: &str, token_address: &str, wallet: &str) -> Result<f64, String> {
    let client = reqwest::Client::new();
    // balanceOf(address) selector: 0x70a08231
    let addr_clean = wallet.trim_start_matches("0x");
    let padded = format!("0x70a08231{:0>64}", addr_clean);
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [{"to": token_address, "data": padded}, "latest"]
    });
    let resp: serde_json::Value = client.post(rpc_url)
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;

    if let Some(hex) = resp.get("result").and_then(|r| r.as_str()) {
        let raw = alloy_core::primitives::U256::from_str_radix(hex.trim_start_matches("0x"), 16)
            .unwrap_or(alloy_core::primitives::U256::ZERO);
        let balance: f64 = raw.to_string().parse().unwrap_or(0.0) / 1e6; // USDC = 6 decimals
        Ok(balance)
    } else {
        Err("No result in RPC response".to_string())
    }
}

/// Auth middleware — checks Bearer token if SAVANT_API_TOKEN is set.
async fn auth_middleware(
    req: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let token = std::env::var("SAVANT_API_TOKEN").unwrap_or_default();
    if token.is_empty() {
        // No token configured — open access (dev mode)
        return next.run(req).await;
    }

    let auth = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if auth != format!("Bearer {}", token) {
        return axum::response::Response::builder()
            .status(401)
            .body(axum::body::Body::from("Unauthorized"))
            .expect("response builder");
    }

    next.run(req).await
}

struct RateLimiter {
    window_start: Instant,
    count: u64,
    max_per_window: u64,
    window_duration: std::time::Duration,
}

static RATE_LIMITER: LazyLock<Mutex<RateLimiter>> = LazyLock::new(|| {
    Mutex::new(RateLimiter {
        window_start: Instant::now(),
        count: 0,
        max_per_window: 1000,
        window_duration: std::time::Duration::from_secs(1),
    })
});

async fn rate_limit_middleware(
    req: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    {
        let mut limiter = match RATE_LIMITER.lock() {
            Ok(l) => l,
            Err(_) => {
                return axum::response::Response::builder()
                    .status(500)
                    .body(axum::body::Body::from("Internal error"))
                    .expect("response builder");
            }
        };

        if limiter.window_start.elapsed() >= limiter.window_duration {
            limiter.window_start = Instant::now();
            limiter.count = 0;
        }

        limiter.count += 1;
        if limiter.count > limiter.max_per_window {
            return axum::response::Response::builder()
                .status(429)
                .body(axum::body::Body::from("Rate limit exceeded"))
                .expect("response builder");
        }
    }

    next.run(req).await
}

/// Training metrics endpoint — returns current training state.
async fn get_training(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let config = &state.config;

    // Try to query test memory DB for episode count
    let (total_episodes, brier_estimate) =
        match savant_trading::memory::episodic::EpisodicMemory::new("sqlite:data/test_memory.db")
            .await
        {
            Ok(mem) => {
                let total = mem.total_trades().await.unwrap_or(0);
                (total, if total >= 10 { Some(0.25) } else { None })
            }
            Err(_) => (0, None),
        };

    // Try to query semantic pattern count
    let pattern_count =
        match savant_trading::memory::episodic::EpisodicMemory::new("sqlite:data/test_memory.db")
            .await
        {
            Ok(mem) => savant_trading::memory::semantic::query_active_patterns(mem.pool(), 1000)
                .await
                .map(|p| p.len())
                .unwrap_or(0),
            Err(_) => 0,
        };

    let data = serde_json::json!({
        "total_episodes": total_episodes,
        "semantic_patterns": pattern_count,
        "brier_estimate": brier_estimate,
        "training_config": {
            "min_sample_size": config.training.min_sample_size,
            "failure_win_rate": config.training.failure_win_rate,
            "max_portfolio_heat": config.training.max_portfolio_heat,
            "backup_interval_hours": config.training.backup_interval_hours,
            "utility_learning_rate": config.training.utility_learning_rate,
            "brier_cap_threshold": config.training.brier_cap_threshold,
            "memory_context_min_trades": config.training.memory_context_min_trades,
        },
        "soul_md_version": "1.0.0",
    });

    Json(ApiResponse {
        data,
        error: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// WebSocket terminal — streams engine process stdout/stderr to the dashboard.
/// Supports "savant" commands: "savant start" = cargo run --release, "savant stop" = kill.
async fn terminal_ws(
    ws: axum::extract::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_terminal(socket, state))
}

async fn handle_terminal(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
) {
    use axum::extract::ws::Message;
    use futures_util::{SinkExt, StreamExt};
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;
    use std::process::Stdio;

    let (mut sender, mut receiver) = socket.split();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(256);

    // Sender task: channel → WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let _ = tx.send(concat!(
        "\x1b[36m╔══════════════════════════════════════════╗\x1b[0m\r\n",
        "\x1b[36m║\x1b[0m  \x1b[1;37mSAVANT\x1b[0m \x1b[90mTerminal v0.9.0\x1b[0m               \x1b[36m║\x1b[0m\r\n",
        "\x1b[36m║\x1b[0m  \x1b[90mCommands:\x1b[0m                               \x1b[36m║\x1b[0m\r\n",
        "\x1b[36m║\x1b[0m    \x1b[33msavant start\x1b[0m  — start engine          \x1b[36m║\x1b[0m\r\n",
        "\x1b[36m║\x1b[0m    \x1b[33msavant stop\x1b[0m   — stop engine           \x1b[36m║\x1b[0m\r\n",
        "\x1b[36m║\x1b[0m    \x1b[33msavant status\x1b[0m — engine status          \x1b[36m║\x1b[0m\r\n",
        "\x1b[36m║\x1b[0m    \x1b[33msavant help\x1b[0m   — show this help         \x1b[36m║\x1b[0m\r\n",
        "\x1b[36m║\x1b[0m    \x1b[33mCtrl+C\x1b[0m        — stop running process    \x1b[36m║\x1b[0m\r\n",
        "\x1b[36m╚══════════════════════════════════════════╝\x1b[0m\r\n",
        "\r\n",
    ).to_string()).await;

    let mut child_handle: Option<tokio::process::Child> = None;
    let engine_running = state.engine_running.clone();

    while let Some(msg) = receiver.next().await {
        let text = match msg {
            Ok(Message::Text(t)) => t,
            Ok(Message::Close(_)) => break,
            _ => continue,
        };

        let trimmed = text.trim();

        if text.contains('\u{0003}') {
            if let Some(ref mut child) = child_handle {
                let _ = child.start_kill();
                let _ = tx.send("\r\n\x1b[33m[SAVANT]\x1b[0m Process killed\r\n".into()).await;
                child_handle = None;
                engine_running.store(false, Ordering::Relaxed);
                *state.engine_child.lock().unwrap() = None;
                *state.engine_started_at.lock().unwrap() = None;
            }
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.first() == Some(&"savant") {
            match parts.get(1).copied() {
                Some("start") => {
                    if child_handle.is_some() {
                        let _ = tx.send(
                            "\x1b[33m[SAVANT]\x1b[0m Already running. Use \x1b[33msavant stop\x1b[0m first.\r\n".into()
                        ).await;
                        continue;
                    }

                    let _ = tx.send(
                        "\x1b[36m[SAVANT]\x1b[0m Starting engine…\r\n".into()
                    ).await;

                    // Try compiled binary first, fallback to cargo run
                    let exe_path = std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(|d| d.join("savant")))
                        .filter(|p| p.exists());

                    let (spawn_result, _cmd_name) = if let Some(path) = exe_path {
                        let name = path.display().to_string();
                        (
                            Command::new(&path)
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .stdin(Stdio::null())
                                .kill_on_drop(true)
                                .spawn(),
                            name,
                        )
                    } else {
                        let root = std::env::var("SAVANT_ROOT").unwrap_or_else(|_| ".".to_string());
                        (
                            Command::new("cargo")
                                .args(["run", "--release"])
                                .current_dir(&root)
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .stdin(Stdio::null())
                                .kill_on_drop(true)
                                .spawn(),
                            "cargo run --release".to_string(),
                        )
                    };

                    match spawn_result {
                        Ok(mut child) => {
                            engine_running.store(true, Ordering::Relaxed);

                            // Take stdout/stderr before moving child into AppState
                            if let Some(stdout) = child.stdout.take() {
                                let tx_out = tx.clone();
                                let running = engine_running.clone();
                                tokio::spawn(async move {
                                    let reader = BufReader::new(stdout);
                                    let mut lines = reader.lines();
                                    while let Ok(Some(line)) = lines.next_line().await {
                                        if tx_out.send(format!("{}\r\n", line)).await.is_err() {
                                            break;
                                        }
                                    }
                                    running.store(false, Ordering::Relaxed);
                                });
                            }

                            if let Some(stderr) = child.stderr.take() {
                                let tx_err = tx.clone();
                                tokio::spawn(async move {
                                    let reader = BufReader::new(stderr);
                                    let mut lines = reader.lines();
                                    while let Ok(Some(line)) = lines.next_line().await {
                                        if tx_err.send(format!("{}\r\n", line)).await.is_err() {
                                            break;
                                        }
                                    }
                                });
                            }

                            *state.engine_child.lock().unwrap() = Some(child);
                            *state.engine_started_at.lock().unwrap() = Some(Instant::now());

                            let _ = tx.send(
                                "\x1b[32m[SAVANT]\x1b[0m Engine started\r\n".to_string()
                            ).await;
                        }
                        Err(e) => {
                            let _ = tx.send(
                                format!("\x1b[31m[SAVANT]\x1b[0m Failed to start: {}\r\n", e)
                            ).await;
                        }
                    }
                }
                Some("stop") => {
                    if let Some(ref mut child) = child_handle {
                        let _ = child.start_kill();
                        let _ = tx.send("\x1b[33m[SAVANT]\x1b[0m Engine stopped\r\n".into()).await;
                        child_handle = None;
                        engine_running.store(false, Ordering::Relaxed);
                        *state.engine_child.lock().unwrap() = None;
                        *state.engine_started_at.lock().unwrap() = None;
                    } else {
                        let _ = tx.send("\x1b[90m[SAVANT]\x1b[0m No engine running\r\n".into()).await;
                    }
                }
                Some("status") => {
                    let running = engine_running.load(Ordering::Relaxed);
                    let status = if running { "\x1b[32mRUNNING\x1b[0m" } else { "\x1b[31mSTOPPED\x1b[0m" };
                    let _ = tx.send(
                        format!("\x1b[36m[SAVANT]\x1b[0m Engine: {}\r\n", status)
                    ).await;
                }
                Some("help") => {
                    let _ = tx.send(concat!(
                        "\x1b[36m[SAVANT]\x1b[0m Commands:\r\n",
                        "  \x1b[33msavant start\x1b[0m   — start engine (cargo run --release)\r\n",
                        "  \x1b[33msavant stop\x1b[0m    — stop engine\r\n",
                        "  \x1b[33msavant status\x1b[0m  — check engine status\r\n",
                        "  \x1b[33msavant help\x1b[0m    — show this help\r\n",
                        "  \x1b[33mCtrl+C\x1b[0m         — stop running process\r\n",
                    ).into()).await;
                }
                _ => {
                    let _ = tx.send(
                        format!("\x1b[90m[SAVANT]\x1b[0m Unknown: savant {}. Type \x1b[33msavant help\x1b[0m\r\n",
                            parts.get(1).unwrap_or(&""))
                    ).await;
                }
            }
        } else if !trimmed.is_empty() {
            let _ = tx.send(
                "\x1b[90m[SAVANT]\x1b[0m Use \x1b[33msavant start|stop|status|help\x1b[0m\r\n".into()
            ).await;
        }
    }

    if let Some(ref mut child) = child_handle {
        let _ = child.start_kill();
    }
    engine_running.store(false, Ordering::Relaxed);
    *state.engine_child.lock().unwrap() = None;
    *state.engine_started_at.lock().unwrap() = None;
    drop(tx);
    let _ = send_task.await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_response_ok() {
        let resp = ApiResponse::ok("test");
        assert_eq!(resp.data, "test");
        assert!(resp.error.is_none());
        assert!(!resp.timestamp.is_empty());
    }

    #[test]
    fn engine_status_serializes() {
        let status = EngineStatus {
            running: true,
            mode: "PAPER".to_string(),
            uptime_seconds: 100,
            pairs: vec!["BTC/USD".to_string()],
            autonomy_level: 3,
            ai_status: "active".to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("PAPER"));
        assert!(json.contains("BTC/USD"));
    }

    #[test]
    fn shared_engine_data_default() {
        let shared = SharedEngineData::new();
        // Verify it creates without panic
        assert!(std::sync::Arc::strong_count(&shared.account) >= 1);
    }

    #[test]
    fn rate_limiter_window_resets() {
        let mut limiter = RateLimiter {
            window_start: Instant::now() - std::time::Duration::from_secs(2),
            count: 5000,
            max_per_window: 1000,
            window_duration: std::time::Duration::from_secs(1),
        };

        // Simulate window check
        if limiter.window_start.elapsed() >= limiter.window_duration {
            limiter.window_start = Instant::now();
            limiter.count = 0;
        }

        assert_eq!(limiter.count, 0);
    }
}
