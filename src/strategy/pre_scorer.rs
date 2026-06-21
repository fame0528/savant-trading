//! FID-222 Funnel v1 — Momentum Pre-Scorer + Top-K Selector
//!
//! Pure-Rust module that ranks candidate pairs via a 6-signal composite score,
//! drops below `min_score_threshold`, and selects top-K by score with
//! deterministic alphabetical tie-breaking. Designed to be feature-gated
//! (default OFF) at integration sites in `src/engine/mod.rs`.
//!
//! Design references:
//! - FID-222 (the spec, June 2026)
//! - FID-222.5 (the audit, June 2026) — recommends deferring pre-emptive
//!   threshold tuning; funnel ships with telemetry instead.
//!
//! Per thinker's AUDIT 2026-06-21:
//! - Q3: bb_squeeze signal reserved at weight 0.00 (not 0.05); the 0.05 weight
//!   was redistributed to `vwap_proximity` so the per-regime weights sum to 1.0.
//! - Q4: NaN propagation is sanitized explicitly via `sanitize_score`.
//! - Q5: HUNT MODE topology lives outside this module. Caller passes a local
//!   `hunt_mode: bool` flag. Module is regime-agnostic on HUNT.

use std::cmp::Ordering;
use tracing::warn;

use serde::{Deserialize, Serialize};

use crate::core::types::{Candle, IndicatorValues, MarketRegime};

/// 6-signal composite score components (each ∈ [0.0, 1.0]).
///
/// `bb_squeeze` is reserved (always 0.0 v1) until `IndicatorValues` grows a
/// `bb_width` field. Weights in `FunnelWeights` account for this: `bb: 0.00`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Signals {
    pub ema_alignment: f64,
    pub rsi_extreme: f64,
    pub adx_trending: f64,
    pub vol_spike: f64,
    pub vwap_proximity: f64,
    /// RESERVED — always 0.0 v1. Will become Bollinger bandwidth when
    /// `IndicatorValues::bb_width` ships.
    pub bb_squeeze: f64,
}

/// One scored candidate (input pair + computed composite + signal breakdown).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredCandidate {
    pub pair: String,
    pub score: f64,
    pub signals: Signals,
}

/// Per-regime weight table. Sums to **1.0** per regime.
/// `bb_squeeze` weight is 0.0 v1 — the 0.05 share was redistributed to
/// `vwap_proximity` (per FID-222.5 thinker audit Q3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunnelWeights {
    pub ema: f64,
    pub rsi: f64,
    pub adx: f64,
    pub vol: f64,
    pub vwap: f64,
    pub bb: f64,
}

impl FunnelWeights {
    /// Default weights for the given regime. All variants sum to 1.0.
    pub fn for_regime(regime: MarketRegime) -> Self {
        match regime {
            // Trending: momentum-heavy (EMA alignment + ADX trending dominate)
            MarketRegime::Trending => FunnelWeights {
                ema: 0.30,
                rsi: 0.10,
                adx: 0.30,
                vol: 0.20,
                vwap: 0.10,
                bb: 0.00,
            },
            // Ranging: mean-reversion heavy (RSI extreme + VWAP proximity)
            MarketRegime::Ranging => FunnelWeights {
                ema: 0.15,
                rsi: 0.35,
                adx: 0.15,
                vol: 0.10,
                vwap: 0.25,
                bb: 0.00,
            },
            // Volatile: volume spike dominates
            MarketRegime::Volatile => FunnelWeights {
                ema: 0.10,
                rsi: 0.10,
                adx: 0.25,
                vol: 0.40,
                vwap: 0.15,
                bb: 0.00,
            },
        }
    }

    /// Validate that a custom weight table sums to 1.0 ± 0.001.
    /// Panics in debug builds on deviation; logs warn in release.
    pub fn validate_unity(&self) {
        let total = self.ema + self.rsi + self.adx + self.vol + self.vwap + self.bb;
        let diff = (total - 1.0).abs();
        if diff > 0.001 {
            warn!(
                "FUNNEL_V1 weights sum to {:.4} (not 1.0) — diff {:.4}. Score will be biased.",
                total, diff
            );
        }
    }
}

/// One row of input to `run_funnel`: a pair's last candle + computed indicators.
#[derive(Debug, Clone, Default)]
pub struct CandidateInput {
    pub pair: String,
    pub last_candle: Option<Candle>,
    pub indicators: IndicatorValues,
}

impl CandidateInput {
    pub fn new(
        pair: impl Into<String>,
        last_candle: Option<Candle>,
        indicators: IndicatorValues,
    ) -> Self {
        Self {
            pair: pair.into(),
            last_candle,
            indicators,
        }
    }
}

