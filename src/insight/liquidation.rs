//! Liquidation data — derived from Kraken Futures open interest and price levels.
//!
//! Kraken Futures doesn't provide direct liquidation heatmap data (that's CoinGlass).
//! However, we can derive useful metrics from the futures data:
//! - Open interest concentration = where liquidations would cluster
//! - Mark/index spread = premium/discount indicates positioning
//! - Funding rate extremes = liquidation cascade risk
//!
//! For true liquidation heatmaps, CoinGlass API key would be needed.
//! This module provides what's available for free.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Liquidation risk data derived from futures market data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiquidationData {
    /// Estimated long liquidation cluster price (below current price)
    /// Derived from OI concentration and typical leverage levels
    pub long_liquidation_cluster: Option<f64>,
    /// Estimated short liquidation cluster price (above current price)
    pub short_liquidation_cluster: Option<f64>,
    /// Open interest in contracts (high OI = more liquidation risk)
    pub open_interest: Option<f64>,
    /// Mark price (futures reference price)
    pub mark_price: Option<f64>,
    /// Index price (spot reference)
    pub index_price: Option<f64>,
    /// Mark-index spread percentage (positive = futures premium = bullish positioning)
    pub mark_index_spread_pct: Option<f64>,
    /// Funding rate (extreme values indicate liquidation cascade risk)
    pub funding_rate: Option<f64>,
    /// Liquidation risk level: Low, Medium, High, Extreme
    pub risk_level: LiquidationRisk,
    /// 24h volume (high volume + high OI = more liquidation events)
    pub volume_24h: Option<f64>,
}

/// Liquidation risk assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LiquidationRisk {
    #[default]
    Low,
    Medium,
    High,
    Extreme,
}

/// Derive liquidation data from Kraken Futures tickers.
///
/// Uses the same API as funding_rates (single call, shared data).
/// Estimates liquidation clusters based on:
/// - Mark/index spread (positioning indicator)
/// - Funding rate extremes (cascade risk)
/// - Open interest (total exposure)
pub async fn fetch_liquidation(client: &reqwest::Client, symbol: &str) -> LiquidationData {
    // Fetch all tickers
    match client
        .get("https://futures.kraken.com/derivatives/api/v3/tickers")
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("Kraken Futures API returned HTTP {}", resp.status());
                return LiquidationData::default();
            }

            match resp.json::<serde_json::Value>().await {
                Ok(json) => parse_liquidation_from_futures(&json, symbol),
                Err(e) => {
                    warn!("Kraken Futures parse error: {}", e);
                    LiquidationData::default()
                }
            }
        }
        Err(e) => {
            warn!("Kraken Futures request failed: {}", e);
            LiquidationData::default()
        }
    }
}

/// Parse liquidation data from Kraken Futures JSON response.
fn parse_liquidation_from_futures(json: &serde_json::Value, symbol: &str) -> LiquidationData {
    let tickers = match json.get("tickers").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return LiquidationData::default(),
    };

    let base = symbol.split('/').next().unwrap_or(symbol).to_uppercase();
    let futures_base = if base == "BTC" { "XBT" } else { &base };
    let expected_pair = format!("{}:USD", base);
    let expected_symbol = format!("PF_{}USD", futures_base);

    for ticker in tickers {
        let pair = ticker.get("pair").and_then(|p| p.as_str()).unwrap_or("");
        let sym = ticker.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
        let tag = ticker.get("tag").and_then(|t| t.as_str()).unwrap_or("");
        let suspended = ticker
            .get("suspended")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        let pair_match = pair.eq_ignore_ascii_case(&expected_pair);
        let symbol_match = sym.eq_ignore_ascii_case(&expected_symbol);

        if (pair_match || symbol_match) && tag == "perpetual" && !suspended {
            let mark_price = ticker.get("markPrice").and_then(|v| v.as_f64());
            let index_price = ticker.get("indexPrice").and_then(|v| v.as_f64());
            let funding_rate = ticker.get("fundingRate").and_then(|v| v.as_f64());
            let open_interest = ticker.get("openInterest").and_then(|v| v.as_f64());
            let volume_24h = ticker.get("vol24h").and_then(|v| v.as_f64());

            // Calculate mark-index spread
            let spread_pct = match (mark_price, index_price) {
                (Some(mark), Some(idx)) if idx > 0.0 => Some((mark - idx) / idx * 100.0),
                _ => None,
            };

            // Estimate liquidation clusters
            // Long liquidations cluster ~5-10% below current price (typical leverage)
            // Short liquidations cluster ~5-10% above current price
            let (long_cluster, short_cluster) = match (mark_price, funding_rate) {
                (Some(price), Some(fr)) => {
                    // Higher funding = more longs = long liquidation cluster closer
                    // Lower/negative funding = more shorts = short liquidation cluster closer
                    let long_distance = if fr > 0.0005 {
                        0.03
                    } else if fr > 0.0 {
                        0.05
                    } else {
                        0.08
                    };
                    let short_distance = if fr < -0.0005 {
                        0.03
                    } else if fr < 0.0 {
                        0.05
                    } else {
                        0.08
                    };
                    (
                        Some(price * (1.0 - long_distance)),
                        Some(price * (1.0 + short_distance)),
                    )
                }
                _ => (None, None),
            };

            // Assess risk level based on funding rate and OI
            let risk_level = assess_liquidation_risk(funding_rate, open_interest, spread_pct);

            let data = LiquidationData {
                long_liquidation_cluster: long_cluster,
                short_liquidation_cluster: short_cluster,
                open_interest,
                mark_price,
                index_price,
                mark_index_spread_pct: spread_pct,
                funding_rate,
                risk_level,
                volume_24h,
            };

            info!(
                "Liquidation {}: risk={:?}, spread={:.4}%, OI={:.0}, long_cluster=${:.2}, short_cluster=${:.2}",
                symbol,
                data.risk_level,
                spread_pct.unwrap_or(0.0),
                open_interest.unwrap_or(0.0),
                long_cluster.unwrap_or(0.0),
                short_cluster.unwrap_or(0.0),
            );

            return data;
        }
    }

    debug!("No perpetual ticker found for {} in Kraken Futures", symbol);
    LiquidationData::default()
}

/// Assess liquidation risk from funding rate, OI, and spread.
fn assess_liquidation_risk(
    funding_rate: Option<f64>,
    open_interest: Option<f64>,
    spread_pct: Option<f64>,
) -> LiquidationRisk {
    let fr = funding_rate.unwrap_or(0.0).abs();
    let _oi = open_interest.unwrap_or(0.0);
    let spread = spread_pct.unwrap_or(0.0).abs();

    // Extreme funding rate = high liquidation risk
    if fr > 0.001 || spread > 0.5 {
        return LiquidationRisk::Extreme;
    }

    if fr > 0.0005 || spread > 0.2 {
        return LiquidationRisk::High;
    }

    if fr > 0.0001 || spread > 0.1 {
        return LiquidationRisk::Medium;
    }

    LiquidationRisk::Low
}
