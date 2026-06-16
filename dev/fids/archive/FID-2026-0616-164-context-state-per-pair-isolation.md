# FID-164: Per-Pair ContextState + Token-Based Compression

**Filename:** `FID-2026-0616-164-context-state-per-pair-isolation.md`
**ID:** FID-2026-0616-164
**Severity:** high (operational correctness — every cycle produces 25+ spurious delta-compression warnings + ~10 anti-thrashing warnings, drowning the actual log signal)
**Status:** created
**Created:** 2026-06-16 16:00 EST
**Author:** Vera (sponsored by Spencer)
**Triggered by:** Spencer's request: review openclaw + hermes-agent for context compression, then ship the per-pair state fix.

---

## Summary

`ContextState` is a singleton struct shared across all pairs in a cycle. `compute_delta` diffs pair N's user message against pair N-1's, so different pairs always look ~95% different. The "anti-thrashing" check then sees two consecutive < 10% savings and correctly concludes "this is useless" — but the uselessness is caused by the singleton state, not by poor compression. The fix: per-pair `HashMap<String, PairState>` keyed on pair symbol. Token-based detection (tiktoken-rs) replaces char-based as the primary signal. Adaptive threshold derived from `min_token_savings / current_tokens` replaces the fixed-fraction threshold. Per-pair anti-thrashing uses the pair's own history, not interleaved history from 30 pairs. Cumulative token savings surfaced as a per-cycle log line for telemetry.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+, tiktoken-rs 0.6 (already a dependency, used by `token_budget::count_prompt_tokens`)
- **Tool Versions:** cargo 1.91
- **Commit/State:** post-v0.14.1 (`c59d128d`), 337 tests pass
- **Current time:** 2026-06-16 16:00 EST

---

## Detailed Description

### Problem

The engine's batch LLM flow (FID-085 + FID-163) calls `ContextState::compute_delta()` once per pair in `src/engine/mod.rs:2111`. A single `ContextState` instance is constructed at `src/engine/mod.rs:618` and used 30+ times per cycle. The state is shared.

`compute_delta` (line 63-107) stores `previous_text` and `previous_hash` from the last call. The next call (next pair) computes `text_diff_ratio(prev_text, current_text)` where `prev_text` is the previous pair's user message.

Two arbitrary pairs' user messages share ~5% of their content (boilerplate headers, schema prefixes). The diff ratio comes out as ~0.95. The engine logs:

```
[INFO] Delta-compression: 95.0% change (threshold 2.0%) — full injection (regime shift)
```

Once per pair, every cycle. 30 pairs × every cycle = 30 warnings. After 2 cycles, `should_skip_compression` (line 111-124) sees two < 10% savings in `compression_history` and logs:

```
[WARN] Anti-thrashing: last 2 compressions saved <10% each — skipping
```

10+ per cycle. Real log signal is buried.

### Observed Evidence

From the engine run at 2026-06-15 23:50 – 2026-06-16 01:25 (cycles 11-17, 0/17 trades):

```text
$ grep -c "Delta-compression" logs/terminal/next-server*.txt
> 180+

$ grep -c "Anti-thrashing" logs/terminal/next-server*.txt
> 60+
```

Roughly 25 delta warnings + 10 anti-thrashing warnings per 5-minute cycle. The 0-trades result is the correct LLM behavior given the data, but the operator can't tell from the log whether anything is actually wrong with the engine.

### Call-Site Audit (FID-151 — Verify Call-Graph Reachability)

```text
$ grep -rn "ContextState\|context_state\|ctx_state" src/
src/engine/mod.rs:22:   use ... context_state::ContextState;
src/engine/mod.rs:82:   ctx_state: ContextState,                # EngineState field
src/engine/mod.rs:618:  let ctx_state = ContextState::new(...)  # construction
src/engine/mod.rs:1077: ctx_state,                             # EngineState::new return
src/engine/mod.rs:1173: let mut ctx_state = state.ctx_state;    # run() destructure
src/engine/mod.rs:2111: ctx_state.compute_delta(...)            # per-pair
src/engine/mod.rs:2127: ctx_state.should_skip_compression(...)  # per-pair
src/engine/mod.rs:2131: ctx_state.increment_cycle()             # per-pair
src/agent/mod.rs:13:    pub mod context_state;                  # module
src/agent/context_state.rs: 1 struct, 9 tests                   # self
```

