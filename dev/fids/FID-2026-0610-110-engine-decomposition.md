# FID-2026-0610-110: Engine Monolith Decomposition — engine.rs (7,214 lines)

**ID:** FID-2026-0610-110
**Created:** 2026-06-10
**Updated:** 2026-06-10 (v3 — Sessions 1-4 complete, 5-7 deferred)
**Severity:** critical
**Status:** partially-complete (Sessions 1-4 done, 5-7 deferred to future session)
**Type:** Sub-FID (supersedes FID-090, architecture refactor)
**Scope:** src/engine.rs (7,214 lines) → src/engine/ directory with 5 modules
**Supersedes:** FID-090 in MASTER-FID-2026-0609 (was P2-1, analyzed since v0.12.6)

---

## Summary

`engine.rs` is **7,214 lines** — 24x over the `max_file_lines: 300` limit in `protocol.config.yaml`. The `run()` function alone is **4,490 lines** (lines 256–4746) with all state local and all logic inline. This is a direct violation of ECHO Protocol Law 13 (Utility-First) and Law 7 (Search Before Create — the size makes finding anything impossible).

This FID proposes a **9-session staged decomposition** using an `Engine` struct with `&mut self` methods. Each session is a pure mechanical extraction with zero behavior change, verified by `cargo clippy -D warnings && cargo test && cargo build --release`. The public API exposed to `main.rs` never changes — `engine::run(config, shared, running)` still works after every session.

---

## Completion Status (v3 — 2026-06-10)

### Sessions Completed

| Session | Task | Status | Evidence |
|---------|------|--------|----------|
| 1 | Extract utilities → `utils.rs` | ✅ Complete | 200 lines, 5 functions |
| 2 | Extract training & debug → `training.rs`, `debug.rs` | ✅ Complete | 1,846 + 407 lines, 13 functions |
| 3 | Create `EngineState` struct (48 fields) | ✅ Complete | Struct defined in mod.rs |
| 4 | Extract init phase → `EngineState::new()` | ✅ Complete | Init code moved to `new()`, `run()` is thin wrapper |

### Sessions Deferred (Future)

| Session | Task | Risk | Notes |
|---------|------|------|-------|
| 5 | Extract loop body → `EngineState::run_loop()` | Medium | Requires changing all variable refs to `self.field` (~3,350 lines) |
| 6 | Extract cycle sub-phases into methods | Low | Pure extract from loop body |
| 7 | Audit & Compliance | Low | Verify all files under 300 lines, final CI check |

### Current File Structure

```
src/engine/
├── mod.rs      — 4,581 lines (EngineState struct + new() + run() wrapper + loop)
├── training.rs — 1,846 lines (11 functions)
├── debug.rs    — 407 lines  (2 functions)
└── utils.rs    — 200 lines  (5 functions)
```

### Verification (2026-06-10)

- ✅ `cargo clippy -- -D warnings` — PASS (0 warnings)
- ✅ `cargo test` — PASS (273/273 tests)
- ✅ `cargo build --release` — PASS
- ✅ Engine tested in production — 3 cycles completed, all systems operational

### Why Deferred

The remaining Sessions 5-7 require moving 3,350 lines of loop body code and changing all local variable references to `self.field`. This is the highest-risk session in the plan — a single missed reference could introduce subtle bugs. The current state (struct + init extracted) is already a significant improvement over the 7,214-line monolith. Sessions 5-7 can be completed in a dedicated future session with fresh context.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust (cargo), tokio async
- **Tool Versions:** See `Cargo.toml` for rust edition and dependencies
- **Commit/State:** v0.13.1 (current stable), engine.rs at 7,214 lines. Decomposition ships as v0.13.1.
- **Protocol:** ECHO v0.1.0, strict_mode: true

---

## Detailed Description

### Problem

`engine.rs` is a single-file monolith containing:

| Component | Lines | % of File |
|-----------|-------|-----------|
| `run()` — main engine loop | 4,490 | 62% |
| `run_sandbox()` — sandbox entry | 607 | 8% |
| `run_training()` — training entry | 173 | 2% |
| `run_training_batch()` — training helper | 978 | 14% |
| `run_action_test()` — action test entry | 51 | <1% |
| `run_live_test()` — live test entry | 240 | 3% |
| `dry_run()` — dry run entry | 202 | 3% |
| 8 utility functions | 473 | 7% |

### Root Cause

Zero decomposition from day 1. Every new feature added inline to `run()` because there was no module structure to add to. 15 FIDs worth of fixes (FID-082 through FID-109) were patched directly into the monolithic function, making it worse each time.

### Why Now

0.13.0 is stable. The engine works. Decomposing it now is risk-free because:
1. No new features being added during refactor
2. Every move is mechanical (zero logic change)
3. CI catches every mistake
4. Each session is independently revertible

---

## Session Plan

All edits use the Edit tool (no PowerShell file manipulation). Law 1: read full file before each edit.

#### Session 1: Extract Utilities → `utils.rs` (~210 lines) ✅ COMPLETE
**Moves (verified line numbers from fresh read):**
- `parse_timeframe` (line 44, 12 lines)
- `parse_timeframe_minutes` (line 56, 12 lines)
- `derive_address_from_key` (line 70, 19 lines)
- `create_executor` (line 96, 103 lines)
- `load_knowledge_base` (line 200, 55 lines)

**Risk:** Very low — no dependencies on `run()` state
**engine.rs drops to:** ~7,004 lines

#### Session 2: Extract Training & Debug → `training.rs`, `debug.rs` (~2,467 lines) ✅ COMPLETE

