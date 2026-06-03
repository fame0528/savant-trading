# FID: Inherited clippy lints on fc46e40 base (not from Kraken work)

**Filename:** `FID-2026-0603-001-inherited-base-clippy-lints.md`
**ID:** FID-2026-0603-001
**Severity:** low
**Status:** analyzed
**Created:** 2026-06-03 14:44
**Author:** savant-agent (Kraken rebase session)

---

## Summary

`cargo clippy -- -D warnings` reports 6 lints on branch `feat/kraken-execution-v2`.
None originate from the Kraken execution work — all are pre-existing on the `fc46e40`
base the branch inherited, in files this branch does not modify. They are flagged per
ECHO ("flag ANY issue, even outside scope") but intentionally NOT fixed on this branch:
every affected file is independently rewritten on `origin/main`, so fixing here would
manufacture merge conflicts, and the lints disappear when the other dev merges (their
file versions win).

## Environment

- **OS:** Windows 10 Pro 19045
- **Language/Runtime:** Rust (cargo), clippy
- **Commit/State:** `feat/kraken-execution-v2` @ base `fc46e40`; upstream `origin/main` @ `21b177c`

## Detailed Description

### Problem

`cargo clippy --all-targets -- -D warnings` fails with 6 warnings:

| File:line | Lint | Upstream change to file |
|-----------|------|-------------------------|
| src/agent/knowledge.rs:432 | `unused_mut` | +61 -61 (rewritten) |
| src/agent/knowledge.rs:230 | `clippy::manual_sort_by_key` (use `sort_by_key`) | +61 -61 |
| src/agent/knowledge.rs:314 | `clippy::items_after_test_module` | +61 -61 |
| src/insight/onchain.rs:297 | `clippy::assertions_on_constants` | +1 -1 |
| src/sandbox/generator.rs:305-340 (×3) | `clippy::field_reassign_with_default` | +17 -11 |
| src/sandbox/scenarios.rs:1618 | `clippy::items_after_test_module` | +465 -42 |

### Expected Behavior

`cargo clippy -- -D warnings` clean. On the MERGED result this is expected to hold
because all six lints live in files that `origin/main` rewrites; the merge takes the
upstream versions.

### Root Cause

Branch `feat/kraken-execution-v2` is based on `fc46e40`, which predates upstream's
rewrites of these four files. The lints are an artifact of the stale base, not of the
Kraken changes layered on top.

### Evidence

```text
warning: variable does not need to be mutable        --> src/agent/knowledge.rs:432:13
warning: consider using `sort_by_key`                --> src/agent/knowledge.rs:230:9
warning: items after a test module                   --> src/agent/knowledge.rs:314:1
warning: this assertion has a constant value         --> src/insight/onchain.rs:297:9
warning: field assignment outside of initializer ... --> src/sandbox/generator.rs:305-340 (x3)
warning: items after a test module                   --> src/sandbox/scenarios.rs:1618:1
```

Kraken-work files (`src/execution/kraken.rs`, `src/agent/decision_parser.rs`,
`src/api/mod.rs`) are clippy-clean after this session's fixes.

## Impact Assessment

### Affected Components

- agent::knowledge, insight::onchain, sandbox::generator, sandbox::scenarios (none are Kraken path)

### Risk Level

- [x] Low: cosmetic lints; no runtime effect; resolved by merge

## Proposed Solution

### Approach

Do nothing on `feat/kraken-execution-v2`. Resolve on the integration branch AFTER merge,
where the upstream-rewritten versions of these files are present, OR confirm
`origin/main` already passes clippy clean (the merge owner's responsibility).

### Verification

After merge onto `origin/main`: `cargo clippy --all-targets -- -D warnings` → 0 warnings.

## Resolution

- **Fixed By:** (deferred to merge owner — the other dev)
- **Status note:** Not fixed on this branch by design (merge-conflict avoidance).

## Lessons Learned

A feature branch based on a stale upstream HEAD inherits that HEAD's lint debt. Lints in
files the branch does not own should be attributed to the base and deferred to the merge,
not "fixed" on the feature branch where they widen the diff and collide with upstream.
