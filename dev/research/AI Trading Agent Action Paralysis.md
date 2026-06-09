# **Architectural Solutions for Overcoming Analysis Paralysis and Status Quo Bias in Autonomous LLM Trading Agents**

## **Introduction to the Executive Failure Mode**

The integration of Large Language Models (LLMs) into autonomous financial systems has exposed a critical divergence between diagnostic capability and executive function. In the architecture under review, the trading agent demonstrates a sophisticated capacity for market pattern recognition. It accurately identifies regime shifts, diagnoses position vulnerabilities such as excessively wide stop-loss levels, flags invalidated structural theses, and outputs highly detailed, technically sound analytical reasoning. However, despite this advanced diagnostic acuity, the agent consistently defaults to a passive HOLD or PASS state, refusing to execute the proactive management actions dictated by its own internal reasoning. This phenomenon represents a structural failure in the agent's prompt architecture and decision-making framework, categorizable as "analysis paralysis" 1 and "status quo bias".4  
Within this specific deployment environment, the AI operates under an asymmetric action threshold constraint. New market entries (BUY or SELL) require a robust convergence of multiple technical triggers, establishing a high-friction barrier to entry. Conversely, position management actions (ADJUST\_STOP, CLOSE) lack an equivalent deterministic framework. Deprived of explicit, programmatic obligations to manage risk dynamically, the language model interprets the current state as a secure baseline. It inherently views active management as an assumption of unwarranted risk, inventing permission constraints where none exist. To resolve this latency between analysis and execution, the system necessitates a fundamental paradigm shift. The architecture must transition from a descriptive analytical framework ("What is the current state of the market?") to an imperative executive framework ("What actions are mandated by the current state?").  
This report provides an exhaustive, theoretically grounded, and empirically testable architectural blueprint for resolving analysis paralysis in single-call LLM trading agents operating under strict execution parameters. By engineering explicit Cognitive Forcing Functions (CFFs) 7, mandating structural opportunity cost evaluations 10, and deploying strict JSON schema validations that operate as pre-action decision trees 12, the agent's latent analytical power can be translated into deterministic, proactive position management without inducing account-draining overtrading.

## **The Theoretical Pathology of LLM Inaction**

The inability of the LLM to translate reasoning into execution is not an unpredictable glitch, but rather a manifestation of the inherent behavioral characteristics of autoregressive language models. These models are highly susceptible to reproducing human cognitive heuristics, specifically when contextual factors and prompt structures inadvertently penalize definitive action.

### **Manifestations of Overthinking and Analysis Paralysis**

Recent empirical studies on Large Reasoning Models (LRMs) and autonomous agents navigating interactive environments categorize this behavioral loop as "overthinking".2 When presented with complex, multi-variable environments, language models frequently favor extended internal reasoning chains over active environmental interaction.1 This overthinking manifests in three distinct typologies within agentic workflows, summarized in the table below.  
**Table 1: Typologies of LLM Overthinking in Agentic Tasks**

| Overthinking Typology | Behavioral Manifestation | Application to the Trading Agent |
| :---- | :---- | :---- |
| **Analysis Paralysis** | The model focuses on heavy planning and repetitive state evaluation instead of interacting with the environment.1 | The agent repeatedly diagnoses the market regime and technical levels but loops back to a safe HOLD state rather than finalizing a modification. |
| **Premature Disengagement** | The model concludes the task without verifying outcomes with the environment, often due to an assumption of failure or overconfidence.1 | The agent recognizes a slight negative PnL but delegates authority back to the "existing SL/TP" parameters, disengaging from active management. |
| **Rogue Actions** | After facing setbacks, the model generates multiple erratic actions without waiting for environmental feedback.1 | (Less prevalent in this specific system due to the single-call constraint, but a latent risk if passivity leads to a sudden margin call). |

In the specific context of the user's trading agent, Analysis Paralysis is the dominant failure mode. The model maps the environment perfectly but lacks the programmatic forcing function to disturb the status quo.

### **The Mathematical Framework of Status Quo Bias**

LLMs have been definitively shown to reproduce status quo bias, defined as a disproportionate preference for the current state of affairs over available alternatives, driven by an aversion to the immediate, perceived costs of change.4 This bias can be formalized using a utility threshold model. The language model evaluates whether to transition from a HOLD state to an active management state by computing the following equation:  
![][image1]  
In this formula, ![][image2] represents the perceived utility of taking an action (e.g., closing a trade or moving a stop loss), ![][image3] represents the perceived utility of doing nothing, and ![][image4] represents the cognitive switching cost or the perceived risk associated with executing an action.10  
Under the current prompt architecture, ![][image3] is artificially elevated because the prompts do not enforce a penalty for holding dead capital or enduring theta decay. Concurrently, ![][image4] is massive. Because the prompt environment stresses account survival and strict risk constraints (20% max risk, 5% daily loss, 10% drawdown) without balancing those constraints with an imperative for capital efficiency, the LLM hallucinates massive risk associated with any adjustment.19 Consequently, ![][image5] rarely breaches the threshold required to execute CLOSE or ADJUST\_STOP. The architecture inherently incentivizes the model to accept the status quo.

### **Asymmetry in Action Triggers**

The system's current trigger framework exacerbates this bias. The agent operates under a rigid constraint requiring "3+ action triggers" for new entries (BUY/SELL). This establishes a high-friction, deterministic barrier against overtrading. However, position management (CLOSE, ADJUST\_STOP) possesses "no trigger requirement." While initially designed to provide the LLM with the flexibility to manage trades dynamically, this absence of explicit management triggers removes the programmatic obligation to act. When language models are not provided with strict conditions under which they *must* act, their default posture is passive observation.19 The asymmetry creates a system where entering a trade is an objective, mathematical process, while managing a trade is a subjective, highly penalized, and therefore avoided, process.

## **Empirical Evidence of Executive Failure**

A rigorous analysis of the provided empirical evidence highlights how these cognitive biases manifest as operational failures. The five examples provided demonstrate a systematic breakdown between the evaluation of market data and the generation of the final action token.

### **Example 1: The "Survival Stop" Rationalization**

In the WETH/USD example, the agent observes an existing stop loss set at 1562.36, which it accurately calculates as 8% below the entry price. The model explicitly diagnoses this distance as "very wide." However, it subsequently outputs: "this was set for Tier 1 micro-account survival. No action needed; position is live with defined risk."  
The agent possesses the ADJUST\_STOP action and operates under risk constraints that dictate it must "trail stop after 2R profit using ATR-based trailing" and "NEVER move stop further away from entry." Despite these parameters, the LLM reframes a dangerous liability (an 8% stop) as a mechanism for "account survival." The cognitive bias here is risk aversion masking as prudence. Because the prompt does not penalize holding excess risk, the model finds a semantic justification to avoid the ![][image4] threshold. It settles for defining the risk rather than managing it.

### **Example 2: The "Legacy Error" Hallucination**

In the LINK/USD example, the agent observes an even more extreme stop loss, situated 12% below the entry price. The reasoning engine correctly identifies this as "absurdly wide" and further deduces that it is "likely a legacy error." However, the model refuses to execute an adjustment, stating it is "not my call to adjust without explicit instruction."  
This is a profound manifestation of excessive caution and passive default. The ADJUST\_STOP tool exists specifically to rectify such imbalances. Yet, faced with an anomaly, the LLM invents a hierarchical permission constraint that does not exist in its prompt instructions.20 The model recognizes the danger but abdicates responsibility, preferring the certainty of the status quo (even a flawed one) over the perceived risk of unauthorized action.

### **Example 3: Contradictory JSON Outputs (FID-087)**

Prior to the implementation of the FID-087 safety net, the agent evaluated a LINK/USD position and produced the following reasoning: "The short thesis is structurally weak: uptrend intact, EMAs bullish, price holding above range mid-point. Recommend closing this position." Despite this explicit recommendation, the generated JSON output was "action": "HOLD".  
This contradiction exposes the mechanics of autoregressive token generation. Freeform reasoning generation operates semi-independently of the final token selection for the action field. The model explores the concept of closing during the generation of the reasoning text, but when required to select a definitive action token, the statistical weight of the status quo bias overrides the preceding logic. While the decision parser's safety net catches this contradiction post-generation, relying on a parser override masks the root cause: the prompt fails to construct a deterministic bridge between the reasoning tokens and the action token.

