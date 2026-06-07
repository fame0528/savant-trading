# FID-069: Batch Fix + Dashboard Design Overhaul + Engine Hardening

**Status:** in_progress
**Severity:** high
**Created:** 2026-06-06
**Author:** Kilo

---

## Perfection Loop — Iteration 1

### RED Phase — All Issues Cataloged (13 items)

#### Original Issues (5)
1. Batch evaluation parse failure (system prompt conflicts with batch prompt)
2. Dashboard design quality (Performance, Market Insight, Risk Controls look bad)
3. Badge text "ADJUSTSTOP" should be "ADJUST STOP"
4. News ticker between rows
5. Savant logo + rename "AI Decisions" → "Savant Decisions"

#### New Suggestions (8)
6. Sound notifications for trades (import exists but unused)
7. Equity chart component (imported but unused)
8. Error/loading states (no connection error indicator)
9. Position close button (no manual override from dashboard)
10. Trade history CSV export
11. Keyboard shortcut for copy
12. FID-068 HeroUI color migration (deferred, low priority)
13. Batch evaluation timeout (no timeout on batch LLM call — can hang indefinitely)

### GREEN Phase — Proposed Fixes

| # | Issue | Fix | File | Lines | HeroUI |
|---|-------|-----|------|-------|--------|
| 1 | Badge text | `.replace("_", " ")` on display text | page.tsx:364 | 1 | — |
| 2 | Batch Part A: raw logging | `tracing::debug!` cleaned response before parse | engine.rs:1618 | 5 | — |
| 3 | Batch Part B: output_format.md | Add: "When evaluating **multiple** pairs, respond with JSON array. For single pair, respond with single JSON object." | output_format.md | 3 | — |
| 4 | Batch Part C: fallback extraction | **DEFERRED** — only if Part B fails | — | — | — |
| 5 | Batch timeout | Wrap batch `chat_stream` in `tokio::time::timeout(120s)` | engine.rs:1602 | 10 | — |
| 6 | Performance redesign | Color-code win/loss, prominence on win rate | page.tsx:215-232 | 20 | Card, Chip, ProgressBar |
| 7 | Market Insight redesign | Color-code Fear & Greed, funding rate | page.tsx:238-260 | 15 | Card, Chip, Badge |
| 8 | Risk Controls redesign | Color-code progress bars by severity | page.tsx:316-340 | 15 | Card, ProgressBar, Chip |
| 9 | News ticker | CSS scroll with trending coins + market data | new: NewsTicker.tsx | 40 | ScrollShadow, Chip |
| 10 | Savant logo + rename | Image icon, rename to "Savant Decisions" | page.tsx:343 | 5 | Avatar or Image |
| 11 | Sound notifications | Play sound on trade open/close events | page.tsx (useEffect) | 15 | — |
| 12 | Equity chart | Wire EquityChart into layout (if equity data available) | page.tsx | 20 | — |
| 13 | Error/loading states | "Disconnected" banner when API unreachable | page.tsx | 15 | Chip (danger) |
| 14 | Position close button | "Close" button on each position row with confirmation | page.tsx:265-295 | 20 | Button (danger) |
| 15 | Trade CSV export | Download CSV button on Closed Trades section | page.tsx:431 | 15 | — |
| 16 | Keyboard copy shortcut | `useEffect` keydown listener for Ctrl+C on focused section | page.tsx | 20 | — |
| 17 | HeroUI color migration | **DEFERRED** to FID-068 (separate, low priority) | — | — | — |

### AUDIT Phase — Five Questions

