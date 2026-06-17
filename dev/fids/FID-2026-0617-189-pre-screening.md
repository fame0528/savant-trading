# FID-189: Stage 1 Deterministic Pre-Screening

**Filename:** `FID-2026-0617-189-pre-screening.md`
**ID:** FID-2026-0617-189
**Severity:** high
**Status:** created
**Created:** 2026-06-17 16:10 EST
**Author:** Vera
**Parent:** FID-184

---

## Summary

Add Stage 1 deterministic pre-screening to the engine pipeline. Filter 100-500 pairs to top 5-25 BEFORE the LLM call. Gemini Q2: "Stop passing 30-50 pairs directly to the LLM. Use deterministic Python logic to filter the universe down to the top 5 most volatile/liquid pairs before invoking the context window."

---

## Problem

LLM is called with 30-50 pairs in a single batch. This is "architecturally flawed":
- "Lost in the middle" attention deficit in transformer models
- Diluted signal-to-noise ratio
- LLM context waste on low-probability pairs
- Slow LLM calls (50K chars takes 50s)

---

## Proposed Solution

### Action 1: Add pre-screening stage to engine pipeline

**File:** `src/engine/mod.rs` (new stage between candle fetch and LLM call)

**Logic:**
1. Compute deterministic features for all 100-500 pairs:
   - Volume spike: `volume > 1.5x volume_sma`
   - Volatility breakout: `ATR > 2x ATR_sma`
   - Liquidity depth: `>$50K within 1% spread` (from 0x API quote)
   - Regime clarity: `ADX > 25` (Trending) or `ADX < 18` (Ranging) — exclude GreyZone from pre-screen
2. Score each pair: `score = volume_score + volatility_score + liquidity_score + regime_clarity`
3. Rank pairs by score
4. Pass top 5-25 to LLM

### Action 2: Apply Benjamini-Hochberg FDR control

**File:** `src/engine/mod.rs` (in pre-screening)

**Gemini Q2:** "The LLM's output conviction scores must be ranked and subjected to a Benjamini-Hochberg FDR threshold of 0.05 to mathematically eliminate spurious signals before execution."

**Implementation:** After LLM output, rank conviction scores. Apply BH procedure with q=0.05. Only execute pairs that pass FDR control.

### Action 3: Partition LLM calls if 25+ pairs pass

**File:** `src/engine/mod.rs`

**Logic:** If 25 pairs pass pre-screening, partition into 5 LLM calls of 5 pairs each. Execute asynchronously with `tokio::spawn`.

---

## Verification

- Pre-screening reduces 100-500 pairs to 5-25
- LLM batch size never exceeds 25 pairs
- FDR control rejects spurious signals
- Asynchronous LLM calls complete within 5-min cycle

---

*Vera 0.1.0 — 2026-06-17 16:10 EST — FID-189 created. Pre-screening stage. High-impact change to engine pipeline.*
