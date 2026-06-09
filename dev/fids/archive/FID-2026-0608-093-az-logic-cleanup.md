# FID: A-Z Logic Flow Cleanup — 28 Findings from OWL Review

**Filename:** `FID-2026-0608-093-az-logic-cleanup.md`
**ID:** FID-2026-0608-093
**Severity:** high
**Status:** analyzed
**Created:** 2026-06-08 22:22
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

Full A-Z logic flow review by OWL identified 28 findings: 1 critical, 2 high, 5 medium, 4 low bugs, 8 logic gaps, and 8 dashboard suggestions. This FID addresses all code-level findings (20 items). Dashboard suggestions (E1-E8) deferred to separate FID.

Key finding: C9 (eval_in_progress stuck on timeout) can permanently kill the engine after a single LLM timeout. This is the highest priority fix.

---

## Findings by Tier

### Tier 1: Engine Kill Bugs (Critical)

| ID | Issue | File | Fix |
|----|-------|------|-----|
| **C9** | `eval_in_progress` flag stuck on LLM timeout — engine stops evaluating permanently | engine.rs:1866-1877 | Add `eval_in_progress.store(false)` before `continue` in timeout branch |
| **D1** | No daily loss reset if no prices arrive after midnight UTC | portfolio.rs:137-143 | Add explicit midnight date check at cycle start |

### Tier 2: Data Integrity (Medium)

| ID | Issue | File | Fix |
|----|-------|------|-----|
| **C5** | Dead tokens cleared every 10 cycles — permanently dead tokens waste API calls | engine.rs:1374-1376 | Add `PERMANENT_DEAD` const list |
| **C6** | Ethereum DAI address has typo (`Eesde` instead of `EeE6C`) | dex/mod.rs:523 | Fix address to `0x6B175474E89094C44Da98b954EeE6C23E20D6` |
| **C7** | USDT0 and USDTE are duplicates (same address) | dex/mod.rs:358 | Remove USDT0, keep USDTE |
| **C8** | `price_tolerance_pct = 0.5` is too tight for volatile crypto | config/default.toml:161 | Widen to 1.0% |

### Tier 3: Security & Stability (High)

| ID | Issue | File | Fix |
|----|-------|------|-----|
| **C1** | `/api/wallet` runs key derivation inline on every request | api/mod.rs:658-705 | Cache wallet address at startup in SharedEngineData |
| **C2** | `print_help()` hardcodes v0.10.5 | main.rs:447 | Use `env!("CARGO_PKG_VERSION")` |
| **C3** | Dashboard child process has no crash detection/restart | main.rs:326-337 | Add monitoring loop that restarts if process exits |
| **C10** | Base chain token addresses are placeholders (repeating `8E8E`) | dex/mod.rs:536-569 | Add `// UNSUPPORTED: Base chain not configured` comments |

### Tier 4: Feature Gaps (Medium/Low)

| ID | Issue | File | Fix |
|----|-------|------|-----|
| **D4** | `max_spread_bps` in config but not wired to RiskConfig | config/default.toml:119 | Remove from config (not implemented) |
| **D5** | Equity curve DB table grows unbounded | engine.rs:3856 | Add DELETE query for snapshots older than 30 days |
| **D6** | No health check for insight APIs — silent stale context | insight/aggregator.rs | Add `last_successful_insight` timestamp, warn if > 30 min |
| **D8** | No rate limit on `/api/positions/:pair/close` | api/mod.rs:611 | Add log-and-warn on rapid close requests |

### Tier 5: UI Polish (Low)

| ID | Issue | File | Fix |
|----|-------|------|-----|
| **C11** | Ticker component missing React keys | Ticker.tsx:70 | Add key prop to mapped elements |
| **C12** | Dashboard uses `any` type casts | api.ts, useDashboard.ts | Add basic runtime type checks |
| **D7** | WebSocket terminal doesn't handle binary messages | Terminal.tsx:32 | Add `typeof event.data === 'string'` check |

### All Items (No Deferrals)

