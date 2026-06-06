# HANDOFF.md — Session Compaction Document

**Generated:** 2026-06-05 20:24 EST
**Version:** 0.9.1
**Branch:** main (latest commit: d3fa281)

---

## Project State

**Savant Trading** is an autonomous DEX trading engine running on Arbitrum via 0x API. Currently holding 2 open long positions (ETH + LINK) with trailing stops active. $26.15 total portfolio value.

### Current Holdings (on-chain, wallet 0x543CA...11fBC)
- **USDC**: $0.26 (cash for new trades)
- **WETH**: 0.01485 (~$23.72) — long from $1,549.66
- **LINK**: 0.3222 (~$2.43) — long from $7.19
- **Native ETH**: ~0.001178 (~$1.88) — gas only

### Active Positions
| Pair | Side | Entry | Current | SL | TP1 | Qty |
|------|------|-------|---------|-----|-----|-----|
| ETH/USD | Long | 1,549.66 | ~1,583 | 1,429.56 | 1,704.63 | 0.0149 |
| LINK/USD | Long | 7.19 | ~7.35 | 6.9756 | 7.909 | 0.322 |

---

## Session Work (2026-06-05)

### Critical Bugs Fixed
1. **Close trade safety** — `close_position()` removed position from map BEFORE swap verified. Tokens stranded on failure. Now stays until on-chain verified.
2. **Equity calculation** — Used `balance + unrealized_pnl` (only P&L). Now uses `balance + sum(position_values)` (includes deployed capital). All 4 callers fixed.
3. **Balance double-counting** — DB restore deducted `entry_price * qty` from balance. Removed.
4. **Wallet desync** — Chain-first reconciliation after candle data loads. `entry_price > 0` guard.
5. **0x `sellEntireBalance`** — Close trades use actual on-chain balance. Prevents dust failures.
6. **LiquidityCheck struct** — Replaces `bool` for `check_liquidity`. Honeypot detection via buy tax.
7. **Gas buffer** — 1.5x with 800K minimum. Fixes "out of gas" on Permit2.
8. **Silent rejections** — Every BUY rejection now logged to dashboard activity feed.

### Architecture Changes
- Multi-chain token DBs: Ethereum (19), Base (14), Optimism (14), Arbitrum (201)
- `savant serve` command: engine + API + dashboard single command
- Terminal WebSocket streaming real-time engine output
- SQLite busy_timeout (5s) via `SqliteConnectOptions`
- Dynamic gas, position sizing, R:R, circuit breaker (no hardcoded values)
- Full deploy mode: 100% capital at <$500 balance

### Dashboard Fixes
- Timer, cash balance, market insight, equity chart, scrollbar, 12h time, copy button
- Rejection logging to activity feed
- Live position/account sync every tick
- `unrealized_pnl` set correctly (was $0.00, now shows actual P&L)
- Timestamp utility `formatTime12h()` replaces broken inline parsing

### Version
- 0.9.0 → 0.9.1 (patch bump per user policy: 9 patches before minor)
- Updated: Cargo.toml, protocol.config.yaml, README.md, main.rs help text, CHANGELOG.md

---

## Known Issues / Next Steps

1. **Closed trades empty on dashboard** — DB has 7 trades, journal loads them, but dashboard shows 0. Likely a path resolution issue with `create_if_missing(true)` creating a new empty DB. Need to verify journal connects to correct `data/savant.db`.
2. **Equity curve flat** — Shows a flat line from old snapshots. Needs fresh data after restart.
3. **Dashboard design pass** — Performance, market insight, risk controls sections need production styling.
4. **Terminal copy button** — Missing. Activity log has one, terminal doesn't.
5. **Multi-chain routing** — Token DBs exist for 4 chains but `resolve_pair()` still hardcodes Arbitrum. Need dynamic chain selection.
6. **ETH gas rebalance** — User has ~$6 in ETH on Arbitrum (way too much for $0.01 swaps). Engine should auto-convert excess ETH → USDC.
7. **Solana support** — 0x doesn't support Solana. Would need Jupiter/Raydium integration.
8. **Dependabot vulnerability** — 1 moderate in JS dependency. Check GitHub security tab.

---

## File Map (changed this session)

