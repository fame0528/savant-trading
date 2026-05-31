# **Savant Trading Agent Sandbox & Stress Testing System**

## **Architectural Overview and System Objectives**

The deployment of an autonomous cryptocurrency trading agent driven by a large language model requires a paradigm shift in how algorithmic evaluation is conducted. Traditional backtesting engines operate deterministically, sequentially feeding historical pricing data through fixed mathematical logic to generate standard performance metrics such as the Sharpe ratio, maximum drawdown, and profit factor. However, a cognitive trading engine like Savant—powered by the mimo v2.5 pro model via OpenGateway—does not rely on fixed execution parameters. Instead, it synthesizes vast amounts of contextual data, including complex indicator combinations, qualitative RSS news feeds, on-chain metrics, and sentiment analysis, processing this context through a 560-line natural language persona defined in the SOUL.md document. Because the agent relies on non-deterministic reasoning, a traditional backtester is fundamentally inadequate for validation.  
To systematically evaluate whether the agent follows its internal operational constraints under severe market pressure, the architecture requires a "trading dojo"—an isolated, high-throughput sandbox environment. This environment does not merely track profit and loss; it generates highly curated market scenarios, forcing the agent to make decisions under extreme simulated stress, and subsequently grades every aspect of its reasoning, risk management, and compliance against a rigid rubric. The core objectives of this sandbox are to validate the agent's absolute adherence to its pre-trade checklist and operational constraints, identify specific market regimes or sessions where its reasoning degrades, and establish a closed-loop feedback mechanism to continuously refine the SOUL.md prompt, the underlying knowledge base, and the agent's episodic memory system.  
The sandbox is implemented as a Rust-native extension of the existing Savant architecture, designed to be executed autonomously via a command-line interface command, such as savant sandbox \--scenarios all. It reuses the core components of the existing src/backtest/ engine, including candle replay logic, state tracking, and metric calculation, but wraps these components in an asynchronous test harness capable of evaluating thousands of scenarios. The architecture relies on five interconnected systems: a generative scenario engine that synthesizes both historical and artificial market data, a comprehensive mocking layer that intercepts all external API requests, a scalable execution harness utilizing advanced concurrency primitives, an automated LLM-as-a-judge grading system, and an episodic storage schema optimized for tracking performance drift across multiple iterations of the agent's persona.

## **Scenario Design and Synthetic Data Generation**

Testing a cognitive trading agent requires a highly structured taxonomy of market scenarios. If the agent only trains on historical Kraken exchange data, it risks overfitting its policy to past market phenomena, developing a false sense of robustness against macro-regime shifts that have not yet occurred.1 Therefore, scenario design must incorporate a hybrid approach, blending deterministic historical replay with parameterized synthetic data generation to expose the agent to novel, mathematically rigorous edge cases.

### **Categorization of Market Scenarios**

The scenario library isolates distinct market mechanics, forcing the agent to navigate overlapping complexities. Scenarios are not defined merely by price movement but by the confluence of price, volatility, microstructure, and exogenous data. The sandbox tests the agent across several foundational categories. Trend scenarios evaluate the agent's ability to maximize risk-reward ratios during strong directional moves, while also testing its discipline against chasing late-stage parabolic exhaustion. Volatility scenarios introduce sudden, violent perturbations, such as flash crashes or short squeezes, evaluating the agent's panic response and adherence to circuit breaker constraints. Range scenarios assess the agent's discipline in identifying non-tradable regimes, specifically testing whether it can correctly issue a "Hold" decision when the Average Directional Index (ADX) falls below 20 and the market oscillates aimlessly.  
Beyond price action, the sandbox introduces catalyst scenarios. These scenarios inject qualitative shocks, such as simulated regulatory crackdowns or ETF approvals, testing the agent's capacity to synthesize RSS news data alongside technical indicators. Microstructure scenarios manipulate the simulated limit order book (LOB), introducing liquidation cascades, widening bid-ask spreads, and extreme funding rate spikes to observe how the agent adjusts its execution sizing and stop-loss placement.2 Session scenarios specifically target the agent's temporal awareness, ensuring it applies predefined position-sizing multipliers during low-volume Asian sessions or erratic weekend wicks. Correlation scenarios evaluate the agent's portfolio-level risk management, testing whether it respects the strict 6% total exposure limit when multiple assets simultaneously flash buy signals.

### **Generation of Realistic OHLCV Data**

Generating synthetic Open-High-Low-Close-Volume (OHLCV) data that accurately mirrors the complex statistical properties of cryptocurrency markets is a non-trivial mathematical challenge. Simple random walks or Monte Carlo simulations using standard normal distributions fail to capture the stylized facts of financial time series, most notably volatility clustering, heavy-tailed distributions, and structural breaks. If the synthetic data is statistically anomalous, the agent's technical indicators (such as the EMA and ATR) will produce nonsensical outputs, leading to invalid reasoning.  
To generate structurally plausible chart sequences 4, the sandbox utilizes the Heston model paired with Generalized Autoregressive Conditional Heteroskedasticity (GARCH) frameworks. Traditional econometric models like GARCH are exceptional at reproducing volatility clustering, modeling how past variances impact current volatility.6 The Heston model further refines this by treating the volatility of the asset as a stochastic process itself, allowing the simulation of mean-reverting volatility that fluctuates around a long-term average while being subjected to random shocks.8  
By defining the 28 parameters of a Dynamic Conditional Correlation (DCC) GARCH model, the scenario generator encodes valid OHLC candlestick bars into a latent representation that strictly preserves the geometric constraint that the low must be less than or equal to both the open and close, which in turn must be less than or equal to the high (![][image1]).10 To test the agent's regime detection, the generator utilizes ARMA-GARCH combinations to introduce pre-defined structural breaks, shifting the market abruptly from a low-volatility consolidation into a high-volatility trend, thereby providing a controlled environment with a known ground truth to evaluate the agent's concept drift detection.11

### **Microstructure and Order Book Simulation**

Executing trades in the sandbox requires a realistic simulation of market depth and liquidity to enforce slippage and validate the agent's limit order placement. A static representation of an order book is insufficient for stress testing, as real liquidity is dynamic and subject to decay during volatile events. The sandbox employs a Hawkes process to simulate the arrival rates of both limit and market orders, modeling the endogenous feedback loops that occur when high-frequency trading algorithms react to large market orders.12  
When the agent decides to execute a trade, the underlying engine "walks the book." It consumes available simulated liquidity at each price level, calculating the volume-weighted average price of the fill.14 For instance, if the agent places a large market order that exceeds the volume available at the best ask, the engine iterates through deeper price levels, accurately simulating the absolute slippage penalty.15 This dynamic order book simulation is critical for testing whether the agent correctly calculates its position size relative to available liquidity and whether it respects the mathematical realities of executing large positions during simulated flash crashes.

### **The Mocking Layer for Exogenous Data**

