# Changelog

All notable changes to Savant Trading will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),

## [0.7.0] — 2026-06-03

### Added

- **FID-035 Phase 3** — Dual timeframe (5m→15m aggregation, no extra API calls). KV cache optimization (`cache_control: ephemeral` on system message for OpenRouter prefix caching). Emergency liquidation CLI (`--liquidate` flag). Retry queue for failed swaps (max 3 attempts).
- **FID-035 Phase 2** — GoPlus Security API (honeypot/tax detection for meme coins). Risk buckets (macro/legacy/meme) with correlation cap. ATR-based position sizing (`calculate_with_atr()`).
- **FID-035 Phase 1** — Meme coin expansion: 13 pairs (8 core + PEPE, SHIB, FLOKI, TURBO, MOG). Spread filter (30bps max). Price tolerance (0.5% drift). `highlight_pairs()` updated.
- **FID-034** — ANSI color placement fix.
- **FID-033** — Uniform console output via custom tracing Layer.
- **Merge strategy** — `dev/MERGE-STRATEGY.md` documenting cherry-pick approach.
- **Handoff docs** — `dev/HANDOFF-OTHER-DEV.md`, `dev/AGENT-PROMPT-PRESTON.md` (archived after consumption).
- **12h EST timestamps** — `SavantTimer` for tracing subscriber, `est_now()` shared function.
- **15s timeout on 0x API** — `tokio::time::timeout(15s)` around `build_swap_tx()` prevents indefinite API hangs.

### Fixed

- **ANSI color placement (FID-034)** — Color codes placed after text in format string, so colors applied to nothing. Fixed with named format params. Capitalized module names. Stripped Debug quotes from tracing messages.
- **Uniform console output (FID-033)** — Two formatting systems running (savant_log vs tracing). Created custom `SavantLayer` tracing Layer. All output now uses `[Savant Trading] [TIME] [ACTION] [RESULT]` format with consistent colors.
- **Panic hook** — Added `std::panic::set_hook` to log panics with file:line:col before crashing. Engine now shows `[PANIC] message at file.rs:123:45` instead of silent exit code 0xffffffff.
- **Pair name highlighting** — `highlight_pairs()` now works on both bare (`BTC/USD`) and already-bracketed (`[BTC/USD]`) pairs.
- **GREY_FG color** — Changed from `\x1b[37m` (light grey = white) to `\x1b[90m` (bright black = grey).
- **Vault level brightness** — Changed from `(GREY_DIM, GREY_DIM)` to `(GREY_DIM, GREY_FG)` — result text now readable.
- **Tracing color bleeding** — tracing's ANSI codes were bleeding into `savant_log()` output. Replaced `fmt()` subscriber with custom `SavantLayer`.
- **12h clock format** — `est_timestamp()` now returns `MM-DD-YYYY H:MM AM/PM` instead of 24h format.
- **Decision reasoning truncation** — Console log truncates reasoning to 100 chars (full text in vault/episodic).
- **Clippy warnings** — Fixed 3 warnings (empty line, empty format string, `and_then` → `map`).

### Changed

