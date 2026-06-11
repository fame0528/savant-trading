# FID-111: Position-Pair Injection — Held Positions Invisible to LLM

**Filename:** `FID-2026-0610-111-position-pair-injection.md`
**ID:** FID-2026-0610-111
**Severity:** high
**Status:** fixed
**Created:** 2026-06-10 20:15
**Author:** Buffy (Codebuff AI)
**Type:** bug-fix
**Scope:** src/engine/mod.rs

---

## Summary

Positions loaded from the trade journal or wallet-recovery can reference pairs not in the discovery list (`active_pairs`). These pairs never get evaluated by the LLM and are missing from AI Decisions. The agent sees the position on-chain but has no intelligence evaluating it — the trade runs on autopilot.

## Detailed Description

### Problem

STG/USD was loaded from the journal as an open position, but STG was not in `config.trading.pairs` or in the discovered pair list. The engine's main loop only evaluates pairs in `active_pairs`. Therefore STG/USD was never sent to the LLM, never appeared in AI Decisions, and had no stop-management or take-profit intelligence running against it.

### Expected Behavior

Any pair with an open position should always be evaluated by the LLM every cycle. The agent must see current price + position state for stop adjustments, even if the pair wasn't in the original discovery list.

### Root Cause

The engine startup sequence: (1) discover pairs from API, (2) load positions from journal, (3) evaluate only discovered pairs. There is no step that ensures discovered pairs include all position pairs.

### Evidence

```text
[CYCLE] No actionable setups — 0/36 pairs (37 evaluated, 0 dead)
```

The dashboard showed STG/USD as an open position but the AI Decisions list had no STG/USD entry.

### Fix

After position loading (journal + wallet sync), iterate all portfolio positions. Any pair not in `active_pairs` is added to both `active_pairs` and `curated_pairs`, a `MarketDataStore` is created, and historical candles are fetched at startup.

**Code location:** `src/engine/mod.rs` lines ~1130-1190 (FID-111 block).

Also made `active_pairs` mutable (line 149) — required for the push.

## Impact Assessment

### Affected Components

- `src/engine/mod.rs` — `EngineState::new()` startup sequence
- `active_pairs` declaration (line 149)

### Risk Level

- [ ] Critical
- [x] High: Position running without LLM intelligence
- [ ] Medium
- [ ] Low

## Proposed Solution

### Approach

Inject position pairs into the active scanning list after all position loading is complete.

### Steps

1. After wallet sync + stale removal, iterate `portfolio.positions()`
2. For each position pair not in `active_pairs`, push to `active_pairs` + `curated_pairs`
3. Create `MarketDataStore` for newly added pairs
4. Fetch candles via `SourceRouter` for newly added pairs
5. Log the injection for observability

### Verification

1. `cargo clippy -- -D warnings` — 0 warnings
2. `cargo test` — all tests pass
3. Law 4 grep: `grep -n 'FID-111' src/engine/mod.rs` — confirms production wiring
4. Runtime: STG/USD appears in AI Decisions on next cycle

## Perfection Loop

### Loop 1

- **RED:** Position pairs missing from `active_pairs` → invisible to LLM
- **GREEN:** Added FID-111 block after wallet sync; made `active_pairs` mutable
- **AUDIT:** clippy clean, tests pass, grep confirms wiring
- **CHANGE DELTA:** ~60 lines added (position-pair injection block + mut keyword)

## Resolution

- **Fixed By:** Buffy (Codebuff AI)
- **Fixed Date:** 2026-06-10 20:15
- **Fix Description:** Added position-pair injection block in `EngineState::new()` that ensures all position pairs are in `active_pairs` + `curated_pairs` with market stores and candle data
- **Tests Added:** No new unit tests (startup-only code, verified via runtime + clippy)
- **Verified By:** clippy + tests + Law 4 grep
- **Commit/PR:** Pending
- **Archived:** Pending (status: verified → closed after commit)

## Lessons Learned

- Journal-loaded positions can reference pairs not in the config discovery list. Any position with an open trade MUST be in the active scanning set, regardless of how it was loaded.
- The stale-removal block removes positions not in `config.trading.pairs`, but wallet recovery can add them back. The FID-111 injection handles all remaining cases.
