# FID-195: Executor Reports Fill/No-Fill to Decision Log

**Filename:** `FID-2026-0617-195-executor-reports-fill.md`
**ID:** FID-2026-0617-195
**Severity:** high
**Status:** closed
**Resolution:** Shipped in v0.14.7 (commit ef606667, 2026-06-17). Executor now reports fill/reject status back to the decision log so the LLM's next cycle sees whether each action actually executed. Execution outcomes included in LLM context window. Child of FID-193. Archived 2026-06-19 per FID-211 Stage 2 Item 6 cleanup.
**Created:** 2026-06-17 21:00 EST
**Author:** Vera
**Parent:** FID-193

---

## Summary

Step 2 of the FID-193 state-sync fix. When the executor rejects a swap, write a `REJECTED` entry to the decision log so the LLM knows its prior `BUY` didn't fill. Currently the executor returns `Err(ExecutionError::Other(...))` and the engine silently drops it. The decision log shows the LLM's `BUY` as if it were a fill. The LLM doesn't know.

---

## Why This Matters

The decision log is the LLM's only memory of "what I did last time." When the LLM sees a `BUY` from cycle 1 in its context for cycle 3, it assumes the position is open. If the executor rejected the swap and the LLM never finds out, the LLM continues to act on a false memory.

**The fix:** every executor outcome (fill, reject, error) is reflected in the decision log with a `status` field. The LLM context filters out rejected decisions so the LLM doesn't act on false memories.

---

## Files Changed

- `src/agent/decision_log.rs` — add `TradeStatus` enum, `status` field on `DecisionEntry`
- `src/engine/mod.rs` — change `executor.open_position(...).await?` to `match` and write `REJECTED` on failure
- `src/agent/context_builder.rs` — filter decision log to exclude `Rejected` from LLM context

---

## Implementation

### 1. New `TradeStatus` enum in `decision_log.rs`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TradeStatus {
    /// Decision made, executor outcome unknown (default for new entries).
    Pending,
    /// Executor filled the order.
    Filled,
    /// Executor rejected the order (spread, dust, slippage, RPC error).
    Rejected,
    /// Decision expired (LLM never had time to act on it).
    Expired,
}

impl Default for TradeStatus {
    fn default() -> Self { TradeStatus::Pending }
}
```

### 2. Add `status` and `rejection_reason` fields to `DecisionEntry`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEntry {
    // ... existing fields ...
    #[serde(default)]
    pub status: TradeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}
```

### 3. New `update_status` method

```rust
impl DecisionLog {
    /// Update the most recent entry for a pair with execution outcome.
    pub fn update_status(&mut self, pair: &str, status: TradeStatus, reason: Option<String>) {
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| e.pair == pair) {
            entry.status = status;
            if status == TradeStatus::Rejected {
                entry.rejection_reason = reason;
            }
            self.flush();
        }
    }
}
```

### 4. Engine change: handle executor Err

**File:** `src/engine/mod.rs`. There are TWO executor call paths and ONE portfolio fallback path. All three need `update_status` on both `Ok` and `Err`.

**4a. `ex.place_order(...)` call site at line 4133-4153 (executor, with timeout):**

```rust
// At line 4165-4296, the match arm becomes:
match order {
    Ok(_) => {
        // ... existing position creation at lines 4167-4276 ...
        // FID-195: Mark the decision as Filled.
        decision_log.update_status(&decision.pair, TradeStatus::Filled, None);
    }
    Err(e) => {
        // Existing failure_tracker call at line 4280-4290.
        // FID-195: Mark the decision as Rejected AND write a REJECTED entry
        // so the LLM sees explicit feedback in its next-cycle context.
        let reason = e.to_string();
        decision_log.append(DecisionEntry {
            timestamp: Utc::now().to_rfc3339(),
            pair: decision.pair.clone(),
            action: "REJECTED".to_string(),
            confidence: decision.confidence,
            risk_reward: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            reasoning: format!(
                "Order rejected: {}. LLM/jury decided {} but executor declined. Check spread/liquidity/RPC.",
                reason,
                format!("{:?}", decision.action)
            ),
            conviction_score: decision.conviction_score,
            regime_label: format!("{:?}", decision.regime_label),
            trigger_strong: decision.trigger_weights.strong as u32,
            trigger_moderate: decision.trigger_weights.moderate as u32,
            trigger_weak: decision.trigger_weights.weak as u32,
            override_source: Some("executor_rejected".to_string()),
            status: TradeStatus::Rejected,
            rejection_reason: Some(reason.clone()),
            outcome: None,
        });
        // Also update the most recent same-pair entry (the BUY) to Rejected.
        decision_log.update_status(&decision.pair, TradeStatus::Rejected, Some(reason.clone()));
        shared.log_activity(ActivityLevel::Warning, Some("EXEC"), &decision.pair, &format!("REJECTED: {}", reason)).await;
    }
}
```

