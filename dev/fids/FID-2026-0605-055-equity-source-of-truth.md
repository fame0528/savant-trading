# FID: Closed Trades Empty + P&L Mismatch — Single Source of Truth for Account Metrics

**Filename:** `FID-2026-0605-055-equity-source-of-truth.md`
**ID:** FID-2026-0605-055
**Severity:** critical
**Status:** in_progress
**Created:** 2026-06-05 21:11
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

Dashboard shows 0 closed trades despite DB having 7. Open position P&L differs between CLI terminal and dashboard. Root cause: equity/P&L computed inline in 4+ places with inconsistent logic, and journal trades not loaded into PaperTrader on startup (only into shared state, which gets overwritten every tick by paper's empty list).

---

## Detailed Description

### Problem 1: Closed trades empty on dashboard

- At startup (engine.rs:444-449): journal loads 7 trades into `shared.closed_trades`
- PaperTrader starts with empty `closed_trades` (paper.rs:55)
- Every 10 ticks (engine.rs:2352-2353): `*shared_trades = paper.closed_trades().to_vec()` — **overwrites journal trades with empty list**

### Problem 2: P&L mismatch between CLI and dashboard

- CLI reads from `paper.account()` / `paper.positions()` directly
- API reads from `shared.account` / `shared.positions`
- Equity was computed inline in 4+ places with slightly different logic
- `update_equity(unrealized)` used `equity = balance + unrealized` — **wrong formula** (unrealized P&L ≠ position market value)

### Problem 3: No single source of truth

- `position_values = sum(current_price * quantity)` computed manually in 4 engine.rs sites
- `unrealized_pnl = sum(p.unrealized_pnl)` computed manually in 4 sites
- `equity = balance + position_values` computed manually in 4 sites
- Each site could drift independently

### Expected Behavior

- Journal trades loaded into PaperTrader AND shared state on startup
- Single `refresh_equity()` method on PaperTrader computes all account metrics
- Both CLI and API read from the same source (paper → shared sync every tick)

---

## Impact Assessment

### Affected Components

- `src/core/types.rs` — `AccountState::refresh_from_positions()` replaces `update_equity()`
- `src/execution/paper.rs` — `refresh_equity()`, `set_closed_trades()` methods added
- `src/engine.rs` — 4 inline equity computation sites replaced, journal trade loading fixed

### Risk Level

- [x] Critical: Dashboard non-functional for closed trades. P&L wrong on dashboard.

---

## Proposed Solution

### 1. Add `set_closed_trades()` to PaperTrader

Allow loading journal trades into PaperTrader so they survive the shared state sync.

### 2. Replace `update_equity(unrealized)` with `refresh_from_positions(positions)`

Old: `equity = balance + unrealized` (wrong — unrealized ≠ position market value)
New: `equity = balance + sum(current_price * quantity)` (correct)

### 3. Add `refresh_equity()` to PaperTrader

Convenience method that calls `self.account.refresh_from_positions(&self.positions)` — avoids borrow checker conflict between `account_mut()` and `positions()`.

### 4. Load journal trades into PaperTrader on startup

At engine.rs:444, after loading from journal, also call `paper.set_closed_trades(closed.clone())`.

### 5. Replace all inline equity computation with `paper.refresh_equity()`

4 sites in engine.rs where position_values/equity/unrealized_pnl were computed manually.

---

## Perfection Loop

### Loop 1

- **RED:** 4 findings. (1) BLOCKER: `check_stops()` and `close_position()` modify account without calling `refresh_from_positions()` — stale equity propagates to dashboard. (2) WARNING: Indentation corruption on 2 lines in engine.rs. (3) WARNING: No unit test for `refresh_from_positions()` edge cases. (4) INFO: `set_balance()` bypasses refresh (currently safe, init-only).
- **GREEN:** (1) Added `self.account.refresh_from_positions(&self.positions)` at end of `check_stops()` (paper.rs:410) and inside `close_position()` (paper.rs:623). Removed redundant `open_positions = len()` since `refresh_from_positions` now sets it. (2) Fixed 2 indentation-corrupted `paper.refresh_equity()` lines in engine.rs (1841, 2243). (3) Added 5 unit tests to types.rs: `refresh_empty_positions_equity_equals_balance`, `refresh_with_long_position_in_profit`, `refresh_peak_equity_tracks_highs`, `refresh_drawdown_calculation`, `refresh_short_position_pnl`.
- **AUDIT:** Double-audited with 2 independent methods:
  - Method 1 (Static): `cargo check` clean, `cargo clippy -- -D warnings` clean
  - Method 2 (Runtime): `cargo test` 215/215 pass (209 lib + 4 bin + 2 doc)
  - Grep verification: `refresh_from_positions` 4 production callers, `refresh_equity` 3 callers, `set_closed_trades` 1 caller, `update_equity` 0 remaining callers, inline equity computations 0 remaining
- **CONVERGED:** Delta = 0% (no further issues found)

---

## Resolution

- **Fixed By:** Kilo (mimo-v2.5-pro)
- **Fixed Date:** 2026-06-05 21:11
- **Fix Description:** Created `AccountState::refresh_from_positions()` as single source of truth for equity/P&L/drawdown. Added `PaperTrader::refresh_equity()` and `set_closed_trades()`. Replaced 4 inline equity computation sites in engine.rs. Fixed check_stops/close_position to refresh after mutation. Added 5 unit tests covering edge cases.
- **Tests Added:** Yes — 5 tests in `core::types::tests`: empty positions, long profit, peak tracking, drawdown calc, short PnL
- **Verified By:** `cargo check` clean + `cargo test` 215/215 pass + `cargo clippy -- -D warnings` clean + grep call-graph verification
- **Commit/PR:** —
- **Archived:** —
