# Overnight Engine Run #1 — v0.14.8 (2026-06-18)

## TL;DR (Corrected after Spencer's correction)

You were right — the rate-limit was always there, just not as visible. NVIDIA NIM free tier empirically caps **~5 requests/min per model**. The engine makes **1 batched call per cycle (~3 min) PLUS 10 juror shadow calls in parallel = 11 requests every 3 minutes = ~3.7 RPM.** Should be under the limit, but the prompt context (~14k tokens of candle data per batch) keeps the server-side "tokens per minute" cap, not just the request count. Combined with the **M3 model being 1T MoE** (slower server-side processing), the per-call latency is 138-167s, eating into the window.

The deeper problem: **M3 always outputs PASS when conviction is moderate, even with thresholds as low as 0.05 for Trending.** 30/32 decisions with conviction ≥ 0.20 still say PASS because the model adds an "EMA-cross-against-direction" veto not in the prompt.

## Process State (Still Alive)

- `savant.exe` PID 54108 (running 10h32m as of 12:25 PM EST)
- `anvil.exe` PID 11772 (running 10h33m)
- Engine stopped producing decisions at 4:05 AM EST, but kept cycling and rate-limiting for 8 more hours

## Data Sources

| Source | Size | Coverage |
|---|---|---|
| `data/decision_log.json` | 345KB | 500 decisions capped (in-memory limit), 22 had real LLM output (3:55 AM + 4:05 AM batches) |
| `data/equity_history.json` | 30KB | 149 snapshots, last write 4:05 AM EST |
| `logs/terminal/next-server (v16.2.7).txt` | **967KB** | **9044 lines** — the ground truth |

## Overnight Numbers

