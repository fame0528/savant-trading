//! GeckoTerminal candle source — CoinGecko's on-chain arm.
//!
//! Uses GeckoTerminal's free API for OHLCV data from DEX pools.
//! Covers all Arbitrum DEX pools including DeFi/LSD tokens.
//!
//! Free tier: 30 requests per minute, no API key required.
//!
//! Two-step process:
//! 1. Get pool address from token: GET /api/v2/networks/arbitrum/tokens/{address}/pools
//! 2. Get OHLCV from pool: GET /api/v2/networks/arbitrum/pools/{pool_address}/ohlcv/5m

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

pub struct GeckoTerminalSource {
    client: reqwest::Client,
}

impl GeckoTerminalSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Get the token address for a pair from our token database.
    fn token_address(&self, pair: &str) -> Option<String> {
        let base = pair.split('/').next()?;
        let (addr, _dec) = crate::execution::dex::lookup_token(base, 42161)?;
        if addr.is_empty() {
            return None;
        }
        Some(addr)
    }
}

impl Default for GeckoTerminalSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for GeckoTerminalSource {
    fn name(&self) -> &str {
        "GeckoTerminal"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.token_address(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let token_addr = self.token_address(pair).ok_or_else(|| {
            ExecutionError::Other(format!("No token address for {} in DB", pair))
        })?;

        // Step 1: Get pool address from token
        let pools_url = format!(
            "https://api.geckoterminal.com/api/v2/networks/arbitrum/tokens/{}/pools?page=1&sort=h24_volume_usd_desc",
            token_addr
        );

        let pools_resp = self
            .client
            .get(&pools_url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("GeckoTerminal pools request failed: {}", e)))?;

        if !pools_resp.status().is_success() {
            let status = pools_resp.status();
            return Err(ExecutionError::Other(format!(
                "GeckoTerminal pools returned {}",
                status
            )));
        }

        let pools_json: serde_json::Value = pools_resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("GeckoTerminal pools parse error: {}", e)))?;

        let pools = pools_json["data"].as_array().ok_or_else(|| {
            ExecutionError::Other(format!("GeckoTerminal: no pools found for {}", token_addr))
        })?;

        if pools.is_empty() {
            return Err(ExecutionError::Other(format!(
                "GeckoTerminal: empty pools for {}",
                token_addr
            )));
        }

        // Use the first (highest volume) pool
        let pool_address = pools[0]["id"].as_str().ok_or_else(|| {
            ExecutionError::Other("GeckoTerminal: pool id missing".into())
        })?;

        // Step 2: Get OHLCV from pool
        let tf_str = match timeframe_minutes {
            1 => "1m",
            5 => "5m",
            15 => "15m",
            60 => "1h",
            240 => "4h",
            1440 => "1d",
            _ => "5m",
        };

        let ohlcv_url = format!(
            "https://api.geckoterminal.com/api/v2/networks/arbitrum/pools/{}/ohlcv/{}?aggregate={}&limit={}",
            pool_address, tf_str, timeframe_minutes, count.min(1000)
        );

        let ohlcv_resp = self
            .client
            .get(&ohlcv_url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("GeckoTerminal OHLCV request failed: {}", e)))?;

        if !ohlcv_resp.status().is_success() {
            let status = ohlcv_resp.status();
            return Err(ExecutionError::Other(format!(
                "GeckoTerminal OHLCV returned {}",
                status
            )));
        }

        let ohlcv_json: serde_json::Value = ohlcv_resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("GeckoTerminal OHLCV parse error: {}", e)))?;

        let candles_data = ohlcv_json["data"]["attributes"]["ohlcv_list"].as_array().ok_or_else(|| {
            ExecutionError::Other("GeckoTerminal OHLCV missing data.attributes.ohlcv_list".into())
        })?;

        // GeckoTerminal returns: [[timestamp, open, high, low, close, volume], ...]
        // Timestamps are in seconds, newest first
        let mut candles = Vec::with_capacity(candles_data.len());
        for entry in candles_data {
            let arr = entry.as_array().ok_or_else(|| {
                ExecutionError::Other("GeckoTerminal candle entry is not an array".into())
            })?;

            if arr.len() < 6 {
                continue;
            }

            let timestamp_secs = arr[0].as_i64().unwrap_or(0);
            let open = arr[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let high = arr[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let low = arr[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let close = arr[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            let volume = arr[5].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

            let timestamp = chrono::DateTime::from_timestamp(timestamp_secs, 0)
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

        // GeckoTerminal returns newest first — reverse to chronological order
        candles.reverse();

        // Limit to requested count
        if candles.len() > count as usize {
            let start = candles.len() - count as usize;
            candles = candles[start..].to_vec();
        }

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "GeckoTerminal: no candles for {}",
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
    fn might_have_requires_token_address() {
        let src = GeckoTerminalSource::new();
        // BTC has an address in the DB
        assert!(src.might_have("BTC/USD"));
        // A fake token won't
        assert!(!src.might_have("FAKE/USD"));
    }
}
