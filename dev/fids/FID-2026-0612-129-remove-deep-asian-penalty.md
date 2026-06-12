# FID-129: Remove Deep Asian Session Penalty

**Filename:** `FID-2026-0612-129-remove-deep-asian-penalty.md`
**ID:** FID-129
**Severity:** medium
**Status:** open
**Phase:** 3 (Risk + Config change)
**Created:** 2026-06-12
**Source:** Gemini Deep Research Q2 (`AI Trading Engine Rule Optimization.md` §Sandbox Data Design — Deep Asian section)

---

## Summary

Remove the deterministic "Deep Asian session" penalty from the LLM prompt and risk layer. Arbitrum is a 24/7 global DEX with continuous liquidity — the "42% less depth" heuristic from traditional finance (Tokyo session) does not apply. The penalty is artificially suppressing trades in scenarios that would otherwise pass.

## Background

The current prompt tells the LLM: "Deep Asian session (02:00-05:59 UTC) — 42% less order book depth, breakout confidence penalty 75%, size multiplier 0.7x." This is true for traditional FX/equity markets (Tokyo session is genuinely thin) but **not** for crypto DEXs:
- Arbitrum is global and continuous
- $30 account size means slippage, not depth, is the binding constraint
- "Deep Asian" scenarios get penalized regardless of actual market data

In the M3 sandbox, **every** scenario was in the "Deep Asian" time window, so the penalty was applied to 100% of trades — one of the four compounding filters that drove Buy actions to 0.

## Audit Scope (Where to Find Deep Asian References)

The "Deep Asian session" penalty may be in any of these locations:
- `src/agent/prompts/strategy_knowledge.md` (LLM prompt)
- `src/agent/prompts/soul.md` (LLM personality)
- `src/agent/prompts/risk_constraints.md` (LLM risk layer)
- `config/default.toml` (`session_penalty_deep_asian` field)
- `config/canary.toml` (canary config)
- `src/risk/position.rs` (position sizing multiplier)
- `src/sandbox/scenarios.rs` (scenario time stamps)
- `knowledge/*.json` (KU mentions)

**Search regex:** `(?i)(deep.{0,3}asian|asian.{0,3}session|tokyo.{0,3}session|02:00.{0,3}05:59|42%.{0,3}depth)`

## Changes

1. **`src/agent/prompts/strategy_knowledge.md`** — Remove or rewrite the "Deep Asian session" section. Replace with: "Crypto markets trade 24/7. While volume may contract outside EU-US overlap, Arbitrum liquidity is sufficient for $30 micro-capital accounts. Do not penalize time-of-day for DEX execution."
2. **`src/agent/prompts/soul.md`** — Search for any "Deep Asian" language and remove.
3. **`src/agent/prompts/risk_constraints.md`** — Search for any "Deep Asian" language and remove.
4. **`config/default.toml`** — Set `session_penalty_deep_asian = 1.0` (neutralize). If audit finds no other code uses the field, remove it entirely and update `config/canary.toml` to match.
5. **`src/risk/position.rs`** — Audit for any deep-asian multiplier in position sizing. If present, neutralize to 1.0.
6. **`src/sandbox/scenarios.rs`** — Update scenario time stamps to span a full 24-hour cycle: distribute 60 scenarios across 24 hours with clustering around EU-US overlap (13:00-21:00 UTC) at 2x density. This ensures the LLM sees time-of-day variance rather than learning "always Deep Asian = always bad."
7. **`knowledge/*.json`** — Audit KUs for "Deep Asian" mentions. Rewrite to be chain-agnostic (e.g., "Consider that DEX liquidity may vary by time of day; check current on-chain volume").

## Soft Liquidity Replacement (Not Just Removal)

