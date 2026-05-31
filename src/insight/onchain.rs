//! On-chain analytics — MVRV, NUPL, SOPR, NVT from free APIs.
//!
//! Uses CoinMetrics Community API (free tier) with graceful fallback.
//! Provides network valuation metrics for AI context enrichment.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// On-chain analytics data for Bitcoin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OnchainData {
    /// Market Value to Realized Value ratio (>3.5 = euphoria, <1.0 = capitulation)
    pub mvrv: Option<f64>,
    /// Net Unrealized Profit/Loss (>0.75 = euphoria, <0 = capitulation)
    pub nupl: Option<f64>,
    /// Spent Output Profit Ratio (>1.0 = profit realization, <1.0 = loss realization)
    pub sopr: Option<f64>,
    /// Network Value to Transactions Signal (90d MA smoothed)
    pub nvt_signal: Option<f64>,
    /// Exchange balance in BTC (declining = bullish supply squeeze)
    pub exchange_balance: Option<f64>,
    /// Exchange net flow (negative = outflow = bullish)
    pub exchange_net_flow_24h: Option<f64>,
    /// Data source attribution
    pub source: String,
}

/// CoinMetrics community API response for asset metrics.
#[derive(Debug, Deserialize)]
struct CoinMetricsResponse {
    data: Option<Vec<CoinMetricsDataPoint>>,
}

#[derive(Debug, Deserialize)]
struct CoinMetricsDataPoint {
    #[serde(rename = "CapMVRVCur")]
    cap_mvrv_cur: Option<String>,
    #[serde(rename = "NVTAdj")]
    nvt_adj: Option<String>,
}

/// Fetch on-chain data from free APIs with fallback chain.
pub async fn fetch_onchain(client: &reqwest::Client) -> OnchainData {
    let mut data = OnchainData {
        source: "pending".to_string(),
        ..Default::default()
    };

    // Try CoinMetrics Community API first (free, no key)
    fetch_coinmetrics(client, &mut data).await;

    // Fallback: CoinGecko if CoinMetrics failed
    if data.mvrv.is_none() {
        fetch_coingecko_onchain(client, &mut data).await;
    }

    // Blockchain.info hashrate (always fetch — supplementary data)
    fetch_blockchain_info(client, &mut data).await;

    if data.source == "pending" {
        data.source = "none".to_string();
    }

    data
}

/// Fetch MVRV, NVT from CoinMetrics Community API (free tier).
async fn fetch_coinmetrics(client: &reqwest::Client, data: &mut OnchainData) {
    let url = "https://community-api.coinmetrics.io/v4/timeseries/asset-metrics";
    let params = [
        ("assets", "btc"),
        ("metrics", "CapMVRVCur,NVTAdj"),
        ("frequency", "1d"),
        ("page_size", "1"),
    ];

    match client.get(url).query(&params).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("CoinMetrics API returned HTTP {}", resp.status());
                return;
            }

            match resp.json::<CoinMetricsResponse>().await {
                Ok(parsed) => {
                    if let Some(points) = parsed.data {
                        if let Some(latest) = points.last() {
                            if let Some(ref mvrv_str) = latest.cap_mvrv_cur {
                                if let Ok(v) = mvrv_str.parse::<f64>() {
                                    data.mvrv = Some(v);
                                    data.source = "coinmetrics".to_string();
                                    info!("CoinMetrics MVRV: {:.4}", v);
                                }
                            }
                            if let Some(ref nvt_str) = latest.nvt_adj {
                                if let Ok(v) = nvt_str.parse::<f64>() {
                                    data.nvt_signal = Some(v);
                                    debug!("CoinMetrics NVT: {:.2}", v);
                                }
                            }
                        }
                    }
                }
                Err(e) => warn!("CoinMetrics response parse error: {}", e),
            }
        }
        Err(e) => warn!("CoinMetrics API request failed: {}", e),
    }
}

