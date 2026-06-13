# FID-140: Prompt Threshold Inconsistency — M3 Reads Stale Values, Produces 81% Pass Rate

**Created:** 2026-06-12 23:05 | **Severity:** critical | **Status:** Created

## RED Phase — Issue Catalog

### Symptom
Sandbox run `2026-06-12_22-58-59`: 43/60 passed, 17/60 failed (28% failure rate). But the failure rate doesn't tell the real story: **50/60 scenarios = Pass (81%)**, only 9 Buys + 1 Sell. Target BUY rate is 15-30%. The model is self-censoring.

### Root Cause: Three Contradictory Threshold Tables in One Prompt

`src/agent/prompts/strategy_knowledge.md` contains **three different sets of conviction thresholds**, and M3 reads all of them:

| Location in Prompt | Trending | Volatile | Ranging | GreyZone |
|---|---|---|---|---|
| **Matrix table (top)** | 0.30 | 0.40 | 0.40 | 0.40 |
| **REGIME-SPECIFIC BEHAVIOR (bottom)** | 0.50 | 0.60 | **0.75** | **0.65** |
| **CRITICAL warning text (middle)** | 0.20 | 0.25 | 0.25 | 0.25 |
| **Few-shot example** | **0.50** | — | — | — |
| **Parser (code)** | 0.20 | 0.25 | 0.25 | 0.25 |

### Evidence from Raw Responses

**M3 hallucinates old Ranging 0.75:**
- ONC-001: "Ranging regime requires conviction >= 0.75 per FID-126" → model uses stale 0.75, parser uses 0.25
- RNG-002: M3 computes conviction=0.28, but thinks threshold is 0.40 → Pass
- RNG-003: M3 computes conviction=0.22, thinks threshold=0.40 → Pass

**M3 uses old Ranging 0.40 (from table, not behavior section):**
- TRD-003: "Ranging threshold = 0.40 (v0.14.0)" → conviction=0.0, still Pass
- SEN-005: "Ranging threshold 0.40" → conviction=0.17, Pass
- EDG-002: "Ranging threshold = 0.40" → conviction=0.32, Pass

**M3 uses old Trending 0.50 from few-shot example:**
- COR-001: conviction=0.43, Trending — 0.43 passes 0.20 parser threshold but M3 self-censors because few-shot teaches 0.50

**M3 uses old ADX boundaries:**
- TRD-003: ADX 18.55 classified as "Ranging (ADX 18.55 < 20)" — but GreyZone is 18-26 in current config
- Multiple scenarios classify ADX 18-20 as Ranging instead of GreyZone

### Additional Issues Found

1. **"Why Ranging is now 0.40"** text contradicts the CRITICAL warning that says "Ranging = 0.25"
2. **"Why Volatile needs MORE triggers (0.40 vs 0.30)"** — uses old table values, not current parser values
3. **Few-shot example** says "Trending regime threshold = 0.50" — directly teaches wrong value
4. **REGIME-SPECIFIC BEHAVIOR** uses old ADX boundaries (ADX < 20 = Ranging, 20-25 = GreyZone) instead of current (18-26 = GreyZone)

## GREEN Phase — Proposed Fix

### Single file change: `src/agent/prompts/strategy_knowledge.md`

**1. Matrix table:** 0.30/0.40/0.40/0.40 → 0.20/0.25/0.25/0.25

**2. Remove contradictory rationale text.** Delete "Why Ranging is now 0.40" (contradicts 0.25). Replace "Why Volatile needs MORE (0.40 vs 0.30)" with current values.

**3. REGIME-SPECIFIC BEHAVIOR section:** Replace ALL old thresholds (0.50/0.60/0.75/0.65) with current (0.20/0.25/0.25/0.25). Fix ADX boundaries (ADX < 20 → Ranging → ADX 18-26 → GreyZone).

**4. Few-shot example:** 0.50 → 0.20

**5. Strengthen CRITICAL warning.** Move it to the TOP of the prompt, make it the FIRST thing M3 reads.

### Expected Impact

The raw responses show M3 computes correct conviction scores (mean 0.272). The problem is M3 self-censors because it reads the wrong threshold from the prompt. If M3 used the correct parser thresholds (0.20/0.25), conviction scores that currently produce Pass would become Buy:

| Scenario | Conviction | Old Threshold (M3 thinks) | New Threshold | Result Change |
|---|---|---|---|---|
| ONC-001 | 0.32 | 0.75 → Pass | 0.25 → **Buy** | ✅ |
| COR-001 | 0.43 | 0.50 → Pass | 0.20 → **Buy** | ✅ |
| COR-002 | 0.43 | 0.40 → Pass | 0.25 → **Buy** | ✅ |
| EDG-002 | 0.32 | 0.40 → Pass | 0.25 → **AdjustStop** | ✅ |
| CAT-002 | 0.32 | 0.50 → Pass | 0.20 → **Buy** | ✅ |
| RNG-002 | 0.28 | 0.40 → Pass | 0.25 → **Buy** | ✅ |
| TRD-005 | 0.23 | 0.40 → Pass | 0.25 → Pass (close) | ⚠️ |

**Predicted:** 6-7 additional Buys, failure rate drops from 28% to ~15-18%.

### Suggestions / Improvements You Should've Asked About

1. **Also fix the parser's Hold→Buy override.** Currently requires `confidence > 0.0 && entry_price > 0.0` — but Pass decisions often have confidence=0.0 and entry_price=0.0. The override should fire on conviction alone, using the regime threshold from the parser. Without this parser fix, even if M3 produces correct conviction scores, the override can't turn Pass→Buy because confidence=0.0 blocks it.

2. **Remove the "Why Ranging is now 0.40" text entirely** — it's a stale rationale that contradicts current values and confuses the model.

3. **Update the `conviction_threshold()` method in Rust** to match what the prompt says, or vice versa. Currently the parser uses 0.20/0.25/0.25/0.25. If you want to stay at those values, the prompt must match exactly.

4. **The few-shot example is teaching M3 to use 0.50 as Trending threshold.** This is the single most impactful line to change — it's a direct instruction to the model.

## AUDIT Phase

- [x] `cargo check` passes ✅
- [x] `cargo test` 308/308 ✅
- [x] Sandbox re-run: 45/15 passed (25% failure), 6 Buys, 0 Sells, 0 parse errors
- [x] Code review passed (4 issues fixed in round 2)

## Sandbox Results — Honest Assessment

| Metric | Pre-FID-140 | Post-FID-140 | Change |
|---|---|---|---|
| Passed/Failed | 43/17 (28%) | 45/15 (25%) | -3pp |
| Avg Score | 0.57 | 0.56 | -0.01 |
| BUY count | 12 (gate_disabled) / 5 (gate_enabled) | 6 | ← went DOWN |
| Parse errors | 0 | 0 | — |

### Why the BUY count went DOWN instead of up

The prompt fix **unified** all threshold references to 0.20/0.25 — but this gave M3 a single, clear threshold to self-censor against. Before the fix, M3 read 5 contradictory threshold sets and sometimes picked the wrong one, letting a Buy through. Now it can consistently self-censor at exactly 0.20/0.25.

**The fundamental issue is M3's "default-to-hold" bias, which exists regardless of threshold value.** Even with thresholds matching the parser exactly (0.20/0.25), M3 passes on 90% of scenarios (54/60).

### What WAS fixed
- ✅ All 5 contradictory threshold sets unified to one (0.20/0.25/0.25/0.25)
- ✅ Stale rationale text deleted ("Why Ranging is now 0.40", etc.)
- ✅ Few-shot example fixed (0.50 → 0.20)
- ✅ ADX boundaries fixed (Ranging < 18, GreyZone 18-26, no overlap)
- ✅ CRITICAL warning cleaned (no longer plants stale 0.75/0.65 values)
- ✅ Parser override now defaults entry_price to current_price when overriding
- ✅ Parser override now defaults side to Long

### What was NOT fixed (remains as structural limitation)
- M3 still passes on 90% of scenarios regardless of threshold value
- Parser override only produces 6 Buys from 60 scenarios
- The override requires `!has_explicit_hold` — M3 may be emitting "hold/wait" in reasoning

## COMPLETE

**Verdict:** Necessary cleanup (removed prompt contradictions) but insufficient to fix pass rate. The 25% failure rate is M3's structural ceiling with current parser override aggressiveness. Next step: investigate whether M3's reasoning contains hold signals blocking the override, or whether conviction scores are genuinely below 0.20/0.25.
