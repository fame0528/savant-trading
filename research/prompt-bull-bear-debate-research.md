# Gemini Deep Research Prompt: Adversarial Debate for Autonomous Crypto Trading

---

**Copy everything below this line into Gemini Deep Research:**

---

I'm building an autonomous crypto trading engine in Rust that runs 24/7 on Arbitrum DEX via the 0x API. It has a $30 micro-capital account, trades spot crypto only (no leverage), and makes decisions every 5-minute candle using an LLM (OpenRouter). The engine already has:

- **Momentum and mean-reversion strategies** that generate technical signals
- **Regime detection** (ADX-based: Trending / Ranging / Volatile)
- **A single-shot LLM decision loop** — the model sees market data, indicators, risk constraints, and knowledge, then outputs a JSON trade decision
- **Memory system** — episodic (stores every decision snapshot) and semantic (curated trading knowledge)
- **Sandbox** — simulated trading environment with grading (compliance, R:R, reasoning quality)
- **Risk management** — circuit breakers, ATR-based stops, time stops, confidence floors

The current problem: the LLM makes **single-shot decisions** with no adversarial challenge. It sees the data and decides BUY/SELL/HOLD in one pass. This leads to overconfident entries, confirmation bias, and poor risk-adjusted returns.

I found a pattern in the open-source **TradingAgents** framework (multi-agent financial trading system) that uses a **Bull vs Bear debate** before every trade. Two LLM agents argue for and against the trade across multiple rounds, and the debate transcript feeds into a final decision. There's also a separate **Risk Analyst Trio** (aggressive, conservative, neutral) that debates the proposed trade before a Portfolio Manager gives final approval.

## What I Need Researched

### 1. Academic and Industry Foundations of Adversarial Debate for Decision-Making

- What does the research say about **adversarial debate improving LLM decision quality**? Look into papers on "society of mind", "multi-agent debate", "self-play for LLMs", "red teaming for decision quality"
- How does **Constitutional AI's** debate/challenge mechanism compare to what I'm describing?
- Are there papers on **adversarial collaboration** in quantitative finance or algorithmic trading specifically?
- What's the evidence that multi-round debate reduces overconfidence vs single-shot reasoning?

### 2. Optimal Debate Architecture for Real-Time Trading

- Should the bull and bear agents see **identical data** or **different slices** of data? (e.g., bull sees momentum/sentiment, bear sees risk/regime/fundamentals)
- How many **debate rounds** are optimal? Is 1 round enough (like TradingAgents defaults to), or does the quality improve with 2-3 rounds? What about diminishing returns and latency?
- Should the debate be **asymmetric** (one side has burden of proof) or **symmetric** (equal weight)?
- Should the bull/bear agents use the **same LLM** or **different models** (e.g., GPT for bull, Claude for bear)?
- How should **confidence scores** change through debate? Does the bull's confidence decrease when the bear makes strong counterarguments?

### 3. Crypto-Specific Considerations

- In crypto markets (24/7, high volatility, sentiment-driven, low liquidity on DEXs), what are the **strongest bear arguments** that a bull agent would miss?
- How should debate adapt to **market regime**? (e.g., in a trending bull market, should the bear get extra weight? In a ranging market, should the bull be penalized?)
- Should the debate incorporate **on-chain data** (whale movements, exchange inflows/outflows, funding rates) as ammunition for either side?
- How does **MEV/sandwich attack risk** factor into the debate? Should the bear always argue against execution risk on DEXs?

### 4. Integration with Existing Risk Management

- How should the debate interact with **hard risk constraints** (0.5% max risk per trade, 5% daily loss limit, 40% confidence floor)?
- Should the debate **override** the technical signal, **moderate** it (adjust confidence), or only act as a **tiebreaker** when the signal is ambiguous?
- Should the risk circuit breaker be a **separate adversarial layer** (like TradingAgents' aggressive/conservative/neutral trio) or integrated into the bull/bear debate?
- What's the best way to handle **HOLD decisions** — should the debate produce a "no trade" consensus, or should there be a separate "skip" vote?

### 5. Prompt Engineering for Bull and Bull Agents

- What system prompts produce the **strongest counterarguments**? Should the bear agent be told to "find every reason this trade will fail" or "objectively assess downside risk"?
- How do you prevent the debate from becoming **performative** (agents arguing for the sake of arguing) vs **genuine** (actually changing the final decision)?
- Should debate agents have access to **past debate outcomes and reflections** (e.g., "last 3 times the bear was right about dead cat bounces, the trade lost 2%")?
- What's the right **token budget** for debate? With a $30 micro-account, each LLM call costs money. How do you balance debate depth vs cost?

### 6. Measuring Debate Effectiveness

- What metrics should I track to determine if the debate is actually **improving** trading performance?
- How do you **A/B test** single-shot vs debate-driven decisions in a live trading system?
- Should I run the debate in **shadow mode** first (debate runs but doesn't influence decisions) to collect data?
- What's a reasonable **minimum sample size** to determine if debate adds value for a $30 crypto scalping account?

### 7. Creative Variations on the Debate Pattern

- Instead of bull vs bear, what about **"fast vs slow" thinking** (System 1 vs System 2)? One agent makes a quick gut decision, the other does deep analysis.
- What about a **"pre-mortem" approach** — before each trade, an agent imagines the trade has already failed and writes the post-mortem explaining why?
- Could the debate be **asymmetric by regime** — in trending markets, the bull gets fewer rounds (since trend is already bullish), but in ranging/volatile markets, the bear gets more rounds?
- What about a **"devil's advocate" that only activates on high-confidence signals** — the debate only fires when the initial confidence is above 70%, targeting overconfidence specifically?
- Could the debate use **structured scoring** rather than free-text arguments? (e.g., bull scores: momentum 8/10, sentiment 6/10, volume 7/10; bear scores: resistance proximity 9/10, funding rate divergence 7/10)
- What about a **"jury" model** — instead of 1v1 debate, have 3 independent agents vote and require consensus?

### 8. Latency and Cost Optimization

- What's the **cheapest debate architecture** that still improves decision quality? Can the bear agent be a smaller/cheaper model?
- Can the debate be **cached or memoized** when market conditions haven't changed significantly between candles?
- Should debate only run on **potential entries** (skip debate for HOLD/management decisions)?
- What's the **acceptable latency** for debate in a 5-minute candle system? How many LLM calls can fit before the next candle?

---

Please search for academic papers, open-source implementations, blog posts from quantitative trading firms, and any real-world case studies on adversarial or multi-agent approaches to trading decisions. I'm especially interested in anything that combines LLM debate with quantitative risk management.
