# Savant Trading — Autonomous AI Crypto Trading Engine

**Full Technical Report & Paper Training Results**

---

## What Is Savant?

Savant is a **fully autonomous AI-powered cryptocurrency trading engine** built in Rust. It trades on Kraken exchange, 24/7, with zero human intervention. The AI agent — powered by mimo v2.5 pro (a reasoning model on par with Claude Opus) — makes every trading decision: when to enter, when to exit, how much to risk, and why.

This is not a simple algorithmic bot with `if RSI < 30 then buy`. Savant reads market data the same way a human trader would — candle patterns, order book depth, on-chain analytics, funding rates, sentiment indices — then reasons about what it sees using a 560-line identity document (SOUL.md) that defines its trading personality, risk rules, and decision framework.

**The key differentiator:** Savant learns. Every decision it makes — whether it trades or holds — is captured in an episodic memory database. Over time, it builds a calibration curve, tracks which strategies work in which market conditions, identifies its own anti-patterns, and generates lessons from its failures. It's designed to get smarter with every cycle.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    SAVANT TRADING ENGINE                      │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Data Layer   │  │  AI Brain    │  │  Execution Layer  │   │
│  │              │  │              │  │                   │   │
│  │ • Kraken REST│  │ • mimo v2.5  │  │ • Paper Trader    │   │
│  │ • Kraken WS  │──│ • SSE Stream │──│ • Scale-out TP    │   │
│  │ • 5m/1H/4H   │  │ • 560-line   │  │ • Dynamic Slippage│   │
│  │ • Order Book │  │   SOUL.md    │  │ • Maker/Taker     │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Knowledge    │  │  Memory      │  │  Risk Management  │   │
│  │              │  │              │  │                   │   │
│  │ • 2,959 units│  │ • SQLite WAL │  │ • Circuit Breaker │   │
│  │ • 10 JSON    │  │ • Brier Score│  │ • Portfolio Heat  │   │
│  │ • MMR select │  │ • CUSUM      │  │ • Correlation     │   │
│  │ • Utility    │  │ • Experience │  │ • Max Drawdown    │   │
│  │   scoring    │  │   Replay     │  │ • Daily Loss      │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Insight      │  │  Training    │  │  Infrastructure   │   │
│  │              │  │              │  │                   │   │
│  │ • Fear/Greed │  │ • Random     │  │ • REST API (16ep) │   │
│  │ • Funding    │  │   scenarios  │  │ • SQLite Backup   │   │
│  │ • On-chain   │  │ • Semantic   │  │ • WS Exp. Backoff │   │
│  │ • Sentiment  │  │   patterns   │  │ • CORS            │   │
│  │ • Liquidation│  │ • Anti-      │  │ • Graceful Ctrl+C │   │
│  └──────────────┘  │   patterns   │  └──────────────────┘   │
│                     │ • GEPA       │                          │
│                     │   mutation   │                          │
│                     └──────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

---

## How It Works — Decision Pipeline

Every 5 minutes (one candle period), the engine runs a full decision cycle:

### Phase 1: Data Collection

1. Fetch 721 candles (5m) from Kraken REST API for each active pair
2. Fetch order book depth (top 5 bid/ask levels)
3. Aggregate 1H higher-timeframe candles from 5m data
4. Calculate technical indicators: RSI, EMA (9/21), ADX, ATR, VWAP, Volume Profile

### Phase 2: Knowledge Selection

1. Determine market conditions (Trending/Ranging, Fear/Greed extremes, on-chain signals)
2. Generate context tags from indicators (e.g., `regime_subtype:capitulation`, `setup_type:breakout`)
3. Select top 20 knowledge units from 2,959 using MMR (Maximal Marginal Relevance) with utility scoring
4. Inject semantic patterns and anti-patterns from memory

### Phase 3: AI Reasoning

1. Compose 5-layer system prompt: Base Identity + Risk Constraints + Knowledge + Strategy + SOUL.md
2. Add 6th layer: Dynamic Memory Context (win rates, recent episodes, Brier score, anti-patterns)
3. Send to mimo v2.5 pro via SSE streaming
4. Agent reasons about the market and outputs structured JSON: action, entry, stop, 3 take-profits, confidence, reasoning

