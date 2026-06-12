# FID-132: 10-Point Checklist → Evaluation Matrix

**Filename:** `FID-2026-0612-132-checklist-evaluation-matrix.md`
**ID:** FID-132
**Severity:** high
**Status:** open
**Phase:** 1 (Prompt only — no Rust change)
**Created:** 2026-06-12
**Source:** Gemini Deep Research Q4 (`AI Trading Engine Rule Optimization.md` §Checklist and Knowledge Unit Audit)

---

## Summary

Restructure the 10-point pre-trade checklist from a rigid Boolean gate into an evaluation matrix where partial completion is acceptable provided the model generates explicit reasoning for each omission. The current checklist forces 10/10 pass to trade; the new matrix allows 7/10 with justification.

## Background

The pre-trade checklist (regime, thesis, invalidation, stop, target, size, conviction, correlation, catalyst) currently functions as an exclusionary gate. The MM3 model treats the checklist as exclusionary because instruction-following tuning biases toward total compliance. Per Gemini research: "The prompt format must be restructured to explicitly transition the checklist from a rigid gate to an evaluation matrix."

## Current vs New Behavior

**Current:** Missing macroeconomic catalyst → mandatory Pass
**New:** Missing catalyst → reduce sizing by 30% + document in reasoning

**Current:** Negative BTC correlation → mandatory Pass
**New:** Negative BTC correlation → reduce conviction by 20% + assess if signal strong enough to override

## Conviction Modifier Calibration

The modifier values (-0.10, -0.20, -0.30) are **initial estimates** based on Gemini research. They need calibration from the KU audit (FID-131) and A/B test (FID-133).

| Criterion | Initial Modifier | Rationale | Calibration Source |
| :--- | :--- | :--- | :--- |
| Missing macro catalyst | -0.10 | Informational, not risk-critical | KU audit + A/B test |
| Negative BTC correlation | -0.20 | Sector risk, but overridable | KU audit + A/B test |
| Missing invalidation level | -0.30 | Increases tail risk | KU audit + A/B test |
| Tight liquidity (< $1M 24h vol) | -0.20 | Execution risk | KU audit + A/B test |
| Extended overbought (RSI > 80) | -0.15 | Mean-reversion risk | KU audit + A/B test |
| **Invalid thesis** | **VETO** | Trade has no edge | Hard rule, no calibration |
| **Missing stop loss** | **VETO** | Unbounded loss | Hard rule, no calibration |
| **Catastrophic correlation** (peers -10%+) | **VETO** | Cascading liquidation | Hard rule, no calibration |

**Calibration procedure:** After 30+ live trades, fit a logistic regression: `actual_outcome ~ sum(checklist_modifiers) + regime` to determine if modifier values are right-sized. Update table in `config/checklist_modifiers.toml`.

## Order of Operations (FID-126 + FID-132 Interaction)

Conviction is computed in this order:
1. **Trigger weights** (FID-126): `trigger_conviction = sum(trigger_weights) / 3.0`
2. **Regime threshold check** (FID-126): `passes_regime = trigger_conviction >= regime_threshold`
3. **Checklist modifier application** (FID-132): `final_conviction = trigger_conviction + sum(checklist_modifiers)`, then `passes_final = final_conviction >= regime_threshold`
4. **Hard veto check** (FID-132): If any VETO criterion triggers, override to PASS regardless of final_conviction

**Example:**
- Trending regime (threshold 0.50)
- Triggers: 1 strong + 1 moderate = 1.0 + 0.7 = 1.7 / 3.0 = 0.57 (passes regime)
- Checklist: missing macro (-0.10) + extended overbought (-0.15)
- Final: 0.57 - 0.25 = 0.32 (fails regime 0.50 → PASS)
- Hard veto: none → override not triggered
- Result: PASS with explicit reasoning

## Minimum Score Floor

"Floor" question: what if 5/10 criteria unmet and modifiers sum to -0.40?
- 5/10 is below 7/10 (the original "allowed partial" threshold)
- Modified conviction: 0.57 - 0.40 = 0.17 (far below 0.50 → PASS)
- The modifier system naturally handles this — no separate floor needed
- **Document this in the prompt:** "Modifiers compound. Multiple missing criteria reduce conviction by their sum. There is no fixed 'X/10 must pass' floor; the regime threshold is the only gate."

## Schema Change Risk

Same as FID-126. `checklist_modifiers` is a new field in the JSON output. Affects:
- `src/agent/decision_parser.rs` (parse new array)
- `src/sandbox/grader.rs` (FID-130, use modifiers in counterfactual grading)
- `data/sandbox_responses/` (re-run baseline for A/B)

**Mitigation:** Versioned capture (same as FID-126).

## Changes

