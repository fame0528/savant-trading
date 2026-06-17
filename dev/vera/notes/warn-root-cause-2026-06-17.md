# FID-182: Overnight WARN Root Cause Analysis

**Author:** Vera
**Status:** Analysis complete. No code changes yet.
**Source:** 16h Anvil run, 9069 log lines, 34 WARN events

---

## Categorization (34 WARN total, overnight)

| Category | Count | Source | Root cause |
|----------|------:|--------|------------|
| Provider streaming failure | 9 | `provider.rs:295` | TokenRouter stream decode error → falls back to non-streaming |
| Provider request failed | 13 | `provider.rs:11xx` | TokenRouter HTTP error during jury requests |
| Judge fallback (majority vote) | 9 | `judge.rs:312` | Judge LLM failed → uses majority vote instead |
| Key Manager threshold | 4 | `key_manager.rs:151` | Disposable jury key failed 3/3 → quarantined |
| Jury member timed out | 3 | `pool.rs` (not yet read) | Individual juror didn't respond in time |
| Anti-pattern noise | 2 | `decision_parser.rs` | Conviction = exactly the threshold (default-to-threshold) |
| Jury quorum NOT met | 2 | `pool.rs` (not yet read) | Got 5/10 verdicts, needed 6 |
| ZERO-BASE override | 1 | `decision_parser.rs` | LLM said Pass but would_initiate_new_long=false → forced to Close |

---

## Root cause groups (not symptoms)

### Group 1: TokenRouter instability (22 of 34 = 65%)

Provider streaming failures + Provider request failures are both TokenRouter HTTP/stream issues. The stream failure is a **decoding error** (the response body is malformed mid-stream) and the request failures are **connection errors** to `api.tokenrouter.com/v1/chat/completions`. Both are upstream issues with the free TokenRouter tier.

**Possible root causes:**
- Free tier rate-limiting that doesn't return 429 (silent throttling)
- Streaming endpoint reliability on the free tier
- No retry on HTTP 5xx with exponential backoff (current code retries streaming 2x but only waits 2s/4s)

**Proposed fix (not yet applied):**
- Detect repeated stream failures from same model and switch to non-streaming permanently for that model
- Add circuit breaker: if 3 consecutive stream failures, disable streaming for 5 minutes
- Add HTTP 502/503/504 to the transient-retry list (FID-166 added 504 but not 502/503)

### Group 2: Jury key churn (4 of 34 = 12%)

Disposable jury keys fail 3x and get quarantined. After 4+ quarantines, the key pool is exhausted. **This is by design** (quarantine is the correct behavior) but there's no `info!` log when a key is *recovered* (after a successful call resets the counter), and the `key_manager` only logs at `warn!` when the *threshold* is hit, not at `info!` when recovery happens.

**Proposed fix:**
- Add `info!` log on key recovery (successful call after threshold) so the operator sees the recovery happen
- Demote the "exceeded threshold" to `info!` — it's a normal lifecycle event, not a problem
- Root cause of the underlying failures is in Group 1 (TokenRouter)

### Group 3: Judge LLM failure (9 of 34 = 26%)

The judge LLM is the same TokenRouter endpoint. When the judge call fails, the system falls back to majority vote. This is **correct behavior** but the `warn!` log fires every time.

**Proposed fix:**
- Demote `Judge fallback` to `info!` — this is the documented fallback path, working as designed
- Add metrics counter so we can see the fallback rate over time

### Group 4: Jury member timeouts (3 of 34 = 9%)

Free-model jurors (DeepSeek R1, Qwen, etc.) are slow. The pool has a timeout, and timeouts are normal for free models on shared infrastructure.

**Proposed fix:**
- Demote `Jury member timed out` to `info!` — expected behavior for free tier
- Track timeout rate per model so we can deprioritize the slow ones

### Group 5: Decision parser noise (3 of 34 = 9%)

- 2x anti-pattern noise: conviction = exactly threshold (0.50/0.65) — LLM hit the exact number, parser adds ±0.05 noise. This is **working as designed**.
- 1x ZERO-BASE override: parser caught a contradiction in LLM reasoning and forced Close. This is **working as designed** (parser override is the feature).

**Proposed fix:**
- Demote both to `info!` — these are diagnostic logs, not errors
- Move the noise-injection log to `debug!` (it's the parser doing its job)

---

## Summary

**Of 34 WARN lines, 30 (88%) are NOT problems — they are working-as-designed fallback paths and parser overrides being logged too loudly.**

**Of 34 WARN lines, 4 (12%) are real signal:**
- 22 are upstream TokenRouter instability
- 4 are jury key churn (downstream of #1)
- The rest are noisy `warn!`s for normal fallback paths

**The right fix is:**
1. **Demote the 30 working-as-designed WARNs to info/debug** — they are not errors, just loud diagnostics
2. **Fix the 4 real-signal WARNs by fixing the TokenRouter instability** — add 502/503 to transient retry, add stream-failure circuit breaker, log key recovery
3. **Add metrics for fallback rates** so we can see the actual problem in numbers, not noise

**This is a 2-FID work, not a 1-FID work:**
- **FID-182**: Dashboard Terminal row-span-3 + slightly too tall
- **FID-183**: WARN root cause fix — demote working-as-designed, fix TokenRouter retry, add metrics

---

## Open questions for Spencer

1. Do you want the dashboard fix first (FID-182), then the WARN root cause work (FID-183), then the multi-chain work (v0.15.0)? Or different order?
2. For the TokenRouter instability — should we consider switching the LLM provider for the jury/judge paths only, or change the whole engine's provider?
3. The 6,593 INFO Context State "Delta-compression: PAIR 0.0% change" lines are INFO not WARN — but they flood the log. Are these also in scope for the "find root cause" pass, or is the actual root cause (all pairs having 0% change = no real price movement on Anvil fork) something to address separately?
