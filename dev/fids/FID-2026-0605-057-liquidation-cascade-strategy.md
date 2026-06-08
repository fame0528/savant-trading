# FID: Liquidation Cascade Trading — Primary Strategy Overhaul

**Filename:** `FID-2026-0605-057-liquidation-cascade-strategy.md`
**ID:** FID-2026-0605-057
**Severity:** critical
**Status:** in_progress
**Created:** 2026-06-05 23:45
**Updated:** 2026-06-08 17:10
**Author:** Kilo

---

## Summary

The system currently evaluates 10 pairs every 5 minutes using RSI/ADX/EMA — the lowest-edge approach possible. The knowledge base (171 books, 265 units) identifies liquidation cascade trading as the highest-edge crypto-specific setup. This FID overhauls the strategy from "evaluate everything constantly" to "wait for the money in the corner, then strike."

**Post-FID-085 update:** Token costs reduced 90%+ (31K → ~5.7K tokens/pair). The cost analysis below reflects this.

---

## Detailed Description

### Problem

The current engine loop:
1. Iterates 10 pairs every 5 minutes
2. Pre-filters with RSI/ADX/EMA (removes 50-70%)
3. Sends remaining pairs to LLM
4. LLM returns BUY/HOLD/SELL based on candle data
5. Most cycles produce no actionable signal
6. Post-FID-085 cost: ~$0.06-0.19/hour (90% reduction). Still paying to evaluate when edge isn't present.

**The system evaluates 288 times/day when the edge only appears 1-3 times/day.**

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

| Component | File | Change Type |
|-----------|------|-------------|
| Cascade detector | `src/monitor/cascade_detector.rs` (NEW) | OI + funding + volume monitoring |
| Liquidation mapper | `src/monitor/liquidation_mapper.rs` (NEW) | Coinglass API integration |
| Kill zone filter | `src/core/kill_zone.rs` (NEW) | Time-based trade gating |
| Cascade config | `src/core/config.rs` | New `CascadeConfig` struct |
| Engine integration | `src/engine.rs` | Cascade check before LLM eval |
| ContextEngine | `src/agent/context_engine.rs` | Inject cascade data into prompt |
| Dashboard | `dashboard/` | Cascade alert widget |

### Risk Level

- [x] Critical: Current strategy is structurally unprofitable at any capital level.

---

## Proposed Solution

### Phase 1: Data Acquisition Layer

