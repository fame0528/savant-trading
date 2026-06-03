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

## Session 2026-0602-0000: Gemini Deep Research, FID-015, Full Optimization Overhaul

**Key Learnings:**

- Gemini Deep Research produced two 300+ line reports with 23 academic citations. Both agreed on: temperature 0.6, risk overhaul, prompt architecture changes. Disagreed on system prompt removal (needs A/B test).
- **Never trust fee assumptions.** Kraken base tier is 0.40% taker / 0.25% maker, not 0.26%. At $50, this difference invalidates every R:R calculation.
- **Small accounts need different risk framework.** Kelly Criterion / 2% risk assumes you can survive losing streaks. At $50, you can't. Full deployment + tight safety nets is the correct approach.
- **Reasoning models need different prompt architecture.** System prompts and few-shot examples degrade MoE reasoning models. XML-tagged user prompts with structured reasoning steps work better.
- **max_tokens must match model specs.** MiMo v2.5 Pro has 128K output. We were capping at 2048-8192. The model's "thinking" consumed the entire budget, leaving no room for JSON.
- **Non-streaming `chat()` is faster but produces broken JSON at low max_tokens.** At 131072 tokens, it works perfectly. At 2048, 77% parse errors.
- **Isotonic Regression (PAVA)** is the correct mathematical approach for LLM confidence calibration. Don't trust the model's self-reported confidence — calibrate it with historical outcomes.
- **Session liquidity matters.** Deep Asian (02:00-06:00 UTC) has 42% less order book depth. Breakouts fail 40% more often. The agent should penalize confidence during low-liquidity sessions.
- **Garman-Klass volatility** uses OHLC data (not just close) for more accurate volatility measurement. Better for dynamic stop-loss widths.
- **Four-factor causal attribution** (Setup/Process/Market/Trader) gives the agent WHY it failed, not just THAT it failed. Injecting this into memory context accelerates learning.

**Agent Behavior (post FID-015):**

- Brier trajectory: 0.355 → 0.211 → **0.172** → 0.256. Best ever at 0.172.
- 50-75% confidence: 80-100% accuracy. Well-calibrated.
- 75-100% confidence: 50-100% accuracy. High variance at small sample.
- LONG trades appearing: 5 across 4 runs. Short bias broken.
- Trend Bull category: improved from 25% to 100% in run 4.
- 0 errors across all runs. JSON repair + 131072 tokens eliminated parse failures.
- Avg latency: 85-92s per scenario (non-streaming).

**Technical Insights:**

- OpenGateway returns compressed responses (zstd/br) that PowerShell can't decode. Use Rust reqwest client for API testing.
- `extract_json()` fails silently when there's no `}` in truncated JSON — falls through to full string. `repair_json_string` must handle this.
- `partial_extract` should try the REPAIRED string first, not the original. This one change fixed truncated string recovery.
- Kraken OHLC API returns max 720 candles per request. For 30 days of 5m data (~8,640 candles), need ~12 paginated requests at 1 req/sec.

---

---

## Session 2026-0602-1800: Historical Data Training, ECHO Law 6 Audit, FID-016 Bug Fixes

**Key Learnings:**

- **Law 1 (Read 0-EOF) is non-negotiable.** Attempting to read only specific line ranges (via sed/grep) instead of full functions was flagged as a violation. The correct approach is to read the entire function or file before any edit.
- **Python scripts for bulk replacements** are effective for patterns like adding fields to 60+ struct literals or removing 19 unwrap() calls across 12 files. Use `encoding='utf-8', errors='replace'` to handle UnicodeDecodeErrors on non-UTF-8 files.
- **`sem.acquire().await?` only works in closures returning `Result`.** When closures return struct types (PairResult, ScenarioResponse), use `let-else` pattern with sentinel returns instead.
- **`count_filter` ordering matters.** `extend()` appends to the end, and `truncate()` removes from the end. Apply truncation BEFORE extend to preserve the appended items.
- **`unwrap_or_else(|| vec![])` → `unwrap_or_default()`** is the most idiomatic form for empty Vec fallbacks that also satisfies clippy.
- **`PartialEq` on enums** is needed for `assert_eq!` in tests — can't assume it's derived.
- **Helper functions in test modules** should be inside `#[cfg(test)]` to avoid dead-code warnings.
- **ECHO.md session lifecycle** requires updating 3 files: session summary (in `dev/session-summaries/`), LEARNINGS.md (in `dev/`), and FID files (in `dev/fids/`).

