//! Context Engine (FID-085)
//!
//! Orchestrates prompt assembly, encoding, and budget enforcement.

use crate::agent::context_builder::{self, FullContext};
use crate::agent::knowledge::{KnowledgeBase, KnowledgeUnit};
use crate::agent::prompts::PromptComposer;
use crate::agent::token_budget;
use crate::core::config::ContextConfig;
use crate::core::tsln::TslnSerializer;
use crate::core::types::MarketRegime;
use crate::data::indicators::IndicatorEngine;

/// Result of context assembly.
pub struct AssembledContext {
    pub system_prompt: String,
    pub user_message: String,
    pub token_count: usize,
}

/// Engine that manages prompt construction and encoding.
pub struct ContextEngine {
    config: ContextConfig,
    tsln_serializer: TslnSerializer,
}

impl ContextEngine {
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            tsln_serializer: TslnSerializer::new(),
        }
    }

    /// Assemble the full prompt context.
    /// Step 1: Select knowledge units (takes immutable ref to KB).
    /// Step 2: Compose system prompt + build user message (takes mutable ref to composer).
    pub fn assemble(
        &mut self,
        ctx: &FullContext,
        knowledge_base: &KnowledgeBase,
        composer: &mut PromptComposer,
    ) -> AssembledContext {
        let conditions = context_builder::determine_conditions(ctx);
        let knowledge_units = knowledge_base.select_with_tags(
            &conditions,
            &ctx.context_tags,
            self.config.knowledge_token_budget,
        );

        let knowledge_refs: Vec<&KnowledgeUnit> = knowledge_units.to_vec();
        let system_prompt = composer.compose(&knowledge_refs);
        let user_message = self.build_user_message(ctx);

        // FID-085 Item 4: Exact BPE token counting via tiktoken-rs
        let (_, _, token_count) = token_budget::count_prompt_tokens(&system_prompt, &user_message);

        AssembledContext {
            system_prompt,
            user_message,
            token_count,
        }
    }

    /// Get the active encoding mode.
    pub fn encoding_mode(&self) -> &str {
        &self.config.encoding_mode
    }

    /// Build the user message for a given context (public for split-call pattern).
    pub fn build_user_message_for(&mut self, ctx: &FullContext) -> String {
        self.build_user_message(ctx)
    }

    fn build_user_message(&mut self, ctx: &FullContext) -> String {
        match self.config.encoding_mode.as_str() {
            "tsln" => self.build_tsln_message(ctx),
            mode => {
                tracing::warn!(
                    "ContextEngine: encoding_mode='{}' — using legacy JSON path. \
                     Set encoding_mode='tsln' for FID-085 compression.",
                    mode
                );
                context_builder::build_user_message_static(ctx)
            }
        }
    }

    fn build_tsln_message(&mut self, ctx: &FullContext) -> String {
        // FID-163 Part B: Reset TSLN serializer state per pair.
        // A single TslnSerializer instance is reused across all 30 pairs in a cycle;
        // without this reset, last_close from pair A bleeds into pair B's first
        // candle diff, producing wildly wrong differential encodings.
        self.tsln_serializer.reset();

        let mut msg = String::new();
        msg.push_str(&format!("## Current Market Data — {}\n\n", ctx.pair));

        // ZigZag pivots (compact structural analysis)
        let pivots = IndicatorEngine::zigzag_pivots(ctx.candles, 14);
        if !pivots.is_empty() {
            msg.push_str("### ZigZag Pivots\n");
            for (idx, price, is_peak) in &pivots {
                let label = if *is_peak { "PEAK" } else { "TROUGH" };
                msg.push_str(&format!("  [{}] #{} @ ${}\n", label, idx, price));
            }
            msg.push('\n');
        }

        // KBar features (compact statistical summary)
        if let Some((z, vol, trend, vol_ratio)) = IndicatorEngine::kbar_features(ctx.candles) {
            msg.push_str(&format!(
        "### KBar Features\nz-score: {} | AnnVol: {} | Trend: {} | VolRatio: {}{}\n\n",
        z, vol, trend, vol_ratio,
        ctx.indicators.volume_sma.map_or_else(
            String::new,
            |v| format!(" (avg_vol: ${})", v),
        )
            ));
        }

        // TSLN candle data with adaptive count based on regime
        let candle_count = self.adaptive_candle_count(ctx.regime, ctx.indicators);
        let candles_to_use = if ctx.candles.len() > candle_count {
            &ctx.candles[ctx.candles.len() - candle_count..]
        } else {
            ctx.candles
        };
        let tsln_data = self.tsln_serializer.serialize_data_only(candles_to_use);
        msg.push_str(&format!("### TSLN Candle Data ({} candles)\n```\n{}\n```\n", candles_to_use.len(), tsln_data));

        // Live price
        if let Some(live) = ctx.live_price {
            msg.push_str(&format!("**LIVE PRICE: ${}**\n", live));
        }

        // Indicators (compact)
        msg.push_str(&format!(
            "Indicators: RSI={:?} ADX={:?} EMA_F={:?} EMA_S={:?} ATR={:?}\n",
            ctx.indicators.rsi,
            ctx.indicators.adx,
            ctx.indicators.ema_fast,
            ctx.indicators.ema_slow,
            ctx.indicators.atr,
        ));

        // Market context
        msg.push_str(&format!("Regime: {:?}\n", ctx.regime));
        if let Some(imb) = ctx.order_book_imbalance {
            msg.push_str(&format!("OrderBook Imbalance: {:+}\n", imb));
        }

        // Market insight (truncated)
        msg.push_str(&format!("\n## Market Insight\n{}\n", ctx.market_context.summary()));

        // Positions
        if !ctx.positions.is_empty() {
            msg.push_str("\n## Open Positions\n");
            for pos in ctx.positions {
                msg.push_str(&format!(
                    "- {} {} @ {} | SL: {} | TP1: {} | PnL: {}\n",
                    pos.pair, pos.side, pos.entry_price, pos.stop_loss, pos.take_profit_1, pos.unrealized_pnl
                ));
            }
        }

        // Account
        msg.push_str(&format!(
            "\n## Account\nBalance: ${} | Equity: ${} | DD: {}%\n",
            ctx.account.balance,
            ctx.account.equity,
            ctx.account.drawdown_pct * 100.0,
        ));
        if ctx.account.open_positions >= ctx.account.max_positions {
            msg.push_str("**AT MAX POSITIONS** — Only ADJUST_STOP or PASS.\n");
        }

        // === FID-163 Part C: Add 9 missing context blocks (full parity with legacy path) ===

        // 1. Higher-timeframe candles
        for (tf, tf_candles) in &ctx.higher_tf_candles {
            if tf_candles.is_empty() {
                continue;
            }
            msg.push_str(&format!("\n### Higher Timeframe — {} {}\n", tf, ctx.pair));
            if let Some(last) = tf_candles.last() {
                msg.push_str(&format!(
                    "Latest {} Candle: O={} H={} L={} C={} V={}\n",
                    tf, last.open, last.high, last.low, last.close, last.volume
                ));
            }
        }

        // 2. Volume profile
        if let Some(vp) = ctx.volume_profile {
            msg.push_str(&format!(
                "\n## Volume Profile\nPOC: {} | VAH: {} | VAL: {}\n",
                vp.poc_price, vp.value_area_high, vp.value_area_low
            ));
        }

        // 3. On-chain analytics
        let oc = &ctx.market_context.onchain;
        if oc.mvrv.is_some() || oc.sopr.is_some() || oc.nvt_signal.is_some() {
            msg.push_str("\n## On-Chain Analytics\n");
            if let Some(mvrv) = oc.mvrv {
                let state = if mvrv > 3.5 {
                    "EUPHORIA (sell signal)"
                } else if mvrv > 2.0 {
                    "Warming up"
                } else if mvrv > 1.0 {
                    "Neutral/Undervalued"
                } else {
                    "CAPITULATION (strong buy)"
                };
                msg.push_str(&format!("MVRV: {} — {}\n", mvrv, state));
            }
            if let Some(sopr) = oc.sopr {
                let state = if sopr > 1.0 {
                    "Profit realization"
                } else {
                    "Loss realization (capitulation)"
                };
                msg.push_str(&format!("SOPR: {} — {}\n", sopr, state));
            }
            if let Some(nvt) = oc.nvt_signal {
                msg.push_str(&format!("NVT Signal: {}\n", nvt));
            }
        }

        // 4. Recent news
        if !ctx.market_context.rss_items.is_empty() {
            msg.push_str(&format!(
                "\n## Recent News\n{}\n",
                crate::insight::rss::format_for_context(&ctx.market_context.rss_items, 5)
            ));
        }

        // 5. Recent trade history
        if let Some(trades) = ctx.recent_trades {
            if !trades.is_empty() {
                msg.push_str("\n## Recent Trade History\n");
                let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
                let losses = trades.iter().filter(|t| t.pnl <= 0.0).count();
                let avg_win = if wins > 0 {
                    trades.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum::<f64>() / wins as f64
                } else { 0.0 };
                let avg_loss = if losses > 0 {
                    trades.iter().filter(|t| t.pnl <= 0.0).map(|t| t.pnl).sum::<f64>() / losses as f64
                } else { 0.0 };
                let profit_factor = if avg_loss != 0.0 {
                    (avg_win * wins as f64) / (avg_loss.abs() * losses as f64)
                } else { f64::INFINITY };
                for (i, trade) in trades.iter().take(10).enumerate() {
                    msg.push_str(&format!(
                        "{}. {} {} @ {} → {} | PnL: ${} ({}%) | {}\n",
                        i + 1,
                        trade.pair,
                        if trade.pnl > 0.0 { "WIN" } else { "LOSS" },
                        trade.entry_price,
                        trade.exit_price,
                        trade.pnl,
                        trade.pnl_pct,
                        trade.closed_at.format("%Y-%m-%d")
                    ));
                }
                msg.push_str(&format!(
                    "Summary: {}W/{}L ({}% WR) | Avg Win: ${} | Avg Loss: ${} | PF: {}\n",
                    wins, losses,
                    if wins + losses > 0 { wins as f64 / (wins + losses) as f64 * 100.0 } else { 0.0 },
                    avg_win, avg_loss, profit_factor
                ));
            }
        }

        // 6. Memory context (pre-formatted by memory::context::format_memory_prompt)
        if let Some(ref memory) = ctx.memory_context {
            if !memory.is_empty() {
                msg.push_str(memory);
            }
        }

        // 7. Decision log context
        if let Some(ref log_ctx) = ctx.decision_log_context {
            if !log_ctx.is_empty() {
                msg.push_str("\n## Recent Decision Log\n");
                msg.push_str(log_ctx);
            }
        }

        // 8. Active trading universe
        if let Some(pairs) = ctx.active_pairs {
            if !pairs.is_empty() {
                msg.push_str(&format!("\n## Active Trading Universe ({} pairs)\n", pairs.len()));
                msg.push_str(&pairs.join(", "));
                msg.push_str("\nThe pair shown above is already vetted for liquidity and safety. Evaluate it.");
            }
        }

        // 9. Market conditions (SOUL.md §XIII action triggers)
        msg.push_str(&format!("\n## Market Conditions\n{}\n", ctx.market_context.conditions_summary()));

        // Decision required
        msg.push_str("\n## Decision Required\n");
        msg.push_str("Analyze the above data and provide your trade decision in the specified JSON format.\n");

        msg
    }

    /// Determine adaptive candle count based on market regime.
    /// Ranging: 50, Trending: 100, Volatile: 200.
    fn adaptive_candle_count(&self, regime: MarketRegime, _indicators: &crate::core::types::IndicatorValues) -> usize {
        match regime {
            MarketRegime::Ranging => self.config.adaptive_candles_ranging,
            MarketRegime::Trending => self.config.adaptive_candles_trending,
            MarketRegime::Volatile => self.config.adaptive_candles_volatile,
        }
    }

    // === Phase 6: SGDR Cosine Annealing Budget (Item 20) ===

    /// SGDR cosine annealing: budget varies over an epoch.
    /// Returns the current token budget based on cycle position.
    /// Peak at epoch start (scanning), trough at epoch end (monitoring).
    pub fn sgdr_budget(&self, cycle_in_epoch: usize) -> u32 {
        let epoch_len = self.config.sgdr_epoch_length.max(1);
        let position = (cycle_in_epoch % epoch_len) as f64 / epoch_len as f64;
        let cosine = (position * std::f64::consts::PI).cos();
        let range = self.config.sgdr_max_budget - self.config.sgdr_min_budget;
        let budget = self.config.sgdr_min_budget as f64 + range as f64 * (1.0 + cosine) / 2.0;
        budget as u32
    }

    // === Phase 7: Cache Observability (Item 24) ===

    /// Compute SHA-256 digest of a prompt component for cache stability tracking.
    pub fn compute_digest(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Log cache break if system prompt digest changed.
    pub fn check_cache_break(&self, current_digest: &str, previous_digest: Option<&str>) -> bool {
        if let Some(prev) = previous_digest {
            if prev != current_digest {
                tracing::warn!(
                    "Cache break detected: systemPrompt digest changed from {} to {}",
                    prev, current_digest
                );
                return true;
            }
        }
        false
    }

    // === Phase 7: Context Window Guard (Item 25) ===

    /// Validate that the model's context window is above minimum thresholds.
    /// Returns (ok, message) — ok=false means block, message explains why.
    pub fn validate_context_window(&self, model_window: u32) -> (bool, String) {
        if model_window < self.config.min_context_guard {
            return (
                false,
                format!(
                    "BLOCKED: Model context window ({} tokens) is below hard minimum ({}). Switch to a larger model.",
                    model_window, self.config.min_context_guard
                ),
            );
        }
        if model_window < self.config.warn_context_guard {
            return (
                true,
                format!(
                    "WARNING: Model context window ({} tokens) is below recommended minimum ({}). Consider raising contextTokens.",
                    model_window, self.config.warn_context_guard
                ),
            );
        }
        (true, "OK".to_string())
    }

    // === Phase 8: Tool Result Summarization (Item 26) ===

    /// Summarize a data block into a compact one-liner.
    /// Pattern: [data_type] entity: key_metric_1, key_metric_2, ...
    pub fn summarize_data_block(block_type: &str, pair: &str, metrics: &[(String, String)]) -> String {
        let metrics_str: Vec<String> = metrics.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        format!("[{}] {}: {}", block_type, pair, metrics_str.join(", "))
    }

    // === Phase 8: Deduplication (Item 27) ===

    /// Check if a data block is unchanged from the previous cycle.
    pub fn is_duplicate(current_hash: u64, previous_hash: Option<u64>) -> bool {
        previous_hash == Some(current_hash)
    }

    /// Hash a data block for deduplication.
    pub fn hash_block(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    // === Phase 8: Deterministic Fallback (Item 28) ===

    /// Extract structured elements from text when summarization fails.
    /// Uses regex-free pattern matching to extract key information.
    pub fn deterministic_fallback(text: &str) -> String {
        let mut result = String::new();
        for line in text.lines() {
            let trimmed = line.trim();
            // Extract lines with structured data markers
            if trimmed.contains("RSI=") || trimmed.contains("ADX=") || trimmed.contains("EMA_") {
                result.push_str(trimmed);
                result.push('\n');
            }
            if trimmed.contains('$') && (trimmed.contains("Balance") || trimmed.contains("Equity")) {
                result.push_str(trimmed);
                result.push('\n');
            }
            if trimmed.contains("SL:") || trimmed.contains("TP") || trimmed.contains("PnL") {
                result.push_str(trimmed);
                result.push('\n');
            }
            if trimmed.contains("Decision") || trimmed.contains("BUY") || trimmed.contains("SELL") || trimmed.contains("PASS") {
                result.push_str(trimmed);
                result.push('\n');
            }
        }
        if result.is_empty() {
            "Unable to extract structured state from failed summarization".to_string()
        } else {
            result
        }
    }

    // === Accessors ===

    /// Get a reference to the config.
    pub fn config(&self) -> &ContextConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgdr_budget_at_peak() {
        let engine = ContextEngine::new(ContextConfig::default());
        let budget = engine.sgdr_budget(0);
        assert_eq!(budget, engine.config().sgdr_max_budget);
    }

    #[test]
    fn sgdr_budget_at_trough() {
        let engine = ContextEngine::new(ContextConfig::default());
        let epoch = engine.config().sgdr_epoch_length;
        // Trough is at end of epoch (cos(π) = -1)
        let budget = engine.sgdr_budget(epoch - 1);
        assert_eq!(budget, engine.config().sgdr_min_budget);
    }

    #[test]
    fn sgdr_budget_smooth_curve() {
        let engine = ContextEngine::new(ContextConfig::default());
        let epoch = engine.config().sgdr_epoch_length;
        let b0 = engine.sgdr_budget(0);
        let b_quarter = engine.sgdr_budget(epoch / 4);
        let b_half = engine.sgdr_budget(epoch / 2);
        assert!(b0 > b_quarter);
        assert!(b_quarter > b_half);
    }

    #[test]
    fn cache_break_detected() {
        let engine = ContextEngine::new(ContextConfig::default());
        assert!(engine.check_cache_break("abc123", Some("def456")));
        assert!(!engine.check_cache_break("abc123", Some("abc123")));
        assert!(!engine.check_cache_break("abc123", None));
    }

    #[test]
    fn context_window_guard_blocks() {
        let engine = ContextEngine::new(ContextConfig::default());
        let (ok, msg) = engine.validate_context_window(2000);
        assert!(!ok);
        assert!(msg.contains("BLOCKED"));
    }

    #[test]
    fn context_window_guard_warns() {
        let engine = ContextEngine::new(ContextConfig::default());
        let (ok, msg) = engine.validate_context_window(6000);
        assert!(ok);
        assert!(msg.contains("WARNING"));
    }

    #[test]
    fn context_window_guard_ok() {
        let engine = ContextEngine::new(ContextConfig::default());
        let (ok, msg) = engine.validate_context_window(100000);
        assert!(ok);
        assert_eq!(msg, "OK");
    }

    #[test]
    fn summarize_data_block_format() {
        let summary = ContextEngine::summarize_data_block(
            "candles",
            "ETH/USD",
            &[
                ("count".to_string(), "48".to_string()),
                ("last".to_string(), "$2450".to_string()),
            ],
        );
        assert!(summary.contains("[candles]"));
        assert!(summary.contains("ETH/USD"));
        assert!(summary.contains("count=48"));
    }

    #[test]
    fn dedup_detects_identical() {
        let hash1 = ContextEngine::hash_block("hello world");
        let hash2 = ContextEngine::hash_block("hello world");
        assert!(ContextEngine::is_duplicate(hash1, Some(hash2)));
    }

    #[test]
    fn dedup_detects_different() {
        let hash1 = ContextEngine::hash_block("hello world");
        let hash2 = ContextEngine::hash_block("goodbye world");
        assert!(!ContextEngine::is_duplicate(hash1, Some(hash2)));
    }

    #[test]
    fn deterministic_fallback_extracts_indicators() {
        let text = "Some noise\nRSI=65.3 ADX=22.1\nMore noise\nBalance: $500.00\nDecision: BUY\n";
        let result = ContextEngine::deterministic_fallback(text);
        assert!(result.contains("RSI=65.3"));
        assert!(result.contains("Balance: $500.00"));
        assert!(result.contains("BUY"));
    }

    #[test]
    fn deterministic_fallback_empty_input() {
        let result = ContextEngine::deterministic_fallback("just random text with no markers");
        assert!(result.contains("Unable to extract"));
    }

    #[test]
    fn compute_digest_consistent() {
        let d1 = ContextEngine::compute_digest("test content");
        let d2 = ContextEngine::compute_digest("test content");
        assert_eq!(d1, d2);
    }

    #[test]
    fn compute_digest_different() {
        let d1 = ContextEngine::compute_digest("content A");
        let d2 = ContextEngine::compute_digest("content B");
        assert_ne!(d1, d2);
    }
}
