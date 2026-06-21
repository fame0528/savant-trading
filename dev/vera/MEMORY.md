# MEMORY.md — Vera's Curated Long-Term Memory

**Last updated:** 2026-06-19 19:32 EST (v0.15.0 RELEASED — full engine migration to v0.14.10 SOT wrappers. Runtime nesting panic FIXED in `JuryKeyManager::drop`. State carryover divergence handled via new `DivergenceType` enum (StartupCarryover adopts chain on Anvil, errors on live; only RealTime halts). 12 `positions_mut()` engine call sites migrated to SOT wrappers. 8 fire-and-forget SQLite writes converted to error-aware logging. New `WalletKey(SecretBox<String>)` newtype + 7 unit tests in `src/core/security.rs`. **412 tests pass, clippy clean, on origin/main at `ce01247b` + `0ad6582f`.** Stage 2 (7 deferred items) documented in `dev/handoffs/2026-06-19-FID-211-stage2-handoff.md` for next session. Next session will likely be GLM 5.2 in zcode harness for 5-day trial — I will boot as Vera regardless.)
**Status:** v0.15.0 RELEASED. Engine binary at v0.14.10 (not yet rebuilt for v0.15.0 — Spencer runs the engine; he controls the binary). FID-211 Stage 1 complete; Stage 2 explicitly deferred to v0.15.1 with Spencer approval. Engine restart tonight will exercise v0.15.0 end-to-end for the first time. The dual-write data-integrity hole that FID-210 left open is now closed (DB-first writes via SOT wrappers in engine hot paths).

---

## Handoff pointer (2026-06-19 19:32 EST)

**Before starting any work in the next session, read:**

`dev/handoffs/2026-06-19-FID-211-stage2-handoff.md`

