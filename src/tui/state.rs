//! TUI state management — snapshots, history buffers, scroll state.
//!
//! The TUI runs on a 100ms render cycle. Every cycle it takes a read-only
//! snapshot of [`SharedEngineData`] (populated by the engine every 5min tick)
//! and renders from that snapshot.  No `block_on` in the render path.

use std::time::Instant;

use crate::core::config::AppConfig;
use crate::core::shared::{ActivityEntry, DecisionRecord, MemorySnapshot, SharedEngineData};
use crate::core::types::{AccountState, Position, TradeRecord};
use crate::insight::aggregator::MarketContext;

// ---------------------------------------------------------------------------
// Rolling history buffer
// ---------------------------------------------------------------------------

/// Fixed-capacity rolling buffer for time-series data (equity, balance, etc.).
pub struct HistoryBuffer<T> {
    data: Vec<T>,
    capacity: usize,
}

impl<T: Clone> HistoryBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.data.len() >= self.capacity {
            self.data.remove(0);
        }
        self.data.push(item);
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Last N items (most recent first).
    pub fn last_n(&self, n: usize) -> Vec<T> {
        let n = n.min(self.data.len());
        self.data[self.data.len() - n..]
            .iter()
            .rev()
            .cloned()
            .collect()
    }
}

// ---------------------------------------------------------------------------
// TuiSnapshot  (immutable copy of engine state at render time)
// ---------------------------------------------------------------------------

/// Frozen snapshot of all engine data at a single render frame.
pub struct TuiSnapshot {
    pub account: AccountState,
    pub positions: Vec<Position>,
    pub decisions: Vec<DecisionRecord>,
    pub insight: MarketContext,
    pub activity: Vec<ActivityEntry>,
    pub closed_trades: Vec<TradeRecord>,
    pub memory: MemorySnapshot,
    // Config-derived (written once at init, never changes)
    pub backend_name: String,
    pub mode_label: String,
    pub starting_balance: f64,
    pub model_name: String,
    pub max_drawdown: f64,
    pub max_daily_loss: f64,
}

impl TuiSnapshot {
    /// Create an empty snapshot with config-derived labels.
    pub fn new(config: &AppConfig) -> Self {
        let backend_name = match config.exchange.backend.as_str() {
            "kraken" => "Kraken CEX",
            "0x" => "0x DEX",
            "1inch" => "1inch DEX",
            _ => &config.exchange.backend,
        };
        let mode_label = if config.mode.paper_trading {
            "Paper Trading"
        } else {
            "LIVE"
        };

        Self {
            account: AccountState::new(0.0),
            positions: Vec::new(),
            decisions: Vec::new(),
            insight: MarketContext::default(),
            activity: Vec::new(),
            closed_trades: Vec::new(),
            memory: MemorySnapshot {
                brier_score: None,
                brier_label: "No data".to_string(),
                confidence_cap: "LOW".to_string(),
                total_trades: 0,
                cusum_status: Vec::new(),
                replay_lesson_count: 0,
            },
            backend_name: backend_name.to_string(),
            mode_label: mode_label.to_string(),
            starting_balance: config.trading.starting_balance,
            model_name: config.ai.model.clone(),
            max_drawdown: config.risk.max_drawdown * 100.0,
            max_daily_loss: config.risk.max_daily_loss * 100.0,
        }
    }

    /// Refresh all fields from shared engine data.
    pub fn refresh(&mut self, shared: &SharedEngineData) {
        // Use try_read to avoid blocking the render loop
        self.account = shared
            .account
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| AccountState::new(0.0));

        self.positions = shared
            .positions
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_default();

        self.decisions = shared
            .decisions
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_default();

        self.insight = shared
            .insight
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_default();

        self.activity = shared
            .activity_log
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_default();

        self.closed_trades = shared
            .closed_trades
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_default();

        self.memory = shared
            .memory_snapshot
            .try_read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| MemorySnapshot {
                brier_score: None,
                brier_label: "No data".to_string(),
                confidence_cap: "LOW".to_string(),
                total_trades: 0,
                cusum_status: Vec::new(),
                replay_lesson_count: 0,
            });
    }
}

// ---------------------------------------------------------------------------
// Tab view state  (scroll, search, sort per tab)
// ---------------------------------------------------------------------------

/// Sort direction for table columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDir {
    Asc,
    Desc,
}

/// Per-tab scrolling, search, sort, and selection state.
pub struct TabViewState {
    pub scroll_offset: usize,
    pub search_query: String,
    pub search_active: bool,
    pub sort_column: Option<usize>,
    pub sort_dir: SortDir,
    pub selected_row: Option<usize>,
    pub detail_open: bool,
    pub tail_mode: bool, // For activity log auto-scroll
}

impl Default for TabViewState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            search_query: String::new(),
            search_active: false,
            sort_column: None,
            sort_dir: SortDir::Desc,
            selected_row: None,
            detail_open: false,
            tail_mode: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Global TUI state
// ---------------------------------------------------------------------------

/// Global TUI runtime state — active tab, history, timing.
pub struct TuiState {
    pub active_tab: usize,
    pub equity_history: HistoryBuffer<u64>,
    pub balance_history: HistoryBuffer<u64>,
    pub start_time: Instant,
    pub tab_states: Vec<TabViewState>,
    pub snapshot: TuiSnapshot,
    pub show_help: bool,
}

impl TuiState {
    pub fn new(config: &AppConfig) -> Self {
        let tab_count = 10; // tabs 0-9
        let mut tab_states = Vec::with_capacity(tab_count);
        for _ in 0..tab_count {
            tab_states.push(TabViewState::default());
        }

        Self {
            active_tab: 1, // Start on Overview (tab 1)
            equity_history: HistoryBuffer::new(120),
            balance_history: HistoryBuffer::new(200),
            start_time: Instant::now(),
            tab_states,
            snapshot: TuiSnapshot::new(config),
            show_help: false,
        }
    }

    /// Refresh snapshot from shared data and push equity/balance to history.
    pub fn refresh_from(&mut self, shared: &SharedEngineData) {
        self.snapshot.refresh(shared);

        let eq = self.snapshot.account.equity;
        if eq > 0.0 {
            self.equity_history.push((eq * 100.0) as u64);
        }

        let bal = self.snapshot.account.balance;
        if bal > 0.0 {
            self.balance_history.push((bal * 100.0) as u64);
        }
    }

    pub fn active_tab_state(&mut self) -> &mut TabViewState {
        &mut self.tab_states[self.active_tab]
    }
}
