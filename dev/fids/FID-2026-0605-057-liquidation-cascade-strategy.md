# FID: Liquidation Cascade Trading — Primary Strategy Overhaul

**Filename:** `FID-2026-0605-057-liquidation-cascade-strategy.md`
**ID:** FID-2026-0605-057
**Severity:** critical
**Status:** created
**Created:** 2026-06-05 23:45
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

The system currently evaluates 10 pairs every 5 minutes using RSI/ADX/EMA — the lowest-edge approach possible. The knowledge base (171 books, 265 units) identifies liquidation cascade trading as the highest-edge crypto-specific setup. This FID overhauls the strategy from "evaluate everything constantly" to "wait for the money in the corner, then strike."

---

## Detailed Description

### Problem

The current engine loop:
1. Iterates 10 pairs every 5 minutes
2. Pre-filters with RSI/ADX/EMA (removes 50-70%)
3. Sends remaining pairs to LLM ($0.01-0.02 per call)
4. LLM returns BUY/HOLD/SELL based on candle data
5. Most cycles produce no actionable signal
6. API cost: $0.60-1.92/hour. Profit: $0.67/day

**The system is paying to look at the market 288 times per day when the edge only appears 1-3 times per day.**

### What the Knowledge Base Says

The 171 books identify ONE setup as consistently profitable in crypto: **liquidation cascade reversals.**

**cn-018 (Liquidation Cascade Mechanics):**
> "Cascades are VIOLENT and FAST — they create 10-20% moves in minutes. They are also PREDICTABLE by mapping liquidation clusters. After cascade completes, price typically V-shapes as forced selling exhausts."

**cn-019 (Liquidation Heatmap Trading):**
> "Price is MAGNETICALLY ATTRACTED to liquidation clusters because exchanges profit from liquidations. Enter long AFTER liquidation cascade completes (not before). Target the next liquidation cluster above."

**cn-027 (Liquidation Cluster Price Targets):**
> "Dense liquidation clusters act as price MAGNETS. Map the largest liquidation clusters above and below current price. These are your directional targets."

**pa-016 (Liquidation Cascades as Wyckoff Springs):**
> "Buy V-recovery when OI dropped 20%+. Crypto's Wyckoff Springs."

**cn-022 (Leverage Reflexivity Loop):**
> "When aggregate OI rises alongside positive funding, the market is building a liquidation bomb. The larger the OI and the higher the funding, the MORE violent the eventual cascade."

**exec-009 (Kill Zones):**
> "EU Kill Zone: 07:00-10:00 UTC. US Kill Zone: 13:30-16:30 UTC. Trade ONLY during kill zones."

### Expected Behavior

**Instead of:** Evaluate 10 pairs × 288 cycles/day = 2,880 evaluations/day
**Do this:** Monitor OI + funding + liquidation levels continuously. Fire LLM only when cascade conditions are met. Expected: 1-5 evaluations/day, each with massive edge.

### Root Cause

The system was built as a general-purpose pair evaluator. The knowledge base identifies a specific, high-edge setup that the system doesn't detect or trade.

---

## Impact Assessment

### Affected Components

- `src/engine.rs` — New cascade detection layer before Phase 2
- `src/monitor/` — New liquidation level monitor (Coinglass API or on-chain OI)
- `src/core/config.rs` — New strategy config (cascade thresholds, kill zones)
- `src/agent/soul.md` — Already updated to v2.0
- `config/default.toml` — Strategy parameters

### Risk Level

- [x] Critical: Current strategy is structurally unprofitable at any capital level.

---

## Proposed Solution

### Phase 1: Cascade Detection Layer

Add a monitoring layer that runs BEFORE the LLM evaluation loop:

1. **OI Divergence Monitor** — Track open interest changes. Rising OI + falling price = building short pressure. Falling OI + rising price = short covering (unsustainable rally).

2. **Funding Rate Extreme Monitor** — Track funding rates across exchanges. Funding > 0.05%/8hr = overleveraged long. Funding < -0.03%/8hr = overleveraged short. These are pre-cascade conditions.

3. **Liquidation Level Mapper** — Use Coinglass API or on-chain data to identify dense liquidation clusters. These are the price targets.

4. **24/7 Monitoring** — Crypto runs around the clock. Liquidation cascades don't check the clock. Monitor continuously, strike when the setup fires regardless of time. Asian session sweeps are some of the highest-edge setups.

### Phase 2: Trigger System

When cascade conditions are detected:
1. **Pre-cascade:** OI extreme + funding extreme + price approaching liquidation cluster = ALERT
2. **Cascade in progress:** Price hits cluster, OI drops 20%+, volume spikes = WATCH
3. **Cascade exhausted:** Volume spike + rapid reversal + OI stabilized = ENTRY
4. **Post-cascade:** Price targets next liquidation cluster = EXIT TARGET

### Phase 3: LLM Integration

The LLM is called ONLY when a cascade trigger fires. Its job:
1. Confirm the cascade is real (not a false signal)
2. Assess the V-recovery probability
3. Define entry, stop, and target based on liquidation cluster levels
4. Size the position using Kelly/leverage framework

### Phase 4: Execution

- Entry on GMX V2 or Hyperliquid with 5-8x leverage
- Stop below the cascade low (invalidation level)
- Target: next liquidation cluster above (cn-027)
- Scale-out: 50% at 1:1, trail rest (ts-028)

---

## Implementation Order

1. **Research phase** — Gemini Deep Research prompt on liquidation cascade detection APIs and data sources
2. **FID-057a** — Liquidation level monitor (data acquisition)
3. **FID-057b** — Cascade trigger detection logic (OI + funding + volume + liquidation levels)
4. **FID-057c** — LLM prompt for cascade confirmation (only fires on trigger)
5. **FID-057d** — GMX V2 / Hyperliquid execution layer (leverage entry)
6. **FID-057e** — Sandbox validation with historical cascade events

---

## Perfection Loop

### Loop 1

- **RED:** —
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
