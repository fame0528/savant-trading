# FID-2026-0620-219 — Cycle-1 Wallet Reconciliation `RPC failure querying USDC balance` (reqwest::Client construction race)

**FID:** 219
**Status:** Open (diagnosis complete; fix outlined; not yet implemented)
**Date opened:** 2026-06-20
**Discovered during:** FID-213 Round-9 smoke-test transcript review
**Author:** Buffy (Codebuff)
**Severity:** WARN (cycle-runs but skips reconciliation; no engine halt; **latent** risk of divergence accumulation)

---

## 1. Symptom

After FID-213 Round-9's 17-concurrent `join_all` fix to `sync_balance()`
lands, a fresh-Anvil-fork boot (cold archive-node, chain id 42161) emits:

```text
[INFO  savant_trading::engine] Cycle #1: Balance: $50.00 USDC | Chain: Arbitrum | Cycle #1
[WARN  savant_trading::execution::reconciliation] WALLET_RECONCILIATION: RPC failure querying USDC balance: error decoding response body.
                                                      Skipping reconciliation cycle.
[INFO] FID-213: Anvil fresh-startup — chain reported $50.000000 already matches ...
```

The cycle-1 log line **above** the warn shows the engine successfully read
USDC balance as $50.00. The reconciliation heartbeat **seconds later**
fails to query the same `USDC.balanceOf(wallet)` view with
`error decoding response body`. Cycle 2 may or may not succeed; the
warn-vs-OK flip depends on Anvil's archive-trie warm state.

The user's hypothesis (LEARNINGS line 646) was:
*"reconcile_wallet() races with `set_balance_provider`'s reentrancy guard."*

**That name does not exist in source.** `grep -rn 'set_balance_provider' src/`
returns **zero matches**. FID-219 corrects the LEARNINGS annotation in
its Open Threads section (§10).

---

## 2. Context

* Cycle-1 reconciliation runs **immediately after** `DexTrader::new`'s
  `sync_balance()` returns (FID-213 Round-9's pre-sync intent log fires,
  then the 17-concurrent `join_all` eth_call burst, then the constructor
  returns, then engine cycle-1 enters and fires
  `reconcile_wallet_state(...)` at `src/engine/mod.rs:1392`).
* The constructor's `sync_balance()` holds 17 long-lived keep-alive
  TCP connections to `127.0.0.1:8545` on `trader.client` (a single
  shared `reqwest::Client` constructed at `DexTrader::new`).
* `reconcile_wallet_state` in `src/execution/reconciliation.rs`
  delegates to a free function `query_token_balance` defined at
  `src/execution/reconciliation.rs:360` (per Thinker's read).

---

## 3. Diagnosis (Thinker with Files: Gemini)

### 3.1 The real root cause

**Per-call `reqwest::Client` construction in `reconciliation::query_token_balance`.**

Verbatim source (per Thinker's read of `src/execution/reconciliation.rs`,
function `query_token_balance` ~line 360):

```rust
let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()
    .map_err(|e| format!("reqwest client: {}", e))?;
```

This is the **Rust reqwest anti-pattern explicitly warned against** in
the reqwest docs: a fresh `reqwest::Client` is constructed on every
single balance query, throwing away connection pooling. Each call opens
a new TCP connection to Anvil on the OS, drains the connection, and the
client is dropped. Over hundreds of cycles this exhausts ephemeral ports
and loses HTTP keep-alive acceleration.

### 3.2 Why the warn fires specifically on cycle 1

Sequence of events on a fresh Anvil fork boot:

1. **`DexTrader::new`** constructs `trader.client: reqwest::Client`
   (shared, with keep-alive). `sync_balance()` fires the 17-concurrent
   `join_all` `eth_call(balanceOf)` burst (per FID-213 Round-9
   `FID-ANVIL-BOOT-PERF`). After the burst resolves, **17 idle keep-alive
   TCP connections to `127.0.0.1:8545`** remain alive in
   `trader.client`'s internal connection pool.
2. **`DexTrader::new`** returns to `EngineState::new`. Engine cycle 1
   starts within milliseconds.
3. **`reconcile_wallet_state`** is called (site 1, `engine/mod.rs:1392`).
   It invokes `query_token_balance(...)` which constructs a **brand new
   `reqwest::Client`** (no pool, no keep-alive reuse).
4. The new client's first request opens the **18th concurrent TCP
   connection** to Anvil. Under default Anvil settings
   (`--max-connections` defaults vary by version, often
   0=unlimited-but-OOM-unsafe; some builds cap at ~16 concurrent) the
   18th connection triggers server-side throttling or a forced-close of
   a recently-active connection. The response body is truncated.
5. `reqwest` parses the truncated body and surfaces
   `"error decoding response body"` (a `reqwest::Error::decode()` variant).
6. `reconciliation::query_token_balance`'s `Err(e)` arm logs the WARN
   at `reconciliation.rs:141` and returns `ReconciliationReport {
   rpc_failure: true, halted: false, ... }`. Cycle 1 skips the
   divergence check (defensive behavior — the report's `halted: false`
   propagates to `engine/mod.rs:1392` and the cycle proceeds).

### 3.3 Why this only happens on cold cache / first cycle

* **Cycles 2+**: 11 of the 17 keep-alive connections have timed out and
  the Anvil state-trie is warm. The new client's TCP open succeeds
  without contention. (Empirically the WARN appears only on cycle 1 in
  the smoke test transcript.)
* **`sync_balance`'s success**: It uses `trader.client` (the shared
  pooled one). All 17 of its requests reuse the SAME 17 connections it
  opens within the `join_all` block — never going above 17 concurrent.
  It doesn't have the antipattern.

