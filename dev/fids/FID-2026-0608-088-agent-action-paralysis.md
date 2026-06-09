# FID: Agent Action Paralysis — Cognitive Forcing Functions and Management Triggers

**Filename:** `FID-2026-0608-088-agent-action-paralysis.md`
**ID:** FID-2026-0608-088
**Severity:** critical
**Status:** analyzed
**Created:** 2026-06-08 20:22
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

The trading agent correctly identifies market patterns, diagnoses position issues (wide stops, invalidated theses, dead capital, ranging regimes), and produces detailed analytical reasoning — but then defaults to PASS/HOLD instead of executing the action its own reasoning demands. This is "analysis paralysis" driven by LLM status quo bias: the agent can see what should be done but won't do it.

Root cause: asymmetric action thresholds (entries require 3+ triggers, management requires none) combined with prompt architecture that penalizes action but ignores opportunity cost of inaction. The LLM interprets the current state as a secure baseline and invents permission constraints that don't exist.

The solution requires 5 architectural changes: (1) a mandatory position audit in the JSON schema that forces mathematical evaluation before action selection, (2) management triggers that mandate action when thresholds are breached, (3) regime-specific behavior matrices, (4) opportunity cost engineering that makes HOLD expensive, and (5) an identity rewrite that establishes absolute authority for position management.

---

## Environment

- **OS:** Windows (win32)
- **Language/Runtime:** Rust 2021, tokio async runtime
- **Tool Versions:** savant-trading v0.11.6
- **LLM:** owl-alpha via OpenRouter (free, 1M context)
- **Chain:** Arbitrum (chain_id=42161)

---

## Detailed Description

### Problem

The agent exhibits 5 distinct failure modes, all rooted in the same architectural gap:

| # | Failure Mode | Example | Root Cause |
|---|-------------|---------|------------|
| 1 | **Wide stop rationalization** | "SL is 8% below entry — set for micro-account survival. No action needed." | Semantic reframing of risk; no imperative to optimize capital efficiency |
| 2 | **Legacy error hallucination** | "SL is 12% below — likely a legacy error but not my call to adjust without explicit instruction." | Invented permission constraint; status quo bias |
| 3 | **Reasoning/action contradiction** | Reasoning says "Recommend closing" but JSON action is "HOLD" | Autoregressive disconnect between reasoning tokens and action token |
| 4 | **Regime recognition without exploitation** | "ADX 19.4, ranging 8.00-8.13. No momentum triggers met. HOLD." | No regime-specific behavior matrix; anchoring to trend triggers |
| 5 | **Dead capital tolerance** | "PnL -0.03, ADX <20, neutral RSI. Let existing SL/TP manage." | No opportunity cost framework; premature disengagement |

### Expected Behavior

1. When stop distance >2.5x ATR, agent MUST execute ADJUST_STOP
2. When regime changes (trending→ranging), agent MUST adjust strategy
3. When structural thesis is invalidated, agent MUST close
4. When position is dead capital (flat/negative PnL in ranging market for N cycles), agent MUST evaluate closing
5. When profit ≥1R, agent MUST trail stop to break-even
6. When in ranging regime, agent MUST use support/resistance as entry triggers (not momentum)

### Root Cause: Status Quo Bias + Asymmetric Thresholds

**Mathematical model:** The LLM evaluates `U(action) > U(inaction) + C_switch`. Currently:
- `U(inaction)` is artificially elevated (no penalty for holding dead capital)
- `C_switch` is massive (risk constraints stress survival without balancing for capital efficiency)
- `U(action)` rarely exceeds the threshold

**Trigger asymmetry:** Entries require 3+ triggers (high friction, deterministic). Management requires NO triggers (low friction, subjective). The LLM defaults to the path of least resistance: HOLD.

---

## Impact Assessment

### Affected Components

- `src/agent/prompts/base_identity.md` — agent identity and authority
- `src/agent/prompts/stop_loss_behavior.md` — stop management protocol
- `src/agent/prompts/risk_constraints.md` — risk rules and management triggers
- `src/agent/prompts/strategy_knowledge.md` — regime-specific behavior
- `src/agent/prompts/output_format.md` — JSON schema and action guidance
- `src/agent/decision_parser.rs` — TradeDecision struct, parsing, validation
- `src/engine.rs` — decision processing, management trigger evaluation

### Risk Level

- [x] High: Feature broken, workaround exists
  - Agent makes correct diagnoses but won't act on them
  - Capital efficiency degraded (wide stops, dead capital, missed range trades)
  - Not a crash risk — agent holds positions safely — but an opportunity cost risk

---

## Proposed Solution

### Approach

5 architectural changes, ordered by impact. Phase 1 (prompt-only) can be deployed immediately. Phase 2 (schema + parser) requires code changes. Phase 3 (engine integration) completes the loop.

### Phase 1: Prompt Architecture (No Code Changes)

#### 1A. Rewrite `base_identity.md`

Shift from passive observer to active executioner. Establish absolute authority for position management.

**Key changes:**
- "You are a ruthless, highly active autonomous trading executioner"
- "You do not require external permission to fix legacy errors, tighten risk parameters, or exit stagnant positions"
- "Inaction carries a severe opportunity cost that you must continuously optimize against"
- Remove "If no high-quality setup exists, output a HOLD decision" (replaces with management-first evaluation)
- Add: "Before evaluating new setups, you MUST first audit all existing positions for management triggers"

#### 1B. Upgrade `stop_loss_behavior.md` — Mandatory Stop Audit Protocol

**Key changes:**
- **Absurdity Check:** If SL distance >2.5x current 14-period ATR, classify as "Structurally Invalid" → MUST execute ADJUST_STOP
- **No Legacy Deference:** If you identify a stop as "legacy error" or "absurdly wide," you have absolute authorization to fix it immediately. Returning HOLD on an invalid stop is a catastrophic failure.
- **Trailing Ratchet:** If position achieves 1R profit, MUST execute ADJUST_STOP to break-even plus fees
- **Quantized Adjustments:** New stop must improve risk profile by ≥0.5R to justify execution cost. Do not execute ADJUST_STOP for micro-movements.

#### 1C. Modify `risk_constraints.md` — Management Triggers + Opportunity Cost

**Key changes:**
- Add 5 Management Triggers (see table below)
- Add "Cost of Holding" section: "HOLD is an active declaration of risk assumption. Dead capital must be aggressively purged."
- Add trigger parity: "HOLD requires the absolute absence of any Management Trigger. If a single trigger evaluates as true, HOLD is prohibited."
- Add: "Management actions (ADJUST_STOP, CLOSE) are NOT gated by the 40% confidence floor. The confidence floor applies only to new entries (BUY/SELL)."

**Management Triggers:**

| Trigger | Condition | Mandated Action |
|---------|-----------|-----------------|
| Stop Distance Violation | SL distance >2.5x ATR | ADJUST_STOP to swing low/high or 1.5x ATR |
| Regime Incompatibility | Position opened in regime X, current regime is Y and ADX has crossed the 20/25 threshold | CLOSE or ADJUST_STOP to match current regime |
| Structural Invalidation | Price crosses MA support / structural low that formed thesis | CLOSE |
| Dead Capital Tolerance | Flat/negative PnL after 3+ cycles (15+ minutes) in ADX <20 with neutral RSI (30-70) | CLOSE |
| Profit Protection Ratchet | PnL ≥1R | ADJUST_STOP to lock break-even + fees |

**Note on Regime Incompatibility:** This trigger fires in BOTH directions:
- Position opened in trending (ADX >25), now ranging (ADX <20) → CLOSE or tighten aggressively
- Position opened in ranging (ADX <20), now trending (ADX >25) → switch to trend-following (trail stops, let winners run)

#### 1D. Enhance `strategy_knowledge.md` — Regime Translation Matrix

**Key changes:**
- **Trending (ADX >25):** Require 3+ momentum triggers for entries. Trail stops using Trailing Ratchet. HOLD permitted only if stop recently optimized.
- **Ranging (ADX <20):** Momentum triggers SUSPENDED. Support/resistance boundaries ARE triggers. MUST execute BUY at support, SELL at resistance. Profit targets at range mid-point or opposite boundary. HOLD only permitted if price is in middle 50% of range moving toward target.
- **Transition (ADX crossing 20):** When ADX crosses below 20, existing momentum positions must be re-evaluated. When ADX crosses above 20, range positions must be re-evaluated.

#### 1E. Overhaul `output_format.md` — Pre-Action Verification Schema

**Key changes:**
- Add `position_audit` array (required for existing positions)
- Add `management_trigger_active` boolean (must be evaluated before action)
- Add `opportunity_cost_of_holding` string (must articulate cost of inaction)
- Place audit fields BEFORE action field in JSON schema (exploits autoregressive token generation)
- Add: "If evaluating multiple pairs in batch, audit ALL open positions first, then evaluate new setups"

**New JSON schema structure:**
```json
{
    "position_audit": [
        {
            "pair": "WETH/USD",
            "current_stop_distance_atr": 3.2,
            "is_stop_valid": false,
            "thesis_status": "intact | weakened | invalidated",
            "management_trigger": "none | stop_violation | regime_change | structural_invalidation | dead_capital | profit_ratchet",
            "mandated_action": "HOLD | ADJUST_STOP | CLOSE",
            "mandated_stop_price": 0.0,
            "opportunity_cost": "What is lost by holding this position"
        }
    ],
    "action": "BUY | SELL | CLOSE | ADJUST_STOP | HOLD",
    "pair": "BTC/USD",
    ...existing fields...
}
```

### Phase 2: Schema + Parser Changes (Code Changes)

#### 2A. Update `TradeDecision` struct

Add optional audit fields with `#[serde(default)]`:
```rust
pub struct TradeDecision {
    // Existing fields...
    pub action: TradeAction,
    pub pair: String,
    // ...
    
    // FID-088: Cognitive Forcing Function fields
    #[serde(default)]
    pub management_trigger_active: bool,
    #[serde(default)]
    pub stop_distance_atr_multiple: f64,
    #[serde(default)]
    pub thesis_invalidated: bool,
    #[serde(default)]
    pub opportunity_cost: String,
    #[serde(default)]
    pub mandated_action: String,
    #[serde(default)]
    pub mandated_stop_price: f64,
}
```

#### 2B. Update decision parser validation

After parsing, enforce management triggers:
- If `management_trigger_active == true` AND action is HOLD/PASS:
  - Log warning: "Management trigger active but action is HOLD — overriding to {mandated_action}"
  - Parse `mandated_action` string to TradeAction enum
  - Override action to mandated action
  - If `mandated_stop_price > 0`, set as stop_loss
- If `thesis_invalidated == true` AND action is HOLD:
  - Log warning: "Thesis invalidated but action is HOLD — overriding to CLOSE"
  - Override action to CLOSE
- This is the structural enforcement that prevents the LLM from ignoring its own audit

#### 2C. Update `normalize_llm_json()`

Handle new field normalization (management_trigger_active, thesis_invalidated as booleans, mandated_action as action string).

#### 2D. Fallback for weak models

If the LLM doesn't produce the new audit fields (owl-alpha may not handle complex schemas):
- The `#[serde(default)]` ensures backward compatibility
- The FID-087 safety net (reasoning/action contradiction override) still works
- The engine-side management trigger evaluation (Phase 3A) provides a second safety net
- Log: "No position_audit in response — falling back to engine-side trigger evaluation"

### Phase 3: Engine Integration

#### 3A. Management trigger evaluation in engine (Independent Safety Net)

After parsing LLM decisions, evaluate management triggers independently using actual market data:
- Calculate actual stop distance / ATR ratio from position data and indicators
- Check regime compatibility (compare position creation regime vs current regime)
- If trigger fires but LLM returned HOLD, log warning and override to mandated action
- This is the SECOND safety net (first is the prompt/schema, second is engine-side)
- Both must agree for HOLD to be permitted on positions with risk issues

#### 3B. ATR data injection into prompt — ALREADY DONE

ATR is already included in the market context via `context_builder.rs:286`: `ATR={:?}`. No changes needed.

#### 3C. Position audit in batch evaluation

When evaluating multiple pairs in a single batch call:
- The position_audit array covers ALL open positions (not just the pair being evaluated)
- The engine should inject all open position data into the prompt context
- The LLM evaluates management triggers for each position before considering new setups

---

## Perfection Loop

### Loop 1

- **RED:** 5 failure modes identified across prompt architecture, schema, and engine. Root cause: status quo bias + asymmetric action thresholds. The LLM has no management triggers, no opportunity cost framework, and no forced evaluation sequence. Additional gaps: ATR already in context (Phase 3B done), dead capital trigger timing too short, missing regime transition handling, missing overtrading enforcement, missing batch audit context, missing confidence floor interaction, missing owl-alpha fallback, missing FID-087 interaction.
- **GREEN:** Fixed 8 gaps: (1) ATR verified as already in context, (2) dead capital trigger changed to 3+ cycles/15+ min, (3) added bidirectional regime transition handling, (4) added quantized ratchet enforcement details, (5) added batch evaluation audit context, (6) clarified confidence floor doesn't gate management actions, (7) added fallback for weak models (serde defaults + engine-side triggers), (8) documented FID-087 interaction (complementary, FID-088 takes precedence when audit present).
- **AUDIT:** FID covers 5 prompt files, 1 Rust struct, 1 parser function, 1 engine integration point. Phased deployment: Phase 1 (prompts) can ship immediately, Phase 2 (schema) requires code changes, Phase 3 (engine) completes the loop. Backward compatible via serde defaults.
- **CHANGE DELTA:** ~100 lines added to FID (8 gap fixes + verification items + FID-087 interaction).

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all 264+ tests pass
3. Decision parser accepts both old and new JSON formats (backward compatible via `#[serde(default)]`)
4. Manual test: restart engine, observe AI decisions for 3 cycles
5. Verify: AI adjusts stops when stop distance >2.5x ATR
6. Verify: AI uses range-trading logic when ADX <20
7. Verify: AI evaluates opportunity cost before returning HOLD
8. Verify: No overtrading (management triggers gate action, not freeform judgment)
9. Verify: Management actions (ADJUST_STOP, CLOSE) are NOT blocked by 40% confidence floor
10. Verify: FID-087 safety net (reasoning/action contradiction) still works alongside new triggers
11. Verify: Engine-side trigger evaluation fires as safety net when LLM doesn't produce audit fields

### Interaction with FID-087 Safety Net

FID-087 added a post-parse safety net: if reasoning contains "close"/"exit" without "hold"/"keep", override HOLD to CLOSE. This is complementary to FID-088:

- **FID-087** catches reasoning/action contradictions (freeform text vs action token)
- **FID-088** forces structured evaluation BEFORE the action token (prevents the contradiction from occurring)
- Both can coexist: FID-088 is the primary fix (prevents the problem), FID-087 is the safety net (catches it if it slips through)
- If FID-088's position_audit is present, it takes precedence over FID-087's text-based override

---

## Resolution

- **Fixed By:** [Pending]
- **Fixed Date:** [Pending]
- **Fix Description:** [Pending]
- **Tests Added:** [Pending]
- **Verified By:** [Pending]
- **Commit/PR:** [Pending]
- **Archived:** [Pending]

---

## Lessons Learned

1. **LLMs reproduce human cognitive biases.** Status quo bias, analysis paralysis, and permission hallucination are predictable failures when prompts don't explicitly mandate action.

2. **Asymmetric thresholds create passive defaults.** High-friction entries + zero-friction management = agent that enters carefully but never manages. Trigger parity is essential.

3. **Freeform reasoning and action tokens are decoupled.** The LLM can reason "should close" but generate "HOLD" because the action token selection is statistically biased toward the status quo. Forcing structured evaluation BEFORE the action token bridges this gap.

4. **HOLD must be earned, not defaulted.** Without explicit conditions under which HOLD is permitted, the LLM will always choose it. Making HOLD require the absence of all management triggers inverts the default.

5. **Opportunity cost must be explicit.** LLMs (like humans) suffer from opportunity cost neglect. Forcing the model to articulate what's lost by holding depresses the utility of inaction.

6. **Regime-specific behavior matrices are essential.** A single set of rules for all market regimes leads to paralysis in non-trending markets. The agent needs distinct operational modes.

7. **The single-call constraint forces embedded multi-step reasoning.** Without multi-agent debate, the cognitive forcing functions must be embedded in the JSON schema itself, exploiting autoregressive token generation to simulate Analyst → Risk Manager → Executioner sequencing.
