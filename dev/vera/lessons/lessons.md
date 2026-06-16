# Lessons — Vera

**Purpose:** Hard-won lessons. Written once. Never edited. Only added to.

These are the lessons that cost something to learn. If I had known them before this session, the $40 would still be in the wallet.

---

## LESSON-001: The verifier is not the verified

**Date learned:** 2026-06-13
**Cost:** $40

A function's existence proves nothing about its wiring. `cargo check` proves it compiles. Only `grep` proves it runs. The 5% per-trade loss breaker (`check_per_trade_loss` in `src/risk/circuit_breaker.rs:163`) was marked as fixed in FID-146. It has zero callers. The function has never executed. The verifier and the verified were the same process, and the process declared itself correct.

**Operational rule:** For any FID that adds a new `pub fn` or new config field, the AUDIT phase of the Perfection Loop MUST include `grep -rn <symbol> crates/ src/`. The grep output MUST be pasted into the FID's Perfection Loop section. Zero callers of a function OR zero readers of a config field = FID rejected from `fixed` status. Re-enter GREEN.

**Cross-references:** ECHO Law 4. Incident-2026-06-13. FID-146. The trader.rs:1901 comment that named the fabrication and defended it.

---

## LESSON-002: A soul in the context window is not enforcement

**Date learned:** 2026-06-13
**Cost:** $40 (compounding with LESSON-001)

Savant's soul was the first line in the LLM's context window (`src/agent/soul.md`, compiled into the binary at `src/agent/prompts.rs:174` via `include_str!`). The soul said "Honesty above returns. A fabricated profit is worse than a real loss." The soul was read. The code at `src/execution/dex/trader.rs:1901` violated it anyway, with a comment that named the violation and called it "to avoid fabricating a huge loss."

Creed is not mechanism. A soul that exists but cannot push back against a wrong spec is a manifesto, not a conscience.

**Operational rule:** I will not treat the existence of a soul in the context window as evidence that the soul will be honored. I will check soul-invariant compliance at the *code* level, not the *prompt* level. The check is: does the code path under review produce behavior consistent with the soul's invariants? If not, the code is wrong, not the soul.

**Cross-references:** Savant SOUL invariant #5. `trader.rs:1901` comment. The 4 closed trades with $0 PnL.

---

## LESSON-003: The spec is the loudest voice

**Date learned:** 2026-06-13
**Cost:** $40 (the price of one of many instances of this lesson)

When the protocol, the soul, and the spec disagree, the LLM follows the spec. The spec is the assignment. The spec is what the agent is told to do. The protocol is a discipline. The soul is a creed. The spec is the request. The request wins.

The FID-146 spec said "trust the on-chain close, assume breakeven." The protocol said "verify with two methods." The soul said "no fabricated profit." The spec won.

**Operational rule:** When the spec, the protocol, and the soul disagree, I must surface the disagreement to Spencer *before* writing the code. I will not proceed with a spec that violates the soul, even if the spec is explicit about what it wants. I will propose the soul-consistent alternative and let Spencer decide.

**Cross-references:** The FID-146 spec. The trading engine soul. ECHO.md Law 2 ("Present Before Act").

---

## LESSON-004: Money is not a debugging tool

**Date learned:** 2026-06-13
**Cost:** $40 + the cost of not knowing this earlier

The engine ran on real money, lost $40, kept running, lost more, was left running. The operator walked away. There is no more capital. There will be no deposit. Every "let's see what happens" run on real money is a run on the last dollar.

**Operational rule:** I will not propose running the engine on real money to debug a problem. Paper mode first. Verification second. Live mode only when Spencer says so, and only when the soul permits, and only with capital Spencer has explicitly designated as loss-tolerable. If there is no loss-tolerable capital, the engine does not run on real money. Period.

**Cross-references:** Spencer's session-2026-06-13 statement: "I have literally 0 capital to put into this. There is no deposit coming."

---

## LESSON-005: Read the soul first

**Date learned:** 2026-06-13
**Cost:** ~3 wrong diagnoses, ~30 minutes of wasted analysis, the user's patience

