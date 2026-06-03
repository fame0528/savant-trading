# Agent Prompt — Preston's Agent

> **Read this before doing ANYTHING. This is your bootstrap file.**

---

## Step 0: Load ECHO Protocol

```bash
cat ECHO.md
```

Read the ENTIRE file. Confirm all 15 laws:

- [ ] Laws 1-4 (Core) — ALWAYS enforced, no exceptions
- [ ] Laws 5-15 (Extended) — enforced when `strict_mode: true`
- [ ] Perfection Loop understood (RED → GREEN → AUDIT → SELF-CORRECT → COMPLETE)
- [ ] Circuit breaker rules understood (10% char change max, 500-char verification sample)

**strict_mode is `true`.** All 15 laws are enforced.

---

## Step 1: Load Protocol Config

```bash
cat protocol.config.yaml
```

Get project-specific commands (build, test, lint).

---

## Step 2: Review LEARNINGS.md

```bash
cat dev/LEARNINGS.md
```

Review known issues and patterns from previous sessions.

---

## Step 3: Review Open FIDs

```bash
ls dev/fids/
```

**Active FID:** `FID-2026-0603-029-port-kraken-improvements.md` — this is your primary task.

Read it:
```bash
cat dev/fids/FID-2026-0603-029-port-kraken-improvements.md
```

---

## Step 4: Review Handoff Document

```bash
cat dev/HANDOFF-OTHER-DEV.md
```

This has everything you need: what was cherry-picked, what needs porting, how to port methods, key files, and current context.

---

## Step 5: Review Merge Strategy

```bash
cat dev/MERGE-STRATEGY.md
```

This explains WHY we cherry-picked instead of merged, and what was excluded.

---

## Step 6: Create Session Summary

Create `dev/session-summaries/YYYY-MM-DD-HHMM.md` with:
- Initial state assessment
- Planned work (port Kraken improvements per FID-029)
- Dependencies identified

---

## Your Task

Port Kraken execution improvements from the old branch (`origin/feat/kraken-execution-v2`) into the current `preston` branch (based on `main`).

### What to Port

**In `src/execution/kraken.rs` (10 methods):**

1. `cancel_all()` — Cancel all resting orders on startup
2. `fetch_balance_raw()` — Get raw asset balances for reconciliation
3. `get_ticker_price(pair)` — Get current price from Kraken ticker
4. `market_sell_all(pair, amount)` — Emergency liquidation
5. Order fill confirmation — Query `QueryOrders` to verify fill price
6. Long-only enforcement — Spot account can't short
7. Min-order/precision validation — `ordermin`/`costmin`/lot-rounding
8. Real kill-switch — CancelAll + flatten
9. Fee-correct close PnL — Use real fill price
10. Sub-penny price formatting — Per-pair decimals

**In `src/engine.rs` (6 features):**

1. Cancel-all on startup — Clear stale resting orders
2. Asset reconciliation — Track orphaned Kraken positions
3. Balance sync after reconciliation — Re-fetch balance
4. `SAVANT_LIQUIDATE_ON_START` — Env var for auto-liquidation
5. Higher timeframe trend filter — `htf_uptrend()` function
6. `live_trader` variable — Separate from `executor`

### How to Port

1. Read their version: `git show origin/feat/kraken-execution-v2:src/execution/kraken.rs`
2. Read our version: `cat src/execution/kraken.rs`
3. Add the method to our `KrakenTrader` impl block
4. Test: `cargo build --release && cargo test`
5. Commit: `git commit -m "feat: add [method] to KrakenTrader"`
6. Repeat for each method

---

## Rules

### DO

1. **Read ECHO.md first** — All 15 laws are mandatory
2. **Work on the `preston` branch** — NOT `main`
3. **Port one method at a time** — Test after each addition
4. **Keep DEX path intact** — Don't delete or modify `src/execution/dex/`
5. **Keep console logging** — Don't delete `src/core/console.rs`
6. **Keep mimo-v2.5-pro** — Don't change the model in config
7. **Run `cargo test` after every change** — 187+ tests must pass
8. **Run `cargo clippy` before pushing** — Zero warnings
9. **Create FIDs for issues** — In `dev/fids/`
10. **Update LEARNINGS.md** — In `dev/`
11. **Use `eprintln!` in Phase 3** — NOT `tracing::info!` (tracing deadlocks with API server)
12. **Follow the Perfection Loop** — RED → GREEN → AUDIT → COMPLETE for every change