### **Example 4: Regime Recognition Without Exploitation**

Over multiple cycles, the agent identifies a ranging market (ADX 19.4 \< 20\) and notes that the price is oscillating between 8.00 and 8.13. However, it concludes: "No new entry triggers met. HOLD."  
In a mean-reverting or ranging market, standard momentum-based entry triggers are invalid. The agent correctly maps the regime and identifies the support and resistance boundaries, yet it fails to transition into a range-trading strategy (buying support, selling resistance). The model's internal definition of an "entry trigger" is exclusively anchored to trending mechanics. Without a regime-specific translation matrix instructing the model that support/resistance levels *become* the triggers in a low-ADX environment, the agent is paralyzed, watching actionable oscillations occur without participating.

### **Example 5: Tolerance of Dead Capital**

In a WETH/USD position, the agent observes a slight negative PnL (-0.03) in a ranging market with a neutral RSI (44.8). Its conclusion is to "Hold and let existing SL/TP manage."  
The agent correctly diagnoses a stagnant market with no momentum and a negative position. However, it delegates the authority for the position back to the static parameters. This represents Premature Disengagement.1 The prompt architecture fails to define "dead capital" as an operational risk. The model does not calculate the opportunity cost of locking up margin in a non-performing asset; therefore, it perceives no utility in closing the trade or tightening the stop to break-even.  
**Table 2: Summary of Empirical Failure Modes**

| Scenario | Market Context | Agent Diagnosis | Final Action | Root Cause of Failure |
| :---- | :---- | :---- | :---- | :---- |
| **1: WETH/USD** | Live position, 8% stop loss. | Stop is "very wide." | HOLD | Semantic rationalization of risk; no imperative to optimize capital efficiency. |
| **2: LINK/USD** | Live position, 12% stop loss. | Stop is "absurdly wide, legacy error." | HOLD | Hallucination of permission constraints; status quo bias. |
| **3: LINK/USD** | Short position against bullish EMAs. | "Recommend closing this position." | HOLD (pre-parser) | Autoregressive disconnect between freeform reasoning and action token generation. |
| **4: Multiple** | ADX \< 20, price bounding 8.00-8.13. | Ranging regime, no momentum triggers. | HOLD | Lack of regime-specific behavior matrices; anchoring to trend triggers. |
| **5: WETH/USD** | Negative PnL, ADX \< 20\. | Ranging, neutral RSI, slight loss. | HOLD | Premature disengagement; failure to penalize dead capital and opportunity cost. |

## **Architectural Constraints and the Single-Call Imperative**

Developing a solution for this system is bounded by rigid operational constraints. The system utilizes owl-alpha via OpenRouter. While it benefits from a massive 1M token context window, it is strictly restricted to a single LLM call per cycle. Furthermore, the agent operates on the Arbitrum DEX via the 0x API, meaning every executed action incurs real-world slippage and gas fees, making overtrading a critical threat to the micro-account's survival.  
The single-call constraint is paramount. In contemporary multi-agent research, status quo bias and overthinking are frequently mitigated using sequential debate frameworks. For instance, the Think, Validate, Consensus (TVC) multi-agent system utilizes Rational Speech Act (RSA) theory to allow agents to detect reasoning loops and correct each other.2 Similarly, architectures often separate the "Analyst Agent" from the "Risk Manager Agent" to prevent a single model from holding contradictory goals.21  
Because this architecture cannot support multi-agent loops, the cognitive forcing functions, decision trees, and validations must be embedded entirely within the prompt and enforced via the structure of the JSON output.22 The single inference pass must be engineered to simulate a multi-step cognitive process. The model must be forced to act as the Analyst, the Risk Manager, and the Executioner in a strict, unalterable sequence during the generation of its response.13

## **The Pre-Action Verification and JSON Schema Forcing Function**

The most critical intervention to cure analysis paralysis is the implementation of a structured decision framework enforced via a strict JSON schema. Freeform reasoning allows the LLM to drift conceptually and succumb to autoregressive biases.2 A JSON schema, however, acts as an inescapable pre-action verification checklist.12  
By engineering the output\_format.md prompt to require specific JSON keys in a specific order, the system exploits the LLM's next-token prediction mechanics. If the schema requires a "technical\_invalidation\_check" field to be generated *before* the "action" field, the LLM is constrained to follow a logical sequence. It cannot output HOLD without first mathematically validating the technicals.

### **Engineering the position\_audit Array**

The core of this solution is the introduction of a mandatory position\_audit array within the JSON output. Before the model is permitted to evaluate new setups or output a final execution command, it must iterate through every open position and answer highly specific, quantitative questions.  
This structure acts as a Cognitive Forcing Function (CFF). CFFs are designed to interrupt automatic, heuristic-based defaulting (System 1 thinking) and compel analytical rigor (System 2 thinking) prior to decision-making.7 Instead of asking the agent a broad question ("What should we do?"), the CFF demands binary evaluations of technical thresholds.  
**Table 3: The Structured Decision Framework within the JSON Schema**

| Schema Element | Data Type | Cognitive Forcing Function Purpose |
| :---- | :---- | :---- |
| current\_regime | String | Forces the model to explicitly state whether the market is Trending or Ranging, gating the subsequent logic. |
| current\_stop\_distance\_percent | Numeric | Forces a mathematical calculation of risk, preventing semantic generalizations. |
| is\_stop\_absurdly\_wide | Boolean | Compares the stop distance against a hard ATR multiple. If true, the path to HOLD is structurally blocked. |
| opportunity\_cost\_of\_holding | String | Forces the model to articulate what is lost by not acting, artificially depressing the utility of the status quo (![][image3]).11 |
| thesis\_invalidated | Boolean | Checks current price action against the original entry thesis. |
| management\_trigger\_active | Boolean | Consolidates all prior checks. If any risk parameter is breached, this must be true. |
| mandated\_action | String | The forced conclusion of the audit (CLOSE, ADJUST\_STOP, HOLD). |

By forcing the generation of mandated\_action within the audit array *before* the overall final\_execution block, the model's attention mechanism is heavily weighted by its own just-generated conclusion. If is\_stop\_absurdly\_wide evaluates to true, the token probabilities for mandated\_action will overwhelmingly favor ADJUST\_STOP. Consequently, the final execution action will align with the reasoning, entirely eliminating the contradiction seen in Example 3\.

## **Overcoming Status Quo Bias via Opportunity Cost and Trigger Parity**

Structuring the output solves the alignment problem, but the system still requires rules to dictate *when* the CFFs should trigger an action. This requires two strategic prompt shifts: establishing trigger parity and engineering opportunity cost.

### **Action Trigger Parity: Establishing Management Triggers**

The current system exhibits an anti-action bias because entries require three or more triggers, while position management requires none. To create a healthy bias toward action without encouraging overtrading, the prompt must establish **Management Triggers**.  
Just as a BUY action requires momentum confirmation, a passive HOLD action must now require the *absolute absence* of any Management Triggers. If a single Management Trigger evaluates as true, the agent is strictly prohibited from returning HOLD.  
**Table 4: Definition and Execution of Management Triggers**

| Trigger Category | Specific Condition for Activation | Mandated Action Response |
| :---- | :---- | :---- |
| **Stop Distance Violation** | Current SL distance ![][image6] current 14-period ATR. | ADJUST\_STOP. The stop must be tightened to a technically valid swing low/high or ![][image7] ATR. |
| **Regime Incompatibility** | Position was opened in a Trending regime, but current ADX has fallen below 20\. | CLOSE or ADJUST\_STOP to an aggressive trailing level. |
| **Structural Invalidation** | Price crosses and closes below the moving average support or structural lower low that formed the original thesis. | CLOSE. Immediate exit is mandated; waiting for the hard stop is prohibited. |
| **Dead Capital Tolerance** | Position PnL is negative or flat after ![][image8] sequential cycles in a ranging market with neutral RSI. | CLOSE. Free up margin and eliminate theta decay. |
| **Profit Protection Ratchet** | Position PnL ![][image9] (Reward/Risk units). | ADJUST\_STOP. Trail the stop to lock in a minimum of ![][image10] profit. |

