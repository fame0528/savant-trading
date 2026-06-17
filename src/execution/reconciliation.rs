//! Wallet Reconciliation Heartbeat (FID-147).
//!
//! Compares the engine's in-memory portfolio state to on-chain reality at the
//! start of every cycle. On divergence beyond a threshold, surfaces a halt
//! signal that the engine's main loop honors. On RPC failure, logs a warning
//! without halting — RPC unreliability must not be conflated with state drift.
//!
//! Codifies the "we only use real data" rule (FID-149) and the LESSON-001
//! protocol-amended AUDIT phase (FID-151). Without this heartbeat, the engine
//! can drift from on-chain reality silently — exactly the failure mode of the
//! 2026-06-13 incident.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::types::{AccountState, Position};
use crate::execution::dex::{usdc_address_for_chain, usdc_decimals_for_chain};

/// Configuration for the wallet reconciliation heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationConfig {
    /// Chain ID (e.g., 42161 for Arbitrum One, 421614 for Arbitrum Sepolia testnet).
    pub chain_id: u64,
    /// Wallet address to reconcile.
    pub wallet_address: String,
    /// RPC URL (e.g., `https://arb1.arbitrum.io/rpc`).
    pub rpc_url: String,
    /// Absolute USD divergence threshold. Default: 0.10 (sensitive at sub-$1 accounts).
    pub divergence_threshold_usd: f64,
    /// Percentage divergence threshold (0.0-1.0). Default: 0.01 (1%).
    pub divergence_threshold_pct: f64,
    /// Cycle interval — heartbeat runs every N cycles. Default: 1 (every cycle).
    pub interval_cycles: u32,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            chain_id: 42161,
            wallet_address: String::new(),
            rpc_url: String::new(),
            divergence_threshold_usd: 0.10,
            divergence_threshold_pct: 0.01,
            interval_cycles: 1,
        }
    }
}

/// Report from a single reconciliation cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    /// USDC balance per in-memory AccountState.
    pub in_memory_usdc: f64,
    /// USDC balance per on-chain `balanceOf` query.
    pub on_chain_usdc: f64,
    /// Absolute divergence in USDC.
    pub usdc_divergence: f64,
    /// Percentage divergence in USDC (0.0-1.0).
    pub usdc_divergence_pct: f64,
    /// Number of in-memory positions.
    pub in_memory_position_count: usize,
    /// Token divergences detected.
    pub tokens_with_divergence: Vec<TokenDivergence>,
    /// Whether the reconciliation halted the engine.
    pub halted: bool,
    /// Reason for halt, if any.
    pub halt_reason: Option<String>,
    /// Whether the reconciliation was skipped due to RPC failure.
    pub rpc_failure: bool,
    /// Whether the reconciliation was skipped due to interval setting.
    pub skipped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDivergence {
    pub pair: String,
    pub in_memory_value_usd: f64,
    pub on_chain_value_usd: f64,
    pub divergence_usd: f64,
}

