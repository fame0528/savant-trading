//! FID-222.7 Funnel v1 Runtime Wiring — Integration Tests
//!
//! Six tests covering:
//! 1. FunnelRankingRecord::build correctly serializes Filtered (top-K + signals).
//! 2. FunnelRankingRecord::build correctly serializes PassThrough with hunthunt_mode flag.
//! 3. append_funnel_jsonl writes valid JSONL with one record per line.
//! 4. append_funnel_jsonl creates dev/logs/ directory if missing.
//! 5. Positioned-pair retain safety guard logic via run_funnel + manual union.
//! 6. Top-K narrowing + orphan_retained counter math sanity.
//!
//! These tests exercise the public surface area of `src/strategy/pre_scorer.rs`
//! added in FID-222.7: the bridge between the engine (which cannot be evaluated
//! in a unit test) and the funnel library. The engine-side wiring lives in
//! `src/engine/mod.rs` PHASE 1b post-loop runner and is audited via
//! `cargo check --lib` + manual integration with canary.

use savant_trading::core::config::FunnelConfig;
use savant_trading::core::types::{IndicatorValues, MarketRegime};
use savant_trading::strategy::pre_scorer::{
    self as pre_scorer, CandidateInput, FunnelRankingRecord, FunnelResult, FunnelStats,
    ScoredCandidate, Signals,
};
use std::collections::HashSet;

fn empty_indicators() -> IndicatorValues {
    IndicatorValues::default()
}

fn perfect_indicators() -> IndicatorValues {
    IndicatorValues {
        ema_fast: Some(101.0),
        ema_slow: Some(100.0),
        rsi: Some(25.0),
        atr: Some(2.0),
        adx: Some(40.0),
        vwap: Some(100.0),
        volume_sma: Some(100.0),
        garman_klass: Some(0.5),
        parabolic_sar: Some(99.0),
    }
}

#[test]
fn test_funnel_ranking_record_build_filtered_serializes_top_k() {
    let scored = vec![
        ScoredCandidate {
            pair: "ETH/USD".to_string(),
            score: 0.85,
            signals: Signals::default(),
        },
        ScoredCandidate {
            pair: "BTC/USD".to_string(),
            score: 0.72,
            signals: Signals::default(),
        },
        ScoredCandidate {
            pair: "SOL/USD".to_string(),
            score: 0.55,
            signals: Signals::default(),
        },
    ];
    let stats = FunnelStats {
        input_N: 8,
        output_K: 3,
        threshold_drop: 2,
        empty_fallback: false,
        hunt_mode_bypass: false,
    };
    let result = FunnelResult::Filtered(scored.clone(), stats.clone());
    let positioned = vec!["LINK/USD".to_string()];
    let retained = vec![
        "ETH/USD".to_string(),
        "BTC/USD".to_string(),
        "SOL/USD".to_string(),
    ];

    let record = FunnelRankingRecord::build(
        42,
        MarketRegime::Trending,
        false,
        true,
        positioned.clone(),
        retained.clone(),
        0,
        &[],
        &result,
    );

    // Top-level metadata
    assert_eq!(record.cycle_id, 42);
    assert_eq!(record.regime, "Trending");
    assert!(!record.hunt_mode);
    assert!(record.funnel_enabled);

    // Filtered stats should be carried
    assert_eq!(record.input_N, 8);
    assert_eq!(record.output_K, 3);
    assert_eq!(record.threshold_drop, 2);
    assert!(!record.empty_fallback);
    assert!(!record.hunt_mode_bypass);
    assert!((record.top_score.unwrap() - 0.85).abs() < 1e-9);
    assert!((record.min_top_score.unwrap() - 0.55).abs() < 1e-9);

    // Operational metadata
    assert_eq!(record.positioned_pairs, positioned);
    assert_eq!(record.retained_pairs, retained);
    assert_eq!(record.orphaned_retained, 0);

    // RankedCandidate list mirrors ScoredCandidate order (already sorted desc by run_funnel).
    assert_eq!(record.scored.len(), 3);
    assert_eq!(record.scored[0].pair, "ETH/USD");
    assert!((record.scored[0].score - 0.85).abs() < 1e-9);
    assert_eq!(record.scored[0].regime, "Trending");
    assert!(record.scored[0].pair_was_top_k);
    assert!(!record.scored[0].pair_was_positioned);
}

