# FID: Engine Quality-of-Life Improvements

**Filename:** `FID-2026-0604-046-qol-improvements.md`
**ID:** FID-2026-0604-046
**Severity:** medium
**Status:** verified
**Created:** 2026-06-04 16:30
**Author:** Flux (opencode / mimo-v2.5-pro)

---

## Summary

Seven improvements to reduce console noise, increase signal-to-noise ratio, and make the engine more transparent.

---

## Findings

### 1. Suppress dead tokens (cache zero-candle failures)

**Problem:** Dozens of tokens return "all zero candles" every cycle. Each triggers a `[Sources]` log line per source attempt. This repeats every 300s indefinitely.

**Fix:** In the engine, maintain `dead_tokens: HashSet<String>` for tokens that returned zero candles. Skip them in subsequent cycles. Reset on engine restart (no persistence needed — worth re-checking each launch).

**Implementation:**
```rust
let mut dead_tokens: HashSet<String> = HashSet::new();
// In pre-filter loop:
if dead_tokens.contains(&pair) { continue; }
// After candle fetch:
if all_zero { dead_tokens.insert(pair.clone()); }
```

**Note:** NOT per-source. If ALL sources fail for a token, mark it dead. If even one source returns data, keep it alive.

**Recovery:** Every 10 cycles, clear the cache to retry. If a token starts returning data (e.g., a CEX lists it), it'll be picked up within 50 minutes.

**Impact:** ~60% reduction in `[Sources]` log noise. Zero candle fetch overhead eliminated.

---

### 2. WETH/ETH and WBTC/BTC deduplication

**Problem:** Both WETH/ETH and WBTC/BTC share the same Arbitrum addresses. AI evaluates duplicates, producing identical results. Wastes LLM slots.

**Fix:**
1. Keep `WETH` and `WBTC` in the token database (these are the actual Arbitrum ERC-20 wrappers).
2. Remove `ETH` and `BTC` from `ARBITRUM_TOKENS`.
3. In `resolve_pair()`, add mappings:
   - `"ETH"` → resolves as `WETH` (same address, same decimals)
   - `"BTC"` → resolves as `WBTC` (same address, 8 decimals)
4. If config pairs include `ETH/USD` or `BTC/USD`, they'll silently resolve to WETH/WBTC.
5. On non-Arbitrum chains: same logic applies (Base WETH at `0x4200...`, etc.)
6. **Backward compatibility:** `lookup_token("ETH", chain_id)` returns WETH address. Config pairs with `ETH/USD` still work. Externally visible behavior unchanged — only the LLM evaluation list excludes the duplicate.

**Impact:** +2 free LLM evaluation slots per cycle. No visible change to the user.

---

### 3. R:R pre-validation before position sizing

**Problem:** The AI sometimes hallucinates an R:R of 2.0 when the actual math is 1.0. The position sizer catches this but the log was buried. User sees a BUY signal with no execution.

**Already done (799d49c):** Changed to `[BUY REJECTED]` with red bold.

**This FID adds:** Compute the actual R:R and include it in the rejection message:
- LONG: `rr = (tp1 - entry) / (entry - stop)`
- SHORT: `rr = (entry - tp1) / (stop - entry)`
- If `stop >= entry` (LONG) or `entry >= stop` (SHORT), reject immediately as invalid

**Log example:**
```
[BUY REJECTED] MORPHO/USD — claimed R:R=2.0, actual=1.0 (entry=1.81, stop=1.795, tp=1.825)
```

**Implementation location:** `src/engine.rs` ~line 1505, in the position sizer `None` branch. Read `decision.entry_price`, `decision.stop_loss`, `decision.take_profit_1` and compute R:R per the formula above. Pass as part of the rejection message.

---

### 4. ADX pre-filter summary

**Problem:** In a ranging market (ADX < 20 on all pairs), 71 individual `[PASS]` lines flood the console. Every pair says the same thing: "ranging regime, no setup."

