# FID-067: Engine Freeze After Batch Complete + Hunt Mode Color

**Status:** closed
**Severity:** critical
**Created:** 2026-06-06
**Closed:** 2026-06-06
**Author:** Kilo

---

## Problem

### Issue 1: Engine Freeze (CRITICAL)
After "BATCH COMPLETE 9 pairs, 7370 chars in 76442ms" at 7:51 PM, the engine never reaches PHASE3. No further log output for 63+ minutes. System is completely hung.

**Root cause hypothesis (needs verification):**
The batch LLM response fails JSON parse at engine.rs:1616, triggering the fallback path (lines 1642-1663) which runs 9 sequential `chat_stream` calls. Each has a 300s timeout. 9 × 300s = 45 minutes of blocking. During this time, no log output appears because the fallback path doesn't log individual calls.

**Alternative hypothesis:**
JSON parse succeeds but the decision processing loop (lines 1686+) hangs on a lock or network call.

### Issue 2: Hunt Mode Color
Dashboard shows hunt mode tag in orange. User wants neon red.

## Scope

### Issue 1 Fix
- Read the fallback path (engine.rs:1642-1663) completely
- Add logging to each fallback LLM call so we can see progress
- Add a timeout wrapper around the entire fallback loop (not just per-call)
- Consider: should the fallback exist at all? If batch JSON fails, should we just skip rather than burn 9 API calls?

### Issue 2 Fix
- Change `var(--orange)` to neon red in hunt mode tag (page.tsx:175, 231)

## Verification
- `cargo clippy -- -D warnings` clean
- `cargo test` pass
- `npm run build` pass
- Engine processes batch results without freezing
- Hunt mode tag displays in neon red
