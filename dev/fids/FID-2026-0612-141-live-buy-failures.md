# FID-141: Live Buy Failures — Dashboard Sort + 0x Liquidity Gate

**Created:** 2026-06-12 23:15 | **Severity:** high | **Status:** GREEN (Implemented)

## RED Phase — Issue Catalog

### Symptom 1: AI Decisions Panel Unsorted
Dashboard shows decisions in parse order (batch JSON array order), not ranked by conviction %. PUMP/USD at 50% appears above GIGA/USD at 55%. Users can't find the highest-conviction trades at a glance.

**Root cause:** `page.tsx` line 596: `decisions.map((d, i) => {...})` — no `.sort()` before mapping.

### Symptom 2: Buy Signals But No On-Chain Trades
Two cycles ran live with M3. Cycle 1 produced 2 Buy signals (GIGA/USD 55%, PUMP/USD 50%). Both failed at the 0x liquidity check:

```
[LIQUIDITY] GIGA/USD — No DEX liquidity available (0x /price returned false)
[LIQUIDITY] PUMP/USD — No DEX liquidity available (0x /price returned false)
```

Both are micro-cap Arbitrum tokens that have no 0x routing. The engine correctly rejects them, but the dashboard shows "BUY" with a confidence bar and no indication the trade was rejected. **User sees BUY signals and expects trades to execute — no rejection feedback in the UI.**

**Root cause:** 0x `/price` returns `liquidityAvailable: false` for these tokens. The engine rejects the buy (correct), but the dashboard still shows the raw BUY decision from the parser with no rejection annotation.

### Additional Finding: Cycle 2 — Zero Buys
Cycle 2 has ZERO Buy signals. M3 emitted Pass for every pair (conviction=0.000 across the board). This is FID-140's "default-to-hold" bias manifesting in live trading — after the prompt unification, M3 self-censors at ANY threshold.

| Cycle | Buys | Passes | Parse Rate | Latency |
|---|---|---|---|---|
| 1 | 2 (GIGA, PUMP) | 32 | 20/34 | 45s |
| 2 | 0 | 34 | 24/34 | 60s |

## GREEN Phase — Proposed Fixes

### Fix 1: Sort decisions by confidence % descending (dashboard)
`page.tsx` line 596 — sort the decisions array before mapping:
```tsx
[...decisions].sort((a, b) => b.confidence - a.confidence).map((d, i) => { ... })
```
One line, no data changes.

### Fix 2: Show liquidity rejection status in AI Decisions (dashboard)
When a decision was rejected by 0x liquidity check, show a red tag instead of just the action badge. The engine currently logs `[LIQUIDITY]` to the activity feed but doesn't annotate the DecisionRecord. 

**Option A (engine):** Annotate `DecisionRecord` with a `rejected: Option<String>` field. Push to shared state. Dashboard reads it.
**Option B (dashboard):** Read activity log for `LIQUIDITY` entries matching the pair name. Flash a `` tag if found.
**Option C (engine):** Add the rejection reason to the decision's `reasoning` field so it shows inline in the dashboard.

Recommendation: **Option C** — simplest, no new fields. Append "❌ REJECTED: [reason]" to the decision reasoning before pushing to shared state. Dashboard shows it inline with no code changes.

### Fix 3: M3 Zero-Buys on Cycle 2 (FID-140 follow-up)
This is the FID-140 structural issue — M3 self-censors. Not fixed here. Separate FID.

## Questions I Should Have Asked / Improvements

1. **Should we filter out tokens that have no 0x liquidity from the evaluation queue?** If PUMP and GIGA can never trade, M3 is wasting token budget evaluating them every cycle. The engine could pre-check 0x `/price` and skip pairs with no liquidity.

2. **Should the dashboard show a "Rejected" badge on decisions that failed execution?** Currently the decision shows as BUY with confidence bar, but the trade never executed. Users need to know the outcome.

3. **Should we add a "Last Trade Outcome" column to AI Decisions?** Shows whether the last BUY for a pair actually executed or was rejected, with the reason.

## AUDIT Phase

- [ ] Dashboard sort: verify decisions render in descending % order
- [ ] Liquidity annotation: verify rejected buys show inline in dashboard
- [ ] `cargo check` + `npm run build` (dashboard)
- [ ] Code review
