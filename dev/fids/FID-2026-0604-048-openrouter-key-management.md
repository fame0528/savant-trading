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

The engine has a complete `OpenRouterManagementClient` (`src/agent/openrouter_management.rs`, 295 lines) with full CRUD operations, but it's **never called** from any production code path. The management key is loaded in `engine.rs:501` but unused. Sandbox tests and training runs use raw `OPENROUTER_API_KEY` without key isolation.

### What Already Exists

- `src/agent/openrouter_management.rs` — Full client: `list_keys`, `create_key`, `get_key`, `update_key`, `delete_key`
- `src/core/config.rs` — `OpenRouterManagementConfig` with `management_key_env` field
- `config/default.toml` — `[ai.openrouter.management]` stubbed
- `.env` — `OPENROUTER_MANAGEMENT_KEY` present

### What's Missing

- No code path calls `OpenRouterManagementClient::create_key()` or `delete_key()`
- Sandbox tests use raw API key — no per-session isolation
- Training runs use raw API key — no spending limits per run
- No auto-cleanup of old keys
- No `--managed-keys` CLI flag

---

## Impact Assessment

### Affected Components

- `src/agent/openrouter_management.rs` — **Already exists** (295 lines). Full CRUD client. No changes needed.
- `src/core/config.rs` — **Already exists** (`OpenRouterManagementConfig`). No changes needed.
- `src/engine.rs` — Wire management client into sandbox/test/training code paths
- `src/main.rs` — Add `--managed-keys` CLI flag to `parse_test_args`
- `src/agent/provider.rs` — Accept dynamically-created API key from `KeyManager`

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

### Phase 1 — Wire existing client into sandbox/training

**Existing code:** `src/agent/openrouter_management.rs` — `OpenRouterManagementClient` with `create_key`, `delete_key`, `list_keys`

**Integration point:** `src/engine.rs` — `run_sandbox()` and `run_training_batch()` create providers at ~line 3404-3420. Currently reads `config.ai.model.clone()` and `api_keys` from env. Needs to optionally create a managed key via `OpenRouterManagementClient` and pass it to the provider.

**Changes needed in `run_sandbox()` / `run_training_batch()`:**
1. Read `OPENROUTER_MANAGEMENT_KEY` from env
2. If present AND `--managed-keys` flag set:
   a. Create `OpenRouterManagementClient`
   b. Call `create_key(CreateKeyRequest { name: "savant-session-{timestamp}", limit: Some(5.0) })`
   c. Use the returned `key` string as the provider API key
   d. Store key hash for cleanup on completion
3. On completion (success or error): `delete_key(hash)` to clean up
4. If management API fails: fall back to existing `OPENROUTER_API_KEY`

**Changes needed in `src/main.rs`:**
- Add `managed_keys: bool` to `TestArgs`
- Parse `--managed-keys` in `parse_test_args`
- Pass to `run_sandbox`, `run_training`, `run_action_test`

### Phase 2 — Cost tracking

- After each test/training run, call `get_key(hash)` to read `usage`, `usage_daily`
- Print cost summary: "Key {name}: ${usage:.4} spent, ${limit_remaining:.4} remaining"
- Compare cost across models when testing

### Phase 3 — Auto-disable on errors

- Track consecutive 429/403 errors per key
- After 3 consecutive errors: `update_key(hash, UpdateKeyRequest { disabled: Some(true) })`
- Fall back to next key in pool or raw API key

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

- **RED:** FID claimed `key_manager.rs` was NEW — actually `openrouter_management.rs` already exists (295 lines, full CRUD). FID overwrote existing code with pseudocode. Config claim unverified. Phase 3 safety vague. `--managed-keys` CLI flag unwired. Key rotation mechanism unspecified.
- **GREEN:** Verified `openrouter_management.rs` exists. Verified `OpenRouterManagementConfig` exists. Rewrote solution to wire existing client instead of building new. Added concrete integration points (run_sandbox, run_training_batch). Added error handling fallback chain. Added auto-disable on 429/403.
- **AUDIT:** `cargo build` (clean). Existing tests pass.
- **CHANGE DELTA:** ~8% (major rewrite of FID content)

### Loop 2

- **RED:** No remaining issues. Client exists. Config exists. Integration points identified. Error handling specified. Cleanup strategy defined.
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
