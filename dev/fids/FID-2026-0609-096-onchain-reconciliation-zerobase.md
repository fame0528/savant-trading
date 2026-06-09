# FID: On-Chain Reconciliation + Zero-Base Enforcement + Phantom Position Cleanup

**Filename:** `FID-2026-0609-096-onchain-reconciliation-zerobase.md`
**ID:** FID-2026-0609-096
**Severity:** critical
**Status:** analyzed
**Created:** 2026-06-09 00:01
**Updated:** 2026-06-09 00:08
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

The engine operates on stale portfolio data. It syncs USDC balance every 3 cycles but never re-checks token balances (WETH, LINK) after startup. If tokens are sold externally (manual swap, another app), the engine continues tracking phantom positions, making decisions on non-existent holdings, and showing incorrect portfolio values on the dashboard.

Additionally, the Zero-Base Review ("Would you buy at current price?") is correctly performed by the LLM but has no parser enforcement. The AI says "No" and then chooses HOLD — the `would_initiate_new_long_at_current_price` field is in the output schema but not in the Rust struct and has no enforcement logic.

This FID covers 11 items: 2 core fixes + 9 supporting fixes for data integrity, dashboard accuracy, and parser enforcement.

---

## Detailed Description

### Issue 1: No On-Chain Token Balance Reconciliation

`sync_balance()` at engine.rs:3883 runs every 3 cycles but only calls `ex.balance()` which returns USDC. Token balances (WETH, LINK) are checked once at startup via `sync_wallet_positions()` and never re-checked.

**Evidence:** User manually swapped LINK and WETH to USDC. On-chain token balances are 0. Engine still shows:
- 2/3 positions (LINK, WETH)
- Portfolio value $52.23 ($25.97 cash + $26.26 in phantom tokens)
- AI evaluating and adjusting stops on positions that don't exist
- FID-088 trigger tightening stops on phantom positions

### Issue 2: Zero-Base Review Not Enforced

AI decision for LINK/USD:
```
Zero-base review: Would I buy LINK at 7.87 with bearish EMA, declining structure,
and no clear support? No. However, stop at 7.38 is ~6% below current price.
ADX 22.92 < 25 threshold for adverse trend forced close. HOLD but monitor closely.
```

The AI correctly says "No" to Zero-Base Review but chooses HOLD. The `would_initiate_new_long_at_current_price` field is in `output_format.md` but:
- Not in the `TradeDecision` Rust struct
- Not parsed from LLM JSON output
- No enforcement in the parser

### Issue 3: `dex_state.json` Not Cleaned on External Close

If positions are removed from PortfolioManager and DexTrader, `dex_state.json` still has them. Next restart reloads phantom positions from the file.

### Issue 4: Zero-Base Field Is Nested, Not Top-Level

The AI outputs `would_initiate_new_long_at_current_price` inside the `position_audit` array, not at the root JSON level. The parser needs to look inside `position_audit[0]` for this field.

### Issue 5: Race Condition with Close Execution

If the engine tries to close a position (FID-088 trigger) in the same cycle the balance sync detects it's already gone, the close will fail. Need to handle: if close fails AND reconciliation detects external close, skip the FID-074 revert.

### Issue 6: Query Token Balance Verification

FID-094 fixed the side correction sync, but `query_token_balance()` at trader.rs:1167 hasn't been verified to return correct values after the fix. The reconciliation code relies on this function.

### Issue 7: Dashboard Notification on External Close

When reconciliation removes a phantom position, the dashboard should show a notification. Otherwise positions just disappear with no explanation.

### Issue 8: Journal Cleanup on External Close

Need to call `delete_position()` on the journal when external close is detected. Currently only mentioned for PortfolioManager and DexTrader cleanup.

### Issue 9: Equity Curve Correction

After removing phantom positions, equity curve will jump. Need to log the correction: "Equity corrected from $X to $Y after external close reconciliation."

### Issue 10: Daily PnL Recalculation

If phantom positions had negative unrealized PnL, removing them changes `daily_pnl`. Need to recalculate after reconciliation.

### Issue 11: AI Confidence on HOLD Is Always 0%

The AI assigns 0% confidence to all HOLD decisions for existing positions. The confidence should reflect conviction in the HOLD thesis. This affects the confidence cap display and Brier score. Needs a prompt fix.

---

## Proposed Solution

### Fix 1: Periodic On-Chain Token Balance Reconciliation

**File:** `src/engine.rs`, around line 3883

After `sync_balance()` runs (every 3 cycles), also query on-chain token balances for all held positions. If on-chain balance is 0 but position exists, the position was externally closed — remove it.

**Logic:**
```
Every 2 cycles (10 min):
  1. sync_balance() for USDC (existing)
  2. For each position in PortfolioManager:
     a. Resolve token address via resolve_pair_on_chain()
     b. Query on-chain balance via query_token_balance()
     c. If on_chain == 0 and position.quantity > 0:
        - Log: "EXTERNAL CLOSE: {pair} — on-chain balance is 0, position removed"
        - Remove position from PortfolioManager
        - Remove from DexTrader via register_position (or remove method)
        - Remove from executor_position_map
        - Delete from journal
        - Clear close_failure_cooldown for this pair
        - Log equity correction
        - Sync positions to shared state for dashboard
     d. If 0 < on_chain < position.quantity * 0.5:
        - Log: "PARTIAL EXTERNAL CLOSE: {pair} — on-chain {qty} vs position {qty}"
        - Update position.quantity to on_chain
        - Recalculate risk_amount
        - Save updated position to journal
```

