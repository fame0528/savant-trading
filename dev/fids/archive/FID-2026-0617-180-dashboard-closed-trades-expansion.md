# FID-180: Dashboard Layout — Closed Trades Spans All 3 Rows + 2-3x Row Height

**Filename:** `FID-2026-0617-180-dashboard-closed-trades-expansion.md`
**ID:** FID-2026-0617-180
**Severity:** medium (UI/UX — Spencer wants the Closed Trades section to be more prominent for readability)
**Status:** created
**Created:** 2026-06-17 00:50 EST
**Author:** Vera
**Triggered by:** Spencer: "i also want to expand the closed trades to fill all 3 bottom rows and be 2-3x the height it currently is"

---

## Summary

The Closed Trades section in `dashboard/src/app/page.tsx` (lines 779-829) currently occupies row 3, column 3 of the bento grid (one cell). The user wants:
1. **Span all 3 rows** of the bento grid (col 3, rows 1+2+3) — making it 3x taller
2. **2-3x row height** for each trade row — more padding, larger text

Currently the table is at line 813: `trades.slice(0, 10).map((t) => ...)` with `py-0.5` (very tight) padding. After: 2-3x padding, larger text, scrollable list that fills the full available height.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Next.js 16.2.7, React 18
- **Commit/State:** post-v0.14.4 + FID-178 (`dcfe3798`)
- **Current time:** 2026-06-17 00:50 EST

---

## Detailed Description

### Current layout (line 381)

```tsx
<div className="flex-1 grid grid-cols-[1.6fr_1fr_1fr] grid-rows-[1.2fr_1fr_1fr] gap-1.5 min-h-0">
```

3-column × 3-row grid:
- Row 1: Equity (col 1) | Performance (col 2) | Market Insight (col 3)
- Row 2: Positions (col 1) | Risk (col 2) | Decisions (col 3)
- Row 2.5: Jury (full-width strip) — outside the grid
- Row 3: Console (col 1) | Activity (col 2) | **Closed Trades (col 3)** ← target

### Change

Wrap the Closed Trades container in Tailwind's `row-span-3 col-span-1`. This makes it span all 3 rows of column 3. The "Console" and "Activity" stay in row 3 of cols 1 and 2 (unchanged).

Alternative: `col-span-3 row-span-1` would make it span all 3 cols of row 3 (the full bottom). But that conflicts with Console and Activity. The user's intent is "all 3 bottom rows" — the bottom 3 rows of the grid are rows 1, 2, 3. So `row-span-3` matches.

### Per-row padding change

Current: `py-0.5` (= 2px top/bottom). At 2-3x: `py-1.5` (= 6px) or `py-2` (= 8px). Use `py-1.5 pr-2` for a clean look.

Current text: `text-[10px]`. Bump to `text-[11px]` or `text-[12px]`. Use `text-[11px]` for a moderate increase.

### Code change

