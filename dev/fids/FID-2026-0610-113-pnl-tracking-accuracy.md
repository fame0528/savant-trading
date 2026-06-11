# FID-2026-0610-113: PnL Tracking — Closed Trade PnL Underreports Actual Costs

**Filename:** `FID-2026-0610-113-pnl-tracking-accuracy.md`
**ID:** FID-2026-0610-113
**Severity:** medium
**Status:** analyzed
**Created:** 2026-06-10 20:45
**Author:** Buffy (Codebuff AI)
**Type:** bug-fix
**Scope:** src/execution/dex/trader.rs, src/engine/mod.rs

---

## Summary

The closed trades table shows -$0.13 total PnL but the actual economic loss is -$10.37. The gap comes from untracked DEX fees: LP fees (0.3% per side), gas costs, and spread/slippage are not included in the per-trade PnL calculation. The fee estimate in DexTrader uses 0.1% instead of the actual 0.3% LP fee, and gas costs are not captured at all.

## Detailed Description

### Problem

Dashboard shows:
- Profit: -$10.37 (equity $19.63 - starting $30.00)
- Closed trades PnL: -$0.13 (individual trade price movement only)

The $10.24 gap is from ~14 swaps × ~$0.73 average cost per swap (LP fees + gas + slippage). The closed trade PnL only captures price movement, not execution costs.

### Expected Behavior

Closed trade PnL should reflect the actual economic outcome including all execution costs (LP fees, gas, slippage). The dashboard profit and closed trades total should reconcile.

### Root Cause

1. `fee_est` in DexTrader uses 0.1% instead of actual 0.3% Uniswap v3 LP fee
2. Gas costs (~$0.025/swap on Arbitrum) are not included in trade PnL
3. Entry price may not match actual DEX execution price (slippage)

### Evidence

```text
Dashboard: Profit -$10.37, Closed Trades: 7 trades, PnL total -$0.13
Gap: $10.24 unaccounted (fees + slippage across ~14 swaps)
```

## Impact Assessment

### Affected Components

- `src/execution/dex/trader.rs` — fee estimation in swap execution
- Dashboard closed trades table — misleading PnL display

### Risk Level

- [ ] Critical
- [ ] High
- [x] Medium: Feature degraded, misleading display
- [ ] Low

## Proposed Solution

### Approach

1. Update `fee_est` from 0.001 (0.1%) to 0.003 (0.3%) to match actual Uniswap v3 LP fees
2. Capture actual gas cost from transaction receipt and include in trade PnL
3. Use actual execution price (from on-chain receipt) instead of quote price for PnL calculation

### Steps

1. Fix fee_rate in DexTrader fee estimation
2. After successful swap, read gas used from receipt and compute gas cost
3. Store actual execution price in trade record
4. Recalculate PnL using actual entry/exit prices minus all costs

### Verification

1. `cargo clippy -- -D warnings` — 0 warnings
2. `cargo test` — all tests pass
3. Runtime: Closed trades PnL total should reconcile with dashboard profit (within $0.50)

## Perfection Loop

### Loop 1

- **RED:** Closed trades PnL underreports by $10.24 due to untracked fees
- **GREEN:** (Deferred — requires on-chain receipt parsing and trade record schema changes)
- **AUDIT:** Deferred to next session
- **CHANGE DELTA:** 0% (analysis only, fix deferred)

## Resolution

- **Fixed By:** Buffy (Codebuff AI)
- **Fixed Date:** 2026-06-10 (analysis complete, fix deferred)
- **Fix Description:** Analysis complete. Fix requires: (1) update fee_est to 0.3%, (2) capture gas from tx receipt, (3) use actual execution price. Deferred due to scope — requires trade record schema changes.
- **Tests Added:** N/A (deferred)
- **Verified By:** Analysis verified via dashboard data reconciliation
- **Commit/PR:** Pending (deferred fix)
- **Archived:** Pending

## Lessons Learned

- The 0.1% fee estimate was carried over from the original Kraken config. DEX LP fees are 0.3% on Uniswap v3. Config drift between CEX and DEX assumptions causes silent PnL miscalculation.
- Gas costs on Arbitrum are small (~$0.025/swap) but compound across many swaps. At 14+ swaps, they contribute ~$0.35 to the total loss.
- The dashboard profit (equity - starting_balance) is economically correct. The closed trades table is misleading because it only shows price movement PnL. Both views should reconcile.
