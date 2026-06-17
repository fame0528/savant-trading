# FID-184: Zero-Conviction Strategy Recalibration (Updated with Gemini Research)

**Filename:** `FID-2026-0617-184-zero-conviction-recalibration.md`
**ID:** FID-2026-0617-184
**Severity:** high
**Status:** analyzed
**Created:** 2026-06-17 16:05 EST
**Updated:** 2026-06-17 16:00 EST (with Gemini research results)
**Author:** Vera
**Parent:** FID-182

---

## Summary

Fix the zero-conviction plateau. Updated with Gemini deep research (2026-06-17 15:58 EST). Key changes from original FID:

1. **Lower conviction thresholds further** — 0.05-0.15 dynamic by regime (was 0.10-0.15)
2. **Add 2-stage pre-screening** — filter 100-500 pairs to top 5-25 BEFORE LLM (new)
3. **Replace "output 0.0" prompt with raw probability** — 0.50 = neutral, engine gates (was remove "0.0" instruction, now full rewrite)
4. **Fractional Kelly sizing** — 0.25x Kelly multiplier, max 5% per trade (new, was flat 20%)
5. **Switch data source from Kraken CEX to on-chain AMM** — for DEX execution (moved to FID-188)
6. **Add cognitive slippage penalty** — penalize PnL by latency × volatility (new)
7. **Statistical sample requirement** — 200-500 executed trades minimum, not 4h run (new, was 4h)

---

## Environment

- **Commit:** `0adcc57c`
- **Gemini source:** `prompts/prompt-results/DEX Crypto Scalping Engine Optimization.md`
- **Files:** `src/agent/prompts/strategy_knowledge.md`, `src/agent/decision_parser.rs`, `src/engine/mod.rs:2353-2354`, `config/default.toml`

---

## Detailed Description

### Problem (Updated)

96 cycles, 703 PASS, 0 BUY, 0 SELL. 87% of decisions at conviction=0.000. Gemini research identifies this as a **default-to-hold bias** induced by the prompt's "output 0.0 if ambiguous" instruction (Gemini Q4). The 0.20-0.25 thresholds are "mathematically prohibitive" for scalping (Gemini Q1).

### Root Causes (Updated with Gemini)

1. **Prompt anti-pattern:** "If you cannot compute a conviction score, output 0.0 and select PASS" causes 87% zero rate. Gemini: "fatal prompt engineering anti-pattern in quantitative algorithmic trading." Fix: rewrite to raw probability (0.5 = neutral).
2. **Conviction thresholds too high:** 0.20-0.25 is institutional. For scalping: 0.05-0.15 dynamic by regime. Gemini: "The static execution gate in the trading engine must be replaced with a dynamic threshold matrix linked to real-time volatility metrics."
3. **No pre-screening:** 100-500 pairs passed directly to LLM is "architectural flaw that guarantees systemic degradation of signal resolution." Gemini: "stop passing 30-50 pairs directly to the LLM."
4. **Jury architecture wrong:** 9-model global jury, once per cycle. Gemini: "Downsize the jury from 9 generic models to 3 specialized LLM calls... per-pair evaluation... 2/3 execution quorum."
5. **Data source wrong:** Kraken CEX WebSocket for Arbitrum DEX execution. Gemini: "Halt the use of Kraken CEX data for Arbitrum trading. Query real AMM liquidity depth."
6. **Position sizing wrong:** Flat 20% per trade. Gemini: "Replace the flat 20% trade size with a 0.25x fractional Kelly sizing algorithm."
7. **No cognitive slippage modeling:** LLM takes seconds to "think," price drifts. Gemini: "Subtract this penalty from the paper PnL to reflect real-world execution decay."
8. **Jury regime hardcoded to Ranging:** Bug at `engine/mod.rs:2353-2354`.
9. **Statistical sample too small:** 96 cycles is not enough. Gemini: "200-500 executed trades minimum, 2-4 weeks live incubation."

