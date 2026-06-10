# SAVANT TRADING v0.12.7

<!-- markdownlint-disable MD033 -->
<div align="center">

<img src="img/banner.png" alt="Savant Trading — AI-Native Autonomous DEX Trading Engine" width="2752" height="1536" />

**AI-Native Autonomous DEX Trading Engine**

No KYC. No CEX. Arbitrum on-chain swaps via 0x API — powered by 6,676+ knowledge units from 168 source books and 548 curated sources.

**Model-agnostic:** Any OpenAI-compatible LLM via [OpenRouter](https://openrouter.ai/). Default: [Owl Alpha](https://openrouter.ai/openrouter/owl-alpha) (free, 1M context, 2.25T weekly tokens). Previously tested with MiMo v2.5 Pro.

[![Rust](https://img.shields.io/badge/Rust-2021-%23000000?style=flat-square&logo=rust&logoColor=%2300fbff)](https://www.rust-lang.org/)[![0x](https://img.shields.io/badge/0x-DEX-%23000000?style=flat-square&logo=ethereum&logoColor=%2300fbff)](https://0x.org/)[![Arbitrum](https://img.shields.io/badge/Arbitrum-L2-%23000000?style=flat-square&logo=arbitrum&logoColor=%2300fbff)](https://arbitrum.io/)[![OpenRouter](https://img.shields.io/badge/OpenRouter-LLM-%23000000?style=flat-square&logo=openai&logoColor=%2300fbff)](https://openrouter.ai/)[![Version](https://img.shields.io/badge/Version-0.12.7-%23000000?style=flat-square&logo=semver&logoColor=%2300fbff)](https://github.com/fame0528/savant-trading/releases)[![License](https://img.shields.io/badge/License-MIT-%23000000?style=flat-square&logo=github&logoColor=%2300fbff)](LICENSE)

</div>

---

## Overview

Savant Trading is an autonomous on-chain trading engine. It evaluates 200+ tokens per cycle across 6 candle sources, runs each through an LLM agent with full market context, and executes BUY decisions directly on Arbitrum via 0x API — no centralized exchange, no KYC.

Traditional algorithmic engines use hardcoded strategies (momentum, RSI crossovers). Savant inverts this — an LLM agent receives candles, indicators, sentiment, on-chain data, and macro context, then makes trading decisions using knowledge extracted from 150+ books and world-class trader transcripts.

**Kraken WebSocket** is used for real-time candle data. All execution is DEX-only via 0x API on Arbitrum.

### How It Works

```
Candle Sources (6) ────┐
On-Chain Data ─────────┤
Sentiment / Macro ─────┤──→ LLM Agent ──→ TradeDecision (JSON)
Knowledge Base ────────┤                       │
(2,959 units)          │              ┌────────┘
                       │              ▼
                  ┌────┘         BUY Signal?
                  │              │
                  │    ┌─────────┴─────────┐
                  │    ▼                   ▼
                  │  PASS              EXECUTE
                  │  (skip)        0x API on Arbitrum
                  │                ├─ Permit2 signing
                  │                ├─ ERC-20 approve
                  │                ├─ eth_call dry-run
                  │                └─ broadcast + verify
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **DEX-first, no KYC** | 0x API on Arbitrum. Swap any token on-chain. No exchange account needed. |
| **AI is the brain** | The LLM reasons across all context — candles, indicators, sentiment, on-chain data. No hardcoded strategies. |
| **Multi-chain ready** | 0x supports 20+ EVM chains. Currently running on Arbitrum; Base, Optimism, BSC supported in code. |
| **Model-agnostic** | Any OpenAI-compatible model works. owl-alpha (free) is currently active. The provider layer supports switching. |
| **5-layer system prompt** | Base identity → Risk constraints → Strategy knowledge → Transcript knowledge → Output format. |
| **6 candle sources** | Kraken, OKX, KuCoin, Gate.io, CryptoCompare, CoinGecko. Automatic fallback with all-zero rejection. |
| **3 autonomy levels** | Suggest (log only), Confirm (human-in-the-loop), Autonomous (full auto). |

---

## Features

### DEX Execution (Primary)
- **0x API v2 on Arbitrum** — No KYC. Permit2 EIP-712 signing, ERC-20 approve(max), eth_call dry-run, receipt verification
- **201 Arbitrum Tokens** — CoinGecko-verified addresses for all high-volume tokens
- **Multi-chain support** — Code-ready for Base, Optimism, BSC, Polygon, and 15+ other chains
- **Gasless API** — 0x pays gas upfront, deducted from swap output (no ETH needed)
- **Cross-Chain API** — Bridge tokens across chains in one transaction
- **Permit2 signing** — Correct `calldata || sig_len (32 bytes) || sig (65 bytes)` format
- **Fallback backend** — 0x primary, 1inch secondary on failure
- **Spread filtering** — Rejects trades with >30bps spread
- **Security checks** — GoPlus API for honeypot/tax detection on meme tokens
- **xStock filter** — SPYX, QQQX, GLDX, CRCLX filtered (require 0x opt-in)

### Market Data
- **6 active candle sources** — Kraken, OKX, KuCoin, Gate.io, CryptoCompare, CoinGecko
- **Automatic source rotation** — Falls back through sources on failure, rejects all-zero responses
- **Multi-timeframe** — 5m, 1H, 4H, 1D candles per pair
- **Indicators** — EMA, RSI, ATR, ADX, VWAP, Bollinger Bands, Volume Profile
- **WebSocket** — Kraken WebSocket v2 for real-time price updates

### AI Agent
- **LLM-agnostic** — Any OpenAI-compatible model via OpenRouter. Currently using owl-alpha (free)
- **SSE streaming** — Real-time response streaming, no timeout risk during long reasoning
- **Structured output** — JSON with entry, stop, 3 take-profit levels, confidence, reasoning
- **Knowledge base** — 2,959 units across 10 JSON files, MMR selection with utility scoring
- **Closed-loop training** — Semantic consolidation, anti-pattern detection, GEPA mutation
- **Persistent memory** — SQLite WAL episodic memory, Brier score calibration, CUSUM edge decay

### Risk & Safety
- **Circuit breakers** — Independent of AI, cannot be overridden: daily loss limit, drawdown kill switch, max positions
- **Position sizing** — Dynamic risk per trade, ATR-based sizing, minimum order value ($1)
- **Portfolio heat** — Total risk exposure tracking, blocks trades at 40% of equity
- **Price tolerance** — Rejects trades if price drifts >0.5% during LLM evaluation
- **eth_call dry-run** — Simulates transaction on-chain before broadcasting
- **Receipt verification** — Confirms swap succeeded on-chain before recording position

### Infrastructure
- **Paper trading** — Full simulation for testing strategies
- **Backtesting** — Historical validation with walk-forward optimization
- **REST API** — 16 endpoints at `http://localhost:8080/api/`
- **Next.js Dashboard** — Real-time dashboard at `http://localhost:3000`
- **Enterprise console** — Color-coded output: `[Savant Trading] [TIME] [ACTION] [RESULT]`
- **Kraken CEX** — REST + WebSocket integration (secondary, available as fallback)

### Sandbox & Model Testing
- **60 curated scenarios** — 11 categories: Trend Bull/Bear, Range, Volatility, Catalyst, Microstructure, Session, Correlation, Sentiment, On-Chain, Edge Cases
- **3-tier grading rubric** — Binary compliance, R:R scoring, reasoning quality
- **GARCH(1,1) OHLCV generator** — Synthetic candle data with configurable trend, volatility regime, market events
- **Order book simulator** — Bid/ask depth, imbalance calculation, slippage simulation
- **GEPA feedback loop** — Failure analysis, SOUL.md mutation proposals
- **Model comparison** — `run-model-tests.ps1` runs multiple models against sandbox with timeout, generates comparison table
- **Training pipeline** — `--test --train` loops until Brier score converges, captures episodes, generates lessons
- **Historical training** — `--historical` trains on real Kraken candle data

---

## Knowledge Base

**6,676+ structured knowledge units** from 168 source books (128 unique titles) and 548 curated sources. The knowledge base is the agent's brain — it contains everything from candlestick patterns to on-chain analytics, trading psychology to DeFi mechanics.

### Architecture

```
Primary (active, loaded at runtime):  2,959 units across 10 JSON files
Backup (supplementary):               3,717 units across 33 JSON files
Source books (parsed into units):      168 files (PDF/EPUB/MOBI/TXT)
```

### Primary Knowledge (10 files, 2,959 units)

| File | Units | Domain |
|------|-------|--------|
| `knowledge_technical_analysis.json` | 506 | RSI, EMA, ADX, MACD, Bollinger, Fibonacci, divergences |
| `knowledge_psychology.json` | 319 | Cognitive biases, tilt, deliberate practice, emotional regulation |
| `knowledge_crypto_native.json` | 319 | MVRV, NUPL, SOPR, NVT, exchange flows, whale movements |
| `knowledge_risk_management.json` | 350 | Kelly Criterion, drawdown recovery, position sizing, anti-martingale |
| `knowledge_sentiment.json` | 291 | Fear & Greed, COT data, put/call ratios, VIX, sentiment extremes |
| `knowledge_execution.json` | 282 | Order types, slippage, fill optimization, rate limits, session timing |
| `knowledge_market_regimes.json` | 250 | Trending, ranging, volatile, capitulation detection, ADX thresholds |
| `knowledge_trading_systems.json` | 226 | Turtle system, Donchian channels, trend following, system rules |
| `knowledge_price_action.json` | 216 | Wyckoff phases, FVG, order blocks, candle patterns, liquidity |
| `knowledge_fundamentals.json` | 200 | Macro analysis, halving cycles, ETF flows, Mr. Market, Fisher |

### How It Works

Knowledge units are tagged with `setup_type`, `regime_subtype`, `trigger`, `indicator`, and `risk_context` for MMR (Maximum Marginal Relevance) selection. The agent's context builder selects ~20 relevant units per decision based on current market conditions. A `utility_score` tracks empirical correlation with successful trades — units that help the agent win get promoted, units that correlate with losses get suppressed.

### Source Material

The 168 source books span candlestick analysis, technical analysis, trading psychology, risk management, forex (foundational market structure for crypto), algorithmic trading, and crypto-native knowledge. Forex and stock books provide the foundation — the agent's crypto-native edge comes from on-chain analytics (MVRV, NUPL, SOPR, NVT), DeFi mechanics, funding rates, and whale tracking.

**Full inventory:** [`docs/KNOWLEDGE.md`](docs/KNOWLEDGE.md) — 710-line reference with every book, every topic, every knowledge unit ID range.

---

## Quick Start

### Prerequisites

- Rust 1.91+ ([rustup](https://rustup.rs/))
- OpenRouter API key (or any OpenAI-compatible provider)
- 0x API key ([free at dashboard.0x.org](https://dashboard.0x.org/))
- An Arbitrum wallet with USDC + ETH for gas

### Setup

```bash
git clone https://github.com/fame0528/savant-trading.git
cd savant-trading

# Copy environment template and fill in your keys
cp .env.example .env

# Build
cargo build --release

# Run DEX trading on Arbitrum
cargo run
```

### Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
# OpenRouter API key (required for AI decisions)
OPENROUTER_API_KEY=your_key_here

# 0x API key (required for DEX trading)
ZEROEX_API_KEY=your_key

# Wallet private key (required for DEX trading — 0x-prefixed hex)
WALLET_PRIVATE_KEY=your_private_key

# Kraken API keys (optional — only if using Kraken CEX backend)
KRAKEN_API_KEY=your_key
KRAKEN_API_SECRET=your_secret
```

---

## Configuration

All settings in `config/default.toml`:

```toml
[exchange]
name = "kraken"                    # Candle source (Kraken WebSocket)
backend = "0x"                     # Execution: "0x" (DEX), "1inch" (DEX), "kraken" (CEX)

[exchange.dex]
chain_id = 42161                   # Arbitrum
rpc_url = "https://arb1.arbitrum.io/rpc"
slippage_pct = 0.005

[trading]
pairs = [
    "ETH/USD", "BTC/USD", "ARB/USD",
    "LINK/USD", "UNI/USD", "AAVE/USD", "PEPE/USD",
    "PENDLE/USD", "COMP/USD", "LDO/USD",
]
full_deploy = true
timeframe = "5m"
timeframes = ["5m", "1h", "4h", "1d"]
starting_balance = 50.0

[risk]
max_risk_per_trade = 0.20          # 20% per trade (dynamic tiers scale up at low balance)
dynamic_risk_tiers = [
    { balance = 500.0, risk_pct = 1.00 },    # <$500: 100% deploy
    { balance = 5000.0, risk_pct = 0.10 },
    { balance = 50000.0, risk_pct = 0.05 },
    { balance = 999999.0, risk_pct = 0.02 },
]
max_daily_loss = 0.05              # 5% daily halt
max_drawdown = 0.10                # 10% kill switch
min_daily_loss_usd = 5.0           # Dollar floor — prevents false halts at tiny balances
min_drawdown_usd = 10.0
max_positions = 3
min_rr_ratio = 1.5
min_rr_ratio_low_balance = 1.2     # Relaxed R:R at <$50 balance

[ai]
provider = "openrouter"
model = "openrouter/owl-alpha"     # Any OpenAI-compatible model
autonomy_level = 3
temperature = 0.6
max_tokens = 16384
```

### Multi-Chain Configuration

```toml
[chains.arbitrum]
chain_id = 42161
rpc_url = "https://arb1.arbitrum.io/rpc"
enabled = true

[chains.ethereum]
chain_id = 1
rpc_url = "https://eth.llamarpc.com"
enabled = true

[chains.arbitrum]
chain_id = 42161
rpc_url = "https://arb1.arbitrum.io/rpc"
enabled = true

[chains.base]
chain_id = 8453
rpc_url = "https://mainnet.base.org"
enabled = true

[chains.optimism]
chain_id = 10
rpc_url = "https://mainnet.optimism.io"
enabled = true
```

---

## CLI Commands

```bash
# Engine + API + Dashboard (single command — recommended)
cargo run --release serve

# Engine + API only (no dashboard)
cargo run --release

# Dry run — one cycle, no execution
cargo run -- --dry-run

# Action test — sandbox scenarios
cargo run -- --test
cargo run -- --test -c "Trend Bull" -a -n 20

# Test with different models (any OpenRouter model)
cargo run --release -- --test --model openrouter/owl-alpha -n 20
cargo run --release -- --test --model deepseek/deepseek-v4-flash -n 20

# Managed API keys ($1 limit per session)
cargo run --release -- --test --managed-keys -n 20

# Training mode — loop until Brier converges
cargo run -- --test --train

# Sandbox — full 60-scenario stress test
cargo run --release -- --test --sandbox

# Historical data training
cargo run -- --historical

# Model comparison — run multiple models against sandbox
.\run-model-tests.ps1

# API server only (no engine)
cargo run -- --api-only

# Backtest
cargo run -- backtest
```

### Model Testing

`run-model-tests.ps1` runs each model against the 60-scenario sandbox with a 180s timeout per model. It outputs a comparison table with:

| Metric | Description |
|--------|-------------|
| **Score** | Average 3-tier grading score (0-10) |
| **Passed** | Scenarios passed / total |
| **Compliance** | Binary rule compliance % |
| **T2 (R:R)** | Risk/reward scoring (0-10) |
| **T3 (Reason)** | Reasoning quality (0-10) |
| **Parse Errors** | JSON parse failures |
| **LLM Errors** | API errors |
| **Retried** | Rate-limited retries |

Models tested: Nemotron Nano, Nemotron Super, Gemma 4 31B, Gemma 4 26B, Kimi K2.6 (all free tiers via OpenRouter). Add models by editing the `$models` array in `run-model-tests.ps1`.

---

## Project Structure

```
savant-trading/
├── src/
│   ├── agent/                       # AI brain (prompts, provider, parser, orchestrator)
│   ├── execution/
│   │   └── dex/                     # DEX backends (0x, 1inch on Arbitrum)
│   │       ├── mod.rs               # 201 tokens, ChainConfig, multi-chain resolution
│   │       ├── trader.rs            # DexTrader — EIP-1559 signing, Permit2, eth_call
│   │       ├── zero_x.rs            # 0x Permit2 + Gasless + Cross-Chain API
│   │       └── inch.rs              # 1inch fallback backend
│   ├── data/
│   │   └── sources/                 # 6 candle sources + SourceRouter
│   │       ├── kraken.rs, okx.rs, kucoin.rs, gate.io.rs
│   │       ├── cryptocompare.rs, coingecko.rs
│   │       └── mod.rs               # SourceRouter with all-zero rejection
│   ├── insight/                     # Sentiment, on-chain, funding, news
│   ├── risk/                        # Position sizing, circuit breakers, correlation
│   ├── memory/                      # Episodic memory, calibration, replay
│   ├── sandbox/                     # GARCH(1,1) synthetic OHLCV, scenarios
│   ├── engine.rs                    # Main trading loop (3,850 lines)
│   └── main.rs                      # CLI entry point
├── config/
│   └── default.toml                 # All non-secret configuration
├── knowledge/                       # 10 JSON knowledge files (2,959 units)
├── docs/
│   └── llms-full.md                 # Full 0x API reference
├── dev/                             # Session docs, FIDs, LEARNINGS
├── Cargo.toml
└── ECHO.md                          # Savant Protocol (agent bootstrap)
```

---

## Risk Management

The risk layer is **independent of the AI brain** — the agent cannot override it.

| Circuit Breaker | Threshold | Action |
|----------------|-----------|--------|
| Single trade risk | 20% of portfolio | Max position size calculated automatically |
| Daily loss limit | 5% | All trading halted |
| Drawdown kill switch | 10% | All positions closed, manual restart required |
| Price tolerance | 0.5% drift | Trade rejected (price moved during LLM eval) |
| Spread filter | 30 bps | Trade rejected (insufficient liquidity) |
| Security (GoPlus) | Tax >1%, pausable | Trade rejected (unsafe token) |
| Gas halt | <0.002 ETH | Trading halted until wallet is funded |

---

## Development

```bash
cargo build
cargo test           # 264 tests
cargo clippy -- -D warnings
```

### Savant Protocol

All development follows the [Savant Protocol](https://github.com/fame0528/savant-protocol):
- 4 immutable process laws (Read-0-EOF, Present-Before-Act, Verify-Before-Proceed, Call-Graph Reachability)
- Perfection Loop FSM (RED → GREEN → AUDIT → SELF-CORRECT → COMPLETE)
- Session lifecycle management

### Findings

Bugs and improvements tracked via Master FID:
- 1 active Master FID (consolidated backlog, prioritized)
- 99 archived FIDs
- FID-093 next: Dashboard tabbed command bridge (P0)

---

## License

MIT
