# FID-196: Per-Cycle Wallet/Decision Reconciliation

**Filename:** `FID-2026-0617-196-cycle-reconciliation.md`
**ID:** FID-2026-0617-196
**Severity:** critical
**Status:** analyzed
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

```rust
/// FID-196: Apply reconciliation to portfolio + decision log.
/// Mutates in-memory state to match on-chain reality. Catches:
/// - Phantom positions: in memory but not on chain (executor rejected)
/// - Orphan positions: on chain but not in memory (crash recovery, manual tx)
///
/// Returns a list of phantom positions cleared and orphan positions added
/// for logging and testing.
pub fn apply_to_portfolio(
    config: &ReconciliationConfig,
    portfolio: &mut PortfolioManager,
    decision_log: &mut DecisionLog,
    executor: &dyn ExecutionEngine,
) -> ReconciliationApplyReport {
    let mut report = ReconciliationApplyReport::default();
    
    // Step 1: Get on-chain positions
    let on_chain_pairs: std::collections::HashSet<String> = executor
        .open_positions()
        .iter()
        .map(|p| p.pair.clone())
        .collect();
    
    // Step 2: For each in-memory position NOT on-chain, mark as phantom and remove
    let in_memory: Vec<(String, Position)> = portfolio
        .positions()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    
    for (id, pos) in &in_memory {
        if !on_chain_pairs.contains(&pos.pair) {
            warn!(
                "FID-196: Phantom position detected: {} (id={}, entry={:.4}) — in memory but not on chain. Removing.",
                pos.pair, id, pos.entry_price
            );
            report.phantom_cleared.push(pos.pair.clone());
            portfolio.positions_mut().remove(id);
            decision_log.update_status(
                &pos.pair,
                TradeStatus::Rejected,
                Some("phantom: not on chain".to_string()),
            );
        }
    }
    
    // Step 3: For each on-chain position NOT in memory, add it
    for on_chain_pos in executor.open_positions() {
        let already_in_memory = portfolio
            .positions()
            .values()
            .any(|p| p.pair == on_chain_pos.pair);
        if !already_in_memory {
            warn!(
                "FID-196: Orphan on-chain position: {} (entry={:.4}) — on chain but not in memory. Adding.",
                on_chain_pos.pair, on_chain_pos.entry_price
            );
            report.orphan_added.push(on_chain_pos.pair.clone());
            portfolio
                .positions_mut()
                .insert(on_chain_pos.id.clone(), on_chain_pos.clone());
            // Mark as Filled since it's actually on chain
            decision_log.update_status(&on_chain_pos.pair, TradeStatus::Filled, None);
        }
    }
    
    // Step 4: Update account state
    portfolio.account_mut().open_positions = portfolio.positions().len();
    
    report
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReconciliationApplyReport {
    pub phantom_cleared: Vec<String>,
    pub orphan_added: Vec<String>,
    pub usdc_divergence_corrected: f64,
}
```

### 2. Call site in `engine/mod.rs`

After the execution phase (around line 3500, after all BUY/SELL/CLOSE attempts), before the LLM context build:

```rust
// FID-196: Per-cycle reconciliation. Catches phantom positions before
// the LLM sees them next cycle.
if let Some(ref ex) = executor {
    let recon_cfg = ReconciliationConfig {
        chain_id: active_chain.chain_id,
        wallet_address: shared.wallet_address.read().await.clone(),
        rpc_url: active_chain.rpc_url.clone(),
        divergence_threshold_usd: 0.10,
        divergence_threshold_pct: 0.01,
        interval_cycles: 1,
    };
    let recon_report = apply_to_portfolio(
        &recon_cfg,
        &mut portfolio,
        &mut decision_log,
        ex.as_ref(),
    );
    if !recon_report.phantom_cleared.is_empty() {
        warn!(
            "FID-196: Cleared {} phantom position(s) this cycle: {:?}",
            recon_report.phantom_cleared.len(),
            recon_report.phantom_cleared
        );
    }
    if !recon_report.orphan_added.is_empty() {
        warn!(
            "FID-196: Added {} orphan position(s) this cycle: {:?}",
            recon_report.orphan_added.len(),
            recon_report.orphan_added
        );
    }
}
```

---

## Verification

### Unit tests
```rust
#[test]
fn apply_to_portfolio_clears_phantom_positions() {
    let mut portfolio = make_test_portfolio();
    portfolio.positions_mut().insert("ENA/USD".to_string(), make_phantom_ena());
    let executor = MockExecutor::new(vec![]); // no on-chain positions
    
    let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &executor);
    assert_eq!(report.phantom_cleared, vec!["ENA/USD".to_string()]);
    assert!(portfolio.positions().is_empty());
}

#[test]
fn apply_to_portfolio_adds_orphan_positions() {
    let mut portfolio = make_test_portfolio();
    let executor = MockExecutor::new(vec![make_ena_on_chain()]);
    
    let report = apply_to_portfolio(&cfg, &mut portfolio, &mut log, &executor);
    assert_eq!(report.orphan_added, vec!["ENA/USD".to_string()]);
    assert_eq!(portfolio.positions().len(), 1);
}
```

### Manual test
1. Inject a phantom position: `portfolio.positions_mut().insert("TEST/USD", fake_pos)`
2. Run cycle
3. Verify reconciliation removes it
4. Verify log shows "FID-196: Phantom position detected: TEST/USD"

### Integration test
1. Run paper mode for 4h
2. Verify `data/phantom_positions.json` log file is created
3. Count phantom_cleared events — should be 0 in normal operation
4. Verify `data/orphan_positions.json` log file shows 0 events
5. Verify no AdjustStop/Close for non-existent positions

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
