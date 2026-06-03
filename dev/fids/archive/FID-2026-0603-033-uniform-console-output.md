# FID: Uniform console output for all log sources

**Filename:** `FID-2026-06-03-033-uniform-console-output.md`
**ID:** FID-2026-06-03-033
**Severity:** medium
**Status:** fixed
**Created:** 2026-06-03 16:20
**Author:** Agent

---

## Summary

Two different formatting systems were running: `savant_log()` with styled `[Savant Trading]` output, and `tracing` with plain `INFO`/`WARN` output. The inconsistency made the console look unprofessional.

## Environment

- **OS:** Windows 11
- **Commit:** `a8c2a34` (pre-fix)

## Detailed Description

### Problem

- `savant_log()` output: `[Savant Trading] [06-03-2026 4:11 PM] [LLM] [EVALUATING] [BTC/USD]` — styled, colored
- `tracing` output: `06-03-2026 4:11 PM INFO savant_trading::data::kraken: Fetched 721 candles` — plain, no colors

Two different formats, two different color systems, no consistency.

### Root Cause

`savant_log()` uses custom ANSI codes with `[Savant Trading]` prefix. The `tracing_subscriber::fmt()` uses its own format with `INFO`/`WARN` prefix and its own ANSI handling. They write to the same stderr but with completely different formats.

## Proposed Solution

### Approach

Create a custom `tracing::Layer` (`SavantLayer`) that formats ALL tracing events in the same `[Savant Trading] [TIME] [LEVEL] [module] message` format as `savant_log()`. This makes all output uniform.

### Implementation

1. Created `SavantLayer` struct implementing `tracing_subscriber::Layer`
2. `on_event()` formats events as `[Savant Trading] [TIME] [ACTION] [module] message`
3. Maps tracing levels to colors: ERROR=red, WARN=orange, INFO=grey/white, DEBUG/TRACE=dim
4. Extracts message via custom `MessageVisitor`
5. Extracts short module name (last segment of target path)
6. Applies pair name highlighting
7. Updated main.rs to use `tracing_subscriber::registry().with(SavantLayer)` instead of `fmt()`

## Perfection Loop

### Loop 1

- **RED:** Two formatting systems running — savant_log styled, tracing plain
- **GREEN:** Created SavantLayer custom tracing Layer with same format
- **AUDIT:** 187 tests pass, clippy clean
- **CHANGE DELTA:** ~100 lines in console.rs, 5 lines in main.rs

## Resolution

- **Fixed By:** Agent
- **Fixed Date:** 2026-06-03 16:40
- **Fix Description:** Created SavantLayer custom tracing Layer. All output now uses `[Savant Trading] [TIME] [ACTION] [RESULT]` format.
- **Verified By:** cargo build + cargo test + cargo clippy
- **Commit/PR:** (pending)

## Lessons Learned

- When mixing `tracing` and custom logging, a custom Layer is needed for uniform output
- `tracing_subscriber::registry().with(layer)` replaces `fmt().init()` — need `tracing_subscriber::prelude::*` import
- Doc comments (`///`) attach to the next item — use `//` for module-level comments that shouldn't document a specific item
