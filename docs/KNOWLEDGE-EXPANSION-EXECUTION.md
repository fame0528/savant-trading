# Knowledge Expansion — Execution Plan

> Based on Gemini Deep Research results. Maps research domains → knowledge files →
> transcript sources → implementation steps.

---

## Research → Knowledge File Mapping

The research covers 10 domains with 106 cited sources. Here's exactly what to build:

---

### PHASE 1 — Highest Impact (fills critical gaps)

#### 1. `onchain_analytics.json` (NEW — 15-20 units)

**Research Section:** "On-Chain Analytics and Network Valuation Dynamics"

**Knowledge units to extract:**

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| onchain-001 | MVRV Ratio: Cycle Top/Bottom Detection | MacroAnalysis | Trending, ExtremeGreed, ExtremeFear | Glassnode, YouTube #7-10 |
| onchain-002 | NUPL: Supply Distribution by LTH/STH | MacroAnalysis | Trending, ExtremeGreed | Glassnode #3 |
| onchain-003 | SOPR: Realized Profit/Loss Ratio | MacroAnalysis | ExtremeFear, ExtremeGreed | Glassnode #3 |
| onchain-004 | NVT Signal: Network Valuation vs Utility | MacroAnalysis | Trending, Ranging | Glassnode #12-13 |
| onchain-005 | Exchange Inflow/Outflow as Leading Indicator | OrderFlow | ExtremeFear, ExtremeGreed | YouTube #14 |
| onchain-006 | Stablecoin Minting Flows as Buy Signal | Sentiment | ExtremeFear | Research §2 |
| onchain-007 | Long-Term Holder Accumulation Patterns | RiskManagement | ExtremeFear | Glassnode #11 |
| onchain-008 | On-Chain AI Agents: Automated Signal Extraction | AiStrategy | Trending, HighVolatility | YouTube #14, Nansen |
| onchain-009 | Realized Cap vs Market Cap Divergence | MacroAnalysis | Trending, ExtremeGreed | YouTube #10 |
| onchain-010 | MVRV Z-Score: Quantitative Thresholds | MacroAnalysis | ExtremeFear, ExtremeGreed | YouTube #7-8 |
| onchain-011 | SOPR Reset as Re-Entry Signal | Execution | ExtremeFear | Glassnode #3 |
| onchain-012 | Exchange Balance Multi-Year Lows | Sentiment | Trending, ExtremeFear | Research §2 |
| onchain-013 | Whale Wallet Tracking via On-Chain Data | OrderFlow | HighVolatility | YouTube #14 |
| onchain-014 | NVT Divergence: Price vs Network Activity | MacroAnalysis | Ranging | Glassnode #12 |
| onchain-015 | UTXO Age Distribution for Cycle Positioning | MacroAnalysis | Trending, Ranging | YouTube #11 |

**YouTube transcripts to pull:**

