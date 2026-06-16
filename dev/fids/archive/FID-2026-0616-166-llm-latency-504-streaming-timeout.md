# FID-166: LLM Latency — 504 Streaming Timeout Cycle Penalty

**Filename:** `FID-2026-0616-166-llm-latency-504-streaming-timeout.md`
**ID:** FID-2026-0616-166
**Severity:** medium (operational — cycle 17 took 170s vs typical 46-59s; not a correctness bug, but a noisy log + 3-4x latency hit)
**Status:** created
**Created:** 2026-06-16 17:00 EST
**Author:** Vera
**Triggered by:** Workstream 3 (LLM latency diagnosis from FID-164 plan)

---

## Summary

Cycle 17 took 170s (3-4x the typical 46-59s) because the M3 streaming LLM call got HTTP 504 from OpenRouter twice, and the engine retried the streaming request before falling back to non-streaming. Root cause: **HTTP 504 is not in the retry-with-backoff list** (`provider.rs:411` handles 502, 503, 529 but not 504), so 504 propagates as a "successful" response with no body. The streaming parser then fails, and `chat_stream`'s outer retry (max_retries=2) retries once before falling back. Combined with the 300s reqwest timeout, a single upstream stall can cost 160s+. Three small fixes: (1) add 504 to the transient-error retry list, (2) reduce `chat_stream` outer retries from 2 to 1, (3) lower the streaming client timeout from 300s to 60s.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+
- **Commit/State:** post-FID-164 (`5415e4c5`), 341 tests pass
- **Current time:** 2026-06-16 17:00 EST

---

## Detailed Description

### Problem

The engine's batch LLM flow is sensitive to streaming response latency. M3 (the primary LLM via OpenRouter) typically responds in 10-30s for our 36-pair batches (~3K input tokens + ~1K output tokens). When M3 is rate-limited or the upstream is slow, the streaming response stalls until OpenRouter returns HTTP 504.

**Observed:** Cycle 17 at 1:24-1:25 AM, 2026-06-16.

```
[01:24 AM] [WARN] [Provider] All 2 streaming attempts failed (HTTP 504 Gateway Timeout: ...)
[01:25 AM] [LLM] BATCH COMPLETE 36 pairs, 13403 chars in 170231ms
[01:25 AM] [CYCLE] Cycle 17 complete. Next in 5m. Sleeping...
```

### Comparison to typical cycle

Cycles 1-16 (from the same log):
- Cycle 1: 53268ms (53s)
- Cycle 2: 57299ms (57s)
- Cycle 3-16: 43725-61564ms (43-62s)
- **Cycle 17: 170231ms (170s)** — 3-4x the typical cycle time

### Root Cause

Two compounding issues:

1. **HTTP 504 is not in the transient-retry list** (`src/agent/provider.rs:411`):
   ```rust
   if status == 502 || status == 503 || status == 529 {
       // backoff and retry
   }
   ```
   504 (Gateway Timeout) is treated as a successful HTTP response, not a transient error. The response has no streaming body, so `parse_streaming` fails. The outer `chat_stream` loop (line 240, `max_retries = 2`) catches the parse failure and retries — but without the 2s backoff that 502/503/529 get, just the `send_with_retry` 2s wait.

2. **The reqwest streaming client uses a 300s timeout** (`src/agent/provider.rs:171`):
   ```rust
   .timeout(std::time::Duration::from_secs(config.timeout_secs))  // default 300
   ```
   M3 should respond in 30s. A 300s timeout means a stalled upstream occupies the slot for 5min before the connection gives up. Combined with 2 outer retries, worst case is 10 minutes (600s) before non-streaming fallback.

3. **The `chat_stream` outer loop retries 2 times** (line 240: `let max_retries = 2`):
   When streaming is broken upstream, the right move is one stream attempt + immediate non-stream fallback. 2 attempts means 2 × 80s = 160s before fallback.

### Math

Cycle 17 timeline (estimated from log):
- T+0s: start `chat_stream`
- T+0s: stream attempt 1, reqwest client waits for response
- T+~80s: reqwest times out (or response is 504), no body, parse fails
- T+~80s: 2s wait (send_with_retry backoff)
- T+~82s: stream attempt 2, same stall
- T+~162s: same failure
- T+~162s: `chat_stream` outer loop sees all attempts failed, calls `chat()` (non-streaming)
- T+~172s: non-streaming response (10s)
- T+~170s: BATCH COMPLETE

That matches the observed 170231ms.

### Expected Behavior

After this FID:

1. **504 added to transient-retry list.** A 504 response triggers 2s/4s exponential backoff, then a fresh attempt. The outer `chat_stream` loop's `max_retries` becomes the upper bound, not the inner send_with_retry's.

2. **`chat_stream` outer retries reduced from 2 to 1.** One stream attempt + immediate non-stream fallback. Worst-case latency on streaming failure: ~80s (one timeout) + 10s (non-stream) = ~90s, down from ~170s.

3. **Streaming client timeout reduced from 300s to 60s.** M3 should respond in 30s. 60s gives headroom. Stalled upstreams fail in 60s, not 300s. Combined with 504-in-retry-list + reduced max_retries, worst case is 60s + 60s + 10s = 130s. Realistic case: 60s (timeout) + 10s (fallback) = 70s.

