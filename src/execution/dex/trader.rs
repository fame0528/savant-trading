//! DexTrader — no-KYC DEX execution engine.
//!
//! Implements the [`ExecutionEngine`] trait by calling a DEX aggregator API
//! (0x or 1inch) to obtain swap calldata, signing it with the wallet's
//! private key via `k256`+`rlp`, and broadcasting the raw transaction to the
//! configured EVM chain (default: Arbitrum).

use async_trait::async_trait;
use chrono::Utc;
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

use crate::core::error::ExecutionError;
use crate::core::types::{Order, OrderStatus, OrderType, Position, ScaleLevel, Side, TradeRecord};
use crate::execution::engine::ExecutionEngine;
use crate::{log_swap, log_swap_fail, log_swap_ok, log_warn};

use super::{amount_to_wei, resolve_pair, DexBackend, SwapParams, SwapTx, TokenInfo};

use alloy_core::primitives::hex;
use alloy_core::primitives::{Address, U256};
use k256::ecdsa::{RecoveryId, Signature, SigningKey};
use sha3::{Digest, Keccak256};

/// Minimal transaction receipt for on-chain verification.
struct TxReceipt {
    status: u64,
    gas_used: u64,
}

// ---------------------------------------------------------------------------
// RLP helpers for EIP-1559
// ---------------------------------------------------------------------------

/// Convert a U256 to its minimal big-endian bytes for RLP integer encoding.
fn u256_to_rlp_bytes(val: U256) -> Vec<u8> {
    let be: [u8; 32] = val.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(32);
    be[first..].to_vec()
}

#[allow(clippy::too_many_arguments)]
/// RLP-encode the unsigned EIP-1559 transaction fields (no signature).
///
/// The 0x02 type prefix is **not** included — the caller prepends it when
/// computing the signing hash.
fn rlp_encode_unsigned(
    chain_id: u64,
    nonce: u64,
    max_priority_fee: U256,
    max_fee: U256,
    gas_limit: u64,
    to_addr: &[u8],
    value: U256,
    data: &[u8],
) -> Vec<u8> {
    use rlp::RlpStream;
    let mut s = RlpStream::new_list(9);
    s.append(&chain_id);
    s.append(&nonce);
    s.append(&u256_to_rlp_bytes(max_priority_fee));
    s.append(&u256_to_rlp_bytes(max_fee));
    s.append(&gas_limit);
    s.append(&to_addr.to_vec());
    s.append(&u256_to_rlp_bytes(value));
    s.append(&data.to_vec());
    // Empty access list — must be RLP empty list (0xc0), NOT byte string (0x80)
    s.begin_list(0);
    s.as_raw().to_vec()
}

#[allow(clippy::too_many_arguments)]
/// RLP-encode the signed EIP-1559 transaction (all fields + signature).
///
/// Does **not** include the 0x02 type prefix — the caller prepends it.
fn rlp_encode_signed(
    chain_id: u64,
    nonce: u64,
    max_priority_fee: U256,
    max_fee: U256,
    gas_limit: u64,
    to_addr: &[u8],
    value: U256,
    data: &[u8],
    r: &[u8],
    s: &[u8],
    y_parity: u64,
) -> Vec<u8> {
    use rlp::RlpStream;
    let mut stream = RlpStream::new_list(12);
    stream.append(&chain_id);
    stream.append(&nonce);
    stream.append(&u256_to_rlp_bytes(max_priority_fee));
    stream.append(&u256_to_rlp_bytes(max_fee));
    stream.append(&gas_limit);
    stream.append(&to_addr.to_vec());
    stream.append(&u256_to_rlp_bytes(value));
    stream.append(&data.to_vec());
    // Empty access list — must be RLP empty list (0xc0), NOT byte string (0x80)
    stream.begin_list(0);
    stream.append(&y_parity);
    stream.append(&r.to_vec());
    stream.append(&s.to_vec());
    stream.as_raw().to_vec()
}

// ---------------------------------------------------------------------------
// DexState — serializable snapshot for crash recovery
// ---------------------------------------------------------------------------

/// Snapshot of [`DexTrader`] state persisted to disk after every mutation.
///
/// Loaded on startup to restore positions, balance, and trade history
/// after a crash or intentional restart.
#[derive(Serialize, Deserialize)]
struct DexState {
    positions: Vec<Position>,
    closed_trades: Vec<TradeRecord>,
    balance: f64,
    order_counter: u64,
}

// ---------------------------------------------------------------------------
// DexTrader
// ---------------------------------------------------------------------------

/// Execution engine that routes trades through a DEX aggregator.
pub struct DexTrader<B: DexBackend> {
    backend: B,
    /// Wallet private key (secp256k1) for signing.
    signing_key: SigningKey,
    /// Derived wallet address.
    wallet_address: Address,
    /// Primary RPC URL.
    rpc_url: String,
    /// Primary HTTP client for JSON-RPC.
    client: reqwest::Client,
    /// Primary chain ID.
    chain_id: u64,
    slippage_pct: f64,

    // ---- Multi-chain support (FID-045) ----
    /// Per-chain RPC clients for multi-chain execution.
    chain_clients: HashMap<u64, reqwest::Client>,
    /// Per-chain configs (gas, slippage).
    chain_configs: HashMap<u64, super::ChainConfig>,
    /// Per-chain gas halted flags.
    chain_gas_halted: HashMap<u64, bool>,

    // ---- State ----
    positions: HashMap<String, Position>,
    closed_trades: Vec<TradeRecord>,
    balance: f64,
    order_counter: u64,

    // ---- Production safety (FID-018) ----
    state_path: PathBuf,
    gas_halted: bool,

    // ---- Retry queue (FID-035) ----
    /// Failed swaps that should be retried on the next cycle.
    retry_queue: Vec<RetrySwap>,
    /// Maximum retries per swap before giving up.
    max_retries: u32,
}

/// A pending swap in the retry queue.
#[derive(Debug, Clone)]
pub struct RetrySwap {
    pub pair: String,
    pub side: Side,
    pub quantity: f64,
    pub entry_price: f64,
    pub attempts: u32,
    pub last_error: String,
}

impl<B: DexBackend + 'static> DexTrader<B> {
    /// Create a new DexTrader.
    pub async fn new(
        backend: B,
        wallet_private_key: &str,
        rpc_url: &str,
        chain_id: u64,
        slippage_pct: f64,
        initial_balance: f64,
    ) -> Result<Self, ExecutionError> {
        let hex_key = wallet_private_key.trim_start_matches("0x");
        let key_bytes = hex::decode(hex_key)
            .map_err(|e| ExecutionError::Other(format!("Invalid private key hex: {}", e)))?;

        let signing_key = SigningKey::from_slice(&key_bytes)
            .map_err(|e| ExecutionError::Other(format!("Invalid private key: {}", e)))?;

        let verifying_key = signing_key.verifying_key();
        let encoded = verifying_key.to_encoded_point(false).to_bytes().to_vec();
        let hash = Keccak256::digest(&encoded[1..]);
        let addr_bytes: [u8; 20] = hash[12..32]
            .try_into()
            .map_err(|_| ExecutionError::Other("Failed to derive address".into()))?;
        let wallet_address = Address::from(addr_bytes);

        let primary_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let mut chain_clients = HashMap::new();
        chain_clients.insert(chain_id, primary_client.clone());

        let mut chain_configs = HashMap::new();
        chain_configs.insert(
            chain_id,
            super::ChainConfig {
                chain_id,
                name: "Primary",
                rpc_url: rpc_url.to_string(),
                native_token: "ETH",
                min_gas_native: 0.002,
                slippage_pct,
            },
        );

        let mut trader = Self {
            backend,
            signing_key,
            wallet_address,
            rpc_url: rpc_url.to_string(),
            client: primary_client,
            chain_id,
            slippage_pct,
            chain_clients,
            chain_configs,
            chain_gas_halted: HashMap::new(),
            positions: HashMap::new(),
            closed_trades: Vec::new(),
            balance: initial_balance,
            order_counter: 0,
            state_path: PathBuf::from("data/dex_state.json"),
            gas_halted: false,
            retry_queue: Vec::new(),
            max_retries: 3,
        };

        // FID-018: Load persisted state on startup (crash recovery)
        let state_path = trader.state_path.clone();
        if trader.load_state(&state_path).unwrap_or(false) {
            info!("DexTrader: restored state from {:?}", state_path);
        }

        // Sync real on-chain balances immediately — don't trust config starting_balance
        let balance_before_sync = trader.balance;
        if let Err(e) = trader.sync_balance().await {
            warn!(
                "DexTrader: initial sync_balance failed ({}), using config balance",
                e
            );
        }

        // Reconcile: if tracked balance drifted significantly from actual on-chain balance,
        // the tracked positions are likely phantom (from reverted swaps that were recorded
        // as successful). Clear them to prevent the engine from managing non-existent positions.
        let drift = (balance_before_sync - trader.balance).abs();
        if drift > 1.0 && !trader.positions.is_empty() {
            warn!(
                "PHANTOM POSITIONS DETECTED: balance drift ${:.2} with {} tracked positions. \
                 Clearing all positions to reconcile with on-chain state.",
                drift,
                trader.positions.len()
            );
            trader.positions.clear();
            trader.save_state().ok();
        }

        // Additional check: if positions exist but no trades have ever completed,
        // the positions are likely phantom from reverted swaps.
        if !trader.positions.is_empty() && trader.closed_trades.is_empty() {
            warn!(
                "PHANTOM POSITIONS DETECTED: {} positions tracked but zero completed trades. \
                 Clearing positions — likely from reverted swaps.",
                trader.positions.len()
            );
            trader.positions.clear();
            trader.save_state().ok();
        }

        info!(
            "DexTrader initialized: backend={}, chain_id={}, wallet={:#x}, balance=${:.2}",
            trader.backend.name(),
            trader.chain_id,
            trader.wallet_address,
            trader.balance,
        );

        Ok(trader)
    }

    pub fn wallet_address(&self) -> Address {
        self.wallet_address
    }

    /// Build a swap transaction via the backend (handles Permit2 signing).
    pub async fn build_swap_tx(&self, params: &SwapParams) -> Result<SwapTx, ExecutionError> {
        self.backend.build_swap_tx(params).await
    }

    /// Register an additional chain for multi-chain execution (FID-045).
    /// Creates a new RPC client for the chain and stores its config.
    pub fn add_chain(&mut self, config: super::ChainConfig) {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        self.chain_clients.insert(config.chain_id, client);
        self.chain_configs.insert(config.chain_id, config);
    }

    /// Get the RPC client for a specific chain.
    fn client_for_chain(&self, chain_id: u64) -> &reqwest::Client {
        self.chain_clients.get(&chain_id).unwrap_or(&self.client)
    }

    /// Get all enabled chain IDs.
    pub fn chain_ids(&self) -> Vec<u64> {
        self.chain_clients.keys().copied().collect()
    }

    // ---- Retry queue (FID-035) ----

    /// Add a failed swap to the retry queue.
    pub fn enqueue_retry(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        entry_price: f64,
        error: &str,
    ) {
        self.retry_queue.push(RetrySwap {
            pair: pair.to_string(),
            side,
            quantity,
            entry_price,
            attempts: 1,
            last_error: error.to_string(),
        });
        log_warn!(
            "RETRY",
            "Enqueued {} {} for retry (error: {})",
            pair,
            side,
            error
        );
    }

    /// Drain the retry queue, returning swaps that should be retried.
    /// Expired entries (attempts >= max_retries) are discarded.
    pub fn drain_retry_queue(&mut self) -> Vec<RetrySwap> {
        let mut to_retry = Vec::new();
        let kept = Vec::new();
        for swap in self.retry_queue.drain(..) {
            if swap.attempts >= self.max_retries {
                log_warn!(
                    "RETRY",
                    "Discarding {} {} — max retries ({}) reached",
                    swap.pair,
                    swap.side,
                    self.max_retries
                );
            } else {
                to_retry.push(swap);
            }
        }
        self.retry_queue = kept;
        to_retry
    }

    /// Get the number of pending retries.
    pub fn pending_retries(&self) -> usize {
        self.retry_queue.len()
    }

    // ---- JSON-RPC ----

    /// Make an RPC call on the primary chain.
    async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ExecutionError> {
        self.rpc_call_on_chain(self.chain_id, method, params).await
    }

    /// Make an RPC call on a specific chain (FID-045).
    async fn rpc_call_on_chain(
        &self,
        chain_id: u64,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ExecutionError> {
        let client = self.client_for_chain(chain_id);
        let rpc_url = if chain_id == self.chain_id {
            &self.rpc_url
        } else if let Some(cfg) = self.chain_configs.get(&chain_id) {
            &cfg.rpc_url
        } else {
            return Err(ExecutionError::Other(format!(
                "No RPC config for chain {}",
                chain_id
            )));
        };

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1u64,
        });

        let resp = client.post(rpc_url).json(&body).send().await.map_err(|e| {
            ExecutionError::Other(format!(
                "RPC {} on chain {} failed: {}",
                method, chain_id, e
            ))
        })?;

        let json: serde_json::Value = resp.json().await.map_err(|e| {
            ExecutionError::Other(format!("RPC {} on chain {} parse: {}", method, chain_id, e))
        })?;

        if let Some(err) = json.get("error") {
            return Err(ExecutionError::Other(format!(
                "RPC {} on chain {} error: {}",
                method, chain_id, err
            )));
        }

        json.get("result").cloned().ok_or_else(|| {
            ExecutionError::Other(format!(
                "RPC {} on chain {} missing result",
                method, chain_id
            ))
        })
    }

    async fn get_nonce(&self) -> Result<u64, ExecutionError> {
        let addr = format!("{:#x}", self.wallet_address);
        let res = self
            .rpc_call(
                "eth_getTransactionCount",
                serde_json::json!([addr, "latest"]),
            )
            .await?;
        let s = res.as_str().ok_or_else(|| {
            ExecutionError::Other("eth_getTransactionCount returned non-string result".into())
        })?;
        u64::from_str_radix(s.trim_start_matches("0x"), 16)
            .map_err(|e| ExecutionError::Other(format!("Invalid nonce: {}", e)))
    }

    async fn get_gas_prices(&self) -> Result<(U256, U256), ExecutionError> {
        let base_res = self.rpc_call("eth_gasPrice", serde_json::json!([])).await?;
        let base_str = base_res.as_str().ok_or_else(|| {
            ExecutionError::Other("eth_gasPrice returned non-string result".into())
        })?;
        let base_fee = U256::from_str_radix(base_str.trim_start_matches("0x"), 16)
            .map_err(|e| ExecutionError::Other(format!("Invalid gas price: {}", e)))?;

        let priority = match self
            .rpc_call("eth_maxPriorityFeePerGas", serde_json::json!([]))
            .await
        {
            Ok(val) => {
                let s = val.as_str().ok_or_else(|| {
                    ExecutionError::Other(
                        "eth_maxPriorityFeePerGas returned non-string result".into(),
                    )
                })?;
                U256::from_str_radix(s.trim_start_matches("0x"), 16)
                    .unwrap_or(U256::from(100_000_000u64))
            }
            Err(_) => U256::from(100_000_000u64),
        };

        // 50% buffer on baseFee to handle baseFee increases between quote and broadcast
        let buffered_base = base_fee + (base_fee / U256::from(2));
        Ok((priority, buffered_base + priority))
    }

    // ---- Transaction signing & broadcasting ----

    /// Sign and broadcast an EIP-1559 transaction. Returns the tx hash.
    pub async fn sign_and_send(
        &self,
        to: Address,
        data: &[u8],
        value: U256,
        gas_limit: u64,
    ) -> Result<String, ExecutionError> {
        let nonce = self.get_nonce().await?;
        let (priority_fee, max_fee) = self.get_gas_prices().await?;

        let encoded = rlp_encode_unsigned(
            self.chain_id,
            nonce,
            priority_fee,
            max_fee,
            gas_limit,
            to.as_slice(),
            value,
            data,
        );

        // *** CRITICAL ***
        // EIP-1559 signing hash: keccak256(0x02 || rlp([chain_id, nonce, ...]))
        // The 0x02 type prefix MUST be included in the hash, not just the wire format.
        let mut signing_payload = vec![0x02u8];
        signing_payload.extend_from_slice(&encoded);
        let hash = Keccak256::digest(&signing_payload);

        // Sign: hash → (Signature, RecoveryId)
        let (signature, recid): (Signature, RecoveryId) = self
            .signing_key
            .sign_prehash_recoverable(&hash)
            .map_err(|e| ExecutionError::Other(format!("Signing failed: {}", e)))?;

        let r = signature.r().to_bytes().to_vec();
        let s = signature.s().to_bytes().to_vec();
        let y_parity: u64 = recid.is_y_odd().into();

        let signed_tx = rlp_encode_signed(
            self.chain_id,
            nonce,
            priority_fee,
            max_fee,
            gas_limit,
            to.as_slice(),
            value,
            data,
            &r,
            &s,
            y_parity,
        );

        // Wire format: 0x02 || rlp([chain_id, nonce, ..., y_parity, r, s])
        let mut raw_tx = vec![0x02u8];
        raw_tx.extend_from_slice(&signed_tx);

        let raw_tx_hex = format!("0x{}", hex::encode(&raw_tx));

        // FID-043: Pre-flight simulation with eth_call before broadcasting to catch malformed
        // calldata (bad Permit2 signature, wrong spender, etc.) before spending gas.
        let preflight_params = serde_json::json!([{
            "from": format!("{:#x}", self.wallet_address),
            "to": format!("{:#x}", to),
            "data": format!("0x{}", hex::encode(data)),
            "value": format!("0x{:x}", value),
            "gas": format!("0x{:x}", gas_limit),
        }, "latest"]);
        match self.rpc_call("eth_call", preflight_params).await {
            Ok(result) => {
                let ret = result.as_str().unwrap_or("0x");
                tracing::debug!("Pre-flight OK (ret={})", &ret[..ret.len().min(66)]);
            }
            Err(e) => {
                let err_str = e.to_string();
                warn!(
                    "PRE-FLIGHT FAILED: eth_call reverted — {} | \
                     This likely means bad Permit2 signature, insufficient gas, or no liquidity. Aborting broadcast.",
                    err_str
                );
                return Err(ExecutionError::Other(format!(
                    "Pre-flight simulation reverted: {}",
                    err_str
                )));
            }
        }

        let result = self
            .rpc_call("eth_sendRawTransaction", serde_json::json!([raw_tx_hex]))
            .await?;
        let tx_hash = result
            .as_str()
            .ok_or_else(|| {
                ExecutionError::Other("eth_sendRawTransaction returned no tx hash".into())
            })?
            .to_string();

        info!("DEX tx broadcast: hash={}", tx_hash);

        // Wait for transaction receipt and verify it succeeded on-chain.
        // Without this, phantom positions get recorded for reverted swaps.
        log_swap!("SWAP", "Waiting for receipt {}...", tx_hash);
        let receipt = self.wait_for_receipt(&tx_hash).await?;
        if receipt.status != 1 {
            return Err(ExecutionError::Other(format!(
                "Swap tx {} reverted on-chain (status=0)",
                tx_hash
            )));
        }
        log_swap_ok!(
            "SWAP",
            "Confirmed on-chain: {} (gas={})",
            tx_hash,
            receipt.gas_used
        );

        Ok(tx_hash)
    }

    /// Poll for transaction receipt with exponential backoff.
    /// Returns error if receipt not found within 60 seconds or tx reverted.
    async fn wait_for_receipt(&self, tx_hash: &str) -> Result<TxReceipt, ExecutionError> {
        let params = serde_json::json!([tx_hash]);
        let max_attempts = 30;
        let mut delay_ms = 1000u64;

        for attempt in 0..max_attempts {
            match self
                .rpc_call("eth_getTransactionReceipt", params.clone())
                .await
            {
                Ok(val) => {
                    if val.is_null() {
                        // Receipt not yet available
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(5000);
                        continue;
                    }
                    let status = val["status"]
                        .as_str()
                        .map(|s: &str| if s == "0x1" { 1u64 } else { 0u64 })
                        .unwrap_or(0);
                    let gas_used = u64::from_str_radix(
                        val["gasUsed"]
                            .as_str()
                            .unwrap_or("0x0")
                            .trim_start_matches("0x"),
                        16,
                    )
                    .unwrap_or(0);
                    return Ok(TxReceipt { status, gas_used });
                }
                Err(e) => {
                    if attempt < max_attempts - 1 {
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(5000);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(ExecutionError::Other(format!(
            "Timeout waiting for receipt of {}",
            tx_hash
        )))
    }

    /// Parse a wei value string from a DEX API response.
    ///
    /// The 0x and 1inch APIs may return `value` as either:
    /// - Hex `"0x0"` or `"0xde0b6b3a7640000"` (Ethereum JSON-RPC convention)
    /// - Decimal `"0"` or `"1000000000000000000"` (plain integer string)
    ///
    /// We detect format by the `0x` prefix.
    fn parse_wei_value(s: &str) -> U256 {
        let s = s.trim();
        if s.starts_with("0x") || s.starts_with("0X") {
            U256::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16)
                .unwrap_or(U256::ZERO)
        } else {
            // Plain decimal string
            U256::from_str_radix(s, 10).unwrap_or(U256::ZERO)
        }
    }

    // ---- Swap execution ----

    async fn execute_swap(
        &self,
        src_token: &TokenInfo,
        dst_token: &TokenInfo,
        amount_wei: &str,
    ) -> Result<String, ExecutionError> {
        // FID-018: Halt if ETH gas too low
        if self.gas_halted {
            return Err(ExecutionError::Other(
                "Trading halted — ETH gas balance too low. Fund wallet to resume.".into(),
            ));
        }

        let wallet_addr = format!("{:#x}", self.wallet_address);

        // Enterprise token resolution:
        // If the token has no address in the local DB, pass its SYMBOL directly.
        // The 0x / 1inch API accepts both addresses and symbols natively and
        // resolves symbols to the most liquid deployed contract on-chain.
        let src_id = if src_token.address.is_empty() {
            warn!(
                "Token '{}' not in local DB — resolving via API symbol lookup",
                src_token.symbol
            );
            src_token.symbol.clone()
        } else {
            src_token.address.clone()
        };
        let dst_id = if dst_token.address.is_empty() {
            dst_token.symbol.clone()
        } else {
            dst_token.address.clone()
        };

        let swap_params = super::SwapParams {
            src_token: src_id,
            dst_token: dst_id,
            amount: amount_wei.to_string(),
            slippage: self.slippage_pct,
            from: wallet_addr,
            chain_id: self.chain_id,
            sell_entire_balance: false,
        };

        log_swap!(
            "0x API",
            "Calling: {} -> {} amount={}",
            swap_params.src_token,
            swap_params.dst_token,
            swap_params.amount
        );

        // Spread filter (FID-035, FID-041): Check spread before executing swap.
        // Compares effective price against market price in USD (not raw wei).
        match self.backend.quote(&swap_params).await {
            Ok(quote) => {
                let buy_raw: f64 = quote.to_amount.parse().unwrap_or(0.0);
                let sell_raw: f64 = swap_params.amount.parse().unwrap_or(0.0);

                if buy_raw > 0.0 && sell_raw > 0.0 {
                    // Resolve decimals: prefer 0x API response, fall back to token DB
                    let buy_decimals = if quote.buy_decimals > 0 {
                        quote.buy_decimals
                    } else {
                        dst_token.decimals as u32
                    };
                    let sell_decimals = src_token.decimals as u32;

                    // Convert to human-readable amounts using correct decimals per token
                    let sell_tokens = sell_raw / 10f64.powi(sell_decimals as i32);
                    let buy_tokens = buy_raw / 10f64.powi(buy_decimals as i32);

                    // Minimum output check: reject dust amounts
                    if buy_tokens < 0.000001 {
                        log_warn!(
                            "SPREAD",
                            "Rejected {} — output {:.6} tokens is dust",
                            swap_params.dst_token,
                            buy_tokens
                        );
                        return Err(ExecutionError::Other(format!(
                            "Dust output: {:.6} tokens for {}",
                            buy_tokens, swap_params.dst_token
                        )));
                    }

                    // Effective price: sell per buy (how many sell tokens per buy token)
                    let effective_price = sell_tokens / buy_tokens;

                    // Parse market price from quote (0x returns USD price, 1inch returns src amount)
                    let market_price_raw: f64 = quote.price.parse().unwrap_or(0.0);

                    // If market price unavailable, calculate from buy/sell amounts
                    // This is the same as effective_price, so spread = 0 (no spread check needed)
                    let market_price = if market_price_raw > 0.0 {
                        market_price_raw
                    } else {
                        effective_price
                    };

                    let spread_bps =
                        ((effective_price - market_price) / market_price).abs() * 10000.0;
                    if spread_bps > 30.0 {
                        log_warn!("SPREAD", "Rejected {} — spread {:.0}bps exceeds 30bps limit (eff=${:.6}, mkt=${:.6})",
                            swap_params.dst_token, spread_bps, effective_price, market_price);
                        return Err(ExecutionError::Other(format!(
                            "Spread {:.0}bps exceeds 30bps limit for {}",
                            spread_bps, swap_params.dst_token
                        )));
                    }
                    log_swap!(
                        "SPREAD",
                        "OK: {:.0}bps for {} (eff=${:.6}, mkt=${:.6})",
                        spread_bps,
                        swap_params.dst_token,
                        effective_price,
                        market_price
                    );
                }
            }
            Err(e) => {
                log_swap_fail!("SPREAD", "Quote failed ({}), aborting swap", e);
                return Err(ExecutionError::Other(format!(
                    "Quote failed, aborting swap: {}",
                    e
                )));
            }
        }

        // Fast-fail: 15s timeout + catch_unwind on the 0x API call.
        // The reqwest client has a 30s timeout but doesn't cover DNS/TLS hangs.
        // catch_unwind prevents panics in the HTTP client from killing the engine.
        let swap_tx = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            std::panic::AssertUnwindSafe(self.backend.build_swap_tx(&swap_params)).catch_unwind(),
        )
        .await
        {
            Ok(Ok(result)) => result?,
            Ok(Err(panic_err)) => {
                let msg = if let Some(s) = panic_err.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_err.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic in 0x API".to_string()
                };
                log_swap_fail!("0x API", "Panicked: {}", msg);
                return Err(ExecutionError::Other(format!("0x API panicked: {}", msg)));
            }
            Err(_) => {
                log_swap_fail!("0x API", "Timeout after 15s — API hung");
                return Err(ExecutionError::Other("0x API timeout after 15s".into()));
            }
        };

        log_swap_ok!(
            "0x QUOTE",
            "OK: to={} gas={} value={}",
            swap_tx.to,
            swap_tx.gas,
            swap_tx.value
        );

        let to: Address = swap_tx
            .to
            .parse()
            .map_err(|e| ExecutionError::Other(format!("Invalid 'to' '{}': {}", swap_tx.to, e)))?;

        let data = hex::decode(swap_tx.data.trim_start_matches("0x"))
            .map_err(|e| ExecutionError::Other(format!("Invalid calldata: {}", e)))?;

        let value = Self::parse_wei_value(&swap_tx.value);

        // Apply gas buffer: 0x API estimates are often too low for Permit2 calldata.
        // Use 1.5x buffer with a minimum of 800,000 for complex swaps.
        let gas_limit = std::cmp::max(swap_tx.gas * 3 / 2, 800_000);

        // Retry logic: up to 3 attempts for transient failures (gas spike, network error)
        let max_retries = 3;
        let mut last_err = None;
        for attempt in 1..=max_retries {
            log_swap!(
                "SIGN",
                "Tx (attempt {}/{}): to={:#x} data_len={} value={} gas={}",
                attempt,
                max_retries,
                to,
                data.len(),
                value,
                gas_limit
            );
            let result = self.sign_and_send(to, &data, value, gas_limit).await;
            match result {
                Ok(hash) => {
                    log_swap_ok!("BROADCAST", "OK: {}", hash);
                    return Ok(hash);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    let is_transient = err_str.contains("max fee per gas")
                        || err_str.contains("nonce too low")
                        || err_str.contains("replacement transaction underpriced")
                        || err_str.contains("network")
                        || err_str.contains("timeout")
                        || err_str.contains("ECONNRESET");

                    if is_transient && attempt < max_retries {
                        log_warn!(
                            "BROADCAST",
                            "Transient failure (attempt {}/{}): {} — retrying in 2s",
                            attempt,
                            max_retries,
                            err_str
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        last_err = Some(e);
                    } else {
                        log_swap_fail!(
                            "BROADCAST",
                            "FAILED (attempt {}/{}): {}",
                            attempt,
                            max_retries,
                            err_str
                        );
                        return Err(e);
                    }
                }
            }
        }
        Err(last_err
            .unwrap_or_else(|| ExecutionError::Other("Swap failed after all retries".into())))
    }

    // ---- Stop monitoring ----

    // ---- ERC-20 approve for Permit2 ----

    /// Ensure a token is approved for the Permit2 contract.
    /// This is REQUIRED before any Permit2 swap can succeed.
    /// Checks current allowance and sends approve(max) if insufficient.
    async fn ensure_permit2_approval(
        &self,
        token_address: &str,
        amount_wei: &str,
    ) -> Result<(), ExecutionError> {
        const PERMIT2_ADDRESS: &str = "0x000000000022d473030f116ddee9f6b43ac78ba3";

        let padded_owner = format!(
            "{:0>64}",
            format!("{:x}", self.wallet_address).trim_start_matches("0x")
        );
        let padded_spender = format!("{:0>64}", PERMIT2_ADDRESS.trim_start_matches("0x"));

        // allowance(owner, spender) selector: 0xdd62ed3e
        let call_data = format!("0xdd62ed3e{}{}", padded_owner, padded_spender);
        let call_params = serde_json::json!([{
            "to": token_address,
            "data": call_data
        }, "latest"]);

        let current_allowance = match self.rpc_call("eth_call", call_params).await {
            Ok(result) => {
                let hex_str = result.as_str().unwrap_or("0x0");
                U256::from_str_radix(hex_str.trim_start_matches("0x"), 16).unwrap_or(U256::ZERO)
            }
            Err(e) => {
                warn!(
                    "Failed to check {} allowance: {} — assuming zero",
                    token_address, e
                );
                U256::ZERO
            }
        };

        let required = U256::from_str_radix(amount_wei, 10).unwrap_or(U256::ZERO);

        if current_allowance >= required && current_allowance > U256::ZERO {
            tracing::debug!(
                "Token {} allowance sufficient: current={}, required={}",
                token_address,
                current_allowance,
                required
            );
            return Ok(());
        }

        info!(
            "Token {} allowance insufficient for Permit2 (current={}, required={}). \
             Sending approve(max) transaction...",
            token_address, current_allowance, required
        );

        // approve(spender, amount) selector: 0x095ea7b3
        // max uint256 = 2^256 - 1
        let max_approval = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let approve_data = format!("0x095ea7b3{}{}", padded_spender, max_approval);

        let token_addr: Address = token_address.parse().map_err(|_| {
            ExecutionError::Other(format!("Invalid token address: {}", token_address))
        })?;

        let approve_bytes = hex::decode(&approve_data[2..])
            .map_err(|e| ExecutionError::Other(format!("Invalid approve calldata: {}", e)))?;

        // Use a reasonable gas limit for approve (typically ~45K on Arbitrum)
        let tx_hash = self
            .sign_and_send(token_addr, &approve_bytes, U256::ZERO, 100_000)
            .await?;
        info!(
            "Token {} approve(max) for Permit2 sent: tx={}",
            token_address, tx_hash
        );

        // Wait for confirmation
        let receipt = self.wait_for_receipt(&tx_hash).await?;
        if receipt.status != 1 {
            return Err(ExecutionError::Other(format!(
                "Token {} approve(max) for Permit2 reverted on-chain",
                token_address
            )));
        }
        info!(
            "Token {} approve(max) for Permit2 confirmed (gas={})",
            token_address, receipt.gas_used
        );

        Ok(())
    }

    // ---- State persistence (FID-018) ----

    /// Persist current positions, closed trades, and balance to disk.
    /// Called automatically on every position mutation (open/close/update).
    pub fn save_state(&self) -> Result<(), ExecutionError> {
        let state = DexState {
            positions: self.positions.values().cloned().collect(),
            closed_trades: self.closed_trades.clone(),
            balance: self.balance,
            order_counter: self.order_counter,
        };
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| ExecutionError::Other(format!("Failed to serialize state: {}", e)))?;

        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ExecutionError::Other(format!("Failed to create state dir: {}", e)))?;
        }

        std::fs::write(&self.state_path, &json)
            .map_err(|e| ExecutionError::Other(format!("Failed to write state: {}", e)))?;
        Ok(())
    }

    /// Load persisted state from disk — returns `true` if state was restored.
    ///
    /// On crash recovery, this restores positions, balance, and trade history.
    /// If stop-loss targets are already breached, `check_stops()` will detect
    /// and close them on the next tick.
    pub fn load_state(&mut self, path: &Path) -> Result<bool, ExecutionError> {
        if !path.exists() {
            return Ok(false);
        }

        let json = std::fs::read_to_string(path)
            .map_err(|e| ExecutionError::Other(format!("Failed to read state: {}", e)))?;

        let state: DexState = serde_json::from_str(&json)
            .map_err(|e| ExecutionError::Other(format!("Failed to parse state: {}", e)))?;

        self.positions = state
            .positions
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();
        self.closed_trades = state.closed_trades;
        self.balance = state.balance;
        self.order_counter = state.order_counter;

        warn!(
            "DEX stop-losses are CLIENT-SIDE only — NOT exchange-guaranteed. \
             Positions loaded from state: {}",
            self.positions.len()
        );

        Ok(true)
    }

    pub fn check_stops(&mut self, prices: &HashMap<String, f64>) -> Vec<TradeRecord> {
        let mut closed = Vec::new();
        let mut to_remove = Vec::new();

        for (id, pos) in &self.positions {
            if let Some(&price) = prices.get(&pos.pair) {
                let hit_stop = match pos.side {
                    Side::Long => price <= pos.stop_loss,
                    Side::Short => price >= pos.stop_loss,
                };

                if hit_stop {
                    let pnl = match pos.side {
                        Side::Long => (pos.stop_loss - pos.entry_price) * pos.quantity,
                        Side::Short => (pos.entry_price - pos.stop_loss) * pos.quantity,
                    };

                    let trade = TradeRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        pair: pos.pair.clone(),
                        side: pos.side,
                        entry_price: pos.entry_price,
                        exit_price: pos.stop_loss,
                        quantity: pos.quantity,
                        pnl,
                        pnl_pct: if pos.entry_price > 0.0 {
                            pnl / (pos.entry_price * pos.quantity) * 100.0
                        } else {
                            0.0
                        },
                        fees: 0.0,
                        strategy_name: pos.strategy_name.clone(),
                        opened_at: pos.opened_at,
                        closed_at: Utc::now(),
                        notes: format!("DEX stop-loss: {:?}", pos.scale_level),
                    };

                    self.balance += pnl;
                    self.closed_trades.push(trade.clone());
                    closed.push(trade);
                    to_remove.push(id.clone());
                }
            }
        }

        for id in to_remove {
            self.positions.remove(&id);
        }

        // FID-018: Persist state after stop-loss closes
        if !closed.is_empty() {
            if let Err(e) = self.save_state() {
                warn!("Failed to save state after stop-loss close: {}", e);
            }
        }

        closed
    }

    pub fn set_balance(&mut self, balance: f64) {
        self.balance = balance;
    }
}

