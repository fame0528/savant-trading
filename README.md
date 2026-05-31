# SAVANT TRADING v0.4.2

<!-- markdownlint-disable MD033 -->
<div align="center">

<img src="img/savant.png" alt="Savant Logo" width="180" />

**AI-Native Autonomous Crypto Trading Engine**

A production-grade, Rust-native trading engine where an AI agent IS the brain — powered by 22 curated transcripts and research-derived knowledge files (254 total units) via mimo v2.5 pro through OpenGateway.

**Free, Unlimited AI — MIMO v2.5 Pro:** Ships with [OpenGateway](https://gitlawb.com/opengateway) as the inference provider — an open gateway sponsored by Xiaomi MiMo. 1M context window, 131K max output.

[![Rust](https://img.shields.io/badge/Rust-2021-%23000000?style=flat-square&logo=rust&logoColor=%2300fbff)](https://www.rust-lang.org/)[![Kraken](https://img.shields.io/badge/Kraken-Exchange-%23000000?style=flat-square&logo=bitcoin&logoColor=%2300fbff)](https://www.kraken.com/)[![OpenGateway](https://img.shields.io/badge/OpenGateway-MIMO%20v2.5%20Pro-%23000000?style=flat-square&logo=openai&logoColor=%2300fbff)](https://gitlawb.com/opengateway)[![Version](https://img.shields.io/badge/Version-0.4.2-%23000000?style=flat-square&logo=semver&logoColor=%2300fbff)](https://github.com/fame0528/savant-trading/releases)[![License](https://img.shields.io/badge/License-MIT-%23000000?style=flat-square&logo=github&logoColor=%2300fbff)](LICENSE)

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

- **AI Agent Brain** — mimo v2.5 pro via OpenGateway makes all trading decisions using full market context
- **Multi-Timeframe Analysis** — Fetches 5m + 1H + 4H candles per pair; higher timeframe context injected into AI prompt
- **Knowledge Injection System** — 22 transcripts processed into 254 discrete knowledge units, dynamically selected by market condition
- **Live Market Insight** — Fear & Greed Index, BTC Dominance, funding rates, liquidation clusters, exchange flows, on-chain analytics (MVRV, SOPR, NVT), news sentiment
- **Pair Discovery** — `scan_all_pairs = true` discovers all USD pairs on Kraken automatically
- **Kraken Integration** — REST + WebSocket for candle data, order execution, and account management
- **WebSocket Real-Time** — Kraken WS v2 streams ticker, order book, and trade data directly into the engine
- **Paper Trading** — Full simulation with realistic fees (0.26% Kraken taker) and slippage modeling
- **Scale-Out Execution** — TP1 → 50% close + break-even stop, TP2 → 60% of remainder, TP3 → full close
- **Circuit Breakers** — Independent risk layer the AI cannot override: 3% daily limit, 10% drawdown kill switch
- **Backtesting Engine** — Historical strategy validation with walk-forward optimization and Sharpe ratio
- **REST API with CORS** — 15 endpoints for dashboard integration: portfolio, positions, decisions, insight, memory, activity
- **Episodic Memory** — SQLite WAL decision ledger with Brier score calibration, CUSUM edge decay detection, experience replay
- **Sandbox Testing** — GARCH(1,1) synthetic OHLCV generation, 50 curated scenarios, 3-tier grading rubric, GEPA feedback
- **Trade Journal** — SQLite persistence for every trade, equity curve, and daily performance summaries
- **Glass House** — Obsidian vault integration for transparent trading state (trades, decisions, portfolio, lessons)
- **Modular Prompts** — System prompt composed from 5 independent layers, knowledge units injected per-condition
- **Structured Decisions** — AI outputs JSON with entry, stop, 3 take-profit levels, confidence score, and reasoning
- **Fallback Mode** — Rule-based strategies activate automatically if LLM is unavailable

---

## Knowledge Base

The AI agent's trading knowledge comes from 22 curated transcripts and research:

### Original Transcripts (11 files, 141 units)

| Transcript | Knowledge Domain | Key Concepts |
|------------|-----------------|--------------|
| `scalping-fabio-valentina-amt` | Order Flow, Volume Profile | 3-step model, aggression detection, CVD, mean reversion vs trend |
| `strategy-pradeep-bondi-episodic-pivot` | Execution, Catalysts | Sell-into-strength, four-factor model, self-leadership |
| `daytrading-tjr-complete-guide` | Technical Analysis | FVG, order blocks, liquidity sweeps, break of structure, multi-TF |
| `crypto-full-course-beginners` | Sentiment, Macro | Fear & Greed, BTC dominance, exchange flows, alt season |
| `crypto-brian-jung-2026-strategy` | Macro Analysis | Halving cycle, FOMC catalysts, DCA, narrative rotation |
| `daytrading-juvier-full-course` | Session Trading | Kill zones, session highs/lows, squeeze breakouts |
| `daytrading-warrior-trading-9-steps` | Stock Selection | 5 Pillars, pullback entry, Level 2, metrics |
| `daytrading-hybrid-super-scalping` | Scalping | Heikin Ashi + EMA, doji entries, prop firm scaling |
| `macro-cathie-wood-ark-invest` | Macro Thesis | Innovation platforms, Wright's Law, convergence |
| `ai-bot-claude-code-trading-bot` | Regime Detection | HMM regimes, walk-forward backtesting, circuit breakers |
| `ai-bot-trading-competition` | Strategy Design | Natural selection, multi-agent, strategy inputs |

### Research-Derived Knowledge (11 files, 113 units)

| File | Domain | Key Concepts |
|------|--------|-------------|
| `onchain_analytics.json` | On-Chain | MVRV, NUPL, SOPR, NVT, exchange flows, whale tracking |
| `risk_management.json` | Risk Math | Kelly Criterion, drawdown recovery, ATR trailing, anti-martingale |
| `crypto_derivatives.json` | Derivatives | Funding rate arb, liquidation cascades, OI divergence |
| `wyckoff_orderflow.json` | Price Action | Wyckoff accumulation/distribution, footprint charts, BTC.D rotation |
| `macro_liquidity.json` | Macro | 65-month liquidity cycle, DXY, ETF flows, debt ratio |
| `defi_execution.json` | DeFi | Hyperliquid, Uniswap V3 LP, options Greeks/DDH |
| `backtesting_deployment.json` | Backtesting | Walk-Forward Optimization, concept drift, Monte Carlo |
| `execution_engineering.json` | Execution | REST vs WebSocket, rate limits, Kraken fees |
| `prop_firm.json` | Prop Firms | Drawdown models, consistency rules, evaluation strategy |
| `trading_psychology.json` | Psychology | Tilt, cognitive debiasing, deliberate practice |
| `compliance.json` | Compliance | 1099-DA, wash sale rules, entity structuring |

Knowledge units are dynamically injected based on market conditions:

| Market Condition | Knowledge Injected |
|-----------------|-------------------|
| High volatility / trending | Fabio's order flow model, aggression detection |
| Ranging / consolidation | Fabio's mean reversion model, volume profile |
| Fear & Greed extreme | Contrarian playbook from crypto full course |
| Breaking news catalyst | Pradeep's episodic pivot rules |
| Session opening | Juvier's kill zone strategy |
| BTC dominance shifting | Alt season rotation playbook |

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
# OpenGateway API key (optional — has built-in defaults)
OPENGATEWAY_API_KEY=your_key_here

# Kraken API keys (required for live trading only)
KRAKEN_API_KEY=your_key
KRAKEN_API_SECRET=your_secret
```

---

## Configuration

All non-secret settings are in `config/default.toml`:

```toml
[exchange]
name = "kraken"
ws_url = "wss://ws.kraken.com/v2"
rest_url = "https://api.kraken.com"

[trading]
pairs = ["BTC/USD", "ETH/USD", "SOL/USD", "XRP/USD", "ADA/USD", "DOGE/USD",
         "AVAX/USD", "DOT/USD", "LINK/USD", "MATIC/USD", "UNI/USD",
         "ATOM/USD", "ALGO/USD", "FIL/USD", "NEAR/USD"]
scan_all_pairs = false           # Set true to discover all Kraken USD pairs
timeframe = "5m"
timeframes = ["5m", "1h", "4h"]  # Multi-timeframe analysis
base_currency = "USD"
starting_balance = 100.0
fee_rate = 0.0026        # Kraken taker fee
slippage_pct = 0.0005    # 0.05% slippage

[risk]
max_risk_per_trade = 0.01   # 1% per trade
max_daily_loss = 0.03        # 3% daily limit
max_drawdown = 0.10          # 10% drawdown kill switch
max_positions = 3
min_rr_ratio = 1.5

[ai]
provider = "opengateway"
model = "mimo-v2.5-pro"
autonomy_level = 3              # 1=suggest, 2=confirm, 3=autonomous
max_decisions_per_hour = 5
knowledge_token_budget = 8000
temperature = 0.7

[insight]
fear_greed_enabled = true
btc_dominance_enabled = true
funding_rate_enabled = true
liquidation_enabled = true
exchange_flows_enabled = true
news_sentiment_enabled = true
rss_enabled = true
rss_max_items = 10
onchain_enabled = true
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
│   │   ├── paper.rs              # Paper trading simulator
│   │   └── engine.rs             # Execution engine trait
│   ├── insight/                  # Live market insight
│   │   ├── sentiment.rs          # Fear & Greed, BTC Dominance
│   │   ├── funding_rates.rs      # Derivatives data
│   │   ├── liquidation.rs        # Liquidation clusters
│   │   ├── flows.rs              # Exchange inflow/outflow
│   │   ├── onchain.rs            # MVRV, SOPR, NVT (CoinMetrics)
│   │   ├── news.rs               # News and social sentiment
│   │   ├── rss.rs                # RSS feed fetcher (15 sources)
│   │   └── aggregator.rs         # Unified MarketContext
│   ├── monitor/                  # Journaling and reporting
│   │   ├── journal.rs            # SQLite trade journal
│   │   ├── metrics.rs            # Performance metrics
│   │   └── report.rs             # CLI reporting
│   ├── risk/                     # Risk management
│   │   ├── position.rs           # Position sizing
│   │   ├── stop_loss.rs          # Stop loss and break-even
│   │   └── circuit_breaker.rs    # Drawdown protection
│   ├── strategy/                 # Rule-based strategies (optional)
│   │   ├── momentum.rs
│   │   ├── mean_reversion.rs
│   │   └── regime.rs
│   ├── vault/                    # Obsidian vault integration
│   │   ├── writer.rs
│   │   ├── watcher.rs
│   │   └── config.rs
│   ├── memory/                   # Episodic memory + calibration
│   │   ├── episodic.rs           # SQLite WAL decision ledger
│   │   ├── context.rs            # 6th prompt layer (memory injection)
│   │   ├── calibration.rs        # Brier score confidence calibration
│   │   ├── cusum.rs              # CUSUM edge decay detection
│   │   └── replay.rs             # Experience replay (lessons from history)
│   ├── sandbox/                  # Synthetic scenario testing
│   │   ├── generator.rs          # GARCH(1,1) OHLCV generator
│   │   ├── scenarios.rs          # 50 curated scenarios (11 categories)
│   │   ├── grader.rs             # 3-tier grading rubric
│   │   ├── harness.rs            # Scenario runner + report cards
│   │   ├── feedback.rs           # GEPA-style SOUL.md mutation
│   │   ├── mock.rs               # Mock API presets
│   │   └── report.rs             # Report card generation
│   ├── tui/                      # Real-time TUI dashboard (ratatui)
│   ├── api/                    # REST API server (axum)
│   ├── engine.rs                 # Main trading loop
│   ├── main.rs                   # CLI entry point
│   └── lib.rs                    # Module declarations
├── config/
│   └── default.toml              # All non-secret configuration
├── knowledge/                    # 22 JSON knowledge files (254 units)
├── templates/
│   ├── FID-TEMPLATE.md           # Finding ID template
│   └── SESSION-SUMMARY.md        # Session summary template
├── transcripts/                  # Curated trading knowledge (12 transcripts)
├── docs/                         # Research documents
│   ├── DEEP-RESEARCH-QUESTIONS.md
│   ├── Crypto Trading Knowledge Expansion Roadmap.md
│   └── KNOWLEDGE-EXPANSION-EXECUTION.md
├── dev/
│   ├── LEARNINGS.md              # Cross-session knowledge
│   ├── findings/                 # FID tracking
│   └── session-summaries/        # Session history
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

# Help
cargo run -- --help
```

**API endpoints** (available at `http://localhost:8080/api/`):
`/status` `/config` `/portfolio` `/positions` `/trades` `/decisions` `/insight` `/knowledge` `/risk` `/session` `/activity` `/memory`

---

## Risk Management

The risk layer is **independent of the AI brain** — the agent cannot override it.

| Circuit Breaker | Threshold | Action |
|----------------|-----------|--------|
| Single trade risk | 1% of portfolio | Max position size calculated automatically |
| Daily loss limit | 3% | All trading halted for the day |
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

All bugs and improvements are tracked as Finding IDs in `dev/findings/`:

```bash
ls dev/findings/archived/
# All 31 FIDs closed and archived:
# FID-2026-0530-001.md  [CLOSED] ECHO Protocol Violations
# FID-2026-0530-002.md  [CLOSED] Paper Trading Persistence
# FID-2026-0530-003.md  [CLOSED] Fee & Slippage
# FID-2026-0530-004.md  [CLOSED] Trailing Stops
# ...
# FID-2026-0531-004.md  [CLOSED] Production Sprint (7 items)
```

See `dev/findings/archived/` for full history. Active FIDs (if any) are in `dev/findings/`.

### ECHO Protocol

All development follows the ECHO Protocol defined in `ECHO.md` — a universal agent bootstrap with:

- 4 immutable process laws (Read-0-EOF, Present-Before-Act, Verify-Before-Proceed, No-Speculation)
- Perfection Loop FSM (RED → GREEN → AUDIT → SELF-CORRECT → COMPLETE)
- Session lifecycle management

---

## License

MIT
