# FID-183: Dashboard Terminal Visual Fix

**Filename:** `FID-2026-0617-183-dashboard-terminal-visual.md`
**ID:** FID-2026-0617-183
**Severity:** medium
**Status:** created
**Created:** 2026-06-17 16:00 EST
**Author:** Vera
**Parent:** FID-182

---

## Summary

Dashboard Terminal column not rendering full-height. Spencer's observation: "the terminal is not filling 3 rows" and "slightly too tall after the row-span." Source code is correct (`grid-cols-3 grid-rows-[1.2fr_1fr_1fr]`, Terminal `row-span-3`), but stale Next.js dev server serves old build. Also: `1.2fr_1fr_1fr` makes top row taller than other two — change to `1fr_1fr_1fr` for equal heights.

---

## Environment

- **Commit:** `0adcc57c`
- **OS:** Windows 11
- **Node:** 16.2.7

---

## Detailed Description

### Problem

Per Spencer (2026-06-17 15:00 EST): "the terminal is not filling 3 rows" and "slightly too tall."

### Root Cause

1. **Stale dev server:** The Next.js dev server was started at 10:19 AM EST (before v0.14.5 build). It serves the pre-v0.14.5 build that has the wrong layout.
2. **Unequal row heights:** `grid-rows-[1.2fr_1fr_1fr]` makes the top row 1.2x taller than the other two. Spencer's "slightly too tall" observation suggests this is the issue.

### Evidence

**Source code at `dashboard/src/app/page.tsx` (line ~812):**
```tsx
<div className="flex-1 grid grid-cols-3 grid-rows-[1.2fr_1fr_1fr] gap-1.5 min-h-0">
  <div className="bg-[#0a0c14] border border-(--line) flex flex-col overflow-hidden row-span-3">
    {/* Terminal */}
  </div>
  {/* Closed Trades, Activity */}
</div>
```

The layout is correct in source. The visual issue is the stale server + the 1.2fr top row.

---

## Proposed Solution

### Actions

1. **Change `grid-rows-[1.2fr_1fr_1fr]` to `grid-rows-[1fr_1fr_1fr]`** in `dashboard/src/app/page.tsx`
   - Makes all 3 rows equal height
   - Eliminates "slightly too tall" issue

2. **Spencer's action (per "engine startup is Spencer's action" rule):**
   - Kill all node processes matching savant-trading
   - Kill any lingering next-server
   - Restart start.bat

3. **Vera's action:**
   - Verify dashboard build succeeds (`npm run build`)
   - Verify build artifact timestamp matches the new code

### Verification

- Visual check: Terminal fills the right column top-to-bottom
- All 3 rows equal height
- No "slightly too tall" issue

---

## Perfection Loop

### Loop 1 (RED)

Issues: None. This is a simple 1-line code change + server restart.

**CHANGE DELTA: N/A (trivial)**

### Loop 2 (GREEN)

Fix: Change `1.2fr_1fr_1fr` to `1fr_1fr_1fr`.

**CHANGE DELTA: 1 line**

### Loop 3 (AUDIT)

- [x] Source file confirmed at `dashboard/src/app/page.tsx`
- [x] Layout structure is correct except for the 1.2fr
- [x] Server restart is Spencer's action (documented in FID)

**CONVERGED at Loop 3.**

### Loop 4 (CONVERGENCE)

Delta = 0%. **COMPLETE.**

---

## Resolution

- **Fixed By:** Vera
- **Fixed Date:** Pending
- **Fix Description:** Changed `1.2fr_1fr_1fr` to `1fr_1fr_1fr`
- **Tests Added:** No
- **Verified By:** Spencer visual check after server restart
- **Commit/PR:** Pending

---

*Vera 0.1.0 — 2026-06-17 16:00 EST — FID-183 created. Trivial fix. 1 line + server restart.*
