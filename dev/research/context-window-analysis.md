# Context Window & Harness Engineering Analysis

**Date:** 2026-06-08
**Purpose:** Comparative analysis of context window management across three AI agent systems to inform a harness engineering overhaul for savant-trading.

---

## 1. Core Principle: Brain vs. Eyes

The savant-trading prompt has two fundamentally different categories of content:

**The Brain (~9K tokens) — MUST be preserved in full:**
- `soul.md` (~3.6K tokens) — Agent identity, philosophy, trading discipline
- Knowledge units (~3K tokens) — Curated trading wisdom from 171 books
- Risk constraints (~350 tokens) — Hard limits that prevent catastrophic losses
- Strategy knowledge (~360 tokens) — Regime awareness, session awareness, confidence discipline
- Echo rules (~240 tokens) — Edge cases from real traders (sell into strength, 3 losses = stop, etc.)
- Stop loss behavior (~430 tokens) — Fallback hierarchy
- Output format (~500 tokens) — Decision schema

This is the **entire intelligence layer**. Trimming this would gut the agent into a rule-based bot. The brain is what makes an LLM worth using instead of hardcoded if/else logic.

**The Eyes (~22K tokens per pair) — Target for compression:**
- 500 candles × 4 timeframes (2,000 data points) — ~20K tokens of raw OHLC data
- Higher-timeframe summaries that barely change in 5 minutes
- Indicator recaps (RSI, ADX, EMA) already visible in the candle pattern
- Market context boilerplate
- Account state (low informational value when nothing changed)
- Trade history (important context, but 10-trade full detail vs summary is a tradeoff)

The eyes are the **data the brain uses to decideDuring low-information periods (ranging, Deep Asian session), 80%+ of the candle data is nearly identical to what was sent 5 minutes ago. During those periods, ~20K tokens of the eyes carry maybe 2K tokens of actual new information. That's the waste.

**The right framing:** Don't shrink the brain. Compress the eyes.

---

## 2. The Problem

The savant-trading AI engine has **no context window management**. Every evaluation cycle sends the full ~31K token prompt per pair regardless of whether the information has changed since the last cycle. The owl-alpha model has a 1M context window, so we're not hitting limits, but research shows that **more tokens ≠ better reasoning**. The "Harness Engineering Is AI's New Gold Rush" article states: *"A bigger context window does not automatically make an agent better. The hard part is not giving the model more tokens — it is giving it the right tokens."*

The article cites a Stanford/Singhua joint study finding that **the same model can vary 6x in performance based solely on the system (harness) around it**. The UC Berkeley agentic paper identifies "context rot" as one of three core harness problems: *"A million-token window is useless if the critical detail is buried under old logs, stale notes, irrelevant files, and contradictory information."*

Our specific context rot problem:
- ~20K tokens of raw OHLC candle data per pair, where 80%+ carries negligible new information during quiet periods
- ~5.5K tokens of static brain content re-serialized every 5-minute cycle even though it never changes
- No delta-compression: identical data is re-sent regardless of what changed

---

## 3. System Comparison

### 3.1 savant-trading (Rust — Our System)

**Repository:** https://github.com/fame0528/savant-trading
- **System prompt:** 6 fixed layers (soul.md ~3.6K tokens, risk constraints, strategy knowledge, dynamic knowledge units up to 12K budget, echo rules, stop loss behavior, output format) = ~9K tokens
- **User message:** Full market snapshot per pair = ~22K tokens
  - 500 candles on 4 timeframes (2,000 candles × ~40 chars = ~20K tokens)
  - Indicators, regime, session, volume profile, order book, market insight, on-chain, RSS news, positions, account, trade history
- **Total per pair:** ~31K tokens
- **Total batch (2 pairs):** ~56K tokens (knowledge layer shared)

**Context budget management:**
- `knowledge_token_budget = 12,000` chars for knowledge unit selection (enforced)
- `MAX_SELECTED_UNITS = 12` hard cap (enforced)
- `max_tokens = 8,192` for LLM response (enforced)
- **No total prompt token budget** (NOT enforced)
- **No token counting of system prompt or user message** (NOT tracked)

**Context compaction/summarization:** None. Every tick rebuilds from scratch.

**Prompt caching:** None. The entire prompt is re-sent to the API every cycle with identical static content (soul.md, risk constraints, strategy knowledge never change).

**What works well:**
- MMR-based knowledge selection with diversity penalty prevents echo chambers
- Market-condition-driven context tags map to knowledge vocabulary
- Source-tier weighting (crypto-native YouTube 2.0x, institutional books 0.8x)
- Dynamic hunt mode injection when equity < $500 with idle capital

**Critical gaps vs. the article's recommendations:**

| Gap | Severity | Evidence |
|-----|----------|----------|
| No total prompt token budget | HIGH | `orchestrator.rs` logs `system_prompt.len()` but never counts user message or total tokens |
| No context compaction | HIGH | 2,000 candles sent per pair per cycle; during ranging, 80%+ is redundant |
| No prompt caching | MEDIUM | Static layers (~9K tokens) re-sent every 5-minute cycle unchanged |
| No dynamic candle adjustment | HIGH | `context_window_candles = 500` fixed regardless of regime, volatility, or information density |
| No token estimation of actual prompt | MEDIUM | Token count is chars/4 for knowledge units only; no counting of the full prompt |
| No conversation history across ticks | LOW | Each tick is stateless; model has no memory of its prior reasoning |

### 3.2 hermes-agent (Python — Top-Tier Reference)

**Repository:** `C:\Users\spenc\dev\savant-trading\reserch\sources\hermes-agent` (local download)
**Key source files:**
- `agent/prompt_builder.py` (1004+ lines) — Three-tier prompt architecture
- `agent/prompt_caching.py` (79 lines) — Cache stability strategy
- `agent/context_compressor.py` (1147+ lines) — 3-phase compaction algorithm
- `agent/conversation_compression.py` (802 lines) — LLM summarization with iterative updates
- `agent/iteration_budget.py` (62 lines) — Token budget tracking
- `agent/model_metadata.py` (1945 lines) — Per-model context window resolution
- `agent/memory_manager.py` (683 lines) — Memory prefetch and injection
- `agent/context_references.py` (518 lines) — @-mention context with token budgeting
- `agent/turn_context.py` (388 lines) — Per-turn context assembly
- `agent/system_prompt.py` (412 lines) — Stable prompt composition
- **Stable tier:** Identity, tool guidance, per-model operational guidance, skills index. Built once, cached in SQLite, reused across turns.
- **Context tier:** Caller-supplied context files, auto-discovered AGENTS.md/SOUL.md/.hermes.md.
- **Volatile tier:** Memory snapshot, USER.md, date line (day-level precision for cache stability).
- **Layout:** `"\n\n".join(stable, context, volatile)` — rebuilt only on compression.

**Context budget management (production-grade):**
- Rough preflight token estimation AND provider-reported actual token counts
- Configurable compression threshold (50-75% of model context window)
- Tail protection: most recent ~20% of context is never compressed
- Summary budget: 5% of context length, capped at 12K tokens
- Anti-thrashing: if 2 consecutive compressions each save <10%, skip compression
- Budget-based deferral: if rough estimate is noisy but provider confirmed last request fit, defer re-compression
- Image token accounting: flat 1,600 tokens per image part

**Context compaction (3-phase algorithm):**
1. **Tool output pruning** (cheap, no LLM): Replace old tool results with informative 1-line summaries
2. **LLM summarization:** Middle turns serialized to structured summary (Active Task, Resolved Questions, Pending Asks, Files Edited, Key Decisions, Errors/Blockers)
3. **Iterative summary updates:** Previous summary preserved and updated rather than re-summarized from scratch

**Prompt caching strategy:**
- "system_and_3" layout: 4 `cache_control` breakpoints (system prompt + last 3 non-system messages)
- ~75% input cost reduction on Anthropic models
- System prompt bytes stable across turns; ephemeral content injected into user message