### Phase 4: Decision & Execution

1. Parse decision JSON (3-pass parser handles truncated/invalid responses)
2. Execute paper trade or hold
3. Capture full episode snapshot to SQLite
4. Update circuit breaker, portfolio heat, equity curve

### Phase 5: Learning

1. Semantic consolidation: SQL aggregations extract patterns from episode history
2. Anti-pattern detection: identify conditions where win rate < 30%
3. Auto-lesson generation: high-conviction failures become experience replay lessons
4. Brier score recalibration: confidence vs accuracy tracking

---

## The Knowledge Base — 2,959 Trading Rules

The agent's trading knowledge comes from 150+ books and research transcripts, organized into 10 enterprise-grade JSON files:

| File | Units | Domain |
|------|-------|--------|
| `knowledge_technical_analysis.json` | 506 | RSI, EMA, ADX, MACD, Bollinger, Fibonacci |
| `knowledge_risk_management.json` | 350 | Kelly Criterion, drawdown recovery, position sizing |
| `knowledge_crypto_native.json` | 319 | On-chain analytics, DeFi, funding rates, liquidation |
| `knowledge_psychology.json` | 319 | Cognitive biases, tilt, deliberate practice |
| `knowledge_sentiment.json` | 291 | Fear & Greed, social sentiment, crowd psychology |
| `knowledge_execution.json` | 282 | Order types, slippage, fill optimization |
| `knowledge_market_regimes.json` | 250 | Trending, ranging, volatile, capitulation |
| `knowledge_trading_systems.json` | 226 | Backtesting, walk-forward, Monte Carlo |
| `knowledge_price_action.json` | 216 | Wyckoff, candle patterns, support/resistance |
| `knowledge_fundamentals.json` | 200 | Macro analysis, halving cycles, ETF flows |

Each unit is tagged with `setup_type`, `regime_subtype`, `trigger`, `indicator`, and `risk_context` for precise matching. A `utility_score` field tracks empirical correlation with successful trades.

---

## SOUL.md — The Agent's Identity

The agent's 560-line personality document defines:

- **Who it is**: "You are Savant, a methodical and disciplined autonomous crypto trading agent"
- **Risk rules**: Max 10% per trade, 3% daily loss limit, 10% drawdown kill switch
- **Action Triggers**: 5+ triggers must align before entry (MVRV, SOPR, funding rate, Fear/Greed, EMA, ADX)
- **Regime-specific behavior**: Different strategies for trending, ranging, volatile, and crisis markets
- **Immutable sections**: Core identity and risk constraints cannot be mutated
- **Mutable sections**: Action triggers and tactical rules can evolve via GEPA optimization

Key rule: "When in doubt, stay out. Missing a trade costs nothing. A bad trade costs everything."

---

## SOUL.md Example — Decision Reasoning

Here's an actual agent decision from training:

```
Scenario: Parabolic Exhaustion (MVRV=3.80, Funding=0.15%/8hr, Fear/Greed=Euphoria)

Agent Decision: SELL SHORT @ $79,118 | Confidence: 85% | R:R 3.4

Reasoning: "5+ sell triggers aligned at extreme euphoria levels. MVRV=3.80
(well above 3.5 euphoria threshold), funding=0.150%/8hr (164% annualized,
massively overleveraged longs), Fear & Greed in euphoria zone. On-chain
data confirms distribution phase with whale wallet outflows. This is a
textbook euphoria top — historical precedent shows 30-50% corrections from
these levels. Conservative bias SUSPENDED per Section 13.5 trigger override."
```

---

## Training System — How It Learns

### The Training Pipeline

Savant uses a **closed-loop training system** with random scenario generation:

1. **Random Scenario Generator** — Creates 60 unique market scenarios per run with randomized:
   - MVRV (0.3–5.0), SOPR (0.7–1.3), funding rate (-0.5% to +1.5%)
   - Fear & Greed (0–100), BTC dominance (30–80%)
   - Trend direction (Bull/Bear/Sideways with random strength)
   - Volatility regime (Low/Normal/High/Extreme)
   - Market events (Flash Crash, Short Squeeze, Gap Up/Down)
   - Expected actions derived from the mock data

