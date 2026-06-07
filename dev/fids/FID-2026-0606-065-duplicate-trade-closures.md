# FID-065: Fix Duplicate Trade Closures Inflating Win/Loss Count

**Status:** created
**Severity:** high
**Created:** 2026-06-06
**Author:** Kilo

---

## Problem

Dashboard shows 9W / 7L = 16 trades, but the closed trades list shows duplicates:
- ETH/USD Short at $1549.66→$1533.77 appears 4 times with identical P&L ($0.15)
- These are phantom closures — the same position was closed multiple times across ticks

The win/loss counter counts each closure independently, inflating the total from ~6 unique trades to 16.

## Root Cause

`check_stops()` in the portfolio/executor removes the position from the local map after the first stop hit, but something re-adds it (likely wallet recovery or state restore on the next tick), causing the stop to fire again.

## Goal

1. Deduplicate closed trades — same pair + same entry + same exit within a time window = 1 trade
2. Fix win/loss count to reflect unique trades only
3. Fix the root cause: prevent re-registration of already-closed positions

## Scope

- Audit `check_stops()` and `close_position()` for re-registration paths
- Add deduplication to closed trade recording (same pair+entry+exit+side within 60s = skip)
- Update win/loss counter to use deduplicated trades

## Verification

- Dashboard shows correct win/loss count matching unique closed trades
- No duplicate entries in closed trades list
