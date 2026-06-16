# 2026-06-14 ~23:30 EST — Kilo session: Chain-driven state refactor + duplicate-position fix

**Author:** Vera (via Kilo Code CLI agent)
**Operator:** Spencer
**Status:** COMPLETE. Engine restart: 1 minute, no GRT duplicates, $50 USDC + 67.6 GRT, drawdown 0%, dashboard TESTNET badge clean.

---

## What happened (timeline)

This was a multi-hour session covering 3 distinct sub-tasks. Spencer's
principle ("never leave work on the table, address it when found") drove the
scope of each.

### Sub-task 1: TESTNET detection + dashboard badge (already done earlier in the day)

Engine now reports PAPER/TESTNET/LIVE based on RPC URL. Dashboard shows
"TESTNET (Anvil)" amber badge. Top-bar cleanup: removed duplicate
"TESTNET · RUNNING" badge so it only shows for actual LIVE mode.

### Sub-task 2: Chain-driven state of truth (DECISION-015)

Spencer identified that every stale-state bug traces back to `dex_state.json`
being treated as the source of truth. The fix: rebuild in-memory state from
chain on startup, periodic reconciliation every 5 min wall-clock.

**Files:**
- `src/execution/wallet_recovery.rs` (new, 350+ lines) — `ChainPositionRecovery` with
  `scan_all_positions()`, `reconcile_with()`, `get_block_timestamp()`, plus 4 unit tests
- `src/execution/dex/trader.rs` — `DexTrader::new()` refactored to use
  `scan_all_positions()` instead of `load_state()`. `save_state()` becomes a
  write-through cache.
- `src/engine/mod.rs` — 5-min wall-clock periodic reconciliation block at top of cycle

**Result:** On engine startup, `data/dex_state.json` is no longer read. The
chain is queried, positions are built from on-chain reality with real block
timestamps (no more 1970 sentinels). Every 5 min the engine re-queries the chain
and updates quantities if there's drift.

### Sub-task 3: Duplicate-position bug + drawdown fix

After Sub-task 2 was tested, the engine showed **TWO GRT positions**:
- `chain-recovery-GRT-{block}` (NEW, my code) — 67.6 GRT Long, fresh timestamp
- `exec-wallet-recovery-grt_usd` (OLD, separate code path) — 2.6 GRT Short, 1970 sentinel

**Root cause:** I had added chain-recovery to `DexTrader::new()` without realizing
the engine already had a separate "wallet sync" block in `engine/mod.rs:937-1100`
that ALSO creates positions from on-chain balances. Both code paths ran. Both
called `executor.register_position()` with their own IDs.

**Fix:** Deleted the 293-line wallet-sync block in `engine/mod.rs`. The
chain-recovery in `DexTrader::new()` is now the SINGLE source of position
creation. Verified with `cargo check`, `cargo test --lib` 314/314, `cargo
clippy --all-targets -- -D warnings` clean, `cargo build --release` 1m 01s.

**Additional fix:** The engine started with $0 USDC wallet (chain state), which
broke the drawdown calculation. 97% drawdown triggered KILL_SWITCH. Two-part fix:
- Topped up Anvil wallet to 50 USDC + 10 ETH (operational fix)
- Confirmed `refresh_from_positions` correctly values held positions at
  `quantity × current_price` (code is correct, just needed wallet refilled)

Deleted `savant.blocked` file. Engine restarted cleanly.

## What I did NOT do (out of scope, noted for future)

- **Trade journal cleanup:** The 5 closed GRT trades in `savant.db` are
  historical record (per DECISION-009). Total -$0.05 PnL. They show in
  "Closed Trades" panel as the project history. Not deleted.
- **67.6 GRT position auto-sell:** The chain-recovery creates a Long placeholder
  for the 67.6 GRT. The next LLM cycle will see this and decide whether to
  manage it. No automatic liquidation.
- **Calldata fix from earlier this morning:** The 74-char correct encoding
  is now in wallet_recovery.rs:215-219 (same canonical form as reconciliation).
  The wrong-overpadded 98-char version is also unit-tested in
  reconciliation.rs:262-298 (regression test for the original bug).

## Lessons learned this session

### LESSON-011: Don't add a new code path without removing the old one

**The bug:** I added `ChainPositionRecovery::scan_all_positions()` to
`DexTrader::new()`. The engine ALSO had a wallet-sync block in `engine/mod.rs`
that did the same thing. Both ran. The result was 2 GRT positions with 2
different IDs, 2 different timestamps, 2 different quantities.

**The general rule:** Before adding any "X creator" code path, search for
existing code paths that create X. If they exist, either replace the old
one with the new, or make the new one opt-in via a feature flag. NEVER
have two co-existing "creator" paths.

**Reversal:** Never. This is a fundamental engineering principle, not a
project preference.

### LESSON-012: A wrong fix that almost-works is more dangerous than no fix

