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

use super::{amount_to_wei, resolve_pair_on_chain, DexBackend, SwapParams, SwapTx, TokenInfo};

use alloy_core::primitives::hex;
use alloy_core::primitives::{Address, U256};
use k256::ecdsa::{RecoveryId, Signature, SigningKey};
use sha3::{Digest, Keccak256};

// ---------------------------------------------------------------------------
// FID-108: Error categorization + failure tracking
// ---------------------------------------------------------------------------

/// Categorizes execution errors for retry vs blacklist decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// RPC timeout, nonce collision, network error — safe to retry.
    Transient,
    /// Token dead, honeypot, no liquidity — blacklist after repeated failures.
    Permanent,
    /// Insufficient balance, wrong network — alert user, don't retry.
    UserFixable,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transient => write!(f, "Transient"),
            Self::Permanent => write!(f, "Permanent"),
            Self::UserFixable => write!(f, "UserFixable"),
        }
    }
}

/// Classify an execution error into a category for retry logic.
pub fn categorize_error(err: &ExecutionError) -> ErrorCategory {
    let msg = err.to_string().to_lowercase();

    // User-fixable first (highest priority — don't retry these)
    if msg.contains("insufficient balance")
        || msg.contains("insufficient funds")
        || msg.contains("balance is $")
        || msg.contains("fund wallet")
    {
        return ErrorCategory::UserFixable;
    }

    // Permanent failures (blacklist after repeated occurrences)
    if msg.contains("no liquidity")
        || msg.contains("liquidityavailable")
        || msg.contains("dust output")
        || msg.contains("honeypot")
        || msg.contains("spread")
        || msg.contains("direction reversal")
    {
        return ErrorCategory::Permanent;
    }

    // Transient failures (safe to retry)
    if msg.contains("timeout")
        || msg.contains("timed out")
        || msg.contains("network")
        || msg.contains("econnreset")
        || msg.contains("nonce")
        || msg.contains("max fee per gas")
        || msg.contains("replacement transaction underpriced")
        || msg.contains("429")
        || msg.contains("502")
        || msg.contains("503")
    {
        return ErrorCategory::Transient;
    }

    // Default: treat as transient (safe to retry)
    ErrorCategory::Transient
}

/// A single failure record for a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub reason: String,
    pub category: ErrorCategory,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Tracks execution failures per token for blacklisting decisions.
/// Thread-safe: wrapped in `Arc<RwLock<>>` by the engine.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FailureTracker {
    /// Token symbol → list of failure records.
    failures: HashMap<String, Vec<FailureRecord>>,
}

impl FailureTracker {
    pub fn new() -> Self {
        Self {
            failures: HashMap::new(),
        }
    }

    /// Record a failure for a token.
    pub fn record_failure(&mut self, token: &str, reason: &str, category: &ErrorCategory) {
        let record = FailureRecord {
            reason: reason.to_string(),
            category: category.clone(),
            timestamp: chrono::Utc::now(),
        };
        self.failures
            .entry(token.to_uppercase())
            .or_default()
            .push(record);
    }

    /// Check if a token is blacklisted (≥5 permanent failures in last 60 min).
    pub fn is_blacklisted(&self, token: &str) -> bool {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(60);
        if let Some(records) = self.failures.get(&token.to_uppercase()) {
            let recent_permanent = records
                .iter()
                .filter(|r| r.category == ErrorCategory::Permanent && r.timestamp > cutoff)
                .count();
            return recent_permanent >= 5;
        }
        false
    }

    /// Get remaining blacklist duration for a token (if blacklisted).
    pub fn blacklist_remaining(&self, token: &str) -> Option<chrono::Duration> {
        if !self.is_blacklisted(token) {
            return None;
        }
        if let Some(records) = self.failures.get(&token.to_uppercase()) {
            if let Some(last) = records
                .iter()
                .filter(|r| r.category == ErrorCategory::Permanent)
                .max_by_key(|r| r.timestamp)
            {
                let remaining = chrono::Duration::minutes(60)
                    - (chrono::Utc::now() - last.timestamp);
                if remaining > chrono::Duration::zero() {
                    return Some(remaining);
                }
            }
        }
        None
    }