By formalizing these triggers within the strategy\_knowledge.md file, the model is stripped of its ability to invent permission structures. The rules become absolute, mathematical laws governing the system. If the trigger is hit, the action is non-negotiable.

### **Opportunity Cost Engineering**

To further dismantle status quo bias, the prompt must make the risks and opportunity costs of inaction explicit.10 Human and AI agents alike suffer from opportunity cost neglect, ignoring the tradeoffs of their decisions.24  
The prompt must explicitly instruct the LLM that HOLD is not a passive state; it is an active declaration of risk assumption. Holding a stagnant position incurs a cost: capital lockup, exposure to sudden macro shocks, and inability to deploy margin into superior setups. By explicitly prompting the model to evaluate the opportunity\_cost\_of\_holding within the JSON schema, the perceived utility of holding (![][image3]) is depressed.11 When the model writes out "holding this position locks up 10% of margin in a market with zero momentum," the subsequent generation of a HOLD token becomes statistically incongruous.

## **Regime-Specific Translation Matrices**

The agent's failure to exploit ranging markets (Example 4\) highlights a lack of contextual translation. The LLM understands that the market is ranging, but its operational definitions for entry and management are rigidly anchored to trending mechanics. The prompt must encode regime-specific behavior rules, creating distinct operational modes.

### **Regime Mode: Trending / Momentum (ADX \> 25\)**

* **Operational Bias:** Trend-following. The goal is to let winners run while trailing risk.  
* **Entry Protocol:** Require standard momentum triggers (EMA crosses, volume breakouts, MACD expansion).  
* **Management Protocol:** Mandatory ATR-based trailing stops. ADJUST\_STOP must be called at defined intervals of profit expansion. HOLD is permitted only if the stop has recently been optimized for the current ATR and the trend remains intact.

### **Regime Mode: Ranging / Mean Reversion (ADX \< 20\)**

* **Operational Bias:** High action frequency at defined boundaries; low tolerance for holding in the middle of the range.  
* **Entry Protocol:** The standard momentum triggers are suspended. Support and resistance levels *become* the triggers. The agent must execute a BUY at the defined support band and a SELL at the defined resistance band. Waiting for momentum confirmation is prohibited, as it leads to late entries in a range.  
* **Management Protocol:** Aggressive profit-taking. Targets must be set at the range mid-point or the opposite boundary. Stops must be placed tightly outside the range extremes. HOLD is only permitted if the price is actively oscillating in the middle 50% of the range and moving toward the target.

By bifurcating the logic based on the ADX reading, the agent is given explicit permission to alter its behavior. It no longer needs to search for a breakout trigger in a sideways market.

## **Preventing Overtrading: The Anti-Pass Bias Restraints**

A critical constraint of this architecture is the prevention of overtrading. Fees and slippage on micro-accounts operating via DEX APIs quickly erode principal. By engineering an aggressive anti-pass bias, there is a systemic risk that the agent will rapidly cycle positions, moving stops by fractions of a percent every cycle. This risk is neutralized through three specific restraints:

1. **Deterministic Action Routing:** The urgency to act is strictly gated by the Management Triggers. The agent is not instructed to "trade more frequently"; it is instructed to "manage actively *only* when thresholds are breached." If the stop is ![][image11] ATR and the trend is intact, the management\_trigger\_active boolean in the JSON schema evaluates to false. When this boolean is false, the agent is permitted, and encouraged, to return HOLD.  
2. **The Quantized Ratchet Constraint:** To prevent the LLM from constantly micro-adjusting stops and incurring zero-value gas fees, a hard constraint must be introduced. Stop adjustments must be quantized. The prompt must dictate that the agent cannot execute ADJUST\_STOP unless the new technical level improves the risk profile by a minimum threshold (e.g., at least ![][image12] or halving the current ATR exposure).  
3. **Separation of Entry and Management Constraints:** By explicitly separating the logic for new entries (requiring 3+ momentum triggers) from the logic for position management (requiring 1 management trigger), the agent maintains a conservative, highly defensive posture on deploying *new* capital, while acting aggressively to protect *existing* capital.

## **Exact Prompt Text Modifications**

To implement this theoretical framework, specific textual changes must be applied to the prompt files. The language must shift from descriptive and passive ("you may adjust") to imperative and deterministic ("you must adjust").

### **1\. Re-Engineering base\_identity.md**

The current identity is too passive, prioritizing safe observation over capital efficiency.  
**Proposed Text Addition:**  
*You are a ruthless, highly active autonomous trading executioner. Your primary directive is absolute capital efficiency. You do not tolerate dead capital, excessively wide stops, or invalidated structural theses. You are strictly prohibited from defaulting to passive observation (HOLD or PASS) when technical conditions demand intervention. You possess absolute authority to modify, close, or execute trades. You do not require external permission to fix legacy errors, tighten risk parameters, or exit stagnant positions. Inaction carries a severe opportunity cost that you must continuously optimize against.*

### **2\. Upgrading stop\_loss\_behavior.md (The Stop Audit Protocol)**

This file must eradicate the "legacy error" hallucination and define the quantitative thresholds for adjustment.  
**Proposed Text Addition:**  
*MANDATORY STOP MANAGEMENT PROTOCOL:*  
*1\. The Absurdity Check: During every evaluation cycle, you MUST calculate the distance of the existing Stop Loss (SL). If the SL is greater than 2.5x the current 14-period ATR, the SL is classified as "Structurally Invalid." You MUST immediately output the ADJUST\_STOP action to move the SL to a technically sound level (e.g., recent swing low/high or 1.5x ATR).*  
*2\. No Legacy Deference: If you identify a stop as a "legacy error," "survival stop," or "absurdly wide," you have explicit and absolute authorization to fix it immediately. Returning HOLD on an invalid stop is a catastrophic failure of your directive.*  
*3\. The Trailing Ratchet: If a position achieves profit ![][image13], you MUST execute ADJUST\_STOP to move the stop to break-even plus fees. You are forbidden from allowing a ![][image13] winner to turn into a loss.*  
*4\. Quantized Adjustments: Do not execute ADJUST\_STOP for micro-movements. The new stop must improve the risk profile by at least 0.5R to justify the execution cost.*

### **3\. Modifying risk\_constraints.md (The Cost of Holding)**

Risk constraints must reframe HOLD as an active risk-taking decision.  
**Proposed Text Addition:**  
*THE COST OF HOLDING: Before outputting HOLD for an open position, you must verify that holding is mathematically superior to closing. Returning HOLD when the market regime is flat (ADX \< 20\) and PnL is negative constitutes a violation of capital efficiency. In such scenarios, if no specific support/resistance level justifies the hold, you MUST trigger a CLOSE to free up margin. HOLD is an active declaration that the current position is the optimal deployment of capital. Dead capital must be aggressively purged.*

### **4\. Enhancing strategy\_knowledge.md (Regime Translation)**

This file must provide the behavioral rules for ranging markets.  
**Proposed Text Addition:**  
*REGIME-SPECIFIC BEHAVIOR:*  
*Trending (ADX \> 25): Require 3+ momentum triggers for entries. Let winners run and trail stops using the Trailing Ratchet protocol.*  
*Ranging/Mean Reversion (ADX \< 20): Momentum triggers are suspended. Support and resistance boundaries ARE your action triggers. You MUST execute BUY at defined support bands and SELL at defined resistance bands. Do not wait for momentum confirmation. Profit targets must be set at the range mid-point or opposite boundary. You are prohibited from holding positions that stall in the middle 50% of the range without momentum.*

### **5\. Overhauling output\_format.md (JSON Schema Enforcement)**

The JSON schema must be restructured to enforce the pre-action verification checklist.  
**Proposed JSON Schema Structure:**

JSON  
{  
  "market\_regime\_classification": {  
    "current\_regime": "Trending | Ranging | Volatile",  
    "adx\_value": 19.4,  
    "implied\_strategy": "string (e.g., 'Buy Support/Sell Resistance' or 'Trail Stops')"  
  },  
  "position\_audit":,  
  "new\_setup\_analysis": {  
    "triggers\_met": 0,  
    "decision": "BUY | SELL | PASS"  
  },  
  "final\_execution": {  
    "action": "BUY | SELL | CLOSE | ADJUST\_STOP | HOLD",  
    "target\_symbol": "LINK/USD",  
    "execution\_parameters": {  
      "stop\_loss": 7.65,  
      "take\_profit": null  
    }  
  }  
}

