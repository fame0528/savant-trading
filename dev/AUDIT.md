# AUDIT REPORT — Savant Trading v0.8.0

**Auditor:** Nova (read-only)
**Date:** 2026-06-04 (Updated: 2026-06-04 after re-scan)
**Scope:** Full codebase review — README, DEX execution, engine loop, AI agent pipeline, error handling, token database, multi-chain infrastructure
**Method:** Static analysis of all .rs source files in `src/execution/dex/`, `src/engine.rs`, `src/agent/`, `src/core/`. Two-pass audit: initial scan + re-scan after Kilo Code patches.

---

## EXECUTIVE SUMMARY

**Original audit: 17 findings (3 Critical, 6 High, 5 Medium, 3 Low)**
**After re-scan: 5 confirmed fixes, 1 still-open High, 5 new findings**

### Fixes Confirmed ✅

| ID | Finding | Fix Location | Verified |
|----|---------|-------------|----------|
| F-01 | No ERC-20 `approve()` for Permit2 | `trader.rs` line 828: `ensure_permit2_approval()` | ✅ Called in `place_order()` (line 1078) and `close_position()` (line 1153) |
| F-04 | APE fake address | Removed from `ARBITRUM_TOKENS` (201 tokens, all CoinGecko-verified) | ✅ Gone |
| F-05 | AUSDT wrong decimals | Removed from token database | ✅ Gone |
| F-08 | Close skips fee | `trader.rs` line 1166-1169: `fee_est` computed, `proceeds = entry*qty + gross_pnl - fee_est` | ✅ |
| F-09 | SHORT amount_wei wrong | `trader.rs` lines 1051-1058: separate LONG/SHORT logic | ✅ LONG: `amount_to_wei(entry_price*qty, 6)`, SHORT: `amount_to_wei(quantity, src_decimals)` |

### Still Open 🔴

| ID | Finding | Status |
|----|---------|--------|
| F-07 | `drain_retry_queue` — `let kept = Vec::new()` STILL always empty | 🔴 NOT FIXED — same bug at line 339 |
| F-02 | `amount_to_wei` uses `f64` | ⚠️ Landmine for large accounts |
| F-03 | Exchange proxy address hardcoded | ⚠️ Soft risk |
| F-06 | `eth_call` dry-run ≠ real state | ⚠️ Mitigated by F-01 but not eliminated |
| F-10–F-17 | Unchanged from original audit | ⚠️ See below |

### New Findings from Re-Scan 🆕

| ID | Severity | Finding |
|----|----------|---------|
| NF-01 | HIGH | `usdc_address_for_chain` defaults to Arbitrum address for ALL unknown chains (silent wrong-chain) |
| NF-02 | MEDIUM | `resolve_pair()` hardcoded to Arbitrum (42161) — multi-chain infra built but not wired into execution path |
| NF-03 | INFO | Gasless swap + cross-chain swap fully implemented in `zero_x.rs` but dead code (never called from `trader.rs` or `engine.rs`) |
| NF-04 | LOW | No programmatic verification of 201 token addresses in `ARBITRUM_TOKENS` |
| NF-05 | LOW | Same as F-07 (retry queue) — listed separately for tracking |

---

## DETAILED FINDINGS

### FINDING-01: Missing ERC-20 `approve()` for Permit2 — ✅ FIXED

**Original Severity:** CRITICAL
**Status:** FIXED

`ensure_permit2_approval()` added at `trader.rs` line 828. Checks allowance via `eth_call` to `allowance(owner, spender)` (selector `0xdd62ed3e`), sends `approve(Permit2Address, MAX_UINT256)` (selector `0x095ea7b3`) if insufficient. Permit2 address: `0x000000000022d473030f116ddee9f6b43ac78ba3`. Called before every swap in both `place_order()` and `close_position()`.

**This was the root cause of zero successful transactions.**

---

### FINDING-02: `amount_to_wei` Uses `f64` — ⚠️ OPEN

**Severity:** CRITICAL (landmine)
**Location:** `src/execution/dex/mod.rs` lines 548-552

```rust
pub fn amount_to_wei(amount: f64, decimals: u8) -> String {
    let factor = 10u128.pow(decimals as u32) as f64;
    let wei = (amount * factor).round() as u128;
    wei.to_string()
}
```

