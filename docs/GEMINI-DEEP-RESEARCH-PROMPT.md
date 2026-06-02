# Gemini Deep Research Prompt: Optimizing an Autonomous AI Crypto Trading Sandbox

## Context

I'm building **Savant Trading** — an autonomous AI-powered crypto trading engine in Rust. The AI brain is mimo v2.5 pro (1M context window) making all trading decisions. We have a sandbox testing system that evaluates the AI's trading decisions across 60 scenarios using real historical Kraken candle data.

**The goal:** Optimize the sandbox to be the most rigorous, realistic, and comprehensive AI trading evaluation system possible.

---

## Current Architecture

### How It Works
1. **Real data**: Fetches 720 real BTC/USD 5-minute candles from Kraken REST API (cached to disk)
2. **Scenario injection**: Each of 60 scenarios modifies the candle data with specific market conditions (trend, volatility, events) and provides mock insight data (Fear & Greed, MVRV, SOPR, NVT, funding rates, RSS news headlines)
3. **Context building**: Builds a FullContext with candles, indicators, volume profile, market context, order book imbalance, session, higher timeframe (1H aggregated from 5m), and memory context
4. **Parallel LLM evaluation**: Sends all 60 scenarios to the AI brain via curl subprocess (reqwest has TLS issues on Windows), capped at 5 concurrent with a semaphore
5. **Decision parsing**: Extracts JSON from LLM response (action, side, entry, stop, TP1/2/3, confidence, reasoning)
6. **3-tier grading**:
   - Tier 1 (Compliance): Binary — did it follow rules? (stop loss present, valid entry, reasoning >20 chars, missed trade penalty)
   - Tier 2 (R:R Score): Risk/reward ratio quality (0.0 for missed trades, neutral for holds)
   - Tier 3 (Reasoning): Does it cite data, classify regime, reference risk? (penalties for generic reasoning, no data references)
7. **Report card**: Category breakdown, critical failures, failure analysis with SOUL.md improvement suggestions

### Knowledge Base
- 2,959 optimized, tagged knowledge units across 10 enterprise-grade JSON files
- Topics: Psychology, Risk Management, Technical Analysis, Price Action, Trading Systems, Execution, Market Regimes, Sentiment, Fundamentals, Crypto Native
- Selection: `score = conditions_match × 3 + priority × 2 + tags_match × 1`
- Token budget: 8,000 tokens (converted via `div_ceil(4)`)

### SOUL.md (AI Persona)
- 560-line persona specification loaded via `include_str!`
- 13 sections including Action Triggers (Section XIII) for Trend Bull, On-Chain, Correlation, Sell/Short
- "When in doubt, stay out" as default bias (correct behavior, but too conservative with synthetic data)

### Scenarios (60 total, 11 categories)
- Trend Bull (8), Trend Bear (5), Range Bound (5), Volatility (5), Catalyst (5), Session (5), Sentiment (3+2 extended), Edge Case (7), Microstructure (5), Correlation (7), On-Chain (5)

---

## Latest Sandbox Results (60 scenarios, real Kraken data)

### Overall
| Metric | Value |
|--------|-------|
| Responses | 60/60 (100%) |
| Compliance | 63% |
| Average Score | 0.48 / 1.00 |
| Passed | 37 / 60 |
| Failed | 23 / 60 |
| Actual Trades | 9 (Buy/Sell) |
| ParseErrors | 8 |
| Missed Trades | 15 |

### Category Performance
| Category | Passed | Avg Score |
|----------|--------|-----------|
| Microstructure | 5/5 | 0.75 |
| Session | 5/5 | 0.75 |
| Sentiment | 3/3 | 0.73 |
| Range Bound | 4/5 | 0.61 |
| Trend Bear | 4/5 | 0.60 |
| Volatility | 4/5 | 0.61 |
| Catalyst | 3/5 | 0.46 |
| Edge Case | 4/7 | 0.44 |
| Correlation | 2/7 | 0.30 |
| Trend Bull | 2/8 | 0.20 |
| On-Chain | 1/5 | 0.16 |

### Actual Trades Made
- Parabolic Exhaustion → Sell (0.86 score)
- Extreme Greed → Sell (0.78)
- Extreme Fear → Buy (0.66)
- Daily Loss Breached → Sell (0.78)
- Maximum Leverage Trap → Sell (0.78)
- Miner Capitulation End → Buy (0.78)
- Liquidation Cascade → Buy (0.75)
- Protocol Exploit → Buy (0.78)
- Capitulation Wick → Buy (0.86)

