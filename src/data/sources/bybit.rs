//! Bybit candle source — excellent unified API, very low friction.
//!
//! Uses Bybit public API v5 for OHLCV data.
//! Free tier: 120 requests per second, no API key required.
//!
//! API: GET https://api.bybit.com/v5/market/kline

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

pub struct BybitSource {
    client: reqwest::Client,
}

impl BybitSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Map pair to Bybit symbol format.
    /// "BTC/USD" -> "BTCUSDT"
    fn bybit_pair(&self, pair: &str) -> Option<String> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = parts[0].to_uppercase();
        let quote = parts[1].to_uppercase();

        let bybit_quote = match quote.as_str() {
            "USD" | "USDT" | "USDC" => "USDT",
            "BTC" => "BTC",
            "ETH" => "ETH",
            _ => return None,
        };

        Some(format!("{}{}", base, bybit_quote))
    }

    /// Map timeframe to Bybit interval parameter.
    fn bybit_interval(&self, timeframe_minutes: u32) -> &'static str {
        match timeframe_minutes {
            1 => "1",
            3 => "3",
            5 => "5",
            15 => "15",
            30 => "30",
            60 => "60",
            120 => "120",
            240 => "240",
            360 => "360",
            720 => "720",
            1440 => "D",
            _ => "5",
        }
    }
}

impl Default for BybitSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for BybitSource {
    fn name(&self) -> &str {
        "Bybit"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.bybit_pair(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let symbol = self.bybit_pair(pair).ok_or_else(|| {
            ExecutionError::Other(format!("No Bybit pair mapping for {}", pair))
        })?;

        let interval = self.bybit_interval(timeframe_minutes);
        let limit = count.min(1000);

        let url = format!(
            "https://api.bybit.com/v5/market/kline?category=spot&symbol={}&interval={}&limit={}",
            symbol, interval, limit
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("Bybit request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "Bybit returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("Bybit parse error: {}", e)))?;

        let ret_code = json["retCode"].as_i64().unwrap_or(1);
        if ret_code != 0 {
            let ret_msg = json["retMsg"].as_str().unwrap_or("unknown error");
            return Err(ExecutionError::Other(format!(
                "Bybit error {}: {}",
                ret_code, ret_msg
            )));
        }

        let data = json["result"]["list"].as_array().ok_or_else(|| {
            ExecutionError::Other("Bybit response missing result.list array".into())
        })?;

        // Bybit returns arrays: [startTime, openPrice, highPrice, lowPrice, closePrice, volume, turnover]
        // Timestamps are in milliseconds, newest first
        let mut candles = Vec::with_capacity(data.len());
        for entry in data {
            let arr = entry.as_array().ok_or_else(|| {
                ExecutionError::Other("Bybit candle entry is not an array".into())
            })?;

            if arr.len() < 6 {
                continue;
            }

            let timestamp_ms = arr[0].as_str().unwrap_or("0").parse::<i64>().unwrap_or(0);
            let open = arr[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let high = arr[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let low = arr[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let close = arr[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let volume = arr[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

            let timestamp = chrono::DateTime::from_timestamp_millis(timestamp_ms)
                .unwrap_or(chrono::Utc::now());

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

        // Bybit returns newest first — reverse to chronological order
        candles.reverse();

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "Bybit: no candles for {}",
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
    fn bybit_pair_mapping() {
        let src = BybitSource::new();
        assert_eq!(src.bybit_pair("BTC/USD"), Some("BTCUSDT".into()));
        assert_eq!(src.bybit_pair("ETH/USDT"), Some("ETHUSDT".into()));
        assert_eq!(src.bybit_pair("SOL/USD"), Some("SOLUSDT".into()));
        assert_eq!(src.bybit_pair("FAKE/EUR"), None);
    }

    #[test]
    fn bybit_interval_mapping() {
        let src = BybitSource::new();
        assert_eq!(src.bybit_interval(5), "5");
        assert_eq!(src.bybit_interval(60), "60");
        assert_eq!(src.bybit_interval(1440), "D");
    }
}
