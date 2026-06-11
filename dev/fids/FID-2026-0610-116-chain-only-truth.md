# FID-116: Chain-Only Truth — Eliminate All Hardcoded & Stale Data

**Status:** 🆕 Proposed
**Priority:** 🔴 CRITICAL — Active fund bleeding ($25+ lost to stale data)
**Created:** 2026-06-10
**Supersedes:** FID-115 (snapshot approach was wrong — captures stale portfolio data, not on-chain truth)

---

## Problem Statement

The system has **5 competing sources of truth** for the same financial data, producing contradictory numbers that cause incorrect position sizing, wrong profit display, phantom trades, and real money loss:

| Source | What it claims | Actual on-chain |
|--------|---------------|-----------------|
| `config/default.toml` | starting_balance = $30.00 | $19.35 at first boot |
| `data/starting_balance.json` (FID-115) | starting_equity = $19.35 | $23.79 (56.65 STG × $0.42) |
| `portfolio.account()` | equity = $19.35 | Missing 10.711 STG ($4.50) |
| `dex_state.json` | STG balance = 56.65 | 10.711 after SL swap |
| On-chain RPC | 10.711 STG | **THE ONLY TRUTH** |

**The user has lost $25+ because the engine trades, displays PnL, and sizes positions based on stale/hardcoded data instead of querying the chain.**

---

## Root Cause Analysis (8 Bugs)

### BUG 1: `AccountState::refresh_from_positions()` does NOT query the chain

**File:** `src/core/types.rs:327-339`
**Formula:** `equity = balance + Σ(position.current_price × position.quantity)`

- `balance` = executor's internal `self.balance` (may be stale USDC)
- `position.quantity` = tracked quantity (may differ from on-chain)
- `position.current_price` = candle price (may be stale)
- **MISSING:** Any on-chain tokens NOT in a tracked position (10.711 STG ghost capital)

### BUG 2: `starting_balance` hardcoded from config

**Files:** `config/default.toml:99`, `src/engine/mod.rs:236`, `src/main.rs:406,438,547,875`, `src/tui/state.rs:124`, `src/execution/portfolio.rs:52`, `src/risk/position.rs:118-174`

- `config.trading.starting_balance = 30.0` is used for:
  - Portfolio initialization → sets initial equity
  - Position sizing → determines trade quantity
  - Context builder → tells LLM available capital
  - TUI → displays budget
  - Profit calculation → `equity - starting_balance`
- Actual on-chain value at first boot was $19.35, not $30.00

### BUG 3: `sync_balance()` is periodic, not reactive

**File:** `src/engine/mod.rs:4271`
- Runs every 3rd tick = every 15 minutes at 5-min cycles
- After the STG SL swap at 9:56 PM, the engine showed stale balance until next sync
- Dashboard displayed wrong equity for up to 15 minutes post-swap

### BUG 4: Wallet sync ignores the "gap" between tracked and on-chain

**File:** `src/engine/mod.rs:792-912`
- Branch 1: `tracked_qty < 0.001 && on_chain_qty > 0.001` → creates new position ✓
- Branch 2: `on_chain_qty < 0.001 && tracked_qty > 0.001` → removes ghost ✓
- **MISSING:** `on_chain_qty > tracked_qty > 0.001` → silently ignores the gap
- Example: tracked 45.94 STG, on-chain 56.65 STG → 10.71 STG invisible

### BUG 5: Journal loaded as source of truth for OPEN positions

**File:** `src/engine/mod.rs:395-405`
- Journal SQLite only stores CLOSED trades, but `load_positions()` restores open positions
- Three separate code paths add positions: (1) journal load, (2) wallet recovery, (3) executor→portfolio sync
- Path 3 can reintroduce positions that path 2 removed (FID-112 root cause)
- Journal positions can have wrong side (SHORT on spot-only DEX), wrong entry price, stale quantity

### BUG 6: FID-115 snapshot captures stale PORTFOLIO equity, not on-chain

**File:** `src/engine/mod.rs:1101-1115`
- Snapshot saves `portfolio.account().equity` = $19.35
- Actual on-chain: 56.65 STG × $0.42 + $0 USDC = $23.79
- The 10.71 STG "gap" means the snapshot is wrong by $4.50
- This is the number the dashboard uses for "invested" — it's wrong

### BUG 7: `set_balance()` resets `peak_equity`

**File:** `src/execution/portfolio.rs:479-481`
```rust
pub fn set_balance(&mut self, balance: f64) {
    self.account.balance = balance;
    self.account.equity = balance;
    self.account.peak_equity = balance;  // ← BUG: resets drawdown tracking
}
```
- Called at startup with on-chain USDC balance only
- If USDC = $0, sets peak_equity = 0
- Then `refresh_from_positions()` adds position values back
- But peak_equity was already corrupted → drawdown % is wrong

### BUG 8: Dashboard profit formula compounds all errors

**File:** `src/api/mod.rs:427-428`
```rust
let starting = load_starting_balance_snapshot(state.config.trading.starting_balance);
let true_pnl = account.equity - starting;
```
- `account.equity` = BUG 1 (missing loose tokens)
- `starting` = BUG 6 (stale snapshot) or BUG 2 (hardcoded config)
- Result: profit display is wrong by $4.50+ in either direction