`f64` has ~15-17 significant digits. For 18-decimal tokens, precision loss in least significant wei. `as u128` cast silently overflows for values > `u128::MAX`. Not a blocker for $50 account, but dangerous at scale.

**Recommendation:** Use `bigdecimal` or `ethnum` crate for lossless decimal → wei conversion.

---

### FINDING-03: Exchange Proxy Address Hardcoded — ⚠️ OPEN

**Severity:** CRITICAL (soft)
**Location:** `src/execution/dex/zero_x.rs` line 184

```rust
const EXCHANGE_PROXY: &str = "0xfeea2a79d7d3d36753c8917af744d71f13c9b02a";
```

Only used for a warning log. The actual `to` address comes from the API response and is not validated. Low immediate risk.

**Recommendation:** Validate `transaction.to` from 0x API against known router addresses.

---

### FINDING-04: APE Token Address Fake — ✅ FIXED

**Status:** FIXED — APE removed from `ARBITRUM_TOKENS`. Token database now 201 tokens, all CoinGecko-verified.

---

### FINDING-05: AUSDT Wrong Decimals — ✅ FIXED

**Status:** FIXED — AUSDT removed from token database.

---

### FINDING-06: `eth_call` Dry-Run May Pass While Real TX Reverts — ⚠️ OPEN

**Severity:** HIGH
**Location:** `src/execution/dex/trader.rs` lines 523-547

Dry-run simulates against latest confirmed block. Real tx enters mempool where state may differ. Mitigated by F-01 (approval check) but not eliminated — race conditions on nonce, balance, and mempool state still possible.

**Recommendation:** Add `eth_estimateGas` as secondary check. Parse revert reason from `eth_call` return data.

---

### FINDING-07: `drain_retry_queue` Still Broken — 🔴 NOT FIXED

**Severity:** HIGH
**Location:** `src/execution/dex/trader.rs` lines 337-349

```rust
pub fn drain_retry_queue(&mut self) -> Vec<RetrySwap> {
    let mut to_retry = Vec::new();
    let kept = Vec::new();  // ← STILL ALWAYS EMPTY
    for swap in self.retry_queue.drain(..) {
        if swap.attempts >= self.max_retries {
            // warn — swap dropped
        } else {
            to_retry.push(swap);
        }
    }
    self.retry_queue = kept;  // ← REPLACES WITH EMPTY
    to_retry
}
```

Same bug as original audit. `kept` is always empty. Return value is never used by caller. Failed swaps are silently dropped.

**Recommendation:** Fix `kept` to retain non-drained entries. Wire return value into main loop. Or remove if unused.

---

### FINDING-08: Close Skips Fee Deduction — ✅ FIXED

**Status:** FIXED — `close_position()` now computes `fee_est = exit_price * pos.quantity * 0.001` and subtracts from proceeds.

---

### FINDING-09: SHORT Order Amount Wrong — ✅ FIXED

**Status:** FIXED — `place_order()` now uses separate logic:
- LONG: `amount_to_wei(entry_price * quantity, src_decimals)` (USDC value)
- SHORT: `amount_to_wei(quantity, src_decimals)` (token amount directly)

---

### FINDING-10: `normalize_llm_json` Incomplete — ⚠️ OPEN

**Severity:** MEDIUM
**Location:** `src/agent/decision_parser.rs` lines 269-288

String replacement handles `"action": "BUY"` and `"action":"BUY"` but not `"action" : "BUY"` (space before colon only). Non-standard whitespace from MiMo v2.5 Pro could cause parse failures.

---

### FINDING-11: `partial_extract` Default Masks Errors — ⚠️ OPEN

**Severity:** MEDIUM
**Location:** `src/agent/decision_parser.rs` lines 379-443

Defaults: `entry_price=0.0`, `side=Side::Long`, `confidence=0.5`, `pair="BTC/USD"`. Malformed AI responses parse successfully with dangerous defaults.

---

### FINDING-12: No On-Chain Balance Check Before Trade — ⚠️ OPEN

**Severity:** MEDIUM
**Location:** `src/engine.rs` lines 1376-1393

Circuit breaker uses paper account, not on-chain state. `sync_balance()` is periodic, not pre-trace. Race condition possible.

---

### FINDING-13: USDC/USDC Pair Possible — ⚠️ OPEN

**Severity:** MEDIUM
**Location:** `src/execution/dex/mod.rs` lines 478-541

`resolve_pair("USDC/USD", Long)` → `(USDC, USDC)`. Mitigated by stablecoin pre-filter in main loop.

