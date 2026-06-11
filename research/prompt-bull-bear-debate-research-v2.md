# Gemini Deep Research Prompt: Adversarial Debate for Autonomous Crypto Trading (v2)

---

**Copy everything below this line into Gemini Deep Research:**

---

I'm building an autonomous crypto trading engine in Rust that runs 24/7 on Arbitrum DEX via the 0x API. It has a $30 micro-capital account, trades spot crypto only (no leverage), and makes decisions every 5-minute candle using an LLM.

## Critical Context for This Research

**The LLM is free.** We run Owl Alpha — a model with 1 million token context window and near-Claude-Opus-level benchmarks, available through a free API. This means:
- Cost is NOT a constraint — we can run as many LLM calls as needed per candle
- Context window is massive — we can feed the full market data, history, memory, and debate transcript without truncation
- Quality of reasoning is high — we're not limited to small/cheap models for any role
- There is NO need for cost gating, token budgeting, or early-exit mechanisms
- There is NO need for model heterogeneity (different models for different agents) — we use the same free model for everything

**The debate must use the same data for both sides.** Both the Bull and Bear agents see IDENTICAL market data, indicators, regime analysis, on-chain metrics, funding rates, order book state, and memory. The adversarial difference comes purely from their opposing SYSTEM PROMPTS and reasoning mandates — not from information asymmetry. Different data slices would destroy the purpose of the debate, which is to have the same facts interpreted through opposing cognitive biases.

**The engine already has:**
- Momentum and mean-reversion strategies generating technical signals
- ADX-based regime detection (Trending / Ranging / Volatile)
- Episodic memory (stores every decision snapshot with full context)
- Semantic memory (curated trading knowledge)
- Risk management with hard constraints (0.5% max risk per trade, 5% daily loss limit, 40% confidence floor, time stops, break-even triggers)
- A single-shot LLM decision loop where one model call produces BUY/SELL/HOLD

**The current problem:** The LLM makes single-shot decisions with no adversarial challenge. It sees the data and decides in one pass, leading to overconfident entries, confirmation bias, and poor risk-adjusted returns.

## What I Need Researched (v2 — No Cost Constraints, Same Data)

### 1. Same-Data Adversarial Debate in Academic Research

- What does the research say about multi-agent debate where all agents receive IDENTICAL information but argue from opposing roles? How does this compare to information-asymmetric debate?
- The "society of mind" paradigm and MAD (Multi-Agent Debate) frameworks — do they require different data or just different personas?
- Are there studies specifically comparing same-data vs different-data multi-agent debate quality?
- How does Constitutional AI (same model, self-critique) compare to externalized debate (same data, different personas)?
- What's the evidence that same-data debate reduces overconfidence when the underlying model is high-quality (near Opus-level)?

### 2. Optimal Debate Architecture When Cost Is Irrelevant

- With no cost constraints, should we run **more** debate rounds (3-5 instead of 1-2)? Does quality continue to improve, or do LLMs hit circular argumentation plateaus?
- Should we run **parallel** debate rounds (bull and bear generate simultaneously) and then a judge synthesizes, or should it be **sequential** (bear responds to bull, bull responds to bear)?
- With a 1M context window, should the debate transcript be appended in full to every round, or should there be a "summary compression" step between rounds?
- How many **independent** bull/bear pairs should we run simultaneously for ensemble voting? (e.g., 3 pairs of bull/bear debating the same signal, then majority vote)
- What's the optimal **debate-to-decision ratio**? Should every potential trade go through debate, or only certain types (entries vs exits, high-confidence vs ambiguous)?

### 3. Regime-Adaptive Debate Structure

I have ADX-based regime detection (Trending / Ranging / Volatile). How should debate structure adapt:

- **Trending markets (ADX > 25):** Should the bear get extra rounds to challenge trend-following, or should the bull get fewer rounds since the trend is already bullish?
- **Ranging markets (ADX < 20):** Should debate be symmetric and deeper, since range-bound markets have more false signals?
- **Volatile markets (ATR expanding):** Should the bear always win ties, since volatility destroys micro-profit targets?
- Should the **number of debate rounds** be regime-dependent? (e.g., trending = 1 round, ranging = 3 rounds, volatile = 2 rounds with mandatory bear veto option)
- How should the **confidence threshold** for triggering debate change by regime?

### 4. Same-Data Debate Prompt Engineering

Since both agents see identical data, the ENTIRE adversarial quality depends on system prompt design:

- What specific **cognitive biases** should the Bear prompt be designed to invoke? (e.g., loss aversion, base rate neglect, availability heuristic)
- What specific **cognitive biases** should the Bull prompt be designed to invoke? (e.g., trend continuation, momentum bias, recency bias)
- How do you prevent **"polite agreement"** where the bear says "good points but..." and capitulates? What prompt mechanisms force genuine disagreement?
- Should the debate be **constrained to specific market dimensions**? (e.g., bear MUST argue about execution risk and liquidity, bull MUST argue about momentum and volume — no cross-dimensional arguments)
- How should **past debate outcomes** be injected? (e.g., "In the last 5 debates where the bear argued about funding rate divergence, the bear was right 3 times and wrong 2 times")
- What's the right **tone and persona** for each agent? Should the bear be paranoid and aggressive, or冷静 and analytical? Does the persona affect decision quality?

