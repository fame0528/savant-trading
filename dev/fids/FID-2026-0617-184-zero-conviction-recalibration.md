# FID-184: Zero-Conviction Strategy Recalibration

**Filename:** `FID-2026-0617-184-zero-conviction-recalibration.md`
**ID:** FID-2026-0617-184
**Severity:** high
**Status:** created
**Created:** 2026-06-17 16:05 EST
**Author:** Vera
**Parent:** FID-182

---

## Summary

Fix the zero-conviction plateau. The current strategy produces 0 trades in 16h because: (1) conviction thresholds 0.20/0.25 are too high for scalping, (2) prompt over-defaults to 0.0 on ambiguity, (3) jury regime hardcoded to Ranging. Spencer's directive: "Sniper strategy, degen trading, not institutional. Turn a penny into a nickel." Lower thresholds to 0.10-0.15, fix prompt anti-pattern, add probe position mechanism, fix jury regime bug, expand universe to 100-500 pairs.

---

## Environment

- **Commit:** `0adcc57c`
- **Files:** `src/agent/prompts/strategy_knowledge.md`, `src/agent/decision_parser.rs`, `src/engine/mod.rs:2353-2354`, `config/default.toml`

---

## Detailed Description

### Problem

96 cycles, 703 PASS, 0 BUY, 0 SELL. 87% of decisions at conviction=0.000. The strategy is not trading.

### Root Causes

1. **Conviction thresholds too high:** 0.20-0.25 is institutional-quality. For scalping (degen, penny-to-nickel), need 0.10-0.15.
2. **Prompt anti-pattern:** "If you cannot compute a conviction score, output 0.0 and select PASS" causes 87% zero rate.
3. **Jury regime hardcoded to Ranging:** `engine/mod.rs:2353-2354` — `regime = current_session.name()` immediately overridden by `MarketRegime::Ranging`.
4. **Universe too narrow:** 48 pairs. Need 100-500, especially for multi-chain.

---

## Proposed Solution

### Action 1: Lower conviction thresholds

**File:** `src/agent/decision_parser.rs:474` (or wherever `regime_threshold()` is defined)

**Change:**
```rust
// OLD
fn conviction_threshold(&self) -> f64 {
    match self {
        RegimeLabel::Trending => 0.20,
        RegimeLabel::Volatile => 0.25,
        RegimeLabel::Ranging => 0.25,
        RegimeLabel::GreyZone => 0.25,
    }
}

// NEW (scalping-tuned)
fn conviction_threshold(&self) -> f64 {
    match self {
        RegimeLabel::Trending => 0.15,
        RegimeLabel::Volatile => 0.18,
        RegimeLabel::Ranging => 0.15,  // with mandatory mean-reversion signal
        RegimeLabel::GreyZone => 0.18,
    }
}
```

**Verification:** Sandbox test with 60 scenarios, measure Buy rate increase. Target: 15-30% Buy rate (currently 10%).

### Action 2: Fix prompt anti-pattern

**File:** `src/agent/prompts/strategy_knowledge.md`

**Change:** Replace this:
```
If you cannot compute a conviction score, output 0.0 and select PASS.
```

With this:
```
If uncertain, output a low-but-nonzero conviction (0.05-0.10) with explicit uncertainty reasoning. The engine will gate against the regime threshold; do not pre-gate yourself to 0.0.
```

**Verification:** Run 16h paper mode, measure distribution of conviction outputs. Target: <30% at exactly 0.000, distribution skewed toward 0.05-0.20.

### Action 3: Fix jury regime hardcoding

**File:** `src/engine/mod.rs:2353-2354`

**Change:**
```rust
// OLD
let regime = current_session.name();
let jury_result = jp.evaluate(&cleaned, savant_trading::core::types::MarketRegime::Ranging).await;

// NEW
let regime_str = current_session.name();
let market_regime = match regime_str.as_str() {
    "Asian" | "European" => savant_trading::core::types::MarketRegime::Ranging,
    "US-EU Overlap" => savant_trading::core::types::MarketRegime::Trending,
    _ => savant_trading::core::types::MarketRegime::Ranging,
};
let jury_result = jp.evaluate(&cleaned, market_regime).await;
```

**Verification:** Log shows jury using Trending regime during US-EU overlap hours (13:00-17:00 UTC).

