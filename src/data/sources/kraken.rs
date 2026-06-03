//! Kraken candle source — highest quality, 5m candles, ~20 pairs.
//!
//! Wraps the existing KrakenClient to implement the CandleSource trait.

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;
use crate::data::kraken::KrakenClient;

pub struct KrakenSource {
    client: KrakenClient,
}

impl KrakenSource {
    pub fn new(rest_url: &str) -> Self {
        Self {
            client: KrakenClient::new(rest_url),
        }
    }
}

#[async_trait]
impl CandleSource for KrakenSource {
    fn name(&self) -> &str {
        "Kraken"
    }

    fn might_have(&self, _pair: &str) -> bool {
        true // Kraken might have any pair — let the API decide
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let mut candles = self.client
            .get_ohlc(pair, timeframe_minutes, None)
            .await
            .map_err(|e| ExecutionError::Other(format!("Kraken OHLC error: {}", e)))?;

        // Remove the last (forming) candle
        if candles.len() > 1 {
            candles.pop();
        }

        // Limit to requested count
        if candles.len() > count as usize {
            let start = candles.len() - count as usize;
            candles = candles[start..].to_vec();
        }

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "Kraken: no candles for {}",
                pair
            )));
        }

        Ok(candles)
    }
}
