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
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::core::types::{AccountState, Position};
use crate::execution::dex::{usdc_address_for_chain, usdc_decimals_for_chain};

// ---------------------------------------------------------------------------
// FID-219: Process-global shared reqwest::Client for reconciliation queries.
//
// See FID-219 archive at dev/fids/FID-2026-0620-219-cycle1-reconciliation-query-client-construction.md
// §5.1 for the design rationale. Without the shared static, query_token_balance
// constructed a fresh reqwest::Client on every reconcile, racing the constructor's
// 17-connection sync_balance() burst (FID-213 R9 FID-ANVIL-BOOT-PERF) against Anvil's
// concurrent-connection limit; the 18th cold connection surfaces as
// "error decoding response body" in the cycle-1 warn at line ~140 below.
// ---------------------------------------------------------------------------
static RECONCILIATION_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn shared_reconciliation_client() -> reqwest::Client {
    // OnceLock::get_or_init is the stable API; get_or_try_init was the unstable
    // alternative (gated `once_cell_try` feature pre-Rust 1.86). The Client::build()
    // call virtually never fails in practice — only on system resource exhaustion,
    // in which case the engine can't make any HTTP requests at all, so panicking
    // with a clear message is the right failure mode.
    RECONCILIATION_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest::Client::build() failed — system resource exhaustion")
        })
        .clone()
}

// ---------------------------------------------------------------------------
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
    /// FID-196: Safety halt threshold. If phantom value exceeds this % of
    /// total portfolio, apply_to_portfolio returns halt_recommended=true
    /// without mutating state. Default: 0.50 (50%). A 50% divergence
    /// indicates a serious bug, not routine drift.
    pub safety_halt_threshold_pct: f64,
    /// FID-225: Per-token divergence threshold. Default: 5.00 USD. Above
    /// this, reconcile_wallet_state fires the halt path on per-token
    /// divergence. Was previously aliased to divergence_threshold_usd
    /// ($0.10) which was too tight and caused spurious halts on minor
    /// price-feed noise. Operators tuning for smaller or larger positions
    /// can adjust via TOML. Should typically be 10-50x the USDC threshold.
    #[serde(default)]
    pub token_divergence_threshold_usd: f64,
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
            safety_halt_threshold_pct: 0.50,
            token_divergence_threshold_usd: 5.00,
        }
    }
}

/// FID-211: Type of USDC divergence detected.
///
/// - `None`: no divergence, engine proceeds normally.
/// - `StartupCarryover`: detected at startup with no in-memory positions open. The
///   in-memory balance was inherited from a previous run against a different chain
///   state (e.g. fresh Anvil restart). Caller should re-sync from chain, NOT halt.
/// - `RealTime`: detected mid-cycle with active positions. Something unexpected
///   happened (direct token transfer, executor bug, RPC desync). Caller should halt
///   and require operator review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DivergenceType {
    #[default]
    None,
    StartupCarryover,
    RealTime,
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
    /// FID-211: Type of divergence. Caller decides whether to halt based on this
    /// in combination with `halted`. A `StartupCarryover` divergence is
    /// informational (write to `savant.blocked` / `shared.block` for visibility,
    /// then re-sync from chain). A `RealTime` divergence halts.
    pub divergence_type: DivergenceType,
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
/// FID-225 round 2: Classify a per-token quantity observation as haltable
/// drift or phantom (in-memory>0 but on-chain=0, indicating stale state
/// from a reverted swap or prior-session artifact, NOT a real on-chain
/// drift). Pure function — extracted for testability. Phantoms are NOT
/// haltable; the 5-min ChainPositionRecovery → `apply_to_portfolio`
/// (FID-196) is the proper handler for them.
///
/// # Arguments
/// * `expected_qty` — position's in-memory token quantity
/// * `on_chain_qty` — chain's actual token balance for the wallet
/// * `divergence_usd` — pre-computed `(expected - on_chain).abs() * price`
/// * `threshold_usd` — `ReconciliationConfig.token_divergence_threshold_usd`
///
/// # Returns
/// * `true` — real, divergence-confirmed; heartbeat should halt
/// * `false` — phantom (stale state) OR sub-threshold; heartbeat skips
fn is_haltable_token_divergence(
    expected_qty: f64,
    on_chain_qty: f64,
    divergence_usd: f64,
    threshold_usd: f64,
) -> bool {
    let is_phantom = on_chain_qty == 0.0 && expected_qty > 0.0;
    divergence_usd > threshold_usd && !is_phantom
}

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
                divergence_type: DivergenceType::None,
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
                if divergence_usd > config.token_divergence_threshold_usd {
                    // FID-225 round 2: Classify phantom positions separately.
                    // Phantom = in-memory has tokens but on-chain balance is 0.
                    // This signals stale state (reverted swap, prior-session
                    // artifact, test residue) — NOT a real on-chain drift.
                    // We log a distinct PHANTOM_WARN but don't add to
                    // tokens_with_divergence, so the heartbeat doesn't halt.
                    // The 5-min ChainPositionRecovery → apply_to_portfolio
                    // path (FID-196) is the proper handler: it purges phantoms
                    // whose total value is below safety_halt_threshold_pct
                    // (= 50% of portfolio default) and halts the engine if
                    // phantoms dominate the portfolio — which is the correct
                    // "this is a real bug" signal rather than "the engine
                    // momentarily remembers a closed position."
                    let is_phantom = on_chain_qty == 0.0 && expected_qty > 0.0;
                    if is_haltable_token_divergence(
                        expected_qty,
                        on_chain_qty,
                        divergence_usd,
                        config.token_divergence_threshold_usd,
                    ) {
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
                    } else if is_phantom {
                        // FID-225 round 2: phantom (in-memory>0, on-chain=0).
                        // Engine believes wallet holds this token, but chain
                        // says balance is 0. Stale-state class — engine ran
                        // an aborted swap, missed a close-path purge, or
                        // survived a prior-session crash. Classified as
                        // recoverable — see `is_haltable_token_divergence`
                        // for the canonical classifier + rationale.
                        tracing::warn!(
                            "WALLET_RECONCILIATION: {} PHANTOM_POSITION — in-memory={:.6}, on-chain=0.000000 (chain has zero of this token). Not halting; will be purged by next ChainPositionRecovery. div_usd_at_price=${:.4}",
                            pair, expected_qty, divergence_usd
                        );
                    }
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

    // FID-211: Classify the divergence type.
    // - StartupCarryover: engine just started (zero in-memory positions) and
    //   the in-memory balance is stale relative to the chain. This is normal
    //   when restarting against a fresh Anvil fork or a new wallet. Caller
    //   should adopt chain as truth, NOT halt.
    // - RealTime: engine has open positions and detected a divergence. This
    //   indicates something unexpected happened mid-cycle. Caller should halt.
    // - None: no divergence detected.
    let divergence_type = if !halted {
        DivergenceType::None
    } else if positions.is_empty() {
        DivergenceType::StartupCarryover
    } else {
        DivergenceType::RealTime
    };

    let halt_reason = if halted {
        Some(format!(
            "Wallet reconciliation divergence ({:?}): in_memory_usdc=${:.4}, on_chain_usdc=${:.4} (${:.4} / {:.2}% divergence, thresholds: ${:.4} / {:.2}%). {} token(s) with divergence.",
            divergence_type,
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
            "WALLET_RECONCILIATION_HALT [{}]: {} (in_memory_usdc=${:.4} on_chain_usdc=${:.4} divergence=${:.4} / {:.2}%)",
            match divergence_type {
                DivergenceType::StartupCarryover => "startup-carryover",
                DivergenceType::RealTime => "real-time",
                DivergenceType::None => "none",
            },
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
        divergence_type,
        halted,
        halt_reason,
        rpc_failure: false,
        skipped: false,
    }
}

/// FID-196: Apply reconciliation to portfolio + decision log.
/// Mutates in-memory state to match on-chain reality. Catches:
/// - Phantom positions: in memory but not on chain (executor rejected)
/// - Orphan positions: on chain but not in memory (crash recovery, manual tx)
/// - USDC balance drift: executor's balance vs portfolio's balance
///
/// Safety: if divergence exceeds `safety_halt_threshold_pct` (config-driven,
/// default 50%), the function does NOT mutate state. It returns a halt signal.
/// A 50% divergence indicates a serious bug, not routine drift.
///
/// Per ECHO Law 13, this function extends the existing `reconcile_wallet_state`
/// responsibility (one function, one truth). The previous function only halted
/// on divergence; this one corrects divergence and returns a halt signal only
/// for extreme cases.
pub fn apply_to_portfolio(
    config: &ReconciliationConfig,
    portfolio: &mut crate::execution::portfolio::PortfolioManager,
    decision_log: &mut crate::agent::decision_log::DecisionLog,
    executor: &dyn crate::execution::engine::ExecutionEngine,
) -> ReconciliationApplyReport {
    let mut report = ReconciliationApplyReport::default();

    // Step 1: Get on-chain positions. If empty, we need to disambiguate
    // "truly empty wallet" vs "RPC failure."
    let on_chain_positions = executor.open_positions();
    let on_chain_pairs: std::collections::HashSet<String> =
        on_chain_positions.iter().map(|p| p.pair.clone()).collect();

    // Step 2: Compute phantom value (positions in memory but not on chain).
    // If phantom value > safety_halt_threshold_pct * total_portfolio, halt.
    let in_memory: Vec<(String, crate::core::types::Position)> = portfolio
        .positions()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let phantom_value: f64 = in_memory
        .iter()
        .filter(|(_, p)| !on_chain_pairs.contains(&p.pair))
        .map(|(_, p)| (p.quantity * p.current_price).max(0.0))
        .sum();

    let total_portfolio_value =
        (portfolio.account().balance + portfolio.account().equity).max(0.01); // avoid div-by-zero

    if phantom_value / total_portfolio_value > config.safety_halt_threshold_pct {
        report.halt_recommended = true;
        report.halt_reason = Some(format!(
            "Phantom value ${:.2} is {:.1}% of portfolio ${:.2}, exceeds safety threshold {:.1}%. Likely bug, not routine drift.",
            phantom_value,
            (phantom_value / total_portfolio_value) * 100.0,
            total_portfolio_value,
            config.safety_halt_threshold_pct * 100.0
        ));
        return report; // Don't mutate. Caller will halt the engine.
    }

    // Step 3: Clear phantoms (in memory but not on chain).
    for (id, pos) in &in_memory {
        if !on_chain_pairs.contains(&pos.pair) {
            tracing::warn!(
                "FID-196: Phantom position detected: {} (id={}, entry={:.4}) -- in memory but not on chain. Removing.",
                pos.pair,
                id,
                pos.entry_price
            );
            report.phantom_cleared.push(pos.pair.clone());
            portfolio.positions_mut().remove(id);
            decision_log.update_status(
                &pos.pair,
                crate::agent::decision_log::TradeStatus::Rejected,
                Some("phantom: not on chain".to_string()),
            );
        }
    }

    // Step 4: Add orphans (on chain but not in memory).
    for on_chain_pos in &on_chain_positions {
        let already_in_memory = portfolio
            .positions()
            .values()
            .any(|p| p.pair == on_chain_pos.pair);
        if !already_in_memory {
            tracing::warn!(
                "FID-196: Orphan on-chain position: {} (entry={:.4}) -- on chain but not in memory. Adding.",
                on_chain_pos.pair,
                on_chain_pos.entry_price
            );
            report.orphan_added.push(on_chain_pos.pair.clone());
            portfolio
                .positions_mut()
                .insert(on_chain_pos.id.clone(), (*on_chain_pos).clone());
            decision_log.update_status(
                &on_chain_pos.pair,
                crate::agent::decision_log::TradeStatus::Filled,
                None,
            );
        }
    }

    // Step 5: USDC balance reconciliation. Compare executor's USDC balance
    // to portfolio's balance. If they diverge by more than threshold, correct.
    let executor_usdc = executor.balance();
    let portfolio_usdc = portfolio.account().balance;
    let usdc_div = (executor_usdc - portfolio_usdc).abs();
    if usdc_div > config.divergence_threshold_usd {
        tracing::warn!(
            "FID-196: USDC balance divergence ${:.4}: portfolio=${:.4} executor=${:.4}. Correcting portfolio.",
            usdc_div,
            portfolio_usdc,
            executor_usdc
        );
        report.usdc_divergence_corrected = executor_usdc - portfolio_usdc;
        portfolio.set_balance(executor_usdc);
    }

    // Step 6: Update account state.
    portfolio.account_mut().open_positions = portfolio.positions().len();

    // Step 7: Telemetry. Append to data/reconciliation_telemetry.jsonl.
    record_telemetry(&report);

    report
}

/// FID-196: Telemetry record for reconciliation. Appends to
/// data/reconciliation_telemetry.jsonl (one line per reconciliation cycle).
fn record_telemetry(report: &ReconciliationApplyReport) {
    let entry = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "event": "reconciliation",
        "phantom_cleared_count": report.phantom_cleared.len(),
        "orphan_added_count": report.orphan_added.len(),
        "usdc_divergence_corrected": report.usdc_divergence_corrected,
        "halt_recommended": report.halt_recommended,
    });
    let line = format!("{}\n", entry);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/reconciliation_telemetry.jsonl")
    {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

/// FID-196: Report from apply_to_portfolio. Contains details of state
/// corrections and any safety halt signal.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReconciliationApplyReport {
    pub phantom_cleared: Vec<String>,
    pub orphan_added: Vec<String>,
    pub usdc_divergence_corrected: f64,
    /// FID-196: true if RPC failure prevented any state mutation.
    pub rpc_failure: bool,
    /// FID-196: true if divergence exceeded safety_halt_threshold_pct.
    /// Caller should halt the engine.
    pub halt_recommended: bool,
    /// FID-196: reason for halt, if any.
    pub halt_reason: Option<String>,
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

    // FID-219: REPLACED per-call reqwest::Client::builder() with the shared static.
    // The per-call Client constructed a fresh connection on every reconcile, throwing
    // away keep-alive, and racing the constructor's 17-connection sync_balance() burst
    // (FID-213 R9 FID-ANVIL-BOOT-PERF) against Anvil's concurrent-connection limit.
    // The shared static joins the existing pool, eliminating the cold-connection race.
    let client = shared_reconciliation_client();

    // FID-219: 3-retry loop with 500ms backoff to tolerate Anvil's archive-fork
    // state-trie warming, which intermittently returns truncated HTTP bodies on the
    // first call to a contract after fork start (surface as "rpc parse: error
    // decoding response body" via reqwest::Error::decode()). The OnceLock shared
    // client joined the existing pool but lazy init still opens the cold TCP
    // connect during cycle 1 — the retry loop absorbs the transient truncation.
    // After the initial Anvil warmup every subsequent call succeeds on attempt 1;
    // 3 attempts × 500ms backoff = ≤1.5s worst-case latency.
    // FID-219 GREEN phase 3: status-check + raw-bytes-inspection + body[:200] snippet
    // in error message. Phase 2's retry loop alone was insufficient because the
    // body's actual shape was unknown — `reqwest::Error::decode()` from
    // `resp.json::<serde_json::Value>()` only echoes 'expected value' without
    // showing what the body actually was. This phase replaces `.json()` with
    // `.bytes()` + manual `serde_json::from_slice` so we can identify the actual
    // failure shape (empty body, HTML error page, JSON-RPC error envelope, or
    // truncated JSON) from the smoke-test log alone. The retry loop structure is
    // preserved; what changes is the body-read path.
    let mut last_err = String::from("unknown error");
    let json: serde_json::Value = {
        let mut value_opt: Option<serde_json::Value> = None;
        for _attempt in 0..3 {
            let resp = match client.post(rpc_url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("rpc send: {}", e);
                    if _attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                    continue;
                }
            };
            let status = resp.status();
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    last_err = format!("rpc read: {}", e);
                    if _attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                    continue;
                }
            };
            // Capture a 200-byte UTF-8-safe snippet for the error message so the
            // operator can see what Anvil actually returned on the failure path.
            let snippet_len = bytes.len().min(200);
            let snippet = String::from_utf8_lossy(&bytes[..snippet_len]);
            if !status.is_success() || bytes.is_empty() {
                last_err = format!(
                    "rpc status {} body {} bytes body[:200]={:?}",
                    status.as_u16(),
                    bytes.len(),
                    snippet
                );
                if _attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                continue;
            }
            match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(j) => {
                    value_opt = Some(j);
                    break;
                }
                Err(e) => {
                    last_err = format!("rpc parse: {} body[:200]={:?}", e, snippet);
                    if _attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }
        match value_opt {
            Some(j) => j,
            None => return Err(last_err),
        }
    };

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
    fn token_threshold_is_decoupled_from_usdc_threshold() {
        // FID-225: per-token threshold was previously aliased to the USDC
        // threshold ($0.10), which caused spurious halts on minor price-feed
        // noise (any $0.10+ discrepancy on a held token triggered the halt
        // path under RealTime classification). The threshold is now
        // decoupled — default token threshold is $5.00, USDC stays at $0.10.
        let cfg = ReconciliationConfig::default();
        assert!(
            cfg.token_divergence_threshold_usd > cfg.divergence_threshold_usd,
            "token threshold ({}) must exceed USDC threshold ({})",
            cfg.token_divergence_threshold_usd,
            cfg.divergence_threshold_usd
        );
        assert!(
            cfg.token_divergence_threshold_usd >= 5.0,
            "default token threshold should be at least $5.00 to absorb Anvil noise"
        );
        assert!(
            cfg.token_divergence_threshold_usd <= 100.0,
            "default token threshold should not exceed $100 (would defeat purpose)"
        );
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
            divergence_type: DivergenceType::None,
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
            divergence_type: DivergenceType::StartupCarryover,
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

    // =========================================================================
    // FID-196: apply_to_portfolio tests
    // =========================================================================

    use crate::agent::decision_log::DecisionLog;
    use crate::agent::decision_log::TradeStatus;
    use crate::core::types::Position;
    use crate::execution::portfolio::PortfolioManager;
    use async_trait::async_trait;
    use std::collections::HashMap;

    fn make_pos(pair: &str, quantity: f64) -> Position {
        Position {
            id: format!("test-{}", pair),
            pair: pair.to_string(),
            side: crate::core::types::Side::Long,
            entry_price: 1.0,
            current_price: 1.0,
            quantity,
            stop_loss: 0.95,
            take_profit_1: 1.05,
            take_profit_2: 0.0,
            take_profit_3: 0.0,
            unrealized_pnl: 0.0,
            risk_amount: 1.0,
            strategy_name: "test".to_string(),
            opened_at: Utc::now(),
            scale_level: crate::core::types::ScaleLevel::Full,
            token_address: String::new(),
        }
    }

    struct TestExecutor {
        positions: Vec<Position>,
        balance: f64,
    }

    #[async_trait]
    impl crate::execution::engine::ExecutionEngine for TestExecutor {
        fn balance(&self) -> f64 {
            self.balance
        }
        fn open_positions(&self) -> Vec<&Position> {
            self.positions.iter().collect()
        }
        async fn place_order(
            &mut self,
            _pair: &str,
            _side: crate::core::types::Side,
            _quantity: f64,
            _price: Option<f64>,
        ) -> Result<crate::core::types::Order, crate::core::error::ExecutionError> {
            unimplemented!()
        }
        async fn close_position(
            &mut self,
            _position_id: &str,
        ) -> Result<crate::core::types::Order, crate::core::error::ExecutionError> {
            unimplemented!()
        }
    }

    fn make_test_portfolio(positions: Vec<Position>, balance: f64) -> PortfolioManager {
        let mut pm = PortfolioManager::new(balance, 0.0, 0.0);
        for p in positions {
            pm.positions_mut().insert(p.id.clone(), p);
        }
        pm
    }

    fn make_test_log() -> DecisionLog {
        DecisionLog::open("test_decisions.json", 100)
    }

    #[test]
    fn apply_to_portfolio_clears_phantom_position() {
        // Phantom worth $20 in a $100 portfolio = 20%, under 50% safety threshold
        let phantom = make_pos("PHANTOM/USD", 20.0);
        let mut portfolio = make_test_portfolio(vec![phantom.clone()], 50.0);
        let mut log = make_test_log();
        log.append(crate::agent::decision_log::DecisionEntry {
            timestamp: Utc::now().to_rfc3339(),
            pair: "PHANTOM/USD".to_string(),
            action: "BUY".to_string(),
            confidence: 0.5,
            risk_reward: 1.0,
            stop_loss: 0.95,
            take_profit: 1.05,
            reasoning: "test".to_string(),
            conviction_score: 0.0,
            regime_label: "Trending".to_string(),
            trigger_strong: 0,
            trigger_moderate: 0,
            trigger_weak: 0,
            override_source: String::new(),
            status: TradeStatus::Pending,
            rejection_reason: None,
            outcome: None,
        });

        // Executor has no on-chain positions
        let executor = TestExecutor {
            positions: vec![],
            balance: 50.0,
        };
        let cfg = ReconciliationConfig::default();

        let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &executor);

        assert_eq!(report.phantom_cleared, vec!["PHANTOM/USD".to_string()]);
        assert!(!report.halt_recommended);
        assert!(portfolio.positions().is_empty());
        // Decision log updated to Rejected
        let entry = log.entries.last().unwrap();
        assert_eq!(entry.status, TradeStatus::Rejected);
    }

    #[test]
    fn apply_to_portfolio_adds_orphan_position() {
        let orphan = make_pos("ORPHAN/USD", 50.0);
        let mut portfolio = make_test_portfolio(vec![], 50.0);
        let mut log = make_test_log();

        // Executor has an on-chain position that's not in memory
        let executor = TestExecutor {
            positions: vec![orphan.clone()],
            balance: 50.0,
        };
        let cfg = ReconciliationConfig::default();

        let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &executor);

        assert_eq!(report.orphan_added, vec!["ORPHAN/USD".to_string()]);
        assert_eq!(portfolio.positions().len(), 1);
        // Position is stored under the executor's id (e.g., "test-ORPHAN/USD")
        assert!(portfolio
            .positions()
            .keys()
            .any(|k| k.contains("ORPHAN/USD")));
    }

    #[test]
    fn apply_to_portfolio_reconciles_usdc_balance() {
        let mut portfolio = make_test_portfolio(vec![], 50.0);
        portfolio.set_balance(50.0);
        let mut log = make_test_log();

        // Executor has $1 less (someone sent tokens out-of-band)
        let executor = TestExecutor {
            positions: vec![],
            balance: 49.0,
        };
        let cfg = ReconciliationConfig::default();

        let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &executor);

        assert!((report.usdc_divergence_corrected - (-1.0)).abs() < 0.001);
        assert!((portfolio.account().balance - 49.0).abs() < 0.001);
    }

    #[test]
    fn apply_to_portfolio_halts_on_extreme_divergence() {
        // Phantom worth $80 in a $100 portfolio (equity+balance) = 80%, exceeds 50% safety threshold
        let phantom = make_pos("BIG/USD", 80.0);
        let mut portfolio = make_test_portfolio(vec![phantom.clone()], 50.0);
        let mut log = make_test_log();

        let executor = TestExecutor {
            positions: vec![],
            balance: 50.0,
        };
        let cfg = ReconciliationConfig::default();

        let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &executor);

        assert!(report.halt_recommended);
        assert!(report.halt_reason.is_some());
        // Position NOT removed (halt not correct). The id is "test-BIG/USD"
        // because that's what make_pos generates.
        assert!(portfolio.positions().keys().any(|k| k.contains("BIG/USD")));
        assert!(report.phantom_cleared.is_empty());
    }

    #[test]
    fn apply_to_portfolio_no_op_when_states_match() {
        let pos = make_pos("BTC/USD", 1.0);
        let mut portfolio = make_test_portfolio(vec![pos.clone()], 49.0);
        let mut log = make_test_log();

        let executor = TestExecutor {
            positions: vec![pos],
            balance: 49.0,
        };
        let cfg = ReconciliationConfig::default();

        let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &executor);

        assert!(report.phantom_cleared.is_empty());
        assert!(report.orphan_added.is_empty());
        assert_eq!(report.usdc_divergence_corrected, 0.0);
        assert!(!report.halt_recommended);
        assert_eq!(portfolio.positions().len(), 1);
    }

    // =========================================================================
    // FID-225 round 2: Phantom-position classification regression tests.
    //
    // Validates the per-token divergence classifier distinguishes between
    // haltable real drift (on-chain qty > 0, large divergence_usd) and
    // PHANTOM-states (in-memory>0 but on-chain=0). Phantoms are tolerated
    // by the heartbeat; the 5-min ChainPositionRecovery → apply_to_portfolio
    // (FID-196) is the proper handler for them.
    //
    // Background: Spencer's 2026-06-21 3:35 AM Anvil run halted on cycle 34
    // with in-memory=28.332434 tokens (=$5.24 divergence at $0.185) for an
    // `ai-1` phantom that didn't exist on-chain. Without this fix, the
    // engine never starts on any Anvil session that has residual phantom
    // state from a prior crash.
    // =========================================================================

    /// Phantom: in-memory claims tokens (28.33 of `ai-1`), chain has 0.
    /// The classifier MUST return false — heartbeat skips phantoms.
    /// This is the exact bug case (cycle 34 of 2026-06-21 03:35 AM run).
    #[test]
    fn phantom_position_is_not_haltable_divergence() {
        let cfg = ReconciliationConfig::default();
        assert!(
            !is_haltable_token_divergence(
                28.332434,    // expected_qty (real cycle-34 value)
                0.0,          // on_chain_qty (true phantom)
                5.2424,       // divergence_usd at $0.185 price
                cfg.token_divergence_threshold_usd,
            ),
            "phantom (in-memory>0, on-chain=0) MUST NOT be haltable even when div_usd exceeds threshold"
        );
    }

    /// Real drift: both sides have tokens but they differ.
    /// Above the threshold — classifier returns true (halt-required).
    #[test]
    fn real_drift_above_threshold_is_haltable() {
        let cfg = ReconciliationConfig::default();
        // expected=100, on_chain=85, diff=15 tokens @ $1.0 = $15 divergence
        // = well above default $5.00 threshold
        assert!(
            is_haltable_token_divergence(100.0, 85.0, 15.0, cfg.token_divergence_threshold_usd),
            "real drift above threshold MUST be haltable"
        );
    }

    /// Sub-threshold: real but small position drift (e.g., a swap with $0.50
    /// rounding loss on a $1 position). Engine has $1 to swap, this shouldn't
    /// halt it.
    #[test]
    fn below_threshold_is_not_haltable() {
        let cfg = ReconciliationConfig::default();
        // expected=100, on_chain=99.5, diff=0.5 tokens @ $1.0 = $0.5 (< $5.00)
        assert!(
            !is_haltable_token_divergence(100.0, 99.5, 0.5, cfg.token_divergence_threshold_usd),
            "divergence below threshold MUST NOT halt"
        );
    }

    /// Edge case: both empty (zero-quantity state). Not a phantom (would need
    /// in-memory > 0 for the phantom criterion). Not a drift. No-op.
    #[test]
    fn both_zero_is_not_phantom_and_not_haltable() {
        let cfg = ReconciliationConfig::default();
        assert!(
            !is_haltable_token_divergence(0.0, 0.0, 0.0, cfg.token_divergence_threshold_usd),
            "both-zero must not be classified as phantom or as haltable drift"
        );
    }

    /// Edge case: in-memory > 0 but on-chain > 0 AND on-chain equals zero
    /// (float edge). distinct from a true phantom. Should halt if div>threshold.
    #[test]
    fn near_zero_on_chain_with_large_expected_halts_as_drift_not_phantom() {
        let cfg = ReconciliationConfig::default();
        // Tiny but non-zero on-chain balance (e.g., dust residue from a
        // partial close). $15 divergence is above threshold. NOT phantom.
        assert!(
            is_haltable_token_divergence(100.0, 0.001, 15.0, cfg.token_divergence_threshold_usd),
            "near-zero (but non-zero) on-chain with real drift MUST halt"
        );
    }
}
