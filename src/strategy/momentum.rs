use async_trait::async_trait;
use chrono::Utc;

use crate::core::types::{
    Candle, IndicatorValues, MarketRegime, Side, Signal, SignalMetadata, VolumeProfile,
};
use crate::strategy::base::Strategy;

pub struct MomentumStrategy {
    pub ema_period: usize,
    pub volume_spike_multiplier: f64,
    pub atr_compression_threshold: f64,
}

impl MomentumStrategy {
    pub fn new(
        ema_period: usize,
        volume_spike_multiplier: f64,
        atr_compression_threshold: f64,
    ) -> Self {
        Self {
            ema_period,
            volume_spike_multiplier,
            atr_compression_threshold,
        }
    }

    fn find_range(&self, candles: &[Candle], atr: f64) -> Option<(f64, f64, usize)> {
        if candles.len() < 20 {
            return None;
        }

        let lookback = 20;
        let recent = &candles[candles.len() - lookback..];

        let avg_range: f64 = recent.iter().map(|c| c.range()).sum::<f64>() / lookback as f64;

        if avg_range > atr * (1.0 / self.atr_compression_threshold) {
            return None;
        }

        let range_high = recent
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let range_low = recent.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);

        Some((range_high, range_low, lookback))
    }
}

#[async_trait]
impl Strategy for MomentumStrategy {
    fn name(&self) -> &str {
        "momentum_breakout"
    }

    async fn evaluate(
        &self,
        candles: &[Candle],
        indicators: &IndicatorValues,
        regime: MarketRegime,
        _volume_profile: Option<&VolumeProfile>,
    ) -> Option<Signal> {
        if candles.len() < self.ema_period + 20 {
            return None;
        }

        let ema = indicators.ema_slow?;
        let atr = indicators.atr?;
        let vol_sma = indicators.volume_sma?;
        let last = candles.last()?;
        let prev = candles.get(candles.len() - 2)?;

        let current_vol_ratio = if vol_sma > 0.0 {
            last.volume / vol_sma
        } else {
            return None;
        };

        let (range_high, range_low, _range_len) = self.find_range(candles, atr)?;

        let above_ema = last.close > ema;
        let below_ema = last.close < ema;

        let vol_breakout = current_vol_ratio >= self.volume_spike_multiplier;

        if above_ema && last.close > range_high && prev.close <= range_high && vol_breakout {
            let stop = range_low + (range_high - range_low) * 0.5;
            let risk = last.close - stop;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Long,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: last.close + risk * 1.0,
                take_profit_2: last.close + risk * 2.0,
                take_profit_3: last.close + risk * 3.0,
                strategy_name: self.name().to_string(),
                confidence: 0.7,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Breakout above range {:.2}-{:.2}, vol ratio: {:.2}",
                        range_low, range_high, current_vol_ratio
                    ),
                },
            });
        }

        if below_ema && last.close < range_low && prev.close >= range_low && vol_breakout {
            let stop = range_high - (range_high - range_low) * 0.5;
            let risk = stop - last.close;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Short,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: last.close - risk * 1.0,
                take_profit_2: last.close - risk * 2.0,
                take_profit_3: last.close - risk * 3.0,
                strategy_name: self.name().to_string(),
                confidence: 0.7,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Breakdown below range {:.2}-{:.2}, vol ratio: {:.2}",
                        range_low, range_high, current_vol_ratio
                    ),
                },
            });
        }

        None
    }

    fn evaluate_sync(
        &self,
        candles: &[Candle],
        indicators: &IndicatorValues,
        regime: MarketRegime,
        volume_profile: Option<&VolumeProfile>,
    ) -> Option<Signal> {
        // Identical logic to evaluate() but without async overhead
        let _ = volume_profile;
        if candles.len() < self.ema_period + 20 {
            return None;
        }

        let ema = indicators.ema_slow?;
        let atr = indicators.atr?;
        let vol_sma = indicators.volume_sma?;
        let last = candles.last()?;
        let prev = candles.get(candles.len() - 2)?;

        let current_vol_ratio = if vol_sma > 0.0 {
            last.volume / vol_sma
        } else {
            return None;
        };

        let (range_high, range_low, _range_len) = self.find_range(candles, atr)?;

        let above_ema = last.close > ema;
        let below_ema = last.close < ema;
        let vol_breakout = current_vol_ratio >= self.volume_spike_multiplier;

        if above_ema && last.close > range_high && prev.close <= range_high && vol_breakout {
            let stop = range_low + (range_high - range_low) * 0.5;
            let risk = last.close - stop;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Long,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: last.close + risk,
                take_profit_2: last.close + risk * 2.0,
                take_profit_3: last.close + risk * 3.0,
                strategy_name: self.name().to_string(),
                confidence: 0.7,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Breakout above range {:.2}-{:.2}, vol ratio: {:.2}",
                        range_low, range_high, current_vol_ratio
                    ),
                },
            });
        }

        if below_ema && last.close < range_low && prev.close >= range_low && vol_breakout {
            let stop = range_high - (range_high - range_low) * 0.5;
            let risk = stop - last.close;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Short,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: last.close - risk,
                take_profit_2: last.close - risk * 2.0,
                take_profit_3: last.close - risk * 3.0,
                strategy_name: self.name().to_string(),
                confidence: 0.7,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Breakdown below range {:.2}-{:.2}, vol ratio: {:.2}",
                        range_low, range_high, current_vol_ratio
                    ),
                },
            });
        }

        None
    }
}