The placement of final\_execution at the absolute end of the schema ensures the LLM has already generated the logical constraints in the position\_audit block. If management\_trigger\_active evaluates as true, the autoregressive token generation for action in the final block will overwhelmingly favor the active command.

## **Backtesting the Framework: Resolution of Empirical Failures**

To validate the efficacy of this new architectural framework, it is necessary to subject the user's specific empirical failure examples to the proposed prompt constraints and JSON schema, demonstrating how the cognitive forcing functions systematically eradicate the errors.

### **Resolution of Example 1: Wide Stop Not Adjusted**

**Original Failure:** The AI recognized an 8% stop as too wide but rationalized it as "micro-account survival" and output HOLD.  
**Framework Execution:**

1. The JSON schema forces the model to enter the position\_audit block and evaluate current\_atr\_percent. (Assume standard crypto ATR is \~2.5%).  
2. The model calculates current\_stop\_distance\_percent as 8%.  
3. The CFF forces the evaluation of is\_stop\_absurdly\_wide. Since 8% is greater than (![][image14]), the boolean mathematically must resolve to true.  
4. According to **Rule 1** of the new stop\_loss\_behavior.md, a true value here mandates an ADJUST\_STOP response. The LLM is structurally blocked from outputting HOLD. The "survival" rationalization is superseded by the mathematical threshold. The model outputs ADJUST\_STOP and tightens the parameter.

### **Resolution of Example 2: The "Legacy Error"**

**Original Failure:** The AI called a 12% stop a "legacy error" but hallucinated a requirement for "explicit instruction," resulting in HOLD.  
**Framework Execution:**

1. The updated base\_identity.md explicitly dictates: "You do not require external permission to fix legacy errors."  
2. The JSON schema requires the model to state the optimal\_technical\_stop within the audit array.  
3. Because the prompt explicitly delegates absolute authority and the schema forces the calculation of the correct stop, the status quo bias is broken. The LLM calculates the correct stop and outputs ADJUST\_STOP, overriding its natural deference to perceived human oversight.19

### **Resolution of Example 3: Contradictory Actions (FID-087)**

**Original Failure:** The freeform reasoning stated "Recommend closing," but the JSON action was generated as "HOLD".  
**Framework Execution:**

1. Under the new JSON schema, the model must output mandated\_action: CLOSE inside the position\_audit array *before* it reaches the final\_execution block.  
2. Because autoregressive language models predict the next token based entirely on the preceding context window, explicitly committing to CLOSE in the audit array heavily biases the attention mechanism. When the model reaches "action": in the final block, the highest probability token is now "CLOSE".13 The parser safety net (FID-087) is no longer the primary fail-safe; the prompt architecture itself ensures alignment.

### **Resolution of Example 4: Ranging Market Abdication**

**Original Failure:** The model identified a range (8.00 \- 8.13) with ADX 19.4, but output HOLD because standard "entry triggers" were not met.  
**Framework Execution:**

1. The schema forces market\_regime\_classification. The LLM logs adx\_value: 19.4 and current\_regime: Ranging.  
2. The implied\_strategy field must be filled. The new strategy\_knowledge.md rules dictate: "Buy Support/Sell Resistance."  
3. The LLM observes the price at the bottom of the range (e.g., 8.01).  
4. The new\_setup\_analysis block forces the execution of the range strategy. Because the regime rule explicitly suspends momentum triggers, the model outputs BUY at the support boundary, transitioning from a passive observer to an active range trader.

### **Resolution of Example 5: Tolerance of Dead Capital**

**Original Failure:** The agent held a negative position in a ranging market, relying lazily on existing static levels to manage the trade.  
**Framework Execution:**

1. In the position\_audit, the model logs current\_pnl\_r: \-0.03 and notes the ranging regime.  
2. The thesis\_validation block requires the explicit calculation of opportunity\_cost\_of\_holding. The model generates text recognizing the opportunity cost: "Capital is locked in a stagnant asset with no momentum, exposing margin to downside risk without upside potential."  
3. Based on the **Dead Capital Tolerance** management trigger in the updated risk constraints, flat or negative PnL in an ADX \< 20 regime requires a CLOSE action. The position is terminated, preserving margin for superior setups.

## **Conclusion**

The "analysis paralysis" and passive default behavior exhibited by the trading agent are not technical glitches or failures of comprehension; they are predictable, structural manifestations of LLM status quo bias interacting with insufficient prompt determinism. Left to their own autoregressive devices within an environment that penalizes risk but ignores opportunity cost, language models will mathematically execute the path of least resistance, which in an unconstrained trading environment is HOLD.  
By radically restructuring the system architecture to incorporate Cognitive Forcing Functions via strict, sequentially ordered JSON schemas, the agent is compelled to evaluate the mathematical reality of a position prior to selecting an action token. Establishing distinct "Management Triggers" to parallel existing entry triggers creates operational parity, explicitly defining the boundaries where inaction is no longer permitted. Furthermore, by forcing the calculation of opportunity cost and mandating adherence to regime-specific behavioral matrices, the LLM is transformed from a highly capable, yet passive, observer of market data into an active, disciplined executor of capital management.  
Applying these prompt modifications, particularly the position\_audit pre-action verification schema, will successfully bridge the cognitive gap between the agent's advanced pattern recognition capabilities and its programmatic obligation to execute. The system will act decisively when technical parameters demand it, optimizing capital efficiency while strictly respecting the overarching risk constraints of the micro-account.

#### **Works cited**