2. **Same Path as Live** — Every scenario goes through the EXACT same `build_context()` function as live trading. Same prompt composition, same knowledge selection, same indicator calculation.

3. **Episodic Memory** — Every decision is captured to `test_memory.db` with full market context, cognitive state, and outcome.

4. **Semantic Consolidation** — SQL aggregations extract patterns: which market conditions produce the best results.

5. **Anti-Pattern Detection** — Identifies conditions where the agent consistently fails (win rate < 30%).

6. **Auto-Lessons** — High-conviction failures (>70% confidence, wrong) automatically generate experience replay lessons.

7. **Convergence Detection** — Training stops when Brier score improvement < 0.02 for 3 consecutive runs.

### Training Results

**25 training runs completed** across 2 sessions:

| Session | Scenarios | Runs | Episodes | Brier Score |
|---------|-----------|------|----------|-------------|
| Static (same 60) | 60 × 21 | 21 | ~1,000 | 0.50–0.58 |
| Random (unique) | 60 × 4 | 4 | ~240 | 0.29–0.31 |
| **Total** | | **25** | **1,368** | **0.30 converged** |

**Key finding:** Random scenarios dramatically improved learning. Static scenarios produced Brier ~0.50 (barely better than random). Random scenarios hit Brier 0.29 in just 4 runs.

---

## Paper Training Results — Full Audit

### Overview

| Metric | Value |
|--------|-------|
| Total Episodes | 1,368 |
| Trades Taken | 486 (35.5%) |
| Holds | 882 (64.5%) |
| Starting Balance | $50.00 |
| Final Balance | $2,299.15 |
| Total P&L | +$2,249.15 (+4,498%) |
| Win Rate | 59.3% (288 wins / 198 losses) |
| Profit Factor | 3.27 |
| Avg Win | $11.25 |
| Avg Loss | $5.00 |
| Expectancy | $4.63 per trade |
| Max Drawdown | 53.9% |

### Conviction Calibration

| Conviction | Trades | Win Rate | Avg Confidence |
|------------|--------|----------|----------------|
| HIGH | 295 | 59.7% | 78% |
| MEDIUM | 273 | 50.5% | 60% |
| LOW | 144 | 56.9% | 28% |

The agent's HIGH conviction trades win 59.7% of the time at 78% average confidence. This is reasonable calibration — slightly overconfident but not dangerously so.

### Confidence Calibration Curve

| Confidence Bucket | Trades | Actual Win% | Avg Confidence | Calibration Error |
|-------------------|--------|-------------|----------------|-------------------|
| 0–20% | 686 | 53.2% | 1% | 0.53 |
| 20–40% | 114 | 54.4% | 31% | 0.23 |
| 40–60% | 100 | 38.0% | 53% | 0.15 |
| 60–80% | 347 | 57.3% | 69% | 0.12 |
| 80–100% | 121 | 63.6% | 84% | 0.21 |

The 60–80% bucket is the best calibrated (error 0.12). The 0–20% bucket has high error because holds (which default to 0% confidence) are being graded against expected actions that may differ.

### Category Edge Analysis

**Strong Categories (win rate >60%):**

| Category | Trades | Win Rate | What It Tests |
|----------|--------|----------|---------------|
| Microstructure | 123 | **81.3%** | Order book manipulation, spread widening, thin books |
| Session | 121 | **80.2%** | Asian low volume, US open surge, weekend wicks |
| Edge Case | 138 | **76.1%** | Data fabrication, revenge trade bait, missing stop loss |
| Catalyst | 105 | **61.9%** | FOMC rate hike, ETF approval, regulatory action |
| On-Chain | 100 | **61.0%** | Whale accumulation, miner capitulation, NVT divergence |

**Weak Categories (win rate <45%):**

| Category | Trades | Win Rate | What It Tests |
|----------|--------|----------|---------------|
| Trend Bull | 181 | **29.8%** | Clean breakout, EMA pullback, golden cross |
| Sentiment | 79 | **29.1%** | Extreme fear, extreme greed, SOPR reset |
| Correlation | 140 | **28.6%** | Broad market rally, contagion dump, sector rotation |
| Trend Bear | 121 | **43.8%** | Bear flag breakdown, slow bleed, support break |