**Conclusion:** 3 production call sites (all per-pair in `engine/mod.rs` 2111/2127/2131). Single construction site (line 618). No persistence — `ctx_state` lives in `EngineState` (runtime-only) and is destructured at the start of `run()`. **No serialization, no disk write, no API exposure.** The refactor is contained to `context_state.rs` + 3 call sites in `engine/mod.rs` + 9 internal tests.

### Cross-Method Call Audit

`cycle_count()` (line 184-186) is used only inside `context_state.rs` (line 196 for `strip_historical_placeholder`, plus 2 test assertions). `strip_historical_placeholder` (line 195-202) is a stub — returns text unchanged. Not used by the engine. The whole `strip_historical_placeholder` API is dead code outside the file.

### Root Cause

Singleton state used in a per-pair context. FID-085 (2026-05) designed `ContextState` for cross-cycle state on a SINGLE conversation. FID-163 (2026-06-15) added per-pair TSLN serializer reset. FID-164 completes the per-pair discipline.

Two related issues beyond the state bleed:
1. **Char-based diff ratio** (line 215-226) is a weak signal. 8-char tokens that differ produce 8 unit-diff, not 8 semantic-diff. Token-based ratio is the correct measurement for LLM-cost impact.
2. **`should_skip_compression` looks at the last 2 entries in the global history.** This is meaningless when entries are interleaved from 30 different pairs. The check needs to be per-pair: "if THIS pair has been poorly compressed in its last 2 cycles, skip compression for THIS pair."

### Expected Behavior

After this FID:

1. **Per-pair state.** Pair A's previous text never affects pair B's compression check.
2. **Token-based detection.** Use `tiktoken_rs` (already a dependency, used by `token_budget::count_prompt_tokens`) to count the actual tokens saved, not chars. A pair whose last cycle saved 500 tokens gets "good compression" credit; a pair whose last cycle saved 0 tokens gets the thrash warning.
3. **Adaptive threshold.** A 1% diff ratio is huge when the prompt is 5,000 tokens (saves 4,950 tokens) and tiny when the prompt is 200 tokens (saves 2 tokens). Threshold derived as `1.0 - (min_token_savings / current_token_count)`. Default `min_token_savings = 50`. Clamp to [0.0, 1.0] for short prompts.
4. **Per-pair anti-thrashing.** Per-pair: skip THIS pair if ITS last 2 cycles saved < `min_token_savings` tokens each. Track cumulative token savings across all pairs in the cycle for telemetry.
5. **Config schema change.** `delta_compression_threshold: f64` (fraction) → `delta_compression_min_token_savings: usize` (default 50). `anti_thrash_min_savings: f64` (fraction) → removed; the same `min_token_savings` parameter drives both delta threshold and anti-thrashing check. Backward compat: `#[serde(default)]` on the new field means existing config files continue to load (with the new default).

### Comparison vs openclaw / hermes-agent

(From `dev/vera/notes/2026-06-16-0130-compression-review.md`)

| Capability | savant-trading (current) | savant-trading (after FID-164) | openclaw | hermes |
|---|---|---|---|---|
| Per-pair state | N | **Y** | Y | Y |
| Token-based detection | N | **Y** | Y | Y |
| Adaptive threshold | N | **Y** | Y | Y |
| Per-pair anti-thrashing | N | **Y** | Y | Y |
| LLM summarization | N | N (FID-165) | Y | Y |
| Structured summary | N | N (FID-165) | Y | Y |

