//! Gate.io candle source — largest selection of obscure/newly listed tokens.
//!
//! Uses Gate.io public API v4 for OHLCV data.
//! Free tier: 300 requests per second per IP, no API key required.
//!
//! API: GET https://api.gateio.ws/api/v4/spot/candlesticks

use super::CandleSource;
use crate::core::error::ExecutionError;
use crate::core::types::Candle;
use async_trait::async_trait;

pub struct GateSource {
    client: reqwest::Client,
}

impl GateSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Map pair to Gate.io currency_pair format.
    /// "BTC/USD" -> "BTC_USDT"
    fn gate_pair(&self, pair: &str) -> Option<String> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = crate::core::types::Candle::exchange_base(parts[0].to_uppercase().as_str()).to_string();
        let quote = parts[1].to_uppercase();

        let gate_quote = match quote.as_str() {
            "USD" | "USDT" | "USDC" => "USDT",
            "BTC" => "BTC",
            "ETH" => "ETH",
            _ => return None,
        };

        Some(format!("{}_{}", base, gate_quote))
    }

    /// Map timeframe to Gate.io interval parameter.
    fn gate_interval(&self, timeframe_minutes: u32) -> &'static str {
        match timeframe_minutes {
            1 => "1m",
            5 => "5m",
            15 => "15m",
            30 => "30m",
            60 => "1h",
            240 => "4h",
            1440 => "1d",
            _ => "5m",
        }
    }
}

impl Default for GateSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for GateSource {
    fn name(&self) -> &str {
        "Gate.io"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.gate_pair(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let currency_pair = self.gate_pair(pair).ok_or_else(|| {
            ExecutionError::Other(format!("No Gate.io pair mapping for {}", pair))
        })?;

        let interval = self.gate_interval(timeframe_minutes);
        let limit = count.min(1000);

        // Gate.io uses unix timestamps for range. We fetch the most recent candles.
        let url = format!(
            "https://api.gateio.ws/api/v4/spot/candlesticks?currency_pair={}&interval={}&limit={}",
            currency_pair, interval, limit
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("Gate.io request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "Gate.io returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("Gate.io parse error: {}", e)))?;

        let data = json.as_array().ok_or_else(|| {
            ExecutionError::Other(format!("Gate.io response is not an array for {}", pair))
        })?;

        // Gate.io returns arrays: [unix_ts, volume_quote, close, high, low, open, volume_base, is_window_closed]
        // Timestamps are in seconds
        let mut candles = Vec::with_capacity(data.len());
        for entry in data {
            let arr = entry.as_array().ok_or_else(|| {
                ExecutionError::Other("Gate.io candle entry is not an array".into())
            })?;

            if arr.len() < 7 {
                continue;
            }

            let timestamp_secs = arr[0].as_str().unwrap_or("0").parse::<i64>().unwrap_or(0);
            let close = arr[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let high = arr[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let low = arr[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let open = arr[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let volume = arr[6].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

            let timestamp =
                chrono::DateTime::from_timestamp(timestamp_secs, 0).unwrap_or(chrono::Utc::now());

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
                "Gate.io: no candles for {}",
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
    fn gate_pair_mapping() {
        let src = GateSource::new();
        assert_eq!(src.gate_pair("BTC/USD"), Some("BTC_USDT".into()));
        assert_eq!(src.gate_pair("ETH/USDT"), Some("ETH_USDT".into()));
        assert_eq!(src.gate_pair("SOL/USD"), Some("SOL_USDT".into()));
        assert_eq!(src.gate_pair("FAKE/EUR"), None);
    }
}
