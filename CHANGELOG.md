# Changelog

All notable changes to Savant Trading are documented here.

## [0.14.10] — 2026-06-18

### SOT Infrastructure: SQLite as Single Source of Truth (Phase 1 of 2)

Phase 1 of FID-210 ships the SOT infrastructure. Phase 2 (FID-211) is the engine migration that wires callers to the new wrappers.

### Added — Schema Migration (`migrate_v210`)

- `migrate_v210` runs on engine startup, idempotent via `PRAGMA table_info` checks
- Adds `token_address TEXT NOT NULL DEFAULT ''` to `positions` table (Bug 6: was read but never written)
- Adds `real_trade BOOLEAN NOT NULL DEFAULT 1` to `trades` table
- One-time cleanup of 5 ghost `wallet_recovery` placeholder trade rows (from 2026-06-15, prior engine version)
- `token_address` is now included in `save_position` INSERT (was missing — silent data loss bug)

### Added — 5 SOT Wrapper Methods on `PortfolioManager`

These are the SOLE mutation points for positions. Persist to SQLite FIRST, then update in-memory cache on success.

- `open_position(pos, journal)` — validates, persists, updates cache
- `close_position_persist(id, exit_price, notes, journal)` — records trade, removes position
- `adjust_stop(id, new_stop, new_tp1/tp2/tp3/current_price, journal)` — partial field updates with stop-ratchet validation
- `partial_close(id, exit_price, scale_qty, new_scale_level, new_stop, notes, journal)` — TP1/TP2 scale-out, handles full close internally
- `load_from_db(journal)` — engine startup hydration from SQLite

Plus 2 helpers: `build_trade_record`, `build_partial_trade_record` (extracted from existing internal logic).
Plus computed property: `pub fn open_positions(&self) -> usize` (replaces 11 manual assignment sites).

### Added — `BlockReason` for Engine Block State

Replaces the `savant.blocked` text file. Serializes to the dashboard `/api/risk` endpoint.

- `SharedEngineData.block: Arc<RwLock<Option<BlockReason>>>` — in-memory field
- `BlockReason` struct: `block_type`, `reason`, `triggered_at`
- 4 helper methods: `set_block`, `clear_block`, `get_block`, `try_get_block`

### Added — 2 New `ExecutionError` Variants

- `DuplicatePositionId(String)` — detected at open time
- `InvalidStopRatchet { old: f64, new: f64 }` — detected at adjust_stop time (prevents locking in a loss)

### Added — `TradeJournal::load_closed_trades(limit)`

New method to hydrate the in-memory `closed_trades` working set at engine startup. Used by `PortfolioManager::load_from_db`.

### Tests

**+0 net tests** (still 405 lib + 10 dashboard = 415 total). The SOT wrappers have internal logic, but full E2E coverage is deferred to FID-211 when the engine migrates to use them.

### Known Limitations (deferred to FID-211)

- Engine still uses `positions_mut()` and `closed_trades_mut()` (15 + 3 sites) — bypasses the new wrappers
- Engine still writes `savant.blocked` file (3 sites) — block never auto-clears
- API still reads `savant.blocked` file (2 sites) — dashboard shows stale block
- 11 manual `account.open_positions = N` sites still exist
- DexTrader still has parallel `positions`/`closed_trades`/`balance`/`order_counter` fields
- `data/dex_state.json` still written

**Engine behavior is identical to v0.14.9 from a runtime perspective.** The SOT infrastructure is dormant until FID-211 migrates callers.

### Empirical

- 405 lib tests pass (was 399 at v0.14.9 release; +6 from FID-209 rev 2 prep)
- 10 dashboard tests pass
- `cargo clippy --all-targets -- -D warnings` clean
- `cargo build --release` clean (1m 11s)

### Files Changed

