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

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub engine_status: Arc<RwLock<EngineStatus>>,
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
pub async fn start_server(config: AppConfig) -> anyhow::Result<()> {
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

async fn get_portfolio() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "balance": 0.0,
        "equity": 0.0,
        "drawdown_pct": 0.0,
        "daily_pnl": 0.0,
        "total_pnl": 0.0,
    })))
}

async fn get_positions() -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::ok(vec![]))
}

async fn get_trades() -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::ok(vec![]))
}

async fn get_decisions() -> Json<ApiResponse<Vec<serde_json::Value>>> {
    Json(ApiResponse::ok(vec![]))
}

async fn get_insight() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "fear_greed": null,
        "btc_dominance": null,
        "funding_rate": null,
        "rss_items": [],
    })))
}

async fn get_knowledge() -> Json<ApiResponse<serde_json::Value>> {
    // Load from external knowledge directory
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

async fn get_risk() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "circuit_breaker": "OK",
        "daily_loss_pct": 0.0,
        "drawdown_pct": 0.0,
        "open_positions": 0,
        "max_positions": 3,
    })))
}

async fn get_session() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "events": [],
        "start_time": chrono::Utc::now().to_rfc3339(),
        "total_decisions": 0,
        "total_trades": 0,
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
