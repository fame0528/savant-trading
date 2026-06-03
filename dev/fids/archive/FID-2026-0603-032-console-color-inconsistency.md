# FID: Console logging color inconsistency — ANSI codes mismanaged

**Filename:** `FID-2026-0603-032-console-color-inconsistency.md`
**ID:** FID-2026-0603-032
**Severity:** medium
**Status:** fixed
**Created:** 2026-06-03 15:50
**Author:** Agent

---

## Summary

Console log colors are inconsistent. Multiple issues found:
1. Tracing logs (INFO/WARN) are all white — ANSI disabled
2. Action and result use same color — no visual distinction
3. Pair names missing brackets — `BTC/USD` should be `[BTC/USD]`
4. LLM level too dim — grey on grey unreadable
5. Decision action is white, result is white — no contrast
6. RESET between sections was killing bold (fixed in initial pass)

## Environment

- **OS:** Windows 11 (PowerShell)
- **Commit:** `a656314` (initial FID-032 fix, incomplete)

## Detailed Description

### Problem

After initial FID-032 fix, user reported:
- Tracing logs are all white (no ANSI colors)
- `[DECISION]` action is white, result is white — same color
- Pair names should be `[BTC/USD]` not `BTC/USD`
- LLM section uses all dark colors — unreadable
- Lines like `[DECISION] Hold LONG @ 0.0000 | Conf: 0%...` have no visual structure

### Root Cause

1. **Tracing subscriber has `with_ansi(false)`** — all tracing output is plain white text
2. **Action and result colors are the same** for most levels — Phase (white/white), Decision (white/white), LLM (dim/dim)
3. **Pair highlighting** wraps pairs in color codes but not brackets
4. **LLM level** uses `GREY_DIM` for both action and result — too dark

### Correct Color Schema

| Level | Action | Result | Example |
|-------|--------|--------|---------|
| Phase | **Bold White** | White | `[PHASE2] 8 pairs queued` |
| Llm | **Grey** | **White** | `[LLM] Evaluating BTC/USD...` |
| LlmDone | **Grey** | **Green** | `[LLM] Complete BTC/USD` |
| Decision | **Bold Cyan** | White | `[DECISION] Hold LONG...` |
| Trade | **Bold Orange** | Orange | `[TRADE] OPENED...` |
| Swap | **Bold Cyan** | Dim | `[0x API] Calling...` |
| SwapOk | **Bold Green** | Green | `[0x QUOTE] OK...` |
| SwapFail | **Bold Red** | Red | `[0x API] Timeout...` |
| Vault | **Dim Grey** | Dim Grey | `[VAULT] Writing...` |
| Circuit | **Bold Red** | Red | `[CIRCUIT BREAKER]...` |
| Warn | **Bold Orange** | Orange | `[SIZING] None...` |

## Proposed Solution

1. Re-enable ANSI on tracing subscriber with custom format
2. Fix action/result color contrast — action always bold, result always different shade
3. Add brackets to pair highlighting — `[BTC/USD]` in cyan
4. Brighten LLM level — grey action, white result
5. Decision action should be bold cyan, not white

## Perfection Loop

### Loop 2 (this fix)

- **RED:** 6 color issues identified (tracing white, no contrast, missing brackets, LLM dim, decision white)
- **GREEN:** Fix all 6 issues in console.rs and main.rs
- **AUDIT:** (pending)
- **CHANGE DELTA:** (pending)