- **Version bump** — 0.6.0 → 0.7.0 (cherry-picked improvements from other dev's branch)
- **Tracing subscriber** — Uses `SavantTimer` for EST timestamps, `with_ansi(false)` to prevent color bleeding.
- **Preston branch created** — `origin/preston` for other dev's Kraken porting work.

### Archived

- **FID-026** — Sell/Close action handling (resolved → archived)
- **FID-027** — Swap execution hang (resolved → archived)
- **FID-028** — Console logging (resolved → archived)
- **FID-025** — NVIDIA NIM provider (verified → archived)
- **Handoff docs** — MERGE-STRATEGY, HANDOFF-OTHER-DEV, AGENT-PROMPT-PRESTON (consumed → archived)

## [0.6.0] — 2026-06-03

### Added

- **Enterprise console logging** — `src/core/console.rs` with single `savant_log()` function. Format: `[Savant Trading] [MM-DD-YYYY HH:mm] [ACTION] [RESULT]`. Cyan brand prefix, grey timestamps, white/green/orange/red results. EST timezone. 11 thin macros (`log_phase!`, `log_llm!`, `log_decision!`, `log_trade!`, `log_swap!`, `log_swap_ok!`, `log_swap_fail!`, `log_vault!`, `log_circuit!`, `log_warn!`).
- **3-retry logic for swap failures** — Retries on gas spike, nonce collision, network error, timeout. 2s delay between retries. Permanent failures (insufficient balance, invalid params) fail immediately.
- **Phantom position reconciliation** — DexTrader auto-clears positions on startup when balance drift > $1 or positions exist with zero completed trades. PaperTrader auto-reconciles when executor has no positions.
- **Position sizer logging** — Logs reason when returning None (stop/R:R invalid, entry/stop/tp1 values).
- **60s timeout on swap execution** — `tokio::time::timeout(60s)` around `place_order()` and `close_position()` prevents indefinite hangs.
- **50% gas buffer** — `maxFeePerGas = baseFee + baseFee/2 + priority` prevents `max fee per gas less than block base fee` errors on Arbitrum.
- **Kraken rebase prompt** — `dev/KRAKEN-REBASE-PROMPT.md` (385 lines) with ECHO Protocol boot sequence, file map, architecture docs, conflict zones, verification checklist.

### Fixed

- **Swap execution hang (FID-027)** — `place_order()` hung indefinitely when RPC call had no timeout. Added 60s timeout + retry logic.
- **Gas price too low** — 0x API returned stale gas estimate. Added 50% buffer to `maxFeePerGas`.
- **Console logging inconsistency (FID-028)** — Mix of `tracing` and `eprintln!`, no colors, no timestamps. Unified through `savant_log()`.
- **Clippy warnings** — Fixed 3 warnings: empty line after doc comment, empty format string, `and_then` → `map`.

### Changed

- **Version bump** — 0.5.0 → 0.6.0 (DEX execution fixes + console logging)
- **All version references updated** — Cargo.toml, VERSION, README

## [0.5.0] — 2026-06-03

### Fixed

- **FID-026: Sell/Close action handling (critical)** — Engine ignored AI's `Sell` and `Close` decisions. All non-Hold actions (including Sell, Close) fell through to `place_order()`, which always **opened** a new position. The agent could never exit positions except via stop-loss. On DEX, this caused on-chain swap failures when the wallet didn't own the base token (nonce stayed 0). On Kraken CEX, Sell opened a new short instead of closing an existing long. Fixed by adding action-aware branching: `Sell` → finds existing position for pair → `close_position()`; `Close` → finds ALL positions for pair → `close_position()` each; `Buy` → duplicate guard before `place_order()`. Creates proper `TradeRecord` with PnL for event bus. Backend-agnostic fix (both Kraken and DEX). Verified: 187/187 tests pass, clippy clean.

### Changed

- **Version bump** — 0.4.4 → 0.5.0 (critical sell logic fix = minor version bump)
- **All version references updated** — Cargo.toml, VERSION, README, protocol.config.yaml, main.rs, vault writer, scripts, HANDOFF.md, run scripts

## [0.4.4] — 2026-06-02

### Closed (FID-016, FID-017, FID-018, FID-019, FID-020, FID-021, FID-022, FID-023, FID-024 — archived 2026-06-02)

- **FID-016: Kraken Live Trading Execution Engine** (critical) — KrakenTrader implemented with 14/16 proposed fixes. Private API client with HMAC-SHA512 signing, order placement (market/limit/stop), balance sync, daily loss halt, kill switch, slippage alerts, Discord webhook notifications. See commit `d2ab69a`. Minor gaps: partial fill tracking, sandbox mode. Status: closed → archived.

- **FID-017: Multi-Exchange DEX Integration (0x + 1inch)** (critical) — DexTrader with ZeroXBackend and InchBackend. Enterprise token resolution (symbol fallback for non-EVM tokens). EIP-1559 signing with ECHO Law 6 compliance. Docs audit fixed 4 0x API issues + 2 1inch API issues + 2 EIP-1559 critical bugs. 176 tests, clippy clean. Status: verified → closed → archived.

- **FID-023: OpenRouter LLM Provider** (high) — Added OpenRouter as first-class AI provider alongside OpenGateway. Provider factory (`create_provider()`), `extra_headers` support, `OpenRouterConfig` struct with endpoint/model/api_key_env/referer/title, config validation, engine wiring in `run()` and `dry_run()`. Zero protocol changes — same OpenAI-compatible wire format. Status: verified → closed → archived.

- **FID-024: OpenRouter Model Env Var + Management Key System** (medium) — Added `OPENROUTER_MODEL` env var override for model switching without config edits. Created `OpenRouterManagementClient` with full CRUD (list/create/get/update/delete keys) via `/api/v1/keys`. `OpenRouterManagementConfig` struct, optional engine startup wiring for key usage monitoring. Status: verified → closed → archived.

- **FID-018: DEX Production Safety** (critical) — Stop-loss persistence + re-establishment, balance reconciliation via RPC (`eth_getBalance` + USDC `balanceOf`), crash recovery via JSON state persistence (`data/dex_state.json`). ETH gas halt at <0.002 ETH. State saved on every position mutation. Status: analyzed → closed → archived.

- **FID-019: DEX Test Infrastructure** (medium) — `ZeroXBackend::with_client()` and `InchBackend::with_client()` constructor injection. `with_client_and_url()` for wiremock routing. 12 hermetic tests covering happy path, 429, 500, malformed JSON, and missing fields for both backends. All pass without API keys or network. Status: analyzed → closed → archived.

- **FID-020: TUI Code Quality** (low) — Dynamic footer reads backend, mode, budget, model from `TuiSnapshot`. Version uses `env!("CARGO_PKG_VERSION")`. Drawdown thresholds derived from config values. Status: analyzed → closed → archived.

- **FID-021: has_actionable_signal Pre-Filter Review** (medium) — EMA spread threshold 0.1% → 0.5%. VWAP deviation check wired (was dead code). Volume spike gate added (`vol / volume_sma > 1.5`). Trending regime gate removed (redundant with ADX > 25). `current_price` and `current_volume` parameters passed to function. Status: analyzed → closed → archived.

- **FID-022: CLI TUI Overhaul** (high) — Multi-tab Ratatui terminal with 5-file modular architecture (`mod.rs`, `state.rs`, `tabs.rs`, `widgets.rs`, `keyboard.rs`). 10 tabs with keyboard navigation (0-9, Tab, arrows, PgUp/PgDn), search (`/`), help overlay (`?`/`F1`). Snapshot-based rendering from `SharedEngineData`. Status: analyzed → closed → archived.

---

**All 7 FIDs closed this session:** FID-018 (DEX Safety), FID-019 (DEX Tests), FID-020 (TUI Quality), FID-021 (Pre-filter), FID-022 (TUI Overhaul), FID-023 (OpenRouter Provider), FID-024 (OpenRouter Management).

### Added (FID-015 — Gemini Deep Research Optimization Overhaul)

- **Config Overhaul** — Corrected fee rate (0.26% → 0.40% taker), tightened risk (daily loss 20% → 10%, drawdown 40% → 20%), raised R:R (1.5 → 2.0), temperature 0.6 + top_p 0.95, knowledge budget 8K → 20K chars, candles 100 → 500, timeout 180s → 300s, max_tokens 131072 (128K), added 1d timeframe.
- **Maker Order Support** — `order_type` field (LIMIT/MARKET) in TradeDecision. Paper trader maker fee corrected to 0.25%.
- **Prompt Architecture** — XML-tagged prompts (`<identity>`, `<risk_constraints>`, `<strategy_knowledge>`, `<trading_rules>`, `<output_format>`, `<thinking>`). 5-step structured reasoning framework.
- **Session Liquidity Profiles** — 9 UTC-based sessions (Deep Asian 0.5x, US-EU Overlap 1.2x). Breakout confidence penalties for low-liquidity sessions.
- **Garman-Klass Volatility** — OHLC-based volatility estimator in indicator engine. More accurate than ATR for stop-loss sizing.
- **Isotonic Regression** — PAVA confidence calibrator. Maps raw LLM confidence to calibrated probability. Wired into training report.
- **Four-Factor Causal Attribution** — Loss classification (Setup/Process/Market/Trader) in training pipeline.
- **Historical Tick Data** — `data/historical.rs` fetches and caches 30 days of 5m candles from Kraken. `--historical` CLI flag.
- **JSON Repair** — Enhanced `repair_json_string()` with mid-word truncation detection, extra-text-after-JSON stripping, partial_extract fallback on repaired strings.

### Fixed

- **maker_fee_rate** — Paper trader maker fee corrected from 0.16% to 0.25% (actual Kraken base tier).
- **Garman-Klass in context** — Now displayed alongside ATR in market data section.

## [0.4.3] — 2026-06-01

### Added

- **SSE Streaming LLM Provider** — `chat_stream()` for real-time response streaming via Server-Sent Events. Keeps connection alive during long reasoning (mimo v2.5 pro can take 30-90s). Parses both `delta.content` and `delta.reasoning` fields. Streaming fallback to non-streaming on failure. 180s timeout.
- **Semantic Consolidation** — `memory/semantic.rs`: SQL aggregations against episodic memory to extract regime/session/pair edge matrix, conviction calibration, category edge. Populates `semantic_patterns` table. Rolling 90-day pattern decay. PF calculated from wins/losses ratio (not pnl).
- **Anti-Pattern Detection** — `memory/anti_pattern.rs`: SQL queries for conditions where win_rate < 30%. Category-level detection via `episode_market_context.condition_tags`. Narrative constraints for prompt injection. Auto-eviction when conditions recover.
- **Multi-Asset Correlation** — `risk/correlation.rs`: Rolling Pearson correlation matrix between active pairs. Effective position counting (correlated pairs count as 1.5-2x).
- **Portfolio Heat Tracking** — `risk/circuit_breaker.rs`: Total risk exposure / equity calculation. Blocks new trades when heat > 40%. Spread width halt at 50bps.
- **Dynamic Slippage** — `execution/paper.rs`: Slippage scales with ATR volatility and order book depth. `update_atr()` and `update_book_depth()` methods.
- **Maker-Fee Routing** — `execution/paper.rs`: If spread > fee differential (10bps), posts limit order at bid/ask instead of crossing with market. Saves 10bps per trade.
- **Knowledge Lifecycle** — `agent/knowledge.rs`: `utility_score` field on KnowledgeUnit (serde default 1.0). MMR scoring adjusted: `base * (1 + log2(utility))`. `save_utility_scores()` and `load_utility_scores()` for persistence.
- **SOUL.md Evolution** — `sandbox/feedback.rs`: `<!-- MUTABLE -->` markers on Section XIII+. `extract_mutable_sections()`, `apply_mutation_to_soul()`. Teacher LLM prompts (critique + GEPA mutation). `soul_versions` table for version control with auto-rollback.
- **Train/Val Split** — `sandbox/scenarios.rs`: `load_train_scenarios()` (first 40), `load_val_scenarios()` (last 20). `load_scenarios_by_difficulty()`, `worst_category()`.
- **WS Exponential Backoff** — `data/websocket.rs`: Reconnection uses exponential backoff (1s→30s cap) with ±20% jitter. CancelAllOrders signal after 3 consecutive failures.
- **BGeometrics On-Chain** — `insight/onchain.rs`: Replaced dead CoinMetrics/CoinGecko (403) with BGeometrics API. Free, no key, daily MVRV/SOPR/NUPL. Range validation on all values.
- **OKX Funding Primary** — `insight/funding_rates.rs`: OKX as primary funding source. Free, no key, no geo-block. Kraken as fallback with range validation (-2% to +2%).
- **RSS Cap + Source Diversity** — `insight/rss.rs`: `fetch_all_feeds_capped()` with per-feed 5s timeout, source diversity (top 2 per source), relevance scoring, cap enforcement.
- **Conditions Summary** — `insight/aggregator.rs`: `conditions_summary()` with SOUL.md thresholds. Actionable market assessments instead of raw data dump. RSS sentiment classification with negation handling.
- **TTL Cache** — `data/cache.rs`: TTL-based cache with LRU eviction. Graceful degradation (serve stale on API failure). Tests included.
- **Training Pipeline** — `engine.rs`: `run_training_batch()` with memory capture, Brier score, confidence distribution, category edge, auto-lessons, knowledge utility update, semantic consolidation, anti-pattern detection. All phases wrapped in error boundaries.
- **Training CLI** — `main.rs`: `savant --test --train` with filters (-c, -a, -n). `savant report --test` for full audit.
- **Training Report** — `monitor/training_report.rs`: P&L simulation, conviction calibration, confidence curve, category edge, anti-patterns, knowledge utility, lessons summary, semantic patterns, recent episodes.
- **SQLite Backup** — `engine.rs`: `backup_databases()` with rolling timestamped backups. Keeps last 7 copies.

### Fixed (FID-012, FID-013, FID-014 — closed 2026-06-01)

- **Confidence Floor** (FID-014) — `decision_parser.rs`: Trades with confidence < 40% automatically downgraded to Hold. Removes the 0-25% confidence bucket (18% accuracy). Highest-impact one-line fix.
- **Short Bias** (FID-014) — `scenarios.rs`: `derive_expected_action()` rebalanced. Capitulation buy signals boosted (2→3), moderate capitulation added (MVRV<1.2+SOPR<1.0), fear signals boosted (1→2), mild fear added (FG≤45). Buy threshold tightened to require `buy > sell`.
- **Vault Wiring** (FID-012, FID-013, FID-014) — `engine.rs`: Training batch now writes to vault. `project_decision()` per scenario, `project_risk_event()` for anti-patterns, `project_sandbox()` for batch report. 5 empty vault folders populated.
- **Training Default** (FID-014) — `engine.rs`, `main.rs`: `--train` defaults to 5 runs (was 20). `--train --full` for 20 runs. Help text updated.
- **Knowledge Selection Overhaul** — Indicator-derived conditions (RSI/ADX/EMA/volume → MarketCondition). Context tags use prefixed format. Unit cap (20). Scoring: tags×3, conditions×2, priority×1.
- **Knowledge Priority Migration** — All 2,959 units migrated from uniform 5 to differentiated 2-5. Risk catch-alls fixed. Execution units given conditions.
- **Random Scenario Generator** — `sandbox/scenarios.rs`: `generate_random_scenarios()` with weighted categories (weak areas get 3x). Expected actions derived from mock data. Every run is unique.
- **Protocol v0.1.0** — ECHO.md, protocol.config.yaml, templates, coding-standards synced from GitHub.
- **Training Workflow** — `docs/TRAINING-WORKFLOW.md`: Formalized closed-loop TRAIN → AUDIT → IDENTIFY → FIX → RETRAIN cycle.
- **/api/training** — Endpoint returning training metrics, config, episode count.

### Changed

- **Double-sleep bug fixed** — Engine had `time::sleep()` followed by `tokio::select!` with another sleep. Removed extra sleep.
- **Dry-run uses build_context()** — Same path as live engine. No custom prompt building.
- **Debug logging in engine** — Phase 1 and Phase 2 have debug-level logging.
- **Knowledge JSON files** — All 10 files migrated: priorities 2-5, risk catch-alls trimmed, execution units given conditions.
- **Max retries reduced** — 1 streaming + 1 fallback = 2 total per pair (was 3+1=4).
- **LLM timeout increased** — 180s (was 90s). Handles large prompts.
- **Dev folder restructured** — `findings` → `fids`, `archived` → `archive`, removed `baselines`/`plans`.
- **FID lifecycle** — Closed FIDs auto-archived per ECHO Protocol.
- **LEARNINGS.md** — Updated with session lessons.

### Fixed

- Context tag format mismatch — Tags were plain words, knowledge units use prefixed format.
- Risk catch-all conditions — 301/350 risk units always matched. Trimmed by content.
- Execution units invisible — 0 conditions → [Trending, Ranging].
- Kraken funding rate garbage — -45% per 8hr. Replaced with OKX (0.01%).
- Dead on-chain APIs — CoinMetrics/CoinGecko 403. Replaced with BGeometrics.
- RSS bloat — 333 items when config says 10. Cap enforced.
- Format string errors in action test output.
- Byte index panic on multi-byte UTF-8 chars in reasoning truncation.

### Tests

- 136 total tests (was 119)
- Cache: 5 tests
- Correlation: 4 tests
- Circuit breaker: 3 new tests (spread width)
- On-chain: 10 tests
- All tests passing, zero clippy warnings
- **Training Config** — `core/config.rs`: `TrainingConfig` struct with all training parameters. `config/default.toml`: `[training]` section with min_sample_size, failure_win_rate, max_portfolio_heat, backup_interval, utility_learning_rate, etc.
- **SQLite Backup** — `engine.rs`: `backup_databases()` function with rolling timestamped backups. Keeps last 7 copies. Called before training starts.
- **/api/training Endpoint** — Returns total episodes, semantic pattern count, Brier estimate, training config, SOUL.md version.
- **Persistent Training Pipeline** — `engine.rs`: `run_action_test()` and `run_training()` with memory capture, Brier score, confidence distribution, category edge, auto-lesson generation, progressive difficulty, convergence detection.
- **6th Prompt Layer Wiring** — Memory context now includes semantic patterns + anti-patterns alongside win rates and recent episodes.
- **Knowledge Selection Overhaul** — Indicator-derived conditions (RSI/ADX/EMA/volume → MarketCondition). Context tags use prefixed format matching knowledge vocabulary (`regime_subtype:trending` not `strong_trend`). Unit cap (20). Scoring rebalanced: tags × 3, conditions × 2, priority × 1.
- **Knowledge Priority Migration** — All 2,959 units migrated from uniform priority 5 to differentiated 2-5 based on content specificity. Risk catch-all conditions fixed. Execution units given conditions.

### Changed

- **Double-sleep bug fixed** — Engine had `time::sleep()` followed by `tokio::select!` with another sleep, doubling the tick interval. Removed the extra sleep.
- **Dry-run uses build_context()** — Dry-run now calls the exact same `build_context()` as the live engine. No more custom prompt building.
- **Debug logging in engine** — Phase 1 (candle fetch, order book, higher TF, pre-filter) and Phase 2 (LLM streaming) have debug-level logging for hang diagnosis.
- **Knowledge JSON files** — All 10 files migrated: priorities 2-5, risk catch-alls trimmed, execution units given [Trending, Ranging] conditions.

### Fixed

- Context tag format mismatch — Tags were plain words (`oversold`, `strong_trend`) but knowledge units use prefixed format (`regime_subtype:capitulation`, `setup_type:breakout`). Zero overlap meant zero tag matching. Fixed to use matching format.
- Risk catch-all conditions — 301/350 risk units had 5+ conditions (ExtremeFear, ExtremeGreed, HighVolatility, LowVolatility + more), always matching regardless of market state. Trimmed by content relevance.
- Execution units invisible — 282 execution units had zero conditions, never selected by the condition filter. Added [Trending, Ranging].
- Format string errors in action test output.

### Tests

- 119 total tests (was 112)
- Knowledge: tests updated for utility_score field
- All tests passing, zero clippy warnings

## [0.4.2] — 2026-05-31

### Added

- **Persistent memory system** — 4-tier architecture based on Gemini Deep Research (40 citations)
  - Episodic capture: SQLite WAL (6 tables, 7 indices), every decision stored with full market context
  - 6th prompt layer: Dynamic Memory Context injected into AI prompt (win rates, recent analogs, CUSUM alerts, operator rules)
  - Brier Score calibration: confidence penalty calculation from trade history
  - CUSUM control chart: edge decay detection per pair, persisted to SQLite
  - Experience Replay: generates lessons from HIGH conviction losses on startup
  - Operator rules: loaded from vault Lessons/ directory, injected as "OPERATOR RULES (override all AI reasoning)"
  - Progressive confidence: 1-25 trades=LOW, 26-50=MEDIUM, 50+=HIGH
  - TUI Memory panel: Brier Score, confidence cap, CUSUM status per pair, replay lesson count

- **Sandbox & stress testing system** — 4-phase "trading dojo" based on Gemini Deep Research (50 citations)
  - GARCH(1,1) OHLCV generator with configurable trend, volatility regime, market events
  - Order book simulator with bid/ask depth, imbalance calculation, slippage simulation
  - 50 curated scenarios across 11 categories (Trend Bull/Bear, Range, Volatility, Catalyst, Microstructure, Session, Correlation, Sentiment, On-Chain, Edge Cases)
  - 3-tier grading rubric: binary compliance, R:R scoring, reasoning quality
  - GEPA-style feedback loop: failure analysis, SOUL.md mutation proposals
  - Report card generator with category breakdown and critical failures
  - Vault/Sandbox/ integration for report output

- **SOUL.md persona** — 560-line enterprise trading identity
  - 12 sections: Identity, Creed, Cognitive Style, Communication, Emotional Architecture, Crypto Philosophy, Risk Management, Decision Framework, Operational Constraints, Operator Relationship, Technical Values, Identity Invariants
  - Resolves all knowledge base contradictions (circuit breakers, R:R, position sizing)
  - 10-point pre-trade checklist, 8 identity invariants, quick reference card

- **Knowledge base expansion** — 141 → 254 units, 11 → 22 JSON files
  - 11 new files: on-chain, risk math, derivatives, Wyckoff, macro, DeFi, backtesting, execution engineering, prop firms, psychology, compliance
  - 7 new MarketCondition variants

- **Deep research documents** — Memory system design (40 citations), Sandbox design (50 citations), SOUL design, 155 research questions

### Changed

- **All dead code wired** — EventBus, VaultWriter, VaultWatcher, StopLossCalculator, OrderBookManager, format_for_context, fetch_funding_multi
- **API + engine merged** — API spawns as background task alongside engine
- **Parallel AI evaluation** — all pairs evaluated simultaneously via JoinSet
- **Crypto-native sessions** — removed stock market "off-hours", all sessions tradeable
- **Decision parser hardened** — normalizes UPPERCASE/empty action/side fields

### Fixed

- Parse crash on markdown-wrapped LLM responses
- Drawdown kill switch was non-functional (update_equity never called)
- Paper trader could open unlimited positions (entry cost not deducted)
- Daily PnL cumulative from engine start (now resets at midnight UTC)
- Rate limiter permanent lockout (now resets every second)
- .gitignore was ignoring src/data/ (anchored to repo root)
- RSS UTF-8 crash on Bulgarian text (floor_char_boundary)
- .env never loaded (added dotenvy)

### Tests

- 112 total tests (was 13)
- Sandbox: 28 tests (generator, grader, harness, scenarios, feedback, report)
- Memory: 9 tests (calibration, cusum)
- Indicators: 13 tests
- Risk: 13 tests
- Paper: 6 tests
- Insight: 10 tests
- API: 4 tests
- Strategy: 4 tests
- Vault: 3 tests
- Agent: 7 tests

## [0.4.1] — 2026-05-30

### Added

- **On-chain analytics** — Live MVRV, SOPR, NVT from CoinMetrics (CoinGecko fallback). On-chain conditions (`MvrvExtreme`, `SoprReset`) injected into knowledge selection.
- **Unit tests** — 73 total tests across 11 modules (was 13). API, insight, vault, indicators, circuit breaker, position, regime, metrics, stop loss, onchain, websocket.
- **Kraken WebSocket v2 client** — `connect()` with auto-reconnection, `parse_message()` for ticker/book/trade channels.
- **Backtesting engine** — Candle replay via `evaluate_sync()`, Sharpe/drawdown/profit factor metrics, walk-forward optimization with cumulative balance.
- **TUI dashboard** — Ratatui 0.30 real-time terminal UI. Snapshot-based rendering (no `block_on` deadlock). Portfolio, positions, decisions, insight panels.
- **Knowledge base expansion** — 141 → 254 units, 11 → 22 JSON files. Added: on-chain, risk math, derivatives, Wyckoff, macro, DeFi, backtesting, execution engineering, prop firms, psychology, compliance.
- **7 new MarketCondition variants** — `LiquidityExpansion`, `LiquidityContraction`, `MvrvExtreme`, `SoprReset`, `OIDivergence`, `WyckoffSpring`, `DeltaDivergence`.
- **REST API** — All 13 endpoints return real engine state via `SharedEngineData`. Rate limiter (sliding window). Knowledge by topic endpoint (`/api/knowledge/:topic`).
- **Production safety** — Graceful shutdown (ctrl_c saves state). Block file mechanism (`savant.blocked`). State persistence (`data/paper_state.json`).
- **Scale-out execution** — TP1 → 50% close + break-even stop, TP2 → 60% of remainder, TP3 → full close.
- **Structure stop validation** — AI-proposed stops validated against 3x ATR bounds. `structure_stop()` fallback.
- **Session multiplier wired** — `position_size_multiplier()` applied to both AI and fallback paths. PreMarket session (5-7 AM EST, 0.7x).
- **Configurable volume profile** — `volume_profile_with_pct()` accepts `value_area_pct` parameter.
- **Deep research documents** — `docs/DEEP-RESEARCH-QUESTIONS.md`, `docs/Crypto Trading Knowledge Expansion Roadmap.md`, `docs/KNOWLEDGE-EXPANSION-EXECUTION.md`.

### Changed

- **All dead code wired** — EventBus, VaultWriter, VaultWatcher, StopLossCalculator, OrderBookManager, `format_for_context`, `fetch_funding_multi`.
- **API + engine merged** — API spawns as background task alongside engine. Both share `SharedEngineData`.
- **Insight aggregator** — `refresh_multi()` batches all pairs in single funding API call.
- **WebSocket refactored** — Removed unused `KrakenWebSocket` struct. Kept `connect()`, `parse_message()`, `create_channel()`.
- **Vault writer guards** — `project_trade()`, `project_decision()`, `project_portfolio()` check `config.enabled`.
- **Ratatui bumped** — 0.29 → 0.30 (fixes `lru` GHSA-rhfx-m35p-ff5j vulnerability).

### Fixed

- `parse_wrapped_json` test — Hold decisions with `entry_price: 0.0` no longer rejected.
- `vault/writer.rs` — Raw string `#` parsing error (Rust 2021 reserved prefix).
- `update_equity()` now called — Drawdown kill switch was non-functional.
- Entry cost + fee deducted from balance — Paper trader could open unlimited positions.
- `daily_pnl` resets at midnight UTC — Daily loss limit was cumulative.
- `Display` for `Side` — Logs show `LONG`/`SHORT` instead of `0`/`1`.
- Rate limiter resets every second — Was permanent lockout after 1000 requests.
- Duplicate `parse_timeframe` removed — Engine and main had different return types.
- RSS UTF-8 crash — `floor_char_boundary` for Bulgarian text.
- `.env` loading — Added `dotenvy::dotenv().ok()` to `main()`.

### Hygiene

- Moved misplaced files (yt.md, SESSION-SUMMARY.md, overview.jpg).
- Deleted Claude Code leftovers (dashboard/AGENTS.md, CLAUDE.md).
- Added `savant-vault/` to `.gitignore`.
- Removed duplicate knowledge files from `src/agent/knowledge/`.
- Removed GitHub Actions CI (billing issue).
- MSRV set to 1.91. Added `uuid` crate. Removed unused `tower` dep.
- Version aligned: Cargo.toml, README, protocol.config.yaml, CHANGELOG.

## [0.4.0] — 2026-05-30

### Added in 0.4.0

- **Glass House — Obsidian vault integration** — Bidirectional vault sync for transparent trading state
  - `VaultWriter` — Projects engine state into Obsidian vault as structured markdown
    - Trades/ — Daily trade logs with entry/exit/PnL
    - Decisions/ — AI decision logs with reasoning
    - Portfolio/ — Balance history, equity curve
    - Knowledge/ — Knowledge unit index
    - INDEX.md — Master index with wiki-links
  - `VaultWatcher` — Monitors vault for user edits
    - Lessons/ — Editable ground truth ingested by engine
    - Injection defense — Scans for 14 prompt injection patterns
    - Invisible unicode detection
  - `VaultConfig` — Configurable vault path, sync interval, max files
  - `.obsidian/appearance.json` — Dark theme, cyan accent (#00d5ff)

## [0.3.1] — 2026-05-30

### Added in 0.3.1

- **Session-aware trading** — Engine knows what trading session is active
  - Asian (7 PM - 2 AM EST): low volume, ranging, 0.5x position size
  - London (2 AM - 5 AM EST): high volume, reversals, 1.0x
  - New York (7 AM - 10 AM EST): highest volume, continuations, 1.0x
  - London/NY Overlap (8 AM - 10 AM EST): peak volume, 1.2x
  - Off-hours: 0.3x position size, avoid new entries
  - Kill zone detection: London, NY, Overlap are high-probability windows
  - Session context injected into AI prompt

### Changed in 0.3.1

- `core/session.rs` — New module: session detection, behavior, position size multiplier
- `context_builder.rs` — Session info added to AI user message

## [0.3.0] — 2026-05-30

### Added in 0.3.0

- **Dashboard UI** — Next.js 16 + React 19 + TypeScript 5, matching Savant design system
  - 11 pages: Overview, Transactions, AI Decisions, Portfolio, Insight, Knowledge, Risk, Session, Settings
  - CSS Modules with Savant design tokens (`#00d5ff` accent, glass morphism, JetBrains Mono + Inter)
  - 3-column layout: sidebar (280px), main content, right panel
  - Ambient background with radial gradients
  - Custom scrollbars, glass panels, accent glow effects
  - All pages build successfully (`npm run build`)

## [0.2.1] — 2026-05-30

### Added in 0.2.1

- **ECHO Protocol in system prompt** — Trading rules from ECHO.md + transcript-derived rules embedded in AI prompt
  - Sell into strength (80% at 10-20% gain)
  - 3 consecutive losses = stop for the day
  - Don't marry positions
  - Four-factor performance model
  - Session awareness (kill zones)
  - Compound strategy (risk profits on directional days)
- **Trade history in AI context** — Last 10 trades + performance summary injected into prompt
  - Win rate, average win/loss, profit factor
  - Individual trade details with entry/exit/PnL
  - Cold start: omit if no trades yet
- **Multi-pair insight refresh** — Insight now refreshes for ALL configured pairs, not just the first
- **Structured logging** — AI context logged with prompt chars, knowledge budget, pair, regime

### Changed in 0.2.1

- `FullContext` struct now includes `recent_trades: Option<&[TradeRecord]>`
- System prompt includes ECHO rules alongside strategy knowledge
- Knowledge section header: "From 11 Curated Transcripts"

## [0.2.0] — 2026-05-30

### Added in 0.2.0

- **Dry-run test mode** — `savant --dry-run` runs one AI decision cycle and prints full pipeline output
  - Market data (candles, indicators, regime)
  - Insight (Fear & Greed, BTC Dominance, funding rates, RSS)
  - Knowledge selection (conditions → matched units)
  - System prompt (composed with knowledge injection)
  - LLM response (raw JSON from mimo v2.5 pro)
  - Parsed decision (action, entry, stop, targets, confidence, reasoning)
- **REST API server** — `savant --api` starts axum server on localhost:8080
  - 13 endpoints: status, config, portfolio, positions, trades, decisions, insight, knowledge, risk, session, engine control
  - All responses use `{data, error, timestamp}` envelope
  - Localhost-only binding (no external access)
- **External knowledge loading** — `knowledge/` directory at project root
  - Engine loads from `knowledge/` first, falls back to embedded
  - 11 JSON files with 141 knowledge units
  - Editable without recompiling
- **Help command** — `savant --help` shows usage

### Changed in 0.2.0

- Version bumped to 0.2.0
- CLI now supports: `savant` (trade), `savant --dry-run`, `savant --api`, `savant report`, `savant --help`
- `context_builder.rs` — `determine_conditions` and `build_user_message` now have static versions for dry-run use
- `axum = "0.8"` and `tower-http = "0.6"` added for REST API

### Dependencies in 0.2.0

| Dependency  | Status | Notes            |
|-------------|--------|------------------|
| `axum`      | NEW    | REST API server  |
| `tower-http`| NEW    | CORS support     |

## [0.1.2] — 2026-05-30

### Added in 0.1.2

- **RSS News Pipeline** — 15 free RSS feeds parsed with `quick-xml`, scored by relevance to trading pairs
  - Crypto-native: Cointelegraph, CoinDesk, CryptoSlate, Decrypt, CryptoNews, CryptoPotato, CryptoBreaking
  - DeFi: The Defiant, SmartLiquidity
  - Institutional: Blockworks, Bitcoin Magazine, Ethereum 2.0
  - Mainstream: Yahoo Finance, CNBC (macro context)
  - Regional: KriptoNovini
- **Kraken Futures Integration** — Funding rates, open interest, mark prices from `futures.kraken.com` (free, no key)
- **Liquidation Risk Assessment** — Derived from futures data: mark/index spread, funding extremes, OI concentration
- **On-chain Data** — Block height, mempool size, 24h tx count from blockchain.info (free, no key)
- **CoinGecko Trending** — Trending coins with price changes, used as social sentiment proxy
- **API-KEYS.md** — Reference document with all endpoints, signup links, and env var names

### Changed in 0.1.2

- All insight modules now use **free APIs only** — no paid API keys required
- CoinGlass replaced with Kraken Futures (free, no geo-block)
- All insight sources enabled by default in config
- 15 RSS feeds (up from 0)

### Removed in 0.1.2

- CoinGlass API dependency (not free)
- CryptoQuant API dependency (not free — blockchain.info used instead)
- API key fields from InsightConfig (all sources free now)

### Fixed in 0.1.2

- FID-015 Perfection Loop: `quick-xml` dependency validated, all API endpoints verified
- Cargo.toml: `quick-xml = "0.37"` added for RSS/XML parsing

## [0.1.1] — 2026-05-30

### Changed in 0.1.1

- **Knowledge Base Expansion** — 88 → 141 knowledge units (+60%)
  - `ai_claude_bot.json`: 5 → 20 units (HMM math, feature engineering, walk-forward, circuit breakers, dashboard)
  - `tjr_smc.json`: 15 → 19 units (partial FVG fills, rejection candles, engulfing, trailing stop models)
  - `crypto_fcb.json`: 13 → 18 units (MACD, Bollinger Bands, Fibonacci, altcoin selection, market cycles)
  - `pradeep_ep.json`: 12 → 15 units (delayed EP, continuation EP, sector rotation)
  - `juvier_daytrading.json`: 6 → 12 units (displacement candles, breaker blocks, Asian range, London vs NY)
  - `warrior_trading.json`: 6 → 10 units (gap fill probability, short selling, morning vs afternoon)
  - `brian_jung.json`: 5 → 8 units (stablecoin strategy, ETF impact, global liquidity)
  - `ai_competition.json`: 3 → 10 units (all 15 bot strategies, risk tier analysis, evolution methodology)
  - `hybrid_scalping.json`: 4 → 8 units (multi-TF scalping, session-specific, fee optimization)
  - `cathie_wood.json`: 4 → 6 units (Tesla robotaxi, humanoid robots, demographics)
  - `fabio_amt.json`: 15 → 15 units (refined with more specific execution details)

### Fixed in 0.1.1

- FID-001, FID-002, FID-003 status corrected from "analyzed" to "fixed"
- FID-007 status set to "superseded" (absorbed into FID-013)
- All 13 FIDs have proper Perfection Loop audits

### Verified in 0.1.1

- Fear & Greed API: `GET https://api.alternative.me/fng/?limit=1` → `23 (Extreme Fear)` — free, no key
- CoinGecko API: `GET https://api.coingecko.com/api/v3/global` → `BTC.D: 57.44%` — free, no key
- Build: `cargo clippy -- -D warnings` zero warnings, `cargo fmt --check` clean

## [0.1.0] — 2026-05-30

### Added in 0.1.0

- **Agent Module** — AI-powered autonomous trading brain with mimo v2.5 pro via OpenGateway
  - Knowledge injection system — 11 curated transcripts processed into discrete knowledge units
  - Modular 5-layer system prompt composer (identity, risk, strategy, knowledge, output format)
  - OpenAI-compatible LLM provider with retry and rate limiting
  - Context builder — aggregates candles, indicators, insight, positions into LLM context
  - Decision parser — extracts structured TradeDecision from LLM JSON responses
  - Orchestrator — main decision loop with 3 autonomy levels (Suggest, Confirm, Autonomous)
  - Fallback mode — rule-based strategies activate if LLM fails 3 consecutive ticks
- **Insight Module** — Live market context from external data sources
  - Fear & Greed Index (alternative.me, free)
  - BTC Dominance and total market cap (CoinGecko, free)
  - Funding rates, open interest, long/short ratio (stub — CoinGlass)
  - Liquidation clusters (stub — CoinGlass)
  - Exchange inflow/outflow (stub — CryptoQuant)
  - News and social sentiment (stub — LunarCrush)
  - Unified MarketContext aggregator with graceful failure handling
- **Core Engine** — Existing rule-based trading engine
  - Kraken REST + WebSocket integration
  - Technical indicators (EMA, SMA, RSI, ATR, ADX, VWAP, Volume Profile)
  - Paper trading simulator with realistic fees (0.26% Kraken taker) and slippage
  - Risk management: position sizing, stop-loss, break-even, circuit breakers
  - Trade journal with SQLite persistence and equity curve tracking
  - CLI with trade, report, and backtest commands
- **Knowledge Base** — 11 curated transcripts
  - Scalping (Fabio Valentina — order flow, volume profile, AMT)
  - Strategy (Pradeep Bondi — episodic pivots, execution edges)
  - Day Trading (TJR — SMC, FVG, order blocks, liquidity sweeps)
  - Crypto (Full Course — sentiment, BTC dominance, alt season)
  - Crypto (Brian Jung — macro catalysts, halving cycle)
  - Day Trading (Juvier — kill zones, session trading)
  - Day Trading (Warrior Trading — 5 Pillars, pullback entry)
  - Scalping (Hybrid — Heikin Ashi + EMA, prop firms)
  - Macro (Cathie Wood — innovation platforms, Wright's Law)
  - AI Trading (Claude Code bot — HMM regimes, circuit breakers)
  - AI Trading (Competition — natural selection, multi-agent)
- **FID System** — 13 tracked findings with Perfection Loop audits
  - 3 fixed (ECHO violations, paper persistence, fee/slippage)
  - 8 pending (trailing stops, scale-out, FVG, backtesting, WebSocket, multi-TF, tests, rate limiting)
  - 1 superseded (sentiment → absorbed into FID-013)
  - 1 new (AI Agent Brain — critical architectural upgrade)
- **ECHO Protocol** — Universal agent bootstrap with 15 laws and Perfection Loop FSM