```
CHANGELOG.md                                       | v0.14.10 section
Cargo.toml                                         | version 0.14.9 ? 0.14.10
VERSION                                            | 0.14.9 ? 0.14.10
README.md                                          | v0.14.9 ? v0.14.10 + test count
protocol.config.yaml                               | version 0.14.10
config/canary.toml                                 | FID-209: is_anvil = false
config/default.toml                                | FID-209: is_anvil = false
config/test-anvil.toml                             | FID-209: is_anvil = true
src/core/error.rs                                  | +2 ExecutionError variants
src/core/shared.rs                                 | +BlockReason + 4 helper methods
src/execution/portfolio.rs                         | +5 SOT wrapper methods + 2 helpers
src/monitor/journal.rs                             | +migrate_v210 + load_closed_trades
dev/fids/FID-2026-0618-209-spread-filter-testnet-bypass.md | archive
dev/fids/FID-2026-0618-210-single-source-of-truth.md        | archive
dev/fids/FID-2026-0618-210-sqlite-single-source-of-truth.md  | archive
dev/fids/FID-2026-0618-210-state-divergence-block-never-clears.md | archive
dev/fids/FID-2026-0618-210-IMPLEMENTATION-STATUS.md          | archive
dev/vera/notes/repo-audit-2026-06-18.md           | archive
dev/vera/MEMORY.md                                | status updated to v0.14.10
dev/LEARNINGS.md                                  | v0.14.10 session lessons
prompts/gemini-research-repo-audit-2026-06-18.md  | archive
```

## [0.14.9] — 2026-06-18

### Rate-Limit Resilience + Bearish-EMA Veto Fix

Five FIDs addressing the operational bottlenecks observed in the v0.14.8 overnight run (2/169 LLM batches succeeded, 682 rate-limit WARNs, 22/22 high-conviction verdicts defaulting to PASS despite EMA bearish veto not being in the prompt).

### Added — FID-204: 10x NVIDIA API Keys for Per-Juror Rate-Limit Isolation

Overnight burst test empirically confirmed NVIDIA NIM free tier caps at ~5 RPM per model per key (5 successful M3 calls ? 429, 60s recovery). All 10 jurors were sharing one bucket.

- `NvidiaConfig.api_key_envs: Vec<String>` — list of env var names for per-juror keys (default empty, backward-compatible)
- `JuryPool` gains `nvidia_api_keys: Vec<String>` field; juror N (N >= 1) uses `keys[(N-1) % keys.len()]` round-robin
- `load_nvidia_api_keys(config)` — engine startup helper that reads all env vars, skips empty/missing with WARN
- 11 NVIDIA keys stored in `.env` as `NVIDIA_API_KEY` (legacy) + `NVIDIA_API_KEY_1..10` (multi-key)
- 11/11 keys verified working via direct API test; 3/3 new keys confirmed can hit M3 model
- Aggregate capacity: ~5 RPM × 10 keys = ~50 RPM (vs ~5 RPM previously)
- `config/default.toml` and `config/test-anvil.toml` updated with `api_key_envs = [...]`

### Added — FID-205: Per-Model Cooldown on HTTP 429

Herd-retry mitigation. When a model returns 429, mark it cooldown for 60s (±10s jitter) and skip other jurors in that window.

- `JuryPool.model_cooldowns: Mutex<HashMap<String, Instant>>` — tracks active cooldowns
- `is_model_in_cooldown()` — auto-prune expired entries on read
- `mark_model_cooldown()` — adds new entry with deterministic-ish jitter from system time
- `is_rate_limit_error()` — detects 429 in LlmError message
- `JuryPool::models_in_cooldown()` and `models_in_cooldown_count()` — telemetry/dashboard visibility

### Added — FID-206: Bearish-EMA Veto Fix (Long/SHORT/NO_SIGNAL vocabulary)

Per Gemini research 2026-06-18, the LLM's "default to PASS despite non-zero conviction" is not the predicted semantic gravity well — it's the model adding a custom bearish-EMA veto not in the prompt. The 22 verified v0.14.8 PASS verdicts all cited "EMA cross is against" as the reason.

**Three mechanisms identified by research (with citations):**
1. **RLHF financial risk-aversion** — alignment training penalizes confident trading advice
2. **Semantic gravity well** — naming the forbidden action (PASS) primes the model to produce it
3. **Autoregressive exposure bias** — early bearish tokens in reasoning skew later action tokens toward caution

