# FID: Engine Trigger Stale Price + Balance Query Zero + Missing Pair Eval + Position Age Reset

**Filename:** `FID-2026-0608-089-trigger-stale-price-balance-zero.md`
**ID:** FID-2026-0608-089
**Severity:** critical
**Status:** merged-into-master
**Superseded by:** MASTER-FID-2026-0609 (P0-1)

**Note:** This FID is a duplicate of FID-2026-0608-091. Both merged into Master FID P0-1.
**Created:** 2026-06-08 21:10
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

Four bugs compounding into a catastrophic chain on the first cycle after v0.11.7 restart:

1. **Engine trigger uses stale `pos.current_price`** (entry price 1698.22) instead of actual market price (~$1670) → stop set ABOVE market → immediate false stop loss on LONG position
2. **Only 1/2 pairs evaluated** — LINK/USD completely skipped from LLM evaluation despite being queued
3. **`query_token_balance` returns 0** despite tokens existing on-chain → prevents proper close execution
4. **Position `opened_at` resets on every restart** — wallet recovery sets `opened_at: Utc::now()`, so 48h positions show as "1h old" → breaks dead capital trigger and dashboard display

The chain: Bug 1 (stale price) → stop set above market → check_stops triggers → Bug 3 (balance=0) → close "succeeds" with 0 qty → phantom trade recorded → FID-074 reverts but damage done.

---

## Environment

- **OS:** Windows (win32)
- **Language/Runtime:** Rust 2021, tokio async runtime
- **Tool Versions:** savant-trading v0.11.7
- **LLM:** owl-alpha via OpenRouter (free, 1M context)
- **Chain:** Arbitrum (chain_id=42161)

---

## Detailed Description

### Bug 1: Engine Trigger Uses Stale Price

**File:** `src/engine.rs`, lines 2180-2270

The FID-088 engine-side management trigger calculates mandated stop using `pos.current_price`:
```rust
mandated_stop = match pos.side {
    Side::Long => pos.current_price - (atr * 1.5),  // line 2212
    Side::Short => pos.current_price + (atr * 1.5),  // line 2213
};
```

But `pos.current_price` is set to `entry_price` at wallet recovery (line 1005: `current_price: market_price.max(entry_price)`) and is only updated by `update_prices()` at line 3525 — which runs AFTER the decision processing loop.

**Timeline:**
- Wallet recovery: `current_price = 1698.22` (entry price)
- Engine trigger: `mandated_stop = 1698.22 - (9.25 * 1.5) = 1684.35`
- Actual market price: ~$1670 (from candle data)
- Result: stop at 1684.35 is ABOVE market 1670 → LONG stop triggers immediately

**Evidence from log:**
```
[STOP] WETH/USD stop updated $1562.3624 → $1688.9693
[  ├─] Entry 1698.2200 | Price 1698.2200 | PnL: $0.00 (+0.00%) | SL: 0.5% from price (below entry)
[SL] WETH/USD LONG | Entry: 1698.2200 → Exit: 1680.5244 | PnL: $-0.16 (-1.14%) | Stop loss hit (Full)
```

The "Price 1698.2200" confirms stale price. Actual market was ~$1670.

### Bug 2: Only 1/2 Pairs Evaluated

**File:** `src/engine.rs`, batch evaluation loop

The log shows:
```
[PHASE2] 2 pairs queued for LLM evaluation
[LLM] BATCH EVALUATING 2 pairs (single call)
[LLM] BATCH COMPLETE 2 pairs, 2881 chars in 57414ms
[PHASE2] Parsed 2 decisions from batch response
[PHASE3] Processing 2 LLM results...
```

But only WETH appears in execution logs. LINK never appears — no PASS log, no trigger check, nothing.

**Root cause:** The LLM (owl-alpha) returned 2 JSON objects but likely both for WETH (duplicate pair field). The batch parser at line 1912 iterates by JSON object, not by unique pair. If the LLM duplicates a pair, the second evaluation overwrites the first in execution but both count toward "Parsed 2 decisions."

**Evidence:** The user's AI output only shows WETH reasoning. No LINK reasoning appears.

### Bug 3: `query_token_balance` Returns 0

**File:** `src/execution/dex/trader.rs`, line 1168

The close path queries on-chain balance:
```rust
let on_chain_balance = self
    .query_token_balance(&src_token.address, src_token.decimals)
    .await
    .unwrap_or(close_qty);
```

The log shows: `Close qty adjusted: requested=0.00802278 on-chain=0.00000000 → using 0.00000000`

This means `query_token_balance` returned `Some(0.0)`, not `None`. The Bug D fix (FID-087) returns `None` on parse failure, but if the RPC returns "0x0" (valid zero), it parses to 0 and returns `Some(0.0)`.

**Root cause:** The RPC returns "0x0" for the WETH balance query at close time, despite the startup balance check showing 0.00802278 WETH. Possible causes:
- The `rpc_call` method returns different results at different times
- The token address from `resolve_pair_on_chain` differs from the one used at startup
- The RPC endpoint returns cached/stale responses