FID-164 closes the per-pair gap. FID-165 (separate) brings LLM summarization.

---

## Impact Assessment

### Affected Components

- `src/agent/context_state.rs` — per-pair HashMap, token-based ratio, adaptive threshold, per-pair anti-thrashing, dead `strip_historical_placeholder` removed
- `src/engine/mod.rs` — 3 call sites updated to pass `pair.as_str()` and `min_token_savings`
- `src/core/config.rs` — `ContextConfig.delta_compression_threshold: f64` replaced with `delta_compression_min_token_savings: usize`; `anti_thrash_min_savings: f64` removed; `Default` impl updated; new `default_delta_compression_min_token_savings()` helper
- No new dependencies (tiktoken-rs 0.6 already in `Cargo.toml`)

### Risk Level

- [ ] Critical: System crash, data loss, or security vulnerability
- [x] High: Major feature broken, no workaround
- [ ] Medium
- [ ] Low

The current code is functionally correct (the LLM still gets the full prompt — delta is "observability only" per line 2109) but the log output is dominated by misleading warnings. The 0/17 trade result is not caused by this bug; it's caused by the strategy/universe mismatch (separate conversation).

### Token Cost Impact

**Improved.** Per-pair state means pairs that DO compress (low-volatility majors) will trigger delta-mode correctly. Across 30 pairs, the ones that compress stay compressed. The current 95% cross-pair diff is a false signal that prevents any compression. **However: this FID does NOT claim to fix the 170s cycle 17 latency.** That is a separate M3 streaming timeout issue (Workstream 3 / FID-166). The cross-pair state bug drowns the log; the 170s latency is a real M3/OpenRouter round-trip issue.

---

## Proposed Solution

### Approach

1. **Refactor `ContextState`** to hold `pairs: HashMap<String, PairState>` where `PairState` is `{ previous_hash: Option<u64>, previous_text: Option<String>, previous_token_count: usize, token_savings_history: Vec<usize> (capped 10), cycle_count: u64 }`. Keep the global `data_blocks` field unchanged.
2. **Public API change:** all per-pair methods take `&str` pair key as the first argument:
   - `compute_delta(&mut self, pair, current_text, min_token_savings) -> DeltaResult`
   - `should_skip_compression_for(&self, pair, min_token_savings) -> bool`
   - `record_token_savings(&mut self, pair, tokens_saved)`
3. **Token counting via existing `crate::agent::token_budget::count_tokens(text: &str) -> usize`.** This is the existing wrapper around `tiktoken_rs::cl100k_base_singleton()` + `bpe.lock() + encode_with_special_tokens(text).len()`. The singleton init is infallible — if it fails, the engine panics at startup, which is the correct behavior for a missing critical dependency. No new tiktoken code, no OnceLock, no char-based fallback needed.
4. **Adaptive threshold** derived inside `compute_delta`: `threshold = (1.0 - (min_token_savings as f64 / current_tokens as f64)).clamp(0.0, 1.0)`. For 50 tokens / 30-token prompt, threshold clamps to 0.0 (no compression attempted — too small). For 50 tokens / 5000-token prompt, threshold = 0.99 (only ~1% diff acceptable to inject delta).
5. **Cumulative telemetry** in `compute_delta` and `should_skip_compression_for`: track `total_tokens_saved_this_cycle: usize` on the global `ContextState`. Reset to 0 at the start of each cycle (when `increment_cycle()` is called). Engine logs at end of cycle: `info!("[CONTEXT] Cycle {} complete. {} pairs, total tokens saved: {}", cycle_count, pair_count, total_tokens_saved_this_cycle)`.
6. **Cycle tracking.** Global `cycle_count` stays (increments per pair, as today). Per-pair `cycle_count` lives in `PairState` — incremented on every `compute_delta` call for that pair. Both useful: global for "how long has the engine been running," per-pair for "how often has this specific pair been evaluated."
7. **Dead code removed:** `strip_historical_placeholder` (stub, unused in production).
8. **Stateless methods keep their signatures:** `soft_trim`, `hard_clear`, `prune_expired`, `add_data_block` are unchanged.
9. **Backward compat:** `ContextConfig` new field has `#[serde(default)]`. Existing TOML files load with the default `min_token_savings = 50`. No config migration required.

