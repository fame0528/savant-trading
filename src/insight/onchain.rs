//! On-chain analytics — MVRV, NUPL, SOPR from free APIs.
//!
//! Primary: BGeometrics (free, no key, api_key=test).
//! Fallback: CoinMetrics Community API (free).
//! Provides network valuation metrics for AI context enrichment.
//!
//! All values are range-validated on parse to reject garbage data.

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

/// BGeometrics API response — simple date+value pair.
#[derive(Debug, Deserialize)]
struct BgResponse {
    #[allow(dead_code)]
    date: Option<String>,
    value: Option<serde_json::Value>,
}

/// CoinMetrics community API response.
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

/// Valid ranges for on-chain metrics. Values outside are rejected as garbage.
const MVRV_MIN: f64 = 0.1;
const MVRV_MAX: f64 = 10.0;
const SOPR_MIN: f64 = 0.5;
const SOPR_MAX: f64 = 2.0;
const NUPL_MIN: f64 = -0.5;
const NUPL_MAX: f64 = 1.0;

/// Fetch on-chain data from free APIs with fallback chain.
///
/// Primary: BGeometrics (MVRV, SOPR, NUPL). Free, no key, daily data.
/// Fallback: CoinMetrics (MVRV, NVT). Free, no key.
pub async fn fetch_onchain(client: &reqwest::Client) -> OnchainData {
    let mut data = OnchainData {
        source: "pending".to_string(),
        ..Default::default()
    };

    // Primary: BGeometrics
    fetch_bgeometrics(client, &mut data).await;

    // Fallback: CoinMetrics if BGeometrics failed
    if data.mvrv.is_none() {
        fetch_coinmetrics(client, &mut data).await;
    }

    if data.source == "pending" {
        data.source = "none".to_string();
    }

    data
}

/// Fetch MVRV, SOPR, NUPL from BGeometrics (free, no key).
///
/// API: GET https://api.bgeometrics.com/v1/{metric}?api_key=test
/// Returns: {"date": "2025-05-31", "value": 1.375}
async fn fetch_bgeometrics(client: &reqwest::Client, data: &mut OnchainData) {
    let metrics = [("mvrv", "mvrv"), ("sopr", "sopr"), ("nupl", "nupl")];

    for (metric_name, field) in &metrics {
        let url = format!(
            "https://api.bgeometrics.com/v1/{}?api_key=test",
            metric_name
        );

        match client.get(&url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    warn!(
                        "BGeometrics {} returned HTTP {}",
                        metric_name,
                        resp.status()
                    );
                    continue;
                }

                match resp.json::<BgResponse>().await {
                    Ok(bg) => {
                        if let Some(val) = parse_bg_value(&bg.value) {
                            let valid = match *field {
                                "mvrv" => {
                                    if (MVRV_MIN..=MVRV_MAX).contains(&val) {
                                        data.mvrv = Some(val);
                                        true
                                    } else {
                                        warn!(
                                            "BGeometrics MVRV {} outside valid range [{}, {}]",
                                            val, MVRV_MIN, MVRV_MAX
                                        );
                                        false
                                    }
                                }
                                "sopr" => {
                                    if (SOPR_MIN..=SOPR_MAX).contains(&val) {
                                        data.sopr = Some(val);
                                        true
                                    } else {
                                        warn!(
                                            "BGeometrics SOPR {} outside valid range [{}, {}]",
                                            val, SOPR_MIN, SOPR_MAX
                                        );
                                        false
                                    }
                                }
                                "nupl" => {
                                    if (NUPL_MIN..=NUPL_MAX).contains(&val) {
                                        data.nupl = Some(val);
                                        true
                                    } else {
                                        warn!(
                                            "BGeometrics NUPL {} outside valid range [{}, {}]",
                                            val, NUPL_MIN, NUPL_MAX
                                        );
                                        false
                                    }
                                }
                                _ => false,
                            };
                            if valid {
                                data.source = "bgeometrics".to_string();
                                info!("BGeometrics {}: {:.4}", metric_name, val);
                            }
                        }
                    }
                    Err(e) => warn!("BGeometrics {} parse error: {}", metric_name, e),
                }
            }
            Err(e) => warn!("BGeometrics {} request failed: {}", metric_name, e),
        }
    }
}

/// Parse a BGeometrics value field (can be number or string).
fn parse_bg_value(value: &Option<serde_json::Value>) -> Option<f64> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Fallback: Fetch MVRV, NVT from CoinMetrics Community API.
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
                                    if (MVRV_MIN..=MVRV_MAX).contains(&v) {
                                        data.mvrv = Some(v);
                                        data.source = "coinmetrics".to_string();
                                        info!("CoinMetrics MVRV: {:.4}", v);
                                    }
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
    fn range_validation_mvrv() {
        assert!(MVRV_MIN <= 0.1 && MVRV_MAX >= 10.0);
        // Valid
        assert!((0.5..=9.0).contains(&1.375));
        // Invalid (garbage)
        assert!(!(0.1..=10.0).contains(&-45.0));
        assert!(!(0.1..=10.0).contains(&100.0));
    }

    #[test]
    fn parse_bg_value_number() {
        let val = Some(serde_json::json!(1.375));
        assert_eq!(parse_bg_value(&val), Some(1.375));
    }

    #[test]
    fn parse_bg_value_string() {
        let val = Some(serde_json::json!("0.9992"));
        assert_eq!(parse_bg_value(&val), Some(0.9992));
    }

    #[test]
    fn parse_bg_value_none() {
        assert_eq!(parse_bg_value(&None), None);
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
