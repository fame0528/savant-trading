# FID: Port Kraken improvements from feat/kraken-execution-v2 branch

**Filename:** `FID-2026-0603-029-port-kraken-improvements.md`
**ID:** FID-2026-0603-029
**Severity:** high
**Status:** created
**Created:** 2026-06-03 22:00
**Author:** Agent

---

## Summary

The other dev's branch (`feat/kraken-execution-v2`) has significant Kraken execution improvements that were NOT merged due to 10+ conflict zones in engine.rs and 6 in kraken.rs. These improvements need to be manually ported into main without breaking the DEX execution path.

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.94, tokio
- **Branch:** `main` (current), `feat/kraken-execution-v2` (reference)
- **Commit:** `f5f22c7` (main), `97b792f` (other dev's branch)

## Detailed Description

### Problem

The other dev's branch has valuable Kraken improvements but was developed independently from our DEX work. A full merge would have deleted the entire DEX execution path, console logging, and all our fixes. Cherry-picking was the correct approach, but left the Kraken improvements unmerged.

### What Needs Porting

#### kraken.rs Improvements (6 conflict zones)

1. **`cancel_all()`** — Cancel all resting orders on startup. Prevents orphaned stop-losses from prior runs.
2. **`fetch_balance_raw()`** — Get raw asset balances from Kraken for position reconciliation.
3. **`market_sell_all()`** — Emergency liquidation of a specific asset.
4. **`get_ticker_price()`** — Get current price for a pair from Kraken ticker.
5. **Order fill confirmation** — Query Kraken `QueryOrders` to verify actual fill price before recording position.
6. **Long-only enforcement** — Spot account can't short; bearish signals skipped instead of becoming phantom shorts.
7. **Sub-penny price formatting** — Per-pair decimals from Kraken AssetPairs.
8. **Min-order/precision validation** — `ordermin`/`costmin`/lot-rounding enforced.
9. **Real kill-switch** — CancelAll + flatten on Kraken.
10. **Fee-correct close PnL** — At the real fill price.

#### engine.rs Improvements (10 conflict zones)

1. **Asset reconciliation on startup** — Orphaned Kraken positions tracked as open positions.
2. **Cancel-all on startup** — Clear stale resting orders.
3. **Balance sync after reconciliation** — Re-fetch balance so cleanup sales don't trigger drawdown.
4. **`SAVANT_LIQUIDATE_ON_START`** — Env var for auto-liquidation of orphaned assets.
5. **Higher timeframe trend filter** — `htf_uptrend()` function using 1d→4h→1h EMA+slope.
6. **`live_trader` variable** — Separate from `executor` for Kraken-specific operations.

#### config Improvements

1. **`entry_mode = "marketable"`** — Fill near market with capped slippage instead of post-only.
2. **Better model comments** — Document model selection rationale.

### What Was Already Cherry-Picked

- `decision_parser.rs` — Casing-tolerant parser, AdjustStop fix, confidence floor exemption
- `risk/position.rs` — Min order value, max position pct, balance cap
- `dashboard.html` — New HTML dashboard
- `config/canary.toml` — Canary config
- `stats.ps1` — Scoreboard script
- `run-canary.ps1` — Launcher script

### Expected Behavior

After porting:
- Kraken execution has all safety rails (cancel_all, fill confirmation, long-only, min-order)
- Engine reconciles orphaned Kraken positions on startup
- Higher timeframe trend filter prevents buying dips into downtrends
- DEX execution path remains fully intact
- Console logging remains fully intact

## Impact Assessment

### Affected Components

- `src/execution/kraken.rs` — Major additions
- `src/engine.rs` — New reconciliation logic
- `config/default.toml` — New settings

### Risk Level

- [x] High: Kraken execution missing safety rails without these improvements

## Proposed Solution

### Steps

1. Read their `kraken.rs` from `origin/feat/kraken-execution-v2`
2. Port `cancel_all()`, `fetch_balance_raw()`, `market_sell_all()`, `get_ticker_price()` methods
3. Port order fill confirmation logic
4. Port long-only enforcement
5. Port sub-penny formatting and min-order validation
6. Port real kill-switch
7. Test: `cargo build --release && cargo test`
8. Read their `engine.rs` from `origin/feat/kraken-execution-v2`
9. Port asset reconciliation and cancel-all on startup
10. Port higher timeframe trend filter
11. Test: `cargo build --release && cargo test`
12. Verify DEX path still works: `cargo test` all 187+ tests

### Verification

- `cargo build --release` — zero errors
- `cargo test` — 187+ tests pass
- `cargo clippy` — zero warnings
- Manual check: DEX execution path still present in engine.rs

## Perfection Loop

### Loop 1

- **RED:** Kraken execution missing safety rails (cancel_all, fill confirmation, long-only)
- **GREEN:** Port methods from their branch one at a time
- **AUDIT:** Build + test after each method
- **CHANGE DELTA:** TBD

## Resolution

- **Fixed By:** (pending)
- **Fixed Date:** (pending)
- **Fix Description:** (pending)
- **Tests Added:** (pending)
- **Verified By:** (pending)
- **Commit/PR:** (pending)

## Lessons Learned

- When two branches diverge significantly, cherry-picking is safer than merging
- The `ExecutionEngine` trait abstraction makes it possible to add Kraken methods without affecting DEX
- Always preserve the working path (DEX) when porting improvements from a parallel branch
