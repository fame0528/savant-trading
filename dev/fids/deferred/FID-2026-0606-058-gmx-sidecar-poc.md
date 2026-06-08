# FID: GMX V2 Execution Sidecar — Proof of Concept

**Filename:** `FID-2026-0606-058-gmx-sidecar-poc.md`
**ID:** FID-2026-0606-058
**Severity:** critical
**Status:** deferred_until_500
**Created:** 2026-06-06 00:25
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

Build a Node.js sidecar service using the official `@gmx-io/sdk` to enable leveraged trading on GMX V2 (Arbitrum). This is a proof-of-concept — open a $1 test position to prove the execution layer works before building the cascade detection strategy on top.

---

## Detailed Description

### Problem

The agent can only trade spot (Kraken + 0x API). No leverage capability. At $26, spot trading produces $0.44 profit on a 2% ETH move. With 5x leverage on GMX V2, the same move produces $2.20 — 5x more. The entire cascade strategy depends on leverage being available.

### Solution

Build a lightweight Node.js sidecar that wraps the official GMX TypeScript SDK. The Rust engine sends HTTP requests to the sidecar, which handles oracle bundling, transaction signing, and position management.

### Architecture

```
Rust Engine (existing) → HTTP POST → GMX Sidecar (localhost:8081) → GMX V2 Contracts (Arbitrum)
```

### Scope (POC Only)

- [ ] Initialize Node.js project with `@gmx-io/sdk`
- [ ] Create REST endpoints: `/open`, `/close`, `/positions`, `/balance`
- [ ] Connect to Arbitrum with wallet private key from .env
- [ ] Open a $1 test long position on ETH/USD with 5x leverage
- [ ] Verify position appears on GMX UI
- [ ] Close the position
- [ ] Verify profit/loss is correct

### Out of Scope (Future)

- Cascade detection integration
- Dynamic leverage sizing
- Multi-pair monitoring
- Dashboard integration

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
