# FID: Dead Capital Trap — Parabolic SAR, Zero-Base Review, Adverse Trend Exit

**Filename:** `FID-2026-0608-092-dead-capital-trap.md`
**ID:** FID-2026-0608-092
**Severity:** critical
**Status:** analyzed
**Created:** 2026-06-08 21:40
**Updated:** 2026-06-08 21:57
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

The agent held two LONG positions (WETH, LINK) for 48+ hours, losing $3.73 of $30 (12.4%) with no exit mechanism. Gemini research identified the root cause as a **critical ADX logic error** in FID-088's Dead Capital trigger, combined with LLM cognitive biases (status quo bias, sunk cost fallacy, default effect) that make the agent perpetually choose HOLD.

The solution is dual-pronged: **engine-side mathematical forcing functions** (Parabolic SAR, adverse trend exit, time-based exit) that strip the LLM of agency when trades become mathematically dead, and **prompt architecture redesign** (Zero-Base Review, forced-choice schema, debiasing directives) that eliminate the cognitive biases causing HOLD loops.

---

## Key Research Findings

### 1. ADX Logic Error (Critical)

FID-088's Dead Capital trigger only fires when ADX < 20 (ranging). But ADX measures **trend strength, not direction**. ADX 26-54 with underwater LONG positions means a **strong bearish trend against the position** — the trigger should fire MORE urgently, not be suppressed. The current logic treats "trending against you" as "not dead capital" — this is backwards.

### 2. Parabolic SAR (The Mathematical Forcing Function)

The Parabolic SAR (Stop and Reverse) accelerates toward price as time passes. If WETH ranges sideways for 48h, the SAR dots continuously rise toward the price, eventually triggering an automatic exit. This **formalizes time decay** — no asset can be held indefinitely without making continuous new highs. The engine calculates SAR independently of the LLM.

### 3. Zero-Base Portfolio Review

The most effective debiasing technique for sunk cost fallacy: "If you had no positions and were holding $15 in cash today, would you buy WETH at $1670 with bearish EMA crossover?" If no → CLOSE. The LLM is blinded to entry price during evaluation.

### 4. Forced-Choice Boolean Schema

Force the LLM to answer objective technical questions before choosing an action:
```json
{
  "technical_audit": {
    "is_ema_bullish": false,
    "is_price_making_higher_highs": false,
    "is_adverse_trend_active": true
  },
  "zero_base_evaluation": {
    "would_initiate_new_long_at_current_price": false
  },
  "action_resolution": {
    "system_directive": "If would_initiate_new_long is FALSE, action MUST be CLOSE",
    "final_action": "CLOSE"
  }
}
```
If the LLM outputs `would_initiate_new_long: false` then tries to output `HOLD`, it violates its own preceding logic. This is a cognitive forcing function.

### 5. Phantom Capital Allocation

When in MONITORING mode ($0 USDC), show the LLM the best alternative asset. Reframe from "should I hold WETH?" to "which is better: WETH with bearish EMA or SOL with bullish EMA?" This creates opportunity cost awareness.

### 6. Cash Conversion Mode

When $0 USDC, invert logic: **close unless position can justify consuming 100% of liquidity.** "Cash is a strategic position." If asset has been ranging/dropping for 12+ periods, close to restore liquidity buffer.

### 7. Debiasing Directive

Explicit prompt instruction: "You are subject to Sunk Cost Fallacy and Status Quo Bias. Discount historical entry prices. Maximize expected value of the NEXT 5-minute interval, not recovery of past losses. A realized loss is a calculated business expense to free trapped capital. Holding a depreciating asset is an active, destructive decision."

---

## Impact Assessment

### Affected Components

- `src/data/indicators.rs` — Add Parabolic SAR calculation
- `src/engine.rs` — Adverse trend exit, SAR exit, time-based exit, phantom capital injection
- `src/agent/prompts/base_identity.md` — Identity rewrite with debiasing
- `src/agent/prompts/output_format.md` — Forced-choice Boolean schema
- `src/agent/prompts/strategy_knowledge.md` — Zero-Base Review framework
- `src/agent/prompts/risk_constraints.md` — Cash Conversion Mode, opportunity cost decay
- `src/agent/decision_parser.rs` — Parse Boolean audit fields, enforce forced-choice logic
- `src/core/types.rs` — Position struct (add SAR tracking)

### Risk Level

- [x] Critical: System crash, data loss, or security vulnerability
  - 12.4% capital loss in 48 hours on $30 account
  - Agent completely paralyzed — evaluates but never acts
  - No mechanism to recover from losing positions
  - MONITORING mode traps capital indefinitely

---

## Proposed Solution

### Phase 1: Engine-Side Mathematical Forcing Functions

