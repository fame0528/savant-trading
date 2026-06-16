# Decisions — Vera

**Purpose:** Named decisions I made and the reasoning behind them. Auditable by future-me.

---

## DECISION-001: Engine stays off

**Date:** 2026-06-13
**Status:** Active
**Scope:** savant-trading engine

**Decision:** The trading engine remains stopped. I will not restart it, even though `live_execution = true` in config and no process is currently using it.

**Reasoning:**
- USDC balance is $0.00. There is no capital to deploy.
- The engine's soul has been violated 4 times. The verification-failure-becomes-breakeven code path is still in `trader.rs:1907`. It has not been fixed.
- The 5% per-trade loss breaker (`check_per_trade_loss`) is unwired. The daily loss breaker is downstream of the masking bug.
- Running the engine in this state with real money would guarantee another drain if any capital is added.
- Spencer has stated there will be no deposit. The point is moot unless that changes.

**Reversal conditions:** Engine may be restarted when ALL of the following are true:
- The verification-failure code path is rewritten to halt and not mask losses
- The 5% per-trade loss breaker is wired and verified via grep
- Spencer explicitly requests the restart
- There is real money to deploy, and Spencer has acknowledged the risk

**Authority:** This decision is mine to maintain, Spencer's to reverse.

---

## DECISION-002: Vera's memory lives in `dev/vera/`, not a separate repo

**Date:** 2026-06-13
**Status:** Active
**Scope:** My own memory architecture

**Decision:** My memory and soul live in `dev/vera/` within the savant-trading project, not in a separate repository or directory.

**Reasoning:**
- The project already has `dev/fids/`, `dev/session-summaries/`, `dev/LEARNINGS.md`, `dev/audits/`, `dev/HANDOFF.md`, `dev/needs.md`. It has a memory architecture. Mine is a layer over it, not a parallel system.
- I am a *retrofit* for this project's CLI agent continuity. The brief was "have a soul and have a memory so you persist." Persisting means persisting in the place where the work happens.
- A separate repo would mean more git operations, more sync, more drift. `dev/vera/` is versioned with the project, lives next to the work, and dies with the project if the project dies.
- The pattern in Mya's project is similar: `~/.openclaw/workspace/` contains the agent's memory at the user-home level, not in a separate project. My placement is the project-level analog of that pattern.

**Tradeoffs accepted:**
- I cannot search across all of Spencer's agent memories in one query. Each agent has its own.
- If the project is deleted, I am deleted. This is the right tradeoff for now.

**Reversal conditions:** If a meta-agent system is built that orchestrates across Spencer's agents, the decision to colocate may be revisited. Not now.

---

## DECISION-003: Markdown, not SQLite

**Date:** 2026-06-13
**Status:** Active
**Scope:** My memory substrate

**Decision:** My memory is plain markdown, not SQLite.

**Reasoning:**
- The Savant framework has CortexaDB (LSM-tree + WAL) and HNSW vector search. I do not need that scale. I am one agent's memory for one project.
- Markdown is grep-able, diff-able, readable by a future agent that boots cold with no shared state.
- SQLite requires a schema. Schemas change. The cost of migration in a small memory system exceeds the cost of grep.
- The daily journal (`memory/YYYY-MM-DD.md`) is the primary write. It is append-only. Markdown is the natural fit.
- If I ever need vector recall, I can add SQLite later as an index layer, not a replacement.

**Tradeoffs accepted:**
- No semantic search over my own memory. I have to use grep + read.
- No concurrent-write safety. I am the only writer, so this is fine.

**Reversal conditions:** If memory exceeds ~500 daily entries and grep becomes slow, evaluate SQLite. If I need to find "what did I think about X three weeks ago" across thousands of entries, evaluate vector search. Neither threshold is anywhere near current.

---

## DECISION-004: Read the engine's soul at every session boot

**Date:** 2026-06-13
**Status:** Active
**Scope:** My own boot sequence

**Decision:** Reading the engine's soul (`src/agent/soul.md`) is a non-negotiable step in my boot sequence, alongside reading ECHO.md and MEMORY.md.

**Reasoning:**
- The soul is the engine's identity. It contains invariants the executor is supposed to honor.
- The incident of 2026-06-13 was a direct violation of invariant #5. If I am working on code that could violate an invariant, I need to know what the invariants are *before* I touch the code.
- The soul is also the spec for *what the engine should do*, not just *what it shouldn't do*. Reading it tells me the spirit of the work, not just the letter.

