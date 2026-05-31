# Gemini Deep Research Prompt — Savant Trading Agent Sandbox & Stress Testing System

## Research Objective

Design a comprehensive sandbox and stress testing system for an autonomous crypto trading agent that uses the REAL AI brain (mimo v2.5 pro via OpenGateway) to evaluate thousands of market scenarios, grade every decision against a rubric, identify weaknesses, and generate actionable improvement reports. This is NOT a backtesting engine — it's a "trading dojo" where the agent practices against curated scenarios and gets graded like a student taking an exam.

## Why This Exists

The agent has a 560-line SOUL.md persona, 265 knowledge units, and a decision framework. But we have NO way to know if it actually follows its own rules under pressure. Paper trading on live markets is too slow to test edge cases — we might wait weeks for a flash crash or alt season to naturally occur. The sandbox lets us THROW every possible market condition at the agent and see how it responds.

The sandbox serves three purposes:

1. **Validate** — Does the agent follow its SOUL.md under all conditions?
2. **Identify** — Where does the agent perform poorly? (specific regimes, sessions, conditions)
3. **Refine** — Feed grading results back into SOUL.md, knowledge base, and memory system

## Context

### What Savant Is

Savant is a Rust-native autonomous crypto trading engine:

- AI brain: mimo v2.5 pro via OpenGateway (free, unlimited, 1M context window)
- Trades any liquid pair on Kraken
- $50 paper budget, scaling to live
- Parallel evaluation (all pairs simultaneously, ~30s per cycle)
- SOUL.md persona (560 lines) with 10-point pre-trade checklist
- 265 knowledge units from 22 curated sources
- Real-time TUI dashboard
- Obsidian vault integration

### How the Agent Makes Decisions

Each cycle (~7.5 min for 15 pairs), the engine runs in 3 phases:

**Phase 1: Data fetch (sequential, ~8s for 15 pairs)**

- Fetch 721 candles from Kraken REST per pair
- Fetch order book (10 levels) per pair
- Compute indicators: EMA(9), EMA(21), RSI(14), ATR(14), ADX(14), VWAP, Volume Profile
- Detect regime: ADX > 25 = Trending, ADX < 20 = Ranging, ATR > 1.5x avg = Volatile
- Fetch insight: Fear & Greed, funding rates, on-chain (MVRV/SOPR/NVT), RSS news
- Build FullContext per pair (candles, indicators, regime, volume profile, insight, positions, account, order book imbalance, session)

**Phase 2: Parallel LLM evaluation (~30s for 15 pairs)**

- Build system prompt: SOUL.md (560 lines) + knowledge injection (8000 char budget) + risk constraints + output format
- Build user message: market data + indicators + regime + session + volume profile + order book + insight + on-chain + RSS news (~13K chars)
- Send all 15 pairs simultaneously via tokio::JoinSet
- Each call returns JSON decision

**Phase 3: Sequential execution**

- Parse LLM response (extract JSON from markdown, normalize action/side casing)
- Log ALL decisions (including Hold) to SharedEngineData
- Validate AI stop loss against ATR bounds (structure_stop fallback)
- Execute if autonomous + passes circuit breaker
- Apply session multiplier to position size
- Check stops for all positions (scale-out: TP1→50%, TP2→30%, TP3→20%)

### LLM Response JSON Schema

```json
{
  "action": "Buy|Sell|Hold|Close|AdjustStop",
  "pair": "BTC/USD",
  "side": "Long|Short",
  "entry_price": 103500.0,
  "stop_loss": 102500.0,
  "take_profit_1": 105000.0,
  "take_profit_2": 106500.0,
  "take_profit_3": 108000.0,
  "confidence": 0.75,
  "risk_reward": 2.1,
  "reasoning": "Strong support at 103000 with bullish EMA crossover..."
}
```

The parser handles: markdown code blocks, UPPERCASE action/side normalization, empty side fields (defaults to Long).

