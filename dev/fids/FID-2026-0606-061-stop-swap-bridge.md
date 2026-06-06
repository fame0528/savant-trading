# FID: Stop-Loss Fires But Never Executes On-Chain Swap

**Filename:** `FID-2026-0606-061-stop-swap-bridge.md`
**ID:** FID-2026-0606-061
**Severity:** critical
**Status:** created
**Created:** 2026-06-06 13:52
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

Overnight test run (4:08 AM – 12:46 PM, 103 cycles) confirmed that the engine's trailing stop system works correctly at the PortfolioManager level — stops trail up, detect price breaches, and record closed trades with accurate P&L. However, **no on-chain swap is ever executed**. Tokens remain in the wallet. The engine logs "Stop loss hit" and moves on, but the DexTrader (which owns the 0x swap execution) never receives a close command.

This is a blocking bug for autonomous trading. Any position managed by the engine will accumulate paper profits/losses but never convert back to USDC on-chain.

---

## Detailed Description

### Problem

The engine has two parallel position tracking systems:

1. **PortfolioManager** (`src/execution/portfolio.rs`) — in-memory position tracking, stop-loss management, trailing logic, P&L calculation. This is the "paper" layer.
2. **DexTrader** (`src/execution/dex/trader.rs`) — on-chain execution via 0x API. Holds its own `positions: HashMap<String, Position>`. This is the "real" layer.

When a stop-loss fires, `portfolio.check_stops()` removes the position from PortfolioManager and returns a `StopCheckResult` with the closed trade. The engine then attempts to route the close to DexTrader via two paths:

**Path 1 (primary):** Look up `executor_position_map` for the DexTrader position ID.
**Path 2 (fallback):** Search `ex.open_positions()` by pair + side.

Both paths fail because:

- `executor_position_map` is **never populated** for wallet-recovered positions. Grep for `executor_position_map.insert` across the entire codebase returns zero results.
- DexTrader's `self.positions` map is **empty** for wallet-recovered positions. Wallet sync (engine.rs lines 885-913) creates positions only in PortfolioManager, never in DexTrader.

### Expected Behavior

When a stop-loss fires:
1. PortfolioManager detects price <= stop_loss
2. Engine routes close to DexTrader
3. DexTrader executes a 0x swap (WETH→USDC or LINK→USDC) on-chain
4. Only after confirmed tx receipt does the position get removed
5. Balance updates reflect the actual on-chain USDC received

### Root Cause

**Three disconnected systems with no bridge:**

```
Wallet Sync (engine.rs:885)
    → Creates position in PortfolioManager only
    → Does NOT register in DexTrader.positions
    → Does NOT populate executor_position_map

Stop-Loss (engine.rs:2267)
    → Reads stop_result.closed from PortfolioManager
    → Tries executor_position_map → EMPTY
    → Tries ex.open_positions() fallback → EMPTY (DexTrader has no positions)
    → Silent failure — no swap, no error log, no retry

DexTrader.close_position() (trader.rs:1140)
    → Fully implemented: 0x /price → /quote → sign → submit
    → Never called for wallet-recovered positions
```

### Evidence

**Terminal log (Cycle 9, 4:49 AM):**
```
[SL] ETH/USD LONG | Entry: 1549.6600 → Exit: 1573.5138 | Qty: 0.0149 | PnL: $0.26 (1.13%) | Stop loss hit (Full)
```

**On-chain (Arbiscan, same timestamp):** Zero transactions. WETH still in wallet.

**Terminal log (Cycle 12, 5:05 AM):**
```
[SL] LINK/USD LONG | Entry: 7.1900 → Exit: 7.3141 | Qty: 0.3222 | PnL: $0.03 (1.32%) | Stop loss hit (Full)
```

**On-chain (Arbiscan, same timestamp):** Zero transactions. LINK still in wallet.

**DexTrader state (dex_state.json):**
```json
{
  "positions": [],
  "closed_trades": [],
  "balance": 0.260677,
  "order_counter": 0
}
```

The `order_counter: 0` confirms zero swaps were ever attempted.

### 0x Swap Flow (from docs/0x-llms-full.md)

The 0x API is a swap aggregation API — no native stop-loss or limit orders. All order management is client-side. The close flow is:

1. `GET /swap/allowance-holder/price` — check liquidity, get indicative price
2. Set token allowance on AllowanceHolder contract (if not already set)
3. `GET /swap/allowance-holder/quote` — get firm quote with transaction data
4. Submit signed transaction to Arbitrum RPC
5. Wait for receipt, confirm swap

