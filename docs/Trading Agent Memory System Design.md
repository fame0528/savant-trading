# **Persistent Memory Architecture for the Savant Autonomous Trading Agent**

## **Introduction and System Objectives**

The evolution of an autonomous trading agent from a stateless execution engine into a cognitive system capable of continuous adaptation requires a fundamental shift in software architecture. Currently, the Savant trading engine operates with an expansive context window, processing market conditions through a dynamic prompt structure that leverages a massive initial knowledge base of curated trading concepts. However, this configuration forces the agent to evaluate each market state in isolation. It lacks the capacity to compound its own experience, adapt to shifting market regimes, or correct its inherent cognitive biases based on empirical outcomes. In financial markets, where alpha decays rapidly as participants adapt, a static intelligence is mathematically destined for eventual drawdown.  
To transform Savant into an adaptive system, a persistent memory architecture must be implemented. This system must move beyond simple trade logging—a function already handled by the existing SQLite schema—to capture the full cognitive state of the artificial intelligence at the exact moment of decision. The architecture must learn from its trading history, extract actionable mathematical patterns, and seamlessly feed those validated patterns back into its real-time decision-making process. The objective is to design a memory hierarchy that captures high-fidelity episodic traces, extracts generalized semantic rules, accurately calibrates the agent's confidence, and manages state within the strict constraints of a high-concurrency Rust ecosystem. Crucially, this memory system must operate within the agent's parallel evaluation cycle, analyzing 15 cryptocurrency pairs simultaneously without exceeding a 500-millisecond latency budget for memory querying.

## **Memory Architecture Design**

The foundation of a learning agent lies in its cognitive architecture. Modern stateful agents are transitioning away from monolithic context windows toward operating-system-like virtual memory systems, enabling explicit hierarchical tiering that mirrors human cognition.1 To simulate the accumulation of "market memory" developed by human traders over thousands of hours of screen time, the agent must implement a multi-tiered memory architecture that balances the richness of retained history against the efficiency of real-time retrieval.2

### **Dual-Process Cognitive Architecture**

Drawing upon established frameworks such as MemGPT and Letta, the memory system must be divided into processes that differentiate between immediate working context and long-term persistent storage.2 This prevents the large language model from suffering from attention dilution, a common failure mode when too much historical data is crammed into the active context window.  
The system will utilize four distinct tiers of memory:

1. **Working Memory**: This represents the agent's immediate operational space. In the Savant architecture, this corresponds to the dynamically assembled prompt containing the base identity, active indicators, and real-time market data. Working memory is highly volatile and scoped entirely to the current evaluation cycle for a specific trading pair. It is strictly limited by the context window allocation.  
2. **Core Memory**: A persistently maintained, highly compressed representation of the agent's identity, active operating rules, and most critical learned parameters. Core memory is always visible in the context window.2 It acts as a bridge between the agent's vast historical archives and its immediate decision-making framework, ensuring that overriding constraints and heavily validated lessons are never forgotten.  
3. **Episodic Memory**: A time-stamped, immutable ledger of specific past events.2 For a trading agent, this entails a complete snapshot of the market state, the agent's internal reasoning process, the condition tags present, and the ultimate financial outcome of every decision. Crucially, this tier must store both trades taken and setups that were explicitly rejected.  
4. **Semantic Memory**: Generalized knowledge, statistical facts, and heuristic rules abstracted from the episodic memory.5 This includes extracted patterns such as the win rate of a specific strategy across different market regimes, or operator feedback indicating that specific setups are invalid under certain order book conditions. Semantic memory transforms raw data into actionable intelligence.

### **Centralized vs. Distributed Storage Mechanics**

Given the Rust-native architecture of the Savant engine and the requirement for parallel evaluation, the storage mechanism must support high concurrency while maintaining strict data integrity. While distributed vector databases are increasingly popular in generalized artificial intelligence applications for semantic search, a centralized relational database utilizing SQLite is optimal for this domain. Financial memory is inherently relational. Extracting statistical patterns—such as determining win rates by regime or calculating the expected value of a specific conviction level—requires relational queries, numerical aggregations, and complex joins that vector databases cannot natively or efficiently perform.  
Furthermore, deploying SQLite embedded directly within the Rust application ensures that memory access latency remains well below the strict 500-millisecond constraint by eliminating network hops entirely.6 By leveraging SQLite, the system can utilize advanced SQL features and strictly typed columns for numerical analysis while reserving JSON-formatted columns for the storage of dynamic, unstructured text generated by the large language model.

### **Granularity and Relevance Decay**

A critical design consideration is the optimal granularity for memory storage. Capturing market data at the tick level introduces unmanageable storage bloat and degrades database query performance without providing commensurate analytical value. Therefore, memory must be captured at the resolution of the decision.2 A decision-level snapshot isolates the exact variables, indicator states, and context that the large language model used to form its conviction, providing a perfect experimental unit for later analysis.  
As market dynamics shift, historical patterns can become obsolete. However, memories should not explicitly "decay" through deletion, as historical data spanning full market cycles—including distinct bull, bear, ranging, and crisis regimes—is invaluable for long-term backtesting and pattern validation.7 Instead, relevance decay should be implemented dynamically through algorithmic weighting during the retrieval phase. This approach prioritizes recent manifestations of a pattern to capture current market microstructure while retaining the full historical sample size to maintain statistical power.

## **Episodic Memory Construction**

Episodic memory forms the raw foundational dataset from which all subsequent intelligence and semantic rules are derived. To support advanced pattern extraction and reinforcement learning, the system must capture the precise market vector alongside the cognitive state of the artificial intelligence at the exact moment a decision is rendered.

### **The Minimum Viable Snapshot**

Storing the entire 13,000-character prompt for every evaluation is highly inefficient. It complicates downstream statistical queries and accelerates database bloat, ultimately degrading the performance of the engine over months of continuous operation. Instead, the Minimum Viable Snapshot must decompose the context into structured, queryable data columns, combined with minimized JSON payloads for flexible, semi-structured indicator data.  
A comprehensive Minimum Viable Snapshot must include:

* **Execution Data**: The chosen action (Long, Short, or None), the target pair, intended entry prices, invalidation stops, profit targets, and the mathematical ratio of planned risk to reward compared against the actually achieved outcome.  
* **Market Context Vectors**: The identified market regime, the active trading session, sentiment metrics like the Fear and Greed Index, derivative funding rates, order book imbalance metrics, and the state of the volume profile.  
* **Cognitive State**: A structured summary of the agent's core thesis, its specific invalidation reasoning, the declared Conviction Level (HIGH, MEDIUM, LOW, NONE), and an array of the specific MarketCondition tags and curated Knowledge Units invoked during the decision cycle.

Crucially, the system must rigorously log decisions where the agent evaluated a setup and explicitly chose not to trade.2 Decisions resulting in a "None" action establish the baseline for evaluating the agent's filtering mechanism. They are essential for calculating the opportunity cost, tracking missed alpha, and enabling a framework for "regret" analysis, where the system evaluates whether its risk constraints are overly punitive.

### **SQLite Schema Design for Episodic Traces**

The database schema must elegantly balance normalization to support fast, concurrent inserts during the 30-second parallel evaluation cycle, with the flexibility required for complex analytical reads during pattern extraction. The following tables outline the proposed structure for the episodic memory system.1

| Table Name | Description | Key Columns |
| :---- | :---- | :---- |
| agent\_episodes | The core ledger of all evaluation cycles. | episode\_id (Primary Key), timestamp, pair, session, regime, action, conviction\_level, planned\_rr, achieved\_rr, pnl\_pct, is\_win, status |
| episode\_market\_context | Stores the exact mathematical state of the market at the time of the episode. | episode\_id (Foreign Key), funding\_rate, fear\_greed\_index, order\_book\_imbalance, mvrv\_ratio, sopr\_ratio, volatility\_atr, condition\_tags (JSON array) |
| episode\_cognitive\_state | Stores the textual and heuristic outputs of the language model. | episode\_id (Foreign Key), knowledge\_units\_used (JSON array), thesis\_summary, invalidation\_reasoning, confidence\_score |

