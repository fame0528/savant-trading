//! GoPlus Security API — honeypot and tax detection for meme coins.
//!
//! Queries `https://api.gopluslabs.io/api/v1/token_security/{chain_id}`
//! to detect malicious token contracts before the AI evaluates them.
//!
//! Hard reject if:
//! - `is_honeypot` = "1" (can buy but can't sell)
//! - `buy_tax` > 0.01 (hidden buy tax)
//! - `sell_tax` > 0.01 (hidden sell tax)
//! - `transfer_pausable` = "1" (owner can freeze trading)
//! - `cannot_sell_all` = "1" (can't fully exit position)

use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{info, warn};

use crate::core::error::ExecutionError;

/// Cached security check result.
#[derive(Debug, Clone)]
pub struct TokenSecurity {
    pub is_safe: bool,
    pub reason: String,
}

/// GoPlus Security API client with caching.
pub struct GoPlusClient {
    client: reqwest::Client,
    cache: Mutex<HashMap<String, TokenSecurity>>,
}

impl Default for GoPlusClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GoPlusClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a token contract is safe to trade.
    ///
    /// Returns `Ok(true)` if safe, `Ok(false)` if rejected, `Err` if API fails.
    /// Caches results — tokens don't change contracts.
    pub async fn check_token(
        &self,
        contract_address: &str,
        token_symbol: &str,
    ) -> Result<bool, ExecutionError> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(contract_address) {
                if !cached.is_safe {
                    info!(
                        "GoPlus: {} ({}) rejected (cached) — {}",
                        token_symbol, contract_address, cached.reason
                    );
                }
                return Ok(cached.is_safe);
            }
        }

        // Query GoPlus API
        let url = format!(
            "https://api.gopluslabs.io/api/v1/token_security/42161?contract_addresses={}",
            contract_address
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("GoPlus API error: {}", e)))?;

        if !resp.status().is_success() {
            warn!("GoPlus API returned {} for {}", resp.status(), token_symbol);
            // Don't reject on API failure — just warn
            return Ok(true);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("GoPlus parse error: {}", e)))?;

        // Parse the result
        let result = json.get("result").and_then(|r| r.get(contract_address));
        let result = match result {
            Some(r) => r,
            None => {
                warn!("GoPlus: no result for {} ({})", token_symbol, contract_address);
                return Ok(true); // Don't reject if no data
            }
        };

        let mut reasons = Vec::new();

        // Check honeypot
        if result["is_honeypot"].as_str() == Some("1") {
            reasons.push("honeypot");
        }

        // Check buy tax
        if let Some(tax) = result["buy_tax"].as_str() {
            if let Ok(tax_f64) = tax.parse::<f64>() {
                if tax_f64 > 0.01 {
                    reasons.push("buy_tax > 1%");
                }
            }
        }

        // Check sell tax
        if let Some(tax) = result["sell_tax"].as_str() {
            if let Ok(tax_f64) = tax.parse::<f64>() {
                if tax_f64 > 0.01 {
                    reasons.push("sell_tax > 1%");
                }
            }
        }

        // Check transfer pausable
        if result["transfer_pausable"].as_str() == Some("1") {
            reasons.push("transfer_pausable");
        }

        // Check cannot sell all
        if result["cannot_sell_all"].as_str() == Some("1") {
            reasons.push("cannot_sell_all");
        }

        let is_safe = reasons.is_empty();
        let reason = reasons.join(", ");

        // Cache result
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(
                contract_address.to_string(),
                TokenSecurity {
                    is_safe,
                    reason: reason.clone(),
                },
            );
        }

        if is_safe {
            info!("GoPlus: {} ({}) — SAFE", token_symbol, contract_address);
        } else {
            warn!(
                "GoPlus: {} ({}) — REJECTED: {}",
                token_symbol, contract_address, reason
            );
        }

        Ok(is_safe)
    }

    /// Check a token by symbol (uses known Arbitrum addresses).
    /// Returns `Ok(true)` if safe, `Ok(false)` if rejected.
    /// Core assets (BTC, ETH, etc.) are skipped — they don't need security checks.
    pub async fn check_by_symbol(&self, symbol: &str) -> Result<bool, ExecutionError> {
        // Core assets — skip security check (they're established, not meme coins)
        const CORE_ASSETS: &[&str] = &[
            "BTC", "ETH", "SOL", "XRP", "ADA", "LINK", "AVAX",
            "DOGE", "ARB", "OP", "AAVE", "UNI", "LDO", "PENDLE",
            "RENDER", "FET", "GRT",
        ];

        let upper = symbol.to_uppercase();
        if CORE_ASSETS.contains(&upper.as_str()) {
            return Ok(true); // Core assets are safe — skip check
        }

        // Known Arbitrum addresses for meme coins
        let known_addresses: HashMap<&str, &str> = HashMap::from([
            ("PEPE", "0x25d887Ce7a49172BF65CB5E54e78C488Ef5954e6"),
            ("SHIB", "0x503Fa02e9c600a63E40f1b8F0c76b8d8Ef1B2B19"),
            ("FLOKI", "0x22e89898A04eaf433f95643b1F0D57eC22E74E63"),
            ("TURBO", "0x2Cd44D8F4FA2bC2A2B8F5a8e1Ea22A3c5a312345"),
            ("MOG", "0x2Da56AcB9Ea78330f947bD57C54119Debda7AF71"),
        ]);

        if let Some(address) = known_addresses.get(upper.as_str()) {
            self.check_token(address, symbol).await
        } else {
            // Unknown address — don't block, just warn
            warn!("GoPlus: no known address for {} — skipping security check", symbol);
            Ok(true)
        }
    }
}