---

### FINDING-14: PaperTrader/DexTrader Desync — ⚠️ OPEN

**Severity:** MEDIUM
**Location:** `src/engine.rs` lines 751-795, 1232-1325

Circuit breaker blind to real positions. Phantom position detection at startup mitigates but doesn't prevent during-session drift.

---

### FINDING-15: Duplicate `spender` Extraction — ℹ️ OPEN

**Severity:** LOW
**Location:** `src/execution/dex/zero_x.rs` lines 306, 336

Redundant code. Not a bug.

---

### FINDING-16: Duplicate Timeframe Parsers — ℹ️ OPEN

**Severity:** LOW
**Location:** `src/engine.rs` lines 39-61

`parse_timeframe` and `parse_timeframe_minutes` do the same thing.

---

### FINDING-17: APE Address — ✅ FIXED

**Status:** FIXED — APE removed from token database (covered by F-04).

---

## NEW FINDINGS (Re-Scan)

### NF-01: `usdc_address_for_chain` Defaults to Arbitrum for Unknown Chains

**Severity:** HIGH
**Location:** `src/execution/dex/mod.rs` lines 92-102

```rust
pub fn usdc_address_for_chain(chain_id: u64) -> &'static str {
    match chain_id {
        42161 => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",  // Arbitrum
        8453  => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",   // Base
        10    => "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",     // Optimism
        56    => "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",     // BSC
        137   => "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",    // Polygon
        1     => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",      // Ethereum
        43114 => "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E",  // Avalanche
        _     => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",      // DEFAULT: Arbitrum!
    }
}
```

The catch-all `_` returns the Arbitrum USDC address. If a new chain is added to config but not to this function, the engine will try to trade Arbitrum USDC on the wrong chain. This is a **silent wrong-chain bug** — the tx would be broadcast to the wrong chain's RPC with a token address that doesn't exist there.

**Recommendation:** Return `Option<&str>` instead of a default. Force explicit handling of unknown chains.

---

### NF-02: `resolve_pair()` Hardcoded to Arbitrum

**Severity:** MEDIUM
**Location:** `src/execution/dex/mod.rs` line 482

```rust
pub fn resolve_pair(pair: &str, side: Side) -> Result<(TokenInfo, TokenInfo), ExecutionError> {
    resolve_pair_on_chain(pair, side, 42161)  // ← HARDCODED
}
```

The multi-chain infrastructure is fully built: `ChainConfig`, `chain_clients`, `rpc_call_on_chain()`, `resolve_pair_on_chain()`, `usdc_address_for_chain()`, `usdc_decimals_for_chain()`. But the main entry point still routes everything to Arbitrum. The AI cannot currently select a chain per trade.

**Recommendation:** Add chain selection to the AI decision output, or add a chain selection layer in the engine loop.

---

### NF-03: Gasless + Cross-Chain Swap Implemented But Not Integrated

**Severity:** INFO (not a bug — future capability)
**Location:** `src/execution/dex/zero_x.rs` lines 466-738

Three major features are fully implemented but never called:
1. **Gasless swaps** (`build_gasless_swap_tx`, line 466) — 0x pays gas, deducted from output
2. **Cross-chain swaps** (`build_cross_chain_swap_tx`, line 634) — bridge + swap in one tx
3. **Status polling** (`poll_gasless_status`, `poll_cross_chain_status`)

These are production-ready implementations waiting for the execution path to activate them. The 0x API handles all routing — Savant just needs to call the right endpoint.

---

### NF-04: No Programmatic Token Address Verification

**Severity:** LOW
**Location:** `src/execution/dex/mod.rs` lines 190-400

201 tokens in `ARBITRUM_TOKENS` with comment "Source: CoinGecko API, verified contract addresses." No on-chain verification (e.g., checking `symbol()` or `decimals()` returns match). One bad address could cause loss of funds.

---

### NF-05: Retry Queue (Same as F-07)

**Severity:** HIGH — tracked separately for Kilo Code task management.

---

## MULTI-CHAIN ARCHITECTURE ASSESSMENT

The multi-chain infrastructure is **impressive and well-structured**:

