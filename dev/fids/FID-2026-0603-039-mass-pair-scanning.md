# FID: Mass pair scanning — hundreds of Arbitrum tokens

**Filename:** `FID-2026-0603-039-mass-pair-scanning.md`
**ID:** FID-2026-0603-039
**Severity:** critical
**Status:** created
**Created:** 2026-06-03 19:45
**Author:** Agent

---

## Summary

The engine currently evaluates 7-13 hardcoded pairs. The goal is **hundreds of pairs** — scan every Arbitrum token with liquidity and let the AI find opportunities. Arbitrum has 700K+ token contracts. We need dynamic token discovery, multi-source candles, and optimized LLM evaluation.

## Problem

- Current: 7 hardcoded pairs, all ranging, AI has no setups
- Goal: 100+ pairs covering all liquid Arbitrum tokens
- Blockscot API has full token list with volume, market cap, holders
- CoinGecko has candle data for thousands of tokens
- The AI is disciplined — it will only trade when 3+ triggers align. More pairs = more chances.

## Proposed Architecture

### Phase 1: Token Discovery

- Query Blockscot API: `GET /api/v2/tokens?type=ERC-20&sort=volume_24h&limit=200`
- Filter: volume > $1M/day, holders > 500, **verified contracts only** (https://arbiscan.io/contractsVerified = 178K contracts)
- Skip: dead charts (< $1M volume = no action), stablecoins, wrapped variants, scam tokens
- Source: https://arbiscan.io/tokens?sort=24h_volume_usd&order=desc (sorted by volume)
- Auto-generate pair list: `{symbol}/USD`
- Cache results, refresh every 4 hours

### Phase 2: Multi-Source Candles

- KrakenSource: BTC, ETH, SOL, XRP, DOGE, ADA, LINK, AVAX (high quality, 5m)
- CoinGeckoSource: everything else (broader coverage, OHLC)
- DeFiLlamaSource: DEX-native data for Arbitrum pairs
- SourceRouter: tries sources in priority order

### Phase 3: LLM Optimization

- Batch prompts: send multiple pairs in one LLM call
- Prefix caching: static rules as prefix, dynamic data as suffix
- Parallel evaluation: current JoinSet with semaphore (already works)
- Target: 100 pairs evaluated in <5 minutes

### Phase 4: Execution Safety

- GoPlus security check for all new tokens
- Spread filter (30bps) for all swaps
- Price tolerance (0.5%) for stale entries
- Correlation cap (max 1 meme position)

## Impact

- Pairs: 7 → 100+
- Cycle time: ~2 min → ~5 min
- Opportunities: 100x more setups to evaluate
- Risk: same (1% per trade, 3 max positions, 5% daily loss)