This normalized relational schema ensures that data types are strictly enforced for numerical metrics, allowing the Rust application to rapidly compute aggregations, averages, and statistical deviations directly via SQL queries rather than pulling massive datasets into application memory.1 The utilization of JSON columns for condition\_tags and knowledge\_units\_used provides the necessary flexibility to accommodate future expansions of the 265 Knowledge Units without requiring continuous schema migrations.

## **Semantic Memory and Pattern Extraction**

Semantic memory represents the distillation of raw experience into wisdom. It is generated through an asynchronous, computationally intensive process known as memory consolidation.9 The agent must periodically review its episodic ledger to detect structural advantages, quantify its true mathematical edge, and identify systemic cognitive failures.

### **Memory Consolidation Stages**

The consolidation process transforms disjointed episodic traces into generalized, executable rules.9 For the Savant architecture, this process should run as an isolated background task, ideally scheduled during the lower-volatility weekend session to prevent interference with live trading operations.

1. **Episodic-to-Semantic Extraction**: The system aggregates data from the agent\_episodes and episode\_market\_context tables, clustering outcomes by variables such as regime, session, and condition\_tags. It computes essential trading metrics—including win rate, profit factor, maximum drawdown, and average achieved risk-to-reward ratio—for each isolated cluster.  
2. **Conflict Resolution**: When a newly extracted pattern conflicts with a previously established rule (for instance, a momentum strategy that generated consistent profit in early 2024 begins to fail consistently in late 2026), the system applies time-decay weights to favor recent market microstructure dynamics while archiving the historical baseline.9  
3. **Incremental Profile Update**: Validated, statistically significant patterns are compiled into highly compressed JSON blocks and injected directly into the agent's Core Memory, seamlessly integrating with the existing SOUL.md persona definition.

To ensure the agent adapts comprehensively, the memory system must track a wide array of specific pattern categories. The most critical variables include the win rate isolated by market regime (bull, bear, range, crisis), the efficacy of specific trading sessions (Asian, European, US), the actual performance of the agent's stated conviction levels, and the correlation of success against funding rate extremes and order book imbalances.

### **Statistical Validation of Small Samples**

A paramount challenge in programmatic pattern extraction is distinguishing true mathematical edge from random variance. This is particularly difficult given the small sample sizes inherent to medium-term swing trading strategies, where an agent might only execute 50 to 200 trades per month. A strategy displaying a 70% win rate on a sample size of only 15 trades carries a massive margin of error and cannot be relied upon for structural decision-making.7  
Because financial returns are notoriously non-normal—often characterized by fat tails, extreme skewness, and significant outliers resulting from liquidation cascades—traditional parametric statistical tests, such as Student's t-test or standard ANOVA, are frequently inadequate and can yield false positives regarding strategy efficacy.10 Consequently, the agent must employ non-parametric statistical methods to rigorously validate its semantic findings.12  
The memory architecture will implement the following statistical algorithms for validation:

* **Mann-Whitney U Test (Wilcoxon Rank-Sum Test)**: This test evaluates whether the median return of a strategy operating in one specific condition (e.g., a Trending regime) is statistically different from its return in a different condition (e.g., a Ranging regime).11 By relying on the rank of the observations rather than the raw numerical returns, the Wilcoxon test is highly robust against the extreme outliers common in cryptocurrency markets, ensuring that a single massive liquidation event does not falsely skew the validation of a strategy's edge.  
* **Sign Test**: This method will be used to evaluate whether the median profitability of a specific setup or cluster of market tags is statistically greater than zero.12 It requires minimal assumptions about the underlying distribution of the trading returns, making it ideal for the highly volatile nature of the target assets.

To rigorously prevent overfitting to historical noise, an extracted pattern must not be permitted to influence the agent's Core Memory until the sample size for that specific condition cluster exceeds a minimum threshold of 30 isolated episodes. This constraint honors the fundamental statistical concept that data distributions begin to approximate normality only as degrees of freedom increase sufficiently.10

### **Detecting Edge Decay via Cumulative Sum Algorithms**

Trading edges decay over time as broader market participants identify and arbitrage away structural inefficiencies.17 To automatically detect when an established semantic pattern is degrading, the agent will implement a Cumulative Sum (CUSUM) control chart algorithm overlaid on its sequential trade histories.18  
The CUSUM algorithm is uniquely suited for this architecture because it detects persistent, small shifts in performance significantly earlier than simple moving averages, which inherently lag the market.18 It operates by tracking the cumulative deviation of the agent's achieved risk-to-reward metrics against its historical target baseline.  
The mathematical calculation for updating the upper and lower cumulative sums (![][image1] and ![][image2]) at each discrete trade ![][image3] is defined as follows:  
![][image4]  
![][image5]  
In this formulation, ![][image6] represents the actual profitability or risk-to-reward ratio of the ![][image3]\-th trade, ![][image7] represents the historical baseline expected value of the specific strategy being employed, and ![][image8] represents the allowance value—the magnitude of acceptable variance that the system should actively ignore to prevent false alarms.  
When either the positive sum (![][image1]) or the negative sum (![][image2]) exceeds a predefined decision interval threshold (![][image9]), the mathematical system automatically flags the strategy as experiencing a structural regime shift or edge decay.18 This mathematical trigger immediately prompts the memory architecture to downgrade the agent's Conviction Level for that specific setup in all future evaluations, protecting the portfolio from extended drawdowns while the agent searches for a new edge.

## **Confidence Calibration and Risk Management**

Large language models operating as reasoning engines frequently exhibit severe overconfidence in their predictions and analyses. For an autonomous trading agent, an uncalibrated 90% confidence score leading to a maximum permissible position size can cause catastrophic portfolio drawdown if the true empirical probability of success is significantly lower. Therefore, the memory system must implement a mathematical framework to rigidly align the agent's expressed textual confidence with empirical, historical probabilities.21

### **Brier Score Decomposition**

