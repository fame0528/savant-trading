//! Multi-asset correlation matrix — detects when active pairs are highly
//! correlated and adjusts effective position count.
//!
//! Uses rolling Pearson correlation between candle returns. Highly correlated
//! pairs effectively multiply risk without the agent knowing.

use std::collections::HashMap;

use crate::core::types::Candle;

/// Correlation matrix between pairs.
///
/// Stores pairwise Pearson correlation coefficients. Used by the circuit
/// breaker to adjust effective position count.
#[derive(Debug, Clone)]
pub struct CorrelationMatrix {
    /// Pairwise correlations: (pair_a, pair_b) -> correlation
    correlations: HashMap<(String, String), f64>,
}

impl Default for CorrelationMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl CorrelationMatrix {
    pub fn new() -> Self {
        Self {
            correlations: HashMap::new(),
        }
    }

    /// Get correlation between two pairs. Returns 0.0 if not found.
    pub fn get(&self, pair_a: &str, pair_b: &str) -> f64 {
        if pair_a == pair_b {
            return 1.0;
        }
        self.correlations
            .get(&(pair_a.to_string(), pair_b.to_string()))
            .or_else(|| {
                self.correlations
                    .get(&(pair_b.to_string(), pair_a.to_string()))
            })
            .copied()
            .unwrap_or(0.0)
    }

    /// Calculate the effective position multiplier for a set of active pairs.
    ///
    /// If two pairs have correlation > 0.8, they count as 2.0 positions each.
    /// If correlation 0.5-0.8, they count as 1.5 each.
    /// Otherwise, they count as 1.0 each.
    ///
    /// Returns the total effective position count.
    pub fn effective_positions(&self, active_pairs: &[String]) -> f64 {
        if active_pairs.len() <= 1 {
            return active_pairs.len() as f64;
        }

        let mut total = 0.0;
        for (i, pair_a) in active_pairs.iter().enumerate() {
            let mut max_corr_with_other = 0.0f64;
            for (j, pair_b) in active_pairs.iter().enumerate() {
                if i != j {
                    let corr = self.get(pair_a, pair_b).abs();
                    max_corr_with_other = max_corr_with_other.max(corr);
                }
            }

            // Adjust position count based on max correlation with any other active pair
            if max_corr_with_other > 0.8 {
                total += 2.0; // Highly correlated — double risk
            } else if max_corr_with_other > 0.5 {
                total += 1.5; // Moderately correlated
            } else {
                total += 1.0; // Independent
            }
        }

        total
    }
}

/// Build a correlation matrix from candle data for multiple pairs.
///
/// Takes a map of pair -> candles and calculates rolling Pearson correlation
/// between each pair's returns over the last `window` candles.
pub fn build_correlation_matrix(
    pair_candles: &HashMap<String, Vec<Candle>>,
    window: usize,
) -> CorrelationMatrix {
    let mut matrix = CorrelationMatrix::new();
    let pairs: Vec<&String> = pair_candles.keys().collect();

    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            let pair_a = pairs[i];
            let pair_b = pairs[j];

            if let (Some(candles_a), Some(candles_b)) =
                (pair_candles.get(pair_a), pair_candles.get(pair_b))
            {
                let returns_a = candle_returns(candles_a, window);
                let returns_b = candle_returns(candles_b, window);

                if returns_a.len() >= 10 && returns_b.len() >= 10 {
                    let corr = pearson_correlation(&returns_a, &returns_b);
                    matrix
                        .correlations
                        .insert((pair_a.clone(), pair_b.clone()), corr);
                }
            }
        }
    }

    matrix
}

/// Calculate log returns from candle close prices.
fn candle_returns(candles: &[Candle], window: usize) -> Vec<f64> {
    let start = candles.len().saturating_sub(window);
    let slice = &candles[start..];

    slice
        .windows(2)
        .filter_map(|w| {
            let prev = w[0].close;
            let curr = w[1].close;
            if prev > 0.0 && curr > 0.0 {
                Some((curr / prev).ln())
            } else {
                None
            }
        })
        .collect()
}

/// Calculate Pearson correlation coefficient between two series.
fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len()) as f64;
    if n < 2.0 {
        return 0.0;
    }

    let x = &x[..n as usize];
    let y = &y[..n as usize];

    let mean_x: f64 = x.iter().sum::<f64>() / n;
    let mean_y: f64 = y.iter().sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n as usize {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < 1e-15 {
        0.0
    } else {
        cov / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let corr = pearson_correlation(&x, &y);
        assert!((corr - 1.0).abs() < 1e-10);
    }

    #[test]
    fn negative_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let corr = pearson_correlation(&x, &y);
        assert!((corr + 1.0).abs() < 1e-10);
    }

    #[test]
    fn effective_positions_independent() {
        let matrix = CorrelationMatrix::new();
        let pairs = vec!["BTC/USD".to_string(), "SOL/USD".to_string()];
        assert_eq!(matrix.effective_positions(&pairs), 2.0);
    }

    #[test]
    fn effective_positions_correlated() {
        let mut matrix = CorrelationMatrix::new();
        matrix
            .correlations
            .insert(("BTC/USD".to_string(), "ETH/USD".to_string()), 0.92);
        let pairs = vec!["BTC/USD".to_string(), "ETH/USD".to_string()];
        assert_eq!(matrix.effective_positions(&pairs), 4.0);
    }
}