**Note:** The startup `sync_balance()` at line 1705 still uses `.unwrap_or(U256::ZERO)` — the OLD unfixed code. The Bug D fix only updated `query_token_balance` at line 1783. This inconsistency should be fixed.

### Bug 4: Position `opened_at` Resets on Restart

**File:** `src/engine.rs`, wallet recovery at line 1012

Wallet recovery creates positions with `opened_at: chrono::Utc::now()`. On every restart, positions appear newly opened. The real positions have been held for 48+ hours but show as "1h old" on the dashboard.

**Impact:**
- Dashboard "Age" display is wrong
- Dead Capital Tolerance management trigger (3+ cycles in ranging) uses wrong time basis
- Any time-based analysis is corrupted

---

## Impact Assessment

### Affected Components

- `src/engine.rs` — engine trigger (lines 2180-2270), wallet recovery (line 1012), batch evaluation
- `src/execution/dex/trader.rs` — `query_token_balance` (line 1783), `sync_balance` (line 1690)
- `src/agent/decision_parser.rs` — batch parsing (no code change needed, model behavior issue)
- Dashboard — position age display

### Risk Level

- [x] Critical: System crash, data loss, or security vulnerability
  - False stop losses close real positions without on-chain execution
  - Phantom trades corrupt journal
  - Missing pair evaluations leave positions unmanaged
  - Wrong position age breaks time-based triggers

---

## Proposed Solution

### Fix 1: Use Actual Market Price in Engine Trigger (3 layers)

Replace `pos.current_price` with `market_stores.get(&pair).and_then(|s| s.last().map(|c| c.close))` in the engine trigger. The `market_stores` HashMap is accessible in scope (line 291).

**Layer 1:** Use market_stores for actual price instead of pos.current_price
**Layer 2:** Add stale-price guard — if `pos.current_price` is within 0.1% of `pos.entry_price`, skip the trigger (price hasn't been refreshed yet)
**Layer 3:** Add ATR sanity check — if `atr > entry_price * 0.10`, ATR data is unreliable, skip trigger

### Fix 2: Validate Batch Pair Diversity

After parsing the batch response, count unique pairs. If fewer unique pairs than requested, log a warning with the specific missing pairs. Missing pairs auto-evaluate next cycle (they remain in the active pair list). No code change needed for recovery — just better observability.

### Fix 3: Debug `query_token_balance` + Fix `sync_balance`

**3a:** Add debug logging to `query_token_balance` to log the exact token address, RPC response hex, and parsed value. This will reveal whether the RPC returns "0x0" or a valid non-zero value.

**3b:** Fix `sync_balance()` at line 1705 to use the same `match` pattern as `query_token_balance` (remove `.unwrap_or(U256::ZERO)`).

**3c:** Add startup balance cache — store the balance seen at startup. If `query_token_balance` returns 0 at close time but startup showed non-zero, log a loud warning. The caller's `unwrap_or(close_qty)` will use the requested quantity as fallback.

### Fix 4: Preserve `opened_at` from Journal

In wallet recovery, if the journal already has a position for that pair, `opened_at` is already preserved (the code uses the existing position object). For new wallet recovery positions (no journal entry, e.g. after nuclear reset), set `opened_at` to `chrono::NaiveDateTime::UNIX_EPOCH` as a sentinel value indicating "unknown entry time."

### Fix 5: Guard Against False Triggers (New)

Add a guard before the engine trigger: if `pos.current_price` is within 0.1% of `pos.entry_price`, the price hasn't been updated from market data yet. Skip the trigger entirely to avoid false positives. This prevents Bug 1 from recurring even if the market_stores lookup fails.

---

## Perfection Loop

### Loop 1

- **RED:** 4 bugs identified. Bug 1 (stale price) and Bug 3 (balance=0) create a catastrophic chain. Bug 2 (missing pair) leaves positions unmanaged. Bug 4 (age reset) breaks time-based triggers.
- **GREEN:** Refined all 4 fixes + added 3 new guards: (1) stale-price guard (skip trigger if current_price ≈ entry_price), (2) ATR sanity check (skip if ATR > 10% of price), (3) startup balance cache for fallback. Also fixed sync_balance error handling.
- **AUDIT:** All fixes verified against codebase. market_stores accessible at trigger scope. query_token_balance and sync_balance both addressed. opened_at preserved from journal automatically for existing positions.
- **CHANGE DELTA:** ~50 lines across 2 files (engine.rs, trader.rs). Within 10% Levenshtein limit.

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all 264+ tests pass
3. Restart engine, verify: engine trigger uses actual market price (not entry price)
4. Verify: both WETH and LINK appear in evaluation logs
5. Verify: `query_token_balance` returns actual balance (not 0)
6. Verify: position age shows correct time since wallet recovery

---

## Resolution

- **Fixed By:** [Pending]
- **Fixed Date:** [Pending]
- **Fix Description:** [Pending]
- **Tests Added:** [Pending]
- **Verified By:** [Pending]
- **Commit/PR:** [Pending]
- **Archived:** [Pending]