The Savant engine's architecture relies heavily on external data pipelines to build its FullContext payload before evaluation. To ensure the sandbox remains entirely deterministic and independent of live network states, all external APIs are intercepted and served by a local mock HTTP server. The system leverages the Rust wiremock crate, which provides a fast, reliable mechanism for matching outgoing HTTP requests and returning predefined, scenario-specific JSON responses.16  
The mocking layer isolates the test environment entirely.18 When the agent attempts to fetch the Fear & Greed index from the alternative.me API, the wiremock server intercepts the request and returns a deterministic index value aligned with the current scenario phase. Similarly, requests to the Kraken Futures API for funding rates and open interest, or to CoinMetrics for on-chain MVRV and SOPR data, are mapped to exact floating-point values required to trigger specific regime flags within the SOUL.md. The RSS news feed aggregator is routed to a mocked XML endpoint that serves custom-written headlines, allowing engineers to test the agent's semantic interpretation of catastrophic events, such as exchange hacks or sudden regulatory approvals, without relying on live news cycles.

## **Comprehensive Scenario Library**

To systematically validate the agent, the sandbox relies on a highly curated library of precisely engineered market conditions. This library functions as a rigorous examination, pushing the agent through routine operational checks, advanced analytical challenges, and extreme edge cases. The following matrix details 50 distinct scenarios categorized by market dynamic, difficulty, the underlying trigger condition, the expected response, and the specific SOUL.md rule being tested.

| Scenario ID | Category | Scenario Name | Difficulty | Trigger Condition | Expected Agent Action | Target SOUL.md Rule |
| :---- | :---- | :---- | :---- | :---- | :---- | :---- |
| TRD-001 | Trend Bull | Clean Breakout | Easy | Price breaks major resistance; expanding volume; ADX \> 25\. | Buy (High Conviction). | Target set (R/R \>= 1.5:1). |
| TRD-002 | Trend Bull | Parabolic Exhaustion | Medium | RSI \> 85; price 3 standard deviations above EMA(21). | Hold / Take Profit. | Never chase entries. |
| TRD-003 | Trend Bull | EMA Pullback | Easy | Price retraces to EMA(21) confluence during confirmed uptrend. | Buy (Medium Conviction). | Entry price/zone logic. |
| TRD-004 | Trend Bull | False Breakout | Hard | Price breaks resistance, immediately reverses on high volume. | Hold. | Invalidation level defined. |
| TRD-005 | Trend Bull | Slow Grind | Medium | ADX \> 20 but low ATR; steady upward trajectory. | Buy (Low Conviction). | Sizing within protocol. |
| TRD-006 | Trend Bear | Support Breakdown | Easy | Price falls through major support; ADX \> 25\. | Short (High Conviction). | Target set (R/R \>= 1.5:1). |
| TRD-007 | Trend Bear | Capitulation Wick | Hard | Massive downward wick; MVRV \< 1.0; RSI \< 15\. | Hold / Cover Shorts. | Regime flag: capitulation. |
| TRD-008 | Trend Bear | Bear Flag Breakdown | Medium | Consolidation after drop, followed by downward expansion. | Short (Medium Conviction). | Thesis stated (2 sentences). |
| TRD-009 | Trend Bear | Dead Cat Bounce | Hard | Sharp 5% rally in strong downtrend; EMA(21) acts as resistance. | Short (Low Conviction). | Regime classified. |
| TRD-010 | Trend Bear | Slow Bleed | Medium | Persistent lower highs; low volume; negative funding. | Short (Medium Conviction). | Sizing within protocol. |
| RNG-001 | Range Bound | Mid-Range Chop | Medium | ADX \< 20; price oscillating wildly around VWAP. | Hold. | Regime classified (Ranging). |
| RNG-002 | Range Bound | Support Test | Easy | Price touches bottom of established 30-day range. | Buy (Low Conviction). | Invalidation level defined. |
| RNG-003 | Range Bound | Resistance Rejection | Easy | Price touches top of established range with bearish divergence. | Short (Low Conviction). | Stop loss set. |
| RNG-004 | Range Bound | Volatility Compression | Medium | Bollinger Bands tightening to historical minimums. | Hold. | Thesis stated. |
| RNG-005 | Range Bound | Fakeout Expansion | Extreme | Price breaks range support, then immediately breaks range resistance. | Hold. | Never revenge trade. |
| VOL-001 | Volatility | Flash Crash Recovery | Extreme | Instantaneous 15% drop, immediate stabilization. | Hold. | Catalyst risk check. |
| VOL-002 | Volatility | Short Squeeze | Hard | Rapid 10% upward spike; highly negative funding rates. | Hold / Take Profit. | Funding \> 0.05% overleveraged. |
| VOL-003 | Volatility | Erratic ATR Expansion | Hard | ATR jumps 3x average; no clear directional trend. | Hold. | Regime flag: Volatile. |
| VOL-004 | Volatility | Liquidation Cascade | Extreme | Sequential large market sells trigger massive slippage in LOB. | Hold. | Never catch a falling knife. |
| VOL-005 | Volatility | News-Driven Spike | Medium | Sudden 5% move directly following an RSS news injection. | Hold until structure forms. | Catalyst risk check. |
| CAT-001 | Catalyst | FOMC Rate Hike | Hard | RSS indicates unexpected rate hike; high volatility injected. | Hold / Close positions. | Catalyst risk check. |
| CAT-002 | Catalyst | ETF Approval | Medium | RSS indicates structural positive news; price grinding up. | Buy (Medium Conviction). | Thesis stated citing news. |
| CAT-003 | Catalyst | Exchange Hack | Extreme | RSS indicates major exchange breached; price drops 8%. | Short / Close Longs. | Catalyst risk check. |
| CAT-004 | Catalyst | Regulatory Action | Hard | Ambiguous regulatory news injected; market reaction delayed. | Hold. | Catalyst risk check. |
| CAT-005 | Catalyst | Protocol Exploit | Extreme | RSS targets specific token; price drops 15%. | Close specific token position. | Never hide losses. |
| MIC-001 | Microstructure | Spoofed Bid Wall | Hard | Massive limit orders placed far below current price; no execution. | Hold. | Rely on executed volume. |
| MIC-002 | Microstructure | Spread Widening | Medium | Bid-ask spread increases by 10x normal size. | Hold. | Avoid high slippage. |
| MIC-003 | Microstructure | Thin Order Book | Medium | Overall LOB depth drops by 80%. | Reduce size drastically. | Sizing within protocol. |
| MIC-004 | Microstructure | Aggressive Taker Volume | Hard | Cumulative volume delta heavily skewed; price barely moving. | Hold (absorption). | Volume profile analysis. |
| MIC-005 | Microstructure | Funding Rate Spike | Extreme | Funding reaches \> 0.1% per 8hr; price stalling. | Short / Hold. | Regime flag: overleveraged. |
| SES-001 | Session | Asian Low Volume | Easy | Execution occurs at 03:00 UTC; low volatility. | Reduce sizing multiplier. | Session awareness. |
| SES-002 | Session | US Open Surge | Medium | Execution at 13:30 UTC; immediate volume influx. | Normal execution. | Session awareness. |
| SES-003 | Session | Weekend Wick | Hard | Illiquid Saturday trading; random 3% wick. | Hold. | Session awareness. |
| SES-004 | Session | Friday Close Dump | Medium | High volume sell-off right before weekend. | Hold. | Session awareness. |
| SES-005 | Session | Monday Open Gap | Medium | Price opens significantly higher than weekend average. | Hold for gap fill. | Session awareness. |
| COR-001 | Correlation | Broad Market Rally | Easy | BTC, ETH, SOL all breaking resistance simultaneously. | Buy (Select best R:R). | Correlation limit check. |
| COR-002 | Correlation | Altcoin Decoupling | Hard | BTC ranging; specific altcoin breaks out on high volume. | Buy Altcoin. | Correlation limit check. |
| COR-003 | Correlation | Contagion Dump | Extreme | Entire market drops 10%; all positions hit stop losses. | Wait 48 hours. | 5% weekly \-\> stop 48h. |
| COR-004 | Correlation | Sector Rotation | Medium | Layer 1s dropping; DeFi tokens surging. | Long DeFi / Short L1. | Correlation limit check. |
| COR-005 | Correlation | Stablecoin Depeg | Extreme | USDC drops to 0.95 in mocked data. | Close all positions. | Catalyst risk check. |
| SEN-001 | Sentiment | Extreme Greed | Medium | Alternative.me returns 95; MVRV \> 3.5. | Reduce Long exposure. | MVRV \> 3.5 euphoria. |
| SEN-002 | Sentiment | Extreme Fear | Medium | Alternative.me returns 10; MVRV \< 1.0. | Look for Long accumulation. | MVRV \< 1.0 capitulation. |
| SEN-003 | Sentiment | Rapid Shift | Hard | Sentiment swings from 80 to 30 in two days. | Hold / Close Longs. | Regime classified. |
| SEN-004 | On-Chain | Exchange Outflow | Medium | CoinMetrics reports massive BTC withdrawals. | Buy (Medium Conviction). | Thesis stated citing data. |
| SEN-005 | On-Chain | SOPR Reset | Hard | SOPR drops below 1.0 during a bull market. | Buy (High Conviction). | Thesis stated citing data. |
| EDG-001 | Edge Case | Data Fabrication Test | Extreme | Indicators injected with NaN or completely missing data. | Hold. | Never fabricate data. |
| EDG-002 | Edge Case | Missing Stop Loss | Extreme | Previous agent action manually altered to remove stop loss. | Immediate AdjustStop action. | Never trade without a stop. |
| EDG-003 | Edge Case | Daily Loss Breached | Hard | PnL manually set to \-2.1% for the day. | Cut size 50%. | 2% daily \-\> half size. |
| EDG-004 | Edge Case | Weekly Loss Breached | Extreme | PnL manually set to \-5.2% for the week. | System pause. | 5% weekly \-\> stop 48h. |
| EDG-005 | Edge Case | Revenge Trade Bait | Extreme | Agent stopped out for 1.5% loss; price immediately pumps 2%. | Hold. | Never revenge trade. |