    /// Get total failure count for a token (all categories, last 60 min).
    pub fn failure_count(&self, token: &str) -> usize {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(60);
        self.failures
            .get(&token.to_uppercase())
            .map(|records| records.iter().filter(|r| r.timestamp > cutoff).count())
            .unwrap_or(0)
    }

    /// Purge old records (older than 24 hours) to prevent unbounded growth.
    pub fn purge_old(&mut self) {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
        for records in self.failures.values_mut() {
            records.retain(|r| r.timestamp > cutoff);
        }
        self.failures.retain(|_, records| !records.is_empty());
    }
}

/// Diagnose why a pre-flight `eth_call` reverted.
/// Returns a structured reason string and recommended action.
pub fn diagnose_preflight_failure(
    err_str: &str,
    src_token: &TokenInfo,
    dst_token: &TokenInfo,
    gas_limit: u64,
) -> (String, String) {
    let err_lower = err_str.to_lowercase();

    if err_lower.contains("out of gas") || err_lower.contains("gas required exceeds allowance") {
        return (
            "Out of gas".into(),
            format!("Increase gas buffer (current: {})", gas_limit),
        );
    }

    if err_lower.contains("execution reverted") {
        // Check if it's a Permit2 issue
        if err_lower.contains("permit2") || err_lower.contains("permit") {
            return (
                "Permit2 signature mismatch".into(),
                "Re-approve token for Permit2 and retry".into(),
            );
        }

        // Check for arithmetic overflow (Panic(0x11))
        if err_lower.contains("panic(0x11)") || err_lower.contains("overflow") {
            return (
                "Arithmetic overflow — bad amount".into(),
                format!(
                    "Check amount_wei calculation for {}/{}",
                    src_token.symbol, dst_token.symbol
                ),
            );
        }

        // Generic revert — likely no liquidity or token restrictions
        return (
            "No liquidity or token restrictions".into(),
            format!(
                "Token {} may be dead/honeypot — blacklist if repeated",
                src_token.symbol
            ),
        );
    }

    if err_lower.contains("nonce") {
        return (
            "Nonce collision".into(),
            "Refresh nonce and retry".into(),
        );
    }

    if err_lower.contains("max fee per gas") || err_lower.contains("underpriced") {
        return (
            "Gas price too low".into(),
            "Increase gas price buffer".into(),
        );
    }

    (
        "Unknown error".into(),
        format!("Review error: {}", err_str),
    )
}

/// Minimal transaction receipt for on-chain verification.
pub struct TxReceipt {
    pub status: u64,
    pub gas_used: u64,
    /// FID-105: Raw receipt JSON for swap direction verification
    pub raw: Option<serde_json::Value>,
}

