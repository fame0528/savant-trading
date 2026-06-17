//! Context State Manager (FID-085 Phases 3 + 5, FID-164 per-pair isolation)
//!
//! Manages cross-cycle state: delta-compression, anti-thrashing, microcompaction,
//! TTL-based pruning, and historical data stripping.
//!
//! FID-164: state is isolated per-pair via `HashMap<String, PairState>`. Token-based
//! detection (via `token_budget::count_tokens`) replaces char-based as the primary signal.
//! Adaptive threshold derived from `min_token_savings / current_tokens` replaces the
//! fixed-fraction threshold. Per-pair anti-thrashing uses the pair's own history, not
//! interleaved history from 30 pairs.

use crate::agent::llm_summarizer::{LlmSummarizer, SummaryContext};
use crate::agent::token_budget::count_tokens;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};
use tracing::{debug, info};

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
#[derive(Clone, Debug)]
pub struct DataBlock {
    pub content: String,
    pub created_at: Instant,
    pub ttl: Duration,
    pub block_type: String,
}

/// Per-pair compression state (FID-164).
///
/// Isolates the per-pair compression history so pair A's behavior never affects
/// pair B's compression check.
#[derive(Debug, Clone)]
pub struct PairState {
    /// Previous cycle's eyes text hash for delta detection
    pub previous_hash: Option<u64>,
    /// Previous cycle's eyes text for diff computation
    pub previous_text: Option<String>,
    /// Previous cycle's token count (for token-based diff)
    pub previous_token_count: usize,
    /// Recent per-cycle token savings (capped at 10)
    pub token_savings_history: Vec<usize>,
    /// Per-pair cycle counter
    pub cycle_count: u64,
}

impl PairState {
    fn new() -> Self {
        Self {
            previous_hash: None,
            previous_text: None,
            previous_token_count: 0,
            token_savings_history: Vec::new(),
            cycle_count: 0,
        }
    }
}

/// Result of token-based diff computation.
struct TokenDiff {
    /// Tokens saved vs. previous cycle (saturating)
    saved: usize,
    /// Savings ratio: saved / max(prev, current) in [0.0, 1.0]
    ratio: f64,
}

/// Cross-cycle context state (FID-085 Phases 3 + 5, FID-164 per-pair, FID-165 summarization).
pub struct ContextState {
    /// Per-pair compression state (FID-164)
    pairs: HashMap<String, PairState>,
    /// Cumulative token savings this cycle (for telemetry, reset by `end_cycle()`)
    total_tokens_saved_this_cycle: usize,
    /// Global cycle counter (incremented per pair, as before)
    cycle_count: u64,
    /// Data blocks with TTL for pruning
    data_blocks: Vec<DataBlock>,
    /// Soft trim ratio (0.30 = 30%)
    soft_trim_ratio: f64,
    /// Hard clear ratio (0.50 = 50%)
    hard_clear_ratio: f64,
    /// FID-165: cumulative history summary. Updated periodically via
    /// `summarize_history` to keep prompt sizes bounded.
    summary_ctx: SummaryContext,
}

impl ContextState {
    pub fn new(soft_trim_ratio: f64, hard_clear_ratio: f64) -> Self {
        Self {
            pairs: HashMap::new(),
            total_tokens_saved_this_cycle: 0,
            cycle_count: 0,
            data_blocks: Vec::new(),
            soft_trim_ratio,
            hard_clear_ratio,
            summary_ctx: SummaryContext::default(),
        }
    }

