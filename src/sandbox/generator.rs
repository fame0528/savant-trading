//! Synthetic OHLCV data generator using GARCH(1,1) volatility clustering.
//!
//! Generates realistic price data with configurable trend, volatility regime,
//! and structural breaks. Preserves statistical properties of crypto markets.

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::core::types::Candle;

/// Configuration for synthetic data generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Starting price
    pub start_price: f64,
    /// Base drift (annualized return, e.g., 0.5 = 50%)
    pub drift: f64,
    /// Base volatility (annualized, e.g., 0.8 = 80%)
    pub base_volatility: f64,
    /// GARCH omega (unconditional variance floor)
    pub garch_omega: f64,
    /// GARCH alpha (shock impact, typically 0.05-0.15)
    pub garch_alpha: f64,
    /// GARCH beta (volatility persistence, typically 0.80-0.95)
    pub garch_beta: f64,
    /// Number of candles to generate
    pub num_candles: usize,
    /// Candle interval in minutes
    pub interval_minutes: u32,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// Trend direction: 1.0 = up, -1.0 = down, 0.0 = random
    pub trend_bias: f64,
    /// Volume base level
    pub base_volume: f64,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            start_price: 100000.0,
            drift: 0.0,            // No drift by default
            base_volatility: 0.80, // 80% annualized vol (crypto-typical)
            garch_omega: 0.00001,
            garch_alpha: 0.10,
            garch_beta: 0.85,
            num_candles: 721,
            interval_minutes: 5,
            start_time: Utc::now() - Duration::days(2),
            trend_bias: 0.0,
            base_volume: 500.0,
        }
    }
}

/// Scenario-specific parameter overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioParams {
    pub trend: TrendDirection,
    pub volatility_regime: VolatilityRegime,
    pub event: Option<MarketEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrendDirection {
    Bull(f64), // strength 0.0-1.0
    Bear(f64),
    Sideways,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VolatilityRegime {
    Low,
    Normal,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    FlashCrash {
        candle_index: usize,
        magnitude_pct: f64,
    },
    ShortSqueeze {
        candle_index: usize,
        magnitude_pct: f64,
    },
    GapUp {
        candle_index: usize,
        gap_pct: f64,
    },
    GapDown {
        candle_index: usize,
        gap_pct: f64,
    },
}

/// Generate synthetic OHLCV candles using GARCH(1,1) model.
pub fn generate_candles(config: &GeneratorConfig) -> Vec<Candle> {
    let mut rng = rand::thread_rng();
    let mut candles = Vec::with_capacity(config.num_candles);

    let dt = config.interval_minutes as f64 / (365.25 * 24.0 * 60.0); // Fraction of year
    let drift_per_step = config.drift * dt;
    let _sqrt_dt = dt.sqrt();

    let mut price = config.start_price;
    let mut variance = config.base_volatility.powi(2) * dt;

    for i in 0..config.num_candles {
        let timestamp =
            config.start_time + Duration::minutes(i as i64 * config.interval_minutes as i64);

        // GARCH(1,1) volatility update
        let innovation: f64 = rng.gen_range(-3.0..3.0); // Normal-ish
        let shock = innovation * variance.sqrt();
        variance =
            config.garch_omega + config.garch_alpha * shock.powi(2) + config.garch_beta * variance;
        variance = variance.max(1e-10); // Floor

        let vol = variance.sqrt();

        // Generate OHLC
        let open = price;
        let direction = if config.trend_bias != 0.0 {
            config.trend_bias
        } else {
            rng.gen_range(-1.0..1.0)
        };

        let return_val = drift_per_step + direction * vol * 0.3 + innovation * vol;
        let close = (open * (1.0 + return_val)).max(1.0); // Floor at 1.0

        // Generate high/low with realistic wicks
        let wick_up = open * vol * rng.gen_range(0.0..0.5);
        let wick_down = open * vol * rng.gen_range(0.0..0.5);
        let high = open.max(close) + wick_up;
        let low = (open.min(close) - wick_down).max(0.01);

        // Volatility with clustering
        let vol_multiplier = 1.0 + (return_val.abs() * 10.0); // Higher volume on big moves
        let volume = config.base_volume * vol_multiplier * rng.gen_range(0.5..1.5);

        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close: close.max(low).min(high), // Ensure close is within range
            volume,
            pair: "BTC/USD".to_string(),
        });

        price = close.max(1.0); // Floor price at 1.0
    }

    candles
}

