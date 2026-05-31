//! Exchange inflow/outflow data — indicates buying/selling pressure.
//!
//! Uses CryptoQuant API (requires API key) with graceful fallback.
//! Also provides blockchain.info as a free fallback for basic on-chain data.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Exchange flow data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowData {
    /// BTC flowing to exchanges (bearish — preparing to sell)
    pub btc_exchange_inflow: Option<f64>,
    /// BTC flowing out of exchanges (bullish — holding)
    pub btc_exchange_outflow: Option<f64>,
    /// Net flow (negative = outflow = bullish)
    pub net_flow: Option<f64>,
    /// Stablecoin inflow to exchanges (buying pressure)
    pub stablecoin_inflow: Option<f64>,
    /// Current block height (from blockchain.info — free)
    pub block_height: Option<u64>,
    /// Mempool size (transactions waiting to be confirmed)
    pub mempool_size: Option<u64>,
    /// 24h transaction count
    pub transactions_24h: Option<u64>,
}

/// Fetch exchange flow data.
///
/// Uses CryptoQuant if API key is available, falls back to blockchain.info (free).
pub async fn fetch_flows(client: &reqwest::Client, api_key: Option<&str>) -> FlowData {
    let mut data = FlowData::default();

    // Always try blockchain.info (free, no key needed)
    fetch_blockchain_info(client, &mut data).await;

    // Try CryptoQuant if key is available
    if let Some(key) = api_key {
        if !key.is_empty() {
            fetch_cryptoquant(client, key, &mut data).await;
        }
    } else {
        debug!("CryptoQuant API key not configured — using blockchain.info only");
    }

    data
}

/// Fetch basic on-chain data from blockchain.info (free, no key).
async fn fetch_blockchain_info(client: &reqwest::Client, data: &mut FlowData) {
    // Block height
    match client
        .get("https://blockchain.info/q/getblockcount")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                if let Ok(height) = text.trim().parse::<u64>() {
                    data.block_height = Some(height);
                    debug!("Blockchain.info block height: {}", height);
                }
            }
        }
        Err(e) => warn!("blockchain.info block height request failed: {}", e),
    }

    // Unconfirmed transactions (mempool)
    match client
        .get("https://blockchain.info/q/unconfirmedcount")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                if let Ok(count) = text.trim().parse::<u64>() {
                    data.mempool_size = Some(count);
                    debug!("Blockchain.info mempool size: {}", count);
                }
            }
        }
        Err(e) => warn!("blockchain.info mempool request failed: {}", e),
    }

    // 24h transaction count
    match client
        .get("https://blockchain.info/q/24hrtransactioncount")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                if let Ok(count) = text.trim().parse::<u64>() {
                    data.transactions_24h = Some(count);
                    debug!("Blockchain.info 24h tx count: {}", count);
                }
            }
        }
        Err(e) => warn!("blockchain.info 24h tx count request failed: {}", e),
    }
}

/// Fetch exchange flow data from CryptoQuant (requires API key).
async fn fetch_cryptoquant(client: &reqwest::Client, api_key: &str, data: &mut FlowData) {
    let url = "https://api.cryptoquant.com/v1/btc/exchange-flows/inflow?window=day&limit=1";

    match client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("CryptoQuant API returned HTTP {}", resp.status());
                return;
            }

            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(result) = json.get("result").and_then(|r| r.get("data")) {
                    if let Some(items) = result.as_array() {
                        if let Some(latest) = items.first() {
                            data.btc_exchange_inflow =
                                latest.get("inflow").and_then(|v| v.as_f64());
                            data.btc_exchange_outflow =
                                latest.get("outflow").and_then(|v| v.as_f64());

                            let inflow = data.btc_exchange_inflow.unwrap_or(0.0);
                            let outflow = data.btc_exchange_outflow.unwrap_or(0.0);
                            data.net_flow = Some(inflow - outflow);

                            info!(
                                "CryptoQuant flows: inflow={:.2} BTC, outflow={:.2} BTC, net={:.2} BTC",
                                inflow, outflow, data.net_flow.unwrap_or(0.0)
                            );
                        }
                    }
                }
            }
        }
        Err(e) => warn!("CryptoQuant API request failed: {}", e),
    }
}
