# FID: Balance Query Zero + Missing Pair Eval + Position Age Reset

**Filename:** `FID-2026-0608-091-balance-zero-missing-pair-age-reset.md`
**ID:** FID-2026-0608-091
**Severity:** high
**Status:** merged-into-master
**Superseded by:** MASTER-FID-2026-0609 (P0-1)
**Created:** 2026-06-08 21:19
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

Three residual bugs from the v0.11.5-v0.11.8 fix cascade:

1. **`query_token_balance` returns 0** despite 0.008 WETH existing on-chain. Debug logging added in v0.11.8 but root cause unconfirmed. Prevents proper close execution.
2. **Only 1/2 pairs evaluated** in LLM batch. The batch parser correctly handles standalone JSON objects, so the LLM likely returned duplicate pair fields (both WETH) or one malformed decision.
3. **Position `opened_at` resets on restart** — wallet recovery sets `opened_at: Utc::now()`, so 48h positions show as "1h old" on dashboard.

---

## Environment

- **OS:** Windows (win32)
- **Language/Runtime:** Rust 2021, tokio async runtime
- **Tool Versions:** savant-trading v0.11.8
- **LLM:** owl-alpha via OpenRouter (free, 1M context)
- **Chain:** Arbitrum (chain_id=42161)

---

## Detailed Description

### Bug 1: `query_token_balance` Returns 0

**File:** `src/execution/dex/trader.rs`, line 1168

The close path queries on-chain balance:
```rust
let on_chain_balance = self.query_token_balance(&src_token.address, src_token.decimals)
    .await
    .unwrap_or(close_qty);
```

Log shows: `Close qty adjusted: requested=0.00802278 on-chain=0.00000000 → using 0.00000000`

This means `query_token_balance` returned `Some(0.0)`, not `None`. The FID-087 Bug D fix returns `None` on parse failure, but if the RPC returns "0x0" (valid zero), it parses to 0 and returns `Some(0.0)`.

**Possible root causes (debug logging will confirm):**
- **Hypothesis A:** Token address mismatch — `resolve_pair_on_chain` returns a different address than `sync_balance` uses at startup. Debug log will show the exact address.
- **Hypothesis B:** RPC returns cached "0x0" — the `rpc_call` method may cache or return stale responses. Debug log will show the exact hex.
- **Hypothesis C:** Precision issue — `wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor` may lose precision for very small balances. But 0.008 WETH (8e15 wei) should parse fine.

**Additional safeguard needed:** If `query_token_balance` returns `Some(0.0)` but startup `sync_balance` showed non-zero, the engine should log a loud warning and use the startup balance as fallback. This prevents the phantom close issue immediately.

### Bug 2: Only 1/2 Pairs Evaluated

**File:** `src/engine.rs`, batch evaluation loop (lines 1830-1929)

The LLM was sent 2 pairs ("2 pairs queued") and the parser extracted 2 JSON objects ("Parsed 2 decisions"). But only WETH appears in execution logs.

**Root cause analysis:**
- `extract_json_array` at line 358 has a slow path that extracts individual JSON objects by brace counting — this works correctly.
- The LLM likely returned 2 JSON objects but both with `pair: "WETH/USD"` (duplicate pair field).
- Or: one decision had a malformed `pair` field that defaulted to "UNKNOWN" and was silently processed as a separate pair.
- The batch parser at line 1912 pushes each decision to `all_results` without validating pair uniqueness.

**Fix:** After parsing the batch response, validate that the number of unique pairs matches the number requested. Log a warning if fewer unique pairs than expected. Missing pairs auto-evaluate next cycle (they remain in `active_pairs`).

### Bug 3: Position `opened_at` Resets on Restart

**File:** `src/engine.rs`, wallet recovery at line 1012

Wallet recovery creates new positions with `opened_at: chrono::Utc::now()`. After a nuclear reset (journal cleared), wallet recovery creates fresh positions with `opened_at = now()`.

