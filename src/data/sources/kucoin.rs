//! KuCoin candle source — massive altcoin selection, no API key.
//!
//! Uses KuCoin public API for OHLCV data.
//! Free tier: ~300 requests per 10 seconds, no API key required.
//!
//! API: GET https://api.kucoin.com/api/v1/market/candles

use super::CandleSource;
use crate::core::error::ExecutionError;
use crate::core::types::Candle;
use async_trait::async_trait;

pub struct KuCoinSource {
    client: reqwest::Client,
}

impl KuCoinSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Map pair symbol to KuCoin symbol format.
    /// "BTC/USD" -> "BTC-USDT"
    fn kucoin_pair(&self, pair: &str) -> Option<String> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = crate::core::types::Candle::exchange_base(parts[0].to_uppercase().as_str()).to_string();
        let quote = parts[1].to_uppercase();

        let kc_quote = match quote.as_str() {
            "USD" | "USDT" | "USDC" => "USDT",
            "BTC" => "BTC",
            "ETH" => "ETH",
            _ => return None,
        };

        Some(format!("{}-{}", base, kc_quote))
    }

    /// Map timeframe to KuCoin klineType parameter.
    fn kucoin_type(&self, timeframe_minutes: u32) -> &'static str {
        match timeframe_minutes {
            1 => "1min",
            3 => "3min",
            5 => "5min",
            15 => "15min",
            30 => "30min",
            60 => "1hour",
            120 => "2hour",
            240 => "4hour",
            360 => "6hour",
            720 => "12hour",
            1440 => "1day",
            _ => "5min",
        }
    }
}

impl Default for KuCoinSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for KuCoinSource {
    fn name(&self) -> &str {
        "KuCoin"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.kucoin_pair(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let symbol = self
            .kucoin_pair(pair)
            .ok_or_else(|| ExecutionError::Other(format!("No KuCoin pair mapping for {}", pair)))?;

        let ktype = self.kucoin_type(timeframe_minutes);

        let url = format!(
            "https://api.kucoin.com/api/v1/market/candles?type={}&symbol={}",
            ktype, symbol
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("KuCoin request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "KuCoin returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("KuCoin parse error: {}", e)))?;

        let code = json["code"].as_str().unwrap_or("1");
        if code != "200000" {
            let msg = json["msg"].as_str().unwrap_or("unknown error");
            return Err(ExecutionError::Other(format!(
                "KuCoin error {}: {}",
                code, msg
            )));
        }

        let data = json["data"]
            .as_array()
            .ok_or_else(|| ExecutionError::Other("KuCoin response missing data array".into()))?;

        // KuCoin returns arrays: [time, open, close, high, low, volume, turnover]
        // Timestamps are in SECONDS (not milliseconds!)
        let mut candles = Vec::with_capacity(data.len());
        for entry in data {
            let arr = entry.as_array().ok_or_else(|| {
                ExecutionError::Other("KuCoin candle entry is not an array".into())
            })?;

            if arr.len() < 6 {
                continue;
            }

            let timestamp_secs = arr[0].as_str().unwrap_or("0").parse::<i64>().unwrap_or(0);
            let open = arr[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let close = arr[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let high = arr[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let low = arr[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let volume = arr[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

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

        // KuCoin returns newest first — reverse to chronological order
        candles.reverse();

        // Limit to requested count
        if candles.len() > count as usize {
            let start = candles.len() - count as usize;
            candles = candles[start..].to_vec();
        }

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "KuCoin: no candles for {}",
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
    fn kucoin_pair_mapping() {
        let src = KuCoinSource::new();
        assert_eq!(src.kucoin_pair("BTC/USD"), Some("BTC-USDT".into()));
        assert_eq!(src.kucoin_pair("ETH/USDT"), Some("ETH-USDT".into()));
        assert_eq!(src.kucoin_pair("SOL/USD"), Some("SOL-USDT".into()));
        assert_eq!(src.kucoin_pair("FAKE/EUR"), None);
    }

    #[test]
    fn kucoin_type_mapping() {
        let src = KuCoinSource::new();
        assert_eq!(src.kucoin_type(5), "5min");
        assert_eq!(src.kucoin_type(60), "1hour");
        assert_eq!(src.kucoin_type(1440), "1day");
    }
}
