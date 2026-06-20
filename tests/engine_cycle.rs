//! FID-211 (audit Finding 2.1): Engine SOT coherence integration test.
//!
//! Root cause of all 9 FID-211 bugs reaching runtime was "engine 0 direct
//! tests" — no integration test ever exercised the SOT wrappers end-to-end,
//! so dual-write / state-divergence bugs survived until the engine ran
//! overnight on a live Anvil fork.
//!
//! This file proves the SOT invariant the wrappers promise:
//!
//!   After every wrapper call, SQLite ≡ in-memory.
//!
//! Concretely, every mutation (`open_position`, `adjust_stop`,
//! `adjust_quantity`, `partial_close`, `close_position_persist`) must leave
//! the SQLite `positions` / `trades` tables in agreement with the
//! in-memory `PortfolioManager` cache. We assert this by re-loading from
//! the DB into a FRESH portfolio after each step — if the two ever drift,
//! one of these assertions fails.
//!
//! The final test simulates an engine restart: it drops the original
//! portfolio, constructs a new one, and calls `load_from_db`. The new
//! portfolio must reconstruct exactly the state the prior run persisted.
//! This is the regression test for FID-211 Bug 2 (state carryover
//! divergence) — the bug that crashed v0.14.10 on first cycle.

use std::collections::HashMap;

use chrono::Utc;

use savant_trading::core::types::{Position, ScaleLevel, Side};
use savant_trading::execution::portfolio::PortfolioManager;
use savant_trading::monitor::journal::TradeJournal;

// ── Test helpers ────────────────────────────────────────────────────────

/// Build a unique on-disk SQLite URL for this test process. Each test gets
/// its own file so parallel `cargo test` runs don't collide on the journal.
///
/// We deliberately use a real file (not `:memory:`) because `load_from_db`
/// and the restart-simulation test require the data to survive the
/// journal handle being dropped and recreated.
fn unique_db_url(test_name: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    // std::env::temp_dir() — never delete production data/savant.db from tests.
    let path = std::env::temp_dir().join(format!(
        "savant_engine_cycle_{}_{}_{}.sqlite",
        test_name, pid, n
    ));
    // Clean any stale file from a prior run with the same pid+n (extremely
    // unlikely, but cheap insurance for hermeticity).
    let _ = std::fs::remove_file(&path);
    format!("sqlite:{}", path.display())
}

/// A canonical long position used across tests. Distinct ids per call so
/// the duplicate-id guard never fires by accident.
fn sample_position(id: &str) -> Position {
    Position {
        id: id.to_string(),
        pair: "BTC/USD".to_string(),
        side: Side::Long,
        entry_price: 100.0,
        current_price: 100.0,
        quantity: 1.0,
        stop_loss: 95.0,
        take_profit_1: 110.0,
        take_profit_2: 120.0,
        take_profit_3: 130.0,
        unrealized_pnl: 0.0,
        risk_amount: 5.0,
        strategy_name: "engine_cycle_test".to_string(),
        opened_at: Utc::now(),
        scale_level: ScaleLevel::Full,
        token_address: "0xdeadbeef".to_string(),
    }
}

/// Open a journal + fresh portfolio wired the same way main.rs wires them.
async fn fresh_setup(db_url: &str) -> (TradeJournal, PortfolioManager) {
    let journal = TradeJournal::new(db_url).await.expect("journal opens");
    let portfolio = PortfolioManager::new(10_000.0, 0.001, 0.0005);
    (journal, portfolio)
}

/// Re-load every persisted position into a brand-new HashMap, independent
/// of any in-memory cache. This is our "ground truth" probe.
async fn db_positions(journal: &TradeJournal) -> HashMap<String, Position> {
    journal
        .load_positions()
        .await
        .expect("load_positions works")
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect()
}

async fn db_trade_count(journal: &TradeJournal) -> usize {
    journal
        .load_closed_trades(1000)
        .await
        .expect("load_closed_trades works")
        .len()
}

// ── Tests ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn open_position_persists_to_db_and_cache_in_lockstep() {
    let db = unique_db_url("open");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open succeeds");

    // In-memory cache reflects the open.
    assert_eq!(portfolio.open_positions(), 1, "in-memory count");
    assert!(portfolio.positions().contains_key("p1"));

    // SQLite reflects the open — independent probe via a fresh load.
    let from_db = db_positions(&journal).await;
    assert_eq!(from_db.len(), 1, "DB positions count");
    let db_p1 = from_db.get("p1").expect("p1 in DB");
    assert_eq!(db_p1.pair, "BTC/USD");
    assert_eq!(db_p1.entry_price, 100.0);
    assert_eq!(db_p1.token_address, "0xdeadbeef", "token_address persisted");
}

