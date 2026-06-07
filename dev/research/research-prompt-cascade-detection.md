# Gemini Deep Research: Liquidation Cascade Detection — Data Sources & APIs

## Research Objective

Identify the best data sources, APIs, and real-time feeds for detecting crypto liquidation cascades in real-time. The goal is to build an automated system that monitors for cascade conditions 24/7 and triggers trades when a cascade completes and price V-reverses.

---

## Context

We're building an autonomous crypto trading agent with $26 starting capital. The agent uses an LLM for decision-making and trades on-chain via DEX (0x API on Arbitrum) and plans to add leveraged perpetual futures via GMX V2 or Hyperliquid.

Our knowledge base of 171 trading books identifies liquidation cascade reversals as the single highest-edge setup in crypto:

- Cascades create 10-20% moves in minutes
- They are predictable by mapping liquidation clusters
- Price is magnetically attracted to dense liquidation zones
- After cascade completes, price V-shapes as forced selling exhausts
- Entry after cascade + OI drop of 20%+ = highest probability reversal

---

## Research Questions

### 1. Liquidation Level Data Sources

- What APIs provide real-time liquidation level data? (Coinglass, Hyblock Capital, etc.)
- What's the cost? (Free tier vs paid)
- What's the latency? (Real-time vs delayed)
- Can we get liquidation level heatmaps programmatically?
- What's the granularity? (Per-exchange, aggregate, per-pair?)
- Which pairs are covered? (Do they cover ARB, LINK, PEPE on GMX/Hyperliquid?)

### 2. Open Interest (OI) Data Sources

- What APIs provide real-time OI data across exchanges?
- Can we get per-pair OI changes in real-time? (Not just aggregate)
- What's the update frequency? (Tick-by-tick, 1-minute, 5-minute?)
- Can we detect OI drops of 20%+ in real-time? (cascade confirmation)
- What about cross-exchange OI divergence? (Binance vs Hyperliquid vs dYdX)

### 3. Funding Rate Data Sources

- We already fetch funding rates from OKX every cycle. Is this sufficient?
- Should we monitor funding rates across multiple exchanges simultaneously?
- What funding rate thresholds indicate imminent cascade? (>0.05%/8hr? >0.1%?)
- How quickly does funding flip during a cascade? (Seconds? Minutes?)

### 4. Cascade Detection Algorithms

- What defines a "cascade in progress"? (OI drop %, volume spike %, price velocity?)
- How do we distinguish a cascade from a normal dip? (Magnitude? Speed? OI change?)
- What's the typical cascade duration? (How long from trigger to exhaustion?)
- How do we detect cascade EXHAUSTION? (Volume spike + rapid reversal + OI stabilized?)
- Are there published algorithms or papers on cascade detection?

### 5. V-Recovery Confirmation

- How do we confirm a V-recovery is real vs a dead cat bounce?
- What volume profile confirms cascade exhaustion?
- What's the typical V-recovery magnitude? (50% retracement? 61.8% Fib?)
- How quickly does the V-recovery happen? (Minutes? Hours?)

### 6. GMX V2 and Hyperliquid APIs

- What's the GMX V2 API for opening leveraged positions programmatically?
- What's the Hyperliquid API? (REST? WebSocket?)
- What leverage is available on each? (Max leverage per pair?)
- What are the fees on each? (Maker/taker, funding costs?)
- Which has better liquidity for altcoins (ARB, LINK, PEPE)?
- Can we set stop-loss and take-profit orders programmatically?
- What's the liquidation logic on each? (Partial close-out vs full liquidation?)

### 7. Historical Cascade Events

- Can we identify historical liquidation cascade events from on-chain data?
- What data would we need to backtest a cascade-trading strategy?
- Are there public datasets of liquidation events?
- How often do cascades occur on major pairs? (Daily? Weekly?)

### 8. Real-Time Monitoring Architecture

- What's the best architecture for 24/7 cascade monitoring?
- WebSocket vs REST polling for OI/funding/liquidation data?
- How much data processing is needed per pair?
- Can a single machine monitor 10 pairs simultaneously?
- What's the latency budget? (How fast do we need to react?)

---

## What We Need

A concrete data acquisition plan that:
1. **Identifies specific APIs** with costs, latency, and coverage
2. **Defines the monitoring architecture** (what data, how often, how processed)
3. **Specifies cascade detection thresholds** (OI drop %, funding extreme, volume spike)
4. **Covers GMX V2 / Hyperliquid integration** (API docs, fees, leverage limits)
5. **Includes historical data sources** for backtesting
6. **Is implementable** — we can build the data layer in 1-2 days

---

## Technical Context

- **Language:** Rust (backend), TypeScript (dashboard)
- **Existing data:** Kraken WebSocket (real-time prices, 5m candles), OKX funding rates (every cycle), BGeometrics on-chain metrics (MVRV, SOPR, NUPL)
- **Chains:** Arbitrum primary (0x API for spot), planning GMX V2 or Hyperliquid for leverage
- **Budget:** Minimal — prefer free/low-cost APIs. $26 account cannot support expensive data feeds.
- **Platform:** Windows, 24/7 operation
