// tests/fid213_anvil_balance_init.rs
//
// FID-213 regression tests. Six tests pin the Anvil fresh-startup
// balance override behavior shipped by FID-213:
//   1. Helper function detects Anvil via rpc_url loopback.
//   2. Helper function correctly identifies mainnet (non-loopback).
//   3. trader.rs source contains the FID-213 override block annotation.
//   4. trader.rs source uses is_anvil_at_startup in struct init.
//   5. trader.rs source persists adopted balance via save_state()
//      UNCONDITIONALLY (NOT in warn branch only) — uses Rust-syntax
//      anchors (`trader.balance = initial_balance;`, `drift_adopted > 0.10`,
//      `trader.save_state().ok()`) so the structural invariant is robust
//      to prose renames in log macros.
//   6. trader.rs source drift-threshold matches FID-212 (0.10 USD).
//
// These tests are SYNTHETIC (source-grep + pure-function invariants).
// They do not exercise the full DexTrader::new path against a real Anvil
// fork. The end-to-end Anvil run is FID-213's open thread — recommend
// running manually via the FID-213 verification recipe after a real
// Anvil+start.bat session.

use savant_trading::execution::dex::trader::is_anvil_rpc;

/// Test 1: rpc_url heuristic correctly identifies Anvil (loopback).
#[test]
fn detect_localhost_as_anvil() {
    assert!(is_anvil_rpc("http://127.0.0.1:8545"));
    assert!(is_anvil_rpc("http://localhost:8545"));
    assert!(is_anvil_rpc("http://localhost:9545")); // common alt port
}

/// Test 2: rpc_url heuristic correctly identifies mainnet (non-loopback).
#[test]
fn mainnet_rpc_not_anvil() {
    assert!(!is_anvil_rpc("https://arb1.arbitrum.io/rpc"));
    assert!(!is_anvil_rpc("https://eth.llamarpc.com"));
    assert!(!is_anvil_rpc("https://mainnet.base.org"));
}

/// Anchor: the FIRST `let chain_reported = trader.balance;` line in
/// the override block. Unique because trader.rs only has it inside
/// the FID-213 override block. Tests 3, 5, 6 anchor here so their
/// windows always capture the override block content regardless of
/// where the log macros appear in the source.
const OVERRIDE_BLOCK_ANCHOR: &str = "let chain_reported = trader.balance;";

/// Window size for forward-only anchor scans. Sized to comfortably
/// cover the override block (~900 chars including 16-line persist
/// comment + multi-line warn!/info! macros). 1800 chars is 2x the
/// current size, allowing for future growth without re-tuning.
// Override block grew from ~900 chars (round-5) to 3024 chars (round-8 NIT enrichment:
// file_state metadata + extended warn! macro). 3500 chars = 16% safety margin over
// current size; bump if future enrichments push past it.
const OVERRIDE_WINDOW_FORWARD: usize = 3500;

/// Test 3: trader.rs source contains the FID-213 override block.
/// Pin the breadcrumb + rationale strings so a future refactor cannot
/// silently drop the override.
#[test]
fn trader_source_contains_fid213_override_block() {
    let src = include_str!("../src/execution/dex/trader.rs");
    assert!(
        src.contains("FID-213: Anvil fresh-startup balance override"),
        "trader.rs should contain the FID-213 override block annotation"
    );
    assert!(
        src.contains("config.starting_balance is canonical"),
        "trader.rs should document the audit rationale"
    );
    assert!(
        src.contains(OVERRIDE_BLOCK_ANCHOR),
        "trader.rs should contain the override block anchor line"
    );
}

/// Test 4: trader.rs source uses is_anvil_at_startup in struct init.
/// Pin the heuristic wiring.
#[test]
fn trader_source_uses_is_anvil_at_startup() {
    let src = include_str!("../src/execution/dex/trader.rs");
    assert!(
        src.contains("let is_anvil_at_startup = is_anvil_rpc(rpc_url)"),
        "trader.rs should compute is_anvil_at_startup via the helper"
    );
    assert!(
        src.contains("is_anvil: is_anvil_at_startup"),
        "trader.rs struct init should use the heuristic"
    );
}