**`debug.rs` moves:**
- `dry_run` (line 4749, 202 lines)
- `run_live_test` (line 4951, 240 lines)

**`training.rs` moves:**
- `fetch_and_cache` (line 5191, 30 lines)
- `expected_is_trade` (line 5281, 13 lines)
- `TrainingRunResult` struct (line 5294, 16 lines)
- `run_training_batch` (line 5310, 978 lines)
- `run_training` (line 6288, 173 lines)
- `run_action_test` (line 6461, 51 lines)
- `run_sandbox` (line 6512, 607 lines)
- `backup_databases` (line 5221, 36 lines)
- `rotate_backups` (line 5257, 21 lines)
- `has_actionable_signal` (line 7119, 56 lines)
- `verify_token_safety` (line 7176, 39 lines)

**Risk:** Low — independent entry points, don't share state with `run()`
**engine.rs drops to:** ~4,537 lines (only `run()` + its imports remain)

#### Session 3: Create Module Root + EngineState Struct ✅ COMPLETE

**Action:**
1. Create `src/engine/` directory
2. Move residual `engine.rs` → `src/engine/mod.rs`
3. Create `src/engine/mod.rs` header with re-exports
4. Add `EngineState` struct definition (48 fields)
5. Add `impl EngineState` with `new()` and `run_loop()` stubs

#### Session 4: Extract Init Phase → `EngineState::new()` ✅ COMPLETE

**Action:**
1. Move init code (~1,150 lines) into `EngineState::new()`
2. At end of `new()`, construct `Self { ... }` with all 48 fields
3. Change `run()` to: `let state = EngineState::new(config, shared, running).await?;`
4. Destructure `state` into local variables (same names as before)
5. Loop body unchanged — still uses local variables

**Key insight:** The loop body doesn't change at all. `run()` destructures the struct back into local variables, then runs the loop exactly as before. This eliminates risk of breaking the loop.

#### Session 5: Extract Loop Body → `EngineState::run_loop()` ⏸ DEFERRED

**Action:**
1. Move loop body (~3,336 lines) into `EngineState::run_loop()`
2. Change `run()` to: `state.run_loop().await`
3. Remove destructuring from `run()` — no longer needed

**Risk:** Medium — large code movement, but all `self.` references work because fields match local variable names
**mod.rs:** ~170 lines (just `run()` wrapper + struct + impl)

#### Session 6: Extract Cycle Sub-Phases into Methods ⏸ DEFERRED

**Extract from `run_loop()`:**
- `fetch_market_data()` — candle refresh + indicators (~524 lines)
- `run_agent_decisions()` — per-pair AI pipeline (~875 lines)
- `check_stops_and_close()` — position management (~1,140 lines)
- `reconcile_on_chain()` — on-chain reconciliation (~206 lines)
- `log_and_update_status()` — logging + shared state (~320 lines)

**Risk:** Low — each is a pure extract from the loop body

#### Session 7: Audit & Compliance ⏸ DEFERRED

1. Verify every sub-module is under 300 lines (or justified)
2. `cargo clippy -D warnings && cargo test && cargo build --release`
3. `cargo run -- --dry-run` (end-to-end pipeline)
4. Update MASTER-FID FID-090 status to "closed"
5. Update CHANGELOG.md
6. Bump version to 0.13.1

### Session Dependencies

```
Session 1 (utils)      ─┐
                        ├─→ Session 3 (struct) ─→ Session 4 (init)     ─┐
Session 2 (training)   ─┘         │                                     ├─→ Session 7 (audit)
                                  ├─→ Session 5 (loop body)            ─┤
                                  └─→ Session 6 (cycle sub-phases)     ─┘
```

### Expected Final File Structure

```
src/engine/
├── mod.rs              ~170 lines  (run() wrapper + EngineState struct + impl)
├── utils.rs            ~210 lines  (5 utility functions)
├── training.rs         ~1,857 lines (11 functions — oversize: test/training code)
├── debug.rs            ~442 lines  (2 functions)
├── init.rs             ~1,155 lines (3 functions — oversize: init code)
├── market.rs           ~524 lines  (fetch_market_data)
├── agent_decision.rs   ~875 lines  (run_agent_decisions)
├── trade.rs            ~269 lines  (execute_trade)
├── manage_positions.rs ~523 lines  (manage_open_positions)
├── check_stops.rs      ~258 lines  (check_stops)
├── close_positions.rs  ~359 lines  (close_positions)
├── reconciliation.rs   ~206 lines  (reconcile_on_chain)
└── status.rs           ~320 lines  (log_and_update_status)
```

---

## Lessons Learned

- Monolithic `run()` functions grow unbounded when no module structure exists to add features into. The fix is to create the module structure BEFORE the file exceeds the limit, not after.
- Rust's compiler is the best refactoring partner — let it tell you what's missing.
- Independent entry points (training, sandbox, dry-run) should NEVER live in the same file as the production loop. They should have been in separate modules from day 1.
- The `EngineState` struct pattern with `&mut self` methods is the idiomatic Rust approach for async state machines. It avoids the parameter explosion problem of free functions.
- Init phases should be decomposed into sub-functions early — they grow faster than loop bodies because every new feature adds setup code.
- **Never use PowerShell scripts to manipulate source files.** Law 1 requires reading the file and using the Edit tool. PowerShell scripts can corrupt files silently (wrong line ranges, missing content, encoding issues). The Edit tool is surgical and verified.
- **Destructuring the struct back into local variables is the safest refactor pattern.** The loop body doesn't change at all — `run()` just calls `new()`, destructures, and runs the loop. This eliminates risk of breaking the loop logic.