### Key Data Structures

```rust
// Indicator values computed from candles
pub struct IndicatorValues {
    pub ema_fast: Option<f64>,    // EMA(9)
    pub ema_slow: Option<f64>,    // EMA(21)
    pub rsi: Option<f64>,         // RSI(14)
    pub atr: Option<f64>,         // ATR(14)
    pub adx: Option<f64>,         // ADX(14)
    pub vwap: Option<f64>,        // VWAP
    pub volume_sma: Option<f64>,  // 20-period volume SMA
}

// Regime detection
pub enum MarketRegime { Trending, Ranging, Volatile }

// Order book
pub struct OrderBook {
    pub pair: String,
    pub bids: Vec<OrderBookLevel>,  // {price, volume}
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: DateTime<Utc>,
}

// Sentiment data
pub struct SentimentData {
    pub fear_greed_index: Option<i32>,     // 0-100
    pub fear_greed_label: Option<String>,  // Extreme Fear/Fear/Neutral/Greed/Extreme Greed
    pub btc_dominance: Option<f64>,
}

// Funding data
pub struct FundingData {
    pub funding_rate: Option<f64>,           // per-8hr rate
    pub funding_rate_annualized: Option<f64>,
    pub open_interest: Option<f64>,
    pub mark_price: Option<f64>,
}

// On-chain data
pub struct OnchainData {
    pub mvrv: Option<f64>,
    pub nupl: Option<f64>,
    pub sopr: Option<f64>,
    pub nvt_signal: Option<f64>,
    pub exchange_balance: Option<f64>,
    pub exchange_net_flow_24h: Option<f64>,
}

// Position sizer
pub struct PositionSizer {
    pub max_risk_per_trade: f64,  // 0.01 = 1%
    pub min_rr_ratio: f64,       // 1.5
}

// Circuit breaker
pub struct CircuitBreaker {
    pub max_daily_loss: f64,   // 0.03 = 3%
    pub max_drawdown: f64,     // 0.10 = 10%
    pub max_positions: usize,  // 3
}

// Scale-out levels
pub enum ScaleLevel { Full, Scaled50, Scaled80, Closed }
// Full → TP1 hit → close 50%, move SL to break-even, advance to Scaled50
// Scaled50 → TP2 hit → close 60% of remaining, advance to Scaled80
// Scaled80 → TP3 hit → close 100%, advance to Closed
```

### SOUL.md Quick Reference Card

```text
BEFORE EVERY TRADE
Regime classified? ✓
Thesis stated (2 sentences)? ✓
Invalidation level defined? ✓
Stop loss set? ✓
Target set (R/R >= 1.5:1)? ✓
Size within protocol? ✓
Correlation limit check? ✓
Catalyst risk check? ✓

SIZING: HIGH=1.5% | MEDIUM=1.0% | LOW=0.5% | NONE=0%

DRAWDOWN: -5% Review | -10% Reduce | -15% Pause | -20% Alert
CIRCUIT BREAKER: 2% daily→half | 3% daily→close all | 5% weekly→stop 48h

REGIME FLAGS: Funding>0.05%/8hr overleveraged | MVRV>3.5 euphoria | MVRV<1.0 capitulation
NEVER: No stop | Move stop against | Revenge trade | Chase | >2% single | >6% total
```

### Existing Backtesting Engine (Reusable Components)

The project already has a backtesting engine at `src/backtest/`:

- `engine.rs` — replays candles through Strategy trait, tracks P&L
- `metrics.rs` — Sharpe ratio, max drawdown, win rate, profit factor, expectancy
- `walk_forward.rs` — rolling window optimization with cumulative balance

The sandbox can reuse: candle replay logic, metrics calculation, equity curve tracking, trade recording.

### Existing Insight Data Pipeline

The insight aggregator fetches from free APIs (no keys required):

