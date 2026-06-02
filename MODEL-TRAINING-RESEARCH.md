# Gemini Deep Research: Fastest Way to Train Savant's Model Using Its Existing Memory & Sandbox Systems

## Research Objective

Determine the most effective and fastest closed-loop training methodology for an LLM-based autonomous crypto trading agent ("Savant") that already has a 4-tier memory system, a 50-scenario sandbox with 3-tier grading, experience replay, Brier score calibration, CUSUM edge decay detection, and a GEPA-style SOUL.md mutation feedback loop — but has NOT yet connected these systems into a cohesive improvement pipeline. The goal is to make the agent measurably smarter with every trading cycle and every sandbox run.

---

## What Already Exists (Fully Implemented)

### 4-Tier Memory Hierarchy (`src/memory/`)

| Tier | Component | Storage | Status |
|------|-----------|---------|--------|
| 1. Working Memory | Current evaluation cycle prompt | In-memory | COMPLETE |
| 2. Core Memory | SOUL.md (879 lines) + 2,959 knowledge units | Static files | COMPLETE |
| 3. Episodic Memory | SQLite WAL ledger — every decision + market snapshot | `agent_episodes`, `episode_market_context`, `episode_cognitive_state` tables | COMPLETE |
| 4. Semantic Memory | `semantic_patterns`, `edge_decay_alerts`, `experience_replay_lessons` tables | SQLite | COMPLETE |

**Episodic Memory captures per decision:**

- Execution data: pair, action, side, entry, stop, TP1, confidence, reasoning, planned R:R
- Market context: regime, session, funding rate, fear/greed, order book imbalance, MVRV, SOPR, NVT, ATR, ADX, RSI, condition tags
- Cognitive state: knowledge units used, thesis summary, invalidation reasoning, confidence score, system prompt version
- Outcome (filled later): PnL, PnL%, is_win, achieved R:R, status

**Semantic Memory includes:**

- `semantic_patterns` table: pattern_id, category, condition_value, sample_size, win_rate, avg_pnl, avg_rr, profit_factor, is_valid, confidence_penalty
- `edge_decay_alerts` table: CUSUM-detected shifts in strategy performance
- `experience_replay_lessons` table: lessons from HIGH conviction losses and missed opportunities

### Brier Score Calibration (`src/memory/calibration.rs`)

- Decomposes into reliability, resolution, uncertainty
- Maps conviction levels to numeric probabilities (HIGH=0.75, MEDIUM=0.50, LOW=0.25)
- Progressive confidence caps: <25 trades = LOW only, <50 trades = MEDIUM if WR>50%, 50+ = HIGH allowed
- Confidence penalty from Brier: 0.0 (excellent) to 0.5 (severe)

### CUSUM Edge Decay Detection (`src/memory/cusum.rs`)

- Control chart tracking R:R deviation from target (1.5)
- Detects persistent performance shifts earlier than moving averages
- Alerts: PositiveShift (edge improving), NegativeShift (edge decaying)
- Default parameters: target=1.5, allowance=0.5, threshold=5.0

### Experience Replay (`src/memory/replay.rs`)

- Queries HIGH conviction losses and missed opportunities (Hold decisions where price hit TP1)
- Designed for weekend retrospective analysis
- Generates single-sentence heuristic lessons stored in `experience_replay_lessons`
- 50 lessons max, tracked with `applied_count`

### Memory Context Injection (`src/memory/context.rs`)

- Queries episodic memory at decision time
- Formats into "Dynamic Memory Context" prompt section
- Includes: win rate by regime, win rate by pair, total trades, recent episode summaries, operator rules, Brier score, CUSUM alerts, confidence penalty
- Only activates after 5+ closed trades

### Sandbox Testing System (`src/sandbox/`)

| Component | File | Purpose |
|-----------|------|---------|
| Scenarios | `scenarios.rs` | 50+ curated market scenarios across 11 categories |
| Generator | `generator.rs` | GARCH(1,1) synthetic OHLCV with configurable trend/volatility/events |
| Mock API | `mock.rs` | Scenario-specific Fear/Greed, funding, MVRV, SOPR, news |
| Order Book Sim | `lob.rs` | Hawkes process order book with dynamic liquidity |
| Simulator | `simulator.rs` | Forward trade simulation with slippage, fees, partial TP fills, MFE/MAE |
| Grader | `grader.rs` | 3-tier: Compliance (binary) → R:R (0-1) → Reasoning quality (0-1) |
| Harness | `harness.rs` | Parallel execution, multi-run α-trimmed mean, regression detection |
| Feedback | `feedback.rs` | GEPA-style failure analysis → SOUL.md mutation proposals |
| Report | `report.rs` + `run_report.rs` | Markdown report cards, category breakdowns, equity curves |
| Schema | `schema.rs` | SQLite: soul_versions, scenario_catalog, evaluation_runs, agent_decisions, rubric_scores |

