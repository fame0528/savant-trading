# FID-136: Release Coordination & Dependency Tracking

**Filename:** `FID-2026-0612-136-release-coordination.md`
**ID:** FID-136
**Severity:** medium
**Status:** open
**Phase:** 2 (Sandbox/release infra — depends on FID-126-134)
**Created:** 2026-06-12
**Source:** Open task identified in MASTER-FID "FID-126-134 Implementation Order" section

---

## Summary

Build a release coordination system that maintains a machine-readable dependency graph across all FIDs, blocks releases when dependencies are unmet, generates a release-readiness report, and owns the `git tag` and `CHANGELOG.md` workflow. Replaces the manual cross-FID dependency checking that is currently described in MASTER-FID as a "workaround" for FID-136.

## Background

The FID-126-134 plan (and the broader 130+ FID archive) has cross-FID dependencies that are tracked in prose only (e.g., "FID-127 depends on FID-126", "FID-133 depends on FID-126, 128, 130"). Without a machine-readable graph, releases are vulnerable to:
- Shipping a FID before its dependencies (e.g., FID-127 with conviction_score field undefined because FID-126 hasn't shipped)
- Drift between the prose description in `dev/fids/MASTER-FID.md` and the actual dependency edges
- Manual check failures during high-velocity release windows

This FID owns the dependency graph, the readiness report, and the tag/CHANGELOG workflow.

## Dependency Graph Format

The dependency graph lives at `dev/fids/dependency-graph.toml`:

```toml
# dev/fids/dependency-graph.toml
# Machine-readable FID dependency graph. Validated by scripts/check_release_readiness.py.
# Format: each FID declares its required FIDs in the [dependencies] array.

[fid-126]
title = "Conviction-Weighted Threshold System"
phase = 1
status = "open"
dependencies = []
blocks = ["fid-127", "fid-130", "fid-132", "fid-133", "fid-134"]
severity = "critical"

[fid-127]
title = "Conviction-Weighted Position Sizing (Rust)"
phase = 2
status = "open"
dependencies = ["fid-126"]
blocks = ["fid-134"]
severity = "high"

[fid-128]
title = "Sandbox Jump-Diffusion Synthetic Data"
phase = 3
status = "open"
dependencies = []
blocks = ["fid-133", "fid-134"]
severity = "high"

[fid-129]
title = "Remove Deep Asian Session Penalty"
phase = 3
status = "open"
dependencies = []
blocks = ["fid-134"]
severity = "medium"

[fid-130]
title = "Counterfactual Grader (OPE + Brier + ECE + HTC)"
phase = 2
status = "open"
dependencies = ["fid-126"]
blocks = ["fid-133"]
severity = "critical"

[fid-131]
title = "Knowledge Unit Absolute-Language Scrub"
phase = 1
status = "open"
dependencies = []
blocks = []
severity = "critical"

[fid-132]
title = "10-Point Checklist → Evaluation Matrix"
phase = 1
status = "open"
dependencies = ["fid-126"]
blocks = ["fid-135"]
severity = "high"

[fid-133]
title = "A/B Test Harness for Prompt Comparison"
phase = 2
status = "open"
dependencies = ["fid-126", "fid-128", "fid-130"]
blocks = []
severity = "high"

[fid-134]
title = "20 Adversarial Scenarios"
phase = 3
status = "open"
dependencies = ["fid-126", "fid-127", "fid-128", "fid-129", "fid-132"]
blocks = []
severity = "high"

[fid-135]
title = "Checklist Modifier Calibration Loop"
phase = 2
status = "open"
dependencies = ["fid-126", "fid-127", "fid-132"]
blocks = []
severity = "medium"

[fid-136]
title = "Release Coordination & Dependency Tracking"
phase = 2
status = "open"
dependencies = ["fid-126", "fid-127", "fid-128", "fid-129", "fid-130", "fid-131", "fid-132", "fid-133", "fid-134", "fid-135"]
blocks = []
severity = "medium"
```

## Release Readiness Check

The script `scripts/check_release_readiness.py` runs before every `cargo release` invocation:

1. **Load dependency graph:** Parse `dev/fids/dependency-graph.toml`
2. **Check status of every FID marked `done` or `implemented`:** For each, verify all `dependencies` are also `done` or `implemented`. If any dependency is `open`, emit ERROR and abort the release.
3. **Check for orphaned FIDs:** If a FID is `done` but its `blocks` list contains `open` FIDs, emit WARN (the `done` FID is not yet consumable).
4. **Generate readiness report:** Output a markdown table to `dev/fids/release-readiness-{date}.md`:

```markdown
# Release Readiness Report — {date}

## Summary
- Total FIDs: 145
- Done: 8
- Open: 12
- Blocked (dependency unmet): 3

## Ready to ship (no unmet dependencies)
- FID-126, FID-131, FID-132 (Phase 1 fast wins)

## Blocked from shipping
- FID-127 (depends on FID-126, status: open) — WAIT
- FID-130 (depends on FID-126, status: open) — WAIT
- FID-134 (depends on FID-126/127/128/129/132, statuses: mixed) — WAIT

## Recommended release order
1. v0.14.0: ship FID-126, FID-131, FID-132 (Phase 1)
2. v0.15.0: ship FID-127, FID-130, FID-133, FID-135, FID-136 (Phase 2)
3. v0.16.0: ship FID-128, FID-129, FID-134 (Phase 3)
```

## CHANGELOG and Tag Workflow

The release script `scripts/release.sh` (already exists per the project) is extended to:

1. **Pre-flight:** Run `check_release_readiness.py` — abort if any open FID blocks a `done` FID.
2. **CHANGELOG.md update:** For each `done` FID in the current release window, append a line:
   ```
   - FID-{ID}: {title} (#{commit_sha})
   ```
   to the appropriate version section (semver inferred from phase).
3. **Git tag:** `git tag -a v{X.Y.Z} -m "Release v{X.Y.Z}: {FIDs shipped in this release}"`
4. **Post-flight:** Update `dev/fids/dependency-graph.toml` to mark shipped FIDs as `released-in = "v{X.Y.Z}"` (for audit).
5. **MASTER-FID.md update:** Move shipped FIDs from "Active" to "Recently Completed" with the version tag.

## Changes

1. **`dev/fids/dependency-graph.toml`** — New file: machine-readable dependency graph (per format above). Initial population extracted from FID-126-134 prose.
2. **`scripts/check_release_readiness.py`** — New script: validates graph, generates readiness report, blocks release if dependencies unmet.
3. **`scripts/release.sh`** — Extend existing release script to call `check_release_readiness.py` as pre-flight, update CHANGELOG.md, and tag.
4. **`scripts/calibration_status.py`** — (Already added by FID-135) — also reads from dependency graph if needed.
5. **`.github/workflows/release.yml`** — New GitHub Actions workflow: on `v*` tag push, run readiness check + CHANGELOG update + post-flight MASTER-FID update.
6. **`docs/release-process.md`** — New document: explains the release process for operators (when to ship which FIDs, how to update the graph, how to interpret the readiness report).

## Edge Cases

1. **FID added retroactively** (e.g., new bug report creates FID-145). Operator runs `scripts/add_fid.py FID-145` which:
   - Creates the FID markdown file from a template
   - Adds an entry to `dev/fids/dependency-graph.toml`
   - Validates the graph (no cycles, no orphans)
2. **Dependency discovered late** (e.g., FID-150 needs FID-127). Operator edits the TOML directly. Next readiness check validates.
3. **Dependency removal** (rare; only if a dependency turns out to be unneeded). Same as #2 but with `dependencies = []`.
4. **Cycles** (FID-A depends on FID-B which depends on FID-A). Reject with ERROR. The readiness check has a topological sort that aborts on cycle.
5. **Self-dependency** (FID-A depends on FID-A). Reject with ERROR.

## Verification

- `python scripts/check_release_readiness.py` exits 0 when no unmet dependencies
- `python scripts/check_release_readiness.py --strict` exits 1 if any open FID is blocking
- Unit tests for graph validation:
  - Empty graph → exit 0
  - Linear chain (A → B → C, all done) → exit 0
  - Linear chain (A done, B open) → ERROR
  - Cycle (A → B → A) → ERROR with cycle path printed
  - Self-dependency (A → A) → ERROR
- Dry-run release: `./scripts/release.sh --dry-run v0.14.0` prints what would be tagged without actually tagging
- CHANGELOG.md diff after a simulated release: contains one line per done FID
- Git tag after a real release: `git tag -l "v*"` shows the new tag

## Live Engine Rollback Plan

This FID is sandbox + release infra only. It does not touch the live engine. **No rollback plan needed.** If `dependency-graph.toml` becomes corrupted, restore from git history.

## Dependencies

- **Depends on:** FID-126, FID-127, FID-128, FID-129, FID-130, FID-131, FID-132, FID-133, FID-134, FID-135 (all of which need to exist as TOML entries in the graph)
- **Required by:** none (this is a meta-FID)
- **Ordering:** Ship in v0.15.0 alongside other Phase 2 work. After FID-136 ships, all subsequent FID additions must update `dependency-graph.toml` to maintain graph integrity.

## Perfection Loop Log

### Iteration 1 (2026-06-12) — Self-review

**Issues found:**
1. **No machine-readable format** — Dependencies were only in prose. Added TOML format with explicit `[fid-NNN]` sections, `dependencies`, `blocks`, `phase`, `status`, `severity` fields.
2. **No validation script** — Without a validator, the graph can have cycles or orphan refs. Added `scripts/check_release_readiness.py` with cycle detection + topological sort.
3. **No release blocking** — Currently nothing prevents shipping FID-127 before FID-126. Added: pre-flight readiness check that aborts release on unmet dependency.
4. **No CHANGELOG automation** — CHANGELOG.md is hand-edited. Added: automatic CHANGELOG append per done FID.
5. **No git tag automation** — Tags are hand-applied. Added: `scripts/release.sh` extension to tag automatically.
6. **No edge case handling** — Added 5 edge cases: retro FID addition, late dependency discovery, dependency removal, cycles, self-dependency.
7. **No MASTER-FID automation** — Operator must move FIDs to "Recently Completed" manually. Added post-flight update.
8. **No `--dry-run` mode** — Operators couldn't preview a release. Added `--dry-run` flag.
9. **No `--strict` mode** — Soft warnings vs hard errors ambiguous. Added: default = WARN only, `--strict` = exit 1.
10. **No documentation** — Process wasn't documented. Added `docs/release-process.md`.
11. **No CI integration** — Readiness check not in CI. Added GitHub Actions workflow on tag push.

**Status:** All issues resolved. Ready for review.

## References

- FID-126 through FID-134: Gemini Deep Research plan (each FID is an entry in the graph)
- FID-135: Checklist Modifier Calibration Loop (also added to the graph)
- `dev/fids/MASTER-FID.md`: human-readable summary that this FID automates
- `scripts/release.sh`: existing release script being extended
- Graph theory: Cormen et al. 2009 (topological sort, cycle detection)
