# FID-130: Counterfactual Grader (OPE + Calibration + HTC)

**Filename:** `FID-2026-0612-130-counterfactual-grader.md`
**ID:** FID-130
**Severity:** critical
**Status:** open
**Phase:** 2 (New module — depends on FID-126 for confidence output)
**Created:** 2026-06-12
**Source:** Gemini Deep Research Q3 (`AI Trading Engine Rule Optimization.md` §Counterfactual Grading Methodology)

---

## Summary

Replace the strict-equality grader (`action == expected_action`) with a multi-metric counterfactual evaluator that:
1. Computes expected PnL of the LLM's decision over forward 5m candles (30-min horizon, 6 candles)
2. Calculates Brier Score and Expected Calibration Error for confidence calibration
3. Spawns a secondary "Grader LLM" using Holistic Trajectory Calibration (HTC) to assess reasoning quality, not just final action
4. Uses Marginal Ratio (MR) or Log-Sum-Exponential (LSE) estimators instead of Inverse Propensity Scoring (IPS) — IPS has infinite variance when the model Pass-heavy

## Background

The current grader rewards "Pass" as default. When the model refuses to trade 56/60 times, every pass counts as a "match" against `expected_action = Pass`, hiding the fact that the model would have missed profitable trades. The grader needs to know:
- Would the LLM's action have made money if executed? (Counterfactual PnL)
- Did the LLM's confidence match its actual accuracy? (Brier, ECE)
- Was the LLM's reasoning sound even if the final action was wrong? (HTC)

## Metrics

| Metric | Definition | Target |
|---|---|---|
| **Divergence Rate** | % actions that deviate from counterfactual optimal | < 10% |
| **Action Distribution** | Ratio of Buy/Sell/Close to Pass | ~15-20% execution |
| **Counterfactual PnL** | Aggregate theoretical return - slippage | Maximize |
| **Brier Score** | MSE between LLM confidence and binary outcome | < 0.15 |
| **Expected Calibration Error** | \|avg_confidence - avg_accuracy\| per bin | < 0.05 |

## OPE Estimator Choice

**Marginal Ratio (MR) vs Inverse Propensity Scoring (IPS):** Gemini research correctly notes IPS has infinite variance when the behavior policy is Pass-heavy (which the current M3 model is). **However, "Marginal Ratio" is not a standard OPE estimator in the literature** — standard estimators are IPS, Doubly Robust (DR), and Self-Normalized IPS (SNIPS).

**Recommended implementation:** Use **Doubly Robust (DR) estimator** with importance weight clipping (to handle Pass-heavy policy):
- DR combines a direct method (reward model) with IPS
- Importance weights are clipped to max 10.0 to prevent variance blowup
- This is the same approach used in production recommender systems (e.g., YouTube, Netflix)

If MR is preferred for theoretical reasons, the FID should cite the source paper (not in standard OPE literature). **Default to DR with clipping** for this FID.

## Confidence Field Clarification

The grader needs to know which field is the "confidence" input:
- **Use `conviction_score` from FID-126** as the LLM's stated confidence
- The grader maps `conviction_score` → Brier Score → ECE
- If `conviction_score` is missing (old prompts), grader uses a default of 0.5 and emits a warning

**Binarization rule for forward window outcome:** A scenario's 6-candle forward window is binarized as:
- `outcome = 1` if max favorable excursion (MFE) > 2 * estimated_slippage
- `outcome = 0` if max adverse excursion (MAE) > MFE (price moved against the trade)
- `outcome = 0.5` (excluded from Brier) if neither threshold hit (ambiguous)

This prevents the Brier Score from being dominated by ambiguous no-move scenarios.

## Historical Replay Source

The 30 historical scenarios need a data source:
- **Primary:** CoinGecko historical OHLCV (free tier, 1-minute resolution for top 200 tokens, last 90 days)
- **Secondary:** Arbitrum chain data via Blockscout API (limited history, last 1000 blocks)
- **Implementation:** Add `src/sandbox/historical_loader.rs` that fetches and caches 1-minute OHLCV into `data/historical_cache/{token}/{date}.json`
- **Cache TTL:** 30 days (historical data is immutable, but allow refresh for bug fixes)

## HTC Grader LLM Cost Control

The secondary Grader LLM doubles the API cost. Mitigation:
- **Default:** OFF (counterfactual grader runs without HTC)
- **Opt-in:** `--grader=counterfactual,htc` flag adds HTC pass
- **Sampling:** HTC runs on a random 10% sample of scenarios by default; `--htc-sample-rate=1.0` for full coverage
- **Cost estimate:** 60 scenarios * 10% = 6 HTC calls, ~$0.06 per call = $0.36 per sandbox run

## `--grader=strict` Deprecation

