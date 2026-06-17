//! Shared engine state for cross-module access (API, TUI, engine).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::jury::{JuryCycleRecord, JuryKeyHealth, JuryPoolMetrics};
use crate::core::config::RegimeSizes;
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

    // ---- FID-162: Jury observability surfaces ----
    /// Live jury state snapshot (cumulative + key health + veto flag).
    pub jury_state: Arc<RwLock<JuryStateSnapshot>>,
    /// Ring buffer of recent jury cycle records (capped 50).
    pub jury_recent: Arc<RwLock<VecDeque<JuryCycleRecord>>>,
}

/// FID-162: Live jury state for `/api/jury/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JuryStateSnapshot {
    pub enabled: bool,
    pub jury_size: usize,
    pub m3_control_active: bool,
    pub free_models_used: Vec<String>,
    pub veto_enabled: bool,
    pub veto_threshold: f64,
    pub regime_sizes: RegimeSizes,
    pub cumulative: JuryPoolMetrics,
    pub key_health: JuryKeyHealth,
    pub estimated_m3_calls: u64,
    pub estimated_free_model_calls: u64,
    pub veto_flag_active_now: bool,
    pub last_cycle_at: Option<String>, // null if never ran
    pub source: String,                // "live" | "stale" | "never_ran" | "engine_off" | "disabled"
}

impl Default for JuryStateSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            jury_size: 0,
            m3_control_active: false,
            free_models_used: vec![],
            veto_enabled: false,
            veto_threshold: 0.0,
            regime_sizes: RegimeSizes::default(),
            cumulative: JuryPoolMetrics::default(),
            key_health: JuryKeyHealth::default(),
            estimated_m3_calls: 0,
            estimated_free_model_calls: 0,
            veto_flag_active_now: false,
            last_cycle_at: None,
            source: "disabled".to_string(),
        }
    }
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
    /// FID-161: Tracks WHY the action was changed from the LLM's original response.
    /// None = LLM's original action. Some = the override that fired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_source: Option<String>,
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
    /// FID-162: Subsystem tag (`"JURY"`, `"ENGINE"`, `"RISK"`, `"EXEC"`, `"LLM"`, `"RECON"`, `"VAULT"`, `"MEM"`).
    /// `None` for legacy callers. Dashboard renders as a colored chip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
            // FID-162: jury observability surfaces
            jury_state: Arc::new(RwLock::new(JuryStateSnapshot::default())),
            jury_recent: Arc::new(RwLock::new(VecDeque::with_capacity(50))),
        }
    }

    /// Log an activity entry. Keeps last 200 entries.
    /// Uses try_write() to avoid blocking the engine if the API server holds a read lock.
    ///
    /// FID-162: `source` tags the subsystem for the dashboard. Pass `Some("JURY")`,
    /// `Some("RISK")`, etc. Pass `None` for legacy callers (will not render a source chip).
    pub async fn log_activity(
        &self,
        level: ActivityLevel,
        source: Option<&str>,
        pair: &str,
        message: &str,
    ) {
        let entry = ActivityEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level,
            source: source.map(String::from),
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

    /// FID-181: Push an equity curve snapshot. Caps in-memory curve at 200
    /// snapshots (FIFO drop). Persistence is the engine's job — see
    /// `engine::run_loop` which calls this and writes to disk.
    pub fn push_equity_snapshot(&self, snapshot: serde_json::Value) {
        match self.equity_curve.try_write() {
            Ok(mut curve) => {
                curve.push(snapshot);
                // FIFO: keep only the most recent 200 snapshots
                if curve.len() > 200 {
                    let drop_count = curve.len() - 200;
                    curve.drain(0..drop_count);
                }
            }
            Err(_) => {
                // Lock held — skip rather than stall
            }
        }
    }

    /// FID-181: Load persisted equity curve from disk on engine startup.
    /// Returns the snapshots (oldest first) or empty Vec if file doesn't exist
    /// or is malformed. Caps at 200.
    pub fn load_equity_history(path: &std::path::Path) -> Vec<serde_json::Value> {
        if !path.exists() {
            return Vec::new();
        }
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("FID-181: failed to read equity_history.json: {}", e);
                return Vec::new();
            }
        };
        let parsed: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("FID-181: failed to parse equity_history.json: {}", e);
                return Vec::new();
            }
        };
        let arr = parsed.get("snapshots").and_then(|v| v.as_array());
        let Some(arr) = arr else {
            return Vec::new();
        };
        // Take the most recent 200
        let start = if arr.len() > 200 { arr.len() - 200 } else { 0 };
        arr[start..].to_vec()
    }

    /// FID-181: Persist equity curve to disk. Atomic write (write to .tmp, rename).
    /// Called after each cycle's push_equity_snapshot. Errors are logged but
    /// don't crash the engine — the in-memory curve still works.
    pub fn save_equity_history(path: &std::path::Path, curve: &[serde_json::Value]) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let payload = serde_json::json!({
            "version": 1,
            "saved_at": chrono::Utc::now().to_rfc3339(),
            "snapshots": curve,
        });
        let serialized = match serde_json::to_string_pretty(&payload) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("FID-181: failed to serialize equity history: {}", e);
                return;
            }
        };
        let tmp_path = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp_path, &serialized) {
            tracing::warn!("FID-181: failed to write {}: {}", tmp_path.display(), e);
            return;
        }
        if let Err(e) = std::fs::rename(&tmp_path, path) {
            tracing::warn!(
                "FID-181: failed to rename {} -> {}: {}",
                tmp_path.display(),
                path.display(),
                e
            );
        }
    }
}

impl Default for SharedEngineData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn push_equity_snapshot_appends_and_caps() {
        let shared = SharedEngineData::new();
        for i in 0..205 {
            shared.push_equity_snapshot(serde_json::json!({"i": i}));
        }
        let curve = shared.equity_curve.try_read().unwrap();
        assert_eq!(curve.len(), 200, "FIFO cap should hold at 200");
        // First surviving snapshot is i=5 (0-4 dropped)
        assert_eq!(curve[0]["i"], 5);
        assert_eq!(curve[199]["i"], 204);
    }

    #[test]
    fn save_and_load_equity_history_round_trip() {
        let tmp = std::env::temp_dir().join("savant_equity_test.json");
        let _ = std::fs::remove_file(&tmp);

        // Save a small curve
        let curve: Vec<serde_json::Value> = (0..3)
            .map(|i| serde_json::json!({"timestamp": "2026-06-17T00:00:00Z", "equity": 50.0 + i as f64}))
            .collect();
        SharedEngineData::save_equity_history(&tmp, &curve);

        // Load it back
        let loaded = SharedEngineData::load_equity_history(&tmp);
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0]["equity"], 50.0);
        assert_eq!(loaded[2]["equity"], 52.0);

        // Cleanup
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(tmp.with_extension("json.tmp"));
    }

    #[test]
    fn load_equity_history_missing_file_returns_empty() {
        let nonexistent = PathBuf::from("/tmp/savant_definitely_does_not_exist_12345.json");
        let result = SharedEngineData::load_equity_history(&nonexistent);
        assert!(result.is_empty());
    }

    #[test]
    fn load_equity_history_malformed_json_returns_empty() {
        let tmp = std::env::temp_dir().join("savant_equity_malformed.json");
        std::fs::write(&tmp, "{ this is not json").unwrap();
        let result = SharedEngineData::load_equity_history(&tmp);
        assert!(
            result.is_empty(),
            "malformed JSON should return empty, not panic"
        );
        let _ = std::fs::remove_file(&tmp);
    }
}
