# Savant Trading — Session Summary

**Date:** 2026-05-30
**Session Duration:** ~4 hours
**Protocol:** ECHO Protocol v4.0.0
**Language:** Rust
**Exchange:** Kraken (free tier — API keys not yet generated)

---

## What This Project Is

An automated crypto trading engine built in Rust that trades on Kraken exchange. The system synthesizes knowledge from 10+ expert trader YouTube transcripts into concrete trading strategies, risk management, and execution logic. Currently in **paper trading mode** — simulates trades with real market data but no real money.

---

## Transcripts Formatted (Knowledge Base)

All in `C:\Users\spenc\dev\savant-trading\transcripts\`:

| File | Source | Key Knowledge |
|------|--------|---------------|
| `Cathie Wood.md` | Diary of a CEO podcast | Macro investing, disruptive innovation, Bitcoin/ARK thesis |
| `How To Start Day Trading As A Beginner In 2026.md` | TJR | Confluence framework (BOS, FVG, order blocks, SMT divergence) |
| `Scalper.md` | Chart Fanatics / Fabio Valentina | Volume profile, order flow, trend following + mean reversion models |
| `StockBee.md` | Pradeep Bondi | Episodic pivots, execution discipline, sell-into-strength framework |
| `yt-action-plan.md` | Duplicate of StockBee | Same content |
| `The Simplest Way To Start Day Trading In 2026.md` | Unknown educator | Hybrid Super Scalping Strategy, Heikin Ashi, 100 EMA |
| `How To Start Day Trading in 2026 (full training).md` | Ross Cameron / Warrior Trading | 9-step framework, momentum trading, small cap selection |
| `how I plan to make millions investing in crypto 2026 (again).md` | Crypto investor | BTC price predictions, macro cycles, altcoin rotation, BTC dominance |
| `How To Start Day Trading For Beginners In 2026 (FULL COURSE).md` | Juvier | Full course: candlesticks, indicators, strategies, prop firms, psychology |
| `Crypto Trading Full Course for beginners.md` | Pakistani educator (translated from Hindi/Urdu) | Crypto fundamentals, spot/futures, TA/FA, risk management |

---

## System Architecture

```
src/
├── main.rs              # Entry point — CLI arg parsing (report subcommand)
├── engine.rs            # Main trading loop — data fetch, signal eval, execution
├── lib.rs               # Library root
├── core/
│   ├── config.rs        # TOML config with validation
│   ├── types.rs         # Candle, Signal, Position, Order, VolumeProfile, AccountState
│   ├── events.rs        # Channel-based event bus
│   └── error.rs         # Typed error hierarchy (thiserror)
├── data/
│   ├── kraken.rs        # Kraken REST client (OHLCV, ticker)
│   ├── market_data.rs   # Sliding-window candle store
│   ├── orderbook.rs     # Order book depth + imbalance
│   └── indicators.rs    # EMA, SMA, RSI, ATR, ADX, VWAP, Volume Profile
├── strategy/
│   ├── base.rs          # Strategy trait (async)
│   ├── momentum.rs      # Break of structure + volume breakout
│   ├── mean_reversion.rs # Volume profile mean reversion to POC
│   └── regime.rs        # ADX-based regime detection
├── risk/
│   ├── position.rs      # Fixed-fractional position sizing with R:R filter
│   ├── stop_loss.rs     # ATR-based + structure-based stops, break-even trigger
│   └── circuit_breaker.rs # Daily loss limit, max drawdown, max positions
├── execution/
│   ├── engine.rs        # ExecutionEngine trait
│   └── paper.rs         # Paper trading simulator with fee/slippage
└── monitor/
    ├── journal.rs       # SQLite trade journal + equity snapshots + daily summary
    ├── metrics.rs       # Win rate, profit factor, expectancy, max drawdown
    └── report.rs        # CLI report generation
```

---

## Config (config/default.toml)

```toml
[exchange]
name = "kraken"
ws_url = "wss://ws.kraken.com/v2"
rest_url = "https://api.kraken.com"

[trading]
pairs = ["BTC/USD", "ETH/USD"]
timeframe = "5m"
base_currency = "USD"
starting_balance = 100.0
database_url = "sqlite:data/savant.db"
fee_rate = 0.0026        # Kraken taker fee
slippage_pct = 0.0005    # 0.05% slippage assumption

[risk]
max_risk_per_trade = 0.01
max_daily_loss = 0.03
max_drawdown = 0.10
max_positions = 3
min_rr_ratio = 1.5

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
adx_trending_threshold = 25.0
adx_ranging_threshold = 20.0
atr_volatility_multiplier = 1.5

