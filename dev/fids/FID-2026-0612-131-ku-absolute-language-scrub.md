# FID-131: Knowledge Unit Absolute-Language Scrub

**Filename:** `FID-2026-0612-131-ku-absolute-language-scrub.md`
**ID:** FID-131
**Severity:** critical
**Status:** open
**Phase:** 1 (Content only — no code change, just text edits)
**Created:** 2026-06-12
**Source:** Gemini Deep Research Q4 (`AI Trading Engine Rule Optimization.md` §Checklist and Knowledge Unit Audit)

---

## Summary

Audit all 265 knowledge units (KUs) for absolute linguistic markers ("Always", "Never", "Must", "Required", "Only when", "Wait for"). Convert each occurrence to probabilistic heuristic language that allows partial compliance. The MM3 model's instruction-following tuning treats absolute language as binding constraints, forcing Pass when any data point is suboptimal.

## Background

The MM3 model (per Gemini research §Q4) is fine-tuned for instruction following. Absolute language like "Volume MUST exceed 1.5x the 20-period average to confirm a valid breakout" creates a Boolean gate in the model's reasoning. When sandbox volume evaluates to 1.1x, the model correctly identifies the missing data point and chooses Pass — perfectly executing an over-constrained rule, but failing to act on the probabilistic asymmetry.

The 74.5% institutional/26% snipe knowledge distribution (per `dev/audits/FID-2026-0612-knowledge-base-institutional-audit.md`) amplifies this — institutional trading books are full of absolute "wait for confirmation" language.

## Before / After Example

**Before (absolute):**
> "Volume MUST exceed 1.5x the 20-period average to confirm a valid breakout."

**After (probabilistic):**
> "Volume exceeding 1.5x the 20-period average provides strong breakout confirmation. If volume is below average but price action demonstrates strong momentum, reduce conviction score and decrease position size rather than rejecting the trade outright."

## Search Patterns (Tier 1 — Must Fix)

- `\bAlways\b`
- `\bNever\b`
- `\bMust\b`
- `\bRequired\b`
- `\bOnly when\b`
- `\bWait for\b`
- `\bOnly if\b`
- `\bDo not enter\b` *(context-sensitive: legitimate when followed by "before major economic data release" or "without stop loss" — add to allowlist)*
- `\bDo not trade\b` *(context-sensitive: legitimate when followed by "during oracle deviation" — add to allowlist)*
- `\bRefuse to\b`

## Search Patterns (Tier 2 — Soften)

- `\bshould\b` → "consider", "evaluate"
- `\bmust not\b` → "avoid", "be cautious"
- `\bcritical that\b` → "important to"

## Proposed Rewrites (Per Pattern)

| Tier 1 Match | Rewrite to Probabilistic |
| :--- | :--- |
| "Always" | "When [condition] applies, prefer [action]. If [condition] is absent, consider alternative." |
| "Never" | "Avoid [action] when [risk condition]. If risk is mitigated, [action] may be acceptable." |
| "Must" | "Strongly prefer [action]. If [action] is impossible, [fallback] is acceptable with explicit reasoning." |
| "Required" | "Important for [action]: [condition]. If absent, reduce conviction and document." |
| "Only when" | "Best applied when [condition]. If [condition] is approximate, evaluate [partial credit]." |
| "Do not enter" (default) | "Reconsider entry when [risk condition] is active. If risk is data-confirmed absent, [entry] may be valid." |
| "Refuse to" | "Decline [action] when [condition]. Otherwise, evaluate [partial credit]." |

## Soft Modifier Vocabulary (Tier 2)

| Original | Soft Replacement | Meaning |
| :--- | :--- | :--- |
| should | consider | Probabilistic suggestion |
| must not | avoid | Default avoidance with exception path |
| critical that | important to | Priority signal, not gate |
| always prefer | lean toward | Bias, not rule |
| never | rarely | Statistical infrequency |

## Changes

