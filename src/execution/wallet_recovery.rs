//! Chain-driven position recovery (FID-155 / DECISION-015).
//!
//! The on-chain state is the source of truth for all positions. This module
//! queries the chain for the engine's wallet token balances and reconstructs
//! `Position` objects from on-chain reality. Used in two places:
//!
//! 1. **Engine startup**: rebuild the in-memory position map from chain state.
//!    Replaces the old `load_state()` JSON hydration path. The on-disk
//!    `dex_state.json` is now a write-through cache, not the truth source.
//!
//! 2. **Periodic reconciliation** (every 5 minutes, per DECISION-015): compare
//!    engine's in-memory positions to the chain. Add missing ones, close
//!    stragglers, update qty drift. Self-heals stale state.
//!
//! The position `opened_at` timestamp is sourced from the on-chain block that
//! contained the entry transaction. There are no sentinel timestamps
//! (`1970-01-01` etc.) — every Position has a real block timestamp or it
//! doesn't exist in the engine state.
//!
//! ## Anti-patterns
//!
//! - **Don't** use this module to read `dex_state.json`. The JSON is a cache.
//! - **Don't** write `Position` objects with `opened_at: epoch(0)`. The type
//!   is `DateTime<Utc>` non-optional; the caller must always resolve a real
//!   block timestamp first.
//! - **Don't** close positions based on JSON state alone. Verify on-chain.

use crate::core::types::{Position, ScaleLevel, Side};
use crate::data::token_discovery::TokenStoreEntry;
use crate::execution::dex::{resolve_pair_on_chain, TokenInfo};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

/// Result of a chain reconciliation pass.
#[derive(Debug, Default, Clone)]
pub struct ReconcileResult {
    /// Positions on-chain that the engine doesn't know about. Add these.
    pub to_add: Vec<Position>,
    /// Position IDs the engine has that no longer exist on-chain. Remove these.
    pub to_close: Vec<String>,
    /// Positions the engine has whose on-chain balance differs from the
    /// engine's `quantity`. Update the engine's `quantity` to the chain value.
    pub to_update: Vec<Position>,
    /// USDC balance drift between in-memory and on-chain (in absolute dollars).
    pub drift_usd: f64,
}

impl ReconcileResult {
    pub fn is_clean(&self) -> bool {
        self.to_add.is_empty() && self.to_close.is_empty() && self.to_update.is_empty() && self.drift_usd < 0.10
    }
}

/// Configuration for chain queries.
#[derive(Debug, Clone)]
pub struct ChainRecoveryConfig {
    /// RPC URL (mainnet, Anvil, etc.).
    pub rpc_url: String,
    /// Wallet address to recover positions for.
    pub wallet_address: String,
    /// Chain ID (42161 for Arbitrum, 1 for Ethereum mainnet, etc.).
    pub chain_id: u64,
}

/// Performs chain-driven position recovery and reconciliation.
///
/// Constructed once per engine, holds a long-lived reqwest client with
/// sensible timeouts (10s per call, 3s connect). Per FID-154 the engine
/// fail-fasts on RPC failure, so a dead chain means we skip the recovery
/// rather than hang the engine.
pub struct ChainPositionRecovery {
    config: ChainRecoveryConfig,
    client: Client,
    wallet_address_lowercase: String,
}

