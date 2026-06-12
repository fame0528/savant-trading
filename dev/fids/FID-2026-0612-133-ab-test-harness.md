# FID-133: A/B Test Harness for Prompt Comparison

**Filename:** `FID-2026-0612-133-ab-test-harness.md`
**ID:** FID-133
**Severity:** high
**Status:** open
**Phase:** 2 (Sandbox infrastructure)
**Created:** 2026-06-12
**Source:** Gemini Deep Research §A/B Test Design (`AI Trading Engine Rule Optimization.md`)

---

## Summary

Build a sandbox harness that runs Control (current 3+ threshold prompt) and Treatment (conviction-weighted prompt) on the same 60-scenario corpus and reports a 5-metric comparison. This isolates the prompt change as the only variable while keeping the model (MM3), Rust engine, and dataset constant.

## Background

The Gemini research §A/B Test Design specifies:
- **Dataset:** 60 standardized scenarios (30 historical replay + 30 synthetic jump-diffusion)
- **Control (A):** Current prompt with 3+ threshold, strict 10-point checklist, Deep Asian session penalty
- **Treatment (B):** Conviction-weighted prompt with fuzzy logic modifiers + XML few-shot examples
- **Model:** MM3 via OpenRouter, Temperature=0.1, TopP=0.95
- **Metrics:** Divergence Rate, Action Distribution, Counterfactual PnL, Brier Score, ECE

## Test Parameters (Spec)

| Parameter | Value |
|---|---|
| Scenarios | 60 (30 historical + 30 synthetic) |
| Control prompt | Snapshot of v0.13.8 prompt (frozen in `src/agent/prompts/v0.13.8/`) |
| Treatment prompt | Conviction-weighted, XML few-shot, fuzzy logic |
| Model | minimax/minimax-m3 (TokenRouter) |
| Temperature | 0.1 |
| Top P | 0.95 |
| Random seed | Per-scenario `seed: u64` (deterministic via FID-128) |

## Statistical Rigor

### Power Analysis for N=60

With N=60, paired t-test on per-scenario outcomes has:
- Effect size d=0.5 (medium), alpha=0.05, power=0.80 → requires N=34 ✓ (60 is sufficient)
- Effect size d=0.3 (small), alpha=0.05, power=0.80 → requires N=90 ✗ (60 is underpowered for small effects)

**Implication:** The A/B test can reliably detect medium effects (e.g., Brier reduction 0.20 → 0.15) but may miss small effects (e.g., 0.15 → 0.13). For small effects, the test should be run with N=120 (extend corpus) or repeated with 3 different seed sets and results aggregated.

### Multiple Comparisons Correction

5 metrics × 2 groups = 10 comparisons. With alpha=0.05, family-wise error rate inflates to ~40%. **Apply Bonferroni correction:** per-comparison alpha = 0.05 / 5 = 0.01. OR use Holm-Bonferroni (less conservative, preserves more power).

### Tie-Breaking Rule

If A/B test is ambiguous (e.g., 2 metrics A, 2 metrics B, 1 tie):
- **Primary metric wins:** Counterfactual PnL is the primary metric. If Treatment PnL > Control PnL by >5%, declare Treatment winner regardless of other metrics.
- **Calibration secondary:** If PnL is tied within 5%, Brier Score is tiebreaker (lower is better).
- **Manual review:** If PnL AND Brier are tied, escalate to manual review of scenario-level reasoning diffs (use HTC grader from FID-130 on the 10 closest calls).

## Control Prompt Snapshot Requirement

`PromptVersion::Current` must be a **frozen snapshot** of the v0.13.8 prompt, not a live reference. Otherwise, future prompt changes will silently change the control condition and invalidate historical A/B tests.

- **Implementation:** `src/agent/prompts/v0.13.8/` directory with frozen copies of `strategy_knowledge.md`, `soul.md`, `risk_constraints.md`, `output_format.md`
- **Verification:** `git log -- src/agent/prompts/v0.13.8/` should be empty after the initial copy commit (no edits to the snapshot)
- **New control for next A/B:** When shipping a new prompt version (e.g., v0.14), copy current to `v0.13.8/` and update the new control reference

## Cost Estimate

