//! Decision Log (FID-085 Phase 4, Item 16)
//!
//! Append-only markdown log with atomic writes and auto-rotation.
//! Stores trade decisions at evaluation time, updates with outcomes when trades close.
//!
//! Reference: TradingAgents `memory.py` pattern.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// A single decision log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEntry {
    pub timestamp: String,
    pub pair: String,
    pub action: String,
    pub confidence: f64,
    pub risk_reward: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub reasoning: String,
    pub outcome: Option<TradeOutcome>,
}

/// Outcome of a closed trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOutcome {
    pub raw_return_pct: f64,
    pub alpha_vs_benchmark: f64,
    pub reflection: String,
}

/// Append-only decision log with atomic writes and rotation.
pub struct DecisionLog {
    path: PathBuf,
    max_entries: usize,
    entries: Vec<DecisionEntry>,
}

impl DecisionLog {
    /// Open or create a decision log at the given path.
    pub fn open(path: impl AsRef<Path>, max_entries: usize) -> Self {
        let path = path.as_ref().to_path_buf();
        let entries = if path.exists() {
            Self::load_entries(&path).unwrap_or_default()
        } else {
            Vec::new()
        };
        Self {
            path,
            max_entries,
            entries,
        }
    }

    /// Append a new decision entry (atomic write).
    pub fn append(&mut self, entry: DecisionEntry) {
        self.entries.push(entry);
        self.rotate_if_needed();
        self.flush();
    }

    /// Update the most recent entry for a pair with outcome.
    pub fn update_outcome(&mut self, pair: &str, outcome: TradeOutcome) {
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| e.pair == pair && e.outcome.is_none()) {
            entry.outcome = Some(outcome);
            self.flush();
        }
    }

    /// Get recent entries for context injection.
    /// Returns same-pair full entries and cross-pair reflections only.
    pub fn context_for_pair(&self, pair: &str, max_same: usize, max_cross: usize) -> String {
        let mut msg = String::new();
        let same: Vec<&DecisionEntry> = self.entries.iter()
            .rev()
            .filter(|e| e.pair == pair)
            .take(max_same)
            .collect();
        let cross: Vec<&DecisionEntry> = self.entries.iter()
            .rev()
            .filter(|e| e.pair != pair && e.outcome.is_some())
            .take(max_cross)
            .collect();

        if !same.is_empty() {
            msg.push_str("### Recent Decisions (This Pair)\n");
            for entry in &same {
                msg.push_str(&format!(
                    "- [{}] {} {} (conf={:.0}%, R:R={:.1}) — {}\n",
                    entry.timestamp, entry.pair, entry.action,
                    entry.confidence * 100.0, entry.risk_reward, entry.reasoning,
                ));
                if let Some(ref out) = entry.outcome {
                    msg.push_str(&format!(
                        "  → Return: {:.2}% | Reflection: {}\n",
                        out.raw_return_pct, out.reflection,
                    ));
                }
            }
        }

        if !cross.is_empty() {
            msg.push_str("### Cross-Pair Lessons\n");
            for entry in &cross {
                if let Some(ref out) = entry.outcome {
                    msg.push_str(&format!(
                        "- [{}] {}: {} → {:.2}% — {}\n",
                        entry.timestamp, entry.pair, entry.action,
                        out.raw_return_pct, out.reflection,
                    ));
                }
            }
        }

        msg
    }

    /// Number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn rotate_if_needed(&mut self) {
        if self.entries.len() > self.max_entries {
            let drain_count = self.entries.len() - self.max_entries;
            self.entries.drain(..drain_count);
        }
    }

    /// Atomic flush: write to temp file, then rename.
    fn flush(&self) {
        let content = serde_json::to_string_pretty(&self.entries).unwrap_or_default();
        let tmp_path = self.path.with_extension("tmp");
        if let Ok(mut f) = fs::File::create(&tmp_path) {
            let _ = f.write_all(content.as_bytes());
            let _ = f.sync_all();
            let _ = fs::rename(&tmp_path, &self.path);
        }
    }

    fn load_entries(path: &Path) -> Option<Vec<DecisionEntry>> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::env;

    fn tmp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("savant_test_{}.json", name))
    }

    fn sample_entry(pair: &str, action: &str) -> DecisionEntry {
        DecisionEntry {
            timestamp: Utc::now().to_rfc3339(),
            pair: pair.to_string(),
            action: action.to_string(),
            confidence: 0.75,
            risk_reward: 2.5,
            stop_loss: 2400.0,
            take_profit: 2500.0,
            reasoning: "Test reasoning".to_string(),
            outcome: None,
        }
    }

    #[test]
    fn append_and_read() {
        let path = tmp_path("append_read");
        let _ = fs::remove_file(&path);
        let mut log = DecisionLog::open(&path, 100);
        log.append(sample_entry("ETH/USD", "BUY"));
        assert_eq!(log.len(), 1);

        let ctx = log.context_for_pair("ETH/USD", 5, 5);
        assert!(ctx.contains("ETH/USD"));
        assert!(ctx.contains("BUY"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn rotation_drops_old() {
        let path = tmp_path("rotation");
        let _ = fs::remove_file(&path);
        let mut log = DecisionLog::open(&path, 3);
        for i in 0..5 {
            log.append(sample_entry(&format!("PAIR_{}", i), "BUY"));
        }
        assert_eq!(log.len(), 3);
        // First two should be rotated out
        assert!(!log.context_for_pair("PAIR_0", 5, 5).contains("PAIR_0"));
        assert!(log.context_for_pair("PAIR_4", 5, 5).contains("PAIR_4"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn outcome_update() {
        let path = tmp_path("outcome");
        let _ = fs::remove_file(&path);
        let mut log = DecisionLog::open(&path, 100);
        log.append(sample_entry("BTC/USD", "BUY"));
        log.update_outcome("BTC/USD", TradeOutcome {
            raw_return_pct: 3.5,
            alpha_vs_benchmark: 1.2,
            reflection: "Good entry timing".to_string(),
        });
        let ctx = log.context_for_pair("BTC/USD", 5, 5);
        assert!(ctx.contains("3.50%"));
        assert!(ctx.contains("Good entry timing"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn cross_pair_reflections() {
        let path = tmp_path("cross_pair");
        let _ = fs::remove_file(&path);
        let mut log = DecisionLog::open(&path, 100);
        log.append(sample_entry("ETH/USD", "BUY"));
        log.update_outcome("ETH/USD", TradeOutcome {
            raw_return_pct: 2.0,
            alpha_vs_benchmark: 0.5,
            reflection: "Lesson learned".to_string(),
        });
        log.append(sample_entry("BTC/USD", "BUY"));
        let ctx = log.context_for_pair("BTC/USD", 5, 5);
        assert!(ctx.contains("Cross-Pair"));
        assert!(ctx.contains("ETH/USD"));
        let _ = fs::remove_file(&path);
    }
}
