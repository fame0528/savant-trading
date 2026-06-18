# FID-194: Pre-flight Guard Against Phantom Management

**Filename:** `FID-2026-0617-194-preflight-guard.md`
**ID:** FID-2026-0617-194
**Severity:** high
**Status:** analyzed
**Created:** 2026-06-17 21:00 EST
**Author:** Vera
**Parent:** FID-193

---

## Summary

Step 1 of the FID-193 state-sync fix. Before the engine executes an `AdjustStop` or `Close` action, verify the executor actually has a position for that pair+side. If not, downgrade to `Pass` with `override_source="no_position_to_manage"`. This is a band-aid that prevents the worst symptom (phantom management) but doesn't fix the underlying hallucination.

---

## Why This Matters

Observed in `logs/terminal/next-server (v16.2.7).txt`:

**Cycle 1 (8:03 PM):** LLM/jury emits `BUY ENA/USD`. Executor calls 0x. Spread check fails at `trader.rs:1258-1270` (no market price for the destination token). Order rejected. No position opened. `portfolio.positions()` correctly stays empty.

**Cycle 3 (8:17 PM):** LLM sees its own prior `BUY ENA` in the decision log context. Doesn't know the executor rejected. Outputs `AdjustStop` for "existing position showing profit." The jury agrees. Engine would try to AdjustStop a position that doesn't exist.

**The pre-flight guard catches this at the engine boundary.** When the parser returns `AdjustStop` and the executor has no matching position, downgrade to `Pass` with an override source. The LLM gets feedback (via `override_source` field) that its management decision was invalid because no position exists.

---

## Files Changed

- `src/engine/mod.rs` — add the guard after `parse_decision` returns, before the executor call

---

## Implementation

**Location:** In `src/engine/mod.rs`, after the `parse_decision` call returns and before the executor call. The exact line number depends on which parse site (there are multiple per cycle — batch, AdjustStop, Close). The guard goes in the post-parse validation block, common to all sites.

```rust
// FID-194: Pre-flight guard against phantom management.
// If the LLM/jury says AdjustStop/Close but no position exists
// for this pair+side in the executor's state, downgrade to Pass.
// This catches the case where the LLM is managing a position
// that was never opened (e.g., spread-rejected swap).
if matches!(decision.action, TradeAction::AdjustStop | TradeAction::Close) {
    let has_position = if let Some(ref ex) = executor {
        ex.open_positions().iter().any(|p| p.pair == decision.pair)
    } else {
        portfolio.positions().values().any(|p| p.pair == decision.pair)
    };
    if !has_position {
        info!(
            "FID-194: {} for {} but no position exists. Downgrading to Pass (override: no_position_to_manage).",
            match decision.action { TradeAction::AdjustStop => "AdjustStop", TradeAction::Close => "Close", _ => "?" },
            decision.pair
        );
        decision.action = TradeAction::Pass;
        decision.override_source = Some("no_position_to_manage".to_string());
    }
}
```

---

## Verification

### Unit test
```rust
#[test]
fn preflight_guard_downgrades_adjuststop_for_phantom_position() {
    let mut decision = make_test_decision();
    decision.action = TradeAction::AdjustStop;
    decision.pair = "ENA/USD".to_string();
    let executor_positions: Vec<Position> = vec![]; // no ENA position
    // ... assert decision is downgraded to Pass
}
```

### Manual test
1. Inject a phantom `AdjustStop` for a pair with no position (via direct decision log write)
2. Run cycle
3. Verify decision is downgraded to `Pass` with `override_source="no_position_to_manage"`
4. Verify no executor call was made for the phantom AdjustStop

### Integration test
Run paper mode for 1h with FID-194 active. Verify:
- 0 phantom AdjustStop executions
- 0 phantom Close executions
- All AdjustStop/Close actions correspond to real positions
- Override source is logged for any downgrades

---

## Risk

**Low.** This is a pure guard — it downgrades invalid actions, never allows invalid ones. Existing behavior is unchanged for valid actions.

---

## Files Changed Summary

- `src/engine/mod.rs`: 1 new function block, ~15 lines
- 1 unit test
- 0 documentation changes (FID-193 documents the parent context)