| # | Fix | All Cases | Scale | Attacker | 2 Years | Standard | Verdict |
|---|-----|-----------|-------|----------|---------|----------|---------|
| 1 | Badge text | Yes | Yes | N/A | Yes | Yes | PASS |
| 2 | Raw logging | Yes | Yes (debug!) | N/A | Yes | Yes | PASS |
| 3 | output_format.md | Guard: "multiple pairs" | Yes | N/A | Yes | Yes | PASS |
| 5 | Batch timeout | Yes — prevents indefinite hang | Yes | Yes — DoS protection | Yes | Yes | PASS |
| 6-8 | Dashboard design | Yes | Yes | N/A | Yes | Yes (HeroUI) | PASS |
| 9 | News ticker | aria-live="polite" | Yes | N/A | Yes | PASS with a11y | PASS |
| 10 | Logo | Yes | Yes | N/A | Yes | Yes | PASS |
| 11 | Sound | **RISK** — autoplay blocked by browsers | Yes | N/A | Yes | **RISK** | PASS with user gesture trigger |
| 12 | Equity chart | Yes — if data available | Yes | N/A | Yes | Yes | PASS |
| 13 | Error states | Yes | Yes | N/A | Yes | Yes | PASS |
| 14 | Close button | **RISK** — needs confirmation dialog | Yes | **RISK** — accidental close | Yes | Yes | PASS with double-confirm |
| 15 | CSV export | Yes | Yes | N/A | Yes | Yes | PASS |
| 16 | Keyboard copy | **RISK** — Ctrl+C conflicts with text selection | Yes | N/A | **RISK** | No — non-standard | REVISE: use Ctrl+Shift+C instead |
| 17 | HeroUI colors | DEFERRED | — | — | — | — | DEFER to FID-068 |

### SELF-CORRECT Phase

| # | Issue | Correction |
|---|-------|-----------|
| 11 | Sound autoplay blocked | Trigger sound only on first user interaction (click/keydown). Use `useRef` to track if user has interacted. |
| 14 | Close button accidental | Add confirmation dialog: "Close {pair} position? This will execute an on-chain swap." Use HeroUI `Modal` or `AlertDialog`. |
| 16 | Ctrl+C conflicts | Change to `Ctrl+Shift+C` to avoid overriding native copy. Or: only trigger when no text is selected. |
| 12 | Equity chart | Check if equity data is available from API before wiring. If `equity_snapshots` is empty, skip. |

### COMPLETE Phase — Iteration 1

**17 items cataloged. 3 corrections applied. 2 items deferred (Part C + HeroUI colors).**

**Convergence check:** All items have clear fixes, all risks mitigated. No oscillation detected. Proceeding to implementation order.

---

## Final Implementation Order

| Phase | Items | Rationale |
|-------|-------|-----------|
| **Phase 1: Quick fixes** | 1, 2, 3, 5 | Badge text, raw logging, output_format, batch timeout. No visual change. |
| **Phase 2: Dashboard design** | 6, 7, 8, 10 | Performance, Market Insight, Risk Controls, Logo. HeroUI components. |
| **Phase 3: New features** | 9, 11, 13 | News ticker, sound notifications, error states. |
| **Phase 4: Advanced** | 12, 14, 15, 16 | Equity chart, close button, CSV export, keyboard shortcuts. |
| **Phase 5: Verify batch** | 4 (if needed) | Only if Part B doesn't fix batch parsing. |

**Deferred:** 17 (HeroUI color migration → FID-068)

---

## Verification Plan

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all pass
3. `cargo fmt` — clean
4. `npm run build` — compiles
5. Dashboard loads with no console errors
6. Batch evaluation succeeds without fallback (check terminal for "Parsed N decisions")
7. News ticker scrolls smoothly
8. Sound plays on trade event (after user interaction)
9. Error banner appears when API disconnects
10. Close button shows confirmation dialog
11. CSV export downloads valid file
12. Keyboard shortcut works without conflicting with native copy

---

## Status

- [x] RED: All 17 issues cataloged
- [x] GREEN: All fixes documented
- [x] AUDIT: Five Questions passed (3 corrections applied)
- [x] SELF-CORRECT: Corrections applied
- [x] COMPLETE: Ready for implementation
