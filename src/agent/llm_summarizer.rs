//! LLM-Based History Summarization (FID-165, ported from openclaw compaction.ts).
//!
//! Phase 1: token-aware history pruning + chunked summarization with progressive fallback.
//! Phase 2 (v0.15.0): stage-based summarization, handoff briefings.
//!
//! Ported from `research/repos/openclaw/src/agents/compaction.ts`:
//! - `pruneHistoryForContextShare` → `prune_for_context_share`
//! - `chunkMessagesByMaxTokens` → `chunk_by_max_tokens`
//! - `summarizeChunks` → `summarize_chunks`
//! - `summarizeWithFallback` → `summarize_with_fallback`

use crate::agent::context_state::DataBlock;
use crate::agent::provider::{LlmProvider, Message};
use crate::agent::token_budget::count_tokens;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Minimum time between summarization calls. Prevents thrashing when many blocks
/// are added in quick succession.
pub const MIN_SUMMARIZATION_INTERVAL: Duration = Duration::from_secs(60);

/// Default summarization prompt template. Forces M3 to extract structured fields
/// rather than paraphrase.
pub const SUMMARIZATION_PROMPT: &str = "\
Summarize the following trading context. Include in your summary:

- Active trades (pair, side, entry, stop, TP) with current P&L
- Current regime (Trending/Ranging/Volatile) and key indicators
- Recent decisions and their outcomes (wins/losses/holds)
- Open risk concerns (max drawdown, concentration, slippage)
- Memory context highlights (recent wins, recent losses, anti-patterns)

Keep the summary concise (under 500 words). Use structured bullet points.
Do NOT add commentary or reasoning — just the summary.

CONTEXT:
";

/// A chunk of context blocks to summarize together.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub blocks: Vec<DataBlock>,
    pub token_count: usize,
}

/// Configuration for the summarizer. Separated from LlmProvider so that
/// chunking and pruning can be tested without constructing an LlmProvider.
#[derive(Debug, Clone)]
pub struct SummarizerConfig {
    /// Max tokens per chunk sent to the LLM. 4K fits M3's 4K output budget.
    pub max_chunk_tokens: usize,
    /// Reserve tokens for the prompt + response.
    pub reserve_tokens: usize,
    /// System prompt for the summarization call.
    pub system_prompt: String,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            max_chunk_tokens: 4000,
            reserve_tokens: 1000,
            system_prompt: SUMMARIZATION_PROMPT.to_string(),
        }
    }
}

/// LlmSummarizer wraps an LLM provider for history-summarization calls.
///
/// Reuses the same LlmProvider as the main engine (M3 via TokenRouter, free).
/// Per FID-138, M3's thinking block is disabled via the m3-proxy.js.
pub struct LlmSummarizer {
    config: SummarizerConfig,
    provider: Option<LlmProvider>,
}

impl LlmSummarizer {
    /// Create a summarizer with an LLM provider.
    pub fn new(provider: LlmProvider) -> Self {
        Self {
            config: SummarizerConfig::default(),
            provider: Some(provider),
        }
    }

    /// Create a chunking-only summarizer (no LLM). Used for tests and for
    /// cases where summarization is disabled but pruning is still desired.
    pub fn chunking_only() -> Self {
        Self {
            config: SummarizerConfig::default(),
            provider: None,
        }
    }

    /// Create with custom config (no LLM).
    pub fn with_config(config: SummarizerConfig) -> Self {
        Self {
            config,
            provider: None,
        }
    }

    pub fn config(&self) -> &SummarizerConfig {
        &self.config
    }

