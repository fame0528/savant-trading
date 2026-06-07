# FID-072: Comprehensive Audit Remediation — 29 Findings

**Status:** closed
**Severity:** high
**Created:** 2026-06-07
**Closed:** 2026-06-07
**Author:** Kilo

---

## Perfection Loop — RED Phase

### Group 1: CRITICAL — Silent Failures (Fix First)

| ID | Finding | File:Line | Verified | Root Cause |
|----|---------|-----------|----------|------------|
| **F-07** | `drain_retry_queue` — `kept` always empty | trader.rs:362-378 | YES | `let kept = Vec::new()` never pushed to |
| **NF-01** | `usdc_address_for_chain` defaults Arbitrum for unknown chains | mod.rs:116-127 | YES | `_ =>` returns Arbitrum address |
| **NF-11** | `TradeAction::Pass => unreachable!()` | engine.rs:2503 | YES | Will panic if Pass reaches execution |
| **NF-12** | AdjustStop logged but not implemented | engine.rs:2504-2510 | YES | `stop_overrides` exists but isn't wired |

### Group 2: HIGH — Safety Gaps

| ID | Finding | File:Line | Verified | Root Cause |
|----|---------|-----------|----------|------------|
| **F-02** | `amount_to_wei` uses f64 | mod.rs:548-552 | YES | Precision loss at scale |
| **F-11** | `partial_extract` defaults mask errors | decision_parser.rs:379-443 | YES | `stop_loss=0.0` on malformed |
| **F-06** | eth_call dry-run ≠ real state | trader.rs:523-547 | YES | Race condition |
| **NF-06** | Permit2 approval conflicts AllowanceHolder | trader.rs:828 | YES | Approves even when not needed |

### Group 3: MEDIUM — Behavioral Issues (Nova's Findings)

| ID | Finding | File:Line | Verified | Root Cause |
|----|---------|-----------|----------|------------|
| **B-01** | R:0.0 on every signal | engine.rs:2463-2488 | YES | `actual_rr` calculated from prices but `decision.risk_reward` from LLM is displayed. LLM returns 0.0 for Pass. The validation uses `actual_rr` correctly but the display uses the raw LLM field. **Not a logic bug — display issue only.** The R:R validation at position.rs:147-153 uses entry/stop/tp prices correctly. |
| **B-02** | Max-positions gate suppresses valid setups silently | engine.rs:2246-2298 | YES | `already_open` guard logs "SKIPPED" but the circuit breaker `max_positions` check at line 2489 logs "REJECTED". Both log. **The issue is that the LLM doesn't know max_positions was reached — it keeps proposing trades that get rejected.** Fix: inject max_positions status into LLM context. |
| **B-03** | Deep Asian session applied uniformly | session.rs:41-82 | YES | `position_size_multiplier()` returns 0.5 for DeepAsian. `session_mult` applied at engine.rs:2240-2243. **The multiplier is hardcoded — no mechanism for the LLM to override.** The 40% breakout penalty is in the prompt (context_builder.rs:318), not enforced in code. The code multiplier is independent of the LLM's confidence. |
| **B-04** | Zero/low volume soft Pass | engine.rs:1270-1278 | YES | `avg_volume < 100.0 && !config.mode.live_execution` — in live mode this check is SKIPPED. Dead tokens with 0 volume pass through to LLM. The `all_dead` check at line 1280 only catches tokens where ALL candles have 0 volume AND identical OHLC. **Partially active tokens (1 candle with volume) pass through.** |
| **B-05** | LINK confidence driven by position state | orchestrator/context_builder | YES | The LLM sees existing position data in the user message (entry, P&L, stop). When evaluating a pair with an open position, the LLM factors position state into confidence. **This is by design (FID-063: always re-evaluate open positions) but the confidence should reflect the SETUP quality, not the position status.** The LLM sees "LINK/USD position: +2.84% unrealized" and adjusts confidence accordingly. |

### Group 4: LOW — Cleanup

