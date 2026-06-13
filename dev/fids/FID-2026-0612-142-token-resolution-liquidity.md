# FID-142: Token Resolution → 0x Liquidity Failures (Live Bot: 2 BUYs, $0 On-Chain)

**Status:** CLOSED — implemented v0.14.0
**Severity:** critical (blocking all live trades on non-database tokens)
**Date:** 2026-06-12
**Version:** v0.14.0-target

---

## Live Symptoms (from terminal log v16.2.7)

### Cycle 1 (6:58 PM)
| Metric | Value |
|---|---|
| Pairs evaluated | 34 |
| BUY decisions | 2 |
| PASS decisions | 32 |
| Parse failures | 0 |
| Latency | 45.0s |

**BUY 1:** GIGA/USD — conviction=0.320, confidence=55%, Trending, ADX 21.1
**BUY 2:** PUMP/USD — conviction=0.300, confidence=50%, Trending, ADX 27.3

**Both rejected:**
```
[LIQUIDITY] GIGA/USD — No DEX liquidity available (0x /price returned false)
[LIQUIDITY] PUMP/USD — No DEX liquidity available (0x /price returned false)
```

**Wallet:** $21.73 USDC → $21.73 USDC (zero change, zero on-chain activity)

### Cycle 2 (7:05 PM)
| Metric | Value |
|---|---|
| Pairs evaluated | 34 |
| BUY decisions | 0 |
| PASS decisions | 34 |
| Latency | 59.7s |

**All 34 pairs:** conviction=0.000 (M3 self-censorship post-FID-140 threshold unification)

### Broader token database gap
```
[GoPlus] GoPlus: no known address for AB — skipping security check
[GoPlus] GoPlus: no known address for ADA — skipping security check
[GoPlus] GoPlus: no known address for GIGA — skipping security check
[GoPlus] GoPlus: no known address for PUMP — skipping security check
... (87 total across 2 cycles, ~30 unique tokens)
```

---

## Root Cause Analysis (3 Layers)

### Layer 1: `check_liquidity()` passes empty address → 0x returns `liquidityAvailable: false`

**File:** `src/execution/dex/trader.rs` ~line 2262
**File:** `src/engine/mod.rs` ~line 3467

```rust
async fn check_liquidity(&self, pair: &str, side: Side, amount_usd: f64) -> Result<...> {
    let (src_token, dst_token) = resolve_pair_on_chain(pair, side, self.chain_id)?;
    // GIGA not in ARBITRUM_TOKENS → dst_token.address = "" (empty string)

    let params = SwapParams {
        src_token: src_token.address.clone(),  // USDC: "0xaf88d..." ✓
        dst_token: dst_token.address.clone(),  // GIGA: "" → 0x /price?buyToken= → FAILS
        chain_id: 42161,  // hardcoded to Arbitrum
        ...
    };
    self.backend.check_liquidity(&params).await
}
```

**0x API call:** `GET /swap/permit2/price?chainId=42161&sellToken=0xaf88d...&buyToken=` — empty buyToken → 0x returns `liquidityAvailable: false`.

### Layer 2: The enterprise fix already exists in `execute_swap()` but not in `check_liquidity()`

**File:** `src/execution/dex/trader.rs` ~line 762

```rust
// THIS CODE ALREADY WORKS (for execute_swap):
let src_id = if src_token.address.is_empty() {
    warn!("Token '{}' not in local DB — resolving via API symbol lookup", src_token.symbol);
    src_token.symbol.clone()  // ← 0x resolves "GIGA" natively
} else {
    src_token.address.clone()
};
```

The same symbol-fallback pattern needs to be applied to `check_liquidity()` and `place_order()`.

### Layer 3: `chain_id` is hardcoded to 42161 everywhere

Even with symbol fallback, `check_liquidity` only tries Arbitrum. If GIGA has liquidity on Ethereum (chain 1) or Base (chain 8453) but not Arbitrum, it still fails. The `DexTrader` already has multi-chain infrastructure (`chain_clients`, `chain_configs`, `add_chain()`), but it's never used for liquidity discovery.

### Bonus finding: GoPlus security checks also skip non-database tokens

