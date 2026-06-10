# Gemini Deep Research Prompt: AI Crypto Scalping Agent Optimization

**Created:** 2026-06-10
**Purpose:** Research best practices for AI trading agent scalping on DEX
**Budget:** $30 total (no additional capital)
**Target:** 100+ trades making $0.05-0.10 each, high win rate

---

## System Overview

### What We Built

An autonomous AI trading agent that:
- Trades crypto spot on Arbitrum DEX via 0x API
- Uses an LLM (MiMo v2.5 Pro via OpenRouter) as the decision engine
- Has a 1.9GB knowledge base (100+ trading books + YouTube interviews)
- Evaluates 10 trading pairs every 5 minutes
- Executes trades automatically with no human intervention

### Architecture

```
Knowledge Base (277 files, 1.9GB)
    ↓
LLM Prompt Assembly (system prompt + market data + knowledge units)
    ↓
LLM Decision (JSON: action, entry, stop_loss, take_profit, confidence)
    ↓
Execution (0x API swap on Arbitrum)
```

### Current Configuration

- **Pairs:** WETH, BTC, ARB, LINK, UNI, AAVE, PEPE, PENDLE, COMP, LDO
- **Timeframe:** 5m candles
- **Position Sizing:** Full deploy below $500 (100% of capital in single trade)
- **Max Positions:** 3
- **Max Risk Per Trade:** 20%
- **Min R:R:** 1.5:1 (1.2:1 below $50)
- **Fee Rate:** 0.1% (actual DEX costs: 0.3-0.8% round-trip)
- **Slippage:** 0.5%

### Knowledge Base Content

- **Books:** Wyckoff, Elder, Bulkowski, VPA, Turtle Trading, Market Wizards
- **YouTube:** Fabio (professional NASDAQ scalper), dual scalpers, Juvier daytrading, AI crypto grid bots
- **Topics:** Risk management, technical analysis, price action, psychology, execution, regime detection

### What We Found (Root Cause Analysis)

**3 Critical Bugs:**

1. **`risk_constraints.md` not loaded into LLM prompt** — The 84-line file with 8 management triggers, cognitive debiasing, and position sizing rules is never sent to the LLM. The LLM gets a one-liner: "Max risk: 20% | Max daily loss: 5%"

2. **`output_format.md` not loaded either** — The 86-line file with detailed JSON schema, zero-base forced-choice rules, and management trigger field definitions is replaced by a hardcoded string.

3. **`soul.md` says swing trading** — The agent's identity is "Momentum Swing Trading, Hold 2-24 hours, Target 3-10%". The operator needs scalping.

**Strategy Mismatch:**
- Agent built for: Swing trading (hold 2-24h, target 3-10%, scale out at TP1/TP2/TP3)
- Operator needs: Scalping (hold minutes, target 0.5-1%, high frequency, 100+ trades)

**Trade History (5 round trips, 0% win rate):**

| Pair | Entry | Exit | Loss | Hold |
|------|-------|------|------|------|
| LINK | $7.88 | $7.71 | -2.2% | ~48h |
| AAVE | $62.14 | $61.18 | -1.5% | ~24h |
| ARB | $0.0806 | $0.0794 | -1.5% | <1h |
| SOLVED | $5.11 | $5.07 | -0.8% | <1h |
| MNT | $0.0144 | $0.0144 | 0% | <1h |

---

## Research Questions

### 1. Prompt Engineering for Trading Agents

