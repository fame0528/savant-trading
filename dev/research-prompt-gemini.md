# Gemini Deep Research Prompt: Rapid Growth Strategy for LLM-Powered Crypto Trading

## Research Objective

Determine the optimal strategy for rapidly growing a $26 crypto trading account using an LLM (Large Language Model) as the primary decision-maker. The system trades on-chain via DEX (0x API on Arbitrum) with real money. The goal is maximum profit in minimum time.

---

## System Overview

### What We Built
An autonomous crypto trading agent ("Savant") that:
- Trades on-chain via 0x API (Arbitrum DEX, no CEX)
- Uses an LLM (MiMo v2.5 Pro via OpenRouter) to make BUY/SELL/HOLD decisions
- Evaluates 10 curated pairs every 5 minutes: ETH, BTC, ARB, LINK, UNI, AAVE, PEPE, PENDLE, COMP, LDO
- Has trailing stop-losses, take-profit targets, and circuit breakers
- Runs 24/7 on a Windows machine

### Current Performance
- **Starting capital:** ~$50
- **Current capital:** ~$26 (lost ~$16 to a scam token buy in early development)
- **Closed trades:** 7 total, 5 wins (71% win rate)
- **Realized profit:** $0.21
- **Unrealized profit:** ~$0.50 (2 open positions: ETH long + LINK long)
- **API costs:** ~$10/day (OpenRouter credits for LLM calls)
- **Net result:** Spending $10/day to make $0.67/day — **net negative**

### Current Constraints
- **Starting capital:** $50 (now $26 after $16 loss to a scam token)
- **Capital:** $26 total ($0.26 USDC cash + ~$25 in WETH + LINK positions)
- **Design target:** $50 starting capital — the system was built from day one for small accounts, not institutional scale
- **Gas:** ~$0.01 per swap on Arbitrum
- **API cost:** ~$0.01-0.02 per LLM call (MiMo v2.5 Pro, ~15K input tokens + 8K output tokens)
- **Cycle frequency:** Every 5 minutes
- **Pairs evaluated:** 10 (reduced from 455+ via pre-filters)
- **LLM calls per cycle:** 5-8 (after pre-filters: stablecoins, dead tokens, GoPlus safety, RSI/ADX/EMA pre-scoring)
- **Trading hours:** 24/7

### Technical Architecture
- **Execution:** 0x API v2 with Permit2 signing on Arbitrum (chain ID 42161)
- **Data:** Kraken WebSocket for real-time prices, 5-minute candles
- **Model:** xiaomi/mimo-v2.5-pro (reasoning model, 128K context, MoE architecture)
- **System prompt:** ~52K characters (~13K tokens) including soul.md identity, strategy knowledge, risk constraints, knowledge units, market data
- **Max output tokens:** 8,192 (reduced from 16,384 — actual output is ~500-2000 tokens)
- **Temperature:** 0.6, top_p: 0.95

### Cost Optimization Already Implemented (FID-056)
1. Skip LLM eval when fully deployed (balance < $1.00)
2. Candle hash cache (skip pairs with unchanged data)
3. Smart pre-scoring (RSI/ADX/EMA filter before LLM — eliminates 50-70% of pairs)
4. Reduced max_tokens (16384 → 8192)
5. Reduced knowledge budget (20K → 12K chars)
6. Skip eval if no new candle formed

**Estimated savings:** 70-90% reduction in API calls. But even at 5-15 calls/hour, costs still exceed profits at $26 capital.

---

## Core Problem

**The economics don't work at small scale.** The LLM costs more per decision than the decisions can earn. A $0.02 API call to decide on a $26 position that might gain 2% ($0.52) means the system needs a >25:1 profit-to-cost ratio just to break even on API fees alone.

The system was designed from day one for a $50 starting foundation — not institutional capital. But even at its intended scale, API costs are currently eating profits alive.

---

## Research Questions

### 1. Optimal Trading Frequency for Small Accounts
- What's the ideal trade frequency for a $26 account trading crypto on-chain?
- Should we be scalping (multiple trades per hour), day trading (few trades per day), or swing trading (few trades per week)?
- What's the minimum profitable trade size given gas costs ($0.01/swap) and slippage?
- How does frequency interact with LLM API costs?

