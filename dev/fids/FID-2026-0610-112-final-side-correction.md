# FID-112: Final Side Correction — SHORT Positions Surviving Into Portfolio

**Filename:** `FID-2026-0610-112-final-side-correction.md`
**ID:** FID-2026-0610-112
**Severity:** critical
**Status:** fixed
**Created:** 2026-06-10 20:30
**Author:** Buffy (Codebuff AI)
**Type:** bug-fix
**Scope:** src/engine/mod.rs

---

## Summary

Journal-loaded SHORT positions survive into the portfolio and dashboard despite being on a spot-only DEX where SHORT is impossible. The existing side-correction in the wallet-sync block misses positions that are removed as "stale" (not in config pairs) and then re-added by the executor→portfolio sync, which still holds the original SHORT side from DexTrader's tracker.

## Detailed Description

### Problem

STG/USD was loaded from the journal as SHORT. The stale-removal block deleted it from the portfolio (not in config pairs). The wallet-sync side-correction ran on an empty portfolio and missed it. Later, the executor→portfolio sync at line ~998 added it back from DexTrader's tracker, which still held the original SHORT side. The shared state then synced the SHORT position to the dashboard, causing:
- Dashboard shows STG/USD as SHORT with -$0.34 PnL (actually LONG, profitable)
- TP2 fires incorrectly (SHORT TP levels above entry trigger at any price below them)
- Failed close attempt (no USDC to buy back a SHORT that doesn't exist)

### Expected Behavior

All positions MUST be LONG on a spot-only DEX. No SHORT positions should ever exist in the portfolio or shared state.

### Root Cause

Three separate code paths can add positions to the portfolio: (1) journal loading, (2) wallet recovery, (3) executor→portfolio sync. The side-correction in the wallet-sync block only catches paths 1 and 2. Path 3 (executor→portfolio sync) bypasses it entirely.

### Evidence

```text
[TP2] STG/USD SHORT | Entry: 0.4200 → Exit: 0.4361 | Qty: 27.5626 | PnL: $-0.45 (-3.88%) | TP2 hit — scale out 60% of remaining
[WARN] [DEX Trader] PRE-FLIGHT FAILED: eth_call reverted — TRANSFER_FROM_FAILED
```

Dashboard showed: `STG/USD Short -$0.34 (-1.74%)` when the position is actually LONG and profitable.

### Fix

Added a FINAL SIDE CORRECTION block right before the shared state sync at line ~1045. This catches ALL remaining SHORT positions regardless of how they entered the portfolio:
- Filters for `Side::Short` positions
- Forces them to `Side::Long` with corrected TP/SL (entry * 0.92/1.10/1.20/1.30)
- Re-registers corrected side in DexTrader
- Saves correction to journal
- Logs activity to shared state

**Code location:** `src/engine/mod.rs` lines ~1045-1085 (FINAL SIDE CORRECTION block).

## Impact Assessment

### Affected Components

- `src/engine/mod.rs` — `EngineState::new()` before shared state sync
- DexTrader position registration
- Trade journal persistence

### Risk Level

- [x] Critical: Incorrect position side causes wrong TP/SL triggers and failed swaps
- [ ] High
- [ ] Medium
- [ ] Low

## Proposed Solution

### Approach

Defense-in-depth: add a final side-correction gate that runs after ALL position-loading paths, immediately before the shared state sync.

### Steps

1. After executor→portfolio sync, before shared state sync
2. Iterate all portfolio positions, filter for SHORT
3. Force to LONG with corrected TP/SL
4. Re-register in DexTrader
5. Save to journal
6. Log activity

### Verification

1. `cargo clippy -- -D warnings` — 0 warnings
2. `cargo test` — all tests pass
3. Law 4 grep: `grep -n 'FINAL SIDE CORRECTION' src/engine/mod.rs` — confirms production wiring
4. Runtime: Log shows `FINAL SIDE CORRECTION: STG/USD LONG — spot-only mode, forcing SHORT → LONG (pre-sync)`
5. Dashboard shows STG/USD as LONG with positive PnL

## Perfection Loop

### Loop 1

- **RED:** SHORT positions surviving into portfolio via executor→portfolio sync path
- **GREEN:** Added FINAL SIDE CORRECTION block before shared state sync
- **AUDIT:** clippy clean, tests pass, grep confirms 4 SIDE CORRECTION sites (2 existing + 2 new)
- **CHANGE DELTA:** ~40 lines added (FINAL SIDE CORRECTION block)

## Resolution

- **Fixed By:** Buffy (Codebuff AI)
- **Fixed Date:** 2026-06-10 20:30
- **Fix Description:** Added FINAL SIDE CORRECTION block that forces all remaining SHORT positions to LONG before shared state sync, covering the executor→portfolio re-add path
- **Tests Added:** No new unit tests (startup-only code, verified via runtime + clippy)
- **Verified By:** clippy + tests + Law 4 grep (4 SIDE CORRECTION sites confirmed)
- **Commit/PR:** Pending
- **Archived:** Pending (status: verified → closed after commit)

## Lessons Learned

- Defense-in-depth is critical for invariant enforcement. The wallet-sync side-correction was correct but incomplete — it only caught positions present at that point in the startup sequence. A final gate before shared state sync catches ALL paths.
- The executor→portfolio sync is a "safety net" that re-adds positions from DexTrader's tracker. It's useful but must not reintroduce bugs that earlier safety layers fixed.
- On a spot-only DEX, SHORT is never valid. The invariant should be enforced at every layer: prompt, parser, portfolio, shared state.