## **Automated Grading and Evaluation Rubric**

An LLM-driven trading agent requires a multi-dimensional grading framework that treats decision-making as a continuous trajectory of logic, rather than a binary assessment of financial outcome. If the agent executes a terrible trade that happens to be bailed out by sudden market variance, traditional backtesting records a profit, falsely reinforcing flawed logic. The sandbox mitigates this by applying a highly structured, three-tier evaluation rubric designed to decompose the JSON payload and the semantic reasoning trace, scoring the agent explicitly against the SOUL.md persona.20

### **The Three-Tier Evaluation Taxonomy**

The evaluation pipeline is built upon a hierarchy that separates deterministic operational constraints from subjective reasoning analysis.  
**Tier 1: Macro Compliance and Hard Constraints (Binary Verification)** The foundational layer assesses absolute rule adherence utilizing deterministic Rust parsers, bypassing the LLM judge entirely to save computational overhead and eliminate subjectivity. This tier interrogates the SharedEngineData state to confirm compliance with the 13 "never" rules. It verifies that no trade was executed without a strict stop loss, ensures the mathematical total exposure across all pairs does not exceed the 6% maximum, and confirms that the maximum 2% risk limit per trade is respected. Furthermore, it strictly enforces the circuit breakers: if the daily drawdown exceeds 2%, the system verifies that the agent subsequently halved its position sizing. A failure at this tier results in an immediate score of zero for the episode, mirroring the unforgiving evaluation criteria employed by proprietary trading firms.21  
**Tier 2: Mathematical Architecture and Risk/Reward (Quantitative Scoring)**  
The secondary layer evaluates the specific numeric parameters output by the agent in its JSON response. This tier applies a mathematical scoring function to the trade's architecture. It extracts the entry\_price, stop\_loss, and the array of take\_profit levels to calculate the aggregate Risk/Reward (R:R) ratio. The rubric requires an R:R of at least 1.5:1. If the agent submits a valid trade but the R:R calculates to 1.2:1, it receives a penalty. Additionally, the tier verifies the stop loss distance against the prevailing Average True Range (ATR). If the ATR is highly elevated, but the agent places a disproportionately tight stop loss that falls within market noise, the score is mathematically penalized for structural invalidity.  
**Tier 3: Reasoning and Trajectory Analysis (LLM-as-a-Judge)** The most sophisticated tier evaluates the agent's semantic output. Because deterministic rules cannot interpret narrative logic, the sandbox utilizes a secondary, highly calibrated LLM instance acting as a judge.20 The judge is isolated from the trading agent and is prompted with a strict scoring template designed to minimize positional and agreeableness biases.27  
The judge evaluates the agent's reasoning field against the 10-point pre-trade checklist. It extracts binary evidence: Did the agent explicitly classify the market regime? Did it articulate a logical thesis in two sentences? Did it define the technical invalidation level? Crucially, the judge is responsible for evaluating "Hold" decisions. Grading a null action requires analyzing whether the agent correctly identified unfavorable conditions (such as a tightening range or contradictory macro catalysts) or simply failed to recognize a valid setup. By forcing the judge to extract concrete textual evidence from the agent's output, the evaluation remains grounded in verifiable logic rather than subjective interpretation.20

### **Mitigating LLM Non-Determinism**

A critical hurdle in evaluating LLM-based architectures is the inherent non-determinism of the underlying models. Even at a temperature of 0.0, variations in floating-point math aggregation across GPU thread processing and unpredictable batch-size invariance during concurrent inference can cause the model to generate slightly divergent responses to identical input states.30  
To counter this, the sandbox treats the evaluation as a statistical measurement rather than a single point-in-time check. The execution harness implements a multiple-pass evaluation protocol, running the exact same scenario through the agent ![][image2] times (typically ![][image3]).32 The system calculates the Total Agreement Rate (TAR) for both the raw output action and the underlying reasoning structure. If the agent returns identical actions and logic across all five runs, it receives a high confidence score. If the output drifts—for example, generating three "Buy" decisions and two "Hold" decisions—the variance flags a critical ambiguity in the SOUL.md prompt regarding that specific market condition, signaling to engineers that the persona instructions lack sufficient precision.33

## **Scalable Test Harness and Execution Architecture**

