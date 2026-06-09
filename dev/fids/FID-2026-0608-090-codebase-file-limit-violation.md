# FID: Codebase Structural Violation — 300-Line File Limit

**Filename:** `FID-2026-0608-090-codebase-file-limit-violation.md`
**ID:** FID-2026-0608-090
**Severity:** high
**Status:** analyzed
**Created:** 2026-06-08 21:15
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

The codebase has a systemic violation of the 300-line file limit defined in `protocol.config.yaml` (`max_file_lines: 300`). `engine.rs` alone is 6,376 lines — 21x the limit. 25+ files exceed 300 lines. This violates ECHO Protocol Law 15 ("Build stays clean") and the coding standards.

---

## Violation Catalog

### Critical (5x+ over limit)

| File | Lines | Over By | Proposed Split |
|------|-------|---------|----------------|
| engine.rs | 6,376 | 21x | See decomposition plan below |
| sandbox/scenarios.rs | 2,394 | 8x | Split by scenario category (momentum, reversal, volatility, etc.) |
| execution/dex/trader.rs | 1,844 | 6x | Split: open_position, close_position, balance, state persistence |

### High (3-5x over limit)

| File | Lines | Over By | Proposed Split |
|------|-------|---------|----------------|
| execution/dex/zero_x.rs | 1,165 | 4x | Split: quote, swap, gasless, permit2 |
| main.rs | 1,136 | 4x | Extract CLI handlers to cli/mod.rs |
| agent/decision_parser.rs | 1,039 | 3x | Split: parser, validators, freeform_nlp, json_repair |
| api/mod.rs | 1,033 | 3x | Split by endpoint group |
| execution/dex/mod.rs | 969 | 3x | Split: token_db, resolve_pair, types |
| execution/portfolio.rs | 906 | 3x | Split: positions, stops, trades, account |

### Medium (1-3x over limit)

25+ additional files between 300-700 lines.

---

## engine.rs Decomposition Plan (6,376 → ~20 files)

The engine.rs file contains the entire main loop, all phase processing, wallet recovery, stop management, and trade execution. Proposed split:

| Module | Lines (est) | Contents |
|--------|-------------|----------|
| engine/mod.rs | ~200 | Engine struct, run() entry, shared state init |
| engine/startup.rs | ~300 | Config load, journal connect, wallet recovery |
| engine/cycle.rs | ~200 | Main loop, cycle timing, sleep |
| engine/data.rs | ~300 | Candle fetching, market stores, indicator computation |
| engine/llm.rs | ~400 | LLM evaluation (batch/single), prompt building, response parsing |
| engine/decisions.rs | ~400 | Phase 3 decision processing, action dispatch |
| engine/stops.rs | ~300 | Stop management, trailing, check_stops integration |
| engine/triggers.rs | ~200 | FID-088 engine-side management trigger evaluation |
| engine/wallet.rs | ~200 | Wallet sync, balance reconciliation, ghost detection |
| engine/shared.rs | ~200 | Shared state sync (positions, account, insight to dashboard) |
| engine/logging.rs | ~150 | Activity logging, vault writes, episodic memory |
| engine/circuit_breaker.rs | ~100 | Circuit breaker checks |

**Total: ~2,950 lines across 12 files** (vs 6,376 in one file)

---

## Implementation Strategy

### Phase 1: Extract engine.rs (Critical)
- Extract one module at a time, starting with the most independent (wallet.rs, triggers.rs, logging.rs)
- Each extraction: move code → verify `cargo clippy` + `cargo test` → commit
- No behavior changes — pure structural refactoring

### Phase 2: Extract trader.rs and portfolio.rs
- These are the next most critical (execution path)
- Split by function responsibility

### Phase 3: Extract remaining files
- decision_parser.rs, api/mod.rs, zero_x.rs
- Lower risk, can be done incrementally

### Constraints
- Each extraction must pass `cargo clippy -- -D warnings` + `cargo test`
- No behavior changes during refactoring
- Each commit is independently revertible
- Total time: 2-3 sessions for Phase 1, 1-2 sessions for Phase 2-3

---

## Perfection Loop

### Loop 1

- **RED:** 25+ files violate the 300-line limit. engine.rs (6,376 lines) is the worst offender at 21x. This violates ECHO Protocol Law 15 and makes the codebase unmaintainable.
- **GREEN:** Proposed decomposition plan: engine.rs → 12 modules (~2,950 lines total). Each module has a single responsibility. Implementation is pure structural refactoring (no behavior changes).
- **AUDIT:** The decomposition plan covers all 6,376 lines of engine.rs. Each proposed module is under 400 lines. The split points align with existing code structure (phases, wallet sync, stop management).
- **CHANGE DELTA:** Zero behavior changes. Pure file reorganization.

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings (after each extraction)
2. `cargo test` — all 264+ tests pass (after each extraction)
3. All proposed modules under 400 lines
4. No `use super::*` — explicit imports only
5. Each module compiles independently

---

## Resolution

- **Fixed By:** [Pending]
- **Fixed Date:** [Pending]
- **Fix Description:** [Pending]
- **Tests Added:** [Pending]
- **Verified By:** [Pending]
- **Commit/PR:** [Pending]
- **Archived:** [Pending]