---

## Solution Architecture

### Principle: Chain Is The Only Truth

Every number displayed to the user or used in calculations MUST derive from on-chain RPC queries. No config values, no snapshots, no journal data for financial state.

### Layer 1: On-Chain Equity Calculator

**New function:** `calculate_on_chain_equity()` in `src/execution/dex/trader.rs`

```
on_chain_equity = on_chain_usdc_balance + Σ(token_balance_i × current_price_i)
```

- Queries USDC `balanceOf(wallet)` via RPC
- Queries every curated pair token `balanceOf(wallet)` via RPC
- Multiplies each by current market price from candle data
- Returns the TRUE total portfolio value on-chain
- Called at startup, after every swap, and on every equity refresh

### Layer 2: Replace `starting_balance` with on-chain snapshot

- On **first-ever startup**: save `on_chain_equity` to `data/starting_equity.json`
- On **subsequent startups**: read from `data/starting_equity.json`
- **Delete** `config.trading.starting_balance` from all production paths
- Keep config value ONLY for backtest/training/sandbox (they need a synthetic starting point)

### Layer 3: Reactive balance sync after every swap

- After `close_position()` swap executes → immediately call `sync_balance()` → update portfolio
- After `place_order()` swap executes → immediately call `sync_balance()` → update portfolio
- Keep periodic sync as backup (every 3rd tick) but primary is reactive

### Layer 4: Fix wallet sync "gap" branch

Add missing branch to `sync_wallet_positions()`:
```
if on_chain_qty > tracked_qty + 0.001 && tracked_qty > 0.001:
    // On-chain has MORE tokens than tracked → update position quantity
    position.quantity = on_chain_qty
```

### Layer 5: Chain-first position management

- On startup: positions are determined BY the chain (what tokens exist in wallet)
- Journal is READ-ONLY for history (closed trades, PnL records)
- Journal is NEVER used to restore open positions
- `dex_state.json` is used ONLY for stop-loss/take-profit parameters, not quantities
- Position quantities are ALWAYS from on-chain `balanceOf()` queries

### Layer 6: Fix `set_balance()` to preserve `peak_equity`

```rust
pub fn set_balance(&mut self, balance: f64) {
    self.account.balance = balance;
    // Don't reset equity/peak — let refresh_equity() handle it
}
```

---

## Implementation Plan

### Phase 1: On-Chain Equity Calculator (FID-116A)

**Files:** `src/execution/dex/trader.rs`, `src/execution/engine.rs`

1. Add `calculate_on_chain_equity(&self) -> f64` to `DexTrader`
   - Query USDC `balanceOf(wallet)`
   - Query each curated pair token `balanceOf(wallet)` (already have these addresses)
   - Get current prices from candle data (pass as parameter)
   - Return sum: `usdc_value + Σ(token_balance × price)`

2. Add `on_chain_equity(&self) -> f64` to `ExecutionEngine` trait
   - Default: returns `self.balance()` (paper trading)
   - DexTrader: returns `calculate_on_chain_equity()`

3. Expose via shared state for dashboard
   - Add `chain_equity: Arc<RwLock<f64>>` to `SharedState`
   - Update every cycle after candle refresh

### Phase 2: Chain-Verified Starting Equity (FID-116B)

**Files:** `src/engine/mod.rs`, `src/api/mod.rs`, `config/default.toml`

