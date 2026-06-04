//! CoinGecko candle source — broader coverage for tokens without Kraken data.
//!
//! Uses CoinGecko's free market_chart API to fetch OHLC data.
//! Covers thousands of tokens including meme coins.
//!
//! Granularity (free API):
//!   - 1 day: 5-minute candles (288 candles)
//!   - 2-90 days: hourly candles
//!   - 90+ days: daily candles

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
            "LDO" => Some("lido-dao"),
            "PENDLE" => Some("pendle"),
            "GRT" => Some("the-graph"),
            "BONK" => Some("bonk"),
            "DOT" => Some("polkadot"),
            "RENDER" => Some("render-token"),
            "FET" => Some("fetch-ai"),
            "ENA" => Some("ethena"),
            "ZRO" => Some("layerzero"),
            "CRV" => Some("curve-dao-token"),
            "SUSHI" => Some("sushi"),
            "COMP" => Some("compound-governance-token"),
            "MATIC" => Some("matic-network"),
            "NEAR" => Some("near"),
            "ATOM" => Some("cosmos"),
            "FTM" => Some("fantom"),
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

        // Use market_chart endpoint — gives 5m candles for 1 day
        let days = match timeframe_minutes {
            1..=5 => 1,    // 5m candles (288 for 1 day)
            6..=60 => 7,   // hourly candles (168 for 7 days)
            _ => 1,
        };

        let url = format!(
            "https://api.coingecko.com/api/v3/coins/{}/market_chart?vs_currency=usd&days={}",
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

        // market_chart returns { "prices": [[ts, price], ...], "market_caps": [...], "total_volumes": [...] }
        let prices = json["prices"]
            .as_array()
            .ok_or_else(|| ExecutionError::Other("CoinGecko: missing prices array".into()))?;

        let volumes = json["total_volumes"]
            .as_array()
            .ok_or_else(|| ExecutionError::Other("CoinGecko: missing volumes array".into()))?;

        let mut candles = Vec::new();
        let len = prices.len().min(volumes.len()).min(count as usize);

        // Convert price points to OHLC candles by grouping consecutive points
        // For 5m candles, group every 6 points (30 min) to create OHLC
        let group_size = if timeframe_minutes <= 5 { 6 } else { 1 };

        let mut i = 0;
        while i + group_size <= len {
            let chunk = &prices[i..i + group_size];
            let vol_chunk = &volumes[i..i + group_size];

            let open = chunk[0][1].as_f64().unwrap_or(0.0);
            let close = chunk[group_size - 1][1].as_f64().unwrap_or(0.0);
            let high = chunk.iter().map(|p| p[1].as_f64().unwrap_or(0.0)).fold(f64::NEG_INFINITY, f64::max);
            let low = chunk.iter().map(|p| p[1].as_f64().unwrap_or(0.0)).fold(f64::INFINITY, f64::min);
            let timestamp = chunk[group_size - 1][0].as_u64().unwrap_or(0);
            let volume = vol_chunk.iter().map(|v| v[1].as_f64().unwrap_or(0.0)).sum::<f64>() / group_size as f64;

            candles.push(Candle {
                pair: pair.to_string(),
                open,
                high,
                low,
                close,
                volume,
                timestamp,
            });

            i += group_size;
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
