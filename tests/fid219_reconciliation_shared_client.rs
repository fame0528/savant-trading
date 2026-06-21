#![allow(clippy::doc_lazy_continuation)]
// tests/fid219_reconciliation_shared_client.rs
//
// FID-219 regression tests. Four source-pattern guards pin the
// "shared reqwest::Client for reconciliation queries" fix shipped
// by FID-219:
//
//   1. Reconciliation module declares the OnceLock static.
//   2. OnceLock helper function replaces the per-call constructor.
//   3. Reconciliation source imports `OnceLock`.
//   4. query_token_balance calls the shared_reconciliation_client
//      helper (NOT per-call reqwest::Client::builder()).
//
// These tests are SYNTHETIC (source-grep + structural anchors).
// They don't exercise the full reconcile flow against a live RPC —
// the end-to-end behavior is covered by FID-213 R9's behavioral
// smoke test (data/boot_logs/savant_boot_final.log), which now must
// show "WALLET_RECONCILIATION: OK (in_memory_usdc=...)" on cycle 1
// instead of "RPC failure querying USDC balance".
//
// The "per-call Client construction is GONE" test (#4) is the
// load-bearing one — it pins that the antipattern can never return
// silently. If a future PR re-introduces `let client =
// reqwest::Client::builder()...`, the test fails loud.

// Note: source-pattern regression tests don't need any imports from the lib
// crate; they only inspect the file contents via `include_str!`.

/// Anchor: the OnceLock static. Unique because the FID-219 block is
/// the only place this Rust-syntax pattern appears in src/.
const RECONCILIATION_CLIENT_ANCHOR: &str =
    "static RECONCILIATION_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();";

/// Window size for forward-only anchor scans from RECONCILIATION_CLIENT_ANCHOR.
/// Sized to comfortably cover the onceLock helper (~600 chars including the
/// 13-line FID-219 comment + 12-line helper body). 1500 chars is 2.5x the
/// current size, allowing for future helper expansion without re-tuning.
const HELPER_WINDOW_FORWARD: usize = 1500;

/// Test 1: declaration of the OnceLock static is present.
/// This is the load-bearing invariant for the reuse guarantee — once
/// this pattern is in the source, `query_token_balance` can only get
/// the shared client via `shared_reconciliation_client()`.
#[test]
fn reconciliation_source_contains_shared_client_static() {
    let src = include_str!("../src/execution/reconciliation.rs");
    assert!(
        src.contains(RECONCILIATION_CLIENT_ANCHOR),
        "src/execution/reconciliation.rs should declare \
         `static RECONCILIATION_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();`"
    );
}

/// Test 2: shared_reconciliation_client helper function is present
/// below the OnceLock static. Anchored forward from the static to
/// avoid false positives if a future refactor moves the helper.
#[test]
fn reconciliation_source_contains_shared_reconciliation_client_helper() {
    let src = include_str!("../src/execution/reconciliation.rs");
    let anchor_idx = src
        .find(RECONCILIATION_CLIENT_ANCHOR)
        .expect("OnceLock static anchor must exist (test 1 covers this)");
    let window_end = (anchor_idx + HELPER_WINDOW_FORWARD).min(src.len());
    let window = &src[anchor_idx..window_end];
    assert!(
        window.contains("fn shared_reconciliation_client() -> reqwest::Client"),
        "OnceLock helper function `shared_reconciliation_client()` must appear within \
         {} chars forward of the static declaration with return type `reqwest::Client`. \
         Window anchored at byte {} ends at {} (helper window size = {}). \
         NOTE: the OLD signature was `Result<reqwest::Client, String>` but was replaced \
         with `reqwest::Client` after get_or_try_init was found to be unstable \
         (E0658 once_cell_try pre-Rust 1.86); we now use the stable get_or_init + \
         `.expect()` inside the closure (see FID-219 GREEN phase for rationale).",
        HELPER_WINDOW_FORWARD,
        anchor_idx,
        window_end,
        HELPER_WINDOW_FORWARD
    );
}

/// Test 3: OnceLock import is added to the use block. Without
/// `use std::sync::OnceLock;`, the static declaration won't compile.
#[test]
fn reconciliation_source_imports_once_lock() {
    let src = include_str!("../src/execution/reconciliation.rs");
    assert!(
        src.contains("use std::sync::OnceLock;"),
        "src/execution/reconciliation.rs must import `std::sync::OnceLock` \
         before declaring the RECONCILIATION_CLIENT static"
    );
}

// Note: source-pattern regression tests don't need any imports from the lib
// crate; they only inspect the file contents via `include_str!`.

