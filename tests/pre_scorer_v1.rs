//! FID-222.6 — end-to-end Funnel v1 glue test.
//!
//! Exercises the integration seam between `pre_scorer` and `core::shared`:
//! - `run_funnel` returns the correct `Filtered` vs `PassThrough` for a
//!   small synthetic universe of `CandidateInput`s.
//! - `core::shared::FunnelRuntimeState` serializes via serde round-trip.
//! - When HUNT MODE is true, the engine-side caller passes a `hunt_mode=true`
//!   flag and the funnel returns PassThrough with `hunt_mode_bypass=true`
//!   — refusing to truncate the universe.
//!
//! These tests live at integration scope (not #[cfg(test)] inside pre_scorer)
//! to verify the public API contracts the FID-222.6 engine wiring depends on.
//! The 15 unit tests inside `src/strategy/pre_scorer.rs` cover signal math;
//! here we cover the boundary.

use chrono::Utc;
use savant_trading::core::config::{FunnelConfig, FunnelWeightsFields, FunnelWeightsTriple};
use savant_trading::core::shared::FunnelRuntimeState;
use savant_trading::core::types::{Candle, IndicatorValues, MarketRegime};
use savant_trading::strategy::pre_scorer::{run_funnel, CandidateInput, FunnelResult};

fn make_candle(close: f64, volume: f64, pair: &str) -> Candle {
    Candle {
        timestamp: Utc::now(),
        open: close,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume,
        pair: pair.to_string(),
    }
}

fn make_indicators(
    ema_fast: Option<f64>,
    ema_slow: Option<f64>,
    rsi: Option<f64>,
    adx: Option<f64>,
    vwap: Option<f64>,
    volume_sma: Option<f64>,
) -> IndicatorValues {
    IndicatorValues {
        ema_fast,
        ema_slow,
        rsi,
        atr: Some(2.0),
        adx,
        vwap,
        volume_sma,
        garman_klass: Some(0.5),
        parabolic_sar: Some(99.0),
    }
}

#[test]
fn funnel_filters_to_top_k_when_enabled() {
    let config = FunnelConfig {
        enabled: true,
        top_k: 3,
        min_score_threshold: 0.05,
        weights_override: None,
    };

    // Build 5 synthetic candidates. First 3 have strong setups, last 2 are weak.
    let inputs = vec![
        CandidateInput::new(
            "WETH/USD".to_string(),
            Some(make_candle(100.0, 200.0, "WETH/USD")),
            make_indicators(
                Some(101.0),
                Some(100.0),
                Some(25.0),
                Some(40.0),
                Some(100.0),
                Some(100.0),
            ),
        ),
        CandidateInput::new(
            "BTC/USD".to_string(),
            Some(make_candle(100.0, 200.0, "BTC/USD")),
            make_indicators(
                Some(102.0),
                Some(100.0),
                Some(28.0),
                Some(38.0),
                Some(100.0),
                Some(100.0),
            ),
        ),
        CandidateInput::new(
            "ARB/USD".to_string(),
            Some(make_candle(100.0, 180.0, "ARB/USD")),
            make_indicators(
                Some(101.0),
                Some(100.0),
                Some(30.0),
                Some(35.0),
                Some(100.0),
                Some(100.0),
            ),
        ),
        CandidateInput::new(
            "WEAK/USD".to_string(),
            Some(make_candle(100.0, 50.0, "WEAK/USD")),
            make_indicators(
                Some(100.01),
                Some(100.0),
                Some(50.0),
                Some(8.0),
                Some(105.0),
                Some(100.0),
            ),
        ),
        CandidateInput::new(
            "WEAKER/USD".to_string(),
            Some(make_candle(100.0, 30.0, "WEAKER/USD")),
            IndicatorValues::default(),
        ),
    ];

    let result = run_funnel(inputs, MarketRegime::Trending, &config, false);
    match result {
        FunnelResult::Filtered(cands, stats) => {
            assert_eq!(cands.len(), 3, "top_k=3 → expect 3 candidates");
            assert_eq!(stats.output_K, 3);
            assert_eq!(stats.input_N, 5);
            assert!(!stats.hunt_mode_bypass);
            // Top-K must include WETH/USD + BTC/USD + ARB/USD (the strong ones)
            assert!(cands.iter().any(|c| c.pair == "WETH/USD"));
            assert!(cands.iter().any(|c| c.pair == "BTC/USD"));
            assert!(cands.iter().any(|c| c.pair == "ARB/USD"));
        }
        FunnelResult::PassThrough(pairs) => panic!(
            "expected Filtered when enabled=true and hunt_mode=false; got PassThrough({} pairs)",
            pairs.len()
        ),
    }
}