**Fix applied** (per Gemini research recommendations):
- **Reasoning-first JSON schema** — `{"reasoning": "...", "is_probe": false, "conviction_score": 0.45, "action": "LONG"}` (CoT before action prevents premature commitment)
- **Sanitized action vocabulary** — `LONG` / `SHORT` / `NO_SIGNAL` (vs `BUY` / `SELL` / `PASS`). NO_SIGNAL means literally "zero edge" — strips the colloquial "passing" implication that triggered risk-aversion training
- **3 few-shot examples** — Trend Continuation, Contrarian Reversal (KEY EXAMPLE: bearish EMA + oversold Z-score ? LONG via mean-reversion), True Noise
- **Engine-level contradictory signal warning** — when LLM outputs `action=Pass` with `conviction_score > 0.10`, parser logs WARN with pair + conviction (does NOT auto-override; surface the pattern for analysis)
- `TradeAction` enum serde aliases updated to accept LONG / SHORT / NO_SIGNAL / NoSignal / NO-SIGNAL / NO SIGNAL / NOSIGNAL (all map to existing Buy / Sell / Pass variants)
- `normalize_llm_json()` regex updated to recognize the new vocabulary
- `src/agent/prompts/output_format.md` rewritten with FID-206 rules + examples + anti-pattern reminders

### Added — FID-207: LLM Timeout Structured Logging

Engine's batched M3 call (180s cap) now logs `[LLM] TIMEOUT — batch of N pairs produced no verdict after Ns` so overnight-log analyzers can count exactly how many cycles produced no decision (vs other failure modes).

### Added — FID-208: Decision Log + Equity History Cap Raise (500 ? 5000)

- `default_decision_log_max_entries`: 500 ? 5000 (~12 min history ? ~2h history at 3 cycles/min × 14 pairs)
- Equity curve in-memory cap: 500 ? 5000 (same rationale)
- Critical for debugging overnight behavior: v0.14.8 lost decision history at ~22 min into the run

### Changed — Engine Wiring

- Engine startup uses `load_nvidia_api_keys()` to populate jury pool
- Engine wires `nvidia_api_keys: Vec<String>` through to `JuryPool::new()`
- Primary `provider_config_nvidia.api_key` uses first key for fallback path
- Batch LLM call site emits structured `[LLM] TIMEOUT` log on 180s elapse

### Tests

- **+15 new tests** (387 ? 402 total in lib suite). 393 in lib (per `cargo test --lib`), 10 in dashboard.
  - FID-204: `pick_nvidia_key_round_robin`, `pick_nvidia_key_empty_returns_none`, `pick_nvidia_key_single_key_all_jurors_share`, `nvidia_config_default_has_empty_api_key_envs`, `nvidia_config_deserializes_api_key_envs_array`, `nvidia_config_backward_compat_without_api_key_envs_field`, `load_nvidia_keys_multi_key_loads_all`, `load_nvidia_keys_skips_empty_env_vars`, `load_nvidia_keys_legacy_single_key`, `load_nvidia_keys_no_keys_returns_empty`, `load_nvidia_keys_prefers_multi_over_legacy`
  - FID-205: `cooldown_insert_and_check`, `cooldown_expired_auto_clears`, `cooldown_multiple_models_independent`, `is_rate_limit_error_detects_429`
  - FID-206: `long_alias_maps_to_buy`, `short_alias_maps_to_sell`, `no_signal_alias_maps_to_pass`, `no_signal_dash_alias_maps_to_pass`, `contradictory_pass_high_conviction_stays_pass`, `contradictory_short_alias_high_conviction`

### Empirical

- 11/11 NVIDIA keys verified returning 200 OK on small llama-3.1-8b ping
- 3/3 new keys verified can hit M3 model (cold start 20-35s, warm 1-2s)
- Release build clean: clippy `--all-targets -D warnings` passes, `cargo build --release` 22.5s

### Files Changed

