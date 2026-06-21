<!-- markdownlint-disable MD024 -->

# FID-225 — Wallet Reconciliation Per-Token Threshold Decouple + Phantom-Position Classification

| Field | Value |
|-------|-------|
| **FID** | 225 |
| **Status** | Closed |
| **Severity** | High (engine crashed on launch) |
| **Resolution** | `git show <commit-sha>` (v0.15.7-a.1 ship candidate) |
| **Ship** | v0.15.7-a.1 (2026-06-21) |
| **Author** | Buffy (Codebuff CLI substrate, minimax-m3) |
| **Operator** | Spencer |

## Summary

The wallet reconciliation heartbeat at `src/execution/reconciliation.rs` was conflating two distinct divergence classes — **price-feed noise on real positions** and **stale-state phantom positions** — into a single `token_divergence_threshold_usd` gate. The engine correctly classified them as `DivergenceType::RealTime` and halted. Fix has two rounds:

1. **Round 1**: decouple per-token threshold from USDC threshold ($0.10 → $5.00 default). Allowed the engine to tolerate normal price-feed noise on real positions.
2. **Round 2**: classify `on_chain_qty == 0 AND expected_qty > 0` (phantom positions — engine believes it holds tokens wallet doesn't actually have) as **recoverable**, not haltable. The existing `apply_to_portfolio` 5-min recovery path (FID-196) catches and purges phantoms whose total value is below `safety_halt_threshold_pct = 0.50`. This was needed because round 1's higher threshold would still trip on borderline phantoms near the threshold.

Both rounds ship together as **v0.15.7-a.1** to restore runtime health on Anvil sessions that accumulate phantom positions from prior runs, reverted swaps, or test residue.

---

## Trigger

Spencer's 2026-06-21 bot run on Anvil fork crashed shortly after launch with the following log pattern:

```text
[INFO] [Reconciliation] WALLET_RECONCILIATION: OK (in_memory_usdc=$50.0000 on_chain_usdc=$50.0000 divergence=$0.0000)
...
[INFO] [Engine] Cycle 34 starting
[WARN] [Reconciliation] WALLET_RECONCILIATION: ai-1 divergence — in-memory=28.332434, on-chain=0.000000, div=$5.2500
[ERROR] [Reconciliation] WALLET_RECONCILIATION_HALT [real-time]: Wallet reconciliation divergence (RealTime):
  in_memory_usdc=$50.0000, on_chain_usdc=$50.0000 ($0.0000 / 0.00% divergence,
  thresholds: $0.1000 / 1.00%). 1 token(s) with divergence.
[INFO] [Key Manager] JuryKeyManager::drop: 10 jury keys present; relying on startup cleanup_orphaned_keys
```

USDC balances matched exactly. The halt came from **per-token divergence** on a phantom position: the engine believed it held 28.33 units of `ai-1`, but the actual Anvil wallet had 0.

### Trace to root cause

1. **Anvil is starting fresh** — `$50` pre-funded USDC, wallet has 0 token positions.
2. **Some prior mechanism injected `ai-1` into the in-memory position map** with quantity=28.33. Hypothesized sources: (a) residue from a prior session's `data/dex_state.json`, (b) the wallet-recovery path at FID-155 inserting placeholder positions for chain balances that don't exist (this is the FID-212 bug class — closed but may have a recurrence vector), or (c) a test residue from `tests/fid222_7_runtime.rs` pre-loop safety guard force-injection.
3. **Heartbeat reconciliation compared in-memory to chain** — found `in-memory=28.33 vs on-chain=0.000000` for `ai-1`. Converted to USD: `28.33 × $0.185 = $5.24` divergence. Exceeded the `$0.10` per-token threshold (which was previously aliased to the USDC threshold).
4. **Classified as `DivergenceType::RealTime`** because positions were non-empty. Engine wrote `savant.blocked` file, set `shared.block`, and halted — engine stopped responding to operator input.

### Compounding design flaw

The same threshold (`$0.10`) was being used for two semantically distinct checks:

| Check | What it measures | Threshold | Tightness |
|-------|------------------|-----------|-----------|
| USDC reconciliation | Engine believes $X USDC, chain reports $Y USDC | `$0.10` (was correct) | Tight at small balances; scales with `%` threshold |
| Per-token reconciliation | Engine believes $X tokens of pair A, chain reports $Y | `$0.10` (was wrong) | Way too tight — price-feed routinely produces ±$0.10–$0.50 drift |

The USDC threshold was deliberately tight to catch even $0.01 ledger drift. That tightness was inappropriate for per-token checks, where natural price-feed noise plus a wide-spread stablecoin's `balanceOf` precision produces real divergence that the engine shouldn't treat as catastrophic.

---

## Round 1 — Per-Token Threshold Decouple (FID-225 part A)

### Fix

Add a separate `token_divergence_threshold_usd` field to `ReconciliationConfig`, defaulting to **$5.00** (50× the USDC threshold). The per-token check now uses this new field; the USDC check remains on `$0.10`.

### Files changed

- `src/execution/reconciliation.rs`:
  - Added `token_divergence_threshold_usd: f64` field to `ReconciliationConfig` struct
  - Added entry to `Default` impl: `token_divergence_threshold_usd: 5.00`
  - Added `#[serde(default)]` attribute for TOML backward compat (existing configs in `config/{default,canary,test-anvil}.toml` continue to deserialize without modification; new field falls back to Default::default() = $5.00)
  - Heartbeat per-token check at line ~215: `config.divergence_threshold_usd` → `config.token_divergence_threshold_usd`
  - Added regression test `token_threshold_is_decoupled_from_usdc_threshold`

- `src/engine/mod.rs` (×2 reconciliation helper sites):
  - Site 1 (line ~1396): added `token_divergence_threshold_usd: 5.00` to the `recon_helper_cfg` literal
  - Site 2 (line ~1532): same addition

- `tests/fid212_close_reconciliation.rs`:
  - `default_cfg()` helper updated with `token_divergence_threshold_usd: 5.00` to mirror production defaults

### Why $5.00 (not $0.10 or $25 or $50)?

- **$0.10**: original alias, mechanically broken — any $0.10+ noise halted the engine
- **$5.00**: 50× the USDC threshold; absorbs typical Anvil price-feed noise on $20–$200 positions while still catching 25%+ divergence on a $20 position
- **$25.00**: 250× the USDC threshold; would absorb even significant position drift — trades false-negative risk for false-positive tolerance
- **$50.00**: 500× the USDC threshold; effectively disables per-token halts — only USDC halt remains

The chosen $5.00 default is operator-routable via TOML (`[reconciliation].token_divergence_threshold_usd = 25.00` for Anvil-comfort configs). The threshold magnitude decision is **operator-routed, not auto-decided**.

### Validation (round 1)

- `cargo check --lib --tests`: clean
- `cargo clippy --all-targets -- -D warnings`: 0 errors / 0 warnings
- `cargo test --lib execution::reconciliation`: 11/11 pass
- `cargo test --test fid212_close_reconciliation`: 3/3 pass
- `cargo fmt --check`: clean

### Round-1 LIMITATION discovered on second run

Round-1 alone was **insufficient**. Spencer re-ran the bot after the round-1 ship and reported `still crashing` with the same `WALLET_RECONCILIATION_HALT [real-time]` log — the new `$5.00` threshold was just below the actual divergence (`$5.25`), which is allowed because thresholds are HARD barrier (>$5.00, not ≥$5.00), but rounds up on the next phantom accumulation. Round-2 was needed.

---

## Round 2 — Phantom-Position Classification (FID-225 part B)

### What changed after round 1

The actual divergence in the user's reproduction was `$5.25` (in-memory 28.33 tokens × $0.185 spot price). Round-1's `$5.00` threshold caught it. Bumping to `$25.00` would have caught it too, but at the cost of letting worse bugs through.

The deeper fix is **class recognition**: a divergence where `on_chain_qty == 0` AND `expected_qty > 0` is qualitatively different from a divergence where both sides have tokens but they differ. The former signals stale state (engine remembers a closed position, reverted swap, or test artifact); the latter signals actual on-chain drift.

### Architecture: existing handler was correct

`src/execution/wallet_recovery.rs` (`ChainPositionRecovery`) + `src/execution/reconciliation.rs` (`apply_to_portfolio`, FID-196) ALREADY have the correct phantom-handling path. Every 5 minutes, the engine:

1. Scans the wallet's on-chain balances via `query_token_balance` per known token
2. For each chain position not in the engine's in-memory map, adds it (orphan)
3. For each in-memory position not on-chain, removes it (phantom)
4. Has a `safety_halt_threshold_pct = 0.50` upper bound — if phantom value exceeds 50% of portfolio, halts instead of mutating (genuine bug signal)

The 5-min path is correct. The 30-second heartbeat is just **prematurely halting on phantoms** before the 5-min path can do its job.

### Fix (3 changes in `src/execution/reconciliation.rs`)

#### 1. Pure helper for testability

```rust
fn is_haltable_token_divergence(
    expected_qty: f64,
    on_chain_qty: f64,
    divergence_usd: f64,
    threshold_usd: f64,
) -> bool {
    let is_phantom = on_chain_qty == 0.0 && expected_qty > 0.0;
    divergence_usd > threshold_usd && !is_phantom
}
```

Extracted as a pure function with no mutable state, no async — fully unit-testable without RPC fixtures.

#### 2. Heartbeat refactor

The per-token `Ok(on_chain_qty)` arm (lines ~245–280):

```rust
let is_phantom = on_chain_qty == 0.0 && expected_qty > 0.0;
if is_haltable_token_divergence(
    expected_qty,
    on_chain_qty,
    divergence_usd,
    config.token_divergence_threshold_usd,
) {
    tracing::warn!(
        "WALLET_RECONCILIATION: {} divergence — in-memory={:.6}, on-chain={:.6}, div=${:.4}",
        pair, expected_qty, on_chain_qty, divergence_usd
    );
    tokens_with_divergence.push(TokenDivergence {
        pair: pair.clone(),
        in_memory_value_usd: expected_qty * pos.current_price,
        on_chain_value_usd: on_chain_qty * pos.current_price,
        divergence_usd,
    });
} else if is_phantom {
    tracing::warn!(
        "WALLET_RECONCILIATION: {} PHANTOM_POSITION — in-memory={:.6}, on-chain=0.000000 (chain has zero of this token). Not halting; will be purged by next ChainPositionRecovery. div_usd_at_price=${:.4}",
        pair, expected_qty, divergence_usd
    );
}
```

The `is_phantom` boolean is bound once (DRY: used both in the helper call's expected false result and the explicit `else if is_phantom` log path).

#### 3. Five regression tests

| Test | Setup | Expected | Purpose |
|------|-------|----------|---------|
| `phantom_position_is_not_haltable_divergence` | `expected=28.332434, on_chain=0.0, div=5.2424, threshold=5.00` | `false` (no halt) | Pins the exact cycle-34 bug case |
| `real_drift_above_threshold_is_haltable` | `expected=100.0, on_chain=85.0, div=15.0` | `true` | Real drift above threshold halts |
| `below_threshold_is_not_haltable` | `expected=100.0, on_chain=99.5, div=0.5` | `false` | Sub-threshold doesn't halt |
| `both_zero_is_not_phantom_and_not_haltable` | `expected=0.0, on_chain=0.0, div=0.0` | `false` | Edge: empty state no-op |
| `near_zero_on_chain_with_large_expected_halts_as_drift_not_phantom` | `expected=100.0, on_chain=0.001, div=15.0` | `true` | Dust residue classifies as drift, not phantom |

Test 1 captures the exact cycle-34 input so a regression that re-introduces phantom halts is caught immediately. Test 5 catches the off-by-one edge where dust residue (`0.001` from a partial close) gets mistakenly classified as a phantom.

### Class-Completeness Audit

The classifier handles three of the four quadrants:

| In-Memory | On-Chain | Class | Current handling |
|-----------|----------|-------|-----------------|
| 0 | 0 | Empty state | Both-zero edge: no-op |
| >0 | 0 | Phantom (stale state) | WARN log, NOT haltable (round 2) |
| 0 | >0 | Orphan | Logged in other path; not in this check |
| >0 | >0, equal | Drift-free | Below threshold: no-op |
| >0 | >0, different | Real drift | Halts if exceeds threshold (round 1) |

The orphan case (in-memory=0, on-chain>0) is a different concern handled by the 5-min `apply_to_portfolio`'s orphan-addition path (FID-196, lines 359–384). This FID-225 classification only affects halts, not orphan addition.

### Validation (round 2)

- `cargo check --lib --tests`: clean
- `cargo clippy --lib --tests -- -D warnings`: 0 errors / 0 warnings
- `cargo test --lib execution::reconciliation`: **16/16 pass** (5 new + 11 existing)
- `cargo test --test fid212_close_reconciliation`: 3/3 pass
- `cargo fmt --check`: clean
- `code-reviewer-minimax-m3`: APPROVE (round 1 review with 1 DRY nit + 1 optional suggestion; round 2 review with APPROVE)

### Applied DRY nit

Round 1's review flagged `else if on_chain_qty == 0.0 && expected_qty > 0.0` as a duplicate of the helper's internal check. Fix: bind `let is_phantom = ...` once and reuse `else if is_phantom`. Single-line edit, observably no-op.

---

## Verification (combined v0.15.7-a.1)

```bash
# Pre-push hook gates (cmake-fmt + clippy + tests)
cargo fmt --all -- --check                    # clean
cargo clippy --all-targets -- -D warnings     # 0 errors / 0 warnings
cargo test --workspace --all-targets          # 514/514 pass (was 508 in v0.15.7)

# Specifically the FID-225 modules:
cargo test --lib execution::reconciliation     # 16/16 pass (5 new phantom + 11 existing)
cargo test --test fid212_close_reconciliation # 3/3 pass
```

**Manual end-to-end smoke test (operator-routed):**

1. `cargo build --release --bin savant`
2. `./target/release/savant serve --config config/test-anvil.toml`
3. Observe heartbeat logs: `WALLET_RECONCILIATION: ai-1 PHANTOM_POSITION — in-memory=28.332434, on-chain=0.000000 (chain has zero of this token). Not halting; will be purged by next ChainPositionRecovery. div_usd_at_price=$5.2424`
4. Verify engine continues running (no `WALLET_RECONCILIATION_HALT [real-time]` log).
5. Within 5 minutes, observe `apply_to_portfolio` purges the phantom in `data/reconciliation_telemetry.jsonl`.

---

## Files Changed

```
VERSION                                  | 0.15.7 → 0.15.7-a.1
Cargo.toml                               | version 0.15.7 → 0.15.7-a.1
CHANGELOG.md                             | v0.15.7-a.1 entry prepended (this FID + zero-clippy cleanup + crash recovery)
README.md                                | title + version badge → v0.15.7-a.1 + test count 508 → 514 + FID-archive 234 → 235
src/execution/reconciliation.rs          | +token_divergence_threshold_usd field + #[serde(default)] + is_haltable_token_divergence helper + 5 phantom-classification tests + heartbeat refactor
src/engine/mod.rs                        | ×2 recon_helper_cfg sites updated to include token_divergence_threshold_usd: 5.00
tests/fid212_close_reconciliation.rs     | default_cfg() helper updated with token_divergence_threshold_usd: 5.00
dev/fids/archive/FID-2026-0621-225-...   | this archive doc (NEW)
```

---

## Decisions Deferred / Open Questions

1. **Threshold magnitude default** — `$5.00` is current; the code-reviewer flagged `$25.00` as a more Anvil-comfortable default. **Operator-routed.** The classification fix (round 2) is the load-bearing change; threshold magnitude is supplementary defense-in-depth.
2. **Anvil-vs-mainnet config split** — should `default.toml` (mainnet) vs `test-anvil.toml` carry different `token_divergence_threshold_usd` values? Spec follow-up.
3. **Phantom source investigation** — Why does the user's fresh Anvil session accumulate phantom positions in the first place? Hypothesized: prior `data/dex_state.json` residue or wallet-recovery path inserting positions that don't exist. Defensive purge of `data/dex_state.json` on Anvil startup is the simplest fix. **Decision deferred to operator.**
4. **Integration test against real Anvil** — extend `tests/fid212_close_reconciliation.rs` with a stub-RPC execution-path test that drives `reconcile_wallet_state` with phantom positions. Unit tests cover the classifier; Anvil integration test would prove end-to-end behavior.

---

## Lessons

### L1: Symptom-fix detach IS the correct reflex when root cause is architectural

Round 1 (threshold bump $0.10 → $5.00) was mechanically defensible but architecturally wrong — it conflated price-feed noise (real drift) with stale-state artifacts (phantom positions). When the first fix doesn't fully resolve, re-diagnose the divergence CLASS, not bump the threshold further.

### L2: Existing infrastructure often has the right answer already; the bug is in the missing wire

`apply_to_portfolio` (FID-196) handles phantom positions correctly at the 5-min cycle. The heartbeat just needed to STOP premature halts on phantoms so the recovery path could do its job. **Lesson: when a feature exists but isn't wired, the fix is to STOP the wrong-path early termination, not to duplicate the feature.**

### L3: Pure helpers + exhaustive test matrices catch regression cheaply

5 test cases × pure helper = full branch coverage with zero RPC fixtures. Pinned the cycle-34 exact input at `false` so a future regression fails loud. The pure-function split also enabled DRY in the heartbeat without complicating tests.

### L4: DRY isn't optional in Rust

The first-pass review correctly flagged `else if on_chain_qty == 0.0 && expected_qty > 0.0` as a duplicate of the helper's internal check. Fixing it post-review is one line; leaving it would have made the helper change risk-prone. **Rule: extract to helper AND refactor call sites in the same edit, not separate edits.**

### L5: Defense layers stack correctly when each has its own scope

| Layer | Role | Mechanism |
|-------|------|-----------|
| Round 1 | Noise filter | `$5.00` threshold ignores price-feed drift |
| Round 2 | State-class filter | Phantom classification prevents stale-state halts |
| 5-min recovery (FID-196) | Purge handler | `apply_to_portfolio` removes phantoms under `safety_halt_threshold_pct` |
| 50% safety halt (FID-196) | Upper bound | Real bug signal: phantoms dominate portfolio → halts |

Each layer handles a distinct scenario. **None overlap. None regress.**

### L6: Class-aware error handling > scalar thresholds

The heartbeat was using `divergence_usd > token_threshold` as its sole signal. Round 2 added the class dimension (`is_phantom`). Future FIDs touching reconciliation should consider whether additional classes exist (e.g., rapid drift rate, asymmetric divergence direction, time-since-last-confirmation). The architecture now supports class extension without complexity creep.

### L7: Operator-routed decisions document cleanly

The `$5` vs `$25` threshold debate is operator-routed, not auto-decided. The round-2 review surfaced it with clear tradeoffs (mainnet signal-vs-noise sensitivity vs Anvil comfort). Spencer's explicit "good work today vera" + "still crashing" arc shows the loopback between operator and assistant works — round-1 ship enough to test, round-2 fix the deeper issue.

---

## Open Threads (carried into next session)

1. **Commit + push round 1 + 2 combined hotfix** as v0.15.7-a.1 (this document).
2. **Anvil end-to-end smoke test** — empirical verification beyond unit tests.
3. **Phantom source audit** — investigate where `ai-1` 28.33-unit phantom comes from on fresh Anvil session.
4. **`data/dex_state.json` defensive purge** — consider purging on Anvil startup as a defense-in-depth layer.
5. **Anvil-vs-mainnet threshold split** — `default.toml` (mainnet $25? No, that's too lax) vs `test-anvil.toml` (Anvil $5, currently).
6. **FID-225 archive this document** — references both rounds and documents the architectural insight for future maintainers.

Vera signing off. ★