    /// Summarize a list of DataBlocks via the LLM. Returns the summary string.
    /// On total failure, returns a generic placeholder.
    pub async fn summarize(&self, blocks: &[DataBlock]) -> Result<String, String> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| "No LLM provider configured".to_string())?;

        if blocks.is_empty() {
            return Ok("No prior history.".to_string());
        }

        let chunks = self.chunk_by_max_tokens(blocks);
        if chunks.is_empty() {
            return Err("No chunks produced".to_string());
        }

        self.summarize_with_fallback(provider, &chunks).await
    }

    /// Split blocks into chunks bounded by `max_chunk_tokens`.
    pub fn chunk_by_max_tokens(&self, blocks: &[DataBlock]) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut current_blocks: Vec<DataBlock> = Vec::new();
        let mut current_tokens = 0;

        for block in blocks {
            let block_tokens = count_tokens(&block.content);
            if !current_blocks.is_empty() && current_tokens + block_tokens > self.config.max_chunk_tokens {
                chunks.push(Chunk {
                    blocks: std::mem::take(&mut current_blocks),
                    token_count: current_tokens,
                });
                current_tokens = 0;
            }
            current_tokens += block_tokens;
            current_blocks.push(block.clone());
        }

        if !current_blocks.is_empty() {
            chunks.push(Chunk {
                blocks: current_blocks,
                token_count: current_tokens,
            });
        }

        chunks
    }

    /// Trim the oldest blocks until total tokens fit within target share of context window.
    /// Returns the number of blocks removed.
    ///
    /// Port of `pruneHistoryForContextShare` from openclaw.
    pub fn prune_for_context_share(
        &self,
        blocks: &mut Vec<DataBlock>,
        target_share: f64,
        context_window: usize,
    ) -> usize {
        let target_tokens = ((context_window as f64) * target_share) as usize;
        let current_tokens: usize = blocks.iter().map(|b| count_tokens(&b.content)).sum();
        if current_tokens <= target_tokens {
            debug!(
                "Prune: no-op ({} tokens ≤ {} target = {}% of {})",
                current_tokens, target_tokens, (target_share * 100.0) as u32, context_window
            );
            return 0;
        }
        let mut removed = 0;
        while count_total_tokens(blocks) > target_tokens {
            if blocks.is_empty() {
                break;
            }
            blocks.remove(0);
            removed += 1;
        }
        let final_tokens = count_total_tokens(blocks);
        info!(
            "Pruned {} oldest blocks ({} tokens → {} tokens, target {}% = {} of {})",
            removed,
            current_tokens,
            final_tokens,
            (target_share * 100.0) as u32,
            target_tokens,
            context_window
        );
        removed
    }

    async fn summarize_with_fallback(
        &self,
        provider: &LlmProvider,
        chunks: &[Chunk],
    ) -> Result<String, String> {
        match self.summarize_chunks(provider, chunks).await {
            Ok(s) => Ok(s),
            Err((partial, err)) => {
                warn!(
                    "Full summarization failed ({}), trying partial without oversized chunks",
                    err
                );
                let small: Vec<DataBlock> = chunks
                    .iter()
                    .flat_map(|c| c.blocks.iter().cloned())
                    .filter(|b| count_tokens(&b.content) <= self.config.max_chunk_tokens)
                    .collect();
                let original_count: usize = chunks.iter().map(|c| c.blocks.len()).sum();
                if small.len() != original_count {
                    let small_chunks = self.chunk_by_max_tokens(&small);
                    match self.summarize_chunks(provider, &small_chunks).await {
                        Ok(s) => Ok(s),
                        Err(_) => {
                            if let Some(p) = partial {
                                Ok(p)
                            } else {
                                Err(err)
                            }
                        }
                    }
                } else if let Some(p) = partial {
                    Ok(p)
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn summarize_chunks(
        &self,
        provider: &LlmProvider,
        chunks: &[Chunk],
    ) -> Result<String, (Option<String>, String)> {
        let mut summary = String::new();
        let mut any_success = false;
        for chunk in chunks {
            let content: String = chunk
                .blocks
                .iter()
                .map(|b| b.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            let user_message = format!("{}{}", self.config.system_prompt, content);
            match provider
                .chat(
                    "You are a trading-context summarizer. Be terse. Use bullet points.",
                    &[Message {
                        role: "user".to_string(),
                        content: user_message,
                    }],
                )
                .await
            {
                Ok(s) => {
                    summary.push_str(&s);
                    summary.push_str("\n\n---\n\n");
                    any_success = true;
                }
                Err(e) => {
                    warn!("Chunk summarization failed: {}", e);
                    if any_success {
                        return Err((
                            Some(summary),
                            format!("Partial failure: {}", e),
                        ));
                    }
                }
            }
        }
        if any_success {
            Ok(summary)
        } else {
            Err((None, "All chunks failed".to_string()))
        }
    }
}

fn count_total_tokens(blocks: &[DataBlock]) -> usize {
    blocks.iter().map(|b| count_tokens(&b.content)).sum()
}

/// Tracks the lifecycle of a context summary.
#[derive(Debug, Clone, Default)]
pub struct SummaryContext {
    /// The current summary text. None = no summary yet.
    pub summary: Option<String>,
    /// When the summary was last updated.
    pub updated_at: Option<Instant>,
    /// Total tokens of all blocks currently held.
    pub current_token_count: usize,
}

impl SummaryContext {
    /// True if at least MIN_SUMMARIZATION_INTERVAL has passed since the last update.
    pub fn is_stale(&self) -> bool {
        match self.updated_at {
            None => true,
            Some(t) => t.elapsed() >= MIN_SUMMARIZATION_INTERVAL,
        }
    }

    /// Update the summary.
    pub fn update(&mut self, new_summary: String, current_token_count: usize) {
        self.summary = Some(new_summary);
        self.updated_at = Some(Instant::now());
        self.current_token_count = current_token_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(content: &str) -> DataBlock {
        DataBlock {
            content: content.to_string(),
            created_at: Instant::now(),
            ttl: Duration::from_secs(3600),
            block_type: "test".to_string(),
        }
    }

    #[test]
    fn chunk_by_max_tokens_splits_correctly() {
        let summarizer = LlmSummarizer::chunking_only();
        let blocks: Vec<DataBlock> = (0..10)
            .map(|i| make_block(&format!("word{} ", i).repeat(100)))
            .collect();
        let chunks = summarizer.chunk_by_max_tokens(&blocks);
        // Each block is ~100 tokens. 10 blocks ≈ 1000 tokens total.
        // max_chunk_tokens=4000. 1 chunk fits all 10.
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].blocks.len(), 10);
    }

    #[test]
    fn chunk_by_max_tokens_splits_when_exceeding() {
        let summarizer = LlmSummarizer::chunking_only();
        // 10 large blocks, each with content that tokenizes to > 4000 tokens
        // (so each block alone exceeds max_chunk_tokens).
        let blocks: Vec<DataBlock> = (0..10)
            .map(|i| make_block(&format!("block{} ", i).repeat(5000)))
            .collect();
        let chunks = summarizer.chunk_by_max_tokens(&blocks);
        // Each block exceeds max, so each becomes its own chunk.
        assert_eq!(chunks.len(), 10);
    }

    #[test]
    fn chunk_by_max_tokens_respects_size() {
        let summarizer = LlmSummarizer::chunking_only();
        // 100 small blocks, each ~50 tokens. Total ~5000 tokens. max=4000.
        // Should produce multiple chunks.
        let blocks: Vec<DataBlock> = (0..100)
            .map(|i| make_block(&format!("word{} ", i).repeat(50)))
            .collect();
        let chunks = summarizer.chunk_by_max_tokens(&blocks);
        // Should produce 2 chunks (each ~50 blocks = 2500 tokens, both under 4000).
        // But individual blocks may force more chunks if a single block is large.
        // The exact count depends on per-block token counts.
        assert!(chunks.len() >= 2, "expected multiple chunks, got {}", chunks.len());
        assert!(chunks.len() <= 100, "expected <= 100 chunks, got {}", chunks.len());
    }

    #[test]
    fn prune_for_context_share_removes_oldest_first() {
        let summarizer = LlmSummarizer::chunking_only();
        let mut blocks: Vec<DataBlock> = (0..10)
            .map(|i| make_block(&format!("word{} ", i).repeat(100)))
            .collect();
        // Each block is ~100 tokens. 10 blocks = 1000 tokens. Context window 1000, target 0.1 = 100 tokens.
        // Need to remove blocks until remaining ≤ 100 tokens.
        let removed = summarizer.prune_for_context_share(&mut blocks, 0.1, 1000);
        // Should remove at least 9 (leaving 1 block = ~100 tokens).
        assert!(removed >= 9, "expected >= 9 removed, got {}", removed);
        assert!(blocks.len() <= 1, "expected <= 1 remaining, got {}", blocks.len());
    }

    #[test]
    fn prune_for_context_share_no_op_when_under_budget() {
        let summarizer = LlmSummarizer::chunking_only();
        let mut blocks: Vec<DataBlock> = (0..3)
            .map(|i| make_block(&format!("word{} ", i).repeat(10)))
            .collect();
        let removed = summarizer.prune_for_context_share(&mut blocks, 0.5, 10000);
        assert_eq!(removed, 0);
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn summary_context_lifecycle() {
        let mut ctx = SummaryContext::default();
        assert!(ctx.is_stale());
        assert!(ctx.summary.is_none());
        ctx.update("Test summary".to_string(), 100);
        assert!(!ctx.is_stale());
        assert_eq!(ctx.summary.as_deref(), Some("Test summary"));
        assert_eq!(ctx.current_token_count, 100);
    }
}