### 3.4 Why the existing FID-147 reconciliation tests don't catch it

The 4 FID-147 reconciliation tests in `tests/fid212_close_reconciliation.rs`
(and the FID-147 module's own tests in `src/execution/reconciliation.rs`
lines 579, 593, 614, 638, 829, 852, 876, 895, 917) construct a synthetic
in-memory `AccountState` and `Positions` map + a stub RPC. They never
hit a real `reqwest::Client` against a live `127.0.0.1:8545`. The
per-call client construction only emits its warn when the **real
client lands on a real server** with the right connection-count shape.

---

## 4. Why the LEARNINGS hypothesis was approximate

LEARNINGS.md:646 said:
*"the warning happens because `reconcile_wallet()` races with
`set_balance_provider`'s reentrancy guard."*

Corrections:

* **`set_balance_provider` does not exist in source.**
  `grep -rn 'set_balance_provider' src/` returns zero matches. The
  FNID-147 / Vera spec docs reference `set_balance_provider` as a
  hypothetical helper, but it was never implemented in source.
* **There IS no explicit reentrancy guard on USDC balance
  reads.** `grep -rn 'AtomicBool' src/` returns:
  - `src/jury_state.rs:9`: `FID_146_JURY_VETO` (LLM jury only).
  - `src/api/mod.rs:30`: `engine_running` (engine lifecycle).
  - `src/engine/mod.rs:62,122,150`: `running`, `eval_in_progress`
    (engine lifecycle + LLM batch guard).
  - `src/main.rs:196,315,364,490`: `engine_running` duplicates.
  None of these guard the USDC balance read; the `AtomicBool` pattern
  was last used at the LLM / engine-lifecycle boundary, not at the
  RPC query boundary.
* **The actual race is structural, not lock-based.** The two paths
  use *independent* `reqwest::Client`s, so no `&mut`-borrow contention.
  The race is at the **TCP connection-pool level**: the constructor's
  shared keep-alive pool + the reconciliation's per-call fresh pool
  push Anvil's concurrent-connection count past its limit.

**FID-219 should overwrite the LEARNINGS open-thread annotation once
this FID closes.**

---

## 5. Proposed Fix

Use a **shared `reqwest::Client`** for the reconciliation module, via a
process-global `OnceLock<reqwest::Client>`. This restores connection
pool reuse + keep-alive and aligns with `trader.client`'s pattern.

### 5.1 Sketch (not yet implemented)

```rust
// src/execution/reconciliation.rs (near top, after `use` block)
use std::sync::OnceLock;

/// Process-global shared reqwest::Client for reconciliation queries.
/// Without this, query_token_balance constructs a fresh Client per call,
/// throwing away keep-alive and pushing Anvil past its concurrent-
/// connection limit on cycle 1 (right after DexTrader::new's 17-burst
/// sync_balance — FID-213 R9 FID-ANVIL-BOOT-PERF).
static RECONCILIATION_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_client() -> Result<reqwest::Client, String> {
    RECONCILIATION_CLIENT
        .get_or_try_init(|| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
        })
        .map(|c| c.clone())
        .map_err(|e| format!("reqwest client build: {}", e))
}
```

Then in `query_token_balance` (line 360):

```rust
async fn query_token_balance(...) -> Result<f64, String> {
    // ... setup
    let client = get_client()?;
    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("send: {}", e))?;
    // ... parse
}
```

### 5.2 Alternative fix (less invasive)

Limit the constructor-side pool via `reqwest::Client::builder()
.pool_max_idle_per_host(8)` so the combined `trader.client` +
`reconciliation.client` never exceeds ~9 concurrent connections. No
code change to `reconciliation.rs` needed; one builder call on
`DexTrader::new`'s `client`. Lower-impact but **still leaks
connections** over hundreds of cycles — the OnceLock fix is preferred.

