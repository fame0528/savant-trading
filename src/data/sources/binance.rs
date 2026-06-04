//! Binance candle source — broadest coverage, no API key required.
//!
//! Uses Binance public klines API for OHLCV data.
//! Covers 1000+ trading pairs including all major Arbitrum tokens.
//!
//! API: GET https://api.binance.com/api/v3/klines
//! Rate limit: 1200 requests/min (no key)

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

pub struct BinanceSource {
    client: reqwest::Client,
}

impl BinanceSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Map pair symbol to Binance trading pair format.
    /// "BTC/USD" -> "BTCUSDT", "ETH/USD" -> "ETHUSDC", etc.
    fn binance_pair(&self, pair: &str) -> Option<String> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = parts[0].to_uppercase();
        let quote = parts[1].to_uppercase();
        
        // Map common quote currencies to Binance format
        let quote_binance = match quote.as_str() {
            "USD" | "USDT" => "USDT",
            "USDC" => "USDC",
            "BTC" => "BTC",
            "ETH" => "ETH",
            _ => return None,
        };
        
        Some(format!("{}{}", base, quote_binance))
    }
}

impl Default for BinanceSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for BinanceSource {
    fn name(&self) -> &str {
        "Binance"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.binance_pair(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let binance_pair = self.binance_pair(pair).ok_or_else(|| {
            ExecutionError::Other(format!("No Binance pair mapping for {}", pair))
        })?;

        // Map timeframe to Binance interval
        let interval = match timeframe_minutes {
            1 => "1m",
            3 => "3m",
            5 => "5m",
            15 => "15m",
            30 => "30m",
            60 => "1h",
            120 => "2h",
            240 => "4h",
            360 => "6h",
            720 => "12h",
            1440 => "1d",
            _ => "5m",
        };

        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}",
            binance_pair, interval, count
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("Binance request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "Binance returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("Binance parse error: {}", e)))?;

        let klines = json.as_array().ok_or_else(|| {
            ExecutionError::Other("Binance response is not an array".into())
        })?;

        let mut candles = Vec::with_capacity(klines.len());
        for kline in klines {
            let arr = kline.as_array().ok_or_else(|| {
                ExecutionError::Other("Binance kline is not an array".into())
            })?;

            if arr.len() < 6 {
                continue;
            }

            let open = arr[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let high = arr[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let low = arr[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let close = arr[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let volume = arr[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let timestamp_ms = arr[0].as_u64().unwrap_or(0);

            let timestamp = chrono::DateTime::from_timestamp_millis(timestamp_ms as i64)
                .unwrap_or(chrono::Utc::now());

            candles.push(Candle {
                pair: pair.to_string(),
                open,
                high,
                low,
                close,
                volume,
                timestamp,
            });
        }

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "Binance: no candles for {}",
                pair
            )));
        }

        Ok(candles)
    }
}
