//! CryptoCompare candle source — broadest US-accessible coverage.
//!
//! Uses CryptoCompare's free histominute/histohour API for OHLCV data.
//! Free tier: 100K calls/month, no geo-blocking, all major tokens.
//!
//! API: GET https://min-api.cryptocompare.com/data/v2/histominute

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

pub struct CryptoCompareSource {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CryptoCompareSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            api_key: std::env::var("CRYPTOCOMPARE_API_KEY").ok(),
        }
    }

    /// Map pair symbol to CryptoCompare fsym/tsym.
    fn cc_pair(&self, pair: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = parts[0].to_uppercase();
        let quote = parts[1].to_uppercase();

        // Map common quote currencies
        let tsym = match quote.as_str() {
            "USD" | "USDT" | "USDC" => "USD",
            "BTC" => "BTC",
            "ETH" => "ETH",
            _ => return None,
        };

        Some((base, tsym.to_string()))
    }
}

impl Default for CryptoCompareSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for CryptoCompareSource {
    fn name(&self) -> &str {
        "CryptoCompare"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.cc_pair(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let (fsym, tsym) = self.cc_pair(pair).ok_or_else(|| {
            ExecutionError::Other(format!("No CryptoCompare pair mapping for {}", pair))
        })?;

        // Choose endpoint based on timeframe
        let (endpoint, aggregate) = if timeframe_minutes >= 60 {
            ("histohour", timeframe_minutes / 60)
        } else {
            ("histominute", timeframe_minutes)
        };

        let limit = count.min(2000);

        let url = format!(
            "https://min-api.cryptocompare.com/data/v2/{}?fsym={}&tsym={}&limit={}&aggregate={}",
            endpoint, fsym, tsym, limit, aggregate
        );

        let mut req = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            req = req.header("authorization", format!("Apikey {}", key));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("CryptoCompare request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "CryptoCompare returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("CryptoCompare parse error: {}", e)))?;

        let data = json
            .get("Data")
            .and_then(|d| d.get("Data"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| {
                ExecutionError::Other("CryptoCompare response missing Data.Data array".into())
            })?;

        let mut candles = Vec::with_capacity(data.len());
        for entry in data {
            let open = entry["open"].as_f64().unwrap_or(0.0);
            let high = entry["high"].as_f64().unwrap_or(0.0);
            let low = entry["low"].as_f64().unwrap_or(0.0);
            let close = entry["close"].as_f64().unwrap_or(0.0);
            let volume = entry["volumefrom"].as_f64().unwrap_or(0.0);
            let timestamp_secs = entry["time"].as_i64().unwrap_or(0);

            let timestamp = chrono::DateTime::from_timestamp(timestamp_secs, 0)
                .unwrap_or(chrono::Utc::now());

            // Skip zero candles
            if close == 0.0 && volume == 0.0 {
                continue;
            }

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
                "CryptoCompare: no candles for {}",
                pair
            )));
        }

        Ok(candles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cc_pair_mapping() {
        let src = CryptoCompareSource::new();
        assert_eq!(src.cc_pair("BTC/USD"), Some(("BTC".into(), "USD".into())));
        assert_eq!(src.cc_pair("ETH/USDT"), Some(("ETH".into(), "USD".into())));
        assert_eq!(src.cc_pair("SOL/USD"), Some(("SOL".into(), "USD".into())));
        assert_eq!(src.cc_pair("BTC/BTC"), Some(("BTC".into(), "BTC".into())));
        assert_eq!(src.cc_pair("FAKE/EUR"), None);
    }

    #[test]
    fn might_have_accepts_valid() {
        let src = CryptoCompareSource::new();
        assert!(src.might_have("BTC/USD"));
        assert!(src.might_have("SOL/USD"));
        assert!(!src.might_have("FAKE/EUR"));
    }
}
