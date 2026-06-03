# Handoff — Instructions for Other Dev's Agent

**Date:** 2026-06-03
**From:** Main branch (Spencer's agent)
**To:** Other dev's agent
**Current Version:** v0.7.0

---

## What Happened

Your branch (`feat/kraken-execution-v2`) was reviewed and selectively merged into `main`. We did NOT do a full merge because it would have broken the DEX execution path that's currently running live with real money on Arbitrum.

### What We Cherry-Picked (8 files)

| File | What You Get |
|------|-------------|
| `src/agent/decision_parser.rs` | Your casing-tolerant parser (BUY/SELL/CLOSE/ADJUST_STOP), AdjustStop validation fix, confidence floor exemption for position management |
| `src/risk/position.rs` | Your min order value ($1), max position pct (30%), balance cap |
| `dashboard.html` | Your single-file vanilla JS dashboard |
| `config/canary.toml` | Your canary config |
| `stats.ps1` | Your scoreboard script |
| `run-canary.ps1` | Your launcher script |
| `IDEAS.md` | Your ideas document |
| `dev/fids/FID-2026-0603-001-inherited-base-clippy-lints.md` | Your FID |

### What We Did NOT Merge (and Why)

| File | Why Not |
|------|---------|
| `engine.rs` | 10 conflict zones — would break DEX execution path |
| `execution/kraken.rs` | 6 conflict zones — needs manual port |
| `config/default.toml` | Changed model to DeepSeek V4 — we keep mimo-v2.5-pro |
| `agent/soul.md` | Changed brain reference — we keep mimo-v2.5-pro |
| `agent/provider.rs` | Changed default model — we keep mimo-v2.5-pro |
| `core/config.rs` | Changed default provider — we keep OpenRouter |
| Deleted DEX files | We need DEX execution (running live on Arbitrum) |

---

## Current State of `main`

### What's Working

- **DEX execution** — 0x API on Arbitrum, live with $35 USDC
- **Console logging** — `[Savant Trading] [MM-DD-YYYY HH:mm AM/PM] [ACTION] [RESULT]`
- **Casing-tolerant parser** — Your code, merged and working
- **Position sizer** — Your improvements, merged and working
- **187 tests passing** — Clippy clean
- **v0.7.0** — Released on GitHub

### What's NOT Working (Yet)

- **Kraken execution** — Missing safety rails from your branch
- **Higher timeframe trend filter** — Not ported yet
- **Asset reconciliation** — Not ported yet
- **Fill confirmation** — Not ported yet

---

## What You Should Work On Next

### Priority 1: Port Kraken Safety Rails (FID-029)

This is documented in `dev/fids/FID-2026-0603-029-port-kraken-improvements.md`. Your branch has these methods that need to be added to the current `main`:

#### In `src/execution/kraken.rs`:

1. **`cancel_all()`** — Cancel all resting orders on startup
2. **`fetch_balance_raw()`** — Get raw asset balances for reconciliation
3. **`get_ticker_price(pair)`** — Get current price from Kraken ticker
4. **`market_sell_all(pair, amount)`** — Emergency liquidation
5. **Order fill confirmation** — Query `QueryOrders` to verify fill price
6. **Long-only enforcement** — Spot account can't short
7. **Min-order/precision validation** — `ordermin`/`costmin`/lot-rounding
8. **Real kill-switch** — CancelAll + flatten
9. **Fee-correct close PnL** — Use real fill price
10. **Sub-penny price formatting** — Per-pair decimals

#### In `src/engine.rs`:

1. **Cancel-all on startup** — Clear stale resting orders
2. **Asset reconciliation** — Track orphaned Kraken positions
3. **Balance sync after reconciliation** — Re-fetch balance
4. **`SAVANT_LIQUIDATE_ON_START`** — Env var for auto-liquidation
5. **Higher timeframe trend filter** — `htf_uptrend()` function

### Priority 2: Test Your Dashboard

Open `http://localhost:8080/dashboard.html` in a browser. If it doesn't load, the API server needs a static file route. Let me know and I'll wire it.

### Priority 3: Review Config

Your `entry_mode = "marketable"` setting is in `config/canary.toml`. If you want it in the main config, add it to `config/default.toml`.

---

## Rules for Your Agent

### DO

1. **Read ECHO.md first** — All 15 laws are mandatory
2. **Create a new branch** — `git checkout -b feat/kraken-port main`
3. **Port one method at a time** — Test after each addition
4. **Keep DEX path intact** — Don't delete or modify `src/execution/dex/`
5. **Keep console logging** — Don't delete `src/core/console.rs`
6. **Keep mimo-v2.5-pro** — Don't change the model in config
7. **Run `cargo test` after every change** — 187+ tests must pass
8. **Run `cargo clippy` before pushing** — Zero warnings
9. **Create FIDs for issues** — In `dev/fids/`
10. **Update LEARNINGS.md** — In `dev/`

### DON'T

1. **Don't push to `main`** — Use a feature branch
2. **Don't delete DEX files** — `src/execution/dex/` is live code
3. **Don't delete console.rs** — It's our logging system
4. **Don't change the model** — mimo-v2.5-pro is non-negotiable
5. **Don't merge without testing** — `cargo build --release && cargo test`
6. **Don't use `tracing::info!` in Phase 3** — Use `eprintln!` (tracing deadlocks with API server)
7. **Don't add comments** — Unless asked (ECHO Law 5)

---

## How to Port a Method

### Example: Porting `cancel_all()`

1. Read their version:
   ```bash
   git show origin/feat/kraken-execution-v2:src/execution/kraken.rs | Select-String -Pattern "cancel_all" -Context 0,50
   ```

2. Read our version:
   ```bash
   Get-Content src/execution/kraken.rs | Select-String -Pattern "cancel_all" -Context 0,10
   ```

3. Add the method to our `KrakenTrader` impl block (after existing methods)

4. Test:
   ```bash
   cargo build --release
   cargo test
   cargo clippy
   ```

5. Commit:
   ```bash
   git add src/execution/kraken.rs
   git commit -m "feat: add cancel_all() to KrakenTrader"
   ```

---

## Key Files

| File | Purpose |
|------|---------|
| `src/engine.rs` | Main decision loop (4500+ lines) |
| `src/execution/kraken.rs` | KrakenTrader implementation |
| `src/execution/engine.rs` | ExecutionEngine trait (shared by DEX and Kraken) |
| `src/execution/paper.rs` | PaperTrader (position tracker) |
| `src/core/console.rs` | Enterprise logging (don't touch) |
| `src/agent/decision_parser.rs` | Decision parser (already merged your changes) |
| `src/risk/position.rs` | Position sizer (already merged your changes) |
| `config/default.toml` | Main config |
| `dev/fids/FID-2026-0603-029-port-kraken-improvements.md` | Your TODO list |
| `dev/MERGE-STRATEGY.md` | Why we cherry-picked instead of merged |

---

## Context You Need

- **Wallet:** `0x543ca0434b84ad38c858d2d178d2082521711fbc` (Arbitrum)
- **USDC balance:** ~$35 on-chain
- **ETH balance:** ~0.00374 ETH (~$9 for gas)
- **Nonce:** 7
- **Model:** `xiaomi/mimo-v2.5-pro` via OpenRouter
- **Fear & Greed:** 11 (Extreme Fear)
- **MVRV:** 1.25 (neutral, not capitulation)
- **SOPR:** 0.9741 (capitulation)
- **Config:** 8 pairs, 3 max positions, 5% daily loss, 10% drawdown

---

## Summary

Your branch has valuable Kraken improvements. We cherry-picked the safe parts (parser, position sizer, dashboard) and documented the rest in FID-029. Your next job is to port the Kraken safety rails method-by-method into `main`, testing after each addition. Don't break the DEX path — it's live with real money.

Good luck.
