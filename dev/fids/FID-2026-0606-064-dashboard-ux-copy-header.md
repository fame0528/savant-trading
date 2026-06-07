# FID-064: Dashboard UX — Copy Buttons + Hunt Mode Header Tag

**Status:** created
**Severity:** medium
**Created:** 2026-06-06
**Author:** Kilo

---

## Problem

1. No way to copy data from dashboard sections (Performance, Market Insight, Open Positions, Risk Controls, AI Decisions, Terminal, Closed Trades). Activity section has a copy button but no other section does.
2. Hunt mode status not visible in the header bar. User should see "HUNT" tag next to "LIVE · RUNNING" when hunt mode is active.

## Goal

1. Add copy button to all dashboard sections (same pattern as Activity)
2. Add hunt mode tag in header next to status indicators

## Scope

### dashboard/src/app/page.tsx
- Add copy button to: Performance, Market Insight, Open Positions, Risk Controls, AI Decisions, Closed Trades
- Add hunt mode tag in header: "HUNT" badge next to "LIVE · RUNNING"

### Existing copy pattern (from Activity section)
- Button with clipboard icon
- Copies section text content to clipboard
- Shows brief "Copied!" feedback

## Verification

- All sections have working copy buttons
- Hunt mode tag appears in header when portfolio.hunt_mode is true
- Hunt mode tag hidden when false
