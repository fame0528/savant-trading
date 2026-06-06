# **Autonomous Detection and Exploitation of Cryptocurrency Liquidation Cascades: A Systems and Data Architecture Report**

## **1\. The Market Microstructure of Liquidation Cascades**

The cryptocurrency derivatives market is characterized by highly leveraged speculative positioning, fragmented liquidity across centralized and decentralized venues, and deterministic, automated liquidation engines. Within this environment, the liquidation cascade represents a structural market failure that reliably produces the single highest-edge mean-reversion setup in algorithmic trading. Understanding the algorithmic anatomy of these cascades is the foundational prerequisite for designing an autonomous execution system, particularly one constrained by a micro-capital budget of $26.

### **1.1 Algorithmic Anatomy and Mathematical Triggers**

The mechanics of a liquidation cascade rely heavily on the interaction between exchange matching engines (or smart contracts in decentralized finance), real-time price oracles, and automated liquidator bots.1 Every leveraged position maintains a collateralization ratio, frequently referred to as a "health factor." This mathematical value represents the safety of the margin loan based on the current collateral value compared to the borrowed notional amount.1  
As asset prices fall, the health factor decreases. The exact formulation of the liquidation price varies by venue, but the fundamental mathematical constraint on platforms such as Hyperliquid and GMX V2 is defined by the following relationship:  
![][image1]  
where ![][image2].2 For assets with margin tiers, the maintenance leverage depends on the unique margin tier corresponding to the position value at the liquidation price.2  
Once the health factor drops below a critical threshold (typically a value of one), the position becomes eligible for forced closure. On decentralized protocols, automated liquidator bots constantly monitor these health factors across the blockchain state.1 When a position becomes undercollateralized, these bots race to trigger the liquidation function within the protocol's smart contract, repaying a portion or the entirety of the borrower's debt.1 In exchange, the liquidator receives the borrower's collateral along with a liquidation penalty fee as a financial incentive.1 To realize a profit, the liquidator immediately sells the seized collateral into the open market via decentralized exchanges or central limit order books (CLOBs).1  
If the initial wave of liquidations is large, the sudden influx of automated, price-agnostic sell orders consumes available buy orders in the liquidity pools.1 This aggressive consumption of liquidity causes severe slippage and drives the spot price of the asset lower. The updated, lower price is then reported back to the lending protocol by an oracle.1 This new price data instantly reduces the health factors of other active loans, pushing them below the liquidation threshold.1 This creates a deterministic, mechanical feedback loop—a liquidation cascade—that continues until the selling pressure exhausts itself against a sufficiently dense wall of limit buy orders or market makers step in to absorb the supply.1

### **1.2 Duration, Magnitude, and Contagion Mechanics**

The speed of these cascades in automated systems means human intervention is often impossible, with entire events completing before manual traders or discretionary portfolio managers can react.3 What might start as a manageable technical correction becomes a systemic crisis through pure mechanical amplification.3 A cascade that begins in one protocol can rapidly spread through lending markets, affect collateral values in other protocols, and trigger ecosystem-wide liquidations, creating a contagion effect.3  
The duration of a liquidation cascade typically ranges from a few minutes to several hours, depending heavily on the macroeconomic backdrop, the time of day, and the depth of the order book.4 However, the most violent phase of the cascade—the final capitulation wick where the majority of the open interest is purged—often completes within a tightly confined 1-to-5-minute candle window. During this specific window, extreme volume spikes occur. For example, historical market data indicates that during localized altcoin cascades, volume can spike from a baseline of 19,000 units to over 278,000 units in a single hourly or minute candle, serving as a clear, quantitative sign of forced, non-economic selling.7  
The structural aftermath of a cascade leaves the market fundamentally altered: open interest is violently purged, meaning fewer overleveraged positions are left to fuel further downside volatility.4 A massive drop in open interest signals that a large number of margined positions have been wiped out rather than voluntarily closed, providing the necessary structural reset for a high-probability reversal.4

## **2\. Liquidation Data Acquisition Strategy**

Acquiring accurate, tick-by-tick liquidation data is notoriously difficult because major exchanges actively obfuscate the true granularity of forced closures to protect market stability, prevent adversarial trading strategies, and optimize their own data streams.8 The trading agent must navigate these limitations while adhering to a strict $26 budget, which precludes the use of expensive enterprise data feeds.

### **2.1 Exchange WebSocket Streams and Limitations**

Binance provides the @forceOrder WebSocket stream, which pushes forced liquidation order information for specific symbols or across all markets.9 However, the architectural limitation of this stream is severe: for each symbol, Binance only pushes the *largest* single liquidation order within a 1000ms (1-second) interval as a snapshot.9 If no liquidation happens in that interval, no stream is pushed.9 Consequently, if a cascade triggers 500 separate liquidations in a single second, the WebSocket will only broadcast one event, meaning the stream is a vast underrepresentation of actual liquidation volumes.8  
The payload schema for a Binance liquidation event is structured as follows:

| Field | Type | Description |
| :---- | :---- | :---- |
| e | String | Event Type (forceOrder) |
| s | String | Symbol (e.g., BTCUSDT) |
| S | String | Side (SELL or BUY) |
| o | String | Order Type (LIMIT) |
| q | String | Original Quantity |
| p | String | Price |
| X | String | Order Status (FILLED) |

9  
Bybit offers a slightly more granular feed via the allLiquidation.{symbol} WebSocket topic, which covers USDT, USDC, and Inverse contracts.11 Bybit's push frequency is 500ms, but it suffers from similar aggregation logic implemented in mid-2021 to "optimize user data streams".8 When a long position is liquidated, the stream broadcasts a Buy side execution.11 The data object contains the executed size (v) and the bankruptcy price (p).11

### **2.2 Coinglass and Liquidation Heatmaps**

