# Research Sources Review — Feature Extraction for Savant Trading

**Date:** 2026-06-11
**Projects Reviewed:** 6 (hermes-agent, kilocode, live-trade-bench, openclaw, QuantDinger, TradingAgents)
**Goal:** Identify concrete, implementable ideas from each project that could enhance savant-trading.

---

## Table of Contents

1. [TradingAgents — Multi-Agent Debate Pipeline](#1-tradingagents--multi-agent-debate-pipeline)
2. [QuantDinger — Data Infrastructure Patterns](#2-quantdinger--data-infrastructure-patterns)
3. [Live Trade Bench — LLM Benchmarking Framework](#3-live-trade-bench--llm-benchmarking-framework)
4. [Hermes Agent — Self-Improving Agent Architecture](#4-hermes-agent--self-improving-agent-architecture)
5. [OpenClaw — Skill-Based Agent Framework](#5-openclaw--skill-based-agent-framework)
6. [KiloCode — Multi-Mode Agent System](#6-kilocode--multi-mode-agent-system)
7. [Cross-Project Patterns](#7-cross-project-patterns)
8. [Prioritized Feature Backlog](#8-prioritized-feature-backlog)

---

## 1. TradingAgents — Multi-Agent Debate Pipeline (Deep Dive)

**Repo:** Multi-agent financial trading framework simulating a real trading firm
**Language:** Python (LangGraph)
**Stars:** Popular academic framework from a research team
**Most Relevant To:** `src/strategy/`, `src/memory/`, `src/risk/`, `src/engine/`

### Full Architecture

TradingAgents decomposes trading decisions into a **9-stage pipeline** of specialized LLM agents, orchestrated via LangGraph's `StateGraph`:

```
┌─────────────────────────────────────────────────────────────────┐
│ STAGE 1: ANALYSTS (sequential, each with tool-calling loop)     │
│   Market Analyst → Sentiment Analyst → News Analyst → Fund.     │
│   Each produces a markdown report stored in AgentState          │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 2: INVESTMENT DEBATE (bull ↔ bear, N rounds)              │
│   Bull Researcher ←→ Bear Researcher                            │
│   Debate count tracked, terminates at 2 × max_debate_rounds    │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 3: RESEARCH MANAGER (synthesizes debate → ResearchPlan)   │
│   Outputs: 5-tier rating + rationale + strategic_actions        │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 4: TRADER (ResearchPlan → TraderProposal)                 │
│   Outputs: action (Buy/Hold/Sell) + entry/stop/position sizing  │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 5: RISK DEBATE (3-way: aggressive ↔ conservative ↔ neutral│
│   Cycles Aggressive → Conservative → Neutral, N rounds          │
│   Terminates at 3 × max_risk_discuss_rounds                    │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 6: PORTFOLIO MANAGER (final approval → PortfolioDecision) │
│   Outputs: 5-tier rating + thesis + price target + time horizon │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 7: SIGNAL PROCESSOR (runs outside graph on final state)   │
│   Extracts rating from PM markdown → Buy/Overweight/Hold/etc.   │
│   Deterministic regex parsing, NOT a graph node                 │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 8: REFLECTION (deferred, runs after outcome is known)     │
│   2-4 sentence lesson stored in markdown memory log             │
├─────────────────────────────────────────────────────────────────┤
│ STAGE 9: MEMORY LOG (append-only, with rotation)                │
│   Past context re-injected into future prompts                  │
└─────────────────────────────────────────────────────────────────┘
```

### Source Module Map

> **Note on naming:** The analyst execution plan uses `"social"` as the key, but it maps to `sentiment_analyst.py`. The old `social_media_analyst.py` is a deprecated shim that re-exports the sentiment analyst. Savant implementers should use "sentiment" terminology, not "social."

```
tradingagents/
├── default_config.py          # Config with env var overrides
├── graph/
│   ├── trading_graph.py       # Main orchestrator (TradingAgentsGraph)
│   ├── setup.py               # StateGraph assembly with edges
│   ├── propagation.py         # Initial state + graph invocation
│   ├── conditional_logic.py   # All branching decisions
│   ├── signal_processing.py   # PM output → trade signal
│   ├── reflection.py          # Post-trade self-improvement
│   ├── checkpointer.py        # SQLite per-ticker persistence
│   └── analyst_execution.py   # Analyst sequencing/parallelism
├── agents/
│   ├── schemas.py             # Pydantic models (ResearchPlan, TraderProposal, etc.)
│   ├── analysts/
│   │   ├── market_analyst.py       # Tool-calling: get_stock_data → get_indicators → get_verified_snapshot
│   │   ├── sentiment_analyst.py    # Pre-fetches Yahoo/StockTwits/Reddit → structured SentimentReport
│   │   ├── news_analyst.py         # Tool-calling: get_news + get_global_news
│   │   └── fundamentals_analyst.py # Tool-calling: get_fundamentals/balance_sheet/cashflow/income
│   ├── researchers/
│   │   ├── bull_researcher.py  # Bullish debate agent
│   │   └── bear_researcher.py  # Bearish debate agent
│   ├── managers/
│   │   ├── research_manager.py # Synthesizes debate → ResearchPlan (structured output)
│   │   └── portfolio_manager.py # Final approval → PortfolioDecision (structured output)
│   ├── trader/
│   │   └── trader.py          # ResearchPlan → TraderProposal (structured output)
│   ├── risk_mgmt/
│   │   ├── aggressive_debator.py  # Champions high-risk opportunities
│   │   ├── conservative_debator.py # Protects assets, minimizes volatility
│   │   └── neutral_debator.py     # Mediates, advocates balance
│   └── utils/
│       ├── agent_states.py    # TypedDict: AgentState, InvestDebateState, RiskDebateState
│       ├── agent_utils.py     # Instrument resolution, language, tool aggregation
│       ├── memory.py          # TradingMemoryLog — append-only markdown with rotation
│       ├── rating.py          # Deterministic 5-tier rating parser
│       ├── structured.py      # bind_structured + invoke_structured_or_freetext
│       ├── core_stock_tools.py           # get_stock_data, get_indicators
│       ├── fundamental_data_tools.py    # get_fundamentals, balance_sheet, cashflow, income
│       ├── news_data_tools.py           # get_news, get_global_news, get_insider_transactions
│       ├── technical_indicators_tools.py # Indicator facade → vendor routing
│       └── market_data_validation_tools.py # get_verified_market_snapshot (source of truth)
├── dataflows/
│   ├── interface.py                # Vendor routing layer (yfinance ↔ alpha_vantage)
│   ├── config.py                   # Data source configuration
│   ├── symbol_utils.py             # Ticker normalization across vendors
│   ├── utils.py                    # Shared data utilities
│   ├── y_finance.py                # Yahoo Finance OHLCV + retry with backoff
│   ├── yfinance_news.py            # Yahoo Finance news fetcher
│   ├── alpha_vantage.py            # Alpha Vantage orchestrator (imports sub-modules)
│   ├── alpha_vantage_common.py     # Shared AV utilities
│   ├── alpha_vantage_stock.py      # AV stock/OHLCV data
│   ├── alpha_vantage_indicator.py  # AV technical indicators
│   ├── alpha_vantage_fundamentals.py # AV financial statements
│   ├── alpha_vantage_news.py       # AV news + insider transactions
│   ├── reddit.py                   # Reddit JSON/RSS fallback for social sentiment
│   ├── stocktwits.py               # StockTwits cashtag stream (retail sentiment)
│   ├── stockstats_utils.py         # Technical indicator computation via stockstats
│   └── market_data_validator.py    # Deterministic snapshot to prevent LLM confabulation
└── llm_clients/
    ├── factory.py             # Provider-specific client creation
    ├── base_client.py         # Abstract interface with content normalization
    ├── capabilities.py        # Model capability lookup (tool_choice, json_mode, etc.)
    └── model_catalog.py       # quick vs deep model tiers per provider
```

### Detailed Borrowable Ideas

#### A. Bull vs Bear Debate Mechanism ⭐⭐⭐
The most novel pattern. Before any trade, a "Bull Researcher" and "Bear Researcher" debate for N rounds (`max_debate_rounds`, default 1). Each side receives identical data (all 4 analyst reports + instrument context) but argues through their bias.

**How it works internally:**
- Bull and Bear alternate turns, each receiving the other's latest argument
- Debate `count` increments each turn; terminates at `2 × max_debate_rounds`
- Each researcher's prompt includes: instrument context, market report, sentiment report, news, fundamentals, full debate history, and the opponent's latest argument
- The prompt explicitly mandates "conversational, engaging style that directly addresses the opponent's points" — not a static data dump
- Both sides update `investment_debate_state` with their `current_response`, `bull_history`/`bear_history`, and cumulative `history`

**Savant Application:** Savant currently makes single-shot LLM decisions. A debate mechanism could be added as a pre-decision step:
- `bull_agent`: Argues FOR the trade using momentum/sentiment data
- `bear_agent`: Argues AGAINST using risk/regime data
- Debate runs 2-3 rounds, then the debate transcript feeds into the final decision
- This would reduce overconfident entries and improve risk-adjusted returns

**Implementation:** New module `src/strategy/debate.rs` — two LLM calls with opposing system prompts, feeding the transcript into the existing decision pipeline.

**Effort:** 3-5 days

#### B. Risk Analyst Trio ⭐⭐⭐
After the trader proposes a trade, three risk analysts debate the proposal in a rotating cycle:

- **Aggressive:** Champions high-reward opportunities, critiques conservative/neutral for being too cautious
- **Conservative:** Protects assets, examines the trader's plan for high-risk components, argues for low-risk adjustments
- **Neutral:** Mediates by challenging both sides — identifies where aggressive is "overly optimistic" and conservative is "overly cautious", advocates for a moderate sustainable strategy

**Cycle logic:** Aggressive → Conservative → Neutral → Aggressive... until `3 × max_risk_discuss_rounds` turns, then routes to Portfolio Manager.

**Savant Application:** Savant has `src/risk/` with circuit breakers but no adversarial review of proposed trades. A lightweight risk debate could catch edge cases the circuit breaker misses.

#### C. 5-Tier Portfolio Rating with Deterministic Parsing ⭐⭐⭐
The Portfolio Manager outputs a structured `PortfolioDecision` with a rating from `PortfolioRating` enum:

| Rating | Meaning | Position Action |
|--------|---------|------------------|
| **Buy** | Strong conviction | Full position |
| **Overweight** | Favorable outlook | Gradually increase |
| **Hold** | Maintain current | No action |
| **Underweight** | Reduce exposure | Partial exit |
| **Sell** | Exit or avoid | Full close |

**Parsing is deterministic** — uses a two-pass regex heuristic:
1. First tries explicit `**Rating**: X` pattern (tolerant of separators and markdown)
2. Falls back to line-by-line keyword search
3. Returns "Hold" if nothing found

No additional LLM call needed for signal extraction.

**Savant Application:** Replace the current binary BUY/SELL/HOLD output with a 5-tier confidence rating. This maps naturally to position sizing.

#### D. Reflection / Self-Improvement Loop ⭐⭐⭐
After a trade closes and outcome is known, the `Reflector` generates a structured lesson:

**Prompt template forces:**
- 2-4 sentences of plain prose (no markdown, bullets, or headers)
- Must cover: (1) whether the directional call was correct (referencing alpha), (2) which part of the thesis held or failed, (3) one concrete lesson for future analyses

**Memory system (`TradingMemoryLog`):**
- Append-only markdown log file, entries separated by `<!-- ENTRY_END -->`
- Pending entries logged on trade; outcome + reflection appended later
- `get_past_context()` retrieves same-ticker lessons + cross-ticker reflections for prompt injection
- Optional log rotation (`memory_log_max_entries`) drops oldest resolved entries
- Atomic writes (temp file → replace) for data integrity

**Savant Application:** Savant has `src/memory/episodic.rs` (stores decisions) and `src/memory/semantic.rs` (stores knowledge). Missing is a **reflection loop** — after a trade closes, an LLM call analyzes "did the thesis hold? why/why not?" and stores the lesson.

**Implementation:** New function in `src/memory/episodic.rs` that triggers on trade close, uses the original reasoning + final P&L to generate a reflection, stored alongside the episode.

#### E. Structured Output with Graceful Fallback ⭐⭐⭐
The `structured.py` utility implements a critical pattern for production LLM systems:

1. **`bind_structured(schema)`** — wraps LLM with `.with_structured_output(schema)` to enforce typed Pydantic responses
2. If provider doesn't support it → catches `NotImplementedError`, returns `None`
3. **`invoke_structured_or_freetext()`** — tries structured call first; if it fails (malformed JSON, provider errors), catches exception and re-runs with plain LLM
4. All outputs rendered back to markdown via `render_*` functions for downstream compatibility

**Schemas defined in `schemas.py`:**
- `ResearchPlan` (5-tier rating + rationale + strategic actions)
- `TraderProposal` (action + reasoning + entry/stop/position sizing)
- `PortfolioDecision` (rating + thesis + price target + time horizon)
- `SentimentReport` (band + score 0-10 + confidence + narrative)

**Savant Application:** Savant's LLM output parsing is fragile. This pattern would make it robust — try structured output, fall back to free-text parsing.

#### F. Verified Market Snapshot (Anti-Confabulation) ⭐⭐⭐
The `get_verified_market_snapshot` tool is a deterministic, LLM-free source of truth for numeric data:

- Returns latest OHLCV row + common technical indicators + recent closes
- Market analyst MUST call this before writing its final report
- Analyst is instructed to "flag any discrepancies if other tool outputs conflict with this snapshot"
- Prevents LLMs from hallucinating price levels or indicator values
- Uses `stockstats` for indicator computation, filters data to prevent look-ahead bias

**Savant Application:** Savant's LLM sees market data in the prompt but has no verification step. A snapshot tool would catch cases where the model misreads or fabricates numbers.

#### G. Dual LLM Tiers (Deep vs Quick) ⭐⭐
TradingAgents uses two LLM tiers configured via `default_config.py`:

- **`deep_think_llm`** (default: `gpt-5.5`): Used for complex reasoning — research manager, portfolio manager, debate
- **`quick_think_llm`** (default: `gpt-5.4-mini`): Used for fast tasks — market analyst tool calls, signal parsing

Provider-agnostic via `llm_clients/factory.py` with lazy imports for OpenAI, Anthropic, Google, Azure.

**Model capability system (`capabilities.py`):**
- `ModelCapabilities` dataclass: `supports_tool_choice`, `supports_json_mode`, `supports_json_schema`, `preferred_structured_method`
- Resolved via: exact ID match → regex pattern match → default
- Avoids hardcoded conditionals in client classes

**Savant Application:** Savant uses a single model per decision. A tiered approach could use a cheaper model for scanning 100+ pairs → top 10, then expensive model for final decisions.

#### H. Vendor Routing Layer ⭐⭐
The `dataflows/interface.py` abstracts data sources behind a routing layer:
- Generic interface strings (e.g., `"get_stock_data"`) mapped to vendor-specific implementations
- Currently supports `yfinance` and `alpha_vantage` as backends
- Config-driven: `config["data_sources"]["stock_data"]` selects the vendor
- New vendors added by implementing the interface and registering in `VENDOR_METHODS`

**Savant Application:** Savant's insight modules are tightly coupled to specific APIs. A routing layer would allow swapping data providers without changing agent code.

#### I. Reddit Data with Graceful Fallback ⭐⭐
The `reddit.py` module demonstrates robust external data collection:
- Primary: Reddit's public JSON search endpoint (`/search.json`)
- Fallback: Atom/RSS search feed (`/search.rss`) when JSON returns 403
- When fallback triggers, engagement metrics (score, comments) are omitted rather than reported as zero
- No API key required; project-specific User-Agent string
- Searches r/wallstreetbets, r/stocks, r/investing for ticker mentions
- Returns placeholder strings instead of raising exceptions on failure

**Savant Application:** Savant's data fetchers fail hard on API errors. This graceful degradation pattern would improve resilience.

#### J. Per-Ticker SQLite Checkpointing ⭐
Uses per-ticker SQLite databases with deterministic `thread_id` (SHA-256 hash of ticker + date) to resume interrupted analysis runs.

- `SqliteSaver` from LangGraph for checkpoint persistence
- Per-ticker databases prevent concurrency contention
- `checkpoint_step()` retrieves latest step for resumption
- `clear_checkpoint()` cleans up by ticker+date

**Savant Application:** Savant already has SQLite journal. This pattern could be useful for resuming interrupted scan batches across 100+ pairs.

#### K. Instrument Identity Resolution ⭐⭐
The `resolve_instrument_identity()` function uses yfinance to deterministically resolve:
- Company name, sector, industry, exchange
- Cached after first resolution to prevent repeated network calls
- `build_instrument_context()` generates a structured prompt string anchoring all agents to the correct identity
- Prevents agents from hallucinating wrong company identities

**Savant Application:** Savant trades crypto pairs but doesn't verify token identity on-chain. A similar resolution step could verify contract addresses and prevent trading scam tokens.

#### L. Analyst Wall Time Tracking ⭐
TradingAgents tracks wall-clock time per analyst via `AnalystWallTimeTracker`. This identifies bottlenecks in the pipeline.

**Savant Application:** Track decision cycle latency per phase (data fetch → analysis → debate → execution). Surface on dashboard as a performance metric.

### V2 Research Findings: Same-Data Adversarial Debate (Deep Dive)

> Based on Gemini Deep Research v2 results — 42 academic papers and open-source implementations analyzed.

#### Critical Insight: The Martingale Curse ⭐⭐⭐
When debate agents share identical data AND identical model weights (Owl Alpha), their errors are correlated. Standard MAD often fails to improve beyond majority voting because agents converge toward erroneous consensus. This is called the "Martingale Curse."

**The Fix — AceMAD (Asymmetric Cognitive Potential Energy):**
- Before debating, each agent predicts what the other will argue
- The agent that better predicts the opponent's misconceptions gets weighted higher via Brier Score
- This creates an information-theoretic superiority that breaks the algorithmic consensus
- **Critical for savant:** We use one model (Owl Alpha) for everything, so this curse directly applies

#### Critical Insight: Disagreement Collapse (Sycophancy) ⭐⭐⭐
The biggest threat to same-data debate is sycophancy — agents prematurely agreeing to achieve conversational harmony. Measured by:
- **Negative Agreement Rate (NAR):** Frequency of inappropriate agreement with flawed premises
- **Disagreement Collapse Rate (DCR):** Rate of converting productive disagreement into sycophantic consensus

**Mitigation — Anti-Conformity Prompts:**
- Bear prompt must explicitly forbid: pleasantries, hedging, capitulation, mutual validation
- Bear is instructed: "You must never agree with the Bull. Do not acknowledge the Bull's valid points."
- Bull is instructed: "Actively reject the Bear's focus on micro-reversions."
- **Track DCR:** If Bull/Bear agree immediately in >15% of setups, debate is degraded — alert for re-calibration

#### Optimal Debate Rounds: Adaptive Stability Detection ⭐⭐
Instead of fixed N rounds, use Kolmogorov-Smirnov (KS) statistic to measure opinion divergence:
- High initial KS (0.25-0.45) = productive disagreement
- KS dropping below threshold for 2 consecutive rounds = stop debate
- Prevents both premature stopping and circular argumentation
- Better than TradingAgents' default of fixed 1 round

#### Parallel Universe Simulation ⭐⭐
With free API and 1M context, run 3 simultaneous Bull/Bear pairs at different temperatures (0.2, 0.4, 0.6):
- If all 3 agree → high confidence signal
- If they diverge → ambiguous setup, reject trade
- Majority voting among independent ensembles accounts for the most significant MAD performance gains
- **Implementation:** Only needed for ranging markets where false signals are rampant

#### Structured 5-Dimension Scoring Matrix ⭐⭐⭐
Both Bull and Bear output JSON scoring matrices (1-10) across 5 dimensions:

| Dimension | Bull Focus | Bear Focus |
|-----------|-----------|------------|
| Momentum Strength | Velocity, ADX, EMA slopes | Exhaustion, divergence |
| Risk Proximity | Distance to invalidation | Stop-loss vulnerability |
| Volume Confirmation | Accumulation, breakout | Volume climax, no follow-through |
| Funding Rate Alignment | Trend support, capital inflow | Overcrowding, squeeze potential |
| Liquidity Depth | Bid support, ease of entry | Slippage risk, AMM depth |

**Confidence Delta:** Judge calculates how much Bear degraded Bull's initial confidence. Massive delta = red flag.

#### Pre-Mortem as Bear Ammunition Generator ⭐⭐⭐
Run pre-mortem BEFORE full debate:
- Pre-mortem identifies specific, localized risks (not generic "flash crash")
- These specific risks are injected into Bear's system prompt for the debate
- Bear gets data-specific ammunition instead of generic arguments
- Research shows prospective hindsight increases failure identification by ~30%

**Prompt:** "It is one hour from now. We executed this long, and it failed catastrophically hitting our stop-loss. Explain exactly what happened."

#### Continuous Deliberation During Holds ⭐⭐
While a position is open, run a running Bull/Bear debate every 5 minutes:
- If Bear wins ongoing debate (identifies bearish divergence forming), close immediately
- Creates AI-powered trailing stop that's smarter than fixed ATR trailing
- **Only possible because Owl Alpha is free** — traditional systems can't afford this

#### Regime-Adaptive Debate Structure ⭐⭐⭐

| Regime | Debate Structure | Rounds | Trigger Confidence | Bear Mandate |
|--------|-----------------|--------|--------------------|---------------|
| **Trending** (ADX > 25) | Asymmetric | Bear +1 extra | 40% | Devil's Advocate (exhaustion focus) |
| **Ranging** (ADX < 20) | Symmetric + Parallel | Deep (3 universes) | 60% | Whipsaw/liquidity sweep ID |
| **Volatile** (ATR expanding) | Truncated | 1 round max | N/A | Mandatory veto on ATR breach |

#### DEX-Specific Bear Arguments ⭐⭐
Bear MUST argue about:
- 0x API slippage quotes vs actual execution
- MEV/sandwich attack risk on Arbitrum sequencer mempool
- Liquidity depth on target AMM (Uniswap v3)
- Fee drag calculation: ~0.6% round-trip on $30 account

Bull MUST argue about:
- 0x API smart order routing optimization
- Momentum velocity sufficient to overcome fee drag
- Execution through highly liquid pools (USDC/WETH)

#### Shadow Mode Validation ⭐⭐
Before live execution, run debate in shadow mode alongside single-shot:
- 14-21 days of continuous shadow data needed
- Primary metric: **Confidence Calibration** — does 60% consensus actually win 60% of the time?
- Track NAR and DCR for debate degradation
- Single-shot models are notoriously poorly calibrated (90% confidence on trades that win 45%)

#### Meta-Debate (Debate Quality Review) ⭐
After Bull/Bear conclude and Judge decides, a Meta-Agent reviews the transcript:
- Checks for logical fallacies, sycophancy, missed dimensions
- Logs feedback into episodic memory
- Injected into next candle's prompts for self-correction
- Phase 2 feature — implement after basic debate is proven

---

## 2. QuantDinger — Data Infrastructure Patterns

**Repo:** Open-source AI infrastructure for quantitative trading (local-first, Docker)
**Language:** Python (FastAPI)
**Most Relevant To:** `src/insight/`, `src/engine/`

### Architecture

QuantDinger separates data infrastructure (sources, caching, rate limiting, circuit breaking) from data providers (sentiment, crypto, forex) and API routes.

### Borrowable Ideas

#### A. Data Source Circuit Breaker ⭐⭐⭐
A proper 3-state circuit breaker (CLOSED → OPEN → HALF_OPEN) for data sources:
- **CLOSED:** Normal operation, count failures
- **OPEN:** Source is down, skip requests for `cooldown_seconds`
- **HALF_OPEN:** After cooldown, allow limited test requests to probe recovery

Config: `failure_threshold=2`, `cooldown_seconds=180`, `half_open_max_calls=1`

**Savant Application:** Savant has `src/risk/circuit_breaker.rs` but it's for *trading risk* (daily loss, drawdown). There's no circuit breaker for *data sources*. If CoinMarketCap or the price feed goes down, the engine keeps hammering the API. QuantDinger's pattern could be applied to every data fetcher in `src/insight/`.

**Implementation:** New `src/engine/data_circuit_breaker.rs` wrapping each insight module's fetch calls.

#### B. LRU Cache with TTL ⭐⭐⭐
Thread-safe, memory-based cache with:
- LRU eviction via `OrderedDict`
- Per-entry TTL (20min for realtime, 5min for klines, 24h for static)
- Hit/miss statistics
- Automatic expiration on `get()`

**Savant Application:** Savant fetches price data, fear/greed, funding rates, etc. on every cycle. A TTL cache would:
- Reduce API calls by 60-80% for data that doesn't change every cycle
- Provide hit/miss metrics for the dashboard
- Prevent redundant fetches when multiple modules need the same data

**Implementation:** New `src/engine/cache.rs` with a generic `DataCache<K, V>` struct.

#### C. Rate Limiter with Jitter ⭐⭐
Per-source rate limiters with:
- Minimum interval between requests
- Random jitter to avoid thundering herd
- Exponential backoff decorator with ±20% jitter
- Pre-configured instances per data source (stricter for flaky APIs)

**Savant Application:** Savant makes parallel API calls for multiple pairs. A per-source rate limiter with jitter would prevent IP blocks and reduce 429 errors.

#### D. Fear & Greed + VIX Integration ⭐⭐
QuantDinger fetches Fear & Greed (alternative.me) and VIX (yfinance) with tiered fallbacks. VIX is categorized into levels (very low → very high) with trading interpretations.

**Savant Application:** Savant already fetches Fear & Greed. Adding VIX as a macro volatility signal would improve regime detection. The tiered fallback pattern (primary → secondary → default) is also more robust than savant's current approach.

#### E. DataSourceFactory Pattern ⭐
A factory that maps market aliases to canonical identifiers, caches source instances, and provides a single `get_source(market)` entry point.

**Savant Application:** As savant adds multi-chain support, a factory pattern for data sources per chain would prevent duplication.

---

## 3. Live Trade Bench — LLM Benchmarking Framework

**Repo:** Real-time evaluation platform for LLM-based trading agents
**Language:** Python (FastAPI backend, TypeScript frontend)
**Most Relevant To:** `src/sandbox/`, `src/monitor/`

### Architecture

Live Trade Bench provides a framework for testing LLM trading agents in live markets (stocks + prediction markets), preventing backtest overfitting by evaluating in real-time.

### Borrowable Ideas

#### A. Live Benchmarking Against Baseline ⭐⭐⭐
The core insight: backtests overfit. Live Trade Bench evaluates agents in real market conditions against a baseline (e.g., buy-and-hold). It tracks:
- Per-decision LLM input/output for audit
- Allocation history with timestamps
- Performance vs benchmark

**Savant Application:** Savant has `src/sandbox/` for simulation but no **live benchmarking**. After each real trade, comparing against a simple baseline (e.g., "what if we just held USDC?") would provide a running measure of agent alpha. The `run_report.rs` could be extended with a benchmark comparison.

#### B. Allocation History with LLM Audit Trail ⭐⭐
`BaseAccount.record_allocation()` stores every portfolio snapshot with the LLM's raw input and output. This creates a complete audit trail: "at time T, the agent saw X data, produced Y reasoning, and made Z allocation."

**Savant Application:** Savant's `src/memory/episodic.rs` captures decisions but doesn't store the raw LLM prompt/response. Adding the full prompt+response to each episode would enable:
- Post-hoc analysis of decision quality
- Training data generation for fine-tuning
- Debugging when trades go wrong

#### C. Multi-Market Fetcher Architecture ⭐⭐
Clean base class (`BaseFetcher`) with market-specific implementations (stocks, Polymarket, BitMEX). Each fetcher normalizes data into a common format.

**Savant Application:** As savant expands to multiple chains (Arbitrum, Base, Solana), a base fetcher with chain-specific implementations would keep the insight layer clean.

#### D. Social Sentiment Fetcher (Reddit) ⭐
Dedicated Reddit fetcher that pulls relevant subreddit posts and converts them into sentiment signals.

**Savant Application:** Savant has `src/insight/sentiment.rs` (Fear & Greed + BTC dominance). Adding Reddit/Twitter sentiment for specific tokens would improve signal quality, especially for meme coins.

---

## 4. Hermes Agent — Self-Improving Agent Architecture

**Repo:** Self-improving AI agent by Nous Research with closed learning loop
**Language:** Python
**Most Relevant To:** `src/memory/`, `src/engine/`

### Architecture

Hermes is a general-purpose AI agent with a focus on self-improvement: it creates skills from experience, manages context windows aggressively, and tracks costs.

### Borrowable Ideas

#### A. Context Compression Engine ⭐⭐⭐
A pluggable context engine with:
- `should_compress()` — checks if context is approaching limits
- `compress()` — summarizes older turns while protecting head/tail
- Configurable: `protect_first_n` (system prompt), `protect_last_n` (recent context)
- Guided compression: `focus_topic` argument to prioritize relevant information
- Token budgeting: 20% of compressed space for summary, min 2K / max 12K tokens

**Savant Application:** Savant's LLM prompts include market data, knowledge, and decision history. As the context window fills, decision quality degrades. A compression step could summarize older market data while preserving recent price action and the system prompt.

**Implementation:** New `src/engine/compress.rs` that runs before each LLM call when token count exceeds threshold.

#### B. Skill Bundle System ⭐⭐
Users can group multiple skills into a "bundle" that loads all referenced skills under a single command. Bundles are YAML files with name, description, skills list, and optional instruction.

**Savant Application:** Savant's knowledge base (`knowledge/*.json`) is loaded wholesale. A bundle system would allow loading knowledge by context:
- Scalping bundle: price_action + technical_analysis + risk_management
- Swing bundle: fundamentals + sentiment + market_regimes
- DEX bundle: execution + crypto_native

This would reduce prompt size and improve focus.

#### C. Error Classifier with Recovery Hints ⭐⭐⭐
A structured taxonomy of API errors with recovery hints:
- `FailoverReason` enum: auth, billing, rate_limit, overloaded, timeout, context_overflow, etc.
- `ClassifiedError` with: `retryable`, `should_compress`, `should_rotate_credential`, `should_fallback`
- Centralized instead of scattered inline string-matching

**Savant Application:** Savant handles API errors ad-hoc. A centralized error classifier would:
- Automatically retry transient errors with backoff
- Rotate API keys on auth failures
- Compress context on context_overflow
- Switch providers on persistent failures

**Implementation:** New `src/engine/error_classifier.rs` with a `FailoverAction` enum.

#### D. Credits/Cost Tracker ⭐⭐
Tracks API costs via response headers, using integer math (micros) to avoid float precision issues. Provides depletion detection and subscription cap usage.

**Savant Application:** Savant uses OpenRouter/local models. A cost tracker would show:
- Cost per decision cycle
- Cost per trade (decisions + retries)
- Daily/weekly/monthly cost trends
- ROI: cost of AI vs profit generated

This data would appear on the dashboard.

**Implementation:** Add cost tracking to `src/monitor/metrics.rs`, parse OpenRouter cost headers from responses.

#### E. Cron Scheduler with Security ⭐
Background scheduler that:
- Runs jobs on a 60-second tick with file-based locking
- Restricts sensitive tools for cron-spawned agents
- Validates delivery platforms against an allowlist
- Blocks prompt injection in scheduled jobs

**Savant Application:** Savant runs 24/7 but has no scheduled tasks. A cron system could:
- Run periodic portfolio rebalancing checks
- Schedule daily performance reports
- Trigger knowledge base updates
- Execute maintenance tasks (journal cleanup, snapshot pruning)

---

## 5. OpenClaw — Skill-Based Agent Framework

**Repo:** Personal AI assistant with multi-platform integration
**Language:** TypeScript/Node.js
**Most Relevant To:** `src/agent/`, architecture patterns

### Borrowable Ideas

#### A. SKILL.md Contract Format ⭐⭐
Each skill has a structured SKILL.md with:
- **Metadata:** name, description (YAML header)
- **Contract:** Mandatory rules — "Shall/Never/Always/Fail closed"
- **Helper:** Commands and scripts with usage examples
- **Workflow:** Step-by-step procedure for using the skill
- **Review Artifacts:** References and links

**Savant Application:** Savant's knowledge files are unstructured JSON. A SKILL.md-style contract for each trading strategy would make the rules explicit and auditable:
```markdown
## Momentum Strategy Contract
- **Shall:** Only enter when regime is trending
- **Shall:** Set stop-loss within 0.5% of entry
- **Never:** Hold through a regime change
- **Never:** Risk more than 2% per trade
- **Fail closed:** If regime detection fails, default to HOLD
```

#### B. Gateway Architecture ⭐
OpenClaw's "Gateway" acts as a control plane between the agent and all external channels (Telegram, Discord, Slack, etc.). All communication flows through a single hub.

**Savant Application:** Savant's dashboard and TUI are separate. A gateway pattern could unify them, allowing the dashboard to send commands to the engine and receive real-time updates through a single WebSocket connection.

#### C. Plugin Architecture for Custom Providers ⭐
OpenClaw and KiloCode both support extending the system with custom plugins (MCP servers, skill packages).

**Savant Application:** A plugin system would allow third parties to add custom data providers, strategies, or insight modules without modifying core code. Define a trait-based interface in Rust that plugins implement.

---

## 6. KiloCode — Multi-Mode Agent System

**Repo:** All-in-one agentic engineering platform
**Language:** TypeScript
**Most Relevant To:** `src/agent/`, `src/tui/`

### Borrowable Ideas

#### A. Multi-Mode System ⭐⭐
KiloCode has specialized modes (Architect, Coder, Debugger) that the user can switch between. Each mode has different capabilities and system prompts.

**Savant Application:** Savant's agent has a single personality. Different modes could optimize for different market conditions:
- **Scalper Mode:** Fast decisions, tight stops, micro-profits
- **Swing Mode:** Patient entries, wider stops, macro targets
- **Hunt Mode:** Aggressive scanning, higher risk tolerance
- **Conservative Mode:** Only high-confidence entries, small positions

Mode switching could be automatic (based on regime detection) or manual (via TUI command).

**Implementation:** Add a `mode` field to config, load different system prompts and knowledge bundles per mode in `src/engine/`.

#### B. Capability-Based Model Selection ⭐
Models are tagged with capabilities (toolcall, attachment, input:audio, etc.) and filtered by what the current task requires.

**Savant Application:** Different decision types could use different models:
- Quick scan: fast/cheap model to filter 100 pairs → top 10
- Deep analysis: expensive model on top 10 for final decisions
- Debate: mid-tier model for bull/bear arguments

---

## 7. Cross-Project Patterns

Several patterns appear across multiple projects:

| Pattern | TradingAgents | QuantDinger | LiveBench | Hermes | OpenClaw | KiloCode |
|---------|:---:|:---:|:---:|:---:|:---:|:---:|
| **Multi-agent debate** | ✅ | | | | | |
| **Structured output schema** | ✅ | | ✅ | | | |
| **Circuit breaker** | | ✅ | | | | |
| **LRU cache with TTL** | | ✅ | | | | |
| **Rate limiter + jitter** | | ✅ | | | | |
| **Context compression** | | | | ✅ | | |
| **Error classification** | | | | ✅ | | |
| **Cost tracking** | | | | ✅ | | |
| **Reflection loop** | ✅ | | | | | |
| **Benchmark comparison** | | | ✅ | | | |
| **LLM audit trail** | | | ✅ | | | |
| **Skill/contract system** | | | | ✅ | ✅ | ✅ |
| **Multi-mode/personality** | | | | | | ✅ |
| **Cron scheduling** | | | | ✅ | | |
| **Per-ticker checkpointing** | ✅ | | | | | |

---

## 8. Prioritized Feature Backlog

### Tier 1 — High Impact, Moderate Effort (1-2 weeks each)

| # | Feature | Source | Savant Module | Effort | Description |
|---|---------|--------|---------------|--------|-------------|
| 1 | **Bull/Bear Debate** | TradingAgents | `src/strategy/debate.rs` | 3-5d | 2-3 round adversarial debate before trade decisions |
| 2 | **Reflection Loop** | TradingAgents | `src/memory/episodic.rs` | 2-3d | Post-trade analysis stored as lessons for future prompts |
| 3 | **TTL Cache for Insight Data** | QuantDinger | `src/engine/cache.rs` | 2-3d | LRU cache with per-source TTL to reduce redundant API calls |
| 4 | **Data Source Circuit Breaker** | QuantDinger | `src/engine/data_circuit_breaker.rs` | 2-3d | 3-state circuit breaker for API calls (CLOSED→OPEN→HALF_OPEN) |
| 5 | **Context Compression** | Hermes | `src/engine/compress.rs` | 3-5d | Summarize older context when approaching token limits |

### Tier 2 — High Impact, Higher Effort (1-3 weeks each)

| # | Feature | Source | Savant Module | Effort | Description |
|---|---------|--------|---------------|--------|-------------|
| 6 | **5-Tier Confidence Rating** | TradingAgents | `src/strategy/` | 3-5d | Replace binary BUY/SELL with Buy/Overweight/Hold/Underweight/Sell |
| 7 | **Error Classifier** | Hermes | `src/engine/error_classifier.rs` | 3-5d | Centralized error taxonomy with automated recovery actions |
| 8 | **Live Benchmark Comparison** | LiveBench | `src/monitor/report.rs` | 3-5d | Compare agent performance vs baseline after each trade |
| 9 | **LLM Audit Trail** | LiveBench | `src/memory/episodic.rs` | 2-3d | Store full prompt+response with each decision episode |
| 10 | **Dual LLM Tiers** | TradingAgents | `src/engine/` | 3-5d | Cheap model for scanning, expensive model for decisions |

### Tier 3 — Nice to Have (1-5 days each)

| # | Feature | Source | Savant Module | Effort | Description |
|---|---------|--------|---------------|--------|-------------|
| 11 | **Knowledge Bundles** | Hermes | `knowledge/` | 2-3d | Load knowledge files by context (scalping vs swing) |
| 12 | **Strategy Contracts** | OpenClaw | `knowledge/*.md` | 1-2d | Explicit shall/never/fail-closed rules per strategy |
| 13 | **Cost Tracker** | Hermes | `src/monitor/metrics.rs` | 1-2d | Track LLM cost per decision and per trade |
| 14 | **Risk Analyst Trio** | TradingAgents | `src/risk/` | 3-5d | Aggressive/conservative/neutral adversarial review of trades |
| 15 | **Multi-Mode Agent** | KiloCode | `src/engine/` | 3-5d | Switchable personalities (scalper/swing/hunt/conservative) |
| 16 | **VIX Integration** | QuantDinger | `src/insight/sentiment.rs` | 1-2d | Add VIX as macro volatility signal for regime detection |
| 17 | **Economic Calendar** | QuantDinger | `src/insight/` | 2-3d | Pause/widen stops around FOMC, CPI, NFP events |
| 18 | **Cron Scheduler** | Hermes | `src/engine/` | 2-3d | Scheduled tasks (reports, rebalancing, maintenance) |
| 19 | **Rate Limiter + Jitter** | QuantDinger | `src/engine/` | 1-2d | Per-source rate limiting with exponential backoff |
| 20 | **Reddit Sentiment** | LiveBench | `src/insight/sentiment.rs` | 2-3d | Token-specific social sentiment from Reddit |
| 21 | **Wall Time Tracker** | TradingAgents | `src/monitor/metrics.rs` | 1d | Track decision cycle latency per phase |
| 22 | **Per-Pair Checkpointing** | TradingAgents | `src/memory/` | 2-3d | Resume interrupted scan batches across 100+ pairs |

---

## Appendix: Project Summaries

### TradingAgents
Multi-agent framework simulating a trading firm. Decomposes decisions into analyst→researcher→trader→risk→PM pipeline. Uses LangGraph for orchestration, dual LLM tiers, and structured output schemas. Most innovative feature is the bull/bear debate and post-trade reflection.

### QuantDinger
Local-first quant trading infrastructure. Clean separation of data sources (with circuit breakers, rate limiters, caches) from data providers (sentiment, crypto, forex) from API routes. Docker-based deployment. Strongest contribution is the infrastructure patterns.

### Live Trade Bench
LLM trading agent benchmarking platform. Tests agents in live markets to prevent backtest overfitting. Supports stocks and prediction markets. Key contribution is the audit trail and benchmark comparison patterns.

### Hermes Agent
Self-improving AI agent by Nous Research. Features context compression, skill bundles, cron scheduling, error classification, and cost tracking. Most relevant for the agent architecture and memory management patterns.

### OpenClaw
Personal AI assistant with multi-platform integration. Uses a gateway architecture and structured SKILL.md contracts for defining agent capabilities. Most relevant for the skill/contract format and gateway pattern.

### KiloCode
Agentic engineering platform with multi-mode system (Architect, Coder, Debugger). Supports 500+ models with capability-based selection. Most relevant for the multi-mode concept and tiered model usage.