Removing the hard penalty entirely may over-correct. Replace with a **soft liquidity signal** that respects data:
- New prompt section: "If Arbitrum 24h DEX volume is below 30-day average, consider reducing size by 20%. This is informational, not a hard veto — judge based on actual market data, not time-of-day."
- This is a "size modifier" (FID-127), not a "session gate." It activates only when data confirms low liquidity, not based on clock time.

## Live Engine Rollback Plan

This FID changes prompt + risk config for the LIVE engine ($30 micro-capital). If a real loss occurs from the removed penalty:
1. **Immediate rollback:** Set `session_penalty_deep_asian = 0.5` in `config/canary.toml` to re-introduce a softer version
2. **Diagnostic:** Check the `data/journal/` for whether the loss correlated with a non-EU-US time window
3. **Long-term:** If time-of-day correlation is statistically significant after 30+ trades, reintroduce a *data-driven* penalty (not time-of-day)

## Deprecation Notice for FID-108

FID-108 introduced the session penalty. This FID reverses it. **Action item:** Add a note at the top of FID-108 saying "Superseded by FID-129 (2026-06-12). Session penalty removed for crypto DEX context."

## Verification

- `cargo check` and `cargo clippy -- -D warnings` pass
- `grep -r "deep.asian\|Deep Asian\|02:00.*05:59\|42%.*depth" src/ config/ knowledge/` returns 0 matches (or only soft-liquidity mentions)
- Re-run sandbox. Verify:
  - "Deep Asian" or "session penalty" no longer appears in LLM reasoning (parse raw responses)
  - Position sizing is identical across time-of-day scenarios (mean size within ±10% across 4 time windows: 02-06, 08-12, 14-18, 20-24 UTC)
  - At least 1 scenario in a non-Deep-Asian window (e.g., 14:00 UTC) produces a Buy
  - Time-stamp distribution: 60 scenarios distributed across 24 hours, with EU-US overlap at 2x density

## Verification

- `cargo check` and `cargo clippy -- -D warnings` pass
- Re-run sandbox. Verify:
  - "Deep Asian" or "session penalty" no longer appears in LLM reasoning
  - Position sizing is identical across time-of-day scenarios
  - At least one scenario in a non-Deep-Asian window produces a Buy (if it should have)

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **Audit scope was underspecified** — "Audit for any deep-asian multiplier" was vague. Added explicit file list + search regex covering prompts, config, risk layer, KUs.
2. **Hard removal may over-correct** — Arbitrum 24/7 doesn't mean liquidity is constant. Added soft liquidity replacement (data-driven, not clock-driven).
3. **No live-engine rollback plan** — This changes the live engine. If a real loss occurs, no mitigation path was defined. Added 3-step rollback (canary config → diagnostic → long-term).
4. **No deprecation notice for FID-108** — The original introducing FID should reference the reversal. Added explicit action item.
5. **Time-stamp distribution underspecified** — "Span a full 24-hour cycle" doesn't say how. Added distribution rule: 2x density in EU-US overlap (13-21 UTC), 1x elsewhere.
6. **canary.toml not mentioned** — Default config + canary config are separate. Both need the change.
7. **No verification check for prompt reasoning** — Just "no longer in reasoning" is weak. Added: parse raw responses, check position sizing is time-of-day-invariant within ±10% across 4 time windows.
8. **KU audit not in scope** — KUs can contain "Deep Asian" language too. Added to audit list.

**Status:** All issues resolved. Ready for review.

## References

- Gemini research §Sandbox Data Design: "Empirical data from continuous perpetual protocols indicates that while volume may contract outside the EU-US overlap, base liquidity remains entirely sufficient for a $30 micro-capital account"
- FID-128: Sandbox Jump-Diffusion Data (companion sandbox fix — remove artificial session constraint)
- FID-108: DEX Execution Reliability (introduced the session penalty in v0.12.9 — this FID REVERSES it; add deprecation note to FID-108)
- FID-126: Conviction-Weighted Thresholds (provides trigger weights that the soft liquidity modifier feeds into)