- `src/core/config.rs` — `NvidiaConfig.api_key_envs`, `load_nvidia_api_keys()`, `default_decision_log_max_entries 500?5000`
- `src/agent/jury/pool.rs` — `nvidia_api_keys`, `model_cooldowns`, `pick_nvidia_key()`, `mark_model_cooldown()`, `is_model_in_cooldown()`, `is_rate_limit_error()`, `models_in_cooldown_count()`, 4 new tests
- `src/agent/decision_parser.rs` — TradeAction serde aliases (LONG/SHORT/NO_SIGNAL), contradictory signal WARN, 6 new tests
- `src/agent/prompts/output_format.md` — rewritten with FID-206 rules + 3 few-shot examples + anti-pattern reminders
- `src/engine/mod.rs` — `load_nvidia_api_keys()` wiring, FID-207 timeout log
- `config/default.toml` + `config/test-anvil.toml` — `api_key_envs = [...]` array
- `.env` — 10 new NVIDIA_API_KEY_N env vars
- `VERSION` — 0.14.8 ? 0.14.9
- `Cargo.toml` — version 0.14.8 ? 0.14.9
- `protocol.config.yaml` — version 0.14.8 ? 0.14.9
- `README.md` — version + test count
- `CHANGELOG.md` — this section

## [0.14.8] — 2026-06-18

### Multi-Model Jury with NVIDIA NIM Expansion (FID-200)

The single-model bias problem — M3 defaults to PASS on flat markets — is structural, not fixable by prompt engineering. v0.14.8 expands the jury from OpenRouter auto-routing to direct hand-selection of 10 free NVIDIA NIM models.

### Changed — Primary LLM Provider

Switched from TokenRouter (quota-limited) to NVIDIA NIM (free, no quota, same M3 model).

- **Before:** TokenRouter ? M3, weekly quota
- **After:** NVIDIA NIM ? `minimaxai/minimax-m3` (verified, working, ~3s latency)

### Added — 10-Model NVIDIA NIM Jury

Hand-selected free models with vendor/size/capability diversity. Each verified via direct API call against `https://integrate.api.nvidia.com/v1/chat/completions` before shipping.

| Slot | Model | Vendor | Size |
|---|---|---|---|
| 0 | M3 control | MiniMax | MoE (Tiebreaker) |
| 1 | llama-3.3-70b-instruct | Meta | 70B |
| 2 | deepseek-v4-pro | DeepSeek | 1T |
| 3 | nemotron-3-super-120b-a12b | NVIDIA | 120B |
| 4 | llama-3.1-70b-instruct | Meta | 70B |
| 5 | qwen3.5-397b-a17b | Alibaba | 397B |
| 6 | mistral-large-3-675b-instruct-2512 | Mistral | 675B |
| 7 | deepseek-v4-flash | DeepSeek | 1T |
| 8 | glm-5.1 | Z.ai | flagship |
| 9 | kimi-k2.6 | Moonshot | 1T |

**Vendor mix:** 7 different vendors, sizes 70B to 1T params. Single-model bias dilution via diversity.

### Preserved — OpenRouter Fallback

Per Spencer's explicit constraint: OpenRouter path NOT ripped out. When `NVIDIA_API_KEY` is missing or NVIDIA calls fail, the jury falls back to OpenRouter (legacy behavior). Existing `[ai.openrouter]` config and `OPENROUTER_MANAGEMENT_KEY` env var still work.

### Tests

- 2 new unit tests in `pool.rs`: compile-time check for nvidia field, 10-model array validation
- 386 total tests pass (was 384 before)
- Clippy clean, fmt clean, pre-push green

### Verification

Per-model latency: 1-15s. Parallel calls: ~5-10s typical for 10 jurors. Total cycle time budget: 60s (well within).

## [0.14.7] — 2026-06-17

### State Sync (LLM/Jury/Executor on a Single Source of Truth)

After 16h of paper-mode testing producing 0 trades despite 703 PASS decisions, the state-sync issue was identified: the LLM hallucinated positions from its own prior decisions, the jury inherited the hallucinated context, and the executor's outcomes were never communicated to the decision layer. Three coordinated fixes shipped:

### Fixed — Pre-flight Guard (FID-194)
- New `src/agent/pre_flight.rs` with `apply_pre_flight_guard()` function.
- AdjustStop/Close actions get downgraded to `Pass` if the executor has no matching position. Prevents phantom management decisions.
- Single call site at `engine/mod.rs:2844` (the only `parse_decision` call).

### Added — Executor Feedback (FID-195)
- New `TradeStatus` enum (Pending/Filled/Rejected/Expired) on `DecisionEntry`.
- New `update_status()` method marks Pending entries as Filled/Rejected with reason.
- New `format_execution_outcomes()` in `context_builder.rs` shows Filled/Rejected entries with explicit `NO POSITION OPENED` marker.
- Filter in `context_for_pair` excludes Rejected from "Recent Decision Log".
- Jury receives executor's open positions prepended to user message for independent verification.
- All 5 executor call sites (open/close/adjust/place_stop/gasless) call `update_status` on Ok/Err.

### Added — Per-Cycle Reconciliation (FID-196)
- New `apply_to_portfolio()` in `reconciliation.rs` mutates state to match on-chain.
- Clears phantom positions (in memory but not on chain).
- Adds orphan positions (on chain but not in memory).
- Reconciles USDC balance divergence.
- Safety halt at >50% divergence (configurable via `safety_halt_threshold_pct`).
- Telemetry to `data/reconciliation_telemetry.jsonl` per cycle.
- Extends `reconcile_wallet_state` per ECHO Law 13 (one function, one truth).

### Added — Probe Position Mechanism (FID-184)
- New `is_probe: bool` field on `TradeDecision` with `#[serde(default)]`.
- When LLM sets `is_probe: true`: 0.5x sizing + auto-TP at 0.6% from entry.
- Max 3 concurrent probes (tracked via `strategy_name = "probe"`).
- Probe open events logged to `data/probe_pnl.jsonl`.
- Note: Gemini follow-up research (`LLM Crypto Trading Engine Diagnostics.md`) recommends smaller sizing (0.15x) and wider TP (1.2%) for DEX. The 0.5x/0.6% here are placeholders pending Gemini-driven refinement in v0.14.8.

### Changed — Prompt Calibration (FID-198)
- Reconciled 4 conflicting threshold sets in `strategy_knowledge.md` and `output_format.md`.
- Added `is_probe` field to JSON schema with concrete examples.
- Note: Gemini research recommends removing all numerical thresholds from prompts entirely (LLM evaluates narrative, engine gates numerically). This is planned for v0.14.8.

### Infrastructure
- Pre-push validation hook (FID-191): `.git/hooks/pre-push` runs `scripts/pre-push-validation.ps1` (fmt + clippy + tests).
- 380 tests pass (was 354 before session).

## [0.14.6] — 2026-06-17

### Strategy Recalibration (Gemini Deep Research Integration)

Following overnight 16h paper-mode run analysis (96 cycles, 703 PASS, 0 trades), the strategy was recalibrated per Gemini Q1/Q2/Q4/Q7 sniper/scalping recommendations.

### Changed — Conviction Thresholds Lowered (FID-184)

- Trending: 0.20 ? 0.05
- Volatile: 0.25 ? 0.15
- Ranging: 0.25 ? 0.10
- GreyZone: 0.25 ? 0.20 (default-to-PASS retained)

### Fixed — Prompt Anti-Pattern (FID-184)

Removed the "if you cannot compute, output 0.0 and select PASS" instruction. Replaced with: "Output granular probability between 0.00 and 1.00. A score of 0.50 represents absolute uncertainty." This eliminates the default-to-hold bias that produced 87% zero-conviction decisions.

### Added — Cognitive Slippage Penalty (FID-184)

Equity snapshots now apply 0.5%/min latency penalty, capped at 50 bps, when cycle elapsed > 10s. This reflects real-world execution decay from LLM "think" time.

### Fixed — Jury Regime Hardcoding (FID-184)

