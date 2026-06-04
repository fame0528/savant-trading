# Multi-Chain 0x System — What We Need

**FID:** FID-2026-0604-045
**Date:** 2026-06-04
**Status:** Verified, ready to implement

---

## Phase 1: Expand Arbitrum Tokens (zero-risk)

### What We Need

| Item | Status | Notes |
|------|--------|-------|
| CoinGecko API key | ✅ Have it | `COINGECKO_API_KEY=zCeVfHxpp1GEPWT8bD8X8L8j` |
| CoinGecko `/coins/list` endpoint | ✅ Works | Already used to build current 198 tokens |
| DeFiLlama API (optional) | 🆕 Free, no key | `https://api.llama.fi/protocols` — TVL data for filtering |
| Blockscout API (optional) | 🆕 Free, no key | Contract verification check |
| Token filter criteria | ✅ Defined | $1M+ volume, verified address, not stablecoin/xStock |

### What We Build

- Script to pull 500+ Arbitrum tokens from CoinGecko
- Filter pipeline: volume → address verification → stablecoin exclusion
- Expanded `ARBITRUM_TOKENS` const array in `mod.rs`
- Runtime loading via `extend_token_db()` for tokens beyond top 100

---

## Phase 2: Chain-Aware Token Database

### What We Need

| Item | Status | Notes |
|------|--------|-------|
| Base RPC endpoint | 🆕 Free | `https://mainnet.base.org` (public, rate-limited) |
| Optimism RPC endpoint | 🆕 Free | `https://mainnet.optimism.io` (public, rate-limited) |
| BSC RPC endpoint | 🆕 Free | `https://bsc-dataseed.binance.org` (public, rate-limited) |
| Alchemy/Infura key (recommended) | ❓ Check | Multi-chain RPC, one key, all chains, better rate limits |
| Base token list | 🆕 Need to pull | CoinGecko with `platform_ids=base` |
| OP token list | 🆕 Need to pull | CoinGecko with `platform_ids=optimistic-ethereum` |
| BSC token list | 🆕 Need to pull | CoinGecko with `platform_ids=binance-smart-chain` |
| Chain-specific USDC addresses | ✅ Have them | See FID-045 table |
| `ChainToken` struct | 🆕 Need to build | `{ chain_id, symbol, address, decimals }` |
| `ChainConfig` struct | 🆕 Need to build | `{ chain_id, rpc_url, native_token, min_gas, slippage_pct }` |
| `Position.chain_id` field | 🆕 Need to add | Track which chain each position lives on |

### What We Build

- `CHAIN_TOKENS` array: top 100 per chain (static const)
- `[chains]` config section in `config/default.toml`
- Chain-aware `resolve_pair(pair, side, chain_id)`
- Chain-aware `lookup_token(symbol, chain_id)`
- Per-chain USDC address mapping

---

## Phase 3: Multi-Chain Execution

### What We Need

| Item | Status | Notes |
|------|--------|-------|
| Per-chain RPC clients | 🆕 Need to build | `HashMap<u64, reqwest::Client>` in DexTrader |
| Per-chain gas monitoring | 🆕 Need to build | `sync_balance(chain_id)` per chain |
| Per-chain nonce | ✅ Already works | `get_nonce()` queries per RPC |
| Per-chain Permit2 approval | ✅ Already works | Same Permit2 address, different RPC |
| Parallel chain scanning | 🆕 Need to build | `tokio::join!(scan_chain(42161), scan_chain(8453), ...)` |
| Engine timeout increase | 🆕 May need | 60s might be tight for 4 chains × 100 tokens |

### What We Build

- Multi-chain `DexTrader` with per-chain RPC clients
- `sync_balance(chain_id)` — per-chain balance + gas check
- `ensure_permit2_approval(token_addr, chain_id)` — per-chain approval
- `sign_and_send(to, data, value, gas, chain_id)` — per-chain signing
- Parallel engine scanning with `tokio::join!`

---

## Phase 4: Gasless API

### What We Need

| Item | Status | Notes |
|------|--------|-------|
| 0x API key | ✅ Have it | Works for Gasless on all chains |
| Gasless API spec | ✅ Have it | `docs/llms-full.md` line 5700+ |
| Gasless response parsing | 🆕 Need to build | Different format: `trade` object instead of `transaction` |
| Gasless signing flow | 🆕 Need to build | Similar to Permit2 but different struct |

### What We Build

- `/gasless/quote` endpoint in `zero_x.rs`
- Gasless trade signing
- No gas management needed on gasless-enabled chains

---

## Phase 5: Cross-Chain API

### What We Need

| Item | Status | Notes |
|------|--------|-------|
| Cross-Chain API spec | ✅ Have it | `docs/llms-full.md` line 1444-1474 |
| Bridge status polling | 🆕 Need to build | `/status` endpoint polling with backoff |
| Recovery handling | 🆕 Need to build | Bridge failure → manual recovery tx |
| Solana support (future) | 🆕 Different model | Non-EVM, different signing |

