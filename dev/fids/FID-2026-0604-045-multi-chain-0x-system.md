# FID: Multi-Chain 0x Swap System

**Filename:** `FID-2026-0604-045-multi-chain-0x-system.md`
**ID:** FID-2026-0604-045
**Severity:** high
**Status:** analyzed
**Created:** 2026-06-04 18:00
**Author:** Flux (opencode / mimo-v2.5-pro)

---

## Summary

Expand the DEX execution engine from single-chain (Arbitrum only, 198 tokens) to multi-chain via 0x's unified API. The 0x Swap API supports 20 EVM chains with the same API key, same Permit2 contract, and same code — only the `chainId` parameter changes. This multiplies our token universe by ~20x and gives access to the deepest liquidity pools across all major L2s and L1s.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+, tokio async
- **Tool Versions:** 0x API v2 (Permit2), reqwest, alloy_core, k256
- **Commit/State:** v0.8.0-rc2 (29071e4)
- **0x API Key:** Configured, active on all supported chains
- **Wallet:** `0x543ca0434b84ad38c858d2d178d2082521711fbc`

---

## Detailed Description

### Problem

Currently the engine only trades on Arbitrum (chain_id 42161) with 198 hardcoded tokens. The 0x API supports 20 EVM chains with millions of tokens. We're leaving 95% of the opportunity space on the table.

### Current Architecture

```
ARBITRUM_TOKENS: &[(&str, &str, u8)] = &[
    ("USDC", "0xaf88d065...", 6),
    ("APE", "0x7f9fbf9b...", 18),
    // 198 entries, all Arbitrum
];
```

- `SwapParams.chain_id` is hardcoded to `42161` in config
- `resolve_pair()` only looks up Arbitrum addresses
- `sync_balance()` only checks Arbitrum USDC + ETH
- `ensure_permit2_approval()` only approves on Arbitrum
- Candle sources (Kraken, OKX, etc.) are chain-agnostic (they return prices, not chain-specific data)

### Target Architecture

```
CHAIN_TOKENS: &[ChainToken] = &[
    // Arbitrum (42161)
    ChainToken { chain_id: 42161, symbol: "USDC", address: "0xaf88d065...", decimals: 6 },
    ChainToken { chain_id: 42161, symbol: "APE", address: "0x7f9fbf9b...", decimals: 18 },
    // 500+ Arbitrum tokens

    // Base (8453)
    ChainToken { chain_id: 8453, symbol: "USDC", address: "0x833589fcd6e...", decimals: 6 },
    ChainToken { chain_id: 8453, symbol: "WETH", address: "0x4200000000000000000000000000000000000006", decimals: 18 },
    // 300+ Base tokens

    // Optimism (10), BSC (56), Polygon (137), etc.
];
```

### 0x API Chain Support (from docs/llms-full.md)

| Chain | ChainId | Swap | Gasless | Notes |
|-------|---------|------|---------|-------|
| Ethereum | 1 | ✅ | ✅ | Deepest liquidity, highest gas |
| Arbitrum | 42161 | ✅ | ✅ | **Current chain** |
| Base | 8453 | ✅ | ✅ | Coinbase L2, massive growth |
| Optimism | 10 | ✅ | ✅ | Superchain, DeFi blue chips |
| BSC | 56 | ✅ | ✅ | Meme coin capital |
| Polygon | 137 | ✅ | ✅ | Huge token count |
| Avalanche | 43114 | ✅ | ✅ | DeFi + meme tokens |
| Berachain | 80094 | ✅ | ✅ | New chain, growing fast |
| Monad | 143 | ✅ | ✅ | High-performance L1 |
| Sonic | 146 | ✅ | ✅ | Fast finality |
| Scroll | 534352 | ✅ | ✅ | zkEVM L2 |
| Linea | 59144 | ✅ | ❌ | Consensys zkEVM |
| Mantle | 5000 | ✅ | ✅ | BitDAO L2 |
| Abstract | 2741 | ✅ | ✅ | Consumer chain |
| Ink | 57073 | ✅ | ✅ | Kraken L2 |
| HyperEVM | 999 | ✅ | ✅ | HyperLiquid |
| Plasma | 9745 | ✅ | ✅ | New chain |
| Tempo | 4217 | ✅ | ✅ | New chain |
| Unichain | 130 | ✅ | ✅ | Uniswap L2 |
| World Chain | 480 | ✅ | ✅ | Worldcoin L2 |

**Cross-Chain API also supports:** Solana, HyperCore, Tron

### Key Constants (same on ALL EVM chains)

| Constant | Value | Notes |
|----------|-------|-------|
| Permit2 Contract | `0x000000000022d473030f116ddee9f6b43ac78ba3` | Same address on all EVM chains |
| 0x Exchange Proxy | `0xfeea2a79d7d3d36753c8917af744d71f13c9b02a` | Same address on all EVM chains |
| 0x API Endpoint | `https://api.0x.org/swap/permit2` | Unified, chain selected by `chainId` param |
| Headers | `0x-api-key`, `0x-version: v2` | Same for all chains |

---

## Impact Assessment

### Affected Components

- `src/execution/dex/mod.rs` — Token database, `resolve_pair()`, `amount_to_wei()`
- `src/execution/dex/trader.rs` — `DexTrader`, `sync_balance()`, `ensure_permit2_approval()`, `place_order()`, `close_position()`
- `src/execution/dex/zero_x.rs` — `api_url()`, `lookup()` — already chain-agnostic via `chainId` param
- `src/engine.rs` — Main loop, token iteration, pre-filters
- `src/data/sources/` — Candle sources (already chain-agnostic for price data)
- `config/default.toml` — New `[chains]` section with RPC URLs per chain

### Risk Level

- [ ] Critical: System crash, data loss, or security vulnerability
- [x] High: Major feature broken, no workaround
- [ ] Medium: Feature degraded, workaround exists
- [ ] Low: Minor issue, cosmetic, or edge case

---

## Proposed Solution

### Approach: Phased Rollout

#### Phase 1 — Expand Arbitrum Token Database (zero-risk)

**Goal:** 198 → 500+ Arbitrum tokens

- Pull top tokens from CoinGecko `/coins/list?include_platform=true&platform_ids=arbitrum-one`
- Pull from DeFiLlama `/protocols` filtered by Arbitrum TVL
- Filter: $1M+ daily volume, verified contract, not stablecoin/xStock
- Replace `ARBITRUM_TOKENS` static array with expanded list
- Zero infrastructure changes — same chain, same RPC, same gas

**Verification:** `cargo test`, engine scan shows 100+ pairs evaluated per cycle

#### Phase 2 — Chain-Aware Token Database

**Goal:** Support multiple chains in the token database

1. Create `ChainToken` struct: `{ chain_id: u64, symbol: String, address: String, decimals: u8 }`
2. Replace `ARBITRUM_TOKENS: &[(&str, &str, u8)]` with `CHAIN_TOKENS: &[ChainToken]`
3. Update `resolve_pair()` to accept `chain_id` parameter
4. Update `lookup_token()` to filter by chain
5. Add chain-specific USDC addresses:

| Chain | USDC Address | Decimals |
|-------|-------------|----------|
| Arbitrum | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` | 6 |
| Base | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | 6 |
| Ethereum | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | 6 |
| Optimism | `0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85` | 6 |
| Polygon | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` | 6 |
| BSC | `0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d` | 18 |
| Avalanche | `0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E` | 6 |

6. Add chain config to `config/default.toml`:

```toml
[chains]
enabled = [42161, 8453, 10, 56]

[chains.arbitrum]
chain_id = 42161
rpc_url = "https://arb1.arbitrum.io/rpc"
native_token = "ETH"
min_gas = 0.002

[chains.base]
chain_id = 8453
rpc_url = "https://mainnet.base.org"
native_token = "ETH"
min_gas = 0.001

[chains.optimism]
chain_id = 10
rpc_url = "https://mainnet.optimism.io"
native_token = "ETH"
min_gas = 0.001

[chains.bsc]
chain_id = 56
rpc_url = "https://bsc-dataseed.binance.org"
native_token = "BNB"
min_gas = 0.005
```

**Verification:** `cargo test`, engine evaluates tokens on multiple chains

#### Phase 3 — Multi-Chain Execution

**Goal:** Execute swaps on any supported chain

1. `DexTrader` becomes chain-aware — holds RPC URL + chain_id per chain
2. `sync_balance()` checks USDC + native gas token on each enabled chain
3. `ensure_permit2_approval()` works on any chain (same Permit2 address)
4. `sign_and_send()` uses chain-specific RPC + gas prices
5. Engine iterates: for each chain, for each token on that chain
6. Best execution: pick the chain with deepest liquidity for each pair

**Verification:** Execute a swap on Base, verify receipt

#### Phase 4 — Gasless API

**Goal:** No native gas token needed

- 0x Gasless API pays gas upfront (deducted from swap output)
- Available on 15+ chains (see table above)
- Simplifies multi-chain: no need to hold ETH/BNB/MATIC on each chain
- Just need USDC balance on each chain

**Verification:** Execute a gasless swap on Base

#### Phase 5 — Cross-Chain API

**Goal:** Swap across chains in one transaction

- 0x Cross-Chain API handles bridging automatically
- Swap Arbitrum USDC → Base WETH in one tx
- Supports 20+ chains + Solana
- Uses bridge providers: Across, Relay, Bungee, CCIP, Squid, etc.

**Verification:** Cross-chain swap Arbitrum → Base

---

## Implementation Steps

### Phase 1 Steps (Expand Arbitrum)

1. [ ] Write CoinGecko token scraper script (or use existing `cg_arbitrum_tokens.json` approach)
2. [ ] Filter: $1M+ volume, verified address, not stablecoin/xStock
3. [ ] Generate expanded `ARBITRUM_TOKENS` array (500+ entries)
4. [ ] Update `mod.rs` with expanded token list
5. [ ] Run `cargo test` — all existing tests must pass
6. [ ] Run engine — verify 100+ pairs evaluated per cycle

### Phase 2 Steps (Chain-Aware Database)

1. [ ] Define `ChainToken` struct in `mod.rs`
2. [ ] Create `CHAIN_TOKENS` array with Arbitrum + Base + Optimism + BSC tokens
3. [ ] Create `ChainConfig` struct with RPC URL, native token, min gas
4. [ ] Add `[chains]` section to `config/default.toml`
5. [ ] Update `resolve_pair()` to accept and filter by `chain_id`
6. [ ] Update `lookup_token()` to accept `chain_id`
7. [ ] Add chain-specific USDC address mapping
8. [ ] Update `DexTrader::new()` to accept chain config
9. [ ] Update `sync_balance()` to check multiple chains
10. [ ] Update `ensure_permit2_approval()` to use chain-specific RPC
11. [ ] Run `cargo test` + `cargo clippy`

### Phase 3 Steps (Multi-Chain Execution)

1. [ ] Refactor `DexTrader` to hold per-chain RPC clients
2. [ ] Update `place_order()` to pass chain_id through to execution
3. [ ] Update `close_position()` to use correct chain for position
4. [ ] Engine main loop: iterate chains → iterate tokens per chain
5. [ ] Balance sync across all enabled chains
6. [ ] Gas monitoring per chain (native token balance)
7. [ ] Execute test swap on Base chain
8. [ ] Verify receipt on Base

### Phase 4 Steps (Gasless)

1. [ ] Add Gasless API endpoint to `zero_x.rs` (`/gasless/quote`)
2. [ ] Gasless response format differs — no `transaction.to`, uses `trade` object
3. [ ] Sign and submit gasless trade
4. [ ] Test on Base (gasless enabled)

### Phase 5 Steps (Cross-Chain)

1. [ ] Add Cross-Chain API endpoint (`/swap/quote` with `bridge` param)
2. [ ] Handle cross-chain response format (bridge tx + destination tx)
3. [ ] Monitor cross-chain status via `/status` endpoint
4. [ ] Handle recovery if bridge fails
5. [ ] Test: Arbitrum USDC → Base WETH

---

## Verification

### Phase 1
- `cargo test` — 201+ tests pass
- Engine scan shows 100+ pairs evaluated per cycle
- No new clippy warnings

### Phase 2
- `cargo test` — all tests pass with chain-aware database
- `resolve_pair("APE/USDC", Long)` returns correct Arbitrum address
- `resolve_pair("APE/USDC", Long, chain_id=8453)` returns correct Base address (or error if not on Base)

### Phase 3
- Execute swap on Base chain — receipt confirmed
- Balance sync shows correct USDC on multiple chains
- Gas monitoring shows ETH balance on each L2

### Phase 4
- Gasless swap executes without ETH for gas
- Gas cost deducted from swap output

### Phase 5
- Cross-chain swap Arbitrum → Base completes
- `/status` endpoint shows bridge completion

---

## Perfection Loop

### Loop 1 (Phase 1)

- **RED:** 198 tokens is too few, engine evaluates only 35-40 per cycle after pre-filters
- **GREEN:** Expand to 500+ Arbitrum tokens via CoinGecko API
- **AUDIT:** Pending
- **CHANGE DELTA:** ~5% (token array expansion only)

### Loop 2 (Phase 2)

- **RED:** Token database is flat, no chain awareness
- **GREEN:** Add ChainToken struct, chain config, multi-chain token lists
- **AUDIT:** Pending
- **CHANGE DELTA:** ~15% (mod.rs, trader.rs, config changes)

### Loop 3 (Phase 3)

- **RED:** Execution only works on Arbitrum
- **GREEN:** Multi-chain RPC clients, chain-aware execution
- **AUDIT:** Pending
- **CHANGE DELTA:** ~10% (trader.rs, engine.rs)

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

(To be filled after implementation)

- 0x's unified API with `chainId` parameter makes multi-chain trivially easy
- Permit2 contract address is identical on all EVM chains — `ensure_permit2_approval()` works everywhere
- Gasless API eliminates the need for native gas tokens on each chain
- Cross-Chain API handles bridging — no need to build bridge infrastructure
- Phase 1 (expand tokens) is zero-risk and should be done first
- Each subsequent phase adds capability without breaking existing functionality

---

## References

- 0x Supported Chains: `docs/llms-full.md` line 1419-1442
- 0x Cheat Sheet: `docs/llms-full.md` line 9643+
- 0x Gasless API: `docs/llms-full.md` line 5700+
- 0x Cross-Chain API: `docs/llms-full.md` line 1444-1474
- Permit2 Contract: `0x000000000022d473030f116ddee9f6b43ac78ba3` (all EVM chains)
- 0x Exchange Proxy: `0xfeea2a79d7d3d36753c8917af744d71f13c9b02a` (all EVM chains)