### 2. LLM Cost vs. Decision Quality Tradeoff
- At what portfolio size does LLM-powered trading become profitable?
- Are there cheaper LLM models that can match MiMo v2.5 Pro's trading performance?
- **Can local models (Gemma 4, Llama 4, Qwen 3, DeepSeek) run via Ollama replace the API entirely?**
- We have a sandbox testing suite that can evaluate any model against 60+ trading scenarios with structured scoring (parse rate, win rate, Brier score). This is how we'd validate a model switch.
- The tradeoff: free local compute (0 API cost) vs. model capability (reasoning quality, JSON compliance, instruction following)
- Could we use a hybrid approach: rule-based for simple decisions, LLM for complex ones?
- What's the minimum viable prompt size? (Currently ~13K input tokens — can we cut this?)
- Could we batch multiple pairs into a single LLM call instead of one call per pair?

### 3. Aggressive Growth Strategies for Micro-Accounts
- What strategies work best for growing $26 → $100 → $500 → $10,000?
- Should we focus on high-volatility meme coins (PEPE, etc.) for larger % swings?
- Is leverage trading available on DEX? (Aave, GMX on Arbitrum)
- Should we concentrate all capital into 1-2 high-conviction trades instead of diversifying?
- What about copy-trading or following whale wallets on-chain?

### 4. On-Chain Alpha Generation
- What on-chain signals predict short-term price movements? (whale transfers, DEX volume spikes, funding rate flips, liquidation cascades)
- Can we front-run or react to on-chain events faster than the LLM evaluates?
- What's the alpha decay on common signals? (How quickly does a signal lose predictive power?)
- Are there Arbitrum-specific opportunities? (Arbitrum incentives, airdrops, protocol launches)

### 5. Risk Management at Small Scale
- With $26, should we even use stop-losses? (A 15% stop on $26 = $3.90 loss — nearly 15% of portfolio)
- Should we use Kelly Criterion for position sizing at this scale?
- How do we handle the "recovery problem" — after a loss, the account is too small to recover via normal trading?
- Is it better to accept higher risk (no stops, full concentration) at small scale and only add risk management above $500?

### 6. Time-to-Scaling Analysis
- Starting from $26 with optimal strategy, what's a realistic timeline to reach $100? $500? $1,000?
- What's the compounding math? (If we can make 5% per day on $26, that's $1.30/day → $26 in 20 days → $52 total → ...)
- What are the bottlenecks? (API costs, gas, liquidity, slippage, market conditions)
- At what point should we increase position size vs. keep compounding?

### 7. Alternative Revenue Streams
- Should the trading bot also farm yield? (Aave lending, LP provision)
- Could we run the bot on multiple chains simultaneously? (Arbitrum + Base + Optimism)
- Is MEV extraction viable on Arbitrum with our setup?
- Could we sell trading signals or bot access to others?

### 8. Market Regime Adaptation
- How should the strategy change in bull vs. bear vs. sideways markets?
- Current Fear & Greed: 12 (Extreme Fear) — should we be buying or waiting?
- What indicators predict regime changes?
- How does the LLM need to adapt its prompt based on market regime?

---

## What We Need

A concrete, actionable growth plan that:
1. **Maximizes profit velocity** — make money as fast as possible
2. **Minimizes costs** — ideally $0 in API costs if local models can handle the task
3. **Scales linearly** — strategies that work at $26 should also work at $260 and $2,600
4. **Is implementable** — we can code changes within 1-2 days
5. **Has clear milestones** — $26 → $50 → $100 → $500 → $1,000 with specific strategies at each tier
6. **Is testable** — we have a sandbox suite with 60+ scenarios that scores any model on parse rate, win rate, Brier score, and response quality. Any proposed model change will be validated through this suite before going live.

### Local Model Opportunity
We can run models locally via Ollama (Gemma 4, Llama 4, Qwen 3, DeepSeek) and eliminate API costs entirely. The sandbox testing suite was built specifically for this — to benchmark local vs. cloud models on the same trading scenarios. The key question: **can a free local model match MiMo v2.5 Pro's trading judgment, or does the quality drop make it unprofitable?**

Include specific recommendations for:
- Optimal trade frequency and holding period
- Position sizing and concentration
- LLM prompt optimization (cost reduction without quality loss)
- On-chain signals to prioritize
- Risk parameters for each account tier
- Expected timeline and returns at each stage
