use async_trait::async_trait;

use crate::core::types::{Candle, IndicatorValues, MarketRegime, Signal, VolumeProfile};

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;

    async fn evaluate(
        &self,
        candles: &[Candle],
        indicators: &IndicatorValues,
        regime: MarketRegime,
        volume_profile: Option<&VolumeProfile>,
    ) -> Option<Signal>;

    /// Synchronous evaluation for backtesting (avoids async overhead per candle).
    /// Default implementation returns None. Override in concrete strategies.
    fn evaluate_sync(
        &self,
        candles: &[Candle],
        indicators: &IndicatorValues,
        regime: MarketRegime,
        volume_profile: Option<&VolumeProfile>,
    ) -> Option<Signal> {
        let _ = (candles, indicators, regime, volume_profile);
        None
    }
}
