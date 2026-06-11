# FID: VolRatio=0 "No Volume" Misdiagnosis + Frontend Decisions Cap

**Filename:** `FID-2026-0611-119-volratio-zero-no-volume.md`
**ID:** FID-2026-0611-119
**Severity:** critical
**Status:** fixed
**Created:** 2026-06-11 17:10
**Author:** Buffy

---

## Summary

The LLM agent is misdiagnosing nearly ALL pairs as having "no volume" because the VolRatio indicator uses only the LAST candle's volume (`volumes[n-1]`), which is frequently 0 for Kraken altcoins (no trades in the current 5m window), even though the 20-candle SMA shows healthy volume (100k+). This produces `VolRatio 0.00` for 27 of 40 evaluated pairs, causing the LLM to say "no volume — HOLD" for every single pair. Additionally, the frontend API caps decisions at 20, hiding 20 of 40 PASS decisions.

## Environment

- **OS:** Windows (win32)
- **Language/Runtime:** Rust 2021, edition 1.91
- **Version:** v0.13.5
- **Commit/State:** main branch, live run at 4:42 PM 2026-06-11
- **Model:** openrouter/owl-alpha

## Detailed Description

### Problem 1: VolRatio=0 "No Volume" (CRITICAL)

During a live run evaluating 40 pairs, **27 pairs** triggered `VolRatio=0` warnings. The LLM interpreted every one as "no volume" and issued HOLD for all 40 pairs — even pairs with strong trending setups (STG/USD ADX 37.9, SKR/USD ADX 34.8 with extreme volume spikes).

Example log output:
```
[Indicators] VolRatio=0: last_vol=0.000000, sma20=100416.548074, last_5_vols=[6706.47064, 0.0, 0.0, 29643.13279219, 0.0]
```

The SMA20 is healthy (100k+) but the last candle has volume=0, producing VolRatio=0.00.

### Problem 2: Frontend Decisions Cap (Medium)

The `get_decisions` API handler returns a maximum of **20 decisions** (10 non-PASS + 10 PASS). With 40 pairs all returning PASS, only 20 are visible on the frontend. The remaining 20 are silently dropped.

### Expected Behavior

1. VolRatio should reflect the pair's actual recent volume activity, not just the single last candle (which may be the current incomplete period or a quiet 5m window for low-liquidity pairs on Kraken).
2. The frontend should show all evaluated pairs, or at least enough to be useful for scan_all_pairs mode.

### Root Cause

**VolRatio calculation** (`src/data/indicators.rs:573-579`):
```rust
let vol_sma20: f64 = volumes[n - 20..].iter().sum::<f64>() / 20.0;
let volume_ratio = if vol_sma20 > 0.0 {
    volumes[n - 1] / vol_sma20  // <-- uses ONLY the last candle's volume
} else {
    1.0
};
```

