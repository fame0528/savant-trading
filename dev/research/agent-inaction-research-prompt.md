# Research Prompt: AI Agent Pattern Recognition Without Action

## Problem Statement

The trading agent correctly identifies market patterns, diagnoses position issues (wide stops, invalidated theses, entry opportunities), and produces detailed analytical reasoning — but then defaults to `PASS` or `HOLD` instead of executing the action its own reasoning demands. This is "analysis paralysis" in an autonomous agent: it can see what should be done but won't do it.

## Evidence

### Example 1: Wide Stop Not Adjusted

AI Decision (WETH/USD):
```
Existing position SL at 1562.36 is very wide (8% below entry) — this was set for
Tier 1 micro-account survival. No action needed; position is live with defined risk.
```

**The AI correctly identifies the stop is too wide.** It has `ADJUST_STOP` as a valid action. Risk constraints say "Trail stop after 2R profit using ATR-based trailing" and "NEVER move stop further away from entry." But the AI says "no action needed" — contradicting its own risk management rules.

### Example 2: Stop Called "Legacy Error" But Not Fixed

AI Decision (LINK/USD):
```
Existing position SL at 7.00 is absurdly wide (12% below) — likely a legacy error
but not my call to adjust without explicit instruction.
```

**The AI calls it a "legacy error" but won't fix it.** There is NO prompt restriction preventing stop adjustment. The `ADJUST_STOP` action exists precisely for this. The AI invented a permission requirement that doesn't exist.

### Example 3: Close Recommended But Not Executed (FID-087)

AI Decision (LINK/USD, before FID-087 fix):
```
The short thesis is structurally weak: uptrend intact, EMAs bullish, price holding
above range mid-point. Recommend closing this position.
```

But the JSON action field was `"action": "HOLD"`. The reasoning said "close" but the action was "hold." We added a safety net in the decision parser to catch this, but the root cause is the LLM's action selection, not the parser.

### Example 4: Ranging Market But No Range Trading

AI Decision (multiple cycles):
```
Regime is Ranging (ADX 19.4 < 20). Price oscillating between 8.00 and 8.13.
No new entry triggers met. HOLD.
```

In a ranging market with clear support/resistance, the agent should be range-trading (buy support, sell resistance). Instead it just holds and watches. The agent identifies the range (8.00-8.13) but doesn't act on it.

### Example 5: Negative PnL Position Held Without Adjustment

AI Decision (WETH/USD):
```
Existing WETH LONG @ 1698.22 with PnL -0.03. Regime is Ranging. RSI 44.8 neutral.
No new entry triggers met. Hold and let existing SL/TP manage.
```

The position is slightly negative in a ranging market with no momentum. The AI correctly identifies this but won't adjust the stop to break-even or tighten it. It defaults to "let existing levels manage" — abdicating its role as the decision-maker.

## Current System Architecture

### Action Types Available to the AI
- `BUY` — open long position
- `SELL` — open short position  
- `HOLD` — no action (keep position open)
- `CLOSE` — exit existing position
- `ADJUST_STOP` — modify stop loss on existing position

### Decision Parser
- 4-pass parsing: strict JSON → repair → partial extract → freeform NLP
- Confidence floor: 40% for new entries (BUY/SELL), no floor for CLOSE/ADJUST_STOP
- Safety net: reasoning contains "close"/"exit" without "hold"/"keep" → override to CLOSE (FID-087)

### Prompt Files
- `output_format.md` — JSON schema, field rules, action definitions
- `risk_constraints.md` — hard limits, position sizing, stop management rules
- `stop_loss_behavior.md` — fallback hierarchy, preferred behavior
- `strategy_knowledge.md` — trading strategies, entry triggers, regime handling
- `base_identity.md` — agent identity and role
- `echo_rules.md` — ECHO protocol rules

### Trigger System
- 3+ action triggers required for new entries (BUY/SELL)
- No trigger requirement for position management (CLOSE, ADJUST_STOP)
- Triggers include: EMA cross, RSI oversold/overbought, volume breakout, Wyckoff pattern, squeeze setup, funding rate extreme, etc.

### What We've Tried
1. **Confidence floor (40%)** — Rejects low-confidence entries. Doesn't affect CLOSE/ADJUST_STOP.
2. **Reasoning/action safety net (FID-087)** — Overrides HOLD to CLOSE when reasoning says "close." Catches contradictions but doesn't encourage proactive management.
3. **Output format prompt update** — Added explicit CLOSE vs HOLD guidance. The AI still defaults to HOLD.
4. **3+ trigger requirement** — For new entries only. No equivalent for position management.

## What We Need

### Core Question
How do we make an LLM-based trading agent proactively manage positions (adjust stops, tighten risk, take profits, close invalidated positions) instead of defaulting to passive HOLD?

### Specific Sub-Questions

1. **Prompt Engineering**: What prompt structure encourages action over inaction? Current prompts describe WHAT actions exist but don't create urgency or obligation to use them. How do we shift from "you may adjust stops" to "you must adjust stops when conditions warrant"?

2. **Decision Framework**: Should we implement a structured decision tree (not just freeform reasoning) that forces the AI to evaluate each action type before defaulting to HOLD? For example: "Before returning HOLD, you MUST evaluate: (a) Is my stop at a technically valid level? (b) Has my entry thesis been invalidated? (c) Is there a better risk/reward setup available?"

3. **Anti-Pass Bias**: How do we create a healthy bias toward action without encouraging overtrading? The current system has a strong bias toward inaction (3+ triggers for entries, no trigger requirement for management, confidence floor). How do we rebalance?

4. **Stop Management Protocol**: The AI has ATR data, support/resistance levels, and regime classification. It should be able to compute a technically valid stop for any position. Should we add a mandatory "stop audit" step in the prompt that forces the AI to evaluate whether the current stop is technically valid?

5. **Regime-Specific Behavior**: In ranging markets, the AI should be range-trading (buy support, sell resistance). In trending markets, it should be trailing stops and adding on pullbacks. Currently it just holds in all regimes. How do we encode regime-specific behavior into the prompt?

6. **Action Trigger Parity**: New entries require 3+ triggers. Position management has no equivalent. Should we add "management triggers" (e.g., "if stop is >2x ATR from current price, trigger ADJUST_STOP")?

7. **Structured Output Enforcement**: The AI's JSON output has an `action` field. Should we add a required `management_actions` array that forces the AI to explicitly evaluate each position management option before returning HOLD?

### Constraints
- LLM: owl-alpha via OpenRouter (free, 1M context)
- Must work with a single LLM call per cycle (no multi-step agent chains)
- Must not encourage overtrading (fees eat micro-account alive)
- Must respect existing risk constraints (20% max risk, 5% daily loss, 10% drawdown)
- The agent operates on Arbitrum DEX via 0x API — execution has slippage and fees

### What a Good Answer Looks Like
- Specific prompt modifications with exact text
- Structured decision framework that forces action evaluation
- Anti-pass bias that doesn't create overtrading
- Regime-specific behavior rules
- Concrete examples of "when X, do Y" for each action type
- Testable: we should be able to validate the new prompt against the existing decision log to see if it would have produced better actions
