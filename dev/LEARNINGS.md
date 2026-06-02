# LEARNINGS

## Session 2026-0601-1955: Protocol v0.1.0, Training Pipeline, Closed-Loop Workflow

**Key Learnings:**

- Protocol synced to v0.1.0 from GitHub (was v0.0.2). 5 versions behind. Always check GitHub for latest before starting a session.
- `/dev` folder renamed: `findings` → `fids`, `archived` → `archive`, removed `baselines`/`plans`. Protocol alignment is not optional.
- Closed FIDs must be auto-archived per ECHO Protocol. 5 FIDs (007-011) were closed but sitting in `dev/fids/`. Moved to `dev/fids/archive/`.
- Vault subfolders (Insight, Lessons, Risk, Sessions, Trades) are empty. VaultWriter has the functions but they're never called from training pipeline. Must wire.
- External audit agents flagged: short bias (agent only shorts), 0-25% confidence bucket at 18% accuracy (noise), knowledge utility drop. All real issues.
- Confidence floor (40%) is the single highest-impact one-line fix. Removing bad trades improves edge more than adding good trades.
- Training data bias produces agent bias. If all scenarios are bull euphoria, the agent learns to always short.
- `docs/TRAINING-WORKFLOW.md` formalized the closed-loop cycle: TRAIN → AUDIT → IDENTIFY → FIX → RETRAIN. Every agent session reads this before running training.
- 136 tests passing, zero clippy warnings. Build clean.

**Agent Behavior:**

- Brier trajectory: 0.30 → 0.28 → 0.29 → 0.24 → 0.24 → 0.24. Converging around 0.24.
- 50-75% confidence bucket: 87-100% accuracy. Well-calibrated.
- 75-100% confidence bucket: 72-85% accuracy. Good but overconfident.
- On-Chain hit 100% (7/7) on one run — small sample, needs 50+ to trust.
- Zero parse errors across multiple runs. Streaming fallback working.
- Short bias: almost every trade is SHORT. Agent only goes long on deep capitulation.

**Technical Insights:**

- BGeometrics API (free, no key) replaces dead CoinMetrics/CoinGecko for MVRV/SOPR/NUPL.
- OKX funding rate (free, no key) replaces garbage Kraken Futures (-45% → 0.01%).
- Range validation on all external data prevents the agent from acting on garbage.
- TTL cache with LRU eviction prevents 240 HTTP requests/hour.
- Per-feed timeout (5s) on RSS prevents slow feeds blocking the insight refresh.
- `conditions_summary()` with SOUL.md thresholds makes raw data actionable.

---

## Session 2026-0531-2100: Closed-Loop Training Pipeline + Knowledge Selection Overhaul

**Key Learnings:**

- Knowledge selection was broken: all 2,959 units had priority 5, 282 execution units had zero conditions (invisible), 301/350 risk units were catch-all (always matched). The MMR architecture was sound but the data inputs were garbage.
- The catalog approach (sending AI a knowledge table to select from) was rejected by Perfection Loop: Trending alone matched 1,879 units, making the catalog 47K tokens (4x worse than the current 11K dump). Fixing data quality was simpler and more effective.
- Context tags must use the same prefixed format as knowledge unit tags (`regime_subtype:trending` not `strong_trend`). Zero overlap = zero tag matching. This was a critical bug caught during iteration.
- Two Gemini Deep Research reports (with and without memory context) independently converged on the same 6-stage self-improvement pipeline: Episodic Capture → Semantic Consolidation → Anti-Pattern Detection → Reflexion Replay → GEPA Mutation → Knowledge Lifecycle.
- When two independent research paths converge on the same solution, it's the right one.
- Streaming LLM responses (SSE) keeps the connection alive during long reasoning (mimo v2.5 pro can take 30-90s) and provides real-time visibility.
- The engine had a double-sleep bug (sleep + tokio::select with another sleep), doubling the tick interval.
- Separate test DB (`test_memory.db`) from live DB (`memory.db`) is critical — test episodes must never pollute live trade history.

**Agent Behavior:**

- The agent correctly identified a funding rate anomaly (27.25%/8hr = 29,842% annualized) and chose not to trade. SOUL.md crisis protocol working as intended.
- Knowledge selection after fixes: 20 units (capped from 113), differentiated priorities 2-5, context_tags from RSI/ADX/EMA/VWAP.
- Prompt size reduced from 66K to 40K chars (-41%).

**Technical Insights:**

- `reqwest::Client::builder().timeout()` works for parallel calls. Bare `reqwest::Client::new()` was the previous fix for TLS issues but lacked timeout protection.
- `cargo clippy -- -D warnings` catches `format!` inside `println!`, `new_without_default`, `too_many_arguments` — all worth fixing.
- PowerShell file editing can break bracket matching when removing line ranges. Always verify with `cargo build` after.
- The `<!-- MUTABLE -->` / `<!-- END MUTABLE -->` marker approach for SOUL.md partitioning is clean and parseable.
- Exponential backoff with ±20% jitter prevents thundering herd on WS reconnection.
- Each phase of the training pipeline must be wrapped in its own error boundary so a failure in one doesn't prevent others from running.

---

## Session 2026-0530-early: Initial Build

**Key Learnings:**

- See archived FIDs 001-026 for detailed findings from the initial build sprint.
- Key architecture decisions: SQLite WAL for episodic memory, tokio for concurrency, axum for REST API, mimo v2.5 pro via OpenGateway.
- The "reasoning" field quirk of mimo v2.5 pro (content returned in "reasoning" not "content") required custom parsing.

---

<!-- Add new entries above this line -->