#[test]
fn funnel_passes_through_when_disabled() {
    let config = FunnelConfig {
        enabled: false,
        top_k: 3,
        min_score_threshold: 0.05,
        weights_override: None,
    };

    let inputs = vec![CandidateInput::new(
        "WETH/USD".to_string(),
        Some(make_candle(100.0, 200.0, "WETH/USD")),
        make_indicators(
            Some(101.0),
            Some(100.0),
            Some(25.0),
            Some(40.0),
            Some(100.0),
            Some(100.0),
        ),
    )];

    let result = run_funnel(inputs, MarketRegime::Trending, &config, false);
    match result {
        FunnelResult::PassThrough(pairs) => assert_eq!(pairs, vec!["WETH/USD".to_string()]),
        FunnelResult::Filtered(_, _) => panic!("expected PassThrough when enabled=false"),
    }
}

#[test]
fn funnel_passes_through_when_hunt_mode_active() {
    // FID-222 Loop 1.7 Q4: HUNT MODE bypasses Funnel entirely to preserve
    // FID-063 intent (full-universe LLM scan).
    let mut config = FunnelConfig {
        enabled: true,
        top_k: 3,
        min_score_threshold: 0.05,
        weights_override: None,
    };
    // Even with an aggressive weights override, HUNT MODE must win.
    config.weights_override = Some(FunnelWeightsTriple {
        trending: FunnelWeightsFields {
            ema: 1.0,
            rsi: 0.0,
            adx: 0.0,
            vol: 0.0,
            vwap: 0.0,
            bb: 0.0,
        },
        ranging: FunnelWeightsFields::default(),
        volatile: FunnelWeightsFields::default(),
    });

    let inputs = vec![
        CandidateInput::new(
            "WETH/USD".to_string(),
            Some(make_candle(100.0, 200.0, "WETH/USD")),
            make_indicators(
                Some(101.0),
                Some(100.0),
                Some(25.0),
                Some(40.0),
                Some(100.0),
                Some(100.0),
            ),
        ),
        CandidateInput::new(
            "BTC/USD".to_string(),
            Some(make_candle(100.0, 200.0, "BTC/USD")),
            make_indicators(
                Some(102.0),
                Some(100.0),
                Some(28.0),
                Some(38.0),
                Some(100.0),
                Some(100.0),
            ),
        ),
    ];

    let result = run_funnel(inputs, MarketRegime::Trending, &config, true);
    match result {
        FunnelResult::PassThrough(pairs) => {
            assert_eq!(pairs.len(), 2);
            assert!(pairs.contains(&"WETH/USD".to_string()));
            assert!(pairs.contains(&"BTC/USD".to_string()));
        }
        FunnelResult::Filtered(_, _) => {
            panic!("HUNT MODE active → expected PassThrough, got Filtered")
        }
    }
}