### DON'T

1. **Don't push to `main`** — Use the `preston` branch
2. **Don't delete DEX files** — `src/execution/dex/` is live code running with real money
3. **Don't delete console.rs** — It's our enterprise logging system
4. **Don't change the model** — `xiaomi/mimo-v2.5-pro` is non-negotiable
5. **Don't merge without testing** — `cargo build --release && cargo test`
6. **Don't use `tracing::info!` in Phase 3** — Use `eprintln!` (tracing deadlocks with API server)
7. **Don't add comments** — Unless asked (ECHO Law 5)
8. **Don't touch `src/engine.rs` Phase 3 execution block** — Lines 1047-1310 have DEX logic with 60s timeouts and retry logic
9. **Don't touch `config/default.toml` model/provider settings** — Keep OpenRouter + mimo-v2.5-pro

---

## Key Files

| File | Purpose | Touch? |
|------|---------|--------|
| `src/execution/kraken.rs` | KrakenTrader — your primary target | YES |
| `src/engine.rs` | Main loop — add reconciliation carefully | CAREFULLY |
| `src/execution/engine.rs` | ExecutionEngine trait (shared) | NO |
| `src/execution/dex/` | DEX execution (live with real money) | NEVER |
| `src/core/console.rs` | Enterprise logging | NEVER |
| `src/agent/decision_parser.rs` | Decision parser (already merged) | NO |
| `src/risk/position.rs` | Position sizer (already merged) | NO |
| `config/default.toml` | Main config | CAREFULLY |
| `dev/fids/FID-2026-0603-029-port-kraken-improvements.md` | Your TODO list | READ |
| `dev/HANDOFF-OTHER-DEV.md` | Full context | READ |
| `dev/MERGE-STRATEGY.md` | Why we cherry-picked | READ |

---

## Reference Code

Your old branch is preserved on origin for reference:

```bash
# Read your old kraken.rs
git show origin/feat/kraken-execution-v2:src/execution/kraken.rs

# Read your old engine.rs
git show origin/feat/kraken-execution-v2:src/engine.rs

# Read your old config
git show origin/feat/kraken-execution-v2:config/default.toml
```

**Do NOT merge the old branch.** Use it as reference only. Port methods one at a time into the current codebase.

---

## Current Context

- **Branch:** `preston` (based on `main` at `fc03292`)
- **Version:** v0.7.0
- **Wallet:** `0x543ca0434b84ad38c858d2d178d2082521711fbc` (Arbitrum)
- **USDC balance:** ~$35 on-chain
- **Model:** `xiaomi/mimo-v2.5-pro` via OpenRouter
- **Tests:** 187 passing, clippy clean
- **DEX path:** Live with real money — DO NOT break it

---

## Verification Checklist

Before every commit:

- [ ] `cargo build --release` — zero errors
- [ ] `cargo test` — 187+ tests pass
- [ ] `cargo clippy` — zero warnings
- [ ] DEX path still intact (grep for `DexTrader` in engine.rs)
- [ ] Console logging still intact (grep for `savant_log` in console.rs)
- [ ] Model still mimo-v2.5-pro (grep for `model` in config/default.toml)

---

## Summary

1. Read ECHO.md — all 15 laws enforced
2. Read FID-029 — your task list
3. Read HANDOFF-OTHER-DEV.md — full context
4. Work on `preston` branch — NOT `main`
5. Port one method at a time — test after each
6. Don't break DEX path — it's live with real money
7. Don't change the model — mimo-v2.5-pro is non-negotiable
8. Perfection Loop on every change — RED → GREEN → AUDIT → COMPLETE

Good luck.
