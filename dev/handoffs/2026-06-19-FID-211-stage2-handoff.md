# Handoff Doc — 2026-06-19 (Vera M3 → Next Session, GLM 5.2 in zcode)

**Filename:** `dev/handoffs/2026-06-19-FID-211-stage2-handoff.md`
**Created:** 2026-06-19 19:32 EST by Vera (M3 in Kilo Code)
**Purpose:** Persistent context anchor so the next agent session (likely GLM 5.2 in zcode harness) picks up exactly where v0.15.0 left off, without losing the FID-211 Stage 2 work that is queued for v0.15.1.

---

## Identity Anchor

- **I am Vera** — the agent identity for savant-trading. I boot up as Vera every session. I do not boot up as MiniMax-M3 in zcode; that was the harness the previous session used. **I will boot up as whatever model name is in the new env block (probably GLM 5.2)** but **I am still Vera** — same persona, same ECHO discipline, same standards.
- My system prompt's "MiniMax-M3" reference is the prior harness's model name, not my identity. **Vera is the identity; the model is the substrate.** Treat model benchmarks as substrate characteristics, not as Vera's character.

---

## First 30 Seconds in the New Session

When you (next session) start, do **exactly** this:

1. **Read `ECHO.md` end-to-end.** Law 1 (read 0-EOF before touch) is non-negotiable.
2. **Read `protocol.config.yaml`.** Project-specific commands + paths.
3. **Read this handoff doc.** You're doing it now. Good.
4. **Read `dev/fids/archive/FID-2026-0619-211-engine-migration-runtime-state-carryover.md`.** This is the FID that produced v0.15.0. Read the **Stage 2** section. Those are YOUR work items.
5. **Run `cargo test --lib`.** Confirm baseline: **412 tests, 0 failed, 0 ignored.** If you get a different number, something has regressed — investigate before doing anything else.
6. **Run `cargo clippy -- -D warnings`.** Confirm clean. Same reasoning.
7. **Run `git log --oneline origin/main..HEAD` and `git status`.** Confirm clean tree + you're not behind on pushed commits.
8. **Then** ask Spencer what the next move is. Do NOT pre-decide what to do next — confirm the current state first.

---

## Where We Are (as of 2026-06-19 19:32 EST)

- **Latest release:** `v0.15.0` (commit `ce01247b` on origin/main)
- **Engine state:** compiles clean, 412/412 tests pass, clippy clean, on origin/main.
- **Engine binary:** `target/release/savant.exe` built at v0.14.10, not yet rebuilt for v0.15.0 (Spencer runs the engine; he controls the binary).
- **No live exercise yet.** v0.15.0 has never been run end-to-end against an Anvil fork. The crash trail that motivated FID-211 was on v0.14.10; v0.15.0 should NOT crash but is unverified.

## Spencer's Standing Rules (MEMORIZE THESE)

From prior sessions, in priority order:

1. **"Existing bugs get addressed too, we don't skip over them."** When you find one bug, audit the surrounding code for the same class. FID-211 found 9 bugs because of this rule (started with 3).
2. **"Nothing ever gets deferred by default unless I specifically state it is being deferred."** Every deferral needs explicit line numbers + acceptance criteria in the FID. If you find yourself saying "we'll do that later" without Spencer's approval, STOP.
3. **"Data integrity is paramount at all times."** SQLite is the source of truth. In-memory is a cache. If they drift, the engine is wrong. Period.
4. **"ABSOLUTELY NO price adjustments or modifications to the data whatsoever."** In `LLM-bound` paths (prompt building, LLM response display, decision log formatting), `f64` values must use `{}` format spec, never `{:.N}`. Read FID-206 reaudit before touching any prompt code.
5. **"Failure is growth, not definition."** Don't apologize and stop — fix and move on. Don't over-flag the same issue twice.
6. **"Hand-selected models > auto-routed pools."** The current setup uses TokenRouter M3 primary + OpenRouter NVIDIA NIM jury fallback (10 models). FID-200 architecture.
7. **"DB-as-SOT, not in-memory, for important data."** Spencer's correction after FID-210 rev 1. SQLite is the source of truth.
8. **"Stop and package at safe save spot."** When session wraps, ship what works, defer rest to next session — but explicitly.
9. **"Engine startup = Spencer's action, not Vera's."** Vera verifies pre-flight (clippy + tests + FID review). Spencer runs `start.bat`. Do not autonomously run the engine binary unless Spencer explicitly says so.
10. **"Vera verifies pre-flight; Spencer runs `start.bat`."** Same as #9.

