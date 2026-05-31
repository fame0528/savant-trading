use async_trait::async_trait;
use chrono::Utc;

use crate::core::types::{
    Candle, IndicatorValues, MarketRegime, Side, Signal, SignalMetadata, VolumeProfile,
};
use crate::strategy::base::Strategy;

pub struct MeanReversionStrategy {
    pub profile_periods: usize,
    pub value_area_pct: f64,
    pub volume_spike_multiplier: f64,
}

impl MeanReversionStrategy {
    pub fn new(profile_periods: usize, value_area_pct: f64, volume_spike_multiplier: f64) -> Self {
        Self {
            profile_periods,
            value_area_pct,
            volume_spike_multiplier,
        }
    }
}

#[async_trait]
impl Strategy for MeanReversionStrategy {
    fn name(&self) -> &str {
        "mean_reversion"
    }

    async fn evaluate(
        &self,
        candles: &[Candle],
        indicators: &IndicatorValues,
        regime: MarketRegime,
        volume_profile: Option<&VolumeProfile>,
    ) -> Option<Signal> {
        let profile = volume_profile?;
        let atr = indicators.atr?;
        let vol_sma = indicators.volume_sma?;
        let last = candles.last()?;
        let prev = candles.get(candles.len() - 2)?;

        let current_vol_ratio = if vol_sma > 0.0 {
            last.volume / vol_sma
        } else {
            return None;
        };

        if prev.low < profile.value_area_low && last.close > profile.value_area_low {
            let stop = prev.low - atr * 0.5;
            let risk = last.close - stop;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Long,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: profile.poc_price,
                take_profit_2: profile.value_area_high,
                take_profit_3: profile.value_area_high + risk,
                strategy_name: self.name().to_string(),
                confidence: 0.65,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Reversion long: VAL={:.2}, POC={:.2}, VAH={:.2}",
                        profile.value_area_low, profile.poc_price, profile.value_area_high
                    ),
                },
            });
        }

        if prev.high > profile.value_area_high && last.close < profile.value_area_high {
            let stop = prev.high + atr * 0.5;
            let risk = stop - last.close;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Short,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: profile.poc_price,
                take_profit_2: profile.value_area_low,
                take_profit_3: profile.value_area_low - risk,
                strategy_name: self.name().to_string(),
                confidence: 0.65,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Reversion short: VAH={:.2}, POC={:.2}, VAL={:.2}",
                        profile.value_area_high, profile.poc_price, profile.value_area_low
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
        let profile = volume_profile?;
        let atr = indicators.atr?;
        let vol_sma = indicators.volume_sma?;
        let last = candles.last()?;
        let prev = candles.get(candles.len() - 2)?;

        let current_vol_ratio = if vol_sma > 0.0 {
            last.volume / vol_sma
        } else {
            return None;
        };

        if prev.low < profile.value_area_low && last.close > profile.value_area_low {
            let stop = prev.low - atr * 0.5;
            let risk = last.close - stop;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Long,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: profile.poc_price,
                take_profit_2: profile.value_area_high,
                take_profit_3: profile.value_area_high + risk,
                strategy_name: self.name().to_string(),
                confidence: 0.65,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Reversion long: VAL={:.2}, POC={:.2}, VAH={:.2}",
                        profile.value_area_low, profile.poc_price, profile.value_area_high
                    ),
                },
            });
        }

        if prev.high > profile.value_area_high && last.close < profile.value_area_high {
            let stop = prev.high + atr * 0.5;
            let risk = stop - last.close;
            if risk <= 0.0 {
                return None;
            }
            return Some(Signal {
                pair: last.pair.clone(),
                side: Side::Short,
                entry_price: last.close,
                stop_loss: stop,
                take_profit_1: profile.poc_price,
                take_profit_2: profile.value_area_low,
                take_profit_3: profile.value_area_low - risk,
                strategy_name: self.name().to_string(),
                confidence: 0.65,
                timestamp: Utc::now(),
                metadata: SignalMetadata {
                    regime: Some(regime),
                    atr: Some(atr),
                    volume_ratio: Some(current_vol_ratio),
                    adx: indicators.adx,
                    notes: format!(
                        "Reversion short: VAH={:.2}, POC={:.2}, VAL={:.2}",
                        profile.value_area_high, profile.poc_price, profile.value_area_low
                    ),
                },
            });
        }

        None
    }
}