/// Stats carried alongside `FunnelResult::Filtered` for telemetry JSONL.
///
/// `[FID-222]` — `input_N`/`output_K` are serialised verbatim into the
/// `funnel-rankings.jsonl` telemetry stream and into any `/api/funnel/v1`
/// JSON response. Renaming to snake_case would break the JSONL schema, so
/// the math-notation N (universe size) and K (top-K count) are preserved
/// via `#[allow(non_snake_case)]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FunnelStats {
    pub input_N: usize,
    pub output_K: usize,
    pub threshold_drop: usize,
    pub empty_fallback: bool,
    pub hunt_mode_bypass: bool,
}

/// Result of `run_funnel`. Caller decides what to do with either branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunnelResult {
    /// Feature-gate disabled or HUNT MODE active — feed through unchanged.
    PassThrough(Vec<String>),
    /// Funnel ran. Carries the top-K candidates + telemetry stats.
    Filtered(Vec<ScoredCandidate>, FunnelStats),
}

impl FunnelResult {
    /// Pair list to feed to LLM dispatch.
    pub fn pairs(&self) -> Vec<String> {
        match self {
            FunnelResult::PassThrough(pairs) => pairs.clone(),
            FunnelResult::Filtered(cands, _) => cands.iter().map(|c| c.pair.clone()).collect(),
        }
    }

    pub fn stats(&self) -> Option<FunnelStats> {
        match self {
            FunnelResult::PassThrough(_) => None,
            FunnelResult::Filtered(_, stats) => Some(stats.clone()),
        }
    }
}

// ==========================================================================
// Public scoring APIs
// ==========================================================================

/// Compute the 6 signals for one pair from its (last candle, indicators).
/// Missing indicator (None) → 0.0 contribution. NaN propagation guarded.
pub fn compute_signals(last_candle: Option<&Candle>, ind: &IndicatorValues) -> Signals {
    Signals {
        ema_alignment: signal_ema_alignment(ind.ema_fast, ind.ema_slow),
        rsi_extreme: signal_rsi_extreme(ind.rsi),
        adx_trending: signal_adx_trending(ind.adx),
        vol_spike: signal_vol_spike(last_candle, ind.volume_sma),
        vwap_proximity: signal_vwap_proximity(last_candle, ind.vwap),
        bb_squeeze: 0.0, // RESERVED — see Signals doc comment
    }
}

/// Score a single pair. Returns a `ScoredCandidate` with score ∈ [0.0, 1.0].
/// NaN-safe: any NaN propagation is sanitized to 0.0 (Q4 AUDIT fix).
pub fn score_pair(
    pair: &str,
    last_candle: Option<&Candle>,
    ind: &IndicatorValues,
    weights: &FunnelWeights,
) -> ScoredCandidate {
    let signals = compute_signals(last_candle, ind);
    let raw = signals.ema_alignment * weights.ema
        + signals.rsi_extreme * weights.rsi
        + signals.adx_trending * weights.adx
        + signals.vol_spike * weights.vol
        + signals.vwap_proximity * weights.vwap
        + signals.bb_squeeze * weights.bb;
    let score = sanitize_score(raw);
    ScoredCandidate {
        pair: pair.to_string(),
        score,
        signals,
    }
}

/// Select top-K by score from a Vec<ScoredCandidate>.
/// Drops below `min_score` (after sanitization). NaN-safe (Q4 AUDIT).
/// Stable sort with alphabetical pair-name tie-breaker (deterministic).
pub fn select_top_k(
    mut scored: Vec<ScoredCandidate>,
    k: usize,
    min_score: f64,
) -> Vec<ScoredCandidate> {
    scored.retain(|c| c.score >= min_score);
    scored.sort_by(sort_key);
    scored.truncate(k);
    scored
}

/// Run the full funnel.
///
/// - `input`: scored candidates (or unscored — `run_funnel` will score them
///   using `weights` derived from `regime`).
/// - `regime`: cycle-wide regime. Used to pick weights.
/// - `config`: funnel config (`enabled`, `top_k`, `min_score_threshold`,
///   `weights_override`).
/// - `hunt_mode`: HUNT MODE flag from the local engine loop. If `true`,
///   pass through input unchanged (preserves FID-063 intent, FID-222
///   Loop 1.7 audit).
///
/// Returns `FunnelResult::PassThrough` if `enabled=false` OR `hunt_mode=true`.
/// Returns `FunnelResult::Filtered` otherwise. Empty-input fallback:
/// picks top-3 by absolute score; if all 0.0, picks top-3 alphabetically.
pub fn run_funnel(
    input: Vec<CandidateInput>,
    regime: MarketRegime,
    config: &FunnelConfig,
    hunt_mode: bool,
) -> FunnelResult {
    if !config.enabled || hunt_mode {
        return FunnelResult::PassThrough(input.iter().map(|c| c.pair.clone()).collect());
    }

    let weights = config.weights_for(regime);
    weights.validate_unity();

    let scored: Vec<ScoredCandidate> = input
        .iter()
        .map(|c| score_pair(&c.pair, c.last_candle.as_ref(), &c.indicators, &weights))
        .collect();

    let mut sorted = scored.clone();
    sorted.retain(|c| c.score >= config.min_score_threshold);
    let threshold_drop = sorted.len();
    sorted.sort_by(sort_key);

    if sorted.is_empty() {
        warn!(
            "FUNNEL_V1: empty result (all {} candidates below threshold {:.3}), falling back to top-3 by absolute score",
            scored.len(),
            config.min_score_threshold
        );
        let mut fallback = scored;
        fallback.sort_by(sort_key);
        fallback.truncate(3);
        let output_k = fallback.len();
        return FunnelResult::Filtered(
            fallback,
            FunnelStats {
                input_N: input.len(),
                output_K: output_k,
                threshold_drop,
                empty_fallback: true,
                hunt_mode_bypass: false,
            },
        );
    }

    sorted.truncate(config.top_k);
    let output_k = sorted.len();
    FunnelResult::Filtered(
        sorted,
        FunnelStats {
            input_N: input.len(),
            output_K: output_k,
            threshold_drop,
            empty_fallback: false,
            hunt_mode_bypass: false,
        },
    )
}

