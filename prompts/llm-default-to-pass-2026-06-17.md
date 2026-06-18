# Gemini Deep Research Prompt: LLM Trader Default-to-PASS Behavior in Flat Markets

**Created:** 2026-06-17 23:25 EST
**Author:** Vera (sponsored by Spencer)
**Purpose:** Targeted research to diagnose and fix the "LLM trader outputs action: PASS for every pair in a flat market" problem. The previous Gemini research (DEX Crypto Scalping Engine Optimization, 2026-06-17) covered strategy calibration broadly; this is a follow-up focused specifically on the LLM behavior, prompt design patterns, and the prompt/threshold/confidence floor stack.

---

## Instructions for Spencer

1. Copy the entire prompt below (everything between the `---` lines marked "PROMPT START" and "PROMPT END")
2. Paste into Gemini Deep Research
3. Save the full response to `C:\Users\spenc\dev\savant-trading\prompts\prompt-results\llm-default-to-pass-2026-06-17.md`
4. Tell me the path. I'll read it before continuing FID-184 (probe positions).

While you run the research, I'll implement the FID-184 / FID-198 prompt and engine changes. We'll meet back when both are done.

---

## PROMPT START

# Deep Research: Why LLM Trading Agents Default to PASS in Flat Markets, and How to Fix It

## Context

I am building an autonomous crypto trading engine that uses a large language model (M3, a free model via TokenRouter, 1M context) to make trade decisions. The engine runs 5-minute cycles, scans 30-50 trading pairs, and asks the LLM to output a JSON decision per pair. The LLM has access to: candle data (OHLCV), technical indicators (EMA, RSI, ATR, ADX, VWAP), regime classification (Trending/Volatile/Ranging/GreyZone), and its own prior decision log.

**The problem I'm trying to solve:** In a 16-hour overnight test + a 90-cycle paper-mode run, the LLM produced 0 trades despite receiving real market data. All decisions were `action: "PASS"`. Yet:
- The conviction gate is set very low (Trending: 0.05, Ranging: 0.10, Volatile: 0.15, GreyZone: 0.20)
- Several pairs had `conviction_score` values of 0.10-0.22, which would PASS the gate
- The LLM was actively reasoning about the pairs (e.g., "Ranging ADX 13.7, EMA near-flat, RSI 58.0, ATR compressed, no momentum trigger, PASS")
- Pre-screening reduced 48 pairs to 15 candidates that had at least one indicator signal
- The market on Anvil (Arbitrum fork) with 5-min Kraken candles for micro-cap pairs is genuinely flat
- The LLM has a prompt that explicitly says "DO NOT default to action: PASS" (FID-192)

So the LLM is being **asked** to commit to a directional position when there's any signal, but it's **choosing** PASS anyway.

## What I Need Researched

### Question 1: Is this a known LLM failure mode?

Is "defaulting to PASS/HOLD" a documented anti-pattern in LLM-based trading systems? What does the literature say about this behavior? Are there papers, blog posts, or practitioner accounts that describe the same problem?

Specifically:
- Is this related to RLHF training bias toward "cautious" outputs?
- Is it a token probability issue (PASS is over-represented in training data for trading contexts)?
- Is it prompt-structural (does the LLM interpret "default" instructions differently than expected)?

### Question 2: What's the right calibration for the prompt/threshold/confidence stack?

My current setup:
- Conviction gate (engine): Trending 0.05, Ranging 0.10, Volatile 0.15, GreyZone 0.20
- Conviction gate (prompt says): Trending 0.30, Volatile 0.40, Ranging 0.40, GreyZone 0.40 (in output_format.md)
- Conviction gate (prompt says): Trending 0.20, Volatile 0.25, Ranging 0.25, GreyZone 0.25 (in strategy_knowledge.md table)
- Conviction gate (prompt says): Trending 0.05, Ranging 0.10, Volatile 0.15, GreyZone 0.20 (in strategy_knowledge.md Out-of-Range section)
- Confidence floor: not set (default 0.0 from serde)
- Probe mechanism: not implemented

The LLM sees 4 different threshold sets across 2 files. Is this a major cause of the PASS default? What's the right way to structure the prompt/threshold/confidence floor for a sniper/scalp degen trading agent?

For a sniper/scalp degen strategy (not institutional, not swing trading):
- What's the appropriate conviction gate range?
- Should the gate be regime-dependent at all, or should it be a single global threshold?
- Should confidence floor be set, and at what level?
- Where should the threshold live (in the engine, in the prompt, or both with sync)?

### Question 3: Probe position mechanisms

