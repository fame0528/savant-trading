# HANDOFF — Session 2026-06-01

## Current State

- **Version:** 0.4.3
- **Protocol:** ECHO v0.1.0
- **Tests:** 136 passing, zero clippy
- **Build:** Clean
- **Pushed:** `e2faa19` on main, GitHub release v0.4.3 published
- **Episodes:** ~700+ in test_memory.db
- **Brier:** 0.24 converged

## What Was Built This Session

### Data Pipeline (FID-012 + FID-013)
- BGeometrics on-chain API (MVRV/SOPR/NUPL) — replaces dead CoinMetrics/CoinGecko
- OKX funding primary + Kraken fallback with range validation
- TTL cache with LRU eviction (`data/cache.rs`)
- RSS cap + per-feed timeout + source diversity
- `conditions_summary()` with SOUL.md thresholds
- RSS sentiment classification with negation handling
- Streaming fallback to non-streaming (180s timeout, 2 retries)

### Training Pipeline (FID-010 + FID-011)
- `run_training_batch()` — full pipeline with memory capture
- `run_training()` — convergence loop (Brier delta < 0.02 for 3 runs)
- `run_action_test()` — single-run test with memory
- Semantic consolidation after each batch
- Anti-pattern detection (category-level via condition_tags)
- Knowledge utility update + persistence
- Auto-lesson generation from high-conviction failures
- Training report (`cargo run -- report --test`)

### Risk Model
- `risk/correlation.rs` — Pearson correlation matrix
- Portfolio heat check (40% max) in circuit_breaker.rs
- Spread width halt (50bps) in circuit_breaker.rs
- Dynamic slippage (ATR + book depth) in paper.rs
- Maker-fee routing in paper.rs

### Protocol Alignment
- Synced to ECHO v0.1.0 from GitHub
- `dev/findings/` → `dev/fids/`, `dev/findings/archived/` → `dev/fids/archive/`
- Removed `dev/baselines/` and `dev/plans/`
- Closed FIDs (007-011) archived
- STARTER-PROMPT.md, MIGRATION.md, coding-standards/ all synced

### Workflow
- `docs/TRAINING-WORKFLOW.md` — formalized closed-loop cycle
- `docs/SAVANT_AGENT_REPORT.md` — full agent report for external sharing

## Open FIDs (3 active)

| FID | Status | What |
|-----|--------|------|
| FID-012 | OPEN | Vault wiring (5/8 subfolders empty), knowledge utility persistence |
| FID-013 | OPEN | 20 data pipeline issues (BGeometrics, OKX, cache, RSS, correlation wired) |
| FID-014 | OPEN | Confidence floor (40%), short bias fix, vault, training default 5 runs |

## What's Next (FID-014 priority)

1. **Confidence floor (40%)** — `decision_parser.rs` line 140. If confidence < 0.40 and action != Hold, downgrade to Hold. Highest-impact one-line fix.
2. **Short bias fix** — Boost capitulation weight in `derive_expected_action()` in `scenarios.rs`. Agent only shorts right now.
3. **Vault wiring** — Wire `project_decision()`, `project_trade()`, `project_risk_event()` into training batch. 5 empty folders.
4. **Training default 5 runs** — `--train` runs 5, `--train --full` runs 20.

## Key Files

| File | Purpose |
|------|---------|
| `src/engine.rs` | Engine loop, training pipeline, dry-run, backup |
| `src/main.rs` | CLI (--test, --train, report, --dry-run) |
| `src/agent/provider.rs` | LLM client (streaming + fallback + retry) |
| `src/agent/decision_parser.rs` | JSON parsing (confidence floor pending) |
| `src/agent/context_builder.rs` | build_context(), conditions, tags |
| `src/agent/knowledge.rs` | MMR selection, utility scoring |
| `src/memory/episodic.rs` | Episode capture (SQLite WAL) |
| `src/memory/semantic.rs` | SQL consolidation → patterns |
| `src/memory/anti_pattern.rs` | Category-level failure detection |
| `src/memory/calibration.rs` | Brier score, confidence caps |
| `src/insight/onchain.rs` | BGeometrics MVRV/SOPR/NUPL |
| `src/insight/funding_rates.rs` | OKX primary + Kraken fallback |
| `src/insight/aggregator.rs` | conditions_summary(), RSS sentiment |
| `src/insight/rss.rs` | Capped, source-diverse RSS |
| `src/data/cache.rs` | TTL cache with LRU |
| `src/sandbox/scenarios.rs` | Random scenario generator |
| `src/sandbox/feedback.rs` | GEPA mutation engine |
| `src/risk/correlation.rs` | Pearson correlation |
| `src/risk/circuit_breaker.rs` | Portfolio heat, spread halt |
| `src/monitor/training_report.rs` | Full audit report |
| `src/vault/writer.rs` | Vault journaling (wiring pending) |
| `docs/TRAINING-WORKFLOW.md` | Closed-loop workflow |
| `docs/SAVANT_AGENT_REPORT.md` | External report |

## Databases

| Database | Purpose |
|----------|---------|
| `data/test_memory.db` | Training episodes |
| `data/memory.db` | Live episodes (separate) |
| `data/knowledge_utility.json` | Knowledge utility scores |
| `data/sandbox_candles.json` | Cached Kraken candles |
| `data/backups/` | Rolling SQLite backups (last 7) |

## Training Results Summary

| Cycle | Brier | Actions | Best Category | 50-75% Acc |
|-------|-------|---------|---------------|------------|
| Static ×21 | 0.50-0.58 | 20% | Microstructure 100% | 25% |
| Random ×4 | 0.28-0.30 | 48-62% | On-Chain 100% | 83-87% |
| Random ×6 | 0.24-0.30 | 37-52% | Trend Bull 80% | 81-100% |

**Key finding:** Random scenarios dramatically improve learning (Brier 0.24 vs 0.50).

## Known Issues

- **Short bias** — Almost every trade is SHORT. Agent only goes long on deep capitulation.
- **0-25% confidence bucket** — 18% accuracy. Agent takes trades it has zero confidence in.
- **Vault empty** — 5/8 subfolders have 0 files (Insight, Lessons, Risk, Sessions, Trades).
- **Dead Cat Bounce** — Occasional parse error (empty LLM response). Not critical.
- **Stream errors** — ~5% of LLM calls fail. Streaming fallback handles it.

## Commands Quick Reference

```bash
# Training
cargo run -- --test --train -n 60          # 60 random scenarios with memory
cargo run -- --test --train -c "Trend Bull" # Filter by category
cargo run -- --test --train -a              # Only Buy/Sell scenarios
cargo run -- --test -n 5                    # Quick 5-scenario test

# Reports
cargo run -- report --test                  # Full training audit

# Engine
cargo run                                   # Live paper trading
cargo run -- --dry-run                      # Single cycle, full pipeline

# Quality
cargo fmt && cargo test && cargo clippy -- -D warnings
```

## Protocol

- ECHO v0.1.0 — all files synced from GitHub
- `protocol.config.yaml` — project version 0.4.3, protocol version 0.1.0
- Autonomy level 3 (Autonomous)
- `dev/fids/` for active FIDs, `dev/fids/archive/` for closed
- `dev/LEARNINGS.md` updated with session lessons

---

*This handoff is the single source of truth for the next session. Read this before doing anything.*
