# FID: OpenRouter Management Key System

**Filename:** `FID-2026-0604-048-openrouter-key-management.md`
**ID:** FID-2026-0604-048
**Severity:** medium
**Status:** verified
**Created:** 2026-06-04 20:30
**Author:** Flux (opencode)

---

## Summary

Add programmatic OpenRouter API key management so the engine can auto-create per-model API keys, enforce per-key spending limits, and rotate keys without manual dashboard intervention. OpenRouter's Management API supports create/read/update/delete/disable operations via `https://openrouter.ai/api/v1/keys`.

---

## Environment

- **OS:** Windows 11
- **API:** OpenRouter Management API (`OPENROUTER_MANAGEMENT_KEY` already in `.env`)
- **Config:** `[ai.openrouter.management]` section already stubbed in `config/default.toml`
- **Commit/State:** 2f88469

---

## Detailed Description

### Problem

Currently the engine uses a single OpenRouter API key for ALL model calls — live trading, sandbox testing, and training. This means:
1. No per-model spending visibility
2. No automated key rotation for security
3. Cannot set per-key spending limits (important for free vs paid models)
4. Management key exists in config but is unused

### Expected Behavior

- Engine creates dedicated API keys per model or per session
- Each key has a spending limit (`limit` field in OpenRouter)
- Keys can be disabled/deleted when limits are reached
- Sandbox tests auto-create temporary keys that self-destruct
- Usage tracking per key visible in OpenRouter dashboard

### Current State

```toml
# config/default.toml
[ai.openrouter.management]
management_key_env = "OPENROUTER_MANAGEMENT_KEY"
endpoint = "https://openrouter.ai/api/v1/keys"
```

The config is stubbed but no code references it. The `.env` file contains `OPENROUTER_MANAGEMENT_KEY`.

---

## Impact Assessment

### Affected Components

- `src/agent/provider.rs` — `LlmProvider` would use a management-backed key pool
- `src/agent/key_manager.rs` — NEW: Management API client
- `src/core/config.rs` — already has `ManagementConfig`, no changes needed
- `src/engine.rs` — sandbox/test/training optionally use managed keys

### Risk Level

- [ ] Critical
- [ ] High
- [x] Medium: Feature additive, no existing behavior changes
- [ ] Low

### Risk Mitigation

- Feature is gated behind `--managed-keys` flag initially
- Falls back to existing single-key behavior if management key absent
- Key limits prevent runaway spending
- Auto-disable prevents bill shock

---

## Proposed Solution

### Phase 1 — Management API Client (`key_manager.rs`)

**API endpoint:** `https://openrouter.ai/api/v1/keys`
**Auth header:** `Authorization: Bearer {MANAGEMENT_KEY}`

**OpenRouter API response fields** (from docs):
```json
{
  "data": [{
    "created_at": "2025-02-19T20:52:27.363244+00:00",
    "updated_at": "2025-02-19T21:24:11.708154+00:00",
    "hash": "<KEY_HASH>",
    "label": "sk-or-v1-abc...123",
    "name": "Customer Key",
    "disabled": false,
    "limit": 10,              // USD credit limit — 0 = unlimited
    "limit_remaining": 10,    // USD remaining
    "limit_reset": null,      // "daily" | "weekly" | "monthly" | null
    "include_byok_in_limit": false,
    "usage": 0,
    "usage_daily": 0,
    "usage_weekly": 0,
    "usage_monthly": 0
  }]
}
```

When creating a key, the response includes the `key` string itself (raw API key).

```rust
pub struct ApiKey {
    pub hash: String,
    pub label: String,
    pub name: String,
    pub disabled: bool,
    pub limit: u32,
    pub limit_remaining: u32,
    pub limit_reset: Option<String>,
    pub usage: u64,
}

pub struct KeyManager {
    management_key: String,  // from env OPENROUTER_MANAGEMENT_KEY
    client: reqwest::Client,
}

impl KeyManager {
    pub fn from_env() -> Result<Self> — reads OPENROUTER_MANAGEMENT_KEY
    async fn list_keys(&self, offset: Option<u32>) -> Result<Vec<ApiKey>>;
    async fn create_key(&self, name: &str, limit: Option<u32>) -> Result<(ApiKey, String)>;  // returns (info, raw_key)
    async fn get_key(&self, hash: &str) -> Result<ApiKey>;
    async fn update_key(&self, hash: &str, disabled: Option<bool>, limit: Option<u32>, name: Option<&str>) -> Result<ApiKey>;
    async fn delete_key(&self, hash: &str) -> Result<()>;
}
```

**Error handling:**
- `401` → management key is invalid/expired — log warning, fall back to single key
- `429` → rate limited — exponential backoff (1s, 2s, 4s)
- `5xx` → OpenRouter down — retry 3x, then fall back to single key
- Graceful degradation: if management API fails, engine uses existing key pool

**Key cleanup (avoid accumulation):**
- List keys on startup, delete any keys older than 24h with `name` starting with `savant-`
- Auto-delete test session keys when `--test` completes
- Training keys auto-delete on convergence

### Phase 2 — Integration Points

1. **Sandbox testing:** Auto-create a temporary key per test session with a $1 limit, disable when done
2. **Training:** Create a dedicated key for training runs, rotate on convergence
3. **Live trading:** Optionally use managed keys for better cost tracking
4. **Model comparison:** Create one key per model, compare cost/performance

**Interaction with SANDBOX_API_KEYS:**
- `SANDBOX_API_KEYS` takes priority for existing key rotation
- When `--managed-keys` is set, the `KeyManager` creates new keys instead of reading from env
- The management key (`OPENROUTER_MANAGEMENT_KEY`) is ONLY used for key CRUD — never for LLM calls
- Fallback: if management API fails, use `OPENROUTER_API_KEY` directly

### Phase 3 — Safety
- Default limit: $5 per key
- Auto-disable after 3 consecutive 429/403 errors
- Key rotation on error detection
- `--managed-keys` CLI flag to opt in

### Verification

```bash
cargo build && cargo test && cargo clippy -- -D warnings
# Create a test key
cargo run --release -- --test --model openrouter/owl-alpha --managed-keys -n 5
# Verify key was created and disabled in OpenRouter dashboard
```

---

## Perfection Loop

### Loop 1

- **RED:** FID missing API response format, error handling strategy, key cleanup, SANDBOX_API_KEYS interaction, limit units unclear
- **GREEN:** Added full API response schema with field descriptions, error handling fallback chain (401/429/5xx), auto-cleanup of 24h-old savant-* keys, SANDBOX_API_KEYS priority documentation, limit confirmed as USD
- **AUDIT:** `cargo build` (clean, FID only)
- **CHANGE DELTA:** ~5%

### Loop 2

- **RED:** No remaining issues. API contract fully specified. Error handling complete. Cleanup strategy defined.
- **GREEN:** N/A
- **AUDIT:** `cargo build` (clean)
- **CONVERGED:** Delta < 2%

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —
