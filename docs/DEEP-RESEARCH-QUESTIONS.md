# Savant Trading — Deep Research Questions for Knowledge Expansion

> **Purpose:** Exhaustive question inventory for Gemini Deep Research sessions. Each question targets a specific gap in the current 141-unit knowledge base. Answers will be sourced from YouTube transcripts of top traders, then processed into new knowledge JSON files.

---

## Current State Summary

|Metric|Value|
|------|-----|
|Knowledge files|11|
|Knowledge units|141|
|Transcript sources|11 YouTube videos|
|Domains covered|Order flow, volume profile, SMC/ICT, scalping, macro, sentiment, AI bots, psychology|
|Primary markets|Crypto (Kraken), futures (NQ/ES), small-cap stocks|
|Exchange|Kraken only|

### Domains With Strong Coverage

- Smart Money Concepts / ICT (TJR — 19 units)
- Order flow & volume profile (Fabio — 15 units)
- Crypto fundamentals & sentiment (Full Course — 18 units)
- Trading psychology (Pradeep — 7 units)
- AI/quant system architecture (Claude Bot — 20 units)

### Domains With Weak or No Coverage

- On-chain analytics (MVRV, NUPL, SOPR, NVT)
- DeFi execution (DEX mechanics, MEV, liquidity pools)
- Crypto-specific order flow (exchange-level)
- Multi-exchange arbitrage
- Market making / liquidity provision
- Options / derivatives trading
- Whale tracking & wallet analysis
- Social sentiment (Twitter/X, Reddit, Telegram, Discord)
- Funding rate arbitrage
- Basis trade / cash-and-carry
- Correlation & pairs trading
- Volatility trading
- High-frequency / latency-sensitive strategies
- Crypto tax optimization
- Portfolio construction theory
- Backtesting methodology & pitfalls
- Live deployment engineering
- Risk of ruin mathematics
- Trade journaling frameworks
- Prop firm evaluation strategies
- Session-specific crypto strategies (Asian vs London vs NY)
- Layer 2 / on-chain trading
- MEV protection and exploitation
- Stablecoin de-peg trading
- Airdrop farming as portfolio yield
- Crypto-specific technical patterns (funding-driven, liquidation cascades)

---

## Research Domain 1: On-Chain Analytics & Whale Tracking

**Goal:** Add a new `onchain_analytics.json` knowledge file covering blockchain-native data that provides edge unavailable on traditional charts.

### Questions for Gemini — On-Chain Analytics

1. Who are the best YouTube educators explaining on-chain metrics for trading (MVRV, NUPL, SOPR, NVT, Stock-to-Flow)? Find channels with 50K+ subscribers who have specific trading setups using these metrics.

2. What are the top YouTube videos explaining how to read Bitcoin exchange inflows/outflows for trading signals? Include videos that show specific entry/exit rules based on exchange flow data.

3. Which YouTube traders demonstrate whale wallet tracking strategies? Find creators who show how to follow large BTC/ETH movements and use them as leading indicators.

4. What are the best YouTube tutorials on using Glassnode, CryptoQuant, or IntoTheBlock for active trading (not just analysis)? Find videos with specific trading rules.

5. Who on YouTube teaches the relationship between funding rates and price direction? Find traders who use funding rate extremes as contrarian signals with specific entry criteria.

6. What YouTube content covers open interest analysis for crypto trading? Find videos explaining how OI changes predict liquidation cascades and price reversals.

7. Which YouTube educators explain long/short ratio trading strategies? Find specific setups where extreme positioning predicts reversals.

8. What are the best YouTube videos on liquidation heatmap trading? Find creators who use CoinGlass or similar tools to predict price targets based on liquidation clusters.

9. Who on YouTube teaches Bitcoin dominance (BTC.D) trading strategies with specific entry/exit rules? Find videos beyond just "alt season is coming."

10. What YouTube content covers the relationship between stablecoin supply (USDT/USDC minting) and crypto market direction? Find traders who use stablecoin flows as leading indicators.

11. Which YouTube creators explain Miner Revenue / Hash Ribbon / Difficulty Ribbon as trading signals? Find videos with backtested results or specific rules.

12. What are the best YouTube videos on using on-chain data to identify market tops and bottoms? Find creators with specific checklists (e.g., "when 3+ of these 5 signals fire, it's a top").

