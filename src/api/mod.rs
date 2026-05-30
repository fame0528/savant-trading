//! REST API server for the Savant Trading dashboard.
//!
//! Exposes engine state, trade history, AI decisions, insight data, and
//! control endpoints via a localhost REST API.

use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use savant_trading::core::config::AppConfig;
use savant_trading::core::types::{AccountState, Position, TradeRecord};
use savant_trading::insight::aggregator::MarketContext;

/// Shared application state accessible by both engine and API.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub engine_status: Arc<RwLock<EngineStatus>>,
    pub shared: SharedEngineData,
}

/// Thread-safe shared data between the trading engine and API handlers.
#[derive(Clone)]
pub struct SharedEngineData {
    pub account: Arc<RwLock<AccountState>>,
    pub positions: Arc<RwLock<Vec<Position>>>,
    pub closed_trades: Arc<RwLock<Vec<TradeRecord>>>,
    pub insight: Arc<RwLock<MarketContext>>,
    pub decisions: Arc<RwLock<Vec<DecisionRecord>>>,
}

impl SharedEngineData {
    pub fn new() -> Self {
        Self {
            account: Arc::new(RwLock::new(AccountState::new(0.0))),
            positions: Arc::new(RwLock::new(Vec::new())),
            closed_trades: Arc::new(RwLock::new(Vec::new())),
            insight: Arc::new(RwLock::new(MarketContext::default())),
            decisions: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for SharedEngineData {
    fn default() -> Self {
        Self::new()
    }
}

/// A recorded AI decision for API exposure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub timestamp: String,
    pub pair: String,
    pub action: String,
    pub side: String,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub confidence: f64,
    pub reasoning: String,
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
pub async fn start_server(config: AppConfig, shared: SharedEngineData) -> anyhow::Result<()> {
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
    };

    let app = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/config", get(get_config))
        .route("/api/portfolio", get(get_portfolio))
        .route("/api/positions", get(get_positions))
        .route("/api/trades", get(get_trades))
        .route("/api/decisions", get(get_decisions))
        .route("/api/insight", get(get_insight))
        .route("/api/knowledge", get(get_knowledge))
        .route("/api/risk", get(get_risk))
        .route("/api/session", get(get_session))
        .route("/api/engine/start", post(start_engine))
        .route("/api/engine/stop", post(stop_engine))
        .route("/api/engine/dry-run", post(trigger_dry_run))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("API server listening on http://127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_status(State(state): State<AppState>) -> Json<ApiResponse<EngineStatus>> {
    let status = state.engine_status.read().await;
    Json(ApiResponse::ok(status.clone()))
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

async fn start_engine(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let mut status = state.engine_status.write().await;
    status.running = true;
    status.ai_status = "active".to_string();
    Json(ApiResponse::ok(
        serde_json::json!({"message": "Engine started"}),
    ))
}

async fn stop_engine(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let mut status = state.engine_status.write().await;
    status.running = false;
    status.ai_status = "stopped".to_string();
    Json(ApiResponse::ok(
        serde_json::json!({"message": "Engine stopped"}),
    ))
}

async fn trigger_dry_run() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(
        serde_json::json!({"message": "Dry run triggered"}),
    ))
}
