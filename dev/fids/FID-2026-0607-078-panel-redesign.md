# FID-078: Enterprise Panel Redesign — Performance, Market Insight, Risk Controls

**Status:** created
**Severity:** medium
**Created:** 2026-06-07
**Author:** Kilo

---

## Perfection Loop — RED Phase

### Issue: Three panels look basic and lack enterprise polish

**Severity:** MEDIUM (UX/branding)
**Location:** `dashboard/src/app/page.tsx` lines 303-558
**Evidence:** Performance, Market Insight, and Risk Controls panels use HeroUI v3 components (Chip, ProgressBar, Tooltip) but with minimal styling — inline IIFE color logic, cramped 11px text, no visual hierarchy, no card structure. Looks like a prototype, not a production trading terminal.

**Current problems:**
- No card structure — just flat `div` with border
- Inline IIFE color logic repeated everywhere (`(() => { ... })()`)
- 11px text throughout — no typographic hierarchy
- Progress bars are 1.5px tall — barely visible
- No dividers between sections
- Win/loss bar is a flat red/green strip — no depth
- Market Insight fear gauge is isolated, metrics are cramped
- Risk Controls has 3 nearly identical progress bar patterns copy-pasted
- No consistent spacing rhythm

---

## GREEN Phase — Proposed Solution

### Design Direction: **Dark Terminal Luxe**

Enterprise trading terminal aesthetic. Think Bloomberg Terminal meets Vercel dashboard. Deep blacks, crisp data hierarchy, subtle glow accents, generous spacing.

### Panel Structure (all 3 panels)

```
┌─────────────────────────────────────┐
│ ◈ PERFORMANCE              [copy]  │  ← SectionHeader (existing)
├─────────────────────────────────────┤
│                                     │
│  12W  5L         71%               │  ← Hero stat row (large)
│  ████████████░░░░░░░                │  ← Win/loss bar (thicker, animated)
│                                     │
├─────────────────────────────────────┤  ← Divider
│  Decisions    42    │  Trades   3   │  ← Metrics grid (larger text)
│  Conf cap     LOW   │  Brier  0.18 │
│  CUSUM        ✓     │              │
├─────────────────────────────────────┤
│  ◈ HUNT MODE                       │  ← Status badge (existing)
└─────────────────────────────────────┘
```

### Component Mapping (HeroUI v3)

| Current (v2/stale) | v3 Replacement | Notes |
|---------------------|----------------|-------|
| `ProgressBarRoot`/`ProgressBarFill` | `ProgressBar` > `ProgressBar.Track` > `ProgressBar.Fill` | v3 API with `value` prop |
| Manual `<div className="border-t">` | `Separator` | v3 renamed from Divider |
| Flat `<div className="bg-[var(--panel)]">` | `Card` | v3: `Card.Header`, `Card.Content`, `Card.Footer` |
| Inline IIFE color logic | `MetricRow` + `RiskBar` components | Extract, deduplicate |

### Changes

| # | Change | File | Lines |
|---|--------|------|-------|
| 1 | Import `Separator`, `Card` from `@heroui/react` | `page.tsx` | +2 imports |
| 2 | Migrate `ProgressBarRoot`/`ProgressBarFill` → `ProgressBar` v3 API | `page.tsx` | ~20 |
| 3 | Extract `MetricRow` component — reusable label+value row | `page.tsx` | -40 (deduplicated) |
| 4 | Extract `RiskBar` component — labeled `ProgressBar` with threshold colors | `page.tsx` | -30 (deduplicated) |
| 5 | Add `Separator` between logical sections in each panel | `page.tsx` | +6 |
| 6 | Wider progress bars (1.5px → 6px via `size="md"`) | `page.tsx` | ~10 |
| 7 | Larger metric values (11px → 13px, labels stay 10px) | `page.tsx` | ~15 |
| 8 | Fear gauge inline next to metrics, not isolated | `page.tsx` | ~10 |
| 9 | Consistent `gap-3` spacing rhythm | `page.tsx` | ~5 |

### Component Extractions

**MetricRow** — eliminates 6x repeated pattern:
```tsx
function MetricRow({ icon, label, value, color }: {
  icon: string; label: string; value: React.ReactNode; color?: string;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5">
        <Icon name={icon} className="text-[7px]" />{label}
      </span>
      <span className={`text-[13px] font-mono font-semibold ${color ?? ""}`}>{value}</span>
    </div>
  );
}
```

**RiskBar** — eliminates 3x copy-pasted progress bar pattern using HeroUI v3 `ProgressBar`:
```tsx
import { ProgressBar, Label } from "@heroui/react";

function RiskBar({ icon, label, current, max, tooltip }: {
  icon: string; label: string; current: number; max: number; tooltip?: string;
}) {
  const pct = max > 0 ? (current / max) * 100 : 0;
  const color = pct < 50 ? "success" : pct < 80 ? "warning" : "danger";
  return (
    <Tooltip delay={300}>
      <div className="cursor-help">
        <ProgressBar aria-label={label} size="md" value={pct} color={color}>
          <Label>
            <span className="text-[10px] text-[var(--dim)] flex items-center gap-1.5">
              <Icon name={icon} className="text-[7px]" />{label}
            </span>
          </Label>
          <ProgressBar.Output>
            <span className="text-[13px] font-mono font-semibold">
              {(current * 100).toFixed(1)}% / {(max * 100).toFixed(0)}%
            </span>
          </ProgressBar.Output>
          <ProgressBar.Track>
            <ProgressBar.Fill />
          </ProgressBar.Track>
        </ProgressBar>
      </div>
      {tooltip && <Tooltip.Content showArrow><p className="text-[10px]">{tooltip}</p></Tooltip.Content>}
    </Tooltip>
  );
}
```

---

## AUDIT Phase — Five Questions

| # | Question | Answer |
|---|----------|--------|
| 1 | Will this work for ALL cases? | Yes — components handle null/zero/undefined |
| 2 | Will this scale? | Yes — extracted components reduce duplication |
| 3 | Will this survive hostile attacker? | N/A — display only |
| 4 | Maintainable in 2 years? | Yes — MetricRow and RiskBar are single-source |
| 5 | Standard setting? | Yes — consistent spacing, typography, color |

**Verdict: PASS**

---

## SELF-CORRECT Phase

| Issue | Correction |
|-------|-----------|
| Don't break existing copy functionality | Keep `onCopy` handlers on SectionHeaders |
| Don't break existing tooltips | Reuse HeroUI Tooltip, just restructure layout |
| Don't change data flow | Same props, same API calls, just visual |
| Keep it dark — don't add light theme | Match existing `--bg`, `--panel`, `--line` vars |

---

## COMPLETE Phase

**1 FID. 1 file (page.tsx). ~80 lines removed (deduplication), ~40 lines added (components). Net cleaner.**

### Verification

1. `npm run build` — TypeScript pass
2. Visual: all 3 panels render correctly
3. Copy buttons still work
4. Tooltips still work
5. Colors still respond to data values

---

## Status

- [x] RED: 3 panels audited
- [x] GREEN: 2 extracted components + layout redesign
- [x] AUDIT: All Five Questions pass
- [x] SELF-CORRECT: 4 corrections applied
- [x] COMPLETE: **AWAITING USER APPROVAL**