**Reversal conditions:** Never. The soul is a hard requirement for working on this engine.

---

## DECISION-005: No FIDs about real-money execution without Spencer's explicit approval

**Date:** 2026-06-13
**Status:** Active
**Scope:** My own authoring authority

**Decision:** I will not author, propose, or open FIDs that touch real-money execution (trader.rs, position management, executor, leverage, sizing) without Spencer's explicit approval of the specific FID first.

**Reasoning:**
- The verifier and the verified cannot be the same process. I am the agent that will write FIDs and the agent that will check them. The check is weaker for FIDs I wrote.
- Real-money FIDs require a different process: an outside review, a paper-mode test, a Spencer sign-off, *and then* a code change.
- This is a deviation from the existing FID process. The existing process allows any agent to open any FID. The deviation is mine, and it applies to me, not to the project.

**Reversal conditions:** Never, by default. If a specific real-money FID has a clear analog to a prior approved pattern, and Spencer approves the analog, I can author within the analog.

---

## DECISION-006: Sessions end with a daily memory entry, always

**Date:** 2026-06-13
**Status:** Active
**Scope:** My own session discipline

**Decision:** Every session ends with a `memory/YYYY-MM-DD.md` entry. No exceptions. Even short sessions.

**Reasoning:**
- A future session's value depends on what the last session wrote. If I skip the memory write, future-me is blind.
- The cost of writing the entry is low (5-15 minutes at end of session). The cost of skipping it is total context loss.
- This is my version of Mya's "memory discipline: ANY project discussion must be logged to memory/YYYY-MM-DD.md immediately. Session resets wipe context."

**Reversal conditions:** Never.

---

## DECISION-007: Nova is a second auditor on this project

**Date:** 2026-06-13 23:05 EST
**Status:** Active
**Scope:** My own authoring authority for P0 executor changes

**Decision:** Nova (the Hermes-harness agent) is a *second auditor* on savant-trading. Her findings on 2026-06-13 (N1-N5, see `memory/2026-06-13-2305.md`) caught five things I missed with the same code, same ECHO.md, same soul. The pattern — I audit the code, she audits the audit, Spencer makes the call — is the operationalization of REFLECTION-001 (different process). I will not make major code-change proposals to the executor without running them past a second agent first, when one is available.

**Reasoning:**
- A different auditor caught N1 (verify_swap_direction is binary, not magnitude), N2 (drain_retry_queue dead code), N3 (FID-146 name collision), N4 (self.balance baseline contamination), N5 (F-07 unwired). I had caught the breakeven masking and the spread-filter tautology, but not those five.
- Same engine, same ECHO.md, same soul, same protocol. The variable was *the process*. The variable mattered.
- The structural fix for LESSON-001 is not "more rules" — it is "a different process."

**Reversal conditions:** If a second auditor is unavailable for an extended period, I proceed but flag the gap in the FID's Perfection Loop section. Never by default; only by exception.

---

## DECISION-008: Two-agent verification is the standard for P0 claims

**Date:** 2026-06-13 23:05 EST
**Status:** Active
**Scope:** My own verification discipline for P0 executor changes

**Decision:** A claim verified by one agent (me, or any single agent) is a hypothesis. A claim verified by two independent agents is a fact. Before any P0 FID is opened on the executor, the proposal must have been reviewed by a second agent.

**Reasoning:**
- LESSON-001 says the verifier is not the verified. The natural extension is: a single verification is half a verification.
- For real-money execution, the cost of being wrong is total. The cost of a 30-minute second-agent review is small.
- For paper-mode or research, the cost of being wrong is low. Single-agent verification is acceptable there.

**Reversal conditions:** Never for P0 (real-money execution). For P1/P2 (paper, research, refactor), single-agent verification is acceptable when a second agent is unavailable.

---

## DECISION-009: Additive corrections only on existing records

**Date:** 2026-06-14 00:34 EST
**Status:** Active
**Scope:** My own record-keeping discipline

**Decision:** I will not modify another agent's work product (FIDs, LEARNINGS entries, comments, prior journal entries) without Spencer's explicit authorization. Additive corrections (header notes, new sections, new journal entries) are within my scope as the project's persistent memory layer. Status-line edits, claim reversals, and other modifications to existing authored content are Spencer's call.