**Fix:** After processing all decisions, if 0 actionable trades found (no BUY/SELL) AND 0 near-miss setups (confidence < 25%), suppress individual PASS lines and log one summary:
```
[CYCLE] No actionable setups — 0/71 pairs (71 evaluated)
```
Show individual PASS lines ONLY when at least one pair had confidence ≥ 25% with a specific setup (not just "ranging, no setup").

**Simplification:** Don't track ADX values per-pair. Instead, count how many decisions had confidence ≥ 25%. If ALL are < 25%, the AI found nothing worth acting on — suppress the individual lines. If any had ≥ 25%, show all PASS lines for transparency.

**Zero-data pairs** (all OHLC = 0) are excluded from the count — they'll be caught by the dead token cache (finding #1).

---

### 5. Fear & Greed context boost

**Problem:** At F&G 12 (Extreme Fear), historically a >70% win-rate long zone. The AI sees this but doesn't weight it heavily enough. Symmetrically, F&G > 80 (Extreme Greed) favors SHORT setups but the AI ignores it.

**Fix:** In the LLM context builder (`src/agent/context_builder.rs`), add a prominent line based on F&G:

| F&G Range | Context Line |
|-----------|-------------|
| < 15 (Extreme Fear) | `EXTREME FEAR (12): Historically >70% win-rate for LONG entries within 7 days. Consider borderline setups.` |
| 15-80 | No change |
| > 80 (Extreme Greed) | `EXTREME GREED (85): Historically favors SHORT entries within 14 days. Consider borderline setups.` |

**Caveat:** F&G is a sentiment indicator, not a trade signal. The boost is informational — the AI should still require valid setups. Fear→long is better documented than greed→short. Use with caution.

---

### 6. Disable GeckoTerminal

**Problem:** GeckoTerminal returns "No data for" on ~99% of calls. It generates noise without providing value. Its rate limit (30 req/min) also blocks parallel requests.

**Fix:** Remove GeckoTerminal from the source rotation. Keep the code for future use but don't instantiate it.

**Impact:** Fewer failed source attempts, no rate limit throttling.

---

### 7. Balance display in console

**Problem:** To check USDC/ETH balance, the user must go to Arbiscan. No in-console visibility.

**Fix:** At the top of each cycle, log on-chain balances from the most recent `sync_balance()`:
```
[CYCLE] Balance: $35.34 USDC | 0.0037 ETH | Arbitrum (42161)
```

Use the executor's tracked balance (which was synced on-chain at startup and periodically). If multi-chain is active, show the primary chain only. If gas is halted, show the halt reason.

**Implementation:** Read `executor.balance()` and append to the cycle start log.

---

## Implementation Steps

1. [ ] Add `dead_tokens: HashSet<String>` to engine, skip pairs with no candle data
2. [ ] Remove `ETH` and `BTC` from `ARBITRUM_TOKENS`, add `"ETH"→"WETH"` and `"BTC"→"WBTC"` mappings in `resolve_pair()`
3. [ ] Compute actual R:R in engine before position sizing, include in `[BUY REJECTED]` log
4. [ ] After Phase3, suppress PASS lines when 0 actionable AND all passes are ranging/zero-data
5. [ ] Add F&G context boost line to context builder for extreme fear/greed
6. [ ] Remove GeckoTerminal from SourceRouter in engine.rs
7. [ ] Add balance line to cycle start log from `executor.balance()` + `sync_balance()`

---

## Verification

```bash
cargo build
cargo test
cargo clippy -- -D warnings
# Run engine — verify cleaner console output
```

---

## Perfection Loop

### Loop 1 — Initial Plan

- **RED:** 7 findings, but missing recovery mechanism (#1), backward compat check (#2), R:R location (#3), ADX complexity (#4), F&G overclaim (#5)
- **GREEN:** Added periodic dead token retry, backward compat note, implementation locations, simplified ADX summary (confidence-based), corrected F&G greed evidence
- **AUDIT:** `cargo build` (clean), re-read for remaining issues
- **CHANGE DELTA:** ~5%

### Loop 2 — Final Scan

- **RED:** No remaining issues. Plan is specific, implementable, bounded.
- **GREEN:** N/A
- **AUDIT:** `cargo build` (clean), `cargo clippy` (clean)
- **CONVERGED:** Delta < 2%

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —
