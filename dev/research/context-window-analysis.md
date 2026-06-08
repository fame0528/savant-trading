# Context Window & Harness Engineering Analysis

**Date:** 2026-06-08
**Purpose:** Comparative analysis of context window management across three AI agent systems to inform a harness engineering overhaul for savant-trading.

---

## 1. The Problem

The savant-trading AI engine has **no context window management**. Every evaluation cycle sends the full prompt — approximately 31K tokens per pair — regardless of whether the information has changed since the last cycle. The owl-alpha model has a 1M context window, so we're not hitting limits, but we're almost certainly degrading the model's reasoning quality through **context rot**: the critical signals (regime change, new triggers, thesis invalidation) drown in 30K+ tokens of repetitive OHLC data, static risk constraints, and boilerplate.

The article "Harness Engineering Is AI's New Gold Rush" puts it clearly: *"A bigger context window does not automatically make an agent better. The hard part is not giving the model more tokens — it is giving it the right tokens."*

---

## 2. System Comparison

### 2.1 savant-trading (Rust — Our System)

**Prompt architecture:** Two-part, rebuilt statelessly every 5-minute cycle.
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

### 2.2 hermes-agent (Python — Top-Tier Reference)

**Prompt architecture:** Three-tier, cached per session.
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

### 2.3 openclaw (TypeScript — Platform Reference)

**Assessment:** OpenClaw is a messaging gateway and session management platform, not a decision-making AI agent. It routes messages across 20+ platforms, manages sessions/transcripts, and hosts agents. It has **no prompt builder, context compressor, or token budgeting system** of its own. Not directly applicable to our context management problem.

---

## 3. Key Insight: Our Context Composition

Breaking down what we actually send per evaluation cycle:

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

### The waste analysis:
- **5,480 tokens of fixed content** are re-sent every 5-minute cycle even though they never change. Over 24 hours that's ~1.6M tokens of pure repetition.
- **20,000 tokens of candle data** are sent per pair per cycle. During ranging markets, 80%+ of those candles show minimal price movement — the model gets identical information repeated 500 times.
- **No prioritization**: A regime-changing trigger gets the same token allocation as 499 candles of sideways action.

---

## 4. Recommended Improvements (Prioritized)

### P0 — Immediate (No API changes, pure prompt engineering):

1. **Reduce candle count based on market conditions**
   - During ranging (ADX < 15): send 50 candles + summary statistic instead of 500
   - During trending (ADX > 25): send 100 candles (enough to see the trend)
   - During volatile (GK > 2× avg): send 200 candles
   - Savings: 60-90% of candle token cost during quiet periods = ~12-18K tokens/pair

2. **Cache static prompt layers across cycles**
   - Soul.md, risk constraints, strategy knowledge don't change cycle-to-cycle
   - Compose once on first cycle, reuse the string
   - Saves ~5.5K tokens per cycle × 288 cycles/day = ~1.6M tokens/day

3. **Compress higher-timeframe candles**
   - 500 candles on 1H and 4D is overkill — how much does the 4D candle change in 5 minutes?
   - Send 50 candles for higher TFs + summary statistics
   - Savings: ~8K tokens/pair

### P1 — Short-term (Config changes):

4. **Add total prompt token budget with soft truncation**
   - Set target: 15K tokens per pair
   - Token-count the full prompt before sending
   - If exceeded: truncate higher-TF candles first, then lower-TF, keep latest candles
   - Log truncation events for tuning

5. **Implement token estimation in the orchestrator**
- Count system_prompt + user_message tokens (chars/4 rough estimate)
- Log total_tokens per pair per cycle
- Set warning threshold at 20K tokens per pair

### P2 — Medium-term (Structural improvements):

6. **Implement context compaction for multi-turn awareness**
   - Track what the model was told N cycles ago
   - If nothing changed (same regime, same triggers, same conclusion), send a delta: "Since last evaluation 5m ago: price moved from X to Y. No new triggers. Previous thesis still valid."
   - Only send full context when regime changes or new signals appear

7. **Dynamic knowledge unit selection based on position state**
   - When holding a position: prioritize monitoring knowledge (stop management, thesis invalidation, scaling) over entry knowledge
   - When scanning for entries: prioritize setup/trigger knowledge
   - This is context-aware skill routing — the article's key recommendation

8. **Context references for large data dumps**
   - Instead of inlining all 500 candles, provide a summary with a reference: "500 candles available. Key: 20-bar range X-Y, trending/flat, volume Z% above/below average. Request specific candle ranges if needed."
   - Let the model ask for detail only when it needs it (selective preview pattern from the article)

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
| `src/agent/context_builder.rs` | Dynamic candle count based on regime; compress higher-TF candles; add token counting | P0 |
| `src/agent/orchestrator.rs` | Cache static prompt layers across cycles; log total tokens per evaluation | P0 |
| `src/core/config.rs` | Add `max_prompt_tokens_per_pair`, `candle_count_ranging`, `candle_count_trending`, `candle_count_volatile` | P1 |
| `config/default.toml` | Add new context budget config values | P1 |
| `src/agent/prompts.rs` | Separate stable vs. volatile prompt composition for caching | P1 |
| NEW: `src/agent/context_budget.rs` | Token budgeting, truncation, delta-compression module | P2 |

---

## 7. Success Metrics

After implementing the overhaul, we should measure:

1. **Tokens per evaluation cycle** — target: <15K per pair (down from ~31K)
2. **Model reasoning quality** — are decisions more decisive? Less repetitive Pass/Pass?
3. **Token cost per day** — even with free model, track for when we scale
4. **Context utilization ratio** — tokens that changed since last cycle / total tokens sent
5. **Compaction events** — how often we truncate and what we truncate

---

*Compiled by OWL. All findings based on direct code analysis. No self-reporting.*