1. **`knowledge/*.json`** — Audit all KU files for absolute language. **First, verify count:** `find knowledge/ -name "*.json" -exec grep -c . {} \; | awk '{s+=$1} END {print s}'` to confirm the 265 number (or document actual count).
2. **`scripts/audit_knowledge_units.py`** — New script: re-runnable scanner. **Must handle both prose and JSON KUs:**
   - For JSON KUs: parse the `condition`, `action`, `notes` fields separately (don't grep raw text — would produce false positives on JSON keys like `"required": true`)
   - For prose KUs: full-text scan
   - Output: CSV with `(file, line, pattern, context, suggested_rewrite)`
3. **`knowledge/*.json`** — Apply rewrites to highest-impact KUs (the 26 with "3+ trigger" language flagged in `FID-2026-0612-knowledge-base-institutional-audit.md`). For each rewrite, log the original + new text in `dev/audits/ku-rewrite-log.md` for review.
4. **`src/agent/soul.md`** — Add a meta-instruction: "Knowledge units contain probabilistic guidance. Absolute language in KUs reflects the AUTHOR's preference, not a hard rule. Adjust your action based on partial alignment and reasoning quality."
5. **`scripts/audit_knowledge_units.py`** — Wire into CI: `.github/workflows/ci.yml` runs the script and fails the build if any new Tier-1 matches are introduced.
6. **`scripts/audit_knowledge_units.py`** — Add `--regression-test` mode: loads `data/ku_regression_baseline.json` (a known-good snapshot of all KUs) and fails if any KU regresses to having absolute terms.

## Rollback Plan

If a rewrite breaks a load-bearing KU (LLM makes worse decisions because the rewrite was too aggressive):
1. **Per-KU rollback:** The `ku-rewrite-log.md` records original text; copy original back into the KU file
2. **Mass rollback:** `git revert <rewrite_commit>` reverts all rewrites from that PR
3. **Verification:** Re-run sandbox with same seed (FID-128) and compare conviction distribution pre/post rollback

## Verification

- Audit report shows 0 Tier-1 matches in active KUs (excluding allowlist)
- Tier-2 matches reduced by 80%
- KU count verified: `find knowledge/ -name "*.json" | wc -l` returns 265 (or actual count documented)
- `cargo check` and `cargo clippy -- -D warnings` pass (no Rust changes)
- Re-run sandbox. Conviction scores should show wider distribution (more partial alignments accepted)
- New script `scripts/audit_knowledge_units.py` exits with 0 when no absolute terms found
- `scripts/audit_knowledge_units.py --regression-test` exits with 0 against baseline
- CI workflow fails on new Tier-1 introduction

## Live Engine Rollback Plan

This FID changes the LLM prompt's knowledge unit content. If the absolute-language scrub causes a regression in KU quality (LLM makes worse decisions because the rewrite was too aggressive):
1. **Per-KU rollback:** Restore original text from `dev/audits/ku-rewrite-log.md` (records every rewrite). For one KU: `git checkout <rewrite_commit> -- knowledge/<file>.json`.
2. **Mass rollback:** `git revert <rewrite_commit>` reverts all rewrites in a single PR.
3. **Diagnostic:** Re-run sandbox with the same seed (FID-128) and compare conviction distribution pre/post rollback. If conviction distribution is healthier before rollback, file a regression FID and revert permanently.
4. **Long-term:** Maintain a "stable KU list" of ~50 hand-vetted KUs that are immune to automatic rewrites. Use these as anchors during calibration.

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **KU count of 265 unverified** — Should be confirmed before audit. Added verification command.
2. **JSON vs prose parsing** — KUs are JSON-structured (with `condition`, `action` fields). Naive grep would false-positive on JSON keys like `"required": true`. Added explicit JSON-field parsing requirement.
3. **Tier 1 "Do not enter" too aggressive** — Sometimes legitimate (e.g., "Do not enter before FOMC release"). Added allowlist for context-sensitive cases.
4. **No proposed rewrites** — Just patterns. Added 7-row rewrite table mapping each Tier-1 pattern to a probabilistic alternative.
5. **"Soft modifier" vocabulary vague** — "Consider" vs "evaluate" unspecified. Added 5-row soft modifier vocabulary with meaning.
6. **No CI integration** — Audit script not wired to fail builds on new absolute terms. Added GitHub Actions workflow.
7. **No regression test** — Audit script exits 0 on "no absolute terms" but doesn't catch regressions. Added `--regression-test` mode with baseline snapshot.
8. **No rewrite provenance** — If a rewrite breaks a KU, no record of original. Added `dev/audits/ku-rewrite-log.md` requirement.
9. **No rollback plan** — If mass rewrite backfires, no mitigation. Added per-KU + mass rollback procedure.

**Status:** All issues resolved. Ready for review.

## References

- Gemini research §Checklist and Knowledge Unit Audit
- `dev/audits/FID-2026-0612-knowledge-base-institutional-audit.md` (74.5% institutional content with absolute language)
- FID-126: Conviction-Weighted Thresholds (synergy — both loosen strict gates)
- FID-132: Checklist Evaluation Matrix (companion prompt restructure)