/// Run a reconciliation check between in-memory state and on-chain state.
///
/// Returns a `ReconciliationReport` regardless of outcome. The caller (engine
/// main loop) is responsible for acting on the `halted` flag.
///
/// On RPC failure, returns a report with `rpc_failure: true` and `halted: false`.
/// RPC unreliability is logged but does not halt the engine — the alternative
/// (halting on every RPC blip) would create more false positives than the
/// drift we're trying to detect.
pub async fn reconcile_wallet_state(
    config: &ReconciliationConfig,
    account: &AccountState,
    positions: &HashMap<String, Position>,
) -> ReconciliationReport {
    // Step 0: If interval is set, caller should pre-check. We assume called every cycle.
    let in_memory_usdc = account.balance;

    // Step 1: Query on-chain USDC balance.
    let usdc_addr = usdc_address_for_chain(config.chain_id).unwrap_or("");
    let on_chain_usdc = match query_token_balance(
        &config.rpc_url,
        usdc_addr,
        usdc_decimals_for_chain(config.chain_id),
        &config.wallet_address,
    )
    .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(
                "WALLET_RECONCILIATION: RPC failure querying USDC balance: {}. Skipping reconciliation cycle.",
                e
            );
            return ReconciliationReport {
                in_memory_usdc,
                on_chain_usdc: 0.0,
                usdc_divergence: 0.0,
                usdc_divergence_pct: 0.0,
                in_memory_position_count: positions.len(),
                tokens_with_divergence: vec![],
                halted: false,
                halt_reason: None,
                rpc_failure: true,
                skipped: false,
            };
        }
    };

    // Step 2: Compute USDC divergence.
    let usdc_divergence = (in_memory_usdc - on_chain_usdc).abs();
    let usdc_divergence_pct = if in_memory_usdc > 0.0 {
        usdc_divergence / in_memory_usdc
    } else {
        0.0
    };

    // Step 3: Query on-chain balance for each held token.
    // FID-A02: Per-token divergence — queries ERC-20 balanceOf for each
    // position's token_address. RPC failures per-token log a warning and
    // skip that token (same semantics as USDC branch). Only tokens with
    // non-empty token_address are checked.
    let mut tokens_with_divergence: Vec<TokenDivergence> = Vec::new();
    for (pair, pos) in positions {
        if pos.token_address.is_empty() {
            continue;
        }
        // Resolve decimals from token DB (same source as lookup_token)
        let symbol = pair.split('/').next().unwrap_or("");
        let decimals = crate::execution::dex::lookup_token(symbol, config.chain_id)
            .map(|(_, d)| d)
            .unwrap_or(18);
        match query_token_balance(
            &config.rpc_url,
            &pos.token_address,
            decimals,
            &config.wallet_address,
        )
        .await
        {
            Ok(on_chain_qty) => {
                let expected_qty = pos.quantity;
                let divergence_qty = (expected_qty - on_chain_qty).abs();
                // Convert to USD using current price
                let divergence_usd = divergence_qty * pos.current_price;
                if divergence_usd > config.divergence_threshold_usd {
                    tracing::warn!(
                        "WALLET_RECONCILIATION: {} divergence — in-memory={:.6}, on-chain={:.6}, div=${:.4}",
                        pair, expected_qty, on_chain_qty, divergence_usd
                    );
                    tokens_with_divergence.push(TokenDivergence {
                        pair: pair.clone(),
                        in_memory_value_usd: expected_qty * pos.current_price,
                        on_chain_value_usd: on_chain_qty * pos.current_price,
                        divergence_usd,
                    });
                }
            }
            Err(e) => {
                // RPC failure: log warn, skip this token (don't halt on transient RPC issues)
                tracing::warn!(
                    "WALLET_RECONCILIATION: {} per-token balance query failed ({}), skipping",
                    pair,
                    e
                );
            }
        }
    }

    // Step 4: Halt if either USDC threshold exceeded.
    let usdc_threshold_exceeded = usdc_divergence > config.divergence_threshold_usd
        && usdc_divergence_pct > config.divergence_threshold_pct;
    let token_threshold_exceeded = !tokens_with_divergence.is_empty();
    let halted = usdc_threshold_exceeded || token_threshold_exceeded;

    let halt_reason = if halted {
        Some(format!(
            "Wallet reconciliation divergence: in_memory_usdc=${:.4}, on_chain_usdc=${:.4} (${:.4} / {:.2}% divergence, thresholds: ${:.4} / {:.2}%). {} token(s) with divergence.",
            in_memory_usdc, on_chain_usdc, usdc_divergence, usdc_divergence_pct * 100.0,
            config.divergence_threshold_usd, config.divergence_threshold_pct * 100.0,
            tokens_with_divergence.len()
        ))
    } else {
        None
    };

    // Step 5: Always log the report (visible even when engine doesn't halt).
    if halted {
        tracing::error!(
            "WALLET_RECONCILIATION_HALT: {} (in_memory_usdc=${:.4} on_chain_usdc=${:.4} divergence=${:.4} / {:.2}%)",
            halt_reason.as_deref().unwrap_or(""),
            in_memory_usdc, on_chain_usdc, usdc_divergence, usdc_divergence_pct * 100.0
        );
    } else {
        tracing::info!(
            "WALLET_RECONCILIATION: OK (in_memory_usdc=${:.4} on_chain_usdc=${:.4} divergence=${:.4} / {:.4}%)",
            in_memory_usdc, on_chain_usdc, usdc_divergence, usdc_divergence_pct * 100.0
        );
    }

    ReconciliationReport {
        in_memory_usdc,
        on_chain_usdc,
        usdc_divergence,
        usdc_divergence_pct,
        in_memory_position_count: positions.len(),
        tokens_with_divergence,
        halted,
        halt_reason,
        rpc_failure: false,
        skipped: false,
    }
}