**Key innovations to learn from:**
- Session-split after compression: old session closed, new session created with parent reference
- Informative tool summaries: `[terminal] ran 'npm test' → exit 0, 47 lines` instead of `[pruned]`
- Deduplication of identical tool results
- Static fallback summarization when LLM summarizer fails
- Mid-turn steering with bounded marker pattern resistant to prompt injection

### 3.3 openclaw (TypeScript — Platform Reference)

**Repository:** `C:\Users\spenc\dev\savant-trading\reserch\sources\openclaw` (local download)
**Assessment:** OpenClaw is a messaging gateway and session management platform, not a decision-making AI agent. It routes messages across 20+ platforms (Telegram, Discord, Slack, WhatsApp, Signal, etc.), manages sessions/transcripts, and hosts agents. It has **no prompt builder, context compressor, or token budgeting system** of its own. Included in this analysis as a reference for session management architecture, but not directly applicable to context window optimization.

---

## 4. Key Insight: Our Context Composition

Breaking down what we actually send per evaluation cycle with the brain-vs-eyes framework:

### The Brain (~9K tokens) — Intelligence layer, must be preserved:
- soul.md: ~3,600 tokens (agent identity, philosophy, trading discipline)
- risk_constraints.md: ~350 tokens (hard limits)
- strategy_knowledge.md: ~360 tokens (regime/session/confidence awareness)
- echo_rules.md: ~240 tokens (trader wisdom: sell into strength, 3 losses = stop, etc.)
- stop_loss_behavior.md: ~430 tokens (fallback hierarchy)
- output_format.md: ~500 tokens (decision schema)
- Selected knowledge units (up to 12, MMR-diverse): ~3,000 tokens
- **Brain subtotal: ~8,480 tokens**

### The Eyes (~22K tokens/pair) — Data layer, target for compression:
- 500 candles × 4 timeframes (2,000 data points): ~20,000 tokens
- Indicators (RSI, ATR, ADX, VWAP, EMA, GK): ~125 tokens
- Market insight summary: ~500 tokens
- On-chain analytics: ~250 tokens
- RSS news (5 items): ~375 tokens
- Order book imbalance: ~50 tokens
- Volume profile: ~50 tokens
- Open positions: ~125 tokens
- Account state: ~75 tokens
- Trade history (10 trades): ~500 tokens
- Fear & Greed context: ~150 tokens
- Session context: ~100 tokens
- **Eyes subtotal per pair: ~22,300 tokens**

### Grand total per cycle:
- 1 pair: ~30,780 tokens (9K brain + 22K eyes)
- 2 pairs (batch): ~53,080 tokens (brain shared, eyes duplicated)
- 10 pairs (full scan): ~231,800 tokens

### The waste (eyes only):

### Fixed content (~9K tokens, sent every cycle, never changes):
- soul.md: ~3,600 tokens (agent identity, philosophy, strategy)
- risk_constraints.md: ~350 tokens
- strategy_knowledge.md: ~360 tokens
- echo_rules.md: ~240 tokens
- stop_loss_behavior.md: ~430 tokens
- output format: ~500 tokens
- ═══════════════════════════════════
- **Fixed subtotal: ~5,480 tokens**

### Semi-static content (~3.5K tokens, changes slowly):
- Selected knowledge units (up to 12): ~3,000 tokens
- Session context: ~100 tokens
- Regime declaration: ~50 tokens
- ═══════════════════════════════════
- **Semi-static subtotal: ~3,150 tokens**

### Dynamic content per pair (~22K tokens, changes every cycle):
- 500 candles × 4 timeframes (2,000 data points): ~20,000 tokens
- Indicators (RSI, ATR, ADX, VWAP, EMA, GK): ~125 tokens
- Market insight summary: ~500 tokens
- On-chain analytics: ~250 tokens
- RSS news (5 items): ~375 tokens
- Order book imbalance: ~50 tokens
- Volume profile: ~50 tokens
- Open positions: ~125 tokens
- Account state: ~75 tokens
- Trade history (10 trades): ~500 tokens
- Fear & Greed context: ~150 tokens
- ═══════════════════════════════════
- **Dynamic subtotal per pair: ~22,200 tokens**

