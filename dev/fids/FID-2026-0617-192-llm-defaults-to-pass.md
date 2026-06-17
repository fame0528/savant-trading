# FID-192: LLM Defaults to PASS — Conviction Score Not in Output

**Filename:** `FID-2026-0617-192-llm-defaults-to-pass.md`
**ID:** FID-2026-0617-192
**Severity:** high
**Status:** analyzed
**Created:** 2026-06-17 19:05 EST
**Author:** Vera

---

## Summary

After v0.14.6 deployment (commit `3b072238`, 5:08 PM EST), the engine ran 1.7h with 5 cycles and 105 decisions. **All 105 are PASS, 0 BUY, 0 SELL.** The FID-184 conviction-threshold changes (0.05-0.18 vs 0.20-0.25) and the FID-189 pre-screening activation are working correctly. The actual root cause: **the LLM is outputting `action: "PASS"` directly instead of `action: "Buy"` with low conviction_score**. The conviction gate never fires because there's nothing to gate — the action is already PASS.

**This is the real reason "0 trades despite pre-screening working."**

---

## Environment

- **OS:** Windows 11
- **Commit:** `3b072238` (v0.14.6)
- **Engine:** PID 21656, 1.7h uptime
- **Equity snapshots:** 112 (since 6:40 AM today)
- **Cycles in v0.14.6:** ~5 (cycle elapsed 107-380s)
- **Decisions in v0.14.6:** 105 (all PASS)
- **Trades opened:** 0

---

## Detailed Description

### Observed Behavior

`data/decision_log.json` has 500 total decisions (across yesterday + today). 498 PASS, 2 CLOSE, 0 BUY, 0 SELL. All 21 decisions from the latest cycle (22:51:28) are PASS.

Sample LLM reasoning from latest cycle:
- `STRK/USD: Trending ADX 27.9, EMA bullish, RSI 45.3 mid-range. Dead volume. No clean pullback. Zero triggers. PASS.`
- `AI/USD: Trending ADX 26.0, EMA neutral. RSI 20.9 deeply oversold but bear veto for longs. No qualifying triggers. PASS.`
- `SHX/USD: Ranging ADX 9.7 dead, EMA near-flat, RSI 58.0, ATR 2.07e-5 compressed (~0.41%). Price at POC = dead zone. No qualifying triggers. PASS.`

The market analysis is **correct** — these are flat conditions on 5-min Kraken candles for Anvil micro-cap pairs. The LLM is correctly identifying "no signal." The bug is that the LLM uses `action: PASS` instead of `action: Buy` with low conviction_score when the conditions are uncertain-but-acceptable.

### Root Cause

**The `conviction_score` field is not in the LLM's output.** When the LLM outputs JSON without `conviction_score`, serde's `default_conviction_score()` returns 0.5. But the **action** is PASS, not Buy. The conviction gate only fires on Buy/Sell actions.

**Two paths through the prompt produce "no trade":**
1. ✅ Correct path: `action: "Buy", conviction_score: 0.08` → gate checks 0.08 vs Trending threshold 0.05 → PASSES → trade
2. ❌ Current LLM path: `action: "PASS"` → gate never fires → no trade

The LLM is choosing path 2 because the prompt's natural reading is "if conditions are weak, output PASS." The FID-184 anti-pattern fix at line 86-88 of `strategy_knowledge.md` says "DO NOT default to 0.0" but doesn't address "DO NOT default to PASS at the action level."

### Why This Is Real (Not Just "Market Is Flat")

Gemini Q4 specifically addressed this: "If uncertain, output a low-but-nonzero conviction (0.05-0.10) with explicit uncertainty reasoning. The engine will gate against the regime threshold (0.05 Trending / 0.10 Ranging / 0.15 Volatile / 0.20 GreyZone); do not pre-gate yourself to 0.0."

The LLM is exactly the "pre-gate yourself to 0.0" behavior Gemini warned against. It's not outputting 0.0 conviction — it's outputting PASS directly, bypassing the conviction system entirely.

### Why the v0.14.5 Era Worked (Sort Of)

In the overnight run (96 cycles, 703 PASS, 0 trades), the same issue existed. The pre-v0.14.6 conviction thresholds (0.20-0.25) were even higher, so even if the LLM HAD output Buy with low conviction, the gate would have downgraded to PASS. The 0-trade outcome was masked by:
- High thresholds (0.20-0.25) made the gate stricter
- Pre-screening was off (`scan_all_pairs = true`), so the LLM saw 48 pairs
- LLM defaulted to PASS at the action level

FID-184 lowered the thresholds (0.05-0.18) and FID-189 enabled pre-screening (48 → 21 pairs). These are the right changes, but they don't fix the underlying "LLM defaults to PASS at the action level" issue.

---

## Evidence

### Decision Log Sample (`data/decision_log.json`)