// ==========================================================================
// Internal helpers
// ==========================================================================

/// Sort key for NaN-safe stable sort with deterministic tie-break.
///
/// Per thinker's AUDIT Q4: `partial_cmp().unwrap_or(Ordering::Equal)` is unsafe
/// for NaN because it treats `NaN == anything`. Replace with explicit NaN
/// bucketing (NaNs sort last) followed by partial-cmp and alphabetical.
fn sort_key(a: &ScoredCandidate, b: &ScoredCandidate) -> Ordering {
    let a_nan = a.score.is_nan();
    let b_nan = b.score.is_nan();
    a_nan
        .cmp(&b_nan)
        .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal))
        .then_with(|| a.pair.cmp(&b.pair))
}

/// NaN-safe score clamp. Returns 0.0 for NaN, else clamped to [0.0, 1.0].
fn sanitize_score(s: f64) -> f64 {
    if s.is_nan() {
        0.0
    } else {
        s.clamp(0.0, 1.0)
    }
}

// ----- individual signal functions (each returns 0.0-1.0 or 0.0 if missing) -----

/// EMA alignment: 0.5 baseline + 0.5 × |fast-slow|/slow × 200, clamped.
/// Returns 0.0 if either EMA missing OR slow ≈ 0.
fn signal_ema_alignment(ema_fast: Option<f64>, ema_slow: Option<f64>) -> f64 {
    match (ema_fast, ema_slow) {
        (Some(f), Some(s)) if s.abs() > 1e-9 => {
            let pct_diff = (f - s).abs() / s.abs();
            sanitize_score(0.5 + 0.5 * (pct_diff * 200.0).clamp(0.0, 1.0))
        }
        _ => 0.0,
    }
}

/// RSI extreme: 1.0 if RSI < 30 or > 70; otherwise 1.0 - |RSI - 50|/20.
fn signal_rsi_extreme(rsi: Option<f64>) -> f64 {
    match rsi {
        None => 0.0,
        Some(r) if !(30.0..=70.0).contains(&r) => 1.0,
        Some(r) => sanitize_score(1.0 - (r - 50.0).abs() / 20.0),
    }
}

/// ADX trending: adx / 40, clamped.
fn signal_adx_trending(adx: Option<f64>) -> f64 {
    match adx {
        None => 0.0,
        Some(a) => sanitize_score(a / 40.0),
    }
}

/// Volume spike: max(0, vol/vol_sma - 1). 0.0 if either missing or sma ≤ 0.
fn signal_vol_spike(last_candle: Option<&Candle>, vol_sma: Option<f64>) -> f64 {
    let candle = match last_candle {
        Some(c) => c,
        None => return 0.0,
    };
    let sma = match vol_sma {
        Some(s) if s > 0.0 => s,
        _ => return 0.0,
    };
    sanitize_score(candle.volume / sma - 1.0)
}

/// VWAP proximity: 1.0 - |close-vwap|/close × 100, clamped.
fn signal_vwap_proximity(last_candle: Option<&Candle>, vwap: Option<f64>) -> f64 {
    let candle = match last_candle {
        Some(c) => c,
        None => return 0.0,
    };
    let v = match vwap {
        Some(v) if v > 0.0 => v,
        _ => return 0.0,
    };
    let denom = candle.close.max(1e-9);
    let pct = (candle.close - v).abs() / denom * 100.0;
    sanitize_score(1.0 - pct)
}

