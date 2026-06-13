# FID-143: Jury Shadow Mode Activation + 9-Free/1-M3 Model Split

**Status:** verified
**Severity:** high (jury configured as enabled but never initializes — zero data collected)
**Date:** 2026-06-12
**Version:** v0.14.0-target

---

## Diagnosis

### Why the jury never runs

`src/engine/mod.rs` line 739:
```rust
if config.ai.provider == "openrouter" && config.ai.jury.enabled {
```

`config/default.toml` has `provider = "tokenrouter"` (MiniMax M3). Condition fails → `jury_key_manager` stays `None` → `jury_pool` stays `None` → the jury evaluation block (line ~2493) is skipped every cycle.

**Evidence:**
- Zero "FID-114 JURY SHADOW" lines in live terminal log (v16.2.7, 2 full cycles)
- `dev/logs/jury-metrics.json` doesn't exist
- `.env` has both `OPENROUTER_MANAGEMENT_KEY` and `OPENROUTER_API_KEY` set — keys are available

### Why the condition is wrong

The jury system creates its own ephemeral API keys via OpenRouter's Management API (the only provider with a "create key" endpoint). TokenRouter's management API can list/enable/disable keys but cannot create them. The jury is provider-independent — it just needs the management API to create keys, and always calls OpenRouter's endpoint with those keys.

---

## Architecture (post-fix)

| Component | Endpoint | Keys | Model |
|---|---|---|---|
| **Primary LLM** | TokenRouter | TokenRouter key | MiniMax M3 |
| **Jury member 1-9** | OpenRouter | OpenRouter mgmt-created key | `openrouter/free` (9 diverse models) |
| **Jury member 10** | OpenRouter | OpenRouter mgmt-created key | `minimax/m3` (via OpenRouter) |
| **Jury Judge** | TokenRouter (primary) | TokenRouter key | Owl Alpha / M3 |

### 9-Free/1-M3 model split rationale

- 9 free models → diverse architectures (Gemma, Llama, Qwen, Mistral, etc.) → uncorrelated with M3 → strong disagreement signal
- 1 M3 → control group. If the M3 jury member disagrees with the primary M3 (same model, different instance), that's a valuable signal about randomness/variance
- Cost: $0 for 9 members, minimal OpenRouter credits for 1 M3 member

---

## Changes (3 files, ~30 lines)

### Fix 1: Remove provider guard (`engine/mod.rs` line 739)

**Before:**
```rust
if config.ai.provider == "openrouter" && config.ai.jury.enabled {
```

**After:**
```rust
if config.ai.jury.enabled {
```

### Fix 2: Jury pool uses OpenRouter config, not primary provider (`engine/mod.rs` lines 760-775)

The `jury_provider_config` must use OpenRouter's endpoint and API key (not TokenRouter's), since the jury keys are OpenRouter keys. Create a dedicated OpenRouter config for the jury pool:

```rust
let jury_provider_config = LlmConfig {
    endpoint: "https://openrouter.ai/api/v1".to_string(),
    model: config.ai.jury.model.clone(),  // "openrouter/free"
    api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
    max_tokens: config.ai.max_tokens,
    temperature: config.ai.temperature,
    top_p: config.ai.top_p,
    timeout_secs: config.ai.jury.timeout_secs,
    extra_headers: vec![
        ("HTTP-Referer".to_string(), config.ai.openrouter.referer.clone()),
        ("X-OpenRouter-Title".to_string(), config.ai.openrouter.title.clone()),
    ],
    disable_thinking: config.ai.disable_thinking,
};
```

### Fix 3: Enhance shadow log with jury-vs-batch comparison (`engine/mod.rs` ~line 2500)

Add the batch decision action to the shadow log so the user can see agreement/disagreement:

```
FID-114 JURY SHADOW: 8/10 verdicts, 73% consensus, BUY [batch: PASS] — dissent: BUY(6@72%), SELL(2@45%)
```

### Fix 4: Startup status log

If jury enabled: `FID-114: Jury initialized — 10 members, OpenRouter, 9 free + 1 M3`
If jury disabled: `FID-114: Jury disabled — {reason}`

---

## Verification

- [ ] `cargo check` passes
- [ ] `cargo test` passes (308 tests)
- [ ] Startup log shows jury initialization
- [ ] Per-cycle JURY SHADOW log appears with verdict count, consensus, action, batch comparison
- [ ] `dev/logs/jury-metrics.json` created on shutdown with aggregate metrics

---

## Files Touched

| File | Lines changed | What |
|---|---|---|
| `src/engine/mod.rs` | ~25 | Remove provider guard, force OpenRouter endpoint for jury pool, add startup status log, enhance shadow log |
| `config/default.toml` | ~3 | Update jury model comment to reflect 9-free/1-M3 split |
| `dev/fids/FID-2026-0612-143-jury-shadow-activation.md` | — | This FID document |
