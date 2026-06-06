# FID: PaperTrader Naming Mismatch — Real Money System Uses Paper Trading Terminology

**Filename:** `FID-2026-0605-054-paper-naming.md`
**ID:** FID-2026-0605-054
**Severity:** high
**Status:** created
**Created:** 2026-06-05 20:57
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

The system trades real money on-chain via 0x API, but the core struct is named `PaperTrader`, the config flag is `paper_trading`, the file is `paper.rs`, order IDs are prefixed `paper-`, and the CLI/TUI display "Paper Trading" as the mode. This is semantically wrong and will confuse every future contributor, AI agent, and code reviewer. "Paper trading" universally means simulated/fake money.

---

## Detailed Description

### Problem

The `PaperTrader` struct in `src/execution/paper.rs` is the **primary state manager** for all positions, balances, and trade records. In live mode, it tracks real on-chain positions with real money. But its name implies simulation.

Affected naming:
- `PaperTrader` struct (30 references across 4 files)
- `paper_trading` config field (12 references across 6 files)
- `paper` variable name (~94 references in engine.rs alone)
- `paper-` / `paper-close-` order ID prefixes (stored in DB)
- "Paper Trading" TUI label, CLI output, log messages
- `paper_state.json` (legacy, already removed)
- `"paper"` console command match arm

### Expected Behavior

Naming should reflect that this is the **portfolio state manager** — it tracks positions, balances, P&L, and stop/TP logic regardless of whether the backend is live or simulated.

### Root Cause

The codebase started as a paper trading prototype. When live execution was added via DEX/0x, the `PaperTrader` was repurposed as the state manager without renaming. The `paper_trading: false` config flag switches the execution backend but the naming stayed.

---

## Impact Assessment

### Affected Components

- `src/execution/paper.rs` — file rename + struct rename
- `src/execution/mod.rs` — module declaration
- `src/engine.rs` — ~114 references (struct, variable, config, comments)
- `src/core/config.rs` — `paper_trading` field
- `src/api/mod.rs` — mode string, JSON API field
- `src/tui/state.rs` — mode display label
- `src/main.rs` — mode check
- `src/core/console.rs` — command match arm
- `config/default.toml`, `config/canary.toml` — config key
- `run-engine.ps1`, `run-engine.bat`, `run-247.ps1`, `run-247.bat` — user-facing output

### Risk Level

- [x] Medium: Naming debt. No functional impact but significant cognitive load and onboarding friction.

---

## Proposed Solution

### Naming Candidates

| Candidate | Pro | Con |
|-----------|-----|-----|
| `PortfolioManager` | Accurate — manages portfolio state | Long variable name (`portfolio`) |
| `Portfolio` | Clean, short | Ambiguous — could be the data, not the manager |
| `StateManager` | Generic, accurate | Too vague — what state? |
| `PositionManager` | Focuses on positions | Doesn't capture balance/trade tracking |
| `TradingState` | Descriptive | Noun, not an actor |

**Recommended: `PortfolioManager`** with variable name `portfolio` (or `pm` for tight contexts).

### Rename Plan

1. **File rename:** `src/execution/paper.rs` → `src/execution/portfolio.rs`
2. **Module rename:** `pub mod paper;` → `pub mod portfolio;`
3. **Struct rename:** `PaperTrader` → `PortfolioManager`
4. **Variable rename:** `paper` → `portfolio` (~94 in engine.rs, ~7 in tests)
5. **Config rename:** `paper_trading` → `live_execution` (inverted semantics: `true` = real money, `false` = simulated)
6. **Order ID prefix:** `paper-` → `sim-` / `live-` depending on mode
7. **Display labels:** "Paper Trading" → "Simulated" / "Live" based on config
8. **Console command:** `"paper"` → `"portfolio"` or `"balance"`
9. **API field:** `"paper_trading"` → `"live_execution"` (breaking change for dashboard)
10. **Dashboard:** Update mode display from API response

### Breaking Changes

- Config key `paper_trading` → `live_execution` (TOML + API)
- Console command `paper` → `portfolio`
- Order ID prefix change (existing DB rows retain old prefix)

### Verification

```bash
cargo build
cargo test
cargo clippy -- -D warnings
# Verify: config loads, engine starts, dashboard shows correct mode
```

---

## Perfection Loop

### Loop 1

- **RED:** —
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
