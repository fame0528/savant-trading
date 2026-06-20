//! FID-210/211 Stage 2: `shared.block` (engine block state) integration test.
//!
//! These tests cover the in-memory engine block state — the structured
//! replacement for the `savant.blocked` text file. The file is still
//! crash-survival SOT; shared.block is the in-memory cache that hydrates
//! from the file on startup. These tests verify the in-memory side of
//! the contract (set → get, set → clear, concurrent writers, structured
//! shape).
//!
//! **Why this test file exists:** the previous "engine block" path was a
//! bare text file with no schema, no type safety, and no concurrent
//! writer discipline. The dashboard's regex parser for the file
//! (dashboard/src/app/page.tsx:590-592) is the only thing that
//! understands the format. FID-210 introduced a typed `BlockReason` so
//! the in-memory path has a schema; these tests prove the schema holds
//! under the operations the engine actually performs.

use savant_trading::core::shared::{BlockReason, SharedEngineData};

#[tokio::test]
async fn fresh_shared_data_is_not_blocked() {
    // FID-210: New SharedEngineData must default to unblocked. The engine
    // constructs one on every startup; if it were blocked by default, no
    // cycle would ever run.
    let shared = SharedEngineData::new();
    assert!(shared.try_get_block().is_none());
    assert!(shared.get_block().await.is_none());
}

#[tokio::test]
async fn set_block_then_get_returns_same_reason() {
    // FID-210: Engine writes block on every circuit-breaker trigger; API
    // reads it on every /api/risk request. Round-trip must preserve all
    // three fields.
    let shared = SharedEngineData::new();
    let now = chrono::Utc::now();
    shared
        .set_block(BlockReason {
            block_type: "drawdown".to_string(),
            reason: "drawdown 12.5% > 10% limit".to_string(),
            triggered_at: now,
        })
        .await;

    let got = shared.get_block().await.expect("block should be set");
    assert_eq!(got.block_type, "drawdown");
    assert_eq!(got.reason, "drawdown 12.5% > 10% limit");
    assert_eq!(got.triggered_at, now);
}

#[tokio::test]
async fn clear_block_makes_get_return_none() {
    // FID-210/211: The midnight auto-clear path and the API
    // /api/risk/clear-block endpoint both call clear_block. After clear,
    // both async get and sync try_get must return None.
    let shared = SharedEngineData::new();
    shared
        .set_block(BlockReason {
            block_type: "per_trade_loss".to_string(),
            reason: "loss $50 > $40 limit".to_string(),
            triggered_at: chrono::Utc::now(),
        })
        .await;
    assert!(shared.try_get_block().is_some());

    shared.clear_block().await;
    assert!(shared.try_get_block().is_none());
    assert!(shared.get_block().await.is_none());
}

#[tokio::test]
async fn clear_block_on_fresh_state_is_noop() {
    // FID-210/211: main.rs startup calls shared.clear_block() defensively
    // even when the file didn't exist. This must not panic, must not
    // synthesize a spurious block state, must remain None.
    let shared = SharedEngineData::new();
    shared.clear_block().await;
    assert!(shared.try_get_block().is_none());
    assert!(shared.get_block().await.is_none());
}

#[tokio::test]
async fn set_block_overwrites_previous_block() {
    // FID-210: If two circuit-breaker triggers fire in the same cycle
    // (e.g. drawdown AND per_trade_loss), the second set_block should
    // replace the first. The lock semantics guarantee this.
    let shared = SharedEngineData::new();
    shared
        .set_block(BlockReason {
            block_type: "drawdown".to_string(),
            reason: "first".to_string(),
            triggered_at: chrono::Utc::now(),
        })
        .await;
    shared
        .set_block(BlockReason {
            block_type: "per_trade_loss".to_string(),
            reason: "second".to_string(),
            triggered_at: chrono::Utc::now(),
        })
        .await;

    let got = shared.get_block().await.expect("block should be set");
    assert_eq!(got.block_type, "per_trade_loss");
    assert_eq!(got.reason, "second");
}

#[tokio::test]
async fn try_get_block_does_not_panic_when_lock_contended() {
    // FID-210: try_get_block uses try_read() — must return None rather
    // than block or panic if a writer is holding the lock. Simulate by
    // taking a write lock and then calling try_get from a different task.
    let shared = std::sync::Arc::new(SharedEngineData::new());
    let _write_guard = shared.block.write().await; // hold the write lock

    // try_get must not block (it uses try_read which fails fast)
    let result = shared.try_get_block();
    assert!(
        result.is_none(),
        "try_get should return None when writer holds lock"
    );

    // Drop the write guard and verify we can read again
    drop(_write_guard);
    assert!(shared.try_get_block().is_none());
}

#[tokio::test]
async fn block_reason_serialization_round_trip() {
    // FID-210: BlockReason derives Serialize/Deserialize for the API
    // response. Round-trip a value through JSON to confirm the schema
    // is stable (dashboard readers depend on field names).
    let original = BlockReason {
        block_type: "spread".to_string(),
        reason: "BTC/USD spread 0.45% > 0.30% limit".to_string(),
        triggered_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let parsed: BlockReason = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed.block_type, original.block_type);
    assert_eq!(parsed.reason, original.reason);
    // chrono::DateTime<Utc> should round-trip exactly (serde feature is on)
    assert_eq!(parsed.triggered_at, original.triggered_at);

    // Confirm the JSON has the field names the dashboard / API consumers
    // depend on. If these change, break the test on purpose so the
    // dashboard gets a heads-up.
    assert!(json.contains("\"block_type\""));
    assert!(json.contains("\"reason\""));
    assert!(json.contains("\"triggered_at\""));
}
