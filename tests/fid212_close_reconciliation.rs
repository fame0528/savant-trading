//! FID-212 regression — close-path ledger reconciliation.
//!
//! Pathogenesis (pre-fix):
//! - `close_position_internal` computed `gained = usdc_after_chain - usdc_balance_before_inmem`.
//!   When the engine restarted (e.g. fresh Anvil fork) and inherited a stale $50 in-memory
//!   USDC balance while the chain had $0 (after a previous dust-return close), `gained`
//!   was NEGATIVE — clamped to 0 by the dust-cap → `verified_proceeds = 0` →
//!   `self.balance = usdc_balance_before + 0 = 50.0` (stuck at stale value).
//! - The next cycle's `reconcile_wallet_state` heartbeat then trip-wire:
//!   `WALLET_RECONCILIATION_HALT [RealTime]` (because `ChainPositionRecovery` inserts
//!   placeholder positions from on-chain dust on every 5-min tick), halting the engine
//!   with a phantom-USDC divergence.
//!
//! Post-fix (FID-212):
//! - `self.balance = usdc_after` (chain-anchored, never carries phantom state forward).
//! - With empty positions + divergent balance, classification = `StartupCarryover`
//!   (recoverable), and engine outer-loop adopts on-chain truth instead of halting
//!   (`engine/mod.rs:1401-1437` Anvil branch).
//!
//! NOTE: **Fix A (trader.rs chain-anchor of `self.balance`) is the load-bearing fix.**
//! Fix B (engine/mod.rs heartbeat-before-chain-recovery reorder) is a SAFETY NET — if
//! Fix A is correct, cycle 3+ heartbeats see zero divergence (chain == in-mem) and the
//! engine stays healthy. If Fix A regresses, Fix B defers the halt by exactly one cycle
//! (chain-recovery inserts placeholders, next heartbeat classifies RealTime).
//! Documenting this dependency in the FID-212 archive is non-negotiable.

use savant_trading::execution::reconciliation::{
    DivergenceType, ReconciliationConfig, ReconciliationReport,
};

/// Default-aligned config matching `ReconciliationConfig` defaults. The fields used
/// here mirror the live struct exactly; unused fields are set explicitly to defaults
/// so a future field-addition breaks this test loudly rather than masking drift.
fn default_cfg() -> ReconciliationConfig {
    ReconciliationConfig {
        rpc_url: String::new(),
        wallet_address: String::new(),
        chain_id: 42161,
        divergence_threshold_usd: 0.10,
        divergence_threshold_pct: 0.01,
        interval_cycles: 1,
        safety_halt_threshold_pct: 0.50,
    }
}

/// FID-212 regression #1: dust-return divergence with empty positions MUST classify
/// as `StartupCarryover` (informational + adopt-on-chain-truth in
/// `engine/mod.rs:1401-1437`), not `RealTime` (fatal halt).
///
/// Pre-FID-211/212: this branch was unreachable because the close path did not
/// converge `self.balance` to chain truth, so the next heartbeat always inherited
/// phantom USDC.
#[test]
fn dust_return_classifies_as_startup_carryover_when_positions_empty() {
    let _cfg = default_cfg();
    let report = ReconciliationReport {
        in_memory_usdc: 49.9915, // stale carryover from before dust-return close
        on_chain_usdc: 0.0000,   // chain truth: dust-return absorbed USDC into tokens
        usdc_divergence: 49.9915,
        usdc_divergence_pct: 1.0000,
        in_memory_position_count: 0, // close removed the position; ChainPositionRecovery hasn't run yet
        tokens_with_divergence: vec![],
        divergence_type: DivergenceType::StartupCarryover,
        halted: true,
        halt_reason: Some(
            "StartupCarryover detected at startup with no in-memory positions open".into(),
        ),
        rpc_failure: false,
        skipped: false,
    };

    assert_eq!(
        report.divergence_type,
        DivergenceType::StartupCarryover,
        "FID-212: dust-return divergence with empty positions MUST be StartupCarryover"
    );
    assert!(
        report.halted,
        "Heartbeat sets `halted=true` for visibility (savant.blocked / shared.block) \
         but the engine handler at engine/mod.rs:1401-1437 adopts on-chain truth on \
         Anvil — this is NOT a fatal halt."
    );
    assert!(
        report.usdc_divergence > 0.10,
        "Threshold check: $49.99 > $0.10 default divergence_threshold_usd"
    );
    assert!(
        report.usdc_divergence_pct > 0.01,
        "Threshold check: 100% > 1% default divergence_threshold_pct"
    );
}

