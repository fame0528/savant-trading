# FID: Close Execution Death Loop — Wrong Token Balance + Retry Spam + 0x Liquidity Failure

**Filename:** `FID-2026-0608-094-close-death-loop.md`
**ID:** FID-2026-0608-094
**Severity:** critical
**Status:** analyzed
**Created:** 2026-06-08 23:20
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

The engine is stuck in a death loop: every 5-minute cycle, the FID-088 trigger tightens the stop, the tighter stop gets hit by market price, the SL fires, the close attempt fails, FID-074 reverts, and the cycle repeats. This has been happening for 45+ minutes across 8+ cycles.

**Root cause:** The side correction at startup updates `PortfolioManager` but NOT `DexTrader`'s internal `self.positions` map. When `close_position_internal` reads from DexTrader's map, it still sees `SHORT`, passes `Side::Long` to `resolve_pair_on_chain`, gets USDC as `src_token`, queries USDC balance (= $0), and the close fails with "Close qty adjusted to 0."

**Three compounding failures:**

1. **Wrong token balance query** — Queries USDC balance (decimals=6, balance=0) instead of WETH balance (decimals=18, balance=0.008) because DexTrader's position side is stale SHORT
2. **No close retry cooldown** — Engine retries the same failed close every cycle, creating a phantom SL death loop
3. **0x liquidity unavailable** — Even if balance were correct, 0x returns `liquidityAvailable=false` for the swap

---

## Evidence from Logs

```
22:07:49 SIDE CORRECTION: WETH/USD — spot-only mode, forced SHORT → LONG
22:07:49 SIDE CORRECTION: LINK/USD — spot-only mode, forced SHORT → LONG
22:09:13 FID-088 TRIGGER: Stop distance 30.5x ATR → ADJUST_STOP
22:15:22 FID-088 TRIGGER: Stop distance 8.4x ATR → ADJUST_STOP
22:21:31 SL LONG | PnL: $-0.38 | Stop loss hit
22:21:31 CLOSE FAILED: No liquidity available
22:28:13 SL LONG | PnL: $-0.36 | Stop loss hit
22:28:13 CLOSE FAILED: No liquidity available
22:34:29 SL LONG | PnL: $-0.36 | Stop loss hit
22:34:29 CLOSE FAILED: No liquidity available
... (repeats every 5 minutes for 45+ minutes)

BALANCE QUERY: 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 returned 0 (decimals=6)
```

`0xaf88d065...` = USDC on Arbitrum. Should be `0x82aF4944...` = WETH (decimals=18).

---

## Root Cause Chain

1. **Startup:** Journal loads positions as SHORT → registered in DexTrader with `side: Short`
2. **Side correction:** Engine forces SHORT → LONG in PortfolioManager — but DexTrader's `self.positions` is NOT updated
3. **Close path:** `close_position_internal()` reads from DexTrader's map → sees `side: Short`
4. **Token resolution:** `resolve_pair_on_chain("WETH/USD", Side::Long)` returns `(USDC, WETH)` — src_token is USDC
5. **Balance query:** Queries USDC balance = 0 → `actual_close_qty = 0`
6. **0x quote:** Requests swap of 0 WETH → `liquidityAvailable=false`
7. **Close fails:** FID-074 reverts phantom trade, position stays open
8. **Next cycle:** SL fires again → repeat from step 3

---

## Proposed Solution

### Fix 1: Sync DexTrader positions after side correction (ROOT CAUSE)

After the side correction loop in engine.rs, re-register the corrected positions in DexTrader by calling `register_position()` on the executor for each corrected position. This ensures DexTrader's internal map has the correct side.

### Fix 2: Close retry cooldown (DEATH LOOP BREAKER)

Add a `HashMap<String, Instant>` tracking the last failed close attempt per pair. If close failed within the last 30 minutes, skip the retry. This breaks the death loop immediately.

### Fix 3: FID-088 trigger guard (FUTILE TIGHTENING PREVENTION)

If close failed for this position in the last 30 minutes, don't fire ADJUST_STOP. The stop is already tight enough — the issue is execution, not risk management. Tightening the stop further just creates more phantom SL events.

### Fix 4: 0x liquidity cooldown

If 0x returns `liquidityAvailable=false`, record the timestamp. Don't retry close for 30 minutes. Log prominently.

### Fix 5: Death loop detection

If the same position has SL triggered 3+ consecutive cycles without successful close, halt close attempts for 1 hour and log a loud alert. Currently the engine retries forever with no circuit breaker.

### Fix 6: Zero-amount swap guard

In `close_position_internal()`, if `actual_close_qty` is 0 or very close to 0 (< 0.0001), return an error immediately instead of calling 0x with a zero-amount swap. This prevents the "sell entire balance of 0" scenario.

### Fix 7: Permanent balance query diagnostic

The `BALANCE QUERY:` warning from FID-089 was the breakthrough that revealed the wrong token address. Make it permanent (not temporary debug logging). Log at WARN level when balance is 0 for a non-stablecoin token.

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all 264+ tests pass
3. Restart engine → verify side correction updates DexTrader
4. Verify: balance query uses WETH address, not USDC
5. Verify: close retry cooldown prevents death loop
6. Verify: FID-088 trigger doesn't fire when close is on cooldown

---

## Perfection Loop

### Loop 1

- **RED:** Death loop identified: side correction only updates PortfolioManager, not DexTrader. Close path reads from DexTrader → wrong token → balance=0 → close fails → SL fires again every cycle. 45+ minutes of phantom SL events. Fixes 2+4 merged (same cooldown). Fix 3 depends on Fix 2. Fix 7 already implemented.
- **GREEN:** 6 effective fixes: (1) Sync DexTrader after side correction. (2) Close retry cooldown (30min, covers both balance=0 and 0x liquidity). (3) FID-088 trigger guard. (4) Death loop detection (3+ consecutive SL → halt 1 hour). (5) Zero-amount swap guard. (6) Balance query diagnostic (already permanent).
- **AUDIT:** Fix 1 uses executor_position_map for ID format with exec-{id} fallback. Fix 2 HashMap accessible from close path and SL check. Fix 3 checks cooldown before ADJUST_STOP. Fix 4 tracks consecutive SL per position. Fix 5 simple guard before 0x call. All verified.
- **CHANGE DELTA:** ~50 lines across 2 files (engine.rs, trader.rs).
