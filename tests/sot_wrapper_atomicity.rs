//! FID-211 (audit Finding 2.1): SOT wrapper atomicity tests.
//!
//! These tests prove the SOT wrappers leave in-memory state unchanged on
//! the error paths — the invariant that the v0.14.10 fire-and-forget
//! patterns (`let _ = j.save_position` etc.) violated. When a wrapper
//! returns `Err`, the in-memory cache MUST be in the exact state it was
//! in before the call: no phantom positions, no half-mutated fields, no
//! spurious trades.
//!
//! ## What this file does NOT cover (and why)
//!
//! True DB-failure injection — proving that if SQLite returns an error
//! mid-write, the wrapper rolls back the in-memory mutation — requires a
//! `Journal` trait so a mock can be substituted for `TradeJournal`. The
//! wrappers today take `&TradeJournal` (concrete struct), so a failing
//! mock cannot be injected without changing every wrapper signature.
//!
//! That refactor is real work (touches portfolio.rs, engine/mod.rs,
//! reconciliation.rs, main.rs) and is tracked as a follow-up recommendation
//! in the FID-211 re-audit, NOT silently deferred here. The tests below
//! cover the error paths the wrappers CAN reach without a mock: input
//! validation, duplicate detection, missing-position guards, and stop-
//! ratchet guards. Together with `engine_cycle.rs`'s happy-path proof,
//! they establish:
//!
//!   validation fails  →  Err returned  →  in-memory untouched
//!   validation passes →  DB write       →  in-memory updated only on Ok
//!
//! Closing the gap (DB-failure rollback) is the trait-refactor follow-up.

use chrono::Utc;

use savant_trading::core::types::{Position, ScaleLevel, Side};
use savant_trading::execution::portfolio::PortfolioManager;
use savant_trading::monitor::journal::TradeJournal;

fn unique_db_url(test_name: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!(
        "savant_sot_atomicity_{}_{}_{}.sqlite",
        test_name, pid, n
    ));
    let _ = std::fs::remove_file(&path);
    format!("sqlite:{}", path.display())
}

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
        strategy_name: "sot_atomicity_test".to_string(),
        opened_at: Utc::now(),
        scale_level: ScaleLevel::Full,
        token_address: "0xdeadbeef".to_string(),
    }
}

async fn fresh_setup(db_url: &str) -> (TradeJournal, PortfolioManager) {
    let journal = TradeJournal::new(db_url).await.expect("journal opens");
    let portfolio = PortfolioManager::new(10_000.0, 0.001, 0.0005);
    (journal, portfolio)
}

async fn db_trade_count(journal: &TradeJournal) -> usize {
    journal
        .load_closed_trades(1000)
        .await
        .expect("load_closed_trades works")
        .len()
}

async fn db_position_count(journal: &TradeJournal) -> usize {
    journal
        .load_positions()
        .await
        .expect("load_positions works")
        .len()
}

// ── open_position: duplicate id is rejected without DB or cache mutation ──

#[tokio::test]
async fn open_position_duplicate_id_does_not_overwrite_existing() {
    let db = unique_db_url("open_dup");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    let original = sample_position("p1");
    portfolio
        .open_position(original.clone(), &journal)
        .await
        .expect("first open");

    // Second open with the same id must fail AND not touch the stored row.
    let err = portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect_err("duplicate rejected");
    assert!(
        format!("{err:?}").contains("DuplicatePositionId"),
        "got {err:?}"
    );

    // The ORIGINAL position survives unchanged in memory.
    let mem = portfolio.positions().get("p1").expect("p1 in memory");
    assert_eq!(mem.token_address, original.token_address);

    // And in the DB.
    let db_positions: Vec<Position> = journal.load_positions().await.expect("load");
    assert_eq!(db_positions.len(), 1);
    assert_eq!(db_positions[0].id, "p1");
}

// ── adjust_stop: invalid ratchet is rejected without mutation ─────────────

#[tokio::test]
async fn adjust_stop_invalid_ratchet_leaves_position_untouched() {
    // The wrapper refuses to ratchet a Long's SL BELOW both entry and the
    // current SL (that would lock in a loss). Verify that on rejection,
    // the in-memory position keeps its original SL.
    let db = unique_db_url("bad_ratchet");
    let (journal, mut portfolio) = fresh_setup(&db).await;

    let mut pos = sample_position("p1");
    pos.entry_price = 100.0;
    pos.stop_loss = 95.0;
    portfolio.open_position(pos, &journal).await.expect("open");

    // Try to ratchet SL down to 90 — below entry (100) AND below current
    // SL (95). Must be rejected.
    let err = portfolio
        .adjust_stop("p1", 90.0, None, None, None, None, &journal)
        .await
        .expect_err("invalid ratchet rejected");
    assert!(
        format!("{err:?}").contains("InvalidStopRatchet"),
        "got {err:?}"
    );

    let mem = portfolio.positions().get("p1").unwrap();
    assert_eq!(mem.stop_loss, 95.0, "in-memory SL unchanged");

    let db_pos = journal
        .load_positions()
        .await
        .expect("load")
        .into_iter()
        .find(|p| p.id == "p1")
        .expect("p1 in DB");
    assert_eq!(db_pos.stop_loss, 95.0, "DB SL unchanged");
}

// ── adjust_stop on unknown id ──────────────────────────────────────────────

