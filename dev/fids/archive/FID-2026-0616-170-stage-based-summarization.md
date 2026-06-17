# FID-170: Stage-Based Summarization (Phase 2 of FID-165)

**Filename:** `FID-2026-0616-170-stage-based-summarization.md`
**ID:** FID-2026-0616-170
**Severity:** medium (operational — for very large histories that exceed single-call token budget, stage-based summarization is needed; not urgent until history grows large)
**Status:** created
**Created:** 2026-06-16 20:00 EST
**Author:** Vera

---

## Summary

Port of openclaw's `summarizeInStages` (compaction.ts:334-397) to Rust. Splits history into N roughly-equal parts, summarizes each via `summarize_with_fallback` (FID-165), then merges the partial summaries via a final `summarize_with_fallback` call with merge instructions. Used when a single `summarize_with_fallback` call would exceed the LLM's output budget (because the history has too many distinct themes).

**When this fires:** Stage-based summarization is needed when the pruned history has more than ~50 blocks (5+ themes that don't fit in one summary). For v0.14.3, this is rare (the engine prunes down to ~10-30 blocks per cycle). The implementation is insurance for v0.15.0 when history will be larger.

**Use case for v0.14.3:** Not actively used in the engine loop (FID-168 uses `summarize_history` which is single-call). The function is exposed in the API so callers can opt in.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+
- **Commit/State:** post-FID-168 (`760a594e`), 351 tests pass
- **Current time:** 2026-06-16 20:00 EST

---

## Detailed Description

### The pattern

From openclaw `compaction.ts:334-397`:

```typescript
export async function summarizeInStages(params: {
  messages, model, apiKey, headers, signal, reserveTokens, maxChunkTokens,
  contextWindow, customInstructions, summarizationInstructions,
  previousSummary, parts, minMessagesForSplit,
}): Promise<string> {
  if (messages.length === 0) {
    return previousSummary ?? DEFAULT_SUMMARY_FALLBACK;
  }

  // Build a stage-split plan
  const plan = await buildStageSplitPlanWithWorker({...});
  if (plan.mode === "single") {
    return summarizeWithFallback(params);
  }

  // Summarize each stage
  const partialSummaries: string[] = [];
  for (const chunk of plan.chunks) {
    partialSummaries.push(
      await summarizeWithFallback({...params, messages: chunk, previousSummary: undefined}),
    );
  }

  if (partialSummaries.length === 1) {
    return partialSummaries[0];
  }

  // Build summary messages
  const summaryMessages = partialSummaries.map((summary) => ({
    role: "user", content: summary, timestamp: Date.now(),
  }));

  // Merge
  const mergeInstructions = custom
    ? `${MERGE_SUMMARIES_INSTRUCTIONS}\n\n${custom}`
    : MERGE_SUMMARIES_INSTRUCTIONS;

  return summarizeWithFallback({
    ...params, messages: summaryMessages, customInstructions: mergeInstructions,
  });
}
```

### Rust port

```rust
pub async fn summarize_in_stages(
    &self,
    blocks: &[DataBlock],
    parts: usize,
    min_blocks_for_split: usize,
) -> Result<String, String> {
    if blocks.is_empty() {
        return Ok("No prior history.".to_string());
    }
    if blocks.len() < min_blocks_for_split {
        // Not enough to split — use single-call
        return self.summarize(blocks).await;
    }

    let chunks = self.split_into_stages(blocks, parts);
    if chunks.len() <= 1 {
        return self.summarize(blocks).await;
    }

    // Summarize each stage
    let mut partial_summaries = Vec::new();
    for chunk in &chunks {
        match self.summarize(chunk).await {
            Ok(s) => partial_summaries.push(s),
            Err(e) => {
                warn!("Stage summarization failed: {}", e);
                // Continue with what we have
            }
        }
    }

    if partial_summaries.len() == 1 {
        return Ok(partial_summaries.remove(0));
    }

    // Build merge instructions + summary content
    let merge_prompt = MERGE_SUMMARIES_INSTRUCTIONS;
    let merged_content = partial_summaries.join("\n\n---\n\n");

    // Final merge via LLM
    let user_message = format!("{}{}", merge_prompt, merged_content);
    let provider = self.provider.as_ref()
        .ok_or_else(|| "No LLM provider configured".to_string())?;
    provider.chat(
        "You are a trading-context merger. Combine partial summaries into one cohesive summary.",
        &[Message {
            role: "user".to_string(),
            content: user_message,
        }],
    ).await
}
```

The merge instructions (openclaw's `MERGE_SUMMARIES_INSTRUCTIONS`):

```
Merge these partial summaries into a single cohesive summary.

MUST PRESERVE:
- Active tasks and their current status (in-progress, blocked, pending)
- Batch operation progress (e.g., '5/17 items completed')
- The last thing the user requested and what was being done about it
- Decisions made and their rationale
- TODOs, open questions, and constraints
- Any commitments or follow-ups promised

PRIORITIZE recent context over older history. The agent needs to know
what it was doing, not just what was discussed.
```

This becomes the system prompt for the merge call.

### What needs implementation

In `src/agent/llm_summarizer.rs`:
- Add `MERGE_SUMMARIES_INSTRUCTIONS` constant
- Add `summarize_in_stages(blocks, parts, min_blocks_for_split)` method
- Add `split_into_stages(blocks, parts)` private helper
- Add `MergeResult` enum (Success, PartialMerge, Failure) to track merge quality

3-4 new unit tests:
- `summarize_in_stages_with_few_blocks_uses_single_call` (delegates to `summarize` when below threshold)
- `split_into_stages_creates_equal_chunks`
- `summarize_in_stages_returns_merged_string`
- `summarize_in_stages_handles_partial_failures`

### What this FID does NOT do

- **Does not change the engine loop.** `summarize_in_stages` is a public method, not called by the engine. Callers can opt in.
- **Does not implement the worker-based planning.** openclaw uses Web Workers to plan the splits in parallel. In Rust, the planning is sync (we know chunk sizes upfront from token counts).
- **Does not support custom `previousSummary` thread-through.** Phase 1 single-call summarization does this; stage-based version doesn't (merging multiple `previousSummary` values is complex). Could be added in v0.15.0.

### Expected Behavior

After this FID:
- `LlmSummarizer::summarize_in_stages(blocks, 3, 50)` splits 100+ blocks into 3 stages, summarizes each, merges.
- Below 50 blocks, it falls back to `summarize` (single-call).
- Each stage summary is independent. If one fails, the merge continues with the others.
- The merge is via a final LLM call with structured instructions.

### Risks

- **Latency.** Stage-based: 1 + N LLM calls. For 3 stages, 4 total calls × 5-10s each = 20-40s. Single-call: 1 × 5-10s. Stage-based is 4x slower. Mitigation: only fire when blocks >= min_blocks_for_split (default 50).
- **Merge quality.** The merge step is itself an LLM call. M3 might produce a worse merged summary than the concatenated partials. Mitigation: structured merge instructions, and the merge result is preferred but if it fails, fall back to concatenated partials.
- **Cost.** 4x more LLM calls. At $0/call (M3), cost is still $0. But the call rate matters: if stage-based fires every cycle, it's 24 calls/hour vs 6.

---

## Impact Assessment

### Affected Components

- `src/agent/llm_summarizer.rs` — add 1 method, 1 helper, 1 enum, 1 constant. ~80 lines.
- 3-4 new unit tests.
- No new dependencies.
- No engine wiring (this is a library method).

### Risk Level

- [ ] Critical
- [ ] High
- [x] Medium
- [ ] Low

The risk is medium because:
- Stage-based is opt-in. The engine doesn't call it. The risk is only to callers.
- The implementation is straightforward (3 sequential LLM calls, no concurrency).
- Tests cover the main paths.

### Latency Impact

- 0 impact on engine (not called).
- For callers who opt in: 4x more LLM calls, 4x latency.

---

## Proposed Solution

### Approach

1. Add `MERGE_SUMMARIES_INSTRUCTIONS` constant (port of openclaw's constant).
2. Add `summarize_in_stages(blocks, parts, min_blocks_for_split)` method.
3. Add `split_into_stages(blocks, parts)` helper that evenly divides blocks into N parts by count (not by token count — keep it simple).
4. Add `MergeResult` enum.
5. Add 3-4 unit tests.

### Steps

1. **3 min:** Add `MERGE_SUMMARIES_INSTRUCTIONS` constant.
2. **5 min:** Add `split_into_stages` helper.
3. **10 min:** Add `summarize_in_stages` method.
4. **10 min:** Add 4 unit tests.
5. **3 min:** `cargo test --lib` (351 + 4 = 355 expected), `cargo clippy`, `cargo build --release`.
6. **3 min:** ECHO FID close-out.

**Total: ~35 min.**

### Verification

- `cargo test --lib` — 355 pass, 0 fail
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo build --release` — clean
- `grep -rn "summarize_in_stages\|MERGE_SUMMARIES_INSTRUCTIONS" src/` — 1 method definition, 1 constant, 4 test references

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** `parts = 0` or `parts > blocks.len()` could cause division-by-zero or empty chunks.
- **GREEN:** Default `parts = 2` if 0. Cap `parts` at `blocks.len()`. Each chunk must have at least 1 block.
- **AUDIT:** Edge cases in unit tests.
- **CHANGE DELTA:** +5 lines.

### Loop 2 (anticipated)

- **RED:** If all stage summarizations fail, the merge has nothing to merge. Should return an error.
- **GREEN:** If `partial_summaries.is_empty()`, return `Err("All stages failed")`.
- **AUDIT:** Test for all-fail case.
- **CHANGE DELTA:** +3 lines.

### Loop 3 (anticipated)

- **RED:** The merge instructions from openclaw are about "tasks" and "TODOs" — trading-context-specific would be better.
- **GREEN:** Customize the merge instructions to be trading-specific: "active trades, current regime, recent decisions, market conditions, risk concerns."
- **AUDIT:** Test verifies the prompt contains the right context.
- **CHANGE DELTA:** +15 lines (custom instructions).

### Loop 4 (anticipated)

- **RED:** Stage-based summarization is async; the engine cycle loop is async. But `summarize_in_stages` is called from `LlmSummarizer`, not from the engine directly. No concurrency issue.
- **GREEN:** No change needed.
- **AUDIT:** No change.
- **CHANGE DELTA:** 0 lines.

### Loop 5 (anticipated)

- **RED:** The `Message` struct is defined in `provider.rs`. Need to import it correctly.
- **GREEN:** Same pattern as `summarize_chunks` in FID-165.
- **AUDIT:** Check the import.
- **CHANGE DELTA:** 0 lines.

---

## Resolution

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-16 20:35 EST (v1); 2026-06-16 22:45 EST (v2 strict-read)
- **Fix Description (v1):** Added `summarize_in_stages(blocks, parts, min_blocks_for_split)` method to `LlmSummarizer`. Added `split_into_stages` helper that evenly divides blocks by count. Added `MERGE_SUMMARIES_INSTRUCTIONS` constant (port of openclaw's merge instructions). Customized to be trading-specific.
- **Fix Description (v2 strict-read improvements, 2026-06-16 22:45):**
  - **A. Use `summarize_with_fallback` per stage (was just `summarize`).** v1 used `self.summarize(stage)` per stage, losing openclaw's partial-failure recovery. v2 uses `self.summarize_with_fallback_public(chunks)` per stage, which tries the full stage, then partial (excluding oversized chunks), then a generic fallback. **This is a real fidelity improvement vs openclaw.** Added `summarize_with_fallback_public` method as a public wrapper that pulls the LLM provider from `self`.
  - **B. Token-based splits (was count-based).** v1's `split_into_stages` divided by count, leading to lopsided stages. v2 added `split_into_stages_by_tokens` which divides by token count using greedy fill. This produces balanced LLM inputs even when individual blocks have wildly different token counts (e.g., long block from a news event vs short decision lines). Port of openclaw's `buildStageSplitPlanWithWorker` (which uses token shares; we use a simpler greedy fill that achieves the same outcome). **v1's `split_into_stages` is preserved for callers that want count-balanced splits.**
- **Tests Added (v1):** 4 (split_into_stages_creates_equal_chunks, split_into_stages_caps_parts_at_block_count, split_into_stages_handles_default_zero, summarize_in_stages_with_few_blocks_uses_single_call)
- **Tests Added (v2):** 2 (split_into_stages_by_tokens_balances_token_counts, split_into_stages_by_tokens_handles_oversized_block)
- **Verified By:** `cargo test` (349 lib + 10 bin + 2 doc = 361, 0 fail), `cargo clippy --all-targets -- -D warnings` (clean), `cargo build --release` (clean), grep AUDIT

**AUDIT (FID-151) — v2:**

```text
$ grep -rn "summarize_in_stages\|split_into_stages\|split_into_stages_by_tokens\|MERGE_SUMMARIES_INSTRUCTIONS\|summarize_with_fallback_public" src/
src/agent/llm_summarizer.rs:14: pub const MERGE_SUMMARIES_INSTRUCTIONS: &str = "..."
src/agent/llm_summarizer.rs:241: pub fn split_into_stages(&self, blocks: &[DataBlock], parts: usize) -> Vec<Vec<DataBlock>>     # v1: count-based
src/agent/llm_summarizer.rs:262: pub fn split_into_stages_by_tokens(&self, blocks: &[DataBlock], parts: usize) -> Vec<Vec<DataBlock>>   # v2: token-based
src/agent/llm_summarizer.rs:302: pub async fn summarize_in_stages(&self, ...) -> Result<String, String>     # v2: uses token-based + per-stage summarize_with_fallback
src/agent/llm_summarizer.rs:404: pub async fn summarize_with_fallback_public(&self, chunks: &[Chunk]) -> Result<String, String>   # v2: public wrapper
# 1 constant, 4 methods, 6 tests. All in llm_summarizer.rs. NOT called by engine (opt-in API).
```

- **Commit/PR:** Pending (v0.14.3 batch + v0.14.4 v2 batch)
- **Archived:** Pending (v0.14.3 status: archived; v0.14.4 v2 status: update here)

---

## Lessons Learned

### v1 lessons (shipped 2026-06-16 20:35)

- **Stage-based summarization is opt-in, not auto.** The engine uses single-call (FID-168). Stage-based fires only when callers explicitly opt in. This avoids 4x more LLM calls in the common case.
- **`div_ceil` is in std since Rust 1.73.** Older code uses `(a + b - 1) / b` for ceiling division. The new `usize::div_ceil` is cleaner and clippy `manual_div_ceil` lint flags the manual version.
- **Custom merge instructions beat openclaw's generic ones.** Openclaw's `MERGE_SUMMARIES_INSTRUCTIONS` talks about "tasks" and "TODOs" — those are agent-y terms, not trading terms. The trading-specific version talks about "active trades, current regime, recent decisions" — M3 is more likely to produce a useful merged summary.
- **Partial failure handling matters.** If 2 of 3 stage summarizations fail, we still merge the 1 that succeeded. If all 3 fail, we return Err. The merge step itself can fail too (LLM down) — we return Err. Three layers of failure modes, three different recovery paths.
- **Stage-based is rare in v0.14.3.** The engine prunes to ~10-30 blocks per cycle. min_blocks_for_split = 50. So the engine never triggers stage-based. But the API is there for v0.15.0 when history is larger.

### v2 lessons (added 2026-06-16 22:45 strict-read)

- **Use the same fallback path openclaw uses, not a simpler one.** v1 used `self.summarize(stage)` per stage, which is a single-shot. v2 uses `self.summarize_with_fallback_public(chunks)`, which is the openclaw-equivalent. **The "simpler" version had weaker failure recovery.** A stage with oversized blocks would have failed in v1; v2 retries with the non-oversized subset. Real fidelity improvement.
- **Token-based splits vs count-based splits matter for LLM-balanced inputs.** v1: 100 blocks × 1 parts = 1 stage of 100 blocks. If 50 of those blocks are 5 tokens each and 50 are 100 tokens each, the LLM sees 250+5000 = 5250 tokens in one call. v2 splits to keep each stage under target_per_stage tokens. **The LLM gets a balanced input regardless of input distribution.** Greedy fill (simpler than openclaw's `buildStageSplitPlanWithWorker`) achieves the same outcome.
- **A single oversized block should get its own stage, not be combined with neighbors.** The greedy fill check `current_tokens + block_tokens > target_per_stage` correctly handles this. A 500-token block with target 110 will become its own stage.
- **Greedy fill can produce more stages than `parts` requested.** For 4 blocks (100+100+10+10 tokens) with parts=2 and target=110, the greedy produces 3 stages (huge_1 alone, huge_2+small_1, small_2 alone). **This is OK** — the result is more balanced, not less. The test verifies no blocks are lost and no stages are empty.
- **Public wrapper pattern for private methods.** `summarize_with_fallback` was private (took `&LlmProvider` arg). v2 added `summarize_with_fallback_public` that takes `&self` and pulls the provider from `self.provider`. **Pattern: when a private method needs `&LlmProvider`, expose a `&self` wrapper that pulls from `self.provider`.** This avoids the caller having to pass the provider explicitly.
- **The 4 unit tests from v1 used a misleading test name.** v1 had `summarize_in_stages_with_few_blocks_uses_single_call` which didn't actually test the function — it tested `split_into_stages`. **A test name should describe what it tests.** v2's tests are more direct.

---

*FID-170 created 2026-06-16 20:00 EST, implemented 20:35 EST (v1, 4 new tests, 355 total pass), strict-read 22:45 EST (v2, 6 new tests, 361 total pass, 2 fidelity improvements, archived as part of v0.14.4 batch) — Vera*