#[test]
fn funnel_runtime_state_serializes_via_serde() {
    // FID-222.6 Q-E: shared/funnel_v1 must serialize cleanly for /api/funnel/v1.
    // Verifies Serialize+Deserialize derive + shape compatibility with the
    // dashboard JSON contract.
    let state = FunnelRuntimeState {
        enabled_at_last_cycle: true,
        last_universe_post_hygiene: 250,
        last_top_k_size: 12,
        last_top_score: Some(0.85),
        last_min_top_score: Some(0.42),
        last_regime: Some(MarketRegime::Ranging),
        last_run_at: Some(Utc::now()),
        hunt_mode_bypass: false,
        disabled_reason: None,
    };

    let json = serde_json::to_string(&state).expect("serialize");
    assert!(json.contains("\"enabled_at_last_cycle\":true"));
    assert!(json.contains("\"last_universe_post_hygiene\":250"));
    assert!(json.contains("\"last_top_k_size\":12"));
    assert!(json.contains("\"last_top_score\":0.85"));
    assert!(json.contains("\"last_regime\":\"Ranging\""));
    assert!(json.contains("\"hunt_mode_bypass\":false"));

    let parsed: FunnelRuntimeState = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.last_universe_post_hygiene, 250);
    assert_eq!(parsed.last_top_k_size, 12);
    assert!((parsed.last_top_score.unwrap() - 0.85).abs() < 1e-9);
}

#[test]
fn funnel_runtime_state_records_hunt_mode_bypass() {
    // FID-222 Q4 / Q-G: when funnel is skipped due to HUNT MODE, the
    // runtime state must record `hunt_mode_bypass=true` so the dashboard
    // surfaces "Funnel bypassed this cycle — HUNT MODE active."
    let state = FunnelRuntimeState {
        enabled_at_last_cycle: false,
        last_universe_post_hygiene: 0,
        last_top_k_size: 0,
        last_top_score: None,
        last_min_top_score: None,
        last_regime: None,
        last_run_at: None,
        hunt_mode_bypass: true,
        disabled_reason: Some("hunt_mode_active".to_string()),
    };

    let json = serde_json::to_string(&state).expect("serialize");
    assert!(json.contains("\"hunt_mode_bypass\":true"));
    assert!(json.contains("\"disabled_reason\":\"hunt_mode_active\""));

    let parsed: FunnelRuntimeState = serde_json::from_str(&json).expect("deserialize");
    assert!(parsed.hunt_mode_bypass);
    assert_eq!(parsed.disabled_reason.as_deref(), Some("hunt_mode_active"));
}

#[test]
fn weights_override_applies_for_all_three_regimes() {
    // Confirm FunnelConfig::weights_for honors operator override.
    let config = FunnelConfig {
        enabled: true,
        top_k: 6,
        min_score_threshold: 0.0,
        weights_override: Some(FunnelWeightsTriple {
            trending: FunnelWeightsFields {
                ema: 0.6,
                rsi: 0.05,
                adx: 0.2,
                vol: 0.1,
                vwap: 0.05,
                bb: 0.0,
            },
            ranging: FunnelWeightsFields {
                ema: 0.1,
                rsi: 0.5,
                adx: 0.1,
                vol: 0.05,
                vwap: 0.25,
                bb: 0.0,
            },
            volatile: FunnelWeightsFields {
                ema: 0.05,
                rsi: 0.05,
                adx: 0.1,
                vol: 0.7,
                vwap: 0.1,
                bb: 0.0,
            },
        }),
    };

    let wt = config.weights_for(MarketRegime::Trending);
    assert!((wt.ema - 0.6).abs() < 1e-9);
    assert!((wt.rsi - 0.05).abs() < 1e-9);
    assert!((wt.adx - 0.2).abs() < 1e-9);

    let wr = config.weights_for(MarketRegime::Ranging);
    assert!((wr.rsi - 0.5).abs() < 1e-9);
    assert!((wr.vwap - 0.25).abs() < 1e-9);

    let wv = config.weights_for(MarketRegime::Volatile);
    assert!((wv.vol - 0.7).abs() < 1e-9);
}
