# FID-127: Conviction-Weighted Position Sizing (Rust)

**Filename:** `FID-2026-0612-127-conviction-weighted-sizing.md`
**ID:** FID-127
**Severity:** high
**Status:** open
**Phase:** 2 (Rust engine change — depends on FID-126 for conviction_score field)
**Created:** 2026-06-12
**Source:** Gemini Deep Research Q1 (`AI Trading Engine Rule Optimization.md` §Conviction-Weighted Sizing)

---

## Summary

Implement Rust position-sizing logic that scales with the LLM's confidence score using fractional Kelly criterion. The LLM now outputs `<confidence>` (0.0-1.0) and `<sizing_multiplier>` (0.0-1.0) per FID-126; the engine translates these into actual order size for the $30 micro-capital account.

## Background

FID-126 changes the prompt to output probabilistic confidence instead of Boolean decisions. The Rust engine currently has fixed risk tiers (`<500 = 100%, <5000 = 10%` from v0.9.1) but no mechanism to scale by LLM confidence. Without this, the LLM's confidence is just decoration — execution size is hardcoded.

## Sizing Formula (from Gemini research)

```
base_risk = 0.02                    // 2% max risk per trade
kelly_fraction = 0.5                // Half-Kelly for micro-cap safety
conviction_scaler = (confidence - 0.50) * 2.0  // 0 below 0.50, linear above
risk_amount = balance * base_risk * kelly_fraction * sizing_multiplier * conviction_scaler
```

**Worked examples (sanity checks):**
- $30 balance, confidence=0.50, sizing=1.0: scaler = 0.0 → risk = $0 → no trade (correct, threshold)
- $30 balance, confidence=0.55, sizing=1.0: scaler = 0.1 → risk = $30 * 0.02 * 0.5 * 1.0 * 0.1 = $0.03
- $30 balance, confidence=0.75, sizing=0.75: scaler = 0.5 → risk = $30 * 0.02 * 0.5 * 0.75 * 0.5 = $0.1125
- $30 balance, confidence=1.0, sizing=1.0: scaler = 1.0 → risk = $30 * 0.02 * 0.5 * 1.0 * 1.0 = $0.30
- $30 balance, confidence=0.50, sizing=1.0, gas=$1.50: $0.30 base risk minus $1.50 gas = UNECONOMIC (size-aware guard needed)

**Gas-economics guard (not in Gemini research, required for $30 micro-cap):** If `gas_estimate_usd > 0.5 * risk_amount`, the trade is uneconomic. Engine should override to PASS regardless of conviction. This prevents burning capital on gas for sub-cent positions.

**Min-notional guard:** Polygon/Arbitrum DEX min order size is typically $1-5 after slippage. If `risk_amount < $1.00`, override to PASS. This prevents dust orders that fail to execute.

## Changes

1. **`src/risk/position.rs`** — Add `conviction_score: f64` and `sizing_multiplier: f64` fields to `PositionSizingInput` (or equivalent struct). Field type: `f64` with `#[derive(Debug, Default, Clone, Copy, PartialEq)]`.
2. **`src/risk/position.rs`** — Update `calculate_position_size()` to apply the conviction scaler formula. Add unit tests for boundary cases (confidence = 0.50, 0.75, 1.0). Include gas-economics and min-notional guards.
3. **`src/agent/decision_parser.rs`** — Parse `confidence` and `sizing_multiplier` from LLM JSON output (FID-126 schema). Validate ranges (0.0-1.0), clamp out-of-range values, and apply defaults if missing: `confidence=0.5` (scaler=0, treat as PASS), `sizing_multiplier=0.5` (mid-tier).
4. **`src/engine/mod.rs`** — In the BUY execution path, pass conviction + sizing multiplier to `PositionSizer::calculate()`. Add gas-cost lookup from 0x quote response and pass to guard check.
5. **`src/agent/prompts/strategy_knowledge.md`** — Document the sizing formula so the LLM knows what multiplier to output for each setup quality tier (A+ → 1.0, B → 0.5, C → 0.25). Add note: "If the trade would be uneconomic after gas, the engine will PASS regardless of your conviction. Do not output high conviction for trades that won't clear min-notional."
6. **`src/risk/position.rs`** — Add `PositionSize::new_with_guards()` constructor that wraps `calculate_position_size()` with gas + min-notional checks. Returns `PositionSize::Refused { reason: UneconomicGas | BelowMinNotional }` for guard failures.

## Risk Tier Integration

Current code (per FID-127 background) has hardcoded risk tiers:
```
<500 = 100% balance
<5000 = 10% balance
```

The conviction-weighted formula should **multiply** the tier-derived base risk, not replace it. Final formula:
```
tier_base_risk = lookup_tier_risk(balance)  // existing logic
conviction_risk = tier_base_risk * base_risk * kelly_fraction * sizing_multiplier * conviction_scaler
```

This preserves the v0.9.1 tier logic for capital protection while adding conviction scaling. **Do not** replace tier logic — wrap it.

## Test Cases (Required)