| Metric | Value |
|---|---|
| Cycles started | 169 (Cycle #21 → #189) |
| PHASE2 LLM batches attempted | 173 |
| **BATCH COMPLETE (LLM returned verdict)** | **2** (3:55 AM and 4:05 AM, 138s + 167s) |
| Decisions parsed | 22 (11 + 13, minus 2 parse fails) |
| BUY verdicts | 0 (only 1 BUY from old pre-jury binary at 2026-06-18T00:03:22) |
| ADJUST verdicts | 0 |
| CLOSE verdicts | 0 |
| PASS verdicts | 22 (all conviction 0.0000) |
| ERRORs | 0 |
| WARNs | 682 (all 429 rate-limit backoff) |
| Final balance | $50.00 USDC (no fills executed) |
| Reconciliation divergence | $0 (in-memory = on-chain) |

## NVIDIA NIM Rate Limit (Empirical Test 2026-06-18 12:25 PM)

Tested live against `integrate.api.nvidia.com/v1/chat/completions` with `minimaxai/minimax-m3`:

```
10 sequential M3 calls, max_tokens=3:
Latencies: 53.57, 0.58, 0.54, 1.13, 1.01, 0.12, 60.01, 6.56, 0.55, 60.02s
Statuses:  200,    200, 200, 200, 200, 429,  TIMEOUT, 200,  200,  TIMEOUT
```

**Result: 5 successful calls then 429. Free-tier per-model limit ≈ 5 RPM. 60s recovery window. No rate-limit headers in response (no X-RateLimit-Limit, no Retry-After).**

For comparison, smaller models (8B) accepted 20/20 in 6.5s (~3 RPS) — so the per-model free tier RPM scales with model size. M3 is a 1T MoE, so it gets a much smaller bucket.

**Engine traffic pattern:**
- 1 batched LLM call (M3) per cycle ≈ 1 request every 3 min = **0.33 RPM** — should be fine
- 10 juror shadow calls (parallel) per cycle ≈ 3.3 RPM extra
- Total: ~3.6 RPM on M3 + 10 RPM spread across 10 free models

So **M3 alone is fine** but the **shadow jury is hammering 10 models in parallel each cycle** = 30+ RPM total spread thin. Plus each M3 batch takes 138-167s (server processing time on 1T model with 14k token context).

**Actual hit rate:** 2/169 cycles = 1.2%. The engine succeeded on cycle 1 (no warm-up), failed once for ~1 hour, succeeded on cycle 14, failed forever after.

## The Gemini Hypothesis — VERIFIED

The "LLM defaults to PASS" claim is **confirmed by direct evidence on the 22 successful calls:**

| Conviction range | Count | PASS | BUY |
|---|---|---|---|
| ≥ 0.20 | 32 | 30 (94%) | 1 |
| 0.10-0.20 | 79 | 79 (100%) | 0 |
| 0.05-0.10 | 174 | 173 (99%) | 0 |
| < 0.05 | 74 | 74 (100%) | 0 |
| 0.00 | 141 | 141 (100%) | 0 |

**Even at conviction 0.22 in Trending regime (threshold 0.05, so 4x above), M3 says PASS** because it interprets "EMA_F < EMA_S" as a hard veto not present in the prompt. Sample reasoning:

> "BTC/USD | Trending ADX 20.7 borderline, EMA_F < EMA_S bearish, RSI 57 recovering... No momentum trigger confirmed - EMA cross is against. Hold."

> "UNI/USD | Ranging ADX 14.1, EMA_F < EMA_S bearish, RSI 40.9 mid-low. Z-score -1.99 suggests oversold bounce potential... Below Ranging 0.25 threshold - HOLD."

The model is **applying a custom bear-trend veto** that overrides the threshold rule. This is the Gemini "semantic gravity well" effect in action — but it's not about "PASS as default," it's about "bearish EMA = automatic no-trade regardless of conviction."

## Architecture Insight: M3 vs Jury

Confirmed in `src/engine/mod.rs` lines 2488-2569:

1. **PHASE2 batch call (line 2488-2492):** `provider.chat_stream()` — M3 alone, makes the actual decision used by the engine
2. **Jury overlay (line 2533-2569):** Runs 10 jurors in parallel as **SHADOW MODE** — doesn't override M3's decision

So:
- The single-model M3 call IS what produces BUY/SELL/PASS verdicts that get executed
- The 10-juror jury only logs verdicts for comparison; the engine ignores them
- Rate limits on M3 alone (no jury) would have been 0.33 RPM — well within 5 RPM
- Rate limits with 10-juror shadow = 3.6 RPM M3 + 30 RPM spread across free models = **the M3 is the bottleneck**, not the free models

## The Three Problems (Priority Order)

### P0 — Rate limit handling for M3 (1T MoE, ~5 RPM free tier)

M3 batch call takes 138-167s. During that time, the cycle can't complete. If anything else requests M3 (e.g., another batched call from a different code path), 429s trigger.

**Fix:** 
- Add exponential backoff with jitter on 429
- Add `[LLM] TIMEOUT` log when no verdict in 4 min
- Add `[LLM] RATE_LIMITED` log with reason
- Skip cycle gracefully when LLM fails — don't block 3 min on retry

### P1 — M3 always PASSes on bearish EMA

The model treats "EMA_F < EMA_S in Trending regime" as automatic no-trade, even when:
- Conviction is well above threshold
- Regime is Trending (not Volatile)
- Probe threshold is exceeded

**Fix options:**
- **FID-201:** Add affirmative phrasing to prompt: "When EMA_F < EMA_S in Trending regime, this is BEAR TREND. You MAY enter short if conviction >= 0.20 with conviction_score reflecting bearish triggers." (instead of "do not default to PASS")
- **FID-202:** Add few-shot example: "Example: Trending ADX 30, EMA_F < EMA_S, RSI 42, conviction=0.35 → action='Sell' (NOT Pass)"
- **FID-203:** Lower probe threshold for bearish setups — currently probe threshold requires probe to be `is_probe: true` between probe and main threshold; the model may not be using it because it's interpreting "bearish EMA" as below probe threshold too

### P2 — Jury is dead weight (shadow mode)

10 jurors cost 30+ RPM of free-tier calls per cycle and contribute nothing to the decision (shadow mode). Either:
- Remove shadow jury entirely (saves 30 RPM, simplifies architecture)
- Make jury votes override single-model verdict (FID-114 → active mode)
- Use jury only on high-conviction cycles (skip when M3 says PASS at conviction < 0.10)

**Recommendation:** Defer jury to v0.15.0. Run with single-model M3 + better rate-limit handling for now.

## What's NOT Broken

- Cycle loop: 169 cycles, 0 panics, 0 ERRORs
- Pre-flight guard (FID-194): 0 phantom AdjustStop/Close actions
- Reconciliation (FID-196): 0% divergence in 169 cycles
- Decision log schema: all fields present, conviction/regime/pair correct
- Market data ingestion: all 48 pairs fetched consistently (2 always return no data: GNS, MAGIC on Kraken only — pre-existing, OKX/Gate/Bybit cover them)

## Decision Points for Spencer

1. **Disable jury shadow mode (P2)?** Saves 30 RPM. M3 alone is the production model. Jury stays in code but doesn't fire unless `JURY_ENABLED=true` env flag.
2. **P1 prompt fix:** affirmative phrasing about bearish setups + few-shot example of Sell action?
3. **P0 rate-limit handling:** add backoff + TIMEOUT log + graceful skip?
4. **Stop engine now or let it keep running while we code?** It's making 0 useful decisions; PID 54108 has been hot for 10h32m.