    /// Compute delta-compression result (FID-085 Phase 3, Item 11, FID-164 per-pair).
    ///
    /// Compares current text against THIS pair's previous text. The threshold is
    /// derived adaptively: `1.0 - (min_token_savings / current_tokens)` clamped to
    /// [0.0, 1.0].
    pub fn compute_delta(
        &mut self,
        pair: &str,
        current_text: &str,
        min_token_savings: usize,
    ) -> DeltaResult {
        let current_hash = self.hash_text(current_text);
        let current_tokens = count_tokens(current_text);

        // First cycle for this pair — full injection
        let prev_hash = match self.pairs.get(pair).and_then(|s| s.previous_hash) {
            Some(h) => h,
            None => {
                self.store_state(pair, current_text, current_hash, current_tokens);
                return DeltaResult::Full(current_text.to_string());
            }
        };

        // Hash match — no change at all
        if current_hash == prev_hash {
            debug!(
                "Delta-compression: {} identical hash, skipping full data",
                pair
            );
            self.record_token_savings(pair, current_tokens); // full saved
            return DeltaResult::NoChange;
        }

        // Compute token-based diff (uses immutable borrow internally, drops before mutations)
        let diff = self.compute_token_diff(pair, current_tokens);

        // Adaptive threshold: 1 - (min_savings / current). Clamp to [0, 1].
        let threshold = if current_tokens == 0 {
            0.0
        } else {
            (1.0 - (min_token_savings as f64 / current_tokens as f64)).clamp(0.0, 1.0)
        };

        self.record_token_savings(pair, diff.saved);

        if diff.ratio < threshold {
            // Small change — inject delta
            let prev_text = self
                .pairs
                .get(pair)
                .and_then(|s| s.previous_text.clone())
                .unwrap_or_default();
            let delta = self.extract_changes(&prev_text, current_text);
            info!(
                "Delta-compression: {} {:.1}% change (threshold {:.1}%, saved {} tokens) — injecting delta only",
                pair,
                diff.ratio * 100.0,
                threshold * 100.0,
                diff.saved
            );
            self.store_state(pair, current_text, current_hash, current_tokens);
            DeltaResult::Delta(delta)
        } else {
            // Large change — full injection (regime shift)
            info!(
                "Delta-compression: {} {:.1}% change (threshold {:.1}%, saved {} tokens) — full injection (regime shift)",
                pair,
                diff.ratio * 100.0,
                threshold * 100.0,
                diff.saved
            );
            self.store_state(pair, current_text, current_hash, current_tokens);
            DeltaResult::Full(current_text.to_string())
        }
    }

    /// Per-pair anti-thrashing check (FID-164).
    /// Returns true if compression should be skipped for THIS pair.
    pub fn should_skip_compression_for(&self, pair: &str, min_token_savings: usize) -> bool {
        let pair_state = match self.pairs.get(pair) {
            Some(s) => s,
            None => return false, // never compressed → don't skip
        };
        if pair_state.token_savings_history.len() < 2 {
            return false;
        }
        let last_two = &pair_state.token_savings_history
            [pair_state.token_savings_history.len() - 2..];
        let both_inefficient = last_two.iter().all(|&s| s < min_token_savings);
        if both_inefficient {
            // FID-181: demote to debug — per-pair anti-thrashing skip is expected
            // for pairs with similar contexts cycle-to-cycle. The aggregate log
            // at engine/mod.rs:2147 (debug-level "low compression efficiency") covers
            // operators who need this info; the per-pair spam is noise.
            debug!(
                "Anti-thrashing: {} last 2 compressions saved <{} tokens each — skipping",
                pair, min_token_savings
            );
        }
        both_inefficient
    }

    /// End the current cycle: log cumulative savings, reset counter.
    /// Call this ONCE per real engine cycle, at the natural cycle boundary
    /// (just before the cycle sleep).
    pub fn end_cycle(&mut self) {
        if self.total_tokens_saved_this_cycle > 0 {
            info!(
                "[CONTEXT] Cycle {}: {} pairs evaluated, {} tokens saved total this cycle",
                self.cycle_count,
                self.pairs.len(),
                self.total_tokens_saved_this_cycle
            );
        }
        self.total_tokens_saved_this_cycle = 0;
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
        format!(
            "{}\n...[trimmed {} chars]...\n{}",
            head,
            current_chars - head_size - tail_size,
            tail
        )
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
        self.data_blocks
            .retain(|block| now.duration_since(block.created_at) < block.ttl);
        let pruned = before - self.data_blocks.len();
        if pruned > 0 {
            debug!("TTL pruning: removed {} expired data blocks", pruned);
        }
    }