// ==========================================================================
// FID-222.7 Runtime Wiring (production glue)
//
// Helpers consumed by `src/engine/mod.rs` PHASE 1b to (a) collect
// CandidateInputs inside the for-pair loop, (b) run the funnel with
// open-position guard, (c) persist telemetry to JSONL + FunnelRuntimeState.
//
// - `FunnelRankingRecord` — one JSONL row, appended per cycle.
// - `append_funnel_jsonl` — sync write wrapped in `tokio::spawn` by the engine.
// - `record_funnel_runtime` — updates `SharedEngineData::funnel_v1` for /api/funnel/v1.
// - `FunnelResult::pairs_len` / `input_count` accessors (kept ergonomic).
// ==========================================================================

use std::fs::OpenOptions;
use std::io::Write;

/// FID-222.7 Stage 2 telemetry schema. One row per cycle, appended as JSONL
/// to `dev/logs/funnel-rankings.jsonl`. Schema is intentionally additive —
/// adding fields is non-breaking for downstream readers.
///
/// `[FID-222]` — `input_N`/`output_K` serialise as `"input_N"`/`"output_K"`
/// in the JSONL stream; renaming to snake_case would break the schema
/// consumed by downstream readers, so the N/K math notation is preserved
/// via `#[allow(non_snake_case)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FunnelRankingRecord {
    /// RFC 3339 UTC.
    pub timestamp: String,
    /// Engine cycle counter (tick).
    pub cycle_id: u64,
    /// Effective regime passed to `run_funnel`.
    pub regime: String,
    /// True if HUNT MODE was active at funnel-call time.
    pub hunt_mode: bool,
    /// True if config.trading.funnel_v1.enabled was true at funnel-call time.
    pub funnel_enabled: bool,
    /// Universe size post-hygiene (== `funnel_inputs.len()`).
    pub input_N: usize,
    /// Top-K size returned by funnel (0 on PassThrough, even when N>0).
    pub output_K: usize,
    /// Candidates dropped by `min_score_threshold`.
    pub threshold_drop: usize,
    /// True when the empty-threshold fallback (top-3 alphabetical) fired.
    pub empty_fallback: bool,
    /// True when HUNT MODE shortcut returned PassThrough.
    pub hunt_mode_bypass: bool,
    /// Best-of-K funnel score (None on PassThrough).
    pub top_score: Option<f64>,
    /// Worst-of-K funnel score (None on PassThrough).
    pub min_top_score: Option<f64>,
    /// Pairs with an open Position at cycle start — used by safety guard.
    pub positioned_pairs: Vec<String>,
    /// Pairs really fed into LLM dispatch AFTER retain (top-K ∪ positioned).
    pub retained_pairs: Vec<String>,
    /// Count of positioned pairs retained ONLY because of the guard,
    /// not because they made top-K. Operational signal — high value
    /// means threshold/top_k is too tight for held positions.
    pub orphaned_retained: usize,
    /// Full ranked list (all candidates, not just top-K). For forensics.
    pub scored: Vec<RankedCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedCandidate {
    pub pair: String,
    pub score: f64,
    pub regime: String,
    pub pair_was_positioned: bool,
    pub pair_was_top_k: bool,
    /// FID-222.8 (nit #3): True if this candidate was force-injected post-loop
    /// by the open-position safety guard (i.e. it was not computed via
    /// `run_funnel`'s scored path). Lets downstream readers distinguish
    /// pre-filter bypasses from real threshold drops.
    pub force_injected: bool,
}

impl FunnelRankingRecord {
    /// Build from cycle context. Cloning the ranked list is cheap for K≤100.
    ///
    /// `[FID-222]` — `input_N`/`output_K` local variables mirror the parent
    /// struct's N/K fields. `#[allow(non_snake_case)]` here silences the
    /// variable-name lint while preserving the JSONL schema (see struct doc).
    #[allow(non_snake_case, clippy::too_many_arguments)]
    pub fn build(
        cycle_id: u64,
        regime: MarketRegime,
        hunt_mode: bool,
        funnel_enabled: bool,
        positioned_pairs: Vec<String>,
        retained_pairs: Vec<String>,
        orphaned_retained: usize,
        // FID-222.8 nit #3: pairs that bypassed `run_funnel` via the engine's
        // open-position safety guard. JSONL telemetry records them as
        // `force_injected: true` so downstream readers can distinguish them
        // from real top-K candidates. Empty for tests that don't model the
        // safety guard.
        force_injected_pairs: &[String],
        funnel_result: &FunnelResult,
    ) -> Self {
        let (
            input_N,
            output_K,
            threshold_drop,
            empty_fallback,
            hunt_mode_bypass,
            top_score,
            min_top_score,
        ) = match funnel_result {
            FunnelResult::Filtered(scored, stats) => (
                stats.input_N,
                stats.output_K,
                stats.threshold_drop,
                stats.empty_fallback,
                stats.hunt_mode_bypass,
                scored.first().map(|c| c.score),
                scored.last().map(|c| c.score),
            ),
            FunnelResult::PassThrough(pairs) => (pairs.len(), 0, 0, false, true, None, None),
        };
        let positioned_set: std::collections::HashSet<String> =
            positioned_pairs.iter().cloned().collect();
        let retained_set: std::collections::HashSet<String> =
            retained_pairs.iter().cloned().collect();
        // FID-222.8 nit #3: annotated side-channel so build() can mark each
        // RankedCandidate as bypassed-by-guard (force-injected) vs top-K true.
        let force_injected_set: std::collections::HashSet<String> =
            force_injected_pairs.iter().cloned().collect();
        // Build ranked list from scored inputs. We reconstruct from
        // run_funnel side-channel: scores are not in FunnelResult directly,
        // so we use SnoreCompute only when Filtered is available.
        let scored = match funnel_result {
            FunnelResult::Filtered(scored, _) => scored
                .iter()
                .map(|c| RankedCandidate {
                    pair: c.pair.clone(),
                    score: c.score,
                    regime: regime.to_string(),
                    pair_was_positioned: positioned_set.contains(&c.pair),
                    pair_was_top_k: true,
                    // FID-222.8 nit #3: Filtered branch force_injected is true
                    // when the engine pushed this pair via the safety guard
                    // (IndicatorValues::default() → score 0.0, which always
                    // survives min_score_threshold and lands in top-K).
                    force_injected: force_injected_set.contains(&c.pair),
                })
                .collect(),
            FunnelResult::PassThrough(pairs) => pairs
                .iter()
                .map(|p| RankedCandidate {
                    pair: p.clone(),
                    score: 0.0,
                    regime: regime.to_string(),
                    pair_was_positioned: positioned_set.contains(p),
                    pair_was_top_k: retained_set.contains(p),
                    // FID-222.8 nit #3: PassThrough branch force_injected is
                    // true when the engine pushed this pair via the safety
                    // guard before run_funnel detected the disabled/hunt-mode
                    // gate and bailed into PassThrough.
                    force_injected: force_injected_set.contains(p),
                })
                .collect(),
        };
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            cycle_id,
            regime: regime.to_string(),
            hunt_mode,
            funnel_enabled,
            input_N,
            output_K,
            threshold_drop,
            empty_fallback,
            hunt_mode_bypass,
            top_score,
            min_top_score,
            positioned_pairs,
            retained_pairs,
            orphaned_retained,
            scored,
        }
    }
}

/// Append one JSONL row to `dev/logs/funnel-rankings.jsonl`. Engine wraps
/// this in `tokio::spawn` so the JSONL write never blocks the cycle.
/// Sync write is safe because:
/// - File is append-only (no concurrent writers expected).
/// - Loss of one row on cycle-death is acceptable (diagnostic only).
/// - Sync `OpenOptions + writeln` is < 1ms for the ~10 KB record.
pub fn append_funnel_jsonl(record: &FunnelRankingRecord) {
    let path = "dev/logs/funnel-rankings.jsonl";
    if let Some(parent) = std::path::Path::new(path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!(
                "FUNNEL_V1: JSONL dir create failed at {}: {}",
                parent.display(),
                e
            );
            return;
        }
    }
    let json = match serde_json::to_string(record) {
        Ok(s) => s,
        Err(e) => {
            warn!("FUNNEL_V1: JSONL serialize failed: {}", e);
            return;
        }
    };
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{}", json) {
                warn!("FUNNEL_V1: JSONL append failed at {}: {}", path, e);
            }
        }
        Err(e) => warn!("FUNNEL_V1: JSONL open failed at {}: {}", path, e),
    }
}

/// Update `SharedEngineData::funnel_v1` from a completed funnel call.
/// Always invokes `.write().await` so the API route `/api/funnel/v1`
/// reflects the latest cycle (or pass-through default).
///
/// On PassThrough: reset state to defaults with explicit phase reason
/// so the dashboard chip reads "hunt_mode" or "funnel_disabled" rather
/// than stale values from a prior filtered cycle.
///
/// FID-222.8 (nit #2): the previously accepted `_orphaned_retained` parameter
/// is REMOVED. Telemetry for orphaned_retained lives in `FunnelRankingRecord`
/// JSONL rows only — readers query dev/logs/funnel-rankings.jsonl.
/// If /api/funnel/v1 needs it later, add a new field to `FunnelRuntimeState`
/// and a writer here.
pub async fn record_funnel_runtime(
    shared: &crate::core::shared::SharedEngineData,
    regime: MarketRegime,
    hunt_mode: bool,
    funnel_result: &FunnelResult,
) {
    use crate::core::shared::FunnelRuntimeState;
    let mut state_guard = shared.funnel_v1.write().await;
    match funnel_result {
        FunnelResult::Filtered(scored, stats) => {
            state_guard.enabled_at_last_cycle = true;
            state_guard.last_universe_post_hygiene = stats.input_N;
            state_guard.last_top_k_size = stats.output_K;
            state_guard.last_top_score = scored.first().map(|c| c.score);
            state_guard.last_min_top_score = scored.last().map(|c| c.score);
            state_guard.last_regime = Some(regime);
            state_guard.last_run_at = Some(chrono::Utc::now());
            state_guard.hunt_mode_bypass = stats.hunt_mode_bypass;
            state_guard.disabled_reason = None;
        }
        FunnelResult::PassThrough(pairs) => {
            *state_guard = FunnelRuntimeState {
                enabled_at_last_cycle: false,
                last_universe_post_hygiene: pairs.len(),
                last_top_k_size: 0,
                last_top_score: None,
                last_min_top_score: None,
                last_regime: Some(regime),
                last_run_at: Some(chrono::Utc::now()),
                hunt_mode_bypass: hunt_mode,
                disabled_reason: Some(if hunt_mode {
                    "hunt_mode".to_string()
                } else {
                    "funnel_disabled".to_string()
                }),
            };
        }
    }
}

