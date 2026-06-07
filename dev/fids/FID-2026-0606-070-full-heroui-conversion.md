# FID-070: Full HeroUI Conversion (Visual Parity — Wiring Only)

**Status:** in_progress
**Severity:** medium
**Created:** 2026-06-06
**Author:** Kilo

---

## CRITICAL CONSTRAINT

**The dashboard must look EXACTLY the same before and after.** Same colors, same spacing, same layout, same font sizes, same visual hierarchy. This is a code-only refactor — replace raw HTML elements with HeroUI components while preserving every visual detail. No design changes. No color changes. No layout changes.

**Verification:** Screenshot before, screenshot after, pixel-perfect comparison.

---

## Perfection Loop — RED Phase

### Custom Components That Need HeroUI Conversion

| # | Custom Pattern | Location | HeroUI Equivalent | Effort |
|---|---------------|----------|-------------------|--------|
| 1 | `KPI` component (custom div) | page.tsx:48-58 | `Card` + `Card.Header` + `Card.Content` | Medium |
| 2 | `CopyButton` (raw `<button>`) | page.tsx:76-86 | `Button variant="ghost" isIconOnly size="sm"` | Low |
| 3 | `SectionHeader` (custom div) | page.tsx:88-97 | `Card.Header` with icon + title + tag | Medium |
| 4 | Mode badge (LIVE/IDLE span) | page.tsx:169-174 | `Chip variant="soft" color="accent"` | Low |
| 5 | HUNT MODE badge (span) | page.tsx:176-179 | `Chip variant="soft" color="danger"` | Low |
| 6 | Panel containers (8 instances) | page.tsx:208,215,238,265,316,343,381,412 | `Card variant="secondary"` | Medium |
| 7 | Trending coin chips | page.tsx:255-259 | `Chip size="sm" color="accent" variant="soft"` | Low |
| 8 | Position side badge | page.tsx:285-287 | `Chip size="sm" color="success"/"danger"` | Low |
| 9 | Decision action badges | page.tsx:358-366 | `Chip size="sm" color="success"/"danger"/"warning"` | Low |
| 10 | Circuit breaker badge | page.tsx:321-324 | `Chip color="success"/"danger"` | Low |
| 11 | Closed trades table | page.tsx:444-469 | `Table` with `Table.Header`, `Table.Body`, `Table.Row`, `Table.Cell` | High |
| 12 | Win/Loss progress bar | page.tsx:222-224 | `ProgressBar` compound API with `color` prop | Low |
| 13 | Risk progress bars (3) | page.tsx:328,332,336 | `ProgressBar color="warning"/"danger"/"accent" size="sm"` | Low |
| 14 | Confidence progress bar | page.tsx:367-369 | `ProgressBar size="sm"` with conditional color | Low |
| 15 | Empty state messages (4) | page.tsx:269-347-422-441 | `Spinner` + text or keep as-is | Low |
| 16 | `react-hot-toast` Toaster | page.tsx:14,159-162 | HeroUI `Toast.Provider` + `toast()` | High |
| 17 | Terminal input (raw `<input>`) | Terminal.tsx:167-176 | `Input variant="secondary"` | Medium |
| 18 | Terminal send button | Terminal.tsx:177-182 | `Button variant="ghost" isIconOnly size="sm"` | Low |
| 19 | ErrorBoundary retry button | ErrorBoundary.tsx:36-39 | `Button variant="ghost" size="sm"` | Low |
| 20 | Equity chart spinner | EquityChart.tsx:14-18 | `Spinner` | Low |
| 21 | Dead dep: `react-hot-toast` | package.json | Remove after HeroUI Toast migration | Low |
| 22 | Dead dep: `howler`/`use-sound` | package.json | Remove (sounds.ts uses Web Audio API) | Low |
| 23 | Dead dep: `lucide-react` | package.json | Remove or use (currently unused) | Low |

### Items to KEEP (no HeroUI equivalent or intentional design)

| Pattern | Reason |
|---------|--------|
| `FearGauge` (SVG arc) | HeroUI `ProgressCircle` is a full circle, not a semicircle |
| `Icon` (FontAwesome) | Design choice, no HeroUI icon equivalent |
| `pnlClass` helper | Utility function, not a component |
| Terminal macOS dots | Intentional terminal aesthetic |
| Price range bar (position) | Unique visualization, no HeroUI equivalent |
| Body background gradient | Complex radial gradient, HeroUI theme doesn't cover |
| Custom scrollbar | No HeroUI scrollbar component |
| `--neon-red` / `--neon-red-glow` | Hunt mode specific, no HeroUI equivalent |
| `--cyan` / `--violet` | No HeroUI equivalent (`accent` is blue, not cyan) |
| ALL CSS variables | Keep every custom color, spacing, and font variable |
| ALL inline styles | Keep exact visual appearance |
| ALL Tailwind classes | Keep exact spacing, sizing, layout |