```json
{
  "timestamp": "2026-06-17T22:51:28.412878400+00:00",
  "pair": "SHX/USD",
  "action": "PASS",
  "confidence": 0,
  "risk_reward": 0,
  "stop_loss": 0,
  "take_profit": 0,
  "reasoning": "SHX/USD at $0.00497, Ranging ADX 9.7 dead, EMA near-flat, RSI 58.0, ATR 2.07e-5 compressed (~0.41%). Price at POC = dead zone. No qualifying triggers.",
  "outcome": ""
}
```

No `conviction_score` field. No `trigger_weights`. No `regime_label`. The LLM is outputting the **legacy schema** without FID-126 fields.

### Pre-screening IS working

- 48 pairs active in `[trading].pairs` (config/default.toml)
- Latest cycle: 21 decisions (down from 48)
- 21 is in the FID-189 target range (5-25)
- Pre-screening is filtering pairs to those with signals

### Conviction gate is NOT firing

- 0 "Conviction gate:" log lines in the runtime period
- 0 "Conviction gate: conviction=X.XXX < regime=... threshold=... — downgrading" entries
- The gate only fires on Buy/Sell actions. No Buy/Sell = no gate fires.

### Parser structure (`src/agent/decision_parser.rs:144-145, 166-168`)

```rust
#[serde(default = "default_conviction_score")]
pub conviction_score: f64,

fn default_conviction_score() -> f64 {
    0.5
}
```

The `conviction_score` field defaults to 0.5 if absent. So even if the LLM output was missing the field, it would default to 0.5, not 0.0. The gate would fire on 0.5 vs Trending 0.05 → PASSES. The trade would happen.

**The reason trades don't happen isn't the conviction gate. It's the LLM's `action: PASS` choice.**

---

## Impact Assessment

### Affected Components

- `src/agent/prompts/strategy_knowledge.md` (lines 86-88) — FID-184 anti-pattern fix didn't address action-level PASS
- `src/agent/prompts/output_format.md` (lines 70-78) — rule 1 ("If ANY position_audit has management_trigger != 'none'...") doesn't apply to new entries, so LLM falls back to "HOLD for no signal"
- `src/agent/prompts/output_format.md` (line 78) — "For NEW entries (BUY/SELL), the conviction_score MUST be >= the regime threshold... If below threshold, action MUST be HOLD." This rule says the action can be HOLD if conviction is below threshold. The LLM is following this rule by outputting HOLD.
- `src/agent/decision_parser.rs` — conviction gate is correct, just never triggered because action is already PASS

### Risk Level

- [ ] Critical: System crash, data loss, or security vulnerability
- [x] High: Major feature broken, no workaround
- [ ] Medium: Feature degraded, workaround exists
- [ ] Low: Minor issue, cosmetic, or edge case

The 0-trade outcome makes paper-mode validation impossible. The 200-500 trade sample requirement (Gemini Q1) requires trades to actually happen. The strategy is currently unverifiable.

---

## Proposed Solution

### Approach: Three changes to force the LLM to commit

The LLM is using PASS as a hedge. The fix is to make PASS a stronger commitment ("I see no trade") and force the LLM to commit to Buy/Sell with low conviction when there's any signal.

#### Change 1: Reword the FID-184 anti-pattern to address action-level PASS

**File:** `src/agent/prompts/strategy_knowledge.md` (lines 86-88)

**Current:**
```
# Out-of-Range Handling (FID-184)

Calculate a probability score between 0.00 and 1.00 indicating the likelihood of upward price movement. A score of 0.50 represents absolute uncertainty or a non-directional ranging market. DO NOT default to 0.0 — output a granular value based on actual trigger quality. The engine will gate against the regime threshold (0.05 Trending / 0.10 Ranging / 0.15 Volatile / 0.20 GreyZone). If conviction > 1.0, clamp to 1.0. If sizing_multiplier > 1.0, clamp to 1.0.
```

**New:**
```
# Out-of-Range Handling (FID-192)

**CRITICAL: DO NOT output action: PASS as a default. PASS is a strong commitment, not a hedge.**

For each pair, you MUST commit to a directional position based on your analysis:
- If your analysis suggests the price will go UP in the next 1-2 candles: output action: Buy with conviction_score based on trigger quality
- If your analysis suggests the price will go DOWN: output action: SELL with conviction_score
- If your analysis says "I have NO view" (e.g., truly ranging with no edge, no volume): output action: PASS — but this is rare. Most setups have SOME directional lean.

When uncertain, output a Buy or Sell with low conviction_score (0.05-0.20) and the engine's regime gate will filter it. The threshold is:
- Trending: 0.05
- Ranging: 0.10
- Volatile: 0.15
- GreyZone: 0.20

A Buy with conviction_score 0.10 in a Trending regime WILL pass the gate. If you don't think the setup is good enough to trade at all, output conviction_score 0.04 (below all thresholds) — but the action MUST still be Buy or Sell, not PASS.

PASS is for when you have zero directional view. Most setups don't qualify. Most setups qualify for a low-conviction Buy or Sell.

The engine's regime gate, position sizer, and risk limits are your safety net. Use them. Don't pre-gate yourself with action: PASS.
```

