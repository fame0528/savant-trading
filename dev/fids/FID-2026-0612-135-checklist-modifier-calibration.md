# FID-135: Checklist Modifier Calibration Loop

**Filename:** `FID-2026-0612-135-checklist-modifier-calibration.md`
**ID:** FID-135
**Severity:** medium
**Status:** open
**Phase:** 2 (New module — depends on FID-126, FID-127, FID-132)
**Created:** 2026-06-12
**Source:** Open task identified in FID-132 Perfection Loop Log (Modifier Calibration Ownership)

---

## Summary

Build a closed-loop calibration system that periodically refits the conviction modifier values in `config/checklist_modifiers.toml` using live trade outcomes. Replaces the static initial estimates (defined in FID-132) with data-driven values, emits warnings when modifiers drift, and tracks the calibration history for auditability.

## Background

FID-132 introduced the 10-point checklist → evaluation matrix with six "soft" conviction modifiers (e.g., missing macro catalyst = -0.10, negative BTC correlation = -0.20). The values are initial estimates from Gemini research — not derived from empirical data. Without calibration, the modifiers are a guess. Without ownership of the calibration loop (this FID), the modifiers stay wrong forever.

**The core problem:** the live engine emits `checklist_modifiers` and `actual_outcome` (PnL sign) on every closed trade. A logistic regression of `outcome ~ sum(modifiers) + regime` can be fit on a rolling window to determine if the modifier values are right-sized. This FID owns that loop.

## Calibration Procedure

```text
For each closed trade in data/journal/trades.jsonl:
  - Extract: { regime, modifier_sum, outcome_binary (1=win, 0=loss) }

Every 30 closed trades:
  1. Fit logistic regression: P(win) = sigmoid(α + β * modifier_sum + γ * regime)
  2. If β is statistically significant (p < 0.05):
     - The current modifier values are working — keep them, log "calibration stable"
     - If β is positive: modifier direction is correct (more modifiers → fewer wins is wrong; revert sign)
     - If β is negative and |β| > 0.5: modifiers may be too punitive, scale by 0.8
  3. If β is not significant (p ≥ 0.05):
     - Modifiers are not predictive — the calibration is "indeterminate"
     - Flag for manual review; do NOT auto-update

Every 90 closed trades:
  1. Refit per-criterion logistic: P(win) = sigmoid(α + β_c * modifier_c + ...)
  2. For each criterion c with |β_c - β_initial| > 0.05:
     - Update config/checklist_modifiers.toml with the new β_c value
     - Emit a calibration log entry: { date, criterion, old, new, sample_size, p_value }
     - If |drift| > 0.15: emit a WARN to operator

On every calibration event:
  - Append to dev/audits/modifier-calibration-log.md
  - Track {date, criterion, old_value, new_value, sample_n, p_value, action_taken}
  - Keep last 100 events in the log; rotate older events to modifier-calibration-log.archive.md
```

## Per-Criterion Coefficient Tracking

The 6 soft modifiers tracked individually:
- `missing_macro_catalyst` (initial -0.10)
- `negative_btc_correlation` (initial -0.20)
- `missing_invalidation_level` (initial -0.30)
- `tight_liquidity` (initial -0.20)
- `extended_overbought` (initial -0.15)

Plus 3 hard vetoes (NOT calibrated — always absolute):
- `invalid_thesis` (VETO)
- `missing_stop_loss` (VETO)
- `catastrophic_correlation` (VETO)

## Statistical Concerns (Production Hardening)

The basic logistic regression loop above is a starting point. For production robustness, the calibrator must handle:

1. **Per-criterion minimum sample size:** A criterion may have 0 trades in a 30-trade window (e.g., "missing invalidation level" is rare). The logistic regression breaks silently. **Mitigation:** Require ≥ 10 trades per criterion before refitting that specific coefficient. If a criterion has < 10 trades in the window, skip the refit for that criterion and log "indeterminate" for that coefficient. The other criteria still refit.

2. **Multicollinearity:** "Missing macro catalyst" and "missing invalidation level" are likely correlated in practice (a sloppy trade setup usually has multiple missing criteria). Per-criterion coefficients become unstable when predictors are correlated. **Mitigation:** Compute VIF (variance inflation factor) per criterion. If VIF > 5, drop the criterion from the refit window and log a WARN. Alternative: use Ridge regularization (L2 penalty) to stabilize coefficients.