To rigorously calibrate the language model, the system will continuously compute the Brier Score of its historical predictions. The Brier Score is a strictly proper scoring rule that measures the mean squared difference between predicted probabilities (the agent's stated confidence) and the actual binary outcomes (a winning or losing trade).22  
The standard formula for the Brier Score is:  
![][image10]  
Where ![][image11] is the forecasted probability derived from the agent's stated Conviction Level (for example, mapping HIGH to 0.75, MEDIUM to 0.50, and LOW to 0.25), and ![][image12] is the binary outcome of the trade (1 for a profitable win, 0 for a loss). A perfectly calibrated agent achieves a Brier Score approaching 0, while a totally uncalibrated agent approaches a score of 1\.22  
Crucially, the Brier Score can be mathematically decomposed into three distinct components: Reliability (Calibration), Resolution, and Uncertainty.23 An agent might possess excellent Resolution—meaning it successfully differentiates between high-quality setups and low-quality setups—but terrible Reliability, meaning it consistently states it is 90% confident when it only wins 60% of the time. If the memory engine detects that the Reliability penalty is high, it will automatically apply a penalization scalar to the agent's future Conviction outputs. This dynamically forces the agent to reduce its position sizing until its internal calibration aligns with reality.21

### **Holistic Trajectory Calibration**

Traditional methods for calibrating language models evaluate single-turn, static outputs. Trading, however, is an inherently multi-step trajectory involving complex hypothesis generation, entry timing, ongoing risk management, and exit execution.25 Applying Holistic Trajectory Calibration (HTC) allows the memory system to assess confidence across the entire lifecycle of a trading decision rather than just the final output.27  
HTC operates by extracting process-level features—such as early-step reasoning entropy, rapid confidence gradients, and cognitive stability—to identify when the agent is entering a state of compounding uncertainty.28 For instance, if the agent's textual thesis changes rapidly over three consecutive 5-minute candles without any corresponding price invalidation or major news events, the HTC framework detects severe behavioral instability. The episodic memory system logs this invisible instability. If the resulting trade fails, the semantic memory engine will learn to recognize similar "confused" or highly entropic states in the future, automatically defaulting the agent's action to "NONE" regardless of what the final indicator setup looks like.29

## **Context Injection and Memory Retrieval**

Retrieving and formatting historical data for the language model requires strict and precise token management. Injecting raw JSON logs from the SQLite database directly into the 1-million token context window, while technically feasible, increases processing latency and severely dilutes the model's attention mechanism, leading to hallucination and decision paralysis.

### **The Sixth Prompt Layer: Dynamic Memory Context**

The existing 5-layer prompt structure of the Savant engine will be expanded to include a 6th layer: **Dynamic Memory Context**. This specific layer bridges the episodic and semantic databases, translating mathematical realities into a highly compressed, narrative format that is easily parsed and weighted by the language model.  
When a new market data payload is generated for a cryptocurrency pair, the Rust engine queries the SQLite memory system using the current MarketCondition tags, the active Session, and the identified market regime. The retrieval mechanism must determine relevance not just by matching the exact asset pair, but by matching the underlying market microstructure.  
The database query returns three distinct components:

1. **Semantic Rules**: High-level statistical realities regarding the current setup.  
2. **Recent Episodic Analogs**: The immediate outcomes of the three most recent trades taken under highly similar mathematical conditions.  
3. **Operator Constraints**: Any manual overrides or specific feedback injected by the human operator via the Obsidian vault.

### **Prompt Formatting for Cognitive Consumption**

The injected text must be deterministic, terse, and aggressively formatted to prevent the language model from suffering from decision paralysis. By presenting contradictory memories explicitly, the agent is forced to engage in secondary reasoning, weighing historical shifts against current price action.  
An example of the injected text structure:

## **Dynamic Memory Context**

Your historical win rate in CURRENT REGIME (Ranging) \+ SESSION (US) across 45 trades is 32%. Your edge here is negative.  
Your recent confidence scaling is poorly calibrated (Brier Score: 0.45). System penalty applied: Maximum permitted risk reduced to MEDIUM (1.0%).  
CUSUM algorithm alert: Breakout strategies in BTC/USD have shown negative drift over the last 15 attempts.  
"Wyckoff Springs are invalid and should be ignored if the funding rate exceeds 0.01%."

* 2 days ago: Shorted at Value Area Low. Result: LOSS (-1R). Reason: Stopped out by liquidity sweep.  
* 5 days ago: Longed at Point of Control. Result: WIN (+2.5R). Reason: Target hit at Value Area High.

By providing the overall performance metrics, recent losing streaks, and direct operator feedback in a structured narrative, the agent can calibrate its immediate response. It is made aware of its own systemic flaws, forcing it to apply extreme caution in regimes where it historically underperforms.

## **Advanced Learning Mechanisms and Adaptation**

The memory system must go beyond passive observation; it must actively reinforce profitable behaviors and systematically extinguish unprofitable ones. Drawing from advanced reinforcement learning architectures specifically designed for financial trading agents, such as FLAG-Trader and AgentRR, the Savant system will implement a structured Experience Replay mechanism.30

### **Prioritized Experience Replay**

During scheduled operational downtime—such as the historically low-volatility weekend sessions—the engine will trigger an automated Experience Replay loop.30 The system queries the agent\_episodes database for trades that represent high-magnitude cognitive errors. These are explicitly defined as instances where the agent expressed HIGH conviction but suffered a maximum allowable \-1R loss, or conversely, instances where the agent selected a NONE action but the specific setup subsequently resulted in a massive \+3R theoretical gain (enabling strict Regret tracking).30  
The agent is re-prompted in a specialized isolated environment with the historical market data leading up to the error, followed immediately by the factual outcome. This process forces the language model to compress long, failed episodes into concise, actionable lessons.30  
The prompt structure for the Experience Replay cycle operates as follows:  
*"Review this past decision. You exhibited HIGH conviction in this setup, but the trade failed and hit the invalidation stop. Analyze your analytical blind spots based on the provided factual outcome. Formulate a single-sentence heuristic rule to avoid this specific trap in the future."*  
These newly generated heuristics are stored in the episode\_cognitive\_state table. If similar market conditions arise in future live evaluations, these heuristics are automatically injected into the Core Memory layer, allowing the agent to self-correct its logic dynamically.31

### **Operator Feedback Loops via the Obsidian Vault**

The existing Lessons/ directory within the Obsidian vault provides the critical human-in-the-loop interface. When the human operator writes a markdown file detailing a manual correction or observation, the memory engine parses this text as absolute ground truth.  
Operator constraints are designed to supersede all AI-derived semantic rules and historical statistics. If an operator explicitly notes in a markdown file, "Stop trading altcoins entirely during periods of extreme BTC dominance," the Rust engine indexes this text against the BtcDominant tag. It is then forcefully prepended to the top of the Layer 1 (SOUL.md) prompt. This architecture creates a closed-loop system where human intuition can permanently and immediately override and calibrate AI behavior without requiring code changes or complex database manipulations.

## **Memory Persistence Architecture: Concurrency and Latency**

To maintain the strict requirement of a 7.5-minute decision cycle across 15 simultaneously evaluated cryptocurrency pairs, the underlying Rust architecture must handle rapid, highly concurrent database access without blocking the asynchronous event loop.

### **Write-Ahead Logging and Database Concurrency**

SQLite's default operational mode utilizes a rollback journal. This standard configuration heavily restricts concurrency by locking the entire database file during any write operation. If 15 parallel evaluation threads attempt to log their episodic traces simultaneously, the system will trigger cascading SQLITE\_BUSY errors, crashing the engine or introducing massive latency spikes.  
By aggressively enabling Write-Ahead Logging (WAL) mode at the database level, readers are no longer blocked by writers, and writers do not block readers.33 This fundamental shift in the database engine is a non-negotiable architectural requirement for an asynchronous tokio environment where multiple tasks will simultaneously attempt to query semantic memory layers while writing new episodic logs.34  
The sqlx database connection pool within the Rust application must be configured explicitly for this highly concurrent model:

Rust  
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};  
use std::time::Duration;

let options \= SqliteConnectOptions::new()  
   .filename("savant\_persistent\_memory.db")  
   .journal\_mode(SqliteJournalMode::Wal) // Enables concurrent read/write mechanics  
   .busy\_timeout(Duration::from\_secs(5)) // Prevents immediate SQLITE\_BUSY thread panics  
   .shared\_cache(true);

let pool \= SqlitePoolOptions::new()  
   .max\_connections(16) // Configured for 1 dedicated writer \+ 15 parallel readers  
   .connect\_with(options)  
   .await?;

To entirely eliminate database locking contention in this highly parallelized environment, the architecture should implement a strict SqliteRwPool routing pattern.36 This dictates the instantiation of a single, dedicated database connection exclusively reserved for all INSERT and UPDATE operations (the writer). Simultaneously, it maintains a broader pool of read-only connections dedicated entirely to processing the complex pattern extraction queries utilized by the 15 parallel pairs.35 This guarantees that the AI evaluation tasks can seamlessly query memory context simultaneously without ever blocking the engine's real-time execution logger.

### **Managing Storage Scalability and Archiving**

Over months of continuous, high-frequency evaluation, the episodic database will inevitably grow in size. Because raw JSON market context blobs and detailed text reasoning are stored directly in the rows, the architecture must actively account for database bloat to preserve the sub-500ms query latency requirement.1  
The system will utilize two methods to maintain performance:

1. **Aggressive Data Normalization**: All standard indicators, price points, and metrics are stored in strictly typed real or integer columns. Only highly dynamic string arrays are relegated to JSON storage, minimizing the processing overhead for the SQLite engine.  
2. **Automated Archiving Pipelines**: A chron job executed at the end of every month will isolate and compress episodic records older than 90 days. While their statistical footprints and mathematical impacts remain permanently embedded in the Semantic Memory tables, the raw textual reasoning and massive JSON blobs are exported from SQLite into compressed markdown files housed within the Obsidian Vault/Archive/ directory. This preserves optimal disk I/O performance for active queries while maintaining a complete, human-readable audit trail.

## **Cold Start and Bootstrap Strategy**

At the moment of initial deployment, the episodic database is completely empty. An artificial intelligence agent attempting to query memory to establish conviction with a sample size of zero will default entirely to the base heuristics of the underlying language model, risking severe initial drawdowns due to hallucination or poor calibration. The memory system must gracefully navigate this cold start problem.

### **Bayesian Priors from Curated Knowledge**

The agent's existing library of 265 Knowledge Units acts as the system's "prior probability" distribution.5 If the system lacks any episodic history for a specific condition, such as the HighVolatility tag, it defaults entirely to the assumed win rate, risk parameters, and structural rules defined manually within that corresponding Knowledge Unit.

### **Progressive Confidence Restraints**

Until the episodic database accumulates enough data to reach a minimum threshold of statistical viability—defined as 50 total trades, or at least 15 trades isolated to a specific market regime—the agent operates under an imposed global confidence penalty.7

* **Trades 1–25**: The agent is hard-capped at LOW conviction (0.5% portfolio risk), regardless of how strong its internal textual thesis appears.  
* **Trades 26–50**: The agent is permitted to scale up to MEDIUM conviction (1.0% portfolio risk) only if the preliminary empirical win rate during the first phase exceeded a 50% baseline.  
* **Trades 50+**: The statistical validation algorithms, including the CUSUM chart and Wilcoxon Rank-Sum tests, fully activate. Memory-informed trading completely overrides the base Bayesian priors. The agent is finally permitted to execute HIGH conviction trades (1.5% portfolio risk), strictly based on proven empirical edge and mathematical calibration.40

## **Visualization, Transparency, and Debugging**

A complex artificial intelligence memory system is only viable if it is completely transparent and auditable.29 The human operator must be able to verify exactly what the agent has learned, how it has mathematically calibrated its confidence, and why it is choosing to execute or ignore specific behaviors.

### **Ratatui TUI Dashboard Integration**