#[tokio::test]
async fn open_position_rejects_duplicate_without_touching_db() {
    let db = unique_db_url("dup");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("first open");

    // Second open with the SAME id must fail.
    let err = portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect_err("duplicate rejected");

    assert!(
        format!("{err:?}").contains("DuplicatePositionId"),
        "expected DuplicatePositionId, got {err:?}"
    );

    // State unchanged: still exactly one position in memory and DB.
    assert_eq!(portfolio.open_positions(), 1);
    assert_eq!(db_positions(&journal).await.len(), 1);
}

#[tokio::test]
async fn adjust_stop_writes_new_sl_to_db_and_cache() {
    let db = unique_db_url("adj_stop");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    // Trail stop up to break-even (allowed for a Long: new_stop >= entry).
    portfolio
        .adjust_stop("p1", 100.0, Some(115.0), None, None, None, &journal)
        .await
        .expect("adjust_stop succeeds");

    let mem = portfolio.positions().get("p1").expect("p1 in memory");
    assert_eq!(mem.stop_loss, 100.0, "in-memory SL updated");
    assert_eq!(mem.take_profit_1, 115.0, "in-memory TP1 updated");

    let db_p1 = db_positions(&journal)
        .await
        .get("p1")
        .expect("p1 in DB")
        .clone();
    assert_eq!(db_p1.stop_loss, 100.0, "DB SL updated");
    assert_eq!(db_p1.take_profit_1, 115.0, "DB TP1 updated");
}

#[tokio::test]
async fn adjust_quantity_writes_new_qty_to_db_and_cache() {
    // Regression test for FID-211 Bug 7: the old code did
    // `portfolio.positions_mut().get_mut(&id).quantity = X` at chain-sync
    // time, mutating in-memory WITHOUT touching SQLite — producing
    // triple-state divergence (on-chain vs DB vs memory).
    let db = unique_db_url("adj_qty");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    portfolio
        .adjust_quantity("p1", 0.75, &journal)
        .await
        .expect("adjust_quantity succeeds");

    let mem = portfolio.positions().get("p1").expect("p1 in memory");
    assert_eq!(mem.quantity, 0.75, "in-memory qty updated");

    let from_db = db_positions(&journal).await;
    let db_p1 = from_db.get("p1").expect("p1 in DB");
    assert_eq!(db_p1.quantity, 0.75, "DB qty updated — Bug 7 regression");
}

#[tokio::test]
async fn partial_close_scales_out_atomically() {
    // Regression test for FID-211 Bug 6: open-position dual-write was
    // fire-and-forget; a partial close that recorded a trade but failed
    // to persist the reduced position left DB/memory divergent.
    let db = unique_db_url("partial");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    // Scale out 50% at TP1, move SL to break-even, advance scale level.
    portfolio
        .partial_close(
            "p1",
            110.0, // exit_price
            0.5,   // scale_qty (50% of 1.0)
            ScaleLevel::Scaled50,
            100.0, // new_stop (break-even)
            "TP1 scale-out".to_string(),
            &journal,
        )
        .await
        .expect("partial_close succeeds");

    // Trade recorded.
    assert_eq!(db_trade_count(&journal).await, 1, "scale-out trade in DB");
    assert_eq!(portfolio.closed_trades().len(), 1, "trade in memory");

    // Position still open, qty reduced, scale advanced.
    let mem = portfolio
        .positions()
        .get("p1")
        .expect("p1 still open in memory");
    assert_eq!(mem.quantity, 0.5, "in-memory qty halved");
    assert_eq!(mem.scale_level, ScaleLevel::Scaled50);
    assert_eq!(mem.stop_loss, 100.0);

    let db_p1 = db_positions(&journal)
        .await
        .get("p1")
        .expect("p1 still in DB")
        .clone();
    assert_eq!(db_p1.quantity, 0.5, "DB qty halved — Bug 6 regression");
    assert_eq!(db_p1.scale_level, ScaleLevel::Scaled50);
    assert_eq!(db_p1.stop_loss, 100.0);
}

#[tokio::test]
async fn close_position_persist_removes_from_db_and_records_trade() {
    // Regression test for FID-211 Bug 5: close-position dual-write used
    // `let _ = j.delete_position` + `let _ = j.record_trade` (fire-and-forget).
    // If either SQLite write failed, the position vanished from memory but
    // lingered in DB (or vice versa). This test asserts the wrapper leaves
    // both stores in the correct terminal state.
    let db = unique_db_url("close");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    let trade = portfolio
        .close_position_persist("p1", 105.0, "manual close".to_string(), &journal)
        .await
        .expect("close succeeds");

    assert_eq!(trade.pair, "BTC/USD");

    // Position gone from both stores.
    assert_eq!(portfolio.open_positions(), 0, "in-memory empty");
    assert!(
        !db_positions(&journal).await.contains_key("p1"),
        "p1 removed from DB — Bug 5 regression"
    );

    // Trade recorded in both stores.
    assert_eq!(portfolio.closed_trades().len(), 1);
    assert_eq!(db_trade_count(&journal).await, 1);
}

