# FID-077: Sound Effects System — Win/Loss Audio Feedback

**Status:** green
**Severity:** low
**Created:** 2026-06-07
**Author:** Kilo

---

## Perfection Loop — RED Phase

### Issue: No audio feedback on trade results

**Severity:** LOW (UX enhancement)
**Location:** `dashboard/src/app/page.tsx`
**Evidence:** When a trade closes (win or loss), the dashboard updates silently. User has to actively watch the dashboard to know results.

---

## GREEN Phase — Proposed Solution

### Approach

Client-side sound system in the dashboard:

1. **Sound files** stored in `dashboard/public/sounds/wins/` and `dashboard/public/sounds/losses/`
2. **Sound utility** (`dashboard/src/lib/sounds.ts`) — loads file lists, plays random selection
3. **Trade detection** — poll `/api/trades` for new verified trades, play sound on new entry
4. **Volume control** — respect browser autoplay policy (sounds play after first user interaction)

### File Structure

```
dashboard/
├── public/
│   └── sounds/
│       ├── wins/        ← user provides .mp3/.wav files here
│       │   ├── win-1.mp3
│       │   └── win-2.mp3
│       └── losses/      ← user provides .mp3/.wav files here
│           ├── loss-1.mp3
│           └── loss-2.mp3
├── src/
│   ├── lib/
│   │   └── sounds.ts    ← sound player utility
│   └── app/
│       └── page.tsx     ← hook into trade polling
```

### Changes

| File | Change | Lines |
|------|--------|-------|
| `dashboard/src/lib/sounds.ts` | Sound player — load lists, random pick, play with volume | ~40 |
| `dashboard/src/app/page.tsx` | Detect new verified trades, trigger sound | ~15 |

### Sound Player Logic

```typescript
// Pre-defined file lists (Next.js serves from /public/sounds/)
const WIN_SOUNDS = ['/sounds/wins/win-1.mp3', '/sounds/wins/win-2.mp3'];
const LOSS_SOUNDS = ['/sounds/losses/loss-1.mp3', '/sounds/losses/loss-2.mp3'];

function playRandom(sounds: string[], volume = 0.5) {
  const pick = sounds[Math.floor(Math.random() * sounds.length)];
  const audio = new Audio(pick);
  audio.volume = volume;
  audio.play().catch(() => {}); // swallow autoplay blocks
}
```

### Trade Detection

Poll `/api/trades` every 15s. Track last seen trade ID. When a new verified trade appears:
- `pnl > 0` → `playRandom(WIN_SOUNDS)`
- `pnl < 0` → `playRandom(LOSS_SOUNDS)`

---

## AUDIT Phase

| # | Question | Answer |
|---|----------|--------|
| 1 | Will this work for ALL cases? | Yes — client-side, no backend changes |
| 2 | Will this scale? | Yes — just Audio() objects |
| 3 | Will this survive hostile attacker? | N/A — UX only |
| 4 | Maintainable in 2 years? | Yes — simple utility + polling |
| 5 | Standard setting? | Yes — standard web audio pattern |

**Verdict: PASS**

---

## Status

- [x] RED: Identified
- [x] GREEN: Architecture designed
- [x] AUDIT: Pass
- [x] SELF-CORRECT: Autoplay policy handled
- [x] COMPLETE: **AWAITING SOUND FILES FROM USER**