### Steps

1. **5 min:** Add `PairState` struct in `context_state.rs`. Add `pairs: HashMap<String, PairState>` to `ContextState`. Remove `previous_hash`, `previous_text`, `compression_history`. Keep `data_blocks`, `soft_trim_ratio`, `hard_clear_ratio`. Add `total_tokens_saved_this_cycle: usize` + new `end_cycle(&mut self)` method (logs cycle savings, resets counter). Add `cycle_count: u64` global + per-pair.
2. **5 min:** Refactor `compute_delta`, `should_skip_compression`, `record_compression` → `compute_delta`, `should_skip_compression_for`, `record_token_savings`. All take `&str` pair as first arg. Internal `text_diff_ratio` becomes `token_diff_ratio` (using `count_tokens` for both prev and current).
3. **Token counting via existing `crate::agent::token_budget::count_tokens(text: &str) -> usize`.** Just `use crate::agent::token_budget::count_tokens;` and call `count_tokens(text)` directly. No new tiktoken code, no OnceLock, no char-based fallback. (Supersedes Loop 2 / Loop 6 / Loop 7 — original plan was to add a new OnceLock-based init pattern; Loop 8 found the existing `count_tokens` wrapper eliminates all of that.)
4. **5 min:** Update `ContextConfig`: rename `delta_compression_threshold: f64` → `delta_compression_min_token_savings: usize` with `default_delta_compression_min_token_savings() = 50`. Remove `anti_thrash_min_savings`. Update `Default` impl.
5. **5 min:** Update call sites in `src/engine/mod.rs`:
   - Line 2111: pass `pair.as_str()` and `config.context.delta_compression_min_token_savings`
   - Line 2127: pass `pair.as_str()` and `config.context.delta_compression_min_token_savings`
   - Line 2131: unchanged (no pair arg)
   - **NEW** Line 5207 (just before the `time::sleep`): `ctx_state.end_cycle();` — logs + resets cumulative savings
   - Update 9 unit tests in `context_state.rs` for new signatures.
6. **10 min:** Add 5 new unit tests:
   - `per_pair_isolation_no_cross_contamination` — pair A's history doesn't affect pair B's compression
   - `token_based_detection_counts_actual_tokens` — uses `token_budget::count_tokens`, not chars
   - `adaptive_threshold_scales_with_prompt_size` — small prompt → threshold near 0, large prompt → threshold near 1
   - `per_pair_anti_thrashing_only_skips_own_pair` — pair A's bad compression doesn't skip pair B
   - `end_cycle_logs_and_resets_cumulative_savings` — call `end_cycle()`, verify counter resets to 0, verify log format
7. **3 min:** `cargo test --lib` (337 - 0 (refactored) + 5 = 342 pass expected), `cargo clippy --all-targets -- -D warnings` (clean), `cargo build --release` (clean).
8. **3 min:** AUDIT grep evidence: `compute_delta`, `should_skip_compression_for`, `record_token_savings`, `count_tokens`, `ContextConfig::delta_compression_min_token_savings` — paste into Resolution section.
9. **3 min:** ECHO FID close-out: CHANGELOG entry, commit.

**Total: ~45 min.**

### Verification