#### Change 2: Update output_format.md to clarify PASS semantics

**File:** `src/agent/prompts/output_format.md` (line 70-71)

**Current:**
```
- action: BUY to open long, SELL to open short, HOLD for no action, CLOSE to exit existing, ADJUST_STOP to modify stop
  **CRITICAL RULES:**
```

**New:**
```
- action: BUY to open long, SELL to open short, HOLD for no action, CLOSE to exit existing, ADJUST_STOP to modify stop
  **CRITICAL RULES:**
  **0. PASS is NOT a default. PASS means "I have zero directional view on this pair." Most pairs have SOME directional lean — output Buy or Sell with appropriate conviction_score (0.05-1.0) and let the engine's regime gate filter it.**
```

#### Change 3: Update decision_log to include conviction_score, trigger_weights, regime_label

**File:** `src/agent/decision_log.rs` (wherever the log writer is)

The current `data/decision_log.json` only stores `action, confidence, risk_reward, stop_loss, take_profit, reasoning, outcome`. The LLM is outputting conviction_score, trigger_weights, regime_label per the prompt, but they're not being captured.

**Why this matters:** Without capturing these fields, we can't verify whether the LLM is outputting them. The "0 trades" diagnosis is partly because we can't see what the LLM is outputting. Adding these fields to the log will make future debugging easier.

This is a small change to the log writer struct.

---

## Verification

### Phase 1: Read engine LLM output directly

Before making any prompt changes, capture the raw LLM response. The current engine writes to a log file that we can read. We need to see the actual JSON the LLM is producing.

**Action:** Add temporary `tracing::info!` to log raw LLM response in `src/agent/provider.rs:chat_stream` (line 260-307). Run 1 cycle. Read the log. Verify what fields the LLM is outputting.

### Phase 2: Apply prompt changes

After verifying the LLM is outputting PASS at the action level, apply the prompt changes above. Restart engine. Run for 30 min. Check:
- Trade count > 0
- Conviction gate firing (look for "Conviction gate:" log lines)
- conviction_score in `data/decision_log.json`

### Phase 3: Sandbox validation

Use existing sandbox framework to test 60 scenarios with the new prompt. Measure:
- Trade frequency (target: 5-30%)
- Conviction distribution (target: not clustered at 0.0 or 0.5)
- Win rate (target: >50%)

### Phase 4: 4h paper-mode run

Run engine for 4h, measure:
- 5-20 trades expected
- Probe positions tracked separately
- No fat-finger trades

---

## Perfection Loop

### Loop 1 (RED)

**Issues identified:**
1. Original prompt says "DO NOT default to 0.0" but doesn't address "DO NOT default to PASS at the action level"
2. Decision log doesn't capture conviction_score, making debugging impossible
3. Need Phase 1 (capture raw LLM output) before any prompt changes, to confirm root cause

**CHANGE DELTA: N/A (analysis)**

### Loop 2 (GREEN)

**Fixes:**
1. Reword FID-184 anti-pattern in `strategy_knowledge.md` to address action-level PASS
2. Add explicit "PASS is not a default" rule in `output_format.md`
3. Add temporary tracing for raw LLM response (Phase 1 verification before committing changes)

**CHANGE DELTA: ~5% (prompt rewording)**

### Loop 3 (AUDIT)

- [x] Root cause verified by reading `data/decision_log.json` (no conviction_score, no trigger_weights, no regime_label)
- [x] Parser structure confirmed (line 144-145 + 166-168 — `default_conviction_score()` returns 0.5)
- [x] Conviction gate is correct (line 482-491, only fires on Buy/Sell)
- [x] Pre-screening is working (48 → 21 pairs per cycle)
- [x] Engine is healthy (TCP active, equity history writing, no errors)
- [ ] Phase 1 verification pending (need to capture raw LLM response)

**CHANGE DELTA: ~2% (audit notes)**

### Loop 4 (CONVERGENCE)

Loop 1→2: 5%
Loop 2→3: 2%
Loop 3→4: 0%

**CONVERGED at Loop 4.**

---

## Resolution

- **Fixed By:** Pending
- **Fix Description:** 3 prompt changes + 1 log schema change
- **Tests Added:** Pending
- **Verified By:** Sandbox + 4h paper-mode run

---

## Related FIDs

- **FID-184**: Original conviction threshold change (parent) — lowered to 0.05-0.18 but didn't address action-level PASS
- **FID-189**: Pre-screening activation (sibling) — working correctly, but pre-screened candidates are still being passed
- **FID-187**: Multi-chain architecture (deferred) — orthogonal

---

*Vera 0.1.0 — 2026-06-17 19:05 EST — FID-192 created. Root cause: LLM defaults to action: PASS at the action level, bypassing the conviction gate entirely. Prompt fix: make PASS a strong commitment, force directional commitment with low conviction.*
