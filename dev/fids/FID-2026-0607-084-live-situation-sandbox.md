# FID-084: Live Situation Sandbox — Test Any Model Against Current Market

**Status:** analyzed
**Severity:** high
**Created:** 2026-06-07
**Author:** Kilo

---

## Perfection Loop — RED Phase

### Issue: No way to test a model against the current live situation

**Severity:** HIGH (operational)
**Location:** `main.rs` CLI parsing, `engine.rs` dry_run()
**Evidence:** 
- Sandbox uses prebuilt scenarios (TRD-001 through VOL-005) — don't reflect actual positions
- `--dry-run` uses the config model — can't swap models
- No way to test if a free model (owl-alpha) can handle the current ETH/LINK positions before switching
- User has $11 on OpenRouter — needs to validate free model before spending credits
- Restarting engine just to test a model wastes time and risks state corruption

### What the user needs
- Feed current live market data (candles, insight, positions) into any model
- See the model's response (action, confidence, reasoning, R:R, stop, TP)
- Do this WITHOUT starting the engine or affecting state
- Run while the engine is already active (read-only)

---

## GREEN Phase — Proposed Solution

### New CLI flag

```bash
cargo run --release -- --live-test --model openrouter/owl-alpha
cargo run --release -- --live-test --model openrouter/owl-alpha --pairs ETH/USD,LINK/USD
cargo run --release -- --live-test --model xiaomi/mimo-v2.5-pro --show-prompt
```

### Data flow

```
CLI --live-test --model owl-alpha
  → load config (pairs, risk, strategy)
  → fetch candles for all pairs (parallel, same as live engine)
  → fetch insight (funding, fear/greed, MVRV, news)
  → load knowledge base
  → load positions from dex_state.json (read-only)
  → build batch prompt (same as live engine Phase 2)
  → call LLM via OpenRouter with specified model
  → parse response (4-pass parser)
  → print decisions with full context
```

### Output format

```
=== LIVE SITUATION TEST ===
Model: openrouter/owl-alpha
Time: 2026-06-07 22:54:00 EDT
Pairs: ETH/USD, LINK/USD
Positions: 2 open (ETH @ 1681.95, LINK @ 7.88)
Equity: $26.39

--- LLM RESPONSE (63610ms) ---
[raw response text]

--- PARSED DECISIONS ---
ETH/USD  ADJUST_STOP  72%  R:3.4  SL: $1638→$1675  Reason: ...
LINK/USD  ADJUST_STOP  68%  R:3.6  SL: $6.71→$7.85  Reason: ...
```

### Implementation

| # | Change | File | Lines |
|---|--------|------|-------|
| 1 | Add `"live-test"` match arm in CLI parser | main.rs | ~15 |
| 2 | Extract shared context-building from `dry_run()` into `build_live_context()` | engine.rs | ~50 (refactor) |
| 3 | New `run_live_test(config, model, pairs, show_prompt)` function | engine.rs | ~80 |
| 4 | Parse `--model`, `--pairs`, `--show-prompt` flags | main.rs | ~10 |

### Key design decisions

- **Reuses all existing infrastructure** — CandleClient, InsightAggregator, KnowledgeBase, PromptComposer, LlmProvider. No new dependencies.
- **Same prompt pipeline as live engine** — tests the EXACT prompt the model would see in production.
- **Read-only** — doesn't start API server, doesn't write to state files, doesn't bind ports.
- **Can run alongside active engine** — no conflicts (reads dex_state.json which is atomically written).
- **Parallel candle fetch** — uses `tokio::join_all` for all pairs, ~3-5s total.

---

## AUDIT Phase — Five Questions

| # | Question | Answer |
|---|----------|--------|
| 1 | ALL cases? | Yes — free/paid models, with/without positions, any market condition |
| 2 | 1000 agents? | N/A — CLI tool, single user |
| 3 | Hostile attacker? | N/A — read-only, no state mutation |
| 4 | 2 years? | Yes — standard CLI pattern, model-agnostic via OpenRouter |
| 5 | Standard? | Yes — every production trading system has "test with live data" |

**Verdict: PASS**

**Double Audit:**
- Static: Reuses existing prompt pipeline — same code path as live engine
- Runtime: `cargo run --release -- --live-test --model openrouter/owl-alpha` — verify output

---

## SELF-CORRECT Phase

| Issue | Correction |
|-------|-----------|
| Candle fetch takes 2-3s per pair | Parallel fetch with `tokio::join_all` — 3-5s total |
| dry_run already does 90% of this for 1 pair | Extract shared logic into `build_live_context()` (Law 13) |
| dex_state.json read while engine running | Atomic writes — read gets old or new, never corrupt |
| Should show raw LLM response? | Yes — always show for quality evaluation |
| Should show the prompt? | Optional `--show-prompt` flag — default off (too long) |
| Output format | Terminal-friendly: pair, action, confidence, R:R, stop, reasoning |

---

## COMPLETE Phase

**1 CLI flag + 1 new function + refactor dry_run. ~80 new lines, ~50 refactored.**

### Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — 217/217 pass
3. Manual: `cargo run --release -- --live-test --model openrouter/owl-alpha` — verify output

### Future enhancements (separate FIDs)

- `--compare` flag: run same prompt against multiple models side by side
- `--record` flag: save test results to DB for model performance tracking
- `--pairs-from-positions` flag: only test pairs we have positions in

---

## Status

- [x] RED: Issue traced
- [x] GREEN: Solution designed
- [x] AUDIT: Five Questions PASS
- [x] SELF-CORRECT: 6 corrections applied
- [x] COMPLETE: **AWAITING USER APPROVAL**
