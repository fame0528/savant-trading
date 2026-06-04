# FID: Port Kraken improvements from feat/kraken-execution-v2 branch

**Filename:** `FID-2026-0603-029-port-kraken-improvements.md`
**ID:** FID-2026-0603-029
**Severity:** high
**Status:** deferred
**Created:** 2026-06-03 22:00
**Author:** Agent

---

## Summary

The other dev's branch (`feat/kraken-execution-v2`) has significant Kraken execution improvements that were NOT merged due to 10+ conflict zones in engine.rs and 6 in kraken.rs. These improvements need to be manually ported into main without breaking the DEX execution path.

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.94, tokio
- **Tool Versions:** cargo 1.94, rustc 1.94, clippy (built-in)
- **Branch:** `main` (current), `feat/kraken-execution-v2` (reference)
- **Commit:** `fba49e2` (main), `97b792f` (other dev's branch)

## Detailed Description

### Problem

The other dev's branch has valuable Kraken improvements but was developed independently from our DEX work. A full merge would have deleted the entire DEX execution path, console logging, and all our fixes. Cherry-picking was the correct approach, but left the Kraken improvements unmerged.

### Root Cause

Two developers worked in parallel on different execution paths (DEX vs Kraken) without a shared merge strategy. The other dev branched from an older `main` (before DEX work) and made sweeping changes that conflicted with our DEX additions. The `engine.rs` file (4500+ lines) is the main conflict zone because both paths route through it.

### Evidence

```text
# Merge attempt showed 10 conflict zones in engine.rs:
$ git merge origin/feat/kraken-execution-v2 --no-commit
CONFLICT (content): Merge conflict in config/default.toml
CONFLICT (content): Merge conflict in src/agent/provider.rs
CONFLICT (content): Merge conflict in src/core/config.rs
CONFLICT (content): Merge conflict in src/engine.rs
CONFLICT (add/add): Merge conflict in src/execution/kraken.rs

# Their branch deleted our DEX files:
D src/execution/dex/trader.rs   (971 lines — our entire DEX execution)
D src/execution/dex/mod.rs      (402 lines — token resolution)
D src/execution/dex/zero_x.rs   (391 lines — 0x API backend)
D src/execution/dex/inch.rs     (396 lines — 1inch backend)
D src/core/console.rs           (217 lines — enterprise logging)
```

### What Needs Porting

#### kraken.rs Improvements — Priority Order

| # | Method | Priority | Complexity | Description |
|---|--------|----------|------------|-------------|
| 1 | `cancel_all()` | CRITICAL | Low | Cancel all resting orders on startup. Prevents orphaned stop-losses. |
| 2 | `fetch_balance_raw()` | CRITICAL | Low | Get raw asset balances from Kraken for position reconciliation. |
| 3 | `get_ticker_price(pair)` | HIGH | Low | Get current price for a pair from Kraken ticker. |
| 4 | `market_sell_all(pair, amount)` | HIGH | Medium | Emergency liquidation of a specific asset. |
| 5 | Order fill confirmation | HIGH | Medium | Query `QueryOrders` to verify actual fill price before recording position. |
| 6 | Long-only enforcement | HIGH | Low | Spot account can't short; bearish signals skipped. |
| 7 | Min-order/precision validation | MEDIUM | Low | `ordermin`/`costmin`/lot-rounding enforced. |
| 8 | Real kill-switch | MEDIUM | Low | CancelAll + flatten on Kraken (already partially in our code via `kill()`). |
| 9 | Fee-correct close PnL | MEDIUM | Low | Use real fill price from QueryOrders, not estimated price. |
| 10 | Sub-penny price formatting | LOW | Low | Per-pair decimals from Kraken AssetPairs. |

#### engine.rs Improvements — Priority Order

| # | Feature | Priority | Complexity | Description |
|---|---------|----------|------------|-------------|
| 1 | Cancel-all on startup | CRITICAL | Low | Clear stale resting orders before trading begins. |
| 2 | Asset reconciliation | HIGH | Medium | Orphaned Kraken positions tracked as open positions. |
| 3 | Balance sync after reconciliation | HIGH | Low | Re-fetch balance so cleanup sales don't trigger drawdown. |
| 4 | `SAVANT_LIQUIDATE_ON_START` env var | MEDIUM | Low | Opt-in auto-liquidation of orphaned assets. |
| 5 | Higher timeframe trend filter | MEDIUM | Medium | `htf_uptrend()` using 1d→4h→1h EMA+slope. Prevents buying dips into downtrends. |
| 6 | `live_trader` variable | LOW | Low | Separate from `executor` for Kraken-specific operations. |

#### config Improvements

| # | Setting | Priority | Description |
|---|---------|----------|-------------|
| 1 | `entry_mode = "marketable"` | MEDIUM | Fill near market with capped slippage instead of post-only. |

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

- `src/execution/kraken.rs` — Major additions (10 new methods)
- `src/engine.rs` — New reconciliation logic (6 features)
- `config/default.toml` — New settings

### Risk Level

- [x] High: Kraken execution missing safety rails without these improvements
- [ ] Critical: System crash, data loss, or security vulnerability
- [ ] Medium: Feature degraded, workaround exists
- [ ] Low: Minor issue, cosmetic, or edge case

## Proposed Solution

### Approach

Port methods from their branch one at a time, testing after each addition. The `ExecutionEngine` trait is identical on both branches, so methods can be added without breaking DEX.

### Steps

1. Read their `kraken.rs` from `origin/feat/kraken-execution-v2`
2. Port `cancel_all()` — add to `KrakenTrader` impl block
3. Port `fetch_balance_raw()` — add to `KrakenTrader` impl block
4. Port `get_ticker_price()` — add to `KrakenTrader` impl block
5. Port `market_sell_all()` — add to `KrakenTrader` impl block
6. Port order fill confirmation — modify `place_order()` to verify fill
7. Port long-only enforcement — add check in `place_order()`
8. Port min-order/precision validation — add check in `place_order()`
9. Test: `cargo build --release && cargo test`
10. Read their `engine.rs` from `origin/feat/kraken-execution-v2`
11. Port cancel-all on startup — add to engine initialization
12. Port asset reconciliation — add after PaperTrader init
13. Port balance sync — add after reconciliation
14. Port `SAVANT_LIQUIDATE_ON_START` — add env var check
15. Port higher timeframe trend filter — add `htf_uptrend()` function
16. Test: `cargo build --release && cargo test`
17. Verify DEX path: grep for `DexTrader`, `execute_swap`, `resolve_pair` in engine.rs

### Verification

- `cargo build --release` — zero errors
- `cargo test` — 187+ tests pass
- `cargo clippy` — zero warnings
- Manual check: DEX execution path still present in engine.rs

## Perfection Loop

### Loop 1

- **RED:** Kraken execution missing safety rails (cancel_all, fill confirmation, long-only, min-order). Engine missing reconciliation for orphaned Kraken positions.
- **GREEN:** Port 10 methods from their kraken.rs, 6 features from their engine.rs. Each tested individually.
- **AUDIT:** (pending — will run after porting)
- **CHANGE DELTA:** (pending)

### Loop 2 (if needed)

- **RED:** (pending)
- **GREEN:** (pending)
- **AUDIT:** (pending)
- **CHANGE DELTA:** (pending)

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
- The `engine.rs` file (4500+ lines) is a merge hazard — consider splitting into smaller modules