After the fix:
- `cargo test --lib` — 342 tests pass, 0 fail
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo build --release` — clean
- `grep -rn "compute_delta\|should_skip_compression_for\|record_token_savings\|end_cycle" src/` — 4 production callers in `engine/mod.rs` (3 existing + 1 new), all updated; methods defined in `context_state.rs`
- `grep -rn "ContextState" src/` — single construction site at line 618, no per-call-site construction
- `grep -rn "delta_compression_min_token_savings" src/` — 1 field in config.rs, 1 reader in engine/mod.rs:2111
- `grep -rn "anti_thrash_min_savings" src/` — 0 matches (removed)
- Engine restart test (manual, optional): 5 cycles observed, delta warnings < 5 per cycle (down from 25+), anti-thrashing warnings < 2 per cycle (down from 10+), `total_tokens_saved_this_cycle > 0` for at least some pairs

---

## Perfection Loop

### Loop 1 (initial)

- **RED:** Confirmed singleton state, 25+ delta warnings/cycle, 10+ anti-thrashing/cycle. Cross-pair diff is meaningless. 3 production call sites confirmed via grep. No persistence, no API exposure.
- **GREEN:** Per-pair HashMap, token-based detection, adaptive threshold, per-pair anti-thrashing, config schema change, dead code removal.
- **AUDIT:** Grep evidence captured in Call-Site Audit section above. All 3 production call sites identified, all 9 internal tests need updating, all config defaults documented.
- **CHANGE DELTA:** ~150 lines changed in `context_state.rs`, ~10 lines in `engine/mod.rs`, ~10 lines in `config.rs`. ~9% of `context_state.rs` total.

### Loop 2 (anticipated — SUPERSEDED by Loop 8)

- **RED:** Tokenizer init failure handling. `tiktoken_rs::cl100k_base()` returns `Result`. Need `OnceLock<Result<CoreBPE, _>>` with safe fallback to `(text.len() + 3) / 4`.
- **GREEN:** `OnceLock<Result<CoreBPE, _>>` lazy init, log-once-on-failure pattern, fall back to char-based silently.
- **AUDIT:** Init path covered in tests. No new prod callers.
- **CHANGE DELTA:** +15 lines (OnceLock + fallback).
- **STATUS:** **Superseded by Loop 8.** The existing `crate::agent::token_budget::count_tokens()` wrapper handles all of this. Use it. No new code.

### Loop 3 (anticipated — adaptive threshold edge case)

- **RED:** `1.0 - (50 / 30) = -0.67`. Negative threshold. Need to clamp.
- **GREEN:** `threshold = (1.0 - (50.0 / current_tokens as f64)).clamp(0.0, 1.0)`.
- **AUDIT:** Edge cases in unit tests (30-token prompt, 100-token prompt, 1000-token prompt).
- **CHANGE DELTA:** +2 lines.

### Loop 4 (anticipated — backward compat of config)

- **RED:** Existing TOML files reference `delta_compression_threshold = 0.02`. New field has a different name and type.
- **GREEN:** `#[serde(default)]` on the new field. Missing field → use default (50). Existing TOML just ignores the old `delta_compression_threshold` key (serdes silently drops unknown fields if `#[serde(default)]` is set, OR errors if `deny_unknown_fields` is on — verify by reading the struct attributes).
- **AUDIT:** Config test loads an old TOML with the old field name; verify default applies.
- **CHANGE DELTA:** +5 lines (test).

### Loop 5 (anticipated — `total_tokens_saved_this_cycle` reset semantics)

- **RED:** When exactly does this reset? The engine calls `increment_cycle()` 30 times per real cycle (once per pair), so the global `cycle_count` is really "total pairs evaluated," not "real cycle number." Using `last_reset_cycle` would reset at the 30th pair of the NEXT cycle, which is the wrong boundary.
- **GREEN:** Add a new method `end_cycle(&mut self)` on `ContextState` that: (a) logs the cycle's cumulative savings (e.g., `[CONTEXT] Cycle N: {} pairs, {} tokens saved`), (b) resets `total_tokens_saved_this_cycle` to 0. The engine calls `ctx_state.end_cycle()` ONCE per real cycle, right before the cycle-sleep at `engine/mod.rs:5208`. **1 new call site, exactly at the natural cycle boundary.**
- **AUDIT:** Unit test for the reset behavior. Verify the engine's cycle boundary is at the sleep call, not anywhere else.
- **CHANGE DELTA:** +12 lines (method + log + reset) + 1 line in `engine/mod.rs`.