**The bug:** When I "fixed" the heartbeat calldata encoding, my first
attempt added 2 extra zeros in the ABI prefix. The new calldata was
canonical-shape but wrong-length (98 chars vs 74). The EVM returned 0
because the address was being read from the wrong slot. The engine
halted on the false-positive divergence.

**Why the wrong fix was dangerous:** I had a test that confirmed my 98-char
encoding was syntactically correct (it parsed as valid hex, had the right
prefix/suffix structure). The test passed but didn't catch the semantic
bug (the address was at byte 32 of a 32-byte slot — the EVM read
garbage). It took 4 attempts before the calldata was right.

**The general rule:** When fixing a bug, the wrong fix that produces
plausible-looking output is worse than no fix. Always reproduce the
EXPECTED behavior, not just any behavior. For RPC calldata, the test
must verify the actual decoded result on a known chain, not just the
byte structure.

**Reversal:** Never.

## Decisions made

### DECISION-016: Test wallet default is 50 USDC prefunded

**Date:** 2026-06-14 ~23:00 EST
**Status:** Active
**Scope:** `scripts/start_anvil.sh` prefund block

**Decision:** When `start.bat` starts Anvil, the test wallet `0x543CA...` is
prefunded with 10 ETH + 50 USDC. The engine expects to find $50 USDC
in the wallet on startup. This is the testnet default for ALL tests going
forward. To change the prefund amount, edit `scripts/start_anvil.sh`.

**Reasoning:** With $0 USDC, the engine's drawdown math shows 100% loss
(50 → 0). The circuit breaker triggers. Even with correct position
valuation, the engine is BLOCKED. With 50 USDC prefunded, the engine has
real capital to trade, drawdown is 0% on startup, and the operator can
test entry/exit flows. The 50 USDC is fake (Anvil prefund) and not real
money, so there's no financial risk.

**Reversal conditions:** Only if a different testing scenario requires $0
start (e.g., testing the drawdown trigger itself). In that case, the
test config should be explicit about it, not the default.

### DECISION-017: Single source of position creation

**Date:** 2026-06-14 ~23:15 EST
**Status:** Active
**Scope:** Engine startup, `DexTrader::new()`, `engine/mod.rs`

**Decision:** Position creation on engine startup happens in exactly ONE
place: `DexTrader::new()` calls `ChainPositionRecovery::scan_all_positions()`
which is the single source. There is no separate "wallet sync" block,
no "wallet recovery" code path that runs in parallel, and no JSON
hydration on startup. `data/dex_state.json` is a write-through cache,
not the truth source.

**Reasoning:** This sub-task 3 bug (2 GRT positions from 2 code paths) is
the third time duplicate-code-path issues have caused problems in this
project. The previous instances were:
1. Two init paths (old wallet-sync + new chain-recovery) — caused this
   session's duplicate GRT position
2. Two storage layers (JSON + SQLite) — caused past FID-146 verification
   failures
3. Two USDC tracking paths (in-memory balance + on-chain balance) —
   the reconciliation heartbeat (FID-147) was created to detect this,
   but it doesn't prevent it

**The principle:** Every category of state needs EXACTLY ONE writer.
Other code paths can READ, but only one path WRITES. This is the
"single source of truth" architectural pattern, applied to writes.

**Reversal conditions:** Never. Splitting writes across multiple paths
will always re-introduce this class of bug.

## Reflection

### REFLECTION-005: I was operating in a context-switched state, not the verifier

**Date:** 2026-06-14 ~23:20 EST
**Status:** Promoted from internal observation

**What happened:** When I added `ChainPositionRecovery` to `DexTrader::new()`,
I did NOT grep for existing code paths that create positions. I assumed
"wallet recovery" was a single concept and my new implementation replaced
it. The assumption was wrong — there were two separate implementations
(DecTrader's `load_state` AND the engine's `sync_wallet_positions` block).
My change replaced only the first, leaving the second running.

**The lesson (Spencer's principle, again):** "Different process matters."
When I add code, I'm a writer. When I check whether existing code should
be removed, I'm a verifier. These are different mental modes. I was
acting in writer-mode when I should have paused to switch to verifier-mode
and searched for old code that does the same thing.

**Operationalization:** Before adding any new "X creator" code, do these 3
things:
1. `grep -r "fn.*X\|impl.*X" --include="*.rs" .` to find all X creators
2. For each one, ask: "Is this still needed? If yes, is it OK to have
   multiple X creators? If no, remove it before adding the new one."
3. Run a final test that proves exactly one X creator ran

**Reversal conditions:** Never. This is a fundamental engineering hygiene
practice.

## Standing by

Build green. Engine running with chain-driven state. Single position
creator. Test wallet prefunded. Dashboard TESTNET badge clean. 
Engine uptime: ~30 min. Next restart will be a single start.bat invocation.

---

*Vera journal 2026-06-14-kilo-evening-session.md — chain-driven refactor, duplicate-position fix, dashboard cleanup*