[mode]
paper_trading = true
```

---

## Three Trading Strategies

1. **Momentum Breakout** (from Ross Cameron + TJR)
   - 100 EMA as trend filter
   - Consolidation range detection (ATR compression)
   - Volume spike > 2x average confirms breakout
   - Entry on break of structure with volume
   - Stop at range midpoint, TPs at 1:1, 1:2, 1:3 R:R

2. **Mean Reversion to POC** (from Fabio Valentina)
   - Volume profile calculates Value Area (70% of volume) and Point of Control
   - Entry when price extends beyond VAL/VAH and reverts back inside
   - Stop beyond the extreme, TP at POC
   - R:R typically 1:2 to 1:4

3. **Regime Detector** (ADX-based, from all sources)
   - ADX > 25 = Trending → use Momentum
   - ADX < 20 = Ranging → use Mean Reversion
   - ATR > 1.5x average = Volatile → reduce size

---

## Risk Management (Composite from ALL Sources)

| Rule | Value | Source |
|------|-------|--------|
| Max risk per trade | 1% of account | TJR, Fabio |
| Max daily loss | 3% of account | All sources |
| Max drawdown | 10% of account | Fabio |
| Max concurrent positions | 3 | Portfolio diversification |
| Scale-out | 50% @ 1:1, 30% @ 1:2, 20% @ 1:3 | Pradeep, Fabio |
| Break-even trigger | After 1R profit | Fabio |
| Min R:R | 1:1.5 | Ross Cameron |
| Fee rate | 0.26% (Kraken taker) | Kraken docs |
| Slippage | 0.05% | Conservative estimate |

---

## FID Backlog (Findings Implementation Documents)

All in `dev/findings/`:

| FID | Title | Status | Severity |
|-----|-------|--------|----------|
| 001 | ECHO Protocol Violations (audit) | ✅ Closed | high |
| 002 | Paper Trading Persistence & Reporting | ✅ Closed | medium |
| 003 | Fee & Slippage Modeling | ✅ Closed | high |
| 004 | Trailing Stops | 📋 Open | medium |
| 005 | Scale-Out Execution | 📋 Open | medium |
| 006 | FVG & Order Block Detection | 📋 Open | medium |
| 007 | Sentiment Data (BTC Dom + Fear/Greed) | 📋 Open | medium |
| 008 | Backtesting Engine | 📋 Open | high |
| 009 | WebSocket Real-Time Data | 📋 Open | high |
| 010 | Multi-Timeframe Analysis | 📋 Open | medium |
| 011 | Unit Tests | 📋 Open | medium |
| 012 | Rate Limiting | 📋 Open | low |

### Recommended Implementation Order

1. **004** Trailing Stops — core risk management from transcripts
2. **005** Scale-Out Execution — partial exits at TP1/TP2/TP3
3. **006** FVG & Order Block Detection — strengthen strategy confluence
4. **007** Sentiment Data — BTC dominance + Fear/Greed API
5. **008** Backtesting Engine — validate strategies on history
6. **009** WebSocket Real-Time Data — replace REST polling
7. **010** Multi-Timeframe Analysis — requires WebSocket first
8. **011** Unit Tests — validate all calculations
9. **012** Rate Limiting — needed for WebSocket

Each FID has detailed steps, affected components, and verification criteria. Read the FID file before implementing.

---

## Build Status

All three validation commands pass clean:
- `cargo build` — zero errors, zero warnings
- `cargo clippy -- -D warnings` — zero errors, zero warnings
- `cargo fmt --check` — no formatting issues

---

## How to Run

```bash
# Paper trading mode (default)
cargo run --bin savant

# View report (after trades have been recorded)
cargo run --bin savant -- report
```

---

## What's NOT Done Yet

1. **Kraken API keys** — User needs to create a Kraken account and generate API keys from Settings → API at pro.kraken.com (free). Keys are only needed for live trading, not paper trading.
2. **Trailing stops** — FID-004 open
3. **Scale-out execution** — FID-005 open
4. **FVG/OB detection** — FID-006 open
5. **Sentiment data** — FID-007 open
6. **Backtesting** — FID-008 open
7. **WebSocket** — FID-009 open
8. **Multi-timeframe** — FID-010 open
9. **Tests** — FID-011 open
10. **Rate limiting** — FID-012 open
11. **Live trading engine** — `live.rs` was removed (was a placeholder). Needs proper implementation when user is ready for live trading.

---

## ECHO Protocol Notes

The agent violated the protocol multiple times during this session:
- Built code without presenting first (Law 2)
- Used `unwrap()` in non-test code (Law 6)
- Created placeholder stubs (Law 5)
- Swallowed errors (Law 14)
- Skipped Perfection Loop on FIDs before implementing

These were caught and fixed in FID-001. **The protocol must be followed strictly going forward.** Every FID must go through the Perfection Loop before implementation. Every step must be verified before moving to the next.

---

## Key Design Decisions

1. **Rust** — User chose for performance
2. **Kraken** — User chose, free tier, in Kentucky (US restricted state)
3. **Paper trading first** — User wants to validate before risking real money
4. **$50-100 starting capital** — Small account, needs micro-position sizing
5. **5m timeframe** — Default, configurable
6. **SQLite for persistence** — Lightweight, no external DB needed
7. **No new dependencies** — Used existing sqlx + std::env::args for report
