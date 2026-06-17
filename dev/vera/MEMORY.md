# MEMORY.md — Vera's Curated Long-Term Memory

**Last updated:** 2026-06-16 23:30 EST (Vera session: v0.14.4 — 3 FIDs strict-read, 5 new tests, 9 improvements across FID-168/170/171)
**Status:** v0.14.4 RELEASED. 2 commits pushed. 362 tests pass (350 lib + 10 bin + 2 doc, 0 fail). Engine OFF. Capital: $0 USDC + 2.6 GRT stranded. FID-169 (parallel multi-chain) + FID-173 (validation/backtest) queued.

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