```tsx
{/* Row 3 col 3: Closed Trades — spans all 3 rows of the right column */}
<div className="bg-(--panel) border border-(--line) backdrop-blur-md flex flex-col overflow-hidden row-span-3">
  <div className="flex items-center gap-2 px-3 pt-2 pb-1 border-b border-(--line)">
    <span className="inline-flex items-center"><Icon name="fa-receipt" className="text-(--dim) text-[10px]" /></span>
    <span className="text-[10px] tracking-[2px] uppercase font-semibold text-(--dim) leading-none">Closed Trades</span>
    <span className="ml-auto text-[9px] font-bold leading-none text-(--cyan)">{trades.length}</span>
    <span className="ml-auto inline-flex items-center">
      <CopyButton text={() => copyFormatters.trades(trades)} title="Copy closed trades" />
    </span>
    {trades.length > 0 && (
      <button
        onClick={() => downloadTradesCSV(trades)}
        className="inline-flex items-center justify-center text-(--dim) hover:text-(--cyan) transition-colors cursor-pointer leading-none"
        title="Download CSV"
      >
        <Icon name="fa-download" className="text-[9px]" />
      </button>
    )}
  </div>
  <div className="flex-1 px-3 pb-2 overflow-y-auto">
    {trades.length === 0 ? (
      <p className="text-(--dimmer) text-xs text-center py-4"><Icon name="fa-inbox" className="mr-1" />No closed trades yet.</p>
    ) : (
      <table className="w-full text-[11px]">
        <thead>
          <tr className="text-(--dimmer) text-left">
            <th className="py-1.5 pr-2"><Icon name="fa-hashtag" className="mr-0.5 text-[7px]" />PAIR</th>
            <th className="py-1.5 pr-2"><Icon name="fa-arrow-right-arrow-left" className="mr-0.5 text-[7px]" />SIDE</th>
            <th className="py-1.5 pr-2"><Icon name="fa-door-open" className="mr-0.5 text-[7px]" />ENTRY</th>
            <th className="py-1.5 pr-2"><Icon name="fa-door-closed" className="mr-0.5 text-[7px]" />EXIT</th>
            <th className="py-1.5 pr-2"><Icon name="fa-sack-dollar" className="mr-0.5 text-[7px]" />P&L</th>
            <th className="py-1.5"><Icon name="fa-percent" className="mr-0.5 text-[7px]" />%</th>
          </tr>
        </thead>
        <tbody>
          {trades.slice(0, 30).map((t) => (  // bumped from 10 to 30
            <tr key={t.id} className="border-t border-white/3 even:bg-white/1.5">
              <td className="py-1.5 pr-2 font-semibold">{t.pair}</td>
              <td className={`py-1.5 pr-2 ${pnlClass(t.side === "Long" ? 1 : -1)}`}>
                <span className="flex items-center gap-0.5"><Icon name={t.side === "Long" ? "fa-arrow-up" : "fa-arrow-down"} className="text-[7px]" />{t.side}</span>
              </td>
              <td className="py-1.5 pr-2 font-mono">{fmt.price(t.entry_price)}</td>
              <td className="py-1.5 pr-2 font-mono">{fmt.price(t.exit_price)}</td>
              <td className={`py-1.5 pr-2 font-mono ${pnlClass(t.pnl)}`}>{fmt.usd(t.pnl)}</td>
              <td className={`py-1.5 font-mono ${pnlClass(t.pnl_pct)}`}>{t.pnl_pct.toFixed(2)}%</td>
            </tr>
          ))}
        </tbody>
      </table>
    )}
  </div>
</div>
```

Key changes:
- Outer div: `row-span-3` added
- Table text: `text-[10px]` → `text-[11px]`
- Padding: `py-0.5` → `py-1.5` (3x)
- Trade slice: `slice(0, 10)` → `slice(0, 30)` (more trades visible at once)

### Expected Behavior

After this FID:
- Closed Trades section spans all 3 rows of the right column
- Each trade row is 3x taller (more padding)
- Text is slightly larger (11px instead of 10px)
- More trades visible at once (30 instead of 10)
- The table scrolls if trades exceed 30 (overflow-y-auto on the parent)

### Risks

- **Console and Activity might get squeezed.** With Closed Trades taking 3x more vertical space, the bento grid's row 3 col 1 and col 2 are still 1 row each. The grid uses `1.2fr_1fr_1fr` for rows, so rows 1+2+3 = total height. Closed Trades spans all 3. The other cells in col 1 and col 2 still take 1 row each. **This should be fine** — the bento grid uses `flex-1 min-h-0` so it fills available vertical space, and the row-span-3 just means Closed Trades has 3x the height of other cells.
- **Console scrolling.** Console currently has its own xterm scrolling. With less vertical space (since Closed Trades is taking more), Console might be cramped. But Console is interactive — user can scroll. Should be fine.
- **CSS Grid browser support.** `row-span-3` and `col-span-1` are CSS Grid features supported in all modern browsers. No risk.

---

## Impact Assessment

### Affected Components

- `dashboard/src/app/page.tsx` — 1 line addition (`row-span-3`), several padding/text changes

### Risk Level

- [ ] Critical
- [ ] High
- [x] Medium
- [ ] Low

UI change only. No logic change. Easy to revert.

### Latency Impact

- None (CSS only)

---

## Proposed Solution

### Approach