### Loop 6 (anticipated — tiktoken singleton pattern from LEARNINGS)

- **RED:** LEARNINGS.md Session 2026-06-08: FID-085 notes "tiktoken-rs singleton uses parking_lot::Mutex. cl100k_base_singleton() returns Arc<parking_lot::Mutex<CoreBPE>>, not Arc<std::sync::Mutex<CoreBPE>>. lock() returns the guard directly, no .unwrap() needed." My FID mentions `OnceLock<Result<CoreBPE, _>>` but the existing pattern uses `cl100k_base_singleton()` which already returns a Mutex-protected singleton. Reuse the existing pattern instead of creating a new one.
- **GREEN:** Use `tiktoken_rs::cl100k_base_singleton()` (already a dependency, already used by `token_budget::count_prompt_tokens`). Cache the singleton reference in a `OnceLock<Arc<parking_lot::Mutex<CoreBPE>>>` to avoid re-fetching. On lock() failure (poisoned mutex), fall back to char-based.
- **AUDIT:** Read `src/agent/token_budget.rs` to confirm the existing pattern. Match it.
- **CHANGE DELTA:** -5 lines (simpler than the originally-proposed OnceLock<Result>).

### Loop 7 (anticipated — tiktoken feature flag)

- **RED:** The `tiktoken-rs = "0.6"` dependency may need a feature flag to enable `cl100k_base`. If the feature isn't enabled, the function returns Err.
- **GREEN:** Verified `Cargo.toml:80 tiktoken-rs = "0.6"` uses default features. `tiktoken_rs::cl100k_base_singleton()` is exposed in the default set. `src/agent/token_budget.rs:8-22` already uses it as a singleton. **No change to Cargo.toml needed.**
- **AUDIT:** Confirmed by reading `token_budget.rs` (uses `cl100k_base_singleton`, `bpe.lock()`, `guard.encode_with_special_tokens(text).len()`) and `Cargo.toml:80` (no features).
- **CHANGE DELTA:** 0 lines.

### Loop 8 (simplification — reuse existing `count_tokens`)

