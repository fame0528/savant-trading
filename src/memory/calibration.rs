//! Brier Score confidence calibration.
//!
//! Measures the mean squared difference between predicted probabilities
//! and actual outcomes. Decomposes into reliability, resolution, uncertainty.

use serde::{Deserialize, Serialize};

/// Brier Score decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrierScore {
    /// Total score (lower = better calibration). Perfect = 0, random = 1.
    pub total: f64,
    /// Reliability (calibration) — how close predictions match outcomes.
    pub reliability: f64,
    /// Resolution — ability to distinguish good setups from bad.
    pub resolution: f64,
    /// Uncertainty — inherent market entropy.
    pub uncertainty: f64,
}

/// Calculate Brier Score from a list of (predicted_probability, actual_outcome) pairs.
pub fn calculate_brier_score(predictions: &[(f64, bool)]) -> BrierScore {
    if predictions.is_empty() {
        return BrierScore {
            total: 0.0,
            reliability: 0.0,
            resolution: 0.0,
            uncertainty: 0.0,
        };
    }

    let n = predictions.len() as f64;
    let mean_outcome = predictions.iter().filter(|(_, o)| *o).count() as f64 / n;

    // Reliability: how close predictions are to actual outcomes
    let reliability = predictions
        .iter()
        .map(|(p, o)| {
            let outcome = if *o { 1.0 } else { 0.0 };
            (p - outcome).powi(2)
        })
        .sum::<f64>()
        / n;

    // Resolution: how much predictions vary from the mean
    let resolution = predictions
        .iter()
        .map(|(p, _)| (p - mean_outcome).powi(2))
        .sum::<f64>()
        / n;

    // Uncertainty: inherent binary entropy
    let uncertainty = mean_outcome * (1.0 - mean_outcome);

    BrierScore {
        total: reliability - resolution + uncertainty,
        reliability,
        resolution,
        uncertainty,
    }
}

/// Map conviction level to numeric probability.
pub fn conviction_to_probability(conviction: &str) -> f64 {
    match conviction {
        "HIGH" => 0.75,
        "MEDIUM" => 0.50,
        "LOW" => 0.25,
        _ => 0.0,
    }
}

/// Progressive confidence caps based on trade count.
pub fn max_conviction_for_trade_count(total_trades: i64, current_win_rate: f64) -> &'static str {
    if total_trades < 25 {
        "LOW"
    } else if total_trades < 50 {
        if current_win_rate > 0.50 {
            "MEDIUM"
        } else {
            "LOW"
        }
    } else {
        "HIGH"
    }
}

/// Calculate confidence penalty from Brier Score.
/// Returns 0.0 (no penalty) to 0.5 (severe penalty).
pub fn confidence_penalty_from_brier(brier: &BrierScore) -> f64 {
    if brier.total <= 0.15 {
        0.0 // Excellent calibration
    } else if brier.total <= 0.25 {
        0.1 // Mild penalty
    } else if brier.total <= 0.35 {
        0.25 // Moderate penalty
    } else {
        0.5 // Severe penalty — agent is poorly calibrated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_calibration() {
        // Predictions match outcomes exactly
        let predictions = vec![(1.0, true), (0.0, false), (1.0, true), (0.0, false)];
        let score = calculate_brier_score(&predictions);
        assert!(score.total < 0.01);
    }

    #[test]
    fn worst_calibration() {
        // Predictions are opposite of outcomes
        let predictions = vec![(1.0, false), (0.0, true), (1.0, false), (0.0, true)];
        let score = calculate_brier_score(&predictions);
        assert!(score.total > 0.9);
    }

    #[test]
    fn empty_predictions() {
        let score = calculate_brier_score(&[]);
        assert_eq!(score.total, 0.0);
    }

    #[test]
    fn progressive_confidence() {
        assert_eq!(max_conviction_for_trade_count(10, 0.6), "LOW");
        assert_eq!(max_conviction_for_trade_count(30, 0.6), "MEDIUM");
        assert_eq!(max_conviction_for_trade_count(30, 0.4), "LOW");
        assert_eq!(max_conviction_for_trade_count(60, 0.6), "HIGH");
    }

    #[test]
    fn confidence_penalty() {
        let excellent = BrierScore {
            total: 0.10,
            reliability: 0.05,
            resolution: 0.05,
            uncertainty: 0.10,
        };
        assert_eq!(confidence_penalty_from_brier(&excellent), 0.0);

        let poor = BrierScore {
            total: 0.40,
            reliability: 0.30,
            resolution: 0.05,
            uncertainty: 0.15,
        };
        assert_eq!(confidence_penalty_from_brier(&poor), 0.5);
    }
}