// ---- ExecutionEngine impl ----

#[async_trait]
impl<B: DexBackend + 'static> ExecutionEngine for DexTrader<B> {
    async fn place_order(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        price: Option<f64>,
    ) -> Result<Order, ExecutionError> {
        // FID-018: Halt if ETH gas too low
        if self.gas_halted {
            return Err(ExecutionError::Other(
                "Trading halted — ETH gas balance too low. Fund wallet to resume.".into(),
            ));
        }

        // Safety: refuse to trade if tracked balance is zero or negative
        if self.balance <= 0.0 {
            return Err(ExecutionError::Other(format!(
                "Trading halted — balance is ${:.2}. Fund wallet with USDC to resume.",
                self.balance
            )));
        }

        self.order_counter += 1;

        let (src_token, dst_token) = resolve_pair(pair, side)?;
        let entry_price = price.unwrap_or(0.0);
        // LONG: src=USDC, amount = entry_price * quantity (USDC value)
        // SHORT: src=base token, amount = quantity (token amount)
        let order_value = match side {
            Side::Long => entry_price * quantity,
            Side::Short => entry_price * quantity, // USD value for balance tracking
        };
        let amount_wei = match side {
            Side::Long => amount_to_wei(order_value, src_token.decimals),
            Side::Short => amount_to_wei(quantity, src_token.decimals),
        };

        info!(
            "DEX {} {} {} | src={} dst={} qty_wei={}",
            if side == Side::Long { "BUY" } else { "SELL" },
            quantity,
            pair,
            src_token.symbol,
            dst_token.symbol,
            amount_wei
        );

        // CRITICAL: Ensure source token is approved for the Permit2 contract before swapping.
        // Without this, the Permit2 contract cannot transfer tokens on our behalf.
        if src_token.address.is_empty() {
            return Err(ExecutionError::Other(format!(
                "Cannot approve token '{}' — no address in local DB. Use a token with a known address.",
                src_token.symbol
            )));
        }
        if let Err(e) = self
            .ensure_permit2_approval(&src_token.address, &amount_wei)
            .await
        {
            log_swap_fail!("APPROVE", "Permit2 approval failed: {}", e);
            return Err(e);
        }

        let tx_hash = self
            .execute_swap(&src_token, &dst_token, &amount_wei)
            .await?;

        info!("DEX filled: {} | tx={}", pair, tx_hash);

        let fill_price = entry_price;
        let position_id = format!("dex-{}-{}", self.order_counter, pair.replace('/', "_"));

        self.positions.insert(
            position_id.clone(),
            Position {
                id: position_id,
                pair: pair.to_string(),
                side,
                entry_price: fill_price,
                current_price: fill_price,
                stop_loss: 0.0,
                take_profit_1: 0.0,
                take_profit_2: 0.0,
                take_profit_3: 0.0,
                quantity,
                unrealized_pnl: 0.0,
                risk_amount: 0.0,
                strategy_name: format!("dex_{}", self.backend.name()),
                opened_at: Utc::now(),
                scale_level: ScaleLevel::Full,
            },
        );

        self.balance -= order_value * 1.001;

        Ok(Order {
            id: format!("dex-{}-{}", self.order_counter, tx_hash),
            pair: pair.to_string(),
            side,
            order_type: OrderType::Market,
            price: Some(fill_price),
            quantity,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(fill_price),
            tx_hash: Some(tx_hash.clone()),
        })
    }

    async fn close_position(&mut self, position_id: &str) -> Result<Order, ExecutionError> {
        // SAFETY: Clone the position but DO NOT remove it from the map until the swap
        // is verified on-chain. If the swap fails, the position stays open.
        let pos = self
            .positions
            .get(position_id)
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?
            .clone();

        self.order_counter += 1;

        let (src_token, dst_token) = resolve_pair(
            &pos.pair,
            match pos.side {
                Side::Long => Side::Short,
                Side::Short => Side::Long,
            },
        )?;
        let qty_wei = amount_to_wei(pos.quantity, src_token.decimals);

        if src_token.address.is_empty() {
            return Err(ExecutionError::Other(format!(
                "Cannot approve token '{}' for close — no address in local DB.",
                src_token.symbol
            )));
        }

        // Step 1: Check liquidity via 0x /price endpoint BEFORE attempting swap.
        // Per 0x docs: "The API will return liquidityAvailable=false if there
        // isn't enough liquidity available for the requested quote."
        let wallet_addr = format!("{:#x}", self.wallet_address);
        let price_params = super::SwapParams {
            src_token: src_token.address.clone(),
            dst_token: dst_token.address.clone(),
            amount: qty_wei.clone(),
            slippage: self.slippage_pct,
            from: wallet_addr.clone(),
            chain_id: self.chain_id,
            sell_entire_balance: true, // Use actual on-chain balance for close
        };
        match self.backend.check_liquidity(&price_params).await {
            Ok(check) if check.available => {
                info!(
                    "Liquidity check OK for {} close — proceeding with swap",
                    pos.pair
                );
            }
            Ok(_check) => {
                return Err(ExecutionError::Other(format!(
                    "No liquidity available to close {} — 0x returned liquidityAvailable=false. \
                     Position stays open. Will retry next cycle.",
                    pos.pair
                )));
            }
            Err(e) => {
                return Err(ExecutionError::Other(format!(
                    "Liquidity pre-check failed for {} close: {}. Position stays open.",
                    pos.pair, e
                )));
            }
        }

        // Step 2: Ensure source token is approved for Permit2.
        if let Err(e) = self
            .ensure_permit2_approval(&src_token.address, &qty_wei)
            .await
        {
            log_swap_fail!("APPROVE", "Permit2 approval failed on close: {}", e);
            return Err(e);
        }

        // Step 3: Record USDC balance BEFORE swap for on-chain verification.
        let usdc_balance_before = self.balance;

        // Step 4: Execute the swap — try standard first, fall back to gasless on dust.
        let tx_hash = match self.execute_swap(&src_token, &dst_token, &qty_wei).await {
            Ok(hash) => hash,
            Err(e) if e.to_string().contains("Dust output") => {
                // Standard swap returned 0 output (0x can't route micro-amounts).
                // Fall back to 0x Gasless API which handles approvals and gas automatically.
                warn!(
                    "Standard swap returned dust for {} — falling back to Gasless API",
                    pos.pair
                );
                let wallet_addr = format!("{:#x}", self.wallet_address);
                let gasless_params = super::SwapParams {
                    src_token: src_token.address.clone(),
                    dst_token: dst_token.address.clone(),
                    amount: qty_wei.clone(),
                    slippage: self.slippage_pct,
                    from: wallet_addr,
                    chain_id: self.chain_id,
                    sell_entire_balance: true,
                };
                let gasless_result = self
                    .backend
                    .build_gasless_swap_tx(&gasless_params)
                    .await
                    .map_err(|ge| {
                        log_swap_fail!(
                            "GASLESS",
                            "Gasless fallback also failed for {}: {}",
                            pos.pair,
                            ge
                        );
                        ge
                    })?;
                log_swap_ok!(
                    "GASLESS",
                    "Submitted: trade_hash={} buy_amount={}",
                    gasless_result.trade_hash,
                    gasless_result.buy_amount
                );
                // Poll for confirmation (up to ~60s)
                match self
                    .backend
                    .poll_gasless_status(&gasless_result.trade_hash, self.chain_id)
                    .await
                {
                    Ok(super::GaslessStatus::Confirmed(hash)) => {
                        log_swap_ok!("GASLESS", "Confirmed on-chain: {}", hash);
                        hash
                    }
                    Ok(super::GaslessStatus::Failed(reason)) => {
                        return Err(ExecutionError::Other(format!(
                            "Gasless trade failed: {}",
                            reason
                        )));
                    }
                    Err(e) => {
                        return Err(ExecutionError::Other(format!(
                            "Gasless status poll error: {}",
                            e
                        )));
                    }
                }
            }
            Err(e) => return Err(e),
        };

        // Step 5: Verify on-chain — query actual USDC balance after swap.
        // A tx can have status=1 but deliver 0 output (failed internal swap).
        let usdc_addr = super::usdc_address_for_chain(self.chain_id);
        let usdc_dec = super::usdc_decimals_for_chain(self.chain_id);
        let padded_addr = format!(
            "{:0>64}",
            self.wallet_address.to_string().trim_start_matches("0x")
        );
        let call_data = format!("0x70a08231{}", padded_addr);
        let call_params = serde_json::json!([{
            "to": usdc_addr,
            "data": call_data
        }, "latest"]);

        let verified_proceeds = match self.rpc_call("eth_call", call_params).await {
            Ok(result) => {
                if let Some(hex) = result.as_str() {
                    let usdc_wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                        .unwrap_or(U256::ZERO);
                    let divisor = 10f64.powi(usdc_dec as i32);
                    let usdc_after: f64 =
                        usdc_wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                    let gained = usdc_after - usdc_balance_before;
                    if gained <= 0.0 {
                        // Swap tx succeeded (status=1) but delivered 0 USDC.
                        // Position stays open — tokens are still in wallet.
                        return Err(ExecutionError::Other(format!(
                            "Close tx {} succeeded but delivered ${:.2} USDC (before=${:.2}, after=${:.2}). \
                             Position stays open. Tokens may be stranded.",
                            tx_hash, gained, usdc_balance_before, usdc_after
                        )));
                    }
                    info!("On-chain verified: {} close delivered ${:.2} USDC (before=${:.2}, after=${:.2})",
                        pos.pair, gained, usdc_balance_before, usdc_after);
                    gained
                } else {
                    warn!("Could not parse USDC balance for close verification — using estimate");
                    pos.entry_price * pos.quantity
                        + (pos.current_price - pos.entry_price) * pos.quantity
                }
            }
            Err(e) => {
                warn!(
                    "Failed to verify USDC balance after close: {} — using estimate",
                    e
                );
                pos.entry_price * pos.quantity
                    + (pos.current_price - pos.entry_price) * pos.quantity
            }
        };

        // Step 6: Swap verified — NOW remove position from map.
        self.positions.remove(position_id);

        info!(
            "DEX closed: {} | tx={} | verified proceeds=${:.2}",
            pos.pair, tx_hash, verified_proceeds
        );

        let exit_price = pos.current_price;
        let gross_pnl = match pos.side {
            Side::Long => (exit_price - pos.entry_price) * pos.quantity,
            Side::Short => (pos.entry_price - exit_price) * pos.quantity,
        };
        let fee_est = exit_price * pos.quantity * 0.001;
        let pnl = gross_pnl - fee_est;
        self.balance = usdc_balance_before + verified_proceeds;

        self.closed_trades.push(TradeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            pair: pos.pair.clone(),
            side: pos.side,
            entry_price: pos.entry_price,
            exit_price,
            quantity: pos.quantity,
            pnl,
            pnl_pct: if pos.entry_price > 0.0 {
                pnl / (pos.entry_price * pos.quantity) * 100.0
            } else {
                0.0
            },
            fees: fee_est,
            strategy_name: pos.strategy_name.clone(),
            opened_at: pos.opened_at,
            closed_at: Utc::now(),
            notes: format!("DEX close via {} — on-chain verified", self.backend.name()),
        });

        if let Err(e) = self.save_state() {
            warn!("Failed to save state after close: {}", e);
        }

        Ok(Order {
            id: format!("dex-close-{}-{}", self.order_counter, tx_hash),
            pair: pos.pair,
            side: match pos.side {
                Side::Long => Side::Short,
                Side::Short => Side::Long,
            },
            order_type: OrderType::Market,
            price: Some(exit_price),
            quantity: pos.quantity,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(exit_price),
            tx_hash: Some(tx_hash.clone()),
        })
    }

    fn open_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    fn balance(&self) -> f64 {
        self.balance
    }

    // ---- FID-061: Wallet recovery position registration ----

    fn register_position(&mut self, id: String, pos: Position) {
        info!(
            "DexTrader: registered wallet-recovered position {} ({})",
            id, pos.pair
        );
        self.positions.insert(id, pos);
    }

    // ---- FID-018: Production safety overrides ----

    async fn sync_balance(&mut self) -> Result<(), ExecutionError> {
        let addr_hex = format!("{:#x}", self.wallet_address);

        // Dynamic gas check (FID-052): query current gas price, calculate how many
        // swaps we can afford. Halt only if we can't afford even 1 swap.
        // A typical 0x swap on Arbitrum uses ~500K gas. On Base/Optimism ~200K.
        let typical_gas_limit: u64 = 500_000;

        for &cid in self.chain_clients.keys() {
            let chain_name = self
                .chain_configs
                .get(&cid)
                .map(|c| c.name)
                .unwrap_or("unknown");

            // 1. Get current gas price from network
            let gas_price_res = self
                .rpc_call_on_chain(cid, "eth_gasPrice", serde_json::json!([]))
                .await;
            let gas_price_gwei: f64 = match gas_price_res {
                Ok(val) => {
                    if let Some(hex) = val.as_str() {
                        let wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                            .unwrap_or(U256::ZERO);
                        wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e9 // Convert wei → gwei
                    } else {
                        0.1
                    } // fallback: 0.1 gwei (typical Arbitrum)
                }
                Err(_) => 0.1, // fallback
            };

            // 2. Estimate cost per swap in native token
            //    cost = gas_limit × gas_price (in ETH)
            let gas_price_eth = gas_price_gwei / 1e9; // gwei → ETH
            let cost_per_swap = typical_gas_limit as f64 * gas_price_eth;

            // 3. We need enough gas for at least 2 swaps (buy + potential stop-loss exit)
            //    plus 50% buffer for gas spikes
            let min_gas = cost_per_swap * 2.0 * 1.5;

            // 4. Get native token balance
            let gas_res = self
                .rpc_call_on_chain(
                    cid,
                    "eth_getBalance",
                    serde_json::json!([&addr_hex, "latest"]),
                )
                .await;
            match gas_res {
                Ok(val) => {
                    if let Some(hex) = val.as_str() {
                        let wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                            .unwrap_or(U256::ZERO);
                        let gas_balance: f64 = wei.to_string().parse().unwrap_or(0.0) / 1e18;
                        let swaps_affordable = if cost_per_swap > 0.0 {
                            (gas_balance / cost_per_swap).floor() as u64
                        } else {
                            u64::MAX
                        };

                        if gas_balance < min_gas {
                            self.chain_gas_halted.insert(cid, true);
                            error!(
                                "CRITICAL: {} ({}) gas balance {:.6} ETH — can afford {} swaps (need 2+). Gas: {:.2} gwei, ~{:.6}/swap. HALTING.",
                                chain_name, cid, gas_balance, swaps_affordable, gas_price_gwei, cost_per_swap
                            );
                        } else {
                            if self.chain_gas_halted.get(&cid) == Some(&true) {
                                info!("{} ({}) gas restored — {:.6} ETH, {} swaps affordable at {:.2} gwei. Resuming.",
                                    chain_name, cid, gas_balance, swaps_affordable, gas_price_gwei);
                            }
                            self.chain_gas_halted.insert(cid, false);
                            tracing::debug!(
                                "{} gas OK: {:.6} ETH, {} swaps affordable at {:.2} gwei",
                                chain_name,
                                gas_balance,
                                swaps_affordable,
                                gas_price_gwei
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to check gas on chain {}: {}", cid, e);
                }
            }
        }

        // Also update the primary gas_halted flag
        self.gas_halted = self.chain_gas_halted.get(&self.chain_id) == Some(&true);

        // Query USDC balance on primary chain — this is the cash available for new trades
        let usdc_addr = super::usdc_address_for_chain(self.chain_id);
        let usdc_dec = super::usdc_decimals_for_chain(self.chain_id);
        let padded_addr = format!("{:0>64}", addr_hex.trim_start_matches("0x"));
        let call_data = format!("0x70a08231{}", padded_addr);

        let call_params = serde_json::json!([{
            "to": usdc_addr,
            "data": call_data
        }, "latest"]);

        match self.rpc_call("eth_call", call_params).await {
            Ok(result) => {
                if let Some(hex) = result.as_str() {
                    let usdc_wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                        .unwrap_or(U256::ZERO);
                    let divisor = 10f64.powi(usdc_dec as i32);
                    let usdc_balance: f64 =
                        usdc_wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;

                    let drift = (usdc_balance - self.balance).abs();
                    if drift > 1.0 {
                        warn!(
                            "USDC balance drift: tracked={:.2} actual={:.2} (diff=${:.2})",
                            self.balance, usdc_balance, drift
                        );
                    }
                    self.balance = usdc_balance;
                    info!("USDC balance: ${:.6}", usdc_balance);
                }
            }
            Err(e) => {
                warn!("Failed to query USDC balance: {}", e);
            }
        }

        // Query all curated pair token balances on primary chain — chain-first, nothing hardcoded.
        // These are the actual on-chain holdings that the engine must track.
        let pair_tokens: Vec<(String, String, u8)> = super::ARBITRUM_TOKENS
            .iter()
            .filter(|(sym, _, _)| *sym != "USDC")
            .map(|(sym, addr, dec)| (sym.to_string(), addr.to_string(), *dec))
            .collect();

        for (sym, token_addr, decimals) in &pair_tokens {
            let bal_call = serde_json::json!([{
                "to": token_addr,
                "data": format!("0x70a08231{}", padded_addr)
            }, "latest"]);

            if let Ok(result) = self.rpc_call("eth_call", bal_call).await {
                if let Some(hex) = result.as_str() {
                    let wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                        .unwrap_or(U256::ZERO);
                    let divisor = 10f64.powi(*decimals as i32);
                    let bal: f64 = wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                    if bal > 0.0001 {
                        info!("On-chain {} balance: {:.8}", sym, bal);
                    }
                }
            }
        }

        let _ = self.save_state();
        Ok(())
    }

    async fn place_stop_loss(&mut self, position_id: &str) -> Result<(), ExecutionError> {
        info!(
            "Stop-loss registered for {} (client-side, DB-persisted)",
            position_id
        );
        // Stop-loss value is on the Position struct, which is persisted to SQLite
        // by the engine on every open/close/trail event. No separate persistence needed.
        // PortfolioManager::check_stops() fires the stop every cycle.
        Ok(())
    }

    async fn check_liquidity(
        &self,
        pair: &str,
        side: Side,
        amount_usd: f64,
    ) -> Result<super::LiquidityCheck, ExecutionError> {
        let (src_token, dst_token) = resolve_pair(pair, side)?;
        let test_amount = format!("{}", (amount_usd.max(1.0) * 1_000_000.0) as u64);
        let params = SwapParams {
            src_token: src_token.address.clone(),
            dst_token: dst_token.address.clone(),
            amount: test_amount,
            slippage: 0.01,
            from: String::new(),
            chain_id: 42161,
            sell_entire_balance: false,
        };
        self.backend.check_liquidity(&params).await
    }

    async fn sync_wallet_positions(&self, curated_pairs: &[String]) -> Vec<(String, f64, f64)> {
        let mut discrepancies = Vec::new();
        for pair in curated_pairs {
            // resolve_pair returns (src, dst). For LONG: src=USDC, dst=ASSET.
            // We need the ASSET token balance (dst for LONG).
            if let Ok((_, asset_token)) = resolve_pair(pair, Side::Long) {
                if asset_token.address.is_empty() {
                    continue;
                }
                if let Some(on_chain_qty) = self
                    .query_token_balance(&asset_token.address, asset_token.decimals)
                    .await
                {
                    let tracked_qty = self
                        .positions
                        .values()
                        .find(|p| p.pair == *pair)
                        .map(|p| p.quantity)
                        .unwrap_or(0.0);
                    let diff = (on_chain_qty - tracked_qty).abs();
                    if diff > 0.001 && on_chain_qty > 0.0001 {
                        discrepancies.push((pair.clone(), on_chain_qty, tracked_qty));
                    }
                }
            }
        }
        discrepancies
    }
}

impl<B: DexBackend + 'static> DexTrader<B> {
    /// Query on-chain ERC-20 balance for a single token via balanceOf(wallet).
    pub async fn query_token_balance(&self, token_address: &str, decimals: u8) -> Option<f64> {
        let padded_addr = format!(
            "{:0>64}",
            self.wallet_address.to_string().trim_start_matches("0x")
        );
        let call_data = format!("0x70a08231{}", padded_addr);
        let call_params = serde_json::json!([{
            "to": token_address,
            "data": call_data
        }, "latest"]);

        match self.rpc_call("eth_call", call_params).await {
            Ok(result) => {
                if let Some(hex) = result.as_str() {
                    let wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                        .unwrap_or(U256::ZERO);
                    let divisor = 10f64.powi(decimals as i32);
                    let balance = wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                    Some(balance)
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::debug!("Failed to query balance for {}: {}", token_address, e);
                None
            }
        }
    }
}