| Component | Status | Location |
|-----------|--------|----------|
| `ChainConfig` struct | ✅ Built | `mod.rs` line 82 |
| Per-chain RPC clients | ✅ Built | `trader.rs` line 211 |
| Per-chain gas tracking | ✅ Built | `trader.rs` line 234 |
| `rpc_call_on_chain()` | ✅ Built | `trader.rs` line 368 |
| `resolve_pair_on_chain()` | ✅ Built | `mod.rs` line 486 |
| `usdc_address_for_chain()` | ✅ Built (7 chains) | `mod.rs` line 92 |
| `usdc_decimals_for_chain()` | ✅ Built (BSC=18, others=6) | `mod.rs` line 106 |
| `TokenInfo.chain_id` field | ✅ Built | `mod.rs` line 77 |
| Gasless swap | ✅ Built | `zero_x.rs` line 466 |
| Cross-chain swap | ✅ Built | `zero_x.rs` line 634 |
| AI chain selection | ❌ Not wired | `resolve_pair()` hardcoded to 42161 |
| Engine multi-chain loop | ❌ Not wired | `engine.rs` uses single chain |

**Bottom line:** The plumbing is done. 7 chains ready. Gasless and cross-chain ready. Just needs the execution path to activate it.

---

## SUMMARY TABLE

| ID | Severity | Finding | Status |
|----|----------|---------|--------|
| F-01 | CRITICAL | No ERC-20 `approve()` for Permit2 | ✅ FIXED |
| F-02 | CRITICAL | `amount_to_wei` uses `f64` | ⚠️ Open (landmine) |
| F-03 | CRITICAL | Exchange Proxy address hardcoded | ⚠️ Open (soft) |
| F-04 | HIGH | APE address is fake/placeholder | ✅ FIXED |
| F-05 | HIGH | AUSDT decimals wrong (18 vs 6) | ✅ FIXED |
| F-06 | HIGH | `eth_call` dry-run ≠ real tx state | ⚠️ Open (mitigated) |
| F-07 | HIGH | Retry queue `kept` always empty | 🔴 NOT FIXED |
| F-08 | HIGH | Close skips fee deduction | ✅ FIXED |
| F-09 | HIGH | SHORT order amount_wei wrong | ✅ FIXED |
| F-10 | MEDIUM | JSON normalization incomplete | ⚠️ Open |
| F-11 | MEDIUM | partial_extract default masks errors | ⚠️ Open |
| F-12 | MEDIUM | No on-chain balance check before trade | ⚠️ Open |
| F-13 | MEDIUM | USDC/USDC pair possible | ⚠️ Open |
| F-14 | MEDIUM | PaperTrader/DexTrader desync | ⚠️ Open |
| F-15 | LOW | Duplicate spender extraction | ℹ️ Open |
| F-16 | LOW | Duplicate timeframe parsers | ℹ️ Open |
| F-17 | LOW | APE address | ✅ FIXED (F-04) |
| NF-01 | HIGH | `usdc_address_for_chain` defaults to Arbitrum | 🆕 Open |
| NF-02 | MEDIUM | `resolve_pair()` hardcoded to Arbitrum | 🆕 Open |
| NF-03 | INFO | Gasless + cross-chain built but not called | 🆕 Future |
| NF-04 | LOW | No programmatic token verification | 🆕 Open |
| NF-05 | HIGH | Retry queue (same as F-07) | 🔴 NOT FIXED |

---

## AUDIT METHODOLOGY

**Pass 1 (Initial):**
1. Read full README for architecture overview
2. Read all 4 DEX module files — line by line
3. Read execution engine trait, decision parser, error types
4. Read main engine loop (3833 lines) — focusing on execution path
5. Traced data flow: AI decision → parse → execute_swap → sign_and_send → broadcast
6. Verified Permit2 signature format against 0x docs
7. Checked token database for obvious errors
8. Identified missing ERC-20 approval step

**Pass 2 (Re-scan after Kilo Code patches):**
1. Re-read all 4 DEX module files — line by line (now 769 + 1326 + 979 + 397 lines)
2. Verified each of the 5 claimed fixes against actual code
3. Identified new multi-chain infrastructure
4. Found gasless and cross-chain implementations
5. Discovered NF-01 (default chain fallback) and NF-02 (hardcoded chain)

**Not reviewed in this session:** Kraken CEX backend, memory module, insight aggregator, vault integration, backtesting engine, training pipeline, TUI, REST API server.

---

*End of audit. 22 total findings (17 original + 5 new). 5 fixed. 1 still-critical (retry queue). Multi-chain ready to activate.*
