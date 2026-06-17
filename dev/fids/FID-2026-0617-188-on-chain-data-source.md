# FID-188: Switch Data Source from Kraken CEX to On-Chain AMM

**Filename:** `FID-2026-0617-188-on-chain-data-source.md`
**ID:** FID-2026-0617-188
**Severity:** high
**Status:** created
**Created:** 2026-06-17 16:05 EST
**Author:** Vera
**Parent:** FID-184

---

## Summary

Switch the data source for DEX execution from Kraken CEX WebSocket v2 to on-chain AMM data (0x API quotes, Uniswap V3 quoter). Gemini Q7: "Halt the use of Kraken CEX data for Arbitrum trading. Query real AMM liquidity depth to prevent theoretical trades that will fail due to on-chain slippage."

---

## Problem

Engine uses Kraken WebSocket v2 + 5 other CEX/DEX sources for candle data. For Arbitrum DEX execution, this is fundamentally wrong:
- CEX price ≠ DEX price (different liquidity pools, different market makers)
- AMM slippage is not captured in CEX data
- MEV sandwich risk is invisible in CEX data
- LVR (Loss-Versus-Rebalancing) is invisible in CEX data
- 0x API quotes will return different prices than Kraken candles

---

## Proposed Solution

### Action 1: Add 0x API as primary data source for Arbitrum

**File:** `src/data/sources/zero_x_quote.rs` (new)

**Logic:** For each Arbitrum pair, query 0x API `/quote` endpoint. Use the returned price + liquidity depth as the primary candle source.

### Action 2: Add Uniswap V3 quoter for liquidity depth

**File:** `src/data/sources/uniswap_v3_quoter.rs` (new)

**Logic:** Query Uniswap V3 quoter contracts for pool reserves, depth, and slippage estimates.

### Action 3: Keep Kraken as fallback for non-DEX pairs

**File:** `src/data/sources/mod.rs`

**Logic:** For Hyperliquid (perps, no DEX equivalent) and cross-chain pairs, keep Kraken CEX data.

### Action 4: Add liquidity depth filter

**File:** `src/data/token_discovery.rs`

**Gemini Q6:** "Add a strict depth-to-spread filter... Query the 0x API or Hyperliquid L1 order book specifically for at least $50,000 of depth. If the simulated slippage exceeds 4 basis points on the quote, drop the pair from the eligible universe immediately."

---

## Verification

- 0x API quotes return real AMM prices
- Liquidity depth is captured
- Slippage estimates match actual execution
- Drop pairs with insufficient depth

---

*Vera 0.1.0 — 2026-06-17 16:05 EST — FID-188 created. On-chain data source switch. High-risk change.*
