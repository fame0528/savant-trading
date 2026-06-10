# FID-2026-0609-105: 0x API Returns Reversed Swap Direction on Close

**ID:** FID-2026-0609-105
**Created:** 2026-06-09 21:00
**Severity:** critical
**Status:** created
**Scope:** src/execution/dex/trader.rs (execute_swap), src/execution/dex/zero_x.rs (quote)

---

## Problem

When closing a LONG position (selling AAVE for USDC), the 0x API `/quote` endpoint returned calldata that **bought AAVE with USDC** instead of **selling AAVE for USDC**. The transaction succeeded on-chain, resulting in:
- Wallet lost 20.359 USDC
- Wallet gained 0.327 AAVE (the position was NOT closed)
- Net effect: the "close" bought more AAVE instead of selling it

## Root Cause

The `execute_swap` function at `trader.rs:686` calls `self.backend.quote(&swap_params)` with:
```
sellToken=AAVE&buyToken=USDC&sellAmount=327666339191087852&taker=0x543CA043...
```

The 0x API returned calldata for a multi-hop route: USDC → WETH → AAVE. This is a **buy** route, not a **sell** route.

The `execute_swap` function has **no verification** that the returned calldata actually sells `src_token` for `dst_token`. It only checks:
1. Dust output (line 759) — passed because the output was 0.327 AAVE (non-zero)
2. Spread check (line 785) — passed because the price was reasonable
3. Pre-flight simulation (line 566) — passed because the calldata was valid

None of these checks verify the **direction** of the swap.

## Evidence

From Arbiscan tx `0x9d5fdefb...`:
```
From 0x543CA043... To 0xfeEA2A79... (0x Executor) For 20.359713 USDC
From 0xfeEA2A79... To 0x543CA043... For 0.327666 AAVE
```

The wallet sent USDC to the executor and received AAVE back. This is a BUY.

From Arbiscan tx `0x7b2098a1...` (your manual close):
```
From 0x543CA043... To 0x0a2854Fb... (Uniswap Router) For 0.327666 AAVE
From 0x0a2854Fb... To 0x543CA043... For 20.050402 USDC
```

Your manual close correctly sent AAVE and received USDC. This is a SELL.

## Fix

Add a post-swap verification step that checks the actual token transfers in the transaction receipt:

1. After `wait_for_receipt` returns successfully, parse the receipt logs
2. Verify that `src_token` was transferred **from** the wallet (or executor) **to** a DEX/router
3. Verify that `dst_token` was transferred **to** the wallet (or executor)
4. If the direction is reversed, return an error and do NOT record the trade as closed

Alternatively (simpler): add a check in `execute_swap` that compares the `buyAmount` from the quote against the expected output. If `buyAmount` is in `src_token` units instead of `dst_token` units, reject the quote.

## Scope

- `src/execution/dex/trader.rs`: Add direction verification in `execute_swap` (after line 600)
- New function: `verify_swap_direction(receipt, src_token, dst_token) -> bool`

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — 264 pass
3. Runtime: Close AAVE/USD LONG → verify AAVE leaves wallet, USDC arrives

---

## Additional Issue: Gas Charges on "Gasless" Transactions

The user noted that none of the transactions were actually gasless — all show gas charges (~0.01c). The gasless fallback path (`build_gasless_swap_tx`) is only triggered when the standard swap returns a "Dust output" error. Since the standard swap succeeded (but in the wrong direction), the gasless path was never reached.

The gasless API fix from FID-104 (adding `chainId` to submit body) is still valid but was never exercised because the standard swap "successfully" returned wrong-direction calldata.
