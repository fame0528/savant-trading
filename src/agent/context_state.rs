//! Context State Manager (FID-085 Phases 3 + 5)
//!
//! Manages cross-cycle state: delta-compression, anti-thrashing, microcompaction,
//! TTL-based pruning, and historical data stripping.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Result of delta-compression analysis.
pub enum DeltaResult {
    /// No significant change — skip full data injection
    NoChange,
    /// Small change — inject only the delta
    Delta(String),
    /// Large change or first cycle — inject full data
    Full(String),
}

/// Data block with TTL for pruning.
#[derive(Clone)]
pub struct DataBlock {
    pub content: String,
    pub created_at: Instant,
    pub ttl: Duration,
    pub block_type: String,
}

/// Cross-cycle context state (FID-085 Phases 3 + 5).
pub struct ContextState {
    /// Previous cycle's eyes text hash for delta detection
    previous_hash: Option<u64>,
    /// Previous cycle's eyes text for diff computation
    previous_text: Option<String>,
    /// Compression efficiency history (savings %)
    compression_history: Vec<f64>,
    /// Cycle counter
    cycle_count: u64,
    /// Data blocks with TTL for pruning
    data_blocks: Vec<DataBlock>,
    /// Soft trim ratio (0.30 = 30%)
    soft_trim_ratio: f64,
    /// Hard clear ratio (0.50 = 50%)
    hard_clear_ratio: f64,
}

impl ContextState {
    pub fn new(soft_trim_ratio: f64, hard_clear_ratio: f64) -> Self {
        Self {
            previous_hash: None,
            previous_text: None,
            compression_history: Vec::new(),
            cycle_count: 0,
            data_blocks: Vec::new(),
            soft_trim_ratio,
            hard_clear_ratio,
        }
    }

    /// Compute delta-compression result (FID-085 Phase 3, Item 11).
    /// Compares current eyes text against previous cycle.
    pub fn compute_delta(&mut self, current_text: &str, threshold: f64) -> DeltaResult {
        let current_hash = self.hash_text(current_text);

        // First cycle or no previous state — full injection
        let prev_hash = match self.previous_hash {
            Some(h) => h,
            None => {
                self.store_state(current_text, current_hash);
                return DeltaResult::Full(current_text.to_string());
            }
        };

        // Hash match — no change at all
        if current_hash == prev_hash {
            debug!("Delta-compression: identical hash, skipping full data");
            self.record_compression(1.0); // 100% savings
            return DeltaResult::NoChange;
        }

        // Compute text diff ratio
        let prev_text = self.previous_text.clone().unwrap_or_default();
        let diff_ratio = self.text_diff_ratio(&prev_text, current_text);

        self.record_compression(1.0 - diff_ratio);

        if diff_ratio < threshold {
            // Small change — inject delta
            let delta = self.extract_changes(&prev_text, current_text);
            info!(
                "Delta-compression: {:.1}% change (threshold {:.1}%) — injecting delta only",
                diff_ratio * 100.0,
                threshold * 100.0
            );
            self.store_state(current_text, current_hash);
            DeltaResult::Delta(delta)
        } else {
            // Large change — full injection (regime shift)
            info!(
                "Delta-compression: {:.1}% change — full injection (regime shift)",
                diff_ratio * 100.0
            );
            self.store_state(current_text, current_hash);
            DeltaResult::Full(current_text.to_string())
        }
    }

    /// Anti-thrashing check (FID-085 Phase 3, Item 12).
    /// Returns true if compression should be skipped.
    pub fn should_skip_compression(&self, min_savings: f64) -> bool {
        if self.compression_history.len() < 2 {
            return false;
        }
        let last_two = &self.compression_history[self.compression_history.len() - 2..];
        let both_inefficient = last_two.iter().all(|&s| s < min_savings);
        if both_inefficient {
            warn!(
                "Anti-thrashing: last 2 compressions saved <{:.0}% each — skipping",
                min_savings * 100.0
            );
        }
        both_inefficient
    }

    /// Microcompaction: soft trim (FID-085 Phase 5, Item 17).
    /// Trims middle sections of old data blocks when context exceeds budget.
    pub fn soft_trim(&self, text: &str, budget_chars: usize) -> String {
        let current_chars = text.len();
        let threshold = (budget_chars as f64 * self.soft_trim_ratio) as usize;
        if current_chars <= threshold {
            return text.to_string();
        }

        // Trim middle sections, keep head and tail
        let head_size = 1500;
        let tail_size = 1500;
        if current_chars <= head_size + tail_size + 20 {
            return text.to_string();
        }

        let head = &text[..head_size];
        let tail = &text[current_chars - tail_size..];
        format!("{}\n...[trimmed {} chars]...\n{}", head, current_chars - head_size - tail_size, tail)
    }

    /// Microcompaction: hard clear (FID-085 Phase 5, Item 18).
    /// Replaces old data blocks with placeholder when context is critically large.
    pub fn hard_clear(&self, text: &str, budget_chars: usize) -> String {
        let current_chars = text.len();
        let threshold = (budget_chars as f64 * self.hard_clear_ratio) as usize;
        if current_chars <= threshold {
            return text.to_string();
        }

        // Keep only the last 3000 chars, replace the rest
        let keep_chars = 3000.min(current_chars);
        let tail = &text[current_chars - keep_chars..];
        format!(
            "[Historical data pruned — {} chars removed. See decision log for prior analysis.]\n\n{}",
            current_chars - keep_chars,
            tail
        )
    }