1. **`src/agent/prompts/strategy_knowledge.md`** — Replace the 10-point gate language with evaluation matrix instructions. Document the modifier table (initial values + calibration procedure). Document the order of operations (trigger → regime → modifiers → veto).
2. **`src/agent/prompts/output_format.md`** — Update the JSON schema to include `<checklist_modifiers>` array: `[{"criterion": "macro_catalyst", "status": "missing", "conviction_modifier": -0.10, "reasoning": "no major events in next 4h"}]`. Default to empty array if all criteria met.
3. **`src/agent/soul.md`** — Add explicit authorization: "Partial checklist completion (7/10+) with documented reasoning is grounds for a low-conviction entry. Reserve hard veto only for: invalid thesis, missing stop loss, catastrophic correlation."
4. **`config/checklist_modifiers.toml`** — New config file with the modifier table. Loaded at engine startup. Edited in-place during calibration (see FID-133 verification).
5. **`src/agent/decision_parser.rs`** — Parse `checklist_modifiers` array. Pass to sizing engine (FID-127) for final conviction computation.

## Verification

- `cargo check` and `cargo clippy -- -D warnings` pass (no Rust changes)
- Re-run sandbox. Verify:
  - LLM output includes `checklist_modifiers` array when criteria are missing
  - Pass decisions are 30-50% lower than current (was 56/60) — target is 48-50/60 Pass
  - Buy actions include at least one entry with 5/10 checklist + explicit reasoning
  - Conviction modifier distribution: 80% of decisions have modifiers in [-0.20, 0.0], 0% have modifiers < -0.50 (sanity check against runaway devaluation)
  - Hard veto criteria trigger correctly: 0 trades pass when invalid_thesis OR missing_stop_loss is flagged

## Live Engine Rollback Plan

This FID changes the LLM prompt's checklist behavior (10-point gate → evaluation matrix) and the JSON output schema (`checklist_modifiers` array). If the change causes regressions:
1. **Prompt rollback:** Revert `src/agent/prompts/strategy_knowledge.md` and `src/agent/prompts/output_format.md` to v0.13.8 snapshot via `git checkout src/agent/prompts/v0.13.8/ -- src/agent/prompts/`.
2. **Schema rollback:** Revert `src/agent/decision_parser.rs` to the pre-FID-132 commit (no `checklist_modifiers` field).
3. **Modifier value reset:** If only the values (not the structure) are wrong, reset `config/checklist_modifiers.toml` to the initial estimates — the calibration loop (FID-135) will re-derive them.
4. **Diagnostic:** Re-run sandbox with the same seed (FID-128). If conviction distribution is healthier before rollback, file a regression FID.

## Modifier Calibration Ownership (Open Task)

**Status:** The modifier values in `config/checklist_modifiers.toml` are initial estimates. The calibration procedure (logistic regression on 30+ live trades) is described above but **not yet owned by any FID**.

**Action:** Create a follow-up FID-135 (or higher) titled "Checklist Modifier Calibration Loop" that:
- Runs the logistic regression on a rolling 30-trade window
- Updates `config/checklist_modifiers.toml` automatically (or with human review)
- Emits a WARN when any modifier drifts > 0.05 from its initial value
- Tracks calibration history in `dev/audits/modifier-calibration-log.md`

**Workaround until FID-135 ships:** Manual review of `data/journal/` every 2 weeks; manually update `config/checklist_modifiers.toml` based on observed modifier-outcome correlation.

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **Modifier values unjustified** — -0.10, -0.20, -0.30 had no rationale. Added 6-row table with rationale per criterion + calibration source (KU audit + A/B test).
2. **Order of operations ambiguous** — FID-126 says conviction from triggers; FID-132 says conviction modified by checklist. Order matters. Added explicit 4-step sequence: trigger → regime → modifiers → veto.
3. **Hard veto criteria underspecified** — "Catastrophic correlation" not defined. Specified: peers -10%+ = catastrophic.
4. **Floor of 7/10 was arbitrary** — What about 5/10? Added: no fixed floor; the regime threshold is the only gate, modifiers compound. Documented explicitly.
5. **No calibration procedure** — Initial values are guesses. Added logistic regression procedure using 30+ live trades.
6. **Schema change risk not flagged** — Same as FID-126. Added schema-change-risk section.
7. **"30-50% lower Pass" was target or observation?** — Clarified: target, with explicit pass-count range (48-50/60).
8. **No config separation** — Modifier values should be in a config file, not hardcoded. Added `config/checklist_modifiers.toml`.

**Status:** All issues resolved. Ready for review.

## References

- Gemini research §Checklist and Knowledge Unit Audit
- FID-126: Conviction-Weighted Thresholds (synergy — both loosen strict gates)
- FID-131: KU Absolute-Language Scrub (provides calibration data for modifier values)
- FID-127: Conviction-Weighted Sizing (consumes the final conviction from order-of-operations step 3)
- FID-133: A/B Test Harness (provides the calibration data)
