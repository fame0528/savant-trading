# Gemini Deep Research Prompt: AI Agent Context Window Management for Trading Systems

## Research Objective

Improve the context window management of an autonomous crypto trading AI agent. The core principle: **never trim the brain (intelligence layer), only compress the eyes (data layer).**

---

## Background: Our System

**Repository:** savant-trading (Rust) — an autonomous cryptocurrency trading engine.

**Model:** owl-alpha via OpenRouter (1M context window, free tier)

**Current prompt architecture (two-part, rebuilt statelessly every 5-minute cycle):**

### The Brain (~9K tokens) — Intelligence layer, MUST be preserved in full:
- `soul.md` (~3,600 tokens) — Agent identity, philosophy, trading discipline, risk framework
- Knowledge units (~3,000 tokens) — Curated trading wisdom from 171 books, MMR-selected (max 12 units)
- `risk_constraints.md` (~350 tokens) — Hard limits: max daily loss, drawdown, position count, R:R minimum
- `strategy_knowledge.md` (~360 tokens) — Regime awareness, session awareness, confidence discipline
- `echo_rules.md` (~240 tokens) — Trader wisdom: sell into strength, 3 losses = stop, don't marry positions
- `stop_loss_behavior.md` (~430 tokens) — Fallback hierarchy for missing stops
- Output format (~500 tokens) — JSON decision schema
- **This is what makes the agent intelligent. Trimming this gutting the software.**

### The Eyes (~22K tokens/pair) — Data layer, target for compression:
- 500 candles × 4 timeframes (2,000 data points): ~20,000 tokens of raw OHLC data
- Indicators (RSI, ATR, ADX, VWAP, EMA, Garman-Klass): ~125 tokens
- Market insight summary: ~500 tokens
- On-chain analytics (MVRV, SOPR, NVT): ~250 tokens
- RSS news (5 items): ~375 tokens
- Order book imbalance: ~50 tokens
- Volume profile (POC, VAH, VAL): ~50 tokens
- Open positions: ~125 tokens
- Account state: ~75 tokens
- Trade history (10 recent): ~500 tokens
- Fear & Greed context: ~150 tokens
- Session context: ~100 tokens

### The Problem:
- **Total per pair: ~31K tokens** (9K brain + 22K eyes)
- **Total batch (2 pairs): ~53K tokens** (brain shared, eyes duplicated)
- **No context window management exists.** Every cycle sends the full prompt regardless of what changed.
- During low-volatility ranging, 80%+ of the 20K candle tokens carry negligible new information.
- The brain is re-serialized every 5 minutes even though it never changes.
- No delta-compression: identical data is re-sent with no signal about what's new vs. unchanged.

### What We've Already Identified (from studying hermes-agent):

The hermes-agent (Python) has a sophisticated 3-tier prompt architecture:
- **Stable tier:** Identity, tool guidance, per-model operational guidance — built once, cached in SQLite, reused across turns
- **Context tier:** Caller-supplied context files, auto-discovered AGENTS.md/SOUL.md
- **Volatile tier:** Memory snapshot, USER.md, date line (day-level precision for cache stability)

Their 3-phase context compaction:
1. **Tool output pruning** (cheap, no LLM): Replace old tool results with informative 1-line summaries
2. **LLM summarization:** Middle turns serialized to structured summary (Active Task, Resolved Questions, Pending Asks, Files Edited, Key Decisions, Errors/Blockers)
3. **Iterative summary updates:** Previous summary preserved and updated rather than re-summarized from scratch

Their token budgeting:
- Rough preflight estimation + provider-reported actual token counts
- Configurable compression threshold (50-75% of model context window)
- Tail protection: most recent ~20% never compressed
- Summary budget: 5% of context length, capped at 12K tokens
- Anti-thrashing: if 2 consecutive compressions each save <10%, skip compression

---

## Research Questions

Please research the following questions thoroughly. Focus on **practical, implementable techniques** for a Rust-based trading agent. Provide specific algorithms, token savings estimates, and implementation guidance.

### Question 1: Dynamic Data Compression for Trading Prompts

**How should we dynamically compress the "eyes" (market data) based on market conditions while preserving the "brain" (intelligence) in full?**

Our specific situation:
- During ranging markets (ADX < 15): 500 candles per timeframe is massive overkill
- During trending markets (ADX > 25): fewer candles needed to define the trend
- During volatile markets (Garman-Klass > 2× avg): more candles carry real signal

