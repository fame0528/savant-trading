# FID-222.6 — Funnel v1 engine wiring session journal

**Date:** 2026-06-21
**Author:** Vera (substrate: Codebuff-M3), sponsored by Spencer
**Session goal:** Wire `pre_scorer::run_funnel` into Savant Trading engine (FID-222 library was shipped earlier this session but never made v0.15.3 CHANGELOG — caught in this session's audit).

---

## What shipped (FID-222 + FID-222.6 → v0.15.4-alpha)

### Library (FID-222, retroactive CHANGELOG entry)

- `src/strategy/pre_scorer.rs` (~660 lines, 15 inline tests): pub API = `for_regime(regime) -> FunnelWeights`, `compute_signals(last_candle, ind) -> Signals`, `score_pair(pair, last_candle, ind, weights) -> ScoredCandidate`, `select_top_k(scored, k, min_score)`, `run_funnel(input, regime, &FunnelConfig, hunt_mode) -> FunnelResult`. FunnelResult enum = `PassThrough(Vec<String>)` (feature-gate disabled OR HUNT MODE active) or `Filtered(Vec<ScoredCandidate>, FunnelStats)`. NaN bucketing via explicit `is_nan()` check; deterministic alphabetical tie-breaker; empty-input top-3 fallback. Q3 bb weight = 0.00 redistributed to vwap/rsi per FID-222.5 audit.
- `src/strategy/mod.rs`: `pub mod pre_scorer;` registered.
- `src/core/config.rs`: `FunnelConfig { enabled, top_k, min_score_threshold, weights_override: Option<FunnelWeightsTriple> }` + `FunnelWeightsTriple/Fields` + `weights_for(regime) -> FunnelWeights`.
- `src/core/shared.rs`: `pub funnel_v1: Arc<RwLock<FunnelRuntimeState>>` field on SharedEngineData, FunnelRuntimeState struct + Default impl (zeroed snapshot).
- `src/core/types.rs`: `impl Default for IndicatorValues` (all fields None).

### Engine wiring (FID-222.6)

- **API route** `src/api/mod.rs`: `GET /api/funnel/v1` + `get_funnel_v1` handler returning `Json<ApiResponse<FunnelRuntimeState>>`. Mirrors `/api/jury/status` exactly. JWT auth + rate-limit middleware inherited via existing Router ordering.
- **Config** `config/default.toml`: `[trading.funnel_v1]` block, `enabled = false` default, `top_k = 12`, `min_score_threshold = 0.20`. Comments document optional `[trading.funnel_v1.weights_override.{trending,ranging,volatile}]` shape.
- **Tokens.json ingestion** `src/engine/mod.rs::EngineState::new`: append to `active_pairs` AFTER the existing live-execution-gated `extend_token_db` block, gated on `config.trading.funnel_v1.enabled`. Filter: `decimals > 0 AND address.len() == 42 AND not blacklisted AND not already in active_pairs`. Format: `{SYMBOL}/USD`. Required `let mut` fix on `active_pairs` declaration (was `let`).
- **Glue test** `tests/pre_scorer_v1.rs` (~273 lines, 6 tests): `funnel_filters_to_top_k_when_enabled`, `funnel_passes_through_when_disabled`, `funnel_passes_through_when_hunt_mode_active`, `funnel_runtime_state_serializes_via_serde`, `funnel_runtime_state_records_hunt_mode_bypass`, `weights_override_applies_for_all_three_regimes`.

### Docs + version markers

- `CHANGELOG.md`: new `[0.15.4-alpha] - 2026-06-21` section at top documenting FID-222 + FID-222.6 + deferred FID-222.7 items.
- `README.md`: header "v0.15.2" → "v0.15.4-alpha"; cargo test count "475" → "496" (was undercounted at v0.15.3).
- `Cargo.toml`: `version = "0.15.1"` → `0.15.4` (Cargo was lagging at v0.15.1 — drift caught in audit).
- `VERSION`: bumped to `0.15.4`.
- `protocol.config.yaml`: 2 stale version lines (`"0.15.0"` + `"0.1.0"`) → `"0.15.4"`.

---

## What deferred to FID-222.7 (per next-session plan)

### 1. In-loop funnel filter call site

The actual `pre_scorer::run_funnel(...)` invocation from inside the engine's pre-LLM dispatch area. Requires:
- A `funnel_inputs: Vec<CandidateInput>` collection alongside the existing `pair_data_vec.push(...)` inside the `for pair in &active_pairs { ... }` loop (line ~2098-2280).
- A post-loop narrowing pass using `pair_data_vec.retain(|pd| top_k_names.contains(&pd.pair))`.
- Per the thinker's risk #1: any pair with an open `Position` MUST be force-retained BEFORE the `.retain()` narrowing, even if its funnel score is 0.0. Without this guard, a 0-conviction score on a position-holding pair drops it from LLM evaluation and breaks stop-loss adjustments.

The blocker for this is anchoring on the closing brace of the `for pair in &active_pairs` for-pair loop body. The current basher diagnostic returned inconsistent awk output (showed "Closing line: 6307" which is wrong — the loop is much shorter). A precise `sed -n` of lines 2240-2400 is needed to anchor the post-loop injection point.

### 2. JSONL telemetry writer (`dev/logs/funnel-rankings.jsonl`)

Append-only per-cycle file using `tokio::spawn` (thinker risk #2: blocking I/O would stall the cycle). Schema per FID-222.5 Stage 2:
- Top-level: `ts, cycle_id, regime, hunt_mode_bypass, input_N, output_K, threshold_drop, top_k: [{pair, score, signals}], verdict_distribution: {buy, hold, sell, parse_fail, jury_fail}, conviction_distribution: {min, max, mean, std, above_threshold, below_threshold}`.
- Rotation: ring-buffer cap 500 entries; flush audit marker every 50.

### 3. Alphabetical tie-breaker repla