---

## GREEN Phase — Implementation Plan

### Phase 1: Core Component Swaps (Low Effort, High Impact)

**Rule: Every HeroUI component must preserve the exact visual output via className/style overrides.**

| # | Change | From | To | Visual Override |
|---|--------|------|-----|-----------------|
| 1 | CopyButton | `<button className="inline-flex items-center justify-center text-[var(--dim)] hover:text-[var(--cyan)] transition-colors cursor-pointer leading-none">` | `<Button variant="ghost" isIconOnly size="sm" className="text-[var(--dim)] hover:text-[var(--cyan)] min-w-0 w-auto h-auto p-0">` | Override HeroUI default padding/sizing to match existing |
| 2 | Mode badge | `<span className="inline-flex items-center gap-1.5 rounded border px-2 py-0.5 text-[9px] font-bold tracking-wider uppercase border-[var(--cyan)]/30 bg-[var(--cyan)]/10 text-[var(--cyan)]">` | `<Chip variant="soft" color="accent" size="sm" className="text-[9px] tracking-wider uppercase border border-[var(--cyan)]/30 bg-[var(--cyan)]/10 text-[var(--cyan)] rounded">` | Override Chip default styles to match existing |
| 3 | HUNT badge | `<span style={{ color: 'var(--neon-red)', ... }}>` | `<Chip variant="soft" color="danger" size="sm" className="..." style={{ color: 'var(--neon-red)', textShadow: 'var(--neon-red-glow)' }}>` | Keep inline styles for neon-red |
| 4 | Side badges | `<span className="text-[8px] px-1 py-0.5 rounded font-bold ...">` | `<Chip size="sm" className="text-[8px] px-1 py-0.5 rounded font-bold min-h-0 h-auto">` | Override Chip height/padding to match |
| 5 | Action badges | Same as #4 | Same as #4 | Same |
| 6 | Circuit breaker | `<span className="text-[9px] font-bold text-[var(--green)]">` | `<Chip size="sm" color="success" className="text-[9px] font-bold min-h-0 h-auto">` | Override |
| 7 | Trending chips | `<span className="text-[9px] px-1.5 py-0.5 rounded bg-[var(--cyan)]/10 border border-[var(--cyan)]/20 text-[var(--cyan)]">` | `<Chip size="sm" variant="soft" className="text-[9px] px-1.5 py-0.5 rounded bg-[var(--cyan)]/10 border border-[var(--cyan)]/20 text-[var(--cyan)] min-h-0 h-auto">` | Override |
| 8 | Empty states | `<i className="fa-solid fa-spinner fa-spin mr-2" />` | `<Spinner size="sm" className="mr-2" />` | Match existing spinner style |

### Phase 2: ProgressBar Migration (Low Effort)

| # | Change | From | To |
|---|--------|------|-----|
| 9 | Win/Loss bar | `ProgressBarRoot` + className colors | `<ProgressBar value={winRate} color="success">` |
| 10 | Drawdown bar | `ProgressBarRoot` + className | `<ProgressBar value={pct} color="warning" size="sm">` |
| 11 | Daily loss bar | `ProgressBarRoot` + className | `<ProgressBar value={pct} color="danger" size="sm">` |
| 12 | Positions bar | `ProgressBarRoot` + className | `<ProgressBar value={pct} color="accent" size="sm">` |
| 13 | Confidence bar | `ProgressBarRoot` + gradient | `<ProgressBar value={conf} size="sm">` with conditional color |

### Phase 3: Container Migration (Medium Effort)

**Rule: `Card` must look identical to existing panels. Use `variant="transparent"` + explicit className to preserve visual.**

| # | Change | From | To |
|---|--------|------|-----|
| 14 | KPI component | Custom div with `bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md p-2` | `<Card variant="transparent" className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md p-2">` + `Card.Header` + `Card.Content` |
| 15 | SectionHeader | Custom div with `flex items-center gap-2 px-3 pt-2 pb-1 border-b border-[var(--line)]` | Keep as custom div — `Card.Header` adds unwanted padding/structure |
| 16 | Panel containers (8) | `<div className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden">` | `<Card variant="transparent" className="bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md flex flex-col overflow-hidden rounded-none">` |

### Phase 4: Table + Toast Migration (High Effort)

**Rule: Table must look identical. Toast must match existing notification style.**

| # | Change | From | To |
|---|--------|------|-----|
| 17 | Closed trades | Raw `<table>` with custom classes | `<Table variant="secondary">` with exact same className on every subcomponent |
| 18 | Toast system | `react-hot-toast` | HeroUI `Toast.Provider` — **DEFERRED** (needs root layout change, high risk of visual change) |

### Phase 5: Component File Updates (Medium Effort)