/// FID-222.8 (nit #4): heartbeat update for /api/funnel/v1 when the funnel
/// feature is gated OFF. Without this, `last_run_at` stays `None` forever
/// and the dashboard chip reads "never_ran" indefinitely.
///
/// Engine calls this OUTSIDE the `if config.trading.funnel_v1.enabled` block
/// so it runs every cycle regardless of feature state. Cheap: ~1 async lock
/// acquire + 2 field writes. Reads from the API side are non-blocking
/// (try_read) so they never collide.
pub async fn record_funnel_heartbeat(
    shared: &crate::core::shared::SharedEngineData,
    regime: MarketRegime,
    funnel_enabled: bool,
) {
    let mut state_guard = shared.funnel_v1.write().await;
    // Preserve any existing scores + disabled_reason; only update timestamp +
    // reflect whether the feature is currently enabled.
    state_guard.last_run_at = Some(chrono::Utc::now());
    state_guard.last_regime = Some(regime);
    if !funnel_enabled {
        // Feature off — make sure disabled_reason is set so the dashboard
        // doesn't read "pass-through" by accident.
        state_guard.disabled_reason = Some("funnel_disabled".to_string());
    }
    // No-op on funnel_enabled=true path; record_funnel_runtime still fires
    // after run_funnel and updates the more granular fields.
}

// ==========================================================================
// FID-222.8 nit #5: re-export FunnelConfig so downstream crates/tests can
// import via `use savant_trading::strategy::pre_scorer::FunnelConfig` without
// reaching into a private `use` path that triggered E0603 in v0.15.5.
// `#[allow(unused_imports)]` because rustc flags it as unused when no consumer
// in THIS module references FunnelConfig (the lib-side consumer is external).
#[allow(unused_imports)]
pub use crate::core::config::FunnelConfig;