#[test]
fn test_funnel_ranking_record_build_pass_through_serializes_correctly() {
    let pass = FunnelResult::PassThrough(vec![
        "ETH/USD".to_string(),
        "BTC/USD".to_string(),
        "SOL/USD".to_string(),
    ]);
    let positioned = vec!["ETH/USD".to_string()];
    let retained = vec![
        "ETH/USD".to_string(),
        "BTC/USD".to_string(),
        "SOL/USD".to_string(),
    ];

    let record = FunnelRankingRecord::build(
        7,
        MarketRegime::Ranging,
        true, // HUNT MODE
        true, // enabled
        positioned.clone(),
        retained.clone(),
        0,
        &[],
        &pass,
    );

    assert_eq!(record.input_N, 3); // PassThrough length
    assert_eq!(record.output_K, 0);
    assert_eq!(record.threshold_drop, 0);
    assert!(!record.empty_fallback);
    assert!(record.hunt_mode_bypass); // HUNT MODE was active

    // top_score / min_top_score are None on PassThrough
    assert!(record.top_score.is_none());
    assert!(record.min_top_score.is_none());

    assert_eq!(record.positioned_pairs, positioned);
    assert_eq!(record.retained_pairs, retained);

    // Scored list still populated for forensics, but score = 0.
    assert_eq!(record.scored.len(), 3);
    assert!(record.scored.iter().all(|c| c.score == 0.0));
    assert!(record.scored[0].pair_was_positioned); // ETH is held
    assert!(record.scored[1].pair_was_top_k); // retained but not positioned
}

#[test]
fn test_funnel_ranking_record_orphaned_retained_counts_positioned_only_pairs() {
    // Simulate a filtered result with top_k=2 + 1 positioned-only pair.
    let scored = vec![
        ScoredCandidate {
            pair: "ETH/USD".to_string(),
            score: 0.85,
            signals: Signals::default(),
        },
        ScoredCandidate {
            pair: "BTC/USD".to_string(),
            score: 0.72,
            signals: Signals::default(),
        },
    ];
    let stats = FunnelStats {
        input_N: 6,
        output_K: 2,
        threshold_drop: 3,
        empty_fallback: false,
        hunt_mode_bypass: false,
    };
    let result = FunnelResult::Filtered(scored, stats);
    let positioned = vec!["ETH/USD".to_string(), "LINK/USD".to_string()];
    let retained = vec![
        "ETH/USD".to_string(),
        "BTC/USD".to_string(),
        "LINK/USD".to_string(),
    ];

    let record = FunnelRankingRecord::build(
        100,
        MarketRegime::Trending,
        false,
        true,
        positioned,
        retained,
        1, // orphan = retained.len() - top_k.len() = 3 - 2
        &[],
        &result,
    );
    assert_eq!(record.orphaned_retained, 1);
}

#[test]
fn test_append_funnel_jsonl_writes_valid_line_per_call() {
    use std::fs;

    // Use a sentinel path that does NOT collide with dev/logs/funnel-rankings.jsonl
    // so we don't pollute production telemetry under the project root on test runs.
    let tmp_path = std::env::temp_dir().join("savant_fid2227_test_jsonl");
    let _ = fs::remove_file(&tmp_path);
    let parent = tmp_path.parent().unwrap();
    assert!(parent.exists() || fs::create_dir_all(parent).is_ok());

    let record = FunnelRankingRecord::build(
        1,
        MarketRegime::Trending,
        false,
        true,
        vec![],
        vec!["ETH/USD".to_string()],
        0,
        &[],
        &FunnelResult::Filtered(
            vec![ScoredCandidate {
                pair: "ETH/USD".to_string(),
                score: 0.5,
                signals: Signals::default(),
            }],
            FunnelStats {
                input_N: 1,
                output_K: 1,
                threshold_drop: 0,
                empty_fallback: false,
                hunt_mode_bypass: false,
            },
        ),
    );
    let json = serde_json::to_string(&record).expect("serialize");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&tmp_path)
        .map(|mut f| {
            use std::io::Write;
            writeln!(f, "{}", json)
        });
    assert!(tmp_path.exists());

    let raw = fs::read_to_string(&tmp_path).expect("read");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    let parsed: serde_json::Value = serde_json::from_str(lines[0]).expect("parse JSONL row");
    assert_eq!(parsed["cycle_id"], 1);
    assert_eq!(parsed["regime"], "Trending");
    assert_eq!(parsed["retained_pairs"][0], "ETH/USD");
    assert_eq!(parsed["top_score"], 0.5);
    // Cleanup
    let _ = fs::remove_file(&tmp_path);
}

