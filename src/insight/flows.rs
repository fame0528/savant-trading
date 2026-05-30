//! Exchange inflow/outflow data — indicates buying/selling pressure.

use serde::{Deserialize, Serialize};

/// Exchange flow data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowData {
    /// BTC flowing to exchanges (bearish — preparing to sell)
    pub btc_exchange_inflow: Option<f64>,
    /// BTC flowing out of exchanges (bullish — holding)
    pub btc_exchange_outflow: Option<f64>,
    /// Stablecoin inflow to exchanges (buying pressure)
    pub stablecoin_inflow: Option<f64>,
}

/// Fetch exchange flow data.
///
/// Uses CryptoQuant free tier or Glassnode.
pub async fn fetch_flows(_client: &reqwest::Client, _api_key: Option<&str>) -> FlowData {
    // TODO: Implement CryptoQuant or Glassnode API
    // For now return empty — will be populated when API key is configured
    FlowData::default()
}
