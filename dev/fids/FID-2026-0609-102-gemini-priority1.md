# FID-2026-0609-102: Gemini Research Priority 1 — ATR TP, BB Squeeze, Dynamic ADX

**Filename:** `FID-2026-0609-102-gemini-priority1.md`
**ID:** FID-2026-0609-102
**Severity:** high
**Status:** created
**Created:** 2026-06-09 16:15
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

Three Priority 1 fixes from Gemini Deep Research: (1) engine-side TP2/TP3 generation from ATR so scale-out targets are structurally sound, (2) Bollinger Band Squeeze detection as pre-filter signal for bear market breakout detection, (3) dynamic ADX threshold scaling based on Fear & Greed index.

---

## Fix 1: Engine-Side TP2/TP3 from ATR (High)

### Problem

LLM picks symmetric round-number TPs (e.g., TP1=18.82, TP2=19.12, TP3=19.42 — all $0.30 apart). These don't reflect market structure or volatility.

### Gemini Recommendation

"Remove TP from the LLM entirely. The engine should compute TP from ATR after the LLM provides direction + stop."

### Our Approach (Hybrid)

Keep LLM's TP1 (with existing FID-101 auto-adjust for R:R). ALWAYS compute TP2/TP3 from ATR:

- TP2 = TP1 + ATR * 1.0 (Long) / TP1 - ATR * 1.0 (Short)
- TP3 = TP1 + ATR * 2.0 (Long) / TP1 - ATR * 2.0 (Short)
- Fallback if ATR is None: TP2 = TP1 + risk, TP3 = TP1 + risk * 2

This ensures scale-out targets reflect actual market volatility.

### Location

`src/engine.rs` BUY path, after FID-101 R:R auto-adjust block, before sizer call.

### Scope

~15 lines

---

## Fix 2: Bollinger Band Squeeze Pre-Filter (High)

### Problem

Current pre-scoring uses RSI, ADX, EMA cross, volume spike. In bear markets, none fire for most pairs. Need a volatility compression signal.

### Gemini Recommendation

"Bollinger Band Squeeze: BB inside Keltner Channels = volatility compression = precursor to breakout."

### Implementation

Compute in pre-scoring section using last 20 closes from candle_data:

- BB upper = SMA(20) + 2 * stddev(20)
- BB lower = SMA(20) - 2 * stddev(20)
- Keltner upper = EMA(20) + 1.5 * ATR(14)
- Keltner lower = EMA(20) - 1.5 * ATR(14)
- Squeeze = BB_upper < Keltner_upper AND BB_lower > Keltner_lower

### Location

`src/engine.rs` pre-scoring section (line ~1739)

### Scope

~25 lines

---

## Fix 3: Dynamic ADX Scaling (Medium)

### Problem

Static ADX threshold of 20 doesn't adapt to market regime. At F&G=10, even ADX 18 pairs may have setups worth evaluating.

### Gemini Recommendation

"ADX threshold should be inversely correlated to the Fear & Greed index using a linear scaling function."

### Implementation

```rust
let fg = insight.cached().sentiment.fear_greed_index.unwrap_or(50) as f64;
let adx_threshold = 25.0 - ((50.0 - fg).max(0.0) / 30.0 * 7.0);
let adx_threshold = adx_threshold.clamp(18.0, 25.0);
```

- F&G = 50 → ADX threshold = 25 (normal)
- F&G = 35 → ADX threshold = 21.5
- F&G = 20 → ADX threshold = 18 (minimum)
- F&G = 10 → ADX threshold = 18 (clamped)

### Location

`src/engine.rs` pre-scoring section

### Scope

~5 lines

---

## Perfection Loop

### RED

All 3 issues traced with exact root causes and Gemini recommendations.

### GREEN

Fix 1: ~15 lines in engine.rs BUY path
Fix 2: ~25 lines in engine.rs pre-scoring
Fix 3: ~5 lines in engine.rs pre-scoring

### AUDIT

| Check | Result |
|-------|--------|
| ATR available in scope | ✅ `indicators.atr` (Option<f64>) |
| Fear/Greed available | ✅ `insight.cached().sentiment.fear_greed_index` |
| 20 closes available | ✅ `candle_data` has 200 candles |
| SMA/stddev computation | ✅ Manual computation from closes slice |
| EMA(20) computation | ✅ Can use existing EMA or compute inline |
| Fallback for ATR=None | ✅ Use risk * multiplier |
| Change delta | ~45 lines, <0.7% of engine.rs |

### SELF-CORRECT

No issues found. BB squeeze computation is straightforward arithmetic on existing data.

### COMPLETE

All checks pass. Ready for implementation.

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all pass
3. Runtime: TP2/TP3 should reflect ATR-based distances (not symmetric)
4. Runtime: More pairs should pass pre-filter in bear market (BB squeeze + dynamic ADX)
5. Runtime: Log should show `FID-102 TP2/TP3 computed from ATR` entries