```rust
// src/security/goplus.rs line 187:
if let Some((address, _decimals)) = crate::execution::dex::lookup_token(&upper, 42161) {
    // runs security check
} else {
    warn!("GoPlus: no known address for {} — skipping security check", upper);
}
```

**Impact:** 30+ tokens have zero security validation (no honeypot check, no tax check). This is a security gap, not just a trading gap.

---

## What Needs to Change

### Fix A: Symbol fallback in `check_liquidity()` and `place_order()`

Apply the same pattern that `execute_swap()` already uses: when `resolve_pair_on_chain` returns an empty address for a token, pass its TICKER SYMBOL to 0x instead. 0x natively resolves symbols to the most liquid on-chain contract.

**Files:** `src/execution/dex/trader.rs` (2 methods), `src/engine/mod.rs` (1 call site)

### Fix B: Multi-chain liquidity discovery

When Arbitrum returns `liquidityAvailable: false`, try Ethereum (chain 1), Base (8453), Optimism (10) in order. Return the chain_id where liquidity was found so execution can proceed on that chain.

**Design question:** Should multi-chain execution be in this FID or a separate FID? Multi-chain execution requires RPC endpoints for each chain, gas management per chain, and cross-chain balance tracking. It's a significant scope expansion.

### Fix C: GoPlus symbol fallback (bonus)

GoPlus also accepts token addresses. If a token isn't in the local DB, we could pass its symbol to GoPlus for security validation. But this is lower priority than the liquidity fix.

---

## Perfection Check — Questions I Might Have Missed

1. **If GIGA is a Solana token (not EVM), will 0x ever route it?** The 0x API only supports EVM chains. If GIGA's primary contract is on Solana, no amount of multi-chain iteration will find liquidity. The bot needs a way to distinguish "token exists on Arbitrum but has no 0x routing" from "token doesn't exist on any EVM chain." → **Need to actually call 0x /price with the symbol and see what happens.**

2. **Does `execute_swap()` symbol fallback actually work?** The code exists but has it ever been tested? The symbol fallback was added with the comment "0x / 1inch API accepts both addresses and symbols natively" — but has this been verified against the live 0x API?

3. **Will multi-chain execution cause issues with the wallet?** The wallet is on Arbitrum (0x543c...). If we execute on Base, we need ETH on Base for gas. The `sync_balance()` already queries multiple chains, but `place_order()` still hardcodes `self.chain_id`.

4. **What about the permit2 approval?** For LONG buys, the src token is USDC (always has an address). But for SHORT sells of unknown tokens, we'd need the address for `ensure_permit2_approval()`. The `place_order()` guard `if src_token.address.is_empty() { return Err }` is a real blocker for SHORT trades on unknown tokens.

5. **The `check_liquidity` in engine/mod.rs line 2267 hardcodes `chain_id: 42161` in the SwapParams, but the `ExecutionEngine::check_liquidity` trait method passes `self.chain_id` — are these the same call site?** Need to verify which code path the engine's liquidity check takes.

---

## Proposed Scope

### In-scope for FID-142 (minimal, safe)
1. **Symbol fallback in `check_liquidity()`** — trader.rs line 2262
2. **Symbol fallback in engine liquidity check** — engine/mod.rs
3. **Relax `place_order()` guard** for LONG trades (src=USDC always has address)
4. **Test:** Call 0x /price with symbol "GIGA" on Arbitrum → verify liquidityAvailable

### Deferred to FID-143+ (multi-chain)
- Multi-chain liquidity discovery (try Ethereum/Base/Optimism)
- Multi-chain execution (RPC, gas, wallet per chain)
- GoPlus symbol fallback

---

## Files Touched

| File | Change |
|---|---|
| `src/execution/dex/trader.rs` | check_liquidity() symbol fallback, relax place_order() guard |
| `src/engine/mod.rs` | Symbol fallback at liquidity check call site |

---

## Verification

- [ ] `cargo check` passes
- [ ] `cargo test` passes (308 tests)
- [ ] Manual 0x /price API call with symbol "GIGA" on Arbitrum confirms liquidityAvailable
- [ ] Manual 0x /price API call with symbol "PUMP" on Arbitrum confirms liquidityAvailable
- [ ] If symbol lookup fails (token is Solana-native), documented as expected behavior