---

## Research Domain 2: Crypto-Specific Execution & Microstructure

**Goal:** Add a new `crypto_execution.json` knowledge file covering exchange-specific mechanics, order types, and execution optimization unique to crypto markets.

### Questions for Gemini — Crypto Execution

1. Who are the best YouTube educators explaining crypto order book dynamics and how to read Level 2 data on Binance/Bybit/Kraken? Find traders who use order book imbalance for entries.

2. What YouTube videos explain the differences between crypto exchange fee structures (maker/taker, VIP tiers, BNB discounts) and how to optimize trading costs? Find creators who calculate break-even win rates based on fees.

3. Which YouTube traders teach crypto-specific order types (iceberg orders, TWAP, VWAP execution, post-only, reduce-only)? Find videos with practical examples.

4. What are the best YouTube videos explaining how to trade crypto during high-volatility events (FOMC, CPI, halvings, exchange hacks)? Find creators with specific rules for these scenarios.

5. Who on YouTube teaches crypto scalping specifically (not futures, not stocks — crypto spot/perp scalping)? Find creators with documented track records and specific timeframes.

6. What YouTube content covers the mechanics of crypto perpetual futures vs spot trading? Find videos explaining when to use each, funding rate implications, and basis trading.

7. Which YouTube educators explain how to use multiple crypto exchanges for best execution? Find videos covering liquidity fragmentation, price discrepancies, and arbitrage.

8. What are the best YouTube videos on crypto market making as a strategy? Find creators who explain spread capture, inventory management, and hedging.

9. Who on YouTube teaches crypto grid trading with specific parameters? Find backtested strategies, not just "set and forget" tutorials.

10. What YouTube content covers trading crypto around exchange-specific events (Binance listings, Coinbase announcements, Kraken delistings)? Find creators with specific event-driven strategies.

---

## Research Domain 3: Advanced Risk Management & Position Sizing

**Goal:** Expand `risk_management.json` (new file) covering mathematical risk frameworks, portfolio-level risk, and advanced position sizing beyond the current 1%-per-trade rule.

### Questions for Gemini — Risk Management

1. Who are the best YouTube educators explaining Kelly Criterion for trading position sizing? Find videos with practical crypto/futures examples and the fractional Kelly approach.

2. What YouTube videos explain risk of ruin calculations for day traders? Find creators who show how to calculate the probability of blowing up an account given win rate and R:R.

3. Which YouTube traders teach correlation-based position sizing? Find videos explaining how to reduce size when holding correlated assets (e.g., BTC + ETH simultaneously).

4. What are the best YouTube videos on drawdown recovery mathematics? Find content explaining why a 50% drawdown requires 100% gain to recover, and strategies for recovery.

5. Who on YouTube teaches portfolio heat management (total open risk across all positions)? Find creators with specific rules like "never more than 5% total portfolio risk at once."

6. What YouTube content covers volatility-adjusted position sizing (ATR-based sizing)? Find videos showing how to reduce size in high-volatility regimes and increase in low-volatility.

7. Which YouTube educators explain the concept of "edge" quantitatively? Find videos showing how to calculate expected value, edge per trade, and how many trades needed for statistical significance.

8. What are the best YouTube videos on trailing stop strategies? Find creators who compare ATR trailing, chandelier exits, percentage trailing, and structure-based trailing with backtest results.

9. Who on YouTube teaches scale-out strategies with specific rules? Find videos beyond "take partial profits" — need exact percentages, R:R triggers, and how to manage the runner.

10. What YouTube content covers anti-martingale vs martingale position sizing for traders? Find videos with backtested comparisons and when each is appropriate.

---

## Research Domain 4: Trading Psychology & Performance Optimization

**Goal:** Expand psychology coverage (currently only 9 units across 2 files) with a new `trading_psychology.json` covering cognitive biases, tilt management, peak performance, and trader development frameworks.

### Questions for Gemini — Trading Psychology

1. Who are the best YouTube trading psychologists / performance coaches? Find channels run by licensed psychologists who specialize in trader performance (not just motivational content).

2. What YouTube videos explain specific cognitive biases that affect traders (recency bias, anchoring, confirmation bias, loss aversion, disposition effect)? Find content with actionable debiasing techniques.