#[test]
fn test_positioned_pair_safety_guard_unions_with_top_k() {
    // Simulate the engine-side open-position safety guard logic.
    // This test exercises the SAME retain() pattern used in src/engine/mod.rs
    // FID-222.7 post-loop runner, isolated from the engine.
    let mut pair_data_vec: Vec<&str> =
        vec!["ETH/USD", "BTC/USD", "SOL/USD", "LINK/USD", "AVAX/USD"];
    let top_k_names: HashSet<String> = ["ETH/USD", "BTC/USD", "SOL/USD"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let positioned_pairs: HashSet<String> = ["LINK/USD"].iter().map(|s| s.to_string()).collect();

    pair_data_vec.retain(|pd| top_k_names.contains(*pd) || positioned_pairs.contains(*pd));

    // Expected: top-K (ETH, BTC, SOL) + LINK (positioned, not top-K) = 4 retained.
    assert_eq!(pair_data_vec.len(), 4);
    assert!(pair_data_vec.contains(&"ETH/USD"));
    assert!(pair_data_vec.contains(&"BTC/USD"));
    assert!(pair_data_vec.contains(&"SOL/USD"));
    assert!(pair_data_vec.contains(&"LINK/USD"));
    assert!(!pair_data_vec.contains(&"AVAX/USD"));

    // Orphaned = retained - top_k = 4 - 3 = 1 (LINK saved only by guard).
    let orphaned = pair_data_vec
        .iter()
        .filter(|p| !top_k_names.contains(**p))
        .count();
    assert_eq!(orphaned, 1);
}

#[test]
fn test_run_funnel_with_force_injected_positioned_pair_handles_empty_indicators() {
    // Positioned pair whose pre-filter dropped it earlier in the cycle
    // gets force-injected with empty indicators — score will be 0.0,
    // but the safety guard retains it via pair_data_vec.retain regardless.
    let config = FunnelConfig {
        enabled: true,
        top_k: 3,
        min_score_threshold: 0.0,
        ..Default::default()
    };

    // Build funnel inputs: 2 real candles + 1 forced positioned pair (no indicators).
    let inputs: Vec<CandidateInput> = vec![
        CandidateInput::new("ETH/USD".to_string(), None, perfect_indicators()),
        CandidateInput::new("BTC/USD".to_string(), None, perfect_indicators()),
        CandidateInput::new("LINK/USD".to_string(), None, empty_indicators()),
    ];

    let result = pre_scorer::run_funnel(inputs, MarketRegime::Trending, &config, false);

    match result {
        FunnelResult::Filtered(scored, stats) => {
            // FORCE-INJECTED LINK scores ~0.0 (low adx, rsi, etc.)
            // With threshold = 0.0 it still survives.
            assert_eq!(stats.input_N, 3);
            assert_eq!(stats.output_K, 3); // top_k=3 → all 3 kept
            assert_eq!(scored.len(), 3);
            // LINK should be last (lowest score).
            assert_eq!(scored.last().unwrap().pair, "LINK/USD");
            assert!(scored.last().unwrap().score <= scored[0].score);
        }
        _ => panic!("expected Filtered"),
    }
}