#[tokio::test]
async fn adjust_stop_unknown_id_returns_error_without_touching_others() {
    let db = unique_db_url("stop_unknown");
    let (journal, mut portfolio) = fresh_setup(&db).await;
    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    portfolio
        .adjust_stop("ghost", 100.0, None, None, None, None, &journal)
        .await
        .expect_err("unknown id rejected");

    // The real position is unaffected.
    assert_eq!(portfolio.open_positions(), 1);
    assert_eq!(db_position_count(&journal).await, 1);
}

// ── adjust_quantity: non-positive qty rejected ─────────────────────────────

#[tokio::test]
async fn adjust_quantity_zero_or_negative_rejected() {
    let db = unique_db_url("qty_zero");
    let (journal, mut portfolio) = fresh_setup(&db).await;
    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    for bad_qty in [0.0, -1.0, f64::MIN] {
        let err = portfolio
            .adjust_quantity("p1", bad_qty, &journal)
            .await
            .expect_err("non-positive qty rejected");
        assert!(
            format!("{err:?}").contains("adjust_quantity"),
            "qty={bad_qty} got {err:?}"
        );
    }

    // Original qty survives.
    assert_eq!(portfolio.positions().get("p1").unwrap().quantity, 1.0);
    let db_pos = journal
        .load_positions()
        .await
        .expect("load")
        .into_iter()
        .find(|p| p.id == "p1")
        .unwrap();
    assert_eq!(db_pos.quantity, 1.0);
}

// ── adjust_quantity on unknown id ──────────────────────────────────────────

#[tokio::test]
async fn adjust_quantity_unknown_id_rejected() {
    let db = unique_db_url("qty_unknown");
    let (journal, mut portfolio) = fresh_setup(&db).await;
    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    portfolio
        .adjust_quantity("ghost", 0.5, &journal)
        .await
        .expect_err("unknown id rejected");

    assert_eq!(portfolio.open_positions(), 1);
}

// ── partial_close on unknown id ────────────────────────────────────────────

#[tokio::test]
async fn partial_close_unknown_id_records_no_trade() {
    let db = unique_db_url("partial_unknown");
    let (journal, mut portfolio) = fresh_setup(&db).await;
    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    portfolio
        .partial_close(
            "ghost",
            110.0,
            0.5,
            ScaleLevel::Scaled50,
            100.0,
            "x".to_string(),
            &journal,
        )
        .await
        .expect_err("unknown id rejected");

    // No phantom trade, original position intact.
    assert_eq!(db_trade_count(&journal).await, 0, "no spurious trade");
    assert_eq!(portfolio.closed_trades().len(), 0);
    assert_eq!(portfolio.positions().get("p1").unwrap().quantity, 1.0);
}

// ── close_position_persist on unknown id records no trade ──────────────────

#[tokio::test]
async fn close_position_persist_unknown_id_records_no_trade() {
    let db = unique_db_url("close_unknown");
    let (journal, mut portfolio) = fresh_setup(&db).await;
    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    portfolio
        .close_position_persist("ghost", 105.0, "x".to_string(), &journal)
        .await
        .expect_err("unknown id rejected");

    assert_eq!(db_trade_count(&journal).await, 0);
    assert_eq!(portfolio.open_positions(), 1);
}

// ── Full-close partial_close leaves no orphan in either store ──────────────
//
// This is the closest we can get to proving DB/in-memory ordering without
// a mock journal: when partial_close's scale_qty >= position qty, the
// wrapper must (a) record the trade, (b) delete the position from DB, and
// (c) remove from memory — atomically on the Ok path. If any of those
// three is skipped or reordered, one of these assertions fails.

#[tokio::test]
async fn partial_close_to_zero_removes_position_everywhere() {
    let db = unique_db_url("partial_full");
    let (journal, mut portfolio) = fresh_setup(&db).await;
    portfolio
        .open_position(sample_position("p1"), &journal)
        .await
        .expect("open");

    // scale_qty == quantity → full close branch.
    portfolio
        .partial_close(
            "p1",
            105.0,
            1.0, // == quantity
            ScaleLevel::Closed,
            105.0,
            "full close via partial".to_string(),
            &journal,
        )
        .await
        .expect("partial_close succeeds");

    assert_eq!(portfolio.open_positions(), 0, "memory empty");
    assert_eq!(
        db_position_count(&journal).await,
        0,
        "DB empty — no orphan position"
    );
    assert_eq!(db_trade_count(&journal).await, 1, "trade recorded");
    assert_eq!(portfolio.closed_trades().len(), 1);
}

// ── remove_synced_position is idempotent on missing ids ────────────────────

#[tokio::test]
async fn remove_synced_position_missing_id_returns_none_no_panic() {
    let db = unique_db_url("remove_missing");
    let (_journal, mut portfolio) = fresh_setup(&db).await;

    assert!(portfolio.remove_synced_position("never-existed").is_none());
    assert_eq!(portfolio.open_positions(), 0);

    // After inserting one, removing a DIFFERENT id still returns None and
    // leaves the inserted one alone.
    portfolio.sync_from_db_position(sample_position("p1"));
    assert!(portfolio.remove_synced_position("not-p1").is_none());
    assert_eq!(portfolio.open_positions(), 1);
}

// ── clear_position_cache on empty cache is a safe no-op ────────────────────

#[tokio::test]
async fn clear_position_cache_empty_is_noop() {
    let db = unique_db_url("clear_empty");
    let (_journal, mut portfolio) = fresh_setup(&db).await;

    portfolio.clear_position_cache(); // must not panic
    portfolio.clear_position_cache(); // idempotent
    assert_eq!(portfolio.open_positions(), 0);
}