**Grading Details:**

- Tier 1 (Compliance): Binary pass/fail. Checks: action not empty, stop loss present, valid entry, confidence 0-1, reasoning >20 chars, no missed trades
- Tier 2 (R:R): 0.0-1.0. R:R ≥3.0→1.0, ≥2.0→0.8, ≥1.5→0.6, ≥1.0→0.3, else 0.0. Hold=0.5 neutral, Hold when trade expected=0.0
- Tier 3 (Reasoning): 0.0-1.0. Checks for: regime classification (+0.15), substantive thesis (+0.15), specific price levels (+0.15), risk management (+0.15), session awareness (+0.1), sentiment/on-chain (+0.1), volume/OI (+0.1). Penalties: generic reasoning when trade expected (×0.5), no data references (-0.15)
- Total: Tier 1 fail → 0.0. Otherwise: T2×0.4 + T3×0.6
- Multi-run: α-trimmed mean (20% trim) across 3-5 runs per scenario
- Regression detection: alerts if score drops >threshold vs rolling average

**Scenario Categories (50+):**

Trend Bull (5), Trend Bear (5), Range Bound (5), Volatility (5), Catalyst (5), Session (5), Correlation (5), Sentiment/On-Chain (5+5), Edge Cases (5), Microstructure (5), Extended On-Chain (3)

### Agent Architecture (`src/agent/`)

**Prompt Composition (5+1 layers):**

1. SOUL.md (879 lines, loaded via `include_str!`)
2. Risk constraints
3. Strategy knowledge
4. Knowledge injection (MMR-selected from 2,959 units, 8K token budget)
5. Output format (JSON schema)
6. Dynamic Memory Context (injected into user message after 5+ trades)

**LLM:** mimo v2.5 pro via OpenGateway (free, unlimited, 1M context window)
**Decision cycle:** ~7.5 min for 15 pairs (parallel), 5-minute candles
**Autonomy levels:** Suggest → Confirm → Autonomous

### What's MISSING (The Gap)

The systems exist in isolation. There is NO:

1. **Automated sandbox→memory feedback loop** — sandbox results don't feed into semantic_patterns
2. **SOUL.md auto-mutation** — feedback.rs generates proposals but nothing applies them
3. **Continuous training schedule** — no automated "run sandbox, analyze, improve, re-test" cycle
4. **Performance correlation** — sandbox scores aren't tracked against live trading performance
5. **Knowledge unit pruning/promotion** — units aren't scored based on actual trade outcomes
6. **Confidence calibration loop** — Brier scores are calculated but not fed back into conviction limits
7. **Anti-pattern injection** — detected losing patterns aren't injected as negative knowledge
8. **Scenario expansion** — scenarios are static, not generated from actual trading failures

---

## Core Research Questions

### 1. Closed-Loop Training Architecture

The sandbox has grading, the memory has pattern extraction, and the feedback system generates SOUL.md mutation proposals. But these are disconnected.

**Question:** What is the optimal architecture for connecting sandbox → memory → SOUL.md → re-test into an automated improvement loop?

Specifically:

- Should sandbox runs automatically update `semantic_patterns` with win rates per scenario category?
- Should the GEPA mutation proposals from `feedback.rs` be auto-applied to SOUL.md or require human approval?
- How do you version SOUL.md changes and track which version produced which sandbox scores?
- Should the loop be: sandbox → analyze → mutate SOUL.md → re-sandbox → compare → keep/revert? Or a different cycle?
- What's the minimum viable loop that produces measurable improvement fastest?

### 2. Training Velocity — What Produces the Fastest Improvement?

The agent currently has:

- 50 sandbox scenarios (can expand)
- Non-deterministic LLM (same scenario, different responses)
- Multi-run α-trimmed mean for reliability

**Question:** What training methodology produces the fastest measurable improvement in sandbox scores?

Research areas:

- **Curriculum learning:** Should scenarios be ordered Easy→Medium→Hard→Extreme? Or mixed?
- **Targeted remediation:** If the agent fails 80% of On-Chain scenarios, should we run 100 On-Chain variants before moving on?
- **Adversarial training:** Should we generate scenarios specifically designed to exploit detected weaknesses?
- **Frequency:** How often should sandbox runs happen? Every SOUL.md change? Daily? After N live trades?
- **Sample size:** With non-deterministic LLM output, how many runs per scenario needed for statistical significance? (Currently 3-5 with α-trimmed mean)
- **Parallel vs sequential:** Should all 50 scenarios run, or should we focus on failing categories?