Research:
- What are proven techniques for summarizing OHLC candle data without losing critical information?
- How do professional trading systems decide how much historical data to present to a decision-maker?
- What's the minimum viable candle count for different market regimes?
- How do you create a "selective preview" for candle data (summary first, detail on request)?
- Are there statistical methods to detect which candles carry the most information (e.g., volume spikes, volatility clusters)?

**Constraint:** The brain (~9K tokens: soul.md, knowledge units, risk constraints, strategy knowledge) must NEVER be compressed or truncated. Only the eyes (market data) should be dynamically adjusted.

### Question 2: Prompt Caching and Static Content Reuse

**How should we cache the brain (static prompt layers) across evaluation cycles?**

Our specific situation:
- The brain (soul.md, risk constraints, strategy knowledge, echo rules, stop loss behavior, output format) never changes between 5-minute cycles
- Currently re-serialized every cycle (~9K tokens of redundant serialization)
- hermes-agent uses SQLite-backed session caching with day-level timestamp precision

Research:
- What's the most efficient way to cache and reuse static prompt content in a Rust application?
- How does Anthropic's prompt cache API work, and how should prompts be structured to maximize cache hits?
- What's the optimal cache granularity — per-session, per-regime, per-day?
- How do you handle cache invalidation when knowledge units change (rare but important)?
- What are the cost savings of prompt caching in production?

### Question 3: Delta-Compression for Multi-Cycle Awareness

**How should we implement delta-compression — only sending what changed since the last evaluation?**

Our specific situation:
- Every 5-minute cycle rebuilds and resends the full ~31K token prompt
- During quiet periods, 80%+ of the content is identical to the last cycle
- The model has no way to distinguish "new information" from "same as before"

Research:
- What delta-compression techniques exist for time-series data in prompts?
- How do you detect "information content" — which data points actually carry new signal?
- What's the right abstraction for a "market delta" summary? (e.g., "Price moved X→Y, ADX stable, no new triggers")
- How do you handle the tradeoff between delta compression and the model occasionally needing full context?
- Are there existing Rust crates for diff-based prompt compression?

### Question 4: Token Counting and Budget Enforcement in Rust

**What are the best token counting methods for a Rust application that needs to enforce a per-pair token budget?**

Our specific situation:
- Currently use `chars / 4` for knowledge units only (rough estimate)
- No token counting of the full prompt (brain + eyes)
- Need to enforce `max_eyes_tokens_per_pair` budget with soft truncation

Research:
- Compare tiktoken-rs vs. HuggingFace tokenizers crate vs. character-based estimation for accuracy vs. performance
- What's the fastest method that's accurate enough for dynamic prompt truncation?
- How do you handle token counting for mixed-language content (English + Chinese knowledge units)?
- Are there streaming/online token counters that can be used during prompt assembly?
- What's the overhead of accurate token counting vs. rough estimation?

### Question 5: Context Rot and Reasoning Quality

**What does the research say about context length vs. reasoning quality, specifically for trading decisions?**

Our specific situation:
- ~31K tokens per pair, where ~20K is raw OHLC data
- During ranging, the model gets 500 nearly identical candles before the decision request
- We suspect this "context rot" degrades decision quality but need evidence

Research:
- What empirical studies exist on context length vs. reasoning quality degradation?
- Is there a "sweet spot" for trading decision prompts?
- How does "Lost in the Middle" affect trading decisions — does information placement matter?
- Are there benchmarks for financial reasoning at different context lengths?
- What does the Stanford/Singhua 6x performance finding mean for prompt design?

### Question 6: Selective Preview and Detail-on-Demand

**How do Claude Code, Gemini CLI, and similar tools implement "selective preview" — summarize first, let the model request detail?**

Our specific situation:
- 2,000 candles per pair is too much to inline, but the model occasionally needs specific candle data
- We want to send a statistical summary by default, with full data available on request

Research:
- What are the implementation details of selective preview for large data sets?
- How do you structure a "preview" that gives the model enough information to decide whether it needs detail?
- What token savings does this pattern achieve?
- How do you handle the model's request for detail — re-query, side-channel, or pre-loaded context?
- What are the failure modes where the model doesn't request detail when it should have?

### Question 7: Knowledge Routing Based on Position State

**How should knowledge unit selection change based on whether the agent is monitoring held positions vs. scanning for new entries?**

Our specific situation:
- When holding positions: should prioritize monitoring knowledge (thesis invalidation, stop adjustment, scaling)
- When scanning for entries: should prioritize pattern recognition and trigger knowledge
- Same knowledge pool, different scoring/routing based on position state

