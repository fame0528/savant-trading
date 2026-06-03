# FID: Arbitrum token availability + multi-source candles + console cleanup

**Filename:** `FID-2026-0603-038-arbitrum-tokens-and-candle-sources.md`
**ID:** FID-2026-0603-038
**Severity:** critical
**Status:** analyzed
**Created:** 2026-06-03 18:30
**Author:** Agent

---

## Summary

Three related issues blocking the engine from trading:
1. **XRP, SOL, ADA, AVAX can't be swapped** — they're not ERC-20 tokens on Arbitrum. 0x API rejects them with "Invalid ethereum address".
2. **Candle source hardcoded to Kraken** — tokens without Kraken data (SHIB, TURBO) get zero candles, wasting LLM cycles.
3. **Console logging has 8 remaining issues** — GoPlus spam for core assets, double brackets, wrong module names.

## Issues Found

### 1. Arbitrum Token Availability (CRITICAL)

The 0x API can only swap ERC-20 tokens on Arbitrum. Our token database has addresses for:
- ✅ ETH, USDC, LINK, DOGE, PEPE — have Arbitrum addresses
- ❌ XRP, SOL, ADA, AVAX, SHIB, FLOKI, TURBO, MOG — no Arbitrum addresses

The "enterprise token resolution" (passing symbols to 0x API) doesn't work for all tokens. XRP was rejected with:
```json
{"name":"INPUT_INVALID","data":{"details":[{"field":"buyToken","reason":"Invalid ethereum address"}]}}
```

**Fix:** Filter pairs to only include tokens with known Arbitrum addresses. Add wrapped versions of missing tokens.

### 2. Candle Source Hardcoded (HIGH)

`KrakenClient::get_ohlc()` is the only candle source. Tokens without Kraken data (SHIB, TURBO) get zero candles, causing the AI to reject them with "OHLC data corrupted".

**Fix:** Abstract candle source behind a `CandleSource` trait. Add CoinGecko and DeFiLlama as fallbacks.

### 3. Console Logging (MEDIUM)

From live log analysis:
- GoPlus warns 13 times per cycle for core assets that don't need checks
- Double brackets: `[[BTC/USD]]` should be `[BTC/USD]`
- Module names wrong: `FundingRates` → `Funding Rates`, `Onchain` → `On Chain`
- `[LLM]` all dark grey — no alternating colors

**Fix:** Already implemented GoPlus core asset skip and bracket fix. Module names and LLM colors still pending.

## Current State

| Pair | Kraken Candles | Arbitrum Swap | Status |
|------|---------------|---------------|--------|
| BTC/USD | ✅ | ✅ | Working |
| ETH/USD | ✅ | ✅ | Working |
| SOL/USD | ✅ | ❌ | Can analyze, can't swap |
| XRP/USD | ✅ | ❌ | Can analyze, can't swap |
| DOGE/USD | ✅ | ✅ | Working |
| ADA/USD | ✅ | ❌ | Can analyze, can't swap |
| LINK/USD | ✅ | ✅ | Working |
| AVAX/USD | ✅ | ❌ | Can analyze, can't swap |
| PEPE/USD | ✅ | ✅ | Working |
| SHIB/USD | ❌ | ❌ | Neither works |
| FLOKI/USD | ✅ | ❌ | Can analyze, can't swap |
| TURBO/USD | ❌ | ❌ | Neither works |
| MOG/USD | ✅ | ❌ | Can analyze, can't swap |

**Only 5 pairs can actually trade:** BTC, ETH, DOGE, LINK, PEPE

## Proposed Solution

### Phase 1: Fix token availability

1. Remove pairs that can't be swapped (XRP, SOL, ADA, AVAX, SHIB, FLOKI, TURBO, MOG)
2. Add Arbitrum-wrapped versions if available (e.g., wrapped SOL on Arbitrum)
3. Keep only pairs with known Arbitrum addresses: BTC, ETH, DOGE, LINK, PEPE + USDC pairs

### Phase 2: Multi-source candles

4. Create `CandleSource` trait abstraction
5. Add CoinGecko candle source (free API, thousands of tokens)
6. Add DeFiLlama candle source (DEX-native, Arbitrum pairs)
7. Create SourceRouter that tries sources in priority order

### Phase 3: Console cleanup

8. Fix module name formatting (FundingRates → Funding Rates)
9. Fix LLM color alternation

## Verification

- `cargo build --release` — zero errors
- `cargo test` — 187+ tests pass
- Manual test: restart engine, verify only swappable pairs are evaluated