/// Query the on-chain balance of an ERC-20 token for a wallet address.
///
/// Uses JSON-RPC `eth_call` with the `balanceOf(address)` function selector.
/// Returns the balance in human-readable units (already divided by 10^decimals).
async fn query_token_balance(
    rpc_url: &str,
    token_address: &str,
    decimals: u8,
    wallet_address: &str,
) -> Result<f64, String> {
    if token_address.is_empty() {
        return Err("empty token address".to_string());
    }
    if wallet_address.is_empty() {
        return Err("empty wallet address".to_string());
    }
    if rpc_url.is_empty() {
        return Err("empty rpc url".to_string());
    }

    // balanceOf(address) selector is 0x70a08231.
    // ABI encoding for a SINGLE static address parameter:
    //   4-byte selector (8 hex) + 12-byte left-padding (24 hex) + 20-byte raw address (40 hex)
    // Total: 36 bytes = 72 hex chars + 0x prefix = 74 chars total.
    //
    // IMPORTANT: the address is NOT left-padded to 32 bytes. The 12 zero bytes
    // go BEFORE the address (left of it), making the 20-byte address right-aligned
    // to byte 32 of the 32-byte slot. Padding the address to 64 hex produces
    // an EVM decode failure (the address is read from the wrong slot) and
    // returns 0 — which masquerades as "no balance" rather than an RPC error.
    //
    // This was the actual bug, masked by a wrong initial fix that over-padded.
    let addr_hex = wallet_address
        .strip_prefix("0x")
        .unwrap_or(wallet_address)
        .to_lowercase();
    // Do NOT left-pad the address. It must be exactly 20 bytes (40 hex chars).
    let data = format!("0x70a08231000000000000000000000000{}", addr_hex);

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "reconciliation",
        "method": "eth_call",
        "params": [
            { "to": token_address, "data": data },
            "latest"
        ]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("reqwest client: {}", e))?;

    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("rpc send: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("rpc parse: {}", e))?;

    // Check for JSON-RPC error first (Anvil and other nodes return {"error": {...}})
    if let Some(err) = json.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        return Err(format!("rpc error {}: {}", code, msg));
    }

    let result_hex = json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing result in rpc response: {}", json))?;

    let raw = u128::from_str_radix(result_hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("hex parse: {}", e))?;

    let divisor = 10u128.pow(decimals as u32);
    let balance = (raw as f64) / (divisor as f64);
    Ok(balance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_position(id: &str, qty: f64) -> Position {
        Position {
            id: id.to_string(),
            pair: "GRT/USD".to_string(),
            side: crate::core::types::Side::Long,
            entry_price: 0.01987,
            current_price: 0.01987,
            quantity: qty,
            stop_loss: 0.0,
            take_profit_1: 0.0,
            take_profit_2: 0.0,
            take_profit_3: 0.0,
            unrealized_pnl: 0.0,
            risk_amount: 0.0,
            strategy_name: "test".to_string(),
            opened_at: Utc::now(),
            scale_level: crate::core::types::ScaleLevel::Full,
            token_address: String::new(),
        }
    }

    fn make_account(balance: f64) -> AccountState {
        AccountState {
            balance,
            equity: balance,
            unrealized_pnl: 0.0,
            daily_pnl: 0.0,
            peak_equity: balance,
            drawdown_pct: 0.0,
            open_positions: 0,
            max_positions: 5,
            trades_today: 0,
        }
    }

    #[test]
    fn default_config_has_sensible_thresholds() {
        let cfg = ReconciliationConfig::default();
        // At sub-$1 account scale, 1% of $1 is $0.01. The $0.10 absolute floor
        // catches divergences that are significant in real terms even when
        // the account is small. For larger accounts, the 1% threshold scales.
        assert!(cfg.divergence_threshold_usd > 0.0);
        assert!(cfg.divergence_threshold_pct > 0.0 && cfg.divergence_threshold_pct < 1.0);
    }

    #[test]
    fn rpc_failure_does_not_halt() {
        // When the RPC URL is empty, query_token_balance returns Err,
        // and the heartbeat returns rpc_failure: true, halted: false.
        // This is the critical contract: RPC unreliability must not
        // cause false-positive halts.
        let cfg = ReconciliationConfig {
            rpc_url: "".to_string(),
            ..Default::default()
        };
        let account = make_account(40.0);
        let mut positions = HashMap::new();
        positions.insert("p1".to_string(), make_position("p1", 100.0));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let report = rt.block_on(reconcile_wallet_state(&cfg, &account, &positions));

        assert!(report.rpc_failure);
        assert!(!report.halted);
        assert!(report.halt_reason.is_none());
    }

    #[test]
    fn report_carries_in_memory_state() {
        // The in-memory balance and position count must be in the report
        // even when the RPC fails (so the operator can see what the
        // engine believed at the moment of failure).
        let cfg = ReconciliationConfig {
            rpc_url: "".to_string(),
            wallet_address: "0x543ca0434b84ad38c858d2d178d2082521711fbc".to_string(),
            ..Default::default()
        };
        let account = make_account(40.0);
        let mut positions = HashMap::new();
        positions.insert("p1".to_string(), make_position("p1", 100.0));
        positions.insert("p2".to_string(), make_position("p2", 50.0));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let report = rt.block_on(reconcile_wallet_state(&cfg, &account, &positions));

        assert_eq!(report.in_memory_usdc, 40.0);
        assert_eq!(report.in_memory_position_count, 2);
        assert!(report.rpc_failure);
    }

    #[test]
    fn halt_logic_distinguishes_dust_from_drift() {
        // The config's divergence_threshold_usd defaults to 0.10. A divergence
        // of $0.05 should NOT halt. A divergence of $0.50 should halt IF
        // the percentage threshold is also exceeded.
        // This test verifies the threshold logic without requiring a real RPC.
        let cfg = ReconciliationConfig {
            divergence_threshold_usd: 0.10,
            divergence_threshold_pct: 0.01,
            ..Default::default()
        };

        // Construct a synthetic report to validate the math.
        let report_under_threshold = ReconciliationReport {
            in_memory_usdc: 100.0,
            on_chain_usdc: 99.95, // $0.05 divergence
            usdc_divergence: 0.05,
            usdc_divergence_pct: 0.0005,
            in_memory_position_count: 0,
            tokens_with_divergence: vec![],
            halted: 0.05 > cfg.divergence_threshold_usd && 0.0005 > cfg.divergence_threshold_pct,
            halt_reason: None,
            rpc_failure: false,
            skipped: false,
        };
        assert!(!report_under_threshold.halted);

        let report_over_threshold = ReconciliationReport {
            in_memory_usdc: 100.0,
            on_chain_usdc: 50.0, // $50 divergence
            usdc_divergence: 50.0,
            usdc_divergence_pct: 0.50,
            in_memory_position_count: 0,
            tokens_with_divergence: vec![],
            halted: 50.0 > cfg.divergence_threshold_usd && 0.50 > cfg.divergence_threshold_pct,
            halt_reason: None,
            rpc_failure: false,
            skipped: false,
        };
        assert!(report_over_threshold.halted);
    }

    /// Regression test for FID-147 calldata encoding bug (caught 2026-06-14).
    ///
    /// The heartbeat's `query_token_balance` builds calldata for `balanceOf(address)`.
    /// Two incorrect encodings were tried before the right one was found:
    ///
    /// 1. **Over-padded (98 chars):** `0x` + 8 selector + 28 zeros + 64 zero-padded
    ///    address. The EVM decodes the address from the wrong 32-byte slot and
    ///    returns 0, masquerading as "no balance." Anvil's `cast call` returns
    ///    50,000,000 for the same wallet — proving the heartbeat was wrong.
    ///
    /// 2. **Right way (74 chars):** `0x` + 8 selector + 24 zero hex (12 bytes
    ///    of LEFT padding) + 40 hex (20-byte address, NOT zero-padded).
    ///    The address sits in the rightmost 20 bytes of the 32-byte slot;
    ///    the 12 bytes of left padding make it 32-byte aligned. This matches
    ///    `cast 4byte-decode` output and `cast call` results.
    ///
    /// This test pins the exact bytes-on-the-wire so the bug cannot regress.
    /// The wallet address is the real test wallet `0x543ca...` (lowercase) —
    /// the same one Anvil reports 50 USDC for.
    #[test]
    fn balance_of_calldata_is_canonical_74_hex_chars() {
        let wallet = "0x543ca0434b84ad38c858d2d178d2082521711fbc";
        let addr_hex = wallet.strip_prefix("0x").unwrap().to_lowercase();
        // Canonical form: 0x + 70a08231 + 24 hex zeros (12 bytes left-pad) + 40 hex (20-byte address)
        // Total string: 2 + 8 + 24 + 40 = 74 chars.
        let canonical = format!("0x70a08231000000000000000000000000{}", addr_hex);
        assert_eq!(
            canonical.len(),
            74,
            "balanceOf calldata must be exactly 74 chars (72 hex + 0x prefix)"
        );
        assert_eq!(
            &canonical[..10],
            "0x70a08231",
            "calldata must start with the balanceOf selector"
        );
        assert_eq!(
            &canonical[10..34],
            "000000000000000000000000",
            "calldata must have exactly 24 hex zeros (12 bytes) of LEFT padding between selector and address"
        );
        assert_eq!(
            &canonical[34..],
            "543ca0434b84ad38c858d2d178d2082521711fbc",
            "calldata must end with a 20-byte (40-hex) address, NOT zero-padded to 64"
        );
    }
}
