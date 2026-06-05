# FID: Model Testing Suite Cannot Complete Runs — Retry, Rate Limit, and Concurrency Failures

**Filename:** `FID-2026-0605-053-sandbox-robustness.md`
**ID:** FID-2026-0605-053
**Severity:** critical
**Status:** created
**Created:** 2026-06-05 17:29
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

The sandbox model testing suite failed to complete runs for 4 of 5 models. Only 1 of 5 free OpenRouter models (Nemotron Nano 30B) completed the 60-scenario evaluation. The failures are infrastructure bugs, not model quality issues: `chat()` has zero retry logic (unlike `chat_stream()` which has 2 retries + non-streaming fallback), there's no rate limit backoff in the sandbox runner, concurrency is hardcoded at 10 regardless of model limits, and there's no scenario-level retry for transient failures.

---

## Detailed Description

### Problem

Test run results for 5 models:

| Model | Result | Root Cause |
|-------|--------|------------|
| nvidia/nemotron-3-nano-30b-a3b:free | OK (0.68 avg, 47/60) | Only model that worked |
| nvidia/nemotron-3-super-120b-a12b:free | CRASHED at 11/60 | JSON decode error on chunked response killed process |
| google/gemma-4-31b-it:free | 0/60 all errors | BYOK key routing — OpenRouter rejects key for batch |
| google/gemma-4-26b-a4b-it:free | 0/60 all errors | Same BYOK issue |
| moonshotai/kimi-k2.6:free | 0/60 all 429 | Rate limited — 10 concurrent requests too aggressive |

### Expected Behavior

All 5 models should complete the 60-scenario sandbox with partial failure tolerance. A rate-limited scenario should retry with backoff, not fail permanently. A JSON decode error should not crash the process.

### Root Cause

**5 independent failures:**

**1. `chat()` has zero retry logic** (`provider.rs:144-156`)
```rust
pub async fn chat(&self, system: &str, messages: &[Message]) -> Result<String, LlmError> {
    let body = self.build_body(system, messages, false);
    let resp = self.send_request(&body).await?;  // single attempt, no retry
    let status = resp.status();
    if status == 429 {
        return Err(LlmError::RateLimited(60));  // immediate failure
    }
    ...
}
```
Compare with `chat_stream()` (line 158-238) which has 2 retries with exponential backoff AND falls back to non-streaming. The sandbox calls `chat()` directly at `engine.rs:4180`.

**2. Sandbox runner treats rate limits as permanent failures** (`engine.rs:4197-4211`)
```rust
let status = match &sr.response {
    Ok(_) => "OK".to_string(),
    Err(e) => {
        warn!("Scenario {} ERR: {}", sr.scenario_name, e);
        format!("ERR: {}", e)
    }
};
```
A 429 gets logged as ERR and the scenario is scored 0.00. No retry, no backoff, no delay.

**3. Concurrency hardcoded at 10** (`engine.rs:4142`)
```rust
let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));
```
No per-model concurrency control. Kimi K2.6 can handle ~1 concurrent request. Nemotron Nano handles 10 fine.

**4. No scenario-level retry for transient failures**
If an LLM call fails with a transient error (429, 500, timeout, JSON decode), the scenario is permanently failed. No mechanism to retry the specific scenario after a delay.

**5. Process crash on JSON decode errors**
The Nemotron Super 120B crashed the process with exit code 1. The `parse_non_streaming` method returns `LlmError::InvalidResponse` which should be caught, but the process still crashed — likely a panic in the bytes stream handling or an unhandled error path.

### Evidence

```
=== Nemotron Super 120B ===
[12/60] Liquidation Cascade — ERR: Invalid response: JSON parse error: error decoding response body
[13/60] Thin Order Book — ERR: Invalid response: JSON parse error: error decoding response body
...crash at 23/60 with exit code 1

=== Kimi K2.6 ===
[1/60] Slow Grind — ERR: Rate limited, retry after 60s
[2/60] News-Driven Spike — ERR: Rate limited, retry after 60s
...all 60 returned 429

=== Gemma 4 31B ===
[1/60] Parabolic Exhaustion — ERR: HTTP 400: API_KEY_INVALID (is_byok: true)
...all 60 returned the same BYOK error
```

---

## Impact Assessment

### Affected Components

- `src/agent/provider.rs` — `chat()` method lacks retry logic
- `src/engine.rs` — `run_sandbox()` hardcoded concurrency, no retry/backoff

### Risk Level

- [x] Critical: Cannot evaluate new models. Model comparison is blocked. The testing suite is the foundation for model selection decisions.

---

## Proposed Solution

### Approach

Make `chat()` have the same retry resilience as `chat_stream()`, add adaptive concurrency to the sandbox, and add scenario-level retry with exponential backoff for rate-limited requests.

### Steps

1. **Add retry loop to `chat()` in provider.rs** — 3 attempts with exponential backoff (2s, 4s, 8s). On rate limit (429), wait for the `Retry-After` header value (default 60s). On JSON decode error, retry immediately. On 5xx, retry with backoff.

2. **Add `send_request_with_retry()` to provider.rs** — Shared retry logic used by both `chat()` and `chat_stream()`. Consolidate the retry pattern.

3. **Make sandbox concurrency configurable** — Read `SANDBOX_CONCURRENCY` env var (default 10). Add `--concurrency` CLI flag.

4. **Add rate limit detection to sandbox runner** — When a scenario returns `RateLimited`, pause ALL in-flight requests for the retry-after duration, then re-enqueue the failed scenarios.

5. **Add scenario retry queue** — After the initial run, collect all rate-limited and transient-error scenarios and retry them with concurrency=1 and exponential backoff.

6. **Catch panics in sandbox scenario tasks** — Wrap each `join_set.spawn` in `catch_unwind` to prevent one scenario's panic from killing the process.

### Verification

```bash
cargo build
cargo test
cargo clippy -- -D warnings
# Run all 5 models — all should complete (even if some score 0 on legitimate failures)
```

---

## Perfection Loop

### Loop 1

- **RED:** —
- **GREEN:** —
- **AUDIT:** —
- **CHANGE DELTA:** —

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —

---

## Lessons Learned

1. **`chat()` and `chat_stream()` should have identical retry semantics.** The divergence was introduced when streaming was added — `chat_stream()` got retries but `chat()` was left as a single-shot call.

2. **Concurrency must be model-aware.** Free tier models on OpenRouter have wildly different rate limits. A hardcoded semaphore of 10 works for Nemotron but kills Kimi.

3. **Rate limits should cause global backoff, not per-request failure.** When a 429 is received, ALL concurrent requests to that provider should pause, not just the one that got the 429.

4. **Process crashes from JSON decode errors indicate a missing error boundary.** The `parse_non_streaming` method should never panic — all errors should be caught and returned as `LlmError`.

5. **Test infrastructure must be more robust than the code it tests.** If the testing suite can't complete a run, we can't evaluate models, which blocks all model selection decisions.