3. Which YouTube traders teach tilt management with specific protocols? Find videos beyond "just stay calm" — need concrete steps like position reduction rules, forced breaks, journaling triggers.

4. What are the best YouTube videos on building and following a trading plan? Find creators who provide actual templates and show how to review plan adherence.

5. Who on YouTube teaches deliberate practice for trading? Find content explaining how to structure practice sessions, review trades, and systematically improve specific skills.

6. What YouTube content covers the psychology of prop firm evaluations? Find videos addressing the specific mental challenges of trading under drawdown rules and profit targets.

7. Which YouTube educators explain the stages of trader development (unconscious incompetence → conscious incompetence → conscious competence → unconscious competence)? Find content with specific exercises for each stage.

8. What are the best YouTube videos on trading journal analysis? Find creators who show how to extract actionable insights from trade history (not just logging P&L).

9. Who on YouTube teaches sleep, exercise, and lifestyle optimization for trading performance? Find evidence-based content, not bro-science.

10. What YouTube content covers the psychology of holding winners and cutting losers? Find specific mental frameworks and rules that address this core challenge.

---

## Research Domain 5: Backtesting, Optimization & Live Deployment

**Goal:** Add a new `backtesting_deployment.json` covering walk-forward analysis, overfitting prevention, paper-to-live transition, and live deployment engineering.

### Questions for Gemini — Backtesting & Deployment

1. Who are the best YouTube educators explaining walk-forward optimization for trading strategies? Find videos with specific software examples (TradingView, Python, Amibroker).

2. What YouTube videos explain how to avoid curve-fitting / overfitting in backtesting? Find creators who demonstrate with examples of strategies that backtest well but fail live.

3. Which YouTube traders teach Monte Carlo simulation for trading strategy validation? Find videos showing how to stress-test strategies with randomized trade sequences.

4. What are the best YouTube videos on backtesting crypto trading strategies specifically? Find content addressing crypto-specific challenges (exchange differences, funding rates, 24/7 markets).

5. Who on YouTube teaches the paper-to-live trading transition? Find videos with specific milestones and rules for when to switch from paper to real money.

6. What YouTube content covers live trading system monitoring? Find creators who explain what to watch for in real-time (drawdown curves, execution quality, slippage tracking).

7. Which YouTube educators explain strategy degradation and when to stop trading a system? Find videos with specific triggers (e.g., "if drawdown exceeds 2x historical max, pause").

8. What are the best YouTube videos on building a trading journal in Notion/Excel/Google Sheets? Find templates that track not just P&L but also process metrics (plan adherence, setup quality).

9. Who on YouTube teaches automated trading system deployment (VPS setup, monitoring, alerting)? Find practical DevOps-for-traders content.

10. What YouTube content covers multi-strategy portfolio construction? Find videos explaining how to combine uncorrelated strategies for smoother equity curves.

---

## Research Domain 6: DeFi & On-Chain Trading

**Goal:** Add a new `defi_trading.json` covering DEX mechanics, MEV, liquidity pools, and on-chain trading strategies that complement the CEX-focused knowledge.

### Questions for Gemini — DeFi Trading

1. Who are the best YouTube educators explaining DEX trading (Uniswap, dYdX, Jupiter, Hyperliquid)? Find creators with specific strategies, not just tutorials.

2. What YouTube videos explain MEV (Maximal Extractable Value) and how traders can protect themselves from sandwich attacks? Find practical content with specific tools (Flashbots, private mempools).

3. Which YouTube traders teach liquidity pool strategies (providing liquidity, impermanent loss management, concentrated liquidity)? Find videos with specific yield optimization strategies.

4. What are the best YouTube videos on on-chain perpetual trading (dYdX, GMX, Hyperliquid)? Find creators who compare execution quality vs centralized exchanges.

5. Who on YouTube teaches wallet tracking and smart money following? Find content using tools like Arkham, Nansen, or DeBank to follow profitable wallets.

6. What YouTube content covers airdrop farming as a portfolio strategy? Find videos with specific protocols, expected yields, and risk management.

7. Which YouTube educators explain stablecoin yield strategies (lending, LP, basis trade)? Find content with risk-adjusted return comparisons.

8. What are the best YouTube videos on on-chain order flow analysis? Find creators who read blockchain transactions to identify accumulation/distribution patterns.

---