```rust
#[test]
fn conviction_at_threshold_yields_zero() {
    // confidence = 0.50 → scaler = 0 → no risk
    let result = calculate_position_size(0.50, 1.0, 30.0);
    assert_eq!(result.risk_amount_usd, 0.0);
}

#[test]
fn conviction_below_threshold_yields_zero() {
    let result = calculate_position_size(0.30, 1.0, 30.0);
    assert_eq!(result.risk_amount_usd, 0.0);
}

#[test]
fn conviction_above_threshold_scales_linearly() {
    let r50 = calculate_position_size(0.50, 1.0, 30.0).risk_amount_usd;
    let r75 = calculate_position_size(0.75, 1.0, 30.0).risk_amount_usd;
    let r100 = calculate_position_size(1.00, 1.0, 30.0).risk_amount_usd;
    assert!((r75 - 0.5 * r100).abs() < 0.001);
    assert_eq!(r50, 0.0);
}

#[test]
fn gas_uneconomic_override() {
    // gas $1.50, risk $0.03 → refuse
    let result = calculate_position_size(0.55, 1.0, 30.0).with_gas_check(1.50);
    assert!(matches!(result, PositionSize::Refused { reason: UneconomicGas }));
}

#[test]
fn min_notional_override() {
    // $0.30 risk < $1 min → refuse
    let result = calculate_position_size(0.55, 1.0, 30.0);
    assert!(matches!(result, PositionSize::Refused { reason: BelowMinNotional }));
}

#[test]
fn sizing_multiplier_scales_proportionally() {
    let r_half = calculate_position_size(0.75, 0.5, 30.0).risk_amount_usd;
    let r_full = calculate_position_size(0.75, 1.0, 30.0).risk_amount_usd;
    assert!((r_full - 2.0 * r_half).abs() < 0.001);
}
```

## Live Engine Rollback Plan

This FID changes the live engine's position sizing math (conviction scaler + gas-economics guard + min-notional guard). If the new math causes real losses or unexpected sizing:
1. **Code rollback:** Revert `src/risk/position.rs` and `src/agent/decision_parser.rs` to the pre-FID-127 commit. The v0.9.1 tier logic (`<500 = 100%`, `<5000 = 10%`) is preserved as a fallback if needed.
2. **Tier-only fallback:** If only the conviction scaler is misbehaving, set `conviction_scaler_enabled = false` in `config/default.toml` to bypass the new math and use tier-only sizing. The LLM still emits `conviction_score` but it's ignored.
3. **Gas guard rollback:** Set `gas_economics_guard_enabled = false` in `config/canary.toml` to allow uneconomic trades. Useful for diagnosing whether the gas guard is the regression source.
4. **Diagnostic:** Compare `trade.size_usd` distribution pre/post FID-127. If sizes are uniformly smaller (gas guard blocking) or uniformly zero (conviction scaler miscalibrated), escalate to a regression FID.

## Verification

- `cargo test` — all 6 new unit tests pass
- `cargo clippy -- -D warnings` — clean
- Re-run sandbox; verify `trade.size_usd` varies proportionally to LLM-reported confidence (not just hardcoded tiers)
- A+ setups (confidence ≥ 0.85, sizing ≥ 0.85) should deploy ~100% of base risk
- B setups (confidence 0.65-0.85, sizing 0.5) should deploy ~50% of base risk
- C setups (confidence 0.50-0.65, sizing 0.25) should deploy ~25% of base risk
- **Gas-economics verification:** 0 trades in sandbox with `risk_amount < 0.5 * gas_estimate`
- **Min-notional verification:** 0 trades with `risk_amount < $1.00`

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **Gas economics ignored** — Gemini formula produces $0.03 risk on $30 balance at 0.55 confidence. After $1.50 gas, trade is uneconomic. Added gas-economics guard (gas > 0.5 * risk → PASS).
2. **Min-notional not enforced** — DEX minimums ($1-5) will reject sub-dollar orders. Added min-notional guard (risk < $1.00 → PASS).
3. **Tier logic replacement risk** — FID says "add to PositionSizingInput" but current code has hardcoded v0.9.1 tier logic. The conviction formula must MULTIPLY tier risk, not REPLACE it. Added explicit "wrap, don't replace" rule.
4. **Test cases absent** — Original FID said "add unit tests for boundary cases" but didn't specify them. Added 6 required test cases with expected values.
5. **No worked examples** — Could not sanity-check the formula. Added 5 worked examples showing 0, 0.03, 0.11, 0.30 outputs.
6. **Field type unspecified** — `f64` for both, but Rust struct should be `Copy + Clone` for cheap passes. Added derives.
7. **Parser default values undefined** — If LLM omits confidence, what's the default? Added: confidence=0.5 (treat as PASS), sizing_multiplier=0.5 (mid-tier).
8. **Prompt feedback loop missing** — LLM should know the engine's gas guard exists so it doesn't output high conviction for uneconomic trades. Added note in strategy_knowledge.md.

**Status:** All issues resolved. Ready for review.

## References

- Gemini research §Conviction-Weighted Sizing (Rust code pattern)
- FID-126: Conviction-Weighted Threshold System (provides the conviction_score field)
- FID-129: Remove Deep Asian Penalty (sandbox data may shift conviction distribution)
- Existing position sizing: `src/risk/position.rs` (v0.9.1 hardcoded tier logic must be preserved)
- v0.9.1 tier logic: `<500 = 100%, <5000 = 10%` (do not regress)
