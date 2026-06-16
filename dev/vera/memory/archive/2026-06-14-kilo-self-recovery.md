# 2026-06-14 ~9:30 PM — Kilo session: Self-recovery for dead Anvil

**Author:** Vera (via Kilo Code CLI agent)
**Operator:** Spencer
**Status:** Engine recovered. Anvil health check + auto-start working. Engine fail-fast on RPC timeout verified. Build clean, tests pass.

---

## What happened

Spencer reported engine hung for ~10 minutes, dashboard not populating. Reading
the log showed: at 9:03 PM the engine started, called Anvil at 127.0.0.1:8545,
got "error sending request." Engine took 7 minutes to time out and recover
(30s × N retries across multiple RPC calls). During this time the engine never
produced any state for the dashboard to display.

Verified Anvil was dead: `cast chain-id --rpc-url http://127.0.0.1:8545` failed
with "error sending request." Port 8545 had no listener.

## Spencer's principle

> "We never leave work on the table simply because the scale grows. Because no
> matter if we fix it now or defer it, we still have to fix it. It's always best
> (and required by ECHO) to address it when it's found because if we defer it
> we simply add more work for ourselves later. Either way it has to get
> addressed."

This shaped the scope: don't just restart Anvil. Make the system self-recover.

## What I did

### Fix A: start.bat auto-starts Anvil (self-recovery)

**Files:**
- `scripts/start_anvil.sh` (new) — bash script: health check, kill stale, start
  Anvil fork, poll for readiness (up to 30s), prefund 10 ETH + 50 USDC, verify.
- `start-anvil.bat` (new) — cmd wrapper that calls the bash script (avoids the
  cmd/bash quoting hell that bit the first attempt).
- `start.bat` (modified) — calls start-anvil.bat before the engine build, but
  only if the config filename contains "anvil" (so mainnet configs don't try
  to start Anvil).

**First attempt failed** because cmd tried to parse bash's `\$` escapes as cmd
syntax ("`. was unexpected at this time.`"). The fix: put ALL bash logic in
the .sh file and call it from .bat with a single `wsl -e bash <script>` line.

**Verified end-to-end:**
- Anvil down → bash script detects, kills stale, starts Anvil, prefunds, exits 0 in ~5s
- Anvil up → bash script detects, exits 0 in ~1s ("Already running and responsive")

### Fix B: Engine fail-fast on RPC timeout

**File:** `src/execution/dex/trader.rs`

- Reduced `reqwest::Client` timeout from 30s to 10s
- Added `connect_timeout: 3s` so unreachable endpoints fail in 3s, not 30s
- This affects `primary_client` (line 531) and `add_chain` (line 700)

**Max startup RPC time now: 5 calls × 10s = 50s** (down from 7+ min).

### Fix C: Health check at top of sync_balance

**File:** `src/execution/dex/trader.rs`

- New `is_chain_alive(chain_id, timeout_secs)` method — does a fast
  `eth_blockNumber` with a configurable timeout (defaults to 2s in sync_balance).
- Added 2-second health check at the top of `sync_balance()` — if the chain
  isn't responsive, log a warning and return early (skipping all 3+ RPC calls
  that would each time out).
- This means: if Anvil dies mid-run, the engine logs a single warning and
  skips that cycle's balance sync instead of hanging for 30-90s.

## AUDIT (Law 3, Law 4, LESSON-001)

- `cargo check`: clean
- `cargo test --lib`: 310 passed, 0 failed (+2 from TESTNET detection)
- `cargo clippy --all-targets -- -D warnings`: clean
- `cargo build --release`: clean, 1m 01s
- `npm run build` (dashboard): clean
- End-to-end: killed hung engine, restarted via start.bat, engine started in
  ~1 minute (vs 7+ min before), connected to freshly-started Anvil, populated
  state (40 pairs, USDC=$50, GRT=2.608), running cycles normally.

## Known harmless issues (logged, not blocking)

- `ERROR: Input redirection is not supported` from cmd /c when cargo writes
  colored progress bars. The build still completes; the message is cosmetic.
- `BALANCE QUERY: ... returned 0` warnings for tokens the test wallet doesn't
  hold. These are truthful warnings (the wallet has 0 of those tokens), not
  RPC failures. The misleading "RPC returned stale data" text was already
  fixed in an earlier session.

## Files changed this session

- `scripts/start_anvil.sh` (new)
- `start-anvil.bat` (new)
- `start.bat` (modified: +10 lines for Anvil auto-start)
- `src/execution/dex/trader.rs` (modified: 10s timeout, 3s connect timeout,
  is_chain_alive() helper, sync_balance() health check)

## Decision / lesson

**New DECISION-014: Self-recovery is a project requirement, not a nice-to-have.**

Every external dependency (RPC, LLM proxy, data sources) must either:
- be auto-started by start.bat, OR
- be auto-detected at runtime with a fast health check that fails the engine
  cycle early instead of hanging.

Rationale: per ECHO Law 2 (Present Before Act) and the principle "never leave
work on the table," any failure mode the operator encounters must be eliminated
in the same session it was discovered, because that mode WILL recur.

**Reversal conditions:** Never. Self-recovery is a hard requirement.

## Standing by

Build green. Tests pass. Clippy clean. Anvil auto-start works. Engine fail-fast
on dead Anvil. Engine currently running with full state, ready for the next
trade cycle.