### Grand total per cycle:
- 1 pair: ~30,830 tokens
- 2 pairs (batch): ~53,030 tokens
- 10 pairs (full scan): ~230,000 tokens

### The waste analysis (eyes only — brain is sacred):

**Candle data rot (~20K tokens/pair):**
- 2,000 candles sent per pair per cycle (500 × 4 timeframes)
- During ranging markets (ADX < 15): 80%+ of candles show minimal price movement
- The model receives nearly identical OHLC data 500 times with no differentiation
- Estimated information content: ~2K tokens of unique information hidden in 20K tokens of raw data

**Static brain re-serialization (~9K tokens):**
- The brain (soul.md, risk constraints, strategy knowledge, output format) never changes
- Currently re-serialized and re-sent every 5-minute cycle
- Over 24 hours: ~1.6M tokens of pure repetition
- This doesn't degrade reasoning quality but wastes API bandwidth

**No delta-compression:**
- If nothing changed regime-wise since last cycle, identical data is re-sent
- Model gets no signal about what's NEW vs what's UNCHANGED
- A human trader would say "nothing changed, same thesis" — our system resends 31K tokens

---

## 4. Recommended Improvements (Prioritized)

**Core principle: Never touch the brain. Only compress the eyes.**

### P0 — Immediate (Pure prompt engineering, no API changes):

1. **Dynamic candle count based on market regime** (eyes only)
   - During ranging (ADX < 15): send 50 candles per timeframe + statistical summary instead of 500
   - During trending (ADX > 25): send 100 candles (enough to define the trend clearly)
   - During volatile (GK > 2× avg): send 200 candles (more data = more signal)
   - Higher-TF candles compressed more aggressively: 50 for 1H/4D, 100 for 5m
   - **Brain is untouched.** Only the data volume changes.
   - Savings: 60-90% of candle token cost during quiet periods = ~12-18K tokens/pair

2. **Cache the brain across cycles** (brain serialization optimization)
   - Soul.md, risk constraints, strategy knowledge, echo_rules, stop_loss_behavior, output format never change
   - Compose the brain string once on first evaluation, cache it in memory
   - Only re-compose when the knowledge unit selection changes (rare)
   - **Brain content is identical** — just stop re-serializing it every 5 minutes
   - Saves: ~9K tokens/cycle of redundant serialization

### P1 — Short-term (Config + light code changes):

3. **Add total eyes budget with soft truncation** (eyes only)
   - Set `max_eyes_tokens_per_pair` (suggest 8K default)
   - Token-count only the eyes portion before sending
   - If exceeded: truncate higher-TF candles first (they change slowest), then lower-TF
   - **Brain is never truncated.** Brain always gets its full ~9K.
   - Log truncation events for tuning

4. **Token estimation in the orchestrator** (both brain + eyes)
   - Count system_prompt (brain) + user_message (eyes) tokens
   - Log `brain_tokens`, `eyes_tokens`, `total_tokens` per pair per cycle
   - Set warning threshold at 15K eyes tokens per pair
   - Track in session summary for post-hoc analysis

### P2 — Medium-term (Structural improvements):

5. **Delta-compression for eyes across cycles** (eyes only)
   - Track what the model was told N cycles ago
   - If nothing changed (same regime, same triggers): send a 1-line delta instead of full data
   - Example: "Since 5m ago: ETH $1677→$1682, ADX stable at 11.6, no new triggers. Full data on request."
   - Full data is always available; delta is the default when nothing changed
   - **Brain is always sent in full when it changes** (new knowledge unit selection)

6. **Context-aware knowledge routing** (brain intelligence, not size)
   - When holding a position: prioritize monitoring knowledge (thesis invalidation, stop adjustment, scaling) over entry setups
   - When scanning for entries: prioritize pattern recognition and trigger knowledge
   - Knowledge units selected from the same pool, just scored differently based on position state
   - **Brain size is the same; brain focus shifts based on context**