**Impact:**
- Dashboard "Age" shows "1h+" instead of "48h+"
- Dead Capital Tolerance trigger uses cycle count (not `opened_at`), so NOT affected
- This is cosmetic but misleading

**Fix:** For wallet recovery without journal entry, set `opened_at` to `chrono::NaiveDateTime::UNIX_EPOCH` as sentinel. Dashboard shows "unknown" for epoch-0 positions.

---

## Impact Assessment

### Affected Components

- `src/execution/dex/trader.rs` — `query_token_balance`, `sync_balance`, `close_position_internal`
- `src/engine.rs` — batch evaluation loop, wallet recovery
- `src/agent/decision_parser.rs` — `extract_json_array` (no change needed — works correctly)
- Dashboard — position age display

### Risk Level

- [x] High: Feature broken, workaround exists
  - Balance query zero prevents proper closes (workaround: `unwrap_or(close_qty)` falls back)
  - Missing pair eval leaves positions unmanaged for one cycle (auto-recovers next cycle)
  - Position age reset is cosmetic (no behavior impact)

---

## Proposed Solution

### Fix 1: Startup Balance Cache + Fallback

**File:** `src/engine.rs` (startup) + `src/execution/dex/trader.rs` (close path)

**1a.** During `sync_balance()` startup, store the observed balances in a `HashMap<String, f64>` on the DexTrader struct (e.g., `startup_balances`).

**1b.** In `close_position_internal()`, after `query_token_balance` returns, check: if result is `Some(0.0)` but `startup_balances` has a non-zero value for this token, log a loud warning and use `startup_balances` value as fallback.

**1c.** Debug logging already added in v0.11.8. On next close attempt, the log will show the exact token address, hex response, and parsed value. This will confirm the root cause.

### Fix 2: Batch Pair Validation

**File:** `src/engine.rs`, after line 1910

After `extract_json_array` returns, count unique pairs in the parsed decisions. If fewer unique pairs than `batch_size`, log a warning:
```
WARN: Batch returned {unique} unique pairs but {batch_size} were requested. Missing: {missing_pairs}
```

Missing pairs auto-evaluate next cycle (no code change needed for recovery — they remain in `active_pairs`).

### Fix 3: Sentinel `opened_at` for Wallet Recovery

**File:** `src/engine.rs`, wallet recovery at line 1012

Change `opened_at: chrono::Utc::now()` to `opened_at: chrono::DateTime::<chrono::Utc>::from(chrono::NaiveDateTime::UNIX_EPOCH)` for new wallet recovery positions (no journal entry).

Dashboard: show "unknown" for positions where `opened_at` is epoch-0.

---

## Perfection Loop

### Loop 1

- **RED:** 3 bugs. Bug 1 (balance=0) prevents closes. Bug 2 (missing pair) leaves positions unmanaged. Bug 3 (age reset) is cosmetic. Bug 1 is highest priority — debug logging added, startup cache is the immediate fix.
- **GREEN:** Bug 1: startup balance cache as fallback. Bug 2: unique pair count validation. Bug 3: sentinel epoch-0 for unknown entry time.
- **AUDIT:** Bug 1: startup cache is clean — HashMap on DexTrader, checked at close time. Bug 2: validation after extract_json_array, no parser change needed. Bug 3: epoch-0 sentinel is standard practice. All fixes are small (< 20 lines each).
- **CHANGE DELTA:** ~30 lines across 2 files (engine.rs, trader.rs).

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all 264+ tests pass
3. Restart engine, trigger a close — verify balance debug log shows token address and hex
4. Verify: if balance returns 0, startup cache fallback activates with loud warning
5. Verify: batch evaluation logs unique pair count
6. Verify: wallet recovery positions show "unknown" age on dashboard

---

## Resolution

- **Fixed By:** [Pending]
- **Fixed Date:** [Pending]
- **Fix Description:** [Pending]
- **Tests Added:** [Pending]
- **Verified By:** [Pending]
- **Commit/PR:** [Pending]
- **Archived:** [Pending]
