# FID-190: Fractional Kelly Position Sizing

**Filename:** `FID-2026-0617-190-kelly-sizing.md`
**ID:** FID-2026-0617-190
**Severity:** high
**Status:** created
**Created:** 2026-06-17 16:15 EST
**Author:** Vera
**Parent:** FID-184

---

## Summary

Replace flat 20% per-trade position sizing with fractional Kelly (0.25x Quarter Kelly, capped at 5% per trade). Gemini Q1: "Replace the flat 20% trade size with a 0.25x fractional Kelly sizing algorithm based on calculated signal edge to manage maximum drawdowns."

---

## Problem

Current position sizer uses flat 20% per trade (FID-108/PositionSizer). This is "overbetting" for a scalping strategy where win rates are 50-60% and payoff ratios are 1-1.5. Kelly Criterion math:
- Win rate p = 0.55
- Payoff ratio b = 1.2
- Full Kelly: f* = (p(b+1) - 1) / b = (0.55 × 2.2 - 1) / 1.2 = 0.0083 = 0.83% of bankroll
- 0.25x Kelly: 0.21% of bankroll
- Current 20% is 95x the optimal Kelly fraction

---

## Proposed Solution

### Action 1: Calculate expected value per signal

**File:** `src/risk/position.rs`

**Logic:** For each trade signal:
- Estimate win probability from conviction + regime + historical
- Estimate payoff ratio from TP1 distance / SL distance
- Calculate expected value: `EV = p × payoff - (1-p) × 1`

### Action 2: Apply fractional Kelly multiplier

**File:** `src/risk/position.rs`

**Logic:**
```
kelly_fraction = (p * (b + 1) - 1) / b
position_size = kelly_fraction * 0.25  // Quarter Kelly
position_size = min(position_size, 0.05)  // Cap at 5% per trade
```

### Action 3: Adjust risk limits

**File:** `config/default.toml`

**Change:** Update `[risk]` section:
- `max_risk_per_trade = 0.05` (was 0.20)
- `kelly_multiplier = 0.25` (new)
- `kelly_min_trades = 50` (require 50+ historical trades before applying Kelly)

---

## Verification

- Position sizes match Kelly calculation
- Max 5% per trade enforced
- Quarter Kelly multiplier applied
- 50+ trade history required

---

*Vera 0.1.0 — 2026-06-17 16:15 EST — FID-190 created. Kelly sizing. Replaces flat 20% rule.*
