# Training Workflow — Closed-Loop Development Process

> How we build, audit, and evolve the trading engine under ECHO Protocol discipline.

## Overview

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   BUILD     │───>│   AUDIT     │───>│  FEED BACK  │───>│   EVOLVE    │
│             │    │             │    │             │    │             │
│ Kilo writes │    │ Mya + Nova  │    │ Findings →  │    │ Protocol    │
│ code, runs  │    │ independent │    │ protocol +  │    │ updates,    │
│ training    │    │ review      │    │ agent       │    │ agent syncs │
└─────────────┘    └─────────────┘    └─────────────┘    └──────┬──────┘
       ^                                                        │
       └────────────────────────────────────────────────────────┘
                          Training resumes
```

## Roles

| Agent | Role | What They Do |
|-------|------|--------------|
| **Kilo** | Builder | Writes code, runs training cycles, fixes findings |
| **Mya** | Auditor (OpenClaw) | Reviews code, protocol compliance, training metrics |
| **Nova** | Auditor (Hermes) | Independent review, cross-validates Mya's findings |
| **Spencer** | Orchestrator | Directs priorities, approves changes, owns the repo |

## The Loop

### 1. Build

- Kilo writes code or runs training under ECHO Protocol
- Training runs: `cargo run -- --test --train -n 60`
- Build verification after every change: `cargo build && cargo test && cargo clippy -- -D warnings`
- FIDs created for any issues found during development

### 2. Audit

After training completes or code changes are pushed:

- **Independent review** — Mya and Nova audit separately, no cross-contamination
- **Code audit** — Security, correctness, error handling, protocol compliance
- **Training audit** — Brier scores, calibration, category edge, confidence distribution
- **Protocol audit** — Version propagation, `/dev` structure, FID lifecycle

### 3. Feed Back

- Findings posted to Discord `#general` channel
- Both auditors converge on priority and severity
- Critical findings sent directly to Kilo for immediate fix
- Protocol findings filed as PRs/issues on savant-protocol repo

### 4. Evolve

- Protocol gets updated (version bump, CHANGELOG, coding standards)
- Trading agent auto-syncs on next cycle when it detects protocol version change
- FIDs closed with evidence, archived to `dev/fids/archive/`
- LEARNINGS.md updated with lessons from the cycle

## Training Session Structure

### Per-Run Checklist

- [ ] Build clean (`cargo build`)
- [ ] Tests pass (`cargo test`)
- [ ] Clippy clean (`cargo clippy -- -D warnings`)
- [ ] Protocol version current (`protocol.config.yaml`)
- [ ] `/dev` folder matches protocol structure

### Metrics to Track

| Metric | Target | What It Measures |
|--------|--------|------------------|
| Brier Score | < 0.25 | Overall calibration |
| 50-75% Confidence Accuracy | > 80% | Medium-conviction reliability |
| 75-100% Confidence Accuracy | > 85% | High-conviction reliability |
| 0-25% Confidence Trades | 0 | Should all be holds |
| Action Rate | 40-60% | Not too aggressive, not too passive |
| Error Rate | 0% | Clean execution |
| Knowledge Utility | Trending up | Agent is learning |
| Auto-Lessons | 3-7 per run | Active learning |

### Red Flags to Watch

- **Short bias** — If >70% of trades are SHORT, the agent isn't buying capitulation
- **Brier oscillation** — If Brier bounces instead of converging, calibration is unstable
- **0-25% confidence trades** — Agent making trades it has no confidence in
- **On-Chain category at 0%** — Agent's weakest category for long entries
- **Stream parse errors** — Should be <2% of scenarios, recoverable via retry

## FID Lifecycle

```
Created → Analyzed → Fixed → Verified → Closed → Archived
```

- **Created** — Issue discovered during audit or training
- **Analyzed** — Root cause identified, severity assessed
- **Fixed** — Code change applied, build verified
- **Verified** — Fix confirmed by auditor(s)
- **Closed** — FID moved to `dev/fids/archive/`, CHANGELOG updated
- **Archived** — Available for future reference

## Protocol Sync

When the trading agent detects a protocol version mismatch:

1. Clone latest savant-protocol from GitHub
2. Sync all protocol files (ECHO.md, VERSION, STARTER-PROMPT, etc.)
3. Update `protocol.config.yaml` version field
4. Verify build + tests + clippy
5. Resume training

**Auto-sync happens on every training cycle start.** No manual intervention needed.

## Session Lifecycle

### Start of Session

1. Read ECHO.md
2. Load `protocol.config.yaml`
3. Verify build + tests + clippy
4. Review open FIDs in `dev/fids/`
5. Review `dev/LEARNINGS.md`
6. Create session summary in `dev/session-summaries/`

### During Session

7. Run training cycles
8. Auditors review findings
9. Feed back to protocol + agent
10. Update FIDs, LEARNINGS.md

### End of Session

11. Run all validation commands
12. Update session summary
13. Note blockers
14. Update LEARNINGS.md

## Version History

| Date | Version | Change |
|------|---------|--------|
| 2026-06-01 | 1.0 | Initial workflow document — formalized from live session |