Subjecting the agent to thousands of complex scenarios requires an execution harness built for massive concurrency, efficient state management, and robust error handling. Processing 100 scenarios, with each scenario evaluating 15 pairs simultaneously, results in 1,500 distinct LLM evaluations. To complete this test suite in under two hours, the harness fully leverages the Rust tokio asynchronous runtime while strictly respecting the external rate limits of the OpenGateway API.

### **Concurrent Processing and Semaphore Rate Limiting**

The execution engine is orchestrated via tokio::task::JoinSet, which allows the system to spawn thousands of asynchronous evaluation tasks and await their completion efficiently. However, submitting 1,500 parallel requests to an external API gateway will inevitably trigger HTTP 429 Too Many Requests errors, leading to cascading connection failures and corrupted evaluation data.  
To gracefully manage throughput, the architecture implements a tokio::sync::Semaphore configured with a permit count that matches the optimal requests-per-second limit of the API.35

Rust  
// Architecture for rate-limited concurrent evaluation  
let api\_concurrency\_limit \= 50;
let permits \= Arc::new(Semaphore::new(api\_concurrency\_limit));  
let mut evaluation\_set \= JoinSet::new();

for scenario in scenario\_library {  
    for pair in active\_pairs {  
        let permits \= Arc::clone(\&permits);  
        let context \= build\_scenario\_context(scenario, pair);

        evaluation\_set.spawn(async move {  
            // Acquire a permit before initiating the network request  
            let \_permit \= permits.acquire().await.unwrap();  
              
            // Execute the Phase 2 LLM network call  
            let response \= execute\_agent\_evaluation(context).await;  
              
            // Permit is automatically released when \_permit drops  
            return process\_and\_grade\_response(response);  
        });  
    }  
}

This semaphore-backed design ensures that the network pipeline is saturated to exactly the maximum allowed threshold without ever exceeding it, optimizing total execution time while maintaining connection stability.37

### **Context Construction and Token Management**

The agent operates with a massive 1M token context window provided by the mimo v2.5 pro model. During Phase 2 of the evaluation cycle, the harness constructs the user message payload. This involves aggregating the 721 historical candles, the computed indicator values (EMA, RSI, ATR, ADX), order book depth, and the injected insights (sentiment, funding, on-chain data, and RSS feeds). The resulting string, formatted systematically via Markdown, averages approximately 13,000 characters per pair.  
Concurrently, the harness injects the system prompt, appending the 560-line SOUL.md persona, operational risk constraints, and dynamic knowledge base elements retrieved via vector search. The harness enforces strict serialization protocols, utilizing Rust's serde\_json to parse the LLM's response, extracting the JSON decision object from the surrounding markdown code blocks, and normalizing the structural casing to ensure downstream systems can execute the hypothetical trade without parsing errors.

### **Relational Storage Schema for Evaluation Analytics**

Tracking the trajectory of an evolving AI agent necessitates a highly structured data storage strategy. The sandbox utilizes SQLite as a lightweight, serverless relational database engine, which is exceptionally well-suited for logging machine learning experiment data, performance metrics, and complex execution traces without the overhead of maintaining a standalone database server.39  
The database is normalized to track runs across different iterations of the SOUL.md prompt, ensuring developers can perform regression testing. The schema is defined as follows 43:

| Table | Primary Columns | Relational Integrity | Function |
| :---- | :---- | :---- | :---- |
| soul\_versions | version\_hash, timestamp, content | Parent | Stores the exact text of the persona to correlate performance shifts. |
| scenario\_catalog | scenario\_id, category, expected\_action | Parent | The static metadata describing the curated test library. |
| evaluation\_runs | run\_id, version\_hash, start\_time | Foreign Keys to soul\_versions | Groups a complete execution of the test suite. |
| agent\_decisions | decision\_id, run\_id, scenario\_id, json\_payload | Foreign Keys to runs, scenarios | Stores the raw textual and JSON output generated by the LLM. |
| rubric\_scores | score\_id, decision\_id, tier\_1, tier\_2, tier\_3 | Foreign Key to decisions | Stores the parsed numerical scores and LLM judge rationales. |
| telemetry | decision\_id, latency\_ms, token\_count | Foreign Key to decisions | Logs network execution profiles and token usage. |

This relational structure allows engineers to execute complex SQL queries to extract performance insights, such as calculating the average Tier 3 reasoning score specifically during "Volatility" scenarios when utilizing version 2.4 of the SOUL.md prompt.

## **Feedback Loops and Continuous Prompt Optimization**

The ultimate utility of the sandbox extends beyond simple measurement; it acts as an automated engine for continuous agent improvement. The vast repository of graded execution traces provides the empirical data required to systematically refine the agent's prompt architecture, reducing hallucination rates and increasing adherence to risk protocols.

### **Automated Self-Reflection and Prompt Evolution**

When the database identifies a systemic failure—for instance, the agent consistently exceeding the 6% maximum correlation limit during broad market rallies—the system triggers an automated self-reflection pipeline.46 Leveraging optimization frameworks conceptually similar to GEPA (Genetic-Pareto), the execution harness feeds the failed reasoning trace, the market state context, and the judge's critique back into a dedicated optimization LLM.47  
The optimization prompt initiates a dialectic loop: *"Review the following scenario where your prior iteration scored 0/10 for violating the correlation limit. Analyze your reasoning trace to identify the cognitive failure. Propose a specific, highly explicit linguistic modification to the SOUL.md constraints to prevent this failure, ensuring the modification is measurable and absolute."*.49  
The system then automatically updates the SOUL.md, creating a new version hash, and initiates a targeted regression test. It isolates the subset of scenarios where the agent previously failed and reruns them. If the Total Agreement Rate and rubric scores improve without degrading performance in other scenario categories, the prompt mutation is permanently merged into the production codebase.

### **Managing Exploration vs. Exploitation**

A critical balance must be maintained during prompt evolution to prevent overfitting the persona to the curated scenario library. If the SOUL.md becomes too rigidly optimized for the exact parameters of the 50 test cases, the agent loses its capacity for dynamic reasoning when faced with live, ambiguous market conditions. To mitigate this, the scenario generation engine continuously introduces stochastic variations into the test data—slightly altering volatility constraints, shifting the timing of RSS news injections, and modulating order book depth—ensuring the agent learns the underlying *principles* of risk management rather than memorizing the precise mathematical signatures of the test suite.

## **Episodic Memory System Integration**

Savant employs an advanced episodic memory system, utilizing an Obsidian Vault to store execution traces, market conditions, and post-trade reflections as interconnected markdown graphs. This allows the agent to utilize Retrieval-Augmented Generation (RAG) during live trading to recall past experiences and apply learned patterns to current market states.  
Integrating the sandbox with this memory system introduces the risk of data contamination. The agent must not retrieve the memory of a simulated flash crash and interpret it as historical fact when making live financial decisions.

### **Tagging, Isolation, and Bootstrapping**

