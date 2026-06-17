# Changelog

All notable changes to Savant Trading are documented here.

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