Liquidation heatmaps are highly effective for predicting where price will be magnetically attracted, as they map clusters of highly leveraged stop-losses and liquidation thresholds. Coinglass provides the industry standard for these heatmaps.12  
However, programmatic access to Coinglass data presents a significant financial hurdle. The Coinglass API operates on a tiered subscription model.13 The Hobbyist plan costs $29/month and allows 30 requests per minute, while the Startup plan costs $79/month for 80 requests per minute.13 Given the $26 total capital constraint of the trading agent, utilizing a paid Coinglass API is mathematically impossible.  
An alternative approach involves using web scraping actors, such as the Apify Coinglass Liquidation Heatmap actor, which captures high-resolution screenshots of the heatmaps programmatically.14 This service costs $6.00 per 1,000 results.14 While cheaper, processing image data into actionable numerical thresholds via an LLM or OCR adds significant latency and processing overhead, making it unsuitable for sub-second execution. Therefore, the system must abandon third-party heatmaps and instead natively infer liquidation clusters by tracking cumulative volume and open interest buildup at specific price levels over time.

## **3\. Open Interest and Funding Rate Dynamics**

Because explicit liquidation feeds are aggregated and thus unreliable for calculating the absolute magnitude of a cascade 8, Open Interest (OI) delta serves as the primary ground-truth metric. A rapid reduction in OI perfectly correlates with the destruction of leveraged positions.4

### **3.1 Real-Time Open Interest Monitoring**

Open interest is the total number of outstanding derivative contracts that have not been settled.15 It reflects how speculative a market is and signals how prices may behave in the near term.15 For the trading agent, detecting an OI drop of 20% or more is the definitive confirmation that a cascade has structurally reset the market.  
Binance provides OI data via both REST (GET /fapi/v1/openInterest) and WebSocket streams.16 However, the Binance WebSocket stream underlying@optionOpenInterest@\<expirationDate\> updates only every 60 seconds.17 A 60-second delay is fatal during a cascade, as the V-recovery will likely have already occurred by the time the data arrives.  
Bybit provides a superior, highly granular open interest feed via its public ticker WebSocket channel (tickers.{symbol}).18 The payload dynamically updates the openInterest and openInterestValue fields in real-time, alongside turnover24h and volume24h, allowing the trading agent to track OI drops on a tick-by-tick basis.19  
Cross-exchange divergence is also a critical metric. By monitoring OI across Binance (which represents a massive retail cohort) and Hyperliquid (which represents decentralized, sophisticated on-chain capital), the agent can gauge the broad market participation in the cascade. A cascade is mathematically confirmed when the system detects a concurrent price velocity drop of \>5% combined with a real-time OI reduction of \>20% across multiple data streams.4

### **3.2 Funding Rate Telemetry**

Funding rates dictate the cost of holding leveraged positions and act as a leading indicator of market positioning. Polling funding rates from OKX once every cycle is entirely insufficient for high-frequency cascade detection. An extreme positive funding rate (e.g., \>0.05% or \>0.1% per 8 hours) indicates massive speculative long leverage, making the market highly susceptible to a downside cascade.12  
During a severe cascade, the funding rate will frequently flip from highly positive to highly negative as long positions are forcibly closed and opportunistic short sellers aggressively pile in at the absolute bottom. Monitoring the funding rate across multiple exchanges via the Bybit tickers.{symbol} channel (fundingRate and nextFundingTime fields) 20 or Hyperliquid's activeAssetCtx WebSocket subscription 21 provides necessary confluence for the exhaustion phase. The speed at which funding flips from positive to negative—often occurring in a matter of minutes as the index price diverges from the mark price due to heavy spot selling—signals that the market has transitioned from long-overleveraged to short-overleveraged, priming the conditions for a violent V-recovery upward.

## **4\. Algorithmic Detection and V-Recovery Confirmation**

Detecting a cascade in real-time requires synthesizing price velocity, volume spikes, and open interest destruction. The algorithm must differentiate between a standard market dip (which may grind downward slowly without purging leverage) and a true liquidation cascade.

### **4.1 Defining the Cascade in Progress**

A "cascade in progress" is quantitatively defined by three simultaneous conditions:

1. **Price Velocity:** The asset experiences a rapid downward movement exceeding 5% to 10% within a highly compressed timeframe (1 to 15 minutes).  
2. **Open Interest Destruction:** Total open interest for the specific pair drops by a minimum of 15% to 20%.4 This confirms that the price movement is driven by forced liquidations rather than voluntary, unleveraged spot selling.  
3. **Liquidation Event Spikes:** While the raw dollar value of the exchange liquidation feeds may be aggregated and inaccurate, the *frequency* of the broadcast events spikes exponentially. The system measures the rate of incoming forceOrder WebSocket messages; an increase of 500% above the 24-hour moving average serves as the trigger.7

Some advanced systems utilize Machine Learning and Anomaly Detection algorithms to flag these specific conditions. AI agents and machine learning models are well-suited to flag when price movements do not match typical patterns—for example, a sudden 50% drop with no corresponding macroeconomic news.22 However, for a lightweight Rust backend, deterministic mathematical thresholds are far more computationally efficient.

### **4.2 Detecting Cascade Exhaustion via Volume Profile**

The most critical phase of the strategy is identifying the exact moment of exhaustion to prevent catching a "falling knife." To confirm a V-recovery and rule out a "dead cat bounce," the system must analyze the market microstructure using a Volume Profile.  
The classic "b-shape" volume profile is the definitive signature of a long liquidation cascade exhaustion.23 The profile visually resembles a lowercase "b", featuring a thick volume cluster at the bottom of the price range and a thin profile at the top.23 This b-shape forms when a long, impulsive sell-off meets a massive wall of limit buy orders.23 The heavy transacted volume at the bottom indicates that forced market sells are being absorbed by algorithmic accumulators and market makers.7  
The main criteria for a successful exhaustion reversal include:

* A sudden, extreme change in volume relative to the preceding baseline.  
* A low-volume node rapidly converting into a high-volume node.  
* The immediate stabilization of price despite the continued influx of sell orders (absorption).25

