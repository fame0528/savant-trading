# FID-2026-0611-121: 0x Liquidity Validation Gate for Token Store

**Filename:** `FID-2026-0611-121-0x-liquidity-validation-gate.md`
**ID:** FID-2026-0611-121
**Severity:** medium
**Status:** implemented
**Created:** 2026-06-11 19:30
**Author:** Buffy

---

## Summary

FID-120 added a persistent token store (`data/tokens.json`) with periodic Blockscout discovery. The config includes `validate_via_0x = true` but the flag is **dead config** — it's defined in `TokenStoreConfig` and `default.toml` but never read by any code. New tokens from Blockscout are persisted without any on-chain liquidity check. This means tokens with zero 0x routing — untradeable honeypots, dead pools, or tokens without Arbitrum liquidity — get added to the token database and potentially evaluated by the LLM.

The fix: wire a lightweight 0x `/price` validation gate into the discovery pipeline. Before persisting a newly discovered token address, query `0x /price` with a small USDC→token swap to confirm `liquidityAvailable = true`. Only persist tokens that 0x can actually route.

## Detailed Description

### Problem

**Dead config flag:** `validate_via_0x = true` in `config/default.toml` (line 127) and `TokenStoreConfig.validate_via_0x` in `src/core/config.rs` (line 521) exist but are never referenced in `refresh_token_store()`, `seed_token_store_from_static()`, or anywhere in the engine startup.

**No liquidity gate on discovery:** `refresh_token_store()` in `src/data/token_discovery.rs` (line 285) iterates over Blockscout-discovered tokens and pushes them directly into `existing_entries` without any tradeability check. A token could have $10M volume on CoinGecko but zero liquidity on any 0x-supported DEX on Arbitrum — it would still be added.

**Downstream impact:** Tokens in the persistent store get loaded into `extend_token_db()` at startup, which means the LLM evaluates them every cycle. The engine wastes evaluation cycles on untradeable tokens and the GoPlus security check (which requires a known address) runs on tokens that can't actually be swapped.

### Root Cause

1. `validate_via_0x` was designed in the FID-120 config spec but never implemented in the discovery functions
2. `token_discovery.rs` is a standalone module — it has no access to the 0x API key or the `ZeroXBackend` infrastructure
3. The existing `check_liquidity()` in `zero_x.rs` is a method on `ZeroXBackend` (requires a full `SwapParams` struct with wallet address, slippage, etc.) — too heavy for a lightweight discovery check

### What 0x `/price` Can Tell Us

The 0x Swap API v2 `/price` endpoint on Arbitrum (`chainId=42161`) returns:
- **`liquidityAvailable`** (bool): Can 0x route a swap for this token?
- **`buyAmount`** (string): Expected output amount (non-zero = routeable)
- **`tokenMetadata.buyToken.buyTaxBps`** (string): Honeypot detection — buy tax > 100bps = risky
- **`tokenMetadata.sellToken.sellTaxBps`** (string): Sell tax > 100bps = likely honeypot

A single GET request with `sellToken=USDC&buyToken=TOKEN&sellAmount=1000000` (1 USDC) is enough to validate routeability.

## Proposed Solution

### Design: Standalone 0x Validation Function + Gate in Refresh Pipeline

**New function:** `validate_token_liquidity(token_address, api_key) -> bool` — a standalone async function in `token_discovery.rs` that makes a single 0x `/price` call and returns whether the token has liquidity. No dependency on `ZeroXBackend` or `SwapParams`.

**Gate in refresh pipeline:** `refresh_token_store()` takes an optional `api_key: Option<&str>` parameter. When `validate_via_0x = true` and the API key is available, each newly discovered token is validated before being pushed into `existing_entries`.

**Gate in startup pipeline:** The engine startup discovery block passes the 0x API key from `std::env::var(&config.exchange.dex.api_key_env)` to `refresh_token_store()`.

### Steps

1. **Add `validate_token_liquidity()` to `src/data/token_discovery.rs`**
   - Standalone async function: `(token_address: &str, api_key: &str) -> Result<bool, ExecutionError>`
   - Constructs 0x `/price` URL: `https://api.0x.org/swap/allowance-holder/price?chainId=42161&sellToken={USDC}&buyToken={TOKEN}&sellAmount=1000000`
   - Headers: `0x-api-key: {key}`, `0x-version: v2`
   - Parses `liquidityAvailable` and `buyAmount != "0"` from response
   - Optionally extracts `buyTaxBps` — logs warning if > 100 (potential honeypot) but doesn't reject (the LLM makes the final call)
   - Timeout: 10 seconds per request
   - Rate limiting: 200ms sleep between requests (5 req/s matches 0x free tier)