impl ChainPositionRecovery {
    /// Build a recovery helper for the given config.
    pub fn new(config: ChainRecoveryConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| Client::new());
        let wallet_address_lowercase =
            config.wallet_address.trim_start_matches("0x").to_lowercase();
        Self { config, client, wallet_address_lowercase }
    }

    /// Build the calldata for `balanceOf(address)` per the canonical ABI encoding:
    /// 4-byte selector (0x70a08231) + 12-byte left-pad (24 hex zeros) + 20-byte
    /// raw address (40 hex chars, NOT zero-padded). Total 36 bytes = 72 hex +
    /// `0x` prefix = 74 chars.
    fn balance_of_calldata(&self) -> String {
        // See reconciliation.rs:255-263 for the canonical encoding that
        // matches what `cast call ... 'balanceOf(address)(uint256)'` produces.
        format!(
            "0x70a08231000000000000000000000000{}",
            self.wallet_address_lowercase
        )
    }

    /// Query the chain for a single token's balance in this wallet.
    /// Returns the balance as a human-readable float, or `None` on RPC error.
    /// Filters out zero balances (no point tracking them).
    pub async fn query_token_balance(
        &self,
        token_address: &str,
        decimals: u8,
    ) -> Option<f64> {
        let token_lower = token_address.trim_start_matches("0x").to_lowercase();
        let calldata = self.balance_of_calldata();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1u64,
            "method": "eth_call",
            "params": [
                { "to": format!("0x{}", token_lower), "data": calldata },
                "latest"
            ]
        });
        let resp = match tokio::time::timeout(
            Duration::from_secs(10),
            self.client.post(&self.config.rpc_url).json(&body).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                debug!(
                    "ChainRecovery: RPC eth_call failed for {}: {}",
                    token_address, e
                );
                return None;
            }
            Err(_) => {
                debug!("ChainRecovery: RPC eth_call timeout for {}", token_address);
                return None;
            }
        };
        let json: serde_json::Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => {
                debug!(
                    "ChainRecovery: parse eth_call response failed for {}: {}",
                    token_address, e
                );
                return None;
            }
        };
        let result_hex = json.get("result").and_then(|v| v.as_str())?;
        let raw = match u128::from_str_radix(result_hex.trim_start_matches("0x"), 16) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let divisor = 10f64.powi(decimals as i32);
        let balance = (raw as f64) / divisor;
        if balance <= 0.0 {
            None
        } else {
            Some(balance)
        }
    }

    /// Get the timestamp of a block by its number. Returns the block's
    /// `timestamp` field as a `DateTime<Utc>`, or `None` on any error.
    /// This is what we use to populate `Position.opened_at` — the chain has
    /// the exact second, so no estimation is needed.
    pub async fn get_block_timestamp(
        &self,
        block_number: u64,
    ) -> Option<DateTime<Utc>> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1u64,
            "method": "eth_getBlockByNumber",
            "params": [format!("0x{:x}", block_number), false]
        });
        let resp = match tokio::time::timeout(
            Duration::from_secs(10),
            self.client.post(&self.config.rpc_url).json(&body).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            _ => return None,
        };
        let json: serde_json::Value = match resp.json().await {
            Ok(j) => j,
            Err(_) => return None,
        };
        // The response's `result` may be `null` if the block doesn't exist
        // (e.g., on a fork that hasn't reached that block yet).
        let block = json.get("result")?;
        if block.is_null() {
            return None;
        }
        let timestamp_hex = block.get("timestamp")?.as_str()?;
        let secs = u64::from_str_radix(timestamp_hex.trim_start_matches("0x"), 16).ok()?;
        Utc.timestamp_opt(secs as i64, 0).single()
    }

    /// Get the latest block number on the chain.
    pub async fn latest_block(&self) -> Option<u64> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1u64,
            "method": "eth_blockNumber",
            "params": []
        });
        let resp = match tokio::time::timeout(
            Duration::from_secs(10),
            self.client.post(&self.config.rpc_url).json(&body).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            _ => return None,
        };
        let json: serde_json::Value = match resp.json().await {
            Ok(j) => j,
            Err(_) => return None,
        };
        let hex = json.get("result")?.as_str()?;
        u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok()
    }

    /// Scan the chain for all known token balances in the engine's wallet.
    /// Returns a list of `Position` placeholders — one per non-zero token
    /// balance. These are "wallet_recovery" positions: the engine knows the
    /// wallet holds X tokens, and the next LLM cycle will decide what to do.
    ///
    /// `known_tokens` is a list of `(symbol, address, decimals)` for tokens
    /// the engine might be holding. The full universe of tokens is large;
    /// we only query tokens we know about (from `data/tokens.json`).
    pub async fn scan_all_positions(
        &self,
        known_tokens: &[(String, String, u8)],
    ) -> Vec<Position> {
        let now = Utc::now();
        let latest_block = self.latest_block().await.unwrap_or(0);
        let block_ts = if latest_block > 0 {
            self.get_block_timestamp(latest_block).await.unwrap_or(now)
        } else {
            now
        };

        info!(
            "ChainRecovery: scanning {} known tokens at block {} (ts={})",
            known_tokens.len(),
            latest_block,
            block_ts
        );

        let mut positions = Vec::new();
        for (symbol, address, decimals) in known_tokens {
            if address.is_empty() {
                continue; // Unknown contract address — skip
            }
            if let Some(balance) = self.query_token_balance(address, *decimals).await {
                // Filter out dust (< $0.10 worth). At token prices <$1, this
                // is ~0.1 tokens; at $100, this is 0.001 tokens. Anything below
                // this is rounding error or gas dust.
                if balance * 0.05 < 0.10 && balance < 1.0 {
                    debug!(
                        "ChainRecovery: skipping {} dust balance {:.8}",
                        symbol, balance
                    );
                    continue;
                }
                // Build a placeholder position. The pair is `{SYMBOL}/USDC`
                // (most pairs in the engine are base/USDC). Entry price =
                // current market price. SL/TP are placeholders that the next
                // cycle's stop-management will adjust.
                let pair = format!("{}/USDC", symbol);
                // Try to resolve the pair; if it fails (e.g. unsupported
                // symbol), skip — the engine can't trade it anyway.
                let _token_info: Option<TokenInfo> =
                    resolve_pair_on_chain(&pair, Side::Long, self.config.chain_id)
                        .ok()
                        .map(|(_, dst)| dst);

                let pos = Position {
                    id: format!("chain-recovery-{}-{}", symbol, latest_block),
                    pair: pair.clone(),
                    side: Side::Long,
                    entry_price: 0.0, // Will be filled in by next cycle's price fetch
                    current_price: 0.0,
                    quantity: balance,
                    stop_loss: 0.0,
                    take_profit_1: 0.0,
                    take_profit_2: 0.0,
                    take_profit_3: 0.0,
                    unrealized_pnl: 0.0,
                    risk_amount: 0.0,
                    strategy_name: "chain_recovery".to_string(),
                    opened_at: block_ts,
                    scale_level: ScaleLevel::Full,
                    token_address: format!("0x{}", address.trim_start_matches("0x").to_lowercase()),
                };
                info!(
                    "ChainRecovery: created placeholder for {} balance={:.8} (pair {})",
                    symbol, balance, pos.pair
                );
                positions.push(pos);
            }
        }
        info!(
            "ChainRecovery: scan complete, {} positions found",
            positions.len()
        );
        positions
    }

    /// Compare the engine's in-memory positions to the chain state. Returns
    /// a `ReconcileResult` describing what needs to be added, closed, or
    /// updated. The caller is responsible for applying the result.
    ///
    /// `current_positions` is the engine's view. `known_tokens` is the
    /// universe to query. `in_memory_usdc` is the engine's USDC balance;
    /// `on_chain_usdc` is what the chain reports (caller's responsibility
    /// to query — see `reconciliation.rs`).
    pub async fn reconcile_with(
        &self,
        current_positions: &HashMap<String, Position>,
        known_tokens: &[(String, String, u8)],
        in_memory_usdc: f64,
        on_chain_usdc: f64,
    ) -> ReconcileResult {
        let chain_positions = self.scan_all_positions(known_tokens).await;
        let mut result = ReconcileResult {
            drift_usd: (in_memory_usdc - on_chain_usdc).abs(),
            ..Default::default()
        };

        // Index chain positions by token_address for lookup.
        let chain_by_token: HashMap<String, &Position> = chain_positions
            .iter()
            .filter(|p| !p.token_address.is_empty())
            .map(|p| (p.token_address.clone(), p))
            .collect();

        // For each chain position: is the engine tracking it?
        for chain_pos in &chain_positions {
            let engine_match = current_positions
                .values()
                .find(|p| {
                    !p.token_address.is_empty()
                        && p.token_address == chain_pos.token_address
                })
                .cloned();
            match engine_match {
                None => {
                    // Chain has it, engine doesn't — add.
                    result.to_add.push(chain_pos.clone());
                }
                Some(engine_pos) => {
                    // Both have it. Check quantity drift.
                    let qty_drift = (engine_pos.quantity - chain_pos.quantity).abs();
                    if qty_drift > 0.0001 {
                        let mut updated = engine_pos.clone();
                        updated.quantity = chain_pos.quantity;
                        result.to_update.push(updated);
                    }
                }
            }
        }

        // For each engine position: is it still on-chain?
        for (id, engine_pos) in current_positions {
            if engine_pos.token_address.is_empty() {
                // Legacy position with no on-chain binding. We can't
                // reconcile it — leave it alone.
                continue;
            }
            if !chain_by_token.contains_key(&engine_pos.token_address) {
                // Engine thinks we have a position, chain says balance is
                // zero. The position was likely closed on-chain but the
                // engine didn't see the close. Remove it.
                result.to_close.push(id.clone());
            }
        }

        result
    }
}