**4b. `portfolio.place_order(...)` fallback call at line 4154-4163 (dry mode, no executor):**

This path doesn't have executor errors in the same way — portfolio trades are simulated. But errors can still occur (sizer rejection at lines 4298-4325). For dry mode, write the rejection to the log similarly.

**4c. `close_position` and `adjust_stop` at lines 3520, 3537, 4698, 4781:**

Each of these call sites needs a `decision_log.update_status(&pair, TradeStatus::Filled, None)` on Ok and a `decision_log.append(DecisionEntry { action: "REJECTED", ... })` on Err. Same pattern as above.

### 5. Context builder changes (2 functions)

**File:** `src/agent/context_builder.rs` and `src/agent/decision_log.rs`.

**5a. Filter in `decision_log.context_for_pair()` (line 99-148):**

```rust
// FID-195: Skip REJECTED entries — the LLM shouldn't see them as
// still-active decisions. The LLM learns from rejections via
// the separate "## Execution Outcomes" section (5b).
let same: Vec<&DecisionEntry> = self
    .entries
    .iter()
    .rev()
    .filter(|e| e.pair == pair)
    .filter(|e| e.status != TradeStatus::Rejected)  // NEW
    .take(max_same)
    .collect();
let cross: Vec<&DecisionEntry> = self
    .entries
    .iter()
    .rev()
    .filter(|e| e.pair != pair && e.outcome.is_some())
    .filter(|e| e.status != TradeStatus::Rejected)  // NEW
    .take(max_cross)
    .collect();
```

**5b. New `format_execution_outcomes()` function for the LLM context:**

The LLM needs explicit feedback on rejected orders, not just absent. Add a new section to the LLM context that shows execution outcomes clearly:

```rust
// In src/agent/context_builder.rs, after the "## Recent Decision Log" section:

/// FID-195: Format execution outcomes for LLM context.
/// Shows REJECTED entries explicitly so the LLM knows the position didn't open.
/// Without this, the LLM would infer from absence of a position that the BUY
/// either filled (and was sold) or is still open. Both are wrong.
pub fn format_execution_outcomes(log: &DecisionLog, max_entries: usize) -> String {
    let recent: Vec<&DecisionEntry> = log
        .entries
        .iter()
        .rev()
        .filter(|e| e.status != TradeStatus::Pending) // only finalized
        .take(max_entries)
        .collect();
    if recent.is_empty() {
        return String::new();
    }
    let mut msg = String::from("\n## Execution Outcomes (Fills & Rejections)\n");
    for entry in &recent {
        match entry.status {
            TradeStatus::Filled => {
                msg.push_str(&format!(
                    "  ✓ FILLED: {} {} @ {} (qty via context)\n",
                    entry.pair, entry.action, entry.take_profit
                ));
            }
            TradeStatus::Rejected => {
                msg.push_str(&format!(
                    "  ✗ REJECTED: {} {} — {} (NO POSITION OPENED on chain)\n",
                    entry.pair,
                    entry.action,
                    entry.rejection_reason.as_deref().unwrap_or("unknown")
                ));
            }
            _ => {} // Pending/Expired are filtered
        }
    }
    msg
}
```

**5c. Call site in `src/engine/mod.rs` (around line 2217):**

```rust
// FID-195: Include execution outcomes in LLM context.
let execution_outcomes = format_execution_outcomes(&decision_log, 5);
let decision_log_ctx = if execution_outcomes.is_empty() {
    decision_log.context_for_pair(pair, 3, 2)
} else {
    format!(
        "{}{}",
        execution_outcomes,
        decision_log.context_for_pair(pair, 3, 2)
    )
};
```

### Decision log size limit

The `context_for_pair(pair, 3, 2)` at line 2191 limits LLM context to 3 same-pair + 2 cross-pair entries. This bounds memory growth. The 5-recent entries for Execution Outcomes is a separate limit — they only show finalized outcomes (Filled/Rejected) so the context stays compact.

### Schema migration

