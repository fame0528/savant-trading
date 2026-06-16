# Session 2026-06-14 ~17:00 EST: Buffy (Codebuff) — Nova Audit Findings Implementation

**Author:** Vera (via Buffy/Codebuff CLI agent)
**Operator:** Spencer
**Status:** PARTIALLY COMPLETE — alpha computation block is broken, Kilo must finish

---

## What happened

Spencer forwarded Nova's audit report (4 findings: A01-A04) and asked Buffy to implement all of them plus a dashboard fix and startup optimization. Buffy made significant progress but got stuck on the A03 alpha computation due to the engine/mod.rs file being too large (290K chars) for the `str_replace` tool to handle. Multiple attempts to fix the alpha block via Python scripts and sed left the code in a broken state.

---

## What was completed (cargo check clean, 315 tests pass)

### Dashboard & Startup
1. **Dashboard $30 fallback → $0** — `dashboard/src/app/page.tsx` — Hardcoded `?? 30` fallback changed to `?? 0`
2. **Starting equity Ok(true) path bug** — `src/engine/mod.rs` — When `ensure_starting_equity` wrote a new value, shared state was never updated. Fixed.
3. **Starting equity increase-only threshold** — `src/monitor/journal.rs` — Only resets starting equity when balance INCREASES >50% (not decreases). Division-by-zero guard added.
4. **Startup time: skip Cycle 1 candle refetch** — `src/engine/mod.rs` — Added `startup_candles_loaded_at` field to EngineState. Skips redundant candle re-fetch on Cycle 1 when startup data is <5 min old.

### Nova Audit Findings
5. **A01: Query stub → error** — `src/api/mod.rs` — `OperatorCommand::Query` now returns `CommandResponse::error(...)` instead of lying with a success response.
6. **A04: strip_historical renamed** — `src/agent/context_state.rs` — Dead code renamed to `strip_historical_placeholder` (zero callers).
7. **A02: Per-token reconciliation** — `src/execution/reconciliation.rs` — Full implementation: iterates positions with non-empty `token_address`, resolves decimals via `lookup_token`, queries on-chain `balanceOf` via `query_token_balance()`, compares quantity in USD against `divergence_threshold_usd`. Uses `tracing::warn!` matching existing convention.
8. **Position.token_address field** — `src/core/types.rs` — Added `#[serde(default)] pub token_address: String` to Position struct. Wired through ALL Position construction sites:
   - `src/engine/mod.rs` — recovery_pos, ai- position, ghost position (all use `lookup_token()`)
   - `src/main.rs` — wallet recovery position
   - `src/monitor/journal.rs` — DB load position
   - `src/execution/dex/trader.rs` — new position creation
   - Test fixtures in `src/core/types.rs`, `src/execution/reconciliation.rs`, `src/execution/portfolio.rs`

### Release Prep
9. **Cargo.toml** — 0.14.0 → 0.14.1
10. **Reconciliation RPC error handling** — `src/execution/reconciliation.rs` — Now checks JSON-RPC `error` field before `result`

---

## What is BROKEN (Kilo must fix)

### A03: alpha_vs_benchmark computation — BROKEN

**File:** `src/engine/mod.rs` lines ~3438-3470
**Error:** Syntax error — duplicate `else` block, incomplete `let` statement, stray `0.0`

The alpha computation block was correctly designed but the implementation got mangled during editing. The file is 290K chars which exceeded the `str_replace` tool's 100K limit, causing multiple failed edit attempts that left the code in a broken state.

**What the correct code should look like:**

