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
├── knowledge/                    # 22 JSON knowledge files (254 units)
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
│   │   └── paper.rs              # Paper trading simulator (with state persistence)
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
│   ├── findings/                 # FID tracking
│   └── session-summaries/
├── .env.example
├── .gitignore
├── Cargo.toml
├── ECHO.md
└── README.md
```

## Strategies

### 1. Momentum Breakout (Ross Cameron + TJR)

**Setup:**
- 100 EMA as trend filter (price above = long bias, below = short bias)
- Identify consolidation range (ATR compression < 0.7x average)
- Volume spike > 2x 20-period average

**Entry:**
- Long: Price breaks above range high with volume confirmation
- Short: Price breaks below range low with volume confirmation
- Confirmation: Break of structure (higher high for long, lower low for short)

**Exit:**
- Stop: Below/Above range midpoint (50% of range)
- TP1: 1:1 R:R (take 50%)
- TP2: 1:2 R:R (take 30%)
- TP3: 1:3 R:R (take 20%)
- Move stop to break-even after TP1 hit

### 2. Volume Profile Mean Reversion (Fabio Valentina)

**Setup:**
- Calculate volume profile over N periods (default: 100)
- Identify Value Area (70% of volume) and Point of Control (POC)
- Price extends beyond Value Area Low (VAL) or Value Area High (VAH)

**Entry:**
- Long: Price dips below VAL, then first green candle back inside value area
- Short: Price extends above VAH, then first red candle back inside value area
- Confirmation: Large order absorption (volume spike without price continuation)

**Exit:**
- Stop: Beyond the extreme (below VAL for long, above VAH for short)
- TP: Point of Control (POC)
- R:R typically 1:2 to 1:4

### 3. Regime-Based Strategy Selector

**Detection (ADX + ATR):**
- ADX > 25 and rising → Trending → Use Momentum Breakout
- ADX < 20 → Ranging → Use Mean Reversion
- ATR > 1.5x 20-period average → Volatile → Reduce size by 50%
- BTC correlation > 0.8 → Correlated → Trade BTC only

## Risk Management

| Rule | Value | Source |
|------|-------|--------|
| Max risk per trade | 1% of account | TJR, Fabio |
| Max daily loss | 3% of account | All sources |
| Max drawdown | 10% of account | Fabio |
| Max concurrent positions | 3 | Portfolio diversification |
| Scale-out | 50% @ 1:1, 30% @ 1:2, 20% @ 1:3 | Pradeep, Fabio |
| Break-even trigger | After 1R profit | Fabio |
| Min R:R | 1:1.5 | Ross Cameron |

## Position Sizing

```
position_size = (account_balance * risk_per_trade) / (entry_price - stop_loss_price)

Example:
  Account: $100
  Risk: 1% = $1
  Entry: $50,000 (BTC)
  Stop: $49,500 (0.5% below)
  Position size: $1 / $500 = 0.00002 BTC ($1 worth of risk)
```

## Tech Stack

- **Language:** Rust (tokio async runtime)
- **Exchange:** Kraken (REST API v0 + WebSocket v2)
- **HTTP:** reqwest
- **WebSocket:** tokio-tungstenite
- **Serialization:** serde + serde_json
- **Config:** toml
- **Database:** sqlx + SQLite (trade journal)
- **Logging:** tracing + tracing-subscriber
- **Indicators:** Custom implementation (no external TA library)

## Phased Implementation

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Foundation: project setup, config, types, error handling | Pending |
| 2 | Data Engine: Kraken client, OHLCV, indicators | Pending |
| 3 | Strategy Engine: momentum, mean reversion, regime | Pending |
| 4 | Risk Management: position sizing, stops, circuit breaker | Pending |
| 5 | Execution: paper trader, then live Kraken | Pending |
| 6 | Monitoring: SQLite journal, metrics, alerts | Pending |
| 7 | Integration: wire together, backtest, live paper trading | Pending |

## Configuration

```toml
[exchange]
name = "kraken"
api_key = ""          # Set via environment variable
api_secret = ""       # Set via environment variable
ws_url = "wss://ws.kraken.com/v2"
rest_url = "https://api.kraken.com"

[trading]
pairs = ["BTC/USD", "ETH/USD"]
timeframe = "5m"
base_currency = "USD"
starting_balance = 100.0

[risk]
max_risk_per_trade = 0.01      # 1%
max_daily_loss = 0.03          # 3%
max_drawdown = 0.10            # 10%
max_positions = 3
min_rr_ratio = 1.5             # Minimum 1:1.5 R:R

[strategy.momentum]
ema_period = 100
volume_spike_multiplier = 2.0
atr_compression_threshold = 0.7

[strategy.mean_reversion]
profile_periods = 100
value_area_pct = 0.70
volume_spike_multiplier = 1.5

[strategy.regime]
adx_period = 14
adx_trending_threshold = 25
adx_ranging_threshold = 20
atr_volatility_multiplier = 1.5

[mode]
paper_trading = true           # Start in paper mode
```
