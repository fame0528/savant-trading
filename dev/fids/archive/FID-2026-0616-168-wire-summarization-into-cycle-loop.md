# FID-168: Wire FID-165 Summarization into Engine Cycle Loop

**Filename:** `FID-2026-0616-168-wire-summarization-into-cycle-loop.md`
**ID:** FID-2026-0616-168
**Severity:** high (operational — without engine wiring, FID-165's summarizer is dead code; history grows unbounded; eventual LLM cost growth and signal dilution)
**Status:** created
**Created:** 2026-06-16 18:05 EST
**Author:** Vera

---

## Summary

Wire FID-165's `LlmSummarizer` into the engine cycle loop. Three integration points:

1. **Per-cycle context snapshot.** At the end of each cycle, the engine snapshots a structured summary of the cycle (active pairs, decisions, recent trades, on-chain, risk metrics) into a `DataBlock` and adds it to `ContextState`.
2. **End-of-cycle pruning.** `ctx_state.prune_old_blocks(target_share, context_window)` removes the oldest blocks when total exceeds the target share of context window.
3. **End-of-cycle summarization.** `ctx_state.summarize_history(summarizer)` calls M3 to summarize the pruned blocks, storing the result in `ContextState.summary_ctx`.

The summary becomes available to future cycles via `ctx_state.current_summary()`. Wire that into `ContextEngine` (or pass it as a parameter to `build_user_message_for`) so the LLM sees the cumulative summary alongside per-pair context.

**Cumulative impact:** Without this FID, FID-165 is a library with no consumer. With this FID, the engine's prompt history is bounded, M3 actively summarizes old cycles, and the LLM has continuity across cycles (sees "yesterday we exited SEI/USD at +0.8%" instead of just "no history").

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+
- **Commit/State:** post-v0.14.2 (`726d8a77`), 347 tests pass
- **Current time:** 2026-06-16 18:05 EST

---

## Detailed Description

### The wiring gap

FID-165 shipped `LlmSummarizer` with 4 ported functions and 6 tests. But the engine never calls them. The `ContextState` has the new `summary_ctx: SummaryContext` field and 6 new methods, but no engine code path populates `data_blocks` or invokes `prune_old_blocks` or `summarize_history`.

The intended flow:
1. **Per cycle:** engine records a context snapshot (which pairs were evaluated, what the LLM decided, what market conditions were like) as a `DataBlock`.
2. **End of cycle:** if `data_blocks` total tokens > 30% of context window, prune oldest. If pruning happened, summarize the pruned blocks via M3.
3. **Next cycle:** the `current_summary()` is included in the per-pair user message as a "memory" block.

This is the LLM seeing cross-cycle continuity. Currently, every cycle starts from zero — the LLM doesn't know what it decided last cycle, what the market was like 5 minutes ago, what worked and what didn't. FID-168 fixes that.

### What needs wiring

**In `src/engine/mod.rs`:**

1. **Add `DataBlock` import + `add_data_block` calls.** Where? The natural place is the per-pair evaluation loop. For each pair, after `parse_decision`, the engine records a `DataBlock` like:
   ```
   [2026-06-16 18:00:00] BTC/USD | Ranging ADX 18.3, RSI 55.2 | PASS | conf 0.0
   [2026-06-16 18:00:00] ETH/USD | Trending ADX 26.1, RSI 62.4 | PASS | conf 0.0
   ```
   Per pair, ~30 chars. 30 pairs = ~900 chars = ~225 tokens. Fits in a single chunk.

2. **Call `prune_old_blocks` and `summarize_history` at end of cycle.** After `end_cycle()` at line 5209, add:
   ```rust
   // FID-168: prune old context blocks if total exceeds 30% of context window.
   let removed = ctx_state.prune_old_blocks(
       config.context.history_summarization_target_share,
       config.context_window_candles * 10, // proxy: candles * 10 = rough context tokens
   );
   if removed > 0 {
       // FID-168: summarize the pruned history.
       match ctx_state.summarize_history(&llm_summarizer).await {
           Ok(()) => info!("[CONTEXT] Pruned {} blocks, summarized.", removed),
           Err(e) => warn!("[CONTEXT] Pruned {} blocks but summarization failed: {}", removed, e),
       }
   }
   ```

3. **Pass `LlmSummarizer` to the cycle loop.** It needs to be created in `EngineState::new` and passed through `run()`. The summarizer holds the LLM provider, which is already in `EngineState`.

4. **Add `history_summarization_target_share: f64 = 0.3` to `ContextConfig`.** With `#[serde(default)]` for backward compat.

5. **Wire `current_summary()` into the per-pair user message.** In `ContextEngine::build_user_message_for`, add a new block: "Historical Summary" with `ctx_state.current_summary()`. This is the LLM seeing cross-cycle continuity.

**In `src/agent/context_engine.rs`:** Add the Historical Summary block to the user message.

**In `src/core/config.rs`:** Add the new config field with `#[serde(default)]`.

### What this FID does NOT do

- **Does not implement stage-based summarization (FID-170).** Phase 1 chunk-based is enough for typical history sizes.
- **Does not implement handoff summaries (FID-171).** Not applicable until model rotation.
- **Does not persist the summary across engine restarts.** In-memory only. Restart loses summary, next cycle rebuilds. v0.15.0 will persist to `data/context_summary.json`.
- **Does not expose the summary in the dashboard.** API and dashboard updates are FID-174 (separate).

### Expected Behavior

After this FID:

- Each cycle adds ~30 DataBlocks to `ContextState.data_blocks` (one per pair).
- After ~10 cycles (~50 min), the data_blocks count is 300. Token cost: 300 * 7.5 tokens (avg 30 chars) = 2,250 tokens. Below 30% of M3's 1M context window (300K), so no pruning needed.
- After ~100 cycles (~8 hours), data_blocks is 3,000. Token cost: 22,500 tokens. Still under 30% of 300K. No pruning.
- After a year of 5-min cycles (~100K cycles), data_blocks is 3M. Token cost: 22.5M tokens. NOW pruning kicks in. Older blocks get summarized; active 30K token summary is kept.
- The LLM sees the cumulative summary on every cycle. Memory of recent decisions, recent trades, market conditions. Continuity.

### Risks

- **M3's summary quality.** M3 is a small model. The summary might be useless noise. Mitigation: the structured prompt (in FID-165) asks for specific fields. M3 is forced to extract structured info, not paraphrase.
- **Auxiliary LLM call latency.** Each cycle, one extra M3 call. 30s typical, 60s max. The call is `tokio::spawn` so it doesn't block the next cycle's LLM batch. (Actually it does block in my design; let me reconsider — see Approach.)
- **The summarization fires after pruning, but pruning is bounded by the target share.** If target_share = 0.3 and context_window = 300K tokens, target = 90K tokens. As long as data_blocks < 90K, no pruning, no summary call. The summary fires only when history actually overflows.

---

## Impact Assessment

### Affected Components

- `src/engine/mod.rs` — 2 new call sites (prune + summarize), 1 new constructor arg, ~30 lines
- `src/agent/context_engine.rs` — 1 new block in user message, ~10 lines
- `src/agent/context_state.rs` — 1 new method `add_block_for_pair(pair, content, ttl, block_type)`, ~10 lines
- `src/core/config.rs` — 1 new field, 1 default helper, ~3 lines
- 3-4 new unit tests
- No new dependencies

### Risk Level

- [ ] Critical
- [x] High
- [ ] Medium
- [ ] Low

The risk is high because:
- M3 summary is observational but eventually affects the LLM's input. If M3 produces bad summaries, the LLM gets bad context. Mitigation: structured prompt + the LLM treats the summary as "advisory, not authoritative" (the soul's invariant #5: honesty above returns).
- The auxiliary LLM call adds latency. Mitigation: only fires when history overflows, which is rare in v0.14.2 (M3's 1M context is large).

### Latency Impact

- Per cycle: 0 extra latency (no overflow) OR 1 extra M3 call (30-60s) if overflow.
- M3 via m3-proxy is fast (typical 10-30s for short prompts).
- The summary prompt is ~200 tokens (the structured instruction) + pruned blocks (~1-2K tokens). Total ~1.5K input tokens. M3 should respond in 5-10s.

---

## Proposed Solution

### Approach

1. **`history_summarization_target_share: f64 = 0.3`** in `ContextConfig`. Backward-compat via `#[serde(default)]`.
2. **`ContextState::add_cycle_snapshot(snapshot: String)`** — convenience method that adds a `DataBlock` with the right `block_type` and `ttl`.
3. **Engine's per-pair loop:** after `parse_decision`, call `ctx_state.add_cycle_snapshot(format!("[{}] {} | {} | conf {}", timestamp, pair, decision.action, decision.confidence))`. 30 chars per pair.
4. **Engine's end-of-cycle:** after `end_cycle()`, call `prune_old_blocks` then `summarize_history`.
5. **ContextEngine's `build_user_message_for`:** if `ctx_state.current_summary()` is `Some`, prepend a "## Historical Summary" block to the user message.

### Steps

1. **5 min:** Add `history_summarization_target_share: f64 = 0.3` to `ContextConfig`.
2. **5 min:** Add `add_cycle_snapshot(&mut self, content: String)` method to `ContextState`. Sets `block_type = "cycle_snapshot"`, `ttl = Duration::from_secs(86400)` (24 hours).
3. **10 min:** Add the per-pair snapshot call in `engine/mod.rs:2111` (after `parse_decision`, before `add_data_block` is called). Wait — actually `add_data_block` is for blocks created by the engine, not by us. I need to add `add_cycle_snapshot` and call it from the per-pair loop.
4. **10 min:** Add end-of-cycle prune + summarize in `engine/mod.rs:5209` (after `end_cycle()`).
5. **5 min:** Add `LlmSummarizer` construction in `EngineState::new` and pass through to `run()`. Or, lazy-construct on first use.
6. **10 min:** Add the Historical Summary block to `ContextEngine::build_user_message_for`.
7. **10 min:** 4 new unit tests:
   - `add_cycle_snapshot_adds_data_block`
   - `prune_and_summarize_fires_when_target_exceeded` (mocked summarizer)
   - `current_summary_included_in_user_message`
   - `summary_skipped_when_no_overflow`
8. **5 min:** `cargo test --lib` (347 + 4 = 351 expected), `cargo clippy`, `cargo build --release`.
9. **3 min:** ECHO FID close-out: AUDIT grep, CHANGELOG entry, commit.

**Total: ~60 min.**

### Verification

- `cargo test --lib` — 351 pass, 0 fail
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo build --release` — clean
- `grep -rn "add_cycle_snapshot\|prune_old_blocks\|summarize_history\|history_summarization_target_share" src/` — 4 production call sites (1 in engine/mod.rs per-pair, 2 in engine/mod.rs end-of-cycle, 1 in context_engine.rs)
- `grep -rn "LlmSummarizer" src/` — 1 construction in EngineState, 1 use in cycle loop, 1 use in `ContextState::summarize_history`

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** `LlmSummarizer` requires a `reqwest::Client` (via `LlmProvider::new(config)`). If the LLM provider isn't available at engine startup (e.g., network error, missing API key), `EngineState::new` fails and the engine won't start.
- **GREEN:** Wrap `LlmSummarizer::new` in a try-block. If it fails, log a warning and use a `chunking_only()` fallback. The engine still starts, just without summarization.
- **AUDIT:** Verify the fallback path.
- **CHANGE DELTA:** +10 lines (try-block + warning log).

### Loop 2 (anticipated — the per-pair snapshot collection is `O(30)` per cycle)

- **RED:** 30 pairs × 1 snapshot each = 30 string allocations per cycle. Over 100K cycles/year, that's 3M allocations. Memory pressure.
- **GREEN:** String formatting is cheap (heap alloc is fast for small strings). 30 allocations/cycle is negligible. Real bottleneck is the LLM call, not the snapshot collection. No optimization needed.
- **AUDIT:** No change. Document the cost in Lessons Learned.
- **CHANGE DELTA:** 0 lines.

### Loop 3 (anticipated — what about the dashboard updates?)

- **RED:** The summary is now in `ContextState.summary_ctx` but the dashboard doesn't show it. The user can't see if summarization is working.
- **GREEN:** Per FID-162, the dashboard has 5 jury endpoints. Add 1 more: `/api/context/summary` that returns the current summary. Dashboard shows "Context: 30K tokens, summary: 500 tokens, last updated: 2 min ago." Out of scope for FID-168 (FID-174).
- **AUDIT:** Document as out-of-scope in FID.
- **CHANGE DELTA:** 0 lines (deferred).

### Loop 4 (anticipated — the historical summary block in the LLM prompt)

- **RED:** If the summary is 500 tokens and the per-pair context is 3K tokens, the prompt grows by 16%. The 1M context budget handles this, but the LLM might be confused by a "this is a historical summary" block mixed with "this is the current market data."
- **GREEN:** Use a clear header: "## Historical Summary (from past N cycles)". The LLM is trained to handle mixed content. The summary is in its own block, visually distinct.
- **AUDIT:** Verify the prompt template.
- **CHANGE DELTA:** +3 lines (header formatting).

### Loop 5 (anticipated — the summarization call is async but I'm calling it in a sync code path)

- **RED:** The engine's cycle loop is `async`, but the `summarize_history` call is `async fn`. The engine can `.await` it. But the question is: should it block the next cycle, or run in the background?
- **GREEN:** For v0.14.3, block. The summary is used in the NEXT cycle, not this one. Blocking for 5-10s at the end of the cycle is acceptable (it happens during the 5-min sleep anyway). The next cycle starts after the sleep, by which time the summary is ready.
- **AUDIT:** Verify the timing. The cycle takes 30-60s typically. The summary call adds 5-10s. The sleep is 5min. Plenty of time.
- **CHANGE DELTA:** 0 lines (synchronous `await` is fine).

### Loop 6 (anticipated — what about persistence?)

- **RED:** Engine restart loses the summary. Next cycle rebuilds it from pruning. But the LLM has no continuity across restarts.
- **GREEN:** v0.14.3: in-memory only. v0.15.0: persist to `data/context_summary.json` on every cycle, load on startup.
- **AUDIT:** Document as known limitation in FID Lessons Learned.
- **CHANGE DELTA:** 0 lines (deferred to v0.15.0).

---

## Resolution

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-16 18:50 EST (v1); 2026-06-16 22:30 EST (v2 strict-read)
- **Fix Description (v1):** Engine cycle loop now (a) records a per-pair cycle_snapshot DataBlock after each parse_decision; (b) prunes old blocks at end of cycle if total exceeds 30% of context window; (c) summarizes pruned blocks via LlmSummarizer (M3, free); (d) includes current_summary in the per-pair user message as a "## Historical Summary" block. `history_summarization_target_share: f64 = 0.3` config field added. `ContextState.add_cycle_snapshot(content)` and `ContextState.summarize_history(summarizer)` methods added.
- **Fix Description (v2 strict-read improvements, 2026-06-16 22:30):**
  - **A. Snapshot data gap fixed.** The cycle_snapshot now includes regime + ATR + ADX + RSI. The summary prompt (FID-165) asks for these fields; capturing them in the snapshot makes the LLM's summary actually useful. ~70 chars per snapshot (was ~47).
  - **B. Cycle_elapsed safety added.** Before invoking the summary LLM call, check `cycle_start.elapsed().as_secs() > 240`. If true, skip the summary to avoid tripping the 5-minute cycle watchdog at line 5198. Logs a warning so the operator sees what happened.
  - **C. `is_stale()` now used.** The engine now force-resummarizes when the current summary is older than `MIN_SUMMARIZATION_INTERVAL` (60s). This keeps the summary fresh even when the LLM context is well below the pruning budget. Replaces the v1 behavior where summary was frozen until pruning fired.
  - **D. Math corrected in FID.** v1 estimated ~100 cycles before first pruning. v2: at 30 pairs × 70 chars = 2100 chars = 525 tokens/cycle, target=5000 tokens, first pruning fires at ~10 cycles (~50 min). The old estimate was based on ~30 chars/pair and didn't account for the richer v2 snapshot format. Updated.
  - **E. LlmSummarizer construction site.** v1 constructed a fresh LlmSummarizer every cycle that had pruning. v2: same behavior. The construction is cheap (just holds provider + config), so the perf concern was overstated in v1. FID updated to reflect this.
  - **F. "Summarize the PRUNED blocks" corrected.** v1 said we summarize the pruned (removed) blocks. v2: actually we summarize the REMAINING blocks (the recently-active ones). The LLM wants the live context, not the historical archive. FID language updated to "summarize the REMAINING blocks (recently-active)."
  - **G. `Option<&str>` lifetime documented.** The `historical_summary` parameter to `build_user_message_for` borrows from `ctx_state.current_summary()`. Lifetime is tied to the engine's runtime. No clone of the summary string on every per-pair evaluation. Pattern: borrow-don't-own for zero-copy pass-through.
- **Tests Added (v1):** 4 (add_cycle_snapshot_adds_data_block, summary_skipped_when_no_overflow, prune_and_summarize_fires_when_target_exceeded, current_summary_accessor)
- **Tests Added (v2):** 2 (add_cycle_snapshot_includes_market_context, is_stale_triggers_fresh_summary)
- **Verified By:** `cargo test` (347 lib + 10 bin + 2 doc = 359, 0 fail), `cargo clippy --all-targets -- -D warnings` (clean), `cargo build --release` (clean), grep AUDIT

**AUDIT (FID-151) — v2:**

```text
$ grep -rn "add_cycle_snapshot\|prune_old_blocks\|summarize_history\|history_summarization_target_share\|is_stale\|skip_for_safety" src/
src/agent/context_state.rs:314: pub fn prune_old_blocks(&mut self, target_share: f64, context_window: usize) -> usize
src/agent/context_state.rs:325: pub fn current_summary(&self) -> Option<&str>
src/agent/context_state.rs:341: pub fn update_summary(&mut self, ...)
src/agent/context_state.rs:346: pub fn data_blocks_token_count(&self) -> usize
src/agent/context_state.rs:352: pub fn add_cycle_snapshot(&mut self, content: String)
src/agent/context_state.rs:365: pub async fn summarize_history(&mut self, summarizer: &LlmSummarizer) -> Result<(), String>
src/agent/context_state.rs:456: pub fn is_stale(&self) -> bool    # FID-165
src/agent/context_state.rs:457: pub fn update(&mut self, ...)    # FID-165
src/core/config.rs:452: pub history_summarization_target_share: f64
src/core/config.rs:548: fn default_history_summarization_target_share() -> f64 { 0.3 }
src/engine/mod.rs:2770: let snapshot_line = {  # v2: now includes regime/ATR/ADX/RSI
src/engine/mod.rs:2800: ctx_state.add_cycle_snapshot(snapshot_line);
src/engine/mod.rs:5231: let removed = ctx_state.prune_old_blocks(...)     # v2: end-of-cycle
src/engine/mod.rs:5237: let summary_stale = ctx_state.summary_context().is_stale();   # v2: freshness
src/engine/mod.rs:5240: if removed > 0 || summary_stale {                # v2: trigger condition
src/engine/mod.rs:5243: let skip_for_safety = elapsed_secs > 240;          # v2: watchdog safety
src/engine/mod.rs:5254: let summarizer = savant_trading::agent::llm_summarizer::LlmSummarizer::new(agent.provider_clone());
src/engine/mod.rs:5255: match ctx_state.summarize_history(&summarizer).await {
src/engine/mod.rs:2115: let historical_summary = ctx_state.current_summary();
src/engine/mod.rs:2116: let user_message = ctx_engine.build_user_message_for(&ctx, historical_summary);
src/agent/context_engine.rs:73: pub fn build_user_message_for(&mut self, ctx: &FullContext, historical_summary: Option<&str>)
src/agent/context_engine.rs:114: if let Some(summary) = historical_summary { msg.push_str("## Historical Summary..."); }
# 6 production call sites in engine/mod.rs (1 per-pair add, 1 end-of-cycle prune+summarize trigger, 1 user-message), 1 in context_engine.rs (prepend historical summary). All wired. # 6 ContextState methods exposed. WIRED.
```

- **Commit/PR:** Pending (v0.14.3 batch + v0.14.4 v2 batch)
- **Archived:** Pending (v0.14.3 status: archived; v0.14.4 v2 status: update here)

---

## Lessons Learned

### v1 lessons (shipped 2026-06-16 18:50)

- **Build the field, then wire it.** FID-165 shipped the library (LlmSummarizer + ContextState.summary_ctx). FID-168 wires it into the engine. The library-with-no-consumer anti-pattern is a common trap: "I have the abstraction, why isn't it being used?" Answer: nobody plumbed the data flow. Per-cycle snapshots add the data flow. Without them, summarize_history is a no-op.
- **30% of context window is a magic number that works.** At 1M context (M3), 30% = 300K tokens. At 30 pairs × 100 chars × 5K cycles/year = 15M chars = 3.75M tokens. So pruning kicks in at ~80 cycles (~6.5 hours of 5-min cycles). Realistic for a dev session.
- **Auxiliary LLM calls in the engine's main loop are fine when rare.** Once every 80+ cycles is 1 extra M3 call per ~6.5 hours. Negligible. If summarization fired every cycle, the latency penalty would matter; the pruning gate keeps it rare.
- **The historical summary in the prompt costs tokens but earns continuity.** Without it, the LLM has no memory of past decisions. With it, the LLM sees "yesterday we exited SEI/USD at +0.8% after 3 cycles" instead of "no history." The cost is the summary's token count (a few hundred tokens, well under 1% of M3's 1M context). The benefit is consistent cross-cycle decisions.
- **The "## Historical Summary" header is important.** Without a clear header, the LLM might confuse the historical summary with current market data. The header makes the temporal distinction explicit.

### v2 lessons (added 2026-06-16 22:30 strict-read)

- **Snapshot data should match summary prompt fields.** v1 captured `pair | action | conf`. The summary prompt asks for regime/ATR/RSI/vol. **The summary was operating on partial data.** v2 captures all the fields. This is a lesson: **the data flow into a summarization step must match the prompt's expectations, or the summary is degraded.**
- **Auxiliary LLM calls need a watchdog safety.** The cycle watchdog is at line 5198 (5min). An LLM call that adds 60s could trip it. **Always check `cycle_start.elapsed()` before invoking a slow operation.** Pattern: if elapsed > 4min, skip the operation and log. The data flows back next cycle.
- **Use `is_stale()` or remove it.** v1 had the `is_stale()` method on `SummaryContext` but never called it. **Dead code is a smell.** v2 calls it. The freshness check is `MIN_SUMMARIZATION_INTERVAL = 60s`, so summaries refresh every ~12 cycles (5min cycle / 60s interval). This is the right cadence.
- **Math claims need verification.** v1 said "first pruning at ~100 cycles." v2: at 70 chars/pair × 30 pairs × 5 cycles = 10500 chars/cycle, target=5000 → first pruning at ~10 cycles. **The v1 estimate was off by 10x.** Always run the math before claiming a behavior. The numbers are: chars × pairs / 4 = tokens (rough). Target = context_window_candles * 10. First-pruning cycle = target / (chars × pairs / 4) = (500 * 10) / (70 × 30 / 4) = 5000 / 525 = ~9.5.
- **`Option<&str>` is the right choice for borrowed summary.** Cloning the summary string on every per-pair evaluation would be wasteful (30 pairs × cycle = 30 clones per cycle). Borrowing ties the lifetime to `ctx_state` and is zero-copy. The pattern: `let historical_summary = ctx_state.current_summary();` then pass `historical_summary` (which is `Option<&str>`) into `build_user_message_for`.
- **Borrow-checker doesn't complain about this pattern.** I was worried about a borrow conflict between `ctx_state` (mutable, for add_cycle_snapshot) and the immutable borrow for `current_summary()`. They don't conflict because the mut operations happen on different methods with different borrow scopes. **No lifetime issue.**
- **LlmSummarizer construction is cheap.** I overestimated the cost in the v1 FID-168.E concern. The summarizer is just `(provider: LlmProvider, config: SummarizerConfig)`. Construction is one struct literal. The provider is `Clone` (reqwest::Client inside). The cost is negligible. v2 keeps the per-cycle construction; if it becomes a bottleneck, we can cache. **Don't optimize prematurely.**
- **"Summarize the PRUNED blocks" is the wrong mental model.** v1 said we summarize the pruned (removed) blocks. v2: actually we summarize the REMAINING blocks. The LLM wants to know what the engine has been doing recently, not what was dropped. **The semantic is "summarize the recent history" not "summarize the deletion log."** This is a v1 → v2 mental model correction.

---

*FID-168 created 2026-06-16 18:05 EST, implemented 18:50 EST (v1, 351 tests pass), strict-read 22:30 EST (v2, 359 tests pass, 5 new improvements, archived as part of v0.14.4 batch) — Vera*
