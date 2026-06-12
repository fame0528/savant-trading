# Gemini Deep Research Prompt: M3 Sandbox Reasoning-Action Divergence (v2)

---

**Copy everything below this line into Gemini Deep Research:**

---

I'm building an autonomous crypto trading engine in Rust that runs 24/7 on Arbitrum DEX via 0x API with a $30 micro-capital account. It uses `minimax/minimax-m3` (free, unlimited, 1M context) to make decisions every 5-minute candle.

## The Headline Finding

**My sandbox divergence count barely moved after fixing the primary root cause (a stale pair list in the system prompt).** Before the fix: 37/60 divergences. After: 34/60 divergences. But the **trade count got WORSE**:

| Metric | Before FID-125 | After FID-125 | Change |
|---|---|---|---|
| Buy actions | 3 | **0** | -3 |
| Sell actions | 1 | 0 | -1 |
| Close actions | 6 | 3 | -3 |
| Pass actions | 50 | 56 | +6 |

**The model went from 3 Buy actions to ZERO Buy actions** even when 15+ scenarios expected a Buy. When I read the raw responses, the LLM is not confused — it is correctly applying its own rules. The rules are just too strict for the synthetic sandbox data. Every setup fails at least one of 4 compounding filters, and the model concludes `Pass`.

## The 4 Compounding Filters (in order of application)

### Filter 1: 3+ Aligned Triggers Required (PRIMARY GATE)

The model's prompt says: *"For a scalp entry, I need 3+ action triggers aligned."*

Example from TRD-003 (Expected: Buy Medium Conviction):
> *"Bull triggers: EMA9 > EMA21 ✓ (1), but ADX 17.44 < 20 ✗, Volume 30.52 vs avg 120.57 (25%) ✗. Only 1-2/3 triggers met. Need 3+ aligned triggers."*

The 3+ threshold is rarely met in synthetic data. This is a **hard-coded decision framework**, not a learned behavior.

### Filter 2: Deep Asian Session Penalty (UNIVERSAL — 75% confidence haircut)

Every sandbox scenario runs in "Deep Asian" (02:00-05:59 UTC). The system prompt applies:
> *"Deep Asian — LIQUIDITY TROUGH, 42% less order book depth, breakout confidence penalty 75%, size multiplier 0.7x."*

**The "42% less depth" statistic is a heuristic number in the prompt — I have no empirical evidence it's accurate.** The model uses it as a negative trigger even when the setup is otherwise valid.

### Filter 3: Ranging Regime Override (ADX < 20)

Synthetic data consistently produces ADX < 20 (ranging), which triggers mean-reversion rules:
> *"Regime: Ranging (ADX 17.4) — mean reversion rules apply, support/resistance ARE triggers, momentum is suspended."*

The ranging regime requires different triggers than trending. Most sandbox scenarios test breakout behavior, but the data doesn't support a trending regime, so the model applies the wrong rule set.

### Filter 4: Low Volume Penalty (PERSISTENT)

Synthetic data shows volume at ~25% of 20-period SMA:
> *"Volume: 30.52 vs avg 120.57 — well below average (25% of normal)."*

This fails the "Volume > 1.5x average" trigger for every entry. Volume never spikes in synthetic data.

### Compounding

The 4 filters don't just add — they multiply. By the time all 4 apply, no setup passes:
- Trigger count fails (only 1-2/3 met)
- Session penalty makes confidence 75% lower
- Ranging regime says momentum signals don't count
- Low volume fails the volume confirmation

Result: model concludes "this setup is not ideal" and outputs `Pass`.

## What I Ruled Out

1. The pair list fix worked — model no longer refuses based on stale pair lists
2. The LLM is not confused — its reasoning is coherent and internally consistent
3. The issue is rule-data mismatch, not LLM capability — extreme on-chain capitulation (F&G=8, MVRV=0.70) still produces a Buy
4. The same model that produced 3 Buy actions before now produces 0 — something about FID-125 shifted the model toward more caution
5. Cost is not a constraint (free LLM with 1M context)

## What I Need Researched (4 Core Questions)

### Q1. Threshold Design — Is 3+ the Right Number?

**Concrete question:** Should the model require 3+ aligned triggers, or should it be 2+ with a confidence penalty? What are the tradeoffs?

**Sub-questions:**
- Is there research on **optimal trigger thresholds** for AI trading decisions? (Bayesian, fuzzy logic, conviction-weighted)
- How do other AI trading systems (TradingAgents, ai-hedge-fund, FinRL) handle the "good enough" cutoff?
- Should the threshold be **regime-dependent**? In trending regime (ADX > 25) where momentum signals are stronger, should the 3+ threshold drop to 2+? (Trending: 2+, Ranging: 3+, Volatile: 2+ with mandatory bear veto)
- What is the **"conviction-weighted entry"** framework? (2/3 triggers = 60% confidence trade, 3/3 = 85% confidence trade, 1/3 = Pass)
- The model has a **10-point pre-trade checklist** (regime, thesis, invalidation, stop, target, size, etc.). Is the rigid "all 10 required" approach wrong? Should partial completion be allowed with explicit reasoning?
- Should the prompt include **2-3 few-shot examples** of "2/3 triggers met → trade at 0.5% size with tight stop" to teach the model that partial alignment is tradeable?

### Q2. Sandbox Data Design — How to Test Decision Quality, Not Rule Compliance?

**Concrete question:** The current sandbox tests whether the model **follows its own rules**. But the model can correctly follow overly-strict rules and still make bad decisions (by passing on valid setups). How do I design a sandbox that tests decision quality?

**Sub-questions:**
- How to generate synthetic OHLCV data that produces **realistic volume spikes** (2x, 3x, 5x average) at breakout points — so scenarios can actually pass the volume filter
- How to generate synthetic data that produces **both trending and ranging regimes** (so the model can be tested on both rule paths)
- How to design **adversarial test scenarios** that test the EDGE of the rules (e.g., 2.5 triggers met, borderline volume, regime shifting mid-scenario)
- Should the sandbox use **real historical data** (replay actual market events like the May 2021 crash, FTX collapse, Luna death spiral) instead of synthetic data?
- How to design scenarios where **session and regime are not constraints** (e.g., test a setup during US-EU overlap with a clear trend — these are the "easy" scenarios the model should be able to pass)

### Q3. Counterfactual Evaluation — How to Grade Decisions Without Execution?

**Concrete question:** The current sandbox grader checks `action == expected_action`. But Pass is sometimes the right answer. How do I grade "correct Pass" vs "incorrect Pass"?

**Sub-questions:**
- How to use a **separate grader LLM** to evaluate the model's reasoning, not just its action? (e.g., "The model's reasoning was excellent, but the action was Pass due to overly-strict rules.")
- What is **counterfactual evaluation**? Simulate what would have happened if the model had taken the trade, and grade the decision based on that.
- How to track **decision confidence calibration** — when the model says Pass, what's the actual confidence? (51% Pass = uncertain, 95% Pass = certain. The sandbox should track this.)
- How to detect **"Pass as default"** behavior — where the model defaults to Pass not because of reasoning, but because Pass is the safest action
- Should the sandbox include **"golden Pass" scenarios** — setups where Pass is clearly the right answer (e.g., ranging market with no volume, no triggers, unclear thesis) — to verify the model can correctly identify when NOT to trade

### Q4. The 10-Point Checklist + Knowledge Base — Is One of Them the Source?

**Concrete question:** The model has a 10-point pre-trade checklist AND 265 knowledge units from curated sources. Could either of these be teaching over-strict behavior?

