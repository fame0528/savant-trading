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
}
