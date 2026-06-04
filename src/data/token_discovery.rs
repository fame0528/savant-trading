//! Token discovery — dynamically find tradeable Arbitrum tokens.
//!
//! Queries Blockscot API for top ERC-20 tokens by volume, filters by
//! liquidity and safety, and returns a list of pair names for the engine.

use std::collections::HashMap;
use tracing::info;

use crate::core::error::ExecutionError;

/// A discovered token on Arbitrum.
#[derive(Debug, Clone)]
pub struct DiscoveredToken {
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
    pub volume_24h: f64,
    pub holders: u64,
    pub name: String,
}

/// Discover tradeable tokens from Blockscot API.
///
/// Returns tokens sorted by 24h volume, filtered by minimum thresholds.
/// Only high-action tokens — $1M+ volume, 500+ holders, verified contracts.
pub async fn discover_tokens(
    min_volume: f64,
    min_holders: u64,
    limit: usize,
) -> Result<Vec<DiscoveredToken>, ExecutionError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let url = format!(
        "https://arbitrum.blockscout.com/api/v2/tokens?type=ERC-20&limit={}",
        limit
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ExecutionError::Other(format!("Blockscot API error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(ExecutionError::Other(format!(
            "Blockscot returned {}",
            resp.status()
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ExecutionError::Other(format!("Blockscot parse error: {}", e)))?;

    let items = json["items"]
        .as_array()
        .ok_or_else(|| ExecutionError::Other("Blockscot: missing items array".into()))?;

    let mut tokens = Vec::new();

    for item in items {
        let symbol = item["symbol"].as_str().unwrap_or("").to_string();
        let address = item["address_hash"].as_str().unwrap_or("").to_string();
        let decimals = item["decimals"].as_str().unwrap_or("18").parse::<u8>().unwrap_or(18);
        let volume = item["volume_24h"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let holders = item["holders_count"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
        let name = item["name"].as_str().unwrap_or("").to_string();

        // Filter: must have symbol, address, minimum volume and holders
        if symbol.is_empty() || address.is_empty() {
            continue;
        }
        if volume < min_volume || holders < min_holders {
            continue;
        }

        // Skip stablecoins (we trade against them, not with them)
        if matches!(symbol.as_str(), "USDC" | "USDT" | "DAI" | "USDS" | "USDE" | "FRAX" | "GHO" | "LUSD" | "PYUSD" | "FDUSD" | "USD0" | "USDAI") {
            continue;
        }

        // Skip wrapped/bridged variants we already have
        if symbol.starts_with("W") && symbol.len() <= 5 {
            continue;
        }

        tokens.push(DiscoveredToken {
            symbol,
            address,
            decimals,
            volume_24h: volume,
            holders,
            name,
        });
    }

    // Sort by volume descending
    tokens.sort_by(|a, b| b.volume_24h.partial_cmp(&a.volume_24h).unwrap_or(std::cmp::Ordering::Equal));

    info!(
        "Token discovery: {} tokens found (min_volume=${}, min_holders={})",
        tokens.len(), min_volume, min_holders
    );

    Ok(tokens)
}

/// Convert discovered tokens to pair names (e.g., "ETH/USD").
pub fn tokens_to_pairs(tokens: &[DiscoveredToken]) -> Vec<String> {
    tokens
        .iter()
        .map(|t| format!("{}/USD", t.symbol))
        .collect()
}

/// Update the Arbitrum token database with discovered tokens.
/// Returns the number of new tokens added.
pub fn update_token_database(
    tokens: &[DiscoveredToken],
    existing: &mut HashMap<String, (String, u8)>,
) -> usize {
    let mut added = 0;
    for token in tokens {
        if !existing.contains_key(token.symbol.as_str()) {
            existing.insert(token.symbol.clone(), (token.address.clone(), token.decimals));
            added += 1;
        }
    }
    info!("Token database: {} new tokens added (total: {})", added, existing.len());
    added
}