### 3. Memory-Driven Learning — Making the Agent Learn From Its Own History

The episodic memory captures everything. The semantic memory tables exist. But there's no extraction pipeline.

**Question:** What is the most effective way to extract actionable patterns from episodic memory and inject them into the agent's decision-making?

Specifically:

- How should `semantic_patterns` be populated? (Automated SQL queries on `agent_episodes`? Periodic batch jobs? Real-time updates?)
- What pattern categories matter most? (Win rate by regime×session, by conviction level, by funding rate range, by pair, by R:R planned vs achieved)
- Minimum sample sizes for statistical significance per pattern category?
- How should extracted patterns be formatted for LLM consumption? (Table? Narrative? Confidence intervals?)
- Should patterns replace, supplement, or override the static knowledge units?
- How do you handle pattern decay (a pattern that worked in January but not March)?
- Should the agent see its own Brier score and CUSUM status at decision time?

### 4. SOUL.md Evolution Strategy

SOUL.md is the agent's identity (879 lines). The feedback system can propose mutations. But uncontrolled mutation could destroy what works.

**Question:** What is the safest and most effective SOUL.md evolution strategy?

Research areas:

- **A/B testing:** How to compare SOUL.md v1 vs v2 on the same 50 scenarios?
- **Mutation constraints:** What sections of SOUL.md should be mutable vs immutable? (Identity invariants vs tactical rules)
- **Rollback:** If sandbox scores drop after a mutation, how to automatically revert?
- **Incremental vs radical:** Should changes be small (adjust one threshold) or large (rewrite a section)?
- **Human-in-the-loop:** When should mutations require Spencer's approval vs auto-apply?
- **Version control:** How to track which SOUL.md version produced which live trading results?

### 5. Experience Replay Optimization

The replay system queries HIGH conviction losses and missed opportunities, then generates heuristic lessons.

**Question:** What is the most effective experience replay methodology for an LLM trading agent?

Specifically:

- When should replay sessions run? (Weekend? After every N trades? After drawdown events?)
- How should lessons be formatted for maximum impact on future decisions?
- Should lessons be injected into the system prompt, user message, or a dedicated "Lessons Learned" section?
- How many lessons can the LLM effectively use before context dilution?
- Should lessons be pair-specific or global?
- How do you measure if a lesson actually improved future decisions?
- Should the agent generate lessons itself (self-reflection) or should a separate "teacher" LLM generate them?

### 6. Confidence Calibration Loop

Brier scores are calculated. Progressive confidence caps exist. But they're not connected to live decision-making.

**Question:** How should the confidence calibration feedback loop work end-to-end?

Specifically:

- Should the agent's max conviction level be automatically adjusted based on Brier score?
- How often should Brier scores be recalculated? (Every trade? Daily? Weekly?)
- Should the agent see its Brier score at decision time? (Could cause under-confidence)
- How do you handle the cold-start problem (<25 trades where LOW is the only allowed conviction)?
- Should calibration be per-pair, per-regime, or global?
- How do you prevent the agent from becoming permanently under-confident after a losing streak?

### 7. Knowledge Unit Lifecycle Management

2,959 knowledge units from 10 JSON files. Selected via MMR based on market conditions and context tags.

**Question:** How should knowledge units be pruned, promoted, and evolved based on trading outcomes?

Specifically:

- Should units that correlate with winning trades get higher priority scores?
- Should units that correlate with losing trades be deprioritized or removed?
- How do you measure the causal impact of a specific knowledge unit on a trade decision?
- Should the agent be able to generate NEW knowledge units from its own experience?
- How do you handle knowledge units that conflict with learned patterns?
- Should the knowledge selection algorithm (currently MMR with condition×2 + priority + tag×3 weights) be tuned based on outcomes?

### 8. Anti-Pattern Detection and Injection

The agent should learn what NOT to do, not just what to do.

**Question:** How should the system detect and inject anti-patterns (conditions where the agent should NOT trade)?

Specifically:

- How do you detect anti-patterns from episodic memory? (e.g., "When funding > 0.05% AND RSI > 70 AND Asian session → 80% loss rate")
- How should anti-patterns be formatted and injected into the prompt?
- Should anti-patterns override the Action Triggers (Section XIII of SOUL.md)?
- How do you prevent anti-patterns from being too conservative (blocking all trades)?
- Minimum sample size for an anti-pattern to be valid?