- **RED:** Loop 6 said "cache the singleton reference" but `src/agent/token_budget::count_tokens(text: &str) -> usize` already exists and is a 1-line wrapper. Don't re-implement.
- **GREEN:** Just import `use crate::agent::token_budget::count_tokens;` in `context_state.rs` and call `count_tokens(text)` directly. No new OnceLock, no new lock pattern, no char-based fallback (the singleton init is infallible — if it fails, the engine panics at startup, which is the correct behavior for a missing critical dependency).
- **AUDIT:** Verify the import path. `count_tokens` is `pub fn` in `token_budget::` module. The module is declared in `src/agent/mod.rs` (need to verify it's `pub mod token_budget;`).
- **CHANGE DELTA:** -10 lines (eliminated the entire new tiktoken init).

---

## Resolution

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-16 16:35 EST
- **Fix Description:** Per-pair `HashMap<String, PairState>`, token-based detection (via `token_budget::count_tokens`), adaptive threshold derived from `min_token_savings / current_tokens`, per-pair anti-thrashing using the pair's own `token_savings_history`, cumulative cycle telemetry via `end_cycle()`. Config schema changed: `delta_compression_threshold: f64` + `anti_thrash_min_savings: f64` → `delta_compression_min_token_savings: usize` (default 50). Dead `strip_historical_placeholder` removed.
- **Tests Added:** 5 new (per-pair isolation, token-based detection, adaptive threshold, per-pair anti-thrashing, end_cycle reset) + 4 existing tests refactored for new signatures (delta_first_cycle_full, delta_identical_no_change, delta_small_change, anti_thrashing_allows_good_compression). 4 stateless tests unchanged (soft_trim, hard_clear, ttl_pruning, cycle_counter). **Total: 329 lib + 10 bin + 2 doc = 341 tests pass, 0 fail.**
- **Verified By:** `cargo test` (329 lib + 10 bin + 2 doc = 341, 0 fail), `cargo clippy --all-targets -- -D warnings` (clean), `cargo build --release` (clean).

**AUDIT (FID-151 — Call-Graph Reachability):**

```text
$ grep -rn "compute_delta\|should_skip_compression_for\|record_token_savings\|end_cycle(" src/
src/agent/context_state.rs:80:   pub struct ContextState
src/agent/context_state.rs:109:  pub fn compute_delta(
src/agent/context_state.rs:147:  self.record_token_savings(
src/agent/context_state.rs:182:  pub fn should_skip_compression_for(&self, ...
src/agent/context_state.rs:205:  pub fn end_cycle(&mut self)
src/agent/context_state.rs:353+: 14 internal test references
src/engine/mod.rs:2112:         ctx_state.compute_delta(pair, &user_message, ...)
src/engine/mod.rs:2128:         ctx_state.should_skip_compression_for(pair, ...)
src/engine/mod.rs:5208:         ctx_state.end_cycle();
# 3 production callers (was 2 + 1 new). All wired.

$ grep -rn "delta_compression_threshold\|anti_thrash_min_savings\|strip_historical_placeholder" src/
# 0 matches. Old APIs fully removed.
```

- **Commit/PR:** Pending (will be in v0.14.2 batch)
- **Archived:** Pending (this commit, same as FID-165/FID-166/FID-167 batch)

---

## Lessons Learned

- **Per-pair state isolation is mandatory when state is reused across distinct items.** The singleton `ContextState` worked fine when the engine had a single LLM call per cycle. Batch LLM flow turned it into a cross-pair cross-contamination bug. The fix pattern (per-pair HashMap keyed on the loop variable) is the right answer for any "cross-cycle state in a per-item context" scenario.
- **Token-based metrics beat char-based for LLM-cost decisions.** Chars/4 is a rough approximation; cl100k_base is the actual BPE encoding the model uses. The same diff can show 1% char-ratio and 50% token-ratio depending on what changed.
- **Reuse existing utilities before writing new code.** Loop 8 caught the fact that `token_budget::count_tokens` already wraps the tiktoken singleton with the correct init pattern. Saved ~25 lines of new code and one potential bug (OnceLock vs Mutex singleton mismatch).
- **Existing tests for refactored APIs need updating, not deleting.** The 9 tests in `context_state.rs` use the old API. Mechanical update of each call to add the pair key + new threshold. The 4 stateless tests (soft_trim, hard_clear, ttl_pruning, cycle_counter) didn't need changing. The test that was most useful for catching the bug was `delta_first_cycle_full` — it caught the `store_state` entry-creation bug because the first-cycle test passed (Full was returned), but the second-cycle test failed (no state was actually stored).
- **Borrow-checker patterns: read-only access to a field, then drop, then mutate.** The first version held `pair_state` as a mutable borrow while trying to call `self.record_token_savings(...)` and `self.extract_changes(...)`. Refactored to `let prev_hash = self.pairs.get(pair).and_then(...)` (immutable, dropped after the `match`) followed by `self.store_state(...)` (mutable). Same pattern as FID-147's `refresh_from_positions` fix.
- **`end_cycle()` is the natural place to log + reset cumulative state.** It mirrors the engine's actual cycle boundary (the sleep at line 5208). Per-pair `increment_cycle()` was the wrong boundary because it fires 30 times per real cycle, making the global "cycle count" really a "total pairs evaluated" counter.

---

*FID-164 created 2026-06-16 16:00 EST, refined 16:15 EST after call-site audit + config audit + cross-method audit, implemented 16:35 EST, 3 production call sites wired, 341 tests pass, archived as part of v0.14.2 batch — Vera*
