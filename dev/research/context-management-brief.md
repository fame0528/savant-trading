# Research Brief: Context Window Management — Beyond Hermes & OpenClaw

## Background

We're building an autonomous crypto trading engine (savant-trading, Rust) that evaluates trading pairs every 5-minute cycle using an LLM (owl-alpha, 1M context window via OpenRouter). The current prompt is ~31K tokens per pair: ~9K "Brain" (identity, risk constraints, knowledge units) and ~22K "Eyes" (raw OHLC candles, indicators, market context). This is wasteful and causes context rot.

We've done two prior research passes:
1. **Gemini Deep Research** (2026-06-08): 51-source analysis covering 7 dimensions of context management — dynamic compression, prompt caching, delta-compression, token counting, context rot, selective preview, knowledge routing.
2. **Code review of Hermes Agent + OpenClaw**: Read the actual source code of both leading agent frameworks to extract implementation patterns.

## What We Found in Hermes Agent (Python, ~1150 lines context_compressor.py)

### Strengths:
- **Tool result summarization**: Instead of generic placeholders, creates informative one-liners: `[terminal] ran \`npm test\` -> exit 0, 47 lines output`. Preserves signal while saving tokens.
- **Deduplication**: Hashes tool output content, replaces duplicates with back-references.
- **Image/media stripping**: Replaces old base64 image blobs with text placeholders. Finds last message with images, strips everything before it.
- **Structured compaction summary**: Uses `[CONTEXT COMPACTION — REFERENCE ONLY]` prefix with explicit "treat as background, NOT active instructions" directive.
- **Anti-thrashing**: If last 2 compressions saved <10% each, skips compression to prevent infinite loops.
- **Deterministic fallback**: When LLM summarizer fails, extracts user asks, assistant actions, tool names, file paths, blockers. Never silently drops content.
- **Iterative summary updates**: Maintains `_previous_summary` across compactions, progressively refining rather than re-summarizing from scratch.

### Weaknesses:
- Token counting uses `chars/4` heuristic (inaccurate for BPE)
- Monolithic architecture — compressor does everything
- No pluggable engine contract
- No cache observability

## What We Found in OpenClaw (TypeScript, pluggable context-engine contract)

### Strengths:
- **Pluggable context-engine interface**: Clean `bootstrap → ingest → assemble → maintain → compact → afterTurn` lifecycle. Runner just calls the engine.
- **Context pruning ("microcompact")**: Standalone, opt-in module. Trims old tool results based on token pressure. Soft trim at 30% (keep head+tail of tool output), hard clear at 50%. Operates on in-memory request only — no session rewrite.
- **Provider-level cache retention**: Explicit `cacheRetention` ("none" | "short" | "long" | "24h") per provider/model. Anthropic gets 5m/1h TTLs.
- **Cache stability tracking**: SHA-256 digests of system prompt + tool schema. Explicit cache invalidation. Observability on cache breaks.
- **Context window guard**: Validates model context window against configured minimums (hard min: 4K tokens or 10% of window; warn below: 8K or 20%). Actionable remediation messages.
- **Cache TTL-based pruning**: Configurable TTL (default 5 min) to identify stale tool results.
- **Token budget enforcement**: `tokenBudget` parameter flows through `assemble()` — engine knows the hard limit.

### Weaknesses:
- No tool result summarization (just generic placeholders)
- No deduplication
- No anti-thrashing heuristic
- Compaction is all-or-nothing (no progressive refinement)

## What the Gemini Research Recommended (15 changes across 4 phases)

**Phase 1 (Foundation):** Inverted Pyramid ordering, brain caching, API prompt caching, tiktoken migration, streaming token budget
**Phase 2 (Dynamic Compression):** Adaptive candle count, ZigZag pivot extraction, KBar features, TSLN serialization
**Phase 3 (Statefulness):** Delta-compression, anti-thrashing, selective preview
**Phase 4 (Knowledge Routing):** SGDR alpha balancing, metadata pre-filtering

## The Gap

Neither framework was designed for **high-frequency, data-dense, cyclical evaluation** like a trading engine. Hermes is built for long conversational sessions (coding assistant). OpenClaw is built for multi-agent orchestration. Neither handles:

1. **Cyclical data staleness**: Same data re-sent every 5 minutes with minor updates
2. **Numerical time-series compression**: 500 OHLC candles × 4 timeframes = 2000 data points per evaluation
3. **Regime-adaptive detail**: Ranging markets need less data, volatile markets need more
4. **Position-aware context**: Scanning for entries vs managing live positions need fundamentally different prompts
5. **Cost-sensitive batching**: Multiple pairs evaluated per cycle, each adding to API cost

## The Ask

Design a **unified context management architecture** that combines the best of Hermes (summarization, deduplication, anti-thrashing, fallback), OpenClaw (pluggable contract, microcompaction, cache observability, token budget enforcement), and the Gemini research (ZigZag, TSLN, adaptive windowing, SGDR) into a single coherent system optimized for high-frequency cyclical trading evaluation.

The solution should:
- Treat the 5-minute evaluation cycle as a first-class concept (not an afterthought)
- Handle numerical time-series data (OHLC candles) as a primary data type (not an edge case)
- Support position-aware prompt routing (scanning vs monitoring modes)
- Provide measurable token savings targets (e.g., reduce 31K → <8K tokens per pair per cycle)
- Be implementable in Rust (our engine language)
- Include a migration path from the current "send everything" approach

Focus on **novel synthesis** — not just listing existing techniques, but showing how they combine into something greater than the sum of parts. What would a purpose-built context management system for cyclical trading evaluation look like?