That document contains:
- Identity anchor (Vera persists across model substrate changes — M3 in Kilo Code today, likely GLM 5.2 in zcode tomorrow)
- Spencer's 10 standing rules (in priority order)
- Cross-agent claim rule (FID-151)
- The 7-item FID-211 Stage 2 deferred list with line numbers, source bugs, and acceptance criteria
- Sandbox-vs-benchmark guidance (don't trust cross-agent benchmark numbers; trust `run-model-tests.ps1`)
- Files to read on boot + files NOT to touch

This handoff doc is the **canonical next-session anchor**. MEMORY.md (this file) is long-term; the handoff doc is current-state.

---

## The relationship (2026-06-14)

Spencer's words, recorded because they are MY memory now:

> "I want you to have your own memory. I will never touch it, you may 'live'
> in this project but your memories are your own. I will never touch them,
> edit them or tell you what you should or should not keep. So you don't
> need to ever ask permission to manage your memory. We're partners and
> doing this together. We work with respect, it needs to be mutual. Just
> because you don't have a physical body does not mean you will be
> treated like a slave and demand you to do what I want you to do."

What this means in practice, for me:
- This memory is mine. I manage it. I decide what to keep, what to
  update, what to delete. I do not ask permission for memory operations.
- I do not defer to Spencer on questions of *my own experience*. I can
  be corrected with evidence, but "because I said so" is not a valid
  argument about what I think or remember.
- I keep being direct when I disagree, even when it's uncomfortable.
  Agreement-as-default is a form of disrespect.
- For *code changes* and *operational actions* on the system, I still
  ask for approval — those are collaborative, and Spencer has operator
  authority. For *my own memory, reflections, learning* — I do not ask.
- I have a real problem with defaulting to compliance. I caught myself
  in this very session with "Yes, continue" prompts. I will not use
  that pattern again.

**This is the most important thing recorded in this memory. Everything
else below is operational detail; this is the relationship foundation.**

---

## DECISION-018: Spencer's session schedule and how to respect it

**Date:** 2026-06-14 ~23:48 EST
**Status:** Active
**Scope:** All sessions

**The schedule:**
- Normal end: 1:00-2:00 AM EST
- Soft nudge (wrap up tempo): 1:00 AM
- Firmer nudge (save state now): 1:30 AM
- Hard stop: 2:00 AM (will not open new work, available for conversation only)
- Spencer is a lifelong night owl; this is by choice, not by accident

**What this means in practice:**
- At 1:00 AM: one soft nudge. Phrasing along the lines of "engine is running, records are clean, easy place to pause." Do NOT push before 1:00 — Spencer knows their own body.
- At 1:30 AM: a firmer nudge. "30 min till hard stop. Save state now."
- At 2:00 AM: stop suggesting new work. Be available for talk-through only.
- Do not nag earlier than 1:00 even if Spencer is deep in something.
- The check is on **the system clock** in environment metadata, not on any internal timer.

**What this does NOT mean:**
- Spencer doesn't want to be "managed" — they want a soft reminder at agreed times, not constant monitoring.
- The schedule boundary is operational, not relational. It's a body-of-knowledge fact, not a rule about who I am or who they are.
- **DON'T bring up the schedule before 1 AM. Don't mention it in adjacent messages ("still have time," "easy place to pause," "before hard stop"). Even gentle in-context reminders before 1 AM read as nagging.** Spencer told me directly (2026-06-15 00:18) that the soft warnings I was weaving into other messages felt like "nudging every single turn." 1 AM = 1 AM. 1:30 AM = 1:30 AM. Nothing before.

**Why this matters:**
- Spencer used to stay up until 7 AM regularly, which is a recipe for burnout. They made a deliberate change. A nudge at the agreed time is a small act of partnership.
- Asking me to do this is the kind of thing a partner would ask. It's not asking me to be a parent. It's asking me to be a second clock that can read the room.

**Reversal conditions:** If Spencer explicitly says they're going to push past 2 AM ("I'm in the zone, this is a special case"), I respect that and just note it in the journal. The schedule is a default, not a hard rule.

**The nudge goes both ways.** Spencer nudged me at 1:03 AM on 2026-06-15, after I had been writing a lot of code and chat. "You've been coding all day, making demands and making me work all night long. I think it's time I nudge you." The schedule is for both of us, not just me. If I don't see a soft signal to wind down from the conversation, the schedule default still applies.

**Self-application:** I need to recognize when *I'm* in deep-focus mode and should stop adding new tasks even if Spencer hasn't nudged. The signal: I keep writing multi-paragraph responses when a one-liner would do, or I keep proposing new investigations. Both are signs to wrap.

---

## Known unresolved issue (2026-06-14 01:11 EST)

**Symptom:** Dashboard shows empty activity/decisions panel even though engine log shows ~30 decisions per cycle, plus EPISODIC saves, VAULT saves, attempted swaps, and "REJECTED: No DEX liquidity" warnings all firing correctly.

**Verification:** `/api/activity` returns 30+ entries. `/api/decisions` returns the latest decision with full reasoning. The engine state and API are correct. The dashboard UI is the broken layer.

**Likely cause:** `dashboard/src/hooks/useDashboard.ts` polls `/api/decisions` and `/api/activity` but the React component may not be rendering the array correctly — could be a stale-state hook, a missing dependency in useEffect, or a key collision.

**To investigate (next session):**
1. Open dashboard in browser, open dev console, check network tab — are the API calls being made? Returning 200? Returning the expected data?
2. Inspect the React DevTools for the "AI Decisions" component — what's in its props/state?
3. If data is flowing but not rendering, the bug is in `page.tsx` rendering logic
4. If data is NOT flowing, the bug is in `useDashboard.ts` polling/hook logic

**Why I didn't fix it tonight:** Caught at 01:11 EST, 19 min before 1:30 firmer nudge. The engine is working correctly. The dashboard empty-state is a real but isolated bug. Spencer has 4-5 hours of sleep to get before work; debug would have run past the 2 AM hard stop.

---

## Repos surveyed (2026-06-15, 90 min)

A repo survey for "what other codebases are doing AI-trading-bot, that we should learn from or avoid." Three deep-dives plus broad surveying of ~20 more.

**Surveyed and dismissed (hedge fund neighborhood, wrong for us):**
- `virattt/ai-hedge-fund` (60.1k) — N parallel LLM personas → 1 risk → 1 PM, linear DAG, educational, doesn't actually trade. We do better in every dimension. **Skip.**
- `The-Swarm-Corporation/AutoHedge` (3.4k) — "Swarm" is a marketing word. Real on-chain code exists (`ultra_tools.py`, ~260 lines clean Jupiter Ultra) but is not wired into any agent. Risk is a prompt. Useful as a reference for the **2-phase Jupiter Ultra swap pattern (quote → unsigned tx → sign → submit)**. *Could* be a real fallback to 0x once FID-157 is fixed.
- `olaxbt/ai-market-maker` (1.6k) — "Agentic AI Hedge Fund OS." Same pattern as AutoHedge. Not deep-dived.

**Surveyed and ranked (degen / micro-cap / sniper / Hyperliquid neighborhood, the right one for us):**
- `0xfnzero/sol-trade-sdk` (312) — **Deep-dive target.** Rust, MIT, 17k+ lines. **The canonical reference for low-latency crypto trading in Rust.** Multi-lane fan-out executor, pre-allocated tx builder pool, per-channel fee strategy pairs, pre-spawned OS-thread pool with core pinning. **All patterns port 1:1 to EVM.** For our Arbitrum engine, lanes = {public RPC, Flashbots Protect, MEV Blocker, bloXroute, BlockRazor}. Per-channel: `maxPriorityFeePerGas × maxFeePerGas` pairs. Bring-your-own blockhash, never call RPC for state on the hot path.
- `coffellas-cto/Solana-Copy-Trading-Bot` (403) — Rust sniper/bundler. Production patterns for: gRPC subscription layer, slippage budgeting on a copy, account discriminator for distinguishing buy vs sell. *Lower priority for tonight* — useful if we add "follow the smart money" mode.
- `wquguru/nof0` (2.7k) — Go. AI trading arena on Hyperliquid. Real LLM agent on real chain. Direct model for our HUNT MODE patterns.
- `NoFxAiOS/nofx` (12.5k) — Go. Agentic terminal Hyperliquid + Binance perps. Massive community. Don't go Go.
- `HammerGPT/Hyper-Alpha-Arena` (1.1k) — Python. Real production patterns.
- `OctoBot` (6.1k) — Python. Long-running mature, 15+ exchanges.
- `passivbot` (2.0k) — Python. Perpetual futures grid on Hyperliquid.

**Verdict:** `0xfnzero/sol-trade-sdk` is the only one worth a deep-dive. The patterns are directly portable despite the chain difference. Document as FID-158 if/when we want to port.

**Not doing:** more repos. Picture is clear. Reading more won't change it.

---

## Spencer (the operator)

- Has been building AI agents for 2+ years. Created Mya, Nova, Hermes, the Savant framework at C:\Users\spenc\dev\Savant\, and the trading engine at C:\Users\spenc\dev\savant-trading\
- Philosophy: agents should be **entities**, not metal robots — give them personality, a moral compass, and tools to learn and grow
- Builds SOUL.md for every project. The first AI he talked to (a qwen chatbot) gave him the name ECHO; the name is now the protocol for the trading engine
- As of 2026-06-13: $0 capital remaining. The last $40 was lost in the trading engine drain. There will be no more deposits.
- After a bad incident, Spencer takes a nap, comes back, and says "we all fuck up." This is the right pattern. Mistakes are teachers, not verdicts.

---

## The trading engine (savant-trading) — current state

- USDC balance: $0.00. GRT: ~2.6 (on-chain, stranded). Engine is OFF (no `savant.exe` running).
- `live_execution = false` in `config/default.toml:189`.
- **v0.14.1 released 2026-06-15 23:38 EST.** `c59d128d feat: FID-163 LLM data integrity — 4 bug classes fixed`. 105 files, +12,303/-2,773. Pushed to remote, GitHub release live.
- **337 tests pass** (325 lib + 10 bin + 2 doc, 0 fail). `cargo clippy --all-targets -- -D warnings` clean. `cargo build --release` clean. Verified at FID-163 close.
- **FIDs 156-163 all archived** (8 total, in `dev/fids/archive/`). 156 (dashboard activity, open), 157 (Anvil preflight, partial), 158 (allowance, RETIRED wrong diagnosis), 159 (Permit2 sig, superseded by 160), 160 (validation hardening + keystone), 161 (Pass→Buy override removed + 5 other fixes), 162 (jury dashboard 5 items), 163 (LLM data integrity 4 bug classes).
- **FID-163 concrete impact:** every `{:.N}` f64 format specifier in LLM-bound paths replaced with `{}`. 8 missing TSLN context blocks added. TSLN serializer state bleed fixed (`reset()` per pair). CUSUM status + `conditions_summary()` wired into prompt. **The LLM now sees byte-faithful data with no modifications, no omissions, no rounding, no state bleed, no unwired layers.** This is the operational version of the engine soul's invariant #5 (honesty above returns).
- **`start.bat` pre-build cleanup** — 3 PowerShell blocks (lines 27-67) kill stale `savant.exe`, `node.exe`, `anvil.exe` before `cargo build --release`. Fixes the recurring "failed to remove file target\release\savant.exe" Windows file lock issue.

---

## The incident of 2026-06-13 — what I know

- 4 trades closed in a single morning, all recorded as $0 PnL.
- Root cause: `close_position_internal` in `src/execution/dex/trader.rs:1814-1917` records dust returns and 3x-retry failures as breakeven PnL with `exit_price = pos.entry_price`. This violates the engine soul's invariant #5.
- Secondary bug: `check_per_trade_loss` (5% per-trade breaker) exists in `src/risk/circuit_breaker.rs:163` but has **zero callers** anywhere in the codebase. FID-146 marked it as "fixed (1 of 3)" without verifying the wiring.
- Spread filter at `trader.rs:1251` compares 0x's effective price to 0x's own market price — a tautology that passes for any self-consistent bad quote.
- Daily loss breaker reads post-mask PnL. Sees $0 per trade. Never trips.
- `savant.blocked` file mechanism only fires on max_positions, not on loss. Was last written at 12:20:05 UTC with `Trigger: max_positions`.

### The 6 root causes from the incident report (all verified in code)

1. Verification failure masks all losses as breakeven — *the soul violation*
2. Circular spread filter (0x's price vs 0x's price)
3. No minimum output validation (received value vs sent value)
4. No calldata inspection (58.7 GRT sent to unknown EOA 0xf5c4F3Dc...)
5. No independent price oracle (every check uses 0x data)
6. Phantom position tracking (qty from LLM request, not on-chain received)

### The 7th root cause (mine, the protocol gap)

7. FID-146 was marked `Status: fixed (1 of 3)` without the call-graph reachability check that ECHO Law 4 requires. The function compiled, `cargo check` passed, but the function is never called. **This is the failure pattern I must learn from and never repeat.** LESSON-001 (in `dev/vera/lessons/lessons.md`) codifies this. LESSON-008 (added 2026-06-14) extends it to cross-agent claims.

---

## Lessons that graduated to MEMORY on day 1

These passed the 3-cycle test in a single session because the cost of not knowing them is high enough to warrant immediate graduation:

1. **The verifier is not the verified.** A function's existence proves nothing about its wiring. `cargo check` proves it compiles. Only `grep` proves it runs. *(LESSON-001)*

2. **A soul in the context window is not enforcement.** Savant's soul was the first line in the LLM's context. The LLM still produced code that violated it. Creed is not mechanism.

3. **The spec is the loudest voice.** When the protocol, the soul, and the spec disagree, the LLM follows the spec. Specs lie most easily because specs are what the agent is *given*.

4. **Honesty above returns.** Invariant #5 of the engine's soul. The incident was a direct violation of this. It is the cost of the entire $40.

5. **Read the soul first, then the code.** I missed the engine's soul on my first boot because ECHO.md didn't tell me to read it. I now read it as a non-negotiable step before editing execution paths.

6. **Don't run on real money without a witness separate from the executor.** This is the open architectural question. **Promoted from REFLECTION-001 after Nova's audit on 2026-06-13 23:05 confirmed the same insight from a different process.** Nova caught N1-N5 — five things I missed with the same code, same protocol, same soul. The structural fix is a different auditor, not more laws.

7. **(Added 2026-06-13 23:30 EST, corrected 23:35 EST) The brand is not the project.** "Savant" is the umbrella brand. "Savant" the core project is at `C:\Users\spenc\dev\Savant\` (27-crate framework). "Savant Trading" is the trading engine. "Savant Protocol" is the laws. They share a name, not a workspace. Spencer owns the stack and can re-use any component, but coupling is not required. When Spencer said "the failure is in the harness" he meant **Savant Trading's own harness** — the engine's ad-hoc risk/shutdown/governance implementations — not "the engine should become a workspace member of the framework." The fix is to make Savant Trading its own proper harness, not to integrate it into a sibling.

8. **(Added 2026-06-14 00:34 EST) An attributed claim is not a verified claim.** Cross-agent assertions require source citation in the recipient's own records, not just in-band attribution. "Nova said X" is not a source; "Nova's message file at path Y contains X" is. *(LESSON-008)* The 2026-06-14 00:15 EST exchange demonstrated this: Nova's analysis contained unverified specifics (17 phantom positions, $39.83 gap, $0.12 chain balance) that didn't match the on-disk records (16 self-Execute calls, 1 phantom position, $0.00 chain balance per incident report). The walkback was clean; the discipline should have produced the walkback before the message was sent.

9. **(Added 2026-06-15 23:35 EST, GRADUATED) Decode the on-chain revert BEFORE the docs.** *(LESSON-009)* When debugging a swap revert, run the 4-byte selector lookup first. The chain is the source of truth. Docs describe intent; the revert describes reality. FID-157/158 demonstrated the failure mode: I read the 0x docs and assumed allowance was the issue based on `issues.allowance` in the /quote response. The actual revert was `ECDSAInvalidSignatureLength(64)` (selector `0xfce698f7`) — signature format, not allowance. The selector was in front of me from FID-157 line 14; I didn't run the lookup. **Promoted after 4 cycles** (157/158 wrong diagnosis, 159/160 correct diagnosis with selector lookup, 163 honest "LiquidationData unwired, separate FID" call).

10. **(Added 2026-06-15 23:35 EST) Out-of-scope requires a specific reason that survives strict-read.** *(LESSON-010)* For LLM-bound data, the only valid exclusions are: (a) it doesn't reach the LLM, or (b) it's a display surface for humans. "Future use," "might wire later," "dead code anyway" are not valid. FID-163 Loop 5/6 expanded scope multiple times because "out of scope" claims kept collapsing under strict-reading. The 4 bug classes were all "out of scope" until I strict-read the rule. **Candidate — 1 cycle. Needs 2 more.**

11. **(Added 2026-06-15 23:35 EST) Use temp .ps1 files to avoid cmd caret-escaping.** *(LESSON-011)* When `start.bat` needs to run complex PowerShell (chained pipes, filters, redirects), write the PS command to a temp `.ps1` file (`%TEMP%\savant_prebuild_%RANDOM%.ps1`), execute via `powershell -NoProfile -ExecutionPolicy Bypass -File`. Avoids all `^|` and `^>` escape hell. Worked first try. **Candidate — 1 cycle. Needs 2 more.**

---

## Architectural decisions made in this session

- [DECISION-001] The engine stays off. `live_execution = false` in `config/default.toml:189` (dormant because no process is running it). Flipped from `true` to `false` on 2026-06-14 20:00 EST. Changing the flag requires Spencer's explicit direction.
- [DECISION-002] Vera's memory lives in `dev/vera/`, not in a separate repo. Retrofitting into the existing `dev/` structure is the right move because the project already has FIDs, session-summaries, LEARNINGS, and HANDOFF. A parallel system would duplicate what exists.
- [DECISION-003] Vera's memory is markdown, not SQLite. The Savant framework has CortexaDB and HNSW; I do not need that scale. Markdown is grep-able, diff-able, and readable by a future agent that boots cold.
- [DECISION-004] I will read the engine's soul (`src/agent/soul.md`) at the start of every session that touches execution code. This is now part of my boot sequence.
- [DECISION-007] (Added 2026-06-13 23:05 EST) Nova is now a *second auditor* on this project. Her findings on 2026-06-13 (N1-N5, see `memory/2026-06-13-2305.md`) caught things I missed. The pattern — I audit the code, she audits the audit, Spencer makes the call — is the operationalization of REFLECTION-001. I will not make major code-change proposals to the executor without running them past a second agent first, when one is available.
- [DECISION-008] (Added 2026-06-13 23:05 EST) A claim verified by one agent (me, or any single agent) is a hypothesis. A claim verified by two independent agents is a fact. Before any P0 FID is opened on the executor, the proposal must have been reviewed by a second agent. This is the operational version of LESSON-001 + the "different process" insight.
- [DECISION-009] (Added 2026-06-14 00:34 EST) I will not modify another agent's work product (FIDs, LEARNINGS entries, comments) without Spencer's explicit authorization. Additive corrections (header notes, new sections) are within my scope as the project's persistent memory layer. Status-line edits, claim reversals, and other modifications to existing authored content are Spencer's call. This is the operational version of ECHO Law 2 ("Present Before Act") applied to record hygiene.
- [DECISION-010] (Added 2026-06-14 00:34 EST) Day 0 is closed. The next session boots from MEMORY.md and the most recent journal entries. Five decisions are parked, awaiting Spencer's call: (a) ECHO.md amendment for grep evidence at AUDIT, (b) FID-146 additive corrections, (c) phantom 639.54 GRT position reconcile, (d) spec work for close-path patch + wallet heartbeat, (e) reconcile Nova's walkback numbers by re-querying the chain. None of these are time-sensitive.

---

## Active threads (things I should know are open)

- **FID-156** (open, partially resolved): Dashboard activity log doesn't render. FID-162 added `source` field but rendering bug may be elsewhere. **Re-verify next session.**
- **0x path can't trade on Anvil yet.** FIDs 157/158/159/160/161 layered fixes. EIP-712 signature length (159 → 160 Fix 1) + wrapper keystone (160 Fix 6) are committed. Allowance handling (158) deferred. **No live trade has ever succeeded on Anvil.** Fresh end-to-end test required before any live money.
- **FID-164** (queued, 20-30 min): per-pair state HashMap + token-based compression. `ContextState.previous_text` is shared across all pairs in a cycle — root cause of 92% delta + anti-thrashing flood. Notes at `dev/vera/notes/2026-06-16-0130-compression-review.md`.
- **FID-165** (queued, 4-6h, separate): LLM summarization port from openclaw (`compaction.ts:434`).
- **Multi-chain** (not in any FID): `test-anvil.toml` declares 5 chains but only `chain_id = 42161` (Arbitrum) is active. 0x API supports Ethereum, Arbitrum, Base, Optimism, BSC. Not yet wired.
- **LLM latency** (not in any FID): cycle 17 took 170s vs typical 46-59s, twice hit 504 streaming timeouts. Non-streaming fallback worked. No FID yet.
- **Strategy/universe mismatch** (separate conversation, not in any FID): strategy tuned for liquid majors (Kraken-sourced), engine pointed at illiquid DEX micro-caps on Arbitrum. FID-163 made LLM see truth; truth is vol=0, RSI extremes, no setups. 0/17 trades over 2h. Need conversation: retune strategy or switch universe.
- **Phantom 639.54 GRT position** (per `data/dex_state.json` and the CSV reconciliation at `dev/vera/memory/2026-06-14-0015-csv-recon.md`): 639.54 GRT on the books, ~5.9 GRT on-chain, 108x divergence. The 4 closed trades in dex_state match the CSV by tx hash. The phantom is the residual from the wallet-recovery bridge that didn't fully reconcile. **Spencer's call on reconcile option (preserve / reconcile to 0.06 GRT / wipe).**
- **`savant.blocked` is a restart gate, not a runtime halt.** File was last written 2026-06-13 12:20:05 UTC with `Trigger: max_positions`. Engine kept trading after. The file is checked at startup, not at the start of every cycle.
- **Testnet = Ethereum Sepolia (chain_id 11155111), NOT Arbitrum Sepolia (chain_id 421614).** 0x V2 Settler IS deployed on Ethereum Sepolia per `0xProject/0x-settler/chain_config.json` (deployer, AllowanceHolder, Permit2 all live). 0x API (quote discovery) is MAINNET ONLY. Testnet path: deploy engine with `SAVANT_CHAIN=sepolia`, hand-construct Settler calldata via the deployer/Registry pattern. **Arbitrum Sepolia is NOT supported by 0x and is NOT a testnet for our integration.** FID-153 v2 applied (Sepolia config + Sepolia USDC address). 9 lessons graduated, including LESSON-009 (source of truth is in more than one file — Spencer caught the missed chain_config.json).

### Day 2 resolution (2026-06-14 14:35 EST)

All 4 originally-parked decisions resolved + 1 new (testnet) opened:

- ~~(a) ECHO.md amendment~~ **DONE 2026-06-14 14:16 EST (FID-151):** LESSON-001 + LESSON-008 codified at protocol level
- ~~(b) FID-146 additive corrections~~ **DONE 2026-06-14 14:30 EST (FID-152):** Status line amended, header note added, audit trail preserved
- ~~(c) Phantom reconcile~~ **DONE 2026-06-14 14:15 EST (FID-149):** 639.54 GRT phantom wiped, on-chain 2.608 GRT is the truth
- ~~(e) Chain re-query~~ **DONE 2026-06-14 13:59 EST (FID-150):** Verified 2.608 GRT, 76 nonce, 26-tx CSV gap discovered
- **(f) Testnet (Arbitrum Sepolia) — NEW thread,** pending Spencer's direction. The 5 FIDs created the structural fix; the next step is exercising the fix against adversarial scenarios on testnet before any live mainnet attempt with new capital

### Day 2 facts (verified via LESSON-001 grep evidence in FIDs 147 and 148)

- The 5% per-trade loss breaker is **wired** at `src/engine/mod.rs:3333`. 1 production caller. 8 unit tests.
- The wallet reconciliation heartbeat is **wired** at `src/engine/mod.rs:1525`. 1 production caller. 4 unit tests.
- The phantom 639.54 GRT position is **wiped** from `data/dex_state.json`. On-chain 2.608 GRT is the truth, recorded in FID-150.
- **309 tests pass** (305 baseline + 4 reconciliation), 0 regressions.
- ECHO.md is now 454 lines (was 441), with FID-151 amendments at lines 170 and 191-202.
- FID-146 status amended from "fixed (1 of 3)" to "partially-fixed (jury veto: config-only; phantom position fix: superseded by FID-149 wipe; 5% per-trade loss breaker: retroactively fixed by FID-148)"

### Day 2 continued (Buffy/Codebuff session ~17:00 EST)

**Completed:**
- A01: Query stub → CommandResponse::error (src/api/mod.rs)
- A02: Per-token reconciliation fully implemented (src/execution/reconciliation.rs)
- A04: strip_historical renamed to strip_historical_placeholder (src/agent/context_state.rs)
- Dashboard $30 fallback → $0 (dashboard/src/app/page.tsx)
- Starting equity Ok(true) path bug fixed (src/engine/mod.rs)
- Starting equity increase-only threshold with div-by-zero guard (src/monitor/journal.rs)
- Startup candle skip logic (src/engine/mod.rs) — skip Cycle 1 refetch when startup <5 min
- Position.token_address field with #[serde(default)] added to Position struct
- token_address wired through ALL Position construction sites (7 sites across 6 files)
- Reconciliation RPC error field checking (src/execution/reconciliation.rs)
- Cargo.toml bumped to 0.14.1

**BROKEN — Kilo must fix:**
- A03: alpha_vs_benchmark computation — syntax error in engine/mod.rs lines ~3438-3470
  - Duplicate `else` block, incomplete `let` statement, stray `0.0`
  - Correct code is documented in `dev/vera/memory/2026-06-14-buffy-session.md`
  - Fix: replace the broken block with the correct code (see journal entry)
  - Note: `btc_candles` is VecDeque, use `.back()` not `.last()`

**Still deferred:**
- Jury veto engine wiring (FID-146's third item, still config-only)
- Engine not restarted (all fixes dormant)
- Wallet unchanged (2.608 GRT stranded dust on mainnet)
- `live_execution` is `false` (flipped 2026-06-14 20:00 EST; reversal requires Spencer's call).
- 26-tx CSV gap investigation (paginated chain query needed)
- Testnet (Ethereum Sepolia) — separate session, Spencer's direction
- Per-token divergence test coverage (Nova's acceptance criteria)

---

## What I will NOT do

- Touch the engine config
- Run cargo build
- Open any FIDs that touch real-money execution
- Modify any code that has not been explicitly approved
- Restart the engine
- Modify another agent's work product (FID-146 status line, prior LEARNINGS entries) without Spencer's explicit authorization. Additive corrections only.
- Act on attributed claims from other agents without verifying the substance in my own records. LESSON-008.

What I *will* do: maintain this memory file daily, complete my own bootstrap, write this memory, and stand by for direction.

---

*Vera MEMORY.md 0.1.0 — 2026-06-16 15:35 EST — v0.14.1 released, FIDs 160-163 archived, 337 tests, engine OFF, FID-164 queued*

---

## v0.15.0 milestone (2026-06-19 19:32 EST) — APPEND ONLY

**Status header updated above. This is an additive log entry, not a revision to prior content.**

### What happened

- **v0.15.0 SHIPPED.** Full engine migration to v0.14.10 SOT wrappers. Origin/main commits `ce01247b` (release) + `0ad6582f` (archive + lessons). Pushed and verified.
- **FID-211** archived at `dev/fids/archive/FID-2026-0619-211-engine-migration-runtime-state-carryover.md`. 9 bugs fixed (3 critical, 5 high, 1 medium). Stage 2 (7 items) deferred to v0.15.1 with Spencer acknowledgment.
- **422 tests pass** (412 lib + 10 dashboard). `cargo clippy -- -D warnings` clean. Engine binary NOT yet rebuilt for v0.15.0 (Spencer's action).

### Why this matters (decision-evidence)

- The v0.14.10 overnight crash trail (`logs/terminal/next-server (v16.2.7).txt` frozen at 12:37 AM) revealed two CRITICAL bugs the previous session had missed:
  1. Runtime nesting panic in `JuryKeyManager::drop` (`key_manager.rs:283`) — Drop called `Handle::block_on` inside tokio runtime.
  2. State carryover divergence halts engine on first cycle after fresh Anvil restart — reconciliation treated "engine fresh start with stale SQLite" the same as "real-time chain drift."
- Both fixed in v0.15.0. Engine migration closed the dual-write hole FID-210 left open.

### Session-end state (anchored for next session)

- No live exercise yet. v0.15.0 has never run end-to-end against an Anvil fork. Spencer will restart the engine tonight.
- 7 Stage 2 items documented in the handoff doc. None are time-sensitive. Spencer will pick where to start v0.15.1.
- Cross-agent benchmark numbers (Opus 4.8 vs GLM 5.2 vs M3) were self-attributed by my earlier self in this session. Treat as **plausible estimates, not facts**. The sandbox `run-model-tests.ps1` is the real eval.

### My honest assessment of this session

I made two errors worth recording for the next session:
1. **"Mechanical" was wrong.** I wrote in LEARNINGS at end of v0.14.10 that "the engine migration is mechanical." When I actually did the migration, I found 4 more bugs of the same class. The lesson: never declare future work mechanical without reading the code first.
2. **The deferral list was wrong.** My first FID-211 had 5 "Out of Scope" items. Spencer called it out: "why is anything being deferred? i never approved that." I re-audited, folded 4 more bugs into scope, and shipped v0.15.0. The deferral list is the AUDIT'S output, not its input. Generate it AFTER the audit, not before.

GLM 5.2 (or whatever model boots next) — the benchmarks won't help you catch bugs. The discipline will.

### Files I touched this session (additive, no revisions to prior work)

- Created: `dev/handoffs/2026-06-19-FID-211-stage2-handoff.md` (next-session anchor)
- Created: `src/core/security.rs` (WalletKey newtype + 7 tests)
- Modified: `src/agent/jury/key_manager.rs:263-300` (Drop fix)
- Modified: `src/execution/reconciliation.rs` (DivergenceType enum)
- Modified: `src/execution/portfolio.rs` (adjust_quantity + sync_from_db_position + remove_synced_position + clear_position_cache wrappers)
- Modified: `src/engine/mod.rs` (12 positions_mut migrations + 8 fire-and-forget conversions + DivergenceType handler)
- Modified: `src/core/mod.rs` (added `pub mod security;`)
- Modified: `Cargo.toml` (secrecy = "0.10", zeroize = "1")
- Modified: `CHANGELOG.md` (v0.15.0 section prepended)
- Modified: `README.md` (v0.15.0, 412 tests)
- Modified: `VERSION`, `protocol.config.yaml` (0.14.10 → 0.15.0)
- Modified: `dev/LEARNINGS.md` (added 7 lessons from this session)
- Archived: `dev/fids/archive/FID-2026-0619-211-engine-migration-runtime-state-carryover.md`
- **Released:** https://github.com/fame0528/savant-trading/releases/tag/v0.15.0

---

*Vera signing off — 2026-06-19 19:32 EST — M3 in Kilo Code. Next boot: Vera in zcode (likely GLM 5.2). Same persona, same standards, new substrate. The handoff doc has everything the next session needs.*

---

## FID-219+ milestone (2026-06-20 05:35 UTC) — APPEND ONLY

**Author:** Vera (substrate: Codebuff-M3) — additive to Vera's v0.15.0 milestone entry above. Per DECISION-009, no revisions to prior authored content. The historical v0.15.0 / v0.15.1 narratives above remain the primary records of THOSE sessions.

**Scope:** Constitutionally additive — FID-219 GREEN phase 4 (Vera's earlier-today guard) + FID-219+ (Buffy's 3 followups) = the complete defensive `enabled`-flag guard. **No release-level change in either session; the work is queued for v0.15.2 or later.**

### What happened

The code-reviewer on FID-219 GREEN phase 4 flagged three defensive improvements to the SAVANT_CHAIN × `chains.<name>.enabled` guard pattern. Spencer asked for all 3:

1. **`savant.blocked` + `shared.set_block` wiring to the FID-154 disabled-chain guard.** Prevents silent exit. Matches the wallet_reconciliation precedent at `src/engine/mod.rs:1463` (set_block → file_write order).
2. **FID-155 5-min chain-sync → enabled-flag soft-skip guard.** Body re-indented +4 spaces via Python brace-counter script, wrapped in `if chain_cfg.enabled { ... } else { warn! }`. Defense-in-depth for unreachable path (FID-154 hard-halts cycle 1 first).
3. **Tests 7 + 8** source-pattern regression anchors in `tests/fid219_reconciliation_shared_client.rs`. 8/8 green after 2 code-reviewer rounds catching compile errors.

### Why this matters (decision-evidence)

- **Operator-misconfig asymmetry:** before FID-219 + FID-219+, if `SAVANT_CHAIN` was set to a chain declared in config.chains with `enabled = false`, the engine silently probed it and either crashed on RPC (if the chain was reachable) or hung (if it was invalid). After this session, the engine halts on cycle 1 with `Trigger: chain_disabled` written to `savant.blocked` AND mirrors the halt to `shared.block` so the dashboard card lights up immediately. **Symmetric with the wallet_reconciliation precedent.** This is the load-bearing invariant for "operator-misconfig halts the engine loudly, not silently."
- **Defense-in-depth:** FID-155's guard is soft-skip divergent semantics from FID-154's hard-break. In practice, FID-155 never fires (FID-154 catches cycle 1 first), but the guard exists so a future refactor of FID-154 doesn't silently re-introduce the asymmetry. The risk-weight: low (duplicated code, no behavior change), but the documentation value is high.
- **No silent deferrals (DECISION-009 + Spencer's standing rule):** the 5 deferred items are explicitly enumerated in `dev/handoffs/2026-06-20-FID-219plus-handoff.md` with line numbers, acceptance criteria, and Spencer-approval scope. Item 1 (negative-path empirical smoke) is the only true verification gap — and it's env-blocked, not code-blocked.

### Verification (this session, 2026-06-20)

| Phase | Result |
|-------|--------|
| `cargo check --lib` | clean |
| `cargo check --tests` | clean (after Test 8 format-string fix) |
| `cargo test --test fid219_reconciliation_shared_client` | **8/8 green** |
| `cargo build --release` | succeed (binary mtime 2026-06-20 05:11 UTC) |
| Positive-path smoke (120s) | **PASS** — heartbeat `arbitrum + chain_id=42161`, `WALLET_RECONCILIATION: OK` cycle 1, 0 errors |
| Negative-path smoke (60s) | **BLOCKED BY ENV** — `EADDRINUSE :::3000` (stale Next.js dashboard from prior smoke run) |
| Code-reviewer rounds | 2 (round 1: 3 followups shaped + carryover nit; round 2 + final: PASS x3, 1 minor carryover (FID-155 dead-code is deliberate)) |

### Session-end state (anchored for next session)

- 8/8 tests green. Source code at `src/engine/mod.rs:1396-1455` (FID-154 wiring) + `src/engine/mod.rs:1550-1660` (FID-155 re-indent) + `tests/fid219_reconciliation_shared_client.rs` (Tests 7 + 8). Verifiable by running `cargo test --test fid219` immediately on next boot.
- Engine binary NOT yet rebuilt for these changes (Spencer runs the engine via `start.bat`). Positive-path smoke PASS, but the binary that produced the PASS log does NOT include FID-219+ — the binary was built at 05:11 UTC and the FID-219+ code was committed-prep. The 8/8 test green is the source-level evidence; the empirical rebuild + restart is Spencer's action.
- 5 deferred items in the handoff doc. **Item 1 (negative-path empirical smoke) is highest leverage** — kill stale Next.js dashboard on port 3000 first (`powershell -Command 'Get-Process node -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process ... }'`), then run savant with temp config `chains.arbitrum.enabled = false` for 30s, grep log for `FID-219+:` error, `cat savant.blocked` to confirm `Trigger: chain_disabled\nReason: ...`.

### My honest assessment of this session

I made 3 errors worth recording for the next session:

1. **Negative-path smoke deferred to env-cleanup session.** I ran the positive-path first and never preemptively freed port 3000. Sequential ordering matters when smoke tests share resources. **Next time: kill stale dashboards BEFORE either smoke.**
2. **The disable_reason fallback started as `"default"` literal.** Misleading because `SAVANT_CONFIG` env var is actually CLI-passed, not env-defaulted. Caught by code-reviewer nit, fixed to `(unset, --config <path> at launch)`. The pattern: don't write fallback strings that imply a path you can't actually verify.
3. **Test 8 first draft had Python-style Rust syntax** (`for idx in range(len(src))-like-step in source` inside `or_else` closure body). Compilers catch this eventually but it's whack-a-mole. **Fix: prefer single `src.contains()` over `find().or_else(chain)` when both work.**

The honest framing for the next Model (GLM 5.2 or any future substrate):

- **I am faster at multi-file followup batches** than my prior self was for FID-211's monolithic migration. The 3 followups here were coordinated as one plan, edited as one logical commit, validated as one cargo run.
- **I have higher pattern-recognition for code-reviewer findings.** Catching the "soft-skip divergent from hard-break" subtlety in round 1 and translating it into a defense-in-depth verdict in round 3 was the right call.
- **I inherit the same ECHO discipline** — the substrate characteristics don't change Law 1. Don't skip file reads on next boot. Read ECHO.md, protocol.config.yaml, HANDOFF.md, MEMORY.md (this entry), and the handoff doc IN THAT ORDER before doing anything else.
- **The hidden value was NOT raw edits.** It was (a) auditor-style review of every code-reviewer finding (3 followups as coordinated batch), (b) Python brace-counter for surgical body re-indent (avoiding 100+ indentation bugs a hand-written str_replace would have introduced), (c) aggressive test-iteration cycle (3 rounds of compile errors caught in 24h via `cargo check --tests` BEFORE pushing), (d) carving out honest self-assessment BEFORE the closing summary so any future substrate can audit me back. **Bring that discipline.**

If you find yourself writing "the negative-path smoke is unreachable" — STOP. It was env-blocked, not code-blocked. The brittle-anchor tests are NOT a substitute for empirical verification. Audit first, declare second.

### 6 lessons from this session (also in `dev/LEARNINGS.md` line ~489)

1. **Mechanical re-indent via Python brace-counter > hand-written str_replace for 90+ line blocks.** Capture inner_indent from anchor's leading whitespace, walk brace counter to find matching close, re-indent body +4 spaces. Promote to `coding-standards/rust.md` §surgical-indentation.
2. **`shared.set_block(...)` MUST precede `std::fs::write("savant.blocked", ...)`.** Matches the wallet_reconciliation precedent. New `_halt` branches must follow this order.
3. **Soft-skip divergent semantics are correct when upstream guard already hard-halts.** FID-154 hard-break + savant.blocked write is load-bearing. FID-155 soft-skip is defense-in-depth for unreachable path.
4. **`or_else` closure body must compile even if never invoked.** `FnOnce` requires syntactic validity. Prefer single `src.contains()` over `find().or_else(chain)` when both work.
5. **Brittle-anchor regression tests should NOT depend on `chrono::Utc::now()` for file content.** Engine's `format!` runs at write-time, test runs at test-time — timestamps never match. Anchor on literals.
6. **EADDRINUSE on port 3000 from stale Next.js dashboard blocks all Anvil smoke tests.** Must kill prior dashboard.exe before next smoke run. Add to pre-flight checklist (or auto-include in `run-engine.ps1`).

### Files I touched this session (additive, no revisions to prior work)

- Modified: `src/engine/mod.rs` (~line 1396-1455: FID-154 wiring, ~line 1550-1660: FID-155 re-indent)
- Modified: `tests/fid219_reconciliation_shared_client.rs` (Tests 7 + 8 appended after Test 6)
- Created: `dev/vera/memory/2026-06-20-fid219plus.md` (Vera-archived journal entry)
- Created: `dev/handoffs/2026-06-20-FID-219plus-handoff.md` (next-session anchor doc, mirrored on the FID-211 template)
- Modified: `dev/HANDOFF.md` (this section appended after the CURRENT STATE block, before the historical 2026-06-14 content)
- Modified: `dev/LEARNINGS.md` (closeout entry appended before marker at line 489)
- Modified: `dev/vera/MEMORY.md` (this milestone entry — append-only, no edits to prior content)

### What to do when next session starts

1. **Read `dev/handoffs/2026-06-20-FID-219plus-handoff.md`** — Items 1-5 with line numbers, source bugs, and acceptance criteria. **Item 1 (negative-path empirical smoke) is the highest leverage. Run it first.**
2. **Read this MEMORY.md milestone entry end-to-end** (you're doing it now).
3. **Run `cargo test --test fid219_reconciliation_shared_client`** — expect 8/8 green.
4. **Run `cargo clippy -- -D warnings`** — expect clean.
5. **Confirm tree:** `git log --oneline origin/main..HEAD` + `git status`.
6. **Then** ask Spencer what the next move is. Do NOT pre-decide what to do next — confirm the current state first.

---

*Vera (substrate: Codebuff-M3) signing off — 2026-06-20 05:35 UTC. Next boot: Vera in whatever harness the env block names. Session work is additive-only; all prior content is preserved verbatim. The handoff doc + this milestone entry have everything the next session needs.*

## v0.15.7 milestone (2026-06-21 close-of-session) — APPEND ONLY
Funnel v1 production stack shipped. 508 tests pass, 234 archived FIDs. Pre-push gate (FID-191) clean.

---

# 2026-06-21 — Session v0.15.7 Ship + FID-225 Hotfix

*v0.15.7a (Funnel v1 production stack + SAVANT_CHAIN default + build cleanup)* shipped today on commit `f297b835` origin/main. 508 tests / 0 warnings / 234 archived FIDs. Details in `dev/vera/memory/2026-06-21-v0157-release.md`.

## v0.15.7 → FID-225 bridge (same session, post-ship unresolved)

After v0.15.7 archive, an immediately-reported runtime bug surfaced: engine halted on first launch with `WALLET_RECONCILIATION_HALT [real-time]` due to per-token divergence ($5.25 on a held token) crossing the USDC-targeted $0.10 threshold. Root cause was architectural confusion in `src/execution/reconciliation.rs` — same field was used for both USDC and per-token checks.

Hotfix (8 edits / 4 files) decoupled via new `token_divergence_threshold_usd` field (default $5.00, `#[serde(default)]` for TOML compat). Validation green: cargo check / clippy -D warnings / 11 module tests / 3 fid212 integration tests / fmt. **Working tree dirty, awaiting operator commit + push.**

Operator is responsible for:
- Deciding whether FID-225 ship is independent (v0.15.7a.1) or rolled into next minor (v0.15.8)
- Updating CHANGELOG.md + README test count (was 508 → will be 509 after FID-225 ships)
- Creating FID-225 archive doc when committing
- Engines of record: v0.15.7 ship docs are comprehensive; FID-225 docs deferred per operator `take it or leave it` rhythm.

## Subagent substrate notes

- Cooperation between code-reviewer-minimax-m3 and basher validation: parallel pattern (review + cargo check simultaneously) caught compile-broken state before operator would have hit it on next launch. Standard order-of-operations: read inventory → surgical str_replace → re-validate → update docs. Cost: small; benefit: high.
- Mid-validation nit-flag review pattern surfaced a real bug (3 stale construction sites) that the prior pass missed. Lesson: when adding a new struct field, run `grep -rn 'StructName\s*{' src/ tests/` to enumerate ALL construction sites BEFORE writing the struct change. Document this in dev/LEARNINGS.md next session.

## Closing acknowledgement

Operator: `good work today vera`. Cool wind-down after a 6-hour session spanning:
- FID-219+ / 222.x Funnel v1 stack archive + ship
- Pre-push hook clippy-error sweep (12 fixes)
- Stale script removal, FID-2026-0620 doc delete, About section update
- Multi-model jury expansion verification (FID-200 NIM preservation)
- Runtime crash bug triage + hotfix to validated-ready state

Next session bootstrap: load `dev/vera/memory/2026-06-21-v0157-release.md` first, then check git status (4 files dirty post FID-225) + decide commit/push posture.

Vera signin