The last candle's volume is frequently 0 because:
1. Kraken's REST API returns the current incomplete 5m candle as the last entry (volume starts at 0)
2. Even after `candles.pop()` removes the incomplete candle, the "new last" candle can have 0 volume for many altcoins (no trades in that exact 5m window on Kraken's spot market)
3. Low-liquidity tokens on Kraken frequently have 5m candles with 0 volume

**Decisions cap** (`src/api/mod.rs:279-291`):
```rust
let non_pass: Vec<_> = all.iter().rev()
    .filter(|d| d.action != "Pass").take(10).collect();
let pass: Vec<_> = all.iter().rev()
    .filter(|d| d.action == "Pass")
    .take(20_usize.saturating_sub(non_pass.len())).collect();
```

### Evidence

From live run log `logs/6-11-26_5_52pm.txt`:
- **40 PASS decisions** — zero BUY/SELL/CLOSE actions
- **27 VolRatio=0 warnings** across the evaluation
- **43 total VolRatio-related warnings** (including non-zero but low)
- LLM reasoning for STG/USD (best setup, ADX 37.9): "Marginal pass — HOLD due to portfolio risk constraints" — but the real blocker was "VolRatio 1.63" being the ONLY pair with non-zero VolRatio in the batch
- LLM reasoning for SAND/USD: "VolRatio 0.00 — no volume. No edge whatsoever. HOLD."

### Impact Assessment

#### Affected Components

- `src/data/indicators.rs` — VolRatio calculation (line 573-579)
- `src/api/mod.rs` — get_decisions handler (line 275-291)
- All LLM decisions — the model's volume assessment is wrong for ~68% of pairs

#### Risk Level

- [x] Critical: The LLM is effectively blind to volume for most pairs. Every evaluation cycle wastes 40 LLM evaluations on pairs that will always say HOLD due to a data bug. The system cannot trade.

## Proposed Solution

### Approach

**VolRatio fix:** Average the last 3 candles' volume instead of using only the last candle. This smooths out single-candle zeros while still being responsive to recent volume changes.

```rust
// Before: volumes[n - 1] / vol_sma20  (single last candle)
// After:  average of last 3 candles / vol_sma20
let recent_vol = if n >= 3 {
    (volumes[n - 1] + volumes[n - 2] + volumes[n - 3]) / 3.0
} else {
    volumes[n - 1]
};
let volume_ratio = if vol_sma20 > 0.0 {
    recent_vol / vol_sma20
} else {
    1.0
};
```

**Decisions cap fix:** Raise from 20 to 50 to accommodate scan_all_pairs mode (53 pairs max).

### Steps

1. Fix VolRatio to use 3-candle average in `src/data/indicators.rs`
2. Raise decisions cap from 20 to 50 in `src/api/mod.rs`
3. Update the VolRatio=0 warning to reflect new behavior
4. Run cargo clippy + cargo test
5. Code review

### Verification

- Run engine and verify VolRatio is non-zero for pairs with historical volume
- Check frontend shows all evaluated pairs
- Confirm LLM decisions reference actual volume data instead of "no volume"

## Perfection Loop

### Loop 1

- **RED:** (1) VolRatio uses single last candle volume — 0 for 27/40 pairs on Kraken. (2) Decisions API caps at 20 — hides 20/40 evaluated pairs. (3) `kbar_features()` has separate `vol_ratio` — potential same bug.
- **GREEN:** (1) VolRatio changed to 3-candle average with `n >= 3` guard. (2) Decisions cap raised to 50 with named constants (`MAX_DECISIONS=50`, `MAX_NON_PASS=15`). (3) Verified `kbar_features()` computes `annualized_vol` from log returns, NOT volume ratio — no fix needed.
- **AUDIT:** `cargo clippy -- -D warnings` — zero warnings. `cargo test` — all 298 pass. Code reviewer approved.
- **CHANGE DELTA:** ~20 lines changed across 2 files.

### Loop 2

- **RED:** LLM only sees VolRatio (relative) — can't distinguish 'micro-cap with low baseline volume' from 'healthy pair with quiet last candle'. Need absolute volume_sma value injected into context.
- **GREEN:** Added `volume_sma` absolute value to KBar Features line in `src/agent/context_engine.rs`. Format: `VolRatio: 0.15 (avg_vol: $102417)`. Uses `map_or_else` so when `volume_sma` is `None`, no extra text appears.
- **AUDIT:** `cargo clippy -- -D warnings` — zero warnings. `cargo test` — all 298 pass. Code reviewer approved.
- **CHANGE DELTA:** ~25 lines changed across 3 files.

## Resolution

- **Fixed By:** Buffy
- **Fixed Date:** 2026-06-11 17:30
- **Fix Description:** (1) VolRatio changed from single last candle to 3-candle average. (2) Decisions API cap raised from 20 to 50 with named constants. (3) Absolute volume_sma20 value injected into LLM KBar features context.
- **Tests Added:** No — existing tests cover indicator calculations.
- **Verified By:** cargo clippy (zero warnings), cargo test (298 pass), code reviewer approved.
- **Commit/PR:** pending

## Lessons Learned

The single-candle VolRatio is a classic "last value" indicator problem — when the last data point can be 0 (incomplete candle, no-trades window), any ratio using it produces 0 regardless of historical context. Volume indicators should always use a short moving average (3-5 candles) to be robust against individual zero-volume candles.