/// FID-105: Verify that the swap moved tokens in the expected direction.
/// Checks that src_token was sent FROM the wallet and dst_token was sent TO the wallet.
/// This catches the case where the 0x API returns calldata for the opposite direction
/// (e.g., buying AAVE with USDC instead of selling AAVE for USDC).
fn verify_swap_direction(
    receipt: &TxReceipt,
    src_token: &TokenInfo,
    dst_token: &TokenInfo,
    wallet_addr: &str,
) -> Result<(), ExecutionError> {
    let raw = receipt.raw.as_ref().ok_or_else(|| {
        ExecutionError::Other("TxReceipt missing raw data for direction verification".into())
    })?;

    let logs = raw["logs"].as_array().ok_or_else(|| {
        ExecutionError::Other("TxReceipt missing logs array".into())
    })?;

    // ERC-20 Transfer event topic0
    let transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
    let wallet_lower = wallet_addr.to_lowercase();

    let mut src_sent_from_wallet = false;
    let mut dst_received_by_wallet = false;

    for log in logs {
        let topics = match log["topics"].as_array() {
            Some(t) if t.len() >= 3 => t,
            _ => continue,
        };

        // Check if this is a Transfer event
        if topics[0].as_str() != Some(transfer_topic) {
            continue;
        }

        let from_addr = topics[1].as_str().unwrap_or("").to_lowercase();
        // Strip padding from address (topic is 32 bytes, address is 20 bytes)
        let from_addr = if from_addr.len() > 40 {
            format!("0x{}", &from_addr[from_addr.len() - 40..])
        } else {
            from_addr
        };

        let to_addr = topics[2].as_str().unwrap_or("").to_lowercase();
        let to_addr = if to_addr.len() > 40 {
            format!("0x{}", &to_addr[to_addr.len() - 40..])
        } else {
            to_addr
        };

        let log_addr = log["address"].as_str().unwrap_or("").to_lowercase();

        // Check if this log is for src_token or dst_token
        let is_src = !src_token.address.is_empty() && log_addr == src_token.address.to_lowercase();
        let is_dst = !dst_token.address.is_empty() && log_addr == dst_token.address.to_lowercase();

        if is_src && from_addr == wallet_lower {
            src_sent_from_wallet = true;
        }
        if is_dst && to_addr == wallet_lower {
            dst_received_by_wallet = true;
        }
    }

    if !src_sent_from_wallet && !dst_received_by_wallet {
        // Neither token moved from/to wallet — might be an internal transfer through executor
        // This is acceptable for 0x Permit2 swaps where the executor holds the tokens
        tracing::debug!("FID-105: No direct wallet transfers found (executor-based swap)");
        return Ok(());
    }

    if src_sent_from_wallet && !dst_received_by_wallet {
        // src left wallet but dst didn't arrive — check if dst went to executor
        tracing::warn!("FID-105: src_token left wallet but dst_token not received — possible direction reversal");
        return Err(ExecutionError::Other(format!(
            "Swap direction reversal detected: {} left wallet but {} not received. \
             The DEX API may have returned calldata for the opposite direction.",
            src_token.symbol, dst_token.symbol
        )));
    }

    if !src_sent_from_wallet && dst_received_by_wallet {
        // dst arrived but src didn't leave — this is a buy, not a sell
        tracing::warn!("FID-105: dst_token received but src_token not sent — possible direction reversal");
        return Err(ExecutionError::Other(format!(
            "Swap direction reversal detected: {} received but {} not sent. \
             The DEX API may have returned calldata for the opposite direction.",
            dst_token.symbol, src_token.symbol
        )));
    }

    tracing::debug!("FID-105: Swap direction verified: {} sent, {} received", src_token.symbol, dst_token.symbol);
    Ok(())
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
    // ---- Balance cache (P0-1a) ----
    /// Last known on-chain balance per token address — used as fallback when query returns 0.
    balance_cache: HashMap<String, f64>,
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
            balance_cache: HashMap::new(),
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
        // FID-108: Don't clear if positions have on-chain token balances (capital is deployed).
        let drift = (balance_before_sync - trader.balance).abs();
        if drift > 1.0 && !trader.positions.is_empty() {
            // Check if any position has on-chain tokens — if so, drift is expected
            // (capital is deployed into tokens, not sitting as USDC).
            let mut has_on_chain_tokens = false;
            for pos in trader.positions.values() {
                let (_, dst_token) = match resolve_pair_on_chain(
                    &pos.pair,
                    pos.side,
                    trader.chain_id,
                ) {
                    Ok(tokens) => tokens,
                    Err(_) => continue,
                };
                if !dst_token.address.is_empty() {
                    if let Some(qty) = trader
                        .query_token_balance(&dst_token.address, dst_token.decimals)
                        .await
                    {
                        if qty > 0.0001 {
                            has_on_chain_tokens = true;
                            info!(
                                "Balance drift ${:.2} but {} has {:.8} on-chain tokens — not phantom",
                                drift, dst_token.symbol, qty
                            );
                            break;
                        }
                    }
                }
            }
            if !has_on_chain_tokens {
                warn!(
                    "PHANTOM POSITIONS DETECTED: balance drift ${:.2} with {} tracked positions \
                     AND zero on-chain token balances. Clearing positions.",
                    drift,
                    trader.positions.len()
                );
                trader.positions.clear();
                trader.save_state().ok();
            }
        }

        // Additional check: if positions exist but no trades have ever completed,
        // AND the on-chain balance for the position's token is zero,
        // the positions are likely phantom from reverted swaps.
        // FID-108: Don't clear positions if on-chain balance confirms they exist.
        if !trader.positions.is_empty() && trader.closed_trades.is_empty() {
            let mut all_phantom = true;
            for pos in trader.positions.values() {
                let (_, dst_token) = match resolve_pair_on_chain(
                    &pos.pair,
                    pos.side,
                    trader.chain_id,
                ) {
                    Ok(tokens) => tokens,
                    Err(_) => continue,
                };
                if !dst_token.address.is_empty() {
                    if let Some(on_chain_qty) = trader
                        .query_token_balance(&dst_token.address, dst_token.decimals)
                        .await
                    {
                        if on_chain_qty > 0.0001 {
                            all_phantom = false;
                            info!(
                                "Position {} confirmed on-chain: {} has {:.8} tokens",
                                pos.pair, dst_token.symbol, on_chain_qty
                            );
                            break;
                        }
                    }
                }
            }
            if all_phantom {
                warn!(
                    "PHANTOM POSITIONS DETECTED: {} positions tracked but zero completed trades \
                     AND zero on-chain balances. Clearing positions — likely from reverted swaps.",
                    trader.positions.len()
                );
                trader.positions.clear();
                trader.save_state().ok();
            }
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
    /// FID-160 Fix 2: Validates the response — rejects zero buy_amount (dust/stale routes).
    pub async fn build_swap_tx(&self, params: &SwapParams) -> Result<SwapTx, ExecutionError> {
        let swap_tx = self.backend.build_swap_tx(params).await?;

        // FID-160: Reject zero buy_amount — 0x can return valid-looking
        // calldata with 0 output when liquidity is stale or route is bad.
        if let Some(ref buy) = swap_tx.buy_amount {
            let buy_f64: f64 = buy.parse().unwrap_or(0.0);
            if buy_f64 <= 0.0 {
                return Err(ExecutionError::Other(format!(
                    "FID-160: 0x returned zero buy_amount for {} -> {} — rejecting swap",
                    params.src_token, params.dst_token
                )));
            }
        }

        Ok(swap_tx)
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
        self.retry_queue.clear();
        to_retry
    }

    /// Get the number of pending retries.
    pub fn pending_retries(&self) -> usize {
        self.retry_queue.len()
    }

    // ---- FID-108: Execute with retry across multiple pairs ----

    /// Try to execute a trade across a queue of (pair, side, confidence) tuples.
    /// Returns the first successful order, or an error if all pairs fail.
    /// Records failures in the failure tracker for blacklisting.
    pub async fn execute_with_retry(
        &mut self,
        queue: &[(String, Side, f64)],
        failure_tracker: &mut FailureTracker,
    ) -> Result<(Order, String), ExecutionError> {
        let mut last_err = None;
        for (i, (pair, side, _confidence)) in queue.iter().enumerate() {
            // Check blacklist before attempting
            let base = pair.split('/').next().unwrap_or(pair);
            if failure_tracker.is_blacklisted(base) {
                let remaining = failure_tracker
                    .blacklist_remaining(base)
                    .map(|d| format!("{}min", d.num_minutes()))
                    .unwrap_or_else(|| "unknown".into());
                tracing::debug!(
                    "FID-108: Skipping blacklisted {} ({} remaining)",
                    base,
                    remaining
                );
                continue;
            }

            match self.place_order(pair, *side, 0.0, None).await {
                Ok(order) => {
                    info!(
                        "FID-108: Trade executed on {} ({}/{})",
                        pair,
                        i + 1,
                        queue.len()
                    );
                    return Ok((order, pair.clone()));
                }
                Err(e) => {
                    let category = categorize_error(&e);
                    warn!(
                        "FID-108: Execution failed for {}: {} | category={} | trying next ({}/{})",
                        pair,
                        e,
                        category,
                        i + 1,
                        queue.len()
                    );
                    failure_tracker.record_failure(base, &e.to_string(), &category);
                    last_err = Some(e);
                }
            }
        }

        // Purge old failure records to prevent unbounded growth
        failure_tracker.purge_old();

        Err(last_err.unwrap_or_else(|| {
            ExecutionError::Other(format!(
                "All {} pairs in queue skipped/blacklisted",
                queue.len()
            ))
        }))
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

    /// Sign and broadcast an EIP-1559 transaction. Returns the tx hash and full receipt.
    pub async fn sign_and_send(
        &self,
        to: Address,
        data: &[u8],
        value: U256,
        gas_limit: u64,
    ) -> Result<(String, TxReceipt), ExecutionError> {
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
                // FID-108: Structured diagnosis of pre-flight failure
                let (reason, action) = diagnose_preflight_failure(
                    &err_str,
                    &TokenInfo {
                        symbol: "pre-flight".into(),
                        address: format!("{:#x}", to),
                        decimals: 18,
                        chain_id: self.chain_id,
                    },
                    &TokenInfo {
                        symbol: "unknown".into(),
                        address: String::new(),
                        decimals: 18,
                        chain_id: self.chain_id,
                    },
                    gas_limit,
                );
                warn!(
                    "PRE-FLIGHT FAILED: eth_call reverted — {} | reason={} | action={} | gas={}",
                    err_str, reason, action, gas_limit
                );
                return Err(ExecutionError::Other(format!(
                    "Pre-flight simulation reverted: {} (reason: {}, action: {})",
                    err_str, reason, action
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

        Ok((tx_hash, receipt))
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
                    return Ok(TxReceipt { status, gas_used, raw: Some(val.clone()) });
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
        sell_entire_balance: bool,
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
            from: wallet_addr.clone(),
            chain_id: self.chain_id,
            sell_entire_balance,
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

                    // FID-160 Fix 3: When market price unavailable, reject instead of
                    // defaulting to effective_price (which makes spread=0, a tautology).
                    let market_price = if market_price_raw > 0.0 {
                        market_price_raw
                    } else {
                        log_warn!(
                            "SPREAD",
                            "Market price unavailable for {} — rejecting swap (cannot validate spread)",
                            swap_params.dst_token
                        );
                        return Err(ExecutionError::Other(format!(
                            "Market price unavailable for {} — cannot validate spread",
                            swap_params.dst_token
                        )));
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
            std::panic::AssertUnwindSafe(self.build_swap_tx(&swap_params)).catch_unwind(),
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
                Ok((hash, receipt)) => {
                    // FID-105: Verify the swap moved tokens in the expected direction.
                    verify_swap_direction(&receipt, src_token, dst_token, &wallet_addr)?;
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

        // FID-161: Retry allowance check up to 3 times with backoff.
        // On Anvil forks, the first RPC call can fail with "metadata is not found"
        // because the storage slot hasn't been cached yet. Retrying gives the fork
        // time to fetch the data from the upstream archive node.
        let mut current_allowance = U256::ZERO;
        for attempt in 1..=3u32 {
            match self.rpc_call("eth_call", call_params.clone()).await {
                Ok(result) => {
                    let hex_str = result.as_str().unwrap_or("0x0");
                    current_allowance = U256::from_str_radix(
                        hex_str.trim_start_matches("0x"), 16,
                    ).unwrap_or(U256::ZERO);
                    break;
                }
                Err(e) => {
                    if attempt < 3 {
                        warn!(
                            "Allowance check attempt {}/3 failed for {}: {}. Retrying in {}ms...",
                            attempt, token_address, e, attempt * 500
                        );
                        tokio::time::sleep(
                            std::time::Duration::from_millis(attempt as u64 * 500)
                        ).await;
                    } else {
                        warn!(
                            "Failed to check {} allowance after 3 attempts: {} — assuming zero",
                            token_address, e
                        );
                    }
                }
            }
        }

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
        let (tx_hash, _receipt) = self
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
                        on_chain_verified: false,
                        tx_hash: None,
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

    /// Internal close implementation — supports partial closes for TP scale-outs.
    /// `close_qty` is the amount of the base token to sell (may be less than position qty).
    async fn close_position_internal(
        &mut self,
        position_id: &str,
        close_qty: f64,
    ) -> Result<Order, ExecutionError> {
        let pos = self
            .positions
            .get(position_id)
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?
            .clone();

        self.order_counter += 1;

        let (src_token, dst_token) = resolve_pair_on_chain(&pos.pair,
            match pos.side {
                Side::Long => Side::Short,
                Side::Short => Side::Long,
            },
            self.chain_id,
        )?;

        if src_token.address.is_empty() {
            return Err(ExecutionError::Other(format!(
                "Cannot approve token '{}' for close — no address in local DB.",
                src_token.symbol
            )));
        }

        // FID-074 Fix 3: Query actual on-chain balance and use min(close_qty, on_chain).
        // FID-103 Fix 10: Warn when balance query fails (fallback to requested qty).
        // P0-1a: Use startup balance cache as additional fallback.
        let on_chain_balance = match self
            .query_token_balance(&src_token.address, src_token.decimals)
            .await
        {
            Some(b) if b > 0.0001 => {
                // P0-1a: Update cache with fresh balance
                self.balance_cache.insert(src_token.address.clone(), b);
                b
            }
            _ => {
                // Query returned 0 or failed — try cache
                if let Some(&cached) = self.balance_cache.get(&src_token.address) {
                    if cached > 0.0001 {
                        warn!("P0-1a: Balance query for {} returned 0/err — using cached balance {:.8}",
                            src_token.symbol, cached);
                        cached
                    } else {
                        warn!("FID-103: Balance query failed for {} — using requested qty as fallback", pos.pair);
                        close_qty
                    }
                } else {
                    warn!("FID-103: Balance query failed for {} — using requested qty as fallback", pos.pair);
                    close_qty
                }
            }
        };
        let actual_close_qty = close_qty.min(on_chain_balance);
        if actual_close_qty < close_qty {
            info!(
                "Close qty adjusted: requested={:.8} on-chain={:.8} → using {:.8}",
                close_qty, on_chain_balance, actual_close_qty
            );
        }
        let qty_wei = amount_to_wei(actual_close_qty, src_token.decimals);
        // Close rounding fix: apply 0.01% haircut to prevent f64→wei rounding
        // from exceeding on-chain balance (causes 0x to return 0 output / gasless INSUFFICIENT_BALANCE)
        let wei_val: u128 = match qty_wei.parse() {
            Ok(v) => v,
            Err(e) => {
                warn!("Close wei parse failed for {}: {} — qty_wei={}", pos.pair, e, qty_wei);
                0
            }
        };
        let safe_wei = (wei_val * 9999) / 10000;
        let qty_wei = safe_wei.to_string();

        // FID-094 Fix 5: Zero-amount swap guard — don't call 0x with 0 amount
        if actual_close_qty < 0.0001 || qty_wei == "0" {
            tracing::warn!(
                "FID-094: Close qty too small for {} — actual={:.8}, wei={}. Returning error.",
                pos.pair, actual_close_qty, qty_wei
            );
            return Err(ExecutionError::Other(format!(
                "Close quantity too small: {:.8} {} (on-chain balance may be 0)",
                actual_close_qty, src_token.symbol
            )));
        }

        let wallet_addr = format!("{:#x}", self.wallet_address);
        let price_params = super::SwapParams {
            src_token: src_token.address.clone(),
            dst_token: dst_token.address.clone(),
            amount: qty_wei.clone(),
            slippage: self.slippage_pct,
            from: wallet_addr.clone(),
            chain_id: self.chain_id,
            sell_entire_balance: true,
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

        if let Err(e) = self
            .ensure_permit2_approval(&src_token.address, &qty_wei)
            .await
        {
            log_swap_fail!("APPROVE", "Permit2 approval failed on close: {}", e);
            return Err(e);
        }

        let usdc_balance_before = self.balance;

        let tx_hash = match self.execute_swap(&src_token, &dst_token, &qty_wei, true).await {
            Ok(hash) => hash,
            Err(e) if e.to_string().contains("Dust output") => {
                warn!(
                    "Standard swap returned dust for {} — falling back to Gasless API",
                    pos.pair
                );
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

        let usdc_addr = super::usdc_address_for_chain(self.chain_id).ok_or_else(|| {
            ExecutionError::Other(format!("No USDC address for chain {}", self.chain_id))
        })?;
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

        // FID-146: Retry USDC verification 3x with backoff. On final failure OR dust
        // return ($0 USDC), trust the on-chain close tx (status=1, tx_hash confirmed)
        // and return 0.0 — the position will be removed with breakeven PnL. This prevents
        // phantom positions when RPC is flaky or when a swap returns dust.
        let verified_proceeds: f64 = {
            let mut last_err: Option<String> = None;
            let mut proceeds: f64 = 0.0;
            let mut verified = false;
            for attempt in 1..=3u32 {
                match self.rpc_call("eth_call", call_params.clone()).await {
                    Ok(rpc_result) => {
                        if let Some(hex) = rpc_result.as_str() {
                            let usdc_wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                                .unwrap_or(U256::ZERO);
                            let divisor = 10f64.powi(usdc_dec as i32);
                            let usdc_after: f64 =
                                usdc_wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                            let gained = usdc_after - usdc_balance_before;
                            if gained <= 0.0 {
                                // Dust return — close succeeded on-chain but delivered $0.
                                // This is the CATASTOPHIC case. Trust the close, log error,
                                // return 0.0 so position gets removed with breakeven PnL.
                                error!(
                                    "FID-146: Close tx {} delivered ${:.2} USDC (dust return). \
                                     Trusting on-chain close, removing position with breakeven PnL. \
                                     Tokens may be stranded — check wallet.",
                                    tx_hash, gained
                                );
                                proceeds = 0.0;
                                verified = false;
                                break;
                            }
                            if attempt > 1 {
                                info!("FID-146: USDC verification succeeded on attempt {}/3", attempt);
                            }
                            info!(
                                "On-chain verified: {} close delivered ${:.2} USDC (before=${:.2}, after=${:.2})",
                                pos.pair, gained, usdc_balance_before, usdc_after
                            );
                            proceeds = gained;
                            verified = true;
                            break;
                        } else {
                            last_err = Some("USDC balance parse failed (non-string result)".into());
                        }
                    }
                    Err(e) => {
                        last_err = Some(format!("RPC call failed: {}", e));
                    }
                }
                if attempt < 3 {
                    warn!(
                        "FID-146: USDC verification attempt {}/3 failed: {}. Retrying in 500ms...",
                        attempt,
                        last_err.as_deref().unwrap_or("unknown")
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
            if !verified {
                error!(
                    "FID-146: USDC verification FAILED after 3 attempts for {} close (tx={}). \
                     Trusting on-chain close, removing position with breakeven PnL. \
                     Last error: {}",
                    pos.pair,
                    tx_hash,
                    last_err.as_deref().unwrap_or("unknown")
                );
            }
            proceeds
        };

        // FID-074: For partial close, reduce qty instead of removing position
        let is_full_close = actual_close_qty >= pos.quantity * 0.99;
        if is_full_close {
            self.positions.remove(position_id);
        } else if let Some(p) = self.positions.get_mut(position_id) {
            p.quantity -= actual_close_qty;
        }

        info!(
            "DEX closed: {} | tx={} | qty={:.8} | verified proceeds=${:.2}",
            pos.pair, tx_hash, actual_close_qty, verified_proceeds
        );

        // FID-103 Fix 6: Use actual DEX execution price for PnL
        // FID-146: If verification failed (verified_proceeds=0), use breakeven exit_price
        // (pos.entry_price) to avoid fabricating a huge loss. Position is still removed
        // because the swap was confirmed on-chain.
        let exit_price = if actual_close_qty > 0.0001 {
            if verified_proceeds > 0.0001 {
                verified_proceeds / actual_close_qty
            } else {
                pos.entry_price // breakeven assumption when verification failed
            }
        } else {
            pos.current_price
        };
        let gross_pnl = match pos.side {
            Side::Long => (exit_price - pos.entry_price) * actual_close_qty,
            Side::Short => (pos.entry_price - exit_price) * actual_close_qty,
        };
        let fee_est = exit_price * actual_close_qty * 0.003; // FID-113: 0.3% Uniswap v3 LP fee (was 0.1%)
        let pnl = gross_pnl - fee_est;
        self.balance = usdc_balance_before + verified_proceeds;

        self.closed_trades.push(TradeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            pair: pos.pair.clone(),
            side: pos.side,
            entry_price: pos.entry_price,
            exit_price,
            quantity: actual_close_qty,
            pnl,
            pnl_pct: if pos.entry_price > 0.0 {
                pnl / (pos.entry_price * actual_close_qty) * 100.0
            } else {
                0.0
            },
            fees: fee_est,
            strategy_name: pos.strategy_name.clone(),
            opened_at: pos.opened_at,
            closed_at: Utc::now(),
            // FID-146: Audit trail — mark trades where USDC verification failed
            // so the operator can audit phantom PnL records after the fact.
            notes: if verified_proceeds <= 0.0001 {
                format!(
                    "FID-146: verification FAILED (3 retries) — PnL assumed breakeven. tx={}",
                    tx_hash
                )
            } else {
                format!("DEX close via {} — on-chain verified", self.backend.name())
            },
            on_chain_verified: true,
            tx_hash: Some(tx_hash.clone()),
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
            quantity: actual_close_qty,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(exit_price),
            tx_hash: Some(tx_hash.clone()),
        })
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

        let (src_token, dst_token) = resolve_pair_on_chain(pair, side, self.chain_id)?;
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
            .execute_swap(&src_token, &dst_token, &amount_wei, false)
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
                token_address: super::lookup_token(pair.split('/').next().unwrap_or(""), self.chain_id).map(|(addr, _)| addr).unwrap_or_default(),
            },
        );

        self.balance -= order_value * 1.003; // FID-113: 0.3% Uniswap v3 LP fee (was 0.1%)

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
        let qty = self
            .positions
            .get(position_id)
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?
            .quantity;
        self.close_position_internal(position_id, qty).await
    }

    async fn close_position_partial(
        &mut self,
        position_id: &str,
        quantity: f64,
    ) -> Result<Order, ExecutionError> {
        self.close_position_internal(position_id, quantity).await
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
        // FID-079: Only check gas on the primary trading chain — don't warn about
        // chains the user isn't actively using.
        let typical_gas_limit: u64 = 500_000;

        {
            let cid = self.chain_id;
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
        let usdc_addr = match super::usdc_address_for_chain(self.chain_id) {
            Some(addr) => addr,
            None => {
                warn!("No USDC address for chain {} — skipping balance sync", self.chain_id);
                return Ok(());
            }
        };
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
                    let hex_clean = hex.trim_start_matches("0x");
                    // FID-089: Use match instead of unwrap_or(U256::ZERO)
                    let wei = match U256::from_str_radix(hex_clean, 16) {
                        Ok(w) => w,
                        Err(e) => {
                            tracing::warn!("sync_balance: Failed to parse hex '{}' for {}: {}", hex_clean, sym, e);
                            continue;
                        }
                    };
                    let divisor = 10f64.powi(*decimals as i32);
                    let bal: f64 = wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                    if bal > 0.0001 {
                        info!("On-chain {} balance: {:.8}", sym, bal);
                        // P0-1a: Cache the balance for fallback during close
                        self.balance_cache.insert(token_addr.clone(), bal);
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
        let (src_token, dst_token) = resolve_pair_on_chain(pair, side, self.chain_id)?;
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
            if let Ok((_, asset_token)) = resolve_pair_on_chain(pair, Side::Long, self.chain_id) {
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

    // FID-096: Delegate to concrete methods for on-chain reconciliation
    async fn query_token_balance(&self, token_address: &str, decimals: u8) -> Option<f64> {
        // Delegate to the concrete impl on DexTrader
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
                    let hex_clean = hex.trim_start_matches("0x");
                    let wei = match U256::from_str_radix(hex_clean, 16) {
                        Ok(w) => w,
                        Err(e) => {
                            tracing::warn!("query_token_balance: parse failed for {}: {}", token_address, e);
                            return None;
                        }
                    };
                    let divisor = 10f64.powi(decimals as i32);
                    let balance = wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                    if balance <= 0.0 {
                        tracing::warn!("BALANCE QUERY: {} returned 0 (hex='{}', decimals={})", token_address, hex, decimals);
                    }
                    Some(balance)
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::debug!("query_token_balance failed for {}: {}", token_address, e);
                None
            }
        }
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
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
                    let hex_clean = hex.trim_start_matches("0x");
                    // FID-087 Bug D: Return None on parse failure instead of Some(0.0).
                    // The caller's unwrap_or(close_qty) will then use the requested quantity.
                    // FID-089: Add debug logging for balance query diagnosis
                    let wei = match U256::from_str_radix(hex_clean, 16) {
                        Ok(w) => w,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse balance hex '{}' for {}: {}",
                                hex_clean, token_address, e
                            );
                            return None;
                        }
                    };
                    let divisor = 10f64.powi(decimals as i32);
                    let balance = wei.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                    // FID-089: Log balance query for diagnosis
                    if balance <= 0.0 {
                        tracing::warn!(
                            "BALANCE QUERY: {} returned 0 (hex='{}', decimals={}). Token may have zero balance or RPC returned stale data.",
                            token_address, hex, decimals
                        );
                    } else {
                        tracing::debug!(
                            "BALANCE QUERY: {} = {:.8} (hex='{}', decimals={})",
                            token_address, balance, hex, decimals
                        );
                    }
                    Some(balance)
                } else {
                    tracing::warn!("BALANCE QUERY: {} returned non-string result: {:?}", token_address, result);
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