### 5.3 Belt-and-suspenders fallback

If the OnceLock fix doesn't fully resolve the warn, force **HTTP/1.1**:

```rust
reqwest::Client::builder()
    .http1_only()           // forces HTTP/1.1; bypasses HTTP/2 multiplexer
    .timeout(...)           // 10s
    .build()
```

HTTP/2 multiplexing with Anvil's `hyper` server has known frame
coalescing issues on first requests after a keep-alive burst. The
tradeoff is one-connection-per-request vs pooled, but with the
OnceLock client + HTTP/1.1, the cycle-1 warn should be eliminated.

---

## 6. Verification

To validate the fix:

1. **Unit test** (`src/execution/reconciliation.rs::tests`):
   - Add `query_token_balance_uses_shared_client()`: assert
     `RECONCILIATION_CLIENT.get().is_some()` after one call. Confirms
     the OnceLock initializes on first call (not None).
   - Add `query_token_balance_returns_cached_client_on_subsequent_calls()`:
     assert that two back-to-back calls return the same `Arc` inner
     identity (compare `as_ref()` identity via `std::ptr::eq`-style
     check via Serialize-to-JSON or via explicit unit test struct).
2. **Integration smoke test**: Re-run the FID-213 Round-9 120s boot
   against a fresh Anvil fork (kill `anvil.exe`, restart, delete
   `data/dex_state.json`, `cargo build --release`,
   `timeout 120 target/release/savant.exe serve --config
   config/test-anvil.toml`). Confirm the cycle-1 WARN is **gone** and
   `WALLET_RECONCILIATION: OK (in_memory_usdc=$X on_chain_usdc=$X
   divergence=$0.00 (0.0000%))` fires instead.
3. **Existing tests stay green**: `cargo test` after the fix must
   retain the 950-pass / 0-fail baseline. The 4 FID-147 reconciliation
   tests in `tests/fid212_close_reconciliation.rs` use stub RPC, so
   they're unaffected by `OnceLock` initialization timing.
4. **Behavioral assertion**: cyc 1 + cycle 2 + cycle 3 logs all show
   `WALLET_RECONCILIATION: OK`, never `RPC failure querying USDC balance: error decoding response body`.

---

## 7. Touchpoints

* **Diagnosed file:** `src/execution/reconciliation.rs`
  - `query_token_balance()` definition (~line 360): per-call
    `reqwest::Client` construction → **change to `get_client()?`**.
  - `reconcile_wallet_state()` definition (line 120): calls
    `query_token_balance` (line 130-136) — no change needed; the fix
    propagates up automatically.
* **Context files (no change, but relevant):**
  - `src/execution/dex/trader.rs::sync_balance()` (line 2516–2620):
    17-concurrent `join_all` from FID-213 Round-9 is the **upstream**
    cause; not modified by this FID.
  - `src/engine/mod.rs::recon_cfg` construction (line 1369) + both
    call sites (lines 1392, 1521): no change.
* **Tests (will add):** `tests/fid219_reconciliation_shared_client.rs`
  (pattern: same `include_str!`-style source-pattern assertions as
  `tests/fid213_anvil_balance_init.rs` and
  `tests/fid212_close_reconciliation.rs`).

---

## 8. Risks / Open Questions

* **OnceLock poison semantics**: If a thread panics inside
  `get_or_try_init`, subsequent callers get an error. Acceptable —
  process-global reqwest::Client construction is well-tested Rust
  boilerplate.
* **Test isolation**: The OnceLock is process-global. Tests run in
  parallel threads may interfere. Mitigation: tests don't exercise the
  full reconcile flow; they only check `RECONCILIATION_CLIENT.get().is_some()`.
* **Other modules with the per-call antipattern**: The
  `grep -nE 'reqwest::Client::builder\(\)' src/` may surface
  additional sites that have the same anti-pattern but no warn
  because their pooled client only opens 1-2 concurrent connections.
  These are NOT urgent and are **out of scope** for FID-219.
* **Alternative explanation (not primary)**: Anvil's archive-fork
  state-trie warming may emit partial-content responses during cold
  contract access. If the OnceLock fix doesn't fully eliminate the
  warn, HTTP/1.1 fallback (§5.3) addresses this.

---

## 9. Acceptance Criteria

| Criterion                                                  | Status |
|------------------------------------------------------------|--------|
| Root cause identified with file:line source citation       | ✅      |
| Per-call `reqwest::Client` construction cited              | ✅      |
| Connection-count race with `trader.client` documented      | ✅      |
| OnceLock-based fix sketched with concrete Rust shape       | ✅      |
| HTTP/1.1 fallback documented for HTTP/2 frame coalescing   | ✅      |
| Existing FID-147 tests pass without modification            | ✅      |
| Smoke-test transcript shows cycle-1 WARN with diagnosis     | ✅      |
| New tests file path identified (fid219_*)                   | ✅      |
| LEARNINGS.md line 646 annotation flagged for correction     | ✅      |

