# FID-2026-0609-101: R:R Auto-Adjust + Bear Market Pre-Scoring Filter

**Filename:** `FID-2026-0609-101-rr-auto-adjust-bear-filter.md`
**ID:** FID-2026-0609-101
**Severity:** high
**Status:** created
**Created:** 2026-06-09 15:55
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

Two fixes to unblock trade execution: (1) auto-extend TP to meet minimum R:R instead of rejecting, (2) lower pre-scoring ADX threshold and add volume spike signal for bear market conditions.

---

## Fix 1: R:R Auto-Adjust (High)

### Problem

LLM proposes symmetric stop/TP distances (e.g., entry=18.52, stop=18.22, tp=18.82 → R:R=1.0). Position sizer requires 1.2:1 at $25 balance. Trade rejected despite correct directional analysis (ADX 46.8, strong trend).

### Root Cause

The LLM uses round-number price increments rather than computing R:R from market structure. It picks symmetric distances ($0.30 stop, $0.30 TP) without considering that reward should exceed risk.

### Fix

After the R:R override (engine.rs ~3025) and before the sizer call (~3030), auto-extend TP to meet minimum R:R:

```rust
// If actual R:R < min_rr, extend TP to meet minimum
if risk > 0.0 && actual_rr < effective_min_rr {
    let required_reward = risk * effective_min_rr;
    decision.take_profit_1 = match decision.side {
        Side::Long => decision.entry_price + required_reward,
        Side::Short => decision.entry_price - required_reward,
    };
}
```

**Example:** entry=18.52, stop=18.22, risk=0.30, min_rr=1.2 → required_reward=0.36 → TP=18.88 (was 18.82). Difference: $0.06.

### Scope

~15 lines in `src/engine.rs` BUY path

---

## Fix 2: Bear Market Pre-Scoring Filter (High)

### Problem

Pre-scoring filter skips pairs where RSI is 30-70 AND ADX < 25 AND no EMA cross. At Fear & Greed = 10 (Extreme Fear), most pairs are ranging. Result: 21/30 pairs skipped, only 9 reach LLM.

### Fix

Two changes to the pre-scoring filter (engine.rs:1739-1756):

1. **Lower ADX threshold from 25.0 to 20.0** — matches `adx_ranging_threshold` in config. Pairs with ADX 20-25 (transitioning) now trigger evaluation.

2. **Add volume spike as 4th signal** — `volume_spike = candle.volume > indicators.volume_sma * 1.5`. Volume spikes in ranging markets often precede breakouts.

### Scope

~5 lines in `src/engine.rs` pre-scoring section

---

## Perfection Loop

### RED

Both issues traced to root cause with exact line numbers.

### GREEN

Fix 1: ~15 lines after R:R override, before sizer call.
Fix 2: ~5 lines in pre-scoring filter.

### AUDIT

| Check | Result |
|-------|--------|
| Fix 1 variables in scope | ✅ config.risk.*, decision.* all accessible |
| Fix 1 edge case: tight stop | ✅ Sizer validates stop direction. Auto-extend TP is correct. |
| Fix 2 volume_sma exists | ✅ IndicatorValues has volume_sma field (indicators.rs:310) |
| Fix 2 ADX 20 threshold | ✅ Matches config adx_ranging_threshold (default.toml:139) |
| Change delta | ~20 lines, <0.3% of engine.rs |

### COMPLETE

All checks pass. Ready for implementation.

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all pass
3. Runtime: COMP/USD BUY with R:R=1.0 should auto-adjust TP and pass sizer
4. Runtime: More pairs should reach LLM in bear market conditions
