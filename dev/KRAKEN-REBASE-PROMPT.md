# Kraken Execution Rebase Prompt

> **For:** The other dev's AI agent
> **Goal:** Rebase Kraken execution changes onto current `main` without breaking the live DEX path
> **Branch:** `feat/kraken-execution-v2` (NOT `main`)

---

## MANDATORY: ECHO Protocol Boot Sequence

**Before doing ANYTHING, you MUST complete this boot sequence. Do not skip any step.**

### Step 0a: Read ECHO Protocol
```bash
cat ECHO.md
```
Read the ENTIRE file. This is the SINGLE bootstrap file for any AI agent session. It contains 15 laws, the Perfection Loop FSM, anti-patterns, and session lifecycle rules. All 15 laws are NON-NEGOTIABLE.

### Step 0b: Confirm Compliance
After reading ECHO.md, you MUST confirm:
- [ ] All 15 laws read and understood
- [ ] `strict_mode: true` — all Extended laws (5-15) are enforced
- [ ] Perfection Loop FSM understood (RED → GREEN → AUDIT → SELF-CORRECT → COMPLETE)
- [ ] Circuit breaker rules understood (10% char change max, 500-char verification sample, convergence detection)

### Step 0c: Load Protocol Config
```bash
cat protocol.config.yaml
```
Get project-specific commands (build, test, lint, type_check).

### Step 0d: Review LEARNINGS.md
```bash
cat dev/LEARNINGS.md
```
Review known issues and patterns from previous sessions.

### Step 0e: Review Open FIDs
```bash
ls dev/fids/
```
Flag any non-`Closed` FIDs as open items for this session.

### Step 0f: Create Session Summary
Create `dev/session-summaries/YYYY-MM-DD-HHMM.md` with:
- Initial state assessment
- Planned work (rebase Kraken changes onto main)
- Dependencies identified (DEX path must not break)

---

## Context

The `main` branch has been updated with **live DEX (Arbitrum/0x) execution changes** that are currently running in production with real money ($35 USDC on Arbitrum). Your local changes are for the **Kraken (CEX) execution path**. Both paths coexist through the `ExecutionEngine` trait.

**Current `main` HEAD:** `e336bd9` (50% gas buffer + 60s swap timeout + debug logging)

---

## Step 1: Create Branch and Rebase

```bash
git fetch origin
git checkout -b feat/kraken-execution-v2 origin/main
git rebase origin/main
```

If you have local uncommitted changes, stash them first:
```bash
git stash
git checkout -b feat/kraken-execution-v2 origin/main
git stash pop
git rebase origin/main
```

---

## Step 2: File Map — What You Can Touch vs. What You Must Not Break

### SAFE TO MODIFY (Kraken-specific)
These files are exclusively yours. Modify freely:
- `src/execution/kraken.rs` — KrakenTrader implementation
- Any new Kraken-specific modules you create

### SHARED — CAREFUL MERGING REQUIRED
These files contain BOTH DEX and Kraken logic. Keep both paths intact:

| File | What's Shared | DEX Lines (DO NOT REMOVE) |
|------|---------------|---------------------------|
| `src/engine.rs` | Main decision loop, execution dispatch | Lines 319-331 (phantom position reconciliation), 1047-1310 (Phase 3 execution with 60s timeouts) |
| `src/agent/decision_parser.rs` | `TradeAction` enum, parsing logic | `#[serde(rename_all = "PascalCase")]` on line 9, existing `Hold` alias on line 13 |
| `src/risk/circuit_breaker.rs` | Circuit breaker checks | Used by both paths at line 1049 of engine.rs |
| `src/risk/position.rs` | Position sizer | Used by both paths at line 1158 of engine.rs |
| `src/risk/correlation.rs` | Correlation matrix | Used by circuit breaker |
| `src/risk/stop_loss.rs` | Stop loss calculations | Used by PaperTrader |
| `src/execution/paper.rs` | PaperTrader (position tracker for both paths) | Both paths read/write positions through PaperTrader |
| `src/execution/engine.rs` | `ExecutionEngine` trait definition | 46 lines, DO NOT change the trait signature |
| `src/execution/mod.rs` | Module declarations | Must export both `dex` and `kraken` |
| `config/default.toml` | Configuration | DEX settings at lines 8-14, Kraken settings at lines 1-6 |

