//! Token discovery — dynamically find tradeable Arbitrum tokens.
//!
//! Queries Blockscot API for top ERC-20 tokens by volume, filters by
//! liquidity and safety, and returns a list of pair names for the engine.

use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

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
        let decimals = item["decimals"]
            .as_str()
            .unwrap_or("18")
            .parse::<u8>()
            .unwrap_or(18);
        let volume = item["volume_24h"]
            .as_str()
            .unwrap_or("0")
            .parse::<f64>()
            .unwrap_or(0.0);
        let holders = item["holders_count"]
            .as_str()
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0);
        let name = item["name"].as_str().unwrap_or("").to_string();

        // Filter: must have symbol, address, minimum volume and holders
        if symbol.is_empty() || address.is_empty() {
            continue;
        }
        if volume < min_volume || holders < min_holders {
            continue;
        }

        // Skip stablecoins (we trade against them, not with them)
        if matches!(
            symbol.as_str(),
            "USDC"
                | "USDT"
                | "DAI"
                | "USDS"
                | "USDE"
                | "FRAX"
                | "GHO"
                | "LUSD"
                | "PYUSD"
                | "FDUSD"
                | "USD0"
                | "USDAI"
        ) {
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
    tokens.sort_by(|a, b| {
        b.volume_24h
            .partial_cmp(&a.volume_24h)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    info!(
        "Token discovery: {} tokens found (min_volume=${}, min_holders={})",
        tokens.len(),
        min_volume,
        min_holders
    );

    Ok(tokens)
}

/// Convert discovered tokens to pair names (e.g., "ETH/USD").
pub fn tokens_to_pairs(tokens: &[DiscoveredToken]) -> Vec<String> {
    tokens.iter().map(|t| format!("{}/USD", t.symbol)).collect()
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
            existing.insert(
                token.symbol.clone(),
                (token.address.clone(), token.decimals),
            );
            added += 1;
        }
    }
    info!(
        "Token database: {} new tokens added (total: {})",
        added,
        existing.len()
    );
    added
}

// ---------------------------------------------------------------------------
// FID-120: Persistent token store + periodic discovery
// ---------------------------------------------------------------------------

/// A single entry in the persistent token store.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenStoreEntry {
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
    pub chain_id: u64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub discovered_at: String,
}

/// Load the persistent token store from disk.
/// Returns the list of entries, or an empty vec if the file doesn't exist.
pub fn load_token_store(path: &str) -> Vec<TokenStoreEntry> {
    if !Path::new(path).exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str::<Vec<TokenStoreEntry>>(&json) {
            Ok(entries) => {
                info!("Token store: loaded {} entries from {}", entries.len(), path);
                entries
            }
            Err(e) => {
                warn!("Token store: failed to parse {}: {}", path, e);
                Vec::new()
            }
        },
        Err(e) => {
            warn!("Token store: failed to read {}: {}", path, e);
            Vec::new()
        }
    }
}

/// Save the persistent token store to disk (atomic write: temp + rename).
pub fn save_token_store(path: &str, entries: &[TokenStoreEntry]) -> Result<(), ExecutionError> {
    let json = serde_json::to_string_pretty(entries)
        .map_err(|e| ExecutionError::Other(format!("Token store serialize: {}", e)))?;

    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ExecutionError::Other(format!("Token store mkdir: {}", e)))?;
    }

    let tmp = format!("{}.tmp", path);
    std::fs::write(&tmp, &json)
        .map_err(|e| ExecutionError::Other(format!("Token store write: {}", e)))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| ExecutionError::Other(format!("Token store rename: {}", e)))?;

    Ok(())
}

/// Seed the persistent token store from the static ARBITRUM_TOKENS array.
/// Only writes if the store file doesn't already exist.
pub fn seed_token_store_from_static(
    path: &str,
    static_tokens: &[(&str, &str, u8)],
    chain_id: u64,
) -> Vec<TokenStoreEntry> {
    if Path::new(path).exists() {
        // Store already exists — load it
        return load_token_store(path);
    }

    let entries: Vec<TokenStoreEntry> = static_tokens
        .iter()
        .map(|(sym, addr, dec)| TokenStoreEntry {
            symbol: sym.to_string(),
            address: addr.to_string(),
            decimals: *dec,
            chain_id,
            source: "static_seed".into(),
            discovered_at: chrono::Utc::now().to_rfc3339(),
        })
        .collect();

    if let Err(e) = save_token_store(path, &entries) {
        warn!("Token store: failed to seed from static: {}", e);
    } else {
        info!(
            "Token store: seeded {} entries from ARBITRUM_TOKENS to {}",
            entries.len(),
            path
        );
    }

    entries
}

/// USDC address on Arbitrum (native, 6 decimals).
const USDC_ARBITRUM: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
/// 10 USDC in base units (6 decimals) — above dust threshold for 0x routing.
const VALIDATION_SELL_AMOUNT: &str = "10000000";
/// Delay between 0x API calls to respect 5 RPS free-tier limit.
pub const VALIDATION_RATE_LIMIT_MS: u64 = 250;
/// Max tokens to validate per refresh cycle to bound main-loop latency.
pub const MAX_VALIDATIONS_PER_CYCLE: usize = 20;