3. **Regime-conditioning:** If market regime shifts (bull → bear), the modifier-outcome relationship may invalidate entirely, not just drift. The basic logistic regression assumes stationarity. **Mitigation:** Fit separate logistic regressions per regime (Trending / Volatile / Ranging per FID-126). When the regime distribution in the calibration window shifts > 20% from the baseline (e.g., 60% Trending becomes 30% Trending), emit a WARN and pause auto-calibration for 24 hours to allow manual review.

4. **Survivorship bias:** Only closed trades feed back into the calibration window. Currently-open positions are invisible. Calibration only sees what the model decided to close, which biases toward safe/quick trades. **Mitigation:** Include currently-open positions in the calibration window with a `pending: true` flag. Weight closed trades at 1.0 and pending positions at 0.5 (lower confidence). This widens the sample and reduces survivorship bias.

**Implementation note:** These four concerns add ~200 lines of Rust. The statistical machinery (VIF, regime-conditional fits, weighted regression) is standard; the per-criterion min-sample check is the simplest of the four.

## Drift Detection Thresholds

| Drift Magnitude | Action |
| :--- | :--- |
| `|new - old| ≤ 0.02` | No change, log "stable" |
| `0.02 < |new - old| ≤ 0.05` | Update config, log INFO |
| `0.05 < |new - old| ≤ 0.10` | Update config, log WARN, require manual confirmation |
| `|new - old| > 0.10` | Update config, log WARN + alert, pause trading for 1 hour to allow operator review |
| `|new - old| > 0.20` | Refuse to auto-update; require explicit operator override (protective circuit breaker) |

## Changes

1. **`src/calibration/modifier_calibrator.rs`** — New module: `ModifierCalibrator`. Implements:
   - `fit_logistic(window: &[Trade]) -> CalibrationResult`
   - `extract_features(trade: &Trade) -> (modifier_sum, regime_one_hot)`
   - `apply_result(result: &CalibrationResult, config: &mut ChecklistModifiersConfig)` — writes new values with safety thresholds
2. **`src/calibration/calibration_log.rs`** — New module: append-only calibration event log with rotation to `.archive.md` after 100 events.
3. **`src/calibration/mod.rs`** — New module root.
4. **`src/engine/mod.rs`** — In the trade-close path, after `data/journal/trades.jsonl` is updated, call `ModifierCalibrator::maybe_run(window_size=30, refit_size=90)`. Frequency: every Nth trade where N = closed_trade_count % 30 == 0 for check, % 90 == 0 for refit.
5. **`src/agent/prompts/strategy_knowledge.md`** — Add a meta-instruction: "Modifier values in `config/checklist_modifiers.toml` are auto-calibrated every 90 trades. A WARN log entry means a criterion's value has drifted > 0.05 from its initial — treat as informational, not a rule change."
6. **`scripts/calibration_status.py`** — New script: reads `dev/audits/modifier-calibration-log.md` and prints a status table. Useful for manual reviews.
7. **`Cargo.toml`** — Add `statrs = "0.16"` for logistic regression (or implement logistic regression from scratch with `nalgebra` — preferred, no new dep).

## Manual Override

Operators can override auto-calibration:
- `config/checklist_modifiers.toml` has a `calibration_enabled: bool` field (default `true`)
- If `false`, the calibrator still runs but only logs without writing to the config
- Manual edits to the TOML are preserved across calibrations if `calibration_enabled: false`

## Verification

- `cargo test` — unit tests for:
  - `fit_logistic` on synthetic data: known β → recovered β within ±0.05
  - Drift detection: |drift| = 0.04 → no change, 0.06 → INFO log, 0.11 → WARN + confirm, 0.21 → refuse
  - Calibration log rotation: 100 events → oldest 50 moved to archive
  - Manual override: `calibration_enabled = false` → no config write, but log still records
- `cargo clippy -- -D warnings` — clean
- Run calibrator on the existing 30+ live trades from `data/journal/trades.jsonl` (if any). If sample size < 30, calibrator should report "indeterminate, not enough data" and NOT auto-update.
- Live verification: After 90 closed trades, `config/checklist_modifiers.toml` reflects the calibrated values. `dev/audits/modifier-calibration-log.md` shows the calibration events.

## Live Engine Rollback Plan

This FID adds a background calibration loop. If it causes regressions (e.g., auto-updated modifiers cause the live engine to behave erratically):
1. **Disable auto-calibration:** Set `calibration_enabled = false` in `config/checklist_modifiers.toml`. The loop still runs but doesn't write to config.
2. **Revert config to initial values:** Restore the FID-132 initial values (e.g., -0.10, -0.20, -0.30) via `git checkout HEAD~N -- config/checklist_modifiers.toml`.
3. **Diagnostic:** Re-run sandbox with the same seed (FID-128). If conviction distribution is healthier before calibration, the auto-updated values are wrong — disable permanently and file a regression FID.
4. **Long-term:** Maintain a "frozen modifier list" of values that have been hand-vetted over 100+ trades. The calibrator's auto-updates should be compared against this list; large deviations should be reviewed.

## Dependencies

- **Depends on:** FID-126 (conviction_score field), FID-127 (sizing math consumes final conviction), FID-132 (provides `config/checklist_modifiers.toml` schema)
- **Required by:** FID-132 (long-term modifier maintenance — the initial values in FID-132 are placeholders until FID-135 ships and starts auto-calibrating)
- **Ordering:** Ship after FID-126, FID-127, FID-132 are merged. Can ship in v0.15.0 alongside Phase 2 work.

## Perfection Loop Log

### Iteration 2 (2026-06-12) — Code review feedback

**Issues found:**
1. **Statistical rigor gap in calibration loop** — Original spec had logistic regression + p-value + drift thresholds, but missed 4 production-grade concerns.
2. **"Required by" misframed** — Said "none" but FID-132's modifier values are the calibration target. Reframed to reference FID-132 long-term maintenance.

**Fixes applied:**
1. **Added "Statistical Concerns" subsection** — 4 production concerns: per-criterion min sample (≥ 10), multicollinearity (VIF > 5 → drop), regime-conditioning (separate fits per regime, WARN on 20% shift), survivorship bias (include open positions at 0.5 weight).
2. **Updated "Required by" to reference FID-132** — Clarified that FID-135 is the long-term owner of the modifier values defined in FID-132.

**Status:** All issues resolved. Ready for review.

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **No frequency cap** — Original spec didn't say how often to run the calibration. Added: every 30 trades for check, every 90 for refit. Prevents overfitting on small samples.
2. **No statistical significance check** — Without p-value gating, the calibrator would update on noise. Added: β must have p < 0.05 to count as "predictive"; otherwise flag as indeterminate.
3. **No drift safety thresholds** — What if a coefficient drifts to -2.0 (huge penalty)? Added 4-tier drift magnitude table with progressive responses (no change / INFO / WARN+confirm / pause+alert / refuse).
4. **No manual override** — Operators may want to freeze values. Added `calibration_enabled: bool` field with clear behavior.
5. **No log rotation** — Calibration log grows forever. Added: rotate to `.archive.md` after 100 events.
6. **No dependency declaration** — Added Depends on / Required by / Ordering sections.
7. **Hard vetoes mixed with soft modifiers** — Spec said "track 6 soft + 3 hard" but didn't make explicit that hard vetoes are NOT calibrated. Added explicit "NOT calibrated — always absolute" note.
8. **No fallback for insufficient data** — What if < 30 trades exist? Added: report "indeterminate, not enough data" and skip update.
9. **No new Cargo dep justification** — `statrs` adds 50+ transitive deps. Recommended implementing logistic regression from scratch with `nalgebra` (already likely present) or pure Rust.

**Status:** All issues resolved. Ready for review.

## References

- FID-132: Checklist Evaluation Matrix (defines initial modifier values, identifies this FID as open task)
- FID-126: Conviction-Weighted Thresholds (provides conviction_score field)
- FID-127: Conviction-Weighted Sizing (consumes final conviction that uses modifiers)
- FID-133: A/B Test Harness (provides initial calibration data via A/B comparisons)
- Gemini research §Checklist and Knowledge Unit Audit (modifier value estimates)
- Calibration literature: Hosmer & Lemeshow 2000 (logistic regression diagnostics), Flach 2012 (calibration in ML)