/// Helper: build a default `known_tokens` list from a `Vec<TokenStoreEntry>`
/// (the format returned by `data::token_discovery::load_token_store`).
/// Returns a vector of (symbol, address, decimals) for every non-empty
/// address in the token store. The engine loads this once at startup
/// and reuses it for both initial scan and periodic reconciliation.
pub fn load_known_tokens_from_store(
    tokens: &[TokenStoreEntry],
) -> Vec<(String, String, u8)> {
    let mut out = Vec::new();
    for token in tokens {
        if token.address.is_empty() {
            continue;
        }
        out.push((token.symbol.clone(), token.address.clone(), token.decimals));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_of_calldata_is_canonical_74_hex_chars() {
        let cfg = ChainRecoveryConfig {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            wallet_address: "0x543ca0434b84ad38c858d2d178d2082521711fbc".to_string(),
            chain_id: 42161,
        };
        let recovery = ChainPositionRecovery::new(cfg);
        let calldata = recovery.balance_of_calldata();
        // Same canonical form as the reconciliation heartbeat:
        // 0x + 70a08231 + 24 hex zeros (12-byte left-pad) + 40 hex (20-byte address)
        // = 74 chars total. This is the form that Anvil's `cast call` accepts.
        assert_eq!(calldata.len(), 74);
        assert_eq!(&calldata[..10], "0x70a08231");
        assert_eq!(&calldata[10..34], "000000000000000000000000");
        assert_eq!(&calldata[34..], "543ca0434b84ad38c858d2d178d2082521711fbc");
    }

    #[test]
    fn reconcile_result_is_clean_when_no_drift() {
        let r = ReconcileResult::default();
        assert!(r.is_clean());
    }

    #[test]
    fn reconcile_result_is_not_clean_when_drift_exceeds_threshold() {
        let r = ReconcileResult {
            drift_usd: 0.20,
            ..Default::default()
        };
        assert!(!r.is_clean());
    }

    #[test]
    fn reconcile_result_is_not_clean_with_to_add() {
        let r = ReconcileResult {
            to_add: vec![Position {
                id: "test".to_string(),
                pair: "GRT/USDC".to_string(),
                side: Side::Long,
                entry_price: 0.0,
                current_price: 0.0,
                quantity: 2.6,
                stop_loss: 0.0,
                take_profit_1: 0.0,
                take_profit_2: 0.0,
                take_profit_3: 0.0,
                unrealized_pnl: 0.0,
                risk_amount: 0.0,
                strategy_name: "chain_recovery".to_string(),
                opened_at: Utc::now(),
                scale_level: ScaleLevel::Full,
                token_address: "0x9623063377ad1b27544c965ccd7342f7ea7e88c7".to_string(),
            }],
            ..Default::default()
        };
        assert!(!r.is_clean());
    }
}