I'm planning to add a "probe position" mechanism: when conviction is low (0.05-0.15) and there's at least one technical indicator agreement, allow a 0.5x sizing probe with auto-TP at 0.6% and auto-timeout at 10 minutes. The goal is to generate trade flow data when the LLM is too cautious.

Is this a valid pattern? What are the standard probe mechanisms in quantitative trading?
- What sizing fraction is standard for probe positions?
- What's a reasonable probe auto-TP target for crypto scalping?
- What's a reasonable probe timeout?
- How do you prevent probes from becoming excessive (e.g., 100 probes per day, 99% of capital in probes)?
- Should probes bypass the conviction gate entirely, or just have a different threshold?

### Question 4: Prompt design patterns for forcing LLM commitment

What's the most effective way to instruct an LLM to commit to a position when the data shows any directional lean? Specifically:
- Should the prompt use positive examples ("when you see X, you MUST output Buy with conviction Y") or negative examples ("DO NOT output PASS when...")?
- Should there be a separate "probe" field in the JSON schema that the LLM explicitly fills?
- Should the prompt include a checklist of conditions that force a Buy (e.g., "If ADX > 25 AND volume > 1.5x AND EMA bullish, output Buy with conviction 0.15+")?
- What's the role of the "regime_disambiguating_trigger" mentioned in the prompt? Does it actually help or does it add complexity without value?

### Question 5: Timeframe and universe

I'm currently using:
- 5-minute candle data
- ~48 pairs across Arbitrum DEX
- 5-minute cycle interval

The LLM reasoning shows "Ranging ADX 13.7, EMA near-flat, RSI mid-range, ATR compressed" — this is consistent across many pairs, suggesting the 5-min timeframe on these micro-cap pairs is genuinely flat.

For a sniper/scalp degen on Anvil (Arbitrum fork), is 5-min the right timeframe? Should I:
- Use 1-min candles for more signal?
- Use 15-min or 1-hour for fewer but higher-quality signals?
- Mix timeframes (use 1-min for entries, 1-hour for trend)?
- Stick with 5-min but increase universe to 100-500 pairs (more chances of finding a setup)?

### Question 6: Verifying the strategy is real vs the LLM being correctly cautious

How do I distinguish between:
- A: The strategy is right, the LLM is too cautious (needs prompt/threshold fix)
- B: The strategy is wrong for this market (micro-cap 5-min on Anvil is flat, no strategy would work)
- C: The market is dead and the LLM is correctly saying "no trade" (strategy is sound, just wait)

For each, what are the diagnostic tests I should run?

## What I Need in the Response

For each question:
1. **Direct answer** — not hedged
2. **Specific numbers** — thresholds, sample sizes, timing
3. **Source citations** — academic papers, practitioner blogs, exchange documentation, GitHub projects
4. **Contradicting evidence** — what would make this advice wrong
5. **Actionable recommendations** — what to change in my code/prompt

## Specific Numbers I Need

- Conviction gate range for scalper/sniper: 0.05, 0.10, 0.15, 0.20, 0.25?
- Probe position sizing: 0.1x, 0.25x, 0.5x of base?
- Probe auto-TP target: 0.3%, 0.5%, 0.8%, 1.2%?
- Probe timeout: 5 min, 10 min, 30 min, 1 hour?
- Probe frequency cap: 1/cycle, 3/cycle, 10/session?
- Confidence floor: 0.0, 0.1, 0.2, 0.3?
- Timeframe for scalper: 1-min, 5-min, 15-min, mixed?
- Universe size for sniper: 30, 50, 100, 200 pairs?

## Constraints

- LLM is M3 (free model via TokenRouter), 1M context
- Running on Anvil (Arbitrum fork) with $50 paper capital
- 5-min cycles
- Real-time on-chain execution via 0x API
- No latency advantage (LLM takes 15-30s per cycle)
- Cannot change the LLM (M3 is fixed by API contract)

## Output Format

Respond with a structured report with one section per question. Each section should be 200-400 words with specific recommendations. End with a "TL;DR Priority Order" section that lists 5-10 concrete changes I should make in priority order, with estimated time to implement each.

---

## PROMPT END

**After Gemini responds, save the full response to:**
`C:\Users\spenc\dev\savant-trading\prompts\prompt-results\llm-default-to-pass-2026-06-17.md`

Tell Vera the path. She'll read it before continuing FID-184 (probe positions) and FID-198 (prompt calibration).

---

*Vera 0.1.0 — 2026-06-17 23:25 EST — Gemini research prompt for LLM-default-to-PASS problem. Run in parallel with FID-184 implementation.*