The existing terminal user interface, built on Ratatui, will be expanded to display real-time cognitive metrics. A dedicated \`\` panel will render alongside the active pricing charts.

| Operational Metric | Display Format Example | Definition |
| :---- | :---- | :---- |
| **Edge Status** | Ranging: \+1.2R (ACTIVE) Trending: \-0.4R (DECAY) | Live CUSUM output indicating whether the strategies currently deployed possess a mathematically valid edge. |
| **Confidence Calibration** | Brier Score: 0.12 (Excellent) | Real-time confidence calibration score, indicating how trustworthy the agent's sizing recommendations are. |
| **Context Focus** | Loaded: 3 Memories, 2 Rules | Indicates the specific volume of historical context actively injected into the 6th prompt layer for the current evaluation. |

### **Obsidian Vault Representation**

The Obsidian Vault serves as the permanent, human-readable archive of the agent's semantic memory.  
A new directory, designated as Vault/Memory/, will be automatically populated and continuously updated by the Rust engine.

* Vault/Memory/Regimes.md: A dynamically updated markdown table displaying the historical win rate, aggregate profit factor, and average achieved risk-to-reward ratio across all tracked market regimes.  
* Vault/Memory/Edge\_Decay.md: Detailed logs generated automatically when the CUSUM chart algorithm triggers a regime shift or edge decay alert.  
* Vault/Memory/Replay\_Lessons.md: An ongoing ledger of the single-sentence heuristics generated by the agent during its automated weekend Experience Replay sessions.

This directory structure allows the operator to natively read the agent's internalized rules formatted beautifully in Markdown. The operator can seamlessly edit them, audit their accuracy, or cross-reference them with the Lessons/ directory to issue direct, programmatic overrides.

## **Implementation Roadmap**

To systematically and safely transition the Savant architecture from a stateless execution engine into an adaptive, self-calibrating cognitive system, development should follow a phased, compartmentalized approach.

### **Phase 1: Episodic Capture and Infrastructure**

The initial phase focuses entirely on establishing the core data plumbing without altering the agent's decision-making logic.

1. Implement the SQLite Write-Ahead Logging connection pool and establish the sqlx repository patterns using the SqliteRwPool architecture.  
2. Deploy the normalized episodic schema, specifically the agent\_episodes, episode\_market\_context, and episode\_cognitive\_state tables.  
3. Modify the main evaluation loop to construct the Minimum Viable Snapshot and asynchronously insert it into the database immediately following a decision.

### **Phase 2: Context Retrieval and Prompt Injection**

The second phase bridges the database to the language model.

1. Develop the high-speed, read-only memory query functions triggered by the detection of MarketCondition tags.  
2. Construct the formatting logic for the 6th Prompt Layer (Dynamic Memory Context) to ensure strict token limits are honored.  
3. Integrate the Obsidian Lessons/ ingestion pipeline to guarantee that human operator rules are successfully prepended to the AI prompt.

### **Phase 3: Semantic Extraction and Mathematical Calibration**

The third phase introduces the intelligence and analytical mechanics.

1. Implement the background Memory Consolidation algorithms to aggregate data across regimes and sessions.  
2. Program the non-parametric statistical validators, specifically the Wilcoxon Rank-Sum Test and Sign Test, in Rust to safely analyze win rates across small sample sizes.  
3. Implement the Brier Score calculation to monitor calibration and dynamically adjust the agent's Conviction Level outputs.  
4. Activate the progressive confidence constraints to safely manage the cold start period and limit early drawdown.

### **Phase 4: Advanced Adaptation and Visualization**

The final phase enables continuous self-improvement and full transparency.

1. Build and tune the Cumulative Sum (CUSUM) control chart algorithm to automatically monitor strategy edge decay in real time.  
2. Implement Prioritized Experience Replay for automated weekend retrospective analysis and heuristic generation.  
3. Deploy the updated Ratatui terminal dashboard and configure the automated generation of the Vault/Memory/ markdown files.

By rigorously executing this specific architectural design, the Savant engine will successfully transcend reactive, single-state decision making. It will actively construct a rich episodic history, distill that history into statistically valid semantic rules, mathematically calibrate its own confidence, and dynamically adjust its risk parameters based on the presence or decay of its empirically proven trading edge.

#### **Works cited**

1. Memory Architectures for Multi-Turn Text-to-SQL: A Benchmark and Empirical Study \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2605.26394v1](https://arxiv.org/html/2605.26394v1)  
2. Memory Architecture Patterns | The AI Agent Factory \- Panaversity, accessed May 30, 2026, [https://agentfactory.panaversity.org/docs/Building-Agent-Factories/augmented-memory/memory-architecture-patterns](https://agentfactory.panaversity.org/docs/Building-Agent-Factories/augmented-memory/memory-architecture-patterns)  
3. Agent Memory: How to Build Agents that Learn and Remember \- Letta, accessed May 30, 2026, [https://www.letta.com/blog/agent-memory](https://www.letta.com/blog/agent-memory)  
4. MemGPT: Engineering Semantic Memory through Adaptive Retention and Context Summarization \- Information Matters, accessed May 30, 2026, [https://informationmatters.org/2025/10/memgpt-engineering-semantic-memory-through-adaptive-retention-and-context-summarization/](https://informationmatters.org/2025/10/memgpt-engineering-semantic-memory-through-adaptive-retention-and-context-summarization/)  
5. How to Build AI Agents with Redis Memory Management, accessed May 30, 2026, [https://redis.io/blog/build-smarter-ai-agents-manage-short-term-and-long-term-memory-with-redis/](https://redis.io/blog/build-smarter-ai-agents-manage-short-term-and-long-term-memory-with-redis/)  
6. In-Database Machine Learning: A Study on SQL-based Predictions \- IEEE Xplore, accessed May 30, 2026, [https://ieeexplore.ieee.org/document/11511003/](https://ieeexplore.ieee.org/document/11511003/)  
7. Minimum Trades for a Valid Backtest? Calculator \+ Research \- BacktestBase, accessed May 30, 2026, [https://www.backtestbase.com/education/how-many-trades-for-backtest](https://www.backtestbase.com/education/how-many-trades-for-backtest)  
8. Building a Fake Economy Run by AI Agents to Negotiate Trade Promotions: Lessons from a CPG Simulation | by Dinand Tinholt | Medium, accessed May 30, 2026, [https://medium.com/@tinholt/building-a-fake-economy-run-by-ai-agents-to-negotiate-trade-promotions-lessons-from-a-cpg-785384cdfb91](https://medium.com/@tinholt/building-a-fake-economy-run-by-ai-agents-to-negotiate-trade-promotions-lessons-from-a-cpg-785384cdfb91)  
9. Episodic-Semantic Memory Architecture for Long-Horizon Scientific Agents \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2605.17625v1](https://arxiv.org/html/2605.17625v1)  
10. What Is T Statistic | Formula, Meaning, and Applications in Trading \- Quantra by QuantInsti, accessed May 30, 2026, [https://quantra.quantinsti.com/glossary/TStatistics](https://quantra.quantinsti.com/glossary/TStatistics)  
11. Statistical Considerations for Preclinical Studies \- PMC \- NIH, accessed May 30, 2026, [https://pmc.ncbi.nlm.nih.gov/articles/PMC4466166/](https://pmc.ncbi.nlm.nih.gov/articles/PMC4466166/)  
12. Testing Statistical Significance in Financial Data with Python \- SEC API, accessed May 30, 2026, [https://sec-api.io/resources/testing-statistical-significance-in-financial-data-with-python](https://sec-api.io/resources/testing-statistical-significance-in-financial-data-with-python)  
13. The day of the week effect in the cryptocurrency market \- Brunel University Research Archive, accessed May 30, 2026, [https://bura.brunel.ac.uk/bitstream/2438/17208/3/FullText.pdf](https://bura.brunel.ac.uk/bitstream/2438/17208/3/FullText.pdf)  
14. Trading strategy: Making the most of the out of sample data \- R-bloggers, accessed May 30, 2026, [https://www.r-bloggers.com/2016/08/trading-strategy-making-the-most-of-the-out-of-sample-data/](https://www.r-bloggers.com/2016/08/trading-strategy-making-the-most-of-the-out-of-sample-data/)  
15. How to Test Significance of Trading Strategy? \- Cross Validated \- Stats StackExchange, accessed May 30, 2026, [https://stats.stackexchange.com/questions/501361/how-to-test-significance-of-trading-strategy](https://stats.stackexchange.com/questions/501361/how-to-test-significance-of-trading-strategy)  
16. 2025 Trader's Guide to T-Distribution \- The Trading Analyst, accessed May 30, 2026, [https://thetradinganalyst.com/what-is-a-t-distribution/](https://thetradinganalyst.com/what-is-a-t-distribution/)  
17. How to Find and Build Your Trading Edge \- JournalPlus, accessed May 30, 2026, [https://journalplus.co/blog/how-to-build-a-trading-edge](https://journalplus.co/blog/how-to-build-a-trading-edge)  
18. What is CUSUM? Meaning, Architecture, Examples, Use Cases, and How to Measure It (2026 Guide) \- DataOpsSchool, accessed May 30, 2026, [https://dataopsschool.com/blog/cusum/](https://dataopsschool.com/blog/cusum/)  
19. CUSUM Anomaly Detection, accessed May 30, 2026, [https://www.measurementlab.net/publications/CUSUMAnomalyDetection.pdf](https://www.measurementlab.net/publications/CUSUMAnomalyDetection.pdf)  
20. An Adjusted CUSUM-Based Method for Change-Point Detection in Two-Phase Inverse Gaussian Degradation Processes \- MDPI, accessed May 30, 2026, [https://www.mdpi.com/2227-7390/13/19/3167](https://www.mdpi.com/2227-7390/13/19/3167)  
21. Confidence Calibration in LLMs \- Emergent Mind, accessed May 30, 2026, [https://www.emergentmind.com/topics/confidence-calibration-in-llms](https://www.emergentmind.com/topics/confidence-calibration-in-llms)  
22. A Brief on Brier Scores | UVA Library, accessed May 30, 2026, [https://library.virginia.edu/data/articles/a-brief-on-brier-scores](https://library.virginia.edu/data/articles/a-brief-on-brier-scores)  
23. Brier score \- Wikipedia, accessed May 30, 2026, [https://en.wikipedia.org/wiki/Brier\_score](https://en.wikipedia.org/wiki/Brier_score)  
24. Some Notes on Probabilistic Classifiers III: Brier Score Decomposition | by Eli Goz | Medium, accessed May 30, 2026, [https://medium.com/@eligoz/some-notes-on-probabilistic-classifiers-iii-brier-score-decomposition-eee5f847d87f](https://medium.com/@eligoz/some-notes-on-probabilistic-classifiers-iii-brier-score-decomposition-eee5f847d87f)  
25. Stop AI from guessing: Appier enables agents to assess confidence before acting, accessed May 30, 2026, [https://www.appier.com/en/press-media/stop-ai-from-guessing-appier-enables-agents-to-assess-confidence-before-acting](https://www.appier.com/en/press-media/stop-ai-from-guessing-appier-enables-agents-to-assess-confidence-before-acting)  
26. \[2601.15778\] Agentic Confidence Calibration \- arXiv, accessed May 30, 2026, [https://arxiv.org/abs/2601.15778](https://arxiv.org/abs/2601.15778)  
27. Holistic Trajectory Calibration Methods \- Emergent Mind, accessed May 30, 2026, [https://www.emergentmind.com/topics/holistic-trajectory-calibration-htc](https://www.emergentmind.com/topics/holistic-trajectory-calibration-htc)  
28. Holistic Trajectory Calibration Advances Confidence \- Quantum Zeitgeist, accessed May 30, 2026, [https://quantumzeitgeist.com/systems-holistic-trajectory-calibration-advances-confidence/](https://quantumzeitgeist.com/systems-holistic-trajectory-calibration-advances-confidence/)  
29. Agentic Confidence Calibration \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2601.15778v1](https://arxiv.org/html/2601.15778v1)  
30. Agentic Trading: When LLM Agents Meet Financial Markets \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2605.19337v1](https://arxiv.org/html/2605.19337v1)  
31. Get Experience from Practice: LLM Agents with Record & Replay \- arXiv, accessed May 30, 2026, [https://arxiv.org/html/2505.17716v1](https://arxiv.org/html/2505.17716v1)  
32. ICML Poster Think Twice, Act Once: A Co-Evolution Framework of LLM and RL for Large-Scale Decision Making, accessed May 30, 2026, [https://icml.cc/virtual/2025/poster/43534](https://icml.cc/virtual/2025/poster/43534)  
33. Write-Ahead Logging \- SQLite, accessed May 30, 2026, [https://sqlite.org/wal.html](https://sqlite.org/wal.html)  
34. SQLite "database is locked" with multiple apps \+ Tokio \+ rusqlite (WAL \+ busy\_timeout not enough?) : r/rust \- Reddit, accessed May 30, 2026, [https://www.reddit.com/r/rust/comments/1sexpau/sqlite\_database\_is\_locked\_with\_multiple\_apps/](https://www.reddit.com/r/rust/comments/1sexpau/sqlite_database_is_locked_with_multiple_apps/)  
35. Concurrency when writing data into SQLite? : r/golang \- Reddit, accessed May 30, 2026, [https://www.reddit.com/r/golang/comments/16xswxd/concurrency\_when\_writing\_data\_into\_sqlite/](https://www.reddit.com/r/golang/comments/16xswxd/concurrency_when_writing_data_into_sqlite/)  
36. feat(sqlite): Add \`SqliteRwPool\` with a single writer and multiple readers by emschwartz · Pull Request \#4177 · launchbadge/sqlx \- GitHub, accessed May 30, 2026, [https://github.com/launchbadge/sqlx/pull/4177](https://github.com/launchbadge/sqlx/pull/4177)  
37. Training a Team of Language Models as Options to Build an SQL-Based Memory \- MDPI, accessed May 30, 2026, [https://www.mdpi.com/2076-3417/15/21/11399](https://www.mdpi.com/2076-3417/15/21/11399)  
38. Episodic-Semantic Memory Architecture for Long-Horizon Scientific Agents \- arXiv, accessed May 30, 2026, [https://arxiv.org/pdf/2605.17625](https://arxiv.org/pdf/2605.17625)  
39. How are you actually validating your edge over time? : r/Daytrading \- Reddit, accessed May 30, 2026, [https://www.reddit.com/r/Daytrading/comments/1r6xoe2/how\_are\_you\_actually\_validating\_your\_edge\_over/](https://www.reddit.com/r/Daytrading/comments/1r6xoe2/how_are_you_actually_validating_your_edge_over/)  
40. How to Find Your Trading Edge: 5-Step Data Process \- TradeZella, accessed May 30, 2026, [https://www.tradezella.com/blog/trading-edge](https://www.tradezella.com/blog/trading-edge)

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABoAAAAaCAYAAACpSkzOAAABUUlEQVR4Xu2UvytHURTAj1D0FUUGxSIGZaPEdCeZLBT1tRmUwUQiq6QoMSibzSKDrJKRyWBRivIPGIx+fE73XZ7Tra/7Fsv71KfeO+f2zrvnnvdESuL0Y5cN/pUWdDiObVmsAzvDghwOh2ywFo24gY+4hIt4izt4jQM/S79xklioAQ/xBCu5uO7kBq/E79TiJLHQKD7joE3AOu5n12N4lPMCz3L3u1LjzDbxBbttAlZw0gYznCTu6Bg/cQ3rTa4P200s4CSx0Kz4Quo7XuI8tuYXRXCSWEgnbkt8kVBQvZN4OwNOEgsFtG06xtv4Kr7Ywq8VBdEH6xnU2QRM4Aeu2kQRevFA/Hdk0Za8YdUmiqBje45NNgFz+IA9NlEE/X70rUdMXNupgzBl4oXQX8opLuN9dq0jvYdPOCPxs0umWfybKzrewzgtfmRjrSwpKfknvgDpLjQhXQInlQAAAABJRU5ErkJggg==>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABoAAAAaCAYAAACpSkzOAAABM0lEQVR4Xu3UvytFcRjH8UcoIops1yKLsrklm0kmC+UWm0EZTG43N2WSlEEMymazmOySkekOFqXc8g8YjH68P33PzfV07uB76C7nU686Pc+3+z3ne55zzfL8Q/oxizkMJrVhjDQWZE03dvCETWzgHoe4xcT30vh04RQX6Guq60nucGPhSTNnBnVM+gap4ji51ma7OGtBJ6Gbbpk9vKDgG6SMBV+MzTk+sY1O1xvHkKtFp2RhI3nHNdYw0LzoL6KJ27ewSWNDqVn6cWaOjk1jfIBXC5ut/1gRGf2w3kGHb5B5fKDiGzEZw4mlj+QU3rDiGzHR2F6hxzfIKh4x6hsx0feju552dR2nBmHR1aOir/wSW3hIrjXSR3jGsqW/u1+n18KdKxrvIpYs/HOnHWWePHnalC8VTTARTe7i7AAAAABJRU5ErkJggg==>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAcAAAAcCAYAAACtQ6WLAAAAkElEQVR4XmNgGOQgCYh3A7EwugQHEG+FYhAbBeCVlAHiJ0DciizIA8SSQBwKxL+BOAKIxYGYFSQZD8SzgPg+EP8E4qVAPAmIlUGSIEC6fTDgAsS/oDQGqALi50CshC4Bs28PEHMzQFzZxQCxikEEiK8yIOwLAuICIGYEcUBEIxDfAeKVUDbYj8hAAIpHATIAAP3zGM9f3v8PAAAAAElFTkSuQmCC>

[image4]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAmwAAAAxCAYAAABnGvUlAAAFcUlEQVR4Xu3dX6hsZRkH4E9SMFJMEyJSUgohClOk6EjJvrDCC0OCLiShIkjpzr9xogslgm7CTBCJLEW8KgIvilDRAwleBEKQClGwE0kQxBuFDmL6/c5aH7PmY2aPM3v2OW57Hng5a31rZpxZ+8D+nfddaywFAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAU+imWmf1i4dA3vOZ/eIW7fQLrOUDtU7rFwGA5T5S66tlFnDOmRw7rIHtt2X+c2ziM7WuK0Ow+GR3bKfbZ28f7fZzTm/u1gCAJR6rdca4/ala/6l1eq1fjfVMGcJPtrN+GPxisp1gcEeti2s9UobPscrXa/1jsv+TWlfUuqTMzssfxz/vmzyOxc6u9ft+sfpsrQv7RQBgXgLYX7q1e7v9w9hhS5hqflrr3+N2gkM+76pR3Ju1vjHZT1jrn7PT7W8qr/1+l3N5db84+mutc/tFAGAmQeytMh9GLp1sx0EEtg/X+kStj5dhDHvtuJ7rmjKCvGjcj4xr76r1zclanv+xsfL8th3Z/9G4HQlvLbDlcxwrqwPC27WenOyfN9ludvqFDb3bwHZVGTqhOUdfGv/ctg+VWbc1EnC34Zdl+Fnn9TN6n8rP5vPdGgDQyZgwASX1p7J+OLu+1ot7VMasi+SX9J9rfbDWBWUYxeax6fplfJb3caTW8fHxCZW743YcrfVaGQLdlZP1BKAEmiaBoA9sLdwtc02tV8rsvLRAeRBWBbacn/vH7Tdq3Vnrf2UYU29bzntCVdxS66XJsU19utarZfhZZvz5rzJ/PduxWj+c7AMAS3yxDNdjJQgsG11tW4LKjeN2AlSCY+v0PVSGcJX9aXcrgaXJDQVPlPnr1SKvOw1BCQjrBrbme7X+XuZHrPvVuoGtvjbZTvjsR68Jse3i/ITXhNHvlyHkRgtYvWkXsq9loXx3sp3wtuhzpwPXv16rRV2/b5Xh79WtZXjud+cPn/h5ZGwNACyQX/w/7tZy7VbrTi0LAr3pSHJRTUdsUwlVrXOVxyWkNS2wJcy8XoYuU0wDWyTM/Ldb6wNbgkcf2PYa9fXX8OU5eY1mUSiZSuBaZ8S3qsPWtPfR3/iRceO2vDzZPl6GTuN+5ZrBvO+jZfEY+VjRYQOApfJLNN2nJmPFL0z2txkEFkmoaRf2Lwps55ehw7M7riW0JbDdXYbwko5crlXLeC2dtvYVHnle69xFQl0b7X25DKPOJuPOXkJr63Llv/nrMgudnyvD668y/SyrrApsGU3mfX6nzALuw2XoxuW9ZOS4DbnGLDcARK4vTNDKuc36fiT45R8BbVR9ZP7wib+DJ6urCwCHTr7OI2HgwTIEjOcmx+4pQ+cjxw9CQldudkgQ+V0ZumjZzldppLKdtctq/bMM49o/1PpbGUZrj46PyS/7dIGynddLVzBhqw+bCYYZIz5f6weT9Yzq+vFg3tuzZfgqkBfK7LVuL8M5y7lZ9VUU2wxsCTn53LlmLT+jB8ospOXYXt3CdSQ0Jaz+vAw/k3TbfjP3iM3kHCb4JVDn/E27lZHPtN9QCAD/l7YZBE6Fp/uFJfpuz15yZ+n0ztF8/1r7TrZU7uJs1gls6S5uKuEn4fHb/YE1Zaydbua7HYNvS7qE6XoCABuYjhQPo3zxbbvubS/rdJDS1XqqX1xincC2H7nhItch7vf/6JAO126/eBLcUJZf4wgArLCtIHAq3VaGu1+35Su1Hq91eX+gk5Flxok/6w+8x/U3Mxy0dHDbna4AwAZyHVj/9RKc/JEhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAALAP7wBJsMRydcDsGwAAAABJRU5ErkJggg==>

[image5]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAmwAAAAxCAYAAABnGvUlAAAFWElEQVR4Xu3dW6htVR0H4BFaKCqVCSIpFQYR2UWiyMyeLOtBjaSHqDfBQCPoYnIikIigh6BU6EG62IP0kBRBihyjNiT4IARBFwgFi0gIpJcCD9Jl/JhzsMYaZ+219tqXc9yH74M/e8wx57rMuRbMH2PMuXYpAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAsLWLa10wdvKycV6tV4ydAMDuXlfrw2URcF7drTuuflgOvh9vq/WxMgWLq4d154rs22vGzjMgr/v5sRMAWO1krVfO7TfX+nuZTqbfrfXgUMfFd7p29uXLtd5U6+Gyt/24pdafu+Wv13p3rfPL6cfk3m674+jjtT4wdp4h19S6auwEAJYlgPxm6HtgWD6OHu3a36j1l7l9SZn2d9NU3EtlCjJNwtqmx+xXnvtsacfjbE4dP13rtWMnALCQ67z+U5bDyDu69lHJFNwbar2+TGHh5rk/1zVlCvKN83JkuvZrtT7R9eXxV8yVx7d2ZPkrczsS3lpgy/7ulM0B4X+1ftUtX9q1D9teA9vbu3ZGRC/qlvfrrbVemNv5DnyoW3cQGTVrI2cfLYvPZpV8Nu8ZOwGAZZkmTEBJPVamULONT9b665rKNOsqOUk/XuvCWleWaSo222bU75EyvY/rap2at0+geG5ux4la/yxToHt/158A1E/xJRCMgW1dgIiEjH+UxXFpgfIo7DWwteMQz5blULpf95fpmGZa911l2teDSujLZ/WjWn8rU2D+99IWy3Zq3TN2AgCne1+Zrsf6b60bh3VHJUHlM3M7ASrBsY305WSfcJXlfnSrP/HnhoJfluXr1SLP24eghJttA1tze63fl+Up1oNqo4GtburaCZ+7Tb0+37VzHMbPKY/L48fnb7VKpiOzfxlpi1U3AfQjmGPl9Uafm//ms0nwvr7WrXPfq8rieslmp0zT1gDAChmF+urQl2u32ujUXqfc1p3QU+MJukmoaiNX2S4hrWmBLWHmX2UahYtxpCYjci8OfWNgS2gYA1uu3drNA8NyHpPnaDJtu06C0zZTfHsZYcvUcQs1GYFMGNrr57NO+7xzg8XPhnUHdaqcfjPDU2URDpudYoQNAHaVEJLRpybTiu/tljNddpQSatqF/asC22VlGtl6bu5LaEtg+3aZQlRG5DIteHmZAkz7CY88ro3cRUJdpubihjJNdTarpgATYtooV17ze2UROt9ZpuffpN+XTfYS2BJoPlUWd7zmuH1kaYvttSCaAJj3m/a3lrbYXj6LfKfeUqYglte4u0zvPVZ9p7L9OFoIAMxO1vpCrYfKdML+Q7fuvjKdcLP+KCR05WaHBKaflGkULe2M9KTSTl+uq3qmTNO1P631u1pfrPXzeZuc7HO9Wdp5vow6JdSMwSAB545af6x1Z9efKeCEil7e22/LFIz+VBbPleCRY5Zjs+mnKA4zsGUEM8E1ofMXte6al3/cb7QPuenjs3P7tjI9f1verwTb7HuuTcx0a0Lgibk/wbC/87bJ9y4jiADAljKVtW7a8OXuybFjF9eNHWvkztL+ztHxd+o+2K3bJrBldHGdjD4ljB537Tv16a4v18Bl1BMA2Id+SvE4yg/ftuve1vnB2LFGrr369di5i20C2yaZDu1vODiu8l8jMtrWT+UmvO12jSMAsEHuvMwNCQf9105n05fKdPfrYcnPVTxR69pxxeD7ZQpY3xxX7NO5+j83M9p25dgJAOxdAsK5GBIO6jDuzAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAzpD/A8texDlUaBXuAAAAAElFTkSuQmCC>

[image6]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABIAAAAaCAYAAAC6nQw6AAABKElEQVR4Xu3TzStEURjH8UdSGCXS1CxsJikWJFlamKLmb5AtZSUlhSVL5WUtWWApG4lCWSplYWXF3kZZWfD93XPui9vcqZGazf3VZ3Huczovz5kxy5Pnf9OKccyggBYMYhodiXl1040TLGMdT9jBFg5whvZodka08yYm/biEV5xiBO+4Q5evZ6YXGxbvOIoPzKINcxj2tTCd5lpRN1rg01y/akVtuLTsehBd8xAP6EnVwujE9+hLF7TDPubNFZ/NLaZFFb2aarr6Cq7whl30+zlBqvjGNqbwhVVf0yZHGPBjZQ8LiXGUMh5xjHMs4cXcs1+gEk8NrnuDicS3X9Gxixa/RHocZgi3VqM/jUYvqpPrZ7Fo7up/ihp/jTWMpWoNR/9DnShPs/ID3zcnxrOyMrkAAAAASUVORK5CYII=>

[image7]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAwAAAAbCAYAAABIpm7EAAAA0klEQVR4XmNgGAWDEcgDsS6aGDMQ86KJgQEPEB8A4qVAzIgk3gDEF4FYGEkMDJSA+DkQFyGJcQPxHgZMQ8DAD4h/ArENkhjMkHQkMTiYBMR3gVgcSQybIXBAkgaYh9cAMQuSeCsDpiFgoAnEb4G4CkkM2RAQuxuIxWCSIKv/M0BMhIFgIP7LADFEB4h7GZBCCuT+f0D8HojbgXgFENcB8VwgPgXEy4HYEKYY3WpJKA0CIBNBEQbjgwHM/cgRhhfgDDpcAGQy1rSCC4DcCUqRIwYAAEHeJrl0xYuMAAAAAElFTkSuQmCC>

[image8]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAsAAAAbCAYAAACqenW9AAAA9UlEQVR4Xu3SoYsCQRTH8ScqKOqhGC+IeAjX/AMuarJpNPgPWLQYxSheunbRfigGu2C0mkwGsVmMd3B33+fOwDK7cggXDP7gAztvht03Mytyk4kgj6w74WaET/yg58yFpoEvvLgTYXnDDo9OPZAMVlgg4cwF8owj+masmy2jhqRdZNPCN6qIY4gx5hKyYdtvCQNUxFsUOJ0c1tjgXbyWNNpGFykzPsf2exLv7drCg3+BP1ct9l/GE7aYyYUjdC9jIt4edC91tE1d0ljiAzFT08U61vN9RdHUpYADOrZAmthjaup6Qefog34uagsm+sU/f9V7/j+/q+oqHbeR1QMAAAAASUVORK5CYII=>

[image9]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAwAAAAaCAYAAACD+r1hAAAA4UlEQVR4Xu3SsQtBURTH8SMMZCCDZJCymK2yGUkWym4yGVgtNguLf0IZrAaTv4KiZDMaJPG93ru69/WYDX71Wc65r3M6PZGfTwBJxL0Nv4zxcPU8vY9p4IaSt/EpU+yR8dR9E8Mac4Tsln8KOGOAFKooI2w+MlPDHSvM0MIGS0SMd++o/dUHTXHOq6KmHZDWj3T0/guxVxhhJ86KVsz9db4eQe1/Ffv+RVzQNmrvqNF7se8/xBE5VNDRDb/RZi2KCfJuT7I4oasL4lypj604H9WN3quZQNAsulF/rZr2z4/lCbTFJhO8bMfAAAAAAElFTkSuQmCC>

[image10]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAmwAAABMCAYAAADQpus6AAAFq0lEQVR4Xu3dXchlVR0H4CVaiBaZCjKYqBGhZoqkRRgmkVEXeeEHKl50oaCEICgaheJICIo3FRKkBhpIEEFEXfQBNhikeBckiOCFIYqJiJEXFlnrN3vvOetdnXk9k+/5mHmfB/6w91qHmTlnbn6sz1IAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABg45xU64Vavxrff1fr+VonHPgEAABrd2Wtn9Y6ptbltT6ztRsAgHU7v9ZXa51b69u1Tt7aDQDAOrXh7Lla1zXvAABsgHb6M4HNdCgAwA66ptZjtV7q2heVDQdv1jpjfM+U6FEHegEA2DH/b2ADAGBFBDYAgA0nsAEAbDiBDQBgwy0a2HLG2n9qvdJ3zJENCN8pw+f3bu06bHys1g9rXdJ3AACs2st9wzYS1hLC7iyL7wh9stYtXdsHan2ka1un/HtaX6/19/H5vlrHN30AAPt9pdal4/Oygs0fa/2jDAEsoe3mrd1z5Rqqf491Wdd3MKfXerxr+1Gtz3Ztk6/VOq1vXLIEtiua9wS0i8fnH9Q6tukDAI5wb9R6Ynw+utZdtW6fde+/PP27zXtGtBYdyVqVBMiEvFQ/MrWIBL15ITR3keZO0p/UeqfrW4Vcr5Xfv/W9Wld3bQDAES4h56bmPTcD7Kv1ofE9twWccqB3Fu42TQ7MzXdpw+aiftM3jD5e69UyjMh9sutbhYTIPzXvCaNfGp8TJgGAXSChLIFsun8zAeGpMlyePvVnqjE3EEzOa543ScLMw2UIbbnx4FBkKraX735PGdaN7SnLmYI8owwbCLYbFcx0bDYbXFqG/4tpJBEA2CUuL8OUW4JOKsGl34GYEbUpJKSmkbfWibX+uk3dPfvo0mXqMsEma9sWke+TRfzz7Kv1875xB1xY629lmFr+cBl+94NNMycsZsMBALBLJai0l6NPx2T0ptGrf9b6cte3abJbNN9hkaM+IoHtW33jKBsf2jD3wea5lSnjBKt5lXWBvX21nh2f8/fnfQrC/efzZ1zftQEAu8TZZdhw0I7sZJH7u2U2/ZcNCK2M9Hyha4v8GX1QaeuE2UdXIqNXCW6L2C6wvV22Btqnm+f3I7/xFHzze/5rfM5U9DQ9PTHCBgC7WKZD+9G0n5VhOnGS4yMmCWXZLTpvSjQjcFdtU5+bfXTpptHA7daFtbJ4/9G+cfRimW24uKHWa2Vn1vC1QTC7Pl+vdUcZpqe/P31olI0PWccGAOwyj5XZmrRpnVmmO89pPnNqrdvK8NlvlGFzwheb/k2UUDlvA8F7+UvfUIag1k6HJti156K9H98sw3TrL8vsN/9oGQ707WU6dF5IBgBYqUwDJmglQOZ4jsm0Hi316aZ9nhx6+0LfuKCMHE5TwxmZyyjXjWXrcSaZuswGgTObtp2UYJYNHglzk0/UeqZ5BwBYu7fK/07T5hyys7q2ef5cFt8ZmnB2bfOeYzM+NT4nGCY0/mLWvV/6syljWS6r9ftaFzRt95bF1+IBACxdrl5KcHqobA1TmZbdTs6OS9A52LEY82QKuN95mQNpb+3a1ilT0vNuXwAAWJssrs/u1M+XrTcPzNuV2prOkFvUg2VYP7asqU0AgCPW3uY506IZMcvRI9tdwdSucTuUyvElAAAcgoyuPdK87611S60HmjYAANYoZ8K1C/qzOzJnnv22aQMAYI0SzPoDad+pdVPXBgDAYay/g7OXNXEX9Y0AAKxGjr9o7/yc59dluK0BAIAVa+/gPK3r6z3eNwAAsBrTJfU5AmRfVwlyx439AhsAwBrkrLY/NO97ujqpzG5BENgAANZg3h2c8/y41qu17u87AABYvtxDCgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAczv4L/PDQX0DznR0AAAAASUVORK5CYII=>

[image11]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAbCAYAAAB1NA+iAAABNElEQVR4Xu2TMShFYRTHj1A8JCX1StmUeoXBxMoiZVRvML5srzeaLMpMUS8lg0HZjMRgw/oWM5ksNkne7zhefd+5t/cRk/zrN9zzP3333HP/n8ifVx8swDh0OC+pMbiCLXiA+dhOa0PsgE14h5XITWgIbmAfhmESOqOOhCbgCWreSKkHilCGN7Gx9bk/bGqnWajDLbzA0efzTNj0FR2K7UB38W0NiG3/BLqcp4ssuFpGo3AP694Q28mBL3r9+IA5eIUlb4jlouKLXtrwLBaelqbE3vwIp7AaeBltQ0MsgaE0XJc59Uh6+84l/w9ouDQXbW9la4F536mT5dU/TqzCGSyLHVCKOixQF2KJnIbF0ByBO7Hk7cKeZMfvhWOxK64XrDs0dYI1uIYdGAzNQJpCTem/fltNri4wDwMvUCEAAAAASUVORK5CYII=>

[image12]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAaCAYAAAC+aNwHAAABEUlEQVR4Xu3SP0tCURjH8UdMyCJICKwhBGmJFoV6A0FChHvg4KiDk+Lea2iMQBoiJKFBBCd7Aa5uESGETb4Ch/o+nGP33MPtHzTeH3wGn+f43HPuPSJx4kQnizJKWPd632YbPTzgHG284NBd9FVymOAKKVtL4BZDpG0tMiu4xgx7Xu8GU+zY30msBW2TfczFbF+HLbOKgYQH6NE6nyts9IW9o+bV83jDnQSDdaf+OjkVM0AHualjgRMUxDxZB/ZRddbJLp7QcGoHeEZLzMvU6FEfsbVc5EZ38Yp7a4xjCf6sqYj5Km4tFP18eok2/YbNpUSc/7fJYIQjFHEWbv8cvUhdXKApwWX7U/QSbfjFOP+QD+BuJgNpmNRdAAAAAElFTkSuQmCC>