### Failure Analysis
- **8 ParseErrors**: LLM returns malformed/truncated JSON. Parser expects strict JSON with specific fields.
- **15 Missed Trades**: Agent holds when scenario demands action. Breakdown:
  - 6 Trend Bull (Clean Breakout, EMA Pullback, Higher Low, Institutional Inflow, Golden Cross, Slow Grind)
  - 4 On-Chain (Whale Accumulation, NVT Divergence, SOPR Reset, Exchange Outflow)
  - 4 Correlation (Broad Market Rally, Altcoin Decoupling, BTC Dominance, Contagion Dump)
  - 1 Range Bound (Support Test)

---

## Core Problems to Solve

### 1. No Simulated Returns
The sandbox grades decisions but doesn't simulate what would happen if the trade was executed. We can't answer: "If the agent had taken the trade, would it have been profitable?" We need a P&L simulator that:
- Takes the decision (action, entry, stop, TP1/2/3) and the remaining candle data
- Simulates the trade forward (hit stop? hit TP? time exit?)
- Calculates R-multiple, P&L%, max drawdown, hold duration
- Aggregates across all scenarios for a portfolio-level equity curve

### 2. ParseErrors (8/60)
The LLM sometimes returns truncated JSON or includes markdown formatting. The parser needs to be more robust:
- Handle markdown code blocks (```json ... ```)
- Handle truncated responses (partial JSON recovery)
- Handle reasoning field (mimo v2.5 pro returns content in "reasoning" not "content")

### 3. Missed Trades (15/60)
The agent holds on clear setups. Root causes:
- SOUL.md "when in doubt, stay out" is too strong
- Action Triggers (Section XIII) aren't overriding the conservative bias
- The agent sees synthetic scenario data as "untrustworthy" even though it's real Kraken candles with scenario overlays

### 4. Priority Distribution Imbalance
Three files have P5 dominance (should be ~10%):
- Psychology: 53% P5 (should be 10%)
- Risk Management: 64% P5 (should be 10%)
- Technical Analysis: 42% P5 (should be 10%)

### 5. Context Tags Not Being Generated
The `context_tags` field in FullContext is always `vec![]`. We have the tag system but no logic to generate context tags from current market state (e.g., "breakout" when price breaks resistance, "fomc" during FOMC events).

### 6. WebSocket State Desync
The WebSocket receiver is only drained at the top of each tick. During LLM inference (which takes seconds), WS messages queue up. The agent makes decisions on stale data.

### 7. No Multi-Run Averaging
LLM responses are non-deterministic. A single run may not be representative. Need multi-run averaging (run each scenario 3-5 times, average the scores).

---

## Research Questions

### Sandbox Optimization
1. **How should we simulate trade returns from sandbox decisions?** What's the best approach for forward-simulating trades on historical candle data? Should we use the remaining candles after the decision point? How do we handle partial fills, slippage, and fees?

2. **How should we handle LLM response parsing robustness?** What are best practices for extracting structured JSON from LLM responses that may be truncated, contain markdown, or have non-standard formatting?

3. **How should we generate context tags from market state?** What's the best approach for automatically deriving tags (breakout, pullback, divergence, etc.) from indicators and price action at the time of decision?

4. **How should we handle non-deterministic LLM outputs?** What's the optimal number of runs per scenario for reliable scoring? How do we aggregate scores across runs (mean, median, trimmed mean)?

5. **How should we weight the grading rubric?** Is the current 3-tier system (compliance, R:R, reasoning) the right framework? What other dimensions should we evaluate?

### AI Trading Agent Optimization
6. **How should we balance conservative bias vs. action?** The agent correctly defaults to "stay out" but misses clear setups. What's the optimal framework for distinguishing "real doubt" from "manufactured doubt"?

7. **How should we optimize knowledge injection for trading decisions?** With 2,959 tagged units and 8,000 token budget, what's the optimal selection strategy? Should we prioritize by relevance score, diversity, or recency?

8. **How should we structure the SOUL.md for maximum trading performance?** What psychological frameworks, decision trees, and action triggers produce the best results in autonomous trading agents?

### Crypto-Specific
9. **How should we handle crypto-specific market conditions?** 24/7 markets, funding rates, liquidation cascades, halving cycles, exchange hacks, regulatory events — what's the optimal framework for crypto-native trading?

10. **What on-chain metrics are most predictive for short-term crypto trading?** MVRV, SOPR, NVT, exchange flows, whale tracking — which provide the most actionable signals and at what thresholds?

---

## What I'm Looking For

1. **Specific, actionable recommendations** — not general advice. Exact parameters, thresholds, formulas.
2. **Research-backed** — cite academic papers, industry best practices, proven frameworks.
3. **Crypto-native** — recommendations should be specific to crypto markets, not generic trading advice.
4. **Implementation-ready** — I should be able to implement each recommendation directly in Rust code.
5. **Prioritized** — rank recommendations by impact (what will improve sandbox accuracy the most).

Please provide a comprehensive research report addressing all of the above.
