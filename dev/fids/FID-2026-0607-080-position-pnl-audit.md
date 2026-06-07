# FID-080: Position PnL Shows $0.00 After Hours of Holding

**Status:** analyzed
**Severity:** high
**Created:** 2026-06-07
**Author:** Kilo

---

## Perfection Loop — RED Phase

### Issue: Position PnL stuck at $0.00 despite price movement

**Severity:** HIGH
**Location:** Full data chain: portfolio.rs → types.rs → engine.rs → api/mod.rs → page.tsx
**Evidence:** User held ETH and LINK for hours. Trailing stops show price movement (1631, 7.74). But position cards show PnL: $0.00. Top bar Profit shows -$0.40 (correct from equity - starting_balance).

### Data Chain Audit

```
Step 1: all_prices = market_stores + ws_ticker_prices
  engine.rs:3089-3095
  STATUS: ✓ Trailing stops use same source and work

Step 2: portfolio.update_prices(&all_prices)
  engine.rs:3096 → portfolio.rs:145-153
  CODE: pos.unrealized_pnl = (price - entry_price) * qty
  STATUS: ✓ Code is correct — recalculates from live prices

Step 3: shared.positions = portfolio.positions().values().cloned()
  engine.rs:3100-3101
  STATUS: ✓ Clones after update_prices

Step 4: shared.account = portfolio.account().clone()
  engine.rs:3104-3105 → types.rs:282 (fixed to recalculate)
  STATUS: ✓ Recalculates from entry vs current

Step 5: API /api/positions returns shared.positions
  api/mod.rs: get_positions()
  STATUS: ✓ Returns positions with unrealized_pnl field

Step 6: Dashboard reads p.unrealized_pnl
  page.tsx: position cards
  STATUS: ✓ Reads correct field
```

**Every step in the chain is correct.** The code SHOULD produce non-zero PnL.

### Possible Root Causes (need user confirmation)

| # | Suspect | How to verify |
|---|---------|---------------|
| 1 | Browser cache — old JS bundle | Hard refresh (Ctrl+Shift+R) |
| 2 | Dashboard poll timing — reads before update_prices runs | Check if PnL updates after ~4 seconds |
| 3 | Wallet recovery positions have different pair key than market_stores | Add debug log to update_prices |
| 4 | `update_prices()` not called in monitoring mode | Check engine logs for "Balance synced" messages |

### GREEN Phase — Proposed Fixes

| # | Fix | File | Risk |
|---|-----|------|------|
| 1 | Add debug log to `update_prices()` showing per-position PnL | portfolio.rs | None |
| 2 | Ensure `update_prices()` runs before first shared state sync | engine.rs | Low |
| 3 | Recalculate PnL in the API response (defense in depth) | api/mod.rs | Low |

### AUDIT — Five Questions

| # | Question | Answer |
|---|----------|--------|
| 1 | ALL cases? | Debug logging + API defense covers all paths |
| 2 | 1000 agents? | N/A — per-agent display |
| 3 | Hostile attacker? | N/A |
| 4 | 2 years? | Debug log can be removed once confirmed working |
| 5 | Standard? | Defense-in-depth PnL calculation in API is best practice |

### SELF-CORRECT

| Issue | Correction |
|-------|-----------|
| Don't add permanent debug logs to hot path | Use `tracing::debug!` not `info!` — only visible with RUST_LOG=debug |
| API defense-in-depth should not replace the primary path | Keep portfolio.rs as primary, API as fallback |

### COMPLETE

**Needs user confirmation:** Hard refresh the dashboard (Ctrl+Shift+R). If PnL still shows $0.00 after 4+ seconds, I'll add debug logging + API defense-in-depth.

---

## Status

- [x] RED: Full data chain audited (6 steps)
- [x] GREEN: 3 fixes proposed
- [x] AUDIT: Five Questions pass
- [x] SELF-CORRECT: 2 corrections applied
- [ ] COMPLETE: **AWAITING USER CONFIRMATION (hard refresh test)**