/// Apply scenario parameters to a candle sequence.
pub fn apply_scenario(candles: &mut [Candle], params: &ScenarioParams) {
    // Apply market events
    if let Some(event) = &params.event {
        match event {
            MarketEvent::FlashCrash {
                candle_index,
                magnitude_pct,
            } => {
                if *candle_index < candles.len() {
                    let c = &mut candles[*candle_index];
                    let crash_factor = 1.0 - magnitude_pct / 100.0;
                    c.close *= crash_factor;
                    c.low = c.close * 0.99;
                    c.high = c.open * 1.01;
                    // Recovery in next few candles
                    for candle in candles.iter_mut().skip(candle_index + 1).take(4) {
                        let recovery = 1.0 + (magnitude_pct / 100.0 / 4.0);
                        candle.close *= recovery;
                        candle.high = candle.close * 1.005;
                    }
                }
            }
            MarketEvent::ShortSqueeze {
                candle_index,
                magnitude_pct,
            } => {
                if *candle_index < candles.len() {
                    let c = &mut candles[*candle_index];
                    let squeeze_factor = 1.0 + magnitude_pct / 100.0;
                    c.close *= squeeze_factor;
                    c.high = c.close * 1.01;
                    c.low = c.open * 0.99;
                }
            }
            MarketEvent::GapUp {
                candle_index,
                gap_pct,
            } => {
                if *candle_index < candles.len() {
                    let gap_factor = 1.0 + gap_pct / 100.0;
                    for candle in candles.iter_mut().skip(*candle_index) {
                        candle.open *= gap_factor;
                        candle.high *= gap_factor;
                        candle.low *= gap_factor;
                        candle.close *= gap_factor;
                    }
                }
            }
            MarketEvent::GapDown {
                candle_index,
                gap_pct,
            } => {
                if *candle_index < candles.len() {
                    let gap_factor = 1.0 - gap_pct / 100.0;
                    for candle in candles.iter_mut().skip(*candle_index) {
                        candle.open *= gap_factor;
                        candle.high *= gap_factor;
                        candle.low *= gap_factor;
                        candle.close *= gap_factor;
                    }
                }
            }
        }
    }

    // Apply trend bias
    match &params.trend {
        TrendDirection::Bull(strength) => {
            let drift = 0.0001 * strength;
            for (i, candle) in candles.iter_mut().enumerate() {
                let factor = 1.0 + drift * i as f64;
                candle.open *= factor;
                candle.high *= factor;
                candle.low *= factor;
                candle.close *= factor;
            }
        }
        TrendDirection::Bear(strength) => {
            let drift = -0.0001 * strength;
            for (i, candle) in candles.iter_mut().enumerate() {
                let factor = 1.0 + drift * i as f64;
                candle.open *= factor;
                candle.high *= factor;
                candle.low *= factor;
                candle.close *= factor;
            }
        }
        TrendDirection::Sideways => {}
    }

    // Apply volatility regime
    match &params.volatility_regime {
        VolatilityRegime::Low => {
            for candle in candles.iter_mut() {
                let mid = (candle.high + candle.low) / 2.0;
                let range = (candle.high - candle.low) * 0.3;
                candle.high = mid + range / 2.0;
                candle.low = mid - range / 2.0;
            }
        }
        VolatilityRegime::High => {
            for candle in candles.iter_mut() {
                let mid = (candle.high + candle.low) / 2.0;
                let range = (candle.high - candle.low) * 2.0;
                candle.high = mid + range / 2.0;
                candle.low = mid - range / 2.0;
            }
        }
        VolatilityRegime::Extreme => {
            for candle in candles.iter_mut() {
                let mid = (candle.high + candle.low) / 2.0;
                let range = (candle.high - candle.low) * 4.0;
                candle.high = mid + range / 2.0;
                candle.low = mid - range / 2.0;
            }
        }
        VolatilityRegime::Normal => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_default_candles() {
        let config = GeneratorConfig::default();
        let candles = generate_candles(&config);
        assert_eq!(candles.len(), 721);
        // All candles should have valid OHLC relationships
        for c in &candles {
            assert!(c.high >= c.open);
            assert!(c.high >= c.close);
            assert!(c.low <= c.open);
            assert!(c.low <= c.close);
            assert!(c.high >= c.low);
            assert!(c.volume > 0.0);
        }
    }

    #[test]
    fn generate_bull_trend() {
        let config = GeneratorConfig {
            num_candles: 100,
            trend_bias: 0.5,
            ..Default::default()
        };
        let candles = generate_candles(&config);
        assert_eq!(candles.len(), 100);
        // Price should be positive
        assert!(candles[99].close > 0.0);
    }

    #[test]
    fn generate_with_flash_crash() {
        let config = GeneratorConfig {
            num_candles: 100,
            ..Default::default()
        };
        let candles_before = generate_candles(&config);
        let pre_crash_close = candles_before[50].close;
        let mut candles = candles_before;

        let params = ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Normal,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 50,
                magnitude_pct: 15.0,
            }),
        };

        apply_scenario(&mut candles, &params);

        // Flash crash candle should have lower close than before crash
        assert!(candles[50].close < pre_crash_close);
    }

    #[test]
    fn volatility_clustering() {
        let config = GeneratorConfig {
            num_candles: 500,
            garch_alpha: 0.12,
            garch_beta: 0.85,
            ..Default::default()
        };
        let candles = generate_candles(&config);

        // Calculate returns
        let returns: Vec<f64> = candles
            .windows(2)
            .map(|w| (w[1].close - w[0].close) / w[0].close)
            .collect();

        // Volatility should cluster — periods of high vol followed by high vol
        let mut high_vol_count = 0;
        let avg_vol = returns.iter().map(|r| r.abs()).sum::<f64>() / returns.len() as f64;
        for w in returns.windows(5) {
            let local_vol = w.iter().map(|r| r.abs()).sum::<f64>() / 5.0;
            if local_vol > avg_vol * 1.5 {
                high_vol_count += 1;
            }
        }

        // Should have some volatility clusters
        assert!(high_vol_count > 0);
    }
}
