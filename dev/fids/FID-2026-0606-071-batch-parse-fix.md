# FID-071: Fix Batch Evaluation JSON Parse Failure

**Status:** closed
**Severity:** critical
**Created:** 2026-06-06
**Closed:** 2026-06-06
**Author:** Kilo

---

## Problem

Batch LLM evaluation consistently fails JSON parse, falling back to 9 individual calls (5-9 minutes per cycle). The fallback wastes 9x the API cost and adds latency.

**Root cause hypothesis (verified from code):**
MiMo v2.5 Pro returns `<think>reasoning</think>` followed by content. After stripping thinking tags, the remaining content is NOT a valid JSON array. Most likely: MiMo returns individual JSON objects separated by newlines or text, not wrapped in `[...]`.

**Evidence:**
- `output_format.md` says "When evaluating MULTIPLE pairs, respond with a JSON array"
- But MiMo's system prompt instruction hierarchy may override this
- The `strip_thinking_tags()` function works correctly (verified in unit tests)
- The parse fails AFTER stripping, meaning the cleaned content isn't valid JSON array

**Code path (verified):**
1. engine.rs:1602 — batch `chat_stream` call with 180s timeout
2. engine.rs:1630 — `strip_thinking_tags(text)` 
3. engine.rs:1637-1640 — `serde_json::from_str::<Vec<Value>>(&cleaned)` — THIS FAILS
4. engine.rs:1656-1689 — fallback: 9 individual calls

## Goal

Make batch evaluation succeed on first parse. Eliminate the fallback path for normal operation.

## Scope

### Part A: Robust JSON array extraction
Instead of requiring the entire cleaned response to be a valid JSON array, extract JSON objects from the response regardless of surrounding text:
1. Try `serde_json::from_str::<Vec<Value>>(&cleaned)` first (fast path)
2. If that fails, use `serde_json` to find all `{...}` balanced brace blocks
3. Collect them into a Vec<Value>
4. If that fails, try regex for `{...}` blocks as last resort

### Part B: Verify with actual MiMo output
The debug logging at INFO level is already in place. On next restart, the terminal will show:
```
BATCH RAW (first 300): ...
BATCH CLEANED (first 300): ...
```
This confirms the exact format. Part A handles all cases regardless.

## Verification
- `cargo clippy -- -D warnings` clean
- `cargo test` pass
- Engine logs show "Parsed N decisions from batch response" instead of FALLBACK
- No individual LLM calls in normal operation

## Risk
- Low: extraction is additive — if Vec<Value> parse succeeds, extraction is skipped
- The fallback path still exists as safety net
