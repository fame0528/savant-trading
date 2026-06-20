# FID-196: Per-Cycle Wallet/Decision Reconciliation

**Filename:** `FID-2026-0617-196-cycle-reconciliation.md`
**ID:** FID-2026-0617-196
**Severity:** critical
**Status:** closed
**Resolution:** Shipped in v0.14.7 (commit 1fda8db5, 2026-06-17). Per-cycle reconciliation with USDC balance + safety halt + telemetry now runs at the start of every engine cycle. Detects on-chain vs in-memory divergence and halts trading until the gap is reconciled. Child of FID-193. Archived 2026-06-19 per FID-211 Stage 2 Item 6 cleanup.
**Created:** 2026-06-17 21:00 EST
**Author:** Vera
**Parent:** FID-193

---

## Summary

Step 3 of the FID-193 state-sync fix. Every cycle, after execution, reconcile on-chain positions vs in-memory positions. If a position is in memory but not on-chain, remove it from memory AND mark the corresponding decision_log entry as `Rejected`. If a position is on-chain but not in memory (e.g., a manually-sent tx or a crash recovery), add it. The chain becomes the single source of truth.

---

## Why This Matters

Steps 1 (pre-flight guard) and 2 (executor reports) prevent new phantom positions from being acted on. But they don't catch:
- Positions that were opened correctly but later orphaned (e.g., sold externally)
- Positions added by manual tx (e.g., user sent tokens to their own wallet)
- Drift from RPC latency or stale cache

The reconciliation is the periodic check that ensures the in-memory state is consistent with the chain. It's already partially implemented (FID-147) for USDC balance divergence, but it only halts the engine on divergence — it doesn't fix the in-memory state.

**The fix:** extend reconciliation to also reconcile position lists and mutate the portfolio + decision log when divergence is detected.

---

## Files Changed

- `src/execution/reconciliation.rs` — add `apply_to_portfolio()` function that mutates state based on on-chain reality
- `src/engine/mod.rs` — call `apply_reconciliation()` after each cycle's execution phase

---

## Implementation

### 1. New `apply_to_portfolio` function in `reconciliation.rs`

Per ECHO Law 13, the existing `reconcile_wallet_state` function already does USDC divergence detection (line 92-234) — but it only halts. The new function extends it to also fix position drift. **Same function, extended responsibility** (per Law 13, utility-first).