DexTrader already implements this full flow in `close_position()` (trader.rs:1140-1305). It handles:
- Pair resolution (WETH→USDC, LINK→USDC, etc.)
- Wei conversion per token decimals
- Liquidity pre-check via /price
- Permit2 signing
- Gas estimation
- Transaction broadcast
- Receipt confirmation

The code exists. It just never gets called.

---

## Impact Assessment

### Affected Components

- `src/engine.rs` — stop-loss close routing (lines 2266-2334)
- `src/engine.rs` — wallet sync position creation (lines 885-913)
- `src/execution/dex/trader.rs` — `close_position()` never reached
- `src/execution/dex/trader.rs` — `positions` map empty for recovered positions

### Risk Level

- [x] Critical: System crash, data loss, or security vulnerability

Positions accumulate on-chain exposure with no automated exit. If the bot is left running overnight with real positions, a market crash would result in total loss of the position value — the engine would log "stop loss hit" but the tokens would still be bleeding on-chain.

---

## Proposed Solution

### Approach

Register wallet-recovered positions in DexTrader so the existing close path works. **One bridge, not two.** The existing `close_position()` + `executor_position_map` lookup + `ex.open_positions()` fallback already handles the close correctly — the only problem is that wallet-recovered positions never enter either map.

### Steps

1. **Add `register_position()` to `ExecutionEngine` trait** (`src/execution/engine.rs`):
   ```rust
   /// Register a wallet-recovered position so close_position() can find it.
   /// Called during wallet sync for positions discovered on-chain but not in the executor.
   /// Default: no-op (paper trading and Kraken don't need this).
   fn register_position(&mut self, _id: String, _pos: Position) {}
   ```
   Implement in DexTrader:
   ```rust
   fn register_position(&mut self, id: String, pos: Position) {
       self.positions.insert(id, pos);
   }
   ```
   PortfolioManager and KrakenTrader use the default no-op. No `positions_mut()` exposure.

2. **Register recovered positions in DexTrader** (engine.rs ~line 902, after `portfolio.positions_mut().insert()`):
   ```rust
   if let Some(ref mut ex) = executor {
       let exec_id = format!("exec-{}", recovery_pos.id);
       ex.register_position(exec_id.clone(), recovery_pos.clone());
       executor_position_map.insert(recovery_pos.id.clone(), exec_id);
       info!("WALLET SYNC: Registered {} in DexTrader as {}", pair, exec_id);
   }
   ```

3. **Verify crash recovery**: DexTrader's `save_state()` persists `self.positions` to `dex_state.json`. After registration, the next `save_state()` call writes recovered positions. On restart, `load_state()` restores them. Verify `dex_state.json` contains the registered positions after one cycle.

4. **Add swap tx hash logging**: After `ex.close_position()` succeeds at engine.rs:2302, the returned `Order` contains `filled_price` but not the tx hash. Add tx hash to the `Order` struct or log it inside `close_position()`.

5. **Log silent failures in close path** (engine.rs ~line 2307): When both `executor_position_map` and `ex.open_positions()` fail, log an error. Currently zero output — the engine silently skips the on-chain close:
   ```rust
   } else {
       error!("CLOSE BRIDGE FAILED: {} {} — no executor position found. Tokens remain on-chain.", trade.pair, trade.side);
       shared.log_activity(
           savant_trading::core::shared::ActivityLevel::Error,
           &trade.pair,
           &format!("CLOSE FAILED: No executor position — tokens stay on-chain"),
       ).await;
   }
   ```

6. **Move auto-stop override after wallet sync** (engine.rs): The auto-stop code at line 528 runs before wallet sync creates positions (line 885+). Move it to after line 976 (`Wallet sync complete`). This also fixes the LINK stop not tightening to $7.00 on boot.

7. **Kill stale Next.js in start.bat**: Port 3000 conflict from previous session. Add process cleanup before starting:
   ```bat
   @echo off
   title SAVANT Trading Engine
   echo.
   echo  ========================================
   echo   SAVANT Trading Engine
   echo   Starting engine + dashboard...
   echo  ========================================
   echo.
   cd /d "%~dp0"
   :: Kill stale Next.js process holding port 3000
   for /f "tokens=5" %%a in ('netstat -aon ^| findstr :3000 ^| findstr LISTENING') do taskkill /F /PID %%a >nul 2>&1
   target\release\savant.exe serve
   echo.
   echo  Engine stopped. Press any key to exit.
   pause >nul
   ```

8. **Add unit test for stop→close bridge** (`src/execution/portfolio.rs` tests): Existing tests verify `check_stops()` returns closed trades. None verify the closed trade routes to the executor. Add a test that:
   - Creates a PortfolioManager with a position
   - Creates a mock executor with the same position registered
   - Populates `executor_position_map`
   - Calls `check_stops()` with a price below stop
   - Asserts `close_position()` was called on the executor
   - Asserts the position was removed from both systems