Jury was hardcoded to `MarketRegime::Ranging`. Now maps session to regime: US-EU Overlap ? Trending, others ? Ranging.

### Changed — Pre-Screening Activated (FID-189)

Set `scan_all_pairs = false` in `config/default.toml` and `config/test-anvil.toml`. This activates the existing pre-scoring at `engine/mod.rs:2052-2120` (FID-056/FID-118) which gates pairs on: RSI extreme, ADX trend, EMA cross, volume spike, BB squeeze. Pairs with no signal no longer reach the LLM.

### Changed — Kelly Sizing 0.5x ? 0.25x (FID-190)

Per Gemini Q1: "0.25x fractional Kelly sizing algorithm based on calculated signal edge to manage maximum drawdowns." Quarter-Kelly provides additional safety margin with limited historical data.

### Added — 0x AMM Price Source (FID-188)

New `src/data/sources/zero_x_price.rs` provides AMM-implied spot price for live trading decisions on Arbitrum, including slippage. Replaces Kraken CEX-derived spot price for live trading. Historical candle data still uses multi-source aggregation (Kraken, OKX, KuCoin, etc.).

### Changed — Log Hygiene (FID-185 + FID-186)

Demoted 8 working-as-designed `warn!` calls to `info!` or `debug!`:
- FID-126 anti-pattern noise ? debug
- FID-096 ZERO-BASE ENFORCEMENT ? info
- Judge fallback (majority vote) ? info
- Jury key threshold ? info
- Jury member timed out ? info
- Jury quorum NOT met ? info
- Context State Delta-compression ? debug

Context State now also writes aggregate metrics to `data/context_state_metrics.json` per cycle (total_compressions, total_tokens_saved, avg_compression_rate).

### Added — Pre-Push Validation Hook (FID-191)

