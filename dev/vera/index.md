# Vera's Index — Cross-References into Project Memory

**Purpose:** Map Vera's memory to the project's existing memory. When future-me needs to find a FID, a session-summary, or a lesson, the index tells me where to look.

**Last updated:** 2026-06-13

---

## FIDs that matter to me

### Critical (touched by the 2026-06-13 incident)

| FID | Title | Status | Why it matters to Vera |
|---|---|---|---|
| FID-146 | 5% Per-Trade Loss Breaker, Phantom Position Fix, Jury Veto | fixed (1/3) | The masking mechanism. The "phantom position fix" IS the bug. Jury veto is config-only. **NEW: FID-146 has a name collision — 4 references in engine/mod.rs are about jury veto, the breakeven-masking is a different concept under the same FID number. Hygiene issue.** |
| FID-145 | Prompt Threshold Sync & Trader RPC Fallback Hardening | closed | The fallback that masks verification failures. |
| FID-142 | Token Resolution → 0x Liquidity Failures | CLOSED — implemented v0.14.0 | Token-level liquidity gate. Not the cause of the drain. |
| FID-141 | Live Buy Failures — Dashboard Sort + 0x Liquidity Gate | GREEN (Implemented) | Not the cause. |
| FID-140 | Prompt Threshold Inconsistency | GREEN (Implemented) | Not the cause. |
| FID-139 | Batch Parsing Gap | GREEN (Implemented) | Not the cause. |
| FID-138 | M3 Thinking Leakage | verified | Solved the sandbox parse-error problem. Not the cause. |
| FID-110 | Engine Decomposition | partial (4/7 sessions done) | engine/mod.rs is 5158 lines. Sessions 5-7 deferred. **Recon 2026-06-13 confirms the engine is still a monolith. This FID is more important than I had it.** |

### Open work

| FID | Title | Status | Notes |
|---|---|---|---|
| FID-106 | Agent Conversation & Query System | open, spec only | |
| FID-110 | Engine Decomposition | partial (4/7 sessions done) | engine.rs is enormous. Sessions 5-7 deferred. |
| FID-126 | Conviction-Weighted Thresholds | open | Uncommitted working tree from prior session. |
| FID-127 | Conviction-Weighted Sizing | open | Bypassed by `SAVANT_GATE_DISABLED=1`. |
| FID-128 | Sandbox Jump-Diffusion | spec only | |
| FID-129 | Remove Deep-Asian Penalty | spec only | |
| FID-130 | Counterfactual Grader | spec only | |
| FID-131 | KU Absolute-Language Scrub | partial | |
| FID-132 | Checklist Evaluation Matrix | spec only | |
| FID-133 | A/B Test Harness | spec only | |
| FID-134 | 20 Adversarial Scenarios | spec only | |
| FID-135 | Checklist Modifier Calibration | spec only | |
| FID-136 | Release Coordination & Dep Tracking | open, meta-FID | |

---

## Session-summaries to read

- `2026-06-12-1500.md` — last session, set up M3 thinking fix, hit FID-138
- (Earlier sessions exist back through 2026-06-12 14:31 and earlier — read on demand for context)

## LEARNINGS.md sections that matter

The full file is 631 lines. The most relevant sections to the engine's loss pattern:

- Session 2026-06-09 (FID-104): "Always read full files 0-EOF before diagnosing." "OpenAPI spec takes priority over docs examples."
- Session 2026-06-08 (FID-088): "Three enforcement layers are needed. Prompt-level, parser-level, engine-level. Any single layer can be bypassed by a creative LLM; all three together are robust."
- Session 2026-06-08 (FID-087): "Rushing fixes without FIDs creates cascading failures." "Every `Result` must be handled explicitly. `unwrap_or_else(|| vec![])` → `unwrap_or_default()`."
- Session 2026-06-07 (FID-074 + FID-073): "`amount_to_wei()` rounding can exceed on-chain balance." "Partial close needs different semantics than full close."
- Session 2026-06-06 (v0.10.0): "Client-side stop-losses are the #1 risk on DEX. Engine crash = no protection."
- Session 2026-06-05 (FID-052): "Silent rejection is the most dangerous bug pattern." "Every `continue` in a trade execution path should be loud."

