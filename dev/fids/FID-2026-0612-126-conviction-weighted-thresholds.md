# FID-126: Conviction-Weighted Threshold System

**Filename:** `FID-2026-0612-126-conviction-weighted-thresholds.md`
**ID:** FID-126
**Severity:** critical
**Status:** open
**Phase:** 1 (Prompt + Content only — no Rust engine change)
**Created:** 2026-06-12
**Source:** Gemini Deep Research Q1 (`AI Trading Engine Rule Optimization.md` §Threshold Design)

---

## Summary

Replace the rigid "3+ aligned triggers" gate in the LLM prompt with a **conviction-weighted fuzzy logic system** that allows partial trigger alignment to produce probabilistic confidence scores. The current Boolean gate multiplies trigger probabilities to near-zero, collapsing Buy actions to 0% (FID-125 follow-up audit).

## Background

The M3 sandbox showed that even after FID-125 (dynamic pair list fix), the model still produced 0 Buy actions across 60 scenarios. Root cause: 4 independent filters (3+ triggers, Deep Asian session, Ranging regime, Low volume) multiply probabilistically. The model defaults to Pass when no path through all 4 filters exists. Gemini research §Threshold Design confirms this is "compounding constraint failure" and prescribes fuzzy logic + regime-dependent thresholds.

## Regime-Dependent Threshold Matrix

| Regime | ADX | ATR | Conviction Threshold | Trigger Equivalence | Risk Veto |
|---|---|---|---|---|---|
| **Trending** | ADX > 25 | ATR ≤ 2x baseline | 0.50 | 2+ Triggers | Standard invalidations |
| **Volatile** | any | ATR > 2x baseline | 0.60 | 2.5+ Triggers | Mandatory bear veto |
| **Ranging** | ADX < 20 | any | 0.75 | 3+ Mean-reversion triggers | Tight stop mandate |
| **Grey Zone** | 20 ≤ ADX ≤ 25 | ATR ≤ 2x baseline | 0.65 | 2.5+ Triggers, must include regime-disambiguating trigger (range break OR trend continuation) | Standard invalidations |

**Why Volatile needs MORE triggers than Trending (0.60 vs 0.50):** Volatile regimes have higher noise-to-signal; requiring more aligned triggers + higher conviction prevents over-trading whipsaws. Risk veto is mandatory bear-only (no counter-trend longs in vol).

**Why Ranging needs 0.75 + 3 mean-reversion triggers:** Mean-reversion requires statistical confluence (Bollinger touch + RSI extreme + range position). One trigger = noise; three = signal.

**Grey zone handling:** ADX 20-25 is a "regime uncertainty" window. Require 2.5+ triggers AND a disambiguating trigger that explicitly resolves the regime (range-boundary break or trend-continuation higher-high). Default to Volatile's risk veto if no disambiguator.

## Trigger-to-Conviction Mapping

Triggers are weighted, not counted. Each trigger contributes a partial conviction score based on its quality:

```text
Strong trigger (full weight 1.0): EMA cross with body > 50% of candle, VWAP bounce with volume, breakout above 20-period high with volume > 1.5x
Moderate trigger (weight 0.7): VWAP support hold, MACD cross, RSI oversold (<30) without divergence
Weak trigger (weight 0.4): Partial EMA alignment, BB touch, low-volume cross

conviction_score = clamp(sum(trigger_weights) / 3.0, 0.0, 1.0)
```

Example: 1 strong + 1 moderate + 1 weak = 1.0 + 0.7 + 0.4 = 2.1 / 3.0 = 0.70 → passes Trending (0.50) and Grey Zone (0.65) thresholds, fails Ranging (0.75).

## Fuzzy Volume Membership

Trapezoidal function: 0.25x avg = 0, 1.1x avg = 0.6, 1.5x+ avg = 1.0. Below-threshold volume contributes partial credit instead of failing Boolean. The volume trigger is itself a fuzzy input to the conviction calculation; it does not gate independently.

## Changes

1. **`src/agent/prompts/strategy_knowledge.md`** — Replace "3+ aligned triggers" rule with conviction-weighted scoring instructions. Document the regime matrix (including grey zone), trigger weights, and trapezoidal volume function in human-readable form.
2. **`src/agent/prompts/strategy_knowledge.md`** — Add few-shot XML examples showing partial compliance (e.g. "Conviction 2.4/3.0 → BUY with 0.5x size despite Deep Asian session").
3. **`src/agent/prompts/output_format.md`** — Add `<conviction_score>` (clamped 0.0-1.0), `<sizing_multiplier>` (clamped 0.0-1.0), and `<regime_label>` (Trending/Volatile/Ranging/GreyZone) fields to the JSON schema. Add explicit instruction: "7/10 checklist points + no critical invalidation = grounds for low-conviction entry."
4. **`src/agent/prompts/strategy_knowledge.md`** — Add anti-pattern block: "DO NOT default to conviction_score=0.50 or 0.65. Output a granular score based on actual trigger quality. Calibration will be measured by Brier Score (FID-130); defaulting to threshold values yields Brier > 0.30 and is a calibration failure."
5. **`src/agent/prompts/strategy_knowledge.md`** — Add out-of-range handling: "If you cannot compute a conviction score, output 0.0 and select PASS. If conviction > 1.0, clamp to 1.0. If sizing_multiplier > 1.0, clamp to 1.0. The engine will reject (PASS) any decision where conviction < regime threshold."

## Schema Change Risk

The JSON schema change in `output_format.md` breaks any code that parses the LLM response. Affected files:
- `src/agent/decision_parser.rs` — must handle new fields
- `src/agent/response_capture.rs` (FID-124) — raw response format versioned to v2
- `data/sandbox_responses/` — old captures become invalid for A/B comparison unless re-run

**Mitigation:** Add `prompt_version` field to response capture so old vs new captures remain distinguishable in A/B tests (FID-133).

## Reference Few-Shot Example (for prompt file)

```xml
<few_shot_example>
  <market_state>
    Regime: Trending (ADX 28)
    Triggers: EMA9 > EMA21 (1.0, strong), VWAP Support (0.7, moderate), Volume at 0.9x Average (0.4, weak)
    Session: Deep Asian
  </market_state>
  <reasoning>
    Cumulative trigger weights = 2.1/3.0 = conviction 0.70. Trending regime threshold = 0.50. Passes with margin. Volume is below 1.5x but fuzzy membership = 0.4, contributing partial credit. Deep Asian session no longer penalized (FID-129). RSI = 62, MACD = flat but positive, range position = mid. No risk veto triggers active.
  </reasoning>
  <action>
    Decision: BUY
    Conviction: 0.70
    Sizing_Multiplier: 0.75
    Regime: Trending
  </action>
</few_shot_example>
```

## Verification

- `cargo check` and `cargo clippy -- -D warnings` pass (no Rust changes, but verify prompts compile via include_str!)
- Re-run sandbox with `--save-responses` and 60-scenario corpus. Verify:
  - Buy action count > 0 (currently 0)
  - Conviction scores vary across scenarios (not all 0.0 or 1.0)
  - Regime-dependent behavior observable in raw responses
- **Target distribution (not just count):**
  - Conviction score std dev > 0.15 (proves no threshold-cliff collapse)
  - At least 1 Buy in Trending regime, at least 1 Buy in Volatile regime (regime coverage)
  - 0 outputs of conviction_score exactly = 0.50 or 0.65 (anti-default verification)
- **Brier Score check (FID-130 grader):**
  - Initial Brier < 0.30 (better than uninformative 0.50)
  - Target after tuning: Brier < 0.15
- **A/B comparison (FID-133):**
  - Treatment Divergence Rate < Control (currently 56%)

## Dependencies

- **Depends on:** nothing (prompt-only)
- **Required by:** FID-127 (consumes conviction_score for sizing), FID-130 (Brier calibration grading), FID-132 (checklist matrix produces modifier values used here)
- **Ordering:** Ship FID-126 + FID-129 together (both loosen the same over-constrained gates). FID-127 should ship in the same release or one release later (Rust can default conviction_scaler to 0.5 if field missing).