#[tokio::test]
async fn close_position_persist_on_unknown_id_leaves_state_untouched() {
    let db = unique_db_url("close_unknown");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    let err = portfolio
        .close_position_persist("does-not-exist", 100.0, "x".to_string(), &journal)
        .await
        .expect_err("unknown id rejected");

    assert!(
        format!("{err:?}").contains("PositionNotFound"),
        "expected PositionNotFound, got {err:?}"
    );

    // Original position survives in both stores.
    assert_eq!(portfolio.open_positions(), 1);
    assert!(db_positions(&journal).await.contains_key("p1"));
    assert_eq!(
        db_trade_count(&journal).await,
        0,
        "no spurious trade recorded"
    );
}

#[tokio::test]
async fn restart_load_from_db_reconstructs_exact_state() {
    // THE FID-211 Bug 2 regression test. The v0.14.10 crash happened
    // because the engine started with stale in-memory state while the
    // chain had fresh state, and reconciliation halted at 100% divergence.
    // The fix: on startup, the engine rebuilds in-memory from SQLite
    // (the SOT) via `load_from_db`. This test proves that reconstruction
    // is faithful: persist a known state, drop the portfolio, reload, and
    // assert the new portfolio matches.
    let db = unique_db_url("restart");

    // Phase 1: build state.
    let (journal, mut portfolio) = fresh_setup(&db).await;
    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open p1");
    portfolio
        .adjust_quantity("p1", 0.75, &journal)
        .await
        .expect("adjust p1 qty");
    portfolio
        .partial_close(
            "p1",
            110.0,
            0.25,
            ScaleLevel::Scaled50,
            100.0,
            "scale-out".to_string(),
            &journal,
        )
        .await
        .expect("partial close");
    // After partial close of 0.25 from 0.75, qty = 0.5, scale = Scaled50.
    let mem_qty_before = portfolio.positions().get("p1").unwrap().quantity;
    assert!(
        (mem_qty_before - 0.5).abs() < 1e-9,
        "pre-restart qty sanity: {mem_qty_before}"
    );
    let trades_before = portfolio.closed_trades().len();
    drop(portfolio);

    // Phase 2: simulate restart — new portfolio, same DB.
    let mut restarted = PortfolioManager::new(10_000.0, 0.001, 0.0005);
    let loaded = restarted
        .load_from_db(&journal)
        .await
        .expect("load_from_db succeeds");

    assert_eq!(loaded, 1, "one position reconstructed");
    assert_eq!(restarted.open_positions(), 1);

    let p1 = restarted.positions().get("p1").expect("p1 reconstructed");
    assert!(
        (p1.quantity - 0.5).abs() < 1e-9,
        "post-restart qty matches pre-restart: got {}",
        p1.quantity
    );
    assert_eq!(p1.scale_level, ScaleLevel::Scaled50);
    assert_eq!(p1.stop_loss, 100.0);
    assert_eq!(p1.pair, "BTC/USD");
    assert_eq!(p1.token_address, "0xdeadbeef");

    // Closed trades reconstructed too.
    assert_eq!(
        restarted.closed_trades().len(),
        trades_before,
        "closed trades reconstructed"
    );
}

#[tokio::test]
async fn sync_from_db_and_remove_helpers_keep_cache_consistent() {
    // The cache-only helpers (sync_from_db_position / remove_synced_position
    // / clear_position_cache) are the sanctioned escape hatch for code
    // paths that have ALREADY written to SQLite and just need the cache to
    // match. Verify they refresh derived state (open_positions count).
    let db = unique_db_url("sync_helpers");
    let (_journal, mut portfolio) = fresh_setup(&db).await;

    assert_eq!(portfolio.open_positions(), 0);

    portfolio.sync_from_db_position(sample_position("p1"));
    assert_eq!(portfolio.open_positions(), 1);
    assert!(portfolio.positions().contains_key("p1"));

    let removed = portfolio.remove_synced_position("p1");
    assert!(removed.is_some());
    assert_eq!(portfolio.open_positions(), 0);

    // clear_position_cache on an already-empty cache is a no-op (no panic).
    portfolio.sync_from_db_position(sample_position("p2"));
    portfolio.sync_from_db_position(sample_position("p3"));
    assert_eq!(portfolio.open_positions(), 2);
    portfolio.clear_position_cache();
    assert_eq!(portfolio.open_positions(), 0);
}