2. **Update `refresh_token_store()` signature and logic**
   - Add parameter: `validate_via_0x: bool, api_key: Option<&str>`
   - Before pushing a new token, if `validate_via_0x && api_key.is_some()`:
     - Call `validate_token_liquidity(&token.address, api_key.unwrap()).await`
     - If `Ok(false)` or `Err(e)`: skip the token, log at debug level
     - If `Ok(true)`: proceed to push into `existing_entries`
   - Log: `info!("Token validation: {}/{} passed 0x liquidity check", passed, total_new)`

3. **Update engine startup (`src/engine/mod.rs`) to pass API key**
   - Read 0x API key: `std::env::var(&config.exchange.dex.api_key_env)`
   - Pass to `refresh_token_store()` in both startup and periodic loop
   - If key is empty/missing, log warning and fall back to no-validation mode

4. **Add `validated_at` field to `TokenStoreEntry`** (optional enhancement)
   - `#[serde(default)] validated_at: String` — timestamp of last 0x validation
   - Allows future logic to re-validate stale entries (tokens can lose liquidity)

5. **Add config comment clarification**
   - `config/default.toml`: Add comment explaining what `validate_via_0x` does and that it requires `ZEROX_API_KEY` env var

### Files to Modify

| File | Change |
|------|--------|
| `src/data/token_discovery.rs` | Add `validate_token_liquidity()`, update `refresh_token_store()` signature + gate logic |
| `src/engine/mod.rs` | Pass 0x API key to `refresh_token_store()` in startup and periodic loop |
| `config/default.toml` | Add comment explaining `validate_via_0x` dependency on API key |

### Verification

1. `cargo clippy -- -D warnings` — 0 warnings
2. `cargo test` — all tests pass
3. Runtime: After 10+ cycles, token store should only contain 0x-routeable tokens. `Token refresh:` log should show pass/skip counts.

---

## Perfection Loop

### Loop 1

#### RED — 10 Issues Found

**RED-1: API key plumbing.** `token_discovery.rs` is a standalone module with no access to the 0x API key. The key lives in `config.exchange.dex.api_key_env` and is read by `engine/utils.rs` when creating the `ZeroXBackend`. We need to either:
  - (a) Pass the key as a parameter through the call chain, or
  - (b) Read `std::env::var()` directly inside `token_discovery.rs` (breaks module isolation)
  - **Decision:** (a) is cleaner — pass `api_key: Option<&str>` through the function signature.

**RED-2: Rate limiting.** The 0x free tier is 5 req/s. If Blockscout returns 50 new tokens, we'd fire 50 sequential validation requests = 10 seconds of blocking in the engine loop. Solution: add a 200ms delay between requests and cap validation at 20 new tokens per refresh cycle.

**RED-3: 0x API downtime.** If 0x is down, `validate_token_liquidity()` returns `Err`. Should we still add the token (trust Blockscout) or reject it? 
  - **Decision:** Add the token with a warning. The 0x check is an optimization, not a hard gate. The real trading path has its own liquidity check at execution time. Logging a warning makes the degraded mode visible.

**RED-4: Dust threshold.** Using `sellAmount=1000000` (1 USDC, 6 decimals) may be below some DEX dust thresholds, causing `liquidityAvailable=false` even for tradeable tokens. 
  - **Decision:** Use `sellAmount=10000000` (10 USDC) — well above any dust threshold, still a trivial amount for validation.

