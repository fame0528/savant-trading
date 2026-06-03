//! DexTrader — no-KYC DEX execution engine.
//!
//! Implements the [`ExecutionEngine`] trait by calling a DEX aggregator API
//! (0x or 1inch) to obtain swap calldata, signing it with the wallet's
//! private key via `k256`+`rlp`, and broadcasting the raw transaction to the
//! configured EVM chain (default: Arbitrum).

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

use crate::core::error::ExecutionError;
use crate::core::types::{Order, OrderStatus, OrderType, Position, ScaleLevel, Side, TradeRecord};
use crate::execution::engine::ExecutionEngine;

use super::{amount_to_wei, resolve_pair, DexBackend, TokenInfo};

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
    /// RPC URL.
    rpc_url: String,
    /// HTTP client for JSON-RPC.
    client: reqwest::Client,
    chain_id: u64,
    slippage_pct: f64,

    // ---- State ----
    positions: HashMap<String, Position>,
    closed_trades: Vec<TradeRecord>,
    balance: f64,
    order_counter: u64,

    // ---- Production safety (FID-018) ----
    state_path: PathBuf,
    gas_halted: bool,
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

        let mut trader = Self {
            backend,
            signing_key,
            wallet_address,
            rpc_url: rpc_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            chain_id,
            slippage_pct,
            positions: HashMap::new(),
            closed_trades: Vec::new(),
            balance: initial_balance,
            order_counter: 0,
            state_path: PathBuf::from("data/dex_state.json"),
            gas_halted: false,
        };

        // FID-018: Load persisted state on startup (crash recovery)
        let state_path = trader.state_path.clone();
        if trader.load_state(&state_path).unwrap_or(false) {
            info!("DexTrader: restored state from {:?}", state_path);
        }

        // Sync real on-chain balances immediately — don't trust config starting_balance
        let balance_before_sync = trader.balance;
        if let Err(e) = trader.sync_balance().await {
            warn!("DexTrader: initial sync_balance failed ({}), using config balance", e);
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

    // ---- JSON-RPC ----

    async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ExecutionError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1u64,
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ExecutionError::Other(format!("RPC {} failed: {}", method, e)))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::Other(format!("RPC {} parse: {}", method, e)))?;

        if let Some(err) = json.get("error") {
            return Err(ExecutionError::Other(format!(
                "RPC {} error: {}",
                method, err
            )));
        }

        json.get("result")
            .cloned()
            .ok_or_else(|| ExecutionError::Other(format!("RPC {} missing result", method)))
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

        Ok((priority, base_fee + priority))
    }

    // ---- Transaction signing & broadcasting ----

    /// Sign and broadcast an EIP-1559 transaction. Returns the tx hash.
    async fn sign_and_send(
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
        eprintln!("[SWAP] Waiting for receipt {}...", tx_hash);
        let receipt = self.wait_for_receipt(&tx_hash).await?;
        if receipt.status != 1 {
            return Err(ExecutionError::Other(format!(
                "Swap tx {} reverted on-chain (status=0)",
                tx_hash
            )));
        }
        eprintln!("[SWAP] TX confirmed on-chain: {} (gas={})", tx_hash, receipt.gas_used);

        Ok(tx_hash)
    }

    /// Poll for transaction receipt with exponential backoff.
    /// Returns error if receipt not found within 60 seconds or tx reverted.
    async fn wait_for_receipt(&self, tx_hash: &str) -> Result<TxReceipt, ExecutionError> {
        let params = serde_json::json!([tx_hash]);
        let max_attempts = 30;
        let mut delay_ms = 1000u64;

        for attempt in 0..max_attempts {
            match self.rpc_call("eth_getTransactionReceipt", params.clone()).await {
                Ok(val) => {
                    if val.is_null() {
                        // Receipt not yet available
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(5000);
                        continue;
                    }
                    let status = val["status"]
                        .as_str()
                        .and_then(|s: &str| {
                            if s == "0x1" { Some(1u64) } else { Some(0u64) }
                        })
                        .unwrap_or(0);
                    let gas_used = u64::from_str_radix(
                        val["gasUsed"].as_str().unwrap_or("0x0").trim_start_matches("0x"),
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
        };

        eprintln!("[SWAP] Calling 0x API: {} -> {} amount={}", swap_params.src_token, swap_params.dst_token, swap_params.amount);
        let swap_tx = self.backend.build_swap_tx(&swap_params).await?;
        eprintln!("[SWAP] 0x quote OK: to={} gas={} value={}", swap_tx.to, swap_tx.gas, swap_tx.value);

        let to: Address = swap_tx
            .to
            .parse()
            .map_err(|e| ExecutionError::Other(format!("Invalid 'to' '{}': {}", swap_tx.to, e)))?;

        let data = hex::decode(swap_tx.data.trim_start_matches("0x"))
            .map_err(|e| ExecutionError::Other(format!("Invalid calldata: {}", e)))?;

        let value = Self::parse_wei_value(&swap_tx.value);
        eprintln!("[SWAP] Signing tx: to={:#x} data_len={} value={} gas={}", to, data.len(), value, swap_tx.gas);
        let result = self.sign_and_send(to, &data, value, swap_tx.gas).await;
        match &result {
            Ok(hash) => eprintln!("[SWAP] TX broadcast OK: {}", hash),
            Err(e) => eprintln!("[SWAP] TX broadcast FAILED: {}", e),
        }
        result
    }

    // ---- Stop monitoring ----

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
        let order_value = entry_price * quantity;
        let amount_wei = amount_to_wei(order_value, src_token.decimals);

        info!(
            "DEX {} {} {} | src={} dst={} qty_wei={}",
            if side == Side::Long { "BUY" } else { "SELL" },
            quantity,
            pair,
            src_token.symbol,
            dst_token.symbol,
            amount_wei
        );

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
        })
    }

    async fn close_position(&mut self, position_id: &str) -> Result<Order, ExecutionError> {
        let pos = self
            .positions
            .remove(position_id)
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?;

        self.order_counter += 1;

        let (src_token, dst_token) = resolve_pair(
            &pos.pair,
            match pos.side {
                Side::Long => Side::Short,
                Side::Short => Side::Long,
            },
        )?;
        let qty_wei = amount_to_wei(pos.quantity, src_token.decimals);

        let tx_hash = self.execute_swap(&src_token, &dst_token, &qty_wei).await?;
        info!("DEX closed: {} | tx={}", pos.pair, tx_hash);

        let exit_price = pos.current_price;
        let gross_pnl = match pos.side {
            Side::Long => (exit_price - pos.entry_price) * pos.quantity,
            Side::Short => (pos.entry_price - exit_price) * pos.quantity,
        };
        let fee_est = exit_price * pos.quantity * 0.001;
        let pnl = gross_pnl - fee_est;
        // Return full proceeds: entry value + PnL
        let proceeds = pos.entry_price * pos.quantity + gross_pnl;
        self.balance += proceeds;

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
            notes: format!("DEX close via {}", self.backend.name()),
        });

        // FID-018: Persist state after position close
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
        })
    }

    fn open_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    fn balance(&self) -> f64 {
        self.balance
    }

    // ---- FID-018: Production safety overrides ----

    async fn sync_balance(&mut self) -> Result<(), ExecutionError> {
        let addr_hex = format!("{:#x}", self.wallet_address);

        // 1. Query ETH balance for gas availability
        let eth_res = self
            .rpc_call("eth_getBalance", serde_json::json!([&addr_hex, "latest"]))
            .await?;
        let eth_hex = eth_res.as_str().ok_or_else(|| {
            ExecutionError::Other("eth_getBalance returned non-string result".into())
        })?;
        let eth_wei = U256::from_str_radix(eth_hex.trim_start_matches("0x"), 16)
            .map_err(|e| ExecutionError::Other(format!("Invalid ETH balance hex: {}", e)))?;
        let eth_balance: f64 = eth_wei.to_string().parse().unwrap_or(0.0) / 1e18;

        // ETH gas check — need enough for 2 close transactions (~0.002 ETH on Arbitrum)
        const ETH_MIN: f64 = 0.002;
        if eth_balance < ETH_MIN {
            self.gas_halted = true;
            error!(
                "CRITICAL: ETH gas balance too low ({:.6} ETH). \
                 Need at least {:.4} ETH for 2 close txs. HALTING ALL TRADING.",
                eth_balance, ETH_MIN
            );
        } else if self.gas_halted {
            // Was halted, now OK — resume
            self.gas_halted = false;
            info!(
                "ETH gas balance restored ({:.6} ETH) — resuming trading",
                eth_balance
            );
        }

        // 2. Query USDC balance via eth_call to balanceOf()
        // USDC on Arbitrum: 0xaf88d065e77c8cC2239327C5EDb3A432268e5831
        // balanceOf selector: 0x70a08231 (keccak256("balanceOf(address)")[..4])
        const USDC_ADDRESS: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
        let padded_addr = format!("{:0>64}", addr_hex.trim_start_matches("0x"));
        let call_data = format!("0x70a08231{}", padded_addr);

        let call_params = serde_json::json!([{
            "to": USDC_ADDRESS,
            "data": call_data
        }, "latest"]);

        match self.rpc_call("eth_call", call_params).await {
            Ok(result) => {
                if let Some(hex) = result.as_str() {
                    let usdc_wei = U256::from_str_radix(hex.trim_start_matches("0x"), 16)
                        .unwrap_or(U256::ZERO);
                    let usdc_balance: f64 =
                        usdc_wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e6;

                    let drift = (usdc_balance - self.balance).abs();
                    if drift > 1.0 {
                        warn!(
                            "USDC balance drift detected: tracked={:.2} actual={:.2} (diff=${:.2})",
                            self.balance, usdc_balance, drift
                        );
                    }

                    self.balance = usdc_balance;
                }
            }
            Err(e) => {
                warn!("Failed to sync USDC balance ({}), using tracked balance", e);
                // Don't fail — use locally tracked balance as fallback
            }
        }

        // Persist synced balance
        let _ = self.save_state();

        Ok(())
    }

    async fn place_stop_loss(&mut self, position_id: &str) -> Result<(), ExecutionError> {
        warn!(
            "DEX stop-loss is CLIENT-SIDE only (position_id={}) — NOT exchange-guaranteed. \
             Persisting SL target for crash recovery.",
            position_id
        );

        // The stop_loss value is already set on the Position when created.
        // Persist state to ensure SL target survives a crash/restart.
        self.save_state().map_err(|e| {
            warn!("Failed to persist stop-loss state: {}", e);
            e
        })?;

        info!(
            "Stop-loss persisted for {} (client-side monitoring)",
            position_id
        );
        Ok(())
    }
}