### Fix 2: Zero-Base Review Parser Enforcement

**Files:** `src/agent/decision_parser.rs`

**2a.** Add `would_initiate_new_long_at_current_price: Option<bool>` to `TradeDecision` struct.

**2b.** Parse from `position_audit[0].would_initiate_new_long_at_current_price` (nested field).

**2c.** Add enforcement:
```
If would_initiate_new_long == Some(false) AND action is HOLD/PASS:
  Override to CLOSE
  Log: "ZERO-BASE ENFORCEMENT: wouldn't buy at current price → overriding to CLOSE"
```

### Fix 3: `dex_state.json` Cleanup on External Close

After removing a position from DexTrader during reconciliation, save the updated DexTrader state to `dex_state.json`. The DexTrader already has a `save_state()` method.

### Fix 4: Nested Position Audit Parsing

The `normalize_llm_json()` function needs to extract `would_initiate_new_long_at_current_price` from the `position_audit` array's first element, not from the root JSON.

### Fix 5: Race Condition Guard

In the close failure path, before applying FID-074 revert, check if the close error indicates zero balance. If the error contains "quantity too small" or "balance" or "on-chain=0", skip the FID-074 revert. The position is likely already gone externally — restoring it would create a phantom.

### Fix 6: Balance Query Verification (Testing Task)

After implementing Fix 1, verify in the engine logs that `query_token_balance()` returns correct values for WETH and LINK. The BALANCE QUERY debug logging from FID-089 will show the exact hex response. If still returning 0, debug the token address resolution.

### Fix 7: Dashboard External Close Notification

When reconciliation removes a position, send an activity log entry:
```
shared.log_activity(ActivityLevel::Warning, pair, "EXTERNAL CLOSE: tokens no longer on-chain — position removed")
```
The dashboard already displays activity log entries. This will show up automatically.

### Fix 8: Journal Cleanup in Reconciliation

In the reconciliation loop, call `journal.delete_position(&pos_id)` for each externally closed position. This prevents the position from being reloaded on next restart.

### Fix 9: Equity Curve Correction Logging

After removing phantom positions, log the equity change:
```
info!("Equity corrected: ${:.2} → ${:.2} after external close reconciliation", old_equity, new_equity);
```

### Fix 10: Daily PnL Self-Correction

After reconciliation removes positions, `refresh_equity()` is called which recalculates `equity` from remaining positions. The `daily_pnl` is updated by `update_prices()` each cycle which sums unrealized PnL changes — phantom positions will naturally be excluded since they're no longer in the positions map. No explicit recalculation needed, but `refresh_equity()` must be called after reconciliation.

### Fix 11: HOLD Confidence Display + Prompt Fix

**Files:** `dashboard/src/app/page.tsx`, `src/agent/prompts/output_format.md`

**Dashboard:** When confidence is 0% on a HOLD decision, display "—" instead of "0%". The 0% is misleading — it suggests no conviction when the agent is actively choosing to hold.

**Prompt:** Add to output_format.md field rules:
```
- confidence: For HOLD decisions on existing positions, set confidence to your conviction
  in the HOLD thesis (0.0-1.0), NOT 0.0. A confidence of 0.0 means "no conviction" which
  contradicts HOLD. If you're holding, you must believe the position will recover.
```

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all 264+ tests pass
3. Restart engine with manually sold positions → verify phantom positions detected and removed
4. Verify: "EXTERNAL CLOSE" appears in activity log for each phantom position
5. Verify: portfolio value reflects only real on-chain holdings ($25.97 cash, no phantom tokens)
6. Verify: AI saying "wouldn't buy" triggers CLOSE override
7. Verify: dex_state.json updated after external close
8. Verify: equity curve shows correction, not phantom jump

---

## Perfection Loop

### Loop 1

- **RED:** 11 items identified. Core issues: (1) no token balance reconciliation after startup, (2) Zero-Base Review field not in Rust struct. Supporting: dex_state cleanup, race condition, journal cleanup, equity correction, PnL recalculation, dashboard notification, balance query verification, confidence prompt fix.
- **GREEN:** 10 gaps found and fixed: (1) reconciliation frequency changed from 3 to 2 cycles, (2) nested parsing for position_audit fields, (3) race condition check on close error message, (4) balance query moved to verification, (5) daily PnL self-corrects via refresh_equity, (6) confidence display shows "—" not "0%", (7) executor_position_map cleanup, (8) shared state sync after reconciliation, (9) journal cleanup in reconciliation, (10) prompt fix for HOLD confidence.
- **AUDIT:** All fixes verified. Reconciliation uses existing query_token_balance() + resolve_pair_on_chain(). Nested parsing extracts from position_audit[0]. Race condition checks close error message. Dashboard change is frontend-only. Total: ~80 lines across 5 files.
- **CHANGE DELTA:** ~80 lines across 5 files (engine.rs, decision_parser.rs, output_format.md, shared.rs, page.tsx).