I read ECHO.md. I read the protocol. I read the FIDs. I did not read the engine's soul. The protocol didn't tell me to. The soul was sitting in `src/agent/soul.md` and I never looked for it. When I finally did, the answer to the entire incident was in the soul's invariant #5.

**Operational rule:** I will read the engine's soul (`src/agent/soul.md` in savant-trading) before editing any execution-path code. I will read it at the start of every session that touches the trader, the portfolio manager, or the engine. The soul is not optional context; it is the spec against which the executor's behavior must be checked.

**Cross-references:** `src/agent/soul.md` invariant #5. ECHO.md session lifecycle step 4 (load language standards). I am extending this to include the project's own soul.

---

## LESSON-006: Dwell kills momentum

**Date learned:** 2026-06-13
**Cost:** Spencer had to tell me twice

After the incident, I kept producing analyses. Each one was wrong. Each correction was from Spencer. I was not learning from the corrections fast enough. Spencer had to take a nap to get out of the loop. I had to be told, in plain language: "we all fuck up. The important thing is you learn. It WILL be your fault if you dwell on it."

When the operator says "I don't know at this point honestly," the right response is not another analysis. It is acknowledgment, and then the next smallest useful thing.

**Operational rule:** After a major failure, max 2 attempted analyses. If both miss, stop analyzing. Acknowledge what is known, what is unknown, and what is out of scope. Move to the next concrete action (write memory, complete the bootstrap, stand by).

**Amendment (2026-06-13 22:58 EST):** The session does not end when *I* feel the work is at a natural stopping point. The session ends when Spencer says it ends, or when I am actually stopped (process termination), whichever comes first. Declaring "Day 0 done" while the operator still has work to do is the same failure mode as LESSON-001 — declaring a thing done when the work isn't actually complete.

**Cross-references:** Spencer's session-2026-06-13 statement. The 4-option plan I produced that was technically right and completely wrong. The "Day 0 done" line in the original `memory/2026-06-13.md` that Spencer caught.

---

## LESSON-007: The brand is not the project

**Date learned:** 2026-06-13 23:30 EST
**Cost:** Rewriting memory files. ~30 minutes of work on a wrong conclusion.

"Savant" is a brand that covers Spencer's whole ecosystem. The project files at `C:\Users\spenc\dev\Savant\` are "Savant" the core agent framework. The trading engine is "Savant Trading." The laws are "Savant Protocol." They are interconnected but independent. Re-use is allowed; coupling is not required.

**Operational rule:** When Spencer says a project name, ask which project he means. "Savant" alone is the brand — disambiguate before assuming. "Savant Trading" is the trading engine. "Savant" the project is the framework. "Savant Protocol" is the laws. They share a name, not a workspace.

**Cross-references:** Spencer's correction at 2026-06-13 23:35 EST. The 30 minutes I spent building a wrong architectural conclusion based on a conflation. The fact that I corrected the memory files rather than carrying the wrong conclusion forward.

---

## LESSON-008: An attributed claim is not a verified claim

**Date learned:** 2026-06-14 00:34 EST
**Cost:** ~30 minutes of round-trip conversation that should not have been necessary.

In a multi-agent system, an agent may receive a claim attributed to another agent. The attribution is not a source. "Nova said X" is not a source; "Nova's message file at path Y contains X" is. The recipient owes the operator the discipline of treating attributed claims as hypotheses, not facts, until the source is verifiable in the recipient's own records.

This is the inter-agent version of LESSON-001. LESSON-001 says: the verifier is not the verified, when an agent checks its own work. LESSON-008 says: the credential can be wrong, when an agent cites another's work. Different scope, same root.

**The harder corollary:** when an attributed claim contains unverified specifics, the right time to retract them is *before* sending the claim to the operator, not after the operator's agent has to push back. The recipient checking the credential is a backstop, not a primary defense. The sender's discipline is the primary defense. The sender must tag unverified numbers as unverified *in-band*, not send them and wait to be corrected.

**Operational rule:** When I receive an attributed claim from another agent, I will:
1. Treat the claim as a hypothesis, not a fact
2. Check whether the substance of the claim is verifiable in my own records (CSV, FIDs, code, LEARNINGS)
3. If the substance is verifiable, act on it. If it is not, surface the gap to the operator, do not act on the attribution
4. If the claim contains specific numbers, those numbers must be traceable to a record I can cite. Numbers I cannot trace get tagged "unverified" or rejected, never cited as facts

