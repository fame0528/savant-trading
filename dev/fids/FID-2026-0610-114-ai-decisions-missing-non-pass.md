# FID-114: AI Decisions Panel Missing Non-PASS Decisions

**Filename:** `FID-2026-0610-114-ai-decisions-missing-non-pass.md`
**ID:** FID-2026-0610-114
**Severity:** high
**Status:** fixed
**Created:** 2026-06-10 21:00
**Author:** Buffy (Codebuff AI)
**Type:** bug-fix
**Scope:** src/api/mod.rs

---

## Summary

The AI Decisions panel in the dashboard only displays PASS decisions because `get_decisions` returns the 20 most recent decisions. When a cycle evaluates 40+ pairs, actionable decisions (BUY/SELL/CLOSE/ADJUST) that appear early in the batch are pushed out by later PASS decisions.

## Detailed Description

### Problem

STG/USD returned ADJUST (65% confidence) but doesn't appear in the dashboard. The LLM evaluates pairs in batch order. STG was processed first (index 0), followed by 39 PASS decisions. `get_decisions` returns the last 20 entries from the vec — STG is at index 0, outside the window.

### Expected Behavior

Non-PASS decisions (BUY/SELL/CLOSE/ADJUST) should always be visible in the AI Decisions panel, regardless of position in the batch.

### Root Cause

`get_decisions` in `src/api/mod.rs` does `decisions.iter().rev().take(20)` — pure recency, no action-type filtering.

### Fix

Pin non-PASS decisions at the top of the response. Show all non-PASS from the current cycle, then fill remaining slots with most recent PASS decisions.

## Resolution

- **Fixed By:** Buffy (Codebuff AI)
- **Fixed Date:** 2026-06-10 21:00
- **Fix Description:** Modified `get_decisions` to separate non-PASS and PASS decisions, return non-PASS first, then fill with recent PASS
- **Verified By:** clippy + tests
