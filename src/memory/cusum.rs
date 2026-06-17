//! CUSUM control chart for edge decay detection.
//!
//! Detects persistent, small shifts in strategy performance
//! earlier than simple moving averages.

use serde::{Deserialize, Serialize};
use tracing::warn;

/// CUSUM control chart state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CusumChart {
    /// Expected R:R target.
    pub target: f64,
    /// Allowance — magnitude of acceptable variance to ignore.
    pub allowance: f64,
    /// Decision interval threshold.
    pub threshold: f64,
    /// Upper cumulative sum (tracks positive shifts).
    pub upper_sum: f64,
    /// Lower cumulative sum (tracks negative shifts).
    pub lower_sum: f64,
    /// Number of data points processed.
    pub sample_count: u64,
}

/// CUSUM alert types.
#[derive(Debug, Clone, PartialEq)]
pub enum CusumAlert {
    None,
    PositiveShift,
    NegativeShift,
}

impl CusumChart {
    /// Create a new CUSUM chart with default parameters.
    pub fn new(target: f64, allowance: f64, threshold: f64) -> Self {
        Self {
            target,
            allowance,
            threshold,
            upper_sum: 0.0,
            lower_sum: 0.0,
            sample_count: 0,
        }
    }

    /// Create with default trading parameters (target R:R = 1.5).
    pub fn default_trading() -> Self {
        Self::new(1.5, 0.5, 5.0)
    }

    /// Update with a new trade result. Returns alert if threshold crossed.
    pub fn update(&mut self, actual_rr: f64) -> CusumAlert {
        let deviation = actual_rr - self.target;
        self.upper_sum = (self.upper_sum + deviation - self.allowance).max(0.0);
        self.lower_sum = (self.lower_sum - deviation - self.allowance).max(0.0);
        self.sample_count += 1;

        if self.upper_sum > self.threshold {
            warn!(
                "CUSUM positive shift detected (S+={:.2} > {:.2}) — edge improving",
                self.upper_sum, self.threshold
            );
            self.upper_sum = 0.0; // Reset after alert
            CusumAlert::PositiveShift
        } else if self.lower_sum > self.threshold {
            warn!(
                "CUSUM negative shift detected (S-={:.2} > {:.2}) — edge decaying",
                self.lower_sum, self.threshold
            );
            self.lower_sum = 0.0; // Reset after alert
            CusumAlert::NegativeShift
        } else {
            CusumAlert::None
        }
    }

    /// Check if edge is currently flagged as decaying.
    pub fn is_decaying(&self) -> bool {
        self.lower_sum > self.threshold * 0.5
    }

    /// Check if edge is currently flagged as improving.
    pub fn is_improving(&self) -> bool {
        self.upper_sum > self.threshold * 0.5
    }

    /// Get status string for display.
    pub fn status(&self) -> String {
        if self.is_decaying() {
            format!("DECAY (S-={})", self.lower_sum)
        } else if self.is_improving() {
            format!("IMPROVING (S+={})", self.upper_sum)
        } else {
            format!("STABLE (S+={}, S-={})", self.upper_sum, self.lower_sum)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_alert_on_normal_values() {
        let mut chart = CusumChart::new(1.5, 0.5, 5.0);
        for _ in 0..10 {
            assert_eq!(chart.update(1.5), CusumAlert::None);
        }
    }

    #[test]
    fn negative_shift_detection() {
        let mut chart = CusumChart::new(1.5, 0.3, 2.0);
        // Consistently bad results should trigger
        let mut triggered = false;
        for _ in 0..20 {
            if chart.update(0.5) == CusumAlert::NegativeShift {
                triggered = true;
                break;
            }
        }
        assert!(triggered);
    }

    #[test]
    fn positive_shift_detection() {
        let mut chart = CusumChart::new(1.5, 0.3, 2.0);
        let mut triggered = false;
        for _ in 0..20 {
            if chart.update(3.0) == CusumAlert::PositiveShift {
                triggered = true;
                break;
            }
        }
        assert!(triggered);
    }

    #[test]
    fn status_display() {
        let chart = CusumChart::new(1.5, 0.5, 5.0);
        assert!(chart.status().contains("STABLE"));
    }

    // === FID-163: Precision preservation test ===

    #[test]
    fn status_preserves_precision() {
        // S+ = 1.234567890123 — old {:.2} would render as "1.23" (lost 7 significant digits).
        // New {} Display must preserve the full f64 value.
        let mut chart = CusumChart::new(1.5, 0.5, 5.0);
        chart.upper_sum = 1.234567890123;
        let s = chart.status();
        assert!(
            s.contains("1.234567890123"),
            "status() should preserve full precision, got: {}",
            s
        );
    }
}
