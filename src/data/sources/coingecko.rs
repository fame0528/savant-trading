//! CoinGecko candle source — broader coverage for tokens without Kraken data.
//!
//! Uses CoinGecko's free API to fetch OHLC data.
//! Covers thousands of tokens including meme coins.

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

pub struct CoinGeckoSource {
    client: reqwest::Client,
}

impl CoinGeckoSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Map pair symbol to CoinGecko coin ID.
    fn coin_id(&self, pair: &str) -> Option<&str> {
        let base = pair.split('/').next()?;
        match base {
            "BTC" => Some("bitcoin"),
            "ETH" => Some("ethereum"),
            "SOL" => Some("solana"),
            "XRP" => Some("ripple"),
            "DOGE" => Some("dogecoin"),
            "ADA" => Some("cardano"),
            "LINK" => Some("chainlink"),
            "AVAX" => Some("avalanche-2"),
            "PEPE" => Some("pepe"),
            "SHIB" => Some("shiba-inu"),
            "FLOKI" => Some("floki"),
            "TURBO" => Some("turbo"),
            "MOG" => Some("mog-coin"),
            "ARB" => Some("arbitrum"),
            "UNI" => Some("uniswap"),
            "AAVE" => Some("aave"),
            _ => None,
        }
    }
}

impl Default for CoinGeckoSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for CoinGeckoSource {
    fn name(&self) -> &str {
        "CoinGecko"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.coin_id(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let coin_id = self.coin_id(pair).ok_or_else(|| {
            ExecutionError::Other(format!("No CoinGecko ID for {}", pair))
        })?;

        // CoinGecko OHLC endpoint: /coins/{id}/ohlc
        // vs_currency=usd, days=1 (for 5m candles) or days=7 (for 1h candles)
        let days = match timeframe_minutes {
            1..=5 => 1,
            6..=60 => 7,
            61..=1440 => 30,
            _ => 1,
        };

        let url = format!(
            "https://api.coingecko.com/api/v3/coins/{}/ohlc?vs_currency=usd&days={}",
            coin_id, days
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("CoinGecko request failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(ExecutionError::Other(format!(
                "CoinGecko returned {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("CoinGecko parse error: {}", e)))?;

        let array = json
            .as_array()
            .ok_or_else(|| ExecutionError::Other("CoinGecko: expected array".into()))?;

        let mut candles = Vec::new();
        for item in array.iter().take(count as usize) {
            if let Some(arr) = item.as_array() {
                if arr.len() >= 5 {
                    let timestamp = arr[0].as_i64().unwrap_or(0) as u64;
                    let open = arr[1].as_f64().unwrap_or(0.0);
                    let high = arr[2].as_f64().unwrap_or(0.0);
                    let low = arr[3].as_f64().unwrap_or(0.0);
                    let close = arr[4].as_f64().unwrap_or(0.0);
                    candles.push(Candle {
                        pair: pair.to_string(),
                        open,
                        high,
                        low,
                        close,
                        volume: 0.0, // CoinGecko OHLC doesn't include volume
                        timestamp,
                    });
                }
            }
        }

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "CoinGecko: no candles for {}",
                pair
            )));
        }

        Ok(candles)
    }
}