1. **Add `row-span-3` to the Closed Trades outer div.**
2. **Bump table padding from `py-0.5` to `py-1.5` (3x).**
3. **Bump table text from `text-[10px]` to `text-[11px]`.**
4. **Bump trade slice from 10 to 30** (more visible at once).

### Steps

1. **2 min:** Edit `dashboard/src/app/page.tsx` lines 779-829.
2. **3 min:** `npm run build` to verify TypeScript compiles.
3. **2 min:** `npm run dev` and verify the dashboard renders.
4. **3 min:** ECHO FID close-out.

**Total: ~10 min.**

### Verification

- `npm run build` — TypeScript compiles
- `npm run dev` — dashboard renders
- Closed Trades spans all 3 rows of column 3 (visually verify in browser)
- Each trade row is 3x taller (more padding)
- Text is 11px (slightly larger)
- 30 trades visible at once (more than before)

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** What if `row-span-3` causes Console and Activity to disappear (z-index issue, overflow)?
- **GREEN:** `row-span-3` is standard CSS Grid. It makes the cell take 3 rows. Other cells in col 1 and col 2 are unaffected. They stay in row 3.
- **AUDIT:** Verify in browser.
- **CHANGE DELTA:** 0 lines.

### Loop 2 (anticipated)

- **RED:** What if the table is too narrow at 1fr width (third of the bento grid)? The 6 columns might be cramped.
- **GREEN:** The user explicitly asked for the right column to be taller. They know it's narrow. The table has 6 columns: PAIR, SIDE, ENTRY, EXIT, P&L, %. Each is ~50px. Total ~300px. Fits in 1fr width.
- **AUDIT:** Verify in browser.
- **CHANGE DELTA:** 0 lines.

### Loop 3 (anticipated)

- **RED:** What if the user wanted "all 3 BOTTOM rows" to mean a separate section BELOW the bento grid, not span?
- **GREEN:** "All 3 bottom rows" most naturally means "all 3 rows of the bottom" (i.e., the right column spans all 3 rows). A separate section below would be "after the bento grid" or "new section at the bottom." The user said "rows" plural, suggesting grid rows.
- **AUDIT:** If wrong, easy to revert and add a separate section instead.
- **CHANGE DELTA:** depends on user feedback.

### Loop 4 (anticipated — questions Spencer should have asked but didn't)

- **Q: Should the table be sortable by P&L or %?**
  - Currently: trades are in reverse-chronological order. Sortable columns would be a UX improvement. Out of scope for this FID.
- **Q: Should the table be filterable by pair or side?**
  - Currently: no filter. All trades shown. Could add a filter row above the table. Out of scope.
- **Q: Should the table be exportable to CSV from the dashboard?**
  - Currently: there's a download button (line 788-794). Already exportable.
- **Q: Should the trade count badge (`{trades.length}`) be more prominent?**
  - Currently: small cyan text. Could be a chip. Out of scope.
- **Q: What if trades exceeds 30? Scroll?**
  - Yes, `overflow-y-auto` on the parent (line 797). After 30 trades, the table scrolls.
- **Q: Should the slice limit be configurable?**
  - Currently hardcoded to 30. Could be a config value. Out of scope.

### Loop 5 (anticipated)

- **RED:** What if the dashboard breaks on mobile (Tailwind responsive)?
- **GREEN:** The current dashboard uses fixed classes (no responsive variants in this section). On mobile, the table is narrow. The `row-span-3` works on all screens. Acceptable.
- **AUDIT:** No change.
- **CHANGE DELTA:** 0 lines.

---

## Resolution

*(Filled at close)*

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-17 01:00 EST
- **Fix Description:** Added `row-span-3` to Closed Trades container. Bumped table text from 10px to 11px. Bumped padding from `py-0.5` to `py-1.5` (3x). Bumped trade slice from 10 to 30.
- **Tests Added:** 0 (UI change only)
- **Verified By:** TBD
- **Commit/PR:** Pending (v0.14.4 batch)
- **Archived:** Pending

---

## Lessons Learned

*(Filled at close)*

---

*FID-180 created 2026-06-17 00:50 EST — Vera — Closed Trades spans all 3 rows of right column; per-row padding 3x; 30 trades visible at once*
