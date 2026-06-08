# FID-083: AI Decisions Panel — Stale Decision Display

**Status:** verified
**Severity:** medium
**Created:** 2026-06-07
**Author:** Kilo

---

## Perfection Loop

### RED Phase
- No timestamp on decisions — user can't tell WHEN the decision was made
- Old decisions persist across cycles — looks like agent is repeating itself
- No "last evaluated X ago" indicator on section header
- Stale decisions (> 30 min) have no visual indicator

### GREEN Phase
1. SectionHeader tag: "live" → "2m ago" (dynamic from most recent decision timestamp)
2. Per-decision: add relative timestamp next to pair name ("ETH/USD · 2m ago")
3. Stale decisions (> 30 min): add `opacity-50` class

### AUDIT — Five Questions
All PASS. Display-only, uses existing dayjs import, standard relative time pattern.

### SELF-CORRECT
- Skipped entry/SL/TP display — too dense for the panel
- Used `dayjs.fromNow(true)` (without "ago" suffix) + manual " ago" for section header

### COMPLETE
3 changes, ~8 lines in page.tsx.

---

## Status
- [x] RED, GREEN, AUDIT, SELF-CORRECT, COMPLETE
- [x] Verified: `npm run build` PASS