The strict-equality grader is a known false-positive generator (rewards "Pass as Default"). Keeping it as an option encourages future regression.
- **Action:** Log a `WARN` deprecation message when `--grader=strict` is used
- **Timeline:** Remove in v0.16.0 (two minor versions from now)

## Changes

1. **`src/sandbox/grader.rs`** — New module: `counterfactual_grader`. Implements:
   - `CounterfactualOutcome { pnl, mfe, mae, was_optimal, outcome_binary }` from forward candle window
   - `doubly_robust_estimator()` with importance weight clipping at 10.0
   - `brier_score()` and `expected_calibration_error()` aggregations (using binarization rule above)
2. **`src/sandbox/harness.rs`** — Wire counterfactual grader into `run_sandbox()`. Replace strict-equality assertion with multi-metric report.
3. **`src/sandbox/htc_grader.rs`** — New module: secondary LLM call to grade reasoning quality. Reads the primary agent's full trajectory + raw response, outputs a reasoning quality score (0-1) and pass/fail flags for: "logic was sound", "constraint was wrong", "hallucinated trigger", "ignored data". Opt-in via `--grader=counterfactual,htc`.
4. **`src/sandbox/historical_loader.rs`** — New module: fetches and caches historical OHLCV from CoinGecko. Used to build 30 historical replay scenarios.
5. **`src/sandbox/scenarios.rs`** — Add `forward_horizon: Vec<Candle>` to each scenario schema (next 6 candles from `expected_action` time). 30 historical scenarios (CoinGecko replay) + 30 synthetic (FID-128 jump-diffusion).
6. **`src/sandbox/mod.rs`** — Add `--grader=counterfactual` CLI flag (default), `--grader=counterfactual,htc` for full grading, `--grader=strict` (deprecated, logs WARN).
7. **`Cargo.toml`** — Add `reqwest = "0.11"` for CoinGecko API. Add `tokio` if not present for async fetching.

## Verification

- `cargo test` — new unit tests for:
  - Brier Score: known-input known-output (e.g., all-correct at 0.8 confidence → Brier = 0.16)
  - ECE: 5 bins, 10 predictions per bin, check bin assignment
  - DR estimator: behavior policy uniform vs target = same policy → DR ≈ true value
  - Outcome binarization: MFE-dominated, MAE-dominated, ambiguous scenarios
  - Importance weight clipping: weight = 1000.0 → clipped to 10.0
- `cargo clippy -- -D warnings` — clean
- Re-run sandbox. New report should show:
  - Divergence Rate < 10% (was 56%)
  - Counterfactual PnL non-zero (some scenarios profitable, some not)
  - Brier < 0.15
  - ECE < 0.05
- HTC Grader LLM (opt-in) should flag 3-5 reasoning failures (e.g., "model said Pass but reasoning admitted bullish setup")

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **"Marginal Ratio" estimator is non-standard** — Not in standard OPE literature. Replaced with Doubly Robust (DR) estimator + importance weight clipping at 10.0, which is the production-grade approach for Pass-heavy policies.
2. **Confidence field source ambiguous** — Is it `confidence` or `conviction_score`? Specified: use `conviction_score` from FID-126, default 0.5 with warning if missing.
3. **Brier binarization rule undefined** — "Binary outcome" from a 6-candle window is ambiguous. Added rule: MFE > 2*slippage = 1, MAE > MFE = 0, else exclude (0.5).
4. **No historical data source** — "30 historical scenarios" was specified but no source. Added CoinGecko primary + Blockscout secondary + caching strategy.
5. **HTC grader doubles API cost** — 120 LLM calls per sandbox run (60 primary + 60 secondary). Made HTC opt-in with 10% default sampling.
6. **`--grader=strict` deprecation** — Keeping the false-positive generator as an option encourages regression. Added deprecation WARN + removal timeline (v0.16.0).
7. **Test cases absent** — Same as FID-127. Added 5 specific test scenarios with expected values.
8. **No required Cargo deps** — `reqwest` and `tokio` needed for CoinGecko. Added.

**Status:** All issues resolved. Ready for review.

## References

- Gemini research §Counterfactual Grading Methodology (DR estimator replaces MR; HTC framework preserved with opt-in cost control)
- FID-126: Conviction-Weighted Thresholds (provides `conviction_score` for Brier)
- FID-128: Sandbox Jump-Diffusion Data (provides synthetic scenarios)
- FID-131: KU Absolute-Language Scrub (grader will see fewer false-positive Pass defaults)
- FID-133: A/B Test Harness (will use the counterfactual grader)
- OPE literature: Dudík et al. 2011 (DR estimator), Swaminathan & Joachims 2015 (SNIPS, self-normalized IPS)
