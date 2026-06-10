# MASTER FID — Savant Trading v0.12.7 Development Plan

**ID:** MASTER-FID-2026-0609
**Created:** 2026-06-09 19:00
**Updated:** 2026-06-09 20:00
**Status:** active (FID-095: Terminal advanced features remaining)
**Scope:** All FIDs consolidated, validated against v0.12.7 codebase

---

## FID Registry

| FID | Title | Status | Implemented? |
|-----|-------|--------|-------------|
| FID-089 | Engine trigger stale price + balance query zero | merged-into-master | ❌ No (dup of 091) |
| FID-090 | Codebase file limit violation (300-line) | analyzed | ❌ No |
| FID-091 | Balance query zero + missing pair eval + age reset | ✅ fixed | Yes (v0.12.7) |
| FID-093 | Dashboard terminal → tabbed command bridge | ✅ fixed | Yes (v0.12.9) |
| FID-095 | Terminal advanced features (deferred) | deferred | ❌ No (depends on 093) |
| FID-097 | Circuit breaker + position resurrection | ✅ fixed | Yes (v0.12.4) |
| FID-098 | Episodic memory feedback loop | ✅ fixed | Yes (v0.12.5) |
| FID-100 | Parser bugs + token discovery wiring | ✅ fixed | Yes (v0.12.7) |
| FID-101 | R:R auto-adjust + bear market filter | ✅ fixed | Yes (v0.12.7) |
| FID-102 | Gemini Priority 1 (ATR TP, BB squeeze, dynamic ADX) | ✅ fixed | Yes (v0.12.7) |
| FID-103 | DEX price authoritative (salvaged) | ✅ fixed | Yes (v0.12.6) |
| FID-104 | On-chain execution failures | ✅ fixed | Yes (v0.12.7) |
| FID-105 | 0x API swap direction reversal | ✅ fixed | Yes (v0.12.8) |

---

## PRIORITY 0 — Critical (Blocks Agent Operation)

### P0-1: FID-091 — Balance Query Zero + Missing Pair Eval + Position Age Reset

**Validated against v0.12.6 codebase:**

1. **`query_token_balance` returns 0** — CONFIRMED. `trader.rs:1167-1176` has fallback to `close_qty` but no startup balance cache. If RPC returns `Some(0.0)` (valid zero), the close proceeds with 0 qty.

2. **Only 1/2 pairs evaluated in batch** — CONFIRMED. Batch dedup exists at `engine.rs:2031-2054` but doesn't warn when LLM returns fewer unique pairs than requested.

3. **Position `opened_at` resets on restart** — CONFIRMED. Wallet recovery at `engine.rs:1068` sets `opened_at: chrono::Utc::now()`. No epoch-0 sentinel.

**Fixes needed:**
- P0-1a: Add startup balance cache in DexTrader, fallback when query returns 0
- P0-1b: Add batch pair count validation with warning for missing pairs
- P0-1c: Use epoch-0 sentinel for wallet recovery opened_at

---

### P0-2: FID-093 — Dashboard Terminal Tabbed Command Bridge

**Validated against v0.12.6 codebase:**
- `TerminalPanel.tsx` — read-only log viewer ✅
- `api/mod.rs:978` — WS handler discards all input except "savant status" ❌
- No tabbed interface ❌
- No command protocol, NLP, undo, autonomy control ❌
- `api.ts:32-48` — Position type updated with `dex_price` ✅ (v0.12.6)

**This is the #1 user-requested feature. Fully designed (461 lines), not implemented.**

**Implementation phases:**
- Phase 1: Backend command channel (3-4 sessions) — `commands.rs`, `command_handler.rs`, `SharedEngineData` additions
- Phase 2: Frontend tabbed terminal (2 sessions) — `TerminalContainer.tsx`, `LogTerminal.tsx`, `CommandTerminal.tsx`
- Phase 3: Polish (1 session) — keyboard shortcuts, connection status

**kilo-cli reference:** User reviewed kilo-cli (in research/sources/kilocode-main, not committed) as design inspiration. FID-093 is the specification.

---

## PRIORITY 1 — High (v0.12.7 Candidates)

### P1-1: FID-100 — Parser Bugs + Token Discovery

**Validated against v0.12.6 codebase:**

1. **`partial_extract` side default** — CONFIRMED. `decision_parser.rs:802` uses `side: Side::Long` hardcoded.

2. **`partial_extract` confidence default** — CONFIRMED. `decision_parser.rs:835` uses `unwrap_or(0.5)` which bypasses 0.40 floor.

3. **`extract_from_freeform` confidence** — CONFIRMED. `decision_parser.rs:484` uses `confidence: 0.5` at Pass/Hold early return.

4. **Token discovery wiring** — CONFIRMED. `src/data/token_discovery.rs` exists (171 lines, `discover_tokens()` function) but is never imported or called. No `use token_discovery` in any file.

**Fixes needed:**
- P1-1a: Extract side from JSON in `partial_extract` (line 802)
- P1-1b: Change `unwrap_or(0.5)` to `unwrap_or(0.0)` in `partial_extract` (line 835)
- P1-1c: Change `confidence: 0.5` to `0.0` in `extract_from_freeform` (line 484)
- P1-1d: Wire `discover_tokens()` into engine.rs startup (~30 lines)

---

### P1-2: FID-101 — R:R Auto-Adjust + Bear Market Filter

**Validated against v0.12.6 codebase:**

