# FID: Swap execution hang — place_order() never returns

**Filename:** `FID-2026-0603-027-swap-execution-hang.md`
**ID:** FID-2026-0603-027
**Severity:** critical
**Status:** resolved
**Created:** 2026-06-03 09:00
**Author:** Agent

---

## Summary

Engine's `place_order()` call hung indefinitely when executing a DEX swap. The AI made a valid BUY decision, position sizer approved, but the swap never completed — no success, no error, no timeout. The engine silently moved to the next cycle.

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.94, tokio
- **Commit/State:** `8300eae` (pre-fix)
- **Backend:** 0x DEX on Arbitrum

## Detailed Description

### Problem

`[PHASE3] Checking execution for XRP/USD (action=Buy)` was logged, but nothing after it — no `[SWAP]` logs, no error, no receipt. The engine moved to Phase 2 of the next cycle without any swap result.

### Expected Behavior

`place_order()` should either succeed (return `Ok(Order)`) or fail (return `Err`) within a reasonable time. A 60-second timeout should prevent indefinite hangs.

### Root Cause

1. `execute_swap()` called `sign_and_send()` which called `eth_sendRawTransaction` — the RPC call could hang indefinitely with no timeout
2. The 0x API returned a gas price that was slightly below the current baseFee by the time the tx was broadcast
3. No retry logic existed — a single transient failure killed the entire trade

### Evidence

```text
[PHASE3] Checking execution for XRP/USD (action=Buy)
[PHASE3] Buy path entered for XRP/USD — calculating position size
[PHASE3] Placing order for XRP/USD via executor...
[SWAP] Calling 0x API: USDC -> XRP amount=...
[SWAP] 0x quote OK: to=0xfee... gas=300000
[SWAP] Signing tx: to=0xfee... data_len=5160 value=0 gas=300000
[SWAP] TX broadcast FAILED: max fee per gas less than block base fee
```

## Impact Assessment

### Affected Components

- `src/execution/dex/trader.rs` — execute_swap(), sign_and_send()
- `src/engine.rs` — Phase 3 execution block

### Risk Level

- [x] Critical: Real money at risk — $35 USDC could not be deployed

## Proposed Solution

### Steps

1. Add 60s `tokio::time::timeout` around `place_order()` and `close_position()` in engine.rs
2. Add 50% gas buffer to `maxFeePerGas` (baseFee + baseFee/2 + priority)
3. Add 3-retry logic for transient failures (gas spike, nonce collision, network error)
4. Add `eprintln!` logging at every execution checkpoint

### Verification

- `cargo build --release` — zero errors
- `cargo test` — 187 tests pass
- `cargo clippy` — zero warnings

## Perfection Loop

### Loop 1

- **RED:** Swap hung indefinitely, gas too low, no retry
- **GREEN:** Added 60s timeout, 50% gas buffer, 3-retry logic
- **AUDIT:** 187 tests pass, clippy clean
- **CHANGE DELTA:** ~5% of trader.rs, ~3% of engine.rs

## Resolution

- **Fixed By:** Agent
- **Fixed Date:** 2026-06-03 10:30
- **Fix Description:** Added timeout, gas buffer, retry logic, debug logging
- **Tests Added:** No new tests (existing tests cover the path)
- **Verified By:** cargo test + cargo clippy
- **Commit/PR:** `e336bd9`, `21b177c`

## Lessons Learned

- Always add timeouts to network calls — `reqwest::Client::new()` has no default timeout
- Gas prices are stale by the time a tx is broadcast — always add a buffer
- Transient failures need retry logic — a single failure shouldn't kill a trade
