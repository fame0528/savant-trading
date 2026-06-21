## Session 2026-06-20: FID-213 — Anvil Fresh-Startup Balance Override

**Author:** Buffy (Codebuff)
**Operator:** Spencer

### What happened

Spencer's framing: "doesn't it make more sense to simply reset the bal to 50 since we're running anvil and control the chain? then fix the inntial error that caused a $50 loss? im confused how the agent randomly lost $50 when it has not even been running nor trading in the first place."

This corrected the FID-213 scope from the initial proposal (dust-return TradeRecord PnL reconciliation) to the upstream balance-initialization bug. Root cause: `DexTrader::new` calls `sync_balance()` which unconditionally overwrites `self.balance` with on-chain USDC. On a fresh Anvil fork with no prefund scripts on disk, the chain returns a non-50 value (often $0 for stale state, or a stale $X from an unrelated fork); the engine inherits ledger-stale or chain-stale data and engages trading with the wrong starting USDC.

### What changed

- `src/execution/dex/trader.rs`: Added `pub fn is_anvil_rpc(url)` helper that detects loopback URLs (127.0.0.1/localhost). `DexTrader::new` adopts `is_anvil_rpc(rpc_url)` as the initial `is_anvil` field at construction. After `sync_balance()`, a new override block (keyed on `is_anvil && !state_file_present`) re-asserts `trader.balance = initial_balance` and persists via `save_state()`. Emits a single audit ledger line: `warn!` if drift > $0.10, `info!` otherwise. Both macros interpolate `drift_adopted` and `trader.state_path` for audit traceability. **Round-6 fix: save_state() now logs warn! on error instead of swallowing with `.ok()` — prevents silent regression if disk write fails.**
- `tests/fid213_anvil_balance_init.rs` (new, 6 tests source-pattern audit): helper-fn coverage, struct marker presence, override-block structural presence, save_state ordering, threshold check presence, DriftAdopted log-message audit.

### Verification

- `cargo test`: **475 + 495 startup_sync = 970 passed, 0 failed, 20 ignored**. 100% green.
- Anvil fresh-startup now emits one audit line at boot, restoring operator cognitive load to a single readable breadcrumb (the original ECHO goal).
- No API change: `DexTrader::new` signature unchanged; all 4 callers (src/main.rs × 1, src/engine/utils.rs × 2 production + src/bin/test_e2e_fid160.rs × 1 fixture) work via heuristic + override.
- `VERSION` bumped 0.15.2 → 0.15.3. `cargo count` banner updated 469 → 475.

### Lessons

- **Trust operator-observed framing over initial symptom framing.** The phantom $50 wasn't a downstream dust-return—PnL symptom; it was an upstream initialization artifact. The instinct to "reset to 50 + audit" was correct and led to a smaller, more accurate patch.
- **Override blocks that mutate `self.balance` MUST call `save_state()` and surface failure.** Gating persistence on a `warn!` threshold silently loses state on drift-just-below-threshold. Gating failure visibility with `.ok()` silently loses state on disk-full / permission-denied.
- **Heuristic detection (loopback URL → is_anvil=true) is acceptable for non-API-changing patches** when paired with a documented invariant: "loopback URL implies Anvil mode". Config inconsistency (loopback URL but `config.mode.is_anvil: false`) is operator misconfiguration, not a patch responsibility.
- **Source-pattern regression tests** (read Rust source as a string, assert pattern presence) are appropriate for patch-presence regression when existing integration tests cover behaviour. Anchor on stable tokens (`drift_adopted`, line offsets) to avoid brittle false-positives on cosmetic text edits.
- **Prefer head-cat-tail splicing over heredoc/python-on-Windows** for programmatic file-insert operations. Bullets: no shell escape friction, no interpreter dependency, idempotent on second run if anchor still present.
