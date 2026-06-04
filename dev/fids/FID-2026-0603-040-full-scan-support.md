# FID: Full top-100 scan support — candles, addresses, batch LLM

**Filename:** `FID-2026-0603-040-full-scan-support.md`
**ID:** FID-2026-0603-040
**Severity:** critical
**Status:** created
**Created:** 2026-06-03 20:00
**Author:** Agent

---

## Summary

Token discovery finds ~100 high-volume Arbitrum tokens, but three missing pieces prevent the engine from actually evaluating them: (1) no candle data for tokens Kraken doesn't list, (2) no Arbitrum addresses for discovered tokens, (3) 100+ LLM calls per cycle is too slow.

## Issues Found

| # | Issue | Impact | Fix |
|---|-------|--------|-----|
| 1 | **No candle data** for non-Kraken tokens | AI rejects with "OHLC corrupted" | Wire CoinGeckoSource into engine |
| 2 | **No Arbitrum addresses** for discovered tokens | 0x API rejects with "Invalid address" | Auto-add discovered addresses to token DB |
| 3 | **Cycle time too slow** | 100 pairs × 30-60s = 50-100 min/cycle | Batch LLM prompts or reduce pair count |
| 4 | **No spread pre-filter** for discovered tokens | Wastes LLM calls on illiquid pairs | Check 0x liquidity before LLM evaluation |

## Proposed Solution

### Phase 1: Wire CoinGecko candles

- `SourceRouter` already exists in `src/data/sources/`
- `CoinGeckoSource` already implemented
- Need to: use SourceRouter in engine instead of direct Kraken calls
- For each pair, try Kraken first → fallback to CoinGecko

### Phase 2: Auto-add discovered addresses

- `token_discovery.rs` discovers tokens with addresses from Blockscot
- Need to: update the static `ARBITRUM_TOKENS` array at runtime (use a `HashMap` instead)
- Or: pass discovered addresses directly to `resolve_pair()`

### Phase 3: Optimize cycle time

- Option A: Batch prompts — send 5-10 pairs per LLM call (requires prompt restructuring)
- Option B: Reduce to top 30 tokens (still 3x current coverage)
- Option C: Parallel evaluation with higher semaphore limit (already at 10)

### Phase 4: Spread pre-filter

- Before LLM evaluation, check 0x API spread for each discovered pair
- Skip pairs with spread > 30bps
- Saves LLM calls on illiquid tokens

## Verification

- `cargo build --release` — zero errors
- `cargo test` — 188+ tests pass
- Manual test: restart engine, verify discovered pairs have candle data
