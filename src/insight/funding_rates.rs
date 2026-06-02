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

/// Valid funding rate range per 8hr. Outside this = garbage data.
/// Normal range: -0.5% to +0.5%. Extreme: -2% to +2%.
const FUNDING_RATE_MIN: f64 = -0.02;
const FUNDING_RATE_MAX: f64 = 0.02;

/// Fetch funding data — OKX primary, Kraken fallback with validation.
///
/// OKX: Free, no key, no geo-block. Returns decimal rate (0.0001 = 0.01%).
/// Kraken: Free, no key. Returns percentage rate but can return garbage.
pub async fn fetch_funding(client: &reqwest::Client, symbol: &str) -> FundingData {
    // Primary: OKX
    if let Some(data) = fetch_okx_funding(client, symbol).await {
        return data;
    }

    // Fallback: Kraken Futures with range validation
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
                Ok(data) => {
                    let result = parse_kraken_futures(&data, symbol);
                    // Range validation — reject garbage values
                    if let Some(fr) = result.funding_rate {
                        if !(FUNDING_RATE_MIN..=FUNDING_RATE_MAX).contains(&fr) {
                            warn!(
                                "Kraken funding rate {:.4}% outside valid range [{}, {}] — rejecting",
                                fr * 100.0,
                                FUNDING_RATE_MIN * 100.0,
                                FUNDING_RATE_MAX * 100.0
                            );
                            return FundingData::default();
                        }
                    }
                    result
                }
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

/// Fetch funding rate from OKX (primary source).
///
/// OKX returns decimal rate: 0.0001 = 0.01% per 8hr.
/// Pair mapping: "BTC/USD" → "BTC-USDT-SWAP"
async fn fetch_okx_funding(client: &reqwest::Client, symbol: &str) -> Option<FundingData> {
    let base = symbol.split('/').next().unwrap_or(symbol).to_uppercase();
    let inst_id = format!("{}-USDT-SWAP", base);

    let url = format!(
        "https://www.okx.com/api/v5/public/funding-rate?instId={}",
        inst_id
    );

    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        warn!("OKX funding returned HTTP {}", resp.status());
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let data = json.get("data")?.as_array()?.first()?;

    let rate_str = data.get("fundingRate")?.as_str()?;
    let rate: f64 = rate_str.parse().ok()?;

    // Range validation
    if !(FUNDING_RATE_MIN..=FUNDING_RATE_MAX).contains(&rate) {
        warn!(
            "OKX funding rate {:.4}% outside valid range — rejecting",
            rate * 100.0
        );
        return None;
    }

    let annualized = rate * 3.0 * 365.0;

    info!(
        "OKX {}: rate={:.6}%, annualized={:.2}%",
        symbol,
        rate * 100.0,
        annualized * 100.0
    );

    Some(FundingData {
        funding_rate: Some(rate),
        funding_rate_prediction: None,
        open_interest: None,
        mark_price: None,
        volume_24h: None,
        change_24h: None,
        index_price: None,
        funding_rate_annualized: Some(annualized),
    })
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

/// Fetch funding data for multiple symbols at once.
///
/// Tries OKX first (per-symbol), falls back to Kraken (single API call for all).
/// Applies range validation to all results.
pub async fn fetch_funding_multi(
    client: &reqwest::Client,
    symbols: &[String],
) -> Vec<(String, FundingData)> {
    // Try OKX for each symbol first
    let mut results: Vec<(String, FundingData)> = Vec::new();
    let mut okx_misses: Vec<String> = Vec::new();

    for symbol in symbols {
        if let Some(data) = fetch_okx_funding(client, symbol).await {
            results.push((symbol.clone(), data));
        } else {
            okx_misses.push(symbol.clone());
        }
    }

    // Fetch Kraken for any that missed OKX
    if !okx_misses.is_empty() {
        match client
            .get("https://futures.kraken.com/derivatives/api/v3/tickers")
            .send()
            .await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    warn!("Kraken Futures API returned HTTP {}", resp.status());
                    for s in &okx_misses {
                        results.push((s.clone(), FundingData::default()));
                    }
                } else {
                    match resp.json::<KrakenFuturesResponse>().await {
                        Ok(data) => {
                            for s in &okx_misses {
                                let parsed = parse_kraken_futures(&data, s);
                                // Range validation — reject garbage values
                                if let Some(fr) = parsed.funding_rate {
                                    if !(FUNDING_RATE_MIN..=FUNDING_RATE_MAX).contains(&fr) {
                                        warn!(
                                            "Kraken funding {:.4}% outside [{}, {}] — rejecting {}",
                                            fr * 100.0,
                                            FUNDING_RATE_MIN * 100.0,
                                            FUNDING_RATE_MAX * 100.0,
                                            s
                                        );
                                        results.push((s.clone(), FundingData::default()));
                                        continue;
                                    }
                                }
                                results.push((s.clone(), parsed));
                            }
                        }
                        Err(e) => {
                            warn!("Kraken Futures parse error: {}", e);
                            for s in &okx_misses {
                                results.push((s.clone(), FundingData::default()));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Kraken Futures request failed: {}", e);
                for s in &okx_misses {
                    results.push((s.clone(), FundingData::default()));
                }
            }
        }
    }

    results
}
