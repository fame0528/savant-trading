# Savant Trading Engine — System Design

## Overview

An automated crypto trading system built in Rust that trades on Kraken exchange.
Synthesized from 11 curated trader transcripts and 11 research-derived knowledge
files covering momentum, volume profile, order flow, risk management, on-chain
analytics, derivatives, Wyckoff, macro liquidity, DeFi, backtesting, execution
engineering, prop firms, psychology, and compliance.

## Architecture

```
savant-trading/
├── Cargo.toml
├── config/
│   └── default.toml              # Runtime configuration
├── knowledge/                    # 10 JSON knowledge files (2,959 units)
├── src/
│   ├── main.rs                   # CLI entry point
│   ├── lib.rs                    # Library root
│   ├── engine.rs                 # Main trading loop
│   ├── api/
│   │   └── mod.rs                # REST API server (axum)
│   ├── tui/
│   │   └── mod.rs                # Real-time TUI dashboard (ratatui)
│   ├── core/                     # Foundation layer
│   │   ├── mod.rs
│   │   ├── config.rs             # Configuration loading/validation
│   │   ├── types.rs              # Shared types (Order, Position, Signal, Candle)
│   │   ├── events.rs             # Event bus (channel-based)
│   │   ├── error.rs              # Error types
│   │   ├── session.rs            # Trading session detection
│   │   └── shared.rs             # SharedEngineData (API/TUI/engine)
│   ├── data/                     # Market data layer
│   │   ├── mod.rs
│   │   ├── kraken.rs             # Kraken REST client
│   │   ├── market_data.rs        # OHLCV candle management
│   │   ├── orderbook.rs          # Order book depth processing
│   │   ├── indicators.rs         # EMA, RSI, ATR, ADX, VWAP, Volume Profile
│   │   └── websocket.rs          # Kraken WebSocket v2 client
│   ├── agent/                    # AI brain
│   │   ├── mod.rs
│   │   ├── knowledge.rs          # Knowledge unit types and selection
│   │   ├── prompts.rs            # Modular system prompt composer
│   │   ├── provider.rs           # OpenAI-compatible LLM client
│   │   ├── context_builder.rs    # Aggregates data into LLM context
│   │   ├── decision_parser.rs    # Extracts TradeDecision from JSON
│   │   └── orchestrator.rs       # Main decision loop
│   ├── strategy/                 # Signal generation
│   │   ├── mod.rs
│   │   ├── base.rs               # Strategy trait (async + sync)
│   │   ├── momentum.rs           # Break of structure + volume breakout
│   │   ├── mean_reversion.rs     # Volume profile mean reversion
│   │   └── regime.rs             # Market regime detection (ADX-based)
│   ├── risk/                     # Risk management
│   │   ├── mod.rs
│   │   ├── position.rs           # Position sizing (fixed fractional)
│   │   ├── stop_loss.rs          # ATR-based + structure-based stops
│   │   └── circuit_breaker.rs    # Daily loss limit, max drawdown
│   ├── execution/                # Order execution
│   │   ├── mod.rs
│   │   ├── engine.rs             # Execution engine trait
│   │   ├── paper.rs              # Paper trading simulator (with state persistence)
│   │   └── dex/                  # DEX backends (0x, 1inch on Arbitrum)
│   │       ├── mod.rs            # Token resolution, DexTrader
│   │       ├── trader.rs         # DexTrader execution logic
│   │       ├── zero_x.rs         # 0x API backend
│   │       └── inch.rs           # 1inch API backend
│   ├── insight/                  # Live market insight
│   │   ├── mod.rs
│   │   ├── sentiment.rs          # Fear & Greed, BTC Dominance
│   │   ├── funding_rates.rs      # Derivatives data
│   │   ├── liquidation.rs        # Liquidation clusters
│   │   ├── flows.rs              # Exchange inflow/outflow
│   │   ├── onchain.rs            # MVRV, SOPR, NVT (CoinMetrics + CoinGecko)
│   │   ├── news.rs               # News and social sentiment
│   │   ├── rss.rs                # RSS feed fetcher (15 sources)
│   │   └── aggregator.rs         # Unified MarketContext
│   ├── memory/                   # Episodic memory + calibration
│   │   ├── mod.rs
│   │   ├── episodic.rs           # SQLite WAL decision ledger
│   │   ├── context.rs            # 6th prompt layer (memory injection)
│   │   ├── calibration.rs        # Brier score confidence calibration
│   │   ├── cusum.rs              # CUSUM edge decay detection
│   │   ├── replay.rs             # Experience replay
│   │   ├── semantic.rs           # Semantic consolidation
│   │   └── anti_pattern.rs       # Anti-pattern detection
│   ├── sandbox/                  # Synthetic scenario testing
│   │   ├── mod.rs
│   │   ├── generator.rs          # GARCH(1,1) OHLCV generator
│   │   ├── scenarios.rs          # 50 curated scenarios (11 categories)
│   │   ├── grader.rs             # 3-tier grading rubric
│   │   ├── harness.rs            # Scenario runner
│   │   ├── feedback.rs           # GEPA-style SOUL.md mutation
│   │   ├── mock.rs               # Mock API presets
│   │   └── report.rs             # Report card generation
│   ├── monitor/                  # Monitoring & journaling
│   │   ├── mod.rs
│   │   ├── journal.rs            # SQLite trade journal
│   │   ├── metrics.rs            # Win rate, profit factor, Sharpe
│   │   └── report.rs             # CLI reporting
│   ├── backtest/                 # Historical strategy validation
│   │   ├── mod.rs
│   │   ├── engine.rs             # Candle replay through Strategy trait
│   │   ├── metrics.rs            # Sharpe, drawdown, win rate, profit factor
│   │   └── walk_forward.rs       # Walk-forward optimization
│   └── vault/                    # Obsidian vault integration
│       ├── mod.rs
│       ├── config.rs             # Vault configuration
│       ├── writer.rs             # Project trades/decisions/portfolio
│       └── watcher.rs            # Ingest lessons
├── templates/
│   ├── FID-TEMPLATE.md
│   └── SESSION-SUMMARY.md
├── transcripts/                  # 12 curated trading transcripts
├── docs/                         # Research documents
├── dev/
│   ├── LEARNINGS.md
│   ├── fids/                     # Active FIDs / closed archive
│   │   └── archive/
│   └── session-summaries/
├── .env.example
├── .gitignore
├── Cargo.toml
├── ECHO.md
└── README.md
```

