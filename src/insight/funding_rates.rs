//! Funding rates, open interest, and long/short ratio from derivatives markets.
//!
//! Uses Kraken Futures API (free, no key, no geo-block).
//! Endpoint: https://futures.kraken.com/derivatives/api/v3/tickers
//! Returns funding rate, open interest, mark price, volume for all perpetual contracts.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Derivatives funding data for a single symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FundingData {
    /// Current funding rate (positive = longs pay shorts)
    pub funding_rate: Option<f64>,
    /// Predicted next funding rate
    pub funding_rate_prediction: Option<f64>,
    /// Total open interest in contracts
    pub open_interest: Option<f64>,
    /// Mark price
    pub mark_price: Option<f64>,
    /// 24h volume in contracts
    pub volume_24h: Option<f64>,
    /// 24h price change percentage
    pub change_24h: Option<f64>,
    /// Index price (spot reference)
    pub index_price: Option<f64>,
    /// Annualized funding rate (rate × 3 × 365 for 8h funding)
    pub funding_rate_annualized: Option<f64>,
}

/// Kraken Futures ticker response.
#[derive(Debug, Deserialize)]
struct KrakenFuturesResponse {
    tickers: Option<Vec<KrakenFuturesTicker>>,
}

#[derive(Debug, Deserialize)]
struct KrakenFuturesTicker {
    symbol: Option<String>,
    #[serde(rename = "fundingRate")]
    funding_rate: Option<f64>,
    #[serde(rename = "fundingRatePrediction")]
    funding_rate_prediction: Option<f64>,
    #[serde(rename = "openInterest")]
    open_interest: Option<f64>,
    #[serde(rename = "markPrice")]
    mark_price: Option<f64>,
    #[serde(rename = "vol24h")]
    vol_24h: Option<f64>,
    #[serde(rename = "change24h")]
    change_24h: Option<f64>,
    #[serde(rename = "indexPrice")]
    index_price: Option<f64>,
    tag: Option<String>,
    pair: Option<String>,
    suspended: Option<bool>,
}

/// Fetch funding data for a symbol from Kraken Futures.
///
/// Free, no key, no geo-block. Maps trading pair (e.g., "BTC/USD") to
/// Kraken Futures perpetual symbol (e.g., "PF_XBTUSD").
pub async fn fetch_funding(client: &reqwest::Client, symbol: &str) -> FundingData {
    // Fetch all tickers from Kraken Futures
    match client
        .get("https://futures.kraken.com/derivatives/api/v3/tickers")
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("Kraken Futures API returned HTTP {}", resp.status());
                return FundingData::default();
            }

            match resp.json::<KrakenFuturesResponse>().await {
                Ok(data) => parse_kraken_futures(&data, symbol),
                Err(e) => {
                    warn!("Kraken Futures response parse error: {}", e);
                    FundingData::default()
                }
            }
        }
        Err(e) => {
            warn!("Kraken Futures API request failed: {}", e);
            FundingData::default()
        }
    }
}

/// Parse Kraken Futures response and find the matching perpetual ticker.
fn parse_kraken_futures(response: &KrakenFuturesResponse, symbol: &str) -> FundingData {
    let tickers = match &response.tickers {
        Some(t) => t,
        None => {
            debug!("No tickers in Kraken Futures response");
            return FundingData::default();
        }
    };

    // Convert symbol to Kraken Futures format
    // "BTC/USD" → look for "PF_XBTUSD" (perpetual)
    // "ETH/USD" → look for "PF_ETHUSD"
    let base = symbol.split('/').next().unwrap_or(symbol).to_uppercase();

    // Kraken uses XBT for BTC in futures symbols
    let futures_base = if base == "BTC" { "XBT" } else { &base };
    let expected_pair = format!("{}:USD", base);
    let expected_symbol = format!("PF_{}USD", futures_base);

    for ticker in tickers {
        // Match by pair (e.g., "BTC:USD") or symbol (e.g., "PF_XBTUSD")
        let pair_match = ticker
            .pair
            .as_deref()
            .map(|p| p.eq_ignore_ascii_case(&expected_pair))
            .unwrap_or(false);

        let symbol_match = ticker
            .symbol
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case(&expected_symbol))
            .unwrap_or(false);

        if pair_match || symbol_match {
            // Only use perpetual contracts (tag = "perpetual"), not futures
            let is_perpetual = ticker
                .tag
                .as_deref()
                .map(|t| t == "perpetual")
                .unwrap_or(false);

            if !is_perpetual {
                continue;
            }

            // Skip suspended tickers
            if ticker.suspended.unwrap_or(false) {
                debug!("Skipping suspended ticker: {:?}", ticker.symbol);
                continue;
            }

            let mut data = FundingData {
                funding_rate: ticker.funding_rate,
                funding_rate_prediction: ticker.funding_rate_prediction,
                open_interest: ticker.open_interest,
                mark_price: ticker.mark_price,
                volume_24h: ticker.vol_24h,
                change_24h: ticker.change_24h,
                index_price: ticker.index_price,
                funding_rate_annualized: None,
            };

            // Calculate annualized rate (funding every 8h = 3x/day × 365 days)
            if let Some(fr) = data.funding_rate {
                data.funding_rate_annualized = Some(fr * 3.0 * 365.0);
            }

            info!(
                "Kraken Futures {}: rate={:.6}%, pred={:.6}%, OI={:.0}, mark=${:.2}, vol={:.0}",
                symbol,
                data.funding_rate.unwrap_or(0.0) * 100.0,
                data.funding_rate_prediction.unwrap_or(0.0) * 100.0,
                data.open_interest.unwrap_or(0.0),
                data.mark_price.unwrap_or(0.0),
                data.volume_24h.unwrap_or(0.0),
            );

            return data;
        }
    }

    debug!("No perpetual ticker found for {} in Kraken Futures", symbol);
    FundingData::default()
}

/// Fetch funding data for multiple symbols at once (single API call).
pub async fn fetch_funding_multi(
    client: &reqwest::Client,
    symbols: &[String],
) -> Vec<(String, FundingData)> {
    match client
        .get("https://futures.kraken.com/derivatives/api/v3/tickers")
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("Kraken Futures API returned HTTP {}", resp.status());
                return symbols
                    .iter()
                    .map(|s| (s.clone(), FundingData::default()))
                    .collect();
            }

            match resp.json::<KrakenFuturesResponse>().await {
                Ok(data) => symbols
                    .iter()
                    .map(|s| (s.clone(), parse_kraken_futures(&data, s)))
                    .collect(),
                Err(e) => {
                    warn!("Kraken Futures parse error: {}", e);
                    symbols
                        .iter()
                        .map(|s| (s.clone(), FundingData::default()))
                        .collect()
                }
            }
        }
        Err(e) => {
            warn!("Kraken Futures request failed: {}", e);
            symbols
                .iter()
                .map(|s| (s.clone(), FundingData::default()))
                .collect()
        }
    }
}
