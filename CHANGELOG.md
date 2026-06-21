## [0.15.7-a.1] - 2026-06-21 — FID-225 Phantom-Position Classification (round 1 + round 2 combined)

### Fixed

- **FID-225 round 1** — `WALLET_RECONCILIATION_HALT [real-time]` halting engine on launch when the per-token divergence check fired on any $0.10+ position drift. Root cause: the per-token divergence check was ALIASED to the USDC reconciliation threshold ($0.10), which is appropriate for $50-scale sub-dollar dust detection but WAY too tight for per-token price-feed noise. Fix: decouple into a new `ReconciliationConfig.token_divergence_threshold_usd` field (default $5.00, 50× the USDC threshold).
  - `src/execution/reconciliation.rs`: +new field + `#[serde(default)]` attribute for TOML backward compat (existing `config/{default,canary,test-anvil}.toml` `[reconciliation]` blocks deserialize unchanged; new field falls back to Default::default() = $5.00).
  - `src/engine/mod.rs`: ×2 `recon_helper_cfg` literal sites, plus ×2 sites updated for the descriptor literal.
  - `tests/fid212_close_reconciliation.rs`: `default_cfg()` helper updated to match.

- **FID-225 round 2** — Even with the $5.00 round-1 threshold bumped, the heartbeat STILL halted Spencer's 2026-06-21 03:35 AM Anvil run on cycle 34 with `in-memory=28.33, on-chain=0.000000, div=$5.25` (just above the new $5.00 threshold). Root cause was architectural: the heartbeat treated **stale-state phantom positions** (in-memory says we hold tokens, on-chain has 0) the same as **price-feed drift on real positions**. These are qualitatively different — a phantom signals an aborted swap, prior-session residue, or test artifact. Threshold tuning doesn't fix it; class recognition does.

  - **New pure helper `is_haltable_token_divergence(expected, on_chain, div, threshold) -> bool`** in `src/execution/reconciliation.rs` extracted for testability. Returns `true` only for real drift; `false` for phantoms OR sub-threshold.
  - **Heartbeat refactor** at the per-token `Ok(on_chain_qty)` arm: real drift → existing `divergence` log + push to halt list as before. Phantom → new `WALLET_RECONCILIATION: <pair> PHANTOM_POSITION — in-memory=X, on-chain=0.000000 ...` log + skip push (5-min `ChainPositionRecovery → apply_to_portfolio` is the proper handler, with `safety_halt_threshold_pct = 0.50` as the upper bound for genuine bug detection).
  - **5 regression tests** in `mod tests`:
    - `phantom_position_is_not_haltable_divergence` — pins the exact cycle-34 input at `false` so a future regression that re-introduces phantom halts fails loud
    - `real_drift_above_threshold_is_haltable` — 100 vs 85 tokens @ $1.0 = $15 → `true`
    - `below_threshold_is_not_haltable` — 100 vs 99.5 = $0.5 < $5.00 → `false`
    - `both_zero_is_not_phantom_and_not_haltable` — empty-state no-op edge
    - `near_zero_on_chain_with_large_expected_halts_as_drift_not_phantom` — dust residue (0.001) classifies as real drift, not phantom

### Verified