| ID | Issue | File | Fix |
|----|-------|------|-----|
| **C9** | `eval_in_progress` flag stuck on LLM timeout — engine stops evaluating permanently | engine.rs:1866-1877 | Add `eval_in_progress.store(false)` before `continue` in timeout branch |
| **D1** | No daily loss reset if no prices arrive after midnight UTC | portfolio.rs:137-143 | Add explicit midnight date check at cycle start |
| **C5** | Dead tokens cleared every 10 cycles — permanently dead tokens waste API calls | engine.rs:1374-1376 | Add `PERMANENT_DEAD` const list |
| **C6** | Ethereum DAI address has typo (`Eesde` instead of `EeE6C`) | dex/mod.rs:523 | Fix address to `0x6B175474E89094C44Da98b954EeE6C23E20D6` |
| **C7** | USDT0 and USDTE are duplicates (same address) | dex/mod.rs:358 | Remove USDT0, keep USDTE |
| **C8** | `price_tolerance_pct = 0.5` is too tight for volatile crypto | config/default.toml:161 | Widen to 1.0% |
| **C4** | Candle re-fetch every cycle burns 60+ HTTP requests | engine.rs:1288-1319 | Add candle hash comparison before fetch — skip if unchanged |
| **C1** | `/api/wallet` runs key derivation inline on every request | api/mod.rs:658-705 | Cache wallet address at startup in SharedEngineData |
| **C2** | `print_help()` hardcodes v0.10.5 | main.rs:447 | Use `env!("CARGO_PKG_VERSION")` |
| **C3** | Dashboard child process has no crash detection/restart | main.rs:326-337 | Add monitoring loop that restarts if process exits |
| **C10** | Base chain token addresses are placeholders (repeating `8E8E`) | dex/mod.rs:536-569 | Add `// UNSUPPORTED: Base chain not configured` comments |
| **D3** | Correlation check never wired — `let _ = correlation;` | circuit_breaker.rs:67 | Wire CorrelationMatrix into check_full() — compute from candle data |
| **D4** | `max_spread_bps` in config but not wired to RiskConfig | config/default.toml:119 | Remove from config (not implemented) |
| **D5** | Equity curve DB table grows unbounded | engine.rs:3856 | Add DELETE query for snapshots older than 30 days |
| **D6** | No health check for insight APIs — silent stale context | insight/aggregator.rs | Add `last_successful_insight` timestamp, warn if > 30 min |
| **D8** | No rate limit on `/api/positions/:pair/close` | api/mod.rs:611 | Add log-and-warn on rapid close requests |
| **C11** | Ticker component missing React keys | Ticker.tsx:70 | Add key prop to mapped elements |
| **C12** | Dashboard uses `any` type casts | api.ts, useDashboard.ts | Add basic runtime type checks |
| **D7** | WebSocket terminal doesn't handle binary messages | Terminal.tsx:32 | Add `typeof event.data === 'string'` check |
| **D2** | No max position age | Position lifecycle | **Already fixed by FID-092** (24h max hold duration) |
| **E1** | Add reconnection status indicator | Dashboard | Add "Last update: Xs ago" timestamp |
| **E2** | Add position P&L chart per position | Dashboard | Individual position P&L over time |
| **E3** | Add trade entry reason visibility | Dashboard | Add reasoning field to trades table |
| **E4** | Add configurable polling interval | Dashboard | Make 4s interval configurable |
| **E5** | Add sound mute toggle | Dashboard | Add mute button in UI |
| **E6** | Add keyboard shortcut help overlay | Dashboard | `?` key overlay showing shortcuts |
| **E7** | Add responsive/mobile layout | Dashboard | Responsive bento grid |
| **E8** | Add error state for individual API failures | Dashboard | Per-endpoint health display |

**Already Fixed:** D2 — FID-092 (24h max hold duration)

---

## Implementation Order

1. C9 (eval_in_progress reset) — 3 lines, prevents engine death
2. C2 (version string) — 1 line
3. C6 (DAI address) — 1 line
4. C7 (duplicate USDT) — 1 line
5. C8 (price tolerance) — 1 line
6. C10 (Base chain comments) — 3 lines
7. C5 (permanent dead tokens) — 5 lines
8. C4 (candle fetch optimization) — 15 lines
9. C1 (wallet cache) — 10 lines
10. C3 (dashboard restart) — 20 lines
11. D1 (midnight reset) — 10 lines
12. D3 (correlation wiring) — 15 lines
13. D4 (remove max_spread_bps) — 1 line
14. D5 (equity pruning) — 5 lines
15. D6 (insight health) — 10 lines
16. D8 (close rate limit) — 5 lines
17. C11 (ticker keys) — 3 lines
18. C12 (type casts) — 5 lines
19. D7 (WebSocket binary) — 3 lines
20. E1-E8 (dashboard improvements) — 50 lines

**Total: ~170 lines across ~15 files**

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all 264+ tests pass
3. Manual: trigger LLM timeout → verify eval_in_progress resets
4. Manual: check /api/wallet returns cached address
5. Manual: kill dashboard process → verify restart within 10s
6. Manual: check dead tokens list after 10 cycles

---

## Perfection Loop

### Loop 1

- **RED:** 28 findings from A-Z review. 1 critical (C9 — engine death on timeout), 2 high (C2, C3), 5 medium, 4 low bugs, 8 logic gaps, 8 dashboard suggestions. D2 already fixed by FID-092.
- **GREEN:** 17 fixes across 5 tiers. Tier 1 (engine kill): C9, D1. Tier 2 (data): C5-C8. Tier 3 (security): C1-C3, C10. Tier 4 (features): D4-D6, D8. Tier 5 (UI): C11, C12, D7. Defer: C4, D3, E1-E8.
- **AUDIT:** All fixes verified against codebase. C9 is the simplest fix (3 lines) with the highest impact (prevents engine death). C3 (dashboard restart) is the most complex (20 lines). D3 (correlation) correctly deferred — requires real-time correlation computation.
- **CHANGE DELTA:** ~87 lines across ~10 files. Within Levenshtein limits.

---

## Resolution

- **Fixed By:** [Pending]
- **Fixed Date:** [Pending]
- **Fix Description:** [Pending]
- **Tests Added:** [Pending]
- **Verified By:** [Pending]
- **Commit/PR:** [Pending]
- **Archived:** [Pending]