`scripts/pre-push-validation.ps1` runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace --all-targets` before any push. Blocks broken builds from reaching remote. Caught a real fmt violation in `test_e2e_fid160.rs` on first run.

### Deferred — Multi-Chain Architecture (FID-187)

Scoped for v0.15.0. The full per-chain sub-strategy execution (`tokio::spawn` per chain, per-chain state isolation, cross-chain portfolio aggregation) is a multi-week architectural change. FID-188 (0x AMM) and FID-189 (pre-screening) are the v0.14.5-era components that enable the v0.15.0 multi-chain refactor.

### Build & Test

- 354 lib tests pass, 0 clippy warnings, 0 build errors
- Engine running on Anvil paper mode (PID 46608)
- 200-500 trade statistical sample required before live mode (Gemini Q1)

## [0.14.5] — 2026-06-17

### Fixed — start.bat Freezes Kilo CLI (FID-175)

The `Stop-Process -Name node -Force` in the PowerShell cleanup block (line 36) was killing ALL node.exe processes on the machine, including Kilo’s own MCP server processes, freezing the Kilo CLI session. Fix: scoped the kill to only processes whose command line contains `savant`.

### Fixed — dotenvy `.env` Parse Failure on `0X_API_KEY` (FID-176)

Spencer's `.env` had `0X_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c` — a line starting with a digit. The dotenvy parser rejected the entire file as invalid, which caused ALL API keys to be empty at startup, producing 401 errors on every LLM call. Root cause was a stale env var from a prior API key format. Fix: commented out the line (not needed — ZEROEX_API_KEY is used instead) and documented the dotenvy gotcha in `.env.example`.

### Fixed — start.bat Default Config Reverted to Anvil (FID-177)

A prior session accidentally changed `start.bat`'s default from `config/test-anvil.toml` to `config/default.toml`, causing the engine to attempt live mainnet execution. Reverted the default back to `config/test-anvil.toml` (Anvil fork). `SAVANT_CONFIG` may still override at runtime.

### Fixed — Anvil Auto-Start Block cmd.exe Parse Error (FID-178)

The Anvil auto-start conditional block in `start.bat` (lines 97–106) used nested `if/else` with `%SAVANT_CONFIG:anvil=%` string substitution. Under certain invocation patterns this produced `. was unexpected at this time.` from cmd.exe. Fix: removed the inline block and replaced with an unconditional `call start-anvil.bat`. `start-anvil.bat` is already idempotent (detects Anvil at port 8545 before launching).

### Fixed — Re-enabled Jury System (FID-179)

`[ai.jury]` was `enabled = false` in both `config/default.toml` and `config/test-anvil.toml`. Flipped to `enabled = true`. The jury (M3 control + 9 free-model jurors + 70% veto threshold) is a core architectural feature of Savant — multi-model adversarial decision validation. It had been disabled in a prior session due to an incorrect assessment of noise issues; the correct fix is to suppress noise, not disable the system. Uses `OPENROUTER_MANAGEMENT_KEY` for juror provisioning and `TOKEN_ROUTER_API_KEY` for the M3 control juror. No code changes.

### Fixed — Dashboard Layout: Terminal Height + Closed Trades Column (FID-180)

The dashboard grid was `grid-cols-2 grid-rows-[60%_40%]`, which gave Terminal only 40% height and no horizontal room. Updated to `grid-cols-3 grid-rows-[1.2fr_1fr_1fr]` with Terminal in column 3 spanning all 3 rows (`row-span-3`), Closed Trades in column 1, and Activity in column 2. Bumped Closed Trades table row padding from `py-0.5` to `py-1.5` and trade slice from 10 to 30 for better scanability. Dashboard builds clean.

### Fixed — Equity Curve Live Data + Atomic Persistence + Dashboard Layout + Warning Cleanup + WebSocket v2 (FID-181)

Master FID consolidating 4 issues found during the v0.14.5 session:

**Equity curve (Issue A):** The engine cycle never wrote equity snapshots to `state.shared.equity_curve`. Only the backtest engine did. Dashboard was permanently "Collecting equity data…" for all live runs. Fix: added `push_equity_snapshot` at end of each cycle (`src/engine/mod.rs`), `load_equity_history` and `save_equity_history` in `src/core/shared.rs`. Atomic write via `.tmp` + `std::fs::rename`. In-memory cap of 200 snapshots, configurable via `equity_history_max_snapshots`. File at `data/equity_history.json` with versioned format.

**Dashboard layout (Issue B):** Per Spencer, Terminal should be the tall element, not Closed Trades. Confirmed `row-span-3` on Terminal column 3 in `dashboard/src/app/page.tsx`. Grid: `grid-cols-3 grid-rows-[1.2fr_1fr_1fr] gap-1.5 min-h-0`. Note: a stale dashboard server process may still serve the old build; restart the dashboard to pick up the new grid.

**Warning log cleanup (Issue C):** ~61 warn-level lines per cycle were demoted to info or debug:
- Anti-thrashing per-pair (21x/cycle) — debug
- VolRatio=0 for illiquid pairs (21x/cycle) — debug
- GoPlus "no known address" — per-token `HashSet` dedup, logged once per token ever
- Jury parse failures — debug
- Judge fallback message — debug
- DEX stop-losses startup info — info (was warn)

**WebSocket v2 fix (Issue D):** `params.symbol` was a single string (`"XRP/USDT"`), but Kraken v2's `subscribe` method expects a JSON array (`["XRP/USDT"]`). Fix: changed to `json!([symbol])`. The response handler was reading `result.channel` which was null in error responses; now reads the `error` field directly, producing real error messages instead of `"Kraken WS subscribe failed for unknown"`.

### Build & Test

- 354 lib tests passing, 0 clippy warnings, 0 build errors
- Engine running on Anvil paper mode, equity curve collecting snapshots every cycle

## [0.14.4] — 2026-06-16

### Fixed — FID-168/170/171 v2 Strict-Read Improvements (3 FIDs)

**FID-168 v2 (Cycle Snapshot Enrichment):**
Cycle_snapshot now captures regime + ATR + ADX + RSI so the LLM's prompt gets the data it actually asks for. Added `cycle_elapsed` safety check before the summary LLM call (skip if >240s elapsed to avoid the 5-min cycle watchdog). `is_stale()` freshness check now used to force re-summarize every 60s when context is below budget. Corrected pruning math: first pruning at ~10 cycles, not 100.

**FID-170 v2 (Token-Based Stage Splits):**
Replaced count-based `split_into_stages` with `split_into_stages_by_tokens` (greedy fill). Each stage stays under `target_per_stage` tokens regardless of block size distribution. Per-stage `summarize_with_fallback_public` replaces plain `summarize`, giving partial-failure recovery (oversized single blocks get their own stage).

**FID-171 v2 (Handoff Prompt Polish):**
Removed dead `let _ = chunk_size_cap;`. Uses the chunked `summarize_chunks_only` private helper pattern (consistent with FID-170). HANDOFF_INSTRUCTIONS updated with explicit "You are the new LLM" second-person role statement + YOUR ROLE section.

### Build & Test

- 362 tests passing (350 lib + 10 bin + 2 doc), 0 clippy warnings

## [0.14.3] — 2026-06-16

### Added — FID-168: Cycle Summarization Wired Into Engine Loop (Phase 1b)

Engine records per-pair `cycle_snapshot` DataBlocks after each `parse_decision`. At cycle end, prunes old blocks (target: 30% of context window) and summarizes via M3. Historical summary prepended to per-pair user message as "## Historical Summary" block.

### Added — FID-170: Stage-Based Summarization (Phase 2)

Port of openclaw's `summarizeInStages`. Splits history into N stages, summarizes each, merges via final LLM call with trading-specific merge instructions. Opt-in API for v0.15.0.

### Added — FID-171: Handoff Summaries (Phase 3)

Port of openclaw's `summarizeForHandoff`, trading-specific. Briefing for model rotation. Opt-in API for v0.15.0 multi-model rotation.

### Added — FID-172: Engine Restart + Paper-Mode Validation Spec

Pre-flight verified. Engine startup is Spencer's action (via `start.bat`); FID is a validation spec. Spencer runs `start.bat` to launch the engine; Vera writes the validation report from cycle data.

### Build & Test

- 357 tests passing (347 lib + 10 bin + 2 doc), 0 clippy warnings

## [0.14.2] — 2026-06-15

### Fixed — FID-164: Per-Pair ContextState + Token-Based Compression

Singleton `ContextState` was diffing pair N's user message against pair N-1's, producing meaningless ~95% diff ratios. Anti-thrashing then concluded "useless" from corrupted data. Fix: per-pair `HashMap<String, PairState>`, tiktoken-based detection, adaptive threshold, per-pair anti-thrashing, `end_cycle()` cumulative telemetry. 5 new tests.

### Fixed — FID-166: HTTP 504 Streaming Retry + Cycle Timeout

Cycle 17 took 170s due to M3 streaming stalling and HTTP 504 from OpenRouter. 504 added to transient-retry list. `chat_stream` outer retries 2?1. New `streaming_timeout_secs: u64 = 60` with separate `streaming_client: reqwest::Client`.

### Added — FID-167: Multi-Chain Enable (Path A)

`start.bat` default config switched to `config/default.toml`. New `SAVANT_CHAIN` env var (default: ethereum). 5-chain support already coded in `config/default.toml`.

### Added — FID-165: LLM Summarization Phase 1 (Foundation)

Port from openclaw `compaction.ts`. 4 functions: `chunk_by_max_tokens`, `prune_for_context_share`, `summarize_chunks`, `summarize_with_fallback`. Stage-based and handoff deferred to v0.15.0.

### Fixed — FID-163: LLM Data Integrity (4 classes of bugs)

1. `{}` format specifiers replaced all `{:.N}` in LLM-bound paths — byte-faithful data
2. `format_diff` zero-collapse threshold `abs < 0.001` ? `v == 0.0`
3. TSLN serializer `reset()` called per pair — fixes state-bleed
4. 8 missing context blocks added to TSLN path — full parity with legacy JSON path

### Build & Test

- 347 tests passing (325 lib + 10 bin + 2 doc), 0 clippy warnings
