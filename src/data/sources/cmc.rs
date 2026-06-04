//! CoinMarketCap candle source — requires free API key (10K calls/month).
//!
//! API: GET https://pro-api.coinmarketcap.com/v2/cryptocurrency/ohlcv/latest
//! Free tier: 10,000 requests per month with free API key.
//!
//! Set CMC_API_KEY environment variable to enable.

use async_trait::async_trait;
use super::CandleSource;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

pub struct CmcSource {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CmcSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            api_key: std::env::var("COINMARKETCAP_API_KEY").ok(),
        }
    }

    /// Map pair symbol to CMC symbol.
    fn cmc_symbol(&self, pair: &str) -> Option<String> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let base = parts[0].to_uppercase();
        let quote = parts[1].to_uppercase();

        // CMC only supports USD/USDT quotes
        match quote.as_str() {
            "USD" | "USDT" | "USDC" => Some(base),
            _ => None,
        }
    }
}

impl Default for CmcSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CandleSource for CmcSource {
    fn name(&self) -> &str {
        "CMC"
    }

    fn might_have(&self, pair: &str) -> bool {
        self.api_key.is_some() && self.cmc_symbol(pair).is_some()
    }

    async fn fetch_candles(
        &self,
        pair: &str,
        _timeframe_minutes: u32,
        _count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        let _api_key = self.api_key.as_ref().ok_or_else(|| {
            ExecutionError::Other("CMC_API_KEY not set".into())
        })?;

        let symbol = self.cmc_symbol(pair).ok_or_else(|| {
            ExecutionError::Other(format!("No CMC pair mapping for {}", pair))
        })?;

        // CMC v2 OHLCV endpoint
        // Note: The free tier only returns the latest OHLCV snapshot, not historical candles.
        // For historical data, we'd need the paid tier. This source is best used as a
        // last-resort price check.
        let url = format!(
            "https://pro-api.coinmarketcap.com/v2/cryptocurrency/ohlcv/latest?symbol={}&convert=USD",
            symbol
        );

        let resp = self
            .client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", _api_key)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("CMC request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ExecutionError::Other(format!(
                "CMC returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("CMC parse error: {}", e)))?;

        // CMC v2 returns data keyed by symbol
        let data = json["data"].get(&symbol).and_then(|d| d.as_array()).ok_or_else(|| {
            ExecutionError::Other(format!("CMC response missing data for {}", symbol))
        })?;

        if data.is_empty() {
            return Err(ExecutionError::Other(format!("CMC: no data for {}", symbol)));
        }

        // Use the first entry's latest OHLCV
        let entry = &data[0];
        let quote = &entry["quote"]["USD"];

        let open = quote["open"].as_f64().unwrap_or(0.0);
        let high = quote["high"].as_f64().unwrap_or(0.0);
        let low = quote["low"].as_f64().unwrap_or(0.0);
        let close = quote["close"].as_f64().unwrap_or(quote["price"].as_f64().unwrap_or(0.0));
        let volume = quote["volume_24h"].as_f64().unwrap_or(0.0);

        if close == 0.0 {
            return Err(ExecutionError::Other(format!("CMC: zero price for {}", symbol)));
        }

        // CMC only returns a single snapshot candle
        let candle = Candle {
            pair: pair.to_string(),
            open,
            high,
            low,
            close,
            volume,
            timestamp: chrono::Utc::now(),
        };

        Ok(vec![candle])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmc_symbol_mapping() {
        let src = CmcSource::new();
        assert_eq!(src.cmc_symbol("BTC/USD"), Some("BTC".into()));
        assert_eq!(src.cmc_symbol("ETH/USDT"), Some("ETH".into()));
        assert_eq!(src.cmc_symbol("FAKE/EUR"), None);
    }
}