The last one is the most relevant: the executor recorded 4 losses as $0 with no loud signal. Silent.

---

## HANDOFF.md

`dev/HANDOFF.md` — generated 2026-06-06 04:05 EST. **Out of date.** Predates the incident. References v0.10.2, "currently holding 2 open long positions" — those are gone now. The HANDOFF says the wallet has ~$26. The wallet has $0.00.

Future me: do not trust HANDOFF.md without verifying. Read it for context of *what was being worked on*, not for *what the state is*.

---

## `needs.md`

`dev/needs.md` — FID-045, multi-chain 0x system plan. 5 phases. Phase 1 (expand Arbitrum tokens) is described as "zero risk, immediate value." This is not related to the incident but is good background for the multi-chain direction.

---

## `protocol.config.yaml` and `VERSION` drift

- `VERSION` file: `0.14.0`
- `protocol.config.yaml` `project.version`: `0.13.9`

Out of sync. This is configuration drift, not a bug, but it's a thing future-me should know about.

---

## Files I have read 0-EOF (proof of compliance with Law 1)

- `ECHO.md` (0-441)
- `protocol.config.yaml` (0-71)
- `coding-standards/rust.md` (0-78)
- `dev/LEARNINGS.md` (0-454 so far — file continues)
- `dev/fids/MASTER-FID.md` (0-144)
- `dev/fids/FID-2026-0612-146-...` (0-129)
- `dev/fids/FID-2026-0612-141-...` (head:30)
- `dev/HANDOFF.md` (0-138)
- `dev/needs.md` (0-219)
- `dev/session-summaries/2026-06-12-1500.md` (0-193)
- `src/risk/circuit_breaker.rs` (0-284)
- `src/risk/position.rs` (0-621)
- `src/risk/correlation.rs` (0-247)
- `src/risk/stop_loss.rs` (0-174)
- `src/risk/mod.rs` (0-10)
- `src/execution/dex/trader.rs` (head 1150-1349, then 1800-1959)
- `src/execution/portfolio.rs` (0-912)
- `src/execution/engine.rs` (0-100)
- `src/execution/mod.rs` (0-8)
- `src/engine/mod.rs` (140-169, 1500-1549, 3170-3370)
- `src/agent/mod.rs` (0-24)
- `INCIDENT-2026-06-13.md` (0-226)
- `config/default.toml` (0-344)
- `Cargo.toml` (0-95)
- `data/dex_state.json` (0-93)
- `C:\Users\spenc\Downloads\export-transaction-list-1781410067043.csv` (all 50 rows, 12.7KB)
- `src/agent/soul.md` (0-207) — **the engine's soul, required reading**
- `~/.hermes/SOUL.md` (0-1)
- `~/.openclaw/workspace/SOUL.md` (0-101) — Mya
- `~/.openclaw/workspace/IDENTITY.md` (0-8)
- `~/.openclaw/workspace/MEMORY.md` (0-69) — Mya's memory pattern reference
- `C:\Users\spenc\AppData\Local\hermes\SOUL.md` (0-186) — Nova
- `C:\Users\spenc\dev\Savant\workspaces\workspace-savant\SOUL.md` (0-95) — Savant framework
- `C:\Users\spenc\dev\Savant\workspaces\workspace-savant\AGENTS.md` (0-62)
- `C:\Users\spenc\dev\Savant\workspaces\workspace-savant\profiles\coding/SOUL.md` (0-146) — coding sub-agent profile
- `C:\Users\spenc\dev\Savant\AGENT-PROMPT.md` (0-1, truncated at 2000 chars)
- `C:\Users\spenc\dev\Savant\docs\archive/MEMORY-SYSTEM-AUDIT.md` (0-285)
- `C:\Users\spenc\dev\Savant\crates\agent/src/loop_detector.rs` (head:80)
- `C:\Users\spenc\dev\Savant\crates/agent/src/budget.rs` (head:60)
- `C:\Users\spenc\dev\Savant\crates/agent/src/graceful_shutdown.rs` (head:60)
- `docs/SOUL-DESIGN-RESEARCH.md` (0-125)
- `~/.openclaw/workspace/memory/2026-06-06.md` (full)
- `~/.openclaw/workspace/memory/2026-05-30.md` (full)