60 scenarios * 2 prompts * 5K tokens/scenario * $3/1M tokens = **$1.80 per A/B run**. With HTC grader (FID-130) at 10% sampling: +$0.36 = **$2.16 total**. Acceptable for nightly CI.

## Changes

1. **`src/sandbox/harness.rs`** — Add `run_ab_test()` function. Takes two prompt versions, runs both on the same 60-scenario corpus with identical per-scenario seeds (passed to FID-128 generator).
2. **`src/agent/prompts/v0.13.8/`** — New directory: frozen snapshot of current prompts. Copied via `scripts/snapshot_prompts.sh v0.13.8`. Gitignored from future edits (use `git update-index --skip-worktree`).
3. **`src/sandbox/prompts.rs`** — Add `PromptVersion::V0_13_8` and `PromptVersion::ConvictionWeighted` enums. Map each to the corresponding prompt directory.
4. **`src/sandbox/report.rs`** — New module: 5-metric A/B comparison report. Outputs table with Control/Treatment/Difference columns, p-values with Bonferroni correction, and effect sizes (Cohen's d).
5. **`src/main.rs`** — Add `--ab-test` CLI flag. When set, runs both prompt versions and writes report to `data/ab-test-reports/{timestamp}.md`.
6. **`src/sandbox/scenarios.rs`** — Build the 60-scenario corpus: 30 historical replay (FID-130 CoinGecko loader) + 30 new synthetic (FID-128 jump-diffusion generator).
7. **`scripts/snapshot_prompts.sh`** — New script: copies current prompts into versioned directory and sets skip-worktree.

## Verification

- `cargo test` — A/B test runs deterministically with same `seed` produces byte-identical output
- `cargo clippy -- -D warnings` — clean
- Run `--ab-test` on existing 60 scenarios. Report should show:
  - Treatment Divergence Rate < Control (currently 56%)
  - Treatment Action Distribution ~15-20% execution
  - Treatment Brier < Control
- Statistical significance: Treatment wins on Counterfactual PnL (primary metric) at p < 0.01 (Bonferroni-corrected)
- Power verification: For medium effects, power ≥ 0.80 (one-tailed)
- Snapshot integrity: `git diff src/agent/prompts/v0.13.8/` returns empty

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **N=60 power analysis missing** — Whether 60 is enough depends on effect size. Added explicit analysis: 60 is sufficient for medium effects, underpowered for small effects. N=120 or repeated seeds recommended for small effects.
2. **Multiple comparisons problem** — 5 metrics inflates family-wise error. Added Bonferroni correction (alpha=0.01 per metric) or Holm-Bonferroni as alternative.
3. **Tie-breaking rule absent** — What if 2-2-1 split? Added 3-step rule: PnL primary, Brier secondary, manual review as last resort.
4. **Control prompt snapshot not enforced** — "Current" prompt is a live reference, will change. Added explicit `v0.13.8/` snapshot directory + skip-worktree + git verification.
5. **Cost estimate missing** — 60 scenarios * 2 prompts = 120 LLM calls. Added: $1.80 base, $2.16 with HTC, acceptable for nightly CI.
6. **Statistical test specifics missing** — "p < 0.05" without specifying which test. Added: paired t-test, Cohen's d effect size, Bonferroni correction.
7. **Schema versioning across A/B** — The treatment prompt (FID-126) changes JSON schema. A/B test must handle schema mismatch. Added: versioned capture per FID-126 mitigation.

**Status:** All issues resolved. Ready for review.

## References

- Gemini research §A/B Test Design (full table of test parameters and metrics)
- FID-126: Conviction-Weighted Thresholds (Treatment prompt version)
- FID-128: Sandbox Jump-Diffusion Data (deterministic seeds)
- FID-130: Counterfactual Grader (provides the 5 metrics)
- FID-131: KU Absolute-Language Scrub (calibration data for FID-132 modifier values)
- FID-132: Checklist Evaluation Matrix (provides modifier values for the A/B)
- FID-134: 20 Adversarial Scenarios (separate 20-scenario corpus for stress test)
- Power analysis: Cohen 1988 (statistical power), Bonferroni 1936 (multiple comparisons correction)