## Research Domain 7: Multi-Timeframe & Session-Based Trading

**Goal:** Add a new `session_trading.json` covering crypto session dynamics, multi-timeframe alignment, and time-based edge.

### Questions for Gemini — Session Trading

1. Who are the best YouTube educators explaining crypto session dynamics (Asian session, London session, NY session) with specific trading rules for each? Find creators who quantify edge by session.

2. What YouTube videos explain how to trade the crypto London open specifically? Find content with specific setups for the 7:00-9:00 UTC window.

3. Which YouTube traders teach the relationship between traditional market opens (US equities) and crypto price action? Find videos explaining spillover effects and correlation trades.

4. What are the best YouTube videos on multi-timeframe analysis for crypto? Find creators who show exactly how to align 4H bias, 1H structure, and 5M/15M execution.

5. Who on YouTube teaches trading around Bitcoin's weekly open (Sunday 00:00 UTC)? Find content with specific setups and historical edge data.

6. What YouTube content covers the "crypto dead zone" (low-volume periods) and how to avoid or exploit them? Find specific time windows and strategies.

7. Which YouTube educators explain how to trade the BTC/ETH ratio? Find videos on when altcoins outperform Bitcoin and how to rotate.

8. What are the best YouTube videos on day-of-week and time-of-day patterns in crypto? Find creators with backtested data showing which days/times have highest edge.

---

## Research Domain 8: Macro & Cross-Asset Analysis for Crypto

**Goal:** Expand macro coverage with a new `macro_cross_asset.json` covering traditional market signals that drive crypto, DXY correlation, bond yields, and global liquidity.

### Questions for Gemini — Macro & Cross-Asset

1. Who are the best YouTube educators explaining the DXY (Dollar Index) and its inverse correlation with Bitcoin? Find traders with specific rules (e.g., "when DXY breaks X, BTC does Y").

2. What YouTube videos explain how US Treasury yields affect crypto prices? Find content with specific yield thresholds that trigger crypto rallies or selloffs.

3. Which YouTube traders teach the relationship between the S&P 500 / NASDAQ and Bitcoin? Find videos with correlation data and specific cross-asset trading rules.

4. What are the best YouTube videos on global liquidity cycles and crypto? Find creators who track M2 money supply, central bank balance sheets, and their impact on crypto.

5. Who on YouTube teaches FOMC/CPI/NFP event trading for crypto? Find content with specific rules for trading around these announcements (not just "be careful").

6. What YouTube content covers the Bitcoin halving cycle with price data and trading rules? Find videos that go beyond "number go up" to specific accumulation/distribution phases.

7. Which YouTube educators explain how to use the Fear & Greed Index as a trading tool? Find content with specific contrarian entry rules backed by historical data.

8. What are the best YouTube videos on crypto market regime identification? Find creators who distinguish between accumulation, markup, distribution, and decline phases with specific indicators.

---

## Research Domain 9: Technical Analysis Specific to Crypto

**Goal:** Add a new `crypto_technicals.json` covering crypto-specific technical patterns, indicator settings, and chart reading that differs from traditional markets.

### Questions for Gemini — Crypto Technicals

1. Who are the best YouTube educators explaining crypto-specific chart patterns (funding-driven ranges, liquidation wicks, exchange-driven pumps)? Find content that explains patterns unique to 24/7 crypto markets.

2. What YouTube videos explain optimal indicator settings for crypto (not stocks)? Find creators who test different EMA/RSI/MACD periods specifically on BTC/ETH and show results.

3. Which YouTube traders teach Volume Profile specifically for crypto? Find videos with crypto-specific settings and edge cases (weekend gaps, low-liquidity periods).

4. What are the best YouTube videos on using the Bitcoin dominance chart (BTC.D) for altcoin trading? Find creators with specific entry/exit rules based on dominance shifts.

5. Who on YouTube teaches Ethereum-specific technical analysis? Find content covering ETH/BTC ratio, gas fees as sentiment, and ETH-specific patterns.

6. What YouTube content covers the ETH/BTC ratio as a market indicator? Find videos explaining when to rotate between BTC and alts based on this ratio.

7. Which YouTube educators explain how to read crypto order flow without expensive tools? Find content using free tools (exchange APIs, open interest, funding rate) as proxies.

