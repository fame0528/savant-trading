# FID-073: Overnight Log Issues + Deferred Items from FID-072

**Status:** created
**Severity:** high
**Created:** 2026-06-07
**Author:** Kilo

---

## Perfection Loop — RED Phase

### Issue 1: Stop Override Allows Backward Move (BUG)

**Severity:** HIGH
**Location:** `engine.rs:2590-2617`
**Evidence:** Overnight log at 2:15 AM shows LINK stop moved $7.58 → $7.51 (DOWNWARD). Trailing stop had ratcheted to $7.58, then LLM's ADJUST_STOP wrote $7.51 — lower than the already-achieved trailing level.

**Root cause:** Line 2608 `pm_pos.stop_loss = new_stop;` is a blind assignment. No guard checks whether `new_stop` is above the current stop for LONG positions (or below for SHORT). The trailing stop mechanism correctly only moves UP for longs, but the ADJUST_STOP override bypasses this invariant.

**Fix:** Add directional guard:
```rust
// For LONG: new_stop must be ABOVE current stop (never move backward)
// For SHORT: new_stop must be BELOW current stop (never move backward)
let valid = match pos.side {
    Side::Long => new_stop > old_stop,
    Side::Short => new_stop < old_stop,
};
if !valid {
    warn!("Stop override rejected: {} — new ${:.4} is worse than current ${:.4}", pair, new_stop, old_stop);
    continue;
}
```

---

### Issue 2: Double LLM Evaluation (Waste)

**Severity:** MEDIUM
**Location:** `engine.rs` main loop
**Evidence:** Cycles 6-7 and 16-17 show two "BATCH EVALUATING" lines without a "BATCH COMPLETE" between them. The previous cycle's LLM call (which takes 100-160s) is still running when the next 5-minute tick fires.

**Root cause:** The 5-minute tick interval is shorter than the LLM response time. The engine doesn't check if a previous evaluation is still in-flight before starting a new one.

**Fix:** Add a `last_eval_started: Arc<Mutex<Instant>>` guard. If less than 3 minutes since last eval start, skip the current tick's evaluation phase.

---

### Issue 3: R:R Always 0.0 (Display Issue)

**Severity:** LOW
**Location:** LLM output
**Evidence:** Every single decision shows `R:0.0`. The LLM never fills the `risk_reward` field in its JSON output.

**Root cause:** The `output_format.md` prompt says `"risk_reward": 0.0` as the example value. MiMo copies the example literally.

**Fix:** Change the example in `output_format.md` from `"risk_reward": 0.0` to `"risk_reward": <calculated from entry/stop/tp, e.g. 2.5>`. Add an explicit instruction: "Calculate risk_reward from your entry, stop_loss, and take_profit_1 values. Do NOT leave at 0.0."

---

### Issue 4: No New Entries Despite Hunt Mode (Design Gap)

**Severity:** MEDIUM
**Location:** LLM decision-making
**Evidence:** 22 cycles, 176 evaluations, zero new entries. The LLM correctly identifies that weekend/low-volume conditions don't justify entries. Confidence on most pairs is 0-40%, below the 40% floor.

**Root cause:** This is the LLM being disciplined, not broken. However, the 40% confidence floor may be too aggressive for hunt mode. The LLM sees "ranging regime, low volume, weekend" and assigns 0-35% confidence — correctly filtering itself out.

**Options (not a code fix — strategic decision for user):**
- a) Lower confidence floor to 30% in hunt mode
- b) Add a "hunt mode override" instruction to the prompt: "In hunt mode with idle capital, lower your conviction threshold. A 35% confidence trade is better than 0% return on idle capital."
- c) Accept that the LLM is right — weekend low-volume conditions genuinely don't justify entries
- d) Reduce the pair set to only high-volatility pairs (PEPE, ARB) where moves are larger

**Recommendation:** Option (c) — the LLM is making correct decisions. The $13 USDC will deploy during US-EU overlap (13:00-17:00 UTC) when volume picks up. Don't override the LLM's discipline.

---

### Issue 5: Deferred from FID-072 (Not Approved for Deferral)

**Severity:** MEDIUM
**Items deferred without user permission:**

