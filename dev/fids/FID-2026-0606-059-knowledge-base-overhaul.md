# FID: Knowledge Base Overhaul — Replace Hedge Fund Mindset with Crypto-Native Trading Framework

**Filename:** `FID-2026-0606-059-knowledge-base-overhaul.md`
**ID:** FID-2026-0606-059
**Severity:** critical
**Status:** created
**Created:** 2026-06-06 03:01
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

The knowledge base has 265 units from 171 books — almost entirely institutional trading knowledge (Wyckoff, Elder, Turtle, VPA, Bulkowski). This hedge fund mindset made the agent cautious, patient, and slow. Crypto trading demands selective aggression: fast entries on cascades, hard stops, and sitting on hands when there's no setup. The 20 new YouTube interview knowledge files provide the missing "degen" mindset that matches how crypto actually moves.

---

## Detailed Description

### Problem

The agent's decision-making is shaped by the knowledge units injected into the LLM prompt. Currently, the knowledge base is dominated by:

- **Wyckoff accumulation/distribution** — institutional, multi-week timeframes
- **Elder's triple screen** — conservative, multi-timeframe confirmation
- **Turtle system** — trend following with wide stops, designed for futures
- **VPA (Volume Price Analysis)** — reading institutional footprints
- **Bulkowski chart patterns** — statistical pattern recognition from stocks

These are all **hedge fund / prop firm** knowledge. They optimize for:
- Capital preservation over capital velocity
- Low win rate with high R:R over high win rate with tight stops
- Multi-day holding periods over intraday scalping
- Diversification over concentration

**Result:** The agent evaluates 10 pairs every 5 minutes, waits for "perfect" setups that rarely come, and holds positions for hours/days instead of striking fast on liquidation cascades.

### What the YouTube Knowledge Adds

The 20 YouTube interview files contain knowledge from traders who actually make money in crypto:

| Source | Key Principle | Contrast with Books |
|--------|--------------|-------------------|
| **Fabio Scalper** | "Be right fast. If wrong fast, smaller stops." | Books say "wait for confirmation" |
| **Two Scalpers** | "Three-loss rule — shut down the day." | Books say "reduce size, keep trading" |
| **AI Bots** | 200-line architecture: candles → LLM → tool calls | Books say "build complex multi-signal systems" |
| **Claude Code Bot** | Momentum squeeze, sleep during chop, no exit rules | Books say "define exits before entries" |
| **Raoul Pal** | AI race = biggest capital event in history | Books say "focus on price, ignore narrative" |
| **Altcoin Bear** | Use alt/BTC ratio, not dollar charts | Books say "trade the dollar chart" |
| **Ultimate Course** | 65% win rate, 4.16R target, structure > intelligence | Books say "edge comes from analysis" |
| **Brian Jung** | "Buying dips in a downtrend is dangerous" | Books say "buy fear" |

### Root Cause

When the knowledge units are selected for the LLM prompt, the system pulls from the full 265-unit pool. The institutional knowledge (Wyckoff, Elder, Turtle) has higher priority scores and more units, so it dominates the prompt. The agent receives a prompt full of "wait for confirmation," "preserve capital," and "multi-timeframe analysis" — which produces a cautious, slow agent.

---

## Impact Assessment

### Affected Components

- `knowledge/extracted/` — 20 new YouTube knowledge files need to be integrated
- `src/agent/soul.md` — Already updated to v2.0, needs knowledge unit selection rules
- `src/engine.rs` — Knowledge unit selection logic (currently dumps all relevant units)
- `src/agent/provider.rs` — System prompt construction

### Risk Level

- [x] Critical: Agent behavior is directly shaped by knowledge base content. Wrong knowledge = wrong trades.

---

## Proposed Solution

### 1. Integrate YouTube Knowledge Files

Move the 20 extracted JSON files from `knowledge/youtube-interviews/extracted/` into the main knowledge pool. Tag them with `source:youtube-interview` for easy filtering.

### 2. Create Knowledge Priority Tiers

| Tier | Source | When Used | Priority |
|------|--------|-----------|----------|
| **Tier 1: Execution** | YouTube scalpers, AI bot architecture | ALWAYS — entry/exit/sizing | Highest |
| **Tier 2: Crypto-Native** | cn-001 through cn-044 (existing) | ALWAYS — on-chain, funding, liquidation | High |
| **Tier 3: Price Action** | YouTube courses, VPA, candle patterns | ALWAYS — setup identification | High |
| **Tier 4: Macro** | Raoul Pal, Brian Jung | REGIME — bull/bear context | Medium |
| **Tier 5: Institutional** | Wyckoff, Elder, Turtle, Bulkowski | BACKGROUND — only when regime is unclear | Low |

### 3. Restructure System Prompt

Currently: All 265 knowledge units are available, selected by condition matching.
New: Tier 1-3 units are ALWAYS included. Tier 4-5 units are only included when regime is unclear or during low-volatility periods.

### 4. Update Knowledge Unit Selection Logic

Add a selection function that:
1. Always includes Tier 1 (execution) and Tier 2 (crypto-native) units
2. Always includes Tier 3 (price action) units matching current conditions
3. Includes Tier 4 (macro) units only when regime detection is uncertain
4. Includes Tier 5 (institutional) units only as background context, max 3 units
5. Total cap: 8-12 knowledge units per evaluation (not all 265)

### 5. Update soul.md Knowledge Section

Add explicit instructions:
- "The YouTube interview knowledge (Tier 1) takes precedence over book knowledge (Tier 5) when they conflict"
- "Speed of execution matters more than confirmation at micro-scale"
- "The scalper mindset: be right fast, cut losses faster"

---

## Perfection Loop

### Loop 1

- **RED:** 8 checks performed. 5 FAIL, 3 PASS with gaps.
  - **FAIL:** YouTube files invisible to loader — `load_knowledge_base()` only reads top-level `knowledge/*.json`, doesn't recurse. 747 units (20% of knowledge) never loaded.
  - **FAIL:** No tier enforcement — MMR is purely score-based. A high-priority hedge fund unit can outrank a crypto-native unit.
  - **FAIL:** soul.md says "265 units from 171 books" in 6 places. Actual: 2,959 + 747 = 3,706 units.
  - **FAIL:** `prompts.rs` line 67: "From 11 Curated Transcripts" — stale, should be 30 sources.
  - **FAIL:** No manifest/index for knowledge discovery.
  - **PASS with gaps:** Token budget has headroom (8-12 units = 1,600-2,400 tokens, budget = 12,000).
  - **PASS with gaps:** MMR algorithm is tier-ready — `priority` field and `utility_score` can be repurposed.
  - **PASS with gaps:** YouTube JSON files use same schema as existing knowledge — will deserialize correctly.
- **GREEN:** —
- **AUDIT:** —
- **CHANGE DELTA:** —

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —
