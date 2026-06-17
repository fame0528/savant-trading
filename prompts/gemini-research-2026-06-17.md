# Gemini Deep Research Prompt: Crypto Scalping on DEX — Strategy Calibration for 100-500 Pairs

**Created:** 2026-06-17 15:50 EST
**Author:** Vera (sponsored by Spencer)
**Purpose:** Strategic research to inform FID-184 (zero-conviction plateau fix) and v0.15.0 multi-chain expansion

---

## Instructions for Spencer

1. Copy the entire prompt below (everything between the `---` lines marked "PROMPT START" and "PROMPT END")
2. Paste into Gemini Deep Research
3. Save the full response to `C:\Users\spenc\dev\savant-trading\prompts\prompt-results\gemini-research-2026-06-17.md`
4. I'll read the results before finalizing FID-184 and FID-185

---

## PROMPT START

# Deep Research: Crypto Scalping Strategy on DEX with Multi-Chain Expansion

## Context

I am building an autonomous crypto trading engine that runs 24/7 on decentralized exchanges (DEX) via the 0x API on Arbitrum (and soon Base, Optimism, BSC, Polygon, Hyperliquid). The engine:

- Scans 30-50 trading pairs per cycle (currently), planning to expand to 100-500 pairs
- Runs 5-minute cycles
- Uses a multi-model "jury" system (1 M3 control + 9 free-model jurors) for adversarial decision validation
- Has $50 USDC starting capital (Anvil fork for testing, real mainnet for live)
- Operates as a **sniper/scalper** — looking to turn small positions into larger ones (penny → nickel → quarter), NOT institutional swing trading

**Critical problem I need research on:** In a recent 16-hour overnight test run on Anvil fork with real Kraken WebSocket v2 data, the engine produced:
- 96 cycles
- 703 PASS decisions (model said "no trade")
- 0 BUY decisions
- 0 SELL decisions
- 0 executed trades
- 87% of decisions output conviction=0.000 (the LLM defaulted to zero per a "if ambiguous, output 0.0 and select PASS" instruction in the prompt)

The model is obedient to the prompt but the prompt may be over-calibrated for institutional-quality trades when I need scalping-frequency trades.

## What I Need You to Research

### Question 1: Scalping Conviction Thresholds

For a **high-frequency crypto scalper** on DEX with 5-minute cycles targeting 0.5-1.2% moves:

- What conviction/confidence threshold range is appropriate for entry signals? (I'm using 0.20-0.25 currently, but this may be too high for scalps)
- How should the threshold vary by market regime (Trending vs Ranging vs Volatile vs GreyZone)?
- What's the trade-off between threshold stringency and trade frequency?
- Are there academic or practitioner papers on this specifically for crypto DEX scalping (not CEX, not traditional markets)?

### Question 2: 100-500 Pairs Per Cycle

When scanning 100-500 pairs per 5-minute cycle:

- What is the expected signal-to-noise ratio per pair?
- Should the LLM batch-evaluate all pairs in one call, or should it be hierarchical (screen → deep-dive)?
- What's the typical LLM context window requirement for evaluating N pairs simultaneously? (I'm using a single batch call with 34 pairs, works fine with M3's 1M context)
- How do professional market makers handle 100-500 asset universes? (BlackRock Aladdin, Citadel, Jump Crypto, Wintermute, etc.)
- Is there a diminishing returns curve for universe size vs alpha capture?

### Question 3: Multi-Chain DEX Scalping

When expanding from 1 chain (Arbitrum) to 5+ chains (Arbitrum, Base, Optimism, BSC, Polygon, Hyperliquid):

- What new signal dimensions does multi-chain add? (Cross-chain arbitrage, chain-specific momentum, gas arbitrage)
- What are the latency and execution challenges?
- How do existing multi-chain bots (e.g., 1inch Fusion, matcha, CowSwap) handle this?
- Should the engine treat each chain as a separate "sub-strategy" or a unified portfolio?
- For Hyperliquid specifically (perpetuals DEX, orderbook-based, no CEX equivalent), how does the strategy differ from spot DEX scalping?

### Question 4: "If Ambiguous, Output 0.0" Anti-Pattern

The LLM is outputting conviction=0.000 for 87% of decisions. The prompt says: "If you cannot compute a conviction score, output 0.0 and select PASS."

- Is this a known anti-pattern in LLM-based trading systems?
- What's the correct calibration: should the model output 0.0 (binary no-trade) or a low-but-nonzero score (0.05-0.10) when uncertain?
- How do successful LLM trading systems (if any exist) handle uncertainty?
- Is there research on "default-to-hold bias" in LLM agents for trading?

### Question 5: Jury System (9-Model Adversarial Validation)

I have a jury system where 9 free models evaluate the same market data in parallel, and a judge synthesizes them. The jury runs ONCE per cycle (not per pair) treating the whole market as Ranging.

- Is this the right architecture? Should the jury run per-pair, per-regime-cluster, or per-cycle-batch?
- How do existing adversarial validation systems work in other domains (fact-checking, medical diagnosis)?
- What's the cost/benefit of 9 vs 3 vs 15 jurors?
- For 100-500 pairs, is per-pair jury evaluation computationally feasible? (Each juror call is ~2-5s, so 100 pairs × 9 jurors = 900-4500s per cycle = way too slow)

### Question 6: Token Discovery and Universe Construction

The current universe is constructed via:
- Static curated list of high-volume pairs
- Dynamic Blockscout token discovery (Arbitrum only currently)
- Filter: min_vol=$1.5M, min_price=$0.001

For 100-500 pairs across 5+ chains:

- What's the right universe construction strategy for a scalper?
- Should the universe be static (curated) or dynamic (re-discovered every N cycles)?
- How do professional quant funds construct tradeable universes?
- What's the relationship between universe size and false discovery rate (pairs that look good but aren't tradeable)?