**Technical Insights:**

- Historical scenario mixing requires converting `HistoricalScenario` → `Scenario` with `candles_override` set to context candles, and skipping `apply_scenario()` (since historical data has real market structure baked in).
- Trend/volatility derivation from historical candles: compute average price change across windows for trend direction, compute average (high-low)/close for volatility regime.
- All 19 non-test `.unwrap()` calls were eliminated without changing program behavior. The `partial_cmp` → `unwrap_or(Ordering::Equal)` pattern was the most common (8 occurrences).

---

## Session 2026-0602-2030: All 5 Open FIDs Implemented + Archived, Full ECHO Compliance

**Key Learnings:**

- **Python replacement scripts are effective but can introduce UTF-8 issues.** Em-dash `—` gets encoded as `\x97` in some Python configurations. Always run `cargo check` after any bulk replacement.
- **Field names must be verified against struct definitions.** Assumed `volume_ratio` existed on `IndicatorValues` but the actual field is `volume_sma`. Always grep the struct definition, never guess field names.
- **Five FIDs in one session is feasible** when each is targeted. Coordination overhead is real but manageable with clear write_todos planning.
- **str_replace on large Rust files with Windows CRLF** can fail silently due to whitespace byte differences. Use `sed -n 'N,Np' | od -c` to debug exact bytes when str_replace fails.
- **ECHO compliance check** as a dedicated maintenance task catches config drift (VERSION file was `0.1.0` protocol version instead of `0.4.4` project version). Should run at least once per session.
- **VERSION file must contain project version** (matching `Cargo.toml`'s `version`), NOT the ECHO protocol version (`protocol.config.yaml`'s `protocol.version`). These are different values.
- **All 7 FIDs closed and archived** means a clean slate — FID-001 through FID-024 are complete. 50 total archived FIDs.

---

---
## Session 2026-0602-1811: Recovery from Set-Content breakage + clippy fix sweep + historical_to_scenario

**Key Learnings:**

