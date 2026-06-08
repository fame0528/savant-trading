# Gemini Deep Research Prompt: AI Agent Context Window Management for Trading Systems

## Background

I am the lead developer of an autonomous cryptocurrency trading engine written in Rust. The system uses an AI model (owl-alpha, 1M context window) to make trading decisions every 5 minutes. Each evaluation sends a complete market snapshot including 500 candles across 4 timeframes (2,000 data points), indicators, on-chain analytics, news, trade history, account state, and a knowledge base of trading rules. Total prompt size is approximately 31,000 tokens per pair evaluation.

The system has NO context window management. Every 5-minute cycle sends the full ~31K token prompt even when 80%+ of the content hasn't changed since the last cycle. During low-volatility ranging markets, this means the model receives 2,000 nearly identical OHLC data points before it gets to the actual decision request. This is "context rot" — the critical signals (regime changes, new triggers, thesis invalidation) drown in repetitive noise.

I have studied three systems:
1. **Our system (savant-trading):** Sophisticated trading agent with knowledge-based decision making, no context management
2. **hermes-agent (Python):** Production-grade agent with 3-phase context compaction, prompt caching, intelligent summarization, and dynamic token budgeting
3. **The "Harness Engineering" framework:** Research showing the same model can vary 6x in performance based on system/harness design alone

## Research Questions

Please research the following questions thoroughly and provide actionable, specific recommendations. Focus on practical implementation techniques, not theoretical frameworks.

### Question 1: Best Practices for Context Window Management in Trading AI Agents

**What are the current best practices (2025-2026) for context window management in AI agents that make trading decisions?**

Specifically:
- How do professional trading firms and open-source trading agents handle the tradeoff between data completeness and prompt conciseness?
- What is the right amount of historical candle data to send? Is there a proven threshold where more candles degrade decision quality?
- How should prompts be structured differently for ranging vs. trending vs. volatile market conditions?
- What techniques exist for "delta-only" updates (only sending what changed since last evaluation)?
- Are there published benchmarks on context length vs. trading decision accuracy?

### Question 2: Context Compaction Algorithms for Multi-Turn Agent Loops

**How do leading AI agent frameworks implement context compaction when the conversation/context exceeds budget?**

Specifically:
- Compare the compaction approaches of LangGraph, AutoGen, CrewAI, and any newer frameworks (2025-2026)
- What algorithms are used for deciding what to keep vs. truncate vs. summarize?
- How does hermes-agent's 3-phase approach (tool pruning → LLM summarization → iterative updates) compare to other methods?
- What is the token savings vs. information loss tradeoff for each approach?
- Are there open-source Rust crates or libraries for context compaction?
- How do you prevent "compaction thrashing" (compressing too frequently)?

### Question 3: Empirical Evidence on Context Length vs. Reasoning Quality

**What empirical evidence exists showing the relationship between prompt length and reasoning quality?**

Specifically:
- Are there studies (academic or industry) showing degradation when prompts exceed certain thresholds?
- What's the "sweet spot" for trading decision prompts — is there an optimal token count?
- How does "context rot" (repetitive, low-information-density content) specifically degrade LLM performance?
- Are there benchmarks for financial reasoning tasks at different context lengths?
- What does the "Lost in the Middle" research say about information placement in long contexts?

### Question 4: Selective Preview and Detail-on-Demand Patterns

**How do Claude Code, Gemini CLI, and similar tools implement the "selective preview" pattern?** (Summarize first, let the model request detail)

Specifically:
- What are the implementation details — how do you summarize 500 candles into a preview that the model can use to decide whether it needs more detail?
- What token savings does this pattern achieve?
- How do you handle the case where the model requests detail — do you re-query, or maintain a side-channel?
- What prompt engineering techniques make this pattern work reliably?
- Are there failure modes where the model doesn't request detail when it should have?

### Question 5: Token Counting Methods for Rust Applications

**What are the best token counting/estimation methods for a Rust application?**

Specifically:
- Compare tiktoken-rs vs. HuggingFace tokenizers crate vs. character-based estimation
- What's the accuracy vs. performance tradeoff for each?
- Are there fast approximation methods that are accurate enough for dynamic prompt truncation?
- How do you handle token counting for non-English content (Chinese knowledge units, etc.)?
- Is there a streaming/online token counter that can be used during prompt assembly?

### Question 6: Context Rot Solutions Specific to Trading Agents

**How do production trading agents handle the "context rot" problem with repetitive market data?**

Specifically:
- What summarization techniques work best for OHLC candle data?
- How do you detect "information density" — i.e., which candles actually contain new information vs. repetitive noise?
- Are there regime-aware prompting strategies that dynamically adjust data volume?
- What's the minimum viable market snapshot for a trading decision?
- How do you balance the statistical need for large historical samples with the context cost of sending them?

### Question 7: Prompt Cache Optimization for Anthropic Models

**What prompt caching strategies are most effective for maximizing Anthropic's prompt cache API?**

Specifically:
- How should prompts be structured to maximize cache hit rates while keeping dynamic content fresh?
- What's the optimal number and placement of cache_control breakpoints?
- How do you handle the tradeoff between cache stability (identical bytes) and content freshness (current data)?
- What are the cost savings in production — is it worth the implementation complexity?
- How does cache interacts with dynamic content injection (memory prefetch, plugin context)?

## Desired Output Format

For each question, provide:
1. **Key Findings** — Bullet points of the most important discoveries
2. **Specific Techniques** — Named algorithms, patterns, or approaches with descriptions
3. **Token Savings Estimates** — Where possible, estimate the token reduction each technique could achieve for our ~31K token per-pair prompt
4. **Implementation Complexity** — Low/Medium/High for a Rust codebase
5. **Recommended Priority** — Which techniques should we implement first for maximum impact
6. **References** — Links to papers, blog posts, repositories, or documentation

## Constraints and Context

- Our system evaluates 2-10 pairs per cycle, every 5 minutes
- The AI model has a 1M context window (owl-alpha via OpenRouter)
- Current prompts are ~31K tokens per pair (9K fixed + 3K semi-static + 22K dynamic)
- Fixed content (identity, risk constraints, strategy knowledge) is ~5.5K tokens and NEVER changes between cycles
- Dynamic content includes 500 candles × 4 timeframes + indicators + market data
- We want to reduce per-pair prompts to ~15K tokens or less without losing decision quality
- Implementation language is Rust
- The system makes autonomous trading decisions — errors have real financial cost

## Current System Architecture (for reference)

The prompt has two parts:
- **System prompt:** Identity (soul.md) + risk constraints + strategy knowledge + dynamic knowledge units (MMR-selected, max 12 units within 12K budget) + output format
- **User message:** Latest candle + 3 higher-timeframe summaries (500 candles each) + indicators + regime + session + volume profile + order book + market insight + on-chain analytics + RSS news + open positions + account state + trade history + memory context + decision request

Each cycle is stateless — no conversation history across ticks. The full prompt is rebuilt and re-sent every 5 minutes.