### Question 7: Anvil Fork for Testing — Limitations

The engine runs on Anvil (local fork of Arbitrum mainnet) for paper-mode testing. The $50 USDC is prefunded, the chain state is real, but the LLM is making decisions on Kraken WebSocket v2 data (not on-chain data from Anvil).

- Is paper-mode testing on Anvil + CEX data representative of live performance?
- What are the failure modes that paper mode WON'T catch? (slippage, MEV, gas spikes, bridge delays, oracle lag)
- How long should paper-mode run before going live? (Industry standard?)
- What's the minimum statistical sample (number of trades, not cycles) to validate a scalping strategy?

## What I Need in the Response

For each question, I need:
1. **Direct answer** — not hedged "it depends"
2. **Specific numbers** — thresholds, sample sizes, latency budgets (not "use a reasonable threshold")
3. **Source citations** — academic papers, practitioner blogs, exchange documentation
4. **Contradicting evidence** — what would make this advice WRONG?
5. **Actionable recommendations** — what should I change in my code, prompt, or config?

## My Current Architecture (for context)

```
Cycle (5 min):
  1. Fetch candles from 6 sources (Kraken, OKX, KuCoin, Gate.io, CryptoCompare, CoinGecko)
  2. Compute indicators (EMA, RSI, ATR, ADX, Bollinger, VWAP)
  3. Build context for each pair (TSLN encoding, ~300 tokens/pair)
  4. Batch LLM call: all 30-50 pairs in one prompt
  5. Parse LLM JSON response into TradeDecision per pair
  6. Apply conviction gate (>= 0.20-0.25 by regime)
  7. Apply confidence floor (>= 0.0 currently)
  8. Jury overlay: 9 free models evaluate the batch, judge synthesizes (SHADOW MODE — verdict logged, not used)
  9. Execute trades via 0x API on Anvil (live_execution=true for paper, =false for full paper)
  10. Log equity snapshot to data/equity_history.json
```

## Specific Numbers I Need

- Conviction threshold for scalping: should it be 0.10, 0.15, 0.20, 0.25?
- Per-pair jury evaluation: should it exist, and if so, how to parallelize 100 pairs × 9 jurors in < 5 min?
- Universe size sweet spot for 5-min cycle DEX scalper: 50, 100, 200, 500?
- Multi-chain: per-chain sub-strategy or unified?
- Hyperliquid: spot-style or perps-style strategy?

## Constraints

- M3 (MiniMax M3 via TokenRouter) is the primary LLM, 1M context, free tier
- 0x API for execution, ~$0.025 gas per swap on Arbitrum
- Risk limits: max 3 positions, 20% per trade, 5% daily loss, 10% drawdown kill
- $50 starting capital, plan to scale to $500, $5000, $50000 as strategy proves out
- The soul of the engine (`src/agent/soul.md`) is a "ruthless, highly active autonomous crypto trading executioner" — speed and aggression are features, not bugs

## Output Format

Respond with a structured report with one section per question. Each section should be 200-400 words with specific recommendations. End with a "TL;DR Action Items" section that lists 5-10 concrete changes I should make in priority order.

---

## PROMPT END

**After Gemini responds, save the full response and let Vera know the path. Vera will:**
1. Read the response and verify citations
2. Update FID-184 (strategy) and FID-185 (jury + logs) with the research-backed recommendations
3. Possibly create FID-187 (multi-chain architecture) based on Question 3 findings
4. Re-run Perfection Loop until converge
5. Present final implementation plan for approval

---

*Vera 0.1.0 — 2026-06-17 15:50 EST — Gemini research prompt created. Awaiting Spencer's run + results.*
