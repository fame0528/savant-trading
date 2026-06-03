# FID: ANSI colors not rendering — codes placed after text

**Filename:** `FID-2026-0603-034-ansi-color-placement.md`
**ID:** FID-2026-0603-034
**Severity:** medium
**Status:** fixed
**Created:** 2026-06-03 16:30
**Author:** Agent

---

## Summary

Console output was uniform in format but all white — ANSI color codes were placed AFTER the text they should color, so they applied to nothing.

## Detailed Description

### Problem

Format string was:
```rust
eprintln!("{}[Savant Trading]{} {}[{}]{} ...", CYAN_BOLD, RESET, GREY_FG, ts, RESET, ...);
```

The color code `CYAN_BOLD` is argument 1, applied to `[Savant Trading]` (correct).
But `GREY_FG` is argument 3, applied to `[` (the bracket after RESET), not to the timestamp.

### Root Cause

In `eprintln!`, the `{}` placeholders consume arguments left-to-right. The color code needs to be the argument IMMEDIATELY BEFORE the text it colors. The old format had colors and text interleaved incorrectly.

### Fix

Use named format parameters:
```rust
format!("{cyan}[Savant Trading]{reset} {grey}[{ts}]{reset} ...",
    cyan = CYAN_BOLD, reset = RESET, grey = GREY_FG, ts = ts, ...);
```

Also:
- Capitalized module names: `funding_rates` → `FundingRates`
- Stripped surrounding quotes from tracing message Debug format

## Resolution

- **Fixed By:** Agent
- **Fixed Date:** 2026-06-03 17:00
- **Commit:** `b7999ab`
