# Gemini Deep Research Prompt — Savant Trading Agent Memory System

## Research Objective

Design a comprehensive persistent memory system for an autonomous crypto trading agent named "Savant" that learns from its trading history, extracts actionable patterns, and feeds those patterns back into its decision-making process. The system must go beyond simple trade logging — it needs to make the agent SMARTER over time by learning what works, what doesn't, and under what conditions.

## Context

Savant is a Rust-native autonomous crypto trading engine with:

- 265 knowledge units from 22 curated sources
- AI brain powered by mimo v2.5 pro LLM via OpenGateway (1M context window)
- Trades 15 crypto pairs on Kraken (can expand to any pair)
- $50 paper trading budget, scaling to live
- Parallel AI evaluation (all pairs simultaneously, ~30s for 15 pairs)
- Real-time TUI dashboard (Ratatui)
- Obsidian vault integration for transparent state
- SOUL.md persona (560 lines) defining identity, risk management, decision framework
- Backtesting engine with walk-forward optimization

### Tech Stack

- Language: Rust 2021 (edition), MSRV 1.91
- Async runtime: tokio
- Database: SQLite via sqlx
- HTTP: reqwest
- Serialization: serde + serde_json
- Config: toml
- TUI: ratatui + crossterm
- WebSocket: tokio-tungstenite

### Current Prompt Structure (5 Layers)

The system prompt is composed from 5 layers:

1. **SOUL.md** (base identity) — 560 lines loaded via `include_str!`. Defines persona, cognitive style, risk management, decision framework, pre-trade checklist, operational constraints, identity invariants.
2. **Risk constraints** — Hard limits (1% per trade, 3% daily loss, 10% drawdown, 1.5:1 R:R minimum).
3. **Strategy knowledge** — Scale-out rules, trailing stops, fee awareness, regime awareness.
4. **Knowledge injection** — Dynamic selection from 265 units based on 22 MarketCondition tags. Token budget: 8000 chars.
5. **Output format** — JSON schema for decisions (action, pair, side, entry, stop, TPs, confidence, reasoning, R:R).

### Current AI Context (What the LLM Sees)

The user message contains:

```text
## Current Market Data — BTC/USD

Latest Candle: O=103450 H=103600 L=103200 C=103500 V=1234.56
Note: Latest candle may still be forming. Use 20-period volume SMA.

Indicators: EMA_FAST=Some(103200) EMA_SLOW=Some(102800) RSI=Some(45.2)
ATR=Some(450.0) ADX=Some(22.3) VWAP=Some(103300)

Regime: Ranging
Session: US | Behavior: Highest volume, major moves | Size: 1.2x | Kill Zone: YES

Volume Profile: POC=103300 VAH=103800 VAL=102800
Order Book Imbalance: +0.45 (bid-heavy — buying pressure)
Current Session: US — Highest volume and volatility

## Market Insight
Fear & Greed: 28 (Fear) | Funding Rate: 0.0123% (per-8hr) | Annualized: 13.49%
OI: 1787 | Block: 951779 | News: 352 items

## On-Chain Analytics
MVRV: 1.82 — Neutral/Undervalued
SOPR: 0.98 — Loss realization (capitulation)

## Recent News
[RSS items scored by relevance to current pair]

## Knowledge Injection
[Dynamically selected knowledge units based on current conditions]
```

Total prompt size: ~13,000 characters. LLM budget: 1M tokens. Plenty of room for memory.

### SOUL.md Pre-Trade Checklist (10 Points)

Every trade must answer all 10:

1. REGIME — Bull/Bear/Range/Crisis?
2. THESIS — Specific reason (1-2 sentences)
3. INVALIDATION — Price level where thesis is wrong
4. ENTRY — Price or zone
5. STOP — Stop loss price and why
6. TARGET — First profit target, R/R >= 1.5:1
7. SIZE — Account % at risk, within protocol?
8. CONVICTION — HIGH (1.5%) / MEDIUM (1.0%) / LOW (0.5%) / NONE (0%)
9. CORRELATION — Exceeds correlated exposure limits?
10. CATALYST RISK — Known events within 24h?

### Conviction Levels

| Level | Criteria | Max Risk |
| --- | --- | --- |
| HIGH | Regime clear + Setup clean + Funding aligned | 1.5% |
| MEDIUM | 2 of 3 factors aligned | 1.0% |
| LOW | 1 factor aligned, others neutral | 0.5% |
| NONE | Factors actively CONFLICT | 0% |

### 22 MarketCondition Tags

Used for knowledge unit selection:

Trending, Ranging, HighVolatility, LowVolatility, ExtremeFear,
ExtremeGreed, BreakingNews, SessionOpen, SessionClose, AltSeason,
BtcDominant, HalvingProximity, FomcDate, FundingRateExtreme,
LiquidationCluster, LiquidityExpansion, LiquidityContraction,
MvrvExtreme, SoprReset, OIDivergence, WyckoffSpring, DeltaDivergence

### Knowledge Unit Format

```json
{
  "id": "tjr-smc-001",
  "source": "daytrading-tjr-complete-guide",
  "topic": "TechnicalAnalysis",
  "conditions": ["Trending", "HighVolatility"],
  "content": "FVG (Fair Value Gap)...",
  "priority": 5
}
```

Topics: TechnicalAnalysis, Execution, RiskManagement, Psychology, MacroAnalysis,
Sentiment, OrderFlow, Scalping, SessionTrading, StockSelection, StrategyDesign,
AiStrategy, Backtesting, Compliance

### Current Session Detection

```rust
pub enum Session {
    Asian,      // 00:00-08:00 UTC, 0.8x size
    European,   // 08:00-14:00 UTC, 1.0x size
    UsSession,  // 14:00-22:00 UTC, 1.2x size
    LateUs,     // 22:00-00:00 UTC, 0.9x size
    Weekend,    // Sat/Sun, 0.7x size
}
```

### Current SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS trades (
    id TEXT PRIMARY KEY,
    pair TEXT NOT NULL,
    side TEXT NOT NULL,
    entry_price REAL NOT NULL,
    exit_price REAL NOT NULL,
    quantity REAL NOT NULL,
    pnl REAL NOT NULL,
    pnl_pct REAL NOT NULL,
    strategy_name TEXT NOT NULL,
    opened_at TEXT NOT NULL,
    closed_at TEXT NOT NULL,
    notes TEXT
);

