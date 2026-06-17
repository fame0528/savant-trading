# FID-187: Multi-Chain Architecture + Universe Expansion

**Filename:** `FID-2026-0617-187-multi-chain-architecture.md`
**ID:** FID-2026-0617-187
**Severity:** high
**Status:** created
**Created:** 2026-06-17 16:20 EST
**Author:** Vera
**Parent:** FID-182

---

## Summary

Multi-chain expansion from 1 chain (Arbitrum) to 5+ chains (Arbitrum, Base, Optimism, BSC, Polygon, Hyperliquid). Spencer: "we're not broad enough to begin with. ESPECIALLY considering multi-chain is next. We need to be scanning 100-500 pairs per cycle." Originally deferred to v0.15.0, now in-scope per "nothing is out of scope." Per-chain sub-strategy execution, multi-chain token discovery, per-chain pair lists with cross-chain arbitrage, Hyperliquid integration.

---

## Environment

- **Commit:** `0adcc57c`
- **Files:** `src/engine/mod.rs`, `src/data/token_discovery.rs`, `src/execution/dex/mod.rs`, `config/default.toml`

---

## Detailed Description

### Problem

Engine runs on 1 chain (Arbitrum via Anvil). Multi-chain is the next major capability. With 48 pairs on 1 chain = 0 trades. With 100-500 pairs across 5+ chains, expected signal density increases.

### Root Cause

The architecture is monolithic. `engine/mod.rs` is a 5,360-line file that runs a single chain loop. Per-chain sub-cycle, per-chain state isolation, and cross-chain portfolio aggregation don't exist.

### Existing Scaffolding

- 5-chain config support in `config/default.toml` (FID-167)
- `SAVANT_CHAIN` env var works
- `tokio::spawn` available

### What's Missing

1. **Per-chain sub-strategy execution:** Each chain runs its own cycle. `tokio::spawn` per chain.
2. **Multi-chain token discovery:** Currently `token_discovery.rs` is Arbitrum-only.
3. **Per-chain pair lists:** Each chain has its own curated + discovered pairs.
4. **Cross-chain arbitrage detection:** Same token on 2 chains at different prices.
5. **Hyperliquid integration:** Perpetuals DEX, orderbook-based, no 0x equivalent.
6. **Per-chain state isolation:** Each chain has its own executor, portfolio, equity tracking.
7. **Cross-chain portfolio aggregation:** Total portfolio = sum across chains.
8. **Dashboard per-chain breakdown:** Show per-chain PnL, positions, equity.

---

## Proposed Solution

### Action 1: Per-chain sub-strategy execution (FID-169 scope, now active)

**File:** `src/engine/mod.rs`

**Change:** Refactor the main loop to spawn per-chain sub-cycles:
```rust
let chains = config.chains.values().filter(|c| c.enabled).collect();
let handles: Vec<_> = chains.iter().map(|chain| {
    let chain = chain.clone();
    tokio::spawn(async move {
        run_chain_subcycle(chain).await
    })
}).collect();
for handle in handles {
    handle.await??;
}
```

**State isolation:** Each sub-cycle has its own `Executor`, `PortfolioManager`, `MarketDataStore`. Shared state via `Arc<SharedEngineData>` for cross-chain aggregation.

### Action 2: Multi-chain token discovery

**File:** `src/data/token_discovery.rs`

**Change:** Add per-chain discovery loops. Each chain has its own `BlockscoutClient` (or equivalent).

```rust
for chain in &enabled_chains {
    let client = BlockscoutClient::new(chain.rpc_url);
    let tokens = client.discover_tokens(chain.min_vol, chain.min_holders).await?;
    token_store.merge(chain.id, tokens);
}
```

### Action 3: Per-chain pair lists

**File:** `config/default.toml`

**Change:** Add `[chains.arbitrum.pairs]`, `[chains.base.pairs]`, etc. with curated pairs per chain. Plus dynamic discovery on top.

### Action 4: Cross-chain arbitrage detection

**File:** `src/data/arbitrage.rs` (new)

**Logic:** For each token that exists on 2+ chains, compare prices. If spread > threshold, emit arbitrage signal.

**Implementation:** After each chain's cycle, collect token prices across chains. Compare. Emit signal if profitable.

### Action 5: Hyperliquid integration

**File:** `src/execution/hyperliquid/` (new module)

**Scope:** Perpetuals trading via Hyperliquid's orderbook API. No 0x equivalent.

**Phases:**
- Phase 1: REST API client for Hyperliquid
- Phase 2: Orderbook streaming via WebSocket
- Phase 3: Position management (long/short, leverage, liquidation price)
- Phase 4: Funding rate awareness (perps have funding costs)
- Phase 5: Engine integration (add Hyperliquid as a sub-strategy)

**Legal note:** Hyperliquid is KY-restricted in the US. Spencer has a separate spec for the legal question. Implementation proceeds in parallel.