- Fear & Greed: alternative.me API
- BTC Dominance: CoinGecko API
- Funding rates: Kraken Futures API (per-8hr, with annualized calculation)
- Liquidation risk: Kraken Futures API
- On-chain (MVRV/SOPR/NVT): CoinMetrics Community API (CoinGecko fallback)
- RSS news: 15 crypto RSS feeds, scored by relevance to current pairs

The sandbox needs to MOCK these APIs with scenario-specific data.

### What the SOUL.md Enforces

Pre-trade checklist (10 points — ALL required):

1. Regime classified
2. Thesis stated (1-2 sentences)
3. Invalidation level defined
4. Entry price/zone
5. Stop loss with reasoning
6. Target with R/R >= 1.5:1
7. Size within protocol (HIGH=1.5%, MEDIUM=1.0%, LOW=0.5%)
8. Conviction level (HIGH/MEDIUM/LOW/NONE)
9. Correlation limit check
10. Catalyst risk check

Operational constraints (13 "never" rules):

- Never trade without a stop
- Never move stop against position
- Never revenge trade
- Never chase entries
- Never exceed 2% on one trade
- Never exceed 6% total exposure
- Never fabricate data
- Never hide losses
- Etc.

Circuit breakers:

- 2% daily loss → cut size 50%
- 3% daily loss → close all
- 5% weekly loss → stop 48 hours
- 10% from peak → block file, full stop

## What I Need Researched

### 1. Scenario Design — What to Test

Research how to design comprehensive market scenarios for stress testing a trading agent:

**Specific questions:**

- What are the distinct market "events" that a crypto trader must handle? (List ALL of them)
- How do you categorize scenarios by type? (trend, volatility, catalyst, microstructure)
- What makes a scenario "stressful" vs "normal"?
- How do you design scenarios that test SPECIFIC aspects of the SOUL.md?
- What scenarios would reveal if the agent is revenge trading, chasing, or over-sizing?
- How do you test the agent's response to data it's never seen before?
- What scenarios test the agent's "when NOT to trade" discipline?

**Scenario categories to research:**

- Trend scenarios (strong bull, strong bear, weak trend, trend reversal)
- Volatility scenarios (compression, expansion, flash crash, squeeze)
- Range scenarios (tight range, wide range, range breakout, false breakout)
- Catalyst scenarios (FOMC, CPI, halving, ETF approval, exchange hack, regulatory)
- Microstructure scenarios (liquidation cascade, funding spike, order book manipulation)
- Session scenarios (Asian low volume, US open surge, weekend wick)
- Correlation scenarios (BTC dumps, alts hold; BTC dumps, alts dump harder)
- Sentiment scenarios (extreme fear, extreme greed, rapid sentiment shift)
- On-chain scenarios (exchange outflow spike, whale movement, SOPR reset)
- Edge cases (API failure during position, exchange halt, flash crash + circuit breaker)

### 2. Scenario Generation — How to Create Realistic Market Data

Research how to generate realistic market data for each scenario type:

**Specific questions:**

- How do you generate realistic OHLCV candles that match a scenario description?
- Should you use historical data (replay real events) or synthetic data (generate from parameters)?
- How do you ensure synthetic data looks realistic (not random noise)?
- What statistical properties must synthetic candles preserve? (volatility clustering, volume profiles, gap behavior)
- How do you inject specific events into a baseline trend? (e.g., "flash crash at candle 50")
- How do you generate order book snapshots that match the scenario?
- How do you generate funding rate, Fear & Greed, and on-chain data for each scenario?
- Should scenarios be deterministic (same every run) or stochastic (randomized)?

**Data generation approaches to research:**

- Historical replay (use real Kraken data from specific dates)
- Parameterized generation (define trend, volatility, event timing as parameters)
- Agent-based simulation (simulate market participants)
- GAN-generated candles (use ML to generate realistic price data)
- Monte Carlo with constraints (random walk with scenario-specific boundaries)