1. **R:R auto-adjust** — NOT IMPLEMENTED. R:R override exists at `engine.rs:2616-2627` (logs actual vs claimed) but no auto-extend TP logic. If actual R:R < min_rr, trade is rejected by sizer.

2. **Pre-scoring ADX threshold** — CONFIRMED at `engine.rs:1708`: `let trend_signal = adx > 25.0;`. FID-101 says lower to 20.0.

3. **Volume spike signal** — NOT in pre-scoring. Volume spike exists in strategy configs (`volume_spike_multiplier`) but not as a pre-scoring filter signal.

**Fixes needed:**
- P1-2a: Add TP auto-extend after R:R check (~15 lines in engine.rs BUY path)
- P1-2b: Lower ADX threshold from 25.0 to 20.0 (line 1708)
- P1-2c: Add volume spike as 4th pre-scoring signal (~5 lines)

---

### P1-3: FID-102 — Gemini Priority 1

**Validated against v0.12.6 codebase:**

1. **Engine-side TP2/TP3 from ATR** — NOT IMPLEMENTED. LLM picks all 3 TPs.

2. **Bollinger Band Squeeze** — NOT IMPLEMENTED. No BB/Keltner computation in pre-scoring.

3. **Dynamic ADX scaling** — NOT IMPLEMENTED. Static threshold at 25.0.

**Fixes needed:**
- P1-3a: Compute TP2/TP3 from ATR in engine BUY path (~15 lines)
- P1-3b: Add BB squeeze detection in pre-scoring (~25 lines)
- P1-3c: Add dynamic ADX threshold based on Fear & Greed (~5 lines)

---

### P1-4: FID-103 — DEX Price Authoritative (Remaining)

**Completed in v0.12.6:**
- ✅ `dex_price: Option<f64>` in FullContext (all 5 construction sites)
- ✅ `dex_prices` in SharedEngineData
- ✅ DEX price in positions API
- ✅ `buy_token_price_usd` parsing from 0x response
- ✅ Dashboard spread indicator + DEX price row
- ✅ TradeRecord PnL uses DEX execution price
- ✅ Balance query fallback warning
- ✅ Dashboard TypeScript Position type updated

**Remaining:**
- P1-4a: Live DEX price population (call 0x `/price` per cycle, wire into `FullContext::dex_price`)
- P1-4b: Pre-trade spread check (reject if DEX/Kraken spread > 2%)
- P1-4c: Balance sync frequency (every 3 ticks → every tick)

---

## PRIORITY 2 — Medium (Technical Debt)

### P2-1: FID-090 — Codebase File Limit Violation

**Current state (v0.12.6):**

| File | Lines | Limit | Over By |
|------|-------|-------|---------|
| engine.rs | 6,972 | 300 | 23x |
| trader.rs | 1,911 | 300 | 6x |
| scenarios.rs | ~2,400 | 300 | 8x |
| zero_x.rs | 1,172 | 300 | 4x |
| decision_parser.rs | 1,062 | 300 | 3.5x |
| api/mod.rs | 1,047 | 300 | 3.5x |
| portfolio.rs | ~906 | 300 | 3x |
| dex/mod.rs | 967 | 300 | 3x |

**Proposed split:** engine.rs → 12 modules (~2,950 lines total). Pure structural refactor — no behavior changes. One module per session.

---

### P2-2: FID-095 — Terminal Advanced Features (Deferred)

Depends on FID-093 completion. Includes: command confirmation, aliases, templates, analytics, auto-complete, webhooks, scheduling. 6 sessions after FID-093.

---

## PRIORITY 3 — Low

- FID-095 advanced terminal features (depends on FID-093)
- Multi-chain RPC expansion (needs.md documents full plan)
- Expanded Arbitrum token list (500+ from CoinGecko)

---

## Duplicate FID Cleanup

| Action | FIDs | Reason |
|--------|------|--------|
| Merge | FID-089 + FID-091 | Same bugs. FID-091 is superset — keep 091, close 089 |
| Close | FID-089 | Duplicate — marked merged-into-master |

---

## Perfection Loop

### RED
- 11 FIDs with overlapping content, duplicate IDs, no clear priority
- FID-089 and FID-091 are duplicates
- FID-093 (tabbed terminal) never implemented despite being #1 user request
- FID-090 (file limits) makes codebase unmaintainable — engine.rs at 23x limit
- FID-100 references `decision_parser.rs:802` for side default — actual line is 802 ✅
- FID-100 references `token_discovery.rs` — exists at `src/data/token_discovery.rs` but never called ✅
- FID-101 references ADX threshold at `engine.rs:1708` — confirmed `adx > 25.0` ✅
- `token_discovery.rs` exists but was never wired — FID-100 Fix 2 is valid

### GREEN
- Consolidated all 11 FIDs into this Master FID
- Validated each finding against v0.12.6 codebase with actual line numbers
- Organized into P0 (critical), P1 (high), P2 (medium), P3 (low)
- Identified duplicate FIDs for merge/closure
- Preserved all planned work — nothing dropped
- Updated FID-089 status to merged-into-master

### AUDIT
- All FID line numbers verified against actual code
- All file references confirmed to exist (or confirmed missing)
- No planned work was removed or lost
- FID-093 design is comprehensive (461 lines) — ready for implementation
- FID-090 decomposition plan is sound — pure structural refactor
- FID-100/101/102 fixes are small and well-scoped

### COMPLETE
- Master FID created at `dev/fids/MASTER-FID-2026-0609.md`
- FID-089 marked as merged-into-master
- Ready for user review and approval