### DO NOT TOUCH (DEX-specific)
These files are exclusively DEX. Do not modify:
- `src/execution/dex/mod.rs` — DexBackend trait, token database, resolve_pair()
- `src/execution/dex/trader.rs` — DexTrader implementation
- `src/execution/dex/zero_x.rs` — 0x API v2 permit2 backend
- `src/execution/dex/inch.rs` — 1inch backend

### OTHER FILES (review before modifying)
- `src/api/mod.rs` — REST API server (reads from SharedEngineData, both paths)
- `src/data/kraken.rs` — KrakenClient (shared data fetcher, used by both paths)
- `src/main.rs` — Entry point, tracing subscriber (uses `stderr` not stdout)
- `src/tui/` — TUI dashboard (separate from your new HTML dashboard)

---

## Step 3: Architecture — How Both Paths Coexist

### The `Box<dyn ExecutionEngine>` Pattern

```
src/execution/engine.rs (46 lines)
  └── trait ExecutionEngine: Send + Sync
        ├── place_order(&mut self, pair, side, quantity, price) -> Result<Order>
        ├── close_position(&mut self, position_id) -> Result<Order>
        ├── open_positions(&self) -> Vec<&Position>
        ├── balance(&self) -> f64
        ├── kill(&mut self) -> Result<()>           // default: no-op
        ├── sync_balance(&mut self) -> Result<()>   // default: no-op
        └── place_stop_loss(&mut self, pos_id) -> Result<()>  // default: no-op
```

Three implementations:
1. **`KrakenTrader`** (`src/execution/kraken.rs`) — your changes
2. **`DexTrader`** (`src/execution/dex/trader.rs`) — live DEX path
3. **`PaperTrader`** (`src/execution/paper.rs`) — position tracker, used by both

The engine creates ONE executor at startup based on `config.exchange.backend`:
- `"kraken"` → `KrakenTrader`
- `"0x"` → `DexTrader<ZeroXBackend>`
- `"1inch"` → `DexTrader<InchBackend>`

All execution goes through `executor: Option<Box<dyn ExecutionEngine>>`. The `PaperTrader` is ALWAYS created and used as a position tracker for reporting, circuit breaker checks, and state persistence — even in live mode.

### `create_executor()` function (engine.rs:69-170)

This is the factory function that creates the executor. Your Kraken changes go in the `"kraken"` match arm (lines 78-107). The `"0x"` and `"1inch"` arms (lines 108-170) are DEX — do not modify.

---

## Step 4: Critical engine.rs Conflict Zones

### Zone 1: Phantom Position Reconciliation (lines 319-331)
```rust
// Reconcile: if the executor has no positions (e.g., phantom positions were
// cleared during DexTrader init), clear PaperTrader positions too.
if let Some(ref ex) = executor {
    if ex.open_positions().is_empty() && !paper.positions().is_empty() {
        warn!("PHANTOM POSITIONS: executor has 0 positions but PaperTrader has {}. Clearing PaperTrader.",
            paper.positions().len());
        paper.positions_mut().clear();
        paper.account_mut().open_positions = 0;
        paper.account_mut().unrealized_pnl = 0.0;
    }
}
```
**Keep this.** It fixes a real bug where PaperTrader had phantom positions that the executor didn't. Your Kraken changes may have a similar fix — merge both approaches.

### Zone 2: Phase 3 Execution Block (lines 1047-1310)
This is the main conflict area. The structure is:

```
[PHASE3] Checking execution for {pair} (action={action})
  ├── Circuit breaker check (line 1049)
  │     └── Triggered → log + write savant.blocked
  └── CircuitBreakerResult::Ok
        ├── TradeAction::Sell | TradeAction::Close (line 1062)
        │     ├── Find positions to close (line 1065)
        │     └── Close each position with 60s timeout (line 1086)
        │           └── executor.close_position(pos_id) OR paper.close_position(pos_id)
        ├── TradeAction::Buy (line 1155)
        │     ├── Position sizer calculates (line 1158)
        │     ├── Duplicate guard (line 1175)
        │     └── Place order with 60s timeout (line 1192)
        │           └── executor.place_order() OR paper.place_order()
        ├── TradeAction::Hold → unreachable!() (line 1296)
        └── TradeAction::AdjustStop (line 1297)
              └── Log "not yet implemented" (currently)
```

**Critical DEX patterns you must preserve:**
- `tokio::time::timeout(60s)` around `executor.place_order()` and `executor.close_position()` — prevents indefinite hangs
- `eprintln!("[PHASE3] ...")` logging at every decision point — NOT `tracing::info!()`
- Position sizer `None` logging at line 1305 — shows why sizing failed
- The `paper` object is always used for circuit breaker checks and position tracking, even in live mode

### Zone 3: Balance Sync (line 1496)
```rust
// Sync balance from Kraken for live mode and propagate to PaperTrader
```
If you add Kraken balance sync, it must also update `paper.account_mut().balance` so circuit breaker checks use the real balance.

---

## Step 5: The `eprintln!` vs `tracing` Issue

**All Phase 3 debug logging uses `eprintln!()`, NOT `tracing::info!()`.**

Reason: `tracing::info!()` deadlocks with the API server's `RwLock<SharedEngineData>`. The API server (axum, port 8080) reads from SharedEngineData using `tracing`, and the engine writes to it using `tracing` — they share the same subscriber, causing a deadlock.

`eprintln!()` writes directly to stderr, bypassing the tracing subscriber.

**Rule:** If you add logging in the Phase 3 execution path (lines 1047-1310), use `eprintln!()`. Use `tracing` (info/warn/error) everywhere else.

---

## Step 6: Decision Parser — Casing Tolerance

Current `TradeAction` enum (decision_parser.rs:8-17):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TradeAction {
    Buy,
    Sell,
    #[serde(alias = "HOLD", alias = "hold")]
    Hold,
    Close,
    AdjustStop,
}
```

If you add casing-tolerant parsing (e.g., `BUY` → `Buy`), use `#[serde(alias = "BUY")]` on each variant. Do NOT change `rename_all` — it's used by the AI prompt to generate PascalCase JSON.

---

## Step 7: Dashboard

The existing API server runs at `localhost:8080` with 16 REST endpoints (src/api/mod.rs). It reads from `SharedEngineData` which both paths populate.

If your new `dashboard.html` reads from these endpoints, it'll work for both paths. If you add new endpoints, add them to `src/api/mod.rs`.

The `src/tui/` directory contains a ratatui TUI (480 lines, 10 tabs). It's a separate interface, active via `--tui` flag. Don't conflict with it.

---

## Step 8: Configuration

Current `config/default.toml`:
- `backend = "0x"` — currently set to DEX mode
- `paper_trading = false` — live mode
- 8 pairs, 3 max positions, 5% daily loss, 10% drawdown
- `model = "xiaomi/mimo-v2.5-pro"` — DO NOT CHANGE

Your Kraken changes should work when `backend = "kraken"`. Don't change the default — the user switches via config or env var.

---

## Step 9: Verification Checklist

After rebasing, run ALL of these:

```bash
# 1. Compile
cargo build --release
# Expected: zero errors

# 2. Run all tests
cargo test
# Expected: 187+ tests pass, 0 failures

# 3. Lint
cargo clippy
# Expected: zero warnings (except the known dead_code warning on zero_x.rs:212)

# 4. Verify DEX path still works (if you have the env vars)
# Set backend = "0x" in config, run engine, verify it starts and fetches candles

# 5. Verify Kraken path works
# Set backend = "kraken" in config, run engine, verify it starts
```

