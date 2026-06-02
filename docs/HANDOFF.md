# HANDOFF — Session 2026-06-02

## Current State

- **Version:** 0.4.4
- **Protocol:** ECHO v0.1.0
- **Tests:** 143 (139 lib + 4 binary)
- **Clippy:** Zero warnings
- **Build:** Clean
- **Pushed:** `aeb29ce` on main
- **Episodes:** ~1,200+ in test_memory.db
- **Brier:** 0.172 (best ever, run 3 of last training)

## What Was Built This Session

### Gemini Deep Research (FID-015)

Two independent reports analyzed Savant against MiMo v2.5 Pro's architecture. Produced 17 recommendations, all implemented across 9 steps:

1. **Config overhaul** — Fee 0.40%, daily loss 10%, drawdown 20%, R:R 2.0, temp 0.6, top_p 0.95, 500 candles, 300s timeout, 128K tokens
2. **Maker order support** — `order_type` field (LIMIT/MARKET), maker fee corrected to 0.25%
3. **Prompt architecture** — XML tags, 5-step structured reasoning, corrected fees
4. **Session liquidity** — 9 UTC-based sessions, breakout penalties
5. **Garman-Klass volatility** — OHLC-based estimator
6. **Isotonic Regression** — PAVA confidence calibrator
7. **Causal attribution** — Four-factor loss classification
8. **Historical tick data** — Fetch/cache from Kraken API
9. **JSON repair** — Truncated strings, extra text, mid-word EOF

### Data Pipeline Fixes

- BGeometrics API: returns `[{d, unixTs, mvrv}]` array, not `{date, value}`. Parser rewritten.
- Funding: `fetch_funding_multi()` now uses OKX primary + Kraken fallback with range validation.
- RSS: Aggregator now calls `fetch_all_feeds_capped()` instead of uncapped version.

### Confidence Floor + Short Bias (FID-014)

- Confidence floor 40% in `decision_parser.rs` — downgrades low-confidence trades to Hold
- Short bias fix in `scenarios.rs` — boosted capitulation buy signals
- Vault wiring — training batch writes to savant-vault/
- Training default 5 runs (`--full` for 20)

### Wiring Fixes (this session)

- Isotonic Regression wired into training report
- Causal Attribution wired into training pipeline
- Garman-Klass displayed in context builder
- Session breakout penalties displayed in session context
- Historical training `--historical` flag wired to function signature
- Maker fee corrected from 0.16% to 0.25%

## Open Items

- **Historical training full integration** — Data fetch + cache works, but scenarios from history aren't replacing random scenarios yet. Need to convert `HistoricalScenario` to `ScenarioResult` format.
- **System prompt A/B test** — With-memory report says reasoning models degrade with system prompts. Need to test user-only with XML tags vs current approach.
- **response_format constrained generation** — If OpenGateway supports `response_format: { "type": "json_schema" }`, we can force valid JSON at inference level. Untested due to compression issues.
- **LEARNINGS.md** — Updated with session notes.
- **GitHub Actions CI** — Account billing issue (blocked).

## Key Files

| File | Purpose |
|------|---------|
| `config/default.toml` | All runtime config (fee, risk, AI, trading, training) |
| `src/agent/provider.rs` | LLM client (streaming + non-streaming + retry) |
| `src/agent/decision_parser.rs` | JSON parsing (3-pass: strict → repair → partial) |
| `src/agent/context_builder.rs` | build_context(), indicators, Garman-Klass |
| `src/agent/prompts/*.md` | XML-tagged prompt templates |
| `src/engine.rs` | Engine loop, training pipeline, historical wiring |
| `src/data/historical.rs` | Historical data fetcher + cache from Kraken |
| `src/data/indicators.rs` | Technical indicators + Garman-Klass |
| `src/core/session.rs` | 9 UTC-based session liquidity profiles |
| `src/memory/calibration.rs` | Brier score + Isotonic Regression (PAVA) |
| `src/sandbox/feedback.rs` | GEPA + Causal Attribution |
| `src/execution/paper.rs` | Paper trader with maker routing |

## Commands Quick Reference

```bash
# Training
cargo run -- --test --train -n 20              # 20 random scenarios
cargo run -- --test --train --full              # Full 20-run training
cargo run -- --test --train --historical        # Train with real Kraken data
cargo run -- --test --train -c "Trend Bull"     # Filter by category

# Reports
cargo run -- report --test                      # Full training audit

# Engine
cargo run                                       # Live paper trading
cargo run -- --dry-run                          # Single cycle, full pipeline

# Quality
cargo fmt && cargo test && cargo clippy -- -D warnings
```

---

*This handoff is the single source of truth for the next session. Read this before doing anything.*