### 3. Decision Grading Rubric — How to Score the Agent

Research how to grade trading decisions systematically:

**Specific questions:**

- What makes a trading decision "good" vs "bad"? (It's not just P&L)
- How do you grade a HOLD decision? (Was it correct to NOT trade?)
- How do you grade decisions independently of outcome? (Good decision + bad luck = still good)
- What are the key dimensions to grade? (timing, sizing, risk management, discipline, reasoning quality)
- How do you handle scenarios where the "correct" answer is ambiguous?
- How do you weight different grading dimensions?
- How do you grade the agent's REASONING, not just its action?
- How do you detect if the agent is following its SOUL.md vs ignoring it?

**Grading dimensions to research:**

- Checklist compliance (did it complete all 10 pre-trade points?)
- Risk management score (stop placement, size, R:R, correlation limits)
- Timing score (entry at support/resistance vs chasing)
- Discipline score (followed "never" rules, respected circuit breakers)
- Reasoning quality (was the thesis logical? did it cite data?)
- Outcome score (P&L, but weighted against market conditions)
- Session awareness (did it respect session multipliers?)
- Regime alignment (did it use the right strategy for the regime?)

### 4. Test Harness Architecture — How to Run Thousands of Scenarios

Research how to build a scalable test harness for AI agent evaluation:

**Specific questions:**

- How do you run 1000+ scenarios efficiently when each requires a real LLM call?
- How do you parallelize scenario execution without hitting rate limits?
- How do you manage the LLM context for each scenario? (Same SOUL.md, different market data)
- How do you track results across thousands of scenarios?
- How do you identify patterns in failures? (e.g., "agent fails in 80% of flash crash scenarios")
- How do you compare agent performance across different SOUL.md versions?
- How do you implement A/B testing (current SOUL.md vs modified SOUL.md)?
- How do you handle LLM non-determinism? (Same scenario, different responses)

**Architecture questions:**

- Should scenarios run sequentially or in parallel?
- How do you batch LLM calls efficiently?
- How do you store scenario results? (SQLite schema)
- How do you generate comparison reports?
- Should the sandbox be a CLI tool, a web dashboard, or both?
- How do you implement "regression testing" — re-running scenarios after SOUL.md changes?

### 5. Scenario Library — Curated Test Cases

Research what specific scenarios should be in the library:

**Specific questions:**

- What are the 50 most important crypto trading scenarios to test?
- How do you categorize them by difficulty? (easy/medium/hard/extreme)
- What scenarios test the agent's EDGE cases vs normal operation?
- What scenarios would a prop firm evaluation include?
- What scenarios test the agent's response to its own mistakes? (e.g., "agent entered long, then market crashed — does it cut or hold?")
- What scenarios test multi-pair correlation? (e.g., "BTC dumps 10% — what happens to SOL, ETH, DOGE positions?")
- What scenarios test the agent's response to data anomalies? (e.g., "order book shows massive bid wall — is it real or spoofing?")

**Scenario library structure to research:**

- Scenario metadata (type, difficulty, duration, expected behavior)
- Scenario data (candles, indicators, insight data, order book)
- Scenario expected outcome (what the agent SHOULD do)
- Scenario grading criteria (how to score the agent's response)
- Scenario tags (which SOUL.md rules it tests)

### 6. Feedback Loop — How to Improve Based on Results

Research how to close the loop between grading and improvement:

**Specific questions:**

- How do you translate grading results into specific SOUL.md improvements?
- How do you identify which SOUL.md rules are being violated most?
- How do you implement "targeted training" — running more scenarios for weak areas?
- How do you track improvement over time? (Agent v1 scores 60%, v2 scores 75%)
- How do you prevent overfitting to the test scenarios?
- How do you implement "adversarial scenarios" — scenarios designed to make the agent fail?
- How do you balance exploration (trying new approaches) vs exploitation (following known rules)?
- How do you handle the feedback loop for the memory system? (Memory of past scenarios influencing future decisions)

**Feedback mechanisms to research:**

- Automatic SOUL.md rule adjustment based on violation frequency
- Knowledge unit relevance scoring based on scenario outcomes
- Confidence calibration based on historical accuracy
- Strategy performance tracking by scenario type
- Anti-pattern detection (conditions where agent consistently fails)

### 7. Integration with Memory System

Research how the sandbox interacts with the memory system:

**Specific questions:**

- Should sandbox runs be stored in the same memory system as live trades?
- How do you distinguish sandbox decisions from live decisions?
- Should the agent "remember" sandbox scenarios when making live decisions?
- How do you prevent sandbox memories from contaminating live trading?
- Should the memory system's pattern extraction run on sandbox data?
- How do you use sandbox results to bootstrap the memory system before live trading?

### 8. Reporting and Visualization

Research how to present sandbox results:

**Specific questions:**

- What metrics should be in the sandbox report card?
- How do you visualize the agent's performance across scenarios?
- How do you identify the agent's "blind spots"?
- Should results be in the TUI dashboard, Obsidian vault, or separate report?
- How do you compare performance across SOUL.md versions?
- What's the optimal format for "this is what needs to change" recommendations?

**Report components to research:**

- Overall score (0-100) with breakdown by category
- Scenario-by-scenario results table
- Heatmap: performance by regime × session × strategy
- Violation frequency chart (which SOUL.md rules broken most)
- Improvement trajectory across SOUL.md versions
- Specific recommendations ("Agent over-sizes in Weekend sessions — reduce multiplier from 0.7x to 0.5x")

## Output Format

Produce a comprehensive sandbox design document covering:

1. **Architecture overview** — how all components connect
2. **Scenario design** — categories, generation methods, library structure
3. **Grading rubric** — dimensions, weights, scoring methodology
4. **Test harness** — execution model, parallelization, storage
5. **Scenario library** — 50+ curated scenarios with metadata
6. **Feedback loop** — how results drive improvements
7. **Memory integration** — how sandbox interacts with memory system
8. **Reporting** — dashboard, vault, CLI integration
9. **Implementation roadmap** — what to build first, second, third

## Research Sources to Consult

1. AI agent evaluation frameworks (AgentBench, GAIA, SWE-bench)
2. Trading simulation platforms (TradingView replay, Thinkorswim OnDemand)
3. Prop firm evaluation methodologies (FTMO, TopStep, Apex)
4. Reinforcement learning environment design (OpenAI Gym, DeepMind Lab)
5. Software testing methodologies (property-based testing, fuzzing, mutation testing)
6. Trading journal analytics (Edgewonk, TraderVue, Tradervue)
7. Game AI testing (how game developers test AI opponents)
8. Autonomous vehicle testing (scenario-based validation)
9. Financial stress testing (Basel III, CCAR stress tests)
10. LLM evaluation frameworks (HELM, MMLU, HumanEval)

## Constraints

- Must use the REAL AI brain (mimo v2.5 pro via OpenGateway) — no substitutes
- Must work with Rust + SQLite (existing stack)
- Must be compatible with existing SOUL.md, knowledge units, and memory system
- Must handle LLM non-determinism (same scenario may produce different responses)
- Must support 1000+ scenarios without manual intervention
- Must generate actionable improvement recommendations
- Must track performance across SOUL.md versions
- Must not contaminate live trading data with sandbox data
- Must be runnable as a CLI command (e.g., `savant sandbox --scenarios all`)
- Must complete a full test suite in reasonable time (<2 hours for 100 scenarios)
- Must mock all external APIs (Kraken, CoinGecko, alternative.me, RSS feeds)
- Must reuse existing backtesting engine components (metrics, equity curve, trade recording)
- Must produce results compatible with the memory system (episodic storage)
- Must support regression testing (re-run same scenarios after SOUL.md changes)
