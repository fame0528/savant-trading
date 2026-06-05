# FID: Dashboard Data Dead — Shared State Not Populated, Positions Not Persisted

**Filename:** `FID-2026-0604-050-dashboard-data-dead.md`
**ID:** FID-2026-0604-050
**Severity:** critical
**Status:** created
**Created:** 2026-06-04 00:05
**Author:** Flux (opencode / mimo-v2.5-pro)

---

## Summary

The HTML dashboard at `http://localhost:8080` shows zero/placeholder for every section except AI Decisions. Balance, positions, closed trades, market insight, activity stream, and memory/calibration are all blank. The engine is running and has executed trades, but the dashboard has no visibility into any of it.

---

## Detailed Description

### Problem

Dashboard shows:
- Equity: $0.00 (actual: $35.34)
- Open Positions: 0 (actual: multiple on-chain positions from earlier trades)
- Closed Trades: 0 (actual: trades exist in journal DB)
- Market Insight: all blank
- Activity Stream: 0 events
- Memory/Calibration: all blank
- Circuit breaker: DAILY_LIMIT (stale state)
- AI Decisions: WORKING (56 decisions visible)

### Expected Behavior

Dashboard should show real balance, open positions with PnL/targets, closed trade history, market insight data, activity log, and memory calibration — immediately on startup, not after 10 ticks.

### Root Cause

Four independent failures:

**1. Shared state initialized empty, not seeded on startup**
- `SharedEngineData::new()` creates `AccountState::new(0.0)` — balance starts at $0
- PaperTrader loads real balance from `data/paper_state.json` ($35.34) at engine.rs:372
- But shared state is only synced from PaperTrader every 10 ticks (engine.rs:1823)
- Dashboard shows $0.00 for the first ~30 seconds after restart

**2. Open positions not persisted to database**
- `paper_state.json` saves positions every 10 ticks (engine.rs:1992)
- If engine crashes before tick 10, positions are lost forever
- No SQLite `positions` table exists — only in-memory HashMap
- DEX executed trades on-chain but PaperTrader has no record after restart
- `paper_state.json` shows 0 positions despite on-chain positions existing

**3. Closed trades not loaded from journal into shared state**
- `TradeJournal` has a `trades` table in SQLite with all closed trades
- On startup, journal restores balance (engine.rs:430-443) but does NOT load trades into `SharedEngineData.closed_trades`
- Shared state stays at empty Vec — dashboard shows "No closed trades yet"

**4. Activity log not populated**
- `SharedEngineData.activity_log` is written by `log_activity()` calls
- Most engine code paths don't call `log_activity()` — they use `info!()` or `log_trade!()` which go to tracing, not to the shared activity log
- Dashboard activity stream shows "No activity yet" even when engine is active

### Evidence

```
// data/paper_state.json (saved 11:38 PM):
{
  "account": { "balance": 35.34, "open_positions": 0 },
  "positions": {},
  "closed_trades": []
}

// data/alerts.jsonl — 5 trades were opened:
{"pair":"AAVE/USD","entry_price":70.38,"type":"TRADE_OPENED","timestamp":"2026-06-04T21:57:35"}
{"pair":"FLUID/USD","entry_price":1.15,"type":"TRADE_OPENED","timestamp":"2026-06-04T22:22:55"}
{"pair":"FLUID/USD","entry_price":1.15,"type":"TRADE_OPENED","timestamp":"2026-06-04T22:48:03"}
{"pair":"VANA/USD","entry_price":1.19,"type":"TRADE_OPENED","timestamp":"2026-06-04T22:56:30"}

// API /api/status returns:
{ "uptime_seconds": 0, "mode": "LIVE" }  // Engine just restarted

// API /api/portfolio returns:
{ "balance": 0.0, "equity": 0.0 }  // Shared state not seeded

// Dashboard shows:
// Equity $0.00, 0 open positions, 0 closed trades, all insight blank
```

---

## Impact Assessment

### Affected Components

- `src/core/shared.rs` — SharedEngineData initialization
- `src/engine.rs` — startup seeding, shared state sync, position persistence
- `src/monitor/journal.rs` — needs positions table, trade loading
- `src/api/mod.rs` — serves stale/empty data to dashboard
- `dashboard.html` — displays zeros

### Risk Level

- [x] Critical: Dashboard is non-functional. User has zero visibility into live positions and real money at risk.

---

## Proposed Solution

### 1. Seed shared state from PaperTrader on startup (engine.rs)

After `paper.load_state()` and journal balance restore (line ~444), immediately sync to shared state:
```rust
// Seed shared state immediately — don't wait for tick 10
{
    let mut shared_account = shared.account.write().await;
    *shared_account = paper.account().clone();
    let mut shared_positions = shared.positions.write().await;
    *shared_positions = paper.positions().values().cloned().collect();
}
```

### 2. Load closed trades from journal into shared state on startup (engine.rs)

After journal balance restore, load trades:
```rust
if let Some(ref j) = journal {
    let trades = j.get_trades(10000).await.unwrap_or_default();
    {
        let mut shared_trades = shared.closed_trades.write().await;
        *shared_trades = trades;
    }
}
```

### 3. Add `positions` table to SQLite (journal.rs)

New table:
```sql
CREATE TABLE IF NOT EXISTS positions (
    id TEXT PRIMARY KEY,
    pair TEXT NOT NULL,
    side TEXT NOT NULL,
    entry_price REAL NOT NULL,
    current_price REAL NOT NULL,
    quantity REAL NOT NULL,
    stop_loss REAL NOT NULL,
    take_profit_1 REAL NOT NULL,
    take_profit_2 REAL NOT NULL,
    take_profit_3 REAL NOT NULL,
    unrealized_pnl REAL NOT NULL,
    risk_amount REAL NOT NULL,
    strategy_name TEXT NOT NULL,
    scale_level TEXT NOT NULL,
    opened_at TEXT NOT NULL
);
```

Add `save_position()`, `load_positions()`, `delete_position()` methods.

### 4. Persist positions on open + delete on close (engine.rs)

- On position open (line ~1574): `journal.save_position(&pos).await`
- On position close (line ~1742): `journal.delete_position(&pos_id).await`
- On startup (line ~444): `paper.positions_mut().extend(journal.load_positions().await)`

### 5. Write activity log entries for trade events (engine.rs)

Add `shared.log_activity()` calls at:
- Position opened: `log_activity(Trade, pair, "OPENED ...")`
- Position closed: `log_activity(Trade, pair, "CLOSED ... PnL: ...")`
- Trailing stop: `log_activity(Trade, pair, "TRAIL SL ...")`
- TP scale-out: `log_activity(Trade, pair, "TP1/TP2/TP3 hit ...")`
- Circuit breaker: `log_activity(Warning, pair, "CIRCUIT BREAKER ...")`

### 6. Seed insight data on startup

After insight aggregator is created, run one initial fetch and sync to shared state:
```rust
let initial_insight = insight.refresh().await;
{
    let mut shared_insight = shared.insight.write().await;
    *shared_insight = initial_insight;
}
```

---

## Verification

```bash
cargo build
cargo test
cargo clippy -- -D warnings
# Start engine → open http://localhost:8080 → verify all sections show real data
# Kill engine → restart → verify positions and trades survive
```

---

## Perfection Loop

### Loop 1

- **RED:** 6 findings. Shared state not seeded on startup, no positions table, trades not loaded from journal, activity log not populated, insight not seeded, no position persistence on open/close.
- **GREEN:** —
- **AUDIT:** —
- **CHANGE DELTA:** —

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —
