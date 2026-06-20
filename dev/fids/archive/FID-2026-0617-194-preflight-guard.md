# FID-194: Pre-flight Guard Against Phantom Management

**Filename:** `FID-2026-0617-194-preflight-guard.md`
**ID:** FID-2026-0617-194
**Severity:** high
**Status:** closed
**Resolution:** Shipped in v0.14.7 (commit b207b9e8, 2026-06-17). The pre-flight guard now cross-references open positions before the LLM can emit Adjust/Close actions, rejecting phantom management attempts at the engine level. Child of FID-193 (state sync). Archived 2026-06-19 per FID-211 Stage 2 Item 6 cleanup.
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

**Call site:** `src/engine/mod.rs:2844` is the only `parse_decision` call in the engine (verified by `grep -n parse_decision src/engine/mod.rs`). The guard goes immediately after this single call returns, before the `decision_log.append(...)` on the same line block.

**Function:** extract the guard into a reusable function so the Perfection Loop can verify call-graph reachability cleanly. Per ECHO Law 13, one function = one truth.

```rust
// FID-194: Pre-flight guard against phantom management.
// Extracted to a function so it can be unit-tested in isolation
// and audited via grep for callers.
pub fn apply_pre_flight_guard(
    decision: &mut TradeDecision,
    executor: Option<&dyn ExecutionEngine>,
    portfolio: &PortfolioManager,
) {
    // Only AdjustStop/Close actions need this check. BUY/SELL/PASS
    // either open a new position (already validated by FID-195) or
    // do nothing.
    if !matches!(decision.action, TradeAction::AdjustStop | TradeAction::Close) {
        return;
    }
    // The executor is the source of truth for live mode. In dry
    // mode (no executor), fall back to the in-memory portfolio.
    let has_position = if let Some(ex) = executor {
        ex.open_positions().iter().any(|p| p.pair == decision.pair)
    } else {
        portfolio.positions().values().any(|p| p.pair == decision.pair)
    };
    if !has_position {
        let action_label = match decision.action {
            TradeAction::AdjustStop => "AdjustStop",
            TradeAction::Close => "Close",
            _ => "unknown",
        };
        info!(
            "FID-194: {} for {} but no position exists. Downgrading to Pass (override: no_position_to_manage).",
            action_label, decision.pair
        );
        decision.action = TradeAction::Pass;
        decision.override_source = Some("no_position_to_manage".to_string());
    }
}
```

**Call site in `engine/mod.rs:2844`:**
```rust
match savant_trading::agent::decision_parser::parse_decision(...) {
    Ok(mut decision) => {
        // FID-194: Pre-flight guard against phantom management.
        apply_pre_flight_guard(&mut decision, executor.as_deref(), &portfolio);
        // ... existing decision_log.append(decision) ...
    }
    Err(e) => { ... }
}
```

### Override feedback to LLM

When the guard downgrades an action, the `override_source` field is set to `"no_position_to_manage"`. The next cycle, the LLM's `decision_log_context` includes this entry. The LLM can see "AdjustStop ENA 8:17 PM → override: no_position_to_manage" and learn that its management was invalid. This is the feedback loop.

### CALL-GRAPH REACHABILITY (Law 4, FID-151)

After implementation, grep verification:
```bash
grep -rn "apply_pre_flight_guard" src/ --include='*.rs'
# Expected: at least 1 definition + 1 call site (engine/mod.rs:2844)
```

If the grep shows 0 callers (function defined but never called), the FID is rejected from `fixed` status and must re-enter GREEN.

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
    let executor = MockExecutor::new(executor_positions);
    let portfolio = make_test_portfolio(); // also no ENA
    
    apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);
    
    assert_eq!(decision.action, TradeAction::Pass);
    assert_eq!(decision.override_source, Some("no_position_to_manage".to_string()));
}

#[test]
fn preflight_guard_keeps_adjuststop_when_position_exists() {
    let mut decision = make_test_decision();
    decision.action = TradeAction::AdjustStop;
    decision.pair = "ENA/USD".to_string();
    let ena_pos = make_ena_position();
    let executor = MockExecutor::new(vec![ena_pos]);
    let portfolio = make_test_portfolio_with_ena();
    
    apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);
    
    // No downgrade because ENA position exists
    assert_eq!(decision.action, TradeAction::AdjustStop);
    assert!(decision.override_source.is_none());
}

#[test]
fn preflight_guard_ignores_buy_action() {
    let mut decision = make_test_decision();
    decision.action = TradeAction::Buy;
    decision.pair = "ENA/USD".to_string();
    let executor = MockExecutor::new(vec![]); // no ENA, but BUY is OK
    
    apply_pre_flight_guard(&mut decision, Some(&executor), &make_test_portfolio());
    
    // BUY is not downgraded (this guard is only for management)
    assert_eq!(decision.action, TradeAction::Buy);
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

### CALL-GRAPH REACHABILITY (Law 4, FID-151)

After implementation:
```bash
grep -rn "apply_pre_flight_guard" src/ --include='*.rs'
# Expected: 1 definition (in a new utility module) + 1 call site (engine/mod.rs:2844)
```

---

## Risk

**Low.** This is a pure guard — it downgrades invalid actions, never allows invalid ones. Existing behavior is unchanged for valid actions.

---

## Files Changed Summary

- `src/engine/mod.rs`: 1 new function block, ~15 lines
- 1 unit test
- 0 documentation changes (FID-193 documents the parent context)
