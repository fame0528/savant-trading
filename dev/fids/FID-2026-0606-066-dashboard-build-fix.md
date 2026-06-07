# FID-066: Consolidated Pending Work — Dashboard, Duplicates, Session Housekeeping

**Status:** closed
**Severity:** high
**Created:** 2026-06-06
**Closed:** 2026-06-06
**Author:** Kilo

---

## Scope: ALL Pending Work

This FID consolidates every unresolved item. Nothing left un-addressed.

### ACTIVE BUGS (Fix Now)

#### 1. Dashboard Build Broken (BLOCKING)
- **File:** `dashboard/src/app/page.tsx:238`
- **Error:** `insight?.block_number` — field doesn't exist on `MarketInsight`
- **Actual field:** `insight?.block_height` (`api.ts:83`)
- **Fix:** Single field rename
- **Impact:** Zero logic change
- **Risk:** None

#### 2. Duplicate Trade Closures (FID-065 — Data Integrity)
- **Symptom:** 9W/7L=16 but only ~6 unique trades. ETH/USD Short at $1549→$1533 appears 4x.
- **Root cause:** Position removed from local map on first stop hit, but re-registered on next tick (wallet recovery re-discovers on-chain balance), causing stop to fire again.
- **Fix:** Deduplicate closed trades — same pair+entry+exit+side within 60s = skip recording.
- **Where:** `src/engine.rs` stop-loss close path and/or `src/execution/portfolio.rs` trade recording
- **Risk:** Low — deduplication is additive, doesn't change existing behavior

#### 3. Stop-Loss Never Executes On-Chain Swap (FID-061 — CRITICAL)
- **Symptom:** Engine logs "Stop loss hit" but tokens remain in wallet. No on-chain swap.
- **Root cause:** Wallet-recovered positions are registered in PortfolioManager but NOT in DexTrader. The close bridge (`executor_position_map`) is never populated for recovered positions.
- **Fix:** Register wallet-recovered positions in DexTrader via `register_position()` trait method. The trait method already exists (added in earlier session). Wire it in engine.rs wallet sync.
- **Risk:** Medium — changes position lifecycle. Must verify crash recovery.

### DASHBOARD REBUILD (After Bug Fixes)

#### 4. Rebuild Next.js Dashboard
- After fixing #1, run `npm run build` to compile all pending frontend changes:
  - Copy buttons on all sections (Performance, Market Insight, Open Positions, Risk Controls, AI Decisions, Closed Trades)
  - "HUNT MODE" orange badge in header
  - `hunt_mode` field in Portfolio API type
- **Risk:** None — all code already written, just needs compilation

### SESSION HOUSEKEEPING

#### 5. Update CHANGELOG.md
- FID-062: Kraken execution removal + data pipeline rename
- FID-063: Hunt mode + position re-evaluation
- FID-064: Dashboard copy buttons + header tag
- FID-065: Duplicate trade closures
- FID-066: This consolidated FID

#### 6. Update LEARNINGS.md
- Lesson: Law 1 violations cause cascading failures (block_number typo)
- Lesson: Law 2 violations waste work (changes without FID must be reverted/redone)
- Lesson: Two-layer architecture (PortfolioManager + DexTrader) requires explicit bridging
- Lesson: Next.js production builds are stale until `npm run build` runs

#### 7. Archive Closed FIDs
- Move FID-065 to archive (subsumed by FID-066)

### DEFERRED (Not In This Pass)

| FID | Title | Status | Reason Deferred |
|-----|-------|--------|----------------|
| FID-057 | Liquidation Cascade Strategy | created | Depends on GMX execution layer |
| FID-058 | GMX V2 Sidecar POC | in_progress | Separate effort, not blocking |
| FID-060 | GMX V2 Native Rust | created | Depends on FID-058 |

---

## Perfection Loop

### RED Phase

**Issues cataloged:** 7 items (3 bugs, 1 rebuild, 3 housekeeping)

**Dependencies:**
- #1 (TypeScript fix) → #4 (dashboard rebuild)
- #2 (duplicate trades) → independent
- #3 (stop-loss bridge) → independent
- #5, #6, #7 (housekeeping) → after all fixes committed

**Order of operations:**
1. Fix TypeScript error (#1)
2. Fix duplicate trades (#2)
3. Wire stop-loss bridge (#3)
4. Rebuild dashboard (#4)
5. Housekeeping (#5, #6, #7)

### GREEN Phase

(Pending approval — see presentation below)

### AUDIT Phase

(Pending GREEN implementation)

### SELF-CORRECT Phase

(Pending AUDIT)

### COMPLETE Phase

(Pending all above)

---

## Verification Plan

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all pass
3. `cargo fmt` — clean
4. `npm run build` — compiles successfully
5. Dashboard loads with no console errors
6. Copy buttons visible on all sections
7. "HUNT MODE" tag visible in header
8. Closed trades list shows no duplicates
9. Win/loss count matches unique trades
10. Stop-loss fires on-chain swap when price hits stop
11. `dex_state.json` shows registered positions after wallet sync
12. CHANGELOG.md updated with all FIDs
13. LEARNINGS.md updated with session lessons

---

## Risk Assessment

| Item | Risk | Mitigation |
|------|------|-----------|
| TypeScript fix | None | Single field rename |
| Duplicate trade dedup | Low | Additive — doesn't change existing logic |
| Stop-loss bridge | Medium | Wire `register_position()` in wallet sync. Verify crash recovery. |
| Dashboard rebuild | None | All code already written |

## Presentation

This FID covers 7 items across 3 categories. The execution order is sequential with dependencies. Estimated scope: ~100 lines of Rust changes (dedup + bridge wiring), 1 line TypeScript, dashboard rebuild, and markdown updates.

Awaiting approval to proceed with GREEN phase.
