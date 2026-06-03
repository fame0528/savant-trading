# Merge Strategy — feat/kraken-execution-v2 into main

**Date:** 2026-06-03
**Branch:** `feat/kraken-execution-v2` → `main`
**Method:** Cherry-pick (not merge — too many conflicts in engine.rs)

---

## Why Cherry-Pick Instead of Merge

The other dev's branch had **10 conflict zones in engine.rs** alone (4500-line file). A full merge would have:
- Deleted the entire DEX execution path (`src/execution/dex/`)
- Deleted our console logging (`src/core/console.rs`)
- Deleted our FIDs and session summaries
- Changed the model from mimo-v2.5-pro to DeepSeek V4
- Required manual resolution of 10+ conflict zones in the most critical file

Cherry-picking preserved our DEX path, console logging, and all fixes while still getting their improvements.

---

## What Was Cherry-Picked (8 files)

### Code Improvements

| File | Change | Impact |
|------|--------|--------|
| `src/agent/decision_parser.rs` | Casing-tolerant parser: `BUY`/`SELL`/`CLOSE`/`ADJUST_STOP` aliases added to `TradeAction` enum | HIGH — AI responses with any casing now parse correctly |
| `src/agent/decision_parser.rs` | AdjustStop validation fix: skips entry price check, only validates stop_loss | MEDIUM — ADJUST_STOP actions no longer rejected |
| `src/agent/decision_parser.rs` | Confidence floor exempts Close/AdjustStop (position management) | MEDIUM — close/adjust actions not blocked by low confidence |
| `src/risk/position.rs` | Min order value ($1), max position pct (30%), balance cap | MEDIUM — prevents orders below Kraken minimums |

### New Files

| File | Description | Lines |
|------|-------------|-------|
| `dashboard.html` | Single-file vanilla JS dashboard with glassmorphic design | 596 |
| `config/canary.toml` | Canary config for testing new features | 160 |
| `stats.ps1` | Scoreboard script for tracking performance | 69 |
| `run-canary.ps1` | Launcher script for canary mode | 48 |
| `IDEAS.md` | Ideas and future improvements | 99 |
| `dev/fids/FID-2026-0603-001-inherited-base-clippy-lints.md` | FID for inherited clippy lints | 100 |

---

## What Was NOT Merged (Intentionally Excluded)

### engine.rs (10 conflict zones)

Their branch refactored engine.rs heavily but:
- Removed `create_executor()` function (we need it for DEX backend selection)
- Removed our DEX execution path (Buy/Sell/Close logic)
- Changed provider from `LlmProvider` to `LlmConfig` (different API)
- Removed OpenRouter management integration
- Changed reconciliation logic

**Action needed:** Port their Kraken improvements (asset reconciliation, cancel_all on startup, balance sync) into our engine.rs manually — without removing DEX support.

### kraken.rs (6 conflict zones)

Their branch significantly improved `KrakenTrader`:
- Added `cancel_all()` method
- Added `fetch_balance_raw()` method
- Added `market_sell_all()` method
- Added `get_ticker_price()` method
- Improved error handling
- Added order fill confirmation

**Action needed:** Merge their kraken.rs improvements into ours. The `ExecutionEngine` trait is the same, so these methods can be added without breaking DEX.

### config/default.toml (2 conflict zones)

Their branch changed:
- Model: `xiaomi/mimo-v2.5-pro` → `deepseek/deepseek-v4-flash` (WE REJECTED THIS)
- Added `entry_mode = "marketable"` setting
- Added comments about model selection

**Action needed:** Cherry-pick the `entry_mode` setting if needed, but keep mimo-v2.5-pro.

### src/agent/soul.md

Their branch changed the brain reference to DeepSeek V4 Flash and rewrote Section V (Risk Management) to focus on small account day trading ($47 capital).

**Action needed:** Review their Section V changes — the small account strategy may be valuable for our $35 balance, but keep mimo-v2.5-pro as the brain.

### src/agent/provider.rs

Changed default model to DeepSeek V4 Flash. We rejected this.

### src/core/config.rs

Changed default AiConfig to OpenRouter with DeepSeek. We rejected this.

### src/main.rs

Changed CLI arguments and startup flow. Needs careful review before merging.

---

## Files That Were Auto-Merged (No Conflicts)

These files from their branch merged cleanly into main:
- `src/core/shared.rs` — shared engine data
- `src/core/types.rs` — type definitions
- `src/execution/engine.rs` — ExecutionEngine trait
- `src/execution/mod.rs` — module declarations
- `src/execution/paper.rs` — paper trader
- `src/lib.rs` — library module declarations
- `src/main.rs` — CLI entry point (partially)

---

## Next Steps

### Priority 1: Port Kraken Improvements

Their `KrakenTrader` has significant improvements that should be merged manually:

1. `cancel_all()` — cancel all resting orders on startup
2. `fetch_balance_raw()` — get raw asset balances for reconciliation
3. `market_sell_all()` — emergency liquidation
4. `get_ticker_price()` — get current price for a pair
5. Order fill confirmation via `QueryOrders`
6. Long-only enforcement (spot account can't short)
7. Sub-penny price formatting
8. Min-order/precision validation
9. Real kill-switch (CancelAll + flatten)
10. Fee-correct close PnL

**Approach:** Read their `kraken.rs` and add these methods to ours one at a time, testing after each addition.

### Priority 2: Port engine.rs Improvements

Their engine.rs has Kraken-specific improvements:
1. Asset reconciliation on startup (orphaned Kraken positions)
2. Cancel-all on startup
3. Balance sync after reconciliation
4. `SAVANT_LIQUIDATE_ON_START` env var for auto-liquidation
5. Higher timeframe trend filter
6. `htf_uptrend()` function

**Approach:** Add these as new code blocks in our engine.rs, without removing existing DEX code.

### Priority 3: Port Config Improvements

Their config has:
1. `entry_mode = "marketable"` — fill near market with capped slippage
2. Better comments about model selection
3. Canary config for testing

**Approach:** Add `entry_mode` to our config, keep mimo-v2.5-pro.

### Priority 4: Review Dashboard

Their `dashboard.html` is a complete single-file dashboard. Test it with our API server at localhost:8080.

---

## Summary

| Category | Files | Status |
|----------|-------|--------|
| Cherry-picked | 8 | ✅ Merged into main |
| engine.rs conflicts | 10 zones | ⏳ Needs manual port |
| kraken.rs conflicts | 6 zones | ⏳ Needs manual port |
| config conflicts | 2 zones | ⏳ Needs manual port |
| soul.md | 1 | ⏳ Review needed |
| Rejected (model change) | 3 | ❌ Not merged |

**Branch status:** `feat/kraken-execution-v2` still exists on origin. Can be used as reference for manual porting.