/// CoinGecko fallback — derive MVRV proxy from market_cap / fdv ratio.
async fn fetch_coingecko_onchain(client: &reqwest::Client, data: &mut OnchainData) {
    let url = "https://api.coingecko.com/api/v3/coins/bitcoin";
    let params = [
        ("localization", "false"),
        ("tickers", "false"),
        ("market_data", "true"),
        ("community_data", "false"),
        ("developer_data", "false"),
    ];

    match client.get(url).query(&params).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("CoinGecko on-chain returned HTTP {}", resp.status());
                return;
            }

            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let md = json.get("market_data");
                let market_cap = md.and_then(|m| m.get("market_cap")?.get("usd")?.as_f64());
                let fdv = md.and_then(|m| m.get("fully_diluted_valuation")?.get("usd")?.as_f64());
                let total_volume = md.and_then(|m| m.get("total_volume")?.get("usd")?.as_f64());

                if let (Some(mcap), Some(fdv_val)) = (market_cap, fdv) {
                    if fdv_val > 0.0 {
                        let mvrv_proxy = mcap / fdv_val;
                        data.mvrv = Some(mvrv_proxy);
                        data.source = "coingecko-proxy".to_string();
                        info!("CoinGecko MVRV proxy: {:.4} (mcap/fdv)", mvrv_proxy);
                    }
                }

                if let (Some(vol), Some(mcap)) = (total_volume, market_cap) {
                    if vol > 0.0 {
                        let nvt_proxy = mcap / vol;
                        data.nvt_signal = Some(nvt_proxy);
                        debug!("CoinGecko NVT proxy: {:.2}", nvt_proxy);
                    }
                }
            }
        }
        Err(e) => warn!("CoinGecko on-chain request failed: {}", e),
    }
}

/// Fetch supplementary data from blockchain.info (free, no key).
async fn fetch_blockchain_info(client: &reqwest::Client, _data: &mut OnchainData) {
    match client
        .get("https://blockchain.info/q/hashrate")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                if let Ok(hashrate) = text.trim().parse::<f64>() {
                    debug!("Blockchain.info hashrate: {:.2} PH/s", hashrate);
                }
            }
        }
        Err(e) => warn!("blockchain.info hashrate request failed: {}", e),
    }
}

/// Derive market condition tags from on-chain data.
pub fn derive_conditions(data: &OnchainData) -> Vec<crate::agent::knowledge::MarketCondition> {
    use crate::agent::knowledge::MarketCondition;
    let mut conditions = Vec::new();

    if let Some(mvrv) = data.mvrv {
        if !(1.0..=3.5).contains(&mvrv) {
            conditions.push(MarketCondition::MvrvExtreme);
        }
    }

    if let Some(sopr) = data.sopr {
        if (0.98..=1.02).contains(&sopr) {
            conditions.push(MarketCondition::SoprReset);
        }
    }

    conditions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_onchain_data_is_empty() {
        let data = OnchainData::default();
        assert!(data.mvrv.is_none());
        assert!(data.nupl.is_none());
        assert!(data.sopr.is_none());
        assert!(data.nvt_signal.is_none());
    }

    #[test]
    fn derive_conditions_mvrv_extreme_high() {
        let data = OnchainData {
            mvrv: Some(4.0),
            ..Default::default()
        };
        let conditions = derive_conditions(&data);
        assert!(conditions.contains(&crate::agent::knowledge::MarketCondition::MvrvExtreme));
    }

    #[test]
    fn derive_conditions_mvrv_extreme_low() {
        let data = OnchainData {
            mvrv: Some(0.8),
            ..Default::default()
        };
        let conditions = derive_conditions(&data);
        assert!(conditions.contains(&crate::agent::knowledge::MarketCondition::MvrvExtreme));
    }

    #[test]
    fn derive_conditions_mvrv_normal() {
        let data = OnchainData {
            mvrv: Some(2.0),
            ..Default::default()
        };
        let conditions = derive_conditions(&data);
        assert!(!conditions.contains(&crate::agent::knowledge::MarketCondition::MvrvExtreme));
    }

    #[test]
    fn derive_conditions_sopr_reset() {
        let data = OnchainData {
            sopr: Some(1.0),
            ..Default::default()
        };
        let conditions = derive_conditions(&data);
        assert!(conditions.contains(&crate::agent::knowledge::MarketCondition::SoprReset));
    }

    #[test]
    fn derive_conditions_sopr_not_reset() {
        let data = OnchainData {
            sopr: Some(1.05),
            ..Default::default()
        };
        let conditions = derive_conditions(&data);
        assert!(!conditions.contains(&crate::agent::knowledge::MarketCondition::SoprReset));
    }
}
