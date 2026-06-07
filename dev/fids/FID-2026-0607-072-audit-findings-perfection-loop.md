# FID-072: Audit Findings Perfection Loop — Execution Layer + Risk Gates

**Status:** complete
**Severity:** high
**Created:** 2026-06-07
**Author:** Nova

---

## RED Phase — Issues Cataloged

### Primary
1. Retry queue `drain_retry_queue` has no recovery behavior.
2. `R:0.0` on every signal.

### Secondary
3. Max-positions gate suppresses otherwise valid trade setups.
4. Session penalty logic is applied uniformly instead of per-market condition.
5. Zero or near-zero volume is treated as a soft pass instead of a data-quality/no-trade condition.
6. LINK confidence is driven by existing position state instead of evaluated edge strength.

---

## GREEN Phase — Fix Plan

| # | Issue | Fix | File | Lines |
|---|-------|-----|------|-------|
| 1 | Retry queue persistence | Keep entries that still have retries remaining and use the drain return value in the engine loop | `src/execution/dex/trader.rs`, `src/engine.rs` | ~20 |
| 2 | R:R appears unset | Trace risk-reward source in `src/agent` and `src/engine.rs`, then set TP/SL or suppress trade until valid | `src/agent`, `src/engine.rs` | ~25 |
| 3 | Max-positions suppression | Preserve max-position protection but log skipped candidates and reasons | `src/engine.rs` | ~10 |
| 4 | Session penalty noise | Verify session classification, then remove flat penalty text when session timing is ambiguous | `src/agent` or config | ~15 |
| 5 | Volume circuit breaker | Require minimum volume threshold before trade can be evaluated; zero-volume data must be no-trade | `src/agent` | ~12 |
| 6 | Confidence weighting | Decouple confidence from open-position metadata; score setup quality first | `src/agent/decision_parser.rs` | ~20 |

Scope is intentionally small: no redesign, no new modules.

---

## AUDIT Phase — Five-Question Check

| # | Fix | All Cases | Scale | Attacker | 2 Years | Standard | Verdict |
|---|-----|-----------|-------|----------|---------|----------|---------|
| 1 | Retry queue recovery | Yes | Yes | No | Yes | Yes | Pass |
| 2 | R:R enforcement | Yes | Yes | No | Yes | Yes | Pass |
| 3 | Max-positions audit log | Yes | Yes | No | Yes | Yes | Pass |
| 4 | Session logic cleanup | Yes | Yes | No | Yes | Yes | Pass |
| 5 | Volume gate | Yes | Yes | No | Yes | Yes | Pass |
| 6 | Confidence decoupling | Yes | Yes | No | Yes | Yes | Pass |

No new trust boundaries introduced. No secret or key handling changed.

---

## SELF-CORRECT Phase

| Issue | Correction |
|-------|-----------|
| R:R root cause unknown from logs alone | Fix stops at exact missing path; do not invent broader math redesign. |
| Confidence scoring may touch defaults | Preserve existing behavior for paper mode; only change weighting path. |
| Session behavior may be timezone-sensitive | Localize the change and avoid changing unrelated market-hours logic. |

---

## COMPLETE Phase

All six items are mapped to minimal behavior fixes with exact files and line intent. Ready for implementation review. No files were changed in this FID submission.
