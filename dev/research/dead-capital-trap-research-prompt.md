# Research Prompt: LLM Trading Agent — Dead Capital Trap & Position Exit Strategy

## Problem Statement

An LLM-driven autonomous crypto trading agent (35K lines Rust, $30 micro-account, DEX-only on Arbitrum) is trapped in a "HOLD loop." It holds two LONG positions (WETH at $1698, LINK at $8.02) for 48+ hours, losing $3.73 of $30 (12.4%). The agent evaluates every 5 minutes, says "hold and monitor," and never exits. The positions are underwater with EMA bearish crossover, but the agent keeps saying "thesis weakened but not invalidated."

## System Architecture

- **LLM:** Single model (owl-alpha via OpenRouter, free) called once per 5-minute cycle
- **Execution:** DEX swaps via 0x API on Arbitrum. Client-side stop losses only (no on-chain protection).
- **Positions:** Spot only (LONG). No leverage, no shorts on DEX.
- **Account:** $30 micro-account. $0 USDC (fully deployed). MONITORING mode — can only evaluate existing positions, not open new ones.
- **Risk:** Max 20% risk per trade. 5% daily loss limit. 10% drawdown limit. 3 max positions.
- **Prompt:** 6-layer prompt architecture (identity, strategy, risk constraints, stop behavior, output format, knowledge). 3+ action triggers required for new entries. No trigger requirement for management actions.

## Current Exit Mechanisms

1. **Stop loss:** Client-side, 8% below entry. Price hasn't reached it in 48 hours.
2. **Take profit:** 10% above entry. Price hasn't reached it.
3. **AI CLOSE action:** LLM must choose CLOSE instead of HOLD. Rarely happens due to status quo bias.
4. **FID-088 Management Triggers:** 5 triggers (stop distance, regime change, structural invalidation, dead capital, profit ratchet). Dead Capital trigger only fires in ranging market (ADX <20), but current ADX is 26-54 (trending).

## What We've Tried

1. **FID-088 Cognitive Forcing Functions:** Added position_audit JSON schema, management triggers, reasoning/action safety net. The agent now evaluates triggers but still chooses HOLD when triggers don't fire.
2. **FID-087 Reasoning Override:** If reasoning says "close" but action is HOLD, override to CLOSE. But the agent says "weakened" not "close" — so the override doesn't trigger.
3. **FID-089 Engine Trigger:** Engine-side trigger evaluates stop distance / ATR. Fires correctly for stop adjustments but doesn't trigger CLOSE for time-based or drawdown-based exits.

## The Core Question

How do you make an LLM trading agent exit losing positions in a timely manner on a micro-account, when:

1. The LLM has a strong status quo bias toward HOLD
2. Stop losses are wide (8%) to avoid premature stop-outs, but this means positions can sit underwater for days
3. The agent is in MONITORING mode ($0 USDC) — it can't open new positions, so it has no incentive to free up capital
4. The "dead capital" management trigger only fires in ranging markets, not trending-against-position
5. The agent interprets "weakened thesis" as insufficient reason to close (needs "invalidated")

## Constraints

- LLM: Free model via OpenRouter (single call per cycle, no multi-agent chains)
- Must work with $30 micro-account (every cent matters)
- DEX-only on Arbitrum (no CEX, no leverage)
- Client-side stop losses only (no on-chain protection)
- Must not overtrade (DEX fees 0.3-0.8% round-trip eat micro-account alive)

## What I Need

1. **Exit strategy framework** — specific rules for when to close a losing position on a micro-account. Time-based? Drawdown-based? Trend-based? What's the optimal combination?

2. **LLM prompt design** — how to phrase the exit rules in the prompt so the LLM actually executes CLOSE instead of HOLD. The current prompt says "CLOSE to exit existing" but the LLM never chooses it. What specific wording, examples, or structural changes would change this?

3. **Position timeout mechanism** — is there a mathematically sound approach to "max hold duration" that doesn't cut winners short? How do you distinguish "this needs more time" from "this is dead capital"?

4. **Micro-account specific rules** — at $30, every trade costs 0.3-0.8% in fees. A position that's flat for 24 hours has already lost to fees. What's the optimal max hold duration for a $30 account?

5. **MONITORING mode strategy** — when the agent has $0 USDC and can only evaluate existing positions, what should its behavior be? Should it be more aggressive about closing? Should it have a "cash conversion" mode that closes everything and waits?

6. **Anti-HOLD bias techniques** — beyond what we've tried (cognitive forcing functions, management triggers, reasoning override), what other techniques exist for making LLMs take action instead of defaulting to inaction?
