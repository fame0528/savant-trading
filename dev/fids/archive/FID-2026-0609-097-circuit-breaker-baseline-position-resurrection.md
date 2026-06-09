# FID-097: Circuit Breaker Baseline Corruption + Position Resurrection + Decision Parser Noise

**ID:** FID-2026-0609-097
**Severity:** critical
**Status:** closed
**Created:** 2026-06-09 02:15
**Closed:** 2026-06-09 02:55
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

v0.12.3 reconciliation successfully detected phantom positions and removed them, but exposed 8 downstream issues. The most critical: circuit breaker baseline corruption blocks ALL trade execution. Secondary: decision parser noise, batch size inconsistency, context state waste, and security leaks.

---

## Issues Found + Fixes Applied

### Fix 1: 🔴 Circuit Breaker Stuck at 50.60% Drawdown → RESET peak_equity

**Root cause:** `peak_equity` was set at $52.20 (including phantom WETH/LINK positions). Real equity was $25.97. Drawdown = 50.2%. Circuit breaker threshold is 10%.

**Fix:** After reconciliation removes phantom positions (both via startup clear and per-cycle external close detection), reset `peak_equity` to current equity and zero `drawdown_pct`.

**Lines changed:** +6 in `src/engine.rs` (3 lines for startup clear, 3 for per-cycle).

### Fix 2: 🔴 Position Resurrection Loop → RECONCILIATION_REMOVED GUARD

**Root cause:** FID-074 revert restores positions after failed closes. Reconciliation removes them, but the next cycle's FID-074 revert path re-creates them.

**Fix:** Added `reconciliation_removed: HashSet<String>` in engine scope. When reconciliation removes a position, the ID is added to the set. Both FID-074 revert paths check this set — if the position was reconciliation-removed, the restore is skipped with a warning.

**Lines changed:** ~15 in `src/engine.rs`. 5 production sites: 1 declaration, 2 inserts (startup clear + external close), 2 checks (both revert paths).

### Fix 3: 🟡 Batch Duplicate Decisions → DEDUPLICATION

**Root cause:** LLM sometimes returns duplicate decisions for the same pair (10 > 8 pairs queued).

**Fix:** After `extract_json_array`, deduplicate by pair name (keep last decision per pair). Log warning with duplicate count.

**Lines changed:** ~25 in `src/engine.rs`.

### Fix 4: 🟡 Wallet Address in Plaintext Logs → MASKING (Law 12)

**Root cause:** `info!("Wallet address cached: {}", addr)` logged the full address.

**Fix:** Mask wallet address in log output: first 6 + last 4 chars (`0x543c...11fBC`).

**Lines changed:** ~6 in `src/engine.rs`.

### Fix 5: 🟡 Batch Size Inconsistency → VALIDATION LOGGING

**Root cause:** LLM sometimes returns fewer decisions than requested (6/8). Missing pairs were silent.

**Fix:** After deduplication, if unique pairs < requested, compute the difference and log missing pair names.

**Lines changed:** ~12 in `src/engine.rs` (integrated into Fix 3 block).

### Issues 6-8: Non-code
- **#6 (delta compression ~90%+):** Anti-thrashing guard already mitigates. Low priority.
- **#7 (Next.js lockfile warning):** Cosmetic, deferred.
- **#8 (Kraken WS reconnect):** Normal behavior, auto-recovered in 0.9s.

---

## Verification

- `cargo clippy -- -D warnings` — zero warnings ✅
- `cargo test` — 264/264 pass ✅
- **Law 4 grep `reconciliation_removed`:** 5 sites wired ✅
  - 1 declaration (line ~371)
  - 2 inserts (startup clear line ~394, external close line ~4046)
  - 2 checks (revert path 1 line ~3605, revert path 2 line ~3654)

---

## Perfection Loop

- **RED:** 8 issues identified. #1-#2 critical (block all trading). #3-#5 secondary. #6-#8 non-actionable.
- **GREEN:** 5 fixes implemented in `src/engine.rs`. Total ~64 lines added.
- **AUDIT:** cargo clippy (0 warnings), cargo test (264 pass), Law 4 grep (all sites reachability confirmed).
- **CHANGE DELTA:** <1% of codebase (64 of ~6780 lines).
- **COMPLETE:** All fixes verified. FID closed.

---

## Change Log

| File | Change | Lines |
|------|--------|-------|
| `src/engine.rs` | Added `HashSet` to imports | 1 |
| `src/engine.rs` | Fix 1: peak_equity + drawdown reset | +6 |
| `src/engine.rs` | Fix 2: reconciliation_removed HashSet (`decl + 2 inserts + 2 checks`) | +15 |
| `src/engine.rs` | Fix 3: Batch deduplication by pair name | +25 |
| `src/engine.rs` | Fix 4: Wallet address masking | +6 |
| `src/engine.rs` | Fix 5: Batch size validation logging | +12 |
| **Total** | | **~64** |

---

## Lessons Learned

1. **`peak_equity` is a derivative of position data.** When positions are externally modified (reconciliation removed), all derivative state must be re-derived. The circuit breaker's peak_equity was computed before reconciliation but never updated after.

2. **Multiple removal paths need a shared guard.** The startup clear and per-cycle external close are independent code paths, but both must feed the same guard set that prevents resurrection. A single `HashSet<String>` is the correct pattern.

3. **Batch LLM responses are unreliable.** The model can return duplicates, truncate, or hallucinate pairs. The parser must validate: deduplicate, check bounds, and surface discrepancies via logging.

4. **Law 12 applies to wallet addresses.** Even though the address is derived from a private key, it's still sensitive — it identifies the on-chain identity. Mask in logs.

5. **Law 4 (reachability) caught a latent wiring bug.** Before this FID, `update_last_close` was called from the WS drain loop but never from the external close detection path. Grep confirmed 0 results for the cross-path usage.
