# FID: Console logging production readiness

**Filename:** `FID-2026-0603-037-console-production-ready.md`
**ID:** FID-2026-06-03-037
**Severity:** high
**Status:** analyzed
**Created:** 2026-06-03 18:20
**Author:** Agent

---

## Summary

Console output has multiple formatting issues that make it unprofessional and inconsistent. Double brackets on pairs, tracing logs all same color, wrong module names, and GoPlus spam for core assets.

## Issues Found (from live log analysis)

| # | Issue | Severity |
|---|-------|----------|
| 1 | Double brackets: `[[BTC/USD]]` should be `[BTC/USD]` | High |
| 2 | Tracing logs all same grey color — no distinction between INFO/WARN/ERROR | High |
| 3 | Module names wrong: `FundingRates` → `Funding Rates`, `Onchain` → `On Chain` | Medium |
| 4 | `Kraken` shown as source when using 0x DEX | Medium |
| 5 | GoPlus warns 13 times per cycle for core assets that don't need checks | Medium |
| 6 | `[LLM]` all dark grey — no alternating colors for readability | Medium |
| 7 | `watcher` injection pattern spam — 14 identical warnings per startup | Low |
| 8 | `[VAULT] writing` and `[VAULT] done` too verbose — can consolidate | Low |

## Proposed Solution

### Fix 1: Double brackets
`highlight_pairs()` is wrapping already-bracketed pairs. Fix: skip if already contains `[pair]`.

### Fix 2: Tracing colors
SavantLayer maps INFO→grey, WARN→orange, ERROR→red. But the module name is always grey. Fix: color the module name based on level.

### Fix 3: Module names
Add a mapping: `funding_rates` → `Funding Rates`, `onchain` → `On Chain`, `websocket` → `WebSocket`.

### Fix 4: Exchange source
Replace `[Kraken]` with actual exchange name from config.

### Fix 5: GoPlus spam
Skip GoPlus check for core pairs (BTC, ETH, SOL, etc.) — only check meme coins.

### Fix 6: LLM colors
Use alternating grey/white for `[LLM]` lines to improve readability.

### Fix 7: Watcher spam
Deduplicate watcher warnings — only log unique patterns once.

### Fix 8: Vault verbosity
Consolidate `[VAULT] writing` + `[VAULT] done` into single line.

## Perfection Loop

### Loop 1

- **RED:** 8 formatting issues identified from live log output
- **GREEN:** (pending)
- **AUDIT:** (pending)
- **CHANGE DELTA:** (pending)