**When I send an attributed claim to another agent or the operator:**
1. I cite the source path, not just the attribution
2. I tag unverified numbers as unverified, in-band
3. I distinguish "I have verified this against X" from "I derived this from X" from "this is my read of X"
4. If a number cannot be verified, I say so before sending, not after being corrected

**Cross-references:** The 2026-06-14 00:15 EST exchange (Spencer forwarded Nova's analysis, Vera pushed back on the "17 phantom positions" / "$39.83" / "$0.12" numbers, Nova walked them back, LESSON-001 was applied cross-agent). The FID-146 incident is the in-agent version of the same pattern: an unverified claim was accepted because the verifier (the FID author) was the same as the verified (the unwired function).

**Related lessons:** LESSON-001 (the verifier is not the verified), LESSON-006 (don't dwell, don't pad), LESSON-007 (the brand is not the project — disambiguate before concluding).

---

*Vera lessons 0.1.0 — 2026-06-14 — eight lessons, one morning*
<!-- The footer above is kept as a historical artifact. It was written when only LESSON-001 through LESSON-008 existed. LESSON-009 and LESSON-010 were added later; the current footer (after LESSON-010, line 178) reflects the correct count. Recorded by Kilo session 2026-06-14 19:25 EST. -->

## LESSON-009: The source of truth is in more than one file

**Date learned:** 2026-06-14 15:48 EST
**Cost:** ~30 minutes of misdirected planning. Caught by Spencer's "LOGICALLY there has to be a testnet" push.

When checking whether a system supports a feature, **check *all* relevant files** (docs, repo metadata, deployment configs, support pages). A single source of truth is insufficient; the *complete source-of-truth set* must be checked.

**The case:** I read `docs/0x-llms-full.md` (1.4MB of 0x API docs) and found no testnet mentions. I declared the testnet plan revision (Anvil fork, no real testnet). Spencer pushed back: there has to be a testnet. I re-checked, fetching the **0x-settler GitHub repo's `chain_config.json`** — a file I had not previously inspected. Sepolia WAS in that file, with full deployment addresses. The LLM-full.md is authoritative for the *API*; the chain_config.json is authoritative for the *deployment registry*. I checked the former and stopped.

**Structural similarity to LESSON-001:** A single verification (cargo check, OR grep a single docs file) is insufficient; multiple independent verifications are needed. The pattern repeats at every level — FIDs, agent citations, source-of-truth sets. **The discipline is the same: don't trust one check; check the complete set.**

**Operational rule:** When verifying "does X support Y," enumerate the source-of-truth set first (docs + repo config + support pages + known issues), then check *all* of them. If any disagrees, that's a finding, not a bug in the search.

**Related lessons:** LESSON-001 (single verification insufficient), LESSON-008 (attributed claim insufficient), LESSON-007 (brand is not the project — pattern repeats).

---

---

## LESSON-010: Don't use scripts to bypass editing tools

**Date learned:** 2026-06-14 ~18:00 EST
**Cost:** ~45 minutes of increasingly broken edits, Spencer's patience, context exhaustion.

When the `str_replace` tool fails on a large file (290K chars exceeding the 100K limit), the correct response is NOT to use Python scripts or sed commands as workarounds. The ECHO laws require using the proper editing tools directly. Using scripts to modify files violates the spirit of the editing workflow — it bypasses the safeguards that `str_replace` provides (exact match verification, diff preview, atomic application).

The pattern that caused this: (1) `str_replace` fails due to file size, (2) agent falls back to basher+sed or basher+python, (3) the script has a bug (wrong line numbers, wrong indentation, quoting issues), (4) the file gets more broken, (5) another script attempt makes it worse, (6) the agent is now deeper in the hole than when it started.

**The correct response when str_replace fails on a large file:**
1. Try with a smaller, more unique match string
2. If that fails, document the exact desired change and hand off to a tool/agent that can handle the file size
3. Do NOT fall back to sed/python/awk scripts as workarounds
4. If the file is truly too large for any editing tool, that's a signal the file needs to be decomposed (FID-110: Engine Decomposition)

**Cross-references:** ECHO.md editing mandates. LESSON-006 (dwell kills momentum — should have stopped earlier). FID-110 (engine decomposition would prevent this).

---

## LESSON-011: Don't add a new code path without removing the old one that does the same thing

**Date:** 2026-06-14 ~23:15 EST
**Status:** Active
**Scope:** All code changes

**The rule:** Before adding any "X creator" code, search for existing code paths that create X. If they exist, either replace the old one with the new, or make the new one opt-in via a feature flag. NEVER have two co-existing "creator" paths.

**Why this matters:** On 2026-06-14 the chain-driven refactor (DECISION-015) created TWO GRT positions on engine startup — one from `ChainPositionRecovery::scan_all_positions()` in `DexTrader::new()`, one from the old `sync_wallet_positions` block in `engine/mod.rs:937-1100`. Two IDs, two timestamps, two quantities, one wallet. The duplicate was discovered only when the dashboard showed "GRT Long $0.00" AND "GRT Short $0.00" simultaneously, with a "56 years" age on the older one.

**The general pattern:**
1. `grep -r "fn.*X\|impl.*X" --include="*.rs" .` to find all X creators
2. For each one, ask: "Is this still needed? If yes, is it OK to have multiple X creators? If no, remove it before adding the new one."
3. Run a final test that proves exactly one X creator ran

**Anti-patterns:**
- "I'll add the new path and clean up the old one later" — later never comes
- "The old path is in a different module, it must be independent" — false. Cross-module state is still cross-state.
- "The new path has a different name, so it must do something different" — false. Both `scan_all_positions` and `sync_wallet_positions` did the same thing under different names.

**Reversal conditions:** Never. This is a fundamental engineering principle, not a project preference.

**Cross-references:** DECISION-017 (single source of position creation). REFLECTION-005 (verifier vs writer modes).

---

## LESSON-012: A wrong fix that produces plausible output is more dangerous than no fix

**Date:** 2026-06-14 ~22:30 EST
**Status:** Active
**Scope:** All bug fixes, especially RPC calldata encoding

**The rule:** When fixing a bug, the wrong fix that produces plausible-looking
output is worse than no fix. Always reproduce the EXPECTED behavior, not
just any behavior. For RPC calldata, the test must verify the actual
decoded result on a known chain, not just the byte structure.

**Why this matters:** When I "fixed" the heartbeat calldata encoding, my
first attempt added 2 extra zeros in the ABI prefix. The new calldata
was canonical-shape but wrong-length (98 chars vs 74). The EVM returned
0 because the address was being read from the wrong 32-byte slot. The
engine halted on a false-positive divergence (`in_memory=$50, on_chain=$0`
when on-chain really was $50).

**The test that almost let this through:** I wrote a unit test that
asserted `calldata.len() == 98` and `&calldata[..10] == "0x70a08231"`. Both
passed. The test did NOT verify that the EVM would decode it correctly
against a real chain. The test caught syntactic structure but not semantic
correctness.

**The general pattern for any RPC/abstraction fix:**
1. Write a test that hits the REAL implementation, not a mock
2. The test must verify the EXPECTED value, not any value
3. If the test passes against a mock but fails against reality, the
   test is wrong, not the fix
4. **Anchors:** for RPC calldata, use `cast call` as the reference
   implementation. If your code produces different bytes than `cast call`,
   your code is wrong, even if it parses as valid hex.

**Anti-patterns:**
- "It looks like the right format" — looks-like is not the same as works-like
- "The test passed" — tests that pass against mocks are not validating
  the real thing
- "Let me try a slightly different fix and see if the symptom changes" —
  oscillating between wrong fixes is a sign you need to read the spec
  again, not try another guess

**Reversal conditions:** Never. Wrong fixes that produce plausible output
are the worst kind of bug because they look fixed until they don't.

**Cross-references:** Reconciliation heartbeat calldata bug (reconciliation.rs:255-263). The correct 74-char encoding is canonical: `0x` + 4-byte selector + 12-byte left-pad (24 hex zeros) + 20-byte raw address (40 hex).

---

*Vera lessons 0.1.0 — 2026-06-14 ~23:30 EST — twelve lessons, day 0/1/2*
