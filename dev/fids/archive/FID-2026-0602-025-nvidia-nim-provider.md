# FID: Add NVIDIA NIM as a free LLM provider

**Filename:** `FID-2026-0602-025-nvidia-nim-provider.md`
**ID:** FID-2026-0602-025
**Severity:** medium
**Status:** verified
**Created:** 2026-06-02 21:00
**Author:** Agent

---

## Summary

Add NVIDIA NIM (`integrate.api.nvidia.com`) as a new provider option alongside the existing OpenRouter and OpenGateway providers. NVIDIA offers free-tier access to DeepSeek-V4-Flash (284B MoE, 13B active params, 1M context, reasoning modes) via an OpenAI-compatible API. This gives the training pipeline a second free provider option for redundancy and cost elimination.

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust (cargo)
- **Tool Versions:** ratatui 0.30, reqwest, serde_json
- **Commit/State:** On top of `2242f6b` (prior session)

## Detailed Description

### Problem

Currently only OpenRouter (owl-alpha) is configured as a free provider. If OpenRouter is down or rate-limited, there is no fallback. Adding NVIDIA NIM provides a second free provider with a different model (DeepSeek-V4-Flash) for redundancy.

### Expected Behavior

- `provider = "nvidia"` in `config/default.toml` activates NVIDIA NIM
- Endpoint: `https://integrate.api.nvidia.com/v1`
- Model: `deepseek-ai/deepseek-v4-flash`
- API key via `NVIDIA_API_KEY` env var
- Max tokens: 16,384 (NVIDIA's limit for this model)
- Streaming supported (SSE, same OpenAI format)
- `reasoning_effort` param not wired yet (set to `none` for non-reasoning use)

### Root Cause

Not a bug — new feature request.

### Evidence

NVIDIA NIM API docs: https://docs.api.nvidia.com/nim/reference/deepseek-ai-deepseek-v4-flash-infer
- POST `https://integrate.api.nvidia.com/v1/chat/completions`
- Bearer token auth
- OpenAI-compatible request/response format
- Free tier available

## Impact Assessment

### Affected Components

- `config/default.toml` — add `[ai.nvidia]` section
- `src/core/config.rs` — add `NvidiaConfig` struct + field in `AiConfig`
- `src/agent/provider.rs` — add `"nvidia"` case in `create_provider()`
- `.env` — add `NVIDIA_API_KEY`

### Risk Level

- [ ] Critical: System crash, data loss, or security vulnerability
- [ ] High: Major feature broken, no workaround
- [x] Medium: Feature degraded, workaround exists
- [ ] Low: Minor issue, cosmetic, or edge case

Low risk: additive change, no existing behavior modified. Falls through to default if provider string doesn't match.

## Proposed Solution

### Approach

Follow the exact same pattern as the existing OpenRouter provider:
1. Add `NvidiaConfig` struct with defaults in `config.rs`
2. Add `[ai.nvidia]` section in `default.toml`
3. Add `"nvidia"` match arm in `create_provider()` in `provider.rs`
4. Add `NVIDIA_API_KEY` to `.env` (gitignored)

### Steps

1. Add `NvidiaConfig` struct to `src/core/config.rs` with fields: `endpoint`, `model`, `api_key_env`
2. Add `nvidia: NvidiaConfig` field to `AiConfig` with `#[serde(default)]`
3. Add `Default` impl for `NvidiaConfig` with NVIDIA defaults
4. Add `[ai.nvidia]` section to `config/default.toml`
5. Add `"nvidia"` match arm in `create_provider()` at `src/agent/provider.rs:66`
6. Add `NVIDIA_API_KEY` to `.env`
7. Verify: `cargo check`, `cargo clippy -- -D warnings`, `cargo test`

### Verification

- `cargo check` — zero errors
- `cargo clippy -- -D warnings` — zero warnings
- `cargo test` — all 187 tests pass
- `cargo fmt --check` — clean
- Grep for `nvidia` in config to confirm wiring

## Perfection Loop

### Loop 1

- **RED:** No existing NVIDIA provider. Needed: NvidiaConfig struct, config section, provider match arm, env var.
- **GREEN:** Added NvidiaConfig with defaults (endpoint, model, api_key_env), wired into AiConfig, added "nvidia" to validation and create_provider(), capped max_tokens to 16384 for NVIDIA's limit.
- **AUDIT:** `cargo check` = clean, `cargo clippy -- -D warnings` = 0 warnings, `cargo test` = 187/187, `cargo fmt --check` = clean. Call-graph grep confirms full wiring.
- **CHANGE DELTA:** ~3% (3 files modified, 1 new struct + match arm)

## Resolution

- **Fixed By:** Agent
- **Fixed Date:** 2026-06-02 21:15
- **Fix Description:** Added NVIDIA NIM as a provider option. NvidiaConfig struct in config.rs with endpoint/model/api_key_env defaults. "nvidia" case in create_provider() at provider.rs. max_tokens capped to 16384 (NVIDIA's limit for deepseek-v4-flash). NVIDIA_API_KEY added to .env (gitignored).
- **Tests Added:** No (existing provider tests cover the pattern)
- **Verified By:** cargo check + clippy + test + fmt, call-graph reachability grep
- **Commit/PR:** [pending]
- **Archived:** [pending]

## Lessons Learned

[To be filled after resolution]