```rust
/// FID-196: Apply reconciliation to portfolio + decision log.
/// Mutates in-memory state to match on-chain reality. Catches:
/// - Phantom positions: in memory but not on chain (executor rejected)
/// - Orphan positions: on chain but not in memory (crash recovery, manual tx)
/// - USDC balance drift: executor's balance vs portfolio's balance
///
/// Safety: if divergence exceeds `safety_halt_threshold_pct` (config-driven,
/// default 50%), the function does NOT mutate state. It returns a halt signal.
/// A 50% divergence indicates a serious bug, not routine drift.
///
/// Returns ReconciliationApplyReport with details for logging.
pub fn apply_to_portfolio(
    config: &ReconciliationConfig,
    portfolio: &mut PortfolioManager,
    decision_log: &mut DecisionLog,
    executor: &dyn ExecutionEngine,
    shared_data: &mut SharedEngineData,
) -> ReconciliationApplyReport {
    let mut report = ReconciliationApplyReport::default();
    
    // Step 0: RPC failure check. If executor.open_positions() returns empty
    // because of RPC failure (not because there are no positions), we can't
    // distinguish. Mitigate by checking balance: if balance is also empty,
    // it's likely RPC failure. If balance is non-zero but positions is empty,
    // the user has no positions — clear phantoms.
    let on_chain_positions = executor.open_positions();
    if on_chain_positions.is_empty() {
        // Cross-check: is the wallet actually empty, or did RPC fail?
        // We use the reconciliation's USDC balance query as the tiebreaker.
        // If RPC fails, reconcile_wallet_state returns rpc_failure=true and
        // we abort apply_to_portfolio to be safe.
        let on_chain_usdc = match query_token_balance(...).await {
            Ok(b) => b,
            Err(_) => {
                report.rpc_failure = true;
                return report;
            }
        };
        if on_chain_usdc < 0.01 {
            // Truly empty wallet, no phantoms to clear
            return report;
        }
        // Non-empty wallet but no positions: phantoms exist, clear them
    }
    
    // Step 1: Position reconciliation (phantom + orphan detection)
    let on_chain_pairs: std::collections::HashSet<String> = on_chain_positions
        .iter()
        .map(|p| p.pair.clone())
        .collect();
    
    let in_memory: Vec<(String, Position)> = portfolio.positions()
        .iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    
    // Step 2: Safety check on total portfolio value before mutating.
    // If divergence is > safety_halt_threshold_pct, abort.
    let total_portfolio_value = portfolio.account().balance + portfolio.account().equity;
    let phantom_value: f64 = in_memory.iter()
        .filter(|(_, p)| !on_chain_pairs.contains(&p.pair))
        .map(|(_, p)| p.quantity * p.current_price)
        .sum();
    if total_portfolio_value > 0.0 
        && (phantom_value / total_portfolio_value) > config.safety_halt_threshold_pct
    {
        report.halt_recommended = true;
        report.halt_reason = Some(format!(
            "Phantom value ${:.2} is {:.1}% of portfolio ${:.2}, exceeds safety threshold {:.1}%. Likely bug, not routine drift.",
            phantom_value,
            (phantom_value / total_portfolio_value) * 100.0,
            total_portfolio_value,
            config.safety_halt_threshold_pct * 100.0
        ));
        return report;  // Don't mutate. Caller will halt the engine.
    }
    
    // Step 3: Clear phantoms (in memory but not on chain)
    for (id, pos) in &in_memory {
        if !on_chain_pairs.contains(&pos.pair) {
            warn!("FID-196: Phantom position detected: {} (id={}, entry={:.4}) — in memory but not on chain. Removing.",
                pos.pair, id, pos.entry_price);
            report.phantom_cleared.push(pos.pair.clone());
            portfolio.positions_mut().remove(id);
            decision_log.update_status(&pos.pair, TradeStatus::Rejected, Some("phantom: not on chain".to_string()));
        }
    }
    
    // Step 4: Add orphans (on chain but not in memory)
    for on_chain_pos in &on_chain_positions {
        let already_in_memory = portfolio.positions().values().any(|p| p.pair == on_chain_pos.pair);
        if !already_in_memory {
            warn!("FID-196: Orphan on-chain position: {} (entry={:.4}) — on chain but not in memory. Adding.",
                on_chain_pos.pair, on_chain_pos.entry_price);
            report.orphan_added.push(on_chain_pos.pair.clone());
            portfolio.positions_mut().insert(on_chain_pos.id.clone(), on_chain_pos.clone());
            decision_log.update_status(&on_chain_pos.pair, TradeStatus::Filled, None);
        }
    }
    
    // Step 5: USDC balance reconciliation
    // Compare executor's USDC balance to portfolio's balance. If they diverge
    // by more than threshold, set portfolio balance to executor balance.
    let executor_usdc = executor.balance();
    let portfolio_usdc = portfolio.account().balance;
    let usdc_div = (executor_usdc - portfolio_usdc).abs();
    if usdc_div > config.divergence_threshold_usd {
        warn!("FID-196: USDC balance divergence ${:.4}: portfolio=${:.4} executor=${:.4}. Correcting portfolio.",
            usdc_div, portfolio_usdc, executor_usdc);
        report.usdc_divergence_corrected = executor_usdc - portfolio_usdc;
        portfolio.set_balance(executor_usdc);
    }
    
    // Step 6: Update account state + emit telemetry
    portfolio.account_mut().open_positions = portfolio.positions().len();
    shared_data.record_telemetry("reconciliation", &report);
    
    report
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReconciliationApplyReport {
    pub phantom_cleared: Vec<String>,
    pub orphan_added: Vec<String>,
    pub usdc_divergence_corrected: f64,
    pub rpc_failure: bool,
    pub halt_recommended: bool,
    pub halt_reason: Option<String>,
    /// FID-196 telemetry: rate of phantoms cleared per cycle (rolling 10-cycle avg)
    pub phantom_rate_rolling: f64,
    /// FID-196 telemetry: rate of orphans added per cycle (rolling 10-cycle avg)
    pub orphan_rate_rolling: f64,
}
```