**Sub-questions:**
- Is the **10-point pre-trade checklist** the bottleneck? The model applies it rigidly because the prompt format forces it. Should it be flexible (passing on partial completion with reasoning) or rigid (all 10 required)?
- Are any of the **265 knowledge units** teaching over-strict behavior? E.g., a knowledge unit saying "always wait for 3+ confirmations" could be the actual source of the threshold, not the soul.md.
- How to **audit knowledge units** for over-strict language and revise them
- How to **A/B test** different threshold designs (3+ vs 2+ vs conviction-weighted) on the same scenarios — what metrics would prove the new design is better?
- Design a **specific A/B test:** same 60 scenarios, run with current 3+ threshold vs proposed 2+ threshold. What metrics matter? (Trade count, divergence count, counterfactual PnL, confidence calibration)

## Output Format

Produce a research document with:

1. **Threshold design recommendation** — concrete framework (Bayesian, fuzzy, conviction-weighted) with formulas and code patterns
2. **Sandbox data design** — synthetic data generation techniques for realistic volume spikes and dual-regime testing
3. **Counterfactual grading methodology** — how to grade decisions without execution
4. **Checklist + knowledge unit audit** — what to change in the model prompt and knowledge base
5. **Concrete A/B test design** — experimental methodology with specific metrics
6. **Adversarial test scenarios** — 10-20 specific scenarios designed to test the edge of the model's rules
7. **Implementation roadmap** — what to fix first, second, third (ordered by expected impact on divergence rate)
8. **End-state metrics** — what divergence rate is achievable, what's the trade-off between trade count and decision quality

## Research Sources to Consult

1. **AI agent evaluation frameworks** — AgentBench, GAIA, LM Evaluation Harness
2. **Reinforcement learning for trading** — FinRL, ElegantRL, AlphaGo decision frameworks
3. **Bayesian decision theory** — optimal thresholds under uncertainty
4. **Fuzzy logic control** — applications to trading
5. **Trading prop firm evaluation** — FTMO, TopStep, Apex Trader Funding
6. **Quantitative trading research** — Renaissance, Two Sigma, Citadel public papers
7. **Crypto market microstructure** — academic papers on session effects, regime detection
8. **Counterfactual evaluation** — causal inference for ML, off-policy evaluation
9. **Few-shot prompt engineering** — Anthropic, OpenAI research on example-based learning
10. **LLM calibration** — papers on confidence calibration, Brier score, ECE

## Constraints

- LLM: **minimax-m3** via TokenRouter (free, unlimited, 1M context window)
- Engine: **Rust + SQLite**
- Model prompt: **markdown** (soul.md, base_identity.md, strategy_knowledge.md, risk_constraints.md, output_format.md)
- Sandbox uses **synthetic OHLCV data** generated from scenario parameters
- Grader is **rule-based** (checks action against expected_action)
- Model is **not fine-tuned** — all behavior comes from prompting
- System runs on **Arbitrum DEX** via 0x API (spot only, no leverage, no shorts)
- Account size: **$30 micro-capital**
- 10-point pre-trade checklist (regime, thesis, invalidation, stop, target, size, conviction, correlation, catalyst)
- 265 knowledge units from curated sources
- Decisions every 5 minutes (288 cycles/day)
- 18+ active pairs evaluated in parallel each cycle

## What I'm Not Looking For

- General trading advice (I know the basics)
- Cost optimization (cost is not a constraint)
- Model architecture recommendations (locked into single LLM call per decision)
- Backtesting engine improvements (sandbox is a separate testing system)
- Live trading execution improvements (issue is in sandbox, not live engine)

## What I Am Looking For

- **Specific, actionable frameworks** for threshold design, session calibration, regime detection
- **Concrete code patterns** for conviction scoring, fuzzy logic, Bayesian updating
- **Synthetic data generation techniques** that produce realistic market signals
- **Adversarial test design** patterns for stress-testing the model's decision quality
- **Counterfactual evaluation** methods for grading decisions without execution
- **A/B testing methodology** with specific metrics
- **Checklist and knowledge unit audit** to identify the source of over-strict behavior

**The end goal is to minimize the divergence count as much as possible while maintaining decision quality — getting the model to make trades when it should and pass when it should, not just defaulting to Pass because the rules are too strict. Quantify the achievable target divergence rate and the trade-off between trade count and decision quality.**
