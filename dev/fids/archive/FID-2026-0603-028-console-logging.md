# FID: Enterprise console logging with structured output

**Filename:** `FID-2026-0603-028-console-logging.md`
**ID:** FID-2026-0603-028
**Severity:** medium
**Status:** resolved
**Created:** 2026-06-03 18:00
**Author:** Agent

---

## Summary

Console output used inconsistent formatting — some logs used `tracing`, some used `eprintln!`, some had colors, most didn't. No timestamps. No structured format. Not enterprise quality.

## Detailed Description

### Problem

- Phase 2/LLM logs were plain text with no colors
- Phase 3 logs had inconsistent formatting
- No timestamps on any `eprintln!` output
- `tracing` deadlocked with API server RwLock (forced switch to `eprintln!`)
- No brand identity in console output

### Expected Behavior

All console output should follow a single structured format:
`[Savant Trading] [MM-DD-YYYY HH:mm] [ACTION] [RESULT]`

Colors should be consistent:
- Cyan — brand prefix
- Grey — timestamps
- White — info, decisions
- Green — success
- Orange — warnings, trade actions
- Red — errors, failures

## Resolution

- **Fixed By:** Agent
- **Fixed Date:** 2026-06-03 19:00
- **Fix Description:** Created `src/core/console.rs` with single `savant_log()` function, `LogLevel` enum, and 11 thin macros
- **Commit/PR:** `950429d`

## Lessons Learned

- Single source of truth for logging prevents format drift
- `#[macro_use]` on the module declaration propagates macros to the entire crate
- Binary files (`src/engine.rs`) need explicit `use crate::log_*` imports