### Rust Backend
| File | Changes |
|------|---------|
| `src/engine.rs` | Equity fix, wallet sync, insight seed, rejection logging, shared state sync |
| `src/execution/dex/trader.rs` | Close safety, gas buffer, LiquidityCheck, sellEntireBalance, on-chain balance query |
| `src/execution/dex/mod.rs` | Token DBs (4 chains), LiquidityCheck struct, trait update |
| `src/execution/dex/zero_x.rs` | Rich check_liquidity, sellEntireBalance param |
| `src/execution/dex/inch.rs` | check_liquidity return type |
| `src/execution/engine.rs` | check_liquidity default, sync_wallet_positions default |
| `src/execution/paper.rs` | Equity calculation fix |
| `src/risk/position.rs` | Full deploy, dynamic R:R, 100% position sizing |
| `src/risk/circuit_breaker.rs` | Dollar floors |
| `src/core/config.rs` | New risk fields, full_deploy |
| `src/core/shared.rs` | equity_curve field |
| `src/core/console.rs` | Log broadcast channel |
| `src/api/mod.rs` | Equity endpoint, timer fix |
| `src/monitor/journal.rs` | SQLite busy_timeout, get_snapshots |
| `src/main.rs` | serve command, version bump, log broadcast init |

### Frontend
| File | Changes |
|------|---------|
| `dashboard/next.config.ts` | API proxy rewrite |
| `dashboard/src/app/page.tsx` | EquityChart, 12h time, copy button, scrollbar, 10 decisions |
| `dashboard/src/app/globals.css` | Scrollbar styling |
| `dashboard/src/components/EquityChart.tsx` | NEW — recharts area chart |
| `dashboard/src/hooks/useDashboard.ts` | equity fetch |
| `dashboard/src/lib/api.ts` | getEquity, EquitySnapshot type |
| `dashboard/src/lib/time.ts` | NEW — formatTime12h, formatTimeShort, formatTime24h |

### Config & Docs
| File | Changes |
|------|---------|
| `config/default.toml` | 10 pairs, full_deploy, risk tiers, 4 chains enabled |
| `CHANGELOG.md` | v0.9.1 entry |
| `README.md` | Updated pairs, config, CLI, sandbox docs, Savant Protocol link, banner image |
| `protocol.config.yaml` | Version 0.9.1 |
| `dev/LEARNINGS.md` | Session learnings |
| `dev/session-summaries/2026-06-05.md` | Full session summary |
| `dev/fids/FID-2026-0605-052-arbitrum-trap.md` | NEW — Arbitrum trap FID |

---

## CLI Reference

```bash
# Production start (engine + API + dashboard)
cargo run --release serve

# Engine + API only
cargo run --release

# Dry run (one cycle, no execution)
cargo run -- --dry-run

# Sandbox testing
cargo run --release -- --test --sandbox

# Model comparison
.\run-model-tests.ps1

# Build
cargo build --release
cargo test
cargo clippy -- -D warnings
```

---

## Architecture Summary

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  Kraken WS   │────▶│  Engine Loop  │────▶│  0x API     │
│  (candles)   │     │  (5min cycle) │     │  (Arbitrum) │
└─────────────┘     └──────┬───────┘     └──────┬──────┘
                           │                     │
                    ┌──────▼───────┐     ┌──────▼──────┐
                    │ LLM Agent    │     │ On-chain    │
                    │ (OpenRouter) │     │ (Permit2)   │
                    └──────┬───────┘     └──────┬──────┘
                           │                     │
                    ┌──────▼───────┐     ┌──────▼──────┐
                    │ Shared State │     │ Dashboard   │
                    │ (API :8080)  │────▶│ (Next.js    │
                    │              │     │  :3000)     │
                    └──────────────┘     └─────────────┘
```

---

## Wallet

- **Address**: 0x543CA0434B84aD38c858D2D178D2082521711fBC
- **Chain**: Arbitrum (42161)
- **Backend**: 0x API v2 with Permit2 signing
- **Key**: WALLET_PRIVATE_KEY in .env (never committed)

## Environment

- **OS**: Windows (PowerShell)
- **Rust**: 1.91+ (edition 2021)
- **Node**: Next.js 16.2.7
- **Model**: xiaomi/mimo-v2.5-pro via OpenRouter
- **Gas**: ~$0.01/swap on Arbitrum