/// Test 4 (load-bearing): per-call `reqwest::Client::builder()` in
/// query_token_balance is GONE, and `shared_reconciliation_client()`
/// is called instead. This test is brittle by design — a future PR
/// that re-introduces the per-call pattern fails this test loud.
#[test]
fn reconciliation_source_drops_per_call_client_builder() {
    let src = include_str!("../src/execution/reconciliation.rs");

    // The OLD per-call antipattern was:
    //     let client = reqwest::Client::builder()
    //         .timeout(std::time::Duration::from_secs(10))
    //         .build()
    //         .map_err(|e| format!("reqwest client: {}", e))?;
    //
    // We assert the substring `reqwest::Client::builder()\n        .timeout(
    // after fn query_token_balance is absent.

    // Find query_token_balance function start.
    let qtb_idx = src
        .find("fn query_token_balance")
        .expect("query_token_balance function must exist in reconciliation.rs");
    // Take a generous window to capture the function body.
    const QTB_WINDOW_FORWARD: usize = 3000;
    let window_end = (qtb_idx + QTB_WINDOW_FORWARD).min(src.len());
    let window = &src[qtb_idx..window_end];

    // Negative assertion: per-call constructor pattern is absent.
    let per_call_pattern = "let client = reqwest::Client::builder()";
    assert!(
        !window.contains(per_call_pattern),
        "FID-219 regression returned: per-call `let client = reqwest::Client::builder()` \
         reappeared in query_token_balance body. The shared static \
         `shared_reconciliation_client()` MUST be used instead."
    );

    // Positive assertion: shared helper call is present.
    assert!(
        window.contains("shared_reconciliation_client()"),
        "query_token_balance must call `shared_reconciliation_client()` (the \
         FID-219 OnceLock helper). Found function start at byte {}.",
        qtb_idx
    );
}

/// Test 5 (FID-219 GREEN phase 4 regression anchor): the heartbeat
/// + FID-155 5-min chain-sync MUST default to `"arbitrum"`, never
/// `"ethereum"`. The original bug was a comment-vs-code typo in
/// `src/engine/mod.rs`: the docstring immediately above the
/// `std::env::var("SAVANT_CHAIN")` line said `(default: "arbitrum")`
/// but the code used `.unwrap_or_else(|_| "ethereum".to_string())`.
/// With `.env` blank, SAVANT_CHAIN was always unset → heartbeat
/// fell back to chain_id 1 → queried Ethereum's USDC contract
/// (0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48) on the Arbitrum
/// fork → Anvil returned JSON-RPC `"missing trie node"` masked by
/// `reqwest` as `error decoding response body`.
#[test]
fn engine_heartbeat_default_is_arbitrum_not_ethereum() {
    let src = include_str!("../src/engine/mod.rs");

    // Whole-string substring scan (not line-based) so the test still
    // catches the buggy default if a future PR splits the
    // `unwrap_or_else` across multiple lines via rustfmt. The buggy
    // sentinel is the EXACT trailing closure form:
    //     .unwrap_or_else(|_| "ethereum".to_string())
    let buggy_sentinel = ".unwrap_or_else(|_| \"ethereum\".to_string())";
    assert!(
        !src.contains(buggy_sentinel),
        "FID-219 root-cause regression in src/engine/mod.rs: SAVANT_CHAIN default \
         reverted to `\"ethereum\"` (detected sentinel `{}`). Engine + heartbeat + \
         FID-155 5-min chain sync must all default to `\"arbitrum\"` (matching \
         DexTrader's actual chain, chain_id 42161). The original bug surfaced as \
         cycle-1 `[WARN] WALLET_RECONCILIATION: rpc parse: error decoding response body` \
         because heartbeat queried Ethereum's USDC against the Arbitrum fork and \
         Anvil returned `missing trie node`. To use a non-Arbitrum deployment, \
         override via the SAVANT_CHAIN env var at runtime — DO NOT change the default.",
        buggy_sentinel
    );

    // Positive sanity check: at least 2 `"arbitrum"` defaults exist
    // (FID-154 heartbeat + FID-155 5-min sync). If both are inverted,
    // the negative assertion above catches it; if only one is reverted,
    // this positive count would drop below 2 — also caught.
    let good_sentinel = ".unwrap_or_else(|_| \"arbitrum\".to_string())";
    let arbitrum_count = src.matches(good_sentinel).count();
    assert!(
        arbitrum_count >= 2,
        "FID-219 sanity check failed: expected at least 2 SAVANT_CHAIN defaults of \
         `\"arbitrum\"` (one in FID-154 heartbeat, one in FID-155 5-min sync), \
         found {} in src/engine/mod.rs. If you removed one but kept the other, \
         please update this assertion AND verify both paths still align on the same default.",
        arbitrum_count
    );
}

