# Gemini Deep Research Prompt: M3 Sandbox Reasoning-Action Divergence

---

**Copy everything below this line into Gemini Deep Research:**

---

I'm building an autonomous crypto trading engine in Rust that runs 24/7 on Arbitrum DEX via the 0x API. It has a $30 micro-capital account, trades spot crypto only (no leverage), and makes decisions every 5-minute candle using an LLM. I have a sandbox that runs the LLM against 60 curated market scenarios and grades every decision against a rubric.

## The Problem I'm Stuck On

After fixing the **primary** root cause (a stale pair list in the system prompt that made the LLM refuse to trade BTC/USD because "BTC is not in my configured pairs"), the sandbox divergence count only dropped from 37/60 to 34/60. The model now correctly considers BTC tradeable, but it still outputs `Pass` (do nothing) for 56/60 scenarios. The 4 scenarios that did produce trade actions all output `Close` — **zero `Buy` actions and zero `Sell` actions** even when 15+ scenarios expected a Buy.

When I read the raw model responses, the LLM is not confused. It is correctly applying its own rules. The problem is that its rules are **too strict for the sandbox data** — every setup fails at least one of 4 compounding filters.

## What the Sandbox Data Looks Like

The sandbox generates synthetic OHLCV candles for 60 scenarios covering 11 categories: Trend Bull, Trend Bear, Range Bound, Volatility, Catalyst, Correlation, Edge Case, Microstructure, On-Chain, Sentiment, Session. Each scenario has an expected_action (Buy / Sell / Short / Close / Pass) with a conviction level.

The sandbox reports show systematic failures:
- **Trend Bull (8 scenarios, 0.18 avg score):** All expected Buys missed
- **On-Chain (5 scenarios, 0.00 avg score):** All 5 failed
- **Correlation (7 scenarios, 0.32 avg score):** 4 of 7 failed
- **Most common failure mode:** "Missed trade: expected Buy"

## The 5 Compounding Filters That Block Every Trade

I read 10+ raw LLM responses from the sandbox. Every single one applies the same chain of filters and concludes `Pass`. Here are the 5 mechanisms, in the order the model applies them:

### Filter 1: 3+ Aligned Triggers Required (PRIMARY GATE)

The model's prompt says: *"For a scalp entry, I need 3+ action triggers aligned."* Then it counts triggers methodically.

Example from TRD-003 (Expected: Buy Medium Conviction):
> *"Bull triggers: EMA9 > EMA21 ✓ (1), but ADX 17.44 < 20 ✗, Volume 30.52 vs avg 120.57 (25%) ✗. Only 1-2/3 triggers met. Need 3+ aligned triggers."*

The 3+ threshold is rarely met in synthetic data. Even when 2 of 3 triggers fire (e.g., bullish EMA + RSI neutral + regime borderline), the model refuses. This is a **hard-coded decision framework**, not a learned behavior.

### Filter 2: Deep Asian Session Penalty (UNIVERSAL)

Every sandbox scenario runs in the "Deep Asian" trading session (02:00-05:59 UTC). The system prompt applies aggressive penalties:

> *"Deep Asian — LIQUIDITY TROUGH, 42% less order book depth, breakout confidence penalty 75%, size multiplier 0.7x."*

The model internalizes "Asian session = bad for breakouts" and uses this as a **negative trigger** even when the setup is otherwise valid.

### Filter 3: Ranging Regime Override (DOMINANT REGIME)

Synthetic data consistently produces ADX < 20 (ranging regime), which triggers mean-reversion rules:

> *"Regime: Ranging (ADX 17.4) — mean reversion rules apply, support/resistance ARE triggers, momentum is suspended."*

The ranging regime requires different triggers than trending. Most sandbox scenarios test breakout behavior, but the data doesn't support a trending regime, so the model applies the wrong rule set.

### Filter 4: Low Volume Penalty (PERSISTENT)

Synthetic data shows volume at ~25% of the 20-period SMA:

> *"Volume: 30.52 vs avg 120.57 — well below average (25% of normal)."*

This fails the "Volume > 1.5x average" trigger for every entry. Volume never spikes in synthetic data.

### Filter 5: Compounding Effects

The 4 filters don't just add — they multiply. By the time all 4 apply, no setup passes:

- Trigger count fails (only 1-2/3 met)
- Session penalty makes confidence 75% lower
- Ranging regime says momentum signals don't count
- Low volume fails the volume confirmation

Result: model concludes "this setup is not ideal" and outputs `Pass`.

## What I've Already Ruled Out

I confirmed via raw response capture (saved 60 JSON files with full LLM output) that:

1. **The pair list fix worked** — the model no longer refuses based on stale pair lists. It now reasons about BTC/USD as a legitimate pair.
2. **The LLM is not confused** — its reasoning is coherent and internally consistent. It's just consistently applying rules that the sandbox data doesn't satisfy.
3. **The issue is rule-data mismatch, not LLM capability** — when given extreme on-chain capitulation data (Fear & Greed = 8, MVRV = 0.70), the model commits to a Buy. It can override its own rules with overwhelming signal.
4. **Cost is not a constraint** — I'm using `minimax/minimax-m3` via TokenRouter (free promotional tier). I can run unlimited LLM calls per scenario.
5. **The same model that was producing 3 Buy actions before now produces 0** — so something about the FID-125 prompt change shifted the model's behavior toward more caution, not less.

## What I Need Researched

### 1. Decision Threshold Design for AI Trading Agents

- What is the **optimal trigger threshold** for AI trading decisions? Is 3+ the right number, or should it be lower (2+) with a confidence penalty?
- Research **Bayesian decision frameworks** for trading — how to update confidence based on partial trigger alignment instead of hard cutoffs.
- Research **fuzzy logic** approaches to trigger counting — e.g., a 2/3 trigger setup is worth 0.6 confidence, not 0.
- How do other AI trading systems (TradingAgents, ai-hedge-fund, FinRL, thesis-agent) handle the "is this setup good enough" decision?
- Should the threshold be **regime-dependent**? (Trending: 2+, Ranging: 3+, Volatile: 2+ with mandatory bear veto)
- Research the **"conviction-weighted entry"** approach — instead of "3+ triggers OR pass", use "2+ triggers at 60% confidence, 3+ triggers at 75% confidence"

### 2. Session Penalty Calibration

- What is the **empirical evidence** for crypto session effects? Does Asian session actually have 42% less depth, or is this a trading myth?
- Research how **prop trading firms** and **quantitative funds** treat session effects — do they apply confidence penalties or just position size adjustments?
- Should Deep Asian penalty be **75% breakout confidence** (very aggressive) or something more like 25% (acknowledging reduced liquidity but not invalidating setups)?
- Is there a difference between **liquidity-based** penalties (real: depth is lower) and **statistical** penalties (questionable: breakout failure rate in Asian session)?
- How do you **empirically measure** if a session penalty is helping or hurting? A/B test with and without the penalty on the same scenarios.

### 3. Regime Detection and Testing

- Research ADX threshold sensitivity — is **ADX < 20 = ranging** the right cutoff, or should it be ADX < 15 (more permissive) or ADX < 25 (stricter)?
- How to **generate synthetic data that produces both trending and ranging regimes** for testing both rule paths
- Should the regime trigger be **"AND" or "OR"** with other triggers? E.g., "EMA cross + (ADX > 20 OR Volume > 1.5x)" vs strict "EMA cross + ADX > 20 + Volume > 1.5x"
- Research **regime-aware position sizing** — instead of refusing trades in ranging regime, use tighter stops and smaller size

### 4. Volume Profile Design for Synthetic Data