### Action 4: Add probe position mechanism

**File:** `src/agent/decision_parser.rs` (new code)

**Logic:** When conviction is 0.05-0.15 AND at least 1 technical indicator (RSI/EMA cross/ADX) + 1 volume signal (volume > 1.5x avg) align, allow a 0.5x sizing probe position. Cap at 1 probe per cycle, 3 per session. Track probe PnL separately in `data/probe_pnl.json`.

**Implementation:** Add `is_probe` field to `TradeDecision`. In `engine/mod.rs` execution path, if `is_probe=true`, use 0.5x sizing and log to `data/probe_pnl.json`.

**Verification:** 4h paper run, measure: (a) # of probe positions, (b) probe PnL, (c) probe stop-loss hit rate. Target: probes have positive expected value.

### Action 5: Expand universe to 100-500 pairs

**File:** `config/default.toml`, `src/data/token_discovery.rs`

**Change:** Increase pair discovery limit from 50 to 500. Add multi-chain token discovery (currently Arbitrum-only).

**Verification:** Engine log shows 100+ pairs active per cycle. Multi-chain discovery works on at least 2 chains (Anvil + simulated).

### Action 6: 4h paper-mode validation

**Action:** Spencer runs engine with all changes, 4h paper mode on Anvil.

**Verification:** 
- 5-20 trades in 4h (not 0)
- Probe positions tracked separately
- No fat-finger trades (e.g., 100% position when risk limit is 20%)
- Equity curve shows actual movement

---

## Perfection Loop

### Loop 1 (RED)

Issues:
1. Lowering thresholds might break calibration if not paired with sandbox testing. Need to run sandbox first.
2. Probe position mechanism is a new feature — needs spec.
3. Universe expansion from 50 to 500 may exceed LLM context budget. M3 has 1M context, but 500 pairs × 300 tokens = 150K, leaving 850K for prompt + response. Feasible.
4. Jury regime fix changes runtime behavior. Need to verify it doesn't break existing tests.

**CHANGE DELTA: N/A (analysis)**

### Loop 2 (GREEN)

Fixes:
1. Action 1 (lower thresholds) — 1 function change
2. Action 2 (prompt anti-pattern) — 1 paragraph change
3. Action 3 (jury regime) — 1 line change
4. Action 4 (probe mechanism) — new code, ~50 lines
5. Action 5 (universe expansion) — config change + multi-chain discovery
6. Action 6 (validation run) — Spencer's action

**CHANGE DELTA: ~10% (significant new code in Action 4)**

### Loop 3 (AUDIT)

- [x] Threshold change: needs sandbox test to verify calibration
- [x] Prompt change: needs paper-mode test to verify LLM behavior change
- [x] Jury fix: 1 line, low risk
- [ ] Probe mechanism: new code, needs grep for callers (Law 4)
- [ ] Universe expansion: needs multi-chain discovery wiring

**CALL-GRAPH REACHABILITY (Law 4):** 
- `is_probe` field on `TradeDecision`: needs to be read in `engine/mod.rs` execution path
- `data/probe_pnl.json`: needs writer + reader
- Multi-chain discovery: needs to be called from `engine/mod.rs` startup

**These will be verified after code is written.**

**CHANGE DELTA: ~5% (AUDIT notes)**

### Loop 4 (SELF-CORRECT)

The probe mechanism is the highest-risk change. Need to:
1. Define the probe criteria clearly (1 indicator + 1 volume signal = probe)
2. Define probe PnL tracking format
3. Add probe-specific risk limits (max 3 probes per session, 0.5x sizing)

**CHANGE DELTA: ~3% (refinement)**

### Loop 5 (CONVERGENCE)

Loop 1→2: 10%
Loop 2→3: 5%
Loop 3→4: 3%
Loop 4→5: 0%

**CONVERGED at Loop 5.**

---

## Resolution

- **Fixed By:** Pending
- **Fix Description:** 6 actions across prompt, thresholds, jury fix, probe mechanism, universe expansion
- **Tests Added:** Pending (sandbox test for threshold calibration)
- **Verified By:** 4h paper-mode validation run

---

*Vera 0.1.0 — 2026-06-17 16:05 EST — FID-184 created. Strategy recalibration. Awaiting Gemini research results to finalize Action 4 (probe mechanism) thresholds.*
