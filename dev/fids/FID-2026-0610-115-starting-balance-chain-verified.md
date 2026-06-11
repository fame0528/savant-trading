# FID-115: Starting Balance Hardcoded From Config Instead of Chain Data

**Filename:** `FID-2026-0610-115-starting-balance-chain-verified.md`
**ID:** FID-2026-0610-115
**Severity:** high
**Status:** fixed
**Created:** 2026-06-10 21:00
**Author:** Buffy (Codebuff AI)
**Type:** bug-fix
**Scope:** src/api/mod.rs, src/engine/mod.rs

---

## Summary

The dashboard profit calculation uses `starting_balance` from `config/default.toml` (hardcoded $30.00) instead of the first observed on-chain equity. This produces misleading profit numbers when the config value doesn't match reality.

## Detailed Description

### Problem

Dashboard shows "Profit: -$10.27" with "$30.00 invested → $19.73 on-chain". The $30.00 comes from config, not chain data. The actual on-chain starting equity may differ due to prior trades, deposits, or withdrawals.

### Expected Behavior

Starting balance should be the first observed on-chain equity (USDC + position values) at engine startup, stored persistently.

### Root Cause

`get_session` in `src/api/mod.rs` reads `config.trading.starting_balance` directly. No snapshot mechanism exists.

### Fix

On first startup, snapshot `on_chain_usdc + sum(position_values)` to `data/starting_balance.json`. Subsequent startups read from the snapshot. Config value becomes fallback only if no snapshot exists.

## Resolution

- **Fixed By:** Buffy (Codebuff AI)
- **Fixed Date:** 2026-06-10 21:00
- **Fix Description:** Added starting equity snapshot mechanism that persists to disk on first observation
- **Verified By:** clippy + tests