## Strategies

The system does not use hardcoded rule-based strategies. The **AI agent** (mimo v2.5 pro via OpenGateway) receives full market context and makes trading decisions using knowledge extracted from 11 curated transcripts. The strategy module contains optional rule-based fallbacks that activate only if the LLM fails 3 consecutive ticks.

## AI Brain Architecture

### Decision Pipeline

```
Market Data (candles, indicators, order book)
    ↓
Insight Data (funding rates, sentiment, on-chain, RSS)
    ↓
Account State + Positions + Trade History
    ↓
6-Layer Prompt Composition:
  1. SOUL.md identity (loaded via include_str!)
  2. Risk constraints
  3. Strategy knowledge (transcript-derived)
  4. ECHO trading rules
  5. Output format (strict JSON schema)
  6. Memory context (Brier, CUSUM, anti-patterns)
    ↓
Knowledge Selection (MMR + utility scoring, 2,959 units, capped at 20)
    ↓
LLM (mimo v2.5 pro via SSE streaming) → JSON decision
    ↓
Decision Parser → TradeDecision validation
    ↓
Execution Engine (Paper / Kraken CEX / 0x DEX / 1inch DEX)
```

### Key Components

| Component | File | Role |
|-----------|------|------|
| SOUL.md identity | `src/agent/prompts/base_identity.md` | Identity, 5-step thinking framework |
| Risk constraints | `src/agent/prompts/risk_constraints.md` | Hard limits AI cannot override |
| Strategy knowledge | `src/agent/prompts/strategy_knowledge.md` | Scale-out, trailing, session awareness |
| Trading rules | `src/agent/prompts/echo_rules.md` | ECHO-derived rules (sell into strength, 3-loss stop) |
| Output format | `src/agent/prompts/output_format.md` | Strict JSON schema enforcement |
| Knowledge selection | `src/agent/knowledge.rs` | MMR + utility scoring from 2,959 units |
| Context builder | `src/agent/context_builder.rs` | Assembles all data into LLM prompt |
| Decision parser | `src/agent/decision_parser.rs` | Extracts + validates JSON decisions |
| Orchestrator | `src/agent/orchestrator.rs` | Main decision loop, 3 autonomy levels |

## Risk Management

| Rule | Value | Source |
|------|-------|--------|
| Max risk per trade | 20% of account ($10 at $50) | FID-015: full deployment at small balance |
| Max daily loss | 10% of account | All transcript sources |
| Max drawdown | 20% of account | Fabio, TJR |
| Max concurrent positions | 5 | Portfolio diversification + small balance |
| Scale-out | 50% @ 1:1, 30% @ 1:2, 20% @ 1:3 | Pradeep, Fabio |
| Break-even trigger | After 1R profit | Fabio |
| Min R:R | 2.0:1 | FID-015: compensates for full deployment risk |

## Position Sizing

```
position_size = (account_balance * risk_per_trade) / (entry_price - stop_loss_price)

Example:
  Account: $50
  Risk: 20% = $10
  Entry: $50,000 (BTC)
  Stop: $49,500 (0.5% below)
  Position size: $10 / $500 = 0.0002 BTC ($10 worth of risk)

Dynamic risk tiers (configurable):
  $50 → 20% risk (fully deployed, safety from stops + circuit breakers)
  $500 → 10% risk (50% deployed)
  $5,000 → 5% risk (25% deployed)
  $50,000+ → 2% risk (10% deployed)
```

## Tech Stack

- **Language:** Rust (tokio async runtime)
- **Exchange:** Kraken (REST API v0 + WebSocket v2) / 0x DEX / 1inch DEX
- **HTTP:** reqwest
- **WebSocket:** tokio-tungstenite
- **Serialization:** serde + serde_json
- **Config:** toml
- **Database:** sqlx + SQLite (trade journal, episodic memory)
- **Logging:** tracing + tracing-subscriber
- **Indicators:** Custom implementation (no external TA library)
- **Cache:** TTL cache with LRU eviction
- **Confidence Calibration:** PAVA isotonic regression (Brier score)
