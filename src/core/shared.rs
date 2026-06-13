//! Shared engine state for cross-module access (API, TUI, engine).

use std::collections::{HashMap, VecDeque};
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
    pub equity_curve: Arc<RwLock<Vec<serde_json::Value>>>,
    /// Stop-loss overrides from API — pair → new stop price.
    /// Engine reads and applies these each cycle, then clears them.
    pub stop_overrides: Arc<RwLock<HashMap<String, f64>>>,
    /// Close requests from API — pair → close signal.
    /// Engine reads and executes on-chain close each cycle, then clears them.
    pub close_overrides: Arc<RwLock<HashMap<String, bool>>>,
    /// FID-063: Hunt mode flag — true when idle capital > $5 and equity < $500.
    /// Exposed to API/dashboard for visibility.
    pub hunt_mode: Arc<RwLock<bool>>,
    pub monitoring_mode: Arc<RwLock<bool>>,
    /// FID-081: Price feed staleness — seconds since last WS price update.
    /// Dashboard shows amber "STALE PRICES" chip when > 300s.
    pub price_staleness_secs: Arc<RwLock<u64>>,
    /// FID-093 C1: Cached wallet address — derived once at startup.
    pub wallet_address: Arc<RwLock<String>>,
    /// FID-103: DEX prices from 0x /price responses. Pair → (price, timestamp).
    pub dex_prices: Arc<RwLock<HashMap<String, (f64, std::time::Instant)>>>,
    /// FID-116: On-chain verified equity — USDC + all token balances at market prices.
    /// Updated every cycle from portfolio.account().equity.
    pub chain_equity: Arc<RwLock<f64>>,
    /// FID-117: Starting equity recorded at first boot, stored in journal.
    /// Single source of truth for P&L calculation. Never changes after first boot.
    pub starting_equity: Arc<RwLock<f64>>,
    // ---- FID-093: Command bridge fields ----
    /// Operator commands queued for the engine to drain each cycle.
    pub pending_commands: Arc<RwLock<VecDeque<crate::agent::commands::PendingCommand>>>,
    /// Runtime autonomy level override (None = use config default).
    pub autonomy_override: Arc<RwLock<Option<crate::agent::commands::AutonomyLevel>>>,
    /// Pending action awaiting operator approval (confirm/suggest mode).
    pub pending_approval: Arc<RwLock<Option<crate::agent::commands::PendingAction>>>,
    /// Operator context messages to inject into next LLM evaluation.
    pub inject_context_queue: Arc<RwLock<Vec<String>>>,
    /// Command history for undo support (last 10 commands).
    pub command_history: Arc<RwLock<VecDeque<crate::agent::commands::CommandHistoryEntry>>>,
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
    pub take_profit_2: f64,
    pub take_profit_3: f64,
    pub confidence: f64,
    pub reasoning: String,
    /// Set when execution rejects the decision (e.g. no DEX liquidity).
    /// Dashboard shows a red "REJECTED" badge when this is Some.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_status: Option<String>,
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
            equity_curve: Arc::new(RwLock::new(Vec::new())),
            stop_overrides: Arc::new(RwLock::new(HashMap::new())),
            close_overrides: Arc::new(RwLock::new(HashMap::new())),
            hunt_mode: Arc::new(RwLock::new(false)),
            monitoring_mode: Arc::new(RwLock::new(false)),
            price_staleness_secs: Arc::new(RwLock::new(0)),
            wallet_address: Arc::new(RwLock::new(String::new())),
            dex_prices: Arc::new(RwLock::new(HashMap::new())),
            chain_equity: Arc::new(RwLock::new(0.0)),
            starting_equity: Arc::new(RwLock::new(0.0)),
            // FID-093: Command bridge fields
            pending_commands: Arc::new(RwLock::new(VecDeque::new())),
            autonomy_override: Arc::new(RwLock::new(None)),
            pending_approval: Arc::new(RwLock::new(None)),
            inject_context_queue: Arc::new(RwLock::new(Vec::new())),
            command_history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Log an activity entry. Keeps last 200 entries.
    /// Uses try_write() to avoid blocking the engine if the API server holds a read lock.
    pub async fn log_activity(&self, level: ActivityLevel, pair: &str, message: &str) {
        let entry = ActivityEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level,
            pair: pair.to_string(),
            message: message.to_string(),
        };
        match self.activity_log.try_write() {
            Ok(mut log) => {
                log.push(entry);
                if log.len() > 200 {
                    log.drain(0..100);
                }
            }
            Err(_) => {
                // Lock held by API server — skip this log entry rather than stalling engine
            }
        }
    }

    /// Push a decision record. Non-blocking — skips if lock is held.
    pub fn push_decision(&self, record: DecisionRecord) {
        match self.decisions.try_write() {
            Ok(mut decisions) => {
                decisions.push(record);
                if decisions.len() > 100 {
                    decisions.drain(0..50);
                }
            }
            Err(_) => {
                // Lock held — skip rather than stall
            }
        }
    }

    /// Annotate the most recent decision for a pair with an execution status.
    /// Used when liquidity checks reject a BUY/SELL after the decision was already pushed.
    pub fn update_decision_status(&self, pair: &str, status: &str) {
        match self.decisions.try_write() {
            Ok(mut decisions) => {
                // Find the most recent decision for this pair (reverse search)
                if let Some(record) = decisions.iter_mut().rev().find(|d| d.pair == pair) {
                    record.execution_status = Some(status.to_string());
                }
            }
            Err(_) => {
                // Lock held — skip rather than stall
            }
        }
    }
}

impl Default for SharedEngineData {
    fn default() -> Self {
        Self::new()
    }
}