5a. **resolve_pair hardcoded to Arbitrum** (NF-02) — `mod.rs:662` calls `resolve_pair_on_chain(pair, side, 42161)`. Multi-chain infra built but not wired. Fix: pass `chain_id` from engine config.

5b. **amount_to_wei uses f64** (F-02) — `mod.rs:548-552`. Precision loss at scale. Fix: use `rust_decimal` crate. Not urgent at $30 but should be fixed.

5c. **0x fee logging** (NF-10) — Quote response fields (platform fee, integrator fee) ignored. Fix: log fees from quote response for cost tracking.

5d. **Gasless API not wired into main execute_swap** (NF-03) — `build_gasless_swap_tx()` exists in zero_x.rs but is only used as fallback in `close_position()`. The main `execute_swap()` path always uses standard Permit2. Fix: make gasless the primary path, standard as fallback.

---

## GREEN Phase — Proposed Fixes

| # | Issue | Fix | File | Lines | Risk |
|---|-------|-----|------|-------|------|
| 1 | Stop backward move | Add directional guard in stop override | engine.rs:2601-2608 | 10 | Low |
| 2 | Double evaluation | Add eval-in-progress guard | engine.rs main loop | 10 | Low |
| 3 | R:R always 0.0 | Update output_format.md example + instruction | output_format.md | 3 | Low |
| 4 | No new entries | Strategic decision — no code fix needed | — | 0 | — |
| 5a | resolve_pair hardcoded | Pass chain_id from config | mod.rs + engine.rs | 15 | Medium |
| 5b | amount_to_wei f64 | Use rust_decimal | mod.rs | 20 | Medium |
| 5c | 0x fee logging | Log fees from quote response | trader.rs | 5 | Low |
| 5d | Gasless as primary | Wire into execute_swap | trader.rs | 30 | Medium |

---

## AUDIT Phase — Five Questions

| # | Fix | All Cases | Scale | Attacker | 2 Years | Standard | Verdict |
|---|-----|-----------|-------|----------|---------|----------|---------|
| 1 | Stop guard | Yes — directional check for Long/Short | Yes | Yes | Yes | Yes | PASS |
| 2 | Eval guard | Yes — prevents overlapping calls | Yes | N/A | Yes | Yes | PASS |
| 3 | R:R example | Yes — prompt change only | Yes | N/A | Yes | Yes | PASS |
| 5a | resolve_pair | **RISK** — callers must pass correct chain | Yes | N/A | Yes | Yes | PASS |
| 5b | rust_decimal | **RISK** — new dependency | Yes | N/A | Yes | Yes | PASS |
| 5c | Fee logging | Yes | Yes | N/A | Yes | Yes | PASS |
| 5d | Gasless primary | **RISK** — changes execution path | Yes | Yes | Yes | Yes | PASS |

---

## SELF-CORRECT Phase

| Issue | Correction |
|-------|-----------|
| Issue 1: Stop guard should also check against the trailing stop level, not just the current stop | Need to track the highest trailing stop achieved for longs (or lowest for shorts) |
| Issue 2: 3-minute guard may be too aggressive — some cycles complete in 2 minutes | Use 2-minute guard instead |
| Issue 5d: Gasless as primary changes the entire execution path | Defer to separate FID — too risky to bundle with other fixes |

---

## COMPLETE Phase

**8 items. 6 code fixes, 1 prompt fix, 1 strategic decision.**

### Recommended Execution Order

| Phase | Items | Est. Lines |
|-------|-------|-----------|
| 1: Critical | 1 (stop guard) | 10 |
| 2: Important | 2 (eval guard), 3 (R:R prompt) | 13 |
| 3: Deferred from FID-072 | 5a (resolve_pair), 5c (fee logging) | 20 |
| 4: Lower priority | 5b (rust_decimal) | 20 |
| 5: Separate FID | 5d (gasless primary) | 30 |

### Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all pass
3. Manual trace: stop override with lower value → rejected with warning
4. Manual trace: overlapping eval → second eval skipped
5. Manual trace: LLM output with R:R > 0.0

---

## Status

- [x] RED: All issues traced to exact file:line
- [x] GREEN: 7 code fixes documented
- [x] AUDIT: All items pass Five Questions
- [x] SELF-CORRECT: 3 corrections applied
- [x] COMPLETE: Ready for implementation — **AWAITING USER APPROVAL**