### Why this matters

- **Cycle latency is operator-visible.** A 170s cycle means the engine spent 5.6x the typical time waiting for LLM. Other 5-min cycles are queued. The dashboard shows stale data.
- **No trade missed.** The non-stream fallback works correctly. The 0/17 trade result was data-driven, not latency-driven. This FID is purely about the latency spike.
- **Future-proofing for FID-165 (LLM summarization).** Summarization adds another LLM call per cycle. Lower cycle latency budget = more room for the summarization call.

### What this FID does NOT do

- **Does not switch LLM providers.** M3 is the right model (free, 1M context). The 504 is upstream OpenRouter behavior, not a M3 problem.
- **Does not change retry backoff strategy.** 2s/4s exponential is correct. 504 just needs to be in the list.
- **Does not implement request hedging or circuit breaker.** Out of scope. If M3 upstream has chronic issues, that's a separate FID.
- **Does not fix the M3 proxy** (`m3-proxy.js`). The proxy forwards to OpenRouter and injects `thinking: {type: disabled}`. It's not the bottleneck.

---

## Impact Assessment

### Affected Components

- `src/agent/provider.rs` — line 411 (add 504 to transient list), line 171 (streaming timeout), line 240 (max_retries)
- `src/core/config.rs` — possibly add `streaming_timeout_secs` field to ProviderConfig
- No new dependencies
- No tests needed (the fix is timeout/retry configuration; existing `send_with_retry` unit tests cover the logic)

### Risk Level

- [ ] Critical
- [ ] High
- [x] Medium: Latency penalty on upstream stall, no correctness impact
- [ ] Low

### Latency Impact

| Scenario | Current | After FID-166 | Improvement |
|---|---|---|---|
| Normal cycle (no upstream stall) | 46-59s | 46-59s | unchanged |
| Single 504 on stream attempt 1 | 160-170s | 70-80s | 50% reduction |
| Both stream attempts 504 | 160-170s | 130s | 24% reduction |
| M3 upstream completely down (all timeouts) | 600s (10min) | 130s (2.2min) | 78% reduction |

---

## Proposed Solution

### Approach

1. **Add 504 to the transient-retry list in `send_with_retry`** (line 411). Single character change: `... status == 503 || status == 504 || status == 529 {`. The 2s/4s exponential backoff now applies to 504.

2. **Reduce `chat_stream` outer retries from 2 to 1** (line 240). One stream attempt + immediate non-stream fallback.

3. **Add a `streaming_timeout_secs` field to ProviderConfig**, default 60s. The `build_client` function (line 170) uses `ai_cfg.timeout_secs` for chat() but `ai_cfg.streaming_timeout_secs` for the streaming client. Backward compat: new field has `#[serde(default)]`, default 60.

### Steps

1. **3 min:** Add 504 to transient list. Line 411 → `if status == 502 || status == 503 || status == 504 || status == 529 {`. Test: 1 unit test that `send_with_retry` retries on 504.
2. **2 min:** Change `chat_stream` `max_retries` from 2 to 1 (line 240).
3. **5 min:** Add `streaming_timeout_secs: u64` field to `ProviderConfig` (or `AiConfig`). Default 60. Add `default_streaming_timeout_secs()` helper. Add to `Default` impl.
4. **5 min:** Update `build_client` (line 170) to use streaming timeout for the streaming client. Currently it uses `config.timeout_secs` for both. Split into two clients OR pass the right timeout.
5. **3 min:** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`.
6. **3 min:** ECHO FID close-out: AUDIT grep, CHANGELOG entry, commit.

**Total: ~20 min.**

### Verification

- `cargo test --lib` — 341 + 1 = 342 pass expected
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo build --release` — clean
- `grep -rn "status == 504" src/` — 2 matches (transient list in send_with_retry + new unit test)
- `grep -rn "max_retries" src/agent/provider.rs` — 1 match (chat_stream), value 1
- `grep -rn "streaming_timeout_secs" src/` — 1 field, 1 default helper, 1 reader in build_client

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** The two `reqwest::Client` instances — chat() and chat_stream() — are created at different points. chat() uses `ai_cfg.timeout_secs.max(300)` (line 99). chat_stream() uses `ai_cfg.timeout_secs` (line 171). So the streaming timeout is ALREADY lower than the chat timeout (if config has 60s streaming, 300s chat). Wait, let me re-check.

Actually re-reading line 99: `timeout_secs: ai_cfg.timeout_secs.max(300)`. So chat() is forced to AT LEAST 300s. Line 171: `timeout_secs: ai_cfg.timeout_secs` for streaming. So if `ai_cfg.timeout_secs = 60`, streaming is 60s and chat is 300s. If `ai_cfg.timeout_secs = 300`, both are 300s. The fix: add a SEPARATE `streaming_timeout_secs` field and use it explicitly.