8. What are the best YouTube videos on using open interest + price action together for crypto entries? Find creators who show the specific combinations (rising OI + rising price = bullish, etc.).

---

## Research Domain 10: AI & Automation in Trading

**Goal:** Expand AI/automation coverage (currently 30 units across 2 files) with new `ai_automation.json` covering LLM-based trading, prompt engineering for markets, and automated execution.

### Questions for Gemini — AI & Automation

1. Who are the best YouTube educators building trading bots with AI/LLM models? Find creators who show actual implementations (not just theory) with GPT, Claude, or open-source models.

2. What YouTube videos explain prompt engineering for trading decisions? Find content showing how to structure market context for LLMs to get reliable trading signals.

3. Which YouTube traders teach building automated crypto trading bots with Python? Find creators with open-source repos and documented backtest results.

4. What are the best YouTube videos on using TradingView Pine Script for automated strategy testing? Find creators who build and backtest complete strategies on camera.

5. Who on YouTube teaches using machine learning for market prediction? Find content that's honest about limitations and shows realistic (not curve-fit) results.

6. What YouTube content covers building a trading dashboard with real-time data? Find videos covering WebSocket integration, charting libraries, and portfolio tracking.

7. Which YouTube educators explain algorithmic execution strategies (TWAP, VWAP, iceberg)? Find content specific to crypto exchange APIs.

8. What are the best YouTube videos on using sentiment analysis (NLP) for crypto trading? Find creators who scrape Twitter/Reddit/news and generate trading signals.

---

## Research Domain 11: Crypto Derivatives & Structured Products

**Goal:** Add a new `crypto_derivatives.json` covering options, structured products, and advanced derivatives strategies unique to crypto.

### Questions for Gemini — Crypto Derivatives

1. Who are the best YouTube educators explaining crypto options trading (Deribit, OKX)? Find creators who teach specific strategies (covered calls, protective puts, straddles).

2. What YouTube videos explain the options Greeks (delta, gamma, theta, vega) with crypto-specific examples? Find content that applies to BTC/ETH options specifically.

3. Which YouTube traders teach the funding rate arbitrage strategy (spot long + perp short)? Find videos with specific entry/exit rules and yield calculations.

4. What are the best YouTube videos on basis trading (cash-and-carry) in crypto? Find creators who explain when the basis is attractive and how to execute.

5. Who on YouTube teaches crypto volatility trading? Find content on trading the VIX equivalent for crypto (DVOL) or using options to trade volatility.

6. What YouTube content covers structured products in crypto (shark fins, dual currency, range bonds)? Find videos explaining these for active traders, not just passive investors.

7. Which YouTube educators explain how to use perpetual futures funding rates as a yield strategy? Find content with specific risk management for the carry trade.

8. What are the best YouTube videos on liquidation cascade trading? Find creators who identify and trade liquidation-driven price moves with specific entry criteria.

---

## Research Domain 12: Altcoin Selection & Narrative Trading

**Goal:** Add a new `altcoin_selection.json` covering how to identify, evaluate, and trade altcoins beyond BTC/ETH.

### Questions for Gemini — Altcoin Selection

1. Who are the best YouTube educators explaining how to evaluate altcoin fundamentals (tokenomics, team, TVL, revenue, community)? Find creators with specific frameworks, not just shills.

2. What YouTube videos explain narrative/sector rotation in crypto (AI tokens, RWA, DePIN, meme coins)? Find traders who identify emerging narratives early and rotate positions.

3. Which YouTube traders teach how to trade meme coins profitably? Find content with risk management rules for high-volatility, low-liquidity tokens.

4. What are the best YouTube videos on using on-chain data to find low-cap gems before they pump? Find creators who show specific tools (DEXScreener, Birdeye, DexTools).

5. Who on YouTube teaches the "listing effect" — trading tokens before/after major exchange listings? Find content with historical data on price impact.

6. What YouTube content covers how to evaluate crypto project tokenomics (vesting schedules, unlock dates, inflation rates)? Find videos showing how unlocks affect price.

7. Which YouTube educators explain DeFi yield farming with specific risk management? Find content covering impermanent loss, smart contract risk, and rug pull identification.

8. What are the best YouTube videos on trading crypto ecosystems (Solana ecosystem, Base ecosystem, Cosmos ecosystem)? Find creators who explain ecosystem-level dynamics.