- What is the optimal prompt structure for an LLM-based trading agent?
- How should we balance identity (who the agent is) vs. rules (what the agent must do) vs. knowledge (what the agent knows)?
- Should the agent have a "personality" (aggressive, conservative) or be neutral?
- How many management triggers should the LLM evaluate? Too many causes analysis paralysis, too few causes bad exits.
- What is the optimal JSON schema for trading decisions? Single take-profit vs. 3-tier scale-out?
- How should we handle the tension between "be aggressive" (take action) and "be conservative" (don't lose money)?

### 2. Scalping Strategy Design

- What is the optimal hold time for crypto scalping on DEX? (5 minutes? 15 minutes? 1 hour?)
- What is the optimal take-profit target for scalping? (0.5%? 1%? 2%?)
- What is the optimal stop-loss for scalping? (0.3%? 0.5%? 1%?)
- Should we use fixed take-profit/target or dynamic (ATR-based)?
- What entry triggers work best for scalping? (RSI, MACD, volume, order flow, support/resistance?)
- How many entry triggers should be required? (2? 3? 4?)
- What is the optimal position sizing for scalping with $30? (Full deploy? 50%? 25%?)

### 3. Knowledge Base Optimization

- How should we select knowledge units for a scalping agent? (12 units out of 277 files)
- Should scalping knowledge be prioritized over swing trading knowledge?
- What is the optimal token budget for knowledge? (12K? 20K? 30K?)
- How should we weight YouTube interviews (crypto-native, fast execution) vs. institutional books (conservative, long-term)?
- Should we create a separate "scalping knowledge base" with curated units?

### 4. Management Trigger Design

- What management triggers are appropriate for scalping? (max hold, dead capital, drawdown limit)
- How should we handle the "dead capital" trigger for scalps? (3 cycles = 15 min is too long for scalps)
- Should we have a "take profit at 0.5%" management trigger?
- How should we handle the "max hold duration" trigger for scalps? (24h is too long, 15 min may be too short)
- What is the optimal circuit breaker configuration for scalping with $30?

### 5. Execution Optimization

- What is the optimal eval frequency for scalping? (1m? 5m? 15m?)
- Should we use LIMIT orders or MARKET orders for scalping?
- How should we handle slippage on Arbitrum DEX?
- What is the minimum R:R ratio for scalping? (1:1? 1.5:1? 2:1?)
- How should we handle the spread filter for scalping? (30bps? 50bps? 100bps?)

### 6. Risk Management for Micro-Accounts

- What is the optimal risk framework for a $30 account?
- Should we use full deploy or split across positions?
- What is the maximum acceptable loss per trade? ($0.50? $1.00? $1.50?)
- What is the optimal daily loss limit for scalping? ($1.50? $3.00? $5.00?)
- How should we handle consecutive losses? (3 losses = stop? 5 losses = stop?)

### 7. Session Timing

- What are the optimal trading hours for crypto scalping on Arbitrum?
- Should we avoid low-liquidity sessions (02:00-06:00 UTC)?
- How does session timing affect scalping performance?

### 8. Model Selection

- Is MiMo v2.5 Pro the right model for scalping decisions?
- Should we use a faster model (DeepSeek-V3) for scalping?
- How does model latency affect scalping performance?
- Should we use streaming responses for faster execution?

---

## Constraints

- **Budget:** $30 total, no additional capital
- **Exchange:** Arbitrum DEX only (no CEX)
- **Leverage:** None (spot only)
- **Pairs:** Crypto only (WETH, BTC, ARB, LINK, UNI, AAVE, PEPE, PENDLE, COMP, LDO)
- **Execution:** 0x API on Arbitrum
- **Model:** MiMo v2.5 Pro via OpenRouter (can change if needed)
- **Goal:** 100+ trades making $0.05-0.10 each, high win rate

---

## What We Want

1. **Optimal prompt structure** for a scalping agent (identity + rules + knowledge)
2. **Optimal scalping strategy** (entry triggers, hold time, take-profit, stop-loss)
3. **Optimal knowledge selection** for scalping (which knowledge units, how many, what budget)
4. **Optimal management triggers** for scalping (max hold, dead capital, take-profit)
5. **Optimal risk framework** for $30 account (position sizing, daily loss, circuit breakers)
6. **Specific recommendations** we can implement immediately

---

## Research Format

Please provide:

1. **Executive Summary** — 3-5 key findings that will have the biggest impact
2. **Prompt Architecture** — Exact prompt structure with sections, priorities, and content
3. **Scalping Strategy** — Entry triggers, hold time, take-profit, stop-loss, position sizing
4. **Knowledge Selection** — How to select and weight knowledge units for scalping
5. **Management Triggers** — Which triggers to keep, modify, or add for scalping
6. **Risk Framework** — Position sizing, daily loss, circuit breakers for $30 account
7. **Implementation Plan** — Specific changes to make, in order of priority
8. **Expected Impact** — What win rate and profit we can expect after optimization

---

## Additional Context

- The agent scores "Opus level" on benchmarks — the model is capable
- The knowledge base has 100+ world-class trading books — the knowledge exists
- The execution pipeline works — trades execute correctly on Arbitrum
- The problem is in the **prompt assembly** and **strategy design**, not the model or execution
- We've already identified 3 critical bugs (prompt loading failures) and a strategy mismatch
- We're willing to rewrite all prompt files if needed
- We want the fastest path to profitability with $30

---

## Questions for Gemini

1. Given our constraints ($30, DEX-only, spot-only), what is the highest-probability scalping strategy?
2. How should we structure the LLM prompt to maximize scalping performance?
3. What management triggers are appropriate for scalping (not swing trading)?
4. How should we select knowledge units for a scalping agent?
5. What is the minimum viable change set to start making money?
6. Are there any academic papers or case studies on LLM-based scalping agents?
7. What are the most common failure modes for AI scalping agents?
8. How do professional human scalpers structure their decision process?