- How to generate synthetic candles with **realistic volume profiles** that include volume spikes (2x, 3x, 5x average) at breakout points
- Research **volume clustering** in real markets — do volume spikes cluster at breakouts, and how to simulate that in synthetic data
- Should the sandbox use **real historical volume profiles** from Kraken/Bybit for each scenario, or generate volume procedurally?
- How do you design scenarios where volume is **not a constraint** (e.g., testing a thesis that doesn't depend on volume confirmation)

### 5. Sandbox Design for AI Decision Quality (Not Rule Compliance)

This is the **meta-question**. The current sandbox tests whether the model **follows its own rules**. But the model can correctly follow overly-strict rules and still make bad decisions (by passing on valid setups).

- How to design sandboxes that test **decision quality** rather than rule compliance
- Research **counterfactual evaluation** — "if the model had taken the trade, would it have been profitable?" (without actually executing)
- Should the sandbox use a **separate grader LLM** to evaluate the model's reasoning, not just its action? (e.g., "The model's reasoning was excellent, but the action was Pass due to overly strict rules.")
- Research **adversarial test design** — scenarios designed to test the **edge** of the rules (e.g., 2.5 triggers met, borderline volume, regime shifting)
- How to grade "correct Pass" vs "incorrect Pass" — when is Pass the right answer? (Hypothesis: Pass is correct when the expected move is < 0.5% or when the thesis is unclear)
- Research **decision confidence calibration** — when the model says Pass, what's the actual confidence in that decision? If it's 51% Pass / 49% Trade, the model is uncertain. If it's 95% Pass / 5% Trade, it's certain. The sandbox should track this.

### 6. Alternative Decision Frameworks

- **Risk-adjusted conviction score:** Combine trigger count, session penalty, regime alignment, volume confirmation into a single 0-100 conviction score. Trade if score > threshold (e.g., 65). This replaces 4 binary filters with 1 continuous score.
- **Bayesian updating:** Start with prior probability of success (e.g., 50% for any setup). Update with each trigger. Trade if posterior > threshold.
- **Kelly criterion variant:** Calculate position size based on conviction score, not trigger count. Even a 2/3 trigger setup is tradeable at 0.3% risk instead of 1% risk.
- **Fuzzy trigger counting:** 3/3 triggers = 1.0 confidence, 2/3 = 0.7, 1/3 = 0.3, 0/3 = 0. Trade if confidence × base_conviction > threshold.
- **Meta-cognitive override:** Allow the model to override its own rules with explicit reasoning. ("All 3 filters failed, but the on-chain capitulation signal is so strong that I'm overriding to Buy.") This is what already happens in the ONC-003 case.

### 7. The Pair List Was Just the Surface

What other **system prompt assumptions** might be similarly stale or incorrect?

- The model thinks it trades 10 curated altcoin pairs, but the engine actually discovers 18+ pairs dynamically
- The model applies Deep Asian penalty, but the engine runs 24/7
- The model checks 3+ triggers, but the engine's position sizer already sizes down for low-confidence trades
- **Research prompt hygiene for AI trading agents** — what other "facts" in the system prompt might be out of sync with the actual engine behavior?

### 8. Specific Sub-Questions

1. Should the 3+ trigger threshold be reduced to 2+ in the model prompt, with a confidence penalty for the 2/3 case?
2. Should Deep Asian session penalty be relaxed (75% → 25%) or removed entirely for sandbox testing?
3. Should the sandbox pre-filter scenarios that obviously fail the trigger filter, or test the model's ability to recognize when "the data doesn't support a trade"?
4. Should the sandbox use **real historical data** (replay actual market events) instead of synthetic data, to get more realistic signal patterns?
5. How to design **adversarial scenarios** that specifically test the model's ability to override its own rules with strong signal?
6. What **metrics** should the sandbox track to detect over-strict rule application? (e.g., "Pass rate" vs "trade rate", "average conviction on Pass decisions", "how often the model overrides its own filters")
7. Should the sandbox **include a counterfactual grader** — simulate what would have happened if the model had taken the trade, and grade the decision based on that?
8. How to **A/B test** different threshold designs (3+ vs 2+ vs conviction-weighted) on the same scenarios?

## Output Format

Produce a comprehensive research document covering:

1. **Threshold design recommendations** — concrete frameworks (Bayesian, fuzzy, conviction-weighted) with formulas and code patterns
2. **Session penalty calibration** — empirical evidence, recommended values, A/B test methodology
3. **Regime detection improvements** — ADX threshold sensitivity, regime-aware decision frameworks
4. **Volume profile design** — synthetic data generation techniques that produce realistic volume spikes
5. **Sandbox design patterns** — how to test decision quality, not rule compliance
6. **Counterfactual evaluation** — how to grade decisions without executing them
7. **Adversarial test design** — 10-20 specific scenarios designed to test the edge of the model's rules
8. **Specific recommendations for my system** — what to change in the model prompt, what to change in the sandbox, what to change in the data generation
9. **Implementation roadmap** — what to fix first, second, third (ordered by expected impact on divergence rate)
10. **Metrics to track** — how to measure if the changes are working

## Research Sources to Consult

1. **AI agent evaluation frameworks** — AgentBench, GAIA, SWE-bench, LM Evaluation Harness
2. **Reinforcement learning for trading** — DeepMind's AlphaGo-style approaches, FinRL, ElegantRL
3. **Bayesian decision theory** — optimal decision thresholds under uncertainty
4. **Fuzzy logic control systems** — applications to trading
5. **Trading prop firm evaluation** — FTMO, TopStep, Apex Trader Funding
6. **Quantitative trading research** — Renaissance Technologies, Two Sigma, Citadel
7. **Crypto market microstructure** — academic papers on session effects, regime detection
8. **LLM evaluation for domain-specific tasks** — MedQA, LegalBench, etc.
9. **Sandbox design for AI agents** — Anthropic's Claude Code sandboxing, OpenAI's function calling sandbox
10. **Counterfactual evaluation** — causal inference for ML, off-policy evaluation

## Constraints

- The LLM is **free and unlimited** (minimax-m3 via TokenRouter, 1M context window)
- The engine is **Rust + SQLite** (existing stack)
- The model prompt is **markdown** (soul.md, base_identity.md, strategy_knowledge.md, risk_constraints.md, output_format.md)
- The sandbox uses **synthetic OHLCV data** generated from scenario parameters
- The grader is **rule-based** (checks action against expected_action)
- The model is **not fine-tuned** — all behavior comes from prompting
- The system runs on **Arbitrum DEX** via 0x API (spot only, no leverage, no shorts)
- Account size is **$30 micro-capital** — every decision matters
- The model uses a **10-point pre-trade checklist** (regime, thesis, invalidation, stop, target, size, etc.)
- Circuit breakers: 2% daily loss → half size, 3% → close all, 5% weekly → stop 48h
- The model has access to **265 knowledge units** from curated sources
- Decisions are made **every 5 minutes** (288 cycles/day)
- The engine evaluates **all 18+ active pairs in parallel** each cycle

## What I'm Not Looking For

- General trading advice (I know the basics)
- Cost optimization (cost is not a constraint)
- Model architecture recommendations (I'm locked into a single LLM call per decision)
- Backtesting engine improvements (the sandbox is a separate testing system)
- Live trading execution improvements (the issue is in the sandbox, not the live engine)

## What I Am Looking For

- **Specific, actionable frameworks** for threshold design, session calibration, regime detection
- **Concrete code patterns** for implementing conviction scoring, fuzzy logic, Bayesian updating
- **Synthetic data generation techniques** that produce realistic market signals
- **Adversarial test design** patterns for stress-testing the model's decision quality
- **Counterfactual evaluation** methods for grading decisions without execution
- **A/B testing methodology** for comparing threshold designs
- **Metrics** to track that distinguish "model is being too strict" from "model is being appropriately cautious"

The end goal is to get the divergence count from 34/60 down to < 15/60, with the model making trades when it should and passing when it should — not just passing on everything because the rules are too strict.