### 5. Structured Output for Debate Agents

Instead of free-text debate, should agents output structured scores?

- Should bull and bear each score the trade on **the same dimensions** (momentum strength, risk proximity, volume confirmation, funding rate alignment, liquidity depth) using a 1-10 scale?
- How should the **judge/final decision module** weight these scores against each other and against the regime?
- Should the debate produce a **confidence delta** (how much did the bear change the bull's confidence) rather than absolute scores?
- What's the minimum number of scored dimensions needed for a useful structured debate?
- How do you handle **disagreement on facts** vs **disagreement on interpretation**? (e.g., both agree RSI is 72, but disagree on whether that's overbought or momentum confirmation)

### 6. Debate Integration with Existing Risk Management

- How should the debate **interact with the 40% confidence floor**? If the bull starts at 65% confidence and the bear degrades it to 42%, should the trade proceed or be rejected?
- Should the debate be able to **change the trade direction** (e.g., bear convinces bull that a long is actually a short setup)?
- How should the debate interact with **management triggers** (time stops, break-even rules, regime shifts)? Should these be debated or remain deterministic?
- Should there be a **"mandatory skip"** condition where debate is bypassed entirely? (e.g., when a hard circuit breaker is already triggered)
- How does debate affect **position sizing**? If debate is strongly bullish (8/10 consensus), should position size increase? Or should size always remain fixed?

### 7. Measuring Debate Effectiveness (No Cost Constraints)

Since we can run both single-shot and debate simultaneously for free:

- How long should we run **shadow mode** (debate runs alongside real trading but doesn't influence it) before switching to debate-driven decisions?
- What **minimum sample size** of debate outcomes do we need to determine statistical significance? Given 5-minute candles = 288/day, how many days of shadow data?
- What **primary metrics** should we compare? Win rate, average profit per trade, max drawdown, Sharpe ratio, confidence calibration (are 60% confidence trades actually winning 60% of the time)?
- Should we track **debate agreement rate** (how often bull and bear agree immediately vs disagree)? A high agreement rate might mean the debate is redundant for certain setups.
- How do we detect **debate degradation** — where the agents start agreeing too quickly due to shared model weights?

### 8. Creative Architecture Variations for Free, Unlimited LLM

With no cost constraints and 1M context, we can explore architectures that would be prohibitively expensive for other systems:

- **Full transcript replay:** Every debate round includes the complete history of ALL previous debates for this pair. The agents can learn from historical patterns across hundreds of past debates.
- **Meta-debate:** After the bull/bear debate, a third agent reviews the debate transcript and argues whether the debate itself was well-conducted. ("The bear only cited price action and ignored the on-chain divergence — this debate was incomplete.")
- **Parallel universe simulation:** For high-confidence signals, run 3 separate bull/bear debates with slightly different temperature settings. If all 3 reach the same conclusion, confidence is very high. If they diverge, the signal is ambiguous.
- **Historical replay testing:** Feed the agent the last 100 candles of data one at a time, letting it "practice" debating past market conditions. Score its past predictions against actual outcomes. Use these scores to calibrate current debate confidence.
- **Continuous debate during position hold:** Instead of debating only at entry, maintain a running bull/bear debate while a position is open. If the bear wins the ongoing debate, close the position immediately.

### 9. Crypto DEX-Specific Debate Arguments

For a $30 spot account on Arbitrum via 0x API, what specific arguments should each agent be mandated to make:

- **Bear MUST argue about:** 0x API slippage quotes vs actual execution, MEV/sandwich attack risk on public mempools, liquidity depth on target AMMs, funding rate divergence (even for spot), exchange whale inflows, network congestion affecting gas
- **Bull MUST argue about:** Momentum continuation probability, volume confirmation, regime alignment, sentiment support, break of structure, risk-reward ratio validation
- **Both MUST address:** The mathematical reality of $0.15 max risk per trade, the 0.25-0.30% round-trip fee drag, the 5-minute candle timeframe constraints

### 10. Pre-Mortem vs Full Debate

- Is a **pre-mortem** (imagine the trade failed, write why) a viable lightweight alternative to full bull/bear debate?
- Can pre-mortem be combined with debate? (e.g., pre-mortem first as a filter, then debate only if the pre-mortem identifies specific risks)
- What does the research say about pre-mortem effectiveness for reducing overconfidence in LLM trading agents?
- Should the pre-mortem be a **separate agent** or the same agent with a different prompt?

---

Please search for academic papers, open-source implementations (especially TradingAgents, ai-hedge-fund, thesis-agent, PolySwarm), blog posts from quantitative trading firms, and any real-world case studies on adversarial debate for autonomous trading systems. Focus specifically on same-data debate architectures and the impact of debate rounds on decision quality when cost is not a constraint.