### 9. Scenario Expansion From Live Failures

The 50 sandbox scenarios are static. But the agent will encounter situations in live trading that aren't covered.

**Question:** How should the sandbox scenario library expand based on live trading failures?

Specifically:

- When the agent loses a live trade, should that market state be captured as a new scenario?
- How do you convert a live trading failure into a replayable sandbox scenario?
- Should scenarios be auto-generated from episodic memory entries where the agent performed poorly?
- How do you prevent scenario library bloat (thousands of scenarios)?
- Should there be a "personalized" scenario set based on the agent's specific weaknesses?

### 10. Measuring Training Effectiveness

Without proper measurement, we can't know if training is working.

**Question:** What metrics should track training effectiveness, and what are the target benchmarks?

Specifically:

- What's the primary metric? (Sandbox average score? Weighted score? Compliance ratio? Category-specific scores?)
- What's a good initial score? What's the target after 10 training iterations?
- How do you distinguish genuine improvement from overfitting to the test set?
- Should there be a held-out validation set of scenarios?
- How do you correlate sandbox scores with live trading performance?
- What's the expected training curve? (Fast initial gains then diminishing returns? Linear? Step functions?)

---

## Technical Context

### Rust Implementation Details

All code is Rust 2021 edition, async with tokio. Key patterns:

- SQLite via sqlx (WAL mode, 16 connections for episodic memory)
- All memory operations are async
- SharedEngineData uses `Arc<RwLock<>>` for concurrent access
- Sandbox runs use semaphore-capped parallelism (5 concurrent LLM calls)
- LLM calls go through curl subprocess (reqwest TLS issues on Windows)

### LLM Constraints

- Model: mimo v2.5 pro (1M context window)
- Provider: OpenGateway (free, unlimited)
- Current prompt size: ~13K chars (system + user message)
- Memory budget available: ~987K tokens unused
- Non-deterministic: same prompt can produce different responses
- Response format: JSON with action, pair, side, entry, stop, TP1/2/3, confidence, reasoning

### Data Pipeline

- 5-minute candles from Kraken REST API
- Indicators: EMA(9/21), RSI(14), ATR(14), ADX(14), VWAP, Volume Profile
- Insight: Fear & Greed (alternative.me), funding rates (Kraken Futures), on-chain (CoinMetrics/CoinGecko), RSS (15 feeds)
- Order book: 10 levels bid/ask from Kraken

### Existing Training Infrastructure

What's already wired:

- `EpisodicMemory::capture_episode()` — captures every decision
- `EpisodicMemory::update_outcome()` — fills PnL after trade closes
- `semantic_patterns` table — ready for pattern storage but not populated
- `experience_replay_lessons` table — ready for lessons but not populated
- `SandboxDb` — stores all evaluation runs with version tracking
- `analyze_failures()` → `generate_mutations()` — produces SOUL.md change proposals
- `check_regression()` — detects score drops across runs
- `alpha_trimmed_mean()` — reliable multi-run scoring
- `always_hold_benchmark()` — baseline comparison

What's NOT wired:

- Sandbox results → semantic_patterns population
- Semantic patterns → memory context injection
- Brier score → conviction level adjustment
- CUSUM alerts → SOUL.md rule changes
- Experience replay → automated lesson generation
- Feedback mutations → SOUL.md auto-update
- Live trade failures → new sandbox scenarios

---

## What I Need

1. **Prioritized training pipeline** — step-by-step architecture for the fastest path from current state to measurable improvement
2. **Specific SQL queries** — for extracting patterns from episodic memory into semantic_patterns
3. **Prompt engineering** — how to format memory context, patterns, lessons, and anti-patterns for maximum LLM impact
4. **Training schedule** — how often to run each component (sandbox, replay, calibration, pattern extraction)
5. **Measurement framework** — what metrics to track, target benchmarks, overfitting detection
6. **SOUL.md evolution protocol** — safe mutation strategy with rollback
7. **Cold-start strategy** — what to do before enough trades for statistical significance
8. **Research citations** — academic papers, industry best practices, proven frameworks for LLM agent self-improvement

---

## Constraints

- Must work with Rust + SQLite + tokio async
- Must not add >500ms to decision latency
- Must be compatible with SOUL.md (879 lines) and 2,959 knowledge units
- Must handle LLM non-determinism
- Must be transparent and auditable (Spencer can see what the agent learned)
- Must not overfit to sandbox scenarios
- Must not cause the agent to become permanently over-conservative
- Must leverage existing infrastructure (don't rebuild what exists)
- Training pipeline should be runnable as a CLI command
- Full training cycle should complete in <2 hours