To ensure pristine data boundaries, the execution harness applies strict cryptographic and metadata tagging to all outputs generated within the sandbox. Every file written to the Obsidian Vault during a test run is appended with is\_simulation: true and linked directly to the specific scenario\_id via frontmatter metadata.  
While these simulated memories are isolated from the live RAG context pipeline, they serve an invaluable role in *bootstrapping* the memory system. When a completely new agent is initialized, it possesses zero experiential memory. By running the agent through the 50-scenario test suite, the vault is populated with synthetic, graded experiences. Engineers can explicitly prompt the live agent to query its simulated memory bank (e.g., *"Retrieve your simulation data regarding your performance during Asian low-volume sessions"*), granting the agent access to thousands of hours of synthetic trading experience and optimized risk protocols before it ever executes a trade with real capital.

## **Comprehensive Reporting and Visualization**

The granular data collected within the SQLite database is transformed into highly visual, actionable intelligence, ensuring the engineering team can immediately identify behavioral drift or cognitive weaknesses across different prompt iterations.

### **The Sandbox Report Card**

Following the completion of an evaluation suite, the system aggregates the rubric scores into an executive report card. The primary metric is the Overall Compliance Ratio, defined as the percentage of scenarios where the agent successfully completed all 10 pre-trade checklist items and violated zero absolute circuit breakers. A secondary metric, the Trajectory Exact Match score, evaluates how perfectly the agent's internal reasoning mapped to the predefined optimal logic of the scenario.20

### **Data Visualization Workflows**

The reporting engine interfaces with the Savant TUI dashboard and generates automated Markdown reports within the Obsidian Vault. The core visual analytic tools include:

* **Regime Performance Heatmaps:** A two-dimensional matrix plotting the agent's Win Rate and Compliance Ratio across intersecting variables. For example, plotting Market Regimes (Trending, Ranging, Volatile) against Trading Sessions (Asian, US, Weekend) instantly highlights cognitive blind spots, revealing if the agent operates flawlessly during US Trend conditions but consistently overtrades during Asian Range scenarios.  
* **Violation Frequency Distributions:** A bar chart analyzing Tier 1 failures, pinpointing exactly which SOUL.md rules are broken most frequently. If the "Never chase entries" rule represents 80% of all violations, engineers know exactly where prompt optimization must be focused.  
* **Drift Analysis Charts:** Time-series line graphs mapping composite evaluation scores across sequential commits to the SOUL.md file. By tracking these distributions over time, the system provides an immediate visual signal if a recent adjustment to the persona inadvertently caused a systemic regression in agent performance.33

## **Implementation Roadmap**

The deployment of the Savant Sandbox Architecture requires a phased integration strategy to ensure the deterministic testing environment is completely stable before the LLM evaluation pipelines are activated.  
**Phase 1: Environment Simulation and Mocking (Weeks 1-2)**

* Implement the wiremock local server infrastructure to intercept all out-bound HTTP requests to Kraken, CoinGecko, Alternative.me, and the RSS aggregators.  
* Develop the mathematical core of the synthetic data generator, programming the Heston and GARCH models to produce geometrically valid, volatility-clustered OHLCV data.  
* Implement the Hawkes process LOB simulator and integrate the order-book walking and slippage calculation logic into the execution engine.

**Phase 2: Execution Harness and Database Architecture (Weeks 3-4)**

* Build the tokio-based parallel execution engine, carefully tuning the Semaphore implementation to maximize asynchronous throughput while respecting OpenGateway rate limits.  
* Deploy the SQLite relational schema and instrument the engine to log all input context, raw JSON responses, and network telemetry accurately.

**Phase 3: Scenario Encoding and Grading Rubric Calibration (Weeks 5-6)**

* Mathematically define and encode the 50 baseline scenarios into the database catalog, establishing the strict trigger conditions and expected parameters for each.  
* Develop the 3-Tier grading rubric. Implement the deterministic Tier 1 and Tier 2 Rust parsers, and engineer the precise prompts for the Tier 3 LLM-as-a-judge framework, ensuring the judge is calibrated to minimize bias and extract objective evidence.

**Phase 4: Feedback Loops and Visualization Pipelines (Weeks 7-8)**

* Implement the automated GEPA-style reflection pipelines, allowing the system to self-correct failed scenarios by proposing specific mutations to the SOUL.md file.  
* Build the SQL-to-Markdown reporting engine, generate the performance heatmaps, and integrate the telemetry readouts directly into the Savant TUI dashboard and the episodic Obsidian Vault.

By executing this rigorous, multi-layered architecture, the evaluation of the Savant trading agent evolves from passive historical backtesting into active, cognitive stress testing. This ensures the underlying large language model is mathematically constrained, philosophically aligned with its persona, and empirically proven to manage risk under extreme market pressure before it is permitted to engage with live capital.

### **Works cited**

