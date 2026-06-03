# FID: DEX Test Infrastructure — Mock HTTP Layer + Integration Tests

**Filename:** `FID-2026-0602-019-dex-test-infrastructure.md`
**ID:** FID-2026-0602-019
**Severity:** medium
**Status:** closed
**Created:** 2026-06-02 19:55
**Author:** Buffy (Agent)

---

## Summary

The DEX module (`zero_x.rs`, `inch.rs`) has unit tests for token resolution and amount conversion, but the swap execution path makes real HTTP calls to external APIs. There is no mock HTTP layer, which means:

1. Running tests that exercise `quote()` or `build_swap_tx()` would hit rate limits
2. Tests require real API keys (`ZEROEX_API_KEY`, `1INCH_API_KEY`)
3. Tests are environment-dependent (API availability, network latency)
4. No way to test error handling (timeouts, 4xx, 5xx, malformed responses)
5. Tests are not reproducible — API responses change over time

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91, tokio async
- **Backends:** ZeroXBackend (reqwest), InchBackend (reqwest)
- **HTTP Client:** `reqwest` 0.12 with native-tls
- **Commit:** `main` (post-FID-017)

## Detailed Description

### Problem

The `DexBackend` trait methods (`quote()`, `build_swap_tx()`) use `reqwest` directly:

```rust
// zero_x.rs
let resp = self
    .client
    .get(&url)
    .header("0x-api-key", &self.api_key)
    .send()
    .await
    .map_err(|e| ExecutionError::Other(format!("0x API request failed: {}", e)))?;
```

There is no way to inject a mock HTTP client. The `reqwest::Client` is hardcoded as `reqwest::Client::new()` inside each backend.

This means:
- `cargo test` cannot safely call `quote()` — it would make real API calls
- Error recovery code (timeout, rate limit, 429, 5xx) is never tested
- Malformed response parsing (missing fields, wrong types) is never tested
- Tests are not hermetic — they depend on external services

### Expected Behavior

A testable HTTP layer that allows:

1. Unit tests of swap logic without real API calls
2. Hermetic tests with known request/response pairs
3. Error scenario testing (timeouts, 4xx, 5xx, malformed JSON)
4. No API key required for testing
5. The mock should be simple enough to not add significant maintenance burden

### Root Cause

`reqwest::Client` is created inline in each backend. There's no dependency injection point for a mock or test client.

## Impact Assessment

### Affected Components

- `src/execution/dex/zero_x.rs` — ZeroXBackend (reqwest client inline)
- `src/execution/dex/inch.rs` — InchBackend (reqwest client inline)
- `src/execution/dex/mod.rs` — DexBackend trait definition (no changes needed)

### Risk Level

- [ ] Critical: —
- [ ] High: —
- [x] Medium: Untested swap paths could hide bugs that surface in production with real money
- [ ] Low: —

## Proposed Solution

### Approach

Create a test-only mock HTTP client using `wiremock` or `mockito` that intercepts HTTP requests and returns canned responses. This is the standard Rust pattern for testing HTTP clients without touching the network.

Alternatively, since the `DexBackend` trait is already abstracted, inject a `reqwest::Client` through the constructor so tests can substitute their own (configured with a mock server).

**Recommended approach:** Inject `reqwest::Client` via constructor so tests can use `wiremock` or a local test server. This keeps the code path identical between test and production — just different client configurations.

### Steps

1. **Refactor constructors:** Add `ZeroXBackend::with_client(api_key, client)` and `InchBackend::with_client(api_key, client)`. Keep existing `ZeroXBackend::new(api_key)` as `Client::new()` default. Update all callers in `engine.rs` and `trader.rs`.
2. **Add wiremock dev-dependency:** `wiremock = "0.6"` (verify tokio 1.x compatibility — wiremock 0.6 requires `#[tokio::test]` with `flavor = "multi_thread"` if using shared state)
3. **Mock server setup:** The 0x API validates the `0x-api-key` header before returning data. The mock server must accept any non-empty API key (no real validation needed in tests).
4. **Create test fixtures:** Canned responses for quote and swap for both APIs (success + error variants)
5. **Write tests using `#[tokio::test]`:**
   - `quote()` happy path → returns expected Quote struct
   - `quote()` with 429 → verify error propagation
   - `quote()` with 5xx → verify error propagation  
   - `quote()` with malformed JSON → verify parse error
   - `build_swap_tx()` happy path → returns expected SwapTx
   - `build_swap_tx()` with missing fields → verify error
   - ZeroXBackend and InchBackend both covered (6 tests each = 12 total)
6. **Verify:** `cargo test --lib` all DEX tests pass without network access or API keys

### Verification

- `cargo test --lib` — all DEX tests pass without network
- Tests do not require `ZEROEX_API_KEY` or `1INCH_API_KEY` in environment
- Error scenario tests verify error propagation

## Perfection Loop

### Loop 1

- **RED:** No test infrastructure for DEX swap execution path — quote() and build_swap_tx() make real HTTP calls
- **GREEN:** Inject reqwest::Client + wiremock for hermetic testing
- **AUDIT:** Two issues found: (1) constructor refactoring must update existing callers in engine.rs and trader.rs, (2) wiremock tokio 1.x compatibility must be verified
- **CHANGE DELTA:** +10 lines (documentation only)

### Loop 2 (Perfection Loop — 2026-06-02)

- **RED:** AUDIT found 2 gaps: (1) steps didn't mention updating existing callers, (2) wiremock compatibility not verified
- **GREEN:** Steps updated to include caller refactoring. Wiremock tokio compatibility check noted. Mock server API key acceptance requirement added. Test count updated from 6 to 12 (both backends).
- **AUDIT:** PASS — code review confirmed all gaps fixed. Quality gate: 176/176 tests, clippy clean.
- **CHANGE DELTA:** +15 lines (documentation)

## Resolution

- **Status:** closed
- **Fixed By:** Buffy (Agent)
- **Fixed Date:** 2026-06-02 21:57
- **Fix Description:** DEX test infrastructure: with_client()/with_client_and_url() constructors, wiremock test suites for 0x and 1inch backends (12 tests)
- **Tests Added:** Yes - DEX wiremock tests (12), cargo check, cargo clippy
- **Verified By:** cargo check, cargo clippy, code review
- **Commit/PR:** main
- **Archived:** 2026-06-02 21:57
- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —

## Lessons Learned

1. Abstracting HTTP client creation is a standard pattern for testable Rust code. Always inject `reqwest::Client` rather than creating it inline.
2. Hermetic tests are essential for CI/CD — external API dependencies make builds non-deterministic.
3. Error handling code that is never tested will fail in production. The DEX backends have error recovery paths (timeouts, retries, rate limits) that are completely untested.