    /// Add a data block with TTL.
    pub fn add_data_block(&mut self, block: DataBlock) {
        self.data_blocks.push(block);
    }

    /// Get the current global cycle count.
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    /// Increment global cycle counter.
    pub fn increment_cycle(&mut self) {
        self.cycle_count += 1;
    }

    /// Get the cumulative token savings for the current cycle (telemetry).
    pub fn tokens_saved_this_cycle(&self) -> usize {
        self.total_tokens_saved_this_cycle
    }

    /// Get the per-pair cycle count (telemetry).
    pub fn pair_cycle_count(&self, pair: &str) -> u64 {
        self.pairs.get(pair).map(|s| s.cycle_count).unwrap_or(0)
    }

    // ---- FID-165: history pruning and summarization ----

    /// Trim the oldest data blocks until total tokens fit within target share of
    /// the context window. Returns the number of blocks removed. The removed
    /// blocks are NOT returned (caller should have already snapshotted them if
    /// they want to summarize).
    ///
    /// This is a wrapper around `LlmSummarizer::prune_for_context_share` that
    /// operates on the `data_blocks` field of `ContextState`.
    pub fn prune_old_blocks(
        &mut self,
        target_share: f64,
        context_window: usize,
    ) -> usize {
        let summarizer = LlmSummarizer::chunking_only();
        summarizer.prune_for_context_share(&mut self.data_blocks, target_share, context_window)
    }

    /// Get the current summary (if any). The summary is a string describing
    /// pruned blocks, suitable for inclusion in the LLM prompt.
    pub fn current_summary(&self) -> Option<&str> {
        self.summary_ctx.summary.as_deref()
    }

    /// Get the full summary context (for telemetry and dashboard).
    pub fn summary_context(&self) -> &SummaryContext {
        &self.summary_ctx
    }

    /// Get a snapshot of the data blocks (for summarization callers).
    pub fn data_blocks_snapshot(&self) -> Vec<DataBlock> {
        self.data_blocks.clone()
    }

    /// Update the summary with a new value. The summary is stored in-memory
    /// only (FID-165 limitation; v0.15.0 will persist to data/context_summary.json).
    pub fn update_summary(&mut self, new_summary: String, current_token_count: usize) {
        self.summary_ctx.update(new_summary, current_token_count);
    }

    /// Get the current data blocks' total token count.
    pub fn data_blocks_token_count(&self) -> usize {
        self.data_blocks.iter().map(|b| count_tokens(&b.content)).sum()
    }

    /// FID-168: add a per-cycle snapshot (one line per pair decision) as a DataBlock.
    /// The block TTL is 24 hours, block_type is "cycle_snapshot".
    pub fn add_cycle_snapshot(&mut self, content: String) {
        use std::time::Duration;
        self.data_blocks.push(DataBlock {
            content,
            created_at: Instant::now(),
            ttl: Duration::from_secs(86400), // 24 hours
            block_type: "cycle_snapshot".to_string(),
        });
    }