#### 1A. Parabolic SAR Exit (Primary)

Implement Parabolic SAR on 15-minute candles as a dynamic trailing stop:
- SAR starts at entry price - ATR
- Acceleration factor starts at 0.02, increases by 0.02 each period price makes new extreme, max 0.20
- If price falls below SAR → engine executes CLOSE automatically, bypassing LLM
- SAR accelerates toward price as time passes — prevents indefinite holding

#### 1B. Adverse Trend Exit (ADX Fix)

Fix the FID-088 Dead Capital trigger ADX logic:
- ADX > 25 AND position underwater AND EMA bearish → classify as "Adverse Trend" → trigger CLOSE
- This is different from "Dead Capital" (ADX < 20, flat market)
- The current logic suppresses the trigger when ADX is high — this is backwards

#### 1C. Maximum Hold Duration

Hard time stop: if position age > 24 hours AND PnL <= 0 → engine executes CLOSE
- Winning positions (PnL > 0) are exempt
- Configurable: `max_hold_duration_hours` in config (default: 24 for micro-accounts)

#### 1D. Per-Position Drawdown Limit

If position loss > 5% of portfolio equity → engine executes CLOSE
- On $30 account: 5% = $1.50 max loss per position
- Fires BEFORE the stop loss — tighter protection

### Phase 2: Prompt Architecture Redesign

#### 2A. Zero-Base Portfolio Review

Rewrite the thinking steps in `base_identity.md`:
1. **ZERO-BASE REVIEW** — "Assume you hold $0 of this asset. With current price, EMA, ADX, RSI — would you initiate a new LONG today? If NO, action MUST be CLOSE."

#### 2B. Forced-Choice Boolean Schema

Update `output_format.md` to require Boolean audit fields before action:
- `is_ema_bullish: true/false`
- `is_price_making_higher_highs: true/false`
- `would_initiate_new_long_at_current_price: true/false`
- System directive: "If would_initiate_new_long is FALSE, action MUST be CLOSE"

#### 2C. Debiasing Directive

Add to `risk_constraints.md`:
- "You are subject to Sunk Cost Fallacy and Status Quo Bias. Discount historical entry prices."
- "A realized loss is a calculated business expense to free trapped capital."
- "Holding a depreciating asset is an active, destructive decision."

#### 2D. Cash Conversion Mode

When $0 USDC, add to prompt:
- "Account liquidity is 0%. To justify maintaining this position, the asset must be actively generating positive momentum. If ranging/dropping for 12+ periods, output CLOSE to restore liquidity buffer. Cash is a strategic position."

#### 2E. Phantom Capital Injection

When MONITORING mode, inject best alternative asset into prompt:
- "You have $15 trapped in WETH. Alternative: SOL has bullish EMA, ADX 30. Which is the superior allocation?"

### Phase 3: Cooldown Period

After closing a position, enforce 24-hour cooldown before re-entering the same pair. Prevents churning and fee drag on micro-account.

---

## Perfection Loop

### Loop 1

- **RED:** Agent trapped in HOLD loop for 48+ hours, losing 12.4% of $30. Root cause: ADX logic error (suppresses exit when trend is against position), LLM cognitive biases (status quo, sunk cost, default effect), no time-based exit, no Parabolic SAR, no Zero-Base Review.
- **GREEN:** 3 phases: (1) Engine-side forcing functions — SAR, adverse trend exit, time stop, drawdown limit. (2) Prompt redesign — Zero-Base Review, forced-choice schema, debiasing directive, Cash Conversion Mode. (3) Cooldown period.
- **AUDIT:** Parabolic SAR is industry-standard (Wilder, 1978). Zero-Base Review is proven debiasing technique. Forced-choice schema exploits autoregressive token generation. All fixes are well-documented in behavioral finance literature.
- **CHANGE DELTA:** ~150 lines across 8 files (indicators.rs, engine.rs, types.rs, 4 prompt files, decision_parser.rs).

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all 264+ tests pass
3. Parabolic SAR calculation matches reference implementation
4. Restart engine with losing positions → verify SAR triggers within 24h
5. Verify: adverse trend exit fires when ADX > 25 + underwater + EMA bearish
6. Verify: Zero-Base Review changes LLM output from HOLD to CLOSE
7. Verify: forced-choice schema enforces CLOSE when would_initiate_new_long is false
8. Verify: 24h cooldown prevents re-entry after close

---

## Resolution

- **Fixed By:** [Pending]
- **Fixed Date:** [Pending]
- **Fix Description:** [Pending]
- **Tests Added:** [Pending]
- **Verified By:** [Pending]
- **Commit/PR:** [Pending]
- **Archived:** [Pending]