/// Test 5: trader.rs source persists the adopted balance via save_state
/// call INSIDE the override block — UNCONDITIONALLY (not gated on the
/// warn branch).
///
/// Anchored entirely on Rust-syntax literals (`trader.balance = initial_balance;`,
/// `drift_adopted > 0.10`, `trader.save_state().ok()`) so the structural invariant
/// is robust to prose-string renames in the warn!/info! log macros.
///
/// Forward-only window (no backward extension) to avoid the code-reviewer
/// round-4 caveat: a backward window could include the existing
/// `trader.save_state().ok()` call in the earlier PhantomPositions block,
/// making `.find()` return the EARLIER (wrong) call site.
#[test]
fn trader_source_persists_adopted_balance_unconditionally() {
    let src = include_str!("../src/execution/dex/trader.rs");
    let anchor_idx = src
        .find(OVERRIDE_BLOCK_ANCHOR)
        .expect("override block anchor must exist (test 3 covers this)");
    let window_end = (anchor_idx + OVERRIDE_WINDOW_FORWARD).min(src.len());
    let window = &src[anchor_idx..window_end];

    // Three Rust-syntax ANCHORS that the unconditional fix establishes.
    // `.find()` returns FIRST occurrence in window — since the override
    // block is uniquely identified by OVERRIDE_BLOCK_ANCHOR, and the
    // structural anchors only appear inside the override block (not in
    // any earlier PhantomPositions block which is OUTSIDE the window
    // because we anchor FORWARD-ONLY), each .find() is unambiguous.
    let balance_init_pos = window
        .find("trader.balance = initial_balance;")
        .expect("balance init line must appear in override block");
    let threshold_pos = window
        .find("drift_adopted > 0.10")
        .expect("threshold check must appear in override block");
    let save_state_pos = window
        .find("if let Err(e) = trader.save_state()")
        .expect("save_state call must appear in override block");

    // Structural invariant: save_state runs UNCONDITIONALLY inside the outer
    // override `if`, AFTER `trader.balance = initial_balance;` AND BEFORE
    // `drift_adopted > 0.10`. If save_state were inside the warn branch, it
    // would be AFTER the threshold check, breaking this ordering.
    assert!(
        balance_init_pos < save_state_pos && save_state_pos < threshold_pos,
        "save_state() must run AFTER balance init AND BEFORE threshold check \
         (proving it's unconditional, not in warn branch). \
         positions in window (offset {}): balance_init={}, save_state={}, threshold={}",
        anchor_idx,
        balance_init_pos,
        save_state_pos,
        threshold_pos
    );
}

/// Test 6: trader.rs threshold uses 0.10 USD (matching FID-212
/// reconciliation cadence). Catches regress to 0.01 or higher.
/// Anchored on the override block's first line so a forward window
/// captures the `drift_adopted > 0.10` check (which is BEFORE the
/// warn macro).
#[test]
fn trader_source_drift_threshold_matches_fid212() {
    let src = include_str!("../src/execution/dex/trader.rs");
    let anchor_idx = src
        .find(OVERRIDE_BLOCK_ANCHOR)
        .expect("override block anchor must exist (test 3 covers this)");
    let window_end = (anchor_idx + OVERRIDE_WINDOW_FORWARD).min(src.len());
    let window = &src[anchor_idx..window_end];
    assert!(
        window.contains("drift_adopted > 0.10"),
        "FID-213 threshold must be 0.10 USD to match FID-212 reconciliation cadence. \
         Window anchored on {} starts at byte {}, ends at {} \
         (window size = {}, current override block is ~900 chars).",
        OVERRIDE_BLOCK_ANCHOR,
        anchor_idx,
        window_end,
        OVERRIDE_WINDOW_FORWARD
    );
}