1. Delete `data/starting_balance.json` and FID-115 snapshot code
2. On first startup (file doesn't exist):
   - Call `calculate_on_chain_equity()` after candles load
   - Save to `data/starting_equity.json` with `equity` and `captured_at`
3. On subsequent startups:
   - Read from `data/starting_equity.json`
   - Do NOT overwrite (immutable once captured)
4. Remove ALL references to `config.trading.starting_balance` in production paths
5. Add to `.gitignore`: `data/starting_equity.json`

### Phase 3: Reactive Post-Swap Sync (FID-116C)

**Files:** `src/engine/mod.rs`, `src/execution/dex/trader.rs`

1. After every successful close swap:
   - Call `ex.sync_balance().await`
   - Call `portfolio.account_mut().balance = ex.balance()`
   - Call `portfolio.refresh_equity()`
   - Update shared state immediately
2. After every successful open swap:
   - Same sync chain
3. Keep periodic sync as safety net

### Phase 4: Fix Wallet Sync Gap (FID-116D)

**File:** `src/engine/mod.rs` (wallet sync block)

1. Add third branch: `on_chain_qty > tracked_qty + threshold`
   - Update existing position quantity to on-chain value
   - Recalculate risk_amount
   - Log the adjustment
2. This handles the 10.71 STG "ghost capital" problem

### Phase 5: Chain-First Positions (FID-116E)

**Files:** `src/engine/mod.rs`, `src/execution/portfolio.rs`

1. Remove journal-based position restoration for OPEN positions
2. Position lifecycle:
   - Chain determines WHAT we hold (token balances)
   - `dex_state.json` provides stop-loss/take-profit parameters
   - Journal records CLOSED trades only
3. On startup:
   - Query on-chain balances for all curated pairs
   - Create positions from on-chain data (entry price from recent candles or journal trade history)
   - Register in both portfolio and executor
4. Delete `load_positions()` from journal for open position restoration

### Phase 6: Fix set_balance() and Equity Pipeline (FID-116F)

**File:** `src/execution/portfolio.rs`

1. Fix `set_balance()` to NOT reset `peak_equity`
2. Ensure `refresh_equity()` is always called after `set_balance()`
3. Verify drawdown tracking survives startup

### Phase 7: Dashboard & API Alignment (FID-116G)

**Files:** `src/api/mod.rs`, `dashboard/src/app/page.tsx`

1. `get_session` API:
   - `equity` = on-chain equity from shared state
   - `starting` = from `data/starting_equity.json`
   - `total_pnl` = `equity - starting`
2. `get_config` API:
   - Remove `starting_balance` from response
   - Add `chain_equity` field
3. Dashboard:
   - Show "Chain Equity" instead of "invested" for starting balance
   - Show real-time on-chain value

---

## Files Modified

| File | Changes |
|------|---------|
| `src/execution/dex/trader.rs` | Add `calculate_on_chain_equity()`, expose via trait |
| `src/execution/engine.rs` | Add `on_chain_equity()` to trait |
| `src/engine/mod.rs` | Reactive sync, wallet gap fix, chain-first positions, remove journal position restore |
| `src/api/mod.rs` | Use chain equity, remove snapshot, fix profit calc |
| `src/execution/portfolio.rs` | Fix `set_balance()`, remove starting_balance dependency |
| `src/core/types.rs` | No changes needed (refresh_from_positions is fine as internal calc) |
| `src/core/shared.rs` | Add `chain_equity` field |
| `src/risk/position.rs` | Use chain equity for position sizing |
| `src/agent/context_builder.rs` | Use chain equity for LLM context |
| `src/tui/state.rs` | Use chain equity for display |
| `config/default.toml` | Keep `starting_balance` but mark as "backtest only" |
| `dashboard/src/app/page.tsx` | Update profit display |
| `dashboard/src/lib/api.ts` | Add `chain_equity` type |

---

## Testing Strategy

1. **Unit tests:** `calculate_on_chain_equity()` with mock RPC responses
2. **Integration test:** Startup flow with mock chain data, verify equity matches
3. **Manual test:** Run engine, verify dashboard shows correct on-chain equity
4. **Regression test:** All 267 existing tests pass
5. **Clippy:** Zero warnings

---

## Success Criteria

- [ ] Dashboard "Profit" shows $0.00 on first boot (chain equity = starting equity)
- [ ] After a swap, dashboard updates within 1 cycle (not 15 minutes)
- [ ] Every token in wallet is reflected in equity calculation
- [ ] `config.trading.starting_balance` is NEVER used for production financial state
- [ ] `data/starting_balance.json` is deleted and replaced with `data/starting_equity.json`
- [ ] No position is ever restored from journal (chain-only)
- [ ] Zero stale data paths remain in production code
- [ ] All 267 tests pass, clippy clean

---

## Risk Assessment

- **High risk:** Changing equity calculation affects position sizing → could open oversized trades
- **Mitigation:** Add safety bounds: if chain equity < config.starting_balance × 0.5, log CRITICAL warning and use the lower value
- **Medium risk:** Removing journal position restore could lose stop-loss/take-profit data
- **Mitigation:** `dex_state.json` preserves SL/TP parameters; journal is only removed for open position restoration
- **Low risk:** RPC failures during equity calculation
- **Mitigation:** Fall back to portfolio-calculated equity with warning log

---

## Dependencies

- FID-111 (position-pair injection) — already merged
- FID-112 (FINAL SIDE CORRECTION) — already merged, will be simplified by chain-first approach
- FID-114 (AI Decisions pinning) — already merged
- FID-115 (starting balance snapshot) — **SUPERSEDED** by this FID

---

## Estimated Effort

- Phase 1 (on-chain equity calculator): ~2 hours
- Phase 2 (chain-verified starting equity): ~1 hour
- Phase 3 (reactive post-swap sync): ~1 hour
- Phase 4 (wallet sync gap fix): ~30 minutes
- Phase 5 (chain-first positions): ~2 hours
- Phase 6 (fix set_balance): ~30 minutes
- Phase 7 (dashboard alignment): ~1 hour
- **Total: ~8 hours of focused implementation**

---

## Open Questions

1. Should we query ALL token balances on every cycle, or only curated pair tokens?
   - **Recommendation:** Only curated pairs + USDC. Querying all 50+ tokens every 5 min is expensive.
2. What entry price to use for chain-discovered positions?
   - **Recommendation:** Use journal trade history if available, else current market price with a note.
3. Should `starting_equity` be re-capturable (e.g., after a major deposit)?
   - **Recommendation:** Add CLI flag `--reset-starting-equity` that deletes the file and re-captures.