| # | Change | From | To |
|---|--------|------|-----|
| 19 | Terminal input | Raw `<input>` | `<Input variant="secondary">` |
| 20 | Terminal button | Raw `<button>` | `<Button variant="ghost" isIconOnly size="sm">` |
| 21 | ErrorBoundary button | Raw `<button>` | `<Button variant="ghost" size="sm">` |
| 22 | EquityChart spinner | `fa-spinner fa-spin` | `<Spinner size="sm">` |

### Phase 6: Dependency Cleanup (Low Effort)

**Rule: Only remove deps that are confirmed dead. Verify with grep first.**

| # | Change | Action | Pre-check |
|---|--------|--------|-----------|
| 23 | `react-hot-toast` | **DEFERRED** — keep until toast migration is confirmed safe | — |
| 24 | `howler` / `use-sound` | Remove after verifying no imports reference them | `grep -r "howler\|use-sound" src/` |
| 25 | `lucide-react` | Remove after verifying no imports reference them | `grep -r "lucide-react" src/` |

---

## AUDIT Phase — Five Questions

| # | Change | All Cases | Scale | Attacker | 2 Years | Standard | Verdict |
|---|--------|-----------|-------|----------|---------|----------|---------|
| 1-8 | Chip/Button swaps | Yes | Yes | N/A | Yes | Yes | PASS |
| 9-13 | ProgressBar migration | Yes — compound API | Yes | N/A | Yes | Yes | PASS |
| 14-16 | Card containers | Yes | Yes | N/A | Yes | Yes | PASS |
| 17 | Table migration | Yes | Yes | N/A | Yes | Yes | PASS |
| 18 | Toast migration | **RISK** — HeroUI toast API may differ from react-hot-toast | Yes | N/A | Yes | Yes | PASS — verify API first |
| 19-22 | Component files | Yes | Yes | N/A | Yes | Yes | PASS |
| 23-25 | Dep cleanup | Yes | Yes | N/A | Yes | Yes | PASS |

### Risks Identified

| Risk | Mitigation |
|------|-----------|
| HeroUI `Card` adds padding/margins that break layout | Use `variant="transparent"` + explicit className to override ALL default styles |
| HeroUI `Chip` adds default height/padding | Override with `min-h-0 h-auto` + exact existing classes |
| HeroUI `Button` adds default sizing | Override with `min-w-0 w-auto h-auto p-0` |
| HeroUI `ProgressBar` default colors differ from existing | Keep manual className coloring, use compound API only for structure |
| HeroUI `Table` default styling differs | Override with exact existing classes on every subcomponent |
| SectionHeader → Card.Header adds unwanted structure | **Keep SectionHeader as custom div** — Card.Header adds padding/structure that would change layout |
| Toast migration changes notification appearance | **DEFERRED** — too risky for visual parity |

---

## SELF-CORRECT Phase

| Issue | Correction |
|-------|-----------|
| Win/Loss bar is dual-color (red bg + green fill) | Keep manual coloring — HeroUI `ProgressBar` doesn't support dual-color natively. Use compound API but override className for colors. |
| `--cyan` and `--violet` have no HeroUI equivalent | Keep custom CSS variables. HeroUI `accent` is blue, not cyan. Don't force-map. |
| `Card` padding may break existing layout | Use `Card variant="transparent"` + explicit padding via className |
| Toast migration needs root layout change | Wrap app in `<ToastProvider>` at layout.tsx level |

---

## COMPLETE Phase — Final Plan

### Execution Order

| Phase | Items | Files Changed | Estimated Lines |
|-------|-------|---------------|----------------|
| 1: Core swaps | 1-8 | page.tsx | ~30 lines changed |
| 2: ProgressBar | 9-13 | page.tsx | ~20 lines changed |
| 3: Containers | 14-16 | page.tsx | ~50 lines changed |
| 4: Table + Toast | 17-18 | page.tsx, layout.tsx, package.json | ~80 lines changed |
| 5: Component files | 19-22 | Terminal.tsx, ErrorBoundary.tsx, EquityChart.tsx | ~15 lines changed |
| 6: Dep cleanup | 23-25 | package.json | 3 lines |

### Verification

1. `npm run build` — compiles
2. `cargo clippy -- -D warnings` — clean
3. `cargo test` — pass
4. **Visual comparison** — screenshot before, screenshot after, pixel-perfect match
5. All HeroUI components render with identical visual to previous raw HTML
6. No layout shift from any component swap
7. No color change from any component swap
8. No spacing/sizing change from any component swap

---

## Status

- [x] RED: 25 items cataloged, 9 items marked "keep as-is"
- [x] GREEN: 6 phases with specific from→to mappings
- [x] AUDIT: 5 risks identified with mitigations
- [x] SELF-CORRECT: 4 corrections applied
- [x] COMPLETE: Ready for implementation
