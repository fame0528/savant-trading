# Changelog

All notable changes to this project are documented here automatically by the
agent when a FID reaches **Closed** status. Entries are added in reverse
chronological order (newest first).

Format: Each entry includes the version, date, and changes.

**Version source of truth:** `VERSION` file at project root. All other files
(protocol.config.yaml, ECHO.md, README.md, STARTER-PROMPT.md) should match.
When bumping, update `VERSION` first, then propagate.

**Version history note:** This project was forked from Savant's internal ECHO
Protocol (formerly v4.0.0). The version was reset to v0.0.1 for the public
boilerplate release to reflect independent versioning.

---

## v0.1.0 — 2026-06-01

Minor bump (10th patch). Protocol stable — zero findings across last 3 audits.

- [LOW] MIGRATION.md: protocol/ namespace convention for downstream projects with existing CHANGELOG/README/coding-standards

## v0.0.9 — 2026-06-01

- [LOW] rust.md: .expect() contradiction resolved — forbidden in library code, acceptable only in tests/examples/main.rs
- [LOW] rust.md: aligns with ECHO.md anti-patterns table (unwrap + expect both forbidden in non-test code)

## v0.0.8 — 2026-06-01

- [LOW] README anti-pattern count: 11→10 (matches ECHO.md table)

## v0.0.7 — 2026-06-01

- [LOW] .gitignore: dev/fids/.gitkeep negation fixed (both repo and MIGRATION template)
- [LOW] Anti-patterns cross-reference: points to coding-standards instead of wrong table
- [LOW] SESSION-SUMMARY date format: fixed to YYYY-MM-DD-HHMM (consistent)
- [LOW] MIGRATION.md: overview.jpg added to copy commands and verify tree
- [LOW] README: authority note expanded — ECHO.md is summarized, not duplicated
- [LOW] FSM diagram: SELF-CORRECT→GREEN transition added (corrections flow to re-verify)
- [LOW] README FSM diagram synced with ECHO.md

## v0.0.6 — 2026-06-01

- [LOW] MIGRATION.md verify tree: README.md listed
- [LOW] MIGRATION.md .gitignore template: uses `/*` pattern + archive/.gitkeep negation
- [LOW] Universal starter prompt: aligned to 8 steps (matches all language variants)
- [LOW] README Law 14: language-agnostic wording (matches ECHO.md)
- [LOW] ECHO.md anti-patterns: Law 14 example references language-specific table

## v0.0.5 — 2026-06-01

- [MEDIUM] README Law 6: language-agnostic wording (matches ECHO.md spec)
- [MEDIUM] CHANGELOG v0.0.3: removed duplicate prose block
- [LOW] dev/fids/archive/.gitkeep created (directory scaffolded on clone)
- [LOW] .gitignore: explicit archive/ exclusion for .gitkeep tracking
- [LOW] README verification step 2: max_function_lines + max_line_length confirmed
- [LOW] MIGRATION.md verify tree: includes MIGRATION.md and LICENSE
- [LOW] Double Audit added to ECHO.md Vocabulary table
- [LOW] MIGRATION.md copy commands: README.md included

## v0.0.4 — 2026-06-01

- [MEDIUM] README tree: added MIGRATION.md and LICENSE
- [MEDIUM] README Law summary: all 11 extended laws now listed
- [MEDIUM] Law 14: language-agnostic wording (replaces Rust-specific Result/?)
- [MEDIUM] C# starter prompt: interfaces confirmation added
- [LOW] Double Audit defined in ECHO.md (two independent methods, no self-reporting)
- [LOW] TS/Python prompts: max_function_lines confirmed (all 6 variants aligned)
- [LOW] MIGRATION.md copy commands: include MIGRATION.md and LICENSE
- [LOW] README config table: added autonomy_level and convergence_passes
- [LOW] .gitkeep files in dev/fids/, dev/fids/archive/, dev/session-summaries/
- [LOW] .gitignore: tracks .gitkeep in gitignored directories

## v0.0.3 — 2026-06-01

- [HIGH] README clarifies ECHO.md as single source of truth
- [HIGH] Go/Java/C# starter prompts added
- [HIGH] Law 3 vs Emergency Procedure contradiction resolved
- [MEDIUM] Final Certification mapped to COMPLETE state
- [MEDIUM] strict_mode: false behavior documented
- [MEDIUM] Quality override precedence: language wins over config
- [MEDIUM] Anti-Patterns: language-specific examples for all 6 languages
- [MEDIUM] Circuit Breaker #2 (500-char sample) fully specified
- [MEDIUM] Savant vs ECHO naming explained
- [LOW] Version source of truth documented (VERSION file canonical)
- [LOW] Version reset from Savant v4.0.0 acknowledged
- [LOW] Law 8 log intent destination (session summary)
- [LOW] Universal prompt aligned to 8 boot steps
- [LOW] LEARNINGS date format fixed to YYYY-MM-DD-HHMM
- [LOW] FID filename convention added
- [LOW] Quickstart notes dev/ directories auto-created

## v0.0.2 — 2026-06-01

- Tiered activation: Core (Laws 1-4 always active) + Extended (Laws 5-15, configurable via `strict_mode`)
- Quality overrides per-language in coding-standards (TS: 400 lines, Python: 400/120, Go: 350/120, Java: 350/40/120, C#: 350/120)
- Honest Assessment nuance: verified claims need proof, design decisions need reasoning
- Language standards: Go, Java, C# coding standards with naming conventions and patterns
- CHANGE_ME boot check: agent halts if language is not configured
- README quickstart hook: 30-second value prop above the fold
- MIGRATION.md: retrofit guide for existing projects with checklist
- Go, Java, C# badges in README

## v0.0.1 — 2026-06-01

Initial boilerplate release.

- ECHO Protocol — universal agent bootstrap (15 Laws, Five Questions, Perfection Loop FSM)
- Language-agnostic configuration via `protocol.config.yaml`
- Coding standards for Rust, TypeScript, and Python
- FID (Feature Implementation Document) lifecycle with auto-archive to `dev/fids/archive/`
- Auto-changelog updates on FID closure
- Session lifecycle management with auto-generated summaries
- Circuit breaker rules (Levenshtein change control, oscillation detection, hard stop)
- Anti-patterns and emergency procedures
- Autonomy levels (Guided, Supervised, Autonomous)
- Starter prompts for universal and language-specific agent activation
- VERSION file for protocol version tracking
- CHANGELOG.md for automated change documentation

<!-- Agent entries are added below this line -->
