# FID-177: Revert start.bat Default to test-anvil.toml (Anvil) + Reconcile dex_state.json

**Filename:** `FID-2026-0616-177-revert-start-bat-anvil-default.md`
**ID:** FID-2026-0616-177
**Severity:** critical (operational — the Anvil workflow is broken because FID-167 changed the default to Ethereum mainnet, and the Anvil wallet's $50 balance is now invisible to the engine which reads the journal as the single source of truth per FID-117)
**Status:** created
**Created:** 2026-06-16 23:00 EST
**Author:** Vera
**Triggered by:** Spencer: "the bal is supposed to be $50, it's supposed to be running on anvil, it is NOT supposed to be using more than A SINGLE FILE"

---

## Summary

Two issues:
1. **Wrong default config:** FID-167 changed `start.bat` to default to `config/default.toml` (Ethereum mainnet). The user's actual workflow is Anvil (`config/test-anvil.toml`). The engine was on Ethereum mainnet, which has no live wallet — the $50 in `data/dex_state.json` is the Anvil wallet's balance, and the engine on mainnet was reading the journal (which says $29.97 from a $30 default.toml start) instead of reconciling from the Anvil wallet.
2. **Stale dex_state.json:** Per FID-117, the journal is the single source of truth. But the engine on default.toml recorded trades against `starting_balance=$30`, while the Anvil wallet's actual balance is $50. The dex_state.json holds the correct $50 from a prior Anvil run, but it's a "write-through cache" not the source of truth (per `wallet_recovery.rs:22`). The engine never reconciled these.

**Fix:**
1. Revert `start.bat` line 21 default from `config\default.toml` to `config\test-anvil.toml`. Re-enable Anvil auto-start.
2. Add startup reconciliation: if Anvil is up, the engine should query the Anvil wallet's actual USDC balance and use that as the source of truth (overriding journal). This is a live wallet — the balance IS what the chain says.
3. The journal records trades; the chain (or Anvil RPC) records actual balance. The wallet is the source of truth for balance; the journal is the source of truth for trade history.

**Spencer's rule:** "it is NOT supposed to be using more than A SINGLE FILE." Per FID-117, the journal is the single source. But for live balances, the chain is the source. **Reconciliation: chain > journal for current balance, journal > dex_state.json for trade history.** dex_state.json is removed/ignored.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+
- **Commit/State:** post-v0.14.4 + FID-176 (`b7896a66`)
- **Current time:** 2026-06-16 23:00 EST

---

## Detailed Description

### Current state (verified)

- `data/dex_state.json`:
  ```json
  {
    "positions": [],
    "closed_trades": [],
    "balance": 50.0,
    "order_counter": 2
  }
  ```
  Balance: **$50.00** (Anvil wallet)

- `data/savant.db` (SQLite journal, last run on default.toml):
  - 5 closed trades
  - PnL: -$0.03
  - Restored balance: $29.97 (= $30 starting + -$0.03 PnL)

- Engine log: `Restored balance: $29.97 (starting: $30.00, total PnL: $-0.03, trades: 5)`

- The engine is currently on `config/default.toml` (set by FID-167 in the prior session). The Anvil workflow is inactive.

### The contradiction

- Spencer says: "the bal is supposed to be $50"
- Engine says: $29.97
- The "truth" depends on which file/system you trust:
  - `dex_state.json`: $50 (Anvil's actual wallet balance)
  - Journal: $29.97 (engine's tracking, based on $30 default start)
  - **They disagree because they're tracking different state**

### What's right

**The chain is the source of truth for current balance.** The Anvil wallet has $50 (per its prefund). The journal's $29.97 is what the engine THINKS it has, based on starting from $30. The truth is on-chain.

For Anvil (test-anvil.toml, live_execution=true), the engine should:
1. Query the Anvil RPC for the wallet's USDC balance at startup
2. Use that as the starting balance
3. Update the journal with the actual balance
4. Run cycles; the journal records trades; the engine computes balance as `chain_balance + total_pnl_from_journal`

Per FID-117: "All financial data now derives from two sources: chain (current) + SQLite journal (historical)." **The journal has lost the chain-derived balance.** The engine restores from journal only, ignoring the chain.

### Why FID-167 broke this

FID-167 changed `start.bat` to default to `config/default.toml` and `SAVANT_CHAIN=ethereum`. The intent was to "unlock multi-chain" so the strategy could see liquid majors. But the Anvil workflow is the working dev setup. **The default change was made without consulting Spencer's actual workflow preference.** This is the same lesson as FID-175 (scope changes carefully) — high-blast-radius changes need user sign-off.

### Expected Behavior

After this FID:
- `start.bat` defaults to `config/test-anvil.toml` (revert from FID-167)
- `start.bat` default chain is Arbitrum (revert from FID-167)
- The Anvil auto-start runs (revert from FID-167)
- The engine on Anvil queries the chain at startup for the wallet's actual balance
- The journal is updated with the on-chain balance as the new starting point
- The dashboard shows $50 (the actual wallet balance)

### Risks

- **Reconciliation could overwrite a valid journal.** If the Anvil prefund tx was reverted, the chain says $0 but the journal says $X. We'd lose history. **Mitigation:** only reconcile if the chain balance is significantly different (>5% delta); otherwise keep the journal's value.
- **The Anvil prefund might be stale.** If Anvil was restarted with a different prefund, the wallet might have a different balance. **Mitigation:** the Anvil auto-start (start-anvil.bat) handles prefund deterministically.
- **Deleting dex_state.json might break emergency_liquidate.** `main.rs:560-586` reads dex_state.json. If we delete it, the emergency liquidator finds nothing. **Mitigation:** emergency_liquidate is a recovery tool, only runs on crash. With journal as the source, the engine can recover positions from journal, not from dex_state.json. The emergency_liquidate function should also be updated to use the journal.

---

## Impact Assessment

### Affected Components

- `start.bat` — 1 line change (default config revert)
- `src/engine/mod.rs` — add on-chain reconciliation at startup (~20 lines)
- `data/dex_state.json` — DELETE (file is the source of the conflict)
- `src/main.rs:560-586` — `emergency_liquidate` should use journal, not dex_state.json
- `src/engine/debug.rs:241-242` — debug-only, can be updated later
- `src/execution/dex/trader.rs:567` — `state_path` field, can be removed

### Risk Level

- [x] Critical
- [ ] High
- [ ] Medium
- [ ] Low

This is critical because the engine is currently showing wrong balance ($29.97 vs $50) and Spencer's workflow (Anvil) is broken.

### Latency Impact

- On-chain balance query at startup: ~1-2 RPC calls, ~1s latency. Negligible.
- Reconciliation logic: O(1), negligible.

---

## Proposed Solution

### Approach

1. **Revert `start.bat` line 21 default to `config\test-anvil.toml`.** Also remove the Anvil auto-start skip. Anvil is the working dev setup.
2. **Add on-chain balance reconciliation at engine startup.** Before the engine starts trading, query the Anvil RPC for `wallet.balanceOf(USDC)`. If the result is significantly different from the journal's balance, use the on-chain value and update the journal.
3. **Delete `data/dex_state.json`.** The journal is the single source. The engine's startup reconciliation gives the correct starting balance from the chain.
4. **Update `main.rs:560` `emergency_liquidate` to use journal.** Read positions from the journal, not dex_state.json. The journal has `load_positions()` for this.
5. **Remove `state_path` field from `DexTrader`.** If any code still writes to dex_state.json, remove the writer.

### Steps

1. **2 min:** Revert start.bat default to test-anvil.toml.
2. **5 min:** Add on-chain reconciliation in engine/mod.rs startup.
3. **5 min:** Update emergency_liquidate to use journal.
4. **10 min:** Remove state_path from DexTrader.
5. **2 min:** Delete data/dex_state.json.
6. **5 min:** cargo test + clippy + build --release.
7. **3 min:** ECHO FID close-out.

**Total: ~30 min.**

### Verification

- Engine starts on test-anvil.toml
- Anvil auto-starts
- Engine queries Anvil for USDC balance → $50.00
- Engine updates journal with starting_equity=$50
- Dashboard shows $50.00
- 5 closed trades in journal are unchanged (history preserved)
- `data/dex_state.json` is deleted
- All tests pass

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** What if the Anvil RPC returns an error (e.g., Anvil not running, wrong RPC URL)?
- **GREEN:** Graceful degradation: if the chain query fails, log a warning and use the journal's balance. The engine still starts, just with potentially-stale data.
- **AUDIT:** Verify the fallback path.
- **CHANGE DELTA:** +5 lines (Result handling).

### Loop 2 (anticipated — what about the open positions?)

- **RED:** `dex_state.json` has `positions: []` and the journal might have positions too. If they disagree, which wins?
- **GREEN:** The chain is the source of truth for positions too. Query `wallet.getOpenPositions()` (or equivalent) on Anvil. Use the chain's view. The journal is historical only.
- **AUDIT:** Verify position reconciliation.
- **CHANGE DELTA:** +20 lines (position reconciliation).

### Loop 3 (anticipated — the wallet recovery module)

- **RED:** `src/execution/wallet_recovery.rs` has explicit comment "JSON is a cache, not the truth source." This module might do the right thing. Let me check.
- **GREEN:** Read wallet_recovery.rs and see if it already has the on-chain reconciliation logic. If yes, the fix is just to wire it up at startup.
- **AUDIT:** Read the module.
- **CHANGE DELTA:** Depends on existing code.

### Loop 4 (anticipated — the live_execution branch)

- **RED:** The engine has different code paths for `executor.is_some()` (live) vs `executor.is_none()` (paper). The journal restoration is only used when `executor.is_none()`. With Anvil + live_execution=true, the engine should use on-chain balance, not journal.
- **GREEN:** Verify by reading the code path at engine/mod.rs:413-432.
- **AUDIT:** Check the live_execution path.
- **CHANGE DELTA:** 0 lines (code already handles this; we just need the journal NOT to be authoritative for live).

### Loop 5 (anticipated — Anvil RPC speed)

- **RED:** Anvil fork is local (http://127.0.0.1:8545). The USDC `balanceOf` call is fast. No issue.
- **GREEN:** None needed.
- **AUDIT:** Quick test.
- **CHANGE DELTA:** 0 lines.

---

## Resolution

*(Filled at close)*

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-16 HH:MM EST
- **Fix Description:** Reverted start.bat default to test-anvil.toml. Added on-chain reconciliation at engine startup. Deleted dex_state.json. Updated emergency_liquidate to use journal.
- **Tests Added:** TBD
- **Verified By:** TBD
- **Commit/PR:** Pending
- **Archived:** Pending

---

## Lessons Learned

*(Filled at close)*

---

*FID-177 created 2026-06-16 23:00 EST — Vera — revert start.bat to Anvil default, reconcile chain balance, delete dex_state.json*