/// Test 6 (FID-219+ defensive `enabled` flag guard): the FID-154
/// heartbeat MUST refuse to probe a chain whose `enabled` flag is
/// `false` in `config.chains.<name>`. Without this guard, an operator
/// who manually sets SAVANT_CHAIN=<name> for a `<name>` block with
/// `enabled = false` (e.g., `chains.ethereum.enabled = false` in
/// `config/test-anvil.toml` but `SAVANT_CHAIN=ethereum` to test
/// mainnet connectivity) would silently probe the disabled chain — the
/// heartbeat would query ethereum USDC addresses against the Arbitrum
/// Anvil fork and surface as `rpc parse: error decoding response body`,
/// reusing the same failure mode FID-219 phase 4 just eliminated via the
/// `arbitrum` default flip. The `enabled` flag on `ChainEntry`
/// (src/core/config.rs) is a per-chain kill-switch and MUST be honored.
#[test]
fn engine_heartbeat_refuses_disabled_chain() {
    let src = include_str!("../src/engine/mod.rs");

    // Positive sentinel: the guard pattern `if !active_chain.enabled`
    // is present in src/engine/mod.rs. Whole-string `contains()` (not
    // line-based, per Test 5's code-reviewer refinement for multi-line
    // false-negatives).
    let guard_pattern = "if !active_chain.enabled";
    assert!(
        src.contains(guard_pattern),
        "FID-219+ regression: `{}` guard pattern missing from src/engine/mod.rs. \
         The FID-154 heartbeat must refuse to probe when `active_chain.enabled == false` \
         (per ChainEntry.enabled bool field in src/core/config.rs). Without this guard, \
         an operator who manually sets SAVANT_CHAIN=<name> to a chain whose \
         `chains.<name>.enabled = false` would silently probe the disabled chain — the \
         heartbeat would query e.g. ethereum's USDC against the Arbitrum Anvil fork and \
         surface `rpc parse: error decoding response body` even though the operator \
         explicitly disabled that chain in config/test-anvil.toml.",
        guard_pattern
    );

    // Positive sentinel: the exact standardized error message must include
    // (a) the FID-219+ tag for boot-log grep traceability and (b) the
    // '<chain>' is disabled in config.chains. Refusing to probe.' phrasing
    // for operator visibility. If a future refactor rewrites either the
    // tag or the phrasing, the operator loses grep-ability.
    let error_pattern = "FID-219+: chain '{}' is disabled in config.chains. Refusing to probe.";
    assert!(
        src.contains(error_pattern),
        "FID-219+ regression: standardized error string `{}` missing from \
         src/engine/mod.rs. The error MUST include the FID-219+ tag (for \
         boot-log grep traceability) AND the standardized phrasing \
         `'<chain>' is disabled in config.chains. Refusing to probe.` for \
         operator visibility (so a future operator reading the boot log \
         understands the engine refused to start because they set \
         SAVANT_CHAIN=<name> against a disabled chain).",
        error_pattern
    );
}

/// Test 7 (FID-219+ FID-155 followup): the FID-155 5-min chain-sync
/// MUST also check the `enabled` flag before spinning up
/// `ChainPositionRecovery`. Without this guard, the periodic sync
/// would query the disabled chain and adopt its truth as ours
/// (corrupting in-memory state). FID-155's guard uses SOFT-SKIP
/// (warn! + control-flow fall-through to `last_chain_sync = now`),
/// divergent from FID-154's HARD-BREAK semantics, because:
/// - FID-154 runs FIRST in every cycle and catches the operator
///   misconfiguration with a hard halt + savant.blocked write.
/// - FID-155 is periodic defense-in-depth — soft-skip avoids
///   compounding halt behavior for the same root cause.
#[test]
fn engine_chain_sync_soft_skips_disabled_chain() {
    let src = include_str!("../src/engine/mod.rs");

    // Positive sentinel A: the soft-skip guard pattern. FID-155 uses
    // `chain_cfg.enabled` (not `active_chain.enabled` like FID-154
    // does, since FID-155 binds the chain via `if let Some(chain_cfg) = ...`).
    let guard_pattern = "if !chain_cfg.enabled";
    assert!(
        src.contains(guard_pattern),
        "FID-219+ regression: `{}` guard pattern missing from FID-155-block in src/engine/mod.rs. \
         The FID-155 5-min chain-sync MUST also check the per-chain `enabled` \
         flag before constructing `ChainPositionRecovery` (mirrors FID-154's \
         defensive `enabled` check). Without this guard, an operator with \
         `chains.<name>.enabled = false` + `SAVANT_CHAIN=<name>` would get \
         per-cycle safe behavior (FID-154 hard-halts) but the 5-min chain-sync \
         would STILL query the disabled chain and silently overwrite our \
         in-memory state with on-chain truth from the wrong chain. Soft-skip \
         (warn! + fall-through to `last_chain_sync = now`) is the correct \
         divergent behavior — see FID-219+ followup comment in src/engine/mod.rs.",
        guard_pattern
    );

    // Positive sentinel B: the standardized skip-line message. Includes
    // (a) the FID-155 tag for boot-log grep traceability and (b) the
    // 'skipping 5-min sync' phrasing so the operator understands WHY
    // the line fired. If a future refactor drops either, the operator
    // loses diagnostic clarity.
    let warn_pattern = "FID-155: chain '{}' is disabled in config.chains — skipping 5-min sync";
    assert!(
        src.contains(warn_pattern),
        "FID-219+ regression: standardized skip-line message `{}` missing from \
         src/engine/mod.rs. The FID-155 soft-skip warn! MUST include the \
         FID-155 tag (for boot-log grep traceability) AND the standardized \
         `skipping 5-min sync` phrasing so a future operator greping the \
         boot log can identify the cause. If you rewrote either the tag or \
         the phrasing, please update this test — the pattern is brittle by \
         design (FID-anchored regression).",
        warn_pattern
    );
}