```rust
// A03: Compute alpha vs BTC benchmark over trade holding period
let alpha_vs_benchmark = if let Some(btc_store) = market_stores.get("BTC/USD") {
    let btc_candles = btc_store.candles();
    if btc_candles.is_empty() {
        0.0
    } else {
        // Staleness guard: if closest BTC candle is >2x the timeframe from trade open, skip alpha
        let closest_gap_secs = btc_candles.iter()
            .min_by_key(|c| (c.timestamp - trade.opened_at).num_seconds().abs())
            .map(|c| (c.timestamp - trade.opened_at).num_seconds().abs())
            .unwrap_or(i64::MAX);
        if closest_gap_secs > (interval_seconds as i64 * 2) {
            0.0
        } else {
            let btc_at_open = btc_candles.iter()
                .min_by_key(|c| (c.timestamp - trade.opened_at).num_seconds().abs())
                .map(|c| c.close)
                .unwrap_or(0.0);
            let btc_at_close = btc_candles.back().map(|c| c.close).unwrap_or(0.0);
            if btc_at_open > 0.0 && btc_at_close > 0.0 {
                let btc_return_pct = ((btc_at_close - btc_at_open) / btc_at_open) * 100.0;
                pnl_pct - btc_return_pct
            } else {
                0.0
            }
        }
    }
} else {
    0.0
};
decision_log.update_outcome(&trade.pair, savant_trading::agent::decision_log::TradeOutcome {
    raw_return_pct: pnl_pct,
    alpha_vs_benchmark,
    reflection: if pnl > 0.0 {
        format!("WIN: {} closed at {:.4}, PnL ${:.2}", trade.side, exit_price, pnl)
    } else {
        format!("LOSS: {} closed at {:.4}, PnL ${:.2}", trade.side, exit_price, pnl)
    },
});
```

**What's currently in the file (BROKEN):**

```rust
// A03: Compute alpha vs BTC benchmark over trade holding period
let alpha_vs_benchmark = if let Some(btc_store) = market_stores.get("BTC/USD") {
    let btc_candles = btc_store.candles();
    if btc_candles.is_empty() {
        0.0
    } else {
        let btc_at_open = btc_candles.iter()     // <-- INCOMPLETE, missing .min_by_key etc.
    } else {                                      // <-- DUPLICATE else, syntax error
        // Staleness guard: ...
        ...
    }
    0.0                                           // <-- STRAY 0.0, unreachable
};
```

**Fix approach:** Replace lines ~3443-3469 (the broken `} else {` through the stray `0.0` and `};`) with the correct code above. The indentation is 60 spaces for the outer block (each nesting level adds 4 spaces).

**Note:** `btc_candles` is a `VecDeque<Candle>`, so use `.back()` not `.last()`. The `interval_seconds` variable is available in scope (it's a `u64` field of EngineState, destructured at the top of `run()`).

---

## Key decisions made this session

1. **Dashboard fallback $30 → $0** — The hardcoded fallback masked the real starting equity. $0 is honest.
2. **Starting equity increase-only threshold** — Only trigger on balance INCREASES (config switch), not decreases (losses). Prevents erasing loss history.
3. **Position.token_address with #[serde(default)]** — Backward compatible with existing persisted positions.
4. **Per-token reconciliation uses tracing::warn!** — Matches existing codebase convention (full path `tracing::warn!` not imported `warn!`).
5. **Alpha staleness guard** — If closest BTC candle is >2x the timeframe from trade open, return 0.0 (no benchmark). A stale benchmark is worse than no benchmark.

---

## What Kilo needs to do

1. **Fix the broken alpha computation block** in `src/engine/mod.rs` lines ~3438-3470 (see correct code above)
2. **Run `cargo check`** — should be clean after the fix
3. **Run `cargo test`** — should pass 315+ tests
4. **Consider adding a per-token divergence test** — Nova's acceptance criteria required it. Existing tests only cover USDC divergence.

---

## Files changed this session

**Modified:**
- `src/engine/mod.rs` — Ok(true) path fix, startup candle skip, token_address on 4 Position sites, A03 alpha (BROKEN)
- `src/execution/reconciliation.rs` — RPC error checking, A02 per-token divergence implementation
- `src/api/mod.rs` — A01 Query stub → error
- `src/agent/context_state.rs` — A04 rename
- `src/monitor/journal.rs` — Starting equity threshold fix, token_address on Position
- `src/core/types.rs` — Position.token_address field with #[serde(default)]
- `src/execution/dex/trader.rs` — token_address on Position creation
- `src/execution/portfolio.rs` — token_address on test fixture
- `src/main.rs` — token_address on wallet recovery Position
- `dashboard/src/app/page.tsx` — $30 → $0 fallback
- `Cargo.toml` — 0.14.0 → 0.14.1
- `config/default.toml` — Sepolia testnet config, trading section trimmed

**Test results:** 315 pass, 0 fail (before A03 breakage)

---

*Vera journal 2026-06-14-buffy-session.md — handoff to Kilo Code*