CREATE TABLE IF NOT EXISTS equity_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    balance REAL NOT NULL,
    equity REAL NOT NULL,
    drawdown_pct REAL NOT NULL,
    open_positions INTEGER NOT NULL
);
```

### SharedEngineData Architecture

```rust
pub struct SharedEngineData {
    pub account: Arc<RwLock<AccountState>>,
    pub positions: Arc<RwLock<Vec<Position>>>,
    pub closed_trades: Arc<RwLock<Vec<TradeRecord>>>,
    pub insight: Arc<RwLock<MarketContext>>,
    pub decisions: Arc<RwLock<Vec<DecisionRecord>>>,
    pub activity_log: Arc<RwLock<Vec<ActivityEntry>>>,
}
```

### ActivityEntry Structure

```rust
pub struct ActivityEntry {
    pub timestamp: String,      // HH:MM:SS
    pub level: ActivityLevel,   // Info, Thinking, Decision, Trade, Warning, Error
    pub pair: String,
    pub message: String,
}
```

### Vault Directory Structure

```text
savant-vault/
├── Trades/           # Daily trade logs (auto-populated)
├── Decisions/        # AI decision logs (auto-populated)
├── Portfolio/        # Balance history, equity curve (auto-populated)
├── Insight/          # Market context snapshots (auto-populated)
├── Knowledge/        # Knowledge unit index (auto-populated)
├── Sessions/         # Session-specific notes (auto-populated)
├── Risk/             # Circuit breaker events (auto-populated)
├── Lessons/          # USER-EDITABLE ground truth (vault → engine)
└── INDEX.md
```

The **Lessons/** directory is already designed for operator feedback. Spencer can write markdown files that the engine ingests as ground truth. This is the existing mechanism for operator corrections.

### Current Decision Cycle Time

- 15 pairs × ~30s per LLM call = ~7.5 minutes per cycle (parallel)
- 5-minute candle timeframe
- Data fetch: ~8 seconds for all 15 pairs
- Indicator computation: <1 second
- Memory queries should add <500ms total (not per pair)

## Current Memory State (What Exists)

| Component | Storage | Persistent? | What It Stores |
|-----------|---------|-------------|---------------|
| TradeJournal | SQLite | Yes | Closed trades, equity snapshots |
| PaperTrader state | JSON file | Yes (on Ctrl+C) | Positions, account balance, daily PnL |
| Knowledge units | JSON files | Yes | 265 curated trading knowledge units |
| AI decisions | In-memory Vec | NO — lost on restart | Decision records with reasoning |
| Activity log | In-memory Vec | NO — lost on restart | Real-time engine events |
| Insight cache | In-memory | NO — lost on restart | Market data snapshots |
| Vault (Obsidian) | Markdown files | Yes | Decisions, trades, portfolio (human-readable) |

## What's Missing

The agent has NO ability to:

1. Learn from its own trading history
2. Extract patterns from wins vs losses
3. Remember what market conditions led to what outcomes
4. Adapt its behavior based on historical performance
5. Surface relevant past experiences when evaluating new setups
6. Track strategy-specific performance over time
7. Detect when its edge is degrading
8. Remember operator corrections and feedback

## What I Need Researched

### 1. Memory Architecture Design

Research how AI agents and trading systems implement persistent memory:

- What are the different types of memory (episodic, semantic, procedural, working)?
- How do human traders build "market memory" over years of screen time?
- How can an AI agent simulate this accumulation of experience?
- What is the optimal granularity for memory storage (every tick? every decision? every trade?)
- How do you balance memory richness (store everything) vs retrieval efficiency (find what matters)?

**Specific questions:**

- Should memory be centralized (one SQLite database) or distributed (multiple specialized stores)?
- What's the optimal schema for trading memory that supports both fast writes and complex queries?
- How do you handle memory that becomes outdated (e.g., a pattern that worked in 2024 but not 2026)?
- Should memories have a "relevance decay" over time?

### 2. Episodic Memory — What to Capture

Research what data points are critical to capture at decision time:

- What market context snapshot should accompany every decision?
- How do you capture the "state of mind" of the AI (confidence, reasoning, knowledge sources used)?
- What indicators and data points are most predictive of trade outcomes?
- How do you capture the full context without creating storage bloat?

**Specific questions:**

- What is the minimum viable snapshot for a trading decision? (Price, indicators, regime, funding, sentiment, order book, session)
- What additional context improves pattern extraction? (RSS news headlines, on-chain metrics, correlation state)
- How do you store LLM reasoning in a way that's queryable later?
- Should you store the full AI prompt + response, or just the structured decision?
- How do you handle decisions where the AI chose NOT to trade? (These are just as valuable as trades)

### 3. Semantic Memory — Pattern Extraction

Research how to extract actionable patterns from trading history:

- What specific patterns should the system look for? (win rate by regime, strategy, session, funding rate, sentiment, etc.)
- How do you calculate "edge" for a specific strategy under specific conditions?
- How do you detect when a strategy's edge is degrading?
- What statistical methods are appropriate for small sample sizes (50-200 trades)?
- How do you avoid overfitting to historical patterns?

**Specific questions:**

- What are the most important pattern categories to track?
  - Win rate by market regime (bull/bear/range/crisis)
  - Win rate by strategy (momentum/mean reversion/scalping)
  - Win rate by session (Asian/European/US/Weekend)
  - Win rate by conviction level (HIGH/MEDIUM/LOW)
  - Win rate by funding rate range (negative/neutral/positive/extreme)
  - Win rate by sentiment range (extreme fear/fear/neutral/greed/extreme greed)
  - Win rate by R:R achieved vs planned
  - Win rate by time held (minutes/hours/days)
  - Win rate by pair
  - Win rate by order book imbalance direction
  - Win rate by on-chain metric ranges (MVRV, SOPR)
- How many trades do you need before a pattern is statistically significant?
- How do you handle the cold-start problem (no history at launch)?
- Should patterns be global (across all pairs) or pair-specific?
- How do you detect regime changes that invalidate old patterns?

### 4. Memory Retrieval — Context Injection

Research how to surface relevant memories at decision time:

- How do you determine which past experiences are relevant to the current setup?
- What's the optimal format for presenting memories to an LLM?
- How do you prevent memory overload (too much context = decision paralysis)?
- How do you weight recent vs historical memories?

**Specific questions:**

- Should the AI see ALL its past decisions, or only relevant ones?
- What makes a past decision "relevant" to a current setup? (Same pair? Same regime? Same strategy? Similar indicators?)
- How do you format memory for LLM consumption? (Table? Narrative? Bullet points?)
- What's the maximum context window allocation for memory? (Current prompt is ~13K chars)
- Should memory be injected into the system prompt or user message?
- How do you handle contradictory memories (e.g., "momentum worked in January but failed in March")?
- Should the AI see its overall performance metrics? (Win rate, profit factor, Sharpe)
- Should the AI see its recent losing streak? (Could cause tilt, but also caution)

### 5. Learning Mechanisms

Research how the agent should adapt based on memory:

- Should the agent adjust its conviction levels based on historical win rates?
- Should the agent avoid strategies that have poor historical performance?
- Should the agent reduce size during conditions where it historically loses?
- How do you implement "lessons learned" that persist across sessions?
- How do you handle operator corrections (Spencer says "don't do X anymore")?

**Specific questions:**

- How do you implement a feedback loop: trade → outcome → pattern extraction → behavior change?
- Should the SOUL.md be dynamically updated based on learned patterns?
- How do you implement "confidence calibration" — adjusting the AI's confidence based on historical accuracy?
- Should the agent have a "memory review" mode where it analyzes its own history?
- How do you implement operator feedback injection? (Spencer marks a trade as "bad decision" → agent learns)
- Should the agent track "regret" — trades it didn't take that would have been profitable?
- How do you implement "anti-patterns" — conditions where the agent should NOT trade based on history?

### 6. Memory Persistence Architecture

Research optimal storage design for trading memory:

- SQLite schema design for trading decisions with full context
- How to handle schema migrations as the memory system evolves
- Backup and recovery strategies
- Query patterns for pattern extraction
- Performance considerations for real-time decision making

**Specific questions:**

- What's the optimal SQLite schema for storing decisions with full market context?
- Should market context be stored inline (JSON blob) or normalized (separate tables)?
- How do you handle the growing database size over months/years?
- Should old memories be archived or compressed?
- What indices are needed for fast pattern queries?
- How do you handle concurrent reads (pattern extraction) and writes (new decisions)?
- Should the memory system be async or sync?
- How do you implement memory snapshots for the TUI dashboard?

### 7. Integration with Existing System

Research how to integrate the memory system with Savant's existing architecture:

- How does memory fit into the 5-layer prompt system?
- How does memory interact with the 265 knowledge units?
- How does memory interact with the SOUL.md persona?
- How does memory interact with the Obsidian vault?

**Specific questions:**

- Should memory be a 6th prompt layer, or integrated into existing layers?
- Should memory override or supplement knowledge units?
- How do you handle conflicts between curated knowledge and learned patterns?
- Should the vault reflect memory state? (e.g., "Strategy X: 12W-8L in trending markets")
- How does memory affect the pre-trade checklist?
- Should the TUI show memory-derived insights? (e.g., "This setup has 67% historical win rate")

### 8. Cold Start and Bootstrap

Research how to handle the initial state with no trading history:

- What default assumptions should the agent make with no history?
- How do you bootstrap memory from the existing 265 knowledge units?
- Should the agent be more conservative or aggressive during the cold-start phase?
- How many trades before memory becomes useful?

**Specific questions:**

- Should the agent start with "assumed" win rates from the knowledge base?
- How do you implement progressive confidence — more confident as more history accumulates?
- What's the minimum number of trades for each pattern category to be statistically meaningful?
- Should the agent track "sample size" alongside each pattern?
- How do you handle the transition from cold-start to memory-informed trading?

### 9. Memory Visualization and Debugging

Research how to make the memory system transparent and debuggable:

- How should memory be displayed in the TUI dashboard?
- How should memory be represented in the Obsidian vault?
- How do you debug memory-related decisions?
- How do you audit what the agent "learned"?

**Specific questions:**

- Should the TUI show a "memory panel" with recent patterns and win rates?
- Should the vault have a "Memory/" directory with extracted patterns?
- How do you visualize the agent's learning curve over time?
- Should there be a CLI command to query memory? (e.g., "savant memory --pattern momentum --regime trending")
- How do you verify that memory is actually improving decisions?

### 10. Advanced Memory Concepts

Research cutting-edge approaches to agent memory:

- How do reinforcement learning agents handle memory?
- What is "experience replay" and can it apply to trading?
- How do you implement "memory consolidation" — periodic review and pattern extraction?
- What is "memory-augmented generation" and how does it apply to LLM-based trading?
- How do you implement "episodic future thinking" — the agent imagining future scenarios based on past experience?

**Specific questions:**

- Can you implement a simple form of reinforcement learning on top of the memory system?
- What is experience replay and how would it work for a trading agent?
- How do you implement periodic "memory consolidation" sessions where the agent reviews its history?
- Can the agent generate "what-if" scenarios based on memory? ("If I had held this trade for 2 more hours, what would have happened?")
- How do you implement "transfer learning" — applying lessons from one pair to another?
- Can the agent develop "intuition" — recognizing patterns before they fully form?

## Output Format

Produce a comprehensive memory system design document covering:

1. **Architecture overview** — how all memory components connect
2. **Episodic memory schema** — SQLite tables, JSON structures
3. **Semantic memory extraction** — specific patterns to track, extraction logic
4. **Context injection** — how memory feeds into the AI prompt
5. **Learning mechanisms** — how the agent adapts based on memory
6. **Cold start strategy** — what to do with no history
7. **Visualization** — TUI and vault integration
8. **Implementation roadmap** — what to build first, second, third

## Research Sources to Consult

1. Memory-augmented neural networks (MANN) research
2. Reinforcement learning experience replay (DeepMind, OpenAI)
3. Trading journal best practices (Edgewonk, TraderVue, Tradervue)
4. Cognitive psychology of expert performance (K. Anders Ericsson)
5. AI agent memory architectures (LangChain memory, MemGPT, Letta)
6. Trading system backtesting and optimization (QuantInsti, QuantConnect)
7. Statistical significance in small samples (trading-specific)
8. Obsidian as a knowledge base for trading
9. Savant AI Framework memory patterns
10. Human trader memory and pattern recognition research

## Constraints

- Must work with Rust + SQLite (sqlx) + tokio async runtime
- Must be compatible with 265 knowledge units and SOUL.md persona
- Must not add more than 500ms latency to decision-making
- Must be queryable for pattern extraction (not just logging)
- Must support concurrent reads (pattern queries) and writes (new decisions)
- Must be transparent — Spencer should be able to audit what the agent learned
- Must handle cold start gracefully (no history at launch)
- Must not cause the AI to overfit to historical patterns
- Must integrate with existing 5-layer prompt system
- Must work with existing SharedEngineData Arc<RwLock<>> architecture
- Must leverage existing Lessons/ vault directory for operator feedback
- Must be validated against the backtesting engine

---

## Perfection Loop

### Loop 1 (initial)

- **RED:** 15 issues found in initial prompt: missing current SQLite schema, prompt structure, AI context output, ActivityEntry struct, SharedEngineData architecture, LLM context budget, Lessons/ vault directory, knowledge unit format, MarketCondition tags, session detection, backtesting engine, SOUL.md checklist, decision cycle time, tech stack details, vault directory structure.

- **GREEN:** Added comprehensive "Context" section with all 15 missing pieces: tech stack, 5-layer prompt structure, current AI context example, SOUL.md pre-trade checklist, conviction levels, 22 MarketCondition tags, knowledge unit JSON format, session detection enum, current SQLite schema, SharedEngineData architecture, ActivityEntry struct, vault directory structure, decision cycle time, LLM context budget.

- **AUDIT:** All 15 issues resolved. The prompt now gives Gemini complete context about the existing system. Gemini can now design a memory system that integrates with the existing architecture rather than proposing a standalone solution.

**Loop result:** PASS — Prompt is complete and actionable.