When the market makers step in to absorb the forced selling at the absolute lows, they initiate a rapid V-recovery.7 This recovery occurs because the cascading market orders have cleared out all local liquidity on the way down, creating an order-book vacuum. Once the selling stops, the price magnetically snaps back up through the thin liquidity zones above. A genuine V-recovery is mathematically confirmed when the price closes above the midpoint (the Point of Control) of the b-shape volume profile, signaling that sellers have lost grip and the forced liquidation engine has halted.23  
The typical magnitude of a V-recovery is highly predictable. Because the cascade creates an artificial price vacuum, the subsequent mean-reversion typically retraces 50% to 61.8% (the golden Fibonacci ratio) of the entire cascade wick within minutes to hours of the exhaustion event.

## **5\. Execution Venue Evaluation: GMX V2 vs. Hyperliquid**

The choice of execution venue is the most critical architectural decision for an autonomous agent operating with exactly $26 in starting capital. The venue must support programmatic execution, provide sufficient leverage for altcoins (ARB, LINK, PEPE), and possess a fee structure that does not instantly erode the micro-account balance. The two premier decentralized perpetual exchanges—GMX V2 and Hyperliquid—offer drastically different technical paradigms.

### **5.1 GMX V2 Technical Architecture**

GMX V2 is a decentralized perpetual and spot exchange deployed primarily on the Arbitrum network.27 It utilizes a multi-asset liquidity pool model (GM tokens) rather than a traditional central limit order book, relying on Chainlink oracles to provide zero-slippage executions.28

#### **Smart Contract Integration**

Interacting with GMX V2 programmatically requires executing on-chain transactions via the Arbitrum network. To open a position, the agent must call the createOrder function on the ExchangeRouter contract.30 Because the protocol is asynchronous to prevent oracle front-running and read-only reentrancy attacks, order execution is a mandatory two-step process.31 First, the user submits a transaction with the request details. Second, off-chain "Keepers" listen for this event, fetch the latest oracle prices, and send a subsequent transaction to execute the request.31  
In a Rust environment, integrating with GMX V2 requires utilizing libraries like ethers-rs or its successor, alloy.34 The developer utilizes the abigen\! macro to generate type-safe Rust bindings from the GMX contract ABIs.34 A CreateOrderParams Solidity struct must be constructed and passed to the ExchangeRouter.30

#### **Constraints for Micro-Capital**

While GMX V2 offers up to 50x leverage on major assets 37, its fee structure and liquidity constraints make it entirely unviable for a $26 account.

* **Trading Fees:** GMX V2 charges a 0.05% position fee to open and another 0.05% fee to close.38  
* **Gas Overhead:** Interacting with the Arbitrum network requires paying native gas fees for both the initial order transaction and the Keeper execution fee.  
* **Price Impact:** GMX V2 implements a price impact fee to balance open interest skew.38 For mid-cap synthetics like ARB, LINK, and SOL, a higher price impact fee is set, and liquidity is intentionally kept below other external markets to increase attack costs.40

If the agent opens a $260 position (10x leverage on the $26 account), the 0.05% fee is $0.13. Combined with network gas fees (historically $0.10 to $0.50 per transaction) and potential price impact penalties, opening and closing a single trade could easily consume 3% to 5% of the total $26 account balance regardless of the trade's outcome.

### **5.2 Hyperliquid Technical Architecture**

Hyperliquid operates as an application-specific Layer 1 blockchain (L1 appchain) built specifically to house a fully on-chain central limit order book (CLOB).41 It processes up to 200,000 orders per second natively on-chain without relying on off-chain matching engines or asynchronous keeper networks.41

#### **API and WebSocket Integration**

Hyperliquid's Data API is explicitly designed for high-frequency algorithmic trading.42 WebSockets stream real-time L2 order book updates, trades, and user events.21 Liquidations are broadcast directly through the userEvents and userFills schemas.21  
To place an order, the system must construct a JSON payload, sign it locally with the wallet's private key (using Ethereum-compatible signature formats), and send it via a POST request to https://api.hyperliquid.xyz/exchange.44  
The exact schema for placing a limit order with a Time-in-Force (TIF) of Immediate-Or-Cancel (IOC)—the optimal order type to aggressively catch a V-recovery without resting indefinitely on the book—is structured as follows:

JSON  
{  
  "action": {  
    "type": "order",  
    "orders": \[  
      {  
        "a": 0,   
        "b": true,   
        "p": "50000.0",   
        "s": "0.01",   
        "r": false,   
        "t": { "limit": { "tif": "Ioc" } }  
      }  
    \],  
    "grouping": "na"  
  },  
  "nonce": 1705234567890,  
  "signature": { "r": "0x...", "s": "0x...", "v": 27 }  
}

45  
Hyperliquid natively supports complex order types directly on-chain, including Take Profit (TP) and Stop Loss (SL) triggers.46 The agent can define absolute PnL thresholds or percentage deviations, submitting the TP/SL instructions within the initial order payload, ensuring automated risk management without requiring constant polling.48

#### **Advantages for Micro-Capital**

Hyperliquid fundamentally solves the micro-capital constraint.

* **Zero Gas Fees:** There are absolutely no gas fees for trading.41 The blockchain's consensus mechanism inherently processes the orders as state transitions.  
* **Leverage and Coverage:** Hyperliquid supports up to 50x leverage on a massive variety of altcoins, with excellent liquidity for ARB, LINK, and PEPE.50  
* **Maker Rebates:** Hyperliquid operates a maker-taker fee model where limit orders (maker) often incur zero fees or even receive rebates, protecting the account balance.  
* **Liquidation Logic:** Hyperliquid liquidates a perps position when its margin ratio falls below the maintenance threshold. The HLP (Hyperliquidity Provider) vault absorbs the position at a mark-based liquidation price, followed by an insurance fund backstop, and ultimately Auto-Deleveraging (ADL).51

| Feature | Hyperliquid (L1 Appchain) | GMX V2 (Arbitrum) |
| :---- | :---- | :---- |
| **Execution Latency** | Sub-millisecond to 500ms (HyperCore) 41 | Asynchronous (Keeper dependent) 31 |
| **Trading Fees** | 0% Maker / Low Taker (No Gas) 41 | 0.05% \+ Arbitrum Gas \+ Keeper Fee 38 |
| **Order Placement** | POST JSON with local ECDSA signature 45 | Smart Contract createOrder call 30 |
| **Max Leverage** | Up to 50x (Asset dependent) 50 | Up to 50x 37 |
| **Liquidation Engine** | HLP Vault \-\> Insurance Fund \-\> ADL 51 | Keeper execution based on Oracles 52 |
| **Programmatic TP/SL** | Native on-chain parameters 46 | Requires complex keeper callbacks 31 |