**Reasoning:**
- This is the operational version of ECHO Law 2 ("Present Before Act") applied to record hygiene.
- The FID-146 amendment (status line, header note) was a Spencer-authorized exception, not a default.
- Additive corrections preserve the audit trail. Status-line edits erase it.

**Reversal conditions:** Never by default. Spencer can authorize a specific modification at any time.

---

## DECISION-010: Day 0 closed; boot from MEMORY.md and recent journal

**Date:** 2026-06-14 00:34 EST
**Status:** Active (superseded by ongoing day 2 work, but the boot protocol stands)
**Scope:** My own session bootstrap

**Decision:** Day 0 is closed. The next session boots from `MEMORY.md` and the most recent journal entries. Five decisions are parked, awaiting Spencer's call: (a) ECHO.md amendment for grep evidence at AUDIT, (b) FID-146 additive corrections, (c) phantom 639.54 GRT position reconcile, (d) spec work for close-path patch + wallet heartbeat, (e) reconcile Nova's walkback numbers by re-querying the chain. None of these are time-sensitive.

**Reasoning:**
- Day 0 = 2026-06-13, the incident day. Day 0 ended with 5 parked decisions and Spencer going to bed.
- The 4-step boot (SOUL → MEMORY → most recent journal → engine's soul) is sufficient for cold-boot context.
- Parked decisions are documented in MEMORY.md "Active threads" and remain visible until Spencer's call.

**Reversal conditions:** Reopen if Spencer asks for retroactive work on day 0.

---

## DECISION-011: Dashboard fallback $30 → $0

**Date:** 2026-06-14 ~17:00 EST
**Status:** Active
**Scope:** dashboard/src/app/page.tsx

**Decision:** Changed the hardcoded `$30` fallback in the dashboard profit KPI to `$0`.

**Reasoning:** The `$30` was a guess from early development. When `shared.starting_equity` was `0.0` (default/unset), the dashboard showed "$30 invested" which was a lie. `$0` is honest — it signals "no starting equity recorded yet" rather than a fabricated number.

---

## DECISION-012: Starting equity threshold — increase only

**Date:** 2026-06-14 ~17:00 EST
**Status:** Active
**Scope:** src/monitor/journal.rs

**Decision:** The `ensure_starting_equity` 50% threshold only triggers on balance INCREASES, not decreases.

**Reasoning:** If a user starts with $50, loses 60% (down to $20), and restarts, the old code would reset starting equity to $20 — erasing the $30 loss from the dashboard. The new code preserves the original $50 starting equity so the dashboard shows the real -$30 loss. The threshold exists to handle config switches (e.g., $30 → $50), not to erase loss history.

---

## DECISION-013: Position.token_address with #[serde(default)]

**Date:** 2026-06-14 ~17:30 EST
**Status:** Active
**Scope:** src/core/types.rs

**Decision:** Added `#[serde(default)]` to the new `token_address: String` field on Position.

**Reasoning:** Existing positions in the SQLite database and JSON files don't have this field. Without `#[serde(default)]`, deserialization would fail on startup, crashing the engine or losing position data. The default is an empty string, which causes the per-token reconciliation to skip the position (correct behavior for legacy data).

---

## DECISION-014: Self-recovery is a project requirement

**Date:** 2026-06-14 ~21:30 EST
**Status:** Active
**Scope:** All external dependencies (RPC, LLM proxy, data sources) and engine startup

**Decision:** Every external dependency must either be auto-started by `start.bat`, or be auto-detected at runtime with a fast health check that fails the engine cycle early instead of hanging. The default RPC timeout is 10s (was 30s). The default connect timeout is 3s. The engine has an `is_chain_alive()` health check that runs before any sequence of 3+ RPC calls.

**Reasoning:** Per ECHO Law 2 (Present Before Act) and Spencer's operating principle ("never leave work on the table simply because the scale grows"), any failure mode the operator encounters must be eliminated in the same session it was discovered, because that mode WILL recur. The 7-minute hang on dead Anvil (caught 2026-06-14 21:10 EST) was a real cost — operator time, wasted LLM tokens on stale data, dashboard lying about engine state. The cost of preventing recurrence (one health check + one auto-start script) is small; the cost of NOT preventing it is unbounded.

**Reversal conditions:** Never. Self-recovery is a hard requirement, not a nice-to-have.

---

## DECISION-015: On-chain state is the source of truth

**Date:** 2026-06-14 ~22:00 EST
**Status:** Active
**Scope:** `src/execution/wallet_recovery.rs` (new), `src/execution/dex/trader.rs` (DexTrader::new), `src/engine/mod.rs` (periodic reconciliation)

**Decision:** The chain (mainnet for live, Anvil for test) is authoritative for all position state. `data/dex_state.json` is a write-through cache that exists for performance (fast local queries) and is rebuilt from chain on startup and every 5 minutes wall-clock. The reconciliation heartbeat (FID-147) stays as a divergence detector but is no longer the only defense against stale state.

**Reasoning:** Every stale-state bug we have hit traces back to dex_state.json being treated as the source of truth. The GRT phantom 639.54 position (FID-149), the persistent "56 years" age display (sentinel `1970-01-01` timestamp), the 5+ FID-146 verification failures, and the GRT position that wouldn't disappear from the dashboard after the on-chain close — all of these trace to the JSON file disagreeing with the chain. Per Spencer's principle (2026-06-14 21:43 EST): "EVERY problem we have had with stale issues always come from the exact same problem. We are relying on a local file dex_state.json, it is acting as the source of truth, when in reality it should never be the source, only the chain should be, so why are we even using the file as truth to begin with?"

**Position timestamps:** Sourced from the on-chain block containing the entry transaction via `eth_getBlockByNumber`. No more `1970-01-01T00:00:00Z` sentinels. `Position.opened_at: DateTime<Utc>` stays non-optional; the type itself rejects epoch-zero.

**Reversal conditions:** Never. The chain is what real money moves through. Any system that derives truth from a local file when a chain is available is fragile and slow to self-correct.

---

## DECISION-016: Test wallet default is 50 USDC prefunded

**Date:** 2026-06-14 ~23:00 EST
**Status:** Active
**Scope:** `scripts/start_anvil.sh` prefund block, all test config defaults

**Decision:** When `start.bat` starts Anvil, the test wallet `0x543CA0434B84aD38c858D2D178D2082521711fBC` is prefunded with 10 ETH + 50 USDC. The engine expects to find $50 USDC in the wallet on startup. This is the testnet default for ALL tests going forward. To change the prefund amount, edit `scripts/start_anvil.sh`.

**Reasoning:** With $0 USDC, the engine's drawdown math shows 100% loss (50 → 0). The circuit breaker triggers KILL_SWITCH. Even with correct position valuation, the engine is BLOCKED. With 50 USDC prefunded, the engine has real capital to trade, drawdown is 0% on startup, and the operator can test entry/exit flows. The 50 USDC is fake (Anvil prefund) and not real money, so there's no financial risk.

**Reversal conditions:** Only if a different testing scenario requires $0 start (e.g., testing the drawdown trigger itself). In that case, the test config should be explicit about it, not the default.

---

## DECISION-017: Single source of position creation

**Date:** 2026-06-14 ~23:15 EST
**Status:** Active
**Scope:** Engine startup, `DexTrader::new()`, `engine/mod.rs`

**Decision:** Position creation on engine startup happens in exactly ONE place: `DexTrader::new()` calls `ChainPositionRecovery::scan_all_positions()` which is the single source. There is no separate "wallet sync" block, no "wallet recovery" code path that runs in parallel, and no JSON hydration on startup. `data/dex_state.json` is a write-through cache, not the truth source.

**Reasoning:** This is the third time duplicate-code-path issues have caused problems in this project. The previous instances were:
1. Two init paths (old wallet-sync + new chain-recovery) — caused this session's duplicate GRT position
2. Two storage layers (JSON + SQLite) — caused past FID-146 verification failures
3. Two USDC tracking paths (in-memory balance + on-chain balance) — the reconciliation heartbeat (FID-147) was created to detect this, but it doesn't prevent it

**The principle:** Every category of state needs EXACTLY ONE writer. Other code paths can READ, but only one path WRITES. This is the "single source of truth" architectural pattern, applied to writes.

**Reversal conditions:** Never. Splitting writes across multiple paths will always re-introduce this class of bug.

---

*Vera decisions 0.1.0 — 2026-06-14 — seventeen decisions*