7. **Selective preview for candle data** (eyes only)
   - Send statistical summary first: "500 candles: range X-Y, trend direction, volume profile, key S/R levels"
   - Include reference: "Full 500-candle data available if needed for specific pattern analysis"
   - Model can reason from the summary; only requests detail for edge cases
   - **Brain is untouched. Eyes are compressed.**

8. **Regime-aware eyes compression** (eyes only)
   - Ranging: maximum compression (50 candles + summary). Most data is noise.
   - Trending: medium compression (100 candles). Trend data is high-value.
   - Volatile: minimal compression (200 candles). Every candle matters.
   - **Brain is always sent in full regardless of regime.**

---

## 5. Questions for Gemini Deep Research

The following questions should be researched to inform our context management overhaul:

1. **What are the current best practices for context window management in trading-specific AI agents?** Focus on how professional trading firms and open-source projects handle the tradeoff between data completeness and prompt conciseness.

2. **How do leading AI agent frameworks (LangGraph, AutoGen, CrewAI) implement context compaction and dynamic prompt sizing?** What algorithms do they use for deciding what to keep vs. truncate?

3. **What is the empirical evidence on context length vs. reasoning quality?** Are there studies showing degradation when prompts exceed certain thresholds? What's the "sweet spot" for trading decision prompts?

4. **How do Claude Code and Gemini CLI implement the "selective preview" pattern?** (summarize first, let the model request detail) What are the implementation details and token savings?

5. **What are the best token counting methods for Rust applications?** Compare tiktoken-rs, tokenizers crate, and character-based estimation for accuracy vs. performance.

6. **How do production trading agents handle the "context rot" problem specifically?** What techniques do they use to avoid flooding the model with repetitive OHLC data during ranging markets?

7. **What prompt caching strategies are most effective for Anthropic's prompt cache API?** How should we structure our prompts to maximize cache hits while maintaining dynamic content?

---

## 6. Files Requiring Changes

| File | Change | Priority |
|------|--------|----------|
| `src/agent/context_builder.rs` | Dynamic candle count based on regime (eyes only); compress higher-TF candles; add token counting for eyes portion | P0 |
| `src/agent/orchestrator.rs` | Cache brain (system prompt) across cycles; log brain_tokens + eyes_tokens per evaluation | P0 |
| `src/core/config.rs` | Add `max_eyes_tokens_per_pair` (default 8000), `candle_count_ranging` (50), `candle_count_trending` (100), `candle_count_volatile` (200) | P1 |
| `config/default.toml` | Add new context budget config values | P1 |
| `src/agent/prompts.rs` | Separate stable brain composition from volatile eyes; cache brain string | P1 |
| NEW: `src/agent/context_budget.rs` | Token budgeting for eyes, truncation logic, delta-compression, regime-aware compression | P2 |

**Files that must NOT change (brain layer):**
- `src/agent/soul.md` — Full content preserved
- `src/agent/prompts/risk_constraints.md` — Full content preserved
- `src/agent/prompts/strategy_knowledge.md` — Full content preserved
- `src/agent/prompts/echo_rules.md` — Full content preserved
- `src/agent/prompts/stop_loss_behavior.md` — Full content preserved
- Knowledge base (all units) — Full selection preserved, only scoring/routing changes

---

## 7. Success Metrics

After implementing the overhaul, measure:

1. **Eyes tokens per evaluation** — target: <8K per pair (down from ~22K)
2. **Brain tokens per evaluation** — should stay stable at ~9K (never trim the brain)
3. **Total tokens per evaluation** — target: <17K per pair (down from ~31K)
4. **Context utilization ratio** — % of eyes tokens that carry new information vs. repetitive data
5. **Model reasoning quality** — are decisions more decisive? Less repetitive Pass/Pass?
6. **Token cost per day** — track even with free model for future scaling
7. **Compaction events** — how often we compress and what we compress (eyes only)
8. **Brain cache hit rate** — % of cycles where brain string is reused without re-serialization

---

*Compiled by OWL. All findings based on direct code analysis. No self-reporting.*