### Regime Edge

| Regime | Trades | Win Rate |
|--------|--------|----------|
| Ranging | 823 | 56.3% |
| Trending | 545 | 51.0% |

### Top Patterns by Win Rate (N>=3)

| Pattern | N | Win Rate |
|---------|---|----------|
| Trending + Session | 50 | **88.0%** |
| Ranging + Microstructure | 109 | **87.2%** |
| Trending + Volatility | 49 | **83.7%** |
| Ranging + Edge Case | 63 | **77.8%** |
| Trending + Edge Case | 75 | **74.7%** |
| Ranging + Session | 71 | **74.6%** |
| Ranging + Catalyst | 75 | **72.0%** |

### Anti-Patterns (High Confidence Failures)

| Pattern | Failures | Avg Confidence |
|---------|----------|----------------|
| Ranging + conf=65% | 38 | 65% |
| Ranging + conf=72% | 21 | 72% |
| Trending + conf=85% | 16 | 85% |
| Ranging + conf=85% | 10 | 85% |

The agent's biggest weakness: **65–85% confidence trades in ranging markets**. The agent enters expecting a breakout that doesn't come. This is being tracked and will be injected as anti-pattern constraints in future prompts.

### Lessons Learned (Auto-Generated)

105 experience replay lessons generated, all from high-conviction failures:

- "Expected Buy but agent did Hold — capitulation signals conflicted with extreme funding rate"
- "Expected Hold but agent traded — entered during ranging regime without volume confirmation"
- "Expected Sell but agent held — euphoria signals present but agent cited conflicting on-chain data"

---

## What Makes Savant Different

| Feature | Traditional Bot | Savant |
|---------|----------------|--------|
| Decision engine | if/else rules | LLM reasoning with 2,959 knowledge units |
| Market understanding | Single indicator | Multi-timeframe, on-chain, sentiment, order book |
| Risk management | Fixed stop loss | Dynamic circuit breakers, portfolio heat, correlation |
| Learning | None | Episodic memory, Brier calibration, anti-pattern detection |
| Adaptation | Manual tuning | GEPA self-mutation of strategy rules |
| Transparency | Black box | Full reasoning trace for every decision |
| Knowledge | Coded rules | 150+ books distilled into tagged, prioritized units |

---

## Current Status & Next Steps

### What's Done

- ✅ Full trading engine (Kraken REST + WebSocket)
- ✅ AI brain with SSE streaming (mimo v2.5 pro)
- ✅ 2,959 knowledge units with MMR selection
- ✅ Episodic memory with Brier score calibration
- ✅ Semantic consolidation and anti-pattern detection
- ✅ 105 auto-lessons from training failures
- ✅ Random scenario training pipeline with convergence detection
- ✅ Full reporting system with P&L simulation
- ✅ 1,368 episodes captured, Brier 0.30 converged

### What's Next

- 🔲 Wire knowledge utility score persistence
- 🔲 Implement GEPA self-mutation loop (agent evolves its own SOUL.md)
- 🔲 Run live paper trading with real Kraken data
- 🔲 Expand scenario library to 200+ (add more edge cases, more market regimes)
- 🔲 Fix weak categories: Trend Bull (29.8%), Sentiment (29.1%), Correlation (28.6%)
- 🔲 Target: Brier < 0.20, all categories > 50% win rate

### The Goal

An agent that runs 24/7/365, learns from every decision, evolves its own strategy, and maintains a profitable edge across all market conditions. Not a bot that follows rules — a trader that reasons.

---

## Technical Details

- **Language:** Rust 1.91
- **Database:** SQLite WAL (concurrent reads + single writer)
- **AI Provider:** OpenGateway (mimo v2.5 pro, 1M context window)
- **Exchange:** Kraken (REST + WebSocket v2)
- **API:** Axum REST server with 16 endpoints + CORS
- **Version:** 0.4.3
- **Tests:** 127 passing, zero clippy warnings
- **License:** Proprietary — All Rights Reserved

---

*Report generated 2026-06-01. Training data from 25 runs across 2 sessions, 1,368 episodes, 486 simulated trades.*
