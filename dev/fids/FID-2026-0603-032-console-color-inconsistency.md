# FID: Console logging color inconsistency — ANSI codes mismanaged

**Filename:** `FID-2026-0603-032-console-color-inconsistency.md`
**ID:** FID-2026-0603-032
**Severity:** medium
**Status:** analyzed
**Created:** 2026-06-03 15:50
**Author:** Agent

---

## Summary

Console log colors are inconsistent. `[Savant Trading]` is not always cyan, `[DECISION]` is white, token names are white, `[VAULT]`/`[EPISODIC]` are too dim. The root cause is ANSI code mismanagement in `savant_log()` — `RESET` between each section kills bold, and result color bleeds to the next line.

## Environment

- **OS:** Windows 11 (PowerShell)
- **Commit:** `3738e45` (pre-fix)

## Detailed Description

### Problem

Expected output:
```
[Savant Trading] [06-03-2026 3:46 PM] [DECISION] Hold LONG @ 0.0000 | Conf: 0% | BTC/USD...
  ^^^^^^^^^^^^^    ^^^^^^^^^^^^^^^^   ^^^^^^^^^   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    cyan bold         grey dim         white bold      white (result)
```

Actual output: colors are inconsistent — some sections are white, some are grey, some are dark.

### Root Cause

The `savant_log()` function uses `RESET` (`\x1b[0m`) between each section:

```rust
eprintln!(
    "{}{}[Savant Trading]{} {}[{}]{} {}{}[{}]{} {}{}",
    BOLD, CYAN, RESET,    // ← RESET kills bold AND cyan
    GREY, ts, RESET,       // ← RESET kills grey
    action_color, BOLD, action, RESET,  // ← RESET kills action color AND bold
    result_color, result,  // ← NO RESET at end — bleeds to next line
);
```

**Problem 1:** `RESET` (`\x1b[0m`) resets ALL attributes — bold, foreground, background, underline. So after `[Savant Trading]{RESET}`, the bold is gone and the next section starts from scratch.

**Problem 2:** No `RESET` at the end of the result — the result color bleeds into the next line (which is why tracing output appears in the wrong color).

**Problem 3:** The tracing subscriber uses `with_ansi(false)` which makes all tracing output white, but the ANSI state from our previous `savant_log()` call can still affect it.

### Evidence

```text
# User report:
- [Savant Trading] is still not consistently cyan
- [DECISION] is white
- Token names (BTC/USD) are white
- [VAULT]/[EPISODIC] are dark gray
- [PHASE] are white
- [LLM] is dark

# Root cause: RESET between each section kills all formatting
```

## Proposed Solution

### Approach

Rewrite `savant_log()` to use compound ANSI codes (`\x1b[1;36m` = bold+cyan) instead of separate codes with RESET between them. Only RESET at the very end.

### Correct Format

```
\x1b[1;36m[Savant Trading] \x1b[0;90m[{ts}] \x1b[1;{action_color}m[{action}] \x1b[0;{result_color}m{result}\x1b[0m
```

This way:
- `[Savant Trading]` is always bold+cyan
- `[{ts}]` is always dim grey
- `[{action}]` is always bold in the level color
- `{result}` is always in the level color
- Final `\x1b[0m` resets everything so the next line starts clean

### Token Name Highlighting

Pair names like `BTC/USD`, `ETH/USD` should be highlighted in the result. Use a regex or simple string replacement to wrap known pair names in color codes.

## Perfection Loop

### Loop 1

- **RED:** Colors inconsistent — RESET between sections kills bold, result bleeds to next line
- **GREEN:** Rewrite savant_log() with compound ANSI codes, single RESET at end, pair highlighting
- **AUDIT:** (pending)
- **CHANGE DELTA:** (pending)

## Lessons Learned

- `RESET` (`\x1b[0m`) resets ALL attributes — bold, foreground, background. Use compound codes (`\x1b[1;36m`) to set multiple attributes at once.
- Always RESET at the end of a colored line to prevent bleeding to the next line.
- Tracing subscriber with `with_ansi(false)` outputs plain text, but the terminal's ANSI state from the previous line can still affect it.