    /// TTL-based pruning (FID-085 Phase 5, Item 19).
    /// Remove data blocks older than their TTL.
    pub fn prune_expired(&mut self) {
        let now = Instant::now();
        let before = self.data_blocks.len();
        self.data_blocks.retain(|block| now.duration_since(block.created_at) < block.ttl);
        let pruned = before - self.data_blocks.len();
        if pruned > 0 {
            debug!("TTL pruning: removed {} expired data blocks", pruned);
        }
    }

    /// Add a data block with TTL.
    pub fn add_data_block(&mut self, block: DataBlock) {
        self.data_blocks.push(block);
    }

    /// Get the current cycle count.
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    /// Increment cycle counter.
    pub fn increment_cycle(&mut self) {
        self.cycle_count += 1;
    }

    /// Historical data stripping (FID-085 Phase 3, Item 13).
    /// Replaces old data with summary placeholder.
    pub fn strip_historical(&self, text: &str, max_age_cycles: u64) -> String {
        if self.cycle_count <= max_age_cycles {
            return text.to_string();
        }
        // For now, just return as-is — the actual stripping happens via TTL pruning
        // and soft/hard trim. This method is the hook for future enhancement.
        text.to_string()
    }

    fn store_state(&mut self, text: &str, hash: u64) {
        self.previous_text = Some(text.to_string());
        self.previous_hash = Some(hash);
    }

    fn hash_text(&self, text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    fn text_diff_ratio(&self, old: &str, new: &str) -> f64 {
        if old.is_empty() || new.is_empty() {
            return 1.0;
        }
        let max_len = old.len().max(new.len());
        if max_len == 0 {
            return 0.0;
        }
        // Simple character-level diff ratio
        let matching = old.chars().zip(new.chars()).filter(|(a, b)| a == b).count();
        1.0 - (matching as f64 / max_len as f64)
    }

    fn extract_changes(&self, old: &str, new: &str) -> String {
        // Simple delta: find lines that changed
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        let mut delta = String::new();
        for (i, line) in new_lines.iter().enumerate() {
            if i >= old_lines.len() || old_lines[i] != *line {
                delta.push_str(line);
                delta.push('\n');
            }
        }
        if delta.is_empty() {
            "No change since last cycle".to_string()
        } else {
            format!("Since last cycle:\n{}", delta)
        }
    }

    fn record_compression(&mut self, savings: f64) {
        self.compression_history.push(savings);
        if self.compression_history.len() > 10 {
            self.compression_history.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_first_cycle_full() {
        let mut state = ContextState::new(0.30, 0.50);
        let result = state.compute_delta("hello world", 0.02);
        assert!(matches!(result, DeltaResult::Full(_)));
    }

    #[test]
    fn delta_identical_no_change() {
        let mut state = ContextState::new(0.30, 0.50);
        state.compute_delta("hello world", 0.02);
        let result = state.compute_delta("hello world", 0.02);
        assert!(matches!(result, DeltaResult::NoChange));
    }

    #[test]
    fn delta_small_change() {
        let mut state = ContextState::new(0.30, 0.50);
        let old = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10";
        let new = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline11";
        state.compute_delta(old, 0.20);
        let result = state.compute_delta(new, 0.20);
        assert!(matches!(result, DeltaResult::Delta(_)));
    }

    #[test]
    fn anti_thrashing_blocks() {
        let mut state = ContextState::new(0.30, 0.50);
        // Simulate 2 inefficient compressions (savings < 10%)
        state.record_compression(0.05);
        state.record_compression(0.08);
        assert!(state.should_skip_compression(0.10));
    }

    #[test]
    fn anti_thrashing_allows_good_compression() {
        let mut state = ContextState::new(0.30, 0.50);
        state.record_compression(0.50);
        state.record_compression(0.05);
        assert!(!state.should_skip_compression(0.10));
    }

    #[test]
    fn soft_trim_reduces_size() {
        let state = ContextState::new(0.30, 0.50);
        let text = "a".repeat(5000);
        let trimmed = state.soft_trim(&text, 3000);
        assert!(trimmed.len() < text.len());
        assert!(trimmed.contains("[trimmed"));
    }

    #[test]
    fn soft_trim_no_change_under_threshold() {
        let state = ContextState::new(0.30, 0.50);
        let text = "short text";
        let trimmed = state.soft_trim(&text, 10000);
        assert_eq!(trimmed, text);
    }

    #[test]
    fn hard_clear_replaces_old_data() {
        let state = ContextState::new(0.30, 0.50);
        let text = "a".repeat(10000);
        let cleared = state.hard_clear(&text, 5000);
        assert!(cleared.contains("Historical data pruned"));
        assert!(cleared.len() < text.len());
    }

    #[test]
    fn ttl_pruning_removes_expired() {
        let mut state = ContextState::new(0.30, 0.50);
        state.add_data_block(DataBlock {
            content: "old data".to_string(),
            created_at: Instant::now() - Duration::from_secs(600),
            ttl: Duration::from_secs(300),
            block_type: "price".to_string(),
        });
        state.add_data_block(DataBlock {
            content: "new data".to_string(),
            created_at: Instant::now(),
            ttl: Duration::from_secs(300),
            block_type: "price".to_string(),
        });
        state.prune_expired();
        assert_eq!(state.data_blocks.len(), 1);
        assert_eq!(state.data_blocks[0].content, "new data");
    }

    #[test]
    fn cycle_counter() {
        let mut state = ContextState::new(0.30, 0.50);
        assert_eq!(state.cycle_count(), 0);
        state.increment_cycle();
        state.increment_cycle();
        assert_eq!(state.cycle_count(), 2);
    }
}