### What We Build

- `/swap/quote` with `bridge` param
- Cross-chain status monitoring
- Bridge failure recovery

---

## The Candle Situation

### Current State

Our candle sources are **chain-agnostic** — they return price data, not chain-specific data:

| Source | Coverage | Rate Limit | Chain Support |
|--------|----------|------------|---------------|
| Kraken | ~46 tokens (top majors) | 15 req/s | Any token Kraken lists |
| OKX | Broad (200+ tokens) | 40 req/2s | Any token OKX lists |
| KuCoin | Massive (500+ tokens) | 300 req/10s | Any token KuCoin lists |
| Gate.io | Obscure tokens | 300 req/s | Any token Gate lists |
| CryptoCompare | 100K calls/month | 10 req/s | Any token with a symbol |
| CoinGecko | Rate-limited | 10K/month | Any token with a CoinGecko ID |

### The Problem

These sources return **spot prices**, not chain-specific prices. A token on Arbitrum and the same token on Base have the **same price** (arbitrage keeps them aligned). So candle data works across chains — we don't need chain-specific candles.

**BUT:** These sources only list tokens they support. A Base-native token (e.g., a Base-only meme coin) won't have candle data on Kraken/OKX/KuCoin.

### The Solution

| Token Type | Candle Source | Works? |
|-----------|---------------|--------|
| Major tokens (BTC, ETH, LINK, etc.) | Kraken, OKX, KuCoin | ✅ Yes — same price on all chains |
| Mid-cap tokens (AAVE, UNI, ARB, etc.) | OKX, KuCoin, CryptoCompare | ✅ Yes — listed on CEXs |
| Chain-native meme coins | ❌ No CEX listing | ❌ No candle data |
| New/Base-only tokens | ❌ No CEX listing | ❌ No candle data |

### What We Do

1. **Phase 1-3:** Only trade tokens that have candle coverage. This means we trade the SAME tokens but on the CHEAPEST chain (Arbitrum gas = $0.01, Ethereum gas = $5). Multi-chain gives us better execution, not more tokens.

2. **Phase 4+:** Add DeFiLlama price API as fallback for chain-native tokens. DeFiLlama has spot prices for every token on every chain. No candles, but we can use:
   - Spot price + 24h change as "micro-candle"
   - On-chain DEX data (reserves, volume) for indicators
   - CoinGecko `simple/price` for any token with a CoinGecko ID

3. **Phase 5:** The AI doesn't strictly need OHLCV candles. It can make decisions with:
   - Current spot price
   - 24h volume
   - Price change %
   - On-chain metrics (TVL, holders, transactions)
   - This is how most DEX trading bots work

### Summary

| Phase | Token Universe | Candle Coverage | Chain Benefit |
|-------|---------------|----------------|---------------|
| 1 | 500 Arbitrum tokens | ✅ Full (Kraken/OKX/KuCoin) | More tokens on same chain |
| 2 | 100 tokens × 4 chains | ✅ Full (same tokens, different chains) | Cheapest gas per trade |
| 3 | 100 tokens × 4 chains | ✅ Full | Best liquidity per trade |
| 4 | 100 tokens × 4 chains + gasless | ✅ Full | No gas management |
| 5 | Cross-chain + chain-native | ⚠️ Partial (DeFiLlama fallback) | Bridge + trade in one tx |

---

## Priority Order

1. **Phase 1** (expand Arbitrum tokens) — zero risk, immediate value
2. **Phase 2** (chain-aware DB) — foundation for everything else
3. **Phase 3** (multi-chain execution) — the real unlock
4. **Phase 4** (gasless) — operational simplification
5. **Phase 5** (cross-chain) — advanced, complex

---

## Quick Reference

### 0x API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/swap/permit2/price` | GET | Indicative price (no calldata) |
| `/swap/permit2/quote` | GET | Firm quote (with calldata + Permit2) |
| `/gasless/price` | GET | Gasless indicative price |
| `/gasless/quote` | GET | Gasless firm quote |
| `/swap/quote` | GET | Cross-chain quote (with `bridge` param) |
| `/swap/status` | GET | Cross-chain bridge status |

### Chain IDs

| Chain | ID | Gas Token | USDC Decimals |
|-------|-----|-----------|---------------|
| Arbitrum | 42161 | ETH | 6 |
| Base | 8453 | ETH | 6 |
| Optimism | 10 | ETH | 6 |
| BSC | 56 | BNB | **18** |
| Polygon | 137 | MATIC | 6 |
| Ethereum | 1 | ETH | 6 |
| Avalanche | 43114 | AVAX | 6 |

### Constants (all EVM chains)

| Constant | Value |
|----------|-------|
| Permit2 | `0x000000000022d473030f116ddee9f6b43ac78ba3` |
| Exchange Proxy | `0xdef1c0ded9bec7f1a1670819833240f027b25eff` (verify per chain) |
| 0x API Base | `https://api.0x.org` |
| Headers | `0x-api-key`, `0x-version: v2` |