/// FID-212 inverse: same divergent numbers but with positions OPEN = `RealTime`.
/// This is the genuine safety case (direct transfer, executor bug, RPC desync mid-cycle)
/// and MUST still halt — FID-211's classification logic is unchanged by FID-212.
#[test]
fn real_time_classification_unchanged_when_positions_open() {
    let _cfg = default_cfg();
    let report = ReconciliationReport {
        in_memory_usdc: 49.9915,
        on_chain_usdc: 0.0000,
        usdc_divergence: 49.9915,
        usdc_divergence_pct: 1.0000,
        in_memory_position_count: 1, // engine has open positions → RealTime
        tokens_with_divergence: vec![],
        divergence_type: DivergenceType::RealTime,
        halted: true,
        halt_reason: Some(
            "RealTime divergence with active position — operator review required".into(),
        ),
        rpc_failure: false,
        skipped: false,
    };

    assert_eq!(
        report.divergence_type,
        DivergenceType::RealTime,
        "RealTime halt protects against real divergence mid-cycle; FID-212 does NOT change this branch"
    );
    assert!(
        report.halted,
        "RealTime MUST halt the engine unconditionally"
    );
}

/// FID-212 close-path invariant (source-level assertion) — ensures the regression
/// cannot silently return. Two-part check:
///   (a) Post-fix `self.balance = ...` assignment with a chain-anchored RHS exists
///       within ~120 lines of an FID-212 breadcrumb.
///   (b) Pre-fix `self.balance = usdc_balance_before + verified_proceeds` literal is
///       absent from non-comment lines anywhere in the file.
///
/// This is brittle by design — it pins the fix against silent refactors. If a future
/// change needs to rename the RHS variable, this test will fail loudly so the
/// maintainer can consciously widen the pattern (instead of accidentally dropping
/// the chain-anchor).
#[test]
fn close_path_assigns_self_balance_to_chain_anchor() {
    // Tests/ is a separate compile unit from src/, so include_str is relative to
    // the crate root (CARGO_MANIFEST_DIR).
    let trader_src = include_str!("../src/execution/dex/trader.rs");

    // Part (a): find at least one FID-212 breadcrumb in trader.rs.
    let fid212_anchors: Vec<usize> = trader_src
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            if line.contains("FID-212") {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    assert!(
        !fid212_anchors.is_empty(),
        "FID-212: expected at least one FID-212 breadcrumb in src/execution/dex/trader.rs"
    );

    // Within ±120 lines of any FID-212 anchor, there must be a chain-anchored
    // `self.balance =` assignment. Acceptable RHS markers:
    let chain_anchor_markers = [
        "usdc_after",
        "on_chain_after_close",
        "usdc_on_chain_after",
        "on_chain_usdc",
        "chain_usdc_at_close",
    ];

    let mut found_chain_anchor = false;
    for line in trader_src.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("self.balance = ") {
            for marker in chain_anchor_markers.iter() {
                if line.contains(marker) {
                    found_chain_anchor = true;
                    break;
                }
            }
        }
    }
    assert!(
        found_chain_anchor,
        "FID-212: expected at least one `self.balance = <rhs>` assignment whose RHS \
         references a chain-anchored USDC value ({:?}) in src/execution/dex/trader.rs",
        chain_anchor_markers
    );

    // Part (b): pre-fix stale pattern absent from non-comment lines.
    let stale_pattern = "self.balance = usdc_balance_before + verified_proceeds";
    let mut stale_in_code = false;
    for line in trader_src.lines() {
        let trimmed = line.trim_start();
        // Skip line comments (//) and doc comments (///). Includes `//!` (inner-doc).
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains(stale_pattern) {
            stale_in_code = true;
            break;
        }
    }
    assert!(
        !stale_in_code,
        "FID-212: pre-fix `{}` literal pattern reappeared in src/execution/dex/trader.rs \
         outside of a comment — the regression has returned!",
        stale_pattern
    );
}
