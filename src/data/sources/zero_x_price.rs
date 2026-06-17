//! FID-188: 0x AMM Price Source
//!
//! Replaces Kraken CEX-derived spot price for live decision-making on
//! Arbitrum. Uses 0x `/swap/allowance-holder/price` endpoint which returns
//! the AMM-implied price including slippage for a 10 USDC swap.
//!
//! Per Gemini Q7: "Halt the use of Kraken CEX data for Arbitrum trading.
//! Query real AMM liquidity depth to prevent theoretical trades that will
//! fail due to on-chain slippage."
//!
//! Candle data (historical OHLC) still comes from Kraken/OKX/KuCoin/etc.
//! Only the live spot price for live trading decisions comes from 0x.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::core::error::DataError;
use crate::data::token_discovery::{USDC_ARBITRUM, VALIDATION_SELL_AMOUNT};

/// 0x AMM-derived price/liquidity snapshot for a single token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmQuote {
    pub pair: String,
    /// Effective price in USDC per token (10 USDC / buyAmount).
    pub price_usdc: f64,
    /// Buy amount in token base units for a 10 USDC sell.
    pub buy_amount: f64,
    /// Estimated price impact from 0x (0.0 = no impact, 0.04 = 4% impact).
    pub estimated_price_impact: f64,
    /// True if 0x found a route (false = no liquidity, can't trade).
    pub liquidity_available: bool,
    /// Timestamp of the quote (ISO 8601).
    pub timestamp: String,
}

#[derive(Clone)]
pub struct ZeroXPriceSource {
    client: Client,
    api_key: String,
    chain_id: u64,
}

impl ZeroXPriceSource {
    pub fn new(api_key: String, chain_id: u64) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            api_key,
            chain_id,
        }
    }

    /// Quote the AMM-implied price for `token_address` on this chain.
    /// Returns `Ok(AmmQuote)` if 0x returned a route, `Ok(quote)` with
    /// `liquidity_available = false` if no route, `Err` on network error.
    pub async fn quote(
        &self,
        token_address: &str,
        pair_label: &str,
    ) -> Result<AmmQuote, DataError> {
        let url = format!(
            "https://api.0x.org/swap/allowance-holder/price?\
             chainId={}&sellToken={}&buyToken={}&sellAmount={}&taker=0x0000000000000000000000000000000000000000",
            self.chain_id, USDC_ARBITRUM, token_address, VALIDATION_SELL_AMOUNT
        );

        let resp = self
            .client
            .get(&url)
            .header("0x-api-key", &self.api_key)
            .header("0x-version", "v2")
            .send()
            .await
            .map_err(|e| DataError::HttpError(format!("0x quote HTTP: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(
                "0x quote returned {} for {} ({}): {}",
                status, pair_label, token_address, body
            );
            return Err(DataError::HttpError(format!(
                "0x quote returned {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(format!("0x quote parse: {}", e)))?;

        let available = json["liquidityAvailable"].as_bool().unwrap_or(false);
        let buy_amount_str = json["buyAmount"].as_str().unwrap_or("0");
        let buy_amount: f64 = buy_amount_str.parse().unwrap_or(0.0);
        // 10 USDC (6 decimals) -> buy_amount (varies by token decimals)
        // price_usdc = (10 / buy_amount) adjusted for token decimals
        // For now: use simple 10/buy_amount ratio assuming 18-decimal tokens
        let price_usdc = if buy_amount > 0.0 {
            10.0 / buy_amount * 1e12 // 10 USDC (6 dec) / buy_amount (18 dec) = price * 1e12
        } else {
            0.0
        };
        let price_impact = json["estimatedPriceImpact"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        debug!(
            "0x quote: {} = ${} (impact: {}%, available: {})",
            pair_label, price_usdc, price_impact, available
        );

        Ok(AmmQuote {
            pair: pair_label.to_string(),
            price_usdc,
            buy_amount,
            estimated_price_impact: price_impact,
            liquidity_available: available,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Validate a token has 0x liquidity (used by token_discovery).
    /// Wrapper around quote() for compatibility with FID-121.
    pub async fn has_liquidity(&self, token_address: &str) -> bool {
        match self.quote(token_address, "validation").await {
            Ok(q) => q.liquidity_available,
            Err(_) => false,
        }
    }
}