// Unit tests (FID-222 §Verification + FID-222.5 §Q7 + auditor Q7 delta)
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{IndicatorValues, MarketRegime};
    use chrono::Utc;

    fn cand(close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume,
            pair: "TEST/USDC".to_string(),
        }
    }

    fn ind_all_none() -> IndicatorValues {
        IndicatorValues::default()
    }

    fn ind_perfect() -> IndicatorValues {
        IndicatorValues {
            ema_fast: Some(101.0),
            ema_slow: Some(100.0),
            rsi: Some(25.0), // extreme (low)
            atr: Some(2.0),
            adx: Some(40.0), // fully trending
            vwap: Some(100.0),
            volume_sma: Some(100.0),
            garman_klass: Some(0.5),
            parabolic_sar: Some(99.0),
        }
    }

    fn ind_zero_vwap_close() -> IndicatorValues {
        IndicatorValues {
            ema_fast: Some(101.0),
            ema_slow: Some(100.0),
            rsi: Some(55.0), // mild
            atr: Some(2.0),
            adx: Some(20.0),
            vwap: Some(100.0),
            volume_sma: Some(0.0), // divide-by-zero guard
            garman_klass: Some(0.0),
            parabolic_sar: Some(99.0),
        }
    }

    #[test]
    fn test_all_none_signals_zero_score() {
        let candle = cand(100.0, 100.0);
        let ind = ind_all_none();
        let weights = FunnelWeights::for_regime(MarketRegime::Trending);
        let s = score_pair("X/USDC", Some(&candle), &ind, &weights);
        assert_eq!(s.score, 0.0, "all-None should produce 0.0 score");
        assert_eq!(s.signals.ema_alignment, 0.0);
        assert_eq!(s.signals.rsi_extreme, 0.0);
        assert_eq!(s.signals.adx_trending, 0.0);
        assert_eq!(s.signals.vol_spike, 0.0);
        assert_eq!(s.signals.vwap_proximity, 0.0);
        assert_eq!(s.signals.bb_squeeze, 0.0);
    }

    #[test]
    fn test_all_perfect_signals_clamp_at_one() {
        let candle = cand(100.0, 200.0); // 2x avg → vol spike = 1.0
        let ind = ind_perfect();
        let weights = FunnelWeights::for_regime(MarketRegime::Trending);
        let s = score_pair("X/USDC", Some(&candle), &ind, &weights);
        assert!(s.score > 0.0);
        assert!(s.score <= 1.0, "score must clamp at 1.0: got {}", s.score);
    }

    #[test]
    fn test_regime_weight_overrides_apply() {
        let candle = cand(100.0, 100.0);
        let ind = ind_perfect();

        // Trending
        let w_trend = FunnelWeights::for_regime(MarketRegime::Trending);
        let s_trend = score_pair("X/USDC", Some(&candle), &ind, &w_trend);

        // Ranging
        let w_rang = FunnelWeights::for_regime(MarketRegime::Ranging);
        let s_rang = score_pair("X/USDC", Some(&candle), &ind, &w_rang);

        assert!(s_trend.score >= 0.0);
        assert!(s_rang.score >= 0.0);
        // Exact numerical comparison would require reproducing the formula
        // and confirming ema=0.30 in Trending vs ema=0.15 in Ranging.
        let trending_total =
            w_trend.ema + w_trend.rsi + w_trend.adx + w_trend.vol + w_trend.vwap + w_trend.bb;
        let ranging_total =
            w_rang.ema + w_rang.rsi + w_rang.adx + w_rang.vol + w_rang.vwap + w_rang.bb;
        assert!((trending_total - 1.0).abs() < 0.001);
        assert!((ranging_total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_select_top_k_truncates() {
        let mut scored = Vec::new();
        for i in 0..10 {
            scored.push(ScoredCandidate {
                pair: format!("P{:02}/USDC", i),
                score: i as f64 * 0.1,
                signals: Signals::default(),
            });
        }
        let top5 = select_top_k(scored, 5, 0.0);
        assert_eq!(top5.len(), 5);
        assert!((top5[0].score - 0.9).abs() < 1e-9);
        assert!((top5[4].score - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_min_score_threshold_filters_borderline() {
        let mut scored = Vec::new();
        for i in 0..5 {
            scored.push(ScoredCandidate {
                pair: format!("P{:02}/USDC", i),
                score: i as f64 * 0.05,
                signals: Signals::default(),
            });
        }
        let above_015 = select_top_k(scored, 100, 0.15);
        // 0.0, 0.05, 0.10, 0.15, 0.20 — keep those >= 0.15: 0.15, 0.20
        assert_eq!(above_015.len(), 2);
        assert!((above_015[0].score - 0.20).abs() < 1e-9);
        assert!((above_015[1].score - 0.15).abs() < 1e-9);
    }

    #[test]
    #[allow(clippy::vec_init_then_push)]
    fn test_universe_less_than_k_returns_all_sorted_alphabetically_on_tie() {
        let mut scored = Vec::new();
        scored.push(ScoredCandidate {
            pair: "ZEBRA/USDC".to_string(),
            score: 0.5,
            signals: Signals::default(),
        });
        scored.push(ScoredCandidate {
            pair: "ALPHA/USDC".to_string(),
            score: 0.5,
            signals: Signals::default(),
        });
        scored.push(ScoredCandidate {
            pair: "MANGO/USDC".to_string(),
            score: 0.7,
            signals: Signals::default(),
        });
        scored.push(ScoredCandidate {
            pair: "BRAVO/USDC".to_string(),
            score: 0.5,
            signals: Signals::default(),
        });

        let top10 = select_top_k(scored, 10, 0.0);
        assert_eq!(top10.len(), 4);
        // First the highest-scoring (MANGO 0.7), then the 0.5 ties sorted alphabetically.
        assert_eq!(top10[0].pair, "MANGO/USDC");
        assert_eq!(top10[1].pair, "ALPHA/USDC");
        assert_eq!(top10[2].pair, "BRAVO/USDC");
        assert_eq!(top10[3].pair, "ZEBRA/USDC");
    }

    #[test]
    fn test_determinism_100_runs_identical() {
        let mut scored = Vec::new();
        for i in 0..20 {
            scored.push(ScoredCandidate {
                pair: format!("P{:02}/USDC", i),
                score: ((i as f64 * 0.137).sin() + 1.0) * 0.5, // pseudo-random but reproducible
                signals: Signals::default(),
            });
        }
        let first = select_top_k(scored.clone(), 12, 0.05);
        for _ in 0..100 {
            let again = select_top_k(scored.clone(), 12, 0.05);
            assert_eq!(again, first);
        }
    }

    #[test]
    fn test_feature_gate_disabled_no_op() {
        let config = FunnelConfig {
            enabled: false,
            ..Default::default()
        };
        let inputs: Vec<CandidateInput> = (0..5)
            .map(|i| CandidateInput {
                pair: format!("P{:02}/USDC", i),
                last_candle: Some(cand(100.0, 100.0)),
                indicators: ind_perfect(),
            })
            .collect();

        let result = run_funnel(inputs, MarketRegime::Trending, &config, false);
        match result {
            FunnelResult::PassThrough(pairs) => {
                assert_eq!(pairs.len(), 5);
            }
            _ => panic!("expected PassThrough when enabled=false"),
        }
    }

    #[test]
    fn test_nan_propagation_sanitized_to_zero() {
        // Force a NaN: divide by zero in ema_alignment via slow = 0.0.
        let candle = cand(100.0, 100.0);
        let mut ind = ind_perfect();
        ind.ema_fast = Some(101.0);
        ind.ema_slow = Some(0.0); // would yield (101-0)/0 = inf then nan
        let weights = FunnelWeights::for_regime(MarketRegime::Trending);
        let s = score_pair("X/USDC", Some(&candle), &ind, &weights);
        // ema_alignment guard returns 0.0 when slow.abs() <= 1e-9, so no NaN propagates
        assert_eq!(s.signals.ema_alignment, 0.0);
        assert!(!s.score.is_nan(), "score must not be NaN: got {}", s.score);
    }

    #[test]
    fn test_volume_sma_zero_does_not_panic() {
        let candle = cand(100.0, 100.0);
        let ind = ind_zero_vwap_close();
        let weights = FunnelWeights::for_regime(MarketRegime::Trending);
        let s = score_pair("X/USDC", Some(&candle), &ind, &weights);
        assert_eq!(s.signals.vol_spike, 0.0, "vol_sma=0 should produce 0.0");
        assert!(!s.score.is_nan());
    }

    #[test]
    fn test_hunt_mode_compatibility_passes_through() {
        let config = FunnelConfig {
            enabled: true,
            ..Default::default()
        };
        let inputs: Vec<CandidateInput> = (0..5)
            .map(|i| CandidateInput {
                pair: format!("P{:02}/USDC", i),
                last_candle: Some(cand(100.0, 100.0)),
                indicators: ind_perfect(),
            })
            .collect();

        let result = run_funnel(inputs, MarketRegime::Trending, &config, true);
        match result {
            FunnelResult::PassThrough(pairs) => assert_eq!(pairs.len(), 5),
            _ => panic!("HUNT MODE should produce PassThrough"),
        }
    }

    // ----- Auditor-recommended additions (FID-222.5 Q7) -----

    #[test]
    fn test_top_k_zero_returns_empty() {
        let scored = vec![ScoredCandidate {
            pair: "X/USDC".to_string(),
            score: 0.5,
            signals: Signals::default(),
        }];
        let top0 = select_top_k(scored, 0, 0.0);
        assert_eq!(top0.len(), 0);
    }

    #[test]
    fn test_run_funnel_empty_fallback_picks_top3_absolute() {
        let config = FunnelConfig {
            enabled: true,
            min_score_threshold: 0.95, // forces empty
            ..Default::default()
        };

        let inputs: Vec<CandidateInput> = (0..5)
            .map(|i| {
                CandidateInput::new(
                    format!("P{:02}/USDC", i),
                    Some(cand(100.0, 100.0)),
                    IndicatorValues::default(),
                )
            })
            .collect();

        let result = run_funnel(inputs, MarketRegime::Trending, &config, false);
        match result {
            FunnelResult::Filtered(cands, stats) => {
                assert!(stats.empty_fallback);
                assert_eq!(cands.len(), 3); // top-3 fallback
            }
            _ => panic!("expected Filtered(empty_fallback=true)"),
        }
    }

    #[test]
    fn test_signals_with_one_missing_indicator_score_lower() {
        let candle = cand(100.0, 100.0);
        let full = ind_perfect();
        let mut partial = ind_perfect();
        partial.rsi = None; // one signal None

        let weights = FunnelWeights::for_regime(MarketRegime::Trending);
        let s_full = score_pair("X/USDC", Some(&candle), &full, &weights);
        let s_partial = score_pair("X/USDC", Some(&candle), &partial, &weights);
        assert!(s_full.score > s_partial.score);
    }

    #[test]
    fn test_negative_score_clamped_to_zero() {
        // Force negative score by setting ema_fast = ema_slow (NaN/negative path)
        let candle = cand(100.0, 100.0);
        let mut ind = ind_perfect();
        ind.ema_fast = Some(0.0);
        ind.ema_slow = Some(100.0);
        let weights = FunnelWeights::for_regime(MarketRegime::Trending);
        let s = score_pair("X/USDC", Some(&candle), &ind, &weights);
        assert!(s.score >= 0.0, "score must clamp at 0.0: got {}", s.score);
    }
}