- **`Set-Content -NoNewline` on a PowerShell array joins ALL elements into ONE LINE with no separator.** Content is preserved but newlines are obliterated. Never use this pattern for Rust files. Use `[regex]::Replace` on raw string content, or `Out-File -Encoding utf8NoBom` after joining with `` `n ``.
- **When recovering from a single-line file, `git checkout` + `git show HEAD:file` restores the file.** Then re-apply changes one at a time with the Edit tool (not regex bulk replacements) to maintain control.
- **`items_after_test_module` in `scenarios.rs` is structural — the test module must be at EOF.** The prior author placed `mod tests {}` mid-file with 11+ pub functions after it. Fix: move `mod tests` to EOF, keep all pub functions before it.
- **`derive_historical_mock_data` thresholds must be > 2% net price change for bull/bear classification.** Test candles with < 2% change produce neutral mock data. Lesson: verify test thresholds match the actual function logic by checking boundary conditions.
- **Reachability verification (Law 4) proved critical.** After adding `historical_to_scenario`, a grep confirmed it's called at `engine.rs:2897`. Without this check, the function would have been dead code — it compiles but is never called.
- **PowerShell `nul` is a reserved Windows device name.** Git cannot index a file named `nul` because Windows treats it as the null device. Solution: `git add --all -- ':!nul'` to exclude it.
- **The 5 named constants pattern** (`HISTORICAL_TREND_THRESHOLD`, `STRENGTH_SCALE_FACTOR`, `VOLATILITY_*_THRESHOLD`, `MOCK_SENTIMENT_THRESHOLD`) satisfies ECHO Law 9 (no magic numbers) while keeping algorithmic thresholds readable.

**Technical Insights:**

- `Candle::close` at index 0 vs last is the correct basis for trend detection in `derive_historical_trend`. The net percentage change between first and last close determines direction; average per-candle return (via `windows(2)`) determines strength.
- `VolatilityRegime` classification uses `(high - low) / close` averaged across all candles. The thresholds (10%, 3%, 1%) were validated against this formula.
- `engine.rs:2094` branches on `candles_override`: `Some(real) → clone directly`, `None → generate synthetic via apply_scenario`. This is the correct architecture for mixing historical and synthetic data.

**Agent Behavior:**

- ECHO Protocol Perfection Loop was correctly followed once violations were acknowledged: RED (identify all 7 errors + 3 additional issues) → GREEN (fix them) → AUDIT (verify with test + clippy) → COMPLETE.
- The earlier violation (bulk regex without reading 0-EOF) wasted ~45 minutes on recovery. Following Law 1 strictly would have saved time.

<!-- Add new entries above this line -->

## Session 2026-06-03-0500: DEX Execution Pipeline, Console Logging, Project Audit

**Key Learnings:**

- **Always add timeouts to network calls.** `reqwest::Client::new()` has NO default timeout. A single hung RPC call can freeze the entire engine. Fix: `tokio::time::timeout(60s, ...)` around all swap execution calls.
- **Gas prices are stale by the time a tx is broadcast.** The 0x API returns a gas estimate from a few seconds ago. By the time the tx is signed and broadcast, baseFee has risen. Fix: 50% buffer on `maxFeePerGas` (`baseFee + baseFee/2 + priority`).
- **`tracing` deadlocks with `RwLock`.** The API server and engine share `SharedEngineData` behind an `RwLock`. Both use the same `tracing` subscriber. When the engine writes via `tracing::info!` and the API reads via `tracing`, they deadlock. Fix: use `eprintln!()` for all Phase 3 logging.
- **Single source of truth for logging prevents format drift.** Created `src/core/console.rs` with one `savant_log()` function and 11 thin macros. All console output goes through the same path. No scattered color logic.
- **`#[macro_use]` on module declaration propagates macros to the entire crate.** But binary files (`src/engine.rs`) that are NOT part of the lib need explicit `use crate::log_*` imports.
- **Phantom positions are a real problem.** The PaperTrader can accumulate positions that don't exist on-chain. Fix: auto-reconcile on startup — if executor has no positions but PaperTrader does, clear PaperTrader.
- **The AI is correctly disciplined.** It waits for valid setups with 3+ action triggers instead of forcing trades. In a ranging market with only 2/3 triggers met, holding is the correct decision.
- **Retry logic is essential for on-chain execution.** A single transient failure (gas spike, nonce collision) shouldn't kill a trade. 3 retries with 2s delay handles most transient issues.
- **`nul` is a reserved Windows device name.** Git cannot index a file named `nul`. Solution: add to `.gitignore`.

**Agent Behavior:**

- Engine ran for ~12 hours across multiple sessions
- AI made 50+ Hold decisions across 8 pairs — all disciplined
- 2 Buy signals fired (ETH/USD, AVAX/USD) — one rejected by position sizer, one reached 0x API
- No successful on-chain swap yet — market conditions not meeting 3+ trigger threshold
- Fear & Greed at 11 (Extreme Fear), SOPR at 0.9741 (capitulation), MVRV at 1.25 (neutral)

**Technical Insights:**

- 0x API v2 uses `permit2/quote` endpoint with `0x-version: v2` header
- Transaction data nested under `response.transaction` key (not flat)
- Permit2 approval needed for USDC → `0x000000000022d473030f116ddee9f6b43ac78ba3`
- Arbitrum baseFee fluctuates ~1-2% between quote and broadcast
- `eth_sendRawTransaction` can hang indefinitely without timeout
- Receipt verification (`wait_for_receipt`) prevents phantom positions from reverted swaps

---
