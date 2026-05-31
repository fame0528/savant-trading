//! Shared engine state for cross-module access (API, TUI, engine).

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::types::{AccountState, Position, TradeRecord};
use crate::insight::aggregator::MarketContext;

/// Thread-safe shared data between the trading engine, API, and TUI.
#[derive(Clone)]
pub struct SharedEngineData {
    pub account: Arc<RwLock<AccountState>>,
    pub positions: Arc<RwLock<Vec<Position>>>,
    pub closed_trades: Arc<RwLock<Vec<TradeRecord>>>,
    pub insight: Arc<RwLock<MarketContext>>,
    pub decisions: Arc<RwLock<Vec<DecisionRecord>>>,
    pub activity_log: Arc<RwLock<Vec<ActivityEntry>>>,
    pub memory_snapshot: Arc<RwLock<MemorySnapshot>>,
}

/// Memory system state for TUI display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemorySnapshot {
    pub brier_score: Option<f64>,
    pub brier_label: String,
    pub confidence_cap: String,
    pub total_trades: i64,
    pub cusum_status: Vec<(String, String)>, // (pair, status)
    pub replay_lesson_count: usize,
}

/// A recorded AI decision for API/TUI exposure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// Severity level for activity log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActivityLevel {
    /// Data fetch, indicator computation
    Info,
    /// AI thinking, analysis in progress
    Thinking,
    /// AI decision made
    Decision,
    /// Trade executed
    Trade,
    /// Warning or alert
    Warning,
    /// Error
    Error,
}

/// A real-time activity log entry — visible in TUI as it happens.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActivityEntry {
    pub timestamp: String,
    pub level: ActivityLevel,
    pub pair: String,
    pub message: String,
}

impl SharedEngineData {
    pub fn new() -> Self {
        Self {
            account: Arc::new(RwLock::new(AccountState::new(0.0))),
            positions: Arc::new(RwLock::new(Vec::new())),
            closed_trades: Arc::new(RwLock::new(Vec::new())),
            insight: Arc::new(RwLock::new(MarketContext::default())),
            decisions: Arc::new(RwLock::new(Vec::new())),
            activity_log: Arc::new(RwLock::new(Vec::new())),
            memory_snapshot: Arc::new(RwLock::new(MemorySnapshot {
                brier_score: None,
                brier_label: "No data".to_string(),
                confidence_cap: "LOW".to_string(),
                total_trades: 0,
                cusum_status: Vec::new(),
                replay_lesson_count: 0,
            })),
        }
    }

    /// Log an activity entry. Keeps last 200 entries.
    pub async fn log_activity(&self, level: ActivityLevel, pair: &str, message: &str) {
        let entry = ActivityEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level,
            pair: pair.to_string(),
            message: message.to_string(),
        };
        let mut log = self.activity_log.write().await;
        log.push(entry);
        if log.len() > 200 {
            log.drain(0..100);
        }
    }
}

impl Default for SharedEngineData {
    fn default() -> Self {
        Self::new()
    }
}
