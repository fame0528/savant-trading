//! OKX candle source — broad token coverage, high rate limits.
//!
//! Uses OKX public API v5 for OHLCV data.
//! Free tier: 40 requests per 2 seconds per IP, no API key required.
//!
//! API: GET https://www.okx.com/api/v5/market/candles

use super::CandleSource;
use crate::core::error::ExecutionError;
use crate::core::types::Candle;
use async_trait::async_trait;

pub struct OkxSource {
    client: reqwest::Client,
}

impl OkxSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Map pair symbol to OKX instId format.
    /// "BTC/USD" -> "BTC-USDT", "ETH/USD" -> "ETH-USDT"
    fn okx_pair(&self, pair: &str) -> Option<String> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = parts[0].to_uppercase();
        let quote = parts[1].to_uppercase();

        let okx_quote = match quote.as_str() {
            "USD" | "USDT" | "USDC" => "USDT",
            "BTC" => "BTC",
            "ETH" => "ETH",
            _ => return None,
        };

        Some(format!("{}-{}", base, okx_quote))
    }

    /// Map timeframe to OKX bar parameter.
    fn okx_bar(&self, timeframe_minutes: u32) -> &'static str {
        match timeframe_minutes {
            1 => "1m",
            3 => "3m",
            5 => "5m",
            15 => "15m",
            30 => "30m",
            60 => "1H",
            120 => "2H",
            240 => "4H",
            360 => "6H",
            720 => "12H",
            1440 => "1D",
            _ => "5m",
        }
    }
}

impl Default for OkxSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for OkxSource {
    fn name(&self) -> &str {
        "OKX"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.okx_pair(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let inst_id = self
            .okx_pair(pair)
            .ok_or_else(|| ExecutionError::Other(format!("No OKX pair mapping for {}", pair)))?;

        let bar = self.okx_bar(timeframe_minutes);
        let limit = count.min(300);

        let url = format!(
            "https://www.okx.com/api/v5/market/candles?instId={}&bar={}&limit={}",
            inst_id, bar, limit
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("OKX request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "OKX returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("OKX parse error: {}", e)))?;

        let code = json["code"].as_str().unwrap_or("1");
        if code != "0" {
            let msg = json["msg"].as_str().unwrap_or("unknown error");
            return Err(ExecutionError::Other(format!(
                "OKX error {}: {}",
                code, msg
            )));
        }

        let data = json["data"]
            .as_array()
            .ok_or_else(|| ExecutionError::Other("OKX response missing data array".into()))?;

        // OKX returns arrays: [ts, open, high, low, close, vol, volCcy, volCcyQuote, confirm]
        // Timestamps are in milliseconds
        let mut candles = Vec::with_capacity(data.len());
        for entry in data {
            let arr = entry
                .as_array()
                .ok_or_else(|| ExecutionError::Other("OKX candle entry is not an array".into()))?;

            if arr.len() < 6 {
                continue;
            }

            let timestamp_ms = arr[0].as_str().unwrap_or("0").parse::<i64>().unwrap_or(0);
            let open = arr[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let high = arr[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let low = arr[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let close = arr[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let volume = arr[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

            let timestamp =
                chrono::DateTime::from_timestamp_millis(timestamp_ms).unwrap_or(chrono::Utc::now());

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

        // OKX returns newest first — reverse to chronological order
        candles.reverse();

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "OKX: no candles for {}",
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
    fn okx_pair_mapping() {
        let src = OkxSource::new();
        assert_eq!(src.okx_pair("BTC/USD"), Some("BTC-USDT".into()));
        assert_eq!(src.okx_pair("ETH/USDT"), Some("ETH-USDT".into()));
        assert_eq!(src.okx_pair("SOL/USD"), Some("SOL-USDT".into()));
        assert_eq!(src.okx_pair("BTC/BTC"), Some("BTC-BTC".into()));
        assert_eq!(src.okx_pair("FAKE/EUR"), None);
    }

    #[test]
    fn okx_bar_mapping() {
        let src = OkxSource::new();
        assert_eq!(src.okx_bar(5), "5m");
        assert_eq!(src.okx_bar(60), "1H");
        assert_eq!(src.okx_bar(1440), "1D");
    }

    #[test]
    fn might_have_accepts_valid() {
        let src = OkxSource::new();
        assert!(src.might_have("BTC/USD"));
        assert!(src.might_have("SOL/USD"));
        assert!(!src.might_have("FAKE/EUR"));
    }
}
