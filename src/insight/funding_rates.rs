//! Funding rates, open interest, and long/short ratio from derivatives markets.

use serde::{Deserialize, Serialize};

/// Derivatives funding data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FundingData {
    /// Current funding rate (positive = longs pay shorts)
    pub funding_rate: Option<f64>,
    /// Total open interest in USD
    pub open_interest: Option<f64>,
    /// Long/short ratio (>1 = more longs)
    pub long_short_ratio: Option<f64>,
}

/// Fetch funding data for a symbol.
///
/// Uses CoinGlass API (may require API key) or falls back to Coinalyze.
pub async fn fetch_funding(
    client: &reqwest::Client,
    _symbol: &str,
    _api_key: Option<&str>,
) -> FundingData {
    // TODO: Implement CoinGlass or Coinalyze API integration
    // For now return empty — will be populated when API key is configured
    FundingData::default()
}
