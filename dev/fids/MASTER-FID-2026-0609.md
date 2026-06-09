# MASTER FID — Savant Trading v0.12.6 Baseline Audit

**ID:** MASTER-FID-2026-0609
**Created:** 2026-06-09 18:40
**Status:** active
**Scope:** All open FIDs consolidated into single prioritized backlog

---

## Source FIDs Consolidated

| FID | Title | Original Status | Validation |
|-----|-------|----------------|------------|
| FID-089 | Engine trigger stale price + balance query zero | analyzed | ⚠️ Partially fixed (FID-089 was committed as v0.11.8 but issues remain) |
| FID-090 | Codebase file limit violation (300-line) | analyzed | ❌ Not addressed — engine.rs still 6,967 lines |
| FID-091 | Balance query zero + missing pair eval (dup of 089) | analyzed | ⚠️ Duplicate — merge into FID-089 |
| FID-093 | Dashboard terminal → tabbed command bridge | analyzed | ❌ Not implemented — terminal still read-only |
| FID-095 | Terminal advanced features (deferred) | deferred | ❌ Depends on FID-093 |
| FID-100 | Parser bugs + token discovery wiring | created | ❌ Not implemented |
| FID-101 | R:R auto-adjust + bear market filter | created | ❌ Not implemented |
| FID-102 | Gemini Priority 1 (ATR TP, BB squeeze, dynamic ADX) | created | ❌ Not implemented |
| FID-103 | DEX price authoritative (salvaged) | analyzed | ⚠️ Structural plumbing done, live data wiring pending |

---

## PRIORITY 0 — Critical (Fix Before Next Session)

### P0-1: FID-089/091 Merge — Engine Trigger Stale Price + Balance Query Zero

**FIDs 089 and 091 are duplicates** — same root causes, different FIDs. Merge into one.

**Validated Issues:**

1. **Engine trigger uses stale `pos.current_price`** — CONFIRMED. Engine trigger at `engine.rs ~line 2212` uses `pos.current_price` which is set to `entry_price` at wallet recovery. The stale-price guard (skip if within 0.1% of entry) was added in v0.11.8 but the ATR sanity check and market_stores lookup were NOT implemented.

2. **`query_token_balance` returns 0 on-chain** — PARTIALLY FIXED. FID-087 Bug D fix returns `None` on parse failure, but `Some(0.0)` from valid RPC "0x0" response is still possible. The startup balance cache fallback (FID-091 Fix 1) was NOT implemented.

3. **Position `opened_at` resets on restart** — CONFIRMED. Wallet recovery at `engine.rs ~line 1068` sets `opened_at: chrono::Utc::now()`. The epoch-0 sentinel fix was NOT implemented.

4. **Batch pair diversity not validated** — CONFIRMED. Batch parser at `engine.rs ~line 2031` deduplicates by pair but doesn't warn when LLM returns fewer unique pairs than requested.

**Merged Fix Plan:**
- F089-A: Add market_stores price lookup in engine trigger (use actual candle close instead of pos.current_price)
- F089-B: Add startup balance cache in DexTrader, use as fallback when query_token_balance returns 0
- F089-C: Use epoch-0 sentinel for wallet recovery opened_at (dashboard shows "unknown")
- F089-D: Add batch pair count validation with warning for missing pairs

---

### P0-2: FID-093 — Dashboard Terminal Tabbed Command Bridge

**Status:** NOT IMPLEMENTED. This was the #1 user-requested feature.

**Current State:**
- `TerminalPanel.tsx` — renders xterm.js, captures input, sends via WS ✅
- `api/mod.rs:978` — WS handler discards all input except "savant status" ❌
- No tabbed interface — single "savant — terminal" panel ❌
- No command protocol, no NLP parsing, no undo, no autonomy control ❌

**What FID-093 Specifies:**
- Tabbed terminal: "Logs" tab (existing) + "Command" tab (new)
- 12 operator commands: override_close, override_stop, pause, resume, status, query, inject_context, approve, reject, undo, set_autonomy, halt
- NLP layer mapping natural language to structured commands
- Command confirmation for dangerous actions
- Command history (last 50, localStorage)
- Agent push messages (unsolicited reasoning to operator)
- `command_log` table in SQLite for audit trail
- `pending_commands` queue in SharedEngineData

**Implementation Phases (from FID-093):**
- Phase 1: Backend command channel (3-4 sessions) — P0
- Phase 2: Frontend tabbed terminal (2 sessions) — P0
- Phase 3: NLP + advanced features (FID-095, 1-2 sessions) — P1

**kilo-cli Reference:** User reviewed kilo-cli (in research/sources/kilocode-main, not committed) as reference for the command bridge design. The FID-093 design is the specification; kilo-cli was inspiration.

---

## PRIORITY 1 — High (Plan for v0.12.7)

