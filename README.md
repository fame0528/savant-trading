# SAVANT TRADING v0.8.0

<!-- markdownlint-disable MD033 -->
<div align="center">

<img src="img/savant.png" alt="Savant Logo" width="180" />

**AI-Native Autonomous Crypto Trading Engine**

A production-grade, Rust-native trading engine where an AI agent IS the brain — powered by 2,959 knowledge units across 10 enterprise-grade JSON files.

**AI Brain:** MiMo v2.5 Pro via [OpenRouter](https://openrouter.ai/xiaomi/mimo-v2.5-pro). 1M context window, 131K max output.

[![Rust](https://img.shields.io/badge/Rust-2021-%23000000?style=flat-square&logo=rust&logoColor=%2300fbff)](https://www.rust-lang.org/)[![Kraken](https://img.shields.io/badge/Kraken-Exchange-%23000000?style=flat-square&logo=bitcoin&logoColor=%2300fbff)](https://www.kraken.com/)[![OpenRouter](https://img.shields.io/badge/OpenRouter-MiMo%20v2.5%20Pro-%23000000?style=flat-square&logo=openai&logoColor=%2300fbff)](https://openrouter.ai/xiaomi/mimo-v2.5-pro)[![Version](https://img.shields.io/badge/Version-0.8.0-%23000000?style=flat-square&logo=semver&logoColor=%2300fbff)](https://github.com/fame0528/savant-trading/releases)[![License](https://img.shields.io/badge/License-MIT-%23000000?style=flat-square&logo=github&logoColor=%2300fbff)](LICENSE)

</div>

---

## Overview

Savant Trading is an autonomous crypto trading engine built on a fundamental insight: **the AI agent IS the brain, not an afterthought**.

Traditional algorithmic engines use hardcoded rule-based strategies (momentum, mean reversion, RSI crossovers). Savant Trading inverts this — an LLM agent receives all market context (candles, indicators, sentiment, derivatives data, macro context) and makes trading decisions using knowledge extracted from 11 curated transcripts of world-class traders and AI trading experiments.

### Architecture

```
Transcripts ──────→ Knowledge Base (11 curated transcripts)
                            ↓
System Prompt ←──── Modular prompt composer (5 layers)
                            ↓
Market Data ──────┐
Insight Data ─────┤
Positions ────────┤──→ AI Brain (mimo v2.5 pro) → Trade Decisions → Execution
Account State ────┤
Trade History ────┘

Rule-Based Strategies ──→ Optional parallel signals (comparison only)
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **AI is the brain** | Rule-based strategies can't adapt to novel conditions. The LLM reasons across all context. |
| **Transcripts as knowledge** | 11 curated transcripts from world-class traders provide structured trading wisdom, not raw text dumps. |
| **Dynamic knowledge injection** | Knowledge units are selected based on current market conditions — high volatility triggers Fabio's order flow rules, extreme fear triggers contrarian playbook. |
| **5-layer system prompt** | Base identity → Risk constraints → Strategy knowledge → Transcript knowledge → Output format. Each layer is independent and composable. |
| **3 autonomy levels** | Suggest (log only), Confirm (human-in-the-loop), Autonomous (full auto). Start with Suggest, graduate to Autonomous. |
| **Fallback to rules** | If the LLM fails 3 consecutive ticks, rule-based strategies take over temporarily. |

---

## Features

- **AI Agent Brain** — MiMo v2.5 Pro via OpenRouter makes all trading decisions using full market context
- **SSE Streaming** — Real-time LLM response streaming, no timeout risk during long reasoning
- **Multi-Timeframe Analysis** — Fetches 5m + 1H + 4H candles per pair; higher timeframe context injected into AI prompt
- **Knowledge Base** — 2,959 units across 10 enterprise-grade JSON files, MMR selection with utility scoring, indicator-driven context tags
- **Closed-Loop Training** — Semantic consolidation, anti-pattern detection, GEPA mutation, Teacher LLM, version control, auto-rollback
- **Persistent Memory** — SQLite WAL episodic memory, Brier score calibration, CUSUM edge decay, experience replay
- **Training Pipeline** — Progressive difficulty, convergence detection, confidence distribution, category edge tracking, auto-lesson generation
- **Live Market Insight** — Fear & Greed Index, BTC Dominance, funding rates, liquidation clusters, exchange flows, on-chain analytics (MVRV, SOPR, NVT), news sentiment
- **Multi-Asset Correlation** — Pearson correlation matrix, effective position counting for correlated pairs
- **Portfolio Heat** — Total risk exposure tracking, blocks trades when heat exceeds 40% of equity
- **Dynamic Slippage** — Slippage scales with ATR volatility and order book depth
- **Pair Discovery** — `scan_all_pairs` can discover 455+ Kraken USD pairs (off by default — 15+ min/cycle)
- **Kraken Integration** — REST + WebSocket (exponential backoff reconnection) for candle data, order execution, and account management
- **Paper Trading** — Full simulation with realistic fees (0.40% Kraken taker) and dynamic slippage modeling
- **Scale-Out Execution** — TP1 → 50% close + break-even stop, TP2 → 60% of remainder, TP3 → full close
- **Circuit Breakers** — Independent risk layer the AI cannot override: daily loss limit, drawdown kill switch, max positions, portfolio heat
- **Backtesting Engine** — Historical strategy validation with walk-forward optimization and Sharpe ratio
- **REST API with CORS** — 16 endpoints including /api/training for training metrics
- **SQLite Backup** — Automatic rolling backup rotation (last 7 copies, 6-hour interval)
- **Sandbox Testing** — GARCH(1,1) synthetic OHLCV, 60 scenarios (11 categories), 3-tier grading, train/val split
- **SOUL.md Evolution** — Immutable/mutable zones, GEPA textual optimization, pareto gatekeeper, version control
- **Trade Journal** — SQLite persistence for every trade, equity curve, and daily performance summaries
- **Glass House** — Obsidian vault integration for transparent trading state
- **Modular Prompts** — System prompt composed from 5 independent layers + 6th memory context layer
- **Structured Decisions** — AI outputs JSON with entry, stop, 3 take-profit levels, confidence score, and reasoning
- **Fallback Mode** — Rule-based strategies activate automatically if LLM is unavailable
- **DEX Execution** — 0x API v2 on Arbitrum (no KYC). EIP-1559 signing, Permit2 EIP-712 approval (with 32-byte length prefix), ERC-20 approve(max) for Permit2, eth_call dry-run, receipt verification, 3-retry logic, 50% gas buffer
- **Multi-Source Candles** — 8 sources: Kraken, OKX, KuCoin, Gate.io, CryptoCompare, CoinGecko, GeckoTerminal, Binance. Automatic fallback with all-zero rejection.
- **198 Arbitrum Tokens** — Real addresses from CoinGecko API. Covers all high-volume tokens on Arbitrum One.
- **Enterprise Console** — Structured output: `[Savant Trading] [MM-DD-YYYY HH:mm AM/PM] [ACTION] [RESULT]`. Cyan brand, grey timestamps, color-coded results (BUY=green, SELL=red, PASS=grey)
- **Casing-Tolerant Parser** — AI responses parse correctly regardless of casing (BUY/Buy/buy all work)
- **Position Sizer Safety** — Min order value ($1), max position pct (30%), balance cap prevents overexposure
- **HTML Dashboard** — Single-file vanilla JS dashboard at `/dashboard.html` with glassmorphic design

---

## Knowledge Base

The AI agent's trading knowledge comes from 150+ books and transcripts, organized into 10 enterprise-grade JSON files with 2,959 tagged knowledge units:

| File | Units | Domain |
|------|-------|--------|
| `knowledge_technical_analysis.json` | 506 | RSI, EMA, ADX, MACD, Bollinger, Fibonacci, divergences |
| `knowledge_psychology.json` | 319 | Cognitive biases, tilt, deliberate practice, emotional regulation |
| `knowledge_crypto_native.json` | 319 | On-chain analytics, DeFi, funding rates, liquidation cascades |
| `knowledge_risk_management.json` | 350 | Kelly Criterion, drawdown recovery, position sizing, anti-martingale |
| `knowledge_sentiment.json` | 291 | Fear & Greed, social sentiment, news analysis, crowd psychology |
| `knowledge_execution.json` | 282 | Order types, slippage, fill optimization, rate limits |
| `knowledge_market_regimes.json` | 250 | Trending, ranging, volatile, capitulation detection |
| `knowledge_trading_systems.json` | 226 | Backtesting, walk-forward, Monte Carlo, strategy design |
| `knowledge_price_action.json` | 216 | Wyckoff, candle patterns, support/resistance, liquidity |
| `knowledge_fundamentals.json` | 200 | Macro analysis, halving cycles, ETF flows, regulatory |

Knowledge units are tagged with `setup_type`, `regime_subtype`, `trigger`, `indicator`, and `risk_context` for precise MMR selection. A `utility_score` field tracks empirical correlation with successful trades — units that help the agent win get promoted, units that correlate with losses get suppressed.

---

## Quick Start

### Prerequisites

- Rust 1.91+ (install via [rustup](https://rustup.rs/))
- Kraken account (for live trading — paper trading works without)

### Setup

```bash
git clone https://github.com/YOUR_USERNAME/savant-trading.git
cd savant-trading

# Copy environment template
cp .env.example .env

# Build
cargo build --release

# Run paper trading
cargo run
```

### Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
# OpenRouter API key (required for AI decisions)
OPENROUTER_API_KEY=your_key_here

# Kraken API keys (required for Kraken CEX trading only)
KRAKEN_API_KEY=your_key
KRAKEN_API_SECRET=your_secret

# 0x API key (required for DEX trading only)
ZEROEX_API_KEY=your_key

# Wallet private key (required for DEX trading only)
WALLET_PRIVATE_KEY=your_key
```

---

## Configuration

All non-secret settings are in `config/default.toml`:

```toml
[exchange]
name = "kraken"
backend = "0x"                     # "kraken" (CEX), "0x" (DEX), "1inch" (DEX)
ws_url = "wss://ws.kraken.com/v2"
rest_url = "https://api.kraken.com"

[exchange.dex]
chain_id = 42161                 # Arbitrum
rpc_url = "https://arb1.arbitrum.io/rpc"
slippage_pct = 0.005

[trading]
pairs = [
    "BTC/USD", "ETH/USD", "SOL/USD", "XRP/USD",
    "DOGE/USD", "ADA/USD", "LINK/USD", "AVAX/USD",
]
scan_all_pairs = false           # 455+ pairs, 15+ min/cycle — only if needed
timeframe = "5m"
timeframes = ["5m", "1h", "4h", "1d"]
base_currency = "USD"
starting_balance = 50.0
fee_rate = 0.0040                # Kraken Pro base tier taker
slippage_pct = 0.0005

[risk]
max_risk_per_trade = 0.20        # 20% per trade ($10 at $50 — fully deployed)
max_daily_loss = 0.05            # 5% daily halt
max_drawdown = 0.10              # 10% drawdown kill switch
max_positions = 3
min_rr_ratio = 2.0

[ai]
provider = "openrouter"
model = "xiaomi/mimo-v2.5-pro"
autonomy_level = 3
max_decisions_per_hour = 20
knowledge_token_budget = 20000
temperature = 0.6
top_p = 0.95
max_tokens = 16384
timeout_secs = 300
```

---

## Project Structure

```
savant-trading/
├── src/
│   ├── agent/                    # AI brain
│   │   ├── soul.md               # 560-line AI persona (loaded via include_str!)
│   │   ├── knowledge.rs          # Knowledge unit types and selection
│   │   ├── prompts.rs            # Modular system prompt composer
│   │   ├── provider.rs           # OpenAI-compatible LLM client
│   │   ├── context_builder.rs    # Aggregates data into LLM context
│   │   ├── decision_parser.rs    # Extracts TradeDecision from JSON
│   │   └── orchestrator.rs       # Main decision loop
│   ├── backtest/                 # Historical strategy validation
│   │   ├── engine.rs             # Candle replay through Strategy trait
│   │   ├── metrics.rs            # Sharpe, drawdown, win rate, profit factor
│   │   └── walk_forward.rs       # Walk-forward optimization
│   ├── core/                     # Types, config, errors
│   │   ├── config.rs
│   │   ├── console.rs            # Enterprise logging (savant_log, SavantTimer)
│   │   ├── types.rs
│   │   ├── error.rs
│   │   ├── events.rs
│   │   ├── session.rs
│   │   └── shared.rs
│   ├── data/                     # Market data
│   │   ├── kraken.rs             # Kraken REST client
│   │   ├── market_data.rs        # Candle store
│   │   ├── indicators.rs         # EMA, RSI, ATR, ADX, VWAP
│   │   ├── orderbook.rs          # Order book
│   │   └── websocket.rs          # Kraken WebSocket v2 client
│   ├── execution/                # Trade execution
│   │   ├── engine.rs             # Execution engine trait
│   │   ├── paper.rs              # Paper trading simulator
│   │   └── dex/                  # DEX backends (0x, 1inch on Arbitrum)
│   │       ├── mod.rs            # Token resolution, DexTrader
│   │       ├── trader.rs         # DexTrader execution logic
│   │       ├── zero_x.rs         # 0x API backend
│   │       └── inch.rs           # 1inch API backend
│   ├── insight/                  # Live market insight
│   │   ├── aggregator.rs         # Unified MarketContext
│   │   ├── sentiment.rs          # Fear & Greed, BTC Dominance
│   │   ├── funding_rates.rs      # Derivatives data
│   │   ├── liquidation.rs        # Liquidation clusters
│   │   ├── flows.rs              # Exchange inflow/outflow
│   │   ├── onchain.rs            # MVRV, SOPR, NVT
│   │   ├── news.rs               # News and social sentiment
│   │   └── rss.rs                # RSS feed fetcher (15 sources)
│   ├── memory/                   # Episodic memory + calibration
│   │   ├── episodic.rs           # SQLite WAL decision ledger
│   │   ├── context.rs            # 6th prompt layer (memory injection)
│   │   ├── calibration.rs        # Brier score confidence calibration
│   │   ├── cusum.rs              # CUSUM edge decay detection
│   │   ├── replay.rs             # Experience replay (lessons from history)
│   │   ├── semantic.rs           # Semantic consolidation (SQL → patterns)
│   │   └── anti_pattern.rs       # Anti-pattern detection
│   ├── sandbox/                  # Synthetic scenario testing
│   │   ├── generator.rs          # GARCH(1,1) OHLCV generator
│   │   ├── scenarios.rs          # 50 curated scenarios (11 categories)
│   │   ├── grader.rs             # 3-tier grading rubric
│   │   ├── harness.rs            # Scenario runner + report cards
│   │   ├── feedback.rs           # GEPA-style SOUL.md mutation
│   │   ├── mock.rs               # Mock API presets
│   │   └── report.rs             # Report card generation
│   ├── monitor/                  # Journaling and reporting
│   │   ├── journal.rs            # SQLite trade journal
│   │   ├── metrics.rs            # Performance metrics
│   │   └── report.rs             # CLI reporting
│   ├── risk/                     # Risk management
│   │   ├── position.rs           # Position sizing
│   │   ├── stop_loss.rs          # Stop loss and break-even
│   │   ├── circuit_breaker.rs    # Drawdown protection + portfolio heat
│   │   └── correlation.rs        # Multi-asset Pearson correlation matrix
│   ├── strategy/                 # Rule-based strategies (optional)
│   │   ├── momentum.rs
│   │   ├── mean_reversion.rs
│   │   └── regime.rs
│   ├── vault/                    # Obsidian vault integration
│   │   ├── writer.rs
│   │   ├── watcher.rs
│   │   └── config.rs
│   ├── tui/                      # Real-time TUI dashboard (ratatui)
│   ├── api/                      # REST API server (axum)
│   ├── engine.rs                 # Main trading loop
│   ├── main.rs                   # CLI entry point
│   └── lib.rs                    # Module declarations
├── config/
│   ├── default.toml              # All non-secret configuration
│   └── canary.toml               # Canary config for testing
├── dashboard.html                # Single-file vanilla JS dashboard
├── knowledge/                    # 10 JSON knowledge files (2,959 units)
├── templates/
│   ├── FID-TEMPLATE.md           # Finding ID template
│   └── SESSION-SUMMARY.md        # Session summary template
├── transcripts/                  # Curated trading knowledge (11 transcripts)
├── docs/                         # Research documents (Nova audit in progress)
├── dev/
│   ├── LEARNINGS.md              # Cross-session knowledge
│   ├── fids/                     # Active FIDs
│   │   └── archive/              # 70 archived FIDs
│   └── session-summaries/        # Session history
├── stats.ps1                     # Performance scoreboard
├── run-canary.ps1                # Canary mode launcher
├── .env.example                  # Environment template
├── .gitignore
├── Cargo.toml
├── ECHO.md                       # Agent protocol
└── README.md
```

---

## CLI Commands

```bash
# Paper trading + API server (default)
cargo run

# Dry run (one AI decision cycle, print pipeline)
cargo run -- --dry-run

# API server only (no engine)
cargo run -- --api-only

# Backtest on historical data
cargo run -- backtest

# View performance report
cargo run -- report

# Action test (all scenarios with memory capture)
cargo run -- --test

# Action test with filters
cargo run -- --test -c "Trend Bull"         # Filter by category
cargo run -- --test -a                       # Only Buy/Sell scenarios
cargo run -- --test -n 20                    # First N scenarios
cargo run -- --test -c "Crash" -a -n 10      # Combine filters

# Training mode (loop until Brier converges)
cargo run -- --test --train
cargo run -- --test --train -a -n 20

# Historical data training (30 days of 5m candles)
cargo run -- --historical

# Help
cargo run -- --help
```

**API endpoints** (available at `http://localhost:8080/api/`):
`/status` `/config` `/portfolio` `/positions` `/assets` `/trades` `/decisions` `/insight` `/knowledge` `/risk` `/session` `/activity` `/memory` `/training`

**Dashboard:** `http://localhost:8080/dashboard.html`

---

## Risk Management

The risk layer is **independent of the AI brain** — the agent cannot override it.

| Circuit Breaker | Threshold | Action |
|----------------|-----------|--------|
| Single trade risk | 20% of portfolio ($10 at $50) | Max position size calculated automatically |
| Daily loss limit | 5% | All trading halted for the day |
| Drawdown kill switch | 10% | All positions closed, bot stops, manual restart required |
| Consecutive failures | 3 LLM failures | Fallback to rule-based strategies temporarily |

---

## Development

### Building

```bash
cargo build
cargo clippy -- -D warnings
cargo fmt --check
```

### Finding IDs (FIDs)

All bugs and improvements are tracked as Finding IDs in `dev/fids/`:

```bash
ls dev/fids/
# (empty — all FIDs closed and archived)

ls dev/fids/archive/
# 50 archived FIDs from development history (FID-001 through 024)
```

### Current FID Status

| FIDs | Count | Location |
|------|-------|----------|
| Active | 0 | `dev/fids/` (clean slate) |
| Archived | 70 | `dev/fids/archive/` |

### ECHO Protocol

All development follows the ECHO Protocol defined in `ECHO.md` — a universal agent bootstrap with:

- 4 immutable process laws (Read-0-EOF, Present-Before-Act, Verify-Before-Proceed, No-Speculation)
- Perfection Loop FSM (RED → GREEN → AUDIT → SELF-CORRECT → COMPLETE)
- Session lifecycle management

---

## License

MIT