---

## Research Domain 13: Kraken-Specific Knowledge

**Goal:** Add a new `kraken_specific.json` covering Kraken's unique features, fee structure, order types, and optimal usage.

### Questions for Gemini — Kraken

1. Who are the best YouTube educators explaining Kraken exchange specifically? Find creators who cover Kraken Pro interface, advanced order types, and fee optimization.

2. What YouTube videos explain Kraken's fee structure in detail (maker/taker tiers, volume discounts, KRAKEN fee token)? Find content calculating optimal trading size for fee tiers.

3. Which YouTube traders teach Kraken's margin trading and futures? Find videos with specific risk management rules for Kraken's margin system.

4. What are the best YouTube videos on using Kraken's API for automated trading? Find creators who show Python/Rust implementations with specific endpoints.

5. Who on YouTube teaches Kraken's staking and earn products as portfolio components? Find content comparing yields and risks.

6. What YouTube content covers Kraken-specific quirks (order minimums, withdrawal limits, verification tiers)? Find practical tips for optimizing Kraken usage.

---

## Research Domain 14: Prop Firm & Funded Account Strategies

**Goal:** Add a new `prop_firm.json` covering evaluation strategies, risk rules, and scaling with funded accounts.

### Questions for Gemini — Prop Firms

1. Who are the best YouTube traders who have passed multiple prop firm evaluations? Find creators who show their actual trades and explain their approach.

2. What YouTube videos explain the specific rules of major prop firms (FTMO, TopStep, Apex, MyFundedFX)? Find content comparing rules and which firms are best for which strategies.

3. Which YouTube educators teach strategies specifically optimized for prop firm evaluations? Find content addressing profit targets, drawdown rules, and minimum trading days.

4. What are the best YouTube videos on managing psychology during prop firm evaluations? Find creators who address the specific pressure of trading with someone else's rules.

5. Who on YouTube teaches scaling from prop firm accounts to personal accounts? Find content on transitioning from constrained evaluation trading to unrestricted live trading.

6. What YouTube content covers crypto-specific prop firms? Find videos evaluating firms that offer crypto perp trading with funded accounts.

---

## Research Domain 15: Trade Management & Exit Strategies

**Goal:** Add a new `trade_management.json` covering the most underdeveloped area — how to manage open positions and optimize exits.

### Questions for Gemini — Trade Management

1. Who are the best YouTube educators explaining trailing stop strategies with backtested results? Find creators who compare ATR trailing, structure-based trailing, and time-based exits.

2. What YouTube videos explain how to manage trades around news events? Find content with specific rules for holding through, reducing before, or closing ahead of catalysts.

3. Which YouTube traders teach the "free risk" technique (moving stop to entry after partial take-profit)? Find videos with specific R:R triggers and management rules.

4. What are the best YouTube videos on managing winning trades (when to hold, when to add, when to scale out)? Find creators with specific rules beyond "let winners run."

5. Who on YouTube teaches time-based exits (closing positions after X bars regardless of target)? Find content with backtested comparisons of time-based vs target-based exits.

6. What YouTube content covers managing multiple simultaneous positions? Find videos on correlation management, total portfolio heat, and prioritization when all positions are live.

7. Which YouTube educators explain how to re-enter after being stopped out? Find content with specific rules for re-entry (same setup, different setup, or move on).

8. What are the best YouTube videos on break-even stop strategies? Find creators who test whether moving to break-even improves or hurts overall performance.

---

## Research Domain 16: Market Making & Liquidity Strategies

**Goal:** Add a new `market_making.json` covering liquidity provision, spread capture, and market microstructure.

### Questions for Gemini — Market Making

1. Who are the best YouTube educators explaining market making concepts for retail traders? Find content on spread capture, inventory management, and adverse selection.

2. What YouTube videos explain how to read and use Level 3 order book data? Find creators who show how to identify hidden liquidity and large resting orders.

3. Which YouTube traders teach the "spoofing" and "layering" patterns in order books? Find content on identifying and avoiding manipulation (not perpetrating it).

4. What are the best YouTube videos on using iceberg order detection for trading signals? Find creators who show how to identify hidden orders and trade around them.

5. Who on YouTube teaches the relationship between volume and price at the tick level? Find content on reading the tape (time and sales) for crypto specifically.

---

## Research Domain 17: Crypto Cycles & Regime Analysis

**Goal:** Add a new `crypto_cycles.json` covering crypto-specific market cycles, halving dynamics, and regime identification.

### Questions for Gemini — Crypto Cycles

1. Who are the best YouTube educators explaining Bitcoin's 4-year halving cycle with specific trading rules for each phase? Find content beyond "buy the halving" — need accumulation, markup, distribution, decline rules.

2. What YouTube videos explain how to identify the current crypto market cycle phase? Find creators with specific indicators or checklists (not just vibes).

3. Which YouTube traders teach the altcoin market cycle (BTC season → ETH season → alt season → crash)? Find videos with specific rotation rules and timing indicators.

4. What are the best YouTube videos on Bitcoin's stock-to-flow model and its limitations? Find honest assessments, not just hopium.

5. Who on YouTube teaches how to trade crypto winter / bear markets? Find content with specific strategies for capital preservation and opportunistic buying.

6. What YouTube content covers the relationship between Bitcoin halving and altcoin performance? Find data-driven analysis of which sectors outperform post-halving.

---

## Research Domain 18: Execution Engineering & Latency

**Goal:** Add a new `execution_engineering.json` covering low-latency execution, order routing, and exchange connectivity.

### Questions for Gemini — Execution Engineering

1. Who are the best YouTube educators explaining co-location and low-latency trading for crypto? Find content on VPS placement, exchange matching engine locations, and latency measurement.

2. What YouTube videos explain WebSocket vs REST API performance for crypto trading? Find creators who benchmark latency and show optimal connection strategies.

3. Which YouTube traders teach order routing optimization across multiple crypto exchanges? Find content on smart order routing and best execution.

4. What are the best YouTube videos on building a reliable crypto trading infrastructure (reconnection logic, heartbeat monitoring, failover)? Find production-grade content, not toy examples.

5. Who on YouTube teaches rate limit management for crypto exchange APIs? Find content with specific strategies for Kraken, Binance, and other exchanges.

---

## Research Domain 19: Regulatory, Tax & Compliance

**Goal:** Add a new `compliance.json` covering tax optimization, regulatory awareness, and compliance for active crypto traders.

### Questions for Gemini — Compliance & Tax

1. Who are the best YouTube educators explaining crypto tax implications for active traders? Find content covering wash sale rules, FIFO/LIFO, and tax-loss harvesting specifically for crypto.

2. What YouTube videos explain how to track crypto trades for tax reporting? Find comparisons of Koinly, CoinTracker, TokenTax, and other tools.

3. Which YouTube traders teach tax-loss harvesting strategies specific to crypto? Find content with specific rules and timing strategies.

4. What are the best YouTube videos on crypto trading business structures (LLC, S-Corp)? Find content on entity structuring for tax optimization.

5. Who on YouTube explains the regulatory landscape for crypto trading across different jurisdictions? Find content covering US, EU, and Asian regulatory differences.

---

## Research Domain 20: Advanced Price Action & Pattern Recognition

**Goal:** Add a new `advanced_price_action.json` covering patterns and price action concepts not yet in the knowledge base.

### Questions for Gemini — Advanced Price Action

1. Who are the best YouTube educators explaining Wyckoff accumulation and distribution schematics for crypto? Find content with specific stage identification rules and historical BTC examples.

2. What YouTube videos explain market profile (TPO charts) for crypto trading? Find creators who show how to use time-at-price for intraday entries.

3. Which YouTube traders teach footprint chart reading for crypto? Find content on delta divergence, cumulative delta, and stacked imbalance detection.

4. What are the best YouTube videos on harmonic patterns (Gartley, butterfly, crab) for crypto? Find creators who show backtested results and specific Fibonacci ratios.

5. Who on YouTube teaches the Elliott Wave Principle for crypto with specific counting rules? Find content that's practical, not theoretical.

6. What YouTube content covers divergence trading (regular, hidden, extended) with specific entry rules? Find creators who compare RSI, MACD, and volume divergence.

7. Which YouTube educators explain supply and demand zones with specific identification criteria? Find content distinguishing S&D zones from simple support/resistance.

---

## Priority Ranking for Research Sessions

### Tier 1 — Highest Impact (addresses biggest knowledge gaps)

