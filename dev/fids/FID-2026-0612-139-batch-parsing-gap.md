# FID-139: Batch Parsing Gap — Missing Pairs Not Defaulted to Pass

**Created:** 2026-06-12 22:45 | **Severity:** high | **Status:** Closed

## RED Phase — Issue Catalog

**Symptom:** Live bot queues 35 pairs for LLM evaluation but only 18-19 appear in dashboard AI Decisions.

**Root Cause:** M3's batch JSON response omits ~17 pairs (the model only includes pairs it has a meaningful opinion on). The parser at `engine/mod.rs:2502-2512` detects the gap and logs a warning, but does NOT create default Pass decisions for the missing pairs. These pairs are silently invisible — no decision record reaches `shared.decisions`, so the dashboard never shows them.

**Impact:**
- Dashboard shows incomplete view of the scan (only 18/35 pairs)
- User can't verify the bot is evaluating all pairs
- Missing pairs are invisible — looks like a partial scan, erodes trust
- The `total_decisions` counter in dashboard only counts parsed pairs

**Evidence:**
- Live bot cycle 1: 35 pairs queued, 18 parsed → `PARSE_DECISION_OUT` for 18 pairs
- Live bot cycle 2: 35 pairs queued, 19 parsed → same pattern
- The batch parser logs "Batch incomplete — missing N pair(s)" but takes no corrective action
- `price_map` contains all 35 queued pairs; `deduped_decisions` contains only 18-19

## GREEN Phase — Fix

### Change 1: `src/engine/mod.rs` — Default missing pairs to Pass

After the batch JSON is parsed and decisions pushed to `all_results`, check which pairs in `price_map` were NOT in the batch response. For each missing pair, create a default Pass decision JSON and push it to `all_results` so it flows through the normal `parse_decision()` → `shared.push_decision()` pipeline.

### Change 2: (if needed) Log improvement

Change the "missing pairs" log from `warn!` to `info!` since we now handle this gracefully.

## AUDIT Phase

- [x] `cargo check` passed
- [x] `cargo test` passed (308/308)
- [x] `cargo clippy` — 2 pre-existing warnings, none from FID-139
- [x] Code review (Nick): approved, minor DRY note on duplicate returned_pairs computation
- [x] Pre-existing test fixed: `conviction_gate_blocks_low_conviction` updated for new thresholds

## COMPLETE

**Verified:** 308/308 tests, cargo check clean, code review passed. Fixed at `src/engine/mod.rs` lines 2517-2595.

**Changes:**
1. `src/engine/mod.rs` — Before for-loop: collect returned_pairs HashSet. After for-loop: if returned < batch_size, generate default Pass JSONs for missing pairs, push to all_results.
2. `src/agent/decision_parser.rs` — Test fix: `conviction_gate_blocks_low_conviction` uses conviction_score=0.19 (below 0.20 Trending threshold).
3. `CHANGELOG.md` — FID-139 entry added to v0.14.0 section.
4. `MASTER-FID.md` — FID-139 added to Recently Completed.
5. `dev/LEARNINGS.md` — 5 key lessons from batch parsing gap session.