### 1a. Extended ReconciliationConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationConfig {
    pub chain_id: u64,
    pub wallet_address: String,
    pub rpc_url: String,
    pub divergence_threshold_usd: f64,
    pub divergence_threshold_pct: f64,
    pub interval_cycles: u32,
    /// FID-196: Safety halt threshold. If phantom value exceeds this % of
    /// total portfolio, apply_to_portfolio returns halt_recommended=true
    /// without mutating state. Default: 0.50 (50%).
    pub safety_halt_threshold_pct: f64,
}
```

### 1b. Telemetry

Per Spencer's directive ("include all suggestions"), add observability. New function in `SharedEngineData`:

```rust
// In src/core/shared.rs
pub fn record_telemetry(&mut self, event: &str, data: &serde_json::Value) {
    let entry = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "event": event,
        "data": data,
    });
    // Append to data/reconciliation_telemetry.jsonl
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/reconciliation_telemetry.jsonl")
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(entry.to_string().as_bytes())?;
            f.write_all(b"\n")
        });
}
```

### 2. Call site in `engine/mod.rs`

After the execution phase (around line 4296, the closing `}` of the BUY match arm), before the LLM context build for the next cycle:

```rust
// FID-196: Per-cycle reconciliation. Catches phantom positions
// AND USDC balance drift before the LLM sees them next cycle.
if let Some(ref ex) = executor {
    let recon_cfg = ReconciliationConfig {
        chain_id: active_chain.chain_id,
        wallet_address: shared.wallet_address.read().await.clone(),
        rpc_url: active_chain.rpc_url.clone(),
        divergence_threshold_usd: 0.10,
        divergence_threshold_pct: 0.01,
        interval_cycles: 1,
        safety_halt_threshold_pct: 0.50,  // 50% = halt, 50% = correct
    };
    let recon_report = apply_to_portfolio(
        &recon_cfg,
        &mut portfolio,
        &mut decision_log,
        ex.as_ref(),
        &mut shared,
    );
    if recon_report.rpc_failure {
        warn!("FID-196: RPC failure during reconciliation, skipping this cycle");
    } else if recon_report.halt_recommended {
        error!("FID-196: Halt recommended — {}", recon_report.halt_reason.as_deref().unwrap_or("unknown"));
        let _ = std::fs::write(
            "savant.blocked",
            format!(
                "{}\nTrigger: reconciliation_halt\nReason: {}\n",
                chrono::Utc::now().to_rfc3339(),
                recon_report.halt_reason.as_deref().unwrap_or("unknown")
            )
        );
        break;  // Exit cycle
    } else {
        if !recon_report.phantom_cleared.is_empty() {
            warn!("FID-196: Cleared {} phantom position(s) this cycle: {:?}",
                recon_report.phantom_cleared.len(), recon_report.phantom_cleared);
        }
        if !recon_report.orphan_added.is_empty() {
            warn!("FID-196: Added {} orphan position(s) this cycle: {:?}",
                recon_report.orphan_added.len(), recon_report.orphan_added);
        }
        if recon_report.usdc_divergence_corrected != 0.0 {
            warn!("FID-196: Corrected USDC balance by ${:.4}", recon_report.usdc_divergence_corrected);
        }
    }
}
```

### 2a. Mid-cycle reconciliation (advanced — optional)

The cycle-start reconciliation catches drift between cycles. For long cycles (5+ min), mid-cycle drift can also occur. Optional: add a second reconciliation just before the LLM context build, so the LLM sees fresh state. This is more expensive (2 RPC calls per cycle) but more accurate. **Decision: defer to v0.15.0 unless v0.14.6 validation shows drift in cycles.** Note in FID-196 as a follow-up.

---

## Verification

### Unit tests
```rust
#[test]
fn apply_to_portfolio_clears_phantom_positions() {
    let mut portfolio = make_test_portfolio();
    portfolio.positions_mut().insert("ENA/USD".to_string(), make_phantom_ena());
    let executor = MockExecutor::new(vec![]); // no on-chain positions
    
    let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &mut shared, &executor);
    assert_eq!(report.phantom_cleared, vec!["ENA/USD".to_string()]);
    assert!(portfolio.positions().is_empty());
}

