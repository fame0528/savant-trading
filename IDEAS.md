# Savant Trading — Improvement Backlog (IDEAS)

Captured 2026-06-03. Things we've discussed but not yet built. Current state:
bot is long-only spot on Kraken (~$45 canary), Tier 1 + Tier 2 edge/selectivity
features are live (trend filter, auto break-even/trailing, bracket TP+OCO, spread
gate, correlation cap, stale-exit). See `memory/savant-trading-project.md` for the
full history.

---

## HIGH PRIORITY — "make money even when BTC is weak"

### 1. Loosen the BTC-downtrend veto in the trend filter
**Problem:** The trend filter blocks ALL longs whenever BTC is in a downtrend
(`Trend filter: skipping LONG X (BTC downtrend)`). But individual coins rip on
their own catalysts even when BTC bleeds (observed: WLD +51%/7d, ENA +19%, NEAR
+10% while BTC was down). The blanket BTC veto throws out legitimate winners.
**Fix:** Make the *pair's own confirmed higher-TF uptrend* the primary gate; only
hard-veto on a *strong* BTC downtrend (not any dip). I.e., a coin in its own
strong uptrend should override mild BTC weakness.
**Where:** `src/engine.rs` — the trend-filter gate in Phase 3 (search
`Trend filter: skipping LONG`). Logic uses `pair_trend` map + `htf_uptrend()`.
Consider: require `pair_state == Some(true)` (confirmed up) to allow when BTC is
down; only block when BTC is in a *severe* downtrend (e.g., below HTF EMA by >X%).
Add config `btc_trend_veto` (bool) and/or `btc_severe_downtrend_pct`.
**Caveat:** Higher variance — longs in BTC-down regimes are riskier; stops/BE/fee
floor still bound the downside.

### 2. Dynamic watchlist / pair discovery (stop using a static 28-pair list)
**Problem:** Bot only scans a fixed list, so it's blind to most movers (ENA, WLD,
ONDO, KAS, POL, FIL, etc. are Kraken-tradeable but not in the list).
**Fix:** Auto-rotate the watchlist to top movers / highest-volume Kraken USD pairs,
refreshed daily. Infra already exists:
- `config/*.toml` `[insight.coinmarketcap]` (top_coins_limit, min_volume_24h,
  min_volatility_score) — currently NOT wired into pair selection.
- `KrakenClient::discover_usd_pairs()` in `src/data/kraken.rs` (already used when
  `scan_all_pairs=true`).
- `src/insight/coinmarketcap.rs` exists for CMC data.
**Where:** `src/engine.rs` `active_pairs` selection (top of `run()`), and a periodic
refresh. Cap the universe (e.g., top 30-40 by 24h volume + volatility) to control
LLM cost/latency.
**Caveat:** Chasing hottest movers = higher variance; keep the watchlist bounded.

---

## MEDIUM — robustness / quality

### 3. Normalize malformed AI confidence (recover dropped decisions)
**Problem:** AI sometimes emits confidence on a 0-10 scale (`got 6.5`, `got 8`),
which the parser rejects → valid setups discarded.
**Fix:** In `src/agent/decision_parser.rs`, if confidence in (1, 10], divide by 10;
if (10,100], divide by 100; else reject. Keep rejecting hallucinated *prices*.
**Note:** Mild guesswork — only do if dropped-decision volume is hurting flow.

### 4. Try a more reliable cheap model
**Problem:** `deepseek/deepseek-v4-flash` fumbles structured JSON fairly often
(hallucinated $0.12 prices, bad confidence). Each fumble = a dropped setup.
**Fix:** One-line config change to `google/gemini-2.5-flash` (cheap, steadier JSON)
in `config/canary.toml` + `default.toml` `[ai] model`. Compare fumble rate.

---

## TIER 3 — once we have ~20-30 real trades (data-driven)

### 5. Confidence-weighted position sizing
Once calibration (Brier) shows the AI's 70%-confidence calls actually win ~70%,
size high-confidence setups bigger and marginal ones smaller. **Do NOT before data
exists** — it amplifies noise. Sizing lives in `src/risk/position.rs`.

### 6. Daily profit/loss circuit
Bank the day after a small profit target; stop after a daily loss cap. Prevents
giving back gains / revenge trading. Add `risk.daily_profit_target` and reuse
existing daily-loss tracking (`account.daily_pnl`, circuit breaker).

### 7. Feed the fee math into the AI prompt
Tell the AI in its system prompt: "you pay ~0.65-0.80% round-trip; never propose a
target that doesn't clear it by 2×." Then it self-filters instead of us rejecting
post-hoc (currently the `min_net_target_pct` floor rejects after the fact). Prompt
lives in `src/agent/` (soul.md / context_builder.rs / prompts.rs).

### 8. Maker take-profit (fee micro-optimization)
Current bracket TP uses Kraken "take-profit" (market-on-trigger = taker). A
"take-profit-limit" resting beyond the trigger could fill as maker (~0.15% saved
per win). Needs live verification of Kraken spot reservation behavior. Low priority
vs the above.

### 9. Backtest before changing params
Use the existing backtest/sandbox harness (`src/backtest/`, `src/sandbox/`,
`savant backtest`) to validate any param change (trend thresholds, min_net, stop
rules) on historical data BEFORE committing to live. Improve deliberately, not by
vibes.

---

## META
The single biggest input right now is **real trade data**. The dormant learning
systems (calibration, CUSUM, experience replay) need ~20-30 completed trades to
activate and start self-tuning. Let the current build run, accumulate a sample,
then revisit this list with evidence (use `stats.ps1` + Kraken balance).