### What NOT to do

- ~~**Direct swap fallback**~~ — Dropped. If Step 1 works (and it should), the existing close path handles everything: liquidity check, Permit2 signing, position removal, closed_trades update, state persistence. A raw `backend.swap()` call would bypass all safety checks and require reimplementing cleanup logic.

### Verification

1. Start engine with existing WETH/LINK on-chain
2. Confirm wallet sync registers positions in both PortfolioManager AND DexTrader
3. Verify `dex_state.json` shows registered positions after cycle 1
4. Verify auto-stop override fires (LINK stop → $7.00) — check terminal for "Auto-stop queued"
5. Manually set a tight stop-loss via API (`PATCH /api/positions/LINK/USD/stop`)
6. Wait for stop to fire
7. Verify on-chain: WETH/LINK balance decreases, USDC balance increases
8. Verify `dex_state.json` shows `order_counter > 0` and positions removed
9. Verify tx hash appears in logs
10. Kill engine, restart — verify crash recovery loads positions from `dex_state.json`
11. Verify `start.bat` kills stale Next.js and starts clean on port 3000
12. Run `cargo test` — verify new stop→close bridge test passes

---

## Perfection Loop

### Loop 1

- **RED:** 6 issues found:
  1. `positions_mut()` doesn't exist on `ExecutionEngine` trait — needs adding
  2. Proposed Step 1 code won't compile (`Box<dyn ExecutionEngine>` can't call non-existent method)
  3. Direct swap fallback (Step 2) bypasses DexTrader safety checks (liquidity, Permit2, cleanup)
  4. No mention of crash recovery verification path
  5. Step 2 is unnecessary if Step 1 works — two solutions for one problem
  6. Missing: tx hash logging after successful close
- **GREEN:** Simplified to single-bridge approach:
  - Dropped Step 2 (direct swap fallback) entirely
  - Added `register_position()` to trait with safe no-op default (avoided unsound `static mut`)
  - Added crash recovery verification step
  - Added tx hash logging step
  - Solution is now 4 steps instead of 4 steps + fallback
- **AUDIT:** Two independent methods:
  1. Code path trace: wallet sync → registration → stop → close → cleanup — all 7 steps verified
  2. Trait implementor check: 3 implementors (DexTrader, PortfolioManager, KrakenTrader) — only DexTrader overrides `register_position()`, others use safe no-op
  3. Found `static mut` UB in initial proposal → replaced with `register_position()` method
- **CHANGE DELTA:** ~50% of Proposed Solution section rewritten

### Loop 2

- **RED:** 4 additional issues found during review:
  1. Silent failure at close fallback path (engine.rs:2307) — no error log when close can't route
  2. Auto-stop override runs before wallet sync (engine.rs:528) — never fires
  3. `start.bat` port 3000 conflict from stale Next.js process
  4. No unit test for stop→close bridge path
- **GREEN:** Added steps 5-8:
  5. Error-level log when close bridge fails
  6. Moved auto-stop override after wallet sync
  7. Added port cleanup to start.bat
  8. Added unit test for stop→close bridge
- **AUDIT:** All 4 items are independent, low-risk changes. No new failure modes introduced.
- **CHANGE DELTA:** ~20% additional (steps 5-8 appended)

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —

---

## Lessons Learned

1. **Two-layer architecture requires explicit bridging.** PortfolioManager and DexTrader are separate systems. Every mutation in one that affects the other must have an explicit bridge call. The "close via stop" path was never bridged.

2. **Wallet sync is a special case.** Positions discovered on-chain bypass the normal open flow (which populates both systems). Any position created outside the normal flow needs explicit registration in both layers.

3. **Silent failure is worse than a crash.** The close path silently fails — no error log, no warning, no retry. The engine happily continues with "closed" positions that are still open on-chain. This should have been an error-level log with a retry mechanism.

4. **`executor_position_map` is a dead path.** Zero inserts across the entire codebase. This mapping was designed but never wired. Any code that depends on it is dead code for wallet-recovered positions.

5. **Nova audit F-14 confirms root cause.** The PaperTrader/DexTrader desync was flagged as a separate finding but is the same underlying issue. FID-061 resolves it.

6. **F-07 dependency: retry queue is broken.** If `close_position()` fails (no liquidity, RPC error), the retry queue silently drops it (`kept` is always empty). A failed close has no recovery path. This is a separate bug but directly impacts FID-061 — if the close swap fails, the position is stuck with no retry.