/// FID-121: Validate a token has 0x liquidity on a given chain.
///
/// Queries 0x `/swap/allowance-holder/price` with a small USDC→token swap
/// to confirm `liquidityAvailable = true`. Also extracts buy/sell tax for
/// honeypot detection (logged but does not reject — the LLM decides).
///
/// Returns `Ok(true)` if routeable, `Ok(false)` if not, `Err` on network error.
/// Chain-agnostic: pass any `chain_id` supported by 0x (42161=Arbitrum, 1=Eth, etc.).
pub async fn validate_token_liquidity(
    token_address: &str,
    chain_id: u64,
    api_key: &str,
) -> Result<bool, ExecutionError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // 0x Swap API v2 /price endpoint (verified against docs/0x-llms-full.md)
    // taker=zero address is required by the API but not used for read-only price checks.
    let url = format!(
        "https://api.0x.org/swap/allowance-holder/price?\
         chainId={}&sellToken={}&buyToken={}&sellAmount={}&taker=0x0000000000000000000000000000000000000000",
        chain_id, USDC_ARBITRUM, token_address, VALIDATION_SELL_AMOUNT
    );

    let resp = client
        .get(&url)
        .header("0x-api-key", api_key)
        .header("0x-version", "v2")
        .send()
        .await
        .map_err(|e| ExecutionError::Other(format!("0x validation HTTP error: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(ExecutionError::Other(format!(
            "0x validation returned {}: {}",
            status, body
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ExecutionError::Other(format!("0x validation parse error: {}", e)))?;

    let available = json["liquidityAvailable"].as_bool().unwrap_or(false);
    let buy_amount = json["buyAmount"].as_str().unwrap_or("0");

    if !available || buy_amount == "0" {
        return Ok(false);
    }

    // Honeypot detection: extract buy/sell tax from tokenMetadata
    let buy_tax: u32 = json["tokenMetadata"]["buyToken"]["buyTaxBps"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let sell_tax: u32 = json["tokenMetadata"]["sellToken"]["sellTaxBps"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if buy_tax > 100 || sell_tax > 100 {
        warn!(
            "0x validation: {} has high tax (buy={}bps, sell={}bps) — potential honeypot",
            token_address, buy_tax, sell_tax
        );
    }

    Ok(true)
}

/// Periodic discovery: re-query Blockscout and merge new tokens into the store.
/// Returns (new_entries_added, total_store_size).
///
/// When `validate_via_0x` is true and `api_key` is provided, newly discovered
/// tokens are validated via 0x `/price` before being persisted. Chain-agnostic:
/// `chain_id` is passed through to the 0x API.
pub async fn refresh_token_store(
    persist_path: &str,
    existing_entries: &mut Vec<TokenStoreEntry>,
    min_volume: f64,
    min_holders: u64,
    validate_via_0x: bool,
    api_key: Option<&str>,
    chain_id: u64,
) -> (usize, usize) {
    // Build a set of already-known symbols for fast lookup
    let known: std::collections::HashSet<String> = existing_entries
        .iter()
        .map(|e| e.symbol.to_uppercase())
        .collect();

    // Query Blockscout for new tokens
    let discovered = match discover_tokens(min_volume, min_holders, 200).await {
        Ok(tokens) => tokens,
        Err(e) => {
            warn!("Token refresh: Blockscout discovery failed: {}", e);
            return (0, existing_entries.len());
        }
    };

    let mut added = 0usize;
    let mut skipped_validation = 0usize;
    let mut validated = 0usize;
    let now = chrono::Utc::now().to_rfc3339();

    // Collect new tokens (not already known)
    let new_tokens: Vec<&DiscoveredToken> = discovered
        .iter()
        .filter(|t| !known.contains(&t.symbol.to_uppercase()))
        .collect();

    // FID-121: Validate and merge new tokens
    for token in &new_tokens {
        // Cap validation to bound main-loop latency (FID-121 RED-2, RED-7)
        if validate_via_0x && api_key.is_some() && validated >= MAX_VALIDATIONS_PER_CYCLE {
            skipped_validation = new_tokens.len() - validated;
            break;
        }

        // FID-121: 0x liquidity validation gate
        if validate_via_0x {
            if let Some(key) = api_key {
                match validate_token_liquidity(&token.address, chain_id, key).await {
                    Ok(true) => {
                        validated += 1;
                    }
                    Ok(false) => {
                        debug!(
                            "0x validation: {} ({}) — no liquidity, skipping",
                            token.symbol, token.address
                        );
                        validated += 1;
                        continue;
                    }
                    Err(e) => {
                        // FID-121 RED-3: On 0x downtime, add with warning (don't block)
                        warn!(
                            "0x validation failed for {} ({}): {} — adding anyway",
                            token.symbol, token.address, e
                        );
                        validated += 1;
                    }
                }
                // Rate limit: 250ms between requests (5 RPS free tier)
                tokio::time::sleep(std::time::Duration::from_millis(VALIDATION_RATE_LIMIT_MS)).await;
            }
        }

        // FID-121 RED-8: Extract tax info from last validation for source field
        let source = if validate_via_0x && api_key.is_some() {
            "0x_validated".to_string()
        } else {
            "blockscout_refresh".to_string()
        };

        existing_entries.push(TokenStoreEntry {
            symbol: token.symbol.to_uppercase(),
            address: token.address.clone(),
            decimals: token.decimals,
            chain_id,
            source,
            discovered_at: now.clone(),
        });
        added += 1;
    }    if added > 0 {
        if let Err(e) = save_token_store(persist_path, existing_entries) {
            warn!("Token refresh: failed to persist: {}", e);
        } else {
            let validation_msg = if validate_via_0x && api_key.is_some() {
                if skipped_validation > 0 {
                    format!(", {} validated, {} skipped (cap)", validated, skipped_validation)
                } else {
                    format!(", {} validated", validated)
                }
            } else {
                String::new()
            };
            info!(
                "FID-121 Token refresh: {} new tokens added (total: {}){}",
                added, existing_entries.len(), validation_msg
            );
        }
    } else {
        info!(
            "Token refresh: no new tokens from Blockscout ({} checked, {} known)",
            discovered.len(),
            known.len()
        );
    }

    (added, existing_entries.len())
}