|#|Domain|New File|Est. Units|Rationale|
|-|------|--------|----------|---------|
|1|On-Chain Analytics|`onchain_analytics.json`|15-20|Completely missing, crypto-native edge|
|2|Trade Management|`trade_management.json`|12-15|Most underdeveloped area across all files|
|3|Crypto Execution|`crypto_execution.json`|12-15|Exchange-specific knowledge critical for Kraken bot|
|4|Altcoin Selection|`altcoin_selection.json`|10-15|Currently only BTC/ETH, need expansion|
|5|Advanced Risk|`risk_management.json`|10-12|Current risk knowledge is basic (1% rule only)|

### Tier 2 — High Impact (fills important gaps)

|#|Domain|New File|Est. Units|Rationale|
|-|------|--------|----------|---------|
|6|Trading Psychology|`trading_psychology.json`|12-15|Only 9 units currently, needs depth|
|7|Macro Cross-Asset|`macro_cross_asset.json`|10-12|DXY, yields, liquidity — drives crypto|
|8|Crypto Cycles|`crypto_cycles.json`|8-10|Halving cycle, regime identification|
|9|DeFi Trading|`defi_trading.json`|8-10|On-chain edge, MEV, DEX|
|10|Backtesting|`backtesting_deployment.json`|8-10|Paper-to-live transition critical|

### Tier 3 — Medium Impact (nice to have)

|#|Domain|New File|Est. Units|Rationale|
|-|------|--------|----------|---------|
|11|Session Trading|`session_trading.json`|8-10|Time-based edge|
|12|Crypto Derivatives|`crypto_derivatives.json`|8-10|Options, structured products|
|13|AI Automation|`ai_automation.json`|6-8|Already have 30 units, expand|
|14|Kraken Specific|`kraken_specific.json`|6-8|Exchange-specific optimization|
|15|Prop Firms|`prop_firm.json`|6-8|Evaluation strategies|
|16|Advanced Price Action|`advanced_price_action.json`|8-10|Wyckoff, footprint, harmonic|
|17|Market Making|`market_making.json`|6-8|Microstructure|
|18|Execution Engineering|`execution_engineering.json`|5-6|Infrastructure|
|19|Compliance|`compliance.json`|4-5|Tax, regulatory|
|20|AI Automation (expand)|existing file|6-8|LLM trading, prompt engineering|

---

## Projected Knowledge Base Growth

|Metric|Current|After Tier 1|After Tier 2|After Tier 3|
|------|-------|------------|------------|------------|
|Knowledge files|11|16|21|31|
|Knowledge units|141|210-235|280-315|370-420|
|Domains covered|6|11|15|20+|

---

## How to Use This Document

1. **Pick a domain** from the priority table
2. **Run Gemini Deep Research** with the questions from that domain section
3. **Find the best YouTube videos** recommended by Gemini
4. **Pull transcripts** from those videos
5. **Process into knowledge JSON** using the existing format
6. **Add to `/knowledge/` directory** — engine loads automatically
7. **Update this document** to mark completed domains

### Knowledge JSON Format Reference

```json
{
  "id": "domain-topic-001",
  "title": "Knowledge unit title",
  "topic": "TechnicalAnalysis",
  "content": "Detailed trading knowledge...",
  "conditions": ["Trending", "HighVolatility"],
  "source": "YouTube Creator Name — Video Title",
  "confidence": 0.85
}
```

### Available Condition Tags

|Tag|When to Use|
|---|-----------|
|`Trending`|ADX > 25, clear directional movement|
|`Ranging`|ADX < 20, consolidation|
|`HighVolatility`|ATR > 1.5x average|
|`LowVolatility`|ATR < 0.7x average|
|`ExtremeFear`|Fear & Greed < 25|
|`ExtremeGreed`|Fear & Greed > 75|
|`BtcDominant`|BTC.D rising|
|`AltSeason`|BTC.D falling, alts outperforming|
|`SessionOpen`|Major session open (London/NY)|
|`BreakingNews`|Major catalyst event|
|`FundingExtreme`|Funding rate > ±0.1%|
|`LiquidationCluster`|Large liquidation levels nearby|
|`MacroEvent`|FOMC, CPI, NFP, halving|

---

> **Note:** This document is a living research roadmap. Update after each Gemini Deep Research session with findings, completed questions, and new gaps discovered.