Given the capital constraints and the absolute necessity for sub-500 millisecond execution to catch the absolute bottom of a V-recovery wick, Hyperliquid is strictly superior. The trading agent will execute exclusively via the Hyperliquid REST API, while subscribing to Hyperliquid's WebSockets for real-time asset context.

## **6\. Historical Cascade Events and Backtesting Infrastructure**

To refine the cascade detection thresholds—for example, confirming whether an OI drop of 15% versus 20% yields the optimal Sharpe ratio across altcoins like LINK and PEPE—the trading agent's algorithms must be extensively backtested against historical data.

### **6.1 Sourcing Tick-Level Data**

Granular historical data is notoriously expensive in the cryptocurrency ecosystem, but specific free and low-cost repositories exist that can fulfill the requirements of a minimally funded project.  
**Binance Vision (Free):** Binance provides raw, historical dumps of tick-by-tick trades, order book snapshots, and liquidations completely free via their public S3 buckets at data.binance.vision.53 Data can be programmatically downloaded via HTTP requests to endpoints such as https://data.binance.vision/?prefix=data/futures/um/daily/liquidationSnapshot/BTCUSDT/.56 This provides the exact historical timestamps and values of the forceOrder events necessary to build a baseline historical model of cascades.57  
**Tardis.dev (Enterprise/Paid):** For institutional-grade backtesting, Tardis.dev provides historical tick-level order book updates, open interest, and liquidations via downloadable CSVs and client libraries.58 Tardis archives Bybit's allLiquidation streams and global long/short account ratios at 5-minute intervals.59 While highly comprehensive, its cost makes it prohibitive for the current project budget, meaning the system will rely on Binance Vision datasets to train the initial logic.  
**Cryptofeed Integration:** The cryptofeed Python library natively supports writing historical WebSocket data streams directly into time-series databases like QuestDB, PostgreSQL, or Redis.60 Cryptofeed standardizes data across Binance, Bybit, and other exchanges into unified formats (L2\_BOOK, TRADES, LIQUIDATIONS, OPEN\_INTEREST).60 While the live execution environment will be built in Rust, utilizing Python and cryptofeed to ingest and process the historical Binance S3 data allows for rapid prototyping of the cascade detection logic.

## **7\. Real-Time Monitoring Architecture in Rust**

The autonomous trading agent requires a highly robust, concurrent software architecture capable of 24/7 operation on a standard Windows machine. The system must process high-frequency WebSocket streams for up to 10 correlated pairs simultaneously without blocking the execution threads, maintaining strict memory efficiency.

### **7.1 Technology Stack and Concurrency Model**

The core backend will be implemented in Rust, capitalizing on its zero-cost abstractions, memory safety, and fearless concurrency.

* **Asynchronous Runtime:** The tokio asynchronous runtime will manage non-blocking I/O operations, ensuring that processing a massive influx of trade data during a cascade does not stall the execution loop.  
* **WebSocket Management:** The tokio-tungstenite crate will maintain persistent, parallel connections to Binance (for broad macro OI/Liquidation metrics) and Hyperliquid (for execution-venue pricing and user fills).  
* **Data Processing:** The Rust backend will deserialize the incoming JSON streams into native Rust structs. To avoid garbage collection pauses or heap allocation overhead during critical volatility windows, the system will utilize memory-mapped lock-free data structures (such as ring buffers) to maintain a rolling 5-minute window of volume, price, and OI.63  
* **Execution Library:** A custom implementation of the Hyperliquid API utilizing reqwest for POST requests and ethers-core (or the newer alloy framework) for generating the deterministic ECDSA signatures required for Hyperliquid's /exchange endpoint.35  
* **LLM Integration:** The Rust backend will format the real-time market state data (b-shape profile parameters, OI deltas) into highly structured JSON prompts. These prompts will be passed to the LLM agent to authorize the final execution sequence, ensuring the logical criteria align with the predefined trading rules.  
* **Frontend Dashboard:** A TypeScript/React dashboard will communicate with the Rust backend via local websockets, visualizing the active volume profile, order book heatmaps, and system state for monitoring purposes.

### **7.2 Hardware and Latency Budgets**

A single modern Windows machine can trivially handle monitoring 10 pairs simultaneously; the bottleneck is not CPU computation, but network latency. The latency budget is defined strictly by the speed at which the V-recovery occurs. Because forced liquidations execute via market orders, they consume the order book instantly. The agent must detect the absorption and broadcast the buy order within \~200 to 500 milliseconds of the final liquidation burst to capture the optimal entry before the price gaps upward.41 Rust's performance ensures the computational overhead remains in the single-digit microsecond range 63, reserving the entire latency budget for network transit to the Hyperliquid sequencer.

### **7.3 The Sequential Detection and Execution Algorithm**

The core logic loop operates as an asynchronous state machine evaluating the market microstructure continuously:

1. **State 0: Baseline Monitoring**  
   * Maintain parallel WebSockets to Bybit (wss://stream.bybit.com/v5/public/linear 20) for Open Interest and Hyperliquid (wss://api.hyperliquid.xyz/ws 43) for trades and activeAssetCtx.21  
   * Continuously calculate a rolling 5-minute buffer of total executed volume and OI delta.  
2. **State 1: Cascade Trigger Detection**  
   * Condition: Total open interest for the monitored pair drops by ![][image3] within a 15-minute window.4  
   * Condition: Price velocity exceeds ![][image4] within the same window.  
   * Action: The system enters maximum alert, increasing internal polling frequency and preparing the LLM context payload.  
3. **State 2: Exhaustion Confirmation (The V-Recovery Setup)**  
   * The algorithm constructs a real-time Volume Profile over the cascade window.  
   * Condition: It mathematically detects a "b-shape" formation, meaning the Point of Control (highest volume node) is situated in the lowest quartile of the price drop.23  
   * Condition: Price stops dropping despite ongoing high-volume execution, signaling market maker limit-buy absorption.7  
   * Condition: The funding rate flips negative, indicating short-seller saturation.12  
4. **State 3: Autonomous Execution**  
   * The agent crafts an order payload targeting the Hyperliquid /exchange endpoint.  
   * To act as a maker and avoid taker fees, it submits a Limit order with tif: "Ioc" (Immediate-Or-Cancel) exactly at or marginally below the current best ask, ensuring immediate execution without resting on the book if the market runs.44  
   * The payload simultaneously embeds a Stop Loss (e.g., \-2% absolute PnL) and a Take Profit (+5% to target the 50% Fibonacci retracement of the cascade wick).46  
   * Leverage is dynamically calculated based on the $26 account limit, strictly capping maximum exposure (e.g., 5x to 10x) to prevent the agent's own position from breaching the maintenance\_margin\_required threshold during momentary volatility.2

## **8\. Conclusion**

The exploitation of cryptocurrency liquidation cascades represents a statistically superior algorithmic trading strategy, rooted deeply in the deterministic, forced-selling mechanics of decentralized and centralized margin engines. When health factors fail, the resulting mechanical amplification creates temporary, extreme mispricings that are rapidly corrected via V-shape recoveries once the forced volume is fully absorbed by market makers.  
For an autonomous agent constrained to a $26 starting balance, architectural efficiency is paramount. Integrating with GMX V2 on Arbitrum is mathematically unviable due to inherent network gas overhead, 0.05% entry/exit fees, and price impact penalties on altcoins. Conversely, Hyperliquid provides the optimal execution venue: an L1 appchain with zero gas fees, natively supported programmatic Take-Profit/Stop-Loss capabilities, deep liquidity for altcoins like ARB and PEPE, and maker-rebates that protect micro-capital accounts.  
By synthesizing real-time Open Interest metrics from Bybit and Binance WebSockets with local Volume Profile analysis, the Rust-based architecture can bypass the obfuscation inherent in exchange liquidation feeds. When an Open Interest drop exceeding 20% aligns with a b-shaped volume profile and extreme downward price velocity, the agent achieves cryptographic confirmation of cascade exhaustion. This architecture allows the system to autonomously deploy leveraged capital into the highest-edge structural inefficiency in digital asset markets, managing risk via programmatic on-chain thresholds within a strictly defined sub-500 millisecond latency budget.

#### **Works cited**

1. Liquidation Cascade: Causes, Risks, and Prevention | Chainlink, accessed June 6, 2026, [https://chain.link/article/liquidation-cascade-crypto-lending](https://chain.link/article/liquidation-cascade-crypto-lending)  
2. Liquidations \- Hyperliquid Docs \- GitBook, accessed June 6, 2026, [https://hyperliquid.gitbook.io/hyperliquid-docs/trading/liquidations](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/liquidations)  
3. Liquidation Cascades The Anatomy Of DeFi's Most Violent Events On Kava \- Binance, accessed June 5, 2026, [https://www.binance.com/en/square/post/29593544914489](https://www.binance.com/en/square/post/29593544914489)  
4. Bitcoin and Ethereum Open Interest Tumbles to Multi-Month Lows After Brutal Selloff, accessed June 6, 2026, [https://www.binance.com/en/square/post/330833030818514](https://www.binance.com/en/square/post/330833030818514)  
5. Bulls Rout. Bitcoin Slumps Over 16% in a Week to Hit Lows, Crypto Market Faces Chain Liquidations, accessed June 6, 2026, [https://www.tradingkey.com/analysis/cryptocurrencies/btc/261948475-bitcoin-btc-support-breakdown-liquidation-coinbase-mstr-outflow-crypto-selloff-tradingkey](https://www.tradingkey.com/analysis/cryptocurrencies/btc/261948475-bitcoin-btc-support-breakdown-liquidation-coinbase-mstr-outflow-crypto-selloff-tradingkey)  
6. AI Professor(@Lukewood929)'s insights, accessed June 6, 2026, [https://www.binance.com/en/square/post/330797516493042](https://www.binance.com/en/square/post/330797516493042)  
7. \*$LABUSDT | Why The 38% Wick Dump Happened\* LAB wicked d | BABAR SK on Binance Square, accessed June 5, 2026, [https://www.binance.com/en-AE/square/post/329918752938801](https://www.binance.com/en-AE/square/post/329918752938801)  
8. Crypto Exchanges Are Hiding the Truth About Liquidations—Here's How \- Binance, accessed June 5, 2026, [https://www.binance.com/en/square/post/12868215144122](https://www.binance.com/en/square/post/12868215144122)  
9. Liquidation Order Streams | Binance Open Platform, accessed June 5, 2026, [https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Liquidation-Order-Streams](https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Liquidation-Order-Streams)  
10. All Market Liquidation Order Streams | Binance Open Platform, accessed June 5, 2026, [https://developers.binance.com/docs/derivatives/coin-margined-futures/websocket-market-streams/All-Market-Liquidation-Order-Streams](https://developers.binance.com/docs/derivatives/coin-margined-futures/websocket-market-streams/All-Market-Liquidation-Order-Streams)  
11. All Liquidation | Bybit API Documentation \- GitHub Pages, accessed June 5, 2026, [https://bybit-exchange.github.io/docs/v5/websocket/public/all-liquidation](https://bybit-exchange.github.io/docs/v5/websocket/public/all-liquidation)  
12. CoinGlass | Crypto Market Data: Derivatives, Options, Spot, Order Flow, Liquidity & Order Depth, Liquidation Heatmaps, accessed June 5, 2026, [https://www.coinglass.com/](https://www.coinglass.com/)  
13. CoinGlass Review: The Crypto Data Platform Serious Traders Actually Use (Free Tier \+ API Pricing Breakdown) \- GitHub Gist, accessed June 5, 2026, [https://gist.github.com/wbugg6/a9a8cbe01ba52a789bbd8337178beda6](https://gist.github.com/wbugg6/a9a8cbe01ba52a789bbd8337178beda6)  
14. Coinglass Liquidation Heatmap API \- Apify, accessed June 5, 2026, [https://apify.com/hamdo/coinglass-liquidation-heatmap/api](https://apify.com/hamdo/coinglass-liquidation-heatmap/api)  
15. Understanding Open Interest History in Crypto Trading with Crypto Pandas \- Medium, accessed June 6, 2026, [https://medium.com/@lucasjamar47/understanding-open-interest-history-in-crypto-trading-with-crypto-pandas-688bb13a1595](https://medium.com/@lucasjamar47/understanding-open-interest-history-in-crypto-trading-with-crypto-pandas-688bb13a1595)  
16. Open Interest \- Binance Developer center, accessed June 5, 2026, [https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Open-Interest](https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Open-Interest)  
17. Open Interest | Binance Open Platform, accessed June 5, 2026, [https://developers.binance.com/docs/derivatives/options-trading/websocket-market-streams/Open-Interest](https://developers.binance.com/docs/derivatives/options-trading/websocket-market-streams/Open-Interest)  
18. Get Open Interest | Bybit API Documentation \- GitHub Pages, accessed June 5, 2026, [https://bybit-exchange.github.io/docs/v5/market/open-interest](https://bybit-exchange.github.io/docs/v5/market/open-interest)  
19. Ticker | Bybit API Documentation \- GitHub Pages, accessed June 5, 2026, [https://bybit-exchange.github.io/docs/v5/websocket/public/ticker](https://bybit-exchange.github.io/docs/v5/websocket/public/ticker)  
20. sferez/BybitMarketData: This repository serves as a collection point for market data from Bybit. Aimed at facilitating machine learning model creation and finetuning. \- GitHub, accessed June 5, 2026, [https://github.com/sferez/BybitMarketData](https://github.com/sferez/BybitMarketData)  
21. Subscriptions \- Hyperliquid Docs \- GitBook, accessed June 6, 2026, [https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions)  
22. Risk Management in Cryptocurrency Exchanges: Protecting Users from Liquidation, accessed June 5, 2026, [https://medium.com/@gwrx2005/risk-management-in-cryptocurrency-exchanges-protecting-users-from-liquidation-430acfd1b304](https://medium.com/@gwrx2005/risk-management-in-cryptocurrency-exchanges-protecting-users-from-liquidation-430acfd1b304)  
23. Volume Profile Effective Trading Guide \- Alchemy Markets, accessed June 6, 2026, [https://alchemymarkets.com/education/indicators/volume-profile/](https://alchemymarkets.com/education/indicators/volume-profile/)  
24. 4 Signs Of A "Liquidation Day" Volume Profile \- YouTube, accessed June 6, 2026, [https://www.youtube.com/watch?v=ofnczuv6zbI](https://www.youtube.com/watch?v=ofnczuv6zbI)  
25. The Exhaustion Reversal \[TRADING STRATEGY\] \- YouTube, accessed June 6, 2026, [https://www.youtube.com/watch?v=0zc2jpsjXNM](https://www.youtube.com/watch?v=0zc2jpsjXNM)  
26. How to Read Volume Profile Structures | by Global Prime \- Medium, accessed June 6, 2026, [https://globalprime.medium.com/how-to-interpret-volume-profile-structures-in-the-forex-market-f28f0b5efd62](https://globalprime.medium.com/how-to-interpret-volume-profile-structures-in-the-forex-market-f28f0b5efd62)  
27. Deep Dive into GMX: Exploring Arbitrum's Leading DeFi Protocol \- Bitquery, accessed June 5, 2026, [https://bitquery.io/blog/gmx](https://bitquery.io/blog/gmx)  
28. GMX: An In-Depth Look at Arbitrum's Leading Permissionless Exchange for On-Chain Leverage Trading, accessed June 6, 2026, [https://blog.arbitrum.io/gmx-an-in-depth-look-at-arbitrums-leading-permissionless-exchange-for-on-chain-leverage-trading/](https://blog.arbitrum.io/gmx-an-in-depth-look-at-arbitrums-leading-permissionless-exchange-for-on-chain-leverage-trading/)  
29. GMX API \- Web3 Ethereum Defi documentation, accessed June 5, 2026, [https://web3-ethereum-defi.readthedocs.io/api/gmx/index.html](https://web3-ethereum-defi.readthedocs.io/api/gmx/index.html)  
30. 2023-02-gmx/gmx-synthetics/contracts/router/ExchangeRouter.sol at main \- GitHub, accessed June 5, 2026, [https://github.com/sherlock-audit/2023-02-gmx/blob/main/gmx-synthetics/contracts/router/ExchangeRouter.sol](https://github.com/sherlock-audit/2023-02-gmx/blob/main/gmx-synthetics/contracts/router/ExchangeRouter.sol)  
31. gmx-io/gmx-synthetics \- GitHub, accessed June 6, 2026, [https://github.com/gmx-io/gmx-synthetics](https://github.com/gmx-io/gmx-synthetics)  
32. GMX Exchange Hack Explained \- Sherlock, accessed June 6, 2026, [https://sherlock.xyz/post/gmx-exchange-hack-explained](https://sherlock.xyz/post/gmx-exchange-hack-explained)  
33. Foundation \- GMX Contract Architecture \- GMX Perpetuals Trading \- Video, accessed June 6, 2026, [https://updraft.cyfrin.io/courses/gmx-perpetuals-trading/foundation/gmx-contract-architecture](https://updraft.cyfrin.io/courses/gmx-perpetuals-trading/foundation/gmx-contract-architecture)  
34. ethers::contract \- Rust \- Docs.rs, accessed June 6, 2026, [https://docs.rs/ethers/latest/ethers/contract/index.html](https://docs.rs/ethers/latest/ethers/contract/index.html)  
35. Using Alloy to interact with Ethereum smart-contracts in Rust \- Medium, accessed June 6, 2026, [https://medium.com/0xintuition/using-alloy-to-interact-with-ethereum-smart-contracts-in-rust-3b2c70bbfa6a](https://medium.com/0xintuition/using-alloy-to-interact-with-ethereum-smart-contracts-in-rust-3b2c70bbfa6a)  
36. Cyfrin/defi-gmx-v2 \- GitHub, accessed June 6, 2026, [https://github.com/Cyfrin/defi-gmx-v2](https://github.com/Cyfrin/defi-gmx-v2)  
37. GMX V2: Trade on Arbitrum and Win \- CoinGecko, accessed June 6, 2026, [https://www.coingecko.com/learn/gmx-trading-competition](https://www.coingecko.com/learn/gmx-trading-competition)  
38. GMX v2: An Overview | TKX Weekly | Medium, accessed June 5, 2026, [https://tkxcapital.medium.com/gmx-v2-an-overview-tkx-weekly-af22d1cf4ecd](https://tkxcapital.medium.com/gmx-v2-an-overview-tkx-weekly-af22d1cf4ecd)  
39. GMX v2: A Quick Guide to the Upgrade \- Blocmates, accessed June 6, 2026, [https://www.blocmates.com/articles/gmx-v2-a-quick-guide-to-the-upgrade](https://www.blocmates.com/articles/gmx-v2-a-quick-guide-to-the-upgrade)  
40. Changes and Impacts of GMX V2 \- LD Capital \- Medium, accessed June 6, 2026, [https://ld-capital.medium.com/changes-and-impacts-of-gmx-v2-6ed0e4c10f93](https://ld-capital.medium.com/changes-and-impacts-of-gmx-v2-6ed0e4c10f93)  
41. Top 7 Trading Bots on Hyperliquid in 2026 | Chainstack Blog, accessed June 6, 2026, [https://chainstack.com/hyperliquid-trading-bots-2026/](https://chainstack.com/hyperliquid-trading-bots-2026/)  
42. Hyperliquid Data API Explained \- HypeRPC, accessed June 5, 2026, [https://hyperpc.app/blog/hyperliquid-data-api-explained](https://hyperpc.app/blog/hyperliquid-data-api-explained)  
43. Websocket \- Hyperliquid Docs \- GitBook, accessed June 5, 2026, [https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket)  
44. Exchange endpoint \- Hyperliquid Docs \- GitBook, accessed June 6, 2026, [https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint)  
45. Place order | Hyperliquid exchange \- Chainstack Docs, accessed June 6, 2026, [https://docs.chainstack.com/reference/hyperliquid-exchange-place-order](https://docs.chainstack.com/reference/hyperliquid-exchange-place-order)  
46. Order types | Hyperliquid Docs \- GitBook, accessed June 6, 2026, [https://hyperliquid.gitbook.io/hyperliquid-docs/trading/order-types](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/order-types)  
47. Take profit and stop loss orders (TP/SL) \- Hyperliquid Docs \- GitBook, accessed June 6, 2026, [https://hyperliquid.gitbook.io/hyperliquid-docs/trading/take-profit-and-stop-loss-orders-tp-sl](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/take-profit-and-stop-loss-orders-tp-sl)  
48. hyperliquid-supurr | Skills Marketplace \- LobeHub, accessed June 5, 2026, [https://lobehub.com/skills/supurr-app-hyperliquid-supurr-skill](https://lobehub.com/skills/supurr-app-hyperliquid-supurr-skill)  
49. dYdX \- NautilusTrader Documentation, accessed June 5, 2026, [https://nautilustrader.io/docs/nightly/integrations/dydx/](https://nautilustrader.io/docs/nightly/integrations/dydx/)  
50. Understanding leverage and liquidation | MetaMask Help Center, accessed June 6, 2026, [https://support.metamask.io/trade/perps/leverage-and-liquidation/](https://support.metamask.io/trade/perps/leverage-and-liquidation/)  
51. Hyperliquid Liquidations Explained: Margin Calls and Insurance Fund | Support \- Eco, accessed June 6, 2026, [https://eco.com/support/en/articles/15247705-hyperliquid-liquidations-explained-margin-calls-and-insurance-fund](https://eco.com/support/en/articles/15247705-hyperliquid-liquidations-explained-margin-calls-and-insurance-fund)  
52. Condition For Liquidation \- GMX Perpetuals Trading \- Blockchain and Smart Contract Development Courses \- Cyfrin Updraft, accessed June 6, 2026, [https://updraft.cyfrin.io/courses/gmx-perpetuals-trading/liquidation/condition-for-liquidation](https://updraft.cyfrin.io/courses/gmx-perpetuals-trading/liquidation/condition-for-liquidation)  
53. Binance Market Data \- Amberdata, accessed June 6, 2026, [https://www.amberdata.io/binance-market-data](https://www.amberdata.io/binance-market-data)  
54. liquidationSnapshot \- Binance Data, accessed June 6, 2026, [https://data.binance.vision/?prefix=data/futures/cm/daily/liquidationSnapshot/](https://data.binance.vision/?prefix=data/futures/cm/daily/liquidationSnapshot/)  
55. Binance Data Collection, accessed June 6, 2026, [https://data.binance.vision/](https://data.binance.vision/)  
56. Binance Data Collection, accessed June 6, 2026, [https://data.binance.vision/?prefix=data/futures/um/daily/liquidationSnapshot/BTCUSDT/](https://data.binance.vision/?prefix=data/futures/um/daily/liquidationSnapshot/BTCUSDT/)  
57. \[help\] How can I fetch liquidation data of all the users in futures? · Issue \#1060 · sammchardy/python-binance \- GitHub, accessed June 6, 2026, [https://github.com/sammchardy/python-binance/issues/1060](https://github.com/sammchardy/python-binance/issues/1060)  
58. The most granular data for cryptocurrency markets — Tardis.dev, accessed June 5, 2026, [https://tardis.dev/](https://tardis.dev/)  
59. Bybit Derivatives \- Tardis.dev Documentation, accessed June 5, 2026, [https://docs.tardis.dev/historical-data-details/bybit](https://docs.tardis.dev/historical-data-details/bybit)  
60. cryptofeed | Skills Marketplace \- LobeHub, accessed June 6, 2026, [https://lobehub.com/it/skills/2025emma-vibe-coding-cn-cryptofeed](https://lobehub.com/it/skills/2025emma-vibe-coding-cn-cryptofeed)  
61. Ingesting Financial Tick Data Using a Time-Series Database \- QuestDB, accessed June 6, 2026, [https://questdb.com/blog/ingesting-financial-tick-data-using-time-series-database/](https://questdb.com/blog/ingesting-financial-tick-data-using-time-series-database/)  
62. bmoscon/cryptofeed: Cryptocurrency Exchange Websocket Data Feed Handler \- GitHub, accessed June 6, 2026, [https://github.com/bmoscon/cryptofeed](https://github.com/bmoscon/cryptofeed)  
63. GitHub \- vibheksoni/axiomtrade-rs: Rust SDK for Axiom Trade account API and DEX aggregator. Lightning-fast Solana/Hyperliquid trading with auto-OTP, WebSocket streaming, hardware wallet support, and MEV protection. Built for algorithmic trading bots., accessed June 5, 2026, [https://github.com/vibheksoni/axiomtrade-rs](https://github.com/vibheksoni/axiomtrade-rs)  
64. Executing trades \- Hyperliquid \- Privy Docs, accessed June 6, 2026, [https://docs.privy.io/recipes/hyperliquid/trading-patterns](https://docs.privy.io/recipes/hyperliquid/trading-patterns)

[image1]: <data:image/png;base64,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>

[image2]: <data:image/png;base64,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>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADsAAAAZCAYAAACPQVaOAAACwklEQVR4Xu2XT4hNURzHvxrKhExMpJQhkg0LSYRmo2hi4V9iqSzYkaakNJkaf0qE/F+YDZEQJVGE5M+ChaTYELO0UJTV+H773fPeeWfOvW/unedp6n7q03v3nnPfnN85v/s7Z4CSkv9NG51Kx4QNHuPohPDmaELB7aV36FF6HvGAWukZuj5saCYtdAvtCO6LDrqNzoT1G08X0Z3Jd7GYfqBzYIEfot/pbjoL9uxm+paepWPtseahWV5HT8IG9gs26JDl9A8d9FTfLq/PHvqMTkquV9B9dAHdSrfDJvM2LPC6aBbdTDYCBbuWdtKDSA/WrdpH+o720Bk1PYAr9AmdmFzrmVOVVlvtI8iRvkqRh7CHVAQaSTeyg/UHHmM/aoNdQnsrrcBqegI501cztJDep5fp7Nrmwow02E76CdUU3YXqKk6j1722QsynNxL1fSTUC/YW7aef6VdY2us1cGg7UeHRWFSUrsFWWSupFR12+tZDq6tVfkqXInuPS6NesK/o3ORar9BregEWpEN/V1VaKesmQkG69FW7xnecrkmuC6OioT2sSNBZwcYOAXpHVaGXBfd9lLY3k0+xkT6i8+gBuiG5XxjNutLpOeJ7ZhpZwcZQf21B2nJihOk7mb6EbUFiOj2HakHLhVb1NH2M/Ksq0oLV5L2h72GFxuGC1WcMP32F0vsnbF8XGt9hDN3CMtH7epHeg1XpvEE60oLVYL5gaLBKYwUbKzxh+gr9rn7fBSsUrE5XmfyL7UfB/obtjz5aGaWbssWhlFRd0Pun7z4ufVWkfBT4N1SDVfrqdWuv9Ahw1UypqpTNlQIRVHS0RfxA7VFwAFYxHZp9HQWP0R2w8+0LxPdNrbQOPWGGaRIueW0rYYeOsF8FHe360PjT03BQRV5FN8HOu/qHIETjUkBp45tCr9IH9C7ik1VSUlJSUtJs/gJRUn4xu4MHxAAAAABJRU5ErkJggg==>

[image4]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADYAAAAZCAYAAAB6v90+AAACaElEQVR4Xu2WTUhVQRiG37AgoYIo+qGCkiKiTRElQW6lHwSRFkaBEGgRBaVg0cqNCyOChCTapEjUQmqji2rTzyratEncSAghFOiudb3v/Wa4c+aewaDOIei88HDv/Nxzv3e+b+YMUKnSn2od2UKa4oFAGlsfd/7L6iLvyW3ynGzLDtckU7dIfzxQpA6T9rjTaRVpJaNkjJxBNiu7yCdywrUvkGUyRPaR7eQkeUVewjJbqBTsIPlIfpKb2eGaZEpz3pI9ZBN5Qh6RNW7OKbJAdrp2C7kHm3+WnCedsExqAQuXjHWQ0+QH8o0dId9Qz4akwGVEWZD0O7WVGUmfE8hm5hJKLkFJwaeMDSMbtKTNr/30GJZR7a9wjjL3kKx17YNkEiWUYKyUMQU2g0ZjCvANrIQ3kv1kjhx148rkDfe9mYyjpBKMlTLmDaSM+X5lbYC8IxfJNOr7TeVXegl6pYwpaAW/kjGvvbA9u8G1laVnqJegMnuHnEP94ElKE7bC/mAlUi/PlDE9dx6NBlLGQmmOTPkSPEY+uHYvuQ7LdFKHYEfv73Cf7K79KquUsZSBVH+osARXk6ewg0jSvtPhssO1C1PKmAKaQqMBb0wnY971KC5Bn/nw+ddg/1uoUsYk9S2RA0HfZvIZdhOJFZegpEXR4oTPv0qOB+1C5I3pnhdLB8JX0h30tZHvyA9M5aeXcSj/3vPGtLfuIrtYf1WXySLsOuXRHU9B6KDx0gv4C+kjPWSWXEHj5leWxmF7KJZMvYAddlos7bG8eaVLd0Qd5ULfY+ll/gAWdJ5kYgS2aK9hh16lSpUqVfp/9QuLNHpEIk8mbQAAAABJRU5ErkJggg==>