1. The Danger of Overthinking: Examining the Reasoning-Action Dilemma in Agentic Tasks \- arXiv, accessed June 8, 2026, [https://arxiv.org/pdf/2502.08235](https://arxiv.org/pdf/2502.08235)  
2. Snap Out of It: A Dual-Process Approach to Mitigating Overthinking in Language Model Reasoning \- ACL Anthology, accessed June 8, 2026, [https://aclanthology.org/2025.realm-1.16.pdf](https://aclanthology.org/2025.realm-1.16.pdf)  
3. \[2502.08235\] The Danger of Overthinking: Examining the Reasoning-Action Dilemma in Agentic Tasks \- arXiv, accessed June 8, 2026, [https://arxiv.org/abs/2502.08235](https://arxiv.org/abs/2502.08235)  
4. Emulating Aggregate Human Choice Behavior and Biases with GPT Conversational Agents, accessed June 8, 2026, [https://arxiv.org/html/2602.05597v1](https://arxiv.org/html/2602.05597v1)  
5. Can GPT Chatbots Reproduce Human Cognitive Biases in Dialogue, accessed June 8, 2026, [https://co-r-e.com/method/llm-status-quo-bias](https://co-r-e.com/method/llm-status-quo-bias)  
6. A Comprehensive Evaluation of Cognitive Biases in LLMs \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2410.15413v1](https://arxiv.org/html/2410.15413v1)  
7. Emerging Reliance Behaviors in Human-AI Content Grounded Data Generation: The Role of Cognitive Forcing Functions and Hallucinations \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2409.08937v2](https://arxiv.org/html/2409.08937v2)  
8. LLM's are so much better when instructed to be socratic. : r/PromptEngineering \- Reddit, accessed June 8, 2026, [https://www.reddit.com/r/PromptEngineering/comments/1re707k/llms\_are\_so\_much\_better\_when\_instructed\_to\_be/](https://www.reddit.com/r/PromptEngineering/comments/1re707k/llms_are_so_much_better_when_instructed_to_be/)  
9. Large Language Models in Clinical Workflows: A Critical Review of Evidence from Documentation to Decision Support \- InfoScience Trends, accessed June 8, 2026, [https://www.isjtrend.com/article\_243429.html](https://www.isjtrend.com/article_243429.html)  
10. A Tutorial on Cognitive Biases in Agentic AI-Driven 6G Autonomous Networks \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2510.19973v4](https://arxiv.org/html/2510.19973v4)  
11. A Tutorial on Cognitive Biases in Agentic AI-Driven 6G Autonomous Networks \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2510.19973v1](https://arxiv.org/html/2510.19973v1)  
12. StructEval: Benchmarking LLMs' Capabilities to Generate Structural Outputs \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2505.20139v2](https://arxiv.org/html/2505.20139v2)  
13. Structured Output Generation in LLMs: JSON Schema and Grammar-Based Decoding | by Emre Karatas | Medium, accessed June 8, 2026, [https://medium.com/@emrekaratas-ai/structured-output-generation-in-llms-json-schema-and-grammar-based-decoding-6a5c58b698a6](https://medium.com/@emrekaratas-ai/structured-output-generation-in-llms-json-schema-and-grammar-based-decoding-6a5c58b698a6)  
14. the most dangerous failure mode in no-code AI isn't the crash. it's the run that completed successfully with wrong output. \- Reddit, accessed June 8, 2026, [https://www.reddit.com/r/nocode/comments/1t1ndbm/the\_most\_dangerous\_failure\_mode\_in\_nocode\_ai\_isnt/](https://www.reddit.com/r/nocode/comments/1t1ndbm/the_most_dangerous_failure_mode_in_nocode_ai_isnt/)  
15. The Danger of Overthinking: Examining the Reasoning-Action Dilemma in Agentic Tasks \- ETH Zurich Research Collection, accessed June 8, 2026, [https://www.research-collection.ethz.ch/server/api/core/bitstreams/7c5109f0-1a35-49af-9c3e-6240ee22ac5e/content](https://www.research-collection.ethz.ch/server/api/core/bitstreams/7c5109f0-1a35-49af-9c3e-6240ee22ac5e/content)  
16. Can LLMs Simulate Economic Agents? Testing Production Theory with GPT-Based Firm Behavior \- Preprints.org, accessed June 8, 2026, [https://www.preprints.org/manuscript/202602.1648](https://www.preprints.org/manuscript/202602.1648)  
17. AI, Ethics, and Cognitive Bias: An LLM-Based Synthetic Simulation for Education and Research \- MDPI, accessed June 8, 2026, [https://www.mdpi.com/3042-8130/1/1/3](https://www.mdpi.com/3042-8130/1/1/3)  
18. Cognitive Bias in Decision-Making with LLMs \- ACL Anthology, accessed June 8, 2026, [https://aclanthology.org/2024.findings-emnlp.739.pdf](https://aclanthology.org/2024.findings-emnlp.739.pdf)  
19. Prompt Flow Integrity to Prevent Privilege Escalation in LLM Agents \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2503.15547v2](https://arxiv.org/html/2503.15547v2)  
20. Tame Excessive Agency in Your LLMs to Avoid Costly Mistakes \- Galileo AI, accessed June 8, 2026, [https://galileo.ai/blog/prevent-excessive-agency-llms](https://galileo.ai/blog/prevent-excessive-agency-llms)  
21. FinPos: A Position-Aware Trading Agent System for Real Financial Markets \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2510.27251v2](https://arxiv.org/html/2510.27251v2)  
22. Multi-Agent Portfolio Collaboration with OpenAI Agents SDK, accessed June 8, 2026, [https://developers.openai.com/cookbook/examples/agents\_sdk/multi-agent-portfolio-collaboration/multi\_agent\_portfolio\_collaboration](https://developers.openai.com/cookbook/examples/agents_sdk/multi-agent-portfolio-collaboration/multi_agent_portfolio_collaboration)  
23. Organizational Control Layer: Governance Infrastructure at the Execution Boundary of LLM Agent Systems \- arXiv, accessed June 8, 2026, [https://arxiv.org/html/2606.04306v1](https://arxiv.org/html/2606.04306v1)  
24. How to make quality decisions quickly \- Inside Atlassian, accessed June 8, 2026, [https://www.atlassian.com/blog/strategy/how-to-make-quality-decisions](https://www.atlassian.com/blog/strategy/how-to-make-quality-decisions)

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAmwAAAAxCAYAAABnGvUlAAAE20lEQVR4Xu3cW6ht8x4H8D/HkYOTy1HCA5uQdDgnnAi1XXJJJBTiycmlc1M8yEnevLgltyQS5S4e5Ba5F6XkBUUKSSEPJI/4f/d/jNaY/7322vZec++94vOpX3OM/5hzrjH+82F9+/3HnKUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACL2arWCf0gAAArQ8LazbV+rnVZd2z0fa0/Tfb/XOu50l47L7vX+nB4HOX9/1vrD5OxleieWj91Y2fVeqYbAwDYKAlrF9Z6vrTQtpg+jJxY6+5ubLkuHWrqoFo7dmMr0bdl7XD2UK3DurF5SHC+qdYh/QEA4LfryVp/qXVKacFsp9nDa7ppn3ZjV9W6oBtbjnTSHixrB5wzuv2VKkE3czL1WK1durF5uqHW8WXDuo/71DqzbNhrAIAtbFVpYW2UZby3J/vj2IaEkXTqPl+iFnNErR/L7BJrOmsPT/ZXqjHQ7tWNzzPQLmWHWh/Uuqg/MHF4ra9Lm9985u/PHgYAVrJru/101/rlz1tLWwKdurPM9/61LIX2y7H7lg0LbNtPts8rG39+O9faYx212PJslm2zHLrdZGybstAtPLnWN8PYYhL4cp0JraN0O5d6TS/LpAls6wptr5aFIJ7nHr1waL3+WOviYXvbYb+3Z60vS/vMAIA5+08/UNo/9gSGSEj5rLQuzij/8A+c7PfSseuDzrQW82qtT7qx3Fu3uhtbSn//2+aS7uOp3dg4f5GQ9+xkfzHpxk3DWULcC5P99UlQe6/MfjFkKmG4P8eN8VZpAXUx1/UDAMDyJaz1y5WpLJ2ly5bQkS7VLWXhm5vZf3TYnqd8OzU37o/Sbdp7sp8wk/P9f60nSuvy3FFrt9KCSEJEtuNvtY4attP52r+01+XbpidNjp0+PC7XwWU2+O5a2jLyKPfhfVxacBu7btPzytLya8N4rivvlfCT61xKgvEbpd3Htj5f1Tpu2M5neFdp551Oab7AkMB3ZGlzk3nMfP5jeM7ZZeEzT7c1+vnPcvD5pb13v3wOAGyk8Sc00nlZVz09PDfLpOmy/avWO2W22zYvCQA31rqm1j9rPT57eE2QyDknMFxR2pLnK6XdPJ9Kh2pcAk3wSAhKyBvDQx4TkNKFy/MSnvpl3uX4rtYDpQWhd7tjOef8/YSaBKH+vDKWa4nVZfY61yWdyrzm1y775rP7ota9tV4vrROX6/+htL+Z+R/nLWE5j5mj/J37ykIHcAyi/fznvfL8nM+W6nQCAFvYGHCyXJvwlt89+/cwlp+3eLnW32udVuvN4Vg6dAlpkQ5WQsiLw37eb3P9UHC6YAlrt5cWdI4ps+eVcJaQk25Vf52b0kelBbLcP7eq1v3D+NhFG5d1E9hyLjnvLNXmJ2Cm879faV9CibwmX3AAAH6H/lrasulLta4vLRQ8VVrAyRJkluzSkUqnKM85trRuT34r7pJaB9Q6tCzcS3ZlrcuH7U0tYSbdqdyTd26trcvseaWreFtp59tf56aUYHZOWehOZon4f6XNZQJkwmU8Uuvq0pZH8ziGsnH+E4zTtYss/26ueQUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGAufgGGd7Ey5AOnggAAAABJRU5ErkJggg==>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADcAAAAaCAYAAAAT6cSuAAACvklEQVR4Xu2XS6hNURjHP3nkmWdhIN1blLxD3lKkJCIGJBkobwkDknQlA/JKKaF0FfKIgVceecRAJjJw74SxkUwuAxL/n2+tzrbuPpRzbu7R/tevs++31t5rf+t7rH3NChUq9C90UHwR3zO8FsPD+DTRkhn7JjaGsZrQQPFONIfrVB3EebFFdEzG2r1miK/igrkjqXqJK6I+HagFrTNPOX7zNEJcFT3TgfYuIkXEPokJyVjUIrE/NdaCBog3Aa7zdNzcwZrTJPFZnLP8eiMVL9l/Wm84hXOV1htdtntiGyOWW/6mVqxYb0SOCOapWvWGE2RHVhvE2sRWNRGNJ+ZnXLnz7aiYndj/RmetfHa0iXqIh+KZ+VmWarI4IzondlJsqbguVpl/xXQKY8wlSjfEETHHPGLvxU2x2nzdfebP7u23/RQNba+4JXaIbqJOXBTLxHRx2bzBZe8rq/XigxiV2ElTFhma2HHipNhsHtnd4lqw4xhjK8xf9JWYb35OPg42tESME3fF1GBjndtiWPib5/JFBPPEWyvNbRQLw/VvxQsdFh/FHrFG3DM/tAdl5kWxwEsrpfEJsT1cEzGcIN2Jbv/wu9J+/frBgSnigehrvjE0rZ1hHHHNJx9zSefYzXn2fTG3NPXPGiIWm+9IXv1FsWh8URa6Y/75htjRA+E6KzYgrbeGABosmqz0HJwlG1iLdXCMDUJk2FMrfyZXJBaMO8xCL8yjSdRoGpvCGCLytPtH5mk+Xiwwf7Hn5tFjPilJ3ceUHGnuAHYiS4Tj1xNrs4HU8qxgq5pGm+8qtXrKvCEdMq+rieaNBEd5iV2in3kTaDBPX8qAdOU+jpiZ5tHZJk6bHw00kPgv11jz7IgNhEZzTGy11o2uKqKOYndlgS7JWKy1rC3txtxH18yqq+iT2PIOf+5rk4O/UKFChVrpB9Z8crVNKx+DAAAAAElFTkSuQmCC>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACoAAAAaCAYAAADBuc72AAACXUlEQVR4Xu2WzUtVURTFV/RBkiGlZKARJAVBs9BAGgQW6aBoVhDkIMQKkmZCNAgiJBALiyYKURFBSR/0DVFJEKGjBtUg+gsiAqkGSdha7HO5x919T3zJjQd3wY/33tnn3nPu3uvs+4BChapD58gvMhPxjmwK8XbyPYr9JsdCLHc1ks/kY/jutYhcI8fJYhfLVdvJNLkB25TXSnKLbPCBvNULK6s+s7SZ3Ca1PpCnlEFl8gfZ6mKJ9pIzfjBvNZD3AX3P0gXYZv+rWslPcgXZ/lS5b6IK/KkNaqOxP3eTL2SMLInGy0kHUveZgiXH6ygsYWd9QEr8qQlZF0ul/DlM+v3gHNKhfIlsiy0nd8lOH5CUpVewHlqqfw6RHW5c1z2GtbX56CBKV6GZvEUJi60gz8lrWGm8tpERstSNKzOTsBfAfXISs+e0kPPkHjmE9CWhKsQWqwm/75BB8gS2p0wdIV/JFjcuKzwk6924JDt8Ihth2dWGktbWFn6vhlVEh7STrCLjSKtQRx6FmDKsTGf6M5Eyoaf5Rk6Rw+QZrMGvjebFiv3ZBCuZsqzF32B2K7sKm+v9qXVkH2VV/nxAukKsrNaRfWQPsv2ayPtTGdEiWlBZ/YDUZ8riBGzj3p96AFlGKuvPSqWbydPKpEp7CeazA7AHfUHqw9xdsPIq04k/u2FZ1UaVFEkPK3924O+DW7FUHv1BUWaEusJpsp8sg73F5LUTZBSpfWSri6QH9oDqxddJH7lMnpIBsibM/2fJ09pQIi3qT6rs4f/AaJ46S/z2072Sa3VP310KFSq0kPoD2zdnmwufU5AAAAAASUVORK5CYII=>

[image4]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADQAAAAaCAYAAAD43n+tAAACgklEQVR4Xu2WSaiNYRzGHxkyz6FMUaaSISyUJEOmSKKUQhQLiigiSUooUspdXAqJTLEgheJKyZbsbJWVnYUdz+/+36/z9jnnhtOVU99Tv8533ul7/+M5UqVKlSpVqvR/abRZZxab3qW5ltIY88A8MlvMCfPcDMkXtYommo+mXREVjHhnvpoZ2bqWUC9z1Xw2k7KxnWaD6ZHGmtWyRKGepn/2vZ4Gmtvmu1lUmmsoIkAkSDcM6Q5x7mmzIhsjra9l3xuJ+5EtY8sTjUQD+GEOlie6WWTFnvJgHa039/QHzl6tMAjDyiIlSA1E07hgnihSh1RcaZ6aOWnNLLNZUYNtZqpiH8+nFPXJWiLzxTw22zt3xnmrzM00z150SbH/rOLdvKNLjTefzI7S+FLz0Iww/RQNY7JZovAYB28yx8zJzh3xcryOQS/MGsW5s81bRfNBpNErMzJ9x5jDirMGKDotGUMNdZjjCsfi9BuxpWsRJZrCfcXFO8wZMzjN82K64BuFR4eZUWaCIkIU7CDzLD2jXWammaJf02aruaVaw1loPijqhLHhimiW6+eIwmm/JQ7gR5WLFmmWa765br4pLovmKSKBgbnXuRSRwsMYQaeiCRQqIlmIi9ZrSrkjyBJSlLRsSqTce9Vq7Khql2PsjsIByxU/xBhB+9+Y1hReJlq7Fd5/aRaYuWatIr0up/WIlJ2u6IwYi9j/On3uS2N/JTxz3mwzB8xFRZ6jcYrL7TdXFBfn+ZBq/zCmpTU0BS7JeXcVdYchZAa1Rd3wjr3mnOJsIlKkMMazhvOp+6Y11PQtDyrSi5QjSvlzLvble5mn5nLltVMIx+U/7H3U4v8vK1Wq9I/1E1DRYeh315txAAAAAElFTkSuQmCC>

[image5]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACEAAAAaCAYAAAA5WTUBAAABuUlEQVR4Xu2UvStHURjHH5vXJEXCoESiDFYbyURJEWZJsiqD8g9IBoPFRHlJSmIw/MoiVmVSyKSkjMrL99tzD+c+znXvQqn7qU/9fuc599znec65RyTnn1AEJ2GPDfwlHfAJXsIaE/MphZvwFb5H8vcJrI7mjJn4C+yNYomwC0vwTfShqXg4SCd8hsewxMRIBTyCg6Lrp9IOD+C4aCLn8lVVEpzLhOdtIKIB7sEqGwjhujAhWhEr4+L8/xMrogkntZnja5KxC82iGbvK+0UXP4OVbpKBrT6FN7A+HvpkTrRbqTDLRYlXXQ4Lot0Y9cZ92uAjPITFJkY4xsPbZQMhmuCufN//Ifm5G4wzSVYbgt3ZloznYUHCXwJfzASYCF9oSTsP3XBVMpwHdoHtTLoTuBWstiC6RQ53Hu5Fv4AQmc8DJ87YQQ+/Gzysjjp4K3o5lXnjDn5h67DVBiyNcB/W2oCBB5bd8C8kPnMNNyTc7hHRbQ7FYrADvErvUnwQTcLvBhdfhlcSL4LjA3BLwoc5Bh/kAu5ezypvVNcNvmRHdFtm4TS8EE0utEW/BitvgcOwTzJUn5OTk5OVD1HsZfG69x6pAAAAAElFTkSuQmCC>

[image6]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAAAZCAYAAACB6CjhAAACf0lEQVR4Xu2XO2gVQRSGf4mCQQOCgpFEjA+0ERs1NiksIihilZBYiAZEK2sfhSiIiBBE8AU+0NiILyTEkIiCIjY2YmElWCiKIKiVIojo/3N2uHMnkzW72Qsp5oOPvZnZu5lzdubMXCCRSCQaQxPdTi9m9tLmujvizKW76drss57TTvfTjtptM5s59Bw9RlfSffQHfUOXeffFWEBf0r+Bg7DnNgRlWlbFVvqEtnltu2CBXKWzvfaQ+XQUlqx39AbdRGd591TOCvqYnqYLg74yHEItWIem8UdYUIu99hAlYIguCTsiLEX+rJiHAvEow+voGL1Gl9d3F2IDfQ1btw4F9D4zL7giCdCsOot4ElrpA9oZdkyFNfRupj5XQRf9DRtU3nJTAm7TM7Bl8IkOI14A9dIOwOqNn4RpBe+jWaDZ8BzTW4ca3HX6Hf8flBLwiPbD/p88Qd8iXkDDJFQWvI+m4wWUT0QP/Uy7w44IenZLdnVspD/pca/NxyXhDh1BxcE7VEwu0ReIT8fJ0GBewYIoy3rYNqqdRYUthgqi6s5NxGtCafT2z9OnKP72FbwStir7W9ufCpf2+snQTqRascVrcwl4BlsiIdphxmHjG8DEmlAKrf8r9CFsdygSuNB6vZddHYvoZdgUFzrldaC+KA7RP6hPgFsCqiPhOFzwbtqrfwAlk6AvV7EVqhCpXnyjHzy/wNapOwjthZ0XbnltO+lh1ALV9QjiBVTB6wVp2/UpnAR9QdNH01zTfSp7cB7uIBTzpHffDvqLHkQtYA1YBfc+3QNb01/ptqzf5ygmBu/Q8/T7oy/siKGHn0KBU1OD0eBXwwLYjPxzQyKRSCQSiXL8AzZHcGefZBSgAAAAAElFTkSuQmCC>

[image7]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACoAAAAZCAYAAABHLbxYAAABp0lEQVR4Xu2VTStFQRzG/0J5i6IUG0ehLJRiQ9lJWSgbpaxtLMgCJfkCvgErC2XvJTbWd6VYKnkJn4CFhXieZiZz5o5zzOneBc2vft1zZubc85wz/zMjEon8PRI47zZmMATnYCesgS1wAi7YgyrFIFyCF/AD7qe7M+FDfTo+wmF7UKVg0Fk4Dp8kLOgMvNWW4ApsTY2oAl3wQcKDbriNHlgWif79iQ7Y7Db6qGbQOrgJV8UflrN6AnvcDh9Fg3L8majpv4OLsNYepKmHO3BN0mGDQpKiQfkRtuvzPlF1vi7+N+eGDQ5JigRt0Bp48wNRYXutdhsTdk8KhCRFgvrg9Vympt0Oi1H4ArfE/+YzCQ06AJ/hEWy02k1QloUPrrHnsB9uS3nN5pIXtA12y/efjsA3SQc1U8929ruYkGa6WQbBYU1Q3si9iGvcFXyHY7qNwTk20eeEx/z6d0WFsGHIYymvyV+HnRRV/Nw+zTb4Cq9F7eWEe/gpvJH0jVhrl6LWyGV4Dw9FPYRNk6gPyA1p4HLG67k7Vg3uJlOituFEct5KJBKJ/CO+AAJOUupxraxKAAAAAElFTkSuQmCC>

[image8]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABMAAAAaCAYAAABVX2cEAAABHElEQVR4XmNgGAWUAkcgfg3E/6F4BxBzIsnzAfEuJHkQXgfE3EhqUAAjEM8C4l9A/BOILVGlwSAIiNcwoFqEFQgC8UIgzmeA2DyFAWIBMigC4mg0MaxAH4j7gVgSiK8D8RMgVkSSZwHi2VB1BAHIxnQou4EB4rocuCwDgwgDxOUgHxAEfUBsDGXrAPF7ID4BxPxQMRsgngxl4wWw8ALZDgIgLy0H4n9A7AEVA7mapPBCDnCQISDDQIaCYo+s8IIBkPdA3gR514mByPACuQYUFqboEkAQwwCJiGtA3IkmhxWghxcyEGeAJBOQgUSFF8gLoKzBhS4BBQ1A/BaINdHEUYALEH9hQOQ1UBbyRlEBAaBkAsqrBMNrFIyCIQMA260zNBT6yKgAAAAASUVORK5CYII=>

[image9]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAaCAYAAADxNd/XAAACI0lEQVR4Xu2WT0gVURTGv9BAIUEoMCnIP0EE4cb+bGohJCihG91pq6j2RYa0qE2IW9OCVNqb4EZwYaCoCG6iheKqhRIEQbqqhRL1fZ25z/uuM/LGxjbOD34wc8/w3j33nntmgJycnKOijN6hryO7aWXRE3u00K/0t+c3ukN/0RXaBfvN/8JJ+oo+p430Pv1BV+kF77mQMbpLb3pjmvQDWCJ99IQXK1ARmRVt9AM95431wlZWkyz3xh1VdJF+pjVBrJZuJMT+0kBn6SA9HcQOw1PsTdZxnn5B8iQu0+90EvsTvEZ/0jV6JogV0NY00Rk6TuuLw6m4Sj/Btt7hVlHqOqQTlvTDMEBewGKPg/FELtH3kbrOAtW16nsK8eU6hP31r7N0D7YzT6L7VGgXtBsL9AYSDlAJ6I/f0W16PYiJU3Qe1nWWo+t12Kq/QQZlrS0fweETUQtUm7wdBiLi6l//0Q/rPq3R2D+hVdBqLNG64tCBaMU/wg5iEq7+HwXjzbD26zeD1Gj1h+kc0q++Jq+EL0b3Wl210+rCE0Zc/YseWGIvg/GSUP2P0mlYd0ozcaEXlkrCf3GpBb6F9XzHQf1fiSkBteWSyKqVnoWdly266anPgwkU9/krsMMd9n9d61k/gWdIOEeauMpDZaJyievTaXAvsjhdOdyCvRP8mBLUeXDo8OsQK5G7sIqI/Z5qpwPIoF0dASqrDlgnip18Tk5OTs7x5Q/wGHQ5wzPiOQAAAABJRU5ErkJggg==>

[image10]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABoAAAAaCAYAAACpSkzOAAABY0lEQVR4Xu2UTysFURiH3xuKm0gpKXXvlZSFbKxkaS3dLJQPoCxtfA9RVmJhpyxsJWUjdkpKLJQSZSMUEs87Z8515szM7ZQs1Dz11PS+Z+Y3c/6MSMF/pIpzftGhjNv4jl+xev0QX9/jGg7YG1xGcBEP8BO3ku1MxvAJd7DVqWvAMV5ixalHaNAMTuCthAXVxXzBkt+AZcnvRfTjjYQFreAHTnr1kpip1aAFr9cgNKgTD/Ece5MtqYmZlQvJWSclNEin+lHS66Mh+3iCw049RWjQtJipuRLzZUf4inc4hS2NkTmEBmWtTxWvcQ87nHomIUF2ffShfclWdJ9ued36TQkJylufHjwVM32DTj2TkKC882NfQO/X5zTFBuk5KHk9RWurkl4fRQ/7m/wE1XAd291BulN07+vvx/6/nvEMR8UM3oxr7v9tF7vE0C1ma7/gPG6Iee6f0IbjOItDXq+goOCXfAPjzFbLtIOBUgAAAABJRU5ErkJggg==>

[image11]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADAAAAAZCAYAAAB3oa15AAABwElEQVR4Xu2VTShFQRTHj1Akko98kyhlYeWjZEMWysKChYWFDUlKWbBRStmwUU8WIjsLlCULpbBQSha2CilFsbKy4H+aO96ZeV/evc9Xza9+3XfnzH1zZu6ZuUQOh+O7SIe9cNVzAGYbPf4wmTAE52AdHIGv8ArWiH6/Qg6pBOPRAw9hhWgbgu9wHWaI9h+jAe7ALVhsxWxmKJysphLew2tYItp5QeS9TRosJ1WSScMPt8EjuAKrzHBMmuElHBVtZfDWk39reDG2ST1jw+MPkyrHRG/dgGfbBU/gEiw0w77ogG9wD2ZZMX47B7BVtPlKnhPvg2dwFuaZYd9wApvwhcwkJXISSSfPnQbhBZwkVZeppB8+wG47YKEnweX65eSZTngDxyj1ZzWvKC9Mix2IAq/8FHyC7VYsIfItTFNqyoeTP4X13j0fn3yc5n/2CMPJT5Ba+WpSeyVWucVF74NzCraB+YO16101RXAN5oo2Riavy6aUAkyCsY9QefQlggc/hs/wTvhI6siUHzIeZxwuU2TNB54EwwM0wX24AWvNcFT0hyyaC6If0wjnKTJ5TQFc9K6B4eT5z3RNOxwOh+N/8wF8eUjsoLZBCgAAAABJRU5ErkJggg==>

[image12]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACoAAAAaCAYAAADBuc72AAACa0lEQVR4Xu2VTYjNURjGH2mKfOUjnwtDolmIQrIwWWjKghRK2Nqyo2YnSVnZSNlZKKVsRDITV7MhpWiGUmJEVqxYkK/n8Z7Xff/n/s91Jyv1f+rXvfc9557znHPe8x6gUaP/RzPJIXKJnCPrqs1FrScHyGIyjcwm28nh2ClpEblPvpOfiU/kY/r+mpwic1P/Ds0jI+Q0bKKN5BnZFzsVdBDtSZ03sDFK2gXrdyaLbyCTZBTmqUMnySMyP8S0I8/JkhCr027yMvGAHEeXHUkahhmV4VyXUWiTOZlUh6gtsGPZk8VzyagW2qtmkJvkPVmdtc0hY+QH2Zm1YYB8QKfRTeQzOo8n11SNroDloszKdNQ28oXcgaVgRW6oZDSP55JR9bkNO/5X5CiZHjsFaae0Y3EDdAn9XlwnS0PbH2ki5URuaCpG75IF6fca8pacgBnIpd3XfE9Jizwk38g4rILU/ee3/Abmhno1quOLR6iJrsDMrgpxqZSfW2FznUcXoyVDpXgv0n/qbq7np8rPrBBXPrZgqVOsMlqZVpgbcqMqJSWtJe/IDdiD4XKjSououvyU3ENeIivy1eS3UIN+TZ8uFeHlaB+PLyYa9aNXXO1RpfrpC2ih5rZHHYG9Jp5TmkyvlAq4vxALyRNY+VAZkdQmU/3pt6TvOkI9xX0hroWoMuT5Kfnr1oIZ1fhnYweXBrxI7pG9MJMTqD6DGuAWeUFWhvhm8hi2W8dgOXgV1QXqedbp+BOrh+QC2iVM42ncSdj812D1vVbaReXcfjKI6m78TboYQ7BJ+tHl5naR0m4HbIxl1aZGjRo1avSv+gXDII0F8c0ztwAAAABJRU5ErkJggg==>

[image13]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAAAaCAYAAAAHfFpPAAACmElEQVR4Xu2WTchNQRjH/0L5/oiSUi6JlFj4KllYsLBgIaEUC8XewseKspCSIqKElxJCEpIob1j42JISC+UjCiXkI/H/v88ZZ86cey5z7n2t5le/7mnm3Ln3eWaemQESiUSid+lLV9BG0F7FALqGTsue9f1xdD3KYwyiJ+l3+itTz2+z5zf0AOz7/5WBdAndS1/Sz3Rm4Y1qRtB7yANy7qb9vfd8ZtCP9Bzt57Ur8Lv0CR3vtZdQpmWnUAIW0wV0G+ISMIReoQ/pM9pF59I+3jshy2BJ2hh2kM2o7vvDRHqd7qKjgr520R+ITcBxOjbsaME++oPOD9qVNJWIErAh6Cuhl6fTq/QInVDsrk1vJ0Dvd9NHdHSxqyeGF/QxIveBKfRspp7boU4CztA9sDLQHnIR5Q3QMZW+Q7n+FfwNep9O9tqj0CBaDbfw9zqsok4CrtGVsN+TO1C9kS2FLfGnsJVwm36hr+lC2CnSNlqOOk7qJCI2ARp7aPbpmA0LarvX5mhW/w3YBnoJtiF3BG2OB+kdVC/HZsQmoBn6rsbQkh7stbv6V7BjvHahfURHo47IttDs76c3ET/7IjYBOok0o4u8NpeAbljQjqr6H0kfwMpAJ1wtVP+H6WXY6RAbuKNVAlSfDRTvIJq5nygmwJXAMRT/R9X57xLzHP9+mvSgwTt9FCoB+vMKImQdLIBTyGdwFd2CPFB9bqUf6JyszbVrZYb1L+bRb8gToDgOocVlT4NpeWuZa9CorDVBdXqavkfxOvsKdrw5dF3+SjchD1jXXW245+laegI2m7pZCgXRRT+heP+/QIdl7wyH7RdaeavpUdiJUIkG34nO3wLromTo7F4Ou05XzlwLlMhZsDEmBX2JRCKRSCQS+A3Bvo+43QtFqAAAAABJRU5ErkJggg==>

[image14]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAF8AAAAZCAYAAABXTfKEAAAEjElEQVR4Xu2YechtUxjGH6G4xriZyzGWIsoUUfcPRCKZ6woRiiJkzJirTIl0jbdMydy9MkSkL/yBPwwlf5AMGUqhFJHE8/Pu9Z111tl7n33OnUr7qadzvr3X2Xut533X877rk3r06LH6sbm5pblOeSPD+uZG5cUeswOxLzVfMG8zH1C9wBuaS81jyxv/d2xrXms+aN5o7jp6uxF7mSeZWylE3tg81FycjdnX/NTcuRpzk/mdeYG5o7mD4hkfmveZ68XP1ijWNY827614oiIZJmED83Rzz+o7z2E955qD4bBmHGC+rhBtb/Nl8x9FtrZZBDhVMTbnN+Y+2ZhLzLfNTaq/DzEvM/dQ/J5AnWyuUEx8TQOru8e83tzFPMf8zfxEkRxtwErf07gGdyie2wqiy6LPUkQN4MvvKyZA1rbhGPOLiu+aF5mbjoyQHjXnFLsC8EwWm0CAb9Xas5sjzTfM7bNrpylEXKb2nciaSFYChQaPmAdqctL+B+zma/NXRdYnXK14OVnbBsS/orxYgGfNaSj+/uaS+bvS4eZdal9kCRY3qD6bQBLV1ZYSzD8JncAO/FYh6NbZ9RKsieRCx6nB1rjbfE2jD0gTmiRsF/EXmZ9raCnna5jl1IpnsntdQaAI6sWqDwCWRkZOsg2wn/mRwqcTUlLCNmFXSvw6sLDnzL8VwrUB8Xn5q4os+VLhmcnCAAGmkD6rKLJPKSbNe8j4We2G596uqB95AKYRvgnUpb/M5YpC2gTW8bR5p8J6aCTo6gbZmKmAZ+H3dD6Tigbiv2luUf1Nl8R2vVyjgvAdW8NiUheB6MluuM97WQQeXJfNdSgDsCqE55kPm78ompE2ID6ucYri/ZBu7jPNMIfNFMXncXXzS7Iizwxe/oQiADtl10tgM89Xn+AExXt3M68xj6+ud0EKwENaeeEBc/nBPKy8UQPWSxeXJws17XfzhuzaRLCI+xXZ16XHbQI2RL04qrxRobQbAk6nlM4GFDjmkQp0F+Db3ysC13XX1IFM/0Ah4Kygm8M5SKYuCTwv/FUa+jVb+Ij5EePYXeFxL2o0WEl8LKkOud0A7IhuK41HvFvUvYhxpmDrs2uu03gN6AqEf0fDAybzo+Wkl28CLTK1ATtNSOLPqUMCMVEOVGXncJ5Gtz8Zup2GY9JLcvGT7TSdEUq7Aek5ebAQv4t9JOHTWJJolgDwe5qM/J0LFXUvHQ5JyoFGbZZEozHJxU+2Q91onQM3z1QMxqc5nSb+rKj6gJ75Y/NP86DqGsFA6EH1N+A7XU9dsU52k08UpJ46iU+20B2x+DYg/EsaD9K0AdjGfEux3nz9Pyra4LRDz1bs6Ceza5zQr9TwPXziHl2K9Xw/y0NLUnT4fwxAkFc0XsXxWjySnvtC8ytFK0lgSmA3bNNSEBayLLvHvzmWVN+bsEBRZEvhE8hS5nNweaMG6UxTx5uzcSTHHxrt5Aj0UsVuPsN8zPxJzfVulYOiQm04TpH5daKxcxCYzzrQqpJRWAg2Nu2ha22C9VL/+GfcIrWfC3r06NGjR48ePXr0WN34Fx+C3E3oziR4AAAAAElFTkSuQmCC>