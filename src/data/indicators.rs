use crate::core::types::{Candle, IndicatorValues, VolumeLevel, VolumeProfile};

pub struct IndicatorEngine;

impl IndicatorEngine {
    pub fn ema(data: &[f64], period: usize) -> Vec<f64> {
        if data.is_empty() || period == 0 {
            return vec![];
        }
        let k = 2.0 / (period as f64 + 1.0);
        let mut result = Vec::with_capacity(data.len());
        result.push(data[0]);
        for i in 1..data.len() {
            let prev = result[i - 1];
            result.push(data[i] * k + prev * (1.0 - k));
        }
        result
    }

    pub fn sma(data: &[f64], period: usize) -> Vec<f64> {
        if data.len() < period {
            return vec![];
        }
        let mut result = Vec::with_capacity(data.len() - period + 1);
        let mut sum: f64 = data[..period].iter().sum();
        result.push(sum / period as f64);
        for i in period..data.len() {
            sum += data[i] - data[i - period];
            result.push(sum / period as f64);
        }
        result
    }

    pub fn rsi(data: &[f64], period: usize) -> Vec<f64> {
        if data.len() < period + 1 {
            return vec![];
        }
        let mut gains = Vec::new();
        let mut losses = Vec::new();
        for i in 1..data.len() {
            let change = data[i] - data[i - 1];
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(-change);
            }
        }
        let avg_gain: f64 = gains[..period].iter().sum::<f64>() / period as f64;
        let avg_loss: f64 = losses[..period].iter().sum::<f64>() / period as f64;
        let mut result = Vec::new();
        let mut ag = avg_gain;
        let mut al = avg_loss;
        if al == 0.0 {
            result.push(100.0);
        } else {
            result.push(100.0 - 100.0 / (1.0 + ag / al));
        }
        for i in period..gains.len() {
            ag = (ag * (period as f64 - 1.0) + gains[i]) / period as f64;
            al = (al * (period as f64 - 1.0) + losses[i]) / period as f64;
            if al == 0.0 {
                result.push(100.0);
            } else {
                result.push(100.0 - 100.0 / (1.0 + ag / al));
            }
        }
        result
    }

    pub fn atr(candles: &[Candle], period: usize) -> Vec<f64> {
        if candles.len() < 2 {
            return vec![];
        }
        let mut true_ranges = Vec::with_capacity(candles.len() - 1);
        for i in 1..candles.len() {
            let high_low = candles[i].high - candles[i].low;
            let high_prev_close = (candles[i].high - candles[i - 1].close).abs();
            let low_prev_close = (candles[i].low - candles[i - 1].close).abs();
            true_ranges.push(high_low.max(high_prev_close).max(low_prev_close));
        }
        Self::sma(&true_ranges, period)
    }

    pub fn adx(candles: &[Candle], period: usize) -> Vec<f64> {
        if candles.len() < period * 2 {
            return vec![];
        }
        let mut plus_dm = Vec::new();
        let mut minus_dm = Vec::new();
        let mut true_ranges = Vec::new();

        for i in 1..candles.len() {
            let high_diff = candles[i].high - candles[i - 1].high;
            let low_diff = candles[i - 1].low - candles[i].low;

            plus_dm.push(if high_diff > low_diff && high_diff > 0.0 {
                high_diff
            } else {
                0.0
            });
            minus_dm.push(if low_diff > high_diff && low_diff > 0.0 {
                low_diff
            } else {
                0.0
            });

            let high_low = candles[i].high - candles[i].low;
            let high_prev = (candles[i].high - candles[i - 1].close).abs();
            let low_prev = (candles[i].low - candles[i - 1].close).abs();
            true_ranges.push(high_low.max(high_prev).max(low_prev));
        }

        let tr_smooth = Self::wilders_smooth(&true_ranges, period);
        let plus_dm_smooth = Self::wilders_smooth(&plus_dm, period);
        let minus_dm_smooth = Self::wilders_smooth(&minus_dm, period);

        let mut dx_values = Vec::new();
        for i in 0..tr_smooth.len() {
            if tr_smooth[i] == 0.0 {
                dx_values.push(0.0);
                continue;
            }
            let plus_di = 100.0 * plus_dm_smooth[i] / tr_smooth[i];
            let minus_di = 100.0 * minus_dm_smooth[i] / tr_smooth[i];
            let di_sum = plus_di + minus_di;
            if di_sum == 0.0 {
                dx_values.push(0.0);
            } else {
                dx_values.push(100.0 * (plus_di - minus_di).abs() / di_sum);
            }
        }

        Self::wilders_smooth(&dx_values, period)
    }

    fn wilders_smooth(data: &[f64], period: usize) -> Vec<f64> {
        if data.len() < period {
            return vec![];
        }
        let mut result = Vec::with_capacity(data.len() - period + 1);
        let init: f64 = data[..period].iter().sum();
        result.push(init / period as f64);
        for &val in data.iter().skip(period) {
            if let Some(&prev) = result.last() {
                result.push((prev * (period as f64 - 1.0) + val) / period as f64);
            }
        }
        result
    }

    pub fn vwap(candles: &[Candle]) -> Vec<f64> {
        let mut result = Vec::with_capacity(candles.len());
        let mut cumulative_volume = 0.0;
        let mut cumulative_tp_volume = 0.0;
        for candle in candles {
            let tp = (candle.high + candle.low + candle.close) / 3.0;
            cumulative_volume += candle.volume;
            cumulative_tp_volume += tp * candle.volume;
            if cumulative_volume > 0.0 {
                result.push(cumulative_tp_volume / cumulative_volume);
            } else {
                result.push(tp);
            }
        }
        result
    }

    pub fn volume_profile(candles: &[Candle], bins: usize) -> VolumeProfile {
        Self::volume_profile_with_pct(candles, bins, 0.70)
    }

    pub fn volume_profile_with_pct(
        candles: &[Candle],
        bins: usize,
        value_area_pct: f64,
    ) -> VolumeProfile {
        if candles.is_empty() {
            return VolumeProfile {
                poc_price: 0.0,
                poc_volume: 0.0,
                value_area_high: 0.0,
                value_area_low: 0.0,
                levels: vec![],
            };
        }

        let min_price = candles.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        let max_price = candles
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);

        if max_price <= min_price {
            return VolumeProfile {
                poc_price: min_price,
                poc_volume: 0.0,
                value_area_high: min_price,
                value_area_low: min_price,
                levels: vec![],
            };
        }

        let bin_size = (max_price - min_price) / bins as f64;
        let mut volume_at_price = vec![0.0_f64; bins];

        for candle in candles {
            let start_bin = ((candle.low - min_price) / bin_size).floor() as usize;
            let end_bin = ((candle.high - min_price) / bin_size).floor() as usize;
            let range_bins = (end_bin - start_bin + 1).max(1);
            let vol_per_bin = candle.volume / range_bins as f64;
            for item in volume_at_price
                .iter_mut()
                .take(end_bin.min(bins - 1) + 1)
                .skip(start_bin)
            {
                *item += vol_per_bin;
            }
        }

        let poc_bin = volume_at_price
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        let poc_price = min_price + (poc_bin as f64 + 0.5) * bin_size;
        let poc_volume = volume_at_price[poc_bin];

        let total_volume: f64 = volume_at_price.iter().sum();
        let target_volume = total_volume * value_area_pct;

        let mut sorted_bins: Vec<(usize, f64)> = volume_at_price
            .iter()
            .enumerate()
            .map(|(i, &v)| (i, v))
            .collect();
        sorted_bins.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut va_volume = 0.0;
        let mut va_bins = Vec::new();
        for &(bin, vol) in &sorted_bins {
            va_bins.push(bin);
            va_volume += vol;
            if va_volume >= target_volume {
                break;
            }
        }

        let min_va_bin = *va_bins.iter().min().unwrap_or(&0);
        let max_va_bin = *va_bins.iter().max().unwrap_or(&0);
        let value_area_low = min_price + min_va_bin as f64 * bin_size;
        let value_area_high = min_price + (max_va_bin as f64 + 1.0) * bin_size;

        let levels: Vec<VolumeLevel> = volume_at_price
            .iter()
            .enumerate()
            .map(|(i, &vol)| VolumeLevel {
                price: min_price + (i as f64 + 0.5) * bin_size,
                volume: vol,
            })
            .collect();

        VolumeProfile {
            poc_price,
            poc_volume,
            value_area_high,
            value_area_low,
            levels,
        }
    }

    pub fn calculate_all(candles: &[Candle], adx_period: usize) -> IndicatorValues {
        if candles.is_empty() {
            return IndicatorValues {
                ema_fast: None,
                ema_slow: None,
                rsi: None,
                atr: None,
                adx: None,
                vwap: None,
                volume_sma: None,
                garman_klass: None,
            };
        }

        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();

        let ema20 = Self::ema(&closes, 20);
        let ema100 = Self::ema(&closes, 100);
        let rsi_vals = Self::rsi(&closes, 14);
        let atr_vals = Self::atr(candles, 14);
        let adx_vals = Self::adx(candles, adx_period);
        let vwap_vals = Self::vwap(candles);
        let vol_sma = Self::sma(&volumes, 20);
        let gk_vals = Self::garman_klass(candles, 14);

        IndicatorValues {
            ema_fast: ema20.last().copied(),
            ema_slow: ema100.last().copied(),
            rsi: rsi_vals.last().copied(),
            atr: atr_vals.last().copied(),
            adx: adx_vals.last().copied(),
            vwap: vwap_vals.last().copied(),
            volume_sma: vol_sma.last().copied(),
            garman_klass: gk_vals.last().copied(),
        }
    }

    /// Garman-Klass volatility estimator — uses OHLC data for more accurate
    /// volatility measurement than ATR (which only uses close/high/low).
    ///
    /// Formula: GK = 0.5 * ln(H/L)^2 - (2*ln(2)-1) * ln(C/O)^2
    /// Averaged over `period` candles and annualized.
    pub fn garman_klass(candles: &[Candle], period: usize) -> Vec<f64> {
        if candles.len() < period {
            return vec![];
        }

        let mut result = Vec::with_capacity(candles.len() - period + 1);

        for window in candles.windows(period) {
            let sum: f64 = window
                .iter()
                .map(|c| {
                    let hl_ratio = (c.high / c.low).max(0.0001).ln();
                    let co_ratio = (c.close / c.open).max(0.0001).ln();
                    0.5 * hl_ratio.powi(2) - (2.0 * 2.0_f64.ln() - 1.0) * co_ratio.powi(2)
                })
                .sum();
            // Annualize: multiply by sqrt(periods_per_day * 365)
            // For 5m candles: 288 periods/day, sqrt(288*365) ≈ 324
            // NOTE: This assumes 5m candles. For other intervals, calculate:
            //   periods_per_day = 1440 / interval_minutes
            //   annualize_factor = sqrt(periods_per_day * 365)
            let avg = sum / period as f64;
            let annualized = avg.sqrt() * 324.0;
            result.push(annualized);
        }

        result
    }

    /// ZigZag pivot extraction (FID-085 Phase 2).
    ///
    /// Identifies significant peaks and troughs in price data using an ATR-based
    /// threshold. Falls back to 1.5% when ATR lookback is insufficient (< 14 periods).
    ///
    /// Returns a list of pivots as (index, price, is_peak) tuples.
    #[allow(clippy::needless_range_loop)]
    pub fn zigzag_pivots(candles: &[Candle], atr_period: usize) -> Vec<(usize, f64, bool)> {
        if candles.len() < 3 {
            return vec![];
        }

        // Compute ATR threshold. Fallback: 1.5% of price.
        let atr_values = Self::atr(candles, atr_period);
        let atr_threshold = if atr_values.len() >= atr_period {
            atr_values.last().copied().unwrap_or(0.0)
        } else {
            // Fallback: 1.5% of average price
            let avg_price: f64 = candles.iter().map(|c| c.close).sum::<f64>() / candles.len() as f64;
            avg_price * 0.015
        };

        if atr_threshold <= 0.0 {
            return vec![];
        }

        let mut pivots: Vec<(usize, f64, bool)> = Vec::new();
        let mut direction: i8 = 0; // 0=unknown, 1=up, -1=down
        let mut last_pivot_price = candles[0].close;
        let mut extreme_idx = 0usize;
        let mut extreme_price = candles[0].close;

        for i in 1..candles.len() {
            let high = candles[i].high;
            let low = candles[i].low;

            if direction == 0 {
                // Determine initial direction
                if high - last_pivot_price >= atr_threshold {
                    direction = 1;
                    extreme_price = high;
                    extreme_idx = i;
                } else if last_pivot_price - low >= atr_threshold {
                    direction = -1;
                    extreme_price = low;
                    extreme_idx = i;
                }
                continue;
            }

            if direction == 1 {
                // Looking for peak
                if high > extreme_price {
                    extreme_price = high;
                    extreme_idx = i;
                }
                if extreme_price - low >= atr_threshold {
                    // Confirmed peak
                    pivots.push((extreme_idx, extreme_price, true));
                    direction = -1;
                    last_pivot_price = extreme_price;
                    extreme_price = low;
                    extreme_idx = i;
                }
            } else {
                // Looking for trough
                if low < extreme_price {
                    extreme_price = low;
                    extreme_idx = i;
                }
                if high - extreme_price >= atr_threshold {
                    // Confirmed trough
                    pivots.push((extreme_idx, extreme_price, false));
                    direction = 1;
                    last_pivot_price = extreme_price;
                    extreme_price = high;
                    extreme_idx = i;
                }
            }
        }

        // Add the last extreme as a pivot if we have enough data
        if pivots.len() >= 2 {
            let is_peak = direction == 1;
            pivots.push((extreme_idx, extreme_price, is_peak));
        }

        pivots
    }

    /// KBar feature extraction (FID-085 Phase 2).
    ///
    /// Pre-computes statistical features from candle data for compact representation.
    /// Returns: (z_score, annualized_vol, trend_score, volume_ratio)
    ///
    /// Requires at least 20 candles for meaningful features.
    pub fn kbar_features(candles: &[Candle]) -> Option<(f64, f64, f64, f64)> {
        if candles.len() < 20 {
            return None;
        }

        let n = candles.len();
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();

        // SMA20 of closes
        let sma20: f64 = closes[n - 20..].iter().sum::<f64>() / 20.0;
        let last_close = closes[n - 1];

        // z-score: distance from SMA20 in standard deviations
        let variance: f64 = closes[n - 20..]
            .iter()
            .map(|c| (c - sma20).powi(2))
            .sum::<f64>()
            / 20.0;
        let std_dev = variance.sqrt();
        let z_score = if std_dev > 0.0 {
            (last_close - sma20) / std_dev
        } else {
            0.0
        };

        // Annualized volatility from 20-period returns
        let returns: Vec<f64> = closes[n - 20..]
            .windows(2)
            .map(|w| (w[1] / w[0]).ln())
            .collect();
        let vol_variance: f64 = returns.iter().map(|r| r.powi(2)).sum::<f64>() / returns.len() as f64;
        let annualized_vol = vol_variance.sqrt() * (525_960.0_f64).sqrt(); // 5-min candles per year

        // Trend score: linear regression slope over 20 periods
        let x_mean = 9.5f64; // mean of 0..19
        let y_mean = closes[n - 20..].iter().sum::<f64>() / 20.0;
        let mut xy_sum = 0.0f64;
        let mut xx_sum = 0.0f64;
        for (i, &y) in closes[n - 20..].iter().enumerate() {
            let x = i as f64;
            xy_sum += (x - x_mean) * (y - y_mean);
            xx_sum += (x - x_mean).powi(2);
        }
        let trend_score = if xx_sum > 0.0 { xy_sum / xx_sum } else { 0.0 };

        // Volume ratio: current volume vs SMA20 volume
        let vol_sma20: f64 = volumes[n - 20..].iter().sum::<f64>() / 20.0;
        let volume_ratio = if vol_sma20 > 0.0 {
            volumes[n - 1] / vol_sma20
        } else {
            1.0
        };

        Some((z_score, annualized_vol, trend_score, volume_ratio))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_candles(prices: Vec<f64>) -> Vec<Candle> {
        prices
            .into_iter()
            .enumerate()
            .map(|(i, p)| Candle {
                timestamp: Utc::now() + chrono::Duration::minutes(i as i64),
                open: p,
                high: p * 1.01,
                low: p * 0.99,
                close: p,
                volume: 100.0 + i as f64,
                pair: "BTC/USD".to_string(),
            })
            .collect()
    }

    #[test]
    fn ema_basic() {
        let data = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let result = IndicatorEngine::ema(&data, 3);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], 10.0);
        assert!(result[4] > result[0]);
    }

    #[test]
    fn ema_empty_input() {
        let result = IndicatorEngine::ema(&[], 3);
        assert!(result.is_empty());
    }

    #[test]
    fn sma_basic() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = IndicatorEngine::sma(&data, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 20.0);
        assert_eq!(result[1], 30.0);
        assert_eq!(result[2], 40.0);
    }

    #[test]
    fn sma_insufficient_data() {
        let data = vec![10.0, 20.0];
        let result = IndicatorEngine::sma(&data, 3);
        assert!(result.is_empty());
    }

    #[test]
    fn rsi_overbought() {
        let data: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 2.0).collect();
        let result = IndicatorEngine::rsi(&data, 14);
        assert!(!result.is_empty());
        assert!(result[0] > 70.0);
    }

    #[test]
    fn rsi_oversold() {
        let data: Vec<f64> = (0..20).map(|i| 100.0 - i as f64 * 2.0).collect();
        let result = IndicatorEngine::rsi(&data, 14);
        assert!(!result.is_empty());
        assert!(result[0] < 30.0);
    }

    #[test]
    fn atr_basic() {
        let candles = make_candles(vec![100.0, 102.0, 101.0, 103.0, 105.0]);
        let result = IndicatorEngine::atr(&candles, 3);
        assert!(!result.is_empty());
        assert!(result[0] > 0.0);
    }

    #[test]
    fn adx_basic() {
        let candles = make_candles(vec![
            100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0, 109.0, 110.0, 108.0,
            106.0, 107.0, 109.0, 111.0, 113.0, 112.0, 114.0, 116.0,
        ]);
        let result = IndicatorEngine::adx(&candles, 5);
        assert!(!result.is_empty());
    }

    #[test]
    fn vwap_basic() {
        let candles = make_candles(vec![100.0, 102.0, 101.0, 103.0, 105.0]);
        let result = IndicatorEngine::vwap(&candles);
        assert_eq!(result.len(), 5);
        assert!(result[0] > 0.0);
    }

    #[test]
    fn volume_profile_basic() {
        let candles = make_candles(vec![100.0, 102.0, 101.0, 103.0, 105.0]);
        let profile = IndicatorEngine::volume_profile(&candles, 5);
        assert!(profile.poc_price > 0.0);
        assert!(profile.value_area_high >= profile.value_area_low);
    }

    #[test]
    fn calculate_all_returns_values() {
        let candles = make_candles(
            (0..120)
                .map(|i| 50000.0 + (i as f64 * 10.0).sin() * 1000.0)
                .collect(),
        );
        let indicators = IndicatorEngine::calculate_all(&candles, 14);
        assert!(indicators.ema_fast.is_some());
        assert!(indicators.ema_slow.is_some());
        assert!(indicators.rsi.is_some());
        assert!(indicators.atr.is_some());
    }

    #[test]
    fn calculate_all_empty_input() {
        let indicators = IndicatorEngine::calculate_all(&[], 14);
        assert!(indicators.ema_fast.is_none());
        assert!(indicators.rsi.is_none());
    }

    #[test]
    fn zigzag_detects_pivots() {
        // Price goes up then down then up — enough variation for pivots
        let prices: Vec<f64> = vec![
            100.0, 102.0, 104.0, 106.0, 108.0, 110.0,
            108.0, 106.0, 104.0, 102.0, 100.0, 98.0,
            100.0, 102.0, 104.0, 106.0, 108.0, 110.0,
            108.0, 106.0, 104.0, 102.0, 100.0, 98.0,
            100.0, 102.0, 104.0, 106.0, 108.0, 110.0,
        ];
        let candles = make_candles(prices);
        let pivots = IndicatorEngine::zigzag_pivots(&candles, 14);
        assert!(pivots.len() >= 2, "Expected >= 2 pivots, got {}", pivots.len());
    }

    #[test]
    fn zigzag_empty_input() {
        let pivots = IndicatorEngine::zigzag_pivots(&[], 14);
        assert!(pivots.is_empty());
    }

    #[test]
    fn kbar_features_computed() {
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.5)).collect();
        let candles = make_candles(prices);
        let features = IndicatorEngine::kbar_features(&candles);
        assert!(features.is_some());
        let (_z, vol, trend, vol_ratio) = features.unwrap();
        assert!(vol >= 0.0);
        assert!(vol_ratio >= 0.0);
        assert!(trend > 0.0, "Expected positive trend for uptrend, got {}", trend);
    }

    #[test]
    fn kbar_features_insufficient_data() {
        let prices: Vec<f64> = (0..10).map(|i| 100.0 + i as f64).collect();
        let candles = make_candles(prices);
        let features = IndicatorEngine::kbar_features(&candles);
        assert!(features.is_none());
    }
}