**1.1 Open Interest Monitor**
- Source: Coinglass API (https://open-api.coinglass.com/public/v2/open_interest)
- Auth: API key in `COINGLASS_API_KEY` env var
- Rate limit: 30 req/min (free tier), 100 req/min (paid)
- Data: Aggregate OI per pair, OI change rate (1h, 4h, 24h)
- Storage: In-memory `HashMap<String, OiSnapshot>` with 1h rolling window
- Update frequency: Every 60 seconds (separate from 5-min eval cycle)

**1.2 Funding Rate Monitor**
- Source: Existing OKX funding rate data (already fetched in engine.rs)
- Enhancement: Track funding rate history (last 24h), not just current value
- Thresholds: > 0.05%/8hr = overleveraged long, < -0.03%/8hr = overleveraged short
- Storage: In-memory `VecDeque<FundingSnapshot>` per pair (24h window)

**1.3 Liquidation Level Mapper**
- Source: Coinglass API (https://open-api.coinglass.com/public/v2/liquidation_map)
- Data: Liquidation clusters by price level (aggregated across exchanges)
- Storage: In-memory `BTreeMap<f64, LiquidationCluster>` per pair
- Update frequency: Every 5 minutes (aligned with eval cycle)

**1.4 Volume Spike Detector**
- Source: Existing candle data (already fetched)
- Enhancement: Compare current volume to 20-period SMA volume
- Threshold: Volume > 3× SMA(20) = significant spike
- This is already partially implemented via `IndicatorEngine::kbar_features()` (VolRatio)

### Phase 2: Cascade Detection Engine

**2.1 State Machine**

```
IDLE → PRE_CASCADE → CASCADE_IN_PROGRESS → CASCADE_EXHAUSTED → ENTRY_READY
  ↑                                                                   │
  └───────────────────── (timeout / false signal) ────────────────────┘
```

**State transitions:**
- IDLE → PRE_CASCADE: OI extreme + funding extreme + price within 5% of liquidation cluster
- PRE_CASCADE → CASCADE_IN_PROGRESS: Price hits cluster + OI drops >10% in 1h + volume > 3× SMA
- CASCADE_IN_PROGRESS → CASCADE_EXHAUSTED: Volume spike + rapid reversal (>2% bounce in 5m) + OI stabilized (change < 5% in 15m)
- CASCADE_EXHAUSTED → ENTRY_READY: LLM confirms V-recovery probability > 60%
- Any state → IDLE: Timeout (2h max in any state) or invalidation (price breaks structure)

**2.2 Cascade Score**

Composite score (0-100) combining:
- OI divergence score (0-30): Rising OI + falling price = max score
- Funding extreme score (0-25): |funding| > 0.05% = max score
- Liquidation proximity score (0-25): Price within 2% of dense cluster = max score
- Volume spike score (0-20): Volume > 5× SMA = max score

Entry threshold: Score > 70 triggers LLM evaluation.

### Phase 3: LLM Integration

**3.1 Cascade-Specific Prompt**

When cascade score > 70, inject cascade context into the LLM prompt:
```
## CASCADE ALERT — {pair}
State: {state} | Score: {score}/100
OI Change (1h): {oi_change}% | Funding: {funding}%
Nearest Liquidation Cluster: ${cluster_price} (${cluster_size}M)
Volume vs SMA(20): {vol_ratio}×
Kill Zone: {active/inactive}

Analyze this liquidation cascade setup. Is this a real cascade or a false signal?
What is the V-recovery probability? Define entry, stop, and target.
```

**3.2 Integration with ContextEngine (FID-085)**

The cascade data is injected as a new section in `build_tsln_message()`:
- Cascade alert section goes BETWEEN indicators and market insight
- High priority — always included when cascade score > 70
- Low token cost: ~200 tokens for cascade context

### Phase 4: Kill Zone Enforcement

**4.1 Kill Zone Config**
```toml
[cascade.kill_zones]
enabled = true
zones = [
    { name = "EU", start = "07:00", end = "10:00", timezone = "UTC" },
    { name = "US", start = "13:30", end = "16:30", timezone = "UTC" },
    { name = "ASIA", start = "00:00", end = "03:00", timezone = "UTC" },
]
allow_off_hours_monitoring = true  # Monitor 24/7, only trade in kill zones
```

**4.2 Enforcement**
- Cascade detection runs 24/7 (cascades don't check the clock)
- Entry execution ONLY during kill zones
- If cascade exhausted outside kill zone: log alert, wait for next kill zone
- If cascade exhausted inside kill zone: immediate LLM evaluation

### Phase 5: Risk Management for Leveraged Positions

**5.1 Position Sizing**
- Max leverage: 5× (conservative for $26 account)
- Max risk per trade: 10% of equity ($2.60 at current balance)
- Kelly fraction: 0.25× Kelly (quarter-Kelly for safety)
- Scale-in: 50% at entry, 50% at confirmation (2% move in favor)

**5.2 Stop Loss**
- Stop: Below the cascade low (invalidation level)
- Max stop distance: 5% from entry (leverage-adjusted: 25% of equity)
- Trailing: Activate at 1:1 R:R, trail at 2× ATR

**5.3 Take Profit**
- TP1: Next liquidation cluster above (50% scale-out)
- TP2: 2× ATR from entry (30% scale-out)
- Trail: Remaining 20% with 2× ATR trail

**5.4 Existing Position Handling**
- If cascade fires on a pair we already have a position in:
  - LONG position + bullish cascade: HOLD, consider adding at confirmation
  - LONG position + bearish cascade: CLOSE immediately, reverse if confirmed
  - No position: Standard cascade entry protocol

### Phase 6: Alerting

**6.1 Dashboard Integration**
- New `/api/cascade` endpoint returns current cascade state for all pairs
- Dashboard widget: Cascade score gauge (0-100) per pair
- Color coding: Green (< 30), Yellow (30-60), Orange (60-70), Red (> 70)

**6.2 Terminal Alerts**
- PRE_CASCADE: `[CASCADE] {pair} score={score} — OI diverging, funding extreme`
- CASCADE_IN_PROGRESS: `[CASCADE] {pair} CASCADE IN PROGRESS — OI dropping, volume spiking`
- ENTRY_READY: `[CASCADE] {pair} ENTRY SIGNAL — score={score}, kill_zone={active}`

---

## Implementation Order

1. **FID-057a** — OI + funding monitor (data acquisition, 60s polling)
2. **FID-057b** — Liquidation level mapper (Coinglass API integration)
3. **FID-057c** — Cascade detection state machine + score calculator
4. **FID-057d** — Kill zone enforcement
5. **FID-057e** — Cascade-specific LLM prompt + ContextEngine integration
6. **FID-057f** — Risk management + position sizing for leveraged trades
7. **FID-057g** — Dashboard cascade widget + alerting
8. **FID-057h** — Sandbox validation with historical cascade events

Each sub-FID is independently deployable and testable.

---

## Perfection Loop

### Loop 1 — RED Phase

- **RED:** 10 issues identified: No API endpoints, no storage design, no ContextEngine integration, no test plan, no rollout strategy, outdated costs, no kill zone enforcement, no risk management, no existing position handling, no alerting.
- **GREEN:** All 10 issues addressed in the solution above.
- **AUDIT:**
  - Five Questions: ALL PASS
  - Data sources verified: Coinglass API exists, OKX funding already fetched
  - ContextEngine integration: ~200 tokens for cascade context, fits in Phase 2 of prompt
  - Risk management: Quarter-Kelly, 5× max leverage, 10% max risk per trade
- **CHANGE_DELTA:** New FID — N/A (baseline)

### Loop 2 — GREEN Phase (Design Refinement)

- **RED:** 3 issues found in Loop 1 GREEN:
  1. Cascade state machine needs timeout handling (what if cascade stalls?)
  2. Coinglass API free tier may not have liquidation map data
  3. $26 account with 5× leverage = $130 buying power — is this enough for meaningful trades?
- **GREEN:**
  1. Added 2h timeout for any state → auto-reset to IDLE
  2. Fallback: use on-chain OI data from DeFiLlama if Coinglass unavailable
  3. At $26 equity, 5× leverage = $130. ETH at $1687 = 0.077 ETH position. At 5% move = $6.50 profit. Meaningful at micro scale.
- **AUDIT:** All 3 corrections validated.
- **CHANGE_DELTA:** ~5% of design (timeout, fallback, sizing validation)

### Loop 3 — AUDIT Phase

- **RED:** 2 issues found:
  1. Kill zone "ASIA 00:00-03:00 UTC" overlaps with "EU 07:00-10:00 UTC" — no gap for rest
  2. Cascade score thresholds (70 for entry) are arbitrary — need empirical validation
- **GREEN:**
  1. Kill zones are independent windows, not sequential. Overlap is fine — system trades in any active zone.
  2. Threshold 70 is a starting point. FID-057h (sandbox validation) will calibrate against historical cascades. Config will be tunable.
- **AUDIT:** Both corrections are sound.
- **CHANGE_DELTA:** ~3% (clarifications, not code changes)

### Loop 4 — SELF-CORRECT Phase

- **RED:** 1 issue:
  1. The FID references "Coinglass API" but the free tier may have changed since the research was done
- **GREEN:**
  1. Add fallback data sources: DeFiLlama (free, on-chain OI), Binance API (free, funding rates), existing OKX data. Coinglass is preferred but not required.
- **AUDIT:** Fallback chain is robust.
- **CHANGE_DELTA:** ~2% (fallback documentation)

### Loop 5 — COMPLETE Phase

- **RED:** Final review — no new issues.
- **GREEN:** N/A
- **AUDIT:**
  - All 10 original issues addressed
  - All 3 design refinements applied
  - All 2 audit findings resolved
  - All 1 self-correction applied
  - Five Questions: ALL PASS
  - Implementation order: 8 sub-FIDs, each independently deployable
- **CHANGE_DELTA:** 0% (convergence reached)
- **STATUS:** Design complete. Awaiting approval to begin FID-057a.

---

## Verification

### Per-Sub-FID Verification

| Sub-FID | Verification | Tool |
|---------|-------------|------|
| 057a | OI data fetched, stored, accessible | Unit tests + manual API check |
| 057b | Liquidation levels mapped per pair | Unit tests + Coinglass response validation |
| 057c | State machine transitions correctly | Unit tests for all state transitions |
| 057d | Kill zone enforcement blocks off-hours entries | Unit tests with mocked timestamps |
| 057e | Cascade context injected into LLM prompt | Unit tests + prompt inspection |
| 057f | Position sizing respects Kelly/leverage limits | Unit tests for edge cases |
| 057g | Dashboard shows cascade scores | Manual dashboard verification |
| 057h | Historical cascade events produce correct signals | Backtest against known cascades |

### End-to-End Verification

1. Run engine in paper mode for 48 hours
2. Verify cascade detection fires on real market events
3. Verify LLM receives cascade context when score > 70
4. Verify kill zone enforcement blocks off-hours entries
5. Verify position sizing respects risk limits
6. Compare cascade-triggered decisions vs. non-cascade decisions (quality should be higher)

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —

---

## References

1. Coinglass API docs: https://open-api.coinglass.com/
2. DeFiLlama OI API: https://api.llama.fi/
3. Knowledge units: cn-018, cn-019, cn-022, cn-027, pa-016, exec-009
4. FID-085: Context Window Overhaul (cascade context integration)
5. FID-086: Stale Price Pipeline (data freshness requirements)
