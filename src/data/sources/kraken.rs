//! Kraken candle feed — highest quality, 5m candles, ~20 pairs.
//!
//! Wraps the CandleClient to implement the CandleSource trait.

use super::CandleSource;
use crate::core::error::ExecutionError;
use crate::core::types::Candle;
use crate::data::candle_client::CandleClient;
use async_trait::async_trait;

pub struct KrakenFeed {
    client: CandleClient,
}

impl KrakenFeed {
    pub fn new(rest_url: &str) -> Self {
        Self {
            client: CandleClient::new(rest_url),
        }
    }
}

#[async_trait]
impl CandleSource for KrakenFeed {
    fn name(&self) -> &str {
        "Market Data"
    }

    fn might_have(&self, _pair: &str) -> bool {
        true
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let mut candles = self
            .client
            .get_ohlc(pair, timeframe_minutes, None)
            .await
            .map_err(|e| ExecutionError::Other(format!("OHLC error: {}", e)))?;

        if candles.len() > 1 {
            candles.pop();
        }

        if candles.len() > count as usize {
            let start = candles.len() - count as usize;
            candles = candles[start..].to_vec();
        }

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!("No candles for {}", pair)));
        }

        Ok(candles)
    }
}