---

## Proposed Solution (Updated with Gemini)

### Action 1: Dynamic conviction thresholds (lower)

**File:** `src/agent/decision_parser.rs` (where `regime_threshold()` is defined)

**Gemini-recommended thresholds:**
| Regime | Old | New (Gemini) | Kelly Fraction |
|--------|----:|-------------:|---------------:|
| Trending | 0.20 | 0.05-0.08 | 0.25x (Quarter) |
| Ranging | 0.25 | 0.10-0.12 | 0.15x |
| Volatile | 0.25 | 0.15-0.18 | 0.10x (Tenth) |
| GreyZone | 0.25 | 0.20+ (Default to PASS) | 0.00x |

**Implementation:** Dynamic threshold based on rolling ATR and Hurst exponent.

### Action 2: Rewrite prompt to use raw probability

**File:** `src/agent/prompts/strategy_knowledge.md`

**Replace:**
```
If you cannot compute a conviction score, output 0.0 and select PASS.
```

**With (Gemini's recommendation):**
```
Calculate a probability score between 0.00 and 1.00 indicating the likelihood of upward price movement. A score of 0.50 represents absolute uncertainty or a non-directional ranging market.
```

**Engine gates (Gemini's table):**
| LLM Score | Engine Action |
|-----------|---------------|
| 0.00-0.35 | SHORT or SELL |
| 0.36-0.49 | PASS (mild bearish) |
| 0.50 | PASS (perfect ambiguity) |
| 0.51-0.64 | PASS (mild bullish) |
| 0.65-1.00 | LONG or BUY |

### Action 3: Add Stage 1 deterministic pre-screening (NEW from Gemini)

**File:** `src/engine/mod.rs` (new stage before LLM call)

**Logic:** Filter 100-500 pairs to top 5-25 based on:
- Volume spikes (volume > 1.5x rolling avg)
- Volatility breakouts (ATR breakout)
- Liquidity depth (>$50K within 1% spread via 0x API quote)
- Regime disambiguation (clear Trending vs Ranging signal)

**Implementation:** Use NumPy-style logic (or Rust equivalent) to compute scores, rank pairs, pass top 5-25 to LLM.

**New FID-189** (separate, to be created) for this.

### Action 4: Fractional Kelly sizing (NEW from Gemini)

**File:** `src/risk/position.rs`

**Logic:** Replace flat 20% per trade with 0.25x Kelly multiplier:
- Calculate expected value of signal
- Apply 0.25x Kelly fraction
- Cap maximum absolute exposure at 5% of portfolio per trade

**New FID-190** (separate, to be created) for this.

### Action 5: Add cognitive slippage penalty (NEW from Gemini)

**File:** `src/engine/mod.rs` (in equity snapshot logic)

**Logic:** 
```
Penalty = Price × (Vol/sec) × LLM_Latency
Subtract penalty from paper PnL
```

**Implementation:** Track LLM call latency per cycle. Apply penalty to equity snapshot.

### Action 6: Fix jury regime hardcoding

**File:** `src/engine/mod.rs:2353-2354`

**Change:** Map session name to market regime (Asian/European → Ranging, US-EU Overlap → Trending).

### Action 7: Restructure jury to 3-model per-pair (FID-184, with FID-189)

**File:** `src/agent/jury/`

**Gemini's recommendation:** 3 specialized agents (Momentum, Contrarian, Risk Manager), per-pair evaluation, 2/3 quorum.

**Implementation:** New jury architecture. See FID-189 for pre-screening integration.

### Action 8: Switch data source from Kraken CEX to on-chain AMM (NEW from Gemini)

**File:** `src/data/`

**Gemini's recommendation:** "Halt the use of Kraken CEX data for Arbitrum trading. Query real AMM liquidity depth."

**New FID-188** (separate, to be created) for this.

### Action 9: Statistical sample validation (UPDATED from Gemini)

**Change:** 4h paper run → 200-500 executed trades minimum, 2-4 weeks live incubation.

**Verification:**
- Paper mode runs until 200+ executed trades
- Track win rate, PnL distribution, max drawdown
- Compare against backtest results
- If 200+ trades with positive expected value → consider live
- If <200 trades in 4 weeks → strategy not finding opportunities, expand universe further

---

## Verification (Updated)

### Phase 1: Unit tests
- Test dynamic threshold calculation (ATR + Hurst)
- Test prompt produces distribution of scores (not 87% at 0)
- Test pre-screening filters 500→25
- Test Kelly sizing calculation
- Test cognitive slippage penalty

### Phase 2: Sandbox test
- Run 60-scenario sandbox with new thresholds + Kelly sizing
- Target: 15-30% Buy rate (currently 0%)

### Phase 3: Paper mode (extended)
- Run until 200+ executed trades
- Track: win rate, PnL, drawdown, slippage
- Compare against Gemini's expected metrics

### Phase 4: Live incubation (2-4 weeks)
- $50 → $500 → $5000 → $50000 as edge proves
- Monitor: real RPC latency, gas spikes, MEV, LVR

---

## Perfection Loop

### Loop 1 (RED)

Issues:
1. Scope is now much larger (3 new FIDs: 188, 189, 190)
2. Pre-screening is a new major stage in the pipeline
3. Kelly sizing replaces existing position sizer
4. Data source change is high-risk (could break 48-pair current setup)
5. Statistical sample requirement extends validation timeline

**CHANGE DELTA: ~40% (3 new FIDs added, major scope expansion)**

### Loop 2 (GREEN)

Fixes:
1. Split into 3 new FIDs: FID-188 (on-chain data), FID-189 (pre-screening), FID-190 (Kelly sizing)
2. Keep Actions 1, 2, 5, 6 in this FID (conviction thresholds, prompt, cognitive slippage, jury regime fix)
3. Reference FID-189 in Action 3 (pre-screening) and FID-190 in Action 4 (Kelly)

**CHANGE DELTA: ~10% (scope split)**

### Loop 3 (AUDIT)

- [x] Gemini citations included (Q1, Q4, Q5, Q7)
- [x] Threshold numbers match Gemini's table
- [x] Prompt rewrite matches Gemini's recommendation
- [ ] CALL-GRAPH REACHABILITY: All new code paths need verification
- [ ] Pre-screening integration with LLM call: needs verification
- [ ] Kelly sizing integration with execution: needs verification

**CHANGE DELTA: ~5% (AUDIT)**

### Loop 4 (SELF-CORRECT)

The 200-500 trade sample requirement changes the validation timeline significantly. 4h run is insufficient. Need to communicate this to Spencer.

**CHANGE DELTA: ~3% (timeline adjustment)**

### Loop 5 (CONVERGENCE)

Loop 1→2: 10%
Loop 2→3: 5%
Loop 3→4: 3%
Loop 4→5: 0%

**CONVERGED at Loop 5.**

---

## Resolution

- **Fixed By:** Pending
- **Fix Description:** Dynamic thresholds + prompt rewrite + cognitive slippage + jury regime fix (Actions 1, 2, 5, 6)
- **Cross-references:** FID-188 (on-chain data), FID-189 (pre-screening), FID-190 (Kelly sizing)
- **Tests Added:** Pending
- **Verified By:** Paper mode until 200+ executed trades, then sandbox + live incubation

---

## Related FIDs

- **FID-188** (to be created): Switch data source from Kraken CEX to on-chain AMM
- **FID-189** (to be created): Stage 1 deterministic pre-screening
- **FID-190** (to be created): Fractional Kelly position sizing

---

*Vera 0.1.0 — 2026-06-17 16:00 EST — FID-184 updated with Gemini research. Scope expanded to include 3 new FIDs. Conviction thresholds lowered further. 200-500 trade sample requirement replaces 4h run.*