/// Test 8 (FID-219+ savant.blocked wiring): the FID-154 heartbeat's
/// disabled-chain path MUST write `savant.blocked` + call
/// `shared.set_block(BlockReason)` so the halt is visible on disk AND
/// in the dashboard's ENGINE BLOCKED card. Without this wiring, the
/// engine exits the cycle silently — operators have no on-disk
/// block file to delete to resume, and the dashboard never shows
/// the blocking card.
///
/// This test pins three independent literals that together prove the
/// disabled-chain halt path is wired correctly:
///   A) `block_type: "chain_disabled"` literal — proves the
///      BlockReason builder picks the right card label.
///   B) `Trigger: chain_disabled` literal — proves the savant.blocked
///      file's content matches operator-grep expectations.
///   C) `std::fs::write("savant.blocked",` literal — proves the disk
///      file is actually written (not just the in-memory vault state).
///
/// The combination is sufficient to pin the wiring without fragile
/// ordering checks or timestamp-dependent comparisons (see code-review
/// feedback from the previous Test 8 draft — the over-engineered
/// or_else-chain + chrono::Utc::now()-dependent file_pattern + ordering
/// assertions were all fault-prone patterns and have been removed).
#[test]
fn engine_heartbeat_disabled_chain_writes_block_file() {
    let src = include_str!("../src/engine/mod.rs");

    // Sentinel A: BlockReason builder carries the right card label.
    let block_type_pattern = r#"block_type: "chain_disabled".to_string()"#;
    assert!(
        src.contains(block_type_pattern),
        "FID-219+ regression: vault block_type literal `{}` missing from \
         src/engine/mod.rs. The FID-154 heartbeat's disabled-chain path \
         MUST write a BlockReason with block_type=`chain_disabled` so the \
         dashboard's /api/risk endpoint surfaces the correct 'ENGINE BLOCKED' \
         card. If you renamed the block_type (e.g., to 'disabled_chain' or \
         'operator_config'), update this test and the dashboard card label \
         accordingly.",
        block_type_pattern
    );

    // Sentinel B: savant.blocked file content carries the right trigger
    // literal so operators can `grep` boot logs and file contents.
    let trigger_pattern = "Trigger: chain_disabled";
    assert!(
        src.contains(trigger_pattern),
        "FID-219+ regression: `Trigger: chain_disabled` literal missing from \
         src/engine/mod.rs. The savant.blocked file MUST contain this trigger \
         prefix for operator-grep traceability. Expected file format: \
         `{{UTC timestamp}}\\nTrigger: <type>\\nReason: <text>`. If you \
         changed the trigger prefix, update this test — operators rely on \
         `grep 'Trigger: chain_disabled' data/savant.blocked` to confirm \
         the disabled-chain path fired (vs wallet_reconciliation or \
         startup_carryover_live halts)."
    );

    // Sentinel C: the disk file is actually written (not just the
    // in-memory vault state). Calling std::fs::write("savant.blocked", ...)
    // is the operator-acknowledged way to halt the engine (EngineState::new
    // refuses to start if the file exists).
    let file_write_pattern = r#"std::fs::write("savant.blocked""#;
    assert!(
        src.contains(file_write_pattern),
        "FID-219+ regression: `std::fs::write(\"savant.blocked\", ...)` call \
         missing from src/engine/mod.rs (expected pattern literal: `{}`). \
         The FID-154 disabled-chain halt MUST write the savant.blocked file \
         so operators can read the block reason (`cat data/savant.blocked`) \
         or resume (`rm data/savant.blocked`). The pattern is brittle by \
         design (FID-anchored regression test).",
        file_write_pattern
    );
}