Research:
- What are proven techniques for context-aware skill/tool routing in AI agents?
- How do you score knowledge units differently based on the agent's current task?
- What's the implementation complexity of dynamic knowledge routing?
- Are there existing patterns for "monitoring mode" vs. "scanning mode" knowledge selection?

---

## Reference Materials

### Systems Studied

1. **savant-trading** (our system, Rust)
   - Repository: https://github.com/fame0528/savant-trading
   - Key files: `src/agent/context_builder.rs`, `src/agent/orchestrator.rs`, `src/agent/knowledge.rs`, `src/agent/prompts.rs`, `src/core/config.rs`
   - Strengths: MMR knowledge selection, regime-aware context tags, sophisticated trading logic
   - Weaknesses: No context management, no caching, no compression, no token budgeting

2. **hermes-agent** (Python — top-tier reference, 186k stars)
   - Repository: https://github.com/NousResearch/hermes-agent
   - Key directories: `agent/` (prompt_builder.py, context_compressor.py, conversation_compression.py, prompt_caching.py, iteration_budget.py, model_metadata.py, context_references.py)
   - Active development: v0.16.0 released 2026-06-05
   - Key innovations: 3-tier prompt architecture (stable/context/volatile), 3-phase compaction, prompt caching with SQLite, per-model context resolution, anti-thrashing

3. **openclaw** (TypeScript — platform reference)
   - Repository: https://github.com/openclaw/openclaw
   - Assessment: Messaging gateway (20+ platforms), not a decision-making agent. Included for session management reference, not context optimization.

### Research Article

"Harness Engineering Is AI's New Gold Rush" (YouTube transcript)
- Key findings: Same model varies 6x in performance based on harness design; context rot is one of three core harness problems (along with memory staleness and skill routing); the harness is the product, not the model; competitive advantage shifts to teams building better scaffolding around commoditized models

### Framework Links for Deeper Research

Please research these specific frameworks and their context management approaches:

- **hermes-agent:** https://github.com/NousResearch/hermes-agent — Study the 3-phase compaction algorithm, prompt caching strategy (`agent/prompt_caching.py`), token budgeting (`agent/iteration_budget.py`), and context references system (`agent/context_references.py`). This is the most relevant reference for our use case.

- **openclaw:** https://github.com/openclaw/openclaw — Study session management, subagent orchestration, and how it hosts agents (not context management). Look at `src/cron/isolated-agent.ts` and `src/gateway/server-methods/agent.ts` for agent lifecycle patterns.

- **LangGraph:** https://github.com/langchain-ai/langgraph — How does it handle context compaction in multi-agent workflows? Look for memory management, state pruning, and conversation summarization.

- **AutoGen:** https://github.com/microsoft/autogen — What context management does it provide for agent conversations? Study the GroupChat and nested chat patterns.

- **CrewAI:** https://github.com/crewAIInc/crewAI — How does it handle context window limits for crew-based agents? Look for task delegation and result aggregation patterns.

- **Claude Code:** How does Anthropic's own coding agent implement selective preview and context management? What can we learn from how it handles large codebases without flooding the context?

- **Gemini CLI:** https://github.com/google-gemini/gemini-cli — How does Google's CLI agent handle large context windows? What compression or summarization techniques does it use?

---

## Desired Output Format

For each question, provide:

1. **Key Findings** — Bullet points of the most important discoveries
2. **Specific Techniques** — Named algorithms, patterns, or approaches with descriptions
3. **Token Savings Estimates** — For our specific situation (31K tokens/pair, 22K eyes, 9K brain), estimate the reduction each technique could achieve
4. **Implementation Complexity** — Low/Medium/High for a Rust codebase
5. **Recommended Priority** — Which techniques should we implement first for maximum impact
6. **Code Structure Suggestions** — What modules/files should be created or modified
7. **References** — Links to papers, blog posts, repositories, documentation

## Constraints

- **The brain (~9K tokens) must NEVER be compressed or truncated.** This is non-negotiable.
- Only the eyes (~22K tokens of market data) should be dynamically compressed.
- Implementation language is Rust.
- The system makes autonomous trading decisions — errors have real financial cost.
- The model has a 1M context window, so we're not hitting limits — the goal is reasoning quality, not fitting within bounds.
- Each evaluation cycle is currently stateless (no conversation history). Multi-turn awareness is a future goal, not a current requirement.
- The system evaluates 2-10 pairs per cycle, every 5 minutes.