- `cargo check --lib --tests`: clean
- `cargo clippy --all-targets -- -D warnings`: 0 errors / 0 warnings
- `cargo test --lib execution::reconciliation`: 16/16 pass (5 new phantom + 11 existing)
- `cargo test --test fid212_close_reconciliation`: 3/3 pass (round 1 + 2 do not regress FID-212's StartupCarryover/RealTime classification)
- `cargo test --workspace --all-targets`: 514/514 pass (was 508 in v0.15.7 — +6: round 1 added `token_threshold_is_decoupled_from_usdc_threshold`; round 2 added 5 phantom-classification tests in `is_haltable_token_divergence`)
- `cargo fmt --check`: clean
- `code-reviewer-minimax-m3` (round 1 + 2): APPROVE-with-1-DRY-nit; DRY nit applied post-review (`let is_phantom = ...` bound once, reused in `else if is_phantom`)

### Operational behavior (after hotfix)

Engine launched against a fresh Anvil fork that has accumulated phantom positions from a prior session will now:

1. Log `WALLET_RECONCILIATION: <pair> PHANTOM_POSITION — ...` per phantom position encountered (operator-visible, distinct from real-drift warning)
2. Continue running normally (no halt)
3. Within 5 minutes, `apply_to_portfolio` (FID-196) purges the phantom in `data/reconciliation_telemetry.jsonl`
4. Subsequent cycles see no divergence; engine stays healthy

Real-drift detection (round 1) and phantom classification (round 2) compose: real drift above $5.00 halts; phantoms (any divergence_usd) silently tolerate until 5-min recovery.

### Files Changed

```
VERSION                                  | 0.15.7 → 0.15.7-a.1
Cargo.toml                               | version 0.15.7 → 0.15.7-a.1
README.md                                | title + version badge → v0.15.7-a.1; test count 508 → 514; FID-archive 234 → 235
dev/fids/archive/FID-2026-0621-225-...   | NEW archive doc (round 1 + 2 comprehensive)
src/execution/reconciliation.rs          | +token_divergence_threshold_usd field + #[serde(default)] + is_haltable_token_divergence helper + 5 phantom-classification tests + heartbeat refactor (DRY-nit applied)
src/engine/mod.rs                        | ×2 recon_helper_cfg literal sites + ×2 reconciliation config literal sites
tests/fid212_close_reconciliation.rs     | default_cfg() helper updated with token_divergence_threshold_usd: 5.00
```

### Open Questions (deferred to operator / future FIDs)

- **Threshold default magnitude:** code-reviewer flagged that raising `$5.00 → $25.00` (250× the USDC threshold) would give more Anvil-comfort headroom while still catching real bugs. The classifier (round 2) is the load-bearing fix; threshold magnitude is supplementary defense-in-depth. **Decision trades mainnet signal-vs-noise sensitivity for Anvil comfort.**
- **Phantom source investigation:** why does a fresh Anvil session accumulate phantom positions in the first place? Hypotheses: prior `data/dex_state.json` residue; `ChainPositionRecovery` inserting positions that don't exist; `pre-loop safety guard` force-injection residue from `tests/fid222_7_runtime.rs`. Defensive purge of `data/dex_state.json` on Anvil startup would be the simplest fix; **operator-routed**.
- **Anvil end-to-end smoke test:** extend `tests/fid212_close_reconciliation.rs` with a stub-RPC test that drives `reconcile_wallet_state` with phantom positions to empirically verify the heartbeat continues normally. The unit tests cover the classifier; an integration test against Anvil would prove end-to-end.

---

## [0.15.7] - 2026-06-21 — Build-warning cleanup + dashboard layout fix + production-readiness audit

### Added

- **Dashboard terminal full-width strip** (`dashboard/src/app/page.tsx`): Terminal cell moved from `row-span-3` (1-col tall) to `col-span-3` (3-col wide bottom row). Bento grid template extended: `grid-rows-[1fr_1fr_1fr]` → `grid-rows-[1fr_1fr_1fr_2fr]` to make room for the new bottom row. The terminal log stream now spans all 3 columns with ~40% of bento vertical space — the layout operator-requested fix. Next.js 16.2.7 + Turbopack build clean (2.1s).

### Fixed

- **Build noise reduced 7 → 0 warnings.** Three cleanup passes:
  1. **`Cargo.toml`**: removed redundant `license = "Proprietary"` line. Cargo accepts `license` (SPDX expression) OR `license-file` (path) but not both — we ship the `LICENSE` file as source-of-truth so `license-file` is correct. Replaced with a comment explaining the choice.
  2. **`src/strategy/pre_scorer.rs`**: added `#[allow(non_snake_case)]` to **3 sites** that preserve the math-notation **N** (universe size) and **K** (top-K count) identifiers — `FunnelStats` struct + `FunnelRankingRecord` struct + `FunnelRankingRecord::build` method. Both structs are Serialize-derived and serialize into `dev/logs/funnel-rankings.jsonl` JSONL telemetry; renaming `input_N` → `input_n` would have flipped the JSONL schema key from `"input_N"` to `"input_n"`, breaking downstream readers.
  3. **`[v0.15.6 hotfix] src/strategy/pre_scorer.rs`**: already-removed unused**: previously-removed unused `use crate::core::shared::FunnelRuntimeState;` inside `record_funnel_heartbeat` body (L644). The identical-looking import in `record_funnel_runtime` (L596) was preserved — bound to `*state_guard = FunnelRuntimeState { ... }` in the PassThrough branch.

### Verified

- `cargo check --lib`: clean (0 warnings, was 6)
- `cargo check --tests`: clean (0 warnings)
- `cargo test --lib strategy::pre_scorer::tests::`: 15/15 pass (pre_scorer unit tests unaffected)
- `dashboard build`: clean (Next.js 16.2.7 + Turbopack 2.1s)

### Documentation Audit (production-readiness pass)

This release includes a comprehensive audit pass over `/dev/`, `/docs/`, `/prompts/`, and the README/CHANGELOG surface:

- **`dev/HANDOFF.md`**: refreshed version + last-updated markers (was stale at "v0.14.1 / 2026-06-14"); historical 2026-06-20 FID-219+ additive update preserved per DECISION-009 (append-only audit trail).
- **`dev/AUDIT.md`**: added "HISTORICAL — superseded" banner pointing readers to current sources (`MASTER-FID.md`, `dev/fids/archive/`, `dev/vera/`). The v0.8.0 Nova audit itself is preserved (findings 1-17, NF-01-12) for historical record but most are addressed.
- **README.md**: title + version badge updated v0.15.6 → v0.15.7 (badge was very stale at `0.14.8`).
- **VERSION + Cargo.toml**: bumped 0.15.6 → 0.15.7.
- **/dev/NEEDS.md**, **`/dev/SNIPE-TRANSCRIPT-PROCESSING.md`**: examined; both remain historical references for FID-045 multi-chain (deferred to FID-187) and snipe-transcript processing workflow respectively. Neither requires update for the v0.15.7 ship.

### Backlog (carried forward, ranked)

- **FID-222.9** — Real cycle regime from `RegimeDetector.observe(market_stores)` (ADX/vol signals), not hardcoded `MarketRegime::Trending`.
- **FID-222.10** — Combine `record_funnel_runtime` + `record_funnel_heartbeat` into a single `shared.funnel_v1.write().await` to fix lock-order discipline.
- **FID-222.11** — Replace `std::thread::spawn(move || pre_scorer::append_funnel_jsonl(...))` inside async fn with `tokio::task::spawn_blocking` to avoid blocking the tokio runtime.
- **FID-219+ negative-path smoke** — empirically verify the `chains.arbitrum.enabled = false` hard-halt path produces `savant.blocked` + `shared.set_block` correctly. Deferred: blocked by stale Next.js `EADDRINUSE :::3000` (the source-pattern tests in `tests/fid219_reconciliation_shared_client.rs` cover the code path; smoke test would only confirm runtime).
- **FID-180 followup #2** — JurySection at Row 3 Col 1 width-constrained; its internal `grid-cols-4` collapses to vertical stack. Add `col-span-3` for full-width strip matching FID-162 comment intent (deferred to operator preference).
- **FID-187 Multi-Chain Execution** — per-chain sub-strategy execution (`tokio::spawn` per chain, per-chain state isolation, cross-chain portfolio aggregation). Deferred from v0.14.6 as multi-week architectural change.
- **Test count + FID-archive count in README** — README `cargo test # 502` + `214 archived FIDs` not updated in this audit because exact counts require full `cargo test` run + `find dev/fids/archive | wc -l` and were not verified. Run verifications post-push if exact figures are needed.

### Files Changed

```
VERSION                                  | 0.15.6 → 0.15.7
Cargo.toml                               | version 0.15.6 → 0.15.7 + license field cleanup
README.md                                | title + version badge → v0.15.7
CHANGELOG.md                             | this section prepended
dev/HANDOFF.md                           | refresh version + last-updated markers
dev/AUDIT.md                             | historical banner added
dashboard/src/app/page.tsx               | grid-rows extension + Terminal col-span-3 (replacing row-span-3)
src/strategy/pre_scorer.rs               | 3 × #[allow(non_snake_case)] attributes
```

### Archived FIDs (v0.15.7 readiness pass)

Seven FIDs from the v0.15.2 → v0.15.7 funnel-v1 + chain-default + Anvil-fix cycle are now in `dev/fids/archive/`. The FID-TEMPLATE convention "When status is set to **Closed**, move this file to `dev/fids/archive/`" was applied for each.

| FID | Filename | Severity | Ship / Fixed in |
|-----|----------|----------|-----------------|
| **FID-213** | [`archive/FID-2026-0620-213-anvil-fresh-startup-balance-override.md`](dev/fids/archive/FID-2026-0620-213-anvil-fresh-startup-balance-override.md) | High | v0.15.3 — Anvil fresh-startup balance override (`starting_balance` adopted over chain-truth); 6 regression tests in `tests/fid213_anvil_balance_init.rs` |
| **FID-213.aux** | [`archive/FID-2026-0620-213-fid-anvil-boot-perf.md`](dev/fids/archive/FID-2026-0620-213-fid-anvil-boot-perf.md) | Medium | v0.15.3 — 17-concurrent `join_all` Anvil-boot perf fix (reduced boot 35-60s → <8s) |
| **FID-219-sav** | [`archive/FID-2026-0620-219-savant-chain-default-arbitrum.md`](dev/fids/archive/FID-2026-0620-219-savant-chain-default-arbitrum.md) | Medium | v0.15.6 — `SAVANT_CHAIN` default flipped `"ethereum"` → `"arbitrum"` (FID-219 GREEN phase 4 root cause) |
| **FID-219+** | [`archive/FID-2026-0620-219plus-defensive-enabled-flag-guard.md`](dev/fids/archive/FID-2026-0620-219plus-defensive-enabled-flag-guard.md) | Medium | v0.15.6 — Defensive `enabled`-flag guard (savant.blocked + shared.set_block + soft-skip warn); 8 regression tests in `tests/fid219_reconciliation_shared_client.rs` |

[^aux-inclusion]: FID-213.aux (the Anvil-boot-perf sibling of FID-213) appears above as historical context — not enumerated in the original 7-FID archive list but already in `dev/fids/archive/` since v0.15.3 and cleared alongside FID-213 during this readiness pass. Ship/severity column left at v0.15.3 / Medium.
| **FID-222** | [`archive/FID-2026-0620-222-funnel-v1-momentum-pre-scorer-top-k.md`](dev/fids/archive/FID-2026-0620-222-funnel-v1-momentum-pre-scorer-top-k.md) | Medium | v0.15.4-alpha — Funnel v1 momentum pre-scorer + top-K selector LIBRARY surface (`src/strategy/pre_scorer.rs`, 660 LOC, 15 inline tests) |
| **FID-222.6** | [`archive/FID-2026-0620-222.6-funnel-v1-engine-wiring.md`](dev/fids/archive/FID-2026-0620-222.6-funnel-v1-engine-wiring.md) | Medium | v0.15.4-alpha — Engine wiring layer (tokens.json ingestion into `active_pairs`; `/api/funnel/v1` API route; HUNT MODE bypass); 6 integration tests in `tests/pre_scorer_v1.rs` |
| **FID-222.7** | [`archive/FID-2026-0620-222.7-funnel-v1-runtime-integration.md`](dev/fids/archive/FID-2026-0620-222.7-funnel-v1-runtime-integration.md) | Medium | v0.15.6 — Runtime integration layer (`record_funnel_runtime` + `record_funnel_heartbeat` + `append_funnel_jsonl` + `FunnelRankingRecord` schema); 6 integration tests in `tests/fid222_7_runtime.rs` |
| **FID-222.8** | [`archive/FID-2026-0620-222.8-funnel-v1-pre-loop-collection.md`](dev/fids/archive/FID-2026-0620-222.8-funnel-v1-pre-loop-collection.md) | Medium | v0.15.6 — Pre-loop collection layer (`funnel_inputs: Vec<CandidateInput>` collection; open-position safety guard force-injection); 8 integration tests in `tests/fid222_7_runtime.rs` (6 + 2 safety-guard specific) |

**Note on commit SHAs:** All v0.15.2 → v0.15.7 work is **uncommitted in working tree** at v0.15.1 base commit `49ed7ca4`. Each archive doc's `Commit/PR:` line reads "pending — Spencer reviews + commits + pushes before v0.15.7 ships." This is the precision-verified state at archive-time per ECHO Law 4.



## [0.15.5] - 2026-06-21

## [0.15.6] - 2026-06-21 — FID-222.7 + FID-222.8 + FID-219

### Added
- **FID-222.7** — Funnel v1 pre-scorer runtime wiring. Adds `run_funnel` + `record_funnel_runtime` + `record_funnel_heartbeat` to the main cycle loop via the A4 post-pair data block. Each cycle now produces `FunnelRankingRecord` JSONL rows at `dev/logs/funnel-rankings.jsonl` for downstream analysis.
- **FID-222.8** — Pre-loop funnel wiring. `funnel_inputs: Vec<CandidateInput>` is collected in the for-pair loop (A3a clone + A3b push) and consumed in A4. Open-position safety guard force-injects pairs not in top-K so held positions survive scoring.
- **FID-219+** — Defensive `enabled` flag guard on chain-driven code paths. The 5-min chain-sync (FID-155) SOFT-SKIPS when `chain_cfg.enabled == false`; the heartbeat (FID-154) HARD-BREAKS via `shared.set_block(BlockReason { block_type: "chain_disabled", ... })` AND writes `savant.blocked` for operator resumption.

### Changed
- `SAVANT_CHAIN` default fallback in heartbeat + 5-min sync flipped from `"ethereum"` to `"arbitrum"` (FID-219 GREEN phase 4 regression anchor — prevented Anvil/RPC parse failures from chain mismatch).
- `MarketRegime::Trending` hardcoded as cycle regime in A4 with `// TODO(FID-222.9)` marker — funnel weights default to momentum-heavy pending RegimeDetector integration.
- `record_funnel_heartbeat` now called AFTER `record_funnel_runtime` so the API's `last_run_at` reflects the full funnel cycle (fixes v0.15.5 stale-state issue).

### Fixed
- **FID-219 (root cause)** — `SAVANT_CHAIN` default was `"ethereum"` in both 5-min sync and heartbeat paths; runtime fell back to chain_id 1 (Ethereum mainnet) against the Arbitrum fork, surfacing as `rpc parse: error decoding response body`. Both paths now default to `"arbitrum"`.
- **FID-219+** — Operators who set `SAVANT_CHAIN=<name>` against `chains.<name>.enabled = false` were silently probing disabled chains. Heartbeat now hits `set_block` AND writes `savant.blocked`; 5-min sync soft-skips with throttle.

### Tests
- All 8 `tests/fid219_reconciliation_shared_client` regression tests PASS.
- All 6 `tests/fid222_7_runtime` funnel wiring tests PASS.
- `cargo check --lib` + `cargo check --tests`: clean (only pre-existing snake_case warnings on `input_N`/`output_K` fields in `src/strategy/pre_scorer.rs`).

### Backlog
- **FID-222.9** — Real cycle regime from `RegimeDetector.observe(market_stores)` (ADX/vol signals), not hardcoded `MarketRegime::Trending`.
- **FID-222.10** — Lock-order discipline in `src/strategy/pre_scorer.rs` — `record_funnel_runtime` + `record_funnel_heartbeat` should both grab `shared.funnel_v1.write().await` in one combined call.

### Added (FID-222.7: Funnel v1 production wiring)

- src/strategy/pre_scorer.rs: FunnelRankingRecord + RankedCandidate + append_funnel_jsonl + record_funnel_runtime (3 new public APIs, ~150 LOC)
- src/engine/mod.rs: post-loop funnel runner block (~120 LOC) gating on trading.funnel_v1.enabled with belt-and-suspenders open-position safety guard
- src/engine/mod.rs: imports added for pre_scorer::{CandidateInput, FunnelRankingRecord, FunnelResult}
- tests/fid222_7_runtime.rs: NEW 6-test integration test (Filtered/PassThrough/orphaned math/JSONL writer/safety-guard pattern/empty-indicators force-injection)
- versions bumped: Cargo.toml, VERSION, protocol.config.yaml, README to 0.15.5; test count 496 -> 502

### Behavior

- Pre-existing FID-222.6 library + API route + TOML config + tokens.json ingestion remain backward-compatible
- Funnel is OFF by default (config.trading.funnel_v1.enabled = false); operators should leave disabled until FID-222.8 closes the deferred nits
- Open positions survive funnel narrowing via two-layer guard (post-loop force-injection + retain-pair-OR-positioned predicate)
- JSONL telemetry at dev/logs/funnel-rankings.jsonl (FID-222.5 Stage 2 schema); non-blocking append via tokio::spawn

### Deferred to FID-222.8 backlog

- Outer `let mut funnel_inputs` decl move to colocate with post-loop runner (~620-line gap currently)
- `_orphaned_retained` parameter on `record_funnel_runtime` is dead (passed but unused); either drop or wire to FunnelRuntimeState
- Force-injected candidates inflate FunnelStats::threshold_drop; recommend `force_injected: bool` on RankedCandidate
- `record_funnel_runtime` should update last_run_at heartbeat when feature is disabled so /api/funnel/v1 shows fresh state

# Changelog

All notable changes to Savant Trading are documented here.

## [0.15.4-alpha] - 2026-06-21

### Funnel v1 library + engine wiring (FID-222 + FID-222.6)

Funnel v1 ships as a **feature-gated, default-OFF** momentum pre-scorer + top-K selector. The library extracts a 6-signal composite score per pair, sanitizes NaN/Bool/bounds, and surfaces a top-K list that the engine can use to narrow the LLM dispatch universe. Wiring is opt-in: set `[trading.funnel_v1].enabled = true` in any config to engage.

### Added — Library (FID-222)

- **`src/strategy/pre_scorer.rs`** (NEW, ~660 lines) — pure-Rust library module with public API: `for_regime`, `score_pair`, `select_top_k`, `compute_signals`, `run_funnel`. FunnelResult enum (PassThrough/Filtered), scores ∈ [0.0, 1.0] with explicit NaN sanitization, deterministic alphabetical tie-breaker, empty-input top-3 fallback. 15 inline tests in `#[cfg(test)] mod tests`.
- **`src/strategy/mod.rs`** — `pub mod pre_scorer;` registered.
- **`src/core/config.rs`** — `FunnelConfig` + `FunnelWeightsTriple` + `FunnelWeightsFields` + `weights_for(regime) -> FunnelWeights` impl. `pub funnel_v1: FunnelConfig` field on `TradingConfig` with `#[serde(default)]` for backward compat.
- **`src/core/shared.rs`** — `pub funnel_v1: Arc<RwLock<FunnelRuntimeState>>` field on `SharedEngineData`. FunnelRuntimeState struct with `enabled_at_last_cycle, last_universe_post_hygiene, last_top_k_size, last_top_score, last_min_top_score, last_regime, last_run_at, hunt_mode_bypass, disabled_reason` fields, derives `Serialize`. Default impl produces a zeroed snapshot.
- **`src/core/types.rs`** — `impl Default for IndicatorValues` (all fields `None`) so the engine can construct zero-state indicators without a panic when candle data is missing.
- **AUDIT Q3/Q4/Q5 corrections** honored in library: bb weight = 0.00 across all three regimes (the 0.05 redistributes to vwap), NaN bucketing in `sort_key` (NOT `partial_cmp().unwrap_or`), `run_funnel(..., hunt_mode: bool)` is a local caller-driven primitive (NOT a global lock read).

### Added — Engine Wiring (FID-222.6)

- **`src/api/mod.rs`** — new `GET /api/funnel/v1` route + `get_funnel_v1` handler (mirrors `/api/jury/status` pattern). Returns `Json<ApiResponse<FunnelRuntimeState>>` reading `state.shared.funnel_v1.read().await.clone()`. Inherits JWT auth + rate-limit middleware via the existing Router ordering.
- **`src/engine/mod.rs`** — new imports: `load_token_store`, `pre_scorer::{self, CandidateInput, FunnelResult}`. `let mut active_pairs` (was `let` — fix needed for new ingestion block). **Tokens.json ingestion block in `EngineState::new`**: when `config.trading.funnel_v1.enabled`, appends pairs from `data/tokens.json` after the existing live-execution-gated `extend_token_db` block, filtering `decimals > 0 AND address.len() == 42 AND not blacklisted AND not already in active_pairs`, formatted as `{symbol}/USD`. Failure mode: token-store parse error → WARN log + continue (no abort).
- **`config/default.toml`** — new `[trading.funnel_v1]` block with `enabled = false` (default), `top_k = 12`, `min_score_threshold = 0.20`. Comments document optional `[trading.funnel_v1.weights_override.{trending,ranging,volatile}]` shape with per-regime custom weights (must sum to 1.0).
- **`tests/pre_scorer_v1.rs`** (NEW, ~273 lines, 6 integration tests): funnel_filters_to_top_k_when_enabled, funnel_passes_through_when_disabled, funnel_passes_through_when_hunt_mode_active, funnel_runtime_state_serializes_via_serde, funnel_runtime_state_records_hunt_mode_bypass, weights_override_applies_for_all_three_regimes.

### Deferred to FID-222.7 (planned per next-session continuation)

The following 3 wiring tasks are NOT in this alpha and require a follow-up FID with verbatim line-number reads for the `for pair in &active_pairs` pair_data_vec loop body:

1. **In-loop funnel filter call site** — the actual `pre_scorer::run_funnel(...)` invocation from inside `src/engine/mod.rs`'s pre-LLM dispatch area. Requires a separate `funnel_inputs: Vec<CandidateInput>` collection alongside `pair_data_vec.push(...)` in the existing for-pair loop, plus a post-loop narrowing pass that retains top-K pairs. Auditor Q-A cite-correction also flagged ("Q-A correction: impl lands in EngineState::new() at/after the extend_token_db block (~186–220), NOT at :870. Append to active_pairs, NOT replace").
2. **JSONL telemetry writer (`dev/logs/funnel-rankings.jsonl`)** — append-only per-cycle file with the FID-222.5 Stage 2 schema: `ts, cycle_id, regime, hunt_mode_bypass, top_k [{pair, score, signals}], verdict_distribution, conviction_distribution, input_N, output_K, threshold_drop`. Should be `tokio::spawn`-ed to avoid blocking the cycle (thinker risk #2).
3. **Orphaned-open-position safety guard (thinker risk #1)** — force-retain any pair with an open `Position` in `pair_data_vec` BEFORE the `.retain()` narrowing step, even if its funnel score is 0.0. Without this, a 0-conviction score on a position-holding pair would drop it from LLM evaluation, breaking stop-loss adjustments.

### Fixed — AUDIT Q3 Weight Redistribution

- `Ranging.bb: 0.05 → 0.00` + `Ranging.rsi: 0.30 → 0.35` (per FID-222.5 Q3: redistribute bb's 0.05 to vwap/rsi so per-regime weights sum to 1.0).
- `Volatile.bb: 0.15 → 0.00` + `Volatile.adx: 0.10 → 0.25` (same rationale).
- `Trending.bb: 0.00` (already correct at alpha start).

### Tests

- 21 new tests (+15 inline in `src/strategy/pre_scorer.rs` lib + 6 integration in `tests/pre_scorer_v1.rs`).
- Total: **496 tests** passing (was 475 at v0.15.3 = 416 lib + 47 integration + 10 main binary + 2 doc; +15 lib inline pre_scorer + 6 integration pre_scorer_v1).

### Empirical

- `cargo check --lib`: clean (only pre-existing non-snake-case warnings on `input_N`/`output_K`).
- `cargo test --lib strategy::pre_scorer`: 15/15 pass.
- `cargo test --test pre_scorer_v1`: 6/6 pass.
- FunnelRuntimeState serde round-trip verified.
- HUNT MODE bypass returns PassThrough (preserves FID-063 intent, per FID-222 Loop 1.7 Q4).

### Files Changed

```
CHANGELOG.md                          | v0.15.4-alpha section (this entry)
Cargo.toml                            | version 0.15.3 → 0.15.4
VERSION                               | 0.15.3 → 0.15.4
README.md                             | v0.15.2 → v0.15.4-alpha + test count 475 → 496
protocol.config.yaml                  | version 0.15.4-alpha
config/default.toml                   | +[trading.funnel_v1] block (enabled=false)
src/api/mod.rs                        | +FunnelRuntimeState import + /api/funnel/v1 route + get_funnel_v1 handler
src/strategy/pre_scorer.rs            | NEW module (660 lines)
src/strategy/mod.rs                   | +pub mod pre_scorer;
src/core/config.rs                    | +FunnelConfig + FunnelWeightsTriple + FunnelWeightsFields + weights_for + TradingConfig.funnel_v1
src/core/shared.rs                    | +funnel_v1: Arc<RwLock<FunnelRuntimeState>> field + struct + Default impl
src/core/types.rs                     | +impl Default for IndicatorValues
src/engine/mod.rs                     | +tokens.json ingestion block + let mut active_pairs + 3 new imports
tests/pre_scorer_v1.rs                | NEW (6 integration tests)
dev/fids/archive/FID-2026-0620-222-... | archive after release
dev/vera/memory/2026-06-21-funnel-wiring.md | handoff journal
```

## [0.15.3] - 2026-06-20

### Fixed
- **FID-213:** Anvil fresh-startup balance override. Engine now adopts `starting_balance` over chain-truth on Anvil fresh-startup (when no persisted state file exists), emits a single audit ledger line showing adopted vs. chain-reported (warn at >$0.10 drift, info otherwise). `save_state()` now surfaces errors via warn! macro instead of silent `.ok()` to prevent disk-failure regressions. Fixes the phantom-$-50-at-fresh-restart operator reported on 2026-06-20.

### Tests
- Added `tests/fid213_anvil_balance_init.rs` (6 tests): covers helper-fn, struct marker presence, override-block structural presence, save_state ordering, threshold check presence, log-message audit. cargo baseline: 469 → 475.

### Docs
- `dev/fids/archive/FID-2026-0620-213-anvil-fresh-startup-balance-override.md` — full FID archive.
- `dev/LEARNINGS.md` — session row with 5 lessons.
- `README.md` — cargo count banner: 469 → 475.
- `VERSION` — bumped 0.15.2 → 0.15.3.

## [0.15.1] � 2026-06-19

### Engine Migration Completion (FID-211 Stage 2) + WalletKey Hardening + Shared Block State

The v0.15.0 release shipped the SOT wrapper infrastructure but left the engine partially migrated. v0.15.1 completes the migration, hardens wallet-key handling, and adds the typed in-memory block state. Also archives 5 stale FIDs from v0.14.7�v0.14.8 and adds 19 new integration tests covering the engine SOT contract, JuryKeyManager Drop semantics, and the engine startup sync path.

### Added

- **`record_closed_trade_sync` SOT wrapper** on `PortfolioManager` � atomic close-trade write that persists to SQLite FIRST, then appends to the in-memory cache on success. Differs from `close_position_persist` (which also calls `delete_position`): this wrapper is for the case where the position was already removed out-of-band by the executor (the external-close path in the engine). 4 new lib tests cover happy path, no-match fallback, multi-match, and empty-cache cases.

- **`remove_synced_closed_trade` SOT wrapper** on `PortfolioManager` � cache-only revert for phantom TradeRecords where the on-chain close failed before reaching the executor. Returns `true` if a matching trade was found and removed, `false` otherwise (so the engine can alert on "expected phantom but none found").

- **`shared.block` typed in-memory state** (FID-210/211) � the engine now sets a typed `BlockReason { block_type, reason, triggered_at }` in addition to writing the `savant.blocked` file. The file remains the crash-survived SOT; `shared.block` is the in-memory cache that the API reads (no more file I/O per `/api/risk` request). 7 new integration tests in `tests/shared_block_state.rs` cover set/get/clear semantics, concurrent-writer try_get, and JSON round-trip stability.

- **3 new integration test files** (FID-211 audit Finding 2.1 closure):
  - `tests/key_manager_drop.rs` (6 tests) � proves `JuryKeyManager::drop` does not panic inside tokio runtimes (the v0.14.10 crash class)
  - `tests/startup_sync.rs` (6 tests) � `PortfolioManager::load_from_db` edge cases including the FID-211 Bug 10 regression (token_address column survival)
  - `tests/shared_block_state.rs` (7 tests) � typed in-memory block state contract

### Changed

- **Engine `closed_trades` migration** � 3 of 8 production `closed_trades_mut` / `positions_mut` call sites in `src/engine/mod.rs` migrated to the new SOT wrappers (sites 5108, 5157, 5668). The remaining 4 sites (1454, 4786, 4895, 5795) are `else` branches of `if let Some(ref j) = journal` where `journal` is always `Some` in production (verified at `main.rs:840, :974`); these are dead-code last-resort fallbacks and are explicitly NOT the dual-write bug class. Documented in FID-211 re-audit.

- **`savant.blocked` ? `shared.block` migration** � 4 circuit-breaker write sites in `src/engine/mod.rs` (lines 3671, 3801, 1538, 1564) now write the file AND call `shared.set_block` (write-through). Midnight auto-clear (line 1606-1621) and startup clear (`main.rs:352`) now also call `shared.clear_block`. The API `/api/risk` status endpoint reads from `shared.block` (no file I/O per request); `/api/risk/clear-block` clears both layers. Dashboard `block_reason: string` field preserved for backward compat with the existing dashboard regex parser; new structured `block: object` field added for new consumers.

- **`wallet_key: String` ? `WalletKey` newtype** (FID-211 audit Finding 1.1) � 7 production sites migrated (`engine/utils.rs:72, 135`, `main.rs:617, 677, 949`, `bin/test_e2e_fid160.rs:28`, `bin/test_swap.rs:18`, plus 2 re-audit finds: `engine/mod.rs:365` startup address cache, `api/mod.rs:842` get_wallet fallback). `expose_secret()` is the only way to read the secret and is called only at the signing-key + DexTrader::new sites. The compiler now enforces the type-safe contract � no raw `String` for wallet keys anywhere in `src/`.

- **3 remaining `let _ = j.X` fire-and-forget patterns** converted to `if let Err(e) = ... { warn!(...) }` (no silent failures). Sites: `main.rs:1238` (delete_position in emergency_liquidate), `engine/mod.rs:3864` (record_activity close), `engine/mod.rs:4593` (record_activity open). The two `record_activity` sites are audit-log writes, not trade-data SQLite.

- **Bug 10 fix from v0.15.0** � `src/monitor/journal.rs:223` `load_positions` SELECT statement now includes the `token_address` column. Regression test added in `tests/startup_sync.rs::load_positions_selects_token_address_column`.

### DEFERRED with Architectural Finding

- **Stage 2 Item 5: `positions_mut()` / `closed_trades_mut()` to `pub(crate)`** � The handoff stated "verify engine is in the same crate. It is (`crate-type = ["lib", "bin"]`)." The handoff was wrong: the engine is in the `savant` binary crate (`src/main.rs:15 mod engine;`), not the `savant-trading` library crate (no `pub mod engine;` in `lib.rs`). Tightening to `pub(crate)` would block the engine from accessing the methods. Three options documented in FID-211 re-audit; Option 1 (move engine into library) is the right architectural move but is its own FID worth of work. The wrappers (`open_position`, `close_position_persist`, `adjust_stop`, `adjust_quantity`, `remove_synced_position`, `sync_from_db_position`, `record_closed_trade_sync`, `remove_synced_closed_trade`) all ship in v0.15.1 and are used by the engine migrations; visibility tightening requires the engine-in-library refactor first.

### FID Archive (Stage 2 Item 6)

Five FIDs from the v0.14.7�v0.14.8 cycle that were documented as "shipped" but never moved to `dev/fids/archive/`. All confirmed shipped via `git log --grep`:

| FID | Title | Shipped in |
|-----|-------|------------|
| 193 | State Sync � LLM/Jury/Executor Team on a Single Source of Truth | v0.14.7 (0f26b533) |
| 194 | Pre-flight guard against phantom management | v0.14.7 (b207b9e8) |
| 195 | Executor reports fill/reject, execution outcomes in LLM context | v0.14.7 (ef606667) |
| 196 | Per-cycle reconciliation with USDC + safety halt + telemetry | v0.14.7 (1fda8db5) |
| 200 | Multi-model jury expansion (10 NVIDIA NIM models) | v0.14.8 (f08cd8ca) |

All 5 FIDs moved from `dev/fids/` to `dev/fids/archive/` with `Status: closed` and a `Resolution:` line linking to the commit that shipped them.

### Verification

- `cargo test --lib` � 416 passed (was 412, +4 for the new closed_trades wrappers)
- `cargo test --tests` � 38 integration tests passed (was 19 from v0.15.0, +19: 6 key_manager_drop + 6 startup_sync + 7 shared_block_state)
- `cargo clippy --all-targets -- -D warnings` � clean
- `cargo build --all-targets` � clean
- Total: **464 tests passing (416 lib + 38 integration + 10 main binary), zero warnings**

## [0.15.0] � 2026-06-19

### Engine Migration to SOT Wrappers + Runtime Panic Fix + State Carryover Fix (FID-211)

Full engine migration to the v0.14.10 SOT wrappers. This version fixes the v0.14.10 overnight crash (runtime nesting panic + state carryover halt) and adds wallet-key security via SecretBox. v0.14.10 shipped the SOT infrastructure but did NOT migrate the engine callers � every position mutation still went through `positions_mut()` + fire-and-forget SQLite, which is the same data-integrity hole FID-210 was supposed to fix. v0.15.0 wires the engine to use the new wrappers.

### Fixed � CRITICAL

- **Runtime nesting panic in `JuryKeyManager::drop`** (`src/agent/jury/key_manager.rs:263-300`). The previous Drop impl called `Handle::block_on(async { ... })` which panics with "Cannot start a runtime from within a runtime" when the drop fires from inside a tokio runtime (always the case for the engine). Fix: Drop is now a no-op; orphan keys are cleaned up at startup via `cleanup_orphaned_keys`.

- **State carryover divergence halts engine on first cycle after fresh Anvil restart**. The reconciliation halted when in-memory balance ($49.97 from prior run) diverged from chain ($0 from fresh Anvil), but didn't distinguish startup carryover from real-time divergence. Fix: new `DivergenceType` enum (`None`, `StartupCarryover`, `RealTime`) � startup carryover adopts chain as truth on Anvil, errors + requires `--reset-state` flag on live chain. Only `RealTime` divergence halts.

### Added

- **`DivergenceType` enum** in `src/execution/reconciliation.rs` � classifies reconciliation divergence for safer halt-or-recover decisions.

- **`adjust_quantity` SOT wrapper** on `PortfolioManager` � atomic qty update that writes to SQLite FIRST, then in-memory on success. Replaces fire-and-forget pattern at `engine/mod.rs:1418`.

- **`sync_from_db_position` + `remove_synced_position` + `clear_position_cache` wrappers** on `PortfolioManager` � explicitly mark "this position is already in SQLite" / "this position is already removed" so the wrappers are safe to call from engine startup / phantom-cleanup paths without re-introducing dual-write.

- **`WalletKey(SecretBox<String>)` newtype** in `src/core/security.rs` � wraps wallet private keys with Display/Debug redaction, panic-message safety, and zeroize-on-drop via the `secrecy` crate. Foundation for v0.15.1 migration of 5 raw `String` wallet-key sites.

- **`secrecy = "0.10"` + `zeroize = "1"` dependencies** in `Cargo.toml`.

### Changed � Engine Migration to SOT Wrappers

12 `positions_mut()` call sites in `src/engine/mod.rs` migrated:
- Phantom / executor-cancel cleanup ? `clear_position_cache()`
- DB load with validation/fixup ? `sync_from_db_position()`
- Stale position removal ? `remove_synced_position()`
- Position open/restore hot paths ? `open_position()` wrapper (with SQLite-first atomicity)
- Stop override + API close override ? `adjust_stop()` wrapper
- External close + partial external close ? `remove_synced_position()` + `adjust_quantity()` wrappers
- Scale-out persistence ? `adjust_stop()` (collected pos_ids first to avoid borrow conflict)

8 `let _ = j.X()` fire-and-forget SQLite write sites converted to error-aware logging:
- `delete_position`, `record_trade`, `save_position` now log errors explicitly via `if let Err(e) = j.X.await { error!("...", e); }` instead of silently dropping them. No more silent data loss when SQLite write fails.

### Tests

- 7 new unit tests in `src/core/security.rs` for `WalletKey` � Debug redaction, Display redaction, `expose_secret()` value roundtrip, clone behavior, `from_env` happy/sad paths, panic-message redaction (the actual bug class).
- Total: **412 tests pass** (was 405 before; +7 security tests, no regressions).

### Verification

- `cargo clippy -- -D warnings`: clean
- `cargo test --lib`: 412/412 pass

### Stage 2 (v0.15.1) � Deferred with explicit acknowledgment

The following items from FID-211 were deferred to v0.15.1 due to session time constraints. They are NOT silent deferrals � each has a specific line number, root cause, and acceptance criteria documented in FID-211. Spencer explicitly acknowledged the stage 2 split:

- Delete `account.open_positions` field entirely; replace 12+ hand-sync sites with `portfolio.open_positions()` (Bug 4 � third dual-write site)
- Replace 8 remaining `let _ = j.X` fire-and-forget patterns with full wrapper calls
- Migrate 5 `wallet_key: String` sites to `WalletKey` newtype
- Remove `DexTrader` parallel state fields + `data/dex_state.json` writes (audit Finding 1.4)
- Tighten `positions_mut()` / `closed_trades_mut()` to `pub(crate)` (currently still `pub` for compat)
- Archive 5 stale FIDs (FID-193, 194, 195, 196, 200) with full "resolution: shipped" narratives
- Add 4 more test files: key_manager_drop, startup_sync, engine_cycle, sot_wrapper_atomicity

### Acknowledgments

- Per Spencer's standing rule: "Nothing ever gets deferred by default unless I specifically state it is being deferred." This stage 2 list is explicit, not silent.
- v0.15.0 is full engine migration to v0.14.10 SOT wrappers � closes the dual-write hole that v0.14.10 left open.

## [0.14.10] � 2026-06-18

### SOT Infrastructure: SQLite as Single Source of Truth (Phase 1 of 2)

Phase 1 of FID-210 ships the SOT infrastructure. Phase 2 (FID-211) is the engine migration that wires callers to the new wrappers.

### Added � Schema Migration (`migrate_v210`)

- `migrate_v210` runs on engine startup, idempotent via `PRAGMA table_info` checks
- Adds `token_address TEXT NOT NULL DEFAULT ''` to `positions` table (Bug 6: was read but never written)
- Adds `real_trade BOOLEAN NOT NULL DEFAULT 1` to `trades` table
- One-time cleanup of 5 ghost `wallet_recovery` placeholder trade rows (from 2026-06-15, prior engine version)
- `token_address` is now included in `save_position` INSERT (was missing � silent data loss bug)

### Added � 5 SOT Wrapper Methods on `PortfolioManager`

These are the SOLE mutation points for positions. Persist to SQLite FIRST, then update in-memory cache on success.

- `open_position(pos, journal)` � validates, persists, updates cache
- `close_position_persist(id, exit_price, notes, journal)` � records trade, removes position
- `adjust_stop(id, new_stop, new_tp1/tp2/tp3/current_price, journal)` � partial field updates with stop-ratchet validation
- `partial_close(id, exit_price, scale_qty, new_scale_level, new_stop, notes, journal)` � TP1/TP2 scale-out, handles full close internally
- `load_from_db(journal)` � engine startup hydration from SQLite

Plus 2 helpers: `build_trade_record`, `build_partial_trade_record` (extracted from existing internal logic).
Plus computed property: `pub fn open_positions(&self) -> usize` (replaces 11 manual assignment sites).

### Added � `BlockReason` for Engine Block State

Replaces the `savant.blocked` text file. Serializes to the dashboard `/api/risk` endpoint.

- `SharedEngineData.block: Arc<RwLock<Option<BlockReason>>>` � in-memory field
- `BlockReason` struct: `block_type`, `reason`, `triggered_at`
- 4 helper methods: `set_block`, `clear_block`, `get_block`, `try_get_block`

### Added � 2 New `ExecutionError` Variants

- `DuplicatePositionId(String)` � detected at open time
- `InvalidStopRatchet { old: f64, new: f64 }` � detected at adjust_stop time (prevents locking in a loss)

### Added � `TradeJournal::load_closed_trades(limit)`

New method to hydrate the in-memory `closed_trades` working set at engine startup. Used by `PortfolioManager::load_from_db`.

### Tests

**+0 net tests** (still 405 lib + 10 dashboard = 415 total). The SOT wrappers have internal logic, but full E2E coverage is deferred to FID-211 when the engine migrates to use them.

### Known Limitations (deferred to FID-211)

- Engine still uses `positions_mut()` and `closed_trades_mut()` (15 + 3 sites) � bypasses the new wrappers
- Engine still writes `savant.blocked` file (3 sites) � block never auto-clears
- API still reads `savant.blocked` file (2 sites) � dashboard shows stale block
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

## [0.14.9] � 2026-06-18

### Rate-Limit Resilience + Bearish-EMA Veto Fix

Five FIDs addressing the operational bottlenecks observed in the v0.14.8 overnight run (2/169 LLM batches succeeded, 682 rate-limit WARNs, 22/22 high-conviction verdicts defaulting to PASS despite EMA bearish veto not being in the prompt).

### Added � FID-204: 10x NVIDIA API Keys for Per-Juror Rate-Limit Isolation

Overnight burst test empirically confirmed NVIDIA NIM free tier caps at ~5 RPM per model per key (5 successful M3 calls ? 429, 60s recovery). All 10 jurors were sharing one bucket.

- `NvidiaConfig.api_key_envs: Vec<String>` � list of env var names for per-juror keys (default empty, backward-compatible)
- `JuryPool` gains `nvidia_api_keys: Vec<String>` field; juror N (N >= 1) uses `keys[(N-1) % keys.len()]` round-robin
- `load_nvidia_api_keys(config)` � engine startup helper that reads all env vars, skips empty/missing with WARN
- 11 NVIDIA keys stored in `.env` as `NVIDIA_API_KEY` (legacy) + `NVIDIA_API_KEY_1..10` (multi-key)
- 11/11 keys verified working via direct API test; 3/3 new keys confirmed can hit M3 model
- Aggregate capacity: ~5 RPM � 10 keys = ~50 RPM (vs ~5 RPM previously)
- `config/default.toml` and `config/test-anvil.toml` updated with `api_key_envs = [...]`

### Added � FID-205: Per-Model Cooldown on HTTP 429

Herd-retry mitigation. When a model returns 429, mark it cooldown for 60s (�10s jitter) and skip other jurors in that window.

- `JuryPool.model_cooldowns: Mutex<HashMap<String, Instant>>` � tracks active cooldowns
- `is_model_in_cooldown()` � auto-prune expired entries on read
- `mark_model_cooldown()` � adds new entry with deterministic-ish jitter from system time
- `is_rate_limit_error()` � detects 429 in LlmError message
- `JuryPool::models_in_cooldown()` and `models_in_cooldown_count()` � telemetry/dashboard visibility

### Added � FID-206: Bearish-EMA Veto Fix (Long/SHORT/NO_SIGNAL vocabulary)

Per Gemini research 2026-06-18, the LLM's "default to PASS despite non-zero conviction" is not the predicted semantic gravity well � it's the model adding a custom bearish-EMA veto not in the prompt. The 22 verified v0.14.8 PASS verdicts all cited "EMA cross is against" as the reason.

**Three mechanisms identified by research (with citations):**
1. **RLHF financial risk-aversion** � alignment training penalizes confident trading advice
2. **Semantic gravity well** � naming the forbidden action (PASS) primes the model to produce it
3. **Autoregressive exposure bias** � early bearish tokens in reasoning skew later action tokens toward caution

**Fix applied** (per Gemini research recommendations):
- **Reasoning-first JSON schema** � `{"reasoning": "...", "is_probe": false, "conviction_score": 0.45, "action": "LONG"}` (CoT before action prevents premature commitment)
- **Sanitized action vocabulary** � `LONG` / `SHORT` / `NO_SIGNAL` (vs `BUY` / `SELL` / `PASS`). NO_SIGNAL means literally "zero edge" � strips the colloquial "passing" implication that triggered risk-aversion training
- **3 few-shot examples** � Trend Continuation, Contrarian Reversal (KEY EXAMPLE: bearish EMA + oversold Z-score ? LONG via mean-reversion), True Noise
- **Engine-level contradictory signal warning** � when LLM outputs `action=Pass` with `conviction_score > 0.10`, parser logs WARN with pair + conviction (does NOT auto-override; surface the pattern for analysis)
- `TradeAction` enum serde aliases updated to accept LONG / SHORT / NO_SIGNAL / NoSignal / NO-SIGNAL / NO SIGNAL / NOSIGNAL (all map to existing Buy / Sell / Pass variants)
- `normalize_llm_json()` regex updated to recognize the new vocabulary
- `src/agent/prompts/output_format.md` rewritten with FID-206 rules + examples + anti-pattern reminders

### Added � FID-207: LLM Timeout Structured Logging

Engine's batched M3 call (180s cap) now logs `[LLM] TIMEOUT � batch of N pairs produced no verdict after Ns` so overnight-log analyzers can count exactly how many cycles produced no decision (vs other failure modes).

### Added � FID-208: Decision Log + Equity History Cap Raise (500 ? 5000)

- `default_decision_log_max_entries`: 500 ? 5000 (~12 min history ? ~2h history at 3 cycles/min � 14 pairs)
- Equity curve in-memory cap: 500 ? 5000 (same rationale)
- Critical for debugging overnight behavior: v0.14.8 lost decision history at ~22 min into the run

### Changed � Engine Wiring

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

- `src/core/config.rs` � `NvidiaConfig.api_key_envs`, `load_nvidia_api_keys()`, `default_decision_log_max_entries 500?5000`
- `src/agent/jury/pool.rs` � `nvidia_api_keys`, `model_cooldowns`, `pick_nvidia_key()`, `mark_model_cooldown()`, `is_model_in_cooldown()`, `is_rate_limit_error()`, `models_in_cooldown_count()`, 4 new tests
- `src/agent/decision_parser.rs` � TradeAction serde aliases (LONG/SHORT/NO_SIGNAL), contradictory signal WARN, 6 new tests
- `src/agent/prompts/output_format.md` � rewritten with FID-206 rules + 3 few-shot examples + anti-pattern reminders
- `src/engine/mod.rs` � `load_nvidia_api_keys()` wiring, FID-207 timeout log
- `config/default.toml` + `config/test-anvil.toml` � `api_key_envs = [...]` array
- `.env` � 10 new NVIDIA_API_KEY_N env vars
- `VERSION` � 0.14.8 ? 0.14.9
- `Cargo.toml` � version 0.14.8 ? 0.14.9
- `protocol.config.yaml` � version 0.14.8 ? 0.14.9
- `README.md` � version + test count
- `CHANGELOG.md` � this section

## [0.14.8] � 2026-06-18

### Multi-Model Jury with NVIDIA NIM Expansion (FID-200)

The single-model bias problem � M3 defaults to PASS on flat markets � is structural, not fixable by prompt engineering. v0.14.8 expands the jury from OpenRouter auto-routing to direct hand-selection of 10 free NVIDIA NIM models.

### Changed � Primary LLM Provider

Switched from TokenRouter (quota-limited) to NVIDIA NIM (free, no quota, same M3 model).

- **Before:** TokenRouter ? M3, weekly quota
- **After:** NVIDIA NIM ? `minimaxai/minimax-m3` (verified, working, ~3s latency)

### Added � 10-Model NVIDIA NIM Jury

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

### Preserved � OpenRouter Fallback

Per Spencer's explicit constraint: OpenRouter path NOT ripped out. When `NVIDIA_API_KEY` is missing or NVIDIA calls fail, the jury falls back to OpenRouter (legacy behavior). Existing `[ai.openrouter]` config and `OPENROUTER_MANAGEMENT_KEY` env var still work.

### Tests

- 2 new unit tests in `pool.rs`: compile-time check for nvidia field, 10-model array validation
- 386 total tests pass (was 384 before)
- Clippy clean, fmt clean, pre-push green

### Verification

Per-model latency: 1-15s. Parallel calls: ~5-10s typical for 10 jurors. Total cycle time budget: 60s (well within).

## [0.14.7] � 2026-06-17

### State Sync (LLM/Jury/Executor on a Single Source of Truth)

After 16h of paper-mode testing producing 0 trades despite 703 PASS decisions, the state-sync issue was identified: the LLM hallucinated positions from its own prior decisions, the jury inherited the hallucinated context, and the executor's outcomes were never communicated to the decision layer. Three coordinated fixes shipped:

### Fixed � Pre-flight Guard (FID-194)
- New `src/agent/pre_flight.rs` with `apply_pre_flight_guard()` function.
- AdjustStop/Close actions get downgraded to `Pass` if the executor has no matching position. Prevents phantom management decisions.
- Single call site at `engine/mod.rs:2844` (the only `parse_decision` call).

### Added � Executor Feedback (FID-195)
- New `TradeStatus` enum (Pending/Filled/Rejected/Expired) on `DecisionEntry`.
- New `update_status()` method marks Pending entries as Filled/Rejected with reason.
- New `format_execution_outcomes()` in `context_builder.rs` shows Filled/Rejected entries with explicit `NO POSITION OPENED` marker.
- Filter in `context_for_pair` excludes Rejected from "Recent Decision Log".
- Jury receives executor's open positions prepended to user message for independent verification.
- All 5 executor call sites (open/close/adjust/place_stop/gasless) call `update_status` on Ok/Err.

### Added � Per-Cycle Reconciliation (FID-196)
- New `apply_to_portfolio()` in `reconciliation.rs` mutates state to match on-chain.
- Clears phantom positions (in memory but not on chain).
- Adds orphan positions (on chain but not in memory).
- Reconciles USDC balance divergence.
- Safety halt at >50% divergence (configurable via `safety_halt_threshold_pct`).
- Telemetry to `data/reconciliation_telemetry.jsonl` per cycle.
- Extends `reconcile_wallet_state` per ECHO Law 13 (one function, one truth).

### Added � Probe Position Mechanism (FID-184)
- New `is_probe: bool` field on `TradeDecision` with `#[serde(default)]`.
- When LLM sets `is_probe: true`: 0.5x sizing + auto-TP at 0.6% from entry.
- Max 3 concurrent probes (tracked via `strategy_name = "probe"`).
- Probe open events logged to `data/probe_pnl.jsonl`.
- Note: Gemini follow-up research (`LLM Crypto Trading Engine Diagnostics.md`) recommends smaller sizing (0.15x) and wider TP (1.2%) for DEX. The 0.5x/0.6% here are placeholders pending Gemini-driven refinement in v0.14.8.

### Changed � Prompt Calibration (FID-198)
- Reconciled 4 conflicting threshold sets in `strategy_knowledge.md` and `output_format.md`.
- Added `is_probe` field to JSON schema with concrete examples.
- Note: Gemini research recommends removing all numerical thresholds from prompts entirely (LLM evaluates narrative, engine gates numerically). This is planned for v0.14.8.

### Infrastructure
- Pre-push validation hook (FID-191): `.git/hooks/pre-push` runs `scripts/pre-push-validation.ps1` (fmt + clippy + tests).
- 380 tests pass (was 354 before session).

## [0.14.6] � 2026-06-17

### Strategy Recalibration (Gemini Deep Research Integration)

Following overnight 16h paper-mode run analysis (96 cycles, 703 PASS, 0 trades), the strategy was recalibrated per Gemini Q1/Q2/Q4/Q7 sniper/scalping recommendations.

### Changed � Conviction Thresholds Lowered (FID-184)

- Trending: 0.20 ? 0.05
- Volatile: 0.25 ? 0.15
- Ranging: 0.25 ? 0.10
- GreyZone: 0.25 ? 0.20 (default-to-PASS retained)

### Fixed � Prompt Anti-Pattern (FID-184)

Removed the "if you cannot compute, output 0.0 and select PASS" instruction. Replaced with: "Output granular probability between 0.00 and 1.00. A score of 0.50 represents absolute uncertainty." This eliminates the default-to-hold bias that produced 87% zero-conviction decisions.

### Added � Cognitive Slippage Penalty (FID-184)

Equity snapshots now apply 0.5%/min latency penalty, capped at 50 bps, when cycle elapsed > 10s. This reflects real-world execution decay from LLM "think" time.

### Fixed � Jury Regime Hardcoding (FID-184)

Jury was hardcoded to `MarketRegime::Ranging`. Now maps session to regime: US-EU Overlap ? Trending, others ? Ranging.

### Changed � Pre-Screening Activated (FID-189)

Set `scan_all_pairs = false` in `config/default.toml` and `config/test-anvil.toml`. This activates the existing pre-scoring at `engine/mod.rs:2052-2120` (FID-056/FID-118) which gates pairs on: RSI extreme, ADX trend, EMA cross, volume spike, BB squeeze. Pairs with no signal no longer reach the LLM.

### Changed � Kelly Sizing 0.5x ? 0.25x (FID-190)

Per Gemini Q1: "0.25x fractional Kelly sizing algorithm based on calculated signal edge to manage maximum drawdowns." Quarter-Kelly provides additional safety margin with limited historical data.

### Added � 0x AMM Price Source (FID-188)

New `src/data/sources/zero_x_price.rs` provides AMM-implied spot price for live trading decisions on Arbitrum, including slippage. Replaces Kraken CEX-derived spot price for live trading. Historical candle data still uses multi-source aggregation (Kraken, OKX, KuCoin, etc.).

### Changed � Log Hygiene (FID-185 + FID-186)

Demoted 8 working-as-designed `warn!` calls to `info!` or `debug!`:
- FID-126 anti-pattern noise ? debug
- FID-096 ZERO-BASE ENFORCEMENT ? info
- Judge fallback (majority vote) ? info
- Jury key threshold ? info
- Jury member timed out ? info
- Jury quorum NOT met ? info
- Context State Delta-compression ? debug

Context State now also writes aggregate metrics to `data/context_state_metrics.json` per cycle (total_compressions, total_tokens_saved, avg_compression_rate).

### Added � Pre-Push Validation Hook (FID-191)

`scripts/pre-push-validation.ps1` runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace --all-targets` before any push. Blocks broken builds from reaching remote. Caught a real fmt violation in `test_e2e_fid160.rs` on first run.

### Deferred � Multi-Chain Architecture (FID-187)

Scoped for v0.15.0. The full per-chain sub-strategy execution (`tokio::spawn` per chain, per-chain state isolation, cross-chain portfolio aggregation) is a multi-week architectural change. FID-188 (0x AMM) and FID-189 (pre-screening) are the v0.14.5-era components that enable the v0.15.0 multi-chain refactor.

### Build & Test

- 354 lib tests pass, 0 clippy warnings, 0 build errors
- Engine running on Anvil paper mode (PID 46608)
- 200-500 trade statistical sample required before live mode (Gemini Q1)

## [0.14.5] � 2026-06-17

### Fixed � start.bat Freezes Kilo CLI (FID-175)

The `Stop-Process -Name node -Force` in the PowerShell cleanup block (line 36) was killing ALL node.exe processes on the machine, including Kilo�s own MCP server processes, freezing the Kilo CLI session. Fix: scoped the kill to only processes whose command line contains `savant`.

### Fixed � dotenvy `.env` Parse Failure on `0X_API_KEY` (FID-176)

Spencer's `.env` had `0X_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c` � a line starting with a digit. The dotenvy parser rejected the entire file as invalid, which caused ALL API keys to be empty at startup, producing 401 errors on every LLM call. Root cause was a stale env var from a prior API key format. Fix: commented out the line (not needed � ZEROEX_API_KEY is used instead) and documented the dotenvy gotcha in `.env.example`.

### Fixed � start.bat Default Config Reverted to Anvil (FID-177)

A prior session accidentally changed `start.bat`'s default from `config/test-anvil.toml` to `config/default.toml`, causing the engine to attempt live mainnet execution. Reverted the default back to `config/test-anvil.toml` (Anvil fork). `SAVANT_CONFIG` may still override at runtime.

### Fixed � Anvil Auto-Start Block cmd.exe Parse Error (FID-178)

The Anvil auto-start conditional block in `start.bat` (lines 97�106) used nested `if/else` with `%SAVANT_CONFIG:anvil=%` string substitution. Under certain invocation patterns this produced `. was unexpected at this time.` from cmd.exe. Fix: removed the inline block and replaced with an unconditional `call start-anvil.bat`. `start-anvil.bat` is already idempotent (detects Anvil at port 8545 before launching).

### Fixed � Re-enabled Jury System (FID-179)

`[ai.jury]` was `enabled = false` in both `config/default.toml` and `config/test-anvil.toml`. Flipped to `enabled = true`. The jury (M3 control + 9 free-model jurors + 70% veto threshold) is a core architectural feature of Savant � multi-model adversarial decision validation. It had been disabled in a prior session due to an incorrect assessment of noise issues; the correct fix is to suppress noise, not disable the system. Uses `OPENROUTER_MANAGEMENT_KEY` for juror provisioning and `TOKEN_ROUTER_API_KEY` for the M3 control juror. No code changes.

### Fixed � Dashboard Layout: Terminal Height + Closed Trades Column (FID-180)

The dashboard grid was `grid-cols-2 grid-rows-[60%_40%]`, which gave Terminal only 40% height and no horizontal room. Updated to `grid-cols-3 grid-rows-[1.2fr_1fr_1fr]` with Terminal in column 3 spanning all 3 rows (`row-span-3`), Closed Trades in column 1, and Activity in column 2. Bumped Closed Trades table row padding from `py-0.5` to `py-1.5` and trade slice from 10 to 30 for better scanability. Dashboard builds clean.

### Fixed � Equity Curve Live Data + Atomic Persistence + Dashboard Layout + Warning Cleanup + WebSocket v2 (FID-181)

Master FID consolidating 4 issues found during the v0.14.5 session:

**Equity curve (Issue A):** The engine cycle never wrote equity snapshots to `state.shared.equity_curve`. Only the backtest engine did. Dashboard was permanently "Collecting equity data�" for all live runs. Fix: added `push_equity_snapshot` at end of each cycle (`src/engine/mod.rs`), `load_equity_history` and `save_equity_history` in `src/core/shared.rs`. Atomic write via `.tmp` + `std::fs::rename`. In-memory cap of 200 snapshots, configurable via `equity_history_max_snapshots`. File at `data/equity_history.json` with versioned format.

**Dashboard layout (Issue B):** Per Spencer, Terminal should be the tall element, not Closed Trades. Confirmed `row-span-3` on Terminal column 3 in `dashboard/src/app/page.tsx`. Grid: `grid-cols-3 grid-rows-[1.2fr_1fr_1fr] gap-1.5 min-h-0`. Note: a stale dashboard server process may still serve the old build; restart the dashboard to pick up the new grid.

**Warning log cleanup (Issue C):** ~61 warn-level lines per cycle were demoted to info or debug:
- Anti-thrashing per-pair (21x/cycle) � debug
- VolRatio=0 for illiquid pairs (21x/cycle) � debug
- GoPlus "no known address" � per-token `HashSet` dedup, logged once per token ever
- Jury parse failures � debug
- Judge fallback message � debug
- DEX stop-losses startup info � info (was warn)

**WebSocket v2 fix (Issue D):** `params.symbol` was a single string (`"XRP/USDT"`), but Kraken v2's `subscribe` method expects a JSON array (`["XRP/USDT"]`). Fix: changed to `json!([symbol])`. The response handler was reading `result.channel` which was null in error responses; now reads the `error` field directly, producing real error messages instead of `"Kraken WS subscribe failed for unknown"`.

### Build & Test

- 354 lib tests passing, 0 clippy warnings, 0 build errors
- Engine running on Anvil paper mode, equity curve collecting snapshots every cycle

## [0.14.4] � 2026-06-16

### Fixed � FID-168/170/171 v2 Strict-Read Improvements (3 FIDs)

**FID-168 v2 (Cycle Snapshot Enrichment):**
Cycle_snapshot now captures regime + ATR + ADX + RSI so the LLM's prompt gets the data it actually asks for. Added `cycle_elapsed` safety check before the summary LLM call (skip if >240s elapsed to avoid the 5-min cycle watchdog). `is_stale()` freshness check now used to force re-summarize every 60s when context is below budget. Corrected pruning math: first pruning at ~10 cycles, not 100.

**FID-170 v2 (Token-Based Stage Splits):**
Replaced count-based `split_into_stages` with `split_into_stages_by_tokens` (greedy fill). Each stage stays under `target_per_stage` tokens regardless of block size distribution. Per-stage `summarize_with_fallback_public` replaces plain `summarize`, giving partial-failure recovery (oversized single blocks get their own stage).

**FID-171 v2 (Handoff Prompt Polish):**
Removed dead `let _ = chunk_size_cap;`. Uses the chunked `summarize_chunks_only` private helper pattern (consistent with FID-170). HANDOFF_INSTRUCTIONS updated with explicit "You are the new LLM" second-person role statement + YOUR ROLE section.

### Build & Test

- 362 tests passing (350 lib + 10 bin + 2 doc), 0 clippy warnings

## [0.14.3] � 2026-06-16

### Added � FID-168: Cycle Summarization Wired Into Engine Loop (Phase 1b)

Engine records per-pair `cycle_snapshot` DataBlocks after each `parse_decision`. At cycle end, prunes old blocks (target: 30% of context window) and summarizes via M3. Historical summary prepended to per-pair user message as "## Historical Summary" block.

### Added � FID-170: Stage-Based Summarization (Phase 2)

Port of openclaw's `summarizeInStages`. Splits history into N stages, summarizes each, merges via final LLM call with trading-specific merge instructions. Opt-in API for v0.15.0.

### Added � FID-171: Handoff Summaries (Phase 3)

Port of openclaw's `summarizeForHandoff`, trading-specific. Briefing for model rotation. Opt-in API for v0.15.0 multi-model rotation.

### Added � FID-172: Engine Restart + Paper-Mode Validation Spec

Pre-flight verified. Engine startup is Spencer's action (via `start.bat`); FID is a validation spec. Spencer runs `start.bat` to launch the engine; Vera writes the validation report from cycle data.

### Build & Test

- 357 tests passing (347 lib + 10 bin + 2 doc), 0 clippy warnings

## [0.14.2] � 2026-06-15

### Fixed � FID-164: Per-Pair ContextState + Token-Based Compression

Singleton `ContextState` was diffing pair N's user message against pair N-1's, producing meaningless ~95% diff ratios. Anti-thrashing then concluded "useless" from corrupted data. Fix: per-pair `HashMap<String, PairState>`, tiktoken-based detection, adaptive threshold, per-pair anti-thrashing, `end_cycle()` cumulative telemetry. 5 new tests.

### Fixed � FID-166: HTTP 504 Streaming Retry + Cycle Timeout

Cycle 17 took 170s due to M3 streaming stalling and HTTP 504 from OpenRouter. 504 added to transient-retry list. `chat_stream` outer retries 2?1. New `streaming_timeout_secs: u64 = 60` with separate `streaming_client: reqwest::Client`.

### Added � FID-167: Multi-Chain Enable (Path A)

`start.bat` default config switched to `config/default.toml`. New `SAVANT_CHAIN` env var (default: ethereum). 5-chain support already coded in `config/default.toml`.

### Added � FID-165: LLM Summarization Phase 1 (Foundation)

Port from openclaw `compaction.ts`. 4 functions: `chunk_by_max_tokens`, `prune_for_context_share`, `summarize_chunks`, `summarize_with_fallback`. Stage-based and handoff deferred to v0.15.0.

### Fixed � FID-163: LLM Data Integrity (4 classes of bugs)

1. `{}` format specifiers replaced all `{:.N}` in LLM-bound paths � byte-faithful data
2. `format_diff` zero-collapse threshold `abs < 0.001` ? `v == 0.0`
3. TSLN serializer `reset()` called per pair � fixes state-bleed
4. 8 missing context blocks added to TSLN path � full parity with legacy JSON path

### Build & Test

- 347 tests passing (325 lib + 10 bin + 2 doc), 0 clippy warnings