## Risks

1. **Model regression to threshold-cliff:** LLM could learn "Trending + 2+ = 0.50" and output exactly 0.50 for all Trending setups. Mitigation: anti-pattern block + Brier Score gating.
2. **Conviction score binarization at 0.50:** Same as #1, different symptom. Mitigation: std dev > 0.15 verification check.
3. **Grey zone collapses everything to "Volatile":** If grey zone rules are too strict, model may default to Volatile classification. Mitigation: monitor regime label distribution; expect 30-50% of scenarios in Grey Zone.
4. **Backwards compat:** Existing response captures (FID-124) are invalid post-change. Mitigation: versioned capture + re-run baseline.

## Live Engine Rollback Plan

This FID changes the LLM prompt for the live engine. If absolute-language scrub causes a regression in KU quality:
1. **Per-KU rollback:** Restore original text from `dev/audits/ku-rewrite-log.md` (records every rewrite). For one KU: `git checkout <commit> -- knowledge/<file>.json`.
2. **Mass rollback:** `git revert <rewrite_commit>` reverts all rewrites in a single PR.
3. **Diagnostic:** Re-run sandbox with the same seed (FID-128) and compare conviction distribution pre/post rollback. If conviction distribution is healthier before rollback, file a regression FID and revert permanently.
4. **Long-term:** Maintain a "stable KU list" of ~50 hand-vetted KUs that are immune to automatic rewrites. Use these as anchors during calibration.

## Perfection Loop Log

## Live Engine Rollback Plan

This FID changes the LLM prompt for the live engine ($30 micro-capital). If conviction-weighted prompts cause real losses:
1. **Immediate rollback (≤ 5 min):** Revert prompt via `git checkout HEAD~1 -- src/agent/prompts/strategy_knowledge.md` and redeploy. Previous prompt version (v0.13.8) is preserved in `src/agent/prompts/v0.13.8/` per FID-133.
2. **Diagnostic (≤ 1 hour):** Query `data/journal/` for the most recent 20 trades. Check: (a) conviction distribution, (b) regime distribution, (c) Brier Score on recent outcomes. If conviction is bimodal at threshold, the anti-pattern guard is failing.
3. **Long-term:** If conviction-weighted framework systematically underperforms, downgrade to a soft "Tier 1: trigger weights computed; Tier 2: score > 0.50 = BUY" hybrid. Document the regression in `dev/audits/fid-126-regression-{date}.md`.

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **Grey zone undefined** — ADX 20-25 had no rule. Added explicit "Grey Zone" row with 0.65 threshold + regime-disambiguator requirement.
2. **Trigger-to-conviction mapping implicit** — "2+ triggers = 0.50" was underspecified. Added weighted-trigger formula (strong=1.0, moderate=0.7, weak=0.4) and worked example.
3. **Volatile threshold higher than Trending was unjustified** — Added explanation (noise-to-signal ratio).
4. **No anti-pattern guard** — Model could default to 0.50. Added explicit anti-pattern block + Brier Score verification.
5. **Schema change breaks downstream parsers** — decision_parser.rs, response_capture.rs, sandbox responses. Added schema-change-risk section with mitigation (versioned capture).
6. **Verification too weak** — "Buy count > 0" doesn't check distribution. Added std dev > 0.15 + regime coverage checks.
7. **Few-shot example not embedded** — Added full reference example in markdown.
8. **Dependencies not enumerated** — Added Depends-on / Required-by / Ordering sections.

**Status:** All issues resolved. Ready for review.

## References

- Gemini research §Threshold Design
- FID-125: Dynamic Pair List (fixed stale pair refusal, but Buy count still 0)
- M3 sandbox raw response capture (FID-124): 34/60 divergences, 0 Buy actions
- FID-129: Remove Deep Asian Penalty (companion Phase 1 fix)
- FID-130: Counterfactual Grader (Brier/ECE grading)
- FID-132: Checklist Evaluation Matrix (provides modifier values)
- FID-133: A/B Test Harness (Treatment prompt validation)
- ADR: not yet created