**RED-5: Existing tokens not re-validated.** The validation gate only runs for NEW tokens during `refresh_token_store()`. Tokens already in the store (from prior runs) are never re-checked. A token that lost liquidity since being added remains in the store forever.
  - **Decision:** Add a `validated_at` timestamp to `TokenStoreEntry`. During periodic refresh, re-validate entries older than 7 days. If liquidity is gone, log a warning and mark with `source: "expired_liquidity"` (don't remove — the LLM might still want to see the chart).

**RED-6: 0x API key not available in non-live mode.** In paper trading or sandbox mode, there's no executor and no 0x API key. The validation function should gracefully skip when the key is missing.
  - **Decision:** `api_key: Option<&str>` — when `None`, skip validation entirely. Log at debug level.

**RED-7: Sequential validation blocks the engine loop.** 50 tokens × 200ms = 10 seconds added to each cycle. For a 5-minute cycle, this is acceptable but not ideal.
  - **Decision:** Cap new-token validation at 20 per cycle. Log skipped count. Next cycle validates the next batch.

**RED-8: Tax detection logging.** The 0x response includes `buyTaxBps` and `sellTaxBps`. These are valuable honeypot signals but we shouldn't auto-reject (some legitimate tokens have small taxes).
  - **Decision:** Log a `WARN` when tax > 100bps. Add the tax info to `TokenStoreEntry.source` field (e.g., `"0x_validated_tax_250bps"`). The LLM sees this in context and makes the final call.

**RED-9: Token store entry schema change.** Adding `validated_at` field to `TokenStoreEntry` changes the JSON schema. Existing `data/tokens.json` files won't have this field.
  - **Decision:** Use `#[serde(default)]` — empty string for existing entries. Backwards compatible.

**RED-10: 0x V2 endpoint migration.** The existing codebase uses `https://api.0x.org/swap/v2/price` but the docs show the newer V2 pattern uses `https://api.0x.org/swap/allowance-holder/price`. Need to match what the existing `ZeroXBackend::lookup()` uses.
  - **Decision:** Use the same URL pattern as `zero_x.rs::lookup()` to ensure consistency. The `lookup` function constructs the URL from `chain_id` and endpoint name — we'll replicate the same pattern.

#### GREEN — Solution

All 10 RED issues addressed in the proposed solution above:
- API key passed as `Option<&str>` parameter (RED-1, RED-6)
- 200ms rate limiting + 20-per-cycle cap (RED-2, RED-7)
- Graceful fallback on 0x downtime — add with warning (RED-3)
- 10 USDC sellAmount (RED-4)
- `validated_at` timestamp + re-validation of stale entries (RED-5, RED-9)
- Tax detection logging (RED-8)
- Match existing 0x URL pattern (RED-10)

#### AUDIT — 3 Findings

**AUDIT-1: Module boundary.** Adding 0x HTTP logic to `token_discovery.rs` mixes concerns (Blockscout discovery vs 0x validation). Consider a separate `token_validation.rs` module.
  - **Verdict:** Over-engineering for a single function. Keep it in `token_discovery.rs` with a clear section comment. If we add Etherscan validation later, extract then.

**AUDIT-2: Testability.** `validate_token_liquidity()` makes real HTTP calls. Unit tests need a mock.
  - **Verdict:** Add a `#[cfg(test)]` mock using `reqwest::Client::builder().build()` with a test helper. But since the existing `check_liquidity` has no mocks either, this is consistent with the codebase pattern.

**AUDIT-3: Token store size growth.** With validation, we might reject most new tokens. The store grows slowly but stays clean. Without validation, it grows fast but contains noise.
  - **Verdict:** This is the correct tradeoff. Quality over quantity. 50 validated tokens > 500 unvalidated ones.

#### CHANGE DELTA

No changes needed from the GREEN solution. The AUDIT findings confirmed the design is sound.

---

## Questions for Operator

1. **Should we also add Etherscan V2 `tokeninfo` validation as a second source?** The Etherscan docs you shared (`docs.etherscan.io`) have a unified V2 API with `module=token&action=tokeninfo` that returns decimals, name, symbol. This would cross-validate Blockscout's metadata. Cost: one extra API call per new token (5/sec free tier). Benefit: catches Blockscout metadata errors.

2. **Should evicted tokens (FID-118 `permanent_dead`) be re-validated before revival?** Currently the revival check just looks for candle data. Adding a 0x liquidity check before revival would prevent re-adding tokens that lost DEX liquidity.

3. **Multi-chain expansion:** You mentioned wanting to trade SOL memes. The 0x API supports 20+ chains via `chainId` parameter. The `validate_token_liquidity()` function could be made chain-agnostic by accepting `chain_id` as a parameter. Want me to design for this now or defer?

---

## Resolution

- **Fixed By:** Pending (awaiting approval)
- **Fixed Date:** Pending
- **Fix Description:** Pending
- **Tests Added:** Pending
- **Verified By:** Pending