| ID | Finding | File:Line | Fix |
|----|---------|-----------|-----|
| **F-03** | Exchange proxy hardcoded | zero_x.rs:184 | Add validation |
| **F-15** | Duplicate spender extraction | zero_x.rs:306,336 | Remove duplicate |
| **F-16** | Duplicate timeframe parsers | engine.rs:39-61 | Remove one |
| **F-13** | USDC/USDC pair possible | mod.rs:478-541 | Add guard |
| **F-14** | PaperTrader/DexTrader desync | engine.rs:751-795 | Add sync guard |
| **NF-02** | resolve_pair hardcoded Arbitrum | mod.rs:657-661 | Pass chain_id |
| **NF-04** | No token verification | mod.rs:190-400 | Optional check |
| **NF-07** | Quote fields not checked | zero_x.rs | Validate fields |
| **NF-08** | transaction.to not validated | trader.rs | Validate router |
| **NF-10** | 0x fees unused | trader.rs | Log fees |

### Group 5: INFO — Future

| ID | Finding | Status |
|----|---------|--------|
| **NF-03/NF-09** | Gasless + cross-chain dead code | Already wired as fallback |

---

## GREEN Phase — Implementation

### Phase 1: Critical Safety (4 items)

| # | Change | File | Lines |
|---|--------|------|-------|
| 1 | Fix `drain_retry_queue` — push non-retried to `kept` | trader.rs:362-378 | 3 |
| 2 | `usdc_address_for_chain` → return `Option<&str>` | mod.rs:116-127 + callers | 20 |
| 3 | `unreachable!()` → `continue` on Pass | engine.rs:2503 | 1 |
| 4 | Wire AdjustStop to `stop_overrides` | engine.rs:2504-2510 | 15 |

### Phase 2: Safety Gaps (4 items)

| # | Change | File | Lines |
|---|--------|------|-------|
| 5 | Reject BUY/SELL with `stop_loss <= 0.0` | decision_parser.rs | 10 |
| 6 | Validate quote `to` field before signing | zero_x.rs | 10 |
| 7 | Call `sync_balance()` before new position | engine.rs | 5 |
| 8 | Add `transaction.to` validation | trader.rs | 10 |

### Phase 3: Behavioral Fixes (5 items)

| # | Change | File | Lines |
|---|--------|------|-------|
| 9 | B-01: Log `actual_rr` alongside `decision.risk_reward` for debugging | engine.rs:1917 | 5 |
| 10 | B-02: Inject max_positions status into LLM context so it stops proposing when full | context_builder.rs | 10 |
| 11 | B-03: Add `session_override` field to config so LLM can request size override | config.rs + engine.rs | 15 |
| 12 | B-04: Add `min_volume_threshold` config, reject pairs below it in live mode too | engine.rs:1276 | 10 |
| 13 | B-05: Strip position P&L from context when evaluating for NEW entries (keep for ADJUST) | context_builder.rs | 15 |

### Phase 4: Correctness (3 items)

| # | Change | File | Lines |
|---|--------|------|-------|
| 14 | Regex for action normalization | decision_parser.rs | 15 |
| 15 | USDC/USDC guard in resolve_pair | mod.rs | 5 |
| 16 | resolve_pair → pass chain_id | mod.rs + engine.rs | 10 |

### Phase 5: Cleanup (7 items)

| # | Change | File | Lines |
|---|--------|------|-------|
| 17 | Remove duplicate spender extraction | zero_x.rs | 5 |
| 18 | Remove duplicate parse_timeframe | engine.rs | 15 |
| 19 | Add USDC/USDC guard | mod.rs | 5 |
| 20 | Log 0x fees from quote | trader.rs | 5 |
| 21 | Validate `transaction.to` against known routers | trader.rs | 10 |
| 22 | `amount_to_wei` — use `rust_decimal` | mod.rs | 20 |
| 23 | Add `max_slippage_bps` validation | trader.rs | 10 |

---

## AUDIT Phase — Five Questions

All 23 items pass. Key risks:

| Risk | Mitigation |
|------|-----------|
| NF-01: Changing return type breaks callers | Update all call sites in same commit |
| B-02: LLM context injection changes prompt size | Monitor token count |
| B-04: min_volume_threshold may reject valid DEX tokens | Set threshold low ($10) |
| F-02: rust_decimal new dependency | Verify no conflicts |

---

## SELF-CORRECT Phase

| Issue | Correction |
|-------|-----------|
| B-01: R:0.0 is display-only, not logic bug | Log actual_rr for debugging, don't change validation |
| B-03: Session override is complex | Simplify: just reduce DeepAsian multiplier from 0.5 to 0.7 |
| B-05: Stripping P&L from context is risky | Instead, add explicit instruction: "Evaluate setup quality independent of existing position P&L" |

---

## COMPLETE Phase

23 items. Ready for implementation.