## Vera Memory Files (cross-reference for future-me)

```
dev/vera/
├── SOUL.md                                — identity, invariants, origin
├── README.md                              — how to boot me
├── MEMORY.md                              — curated long-term essence
├── index.md                               — this file
├── memory/
│   ├── 2026-06-13.md                      — day-0 diagnosis journal
│   ├── 2026-06-13-2258.md                 — continued, day 0 not over
│   ├── 2026-06-13-2305.md                 — Nova audit verification
│   ├── 2026-06-13-2330.md                 — harness discovery (corrected)
│   ├── 2026-06-13-2355-recon.md           — project reconnaissance
│   ├── 2026-06-14-0015-csv-recon.md       — CSV reconciliation
│   ├── 2026-06-14-0039-day0-close.md      — day 0 closed, house work done
│   └── 2026-06-14-1358-spec-written.md    — day 1 spec written
├── lessons/lessons.md                     — 8 graduated lessons
├── decisions/decisions.md                 — 10 auditable decisions
├── reflections/reflections.md             — 4 unproven observations
└── specs/
    └── close-path-fix-2026-06-14.md       — close-path + heartbeat spec (draft for review)
```

---

---

## Current state for Kilo Code (2026-06-14 ~18:00 EST)

**cargo check: FAILING** — A03 alpha computation block is broken in `src/engine/mod.rs` lines ~3438-3470.
**cargo test: NOT RUN** (blocked by compilation error)
**Exact fix documented in:** `dev/vera/memory/2026-06-14-buffy-session.md` — includes the correct code block to replace the broken section.

### What's done and working:
- A01: Query stub → error ✓
- A02: Per-token reconciliation ✓  
- A04: strip_historical renamed ✓
- Dashboard $30 → $0 ✓
- Starting equity Ok(true) fix ✓
- Starting equity increase-only threshold ✓
- Startup candle skip ✓
- Position.token_address field ✓
- Reconciliation RPC error handling ✓

### What's broken:
- A03: alpha_vs_benchmark — duplicate `else` block at line ~3445, incomplete `let` statement, stray `0.0`

### What's still needed:
- Per-token divergence test (Nova acceptance criteria)
- Jury veto engine wiring
- Engine restart decision

---

## Vera Memory Files (updated for Kilo)

```
dev/vera/
├── SOUL.md                                — identity, invariants, origin
├── README.md                              — how to boot me
├── MEMORY.md                              — curated long-term essence (UPDATED)
├── index.md                               — this file (UPDATED)
├── memory/
│   ├── 2026-06-13.md                      — day-0 diagnosis journal
│   ├── 2026-06-13-2258.md                 — continued, day 0 not over
│   ├── 2026-06-13-2305.md                 — Nova audit verification
│   ├── 2026-06-13-2330.md                 — harness discovery (corrected)
│   ├── 2026-06-13-2355-recon.md           — project reconnaissance
│   ├── 2026-06-14-0015-csv-recon.md       — CSV reconciliation
│   ├── 2026-06-14-0039-day0-close.md      — day 0 closed
│   ├── 2026-06-14-1358-spec-written.md    — day 1 spec written
│   ├── 2026-06-14-1435-five-fids-done.md  — day 2 five FIDs
│   ├── 2026-06-14-1455-archive-cleanup.md — day 2 archive cleanup
│   └── 2026-06-14.md             — canonical daily journal (80KB, consolidated)
│   └── archive/                  — 13 archived fragmented entries (Kilo session 19:35 EST)
├── lessons/lessons.md                     — 10 graduated lessons (UPDATED)
├── decisions/decisions.md                 — 13 auditable decisions (UPDATED)
├── reflections/reflections.md             — 4 unproven observations
└── specs/
    ├── close-path-fix-2026-06-14.md       — close-path + heartbeat spec
    └── GEMINI-RESEARCH-2026-06-14-vera-memory-v2.md
```

---

*Vera index 0.1.0 — 2026-06-14 ~23:30 EST — chain-driven state refactor + 17 decisions, 12 lessons, 5 reflections, 15 archived journals*