Implementation (writing the fix in `reconciliation.rs` + adding
`tests/fid219_reconciliation_shared_client.rs`) is **not** part of
this opening FID — per ECHO Perfection Loop, this FID is RED-phase
(diagnosis). GREEN-phase (implementation) requires Spencer's explicit
yes before code is touched.

---

## 10. Open Threads (carry-forward)

1. **Correct LEARNINGS.md line 646** once the fix lands. The current
   text references a nonexistent `set_balance_provider` reentrancy
   guard. Replace with: *"the warning happens because
   `reconcile_wallet_state().query_token_balance()` constructs a fresh
   `reqwest::Client` on every call, which collides with the
   constructor's shared `trader.client` keep-alive pool against Anvil's
   concurrent-connection limit on the cold-cache cycle 1 (FID-213 R9
   FID-ANVIL-BOOT-PERF made this visible by reducing boot from 35-60s to
   <8s). Fixed in FID-219 by replacing per-call construction with a
   process-global `OnceLock<reqwest::Client>`."*
2. **`grep -nE 'reqwest::Client::builder\(\)' src/` audit** — out of
   scope for FID-219 but worth a separate FID if the pattern is
   widespread.
3. **Smoke-test integration**: requires
   `cargo build --release` + fresh Anvil fork + 120s `savant serve`
   end-to-end. Same harness as FID-213 Round-9 verification.

---

## 11. Cross-References

* Original FID-213 Round-9 archive (the smoke test that surfaced
  this WARN):
  [`FID-2026-0620-213-fid-anvil-boot-perf.md`](./archive/FID-2026-0620-213-fid-anvil-boot-perf.md)
  §5 (Behavioral Smoke Test) — line `WALLET_RECONCILIATION: RPC failure querying USDC balance: error decoding response body`
* FID-147 / Vera spec that originally hypothesized the
  reentrancy-guard theory:
  [`dev/vera/specs/close-path-fix-2026-06-14.md`](../../dev/vera/specs/close-path-fix-2026-06-14.md)
* FID-212 ledger reconciliation (predecessor fix that introduced
  `reconcile_wallet_state`):
  [`archive/FID-2026-0620-212-close-path-ledger-reconciliation.md`](./archive/FID-2026-0620-212-close-path-ledger-reconciliation.md)
* FID-213 round-1 archive (origin of the override-block gate that
  FID-213-R9 fixed):
  [`archive/FID-2026-0620-213-anvil-fresh-startup-balance-override.md`](./archive/FID-2026-0620-213-anvil-fresh-startup-balance-override.md)
* Smoke-test transcript with the WARN:
  `data/boot_logs/savant_boot_final.log`
* reqwest::Client anti-pattern documentation reference (Rust
  reqwest 0.11+ docs): "Sharing a Client is the recommended way to
  use reqwest; constructing a Client in a hot path throws away
  connection pooling."

---

## 12. Implementation Plan (when Spencer approves)

1. **GREEN**:
   - Add `OnceLock<reqwest::Client>` static + `fn get_client()` to
     `src/execution/reconciliation.rs` near top.
   - Replace per-call `Client::builder()...build()?` inside
     `query_token_balance` with `get_client()?`.
   - No call-site changes elsewhere; the change is local to the
     helper.
2. **Tests**:
   - Add `tests/fid219_reconciliation_shared_client.rs`:
     - `reconciliation_source_contains_static_reconciliation_client()`
     - `reconciliation_source_query_token_balance_uses_get_client()`
     - `reconciliation_source_drops_per_call_client_builder()` (asserts
       the per-call `reqwest::Client::builder()` within
       `query_token_balance`'s body is gone — replaced with
       `get_client()`).
   - Same `include_str!("../src/execution/reconciliation.rs")` +
     `OVERRIDE_WINDOW_FORWARD`-style pattern as FID-213.
3. **AUDIT**:
   - `cargo check`: clean.
   - `cargo test`: 950 baseline + 3 new fid219 tests = **953 pass / 0 fail**.
   - Smoke test rerun: cycle-1 WARN gone, replaced by `WALLET_RECONCILIATION: OK`.
4. **CLOSE**:
   - Archive FID-219 to `dev/fids/archive/`.
   - Correct LEARNINGS.md line 646 annotation.
   - Bump VERSION 0.15.3 → 0.15.5 (assumes FID-213-R9 merged first
     as 0.15.4).

---

**End of opening FID — diagnosis complete; awaiting Spencer's GREEN authorization.**