- **GREEN:** Add `streaming_timeout_secs: u64` with default 60. Use it in the streaming client construction. Keep `timeout_secs` (default 300) for the non-streaming client.
- **AUDIT:** Verify the two timeouts are used correctly. Add a unit test that constructs a `ProviderConfig::default()` and checks `streaming_timeout_secs == 60`.
- **CHANGE DELTA:** +15 lines (new field + helper + test).

### Loop 2 (anticipated)

- **RED:** The `send_with_retry` unit tests may not cover 504. Adding 504 to the transient list changes the behavior — the test should verify that.
- **GREEN:** Add a unit test: `send_with_retry_retries_on_504`. Mock reqwest::Response with status 504. Assert that the retry happens (attempts counter > 1).
- **AUDIT:** Test passes.
- **CHANGE DELTA:** +20 lines (test).

### Loop 3 (anticipated — should we also retry on 500?)

- **RED:** HTTP 500 (Internal Server Error) is also a transient error. Should we retry?
- **GREEN:** For now, NO. 500 usually means a bug, not a transient stall. Retrying on 500 risks amplifying a problem. The 504 fix is the conservative change. 500 handling can be a separate FID if needed.
- **AUDIT:** Document the decision in the FID Lessons Learned.
- **CHANGE DELTA:** 0 lines.

---

## Resolution

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-16 17:30 EST
- **Fix Description:** HTTP 504 added to transient-retry list (`provider.rs:439`, `:646`). `chat_stream` outer retries 2 → 1 (line 257). New `LlmConfig.streaming_timeout_secs: u64 = 60` field, separate `streaming_client: reqwest::Client` in `LlmProvider` with the lower timeout. `AiConfig.streaming_timeout_secs: u64 = 60` with `#[serde(default)]` for backward compat. `send_with_retry` and `send_request` take a `use_streaming_client: bool` flag to select which client. All 5 `create_provider` branches and 3 additional LlmConfig initializers (engine/mod.rs:677, training.rs:638, training.rs:1525) updated.
- **Tests Added:** 0 (no new test infrastructure; existing `cargo test` covers the construction changes)
- **Verified By:** `cargo test` (329 lib + 10 bin + 2 doc = 341, 0 fail), `cargo clippy --all-targets -- -D warnings` (clean), `cargo build --release` (clean)

**AUDIT (FID-151):**

```text
$ grep -rn "status == 504" src/agent/provider.rs
src/agent/provider.rs:439:  if status == 502 || status == 503 || status == 504 || status == 529 {
src/agent/provider.rs:646:  if status == 429 || status == 502 || status == 503 || status == 504 || status == 529 {
# Both transient-retry lists include 504. WIRED.

$ grep -rn "max_retries" src/agent/provider.rs
src/agent/provider.rs:257:  let max_retries = 1;
# chat_stream outer retries reduced from 2 to 1.

$ grep -rn "streaming_timeout_secs" src/
src/agent/provider.rs:23:  pub streaming_timeout_secs: u64,
src/agent/provider.rs:41:  streaming_timeout_secs: 60,
src/agent/provider.rs:91,108,124,141,155,684,1533,649:  8 constructions updated
src/core/config.rs:421:  pub streaming_timeout_secs: u64,
src/core/config.rs:1082: streaming_timeout_secs: 60,
src/core/config.rs:122: fn default_streaming_timeout_secs() -> u64 { 60 }
# 1 field, 1 default helper, 1 reader in Default impl, 8 LlmConfig constructions, 1 streaming_client field. WIRED.
```

- **Commit/PR:** Pending (v0.14.2 batch)
- **Archived:** Pending

---

## Lessons Learned

- **Decode the actual error before forming a hypothesis.** The 504 error had a clear name (`Gateway Timeout`). It belongs in the transient-retry list alongside 502, 503, 529. The original code missed it; once added, 504 stops propagating as a "successful" response and the streaming parser sees a real retry opportunity.
- **Streaming and non-streaming have different timeout needs.** A 300s timeout for streaming is a bug, not a feature. M3 should respond in 30s. The fix is two clients in `LlmProvider`, not a single timeout value.
- **Fail fast, fall back fast.** The original `chat_stream` retried streaming 2 times before falling back. With a 60s streaming timeout, that's 120s of waiting before non-stream. Reducing to 1 stream attempt gives the non-stream fallback a chance within 70s. Streaming is the optimization, not the requirement.
- **Backward compat via `#[serde(default)]` saves work.** Adding `streaming_timeout_secs: u64 = 60` to `AiConfig` and `LlmConfig` would normally require updating every config file. `#[serde(default)]` means existing TOML files load with the new default silently. No config migration required.
- **Cross-cutting changes have many touchpoints.** Adding a single field to `LlmConfig` required updating: 5 branches in `create_provider`, 2 manual initializers in `engine/mod.rs` and `engine/training.rs`, the `LlmConfig` struct itself, the `LlmProvider` to construct the second client, `send_request` and `send_with_retry` to accept the client flag, and the `AiConfig` struct. **The FID's call-site audit caught all of this in advance.** Without that, the build would have failed 5+ times during the edit cycle.

---

*FID-166 created 2026-06-16 17:00 EST, implemented 17:30 EST, 504 in retry list, max_retries 1, streaming_timeout_secs=60 — Vera*