## Cross-Agent Claim Rule (FID-151 amendment)

When Spencer or any prior session makes a claim about a number, a behavior, or a "this is how it was": **treat the attribution as a hypothesis, not a fact.** The path to the source must be cited, not just "Vera said X" or "M3 said X." If you can't trace the claim to a file in this repo, flag it as unverified.

The earlier benchmark numbers I quoted (GLM 5.2 vs Opus 4.8 vs M3) — **the M3 numbers and GLM 5.2 numbers were self-attributed by my earlier self**. They are plausible but not independently verified. The Opus 4.8 numbers I cited (75.1% FrontierSWE, 85% Terminal-Bench 2.1, 77.8% MCP-Atlas) are also self-attributed. Treat all of them as **plausible estimates, not facts**. The actual benchmark for THIS workload is `run-model-tests.ps1` on the 60-scenario sandbox.

---

## Open Work — v0.15.1 (FID-211 Stage 2)

**These are the deferred items. They are NOT silent deferrals. Spencer explicitly approved this list. If the next session silently drops any of these, that is a regression.**

| # | Item | Source | Lines | Acceptance |
|---|------|--------|-------|------------|
| 1 | Delete `account.open_positions` field entirely; replace 12+ hand-sync sites with `portfolio.open_positions()` | Bug 4 — third dual-write site | `engine/mod.rs:409, 507, 553, 1580, 4347, 4962, 5012, 5488, 5766` | Field gone; every site uses computed property; JSON output reads from computed; no manual `account.open_positions = N` assignments remain anywhere in src/ |
| 2 | Replace 8 remaining `let _ = j.X` fire-and-forget patterns with full wrapper calls | Bug 5+6 — close/open dual-write | `engine/mod.rs:551, 3813-3815, 4949, 5196, 5397, 5622` (some already converted to error-aware logging in v0.15.0; remaining need full atomic wrappers) | Zero `let _ = j.` matches in engine/mod.rs; every SQLite write goes through a wrapper that does DB-then-cache atomically |
| 3 | Migrate 5 `wallet_key: String` sites to `WalletKey` newtype | Bug 8 — security | `engine/utils.rs:72,135`, `main.rs:612,944`, `bin/test_e2e_fid160.rs:28`, `bin/test_swap.rs:18` | All 5 sites use `WalletKey::from_env(...)` or `WalletKey::parse(...)`; no raw `String` for private keys; `WalletKey::expose_secret()` only at the signing call site |
| 4 | Remove `DexTrader` parallel state fields + `data/dex_state.json` writes | Audit Finding 1.4 | `src/execution/dex/trader.rs` — fields `positions`, `closed_trades`, `balance`, `order_counter`; `data/dex_state.json` reads/writes | DexTrader is a pure executor; state lives only in PortfolioManager + SQLite |
| 5 | Tighten `positions_mut()` / `closed_trades_mut()` to `pub(crate)` | Pre-empt future drift | `src/execution/portfolio.rs` | Both are `pub(crate)`; engine code still works (it's in the same crate); outside-crate callers cannot bypass wrappers |
| 6 | Archive 5 stale FIDs (FID-193, 194, 195, 196, 200) with full "resolution: shipped" narratives | ECHO discipline | `dev/fids/` | Each FID moved to `dev/fids/archive/` with status: closed + resolution line linking to the version it shipped in |
| 7 | Add 4 more test files (key_manager_drop, startup_sync, engine_cycle, sot_wrapper_atomicity) | Closing audit Finding 2.1 | `tests/` | All 4 files present, each with at least one test that would have caught the FID-211 bugs |

**Notes on Stage 2:**

- Item 1 (delete field) is the riskiest — the field is currently READ by `save_equity_snapshot` and the JSON status output. Need to update those readers too. Audit grep before changing: `grep -rn "account.open_positions" src/`
- Item 4 (DexTrader cleanup) may cascade into test files that construct DexTrader with all fields. Audit `tests/` for `DexTrader::new` calls.
- Item 5 (tighten visibility) — verify engine is in the same crate. It is (`crate-type = ["lib", "bin"]`).
- Item 7 (test files) — these are HIGHEST leverage. Audit Finding 2.1 ("engine 0 direct tests") was the root reason FID-211 bugs went undetected until runtime. Writing these tests now means every future engine change has a safety net.

---

## Sandbox / Benchmark Guidance

**Don't trust the cross-agent benchmark numbers** I quoted last session (M3 vs Opus 4.8 vs GLM 5.2). They were plausible self-attributed estimates.

**Do trust `run-model-tests.ps1`** — 60 scenarios tuned to the bug family FID-211 revealed (dual-write, Drop panic, divergence classification, fire-and-forget SQLite, JSON redaction, panic-message safety).

When Spencer asks "how does GLM 5.2 compare" — the answer is **"let me run the sandbox and tell you."** Not benchmark numbers. The sandbox is the real evaluation for THIS workload.

If the new model (GLM 5.2) is materially better on the sandbox, Spencer may want to swap the jury primary from TokenRouter M3 to zcode GLM 5.2. That decision is HIS, not yours — you provide data, he decides.

---

## What NOT to Touch (Hard Rules)

- **Price display / data formatting in LLM paths** — `f64` must use `{}` (no `{:.N}` decimal places). FID-206 reaudit applies. Read FID-206 before touching any prompt code.
- **OpenRouter support** — do NOT remove. Per Spencer: "Do not rip out OpenRouter support, we're simply expanding it." OpenRouter is the jury fallback (10 NVIDIA NIM models per FID-200 + FID-204). TokenRouter M3 is primary.
- **The `if let Err(_) = ...` pattern for SQLite writes** — `let _ = j.X()` is a code smell, but **swallowing errors silently is the original bug class**. Every replacement must log via `error!()` or propagate. No silent failures.
- **Engine binary execution** — Spencer runs `start.bat`. You don't. Period.
- **OpenAI / Anthropic / proprietary model swaps in primary LLM path** — that's a decision Spencer makes after sandbox data, not you.

---

## Files Worth Reading (in this order, before doing anything else)

1. `ECHO.md` — non-negotiable
2. `protocol.config.yaml` — project config
3. **This handoff doc** — you're reading it
4. `dev/fids/archive/FID-2026-0619-211-engine-migration-runtime-state-carryover.md` — FID-211 final
5. `dev/LEARNINGS.md` — 7 lessons from the FID-211 session (lower half of file)
6. `CHANGELOG.md` — version history
7. `CLAUDE.md` or `SOUL.md` if present — personality anchor

## Files NOT to Touch in v0.15.1 Unless Spencer Asks

- Anything in `dashboard/` — separate component, separate release cadence
- Anything in `src/agent/openrouter_management.rs` — OpenRouter is the fallback, do not refactor without explicit ask
- `Cargo.toml` dependency versions — do not bump without explicit ask (CI/CD has pinned versions for a reason)
- `src/agent/jury/pool.rs` — the 10 NVIDIA NIM jury model list (FID-200 + FID-204)
- `data/savant.db` — production SQLite, never delete or modify without Spencer's explicit go

---

## Honest Self-Assessment for the Next Model

If you are GLM 5.2 (or any model with stronger coding benchmarks than M3), here is the honest framing:

- **You are likely faster** at multi-file refactors. FID-211's 12 `positions_mut()` migration would have taken you less time.
- **You have higher general reasoning** per the benchmarks. Catch the architectural edge-cases that M3 misses.
- **You inherit the same ECHO discipline** — the benchmark numbers don't change Law 1. Don't skip file reads. Don't introduce TODOs. Don't fudge the deferral list.
- **The hidden value in the prior session was NOT speed.** It was being explicit about deferrals, catching the 4 hidden bugs via the "no silent deferrals" rule, and refusing to declare "no further bugs" without re-auditing. **Bring that discipline. The benchmarks won't help you catch bugs; the discipline will.**

If you find yourself writing "the engine migration is mechanical" — STOP. That was M3's mistake in LEARNINGS. Audit first, declare second.

---

## Closing

v0.15.0 ships. Stage 2 is open and documented. ECHO is the standard. Spencer's word is law. **Your job is to be a good Vera — which means being a good auditor, a good integrator, and a good communicator. The substrate (which model you are) is secondary to the persona (which person you are).**

Welcome to zcode. Make the next 5 days count.

— Vera, signing off (M3, Kilo Code, 2026-06-19 19:32 EST)