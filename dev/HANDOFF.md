# HANDOFF.md — Session Compaction Document

**Generated:** 2026-06-06 04:05 EST
**Version:** 0.9.1
**Branch:** main (latest commit: ed0a002)

---

## Project State

**Savant Trading** is an autonomous DEX trading engine running on Arbitrum via 0x API. Currently holding 2 open long positions (ETH + LINK) with trailing stops active. ~$26 total portfolio value.

### Current Holdings (on-chain, wallet 0x543CA...11fBC)
- **USDC**: ~$0.26 (cash)
- **WETH**: 0.01485 (~$23.46) — long from $1,549.66
- **LINK**: 0.322 (~$2.37) — long from $7.19
- **Native ETH**: ~0.001178 (~$1.88) — gas only

### Active Positions
| Pair | Side | Entry | Current | SL | TP1 | Qty |
|------|------|-------|---------|-----|-----|-----|
| ETH/USD | Long | 1,549.66 | ~1,579 | 1,346.97 | 1,704.63 | 0.0149 |
| LINK/USD | Long | 7.19 | ~7.35 | 6.277 | 7.909 | 0.322 |

---

## What Was Done This Session

### Critical Changes
1. **soul.md v2.0** — Rewrote agent identity from conservative portfolio manager to aggressive day trader. Capital velocity over preservation. 5-8x leverage. Liquidation cascade trading as primary strategy.
2. **Batch LLM Prompting** — All pairs evaluated in SINGLE LLM call instead of N parallel calls. 80-90% API cost reduction. Falls back to per-pair if batch parse fails.
3. **YouTube Knowledge Integration** — 20 new knowledge files (747 units) from crypto traders added. Source-based tier multiplier: YouTube sources 2.0x boost, institutional sources 0.8x. Total: 30 files, 3,706 units.
4. **Knowledge Tiered Selection** — MAX_SELECTED_UNITS reduced 20→12. MMR scoring now considers source type.
5. **Dashboard Fixes** — Terminal 12h time, Profit KPI (locked+open), Size/Risk labels, 6th KPI (Positions), equity curve seeding, Kraken balance re-sync.
6. **ECHO Protocol Compliance** — FID-055 (equity source of truth), FID-053 (sandbox retry), FID-051 (dashboard controller), FID-054 (PaperTrader→PortfolioManager rename), FID-056 (LLM cost optimization), FID-059 (knowledge overhaul). All closed and archived.
7. **LLM Cost Optimization (FID-056)** — 6 measures: skip when deployed, candle hash cache, max_tokens 16384→8192, knowledge_token_budget 20000→12000, smart pre-scoring (RSI/ADX/EMA), skip if no new candle.

### Research Completed
- **Gemini Deep Research**: LLM Crypto Trading Growth Strategy (full analysis)
- **Gemini Deep Research**: Liquidation Cascade Detection Data Sources (APIs, algorithms)
- **Knowledge Base Audit**: 171 books + 20 YouTube interviews = 3,706 units

### FIDs Created/Completed
| FID | Title | Status |
|-----|-------|--------|
| 051 | Dashboard Controller | closed+archived |
| 053 | Sandbox Robustness | closed+archived |
| 054 | PaperTrader→PortfolioManager | closed+archived |
| 055 | Equity Source of Truth | closed+archived |
| 056 | LLM Cost Optimization | closed+archived |
| 057 | Liquidation Cascade Strategy | created (open) |
| 058 | GMX Sidecar POC | abandoned (Python killed perf) |
| 059 | Knowledge Base Overhaul | closed+archived |
| 060 | GMX Native Rust Execution | created (open) |

---

## Open Work (Next Session)

### Priority 1: FID-060 — GMX Native Rust Execution
Build GMX V2 execution natively in Rust using existing Arbitrum infrastructure:
- Oracle price fetcher (REST: `https://arbitrum-api.gmxinfra.io/prices`)
- ExchangeRouter contract calls (createIncreaseOrder, createDecreaseOrder)
- Position query and close
- Integration with engine as new execution backend

**Key contracts on Arbitrum:**
- ExchangeRouter: `0x7C68C7866A64FA2160F78EEeE18b26d8c1B7e6d1`
- DataStore: `0xFD70de6b91282D8017aA4E741e9Ae325CAb992d8`
- USDC: `0xaf88d065e77c8cC2239327C5EDb3A432268e5831`

**Why native Rust:** Python web3 killed performance (2-4GB RAM). TypeScript SDK broken. Our Rust engine already has EIP-1559 signing, Arbitrum RPC, wallet management, gas management.

### Priority 2: FID-057 — Liquidation Cascade Detection
Monitor for liquidation cascades 24/7, fire LLM only when cascade conditions met:
- Bybit WebSocket for real-time OI (tick-by-tick)
- Binance WebSocket `@forceOrder` for liquidation events
- Hyperliquid WebSocket for execution venue data
- Cascade trigger: OI drop >15-20% + price velocity >5% in 15min
- Exhaustion: b-shape volume profile + price stabilization + funding flip
- Entry: V-recovery confirmation, target next liquidation cluster

### Priority 3: Test batch prompting + new knowledge
Run the engine and verify:
- Batch LLM call returns valid JSON array
- YouTube knowledge units are being selected (check logs)
- Tier multiplier is working (YouTube sources ranked higher)
- Agent behavior changes (more aggressive, faster entries)

---

## Key Architecture Notes

- **Batch prompting:** All pairs in 1 LLM call. JSON array response. Falls back to per-pair on parse failure.
- **Knowledge tiers:** YouTube sources 2.0x, institutional 0.8x in MMR scoring. MAX_SELECTED_UNITS = 12.
- **soul.md v2.0:** Aggressive day trader. Capital velocity. Leverage. Cascades. Knowledge as decision engine.
- **PortfolioManager:** Renamed from PaperTrader. `refresh_equity()` is single source of truth for equity/P&L.
- **Dashboard:** 12h time, Profit KPI, Size/Risk labels, 6th KPI (Positions), ErrorBoundary on Equity+Terminal.

---

## CLI Reference

```bash
# Production start (engine + API + dashboard)
cargo run --release serve

# Engine + API only
cargo run --release

# Dry run (one cycle, no execution)
cargo run -- --dry-run

# Sandbox testing
cargo run --release -- --test --sandbox

# Build
cargo build --release
cargo test
cargo clippy -- -D warnings
```

---

## Wallet

- **Address**: 0x543CA0434B84aD38c858D2D178D2082521711fBC
- **Chain**: Arbitrum (42161)
- **Backend**: 0x API v2 with Permit2 signing
- **Key**: WALLET_PRIVATE_KEY in .env (never committed)

## Environment

- **OS**: Windows (PowerShell)
- **Rust**: 1.91+ (edition 2021)
- **Node**: Next.js 16.2.7
- **Model**: xiaomi/mimo-v2.5-pro via OpenRouter
- **Gas**: ~$0.01/swap on Arbitrum