- [ ] "Mastering the Bitcoin MVRV Ratio" (YouTube #7)
- [ ] "Onchain Analysis: Mastering MVRV" (YouTube #8)
- [ ] "Bitcoin: MVRV Analysis — Glassnode Clips" (YouTube #9)
- [ ] "One Bitcoin On-chain Metric to Rule Them All" (YouTube #10)
- [ ] "The Essential Bitcoin On-chain Guide" (YouTube #11)
- [ ] "On-Chain AI Agents and the Future of Crypto Trading: Nansen's Vision" (YouTube #14)

---

#### 2. `risk_management.json` (NEW — 12-15 units)

**Research Section:** "Mathematical Frameworks for Risk Management and Capital Allocation"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| risk-001 | Drawdown Recovery Mathematics | RiskManagement | ALL | Research §5, Table |
| risk-002 | Kelly Criterion: Optimal Position Sizing | RiskManagement | ALL | YouTube #34-38 |
| risk-003 | Fractional Kelly: Half/Quarter Kelly for Survival | RiskManagement | ALL | YouTube #35-36 |
| risk-004 | Anti-Martingale: Scale Down on Losses | RiskManagement | ALL | YouTube #43 |
| risk-005 | ATR Trailing Stop: Chandelier Exit | Execution | Trending, HighVolatility | YouTube #44-48 |
| risk-006 | Risk of Ruin Calculation | RiskManagement | ALL | Research §5 |
| risk-007 | Portfolio Heat: Total Open Risk Management | RiskManagement | ALL | Research §5 |
| risk-008 | Correlation-Based Position Sizing | RiskManagement | HighVolatility | Research §5 |
| risk-009 | Volatility-Adjusted Position Sizing | RiskManagement | HighVolatility, LowVolatility | Research §5 |
| risk-010 | Maximum Favorable Excursion (MFE) Analysis | Execution | Trending | Research §5 |
| risk-011 | Risk-Reward Ratio Optimization | RiskManagement | ALL | YouTube #34 |
| risk-012 | Drawdown Circuit Breaker Thresholds | RiskManagement | ALL | ai_claude_bot (merge) |
| risk-013 | Scale-Out Mathematics: Partial Exit Optimization | Execution | Trending | Research §5 |
| risk-014 | Break-Even Stop: When It Helps vs Hurts | Execution | Trending, Ranging | YouTube #46-48 |

**YouTube transcripts to pull:**

- [ ] "How to Calculate Trade Size | Kelly Criterion Explained" (YouTube #34)
- [ ] "The $1,000,000 Trading Formula Nobody Talks About" (YouTube #35)
- [ ] "KELLY CRITERION | Ed Thorp | Optimal Position Sizing" (YouTube #36)
- [ ] "How Mathematicians Invest (Full Math Derivation) | Kelly Criterion Part 2" (YouTube #37)
- [ ] "Anti-Martingale STRATEGY & Position Sizing for MASSIVE PROFITS" (YouTube #43)
- [ ] "Average True Range Indicator Strategies & Techniques" (YouTube #45)
- [ ] "How to Use Trailing Stop Loss (5 Powerful Techniques)" (YouTube #46)
- [ ] "5 Trailing Stop Loss Techniques" (YouTube #48)

---

#### 3. `crypto_derivatives.json` (NEW — 10-12 units)

**Research Section:** "Derivatives Microstructure, Funding Arbitrage, and Liquidation Cascades"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| deriv-001 | Funding Rate Arbitrage (Cash-and-Carry) | Execution | Trending, HighVolatility | BitMEX #15 |
| deriv-002 | Funding Rate Convergence Across Exchanges | Execution | HighVolatility | Research §3 |
| deriv-003 | Liquidation Cascade Mechanics | RiskManagement | HighVolatility | Bookmap #19 |
| deriv-004 | Liquidation Heatmap Trading | Execution | HighVolatility | YouTube #20-21 |
| deriv-005 | Open Interest + Price Divergence | OrderFlow | HighVolatility, Trending | Research §3 |
| deriv-006 | Perpetual Futures vs Spot: When to Use Each | Execution | ALL | Research §3 |
| deriv-007 | Leverage Reflexivity Loop | RiskManagement | ExtremeGreed | Research §3 |
| deriv-008 | Basis Trade: Spot Long + Perp Short | Execution | Trending | BitMEX #15 |
| deriv-009 | 48-72 Hour Mean Reversion After OI Divergence | Execution | HighVolatility | Research §3 |
| deriv-010 | Kraken Futures: Fee Structure & Execution | Execution | ALL | Kraken #59-63 |
| deriv-011 | Cross-Exchange Funding Spread Arbitrage | Execution | HighVolatility | Research §3 |
| deriv-012 | Liquidation Cluster Price Targets | Execution | HighVolatility | YouTube #20-21 |

**YouTube transcripts to pull:**

- [ ] "Crypto Arbitrage: A Practical Guide (with Basis Trade Example)" — BitMEX Blog
- [ ] "BEST Liquidation Heatmap for Your Trading | Hyblock VS Coinglass" (YouTube #21)
- [ ] "Coinglass" (YouTube #20)
- [ ] Kraken fee structure videos (YouTube #59-63)

---

#### 4. `wyckoff_orderflow.json` (NEW — 10-12 units)

**Research Section:** "Price Action Microstructure and Order Flow Interpretation"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| wyckoff-001 | Wyckoff Accumulation Schematic (Phases A-E) | TechnicalAnalysis | Ranging, LowVolatility | Altrady #23 |
| wyckoff-002 | Wyckoff Distribution Schematic | TechnicalAnalysis | Trending, ExtremeGreed | Altrady #23 |
| wyckoff-003 | Spring/Shakeout: High-Probability Entry | Execution | Ranging | Altrady #23 |
| wyckoff-004 | Sign of Strength (SOS) Confirmation | Execution | Trending | Altrady #23 |
| wyckoff-005 | Footprint Chart: Delta Divergence | OrderFlow | ALL | TradingView #24-26 |
| wyckoff-006 | Footprint Chart: Stacked Imbalances | OrderFlow | ALL | TradingView #24 |
| wyckoff-007 | BTC.D Rotation Template | MacroAnalysis | Trending, AltSeason | Altrady #27-30 |
| wyckoff-008 | Stablecoin-Diluted Dominance Filter | MacroAnalysis | BtcDominant, AltSeason | MarsBit #29 |
| wyckoff-009 | Composite Operator Behavior in Crypto | TechnicalAnalysis | ALL | Altrady #23 |
| wyckoff-010 | UTAD: Distribution Trap Pattern | TechnicalAnalysis | ExtremeGreed | Altrady #23 |
| wyckoff-011 | Volume Spread Analysis (VSA) Integration | TechnicalAnalysis | ALL | Research §4 |
| wyckoff-012 | Automatic Rally (AR) Range Definition | TechnicalAnalysis | Ranging | Altrady #23 |

**YouTube transcripts to pull:**

- [ ] "Wyckoff Method: A Complete Guide for Crypto Traders" — Altrady
- [ ] "Footprint Charts for Beginners: A Practical Guide to Order Flow Trading"
- [ ] "Volume footprint charts: a complete guide" — TradingView
- [ ] "Bitcoin Dominance: What It Means for Traders" — Altrady
- [ ] "What Is Bitcoin Dominance (BTC.D) And How To Use It" — ATAS

---

### PHASE 2 — High Impact

#### 5. `macro_liquidity.json` (NEW — 8-10 units)

**Research Section:** "Macro Regime Identification and Global Liquidity Cycles"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| macro-001 | 65-Month Global Liquidity Cycle | MacroAnalysis | ALL | Michael Howell #2,5 |
| macro-002 | Debt-to-Liquidity Ratio as Cycle Indicator | MacroAnalysis | ALL | TradingView #4 |
| macro-003 | Halving Cycle Degradation: Macro > Supply | MacroAnalysis | ALL | YouTube #1 |
| macro-004 | DXY Inverse Correlation with BTC | MacroAnalysis | Trending, Ranging | Research §1 |
| macro-005 | Treasury QE / Central Bank Balance Sheets | MacroAnalysis | ALL | Research §1 |
| macro-006 | CBDC Impact on Crypto Market Structure | MacroAnalysis | ALL | YouTube #1 |
| macro-007 | ETF Capital Flows as Price Driver | MacroAnalysis | Trending | Research §1 |
| macro-008 | Debt Refinancing Cycle and Risk Assets | MacroAnalysis | ALL | TradingView #4 |
| macro-009 | Sovereign Debt Yields vs Crypto Correlation | MacroAnalysis | Trending | Research §1 |
| macro-010 | Repo Market and Shadow Banking Liquidity | MacroAnalysis | ALL | TradingView #4 |

**YouTube transcripts to pull:**

- [ ] "Bitcoin Breaking Cycle: 2026 Market Revolution Explained" (YouTube #1)
- [ ] "The Real Crypto Cycle: What Happens When Global Liquidity Peaks | Michael Howell" (YouTube #2, Bankless)
- [ ] "Crypto Has Entered Late-Cycle Territory" — TradingView #4

---

#### 6. `defi_execution.json` (NEW — 8-10 units)

**Research Section:** "Decentralized Finance (DeFi) Execution and Derivatives Yield"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| defi-001 | Hyperliquid: On-Chain Perps Mechanics | Execution | ALL | YouTube #65-69 |
| defi-002 | Hyperliquid Liquidation Logic (Partial Close) | RiskManagement | HighVolatility | Eco #67 |
| defi-003 | Auto-Deleveraging (ADL) as Systemic Failsafe | RiskManagement | HighVolatility | Eco #67 |
| defi-004 | Uniswap V3 Concentrated Liquidity | Execution | ALL | Wardour #70-73 |
| defi-005 | Impermanent Loss Mitigation Strategies | RiskManagement | HighVolatility | UFM Madrid #71 |
| defi-006 | Delta-Neutral Options: DDH on Deribit | Execution | ALL | YouTube #75-77 |
| defi-007 | Options Greeks: Delta, Gamma, Theta, Vega | RiskManagement | ALL | Deribit #74 |
| defi-008 | L2 Gas Optimization for LP Rebalancing | Execution | ALL | Wardour #70 |
| defi-009 | HYPE Token: Fee Burn Deflationary Model | MacroAnalysis | Trending | Grayscale #69 |
| defi-010 | On-Chain Order Book vs AMM: When to Use | Execution | ALL | Research §7 |

**YouTube transcripts to pull:**

- [ ] "Hyperliquid And SpaceX Synthetic Trading" (YouTube #65)
- [ ] "Hyperliquid on FIRE: 24/7 Crypto, Oil & War-Driven Markets" (YouTube #66)
- [ ] "Dynamic Delta Hedging On Deribit — Greeks Live Auto DDH Tool" (YouTube #75)
- [ ] "8.6 Delta hedging — Deribit Options Course Basics" (YouTube #76)
- [ ] "How to Use Delta Hedging to Lock up Profits" — Deribit #77

---

#### 7. `backtesting_deployment.json` (NEW — 8-10 units)

**Research Section:** "Algorithmic Strategy Validation and Walk-Forward Optimization"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| backtest-001 | Walk-Forward Optimization (WFO) Framework | Backtesting | ALL | QuantInsti #49 |
| backtest-002 | Concept Drift and Model Drift Detection | Backtesting | ALL | Interactive Brokers #51 |
| backtest-003 | K-Fold Cross-Validation for Trading | Backtesting | ALL | QuantInsti #49 |
| backtest-004 | Curve-Fitting vs Genuine Edge Detection | Backtesting | ALL | Reddit #50 |
| backtest-005 | Out-of-Sample Performance Stitching | Backtesting | ALL | QuantInsti #49 |
| backtest-006 | Window Selection Bias in WFO | Backtesting | ALL | QuantInsti #49 |
| backtest-007 | Black Swan Stress Testing | Backtesting | ALL | QuantInsti #49 |
| backtest-008 | Paper-to-Live Transition Milestones | Execution | ALL | Research §6 |
| backtest-009 | Monte Carlo Simulation for Strategy Validation | Backtesting | ALL | Research §6 |
| backtest-010 | Strategy Degradation Triggers | Backtesting | ALL | Research §6 |

**YouTube transcripts to pull:**

- [ ] "Walk-Forward Optimization: How It Works, Its Limitations" — QuantInsti
- [ ] "The Future of Backtesting: A Deep Dive into Walk Forward Analysis" — Interactive Brokers
- [ ] "Backtested 16000 retail trading strategies" — Reddit #50

---

#### 8. `execution_engineering.json` (NEW — 6-8 units)

**Research Section:** "Low-Latency Infrastructure and Exchange Connectivity"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| exec-001 | REST vs WebSocket: Latency Tradeoffs | Execution | ALL | CoinAPI #57 |
| exec-002 | WebSocket Order Book Sync: Reconnection Logic | Execution | ALL | Gist #58 |
| exec-003 | Rate Limit Management Across Exchanges | Execution | ALL | Simplified #53 |
| exec-004 | Kraken Fee Tier Optimization | Execution | ALL | Kraken #59-63 |
| exec-005 | Post-Only Limit Orders for Maker Fee Capture | Execution | ALL | Kraken #61 |
| exec-006 | CCXT Rate Limit Integration | Execution | ALL | Simplified #53 |
| exec-007 | WebSocket Packet Loss Detection & Re-sync | Execution | ALL | Simplified #53 |
| exec-008 | Kraken WebSocket v2 Protocol | Execution | ALL | Kraken #58 |

**YouTube transcripts to pull:**

- [ ] "Building Real-Time Trading Pipelines: From Polling to Streaming"
- [ ] "A realtime order book for trading Spot on Kraken using pyhton-kraken-sdk" — GitHub Gist
- [ ] Kraken fee/execution guides (YouTube #59-63)

---

### PHASE 3 — Medium Impact

#### 9. `prop_firm.json` (NEW — 6-8 units)

**Research Section:** "Proprietary Trading Firm Evaluation Frameworks"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| prop-001 | Static vs EOD Trailing vs Tick-by-Tick Drawdown | RiskManagement | ALL | Velotrade #81 |
| prop-002 | Consistency Rule Navigation | Execution | ALL | Apex #80 |
| prop-003 | Micro-Scalping for Prop Firm Passing | Execution | ALL | Velotrade #83 |
| prop-004 | 1% Max Per-Trade Risk for Evaluations | RiskManagement | ALL | ForTraders #84 |
| prop-005 | Crypto Prop Firm Comparison 2026 | Execution | ALL | altFINS #82 |
| prop-006 | News Trading Prohibition in Evaluations | Execution | BreakingNews | Apex #80 |
| prop-007 | Payout Validation and Withdrawal Rules | RiskManagement | ALL | Velotrade #81 |

**YouTube transcripts to pull:**

- [ ] "What Is the Smartest Way to Evaluate a Prop Trading Firm?" — Black Eagle #78
- [ ] "Crypto Prop Firms: What Nobody Tells You About Getting Funded in 2026" #79
- [ ] "Prop Firm Rules Explained: The Complete Trader Guide (2026)" #80

---

#### 10. `trading_psychology.json` (NEW — 8-10 units)

**Research Section:** "Cognitive Optimization and Trading Psychology"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| psych-001 | Tilt: Compounding Emotional Triggers | Psychology | ALL | Jared Tendler #85,88 |
| psych-002 | Cognitive Debiasing Frameworks | Psychology | ALL | Psychology Today #91 |
| psych-003 | Deliberate Practice for Trading | Psychology | ALL | Bear Bull Traders #89 |
| psych-004 | Separating Self-Worth from P&L | Psychology | ALL | Steenbarger #90 |
| psych-005 | Emotions as System Flaw Signals | Psychology | ALL | Jared Tendler #85 |
| psych-006 | Loss Aversion and Disposition Effect | Psychology | ALL | Research §9 |
| psych-007 | Metacognitive Awareness in Trading | Psychology | ALL | Bear Bull Traders #89 |
| psych-008 | Fight-or-Flight Response in Drawdowns | Psychology | ExtremeFear | Research §9 |
| psych-009 | Journal-Based Performance Review Protocol | Psychology | ALL | Dukascopy #86 |
| psych-010 | Consequence-Based Discipline Systems | Psychology | ALL | Juvier (existing) |

**YouTube transcripts to pull:**

- [ ] "The Mental Game of Trading Summary" — Trader Lion #85
- [ ] "When Setbacks Strike" — Jared Tendler #87
- [ ] "Surprising Facts About Tilt" — Jared Tendler #88
- [ ] "Mastering Trading Psychology" — Bear Bull Traders #89

---

#### 11. `compliance.json` (NEW — 4-5 units)

**Research Section:** "Regulatory Compliance, Taxation, and Entity Structuring"

| ID | Title | Topic | Conditions | Source |
|----|-------|-------|------------|--------|
| comply-001 | Form 1099-DA: New IRS Reporting Rules | RiskManagement | ALL | IRS #92-95 |
| comply-002 | Wash Sale Rule Coming to Crypto | RiskManagement | ALL | Brager #92 |
| comply-003 | Cost Basis Tracking: FIFO vs LIFO | RiskManagement | ALL | Research §10 |
| comply-004 | Entity Structuring: LLC vs S-Corp for Traders | RiskManagement | ALL | Research §10 |
| comply-005 | Tax-Loss Harvesting Before Wash Sale Rules | RiskManagement | ALL | Research §10 |

**YouTube transcripts to pull:**

- [ ] IRS Form 1099-DA guidance videos
- [ ] Crypto tax reporting 2026 guides

---

## New MarketCondition Tags Needed

The research introduces market states not covered by current conditions:

| New Condition | Trigger | Used By |
|---------------|---------|---------|
| `LiquidityExpansion` | Global liquidity cycle in expansion phase | macro-001 to macro-010 |
| `LiquidityContraction` | Global liquidity cycle in contraction phase | macro-001 to macro-010 |
| `MvrvExtreme` | MVRV ratio above 3.5 or below 1.0 | onchain-001, onchain-010 |
| `SoprReset` | SOPR returning to 1.0 after capitulation | onchain-011 |
| `OIDivergence` | Open interest rising while price falls | deriv-005, deriv-009 |
| `FundingExtreme` | Already exists — extend to ±0.05% | deriv-001 to deriv-003 |
| `WyckoffSpring` | Price drops below support on low volume | wyckoff-003 |
| `DeltaDivergence` | Positive delta at price low | wyckoff-005 |
| `AltSeason` | Already exists — enhance with BTC.D filter | wyckoff-007, wyckoff-008 |

---

## Code Changes Required

### 1. New `MarketCondition` variants

Add to `src/agent/knowledge.rs`:

```rust
pub enum MarketCondition {
    // ... existing variants ...
    LiquidityExpansion,
    LiquidityContraction,
    MvrvExtreme,
    SoprReset,
    OIDivergence,
    WyckoffSpring,
    DeltaDivergence,
}
```

### 2. New insight sources

Add to `src/insight/`:

- `onchain.rs` — MVRV, NUPL, SOPR, NVT from Glassnode/free APIs
- `derivatives.rs` — Enhanced funding rate analytics, OI tracking

### 3. Context builder updates

Add on-chain metrics to `FullContext` and `build_user_message_static`:

- MVRV ratio
- NUPL state
- SOPR value
- NVT Signal

### 4. Config additions

Add to `config/default.toml`:

```toml
[insight]
onchain_enabled = true
mvrv_enabled = true
sopr_enabled = true
nvt_enabled = true

[vault]
enabled = true
vault_path = "./savant-vault"
sync_interval_secs = 60
max_files = 15000
```

---

## Projected Growth

| Metric | Current | After Phase 1 | After Phase 2 | After Phase 3 |
|--------|---------|--------------|--------------|--------------|
| Knowledge files | 11 | 15 | 20 | 23 |
| Knowledge units | 141 | ~215 | ~295 | ~335 |
| Domains covered | 6 | 10 | 14 | 17 |
| MarketCondition tags | 15 | 22 | 22 | 22 |

---

## Execution Order

1. **Pull YouTube transcripts** — Start with Phase 1 on-chain videos (6 videos)
2. **Process into JSON** — Use existing knowledge JSON format
3. **Add new MarketCondition variants** to `knowledge.rs`
4. **Add on-chain insight module** (`src/insight/onchain.rs`)
5. **Wire into context builder** — Include on-chain metrics in AI prompt
6. **Test with `--dry-run`** — Verify new knowledge units get selected
7. **Repeat for Phase 2 and 3**

---

> **Next step:** Pick a Phase 1 file, pull the YouTube transcripts, and start processing.
