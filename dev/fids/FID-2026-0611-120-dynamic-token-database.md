# FID-2026-0611-120: Dynamic Token Database — Self-Updating Address Resolution

**Filename:** `FID-2026-0611-120-dynamic-token-database.md`
**ID:** FID-2026-0611-120
**Severity:** high
**Status:** implemented
**Created:** 2026-06-11 18:00
**Author:** Buffy

---

## Summary

The Arbitrum token database is a static `const` array of 238 entries in `src/execution/dex/mod.rs`. Token discovery runs once at startup via Blockscout, adding addresses to `TOKEN_EXTENSIONS`. After startup, no new addresses can be resolved — if a new pair passes discovery thresholds mid-session, it can't be traded because there's no address to resolve. The static array never grows; discovered tokens live only in memory and are lost on restart.

## Detailed Description

### Problem

The token address resolution chain is: `lookup_token()` → `TOKEN_EXTENSIONS` (runtime HashMap) → `ARBITRUM_TOKENS` (static const array). Both are immutable after startup.

**Failure 1: Discovery runs once, never again.** `discover_tokens()` is called at line 212 of `engine/mod.rs` during startup. It queries Blockscout for tokens with $1M+ volume and 500+ holders. Discovered addresses are added to `TOKEN_EXTENSIONS`. But if a new token hits these thresholds mid-session (e.g., a meme coin pumps), it's invisible until restart.

**Failure 2: Discovered tokens don't persist.** `TOKEN_EXTENSIONS` is a `Mutex<Option<HashMap>>` in static memory. On restart, all discovered addresses are lost and must be re-fetched from Blockscout. If Blockscout is down or slow at startup, the engine runs with only the static 238 entries.

**Failure 3: Static array is frozen.** The 238 entries in `ARBITRUM_TOKENS` were populated from CoinGecko months ago. New tokens that deploy on Arbitrum are never added. The array is `const` — it can only change via a code edit and redeployment.

**Failure 4: 14 pairs in live run show "no known address."** Tokens like SAND, SEI, SUI, VET, GIGA, GOAT, etc. are in the candle discovery list (Kraken has data for them) but have no Arbitrum address in either the static array or extensions. The 0x API handles them via symbol fallback, but GoPlus security checks are skipped and GeckoTerminal candle fetching fails.

### Root Cause

1. No persistent storage for discovered token addresses (only in-memory HashMap)
2. No periodic re-discovery during engine lifetime
3. No multi-source address resolution (only Blockscout at startup)
4. Static array is a code artifact, not a managed dataset

## Proposed Solution

### Design: Persistent Token Store + Periodic Discovery + Multi-Source Resolution

**Layer 1: Persistent JSON store** — `data/tokens.json` stores all known token addresses, loaded at startup and updated whenever new tokens are discovered. Survives restarts.

**Layer 2: Periodic discovery loop** — Every N cycles (configurable, default 30 = ~2.5 hours), re-run token discovery against multiple sources and merge results into the persistent store + `TOKEN_EXTENSIONS`.

**Layer 3: Multi-source address resolution** — For each active pair missing an address, try multiple sources in order:
1. Persistent token store (`data/tokens.json`)
2. Blockscout API (existing `discover_tokens()`)
3. CoinGecko API `/coins/{id}` → `platforms.arbitrum-one`
4. 0x API quote check — if `0x/quote` returns a valid route for the symbol, the token is tradeable even without a known address

**Layer 4: Validation gate** — Before adding a new address to the store, validate it via:
1. 0x `/price` endpoint — confirms liquidity exists
2. On-chain `decimals()` call — confirms it's a valid ERC-20
3. Optional GoPlus security check — flags honeypots/taxes

### Steps

1. **Create `data/tokens.json` persistent store** — JSON array of `{symbol, address, decimals, chain_id, source, discovered_at}` objects. Loaded at startup into `TOKEN_EXTENSIONS`. Updated atomically (write temp + rename).

2. **Add periodic discovery to engine loop** — Every `token_discovery_interval_cycles` (default 30), call a new `refresh_token_db()` function that:
   - Queries Blockscout for new tokens above volume/holder thresholds
   - For pairs in `active_pairs` with empty addresses, tries CoinGecko + 0x symbol resolution
   - Validates new addresses via 0x `/price`
   - Writes new entries to `data/tokens.json`
   - Calls `extend_token_db()` to update runtime HashMap
   - Logs: `info!("Token refresh: {} new addresses, {} total in store", new_count, total)`

3. **Add `TokenStoreConfig` to config** — New config section:
   ```toml
   [token_store]
   enabled = true
   discovery_interval_cycles = 30
   min_volume_usd = 1_000_000.0
   min_holders = 500
   validate_via_0x = true
   persist_path = "data/tokens.json"
   ```

4. **Migrate static `ARBITRUM_TOKENS` to `data/tokens.json`** — On first run, seed the persistent store from the static array. On subsequent runs, the store IS the source of truth — static array becomes fallback only.

5. **Address resolution for active pairs** — In the engine loop, before LLM evaluation, check if any active pair has an empty address. If so, queue it for async address resolution via CoinGecko/0x. Don't block the evaluation — resolve in the background and update for next cycle.

6. **Dashboard visibility** — Log token DB size and last refresh time in the health summary (every 10 cycles alongside pair health).

### Files to Modify

| File | Change |
|------|--------|
| `src/data/token_discovery.rs` | Add multi-source discovery (CoinGecko, 0x), validation, persistence |
| `src/execution/dex/mod.rs` | Add `load_token_store()`, `save_token_store()`, `TokenStoreConfig` |
| `src/engine/mod.rs` | Wire periodic discovery into engine loop |
| `config/default.toml` | Add `[token_store]` section |

### Verification

1. `cargo clippy -- -D warnings` — 0 warnings
2. `cargo test` — all tests pass
3. Runtime: After 2+ cycles, token DB should grow beyond 238 entries. New tokens visible in health summary.

## Perfection Loop

### Loop 1

- **RED:** Token DB is static (238 entries, never grows). Discovery runs once at startup, doesn't persist. 14 pairs in live run have "no known address" — can't be security-checked or GeckoTerminal-fetched.
- **GREEN:** (Pending — awaiting user approval)
- **AUDIT:** (Pending)
- **CHANGE DELTA:** (Pending)

## Resolution

- **Fixed By:** Pending
- **Fixed Date:** Pending
- **Fix Description:** Pending
- **Tests Added:** Pending
- **Verified By:** Pending

## Lessons Learned

- Static const arrays for token databases are a maintenance trap — they require code changes for every new token. Persistent JSON stores with runtime loading are the idiomatic approach.
- Token discovery should be a continuous process, not a one-time startup task. The crypto market moves fast — new tokens deploy hourly.
- The 0x API's symbol-based resolution is a powerful fallback that makes the system resilient to missing addresses, but it bypasses security checks. Always validate addresses before adding them.
