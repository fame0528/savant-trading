//! Liquidation cluster data — where price is likely to hunt.

use serde::{Deserialize, Serialize};

/// Liquidation data from derivatives markets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiquidationData {
    /// Price level where long liquidations cluster
    pub long_liquidation_cluster: Option<f64>,
    /// Price level where short liquidations cluster
    pub short_liquidation_cluster: Option<f64>,
    /// Total liquidations in last 24h (USD)
    pub total_liquidations_24h: Option<f64>,
}

/// Fetch liquidation data for a symbol.
///
/// Uses CoinGlass API (may require API key).
pub async fn fetch_liquidation(
    _client: &reqwest::Client,
    _symbol: &str,
    _api_key: Option<&str>,
) -> LiquidationData {
    // TODO: Implement CoinGlass liquidation API
    // For now return empty — will be populated when API key is configured
    LiquidationData::default()
}