#[test]
fn apply_to_portfolio_adds_orphan_positions() {
    let mut portfolio = make_test_portfolio();
    let executor = MockExecutor::new(vec![make_ena_on_chain()]);
    
    let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &mut shared, &executor);
    assert_eq!(report.orphan_added, vec!["ENA/USD".to_string()]);
    assert_eq!(portfolio.positions().len(), 1);
}

#[test]
fn apply_to_portfolio_reconciles_usdc() {
    let mut portfolio = make_test_portfolio();
    portfolio.account_mut().balance = 50.0;
    let executor = MockExecutor::new_with_balance(vec![], 49.0); // $1 divergence
    
    let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &mut shared, &executor);
    assert!(report.usdc_divergence_corrected.abs() < 0.001);
    assert!((portfolio.account().balance - 49.0).abs() < 0.001);
}

#[test]
fn apply_to_portfolio_halts_on_extreme_divergence() {
    let mut portfolio = make_test_portfolio();
    portfolio.account_mut().balance = 50.0;
    // Phantom worth $30 = 60% of portfolio, exceeds 50% threshold
    portfolio.positions_mut().insert("BIG/USD".to_string(), make_phantom_worth_30());
    let executor = MockExecutor::new(vec![]);
    
    let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &mut shared, &executor);
    assert!(report.halt_recommended);
    assert!(!report.phantom_cleared.is_empty() == false); // not cleared, halted
    assert!(portfolio.positions().contains_key("BIG/USD")); // not removed
}

#[test]
fn apply_to_portfolio_skips_on_rpc_failure() {
    let mut portfolio = make_test_portfolio();
    let executor = MockExecutor::new_with_rpc_error();
    
    let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &mut shared, &executor);
    assert!(report.rpc_failure);
    assert!(!report.halt_recommended);
    assert!(portfolio.positions().is_empty()); // no mutation
}

#[test]
fn apply_to_portfolio_records_telemetry() {
    let mut portfolio = make_test_portfolio();
    let executor = MockExecutor::new(vec![]);
    
    apply_to_portfolio(&cfg, &mut portfolio, &mut log, &mut shared, &executor);
    
    // Verify telemetry file was written
    let telemetry = std::fs::read_to_string("data/reconciliation_telemetry.jsonl").unwrap();
    assert!(telemetry.contains("reconciliation"));
    assert!(telemetry.contains("phantom_cleared"));
}
```

### Manual test
1. Inject a phantom position: `portfolio.positions_mut().insert("TEST/USD", fake_pos)`
2. Run cycle
3. Verify reconciliation removes it
4. Verify log shows "FID-196: Phantom position detected: TEST/USD"
5. Verify `data/reconciliation_telemetry.jsonl` was appended

### Integration test
1. Run paper mode for 4h
2. Verify `data/reconciliation_telemetry.jsonl` has one entry per cycle
3. Count phantom_cleared events — should be 0 in normal operation
4. Verify USDC divergence correction events match the executor's balance changes
5. Verify no `savant.blocked` file created (unless divergence > 50%)

---

## Risk

**Medium.** This mutates the portfolio based on on-chain state. If the on-chain query is wrong (RPC failure, wrong address), the portfolio could be cleared incorrectly. Mitigations:
- Skip reconciliation on RPC failure (existing behavior in `reconcile_wallet_state`)
- Log all reconciliation actions to `data/reconciliation_log.json` for audit
- Don't clear positions if divergence > 50% of portfolio value (safety threshold)

---

## Files Changed Summary

- `src/execution/reconciliation.rs`: 1 new function `apply_to_portfolio`, 1 new struct `ReconciliationApplyReport`, ~80 lines
- `src/engine/mod.rs`: 1 new call block after execution phase, ~25 lines
- 2-3 unit tests
- 0 documentation changes (FID-193 documents parent context)