---

## Step 10: Push and PR

```bash
git push -u origin feat/kraken-execution-v2
```

Create a PR targeting `main` with:
1. Summary of all changes
2. Which files were modified
3. Confirmation that `cargo test` passes
4. Confirmation that `cargo clippy` is clean

---

## Step 11: Perfection Loop (MANDATORY)

**Every change must go through the Perfection Loop. No exceptions.**

For each file you modify:

### RED Phase — Identify ALL failures
- Read the file 0-EOF before any edit (Law 1)
- Identify all issues: compilation errors, test failures, logic bugs, missing error handling
- Catalog every issue before fixing any

### GREEN Phase — Fix with MINIMAL changes
- Fix each issue with the smallest possible change
- Do not refactor unrelated code
- Do not add features not in scope

### AUDIT Phase — Double verification
Every change must be verified by TWO independent methods:
1. **Static analysis:** `cargo clippy` — zero warnings
2. **Runtime tests:** `cargo test` — all 187+ tests pass

Self-reporting is prohibited. The tool output IS the evidence.

### SELF-CORRECT Phase — Fix audit findings
If audit fails, fix and re-verify. Do not proceed until both checks pass.

### COMPLETE Phase — Document
- Update session summary with what was changed
- Create FIDs for any new issues discovered
- Note any blockers or open questions

---

## Step 12: FID Creation (MANDATORY)

For every bug, architectural issue, or improvement you discover:

1. Create `dev/fids/FID-YYYY-MMDD-NNN-short-name.md`
2. Use the FID template from `templates/FID-TEMPLATE.md`
3. Include: severity, description, root cause, proposed fix, verification
4. When fixed, move to `dev/fids/archive/` and update `CHANGELOG.md`

**Do NOT skip FID creation because "it's not what we're working on."** (Law: "If you encounter ANY issue — even outside the current scope — you must flag it immediately.")

---

## Step 13: Session Summary (REQUIRED)

At the end of your session, create/update `dev/session-summaries/YYYY-MM-DD-HHMM.md` with:
- What was done
- What was verified (with tool output evidence)
- What was NOT done (and why)
- Open questions or blockers
- Any FIDs created

---

## Known Issues on Current `main`

These are already fixed — don't re-introduce them:

| Issue | Fix | Commit |
|-------|-----|--------|
| Phantom positions in PaperTrader | Auto-reconcile on startup | `c11812b` |
| Swap hangs indefinitely | 60s tokio::time::timeout | `bdff326` |
| Gas price too low (baseFee spike) | 50% buffer on maxFeePerGas | `e336bd9` |
| tracing deadlocks with API server | eprintln! for Phase 3 logging | `b677f2c` |
| SQLite WAL-mode hangs | 2s timeout on all SQLite calls | `8e43cde` |
| 0x API v2 endpoint wrong | Migrated to permit2/quote endpoint | `b5ad337` |
| HTTP clients hang forever | 15-30s timeouts on all HTTP clients | `1931384` |
| PaperTrader balance drift | sync_balance() at startup | `a32a75b` |

---

## Summary of Rules

1. **Do NOT push to `main`.** Use `feat/kraken-execution-v2`.
2. **Keep both execution paths intact.** Kraken AND DEX must work.
3. **Preserve 60s timeouts** on `place_order()` and `close_position()`.
4. **Use `eprintln!()` in Phase 3**, not `tracing`.
5. **PaperTrader is shared.** Both paths read/write positions through it.
6. **`model = "xiaomi/mimo-v2.5-pro"`** — do not change.
7. **187+ tests must pass.** Run `cargo test` before pushing.
8. **Zero clippy warnings.** Run `cargo clippy` before pushing.
9. **ECHO Protocol mandatory.** Read ECHO.md first. All 15 laws enforced.
10. **Perfection Loop on every change.** RED → GREEN → AUDIT → COMPLETE.
11. **FIDs for every issue discovered.** No exceptions.
12. **Session summary required.** Document what was done and verified.