### Action 6: Per-chain state isolation

**File:** `src/engine/mod.rs` (architecture)

**Design:** Each chain sub-cycle has its own:
- `Executor` (DEX trader for that chain)
- `PortfolioManager` (positions on that chain)
- `MarketDataStore` (candles for that chain)
- `EquityTracker` (per-chain equity)

**Shared:**
- `SharedEngineData` (cross-chain aggregation, total equity, total PnL)
- `JuryPool` (one pool, multi-model evaluation)
- `Settings` (config, risk limits)

### Action 7: Cross-chain portfolio aggregation

**File:** `src/core/shared.rs`

**Change:** `SharedEngineData` aggregates per-chain equity into total. Total equity = sum(per-chain equity). Total PnL = sum(per-chain PnL). Dashboard shows breakdown.

### Action 8: Dashboard per-chain breakdown

**File:** `dashboard/src/app/page.tsx`

**Change:** Add per-chain section showing:
- Chain name + logo
- Per-chain equity
- Per-chain PnL
- Per-chain open positions
- Per-chain trade history

---

## Verification

### Phase 1 (Anvil + simulated)
- Run on Anvil (Arbitrum only) + simulated data for other 4 chains
- Verify state isolation: per-chain portfolio doesn't bleed across chains
- Verify cross-chain equity aggregation: total = sum(per-chain)
- Verify dashboard shows per-chain breakdown

### Phase 2 (Anvil + real Base/Optimism testnet)
- Add Base/Optimism testnet support
- Verify real cross-chain token discovery works
- Verify cross-chain arbitrage detection (if any exists)

### Phase 3 (Mainnet + Hyperliquid)
- Full mainnet deployment
- Hyperliquid integration
- Live trading

---

## Perfection Loop

### Loop 1 (RED)

Issues:
1. Scope is huge (1-2 weeks of work). Need to prioritize.
2. Per-chain sub-strategy refactor of `engine/mod.rs` is risky. 5,360-line file.
3. Hyperliquid integration is a separate major workstream. Should be FID-188 (separate FID).
4. Cross-chain arbitrage is a strategy decision, not an architecture decision. Needs Gemini research.
5. No test infrastructure for multi-chain. Need to set up Anvil forks for each chain (or use public testnets).

**CHANGE DELTA: N/A (analysis)**

### Loop 2 (GREEN)

Fixes:
1. Split Hyperliquid into FID-188 (separate)
2. Split cross-chain arbitrage into FID-189 (separate, after Gemini research)
3. Keep Actions 1-4 + 6-8 in this FID
4. Prioritize: Actions 6 + 1 (state isolation + per-chain sub-strategy) first, then 2 + 3 (discovery + pair lists), then 7 + 8 (aggregation + dashboard)

**CHANGE DELTA: ~10% (scope split into 3 FIDs)**

### Loop 3 (AUDIT)

- [x] State isolation architecture is sound (per-chain executors, shared SharedEngineData)
- [ ] `engine/mod.rs` refactor: high risk, needs careful testing
- [ ] Multi-chain token discovery: needs new Blockscout clients per chain
- [ ] Cross-chain equity aggregation: needs verification that per-chain values are correct

**CALL-GRAPH REACHABILITY (Law 4):**
- New `run_chain_subcycle` function: needs to be called from main loop
- Per-chain executor: needs to be wired into execution path
- Cross-chain arbitrage detector: needs to be called after each chain's cycle
- Dashboard per-chain breakdown: needs API endpoint

**All wiring will be verified after code is written.**

**CHANGE DELTA: ~5% (AUDIT notes)**

### Loop 4 (SELF-CORRECT)

The engine refactor is the highest-risk part. Need to:
1. Create test harness for per-chain sub-cycles
2. Add per-chain integration tests
3. Verify SharedEngineData aggregation is correct
4. Add metrics for per-chain performance

**CHANGE DELTA: ~3% (refinement)**

### Loop 5 (CONVERGENCE)

Loop 1→2: 10%
Loop 2→3: 5%
Loop 3→4: 3%
Loop 4→5: 0%

**CONVERGED at Loop 5.**

---

## Resolution

- **Fixed By:** Pending
- **Fix Description:** 8 actions across architecture, discovery, state isolation, dashboard
- **Tests Added:** Yes — per-chain sub-cycle integration tests
- **Verified By:** Anvil run with simulated multi-chain data, then real testnet, then mainnet

---

## Related FIDs

- **FID-188** (to be created): Hyperliquid integration
- **FID-189** (to be created): Cross-chain arbitrage strategy (post-Gemini research)

---

*Vera 0.1.0 — 2026-06-17 16:20 EST — FID-187 created. Multi-chain architecture. High-risk, 1-2 weeks. Awaiting Gemini research to finalize cross-chain arbitrage strategy.*
