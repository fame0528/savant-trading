//! DexScreener candle source — DEX-native coverage for Arbitrum tokens.
//!
//! Uses DexScreener's free API for OHLCV data from on-chain pools.
//! Covers all Arbitrum DEX pairs including DeFi/LSD tokens not on CEXes.
//!
//! Free tier: 300 requests per minute, no API key required.
//!
//! Two-step process:
//! 1. Get pool address from token address: GET /latest/dex/tokens/{address}
//! 2. Get OHLCV from pool: GET /latest/dex/pairs/{chainId}/{address}/ohlcv/5m

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

pub struct DexScreenerSource {
    client: reqwest::Client,
}

impl DexScreenerSource {
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

impl Default for DexScreenerSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for DexScreenerSource {
    fn name(&self) -> &str {
        "DexScreener"
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

        // Step 1: Get pool address from token address
        let url = format!(
            "https://api.dexscreener.com/latest/dex/tokens/{}",
            token_addr
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("DexScreener tokens request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(ExecutionError::Other(format!(
                "DexScreener tokens returned {}",
                status
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("DexScreener tokens parse error: {}", e)))?;

        let pairs = json["pairs"].as_array().ok_or_else(|| {
            ExecutionError::Other(format!("DexScreener: no pairs found for {}", token_addr))
        })?;

        if pairs.is_empty() {
            return Err(ExecutionError::Other(format!(
                "DexScreener: empty pairs for {}",
                token_addr
            )));
        }

        // Find the best Arbitrum pair (highest liquidity)
        let best_pair = pairs
            .iter()
            .filter(|p| p["chainId"].as_str() == Some("arbitrum"))
            .filter(|p| p["pairAddress"].as_str().is_some())
            .max_by(|a, b| {
                let liq_a = a["liquidity"]["usd"].as_f64().unwrap_or(0.0);
                let liq_b = b["liquidity"]["usd"].as_f64().unwrap_or(0.0);
                liq_a.partial_cmp(&liq_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| {
                ExecutionError::Other(format!("DexScreener: no Arbitrum pairs for {}", pair))
            })?;

        let pair_address = best_pair["pairAddress"].as_str().ok_or_else(|| {
            ExecutionError::Other("DexScreener: pairAddress missing".into())
        })?;

        // Step 2: Get OHLCV from the pair
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
            "https://api.dexscreener.com/latest/dex/pairs/arbitrum/{}/ohlcv/{}",
            pair_address, tf_str
        );

        let ohlcv_resp = self
            .client
            .get(&ohlcv_url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("DexScreener OHLCV request failed: {}", e)))?;

        if !ohlcv_resp.status().is_success() {
            let status = ohlcv_resp.status();
            return Err(ExecutionError::Other(format!(
                "DexScreener OHLCV returned {}",
                status
            )));
        }

        let ohlcv_json: serde_json::Value = ohlcv_resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("DexScreener OHLCV parse error: {}", e)))?;

        let candles_data = ohlcv_json["candles"].as_array().ok_or_else(|| {
            ExecutionError::Other("DexScreener OHLCV missing candles array".into())
        })?;

        let mut candles = Vec::with_capacity(candles_data.len());
        for entry in candles_data {
            let timestamp_ms = entry["timestamp"].as_i64().unwrap_or(0);
            let open = entry["open"].as_f64().unwrap_or(0.0);
            let high = entry["high"].as_f64().unwrap_or(0.0);
            let low = entry["low"].as_f64().unwrap_or(0.0);
            let close = entry["close"].as_f64().unwrap_or(0.0);
            let volume = entry["volume"].as_f64().unwrap_or(0.0);

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

        // Limit to requested count
        if candles.len() > count as usize {
            let start = candles.len() - count as usize;
            candles = candles[start..].to_vec();
        }

        if candles.is_empty() {
            return Err(ExecutionError::Other(format!(
                "DexScreener: no candles for {}",
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
        let src = DexScreenerSource::new();
        // BTC has an address in the DB
        assert!(src.might_have("WBTC/USD"), "DexScreener should support WBTC (BTC→WBTC mapping)");
        // A fake token won't
        assert!(!src.might_have("FAKE/USD"));
    }
}