Old `decision_log.json` entries (pre-FID-195) won't have `status` or `rejection_reason` fields. With `#[serde(default)]`, status defaults to `TradeStatus::Pending` and `rejection_reason` defaults to `None`. This is the correct default:
- **Pending** means "we don't know the outcome." For legacy entries, this is true — the executor may have filled or rejected, we can't reconstruct.
- The LLM treats legacy Pending entries like current Pending entries. It doesn't act on them; it just sees the decision was made.
- Once FID-196 reconciliation runs, it will set the status based on on-chain reality.

### 5d. Jury position context (added in Perfection Loop)

**File:** `src/agent/jury/pool.rs` line 256.

**Problem:** The jury's `evaluate` takes only `user_message: &str` (the LLM's batch response). The jury reads the LLM's text and votes on it — so if the LLM hallucinates a position, the jury inherits the hallucination.

**Fix:** Give the jury the executor's actual positions as part of the user message. Each juror will see the LLM response + the actual on-chain state, and can independently verify.

```rust
// In pool.rs:256
pub async fn evaluate(
    &mut self,
    user_message: &str,
    regime: MarketRegime,
    executor: Option<&dyn ExecutionEngine>,  // NEW
) -> JuryResult {
    // FID-195: Prepend executor's open positions to the user message.
    // This gives the jury independent verification of position state,
    // not just whatever the LLM said.
    let position_context = if let Some(ex) = executor {
        let positions = ex.open_positions();
        if positions.is_empty() {
            String::from("\n\n## Executor State: No open positions.")
        } else {
            let mut ctx = String::from("\n\n## Executor State: Open positions on chain:\n");
            for p in positions {
                ctx.push_str(&format!(
                    "  - {} {} @ {} (qty {:.4})\n",
                    p.pair, p.side, p.entry_price, p.quantity
                ));
            }
            ctx.push_str("The LLM response above may or may not match this. If the LLM is managing a position NOT listed here, it is hallucinating. Veto.\n");
            ctx
        }
    } else {
        String::new()
    };
    
    let user_with_context = format!("{}{}", user_message, position_context);
    // ... use user_with_context for juror API calls ...
}
```

**Impact:** When the LLM says "AdjustStop ENA/USD for existing position" and there's no ENA on chain, the jury sees the executor state and votes VETO. The judge's consensus shifts to Pass, not Buy/Sell/AdjustStop.

### 5e. Jury override logging

When the jury overrides the LLM (cycle 1 ENA case), the LLM's verdict is lost. The decision log should record both the LLM's verdict AND the override reason so the LLM learns from jury corrections.

**File:** `src/agent/jury/judge.rs` — when the judge synthesizes a final verdict different from the LLM batch verdict, write an entry to the decision log:

```rust
// In judge.rs, after judgment is computed
if judgment.decision.action != llm_batch_action {
    decision_log.append(DecisionEntry {
        timestamp: Utc::now().to_rfc3339(),
        pair: judgment.pair.clone(),
        action: "JURY_OVERRIDE".to_string(),
        confidence: judgment.consensus_strength,
        reasoning: format!(
            "Jury overrode LLM verdict. LLM said {} but jury voted {} (consensus {:.0}%)",
            llm_batch_action, judgment.decision.action, judgment.consensus_strength * 100.0
        ),
        override_source: Some(format!("jury_override_{}", judgment.consensus_strength)),
        status: TradeStatus::Pending,
        ..Default::default()
    });
}
```

### CALL-GRAPH REACHABILITY (Law 4, FID-151)

After implementation:
```bash
grep -rn "TradeStatus::" src/ --include='*.rs'
# Expected: definition (decision_log.rs) + uses in engine/mod.rs, context_builder.rs, jury/

grep -rn "update_status" src/ --include='*.rs'
# Expected: definition (decision_log.rs) + 4+ call sites (open_position, close_position, adjust_stop, executor Err, reconciliation)

grep -rn "format_execution_outcomes" src/ --include='*.rs'
# Expected: definition (context_builder.rs) + 1 call site (engine/mod.rs:2217)
```

If a path is unwired (e.g., close_position's Ok branch doesn't call update_status), the FID is rejected.

---

## Verification