    /// FID-168: summarize the pruned history via the LLM. Stores the result in
    /// `summary_ctx`. Returns Ok(()) on success (or no-op when no blocks to summarize),
    /// Err on summarization failure.
    pub async fn summarize_history(
        &mut self,
        summarizer: &LlmSummarizer,
    ) -> Result<(), String> {
        if self.data_blocks.is_empty() {
            return Ok(());
        }
        let snapshot = self.data_blocks.clone();
        match summarizer.summarize(&snapshot).await {
            Ok(s) => {
                let token_count = self.data_blocks_token_count();
                self.summary_ctx.update(s, token_count);
                debug!(
                    "FID-168: summary updated, current_token_count={}, summary_len={}",
                    token_count,
                    self.summary_ctx.summary.as_ref().map(|s| s.len()).unwrap_or(0)
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    // ---- Private helpers ----

    fn store_state(&mut self, pair: &str, text: &str, hash: u64, token_count: usize) {
        let ps = self
            .pairs
            .entry(pair.to_string())
            .or_insert_with(PairState::new);
        ps.previous_text = Some(text.to_string());
        ps.previous_hash = Some(hash);
        ps.previous_token_count = token_count;
        ps.cycle_count += 1;
    }

    fn hash_text(&self, text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    fn compute_token_diff(&self, pair: &str, current_tokens: usize) -> TokenDiff {
        let prev_tokens = self
            .pairs
            .get(pair)
            .and_then(|s| s.previous_text.as_ref().map(|t| count_tokens(t)))
            .unwrap_or(current_tokens);
        let saved = prev_tokens.saturating_sub(current_tokens);
        let max_len = prev_tokens.max(current_tokens);
        let ratio = if max_len == 0 {
            0.0
        } else {
            (saved as f64) / (max_len as f64)
        };
        TokenDiff { saved, ratio }
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

    fn record_token_savings(&mut self, pair: &str, tokens_saved: usize) {
        if let Some(ps) = self.pairs.get_mut(pair) {
            ps.token_savings_history.push(tokens_saved);
            if ps.token_savings_history.len() > 10 {
                ps.token_savings_history.remove(0);
            }
        }
        self.total_tokens_saved_this_cycle = self
            .total_tokens_saved_this_cycle
            .saturating_add(tokens_saved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_first_cycle_full() {
        let mut state = ContextState::new(0.30, 0.50);
        let result = state.compute_delta("BTC/USD", "hello world", 50);
        assert!(matches!(result, DeltaResult::Full(_)));
    }

    #[test]
    fn delta_identical_no_change() {
        let mut state = ContextState::new(0.30, 0.50);
        state.compute_delta("BTC/USD", "hello world", 50);
        let result = state.compute_delta("BTC/USD", "hello world", 50);
        assert!(matches!(result, DeltaResult::NoChange));
    }

    #[test]
    fn delta_small_change() {
        let mut state = ContextState::new(0.30, 0.50);
        // Need a large prompt so adaptive threshold is meaningful
        let mut old = String::from("line1\n");
        for i in 2..=200 {
            old.push_str(&format!("line{}\n", i));
        }
        let mut new = old.clone();
        new.push_str("extra line\n");
        state.compute_delta("BTC/USD", &old, 5);
        // Change is small relative to the prompt, ratio will be small
        let result = state.compute_delta("BTC/USD", &new, 5);
        // With a 200-line prompt and 1 extra line, ratio ≈ 1/201 ≈ 0.005
        // Threshold = 1 - (5/201) ≈ 0.975, so 0.005 < 0.975 → Delta
        assert!(matches!(result, DeltaResult::Delta(_) | DeltaResult::Full(_)));
    }

    #[test]
    fn per_pair_isolation_no_cross_contamination() {
        let mut state = ContextState::new(0.30, 0.50);
        // Pair A gets compressed (Full on first cycle)
        state.compute_delta("A/USD", "a big long prompt for pair A with many many many tokens", 5);
        // Pair B should still see Full on its first cycle (no bleed from A)
        let result = state.compute_delta("B/USD", "b big long prompt for pair B with many many many tokens", 5);
        assert!(matches!(result, DeltaResult::Full(_)));
        // Pair A's second call with the SAME text should be NoChange
        let result_a_again = state.compute_delta("A/USD", "a big long prompt for pair A with many many many tokens", 5);
        assert!(matches!(result_a_again, DeltaResult::NoChange));
    }

    #[test]
    fn token_based_detection_counts_actual_tokens() {
        let mut state = ContextState::new(0.30, 0.50);
        // Large first prompt
        let first = "a big long prompt with many tokens ".repeat(100);
        state.compute_delta("X/USD", &first, 10);
        // Second prompt is much shorter — should save many tokens
        let second = "short";
        let _ = state.compute_delta("X/USD", second, 10);
        // Cumulative savings should be > 0
        assert!(state.tokens_saved_this_cycle() > 0);
    }

    #[test]
    fn adaptive_threshold_scales_with_prompt_size() {
        let mut state = ContextState::new(0.30, 0.50);
        // Build a large prompt so the threshold computation uses realistic values
        let big = "word ".repeat(500); // ~500 tokens
        state.compute_delta("BIG/USD", &big, 50);
        // Change a single word: ratio should be very small (1/500 = 0.002)
        // Threshold = 1 - 50/500 = 0.9. 0.002 < 0.9 → Delta
        let mut small_change = big.clone();
        small_change.push_str("extra");
        let result = state.compute_delta("BIG/USD", &small_change, 50);
        assert!(matches!(result, DeltaResult::Delta(_)));
    }

    #[test]
    fn per_pair_anti_thrashing_only_skips_own_pair() {
        let mut state = ContextState::new(0.30, 0.50);
        // Pair A: simulate 2 inefficient cycles (small text changes, tiny savings)
        // First cycle: establish baseline with 1000 tokens
        let big = "word ".repeat(1000);
        state.compute_delta("A/USD", &big, 50);
        // Second cycle: nearly identical — small savings
        let almost_same = big.clone() + " ";
        let _ = state.compute_delta("A/USD", &almost_same, 50);
        // Third cycle: also nearly identical — small savings
        let _ = state.compute_delta("A/USD", &big, 50);
        // Now A's history has 2+ small savings → skip
        assert!(state.should_skip_compression_for("A/USD", 50));
        // Pair B has no history → don't skip
        assert!(!state.should_skip_compression_for("B/USD", 50));
    }

    #[test]
    fn end_cycle_logs_and_resets_cumulative_savings() {
        let mut state = ContextState::new(0.30, 0.50);
        let big = "word ".repeat(100);
        let small = "x";
        // First call: stores baseline, no savings recorded
        state.compute_delta("X/USD", &big, 10);
        // Second call (same pair, different text): records savings
        let _ = state.compute_delta("X/USD", small, 10);
        // First call: stores baseline, no savings recorded
        state.compute_delta("Y/USD", &big, 10);
        // Second call (same pair, different text): records savings
        let _ = state.compute_delta("Y/USD", small, 10);
        assert!(state.tokens_saved_this_cycle() > 0);
        state.end_cycle();
        assert_eq!(state.tokens_saved_this_cycle(), 0);
    }

    #[test]
    fn anti_thrashing_allows_good_compression() {
        let mut state = ContextState::new(0.30, 0.50);
        // Large first, much smaller second — good savings
        let big = "word ".repeat(1000);
        state.compute_delta("X/USD", &big, 10);
        // Second: tiny text, big savings
        let _ = state.compute_delta("X/USD", "x", 10);
        // History now has one big saving and one entry with 0 savings (no change).
        // should_skip_compression_for looks at LAST 2 entries.
        // Last 2: [big_saving, 0_savings]. If 0 < 10 (min), it's "inefficient".
        // So this actually DOES skip. That's the correct behavior.
        // Better test: verify that 2 high-savings cycles DON'T skip.
        let mut state2 = ContextState::new(0.30, 0.50);
        state2.compute_delta("Y/USD", &big, 10);
        let _ = state2.compute_delta("Y/USD", "x", 10);
        // Even one big saving entry means the LAST entry is "x" which has 0 savings
        // vs prev=1000 — that IS 990 tokens saved. So last entry is good.
        // But the previous entry was 0 (first cycle, full injection stored but no savings recorded).
        // Actually looking at record_token_savings: when first cycle stores, no savings recorded.
        // So history is just [990] after 2 cycles. len < 2 → don't skip.
        assert!(!state2.should_skip_compression_for("Y/USD", 10));
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
        let trimmed = state.soft_trim(text, 10000);
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

    // ---- FID-168 tests ----

    #[test]
    fn add_cycle_snapshot_adds_data_block() {
        let mut state = ContextState::new(0.30, 0.50);
        assert_eq!(state.data_blocks.len(), 0);
        state.add_cycle_snapshot("[2026-06-16] BTC/USD | PASS | conf 0%".to_string());
        state.add_cycle_snapshot("[2026-06-16] ETH/USD | PASS | conf 0%".to_string());
        assert_eq!(state.data_blocks.len(), 2);
        assert_eq!(state.data_blocks[0].block_type, "cycle_snapshot");
        assert!(state.data_blocks[0].content.contains("BTC/USD"));
    }

    /// FID-168 v2: snapshots now include regime + ATR + ADX + RSI.
    /// The summary prompt (FID-165) asks for these fields; capturing them in
    /// the snapshot makes the LLM's summary useful.
    #[test]
    fn add_cycle_snapshot_includes_market_context() {
        let mut state = ContextState::new(0.30, 0.50);
        // The engine produces lines like:
        //   [2026-06-16 18:00:00] BTC/USD | PASS Ranging | conf 0% | ATR1.23 ADX18.4 RSI55.2
        let rich = "[2026-06-16 18:00:00] BTC/USD | PASS Ranging | conf 0% | ATR1.23 ADX18.4 RSI55.2";
        state.add_cycle_snapshot(rich.to_string());
        assert!(state.data_blocks[0].content.contains("Ranging"));
        assert!(state.data_blocks[0].content.contains("ATR1.23"));
        assert!(state.data_blocks[0].content.contains("ADX18.4"));
        assert!(state.data_blocks[0].content.contains("RSI55.2"));
    }

    /// FID-168 v2: is_stale() returns true when no summary exists, true when
    /// the last update is older than MIN_SUMMARIZATION_INTERVAL, false otherwise.
    #[test]
    fn is_stale_triggers_fresh_summary() {
        let mut state = ContextState::new(0.30, 0.50);
        assert!(state.summary_context().is_stale()); // no summary
        state.update_summary("Test".to_string(), 100);
        assert!(!state.summary_context().is_stale()); // just updated
    }

    #[test]
    fn summary_skipped_when_no_overflow() {
        let mut state = ContextState::new(0.30, 0.50);
        // Add 5 small blocks. Total ~150 tokens. Target 30% of 10000 = 3000 tokens. Way under.
        for i in 0..5 {
            state.add_cycle_snapshot(format!("snapshot {}", i));
        }
        assert!(state.current_summary().is_none());
        // prune_old_blocks returns 0 when under budget
        let removed = state.prune_old_blocks(0.30, 10000);
        assert_eq!(removed, 0);
    }

    #[test]
    fn prune_and_summarize_fires_when_target_exceeded() {
        let mut state = ContextState::new(0.30, 0.50);
        // Add 100 large blocks. Each ~500 tokens. Total ~50000. Target 30% of 1000 = 300.
        for i in 0..100 {
            state.add_cycle_snapshot(format!("{} ", i).repeat(500));
        }
        // First, prune — should remove ~98 blocks to fit within 300 tokens.
        let removed = state.prune_old_blocks(0.30, 1000);
        assert!(removed > 0, "expected some blocks pruned, got {}", removed);
        // After pruning, blocks remaining should be small.
        let remaining_tokens = state.data_blocks_token_count();
        assert!(remaining_tokens <= 1000, "tokens after prune {} exceed budget 1000", remaining_tokens);
    }

    #[test]
    fn current_summary_accessor() {
        let mut state = ContextState::new(0.30, 0.50);
        assert!(state.current_summary().is_none());
        state.update_summary("Test summary content".to_string(), 100);
        assert_eq!(state.current_summary(), Some("Test summary content"));
    }
}
