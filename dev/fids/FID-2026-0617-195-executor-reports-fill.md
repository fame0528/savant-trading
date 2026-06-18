# FID-195: Executor Reports Fill/No-Fill to Decision Log

**Filename:** `FID-2026-0617-195-executor-reports-fill.md`
**ID:** FID-2026-0617-195
**Severity:** high
**Status:** analyzed
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

**File:** `src/engine/mod.rs` around line 4042 (the executor call site).

```rust
// OLD:
let tx_hash = executor.open_position(...).await?;
// ... insert position ...

// NEW:
match executor.open_position(...).await {
    Ok(tx_hash) => {
        // Existing success path
        portfolio.positions_mut().insert(...);
        // Mark the decision as Filled
        decision_log.update_status(&decision.pair, TradeStatus::Filled, None);
    }
    Err(e) => {
        // Mark the decision as Rejected
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
                "Order rejected: {}. LLM/jury decided {} but executor declined. Check spread/liquidity.",
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
        shared.log_activity(ActivityLevel::Warning, Some("EXEC"), &decision.pair, &format!("REJECTED: {}", reason)).await;
    }
}
```

### 5. Context builder filter

**File:** `src/agent/context_builder.rs` around line 522.

```rust
// FID-195: Filter rejected decisions from LLM context.
// The LLM should not see its own prior BUY as still active if the executor
// rejected it. Filter to only show Filled and Pending entries.
if let Some(ref log_ctx) = ctx.decision_log_context {
    if !log_ctx.is_empty() {
        let filtered = filter_decision_log(log_ctx);
        if !filtered.is_empty() {
            msg.push_str("\n## Recent Decision Log\n");
            msg.push_str(&filtered);
        }
    }
}

fn filter_decision_log(raw: &str) -> String {
    // Parse each line, filter out lines containing "[REJECTED]"
    raw.lines()
        .filter(|line| !line.contains("[REJECTED]") && !line.contains("REJECTED"))
        .collect::<Vec<_>>()
        .join("\n")
}
```

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
```

### Manual test
1. Trigger a spread rejection (use a token with no 0x price feed)
2. Verify `REJECTED` entry appears in `data/decision_log.json` with reason
3. Run next cycle
4. Verify LLM context's "Recent Decision Log" does NOT include the rejected BUY
5. Verify LLM does not emit AdjustStop/Close for the rejected pair

### Integration test
1. Run paper mode for 1h
2. Count REJECTED entries in decision log
3. Count executor error logs
4. Verify they match (every executor error → one REJECTED entry)
5. Verify LLM's "Recent Decision Log" never shows rejected decisions

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