### Unit tests
```rust
#[test]
fn update_status_finds_recent_entry() {
    let mut log = DecisionLog::open("test.json", 100);
    log.append(sample_entry("ENA/USD", "BUY"));
    log.update_status("ENA/USD", TradeStatus::Rejected, Some("spread exceeded".to_string()));
    let entry = log.entries.last().unwrap();
    assert_eq!(entry.status, TradeStatus::Rejected);
    assert_eq!(entry.rejection_reason, Some("spread exceeded".to_string()));
}

#[test]
fn update_status_skips_old_entries() {
    let mut log = DecisionLog::open("test.json", 100);
    log.append(sample_entry("ENA/USD", "BUY"));
    log.append(sample_entry("ENA/USD", "PASS"));  // newer entry
    log.update_status("ENA/USD", TradeStatus::Rejected, Some("x".to_string()));
    // The most recent entry (PASS) shouldn't be changed
    let entry = log.entries.last().unwrap();
    assert_ne!(entry.status, TradeStatus::Rejected);
}

#[test]
fn context_for_pair_filters_rejected() {
    let mut log = DecisionLog::open("test.json", 100);
    log.append(sample_entry("ENA/USD", "BUY"));
    log.entries.last_mut().unwrap().status = TradeStatus::Rejected;
    log.append(sample_entry("ENA/USD", "PASS"));
    let ctx = log.context_for_pair("ENA/USD", 3, 2);
    // Should NOT contain "REJECTED" or the rejected entry
    assert!(!ctx.contains("REJECTED"));
}

#[test]
fn format_execution_outcomes_shows_filled_and_rejected() {
    let mut log = DecisionLog::open("test.json", 100);
    let mut entry = sample_entry("ENA/USD", "BUY");
    entry.status = TradeStatus::Filled;
    log.append(entry);
    let mut entry2 = sample_entry("BTC/USD", "BUY");
    entry2.status = TradeStatus::Rejected;
    entry2.rejection_reason = Some("spread".to_string());
    log.append(entry2);
    
    let out = format_execution_outcomes(&log, 5);
    assert!(out.contains("FILLED: ENA/USD"));
    assert!(out.contains("REJECTED: BTC/USD"));
    assert!(out.contains("spread"));
    assert!(out.contains("NO POSITION OPENED"));
}

#[test]
fn jury_receives_executor_position_context() {
    // Setup: executor with one position (BTC)
    let mut executor = MockExecutor::new(vec![make_btc_position()]);
    let mut pool = JuryPool::new(...);
    
    // LLM says "AdjustStop ETH" (no ETH on chain)
    let llm_msg = "ETH: AdjustStop existing position...";
    
    let result = pool.evaluate(llm_msg, MarketRegime::Ranging, Some(&executor)).await;
    
    // The user_message passed to jurors should include executor state showing
    // no ETH, which should bias jurors toward Pass
    assert!(result.user_message_with_context.contains("BTC/USD"));
    assert!(!result.user_message_with_context.contains("ETH"));
}
```

### Manual test
1. Trigger a spread rejection (use a token with no 0x price feed)
2. Verify `REJECTED` entry appears in `data/decision_log.json` with reason
3. Run next cycle
4. Verify LLM context's "Recent Decision Log" does NOT include the rejected BUY
5. Verify LLM context's "## Execution Outcomes" section DOES include the rejected BUY with reason
6. Verify LLM does not emit AdjustStop/Close for the rejected pair in subsequent cycles
7. Verify jury's `user_message_with_context` includes executor positions

### Integration test
1. Run paper mode for 1h
2. Count REJECTED entries in decision log
3. Count executor error logs
4. Verify they match (every executor error → one REJECTED entry)
5. Verify LLM's "Recent Decision Log" never shows rejected decisions
6. Verify "## Execution Outcomes" section shows the same count of Filled+Rejected
7. Verify jury is vetoing phantom management decisions (count jury vetoes > 0)

---

## Risk

**Low.** This is additive — new status field, new REJECTED entry, new filter. Existing behavior unchanged for filled orders.

**Caveat:** The `status` field is a breaking change to the `data/decision_log.json` schema. Old entries (from before this FID) won't have a status. The `#[serde(default)]` ensures they default to `Pending` (which is what we want for legacy entries — they may or may not be filled, we don't know). This is correct behavior: the LLM sees legacy entries as potentially-pending rather than filled.

---

## Files Changed Summary

- `src/agent/decision_log.rs`: new `TradeStatus` enum, 2 new fields on `DecisionEntry`, 1 new method
- `src/engine/mod.rs`: 1 match block replacing `?` operator (~30 lines added)
- `src/agent/context_builder.rs`: 1 filter function (~10 lines)
- 2 unit tests
- 0 documentation changes (FID-193 documents parent context)
