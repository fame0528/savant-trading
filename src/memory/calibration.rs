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

/// Isotonic Regression calibrator using Pool Adjacent Violators (PAVA).
///
/// Maps raw LLM confidence scores to calibrated probabilities based on
/// historical (confidence, outcome) pairs. Non-parametric — makes no
/// assumptions about the distribution shape.
///
/// Usage:
/// 1. Train on historical data: `let cal = IsotonicCalibrator::fit(&data);`
/// 2. Calibrate new scores: `let calibrated = cal.calibrate(raw_confidence);`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsotonicCalibrator {
    /// Sorted (confidence, calibrated_probability) pairs
    thresholds: Vec<(f64, f64)>,
}

impl IsotonicCalibrator {
    /// Fit an Isotonic Regression model from (confidence, outcome) pairs.
    ///
    /// Implements the Pool Adjacent Violators Algorithm (PAVA):
    /// - Sort by predicted confidence
    /// - Merge adjacent bins that violate monotonicity
    /// - Result: a non-decreasing mapping from confidence to probability
    pub fn fit(predictions: &[(f64, bool)]) -> Self {
        if predictions.is_empty() {
            return Self {
                thresholds: vec![(0.0, 0.0), (1.0, 1.0)],
            };
        }

        // Sort by confidence score
        let mut sorted: Vec<(f64, f64)> = predictions
            .iter()
            .map(|(conf, outcome)| (*conf, if *outcome { 1.0 } else { 0.0 }))
            .collect();
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Pool Adjacent Violators Algorithm
        let mut bins: Vec<(f64, f64, usize)> = sorted
            .iter()
            .map(|(conf, outcome)| (*conf, *outcome, 1))
            .collect();

        loop {
            let mut violated = false;
            let mut new_bins: Vec<(f64, f64, usize)> = Vec::new();

            for bin in &bins {
                if let Some(last) = new_bins.last_mut() {
                    if bin.1 < last.1 {
                        // Violation: merge with previous bin
                        let total_count = last.2 + bin.2;
                        last.1 =
                            (last.1 * last.2 as f64 + bin.1 * bin.2 as f64) / total_count as f64;
                        last.0 =
                            (last.0 * last.2 as f64 + bin.0 * bin.2 as f64) / total_count as f64;
                        last.2 = total_count;
                        violated = true;
                    } else {
                        new_bins.push(*bin);
                    }
                } else {
                    new_bins.push(*bin);
                }
            }

            bins = new_bins;
            if !violated {
                break;
            }
        }

        // Convert bins to threshold pairs
        let thresholds: Vec<(f64, f64)> =
            bins.iter().map(|(conf, prob, _)| (*conf, *prob)).collect();

        Self { thresholds }
    }

    /// Calibrate a raw confidence score using the fitted model.
    ///
    /// Uses linear interpolation between threshold points.
    /// Clamps to [0.0, 1.0].
    pub fn calibrate(&self, raw_confidence: f64) -> f64 {
        if self.thresholds.is_empty() {
            return raw_confidence;
        }

        // Below first threshold: use first calibrated probability
        if raw_confidence <= self.thresholds[0].0 {
            return self.thresholds[0].1;
        }

        // Above last threshold: use last calibrated probability
        if raw_confidence >= self.thresholds.last().unwrap().0 {
            return self.thresholds.last().unwrap().1;
        }

        // Linear interpolation between adjacent thresholds
        for i in 0..self.thresholds.len() - 1 {
            let (x0, y0) = self.thresholds[i];
            let (x1, y1) = self.thresholds[i + 1];
            if raw_confidence >= x0 && raw_confidence <= x1 {
                if (x1 - x0).abs() < f64::EPSILON {
                    return y0;
                }
                let t = (raw_confidence - x0) / (x1 - x0);
                return y0 + t * (y1 - y0);
            }
        }

        raw_confidence
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

    #[test]
    fn isotonic_perfect_calibration() {
        let data = vec![
            (0.1, false),
            (0.2, false),
            (0.3, false),
            (0.4, false),
            (0.5, true),
            (0.6, true),
            (0.7, true),
            (0.8, true),
            (0.9, true),
        ];
        let cal = IsotonicCalibrator::fit(&data);
        // Low confidence → low calibrated probability
        assert!(cal.calibrate(0.1) < 0.3);
        // High confidence → high calibrated probability
        assert!(cal.calibrate(0.9) > 0.7);
    }

    #[test]
    fn isotonic_miscalibrated_model() {
        // Model outputs 80% confidence but only wins 40% of the time
        let data: Vec<(f64, bool)> = (0..100)
            .map(|i| {
                let outcome = i < 40; // 40% win rate
                (0.8, outcome)
            })
            .collect();
        let cal = IsotonicCalibrator::fit(&data);
        let calibrated = cal.calibrate(0.8);
        // Should be close to 0.4 (actual win rate), not 0.8 (reported confidence)
        assert!((calibrated - 0.4).abs() < 0.1);
    }

    #[test]
    fn isotonic_empty_data() {
        let cal = IsotonicCalibrator::fit(&[]);
        assert_eq!(cal.calibrate(0.5), 0.5);
    }
}