### P1-1: FID-100 — Parser Bugs + Token Discovery

**Validated Issues:**
1. `partial_extract` side default — CONFIRMED. `decision_parser.rs ~line 842` uses `Side::Long` as default for broken JSON salvage.
2. `partial_extract` confidence default — CONFIRMED. Uses `unwrap_or(0.5)` which bypasses the 0.40 confidence floor.
3. `extract_from_freeform` confidence — CONFIRMED. Uses `confidence: 0.5` at Pass/Hold early return.
4. Token discovery wiring — `token_discovery.rs` exists but is never called from engine.rs.

### P1-2: FID-101 — R:R Auto-Adjust + Bear Market Filter

**Validated Issues:**
1. LLM proposes symmetric stops → R:R < min_rr → trade rejected. Auto-extend TP fix NOT implemented.
2. Pre-scoring ADX threshold at 25.0 is too high for bear markets. Volume spike signal NOT added.

### P1-3: FID-102 — Gemini Priority 1

**Validated Issues:**
1. TP2/TP3 computed by LLM as round numbers, not from ATR. Engine-side ATR computation NOT implemented.
2. Bollinger Band Squeeze pre-filter NOT implemented.
3. Dynamic ADX threshold scaling NOT implemented.

### P1-4: FID-103 — DEX Price Authoritative (Remaining Work)

**Completed (v0.12.6):**
- ✅ `dex_price: Option<f64>` in FullContext (all 5 sites)
- ✅ `dex_prices` in SharedEngineData
- ✅ DEX price in positions API
- ✅ `buy_token_price_usd` parsing from 0x response
- ✅ Dashboard spread indicator + DEX price row
- ✅ TradeRecord PnL uses DEX execution price
- ✅ Balance query fallback warning

**Remaining:**
- Live DEX price population in main evaluation loop (call 0x `/price` per cycle, wire into `FullContext::dex_price`)
- Pre-trade spread check (reject if DEX/Kraken spread > 2%)
- Balance sync frequency increase (every 3 ticks → every tick)

---

## PRIORITY 2 — Medium (Technical Debt)

### P2-1: FID-090 — Codebase File Limit Violation

**Current State (v0.12.6):**
| File | Lines | Limit | Over By |
|------|-------|-------|---------|
| engine.rs | 6,967 | 300 | 23x |
| trader.rs | 1,911 | 300 | 6x |
| scenarios.rs | ~2,400 | 300 | 8x |
| zero_x.rs | 1,172 | 300 | 4x |
| decision_parser.rs | ~1,040 | 300 | 3.5x |
| api/mod.rs | 1,047 | 300 | 3.5x |
| portfolio.rs | ~906 | 300 | 3x |
| mod.rs (dex) | 967 | 300 | 3x |

**Proposed Split (from FID-090):** engine.rs → 12 modules (~2,950 lines total). This is a pure structural refactor — no behavior changes. Should be done incrementally (one module per session).

### P2-2: FID-095 — Terminal Advanced Features (Deferred)

Depends on FID-093 completion. Includes: command confirmation, aliases, analytics, templates, scheduling.

---

## PRIORITY 3 — Low (Nice to Have)

- FID-095 advanced terminal features (depends on FID-093)
- Multi-chain RPC client expansion (needs.md documents the full plan)
- Expanded Arbitrum token list (500+ tokens from CoinGecko)

---

## Duplicate FID Cleanup

| Action | FIDs | Reason |
|--------|------|--------|
| Merge | FID-089 + FID-091 | Same bugs, same fixes. FID-091 is a superset — keep 091, close 089 |
| Close | FID-089 | Duplicate of FID-091 |
| Rename | FID-103 contents | Labeled as FID-102 by bad session — already renamed to FID-2026-0609-103 |

---

## Perfection Loop

### RED
- 11 FIDs in dev/fids/ with overlapping content, duplicate IDs, and no clear priority
- FID-089 and FID-091 are duplicates
- FID-093 (tabbed terminal) never implemented despite being the #1 user request
- FID-090 (file limits) makes codebase unmaintainable — engine.rs at 23x the limit
- Multiple FIDs reference code that has since been modified (stale line numbers)

### GREEN
- Consolidated all 11 FIDs into this Master FID
- Validated each finding against current v0.12.6 codebase
- Organized into P0 (critical), P1 (high), P2 (medium), P3 (low)
- Identified duplicate FIDs for merge/closure
- Preserved all planned work — nothing dropped

### AUDIT
- All FID findings cross-referenced with actual codebase
- Line numbers verified where possible
- No planned work was removed or lost
- FID-093 design is comprehensive (461 lines) — ready for implementation
- FID-090 decomposition plan is sound — pure structural refactor

### COMPLETE
- Master FID created at `dev/fids/MASTER-FID-2026-0609.md`
- Ready for user review and approval