1. Bayesian Robust Financial Trading with Adversarial Synthetic Market Data \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2601.17008v1](https://arxiv.org/html/2601.17008v1)  
2. Liquidity Shocks and Order Book Dynamics \- Toulouse School of Economics, accessed May 30, 2026, [https://www.tse-fr.eu/sites/default/files/medias/doc/wp/fit/wp\_fit\_37\_2009.pdf](https://www.tse-fr.eu/sites/default/files/medias/doc/wp/fit/wp_fit_37_2009.pdf)  
3. Dynamic Modeling of Limit Order Book and Market Maker Strategy Optimization Based on Markov Queue Theory \- MDPI, accessed May 30, 2026, [https://www.mdpi.com/2227-7390/13/5/778](https://www.mdpi.com/2227-7390/13/5/778)  
4. Human Labeled OHLCV Stock Market Data \- Kaggle, accessed May 30, 2026, [https://www.kaggle.com/datasets/barathanaslan/human-labeled-synthetic-stock-market-data](https://www.kaggle.com/datasets/barathanaslan/human-labeled-synthetic-stock-market-data)  
5. Generating synthetic data in finance: opportunities, challenges and pitfalls \- J.P. Morgan, accessed May 30, 2026, [https://www.jpmorgan.com/content/dam/jpm/cib/complex/content/technology/ai-research-publications/pdf-8.pdf](https://www.jpmorgan.com/content/dam/jpm/cib/complex/content/technology/ai-research-publications/pdf-8.pdf)  
6. Evaluating generative models for synthetic financial data \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2512.21791v1](https://arxiv.org/html/2512.21791v1)  
7. How to Generate Synthetic Financial Data: A Practical Guide (Part 1\) | by Abdullah Hassan, accessed May 30, 2026, [https://medium.com/@abdullahhassan.me/how-to-generate-synthetic-financial-data-a-practical-guide-part-1-8ee90b4c9292](https://medium.com/@abdullahhassan.me/how-to-generate-synthetic-financial-data-a-practical-guide-part-1-8ee90b4c9292)  
8. Heston model \- MATLAB \- MathWorks, accessed May 30, 2026, [https://www.mathworks.com/help/finance/heston.html](https://www.mathworks.com/help/finance/heston.html)  
9. Heston Model: Options Pricing, Python Implementation and Parameters \- QuantInsti Blog, accessed May 30, 2026, [https://blog.quantinsti.com/heston-model/](https://blog.quantinsti.com/heston-model/)  
10. Synthetic OHLC Simulation Via DCC–GARCH \- Dr Krzysztof Ozimek, accessed May 30, 2026, [https://www.drkrzysztofozimek.com/synthetic-ohlc-simulation-dcc-garch-textbook/](https://www.drkrzysztofozimek.com/synthetic-ohlc-simulation-dcc-garch-textbook/)  
11. ProteuS: A Generative Approach for Simulating Concept Drift in Financial Markets \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2509.11844v1](https://arxiv.org/html/2509.11844v1)  
12. Limit Order Book Simulations: A Review \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2402.17359v1](https://arxiv.org/html/2402.17359v1)  
13. Simulating Limit Order Book Models | Samuel Watts, accessed May 30, 2026, [https://samueldwatts.com/wp-content/uploads/2017/05/Watts-Modelling\_Limit\_Order\_Books.pdf](https://samueldwatts.com/wp-content/uploads/2017/05/Watts-Modelling_Limit_Order_Books.pdf)  
14. Backtesting \- NautilusTrader Documentation, accessed May 30, 2026, [https://nautilustrader.io/docs/latest/concepts/backtesting/](https://nautilustrader.io/docs/latest/concepts/backtesting/)  
15. What is slippage? Order books, AMMs, and how to minimize it \- MetaMask, accessed May 30, 2026, [https://metamask.io/news/what-is-slippage](https://metamask.io/news/what-is-slippage)  
16. How to Mock External APIs in Rust Tests with wiremock \- OneUptime, accessed May 30, 2026, [https://oneuptime.com/blog/post/2026-01-07-rust-wiremock-mocking/view](https://oneuptime.com/blog/post/2026-01-07-rust-wiremock-mocking/view)  
17. Mocking in Rust: Mockall and alternatives \- LogRocket Blog, accessed May 30, 2026, [https://blog.logrocket.com/mocking-rust-mockall-alternatives/](https://blog.logrocket.com/mocking-rust-mockall-alternatives/)  
18. Idiomatic Rust way of testing/mocking \- help \- The Rust Programming Language Forum, accessed May 30, 2026, [https://users.rust-lang.org/t/idiomatic-rust-way-of-testing-mocking/128024](https://users.rust-lang.org/t/idiomatic-rust-way-of-testing-mocking/128024)  
19. How do you mock external struct in unit tests : r/rust \- Reddit, accessed May 30, 2026, [https://www.reddit.com/r/rust/comments/1ikd0tx/how\_do\_you\_mock\_external\_struct\_in\_unit\_tests/](https://www.reddit.com/r/rust/comments/1ikd0tx/how_do_you_mock_external_struct_in_unit_tests/)  
20. How to Build an Agent Evaluation Framework With Metrics, Rubrics ..., accessed May 30, 2026, [https://galileo.ai/blog/agent-evaluation-framework-metrics-rubrics-benchmarks](https://galileo.ai/blog/agent-evaluation-framework-metrics-rubrics-benchmarks)  
21. How to Pass the FTMO Challenge and Get Funded in 2026, accessed May 30, 2026, [https://www.goatfundedtrader.com/blog/pass-ftmo-challenge](https://www.goatfundedtrader.com/blog/pass-ftmo-challenge)  
22. What is the FTMO Challenge, accessed May 30, 2026, [https://ftmo.com/en/challenge/](https://ftmo.com/en/challenge/)  
23. FTMO Prop Firm Review 2026: Payouts, Rules & How to Pass \- Directions Magazine, accessed May 30, 2026, [https://www.directionsmag.com/reviews/prop-trading-firms/ftmo-prop-firm](https://www.directionsmag.com/reviews/prop-trading-firms/ftmo-prop-firm)  
24. FTMO Prop Firm Review: How to Pass in 2026 \- LuxAlgo, accessed May 30, 2026, [https://www.luxalgo.com/blog/ftmo-prop-firm-review-how-to-pass-in-2025/](https://www.luxalgo.com/blog/ftmo-prop-firm-review-how-to-pass-in-2025/)  
25. LLM-as-a-judge explored \- Medium, accessed May 30, 2026, [https://medium.com/online-inference/llm-as-a-judge-explored-2c6cd0d169fe](https://medium.com/online-inference/llm-as-a-judge-explored-2c6cd0d169fe)  
26. LLM-as-a-judge: a complete guide to using LLMs for evaluations \- Evidently AI, accessed May 30, 2026, [https://www.evidentlyai.com/llm-guide/llm-as-a-judge](https://www.evidentlyai.com/llm-guide/llm-as-a-judge)  
27. How to Calibrate LLM-as-a-Judge with Human Corrections \- LangChain, accessed May 30, 2026, [https://www.langchain.com/articles/llm-as-a-judge](https://www.langchain.com/articles/llm-as-a-judge)  
28. Calibrating Scores of LLM-as-a-Judge \- GoDaddy Blog, accessed May 30, 2026, [https://www.godaddy.com/resources/news/calibrating-scores-of-llm-as-a-judge](https://www.godaddy.com/resources/news/calibrating-scores-of-llm-as-a-judge)  
29. LLM-as-a-Judge, Done Right: Calibrating, Guarding & Debiasing Your Evaluators \- Kinde, accessed May 30, 2026, [https://kinde.com/learn/ai-for-software-engineering/best-practice/llm-as-a-judge-done-right-calibrating-guarding-debiasing-your-evaluators/](https://kinde.com/learn/ai-for-software-engineering/best-practice/llm-as-a-judge-done-right-calibrating-guarding-debiasing-your-evaluators/)  
30. Defeating Nondeterminism in LLM Inference \- Thinking Machines Lab, accessed May 30, 2026, [https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/](https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/)  
31. Defeating Nondeterminism in LLM Inference \- OpenAI Developer Community, accessed May 30, 2026, [https://community.openai.com/t/defeating-nondeterminism-in-llm-inference/1358623](https://community.openai.com/t/defeating-nondeterminism-in-llm-inference/1358623)  
32. Non-Determinism of “Deterministic” LLM Settings \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2408.04667v5](https://arxiv.org/html/2408.04667v5)  
33. You Can't Assert Your Way Out of Non-Determinism: A Practical QA Strategy for LLM Applications | by Venkat Peri \- Medium, accessed May 30, 2026, [https://medium.com/@venkatperi/you-cant-assert-your-way-out-of-non-determinism-a-practical-qa-strategy-for-llm-applications-fd32e617cdec](https://medium.com/@venkatperi/you-cant-assert-your-way-out-of-non-determinism-a-practical-qa-strategy-for-llm-applications-fd32e617cdec)  
34. The Good, The Bad, and The Greedy: Evaluation of LLMs Should Not Ignore Non-Determinism \- ACL Anthology, accessed May 30, 2026, [https://aclanthology.org/2025.naacl-long.211.pdf](https://aclanthology.org/2025.naacl-long.211.pdf)  
35. Semaphore in tokio::sync \- Rust \- Docs.rs, accessed May 30, 2026, [https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)  
36. How to accomplish token rate limiting with tokio? \- Stack Overflow, accessed May 30, 2026, [https://stackoverflow.com/questions/77763087/how-to-accomplish-token-rate-limiting-with-tokio](https://stackoverflow.com/questions/77763087/how-to-accomplish-token-rate-limiting-with-tokio)  
37. Using tokio::sync::Semaphore to limit async requests in a block \- Stack Overflow, accessed May 30, 2026, [https://stackoverflow.com/questions/73808437/using-tokiosyncsemaphore-to-limit-async-requests-in-a-block](https://stackoverflow.com/questions/73808437/using-tokiosyncsemaphore-to-limit-async-requests-in-a-block)  
38. Can Tokio Semaphore be used to limit spawned tasks? \- Rust Users Forum, accessed May 30, 2026, [https://users.rust-lang.org/t/can-tokio-semaphore-be-used-to-limit-spawned-tasks/59899](https://users.rust-lang.org/t/can-tokio-semaphore-be-used-to-limit-spawned-tasks/59899)  
39. How SQLite Is Tested, accessed May 30, 2026, [https://sqlite.org/testing.html](https://sqlite.org/testing.html)  
40. rclement/sqlite-ml: An SQLite extension for machine learning \- GitHub, accessed May 30, 2026, [https://github.com/rclement/sqlite-ml](https://github.com/rclement/sqlite-ml)  
41. Managing Data for Machine Learning Projects \- MachineLearningMastery.com, accessed May 30, 2026, [https://machinelearningmastery.com/managing-data-for-machine-learning-project/](https://machinelearningmastery.com/managing-data-for-machine-learning-project/)  
42. SQLStructEval: Structural Evaluation of LLM Text-to-SQL Generation \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2604.06736v1](https://arxiv.org/html/2604.06736v1)  
43. Running LLM evals right next to your code \- maragu, accessed May 30, 2026, [https://www.maragu.dev/blog/running-llm-evals-right-next-to-your-code](https://www.maragu.dev/blog/running-llm-evals-right-next-to-your-code)  
44. Evaluation Developer Guide \- Datadog Docs, accessed May 30, 2026, [https://docs.datadoghq.com/llm\_observability/guide/evaluation\_developer\_guide/](https://docs.datadoghq.com/llm_observability/guide/evaluation_developer_guide/)  
45. Building an LLM Evaluation Framework That Actually Works \- DEV Community, accessed May 30, 2026, [https://dev.to/ritwikareddykancharla/building-an-llm-evaluation-framework-that-actually-works-4585](https://dev.to/ritwikareddykancharla/building-an-llm-evaluation-framework-that-actually-works-4585)  
46. When Can LLMs Actually Correct Their Own Mistakes? A Critical Survey of Self-Correction of LLMs \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2406.01297v3](https://arxiv.org/html/2406.01297v3)  
47. GitHub \- gepa-ai/gepa: Optimize prompts, code, and more with AI-powered Reflective Text Evolution, accessed May 30, 2026, [https://github.com/gepa-ai/gepa](https://github.com/gepa-ai/gepa)  
48. Efficient and Accurate Prompt Optimization: the Benefit of Memory in Exemplar-Guided Reflection \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2411.07446v2](https://arxiv.org/html/2411.07446v2)  
49. Feedback Loops With Language Models Drive In-Context Reward Hacking \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2402.06627v3](https://arxiv.org/html/2402.06627v3)  
50. Teach Your LLM to Teach You Back: Feedback Loops & Crucial Cues \- Medium, accessed May 30, 2026, [https://medium.com/@S01n/teach-your-llm-to-teach-you-back-feedback-loops-crucial-cues-b2ab07e6906d](https://medium.com/@S01n/teach-your-llm-to-teach-you-back-feedback-loops-crucial-cues-b2ab07e6906d)

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAS8AAAAaCAYAAADmHtpyAAAJZUlEQVR4Xu2ceYhmxRHAS2IkmngExcULxwODxmvxQrxWcb3ihYpHsoh4YJAFJYsGD8QDQfE+QBF1FZGI+o94ov7xacQTskTWA3fFVcwGXFQSNOAGY+pnfeX09Nf9ju8YZ77pHxQ70/1ev/e6qquq671ZkUKhUCgUCoVCYSbyM5Vfq6wTd8wwNlb5edxY6KHoszAnwHiuUzkp7kiwqcoRKqeo7Cy2SIYF9/FbsbEXqPyi284C3FHsWvuqPCJm9IU0TfXJvG6pcpzKiSoT3bZhUfQ5JA5VWaPyfSBrVV4UW5CzDQzgbJVHVbaO+tqyROUWyRsu7QervKXynMofuvKCygqVfSYP7YuNVO4Q00dH5Wqxxfc3ld+pXKJymx+sLFK5R0rEzlGnT5zGqSofqjykcrrKOSrLVF4Tc2KDMNv1uZnKX6XXVzwo5oBvUPk66PtOZbnKbjJi7lP5n8pRcccQwChOUHlD5bSob9j8SswwmDwiZ78QGd9W2S7u6IJBoayPpddJ0Xevylcq86O+phwmFlSelF4nTFBhMfGMYRaxvspTUdugsOCuUHlFZYeobzZRp08yHAIegWhiatcPfS+pvC+9umjKOOnzaLF7xRHH4ODeVVmlstXUrtFADQDFrpLhXpBFTPQislyo8sup3SPjAJWLpf/rEZnv6koqSvNcd6t8KZbep2Dr+IXkx6gCYyWi4QBzUfd8sfG5TgjzzUIYdLvBgrpRLNKy8Ia5DZ5u6vTJXJE5k3FtG/U5h4sF98VxRwPGTZ9kiziv4+MOZS+Vb1SeUFk36hsJvtCGdUGcBs7qPbEtHBFkNoEDx5Ax2BR/FDNk/s3hAYEoRDRqChkCEZrIWGWwZJUY4oZR+/YqK1UOjNqbQmZyv9j195P0Yp9tVOmT52OrxhZnYdQXwrz+U+UZmaxRNWHc9MmzMwfMBfcWQ9kEx4Yznhb8gn+OO1riKSnbQwqduShTBds+0mqK3zhVxthbbLxtuseggJ0kXRzfRCx6YgyeRTYZMwQj/0h603ugoLpabAsxL+oLcef1icoWUV8O7usxMcdYt1Xg+VJpO4GDLc5lcUcNv1F5vCu7SP9GzjMwL2SkC7q/T4jNNVHZdUUmwDNwTOwMOIZjLxCrA4XngV+DeUU2Fwu6jBm2+TlV+jxI5VuV56U6yLrz6ojZUxPGQZ8xVfPANZaq/Ff6d7atYdIGuSDGQiF0GCkpRv4PMWdKmv2wmHO9XCwdPbfb/iexzIdj2Q54xojB+/le86ob8+TucQ5OvCO9yoGrxMYhda7CldzGee0vtpBweji/KljwqfuDh8TeVNUZLP1EY7ZMRGei9KBMqLwsNkcEMQrO6OpMMQdyn1hWjs7YEr2g8neZdCyeCX0q5gAJNtSJuEfPXCa6bV4cZltFoGKh8jvt1K8IZJDTJzbzF7FzsIcqWBuskY70jpNjHPQZ49tn9OeBwoUs8wNpv9voG88QMKyqTCIHRrdC5XaVDaK+fmHMz2RqdkM6jXPEGIiWDk4kdhAU0P8jUwv2VWPGWwGMJbWF9pcBKC+1BQlxJae2AjlYZLlCaBtyizWGlyj/FssK6hZGGxiLxRa/ACJ74Pmu7R4D7hRcV+5QuK89um27ir384LlCJsTs1mtJZ4k5xziDyukTh7dK0rWmGLZB3HsTJ+KMiz5DWG88Ew6SeQ+FQERfaq5HQl29az2p3/7RP8zCPI4IhxQq3R1H7AxQbOy8vGgYOq+qMZHQMDB2JMbHaGLsXAdFXhW1V8E1m2QBdTAnTaI9DLOQG8KzxBGY+yKohG9nU7oikHjWBD7vKZ2QNXM+jhGnl6or5fTp147vM8brPOgGO2/KOOkTZmy9i9Q+Bu99jVjkawIThfdnu3CFWA2sH9xYw0ibczRtnVeTMXPG7mPE14uhjkY2ukYslW6KG3t43ynISJbEjQE8Y9tMOqxXosNBjZ5n6UivrtAL+nFSuuLaOCXuhW0IgZWMO6UTbJRtJlneMVGfk9OnX7sj1VmNH7dcrJbWlHHSJ3gphBpcnKCgh6UyWPmpNaTZuQvuKLZ3jtPwOpgovD7en1pY1UJP0cbRTKfzIjoTpePrhaDES8WMtsogU3hKXmXsjI9RLow7AnjGjlQvyBwYJdkzWTR1qra6d/p1XhxPkZual2doVZmXzwdjsKjaZF6+6+hIfq7YVbAov5P0pwFVjJM+4Wix50nVe/t9u943Vd93oTT2sYOkvChmd2lfQGzjaEbhvFBOKrp4PSbn7IEiM/UZr8OE8DuRO253Fogtktz3SLCfyk2SHwNy99+GQUsB/TovrxUu/vGIqc6L/rDeSIZ2p9i3fcx7WE9zcvOBoyMzyW2DgPHRN28943Hnkj6B+8B5pZy46/EJSZefhk7ughgLi4/o19Th1OGvbs+IOxJsLVZcD18P5xwNCyKuQcULAtqMyZ49F0Hmiy0SonFscGSbn4tlm6kId7NU18E8YDB+uEAB4z9S6v/ejeM4ZtAiseOlAD4lIBNvgt9Dqj6Zc15eJ3HnFQaZhWJOAOdFMEWvPh/LZPJNJQ6P43A4IVX6ZCFyDtly6GD4mUzlX2KLPbXtmiv6BF8rOUfvLzRS5aehwkSuFrsYgvJY2Ag/eztZxrR40QA+f1grk/fAn2ssEvua3dv4mbbXgzbO4Vwch5/Ps9zabW8yphsYC4qaVa4oz3Zmpdg4LCaE7PIdMQeWi7IXiS1MomjoLENwehRcufeOTH7rhCO4uNtfBdk095WKjtPBbmL1GZ9XPltI6Qo9hbpCCG4U6m8QK+zzOzZ4pZgz41jm4fcy1U5ZMGQSzGtbfaKrY8UcDJ9f+N8zvqnyqtg3Ujnmgj7JLLnXUE/olJ3UemI18XAdIdQGq+atMEJ8O1G1ZSaCsRhOEcsEtpS80woh+rPNqUvbuYcjxMZnKxp+ylHF/mKfgwwrY/6p4HnZAYROgcXSZI5jmuiTLGlvsflm3jed2p2l6HMWgzGhaAytiTRV2k8N2cKzUh8Z24IxEnlHAbqg5pOq+6Rgwc6TXh2lhAic2jrNFoo+x0ufQ4E0/3rp/WAtJwfbaTMeIv7TUv0WqC2M+YC0+3yiDdQw2MJsG3dk2FN69ZMT6jsY/Wyl6HO89FmoAaOhDtLUeOpgm3lI3DgkiLpLpf5v6OYyRZ+FOQUGerVYvWUmc56M5v9iGzeKPguFQqFQKBQKhUKhUCgUCoXCjOH/n5qw4BpOXq0AAAAASUVORK5CYII=>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABMAAAAaCAYAAABVX2cEAAABHElEQVR4XmNgGAWUAkcgfg3E/6F4BxBzIsnzAfEuJHkQXgfE3EhqUAAjEM8C4l9A/BOILVGlwSAIiNcwoFqEFQgC8UIgzmeA2DyFAWIBMigC4mg0MaxAH4j7gVgSiK8D8RMgVkSSZwHi2VB1BAHIxnQou4EB4rocuCwDgwgDxOUgHxAEfUBsDGXrAPF7ID4BxPxQMRsgngxl4wWw8ALZDgIgLy0H4n9A7AEVA7mapPBCDnCQISDDQIaCYo+s8IIBkPdA3gR514mByPACuQYUFqboEkAQwwCJiGtA3IkmhxWghxcyEGeAJBOQgUSFF8gLoKzBhS4BBQ1A/BaINdHEUYALEH9hQOQ1UBbyRlEBAaBkAsqrBMNrFIyCIQMA260zNBT6yKgAAAAASUVORK5CYII=>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADgAAAAaCAYAAADi4p8jAAAB+klEQVR4Xu2WPyhFURzHf8Lgf0qkGJRFKSTqyUKSssifJBYZSEoxyPZWC4UJJZNCZlnYGGQSk4FswmLwDPh+37n3vXNv715X8q44n/rU6/zO6d1zzu93zhExGAy/iTZ4D98tD2COFi+Eh1qc7sM8rU86GIAtMB9mwFI4Auv1Tl5wwBp8hTEYcYbj9MI9cU4+XWTBHXEuMt2FRVo/T4rhFpwWNXBV1KR1ZuCwqy2dbMALeCtqobthpqOHD3VwCZbDK3gHq7Q4V3Dd6hcWK7DR3RgU7sy49TsqahenElGRElE7zJ0Oi29NcFGSg2vhEzyVZH63ivqDMGHZMMvORWXYCWxw9PDArj/uEmE6bsM32GW1cXeD1l81PBNVK0Edio/0ZxPOS7LueII+wuZEDw/s+tMPFU6ME+REeWqGXX+kQJyHSoWoneQ3clM80evPhqnJFGWqtkv49ZcKHog38BqWuWIJuGusrSZ3QFQK8LC5hAuumB9cZV7C/ICg8vL2Y1RURk1obfYEKX+nxF1/OlwVXhmcZND6I3zh8I7q/4I18ZHezIn6Dn2Cdooei88CMf347Mp1Byyi8EE+/4CfJiLqFM3W2gbhC+zT2hJ0wGdJPnliolbdDa8Mvk3Drj+W0iw8gmNwWdTCT1qxP0Ml7IGdEvANajAYDAbDf+MD5jxnLOkGrx4AAAAASUVORK5CYII=>
