# AUDIT REPORT — Savant Trading v0.8.0

**Auditor:** Nova (read-only)
**Date:** 2026-06-04
**Scope:** Full codebase review — README, DEX execution, engine loop, AI agent pipeline, error handling, token database
**Method:** Static analysis of all .rs source files in `src/execution/dex/`, `src/engine.rs`, `src/agent/`, `src/core/`

---

## EXECUTIVE SUMMARY

17 findings across 4 severity tiers. **3 Critical** findings likely explain why zero transactions have succeeded. **6 High** findings are latent bugs that will cause failures under specific conditions. **5 Medium** are correctness issues. **3 Low/style**.

### Top Root Cause Candidates (Why No TX Has Ever Succeeded)

1. **FINDING-01 [CRITICAL]: No ERC-20 `approve()` call before Permit2 swaps** — Without this, every swap reverts on-chain.
2. **FINDING-06 [CRITICAL]: `eth_call` dry-run may pass while real tx reverts** — Simulated state ≠ real mempool state.
3. **FINDING-04 [HIGH]: APE token has a fake/placeholder address** — Will produce a tx to a non-existent contract.

---

## CRITICAL FINDINGS

### FINDING-01: Missing ERC-20 `approve()` for Permit2 Contract

**Severity:** CRITICAL
**Location:** `src/execution/dex/trader.rs` — `execute_swap()` method (line 573+)
**Location:** `src/execution/dex/zero_x.rs` — `build_swap_tx()` method (line 383+)

**Description:**
The 0x Permit2 flow requires TWO on-chain authorizations before a swap can execute:

1. **ERC-20 `approve(Permit2Address, amount)`** — The token contract (USDC) must approve the Permit2 contract to transfer tokens on behalf of the wallet.
2. **Permit2 `permit()` (EIP-712 signature)** — The wallet signs a message allowing the 0x Exchange Proxy to spend tokens via Permit2.

Your code handles step 2 correctly — the EIP-712 signing in `sign_permit2()` and the calldata construction in `build_swap_tx()` are properly implemented with the correct `calldata || sig_len (32 bytes) || sig (65 bytes)` format.

**However, step 1 is completely missing.** There is no code anywhere in the codebase that sends an ERC-20 `approve()` transaction to the USDC contract targeting the Permit2 contract. If the wallet has never called `USDC.approve(0x...Permit2Address, ...)`, the Permit2 contract will revert the token transfer, causing every swap to fail.

**Evidence:**
- grep for `approve` in the DEX module: only appears in comment strings describing what the 0x API does, never as an actual contract call.
- The `execute_swap()` method goes directly from `quote()` → `build_swap_tx()` → `sign_and_send()`. No intermediate approval tx.
- The `DexTrader::new()` constructor does not check or establish Permit2 approvals.

**Impact:** 100% of DEX swaps will revert. This single finding likely explains why zero transactions have ever succeeded.

**Recommendation:**
- Before the first swap for any token, check the current ERC-20 allowance: `eth_call` to `USDC.allowance(wallet, Permit2Address)`.
- If allowance < swap amount, send an `approve(Permit2Address, type(uint256).max)` transaction.
- The Permit2 contract address on Arbitrum is `0x000000000022D473030F116dDEE9F6B43aC78BA3`.
- Consider approving `uint256.MAX` to avoid repeated approval transactions.

**Verification Command:**
```bash
# Check on Arbiscan if your wallet has approved the Permit2 contract:
# https://arbiscan.io/token/0xaf88d065e77c8cC2239327C5EDb3A432268e5831#readContract
# Call allowance(yourWallet, 0x000000000022D473030F116dDEE9F6B43aC78BA3)
```

---

### FINDING-02: `amount_to_wei` Uses `f64` — Precision Loss for 18-Decimal Tokens

**Severity:** CRITICAL
**Location:** `src/execution/dex/mod.rs` lines 494-497

**Code:**
```rust
pub fn amount_to_wei(amount: f64, decimals: u8) -> String {
    let factor = 10u128.pow(decimals as u32) as f64;
    let wei = (amount * factor).round() as u128;
    wei.to_string()
}
```

**Description:**
`f64` has ~15-17 significant decimal digits. For 18-decimal tokens, `amount * 10^18` can lose precision in the least significant digits. This means the wei amount sent to the 0x API may differ from the actual intended amount by a few wei. While this is unlikely to cause a revert (the 0x API rounds to the nearest valid amount), it creates a silent discrepancy between what the AI thinks it's trading and what actually gets executed.

More critically, the `as u128` cast from `f64` is a **silent overflow** for values > `u128::MAX` (~3.4 × 10^38). For a small account this won't trigger, but it's a landmine for future scaling.

**Impact:** Low immediate impact (small account), but architecturally dangerous. Could cause silent wrong-amount trades at scale.

**Recommendation:**
Use integer arithmetic throughout: `amount.to_string().parse::<BigDecimal>() * BigDecimal::from(10).pow(decimals)`. The `bigdecimal` or `ethnum` crates provide lossless decimal → wei conversion.

---

### FINDING-03: Exchange Proxy Address Hardcoded Without Verification

**Severity:** CRITICAL
**Location:** `src/execution/dex/zero_x.rs` line 184

**Code:**
```rust
const EXCHANGE_PROXY: &str = "0xfeea2a79d7d3d36753c8917af744d71f13c9b02a";
```

**Description:**
The Exchange Proxy address is hardcoded as a constant. If 0x deploys a new proxy contract (they've done this before —Proxy → V2 → V3 → V4 → V5), all transactions will be sent to a dead contract. The address is only used for a warning log, not for the actual `to` address (which comes from the API response), so this is a soft failure — but the warning is misleading.

More importantly, the `swap_tx.to` address from the API response is **not validated** against any known list. If the 0x API is compromised or returns unexpected data, funds could be sent to an arbitrary contract.

**Impact:** Low immediate risk, but the hardcoded constant suggests a trust assumption that should be validated.

**Recommendation:**
Validate the `transaction.to` field from the 0x API response against a list of known 0x router addresses. Reject the swap with an explicit error if the `to` address is unknown.

---

## HIGH FINDINGS

### FINDING-04: APE Token Address Is Clearly Fake/Placeholder

**Severity:** HIGH
**Location:** `src/execution/dex/mod.rs` line 197

**Code:**
```rust
("APE", "0x7f9FBf9bDd1F0e6E2c2c2c2c2c2c2c2c2c2c2c2c", 18),
```

**Description:**
The APE address is `0x7f9FBf9bDd1F0e6E2` followed by `c2` repeated 7 times to fill 20 bytes. This is not a real Arbitrum token address. It's a placeholder that was never replaced with the correct address.

If the AI decides to buy APE, the swap calldata will be constructed with this fake address. The 0x API may reject it at quote time, or if it accepts it, the tx will send tokens to a non-existent contract (or worse, a contract controlled by someone else).

**Impact:** If APE is selected, either a failed tx or loss of funds.

**Recommendation:** Either remove APE from the token database or replace with the correct Arbitrum address. Find the real address via Arbiscan or CoinGecko API.

---

### FINDING-05: AUSDT Has Wrong Decimals (18 instead of 6)

**Severity:** HIGH
**Location:** `src/execution/dex/mod.rs` line 233

**Code:**
```rust
("AUSDT", "0x6ab707aca953edaefbc4fd23ba73294241490620", 18),
```

**Description:**
`aUSDT` (Aave interest-bearing USDT) has the same decimals as USDT: **6**. The database declares 18 decimals, which means `amount_to_wei()` will produce a wei amount that is 10^12 times too large. A $50 swap would be encoded as $50 trillion wei.

**Impact:** If Aave USDT is selected for trading, the swap will either fail or attempt to spend an astronomically wrong amount.

**Recommendation:** Change decimals from 18 to 6.

---

### FINDING-06: `eth_call` Dry-Run May Pass While Real TX Reverts

**Severity:** HIGH
**Location:** `src/execution/dex/trader.rs` lines 454-479

**Description:**
The `sign_and_send()` method performs an `eth_call` dry-run before broadcasting. This simulates the transaction against the **latest confirmed block**. However, the real transaction enters the **mempool**, where state may differ:

- Another transaction from the same wallet may change the nonce or token balance between dry-run and execution.
- The `eth_call` simulation doesn't check ERC-20 allowances on most Ethereum nodes (by default, `eth_call` only simulates EOA-to-EOA calls faithfully).
- RPC nodes may return `Ok` for `eth_call` even when the tx would fail on-chain, due to incomplete state simulation.

**Impact:** The dry-run gives false confidence. You might see "DRY-RUN OK" in logs but the tx still reverts.

**Recommendation:**
- After `eth_call` succeeds, also check that the wallet's USDC balance ≥ swap amount AND USDC allowance ≥ swap amount.
- Parse the `eth_call` return value — if it reverts, the return data contains the revert reason. Decode it instead of just checking Ok/Err.
- Consider using `eth_estimateGas` in addition to `eth_call` — if gas estimation fails, the tx will definitely revert.

---

### FINDING-07: `drain_retry_queue` Loses Retry Tracking

**Severity:** HIGH
**Location:** `src/execution/dex/trader.rs` lines 289-301

**Code:**
```rust
pub fn drain_retry_queue(&mut self) -> Vec<RetrySwap> {
    let mut to_retry = Vec::new();
    let kept = Vec::new();  // ← ALWAYS EMPTY
    for swap in self.retry_queue.drain(..) {
        if swap.attempts >= self.max_retries {
            // warn log — but swap is dropped
        } else {
            to_retry.push(swap);
        }
    }
    self.retry_queue = kept;  // ← REPLACES WITH EMPTY
    to_retry
}
```

**Description:**
The method drains the entire retry queue (`.drain(..)` empties `self.retry_queue`), returns non-expired entries as `to_retry`, but then replaces the queue with `kept` which is always empty. This means:

1. Entries that succeed on retry are lost (not put back, but that's fine).
2. Entries that fail on retry are lost too — they're returned from `drain_retry_queue()` but never re-inserted into the queue.
3. **Crucially**, the returned `Vec<RetrySwap>` is never iterated by the caller in `engine.rs`. The `drain_retry_queue()` return value is never used anywhere in the codebase — grievances the retry queue is dead code.

**Impact:** Failed swaps are silently dropped — no retry, no error logged, nothing.

**Recommendation:**
1. Fix the `kept` vector to actually retain entries that should not be retried yet.
2. Wire `drrain_retry_queue()` into the main loop so retried swaps are actually executed.
3. If retry queue is intended to be unused, remove it to avoid confusion.

---

### FINDING-08: `close_position` Balance Accounting Skips Fee Deduction

**Severity:** HIGH
**Location:** `src/execution/dex/trader.rs` lines 994-995

**Code:**
```rust
let proceeds = pos.entry_price * pos.quantity + gross_pnl;
self.balance += proceeds;
```

**Description:**
The close logic adds `entry_price * qty + gross_pnl` (= `exit_price * qty`) to balance with **no fee deduction**. Meanwhile, `place_order` at line 950 deducts `order_value * 1.001` (0.1% fee on entry). This means:
- Entry: you pay 0.1% fee correctly.
- Exit: you receive 0% fee (should also be ~0.1% on DEX swaps).

Over many round-trips, tracked balance will drift significantly above actual on-chain balance.

**Impact:** Balance drift accumulates, eventually causing the phantom position detection to fire and wipe all tracked positions.

**Recommendation:** Deduct estimated swap fee on close: `self.balance += proceeds * (1.0 - fee_rate)`.

---

### FINDING-09: `place_order` Computes `amount_wei` from `order_value = entry_price * quantity`

**Severity:** HIGH
**Location:** `src/execution/dex/mod.rs` lines 906-908

**Code:**
```rust
let entry_price = price.unwrap_or(0.0);
let order_value = entry_price * quantity;
let amount_wei = amount_to_wei(order_value, src_token.decimals);
```

**Description:**
The AI returns `entry_price` (e.g., $65000 for BTC) and the position sizer calculates `quantity` (e.g., 0.000769 BTC for $50). The code computes `amount_wei = amount_to_wei(65000 * 0.000769, 6)` = `amount_to_wei(50.0, 6)` = `"50000000"` (50 USDC).

This works **only when the source token is USDC** (6 decimals). For non-stablecoin source tokens (e.g., WETH as source for a SHORT sell), `entry_price * quantity` gives the value in WETH terms, but `amount_to_wei` converts it as if it were already in wei of the source token. The 0x API expects `sellAmount` in the **source token's smallest unit**, which for WETH is 18 decimals.

For LONG orders (buy base with USDC), src_token = USDC, so this is correct.
For SHORT orders (sell base for USDC), src_token = the base token (e.g., WETH), and `amount_wei` should be `amount_to_wei(quantity, src_decimals)` — NOT `amount_to_wei(entry_price * quantity, src_decimals)`.

**Impact:** SHORT orders will encode the wrong sell amount by a factor of `entry_price`. A SHORT of 0.01 WETH at $2500 would encode `amount_to_wei(25.0, 18)` = 25 WETH (~$62,500) instead of `amount_to_wei(0.01, 18)` = 0.01 WETH (~$25).

**Recommendation:**
- For LONG: `amount_wei = amount_to_wei(usdc_amount, 6)` where `usdc_amount = entry_price * quantity`
- For SHORT: `amount_wei = amount_to_wei(quantity, src_decimals)` — quantity IS the token amount

---

## MEDIUM FINDINGS

### FINDING-10: `normalize_llm_json` Is Incomplete

**Severity:** MEDIUM
**Location:** `src/agent/decision_parser.rs` lines 269-288

**Description:**
The `normalize_llm_json` function handles common casing variations (`"BUY"` → `"Buy"`, `"LONG"` → `"Long"`, etc.) via string replacement. However, it only handles one level of whitespace variation:
- `"action": "BUY"` (with space) ✓
- `"action":"BUY"` (no space) ✓
- `"action" : "BUY"` (space before colon only) ✗
- `"action" : "buy"` (mixed) ✗

If MiMo v2.5 Pro returns JSON with non-standard whitespace, the normalization will fail and the strict parse will fall through to the repair passes. The repair passes may produce a valid but incorrect decision.

**Impact:** Occasional parse failures leading to missed trades or wrong trades.

**Recommendation:** Use a regex or a proper JSON key normalization (case-insensitive field matching) instead of string replacement.

---

### FINDING-11: Default Values in `partial_extract` Mask Errors

**Severity:** MEDIUM
**Location:** `src/agent/decision_parser.rs` lines 379-443

**Description:**
The `partial_extract` function fills in defaults for every field:
- `entry_price`: 0.0
- `stop_loss`: 0.0
- `confidence`: 0.5
- `side`: Side::Long
- `pair`: "BTC/USD"

If the AI returns a JSON with missing or malformed fields, `partial_extract` silently fills in dangerously wrong defaults:
- `entry_price = 0.0` bypasses the entry_price > 0.0 validation (line 158-160) only because the parse succeeds.
- `side = Side::Long` for what was meant to be a SHORT.
- `confidence = 0.5` for what was 0.2 confidence.

A malformed AI response that parses successfully will execute with hallucinated-but-defaulted parameters.

**Impact:** Wrong trades executed with "safe-looking" defaults.

**Recommendation:** Log a prominent warning when partial_extract is used. Reject entries with entry_price = 0.0 for Buy/Sell actions.

---

### FINDING-12: No Validation of AI Decision Against USDC Balance

**Severity:** MEDIUM
**Location:** `src/engine.rs` lines 1376-1393

**Description:**
When the AI decides to BUY, the engine checks:
1. Circuit breaker (paper account — not on-chain state)
2. Price tolerance (entry vs current)
3. Position sizing (based on paper account balance)

But it does **not** check whether the wallet actually has enough USDC on-chain to execute the swap. The `sync_balance()` is called at startup and periodically, but there's a race: the balance could be $50 at sync time but $0 at execution time (if another process spends it), or vice versa.

The `DexTrader::place_order()` method DOES check `self.balance <= 0.0` (line 896-899), but `self.balance` is the **tracked** balance, not the actual on-chain balance.

**Impact:** AI decides to spend $50 USDC, but wallet only has $30. The tx will revert (or the position will be recorded as open with no actual tokens received).

**Recommendation:** Call `ex.sync_balance()` immediately before `ex.place_order()` in the main loop, or use a semaphore to ensure no other code path modifies the tracked balance between sync and execution.

---

### FINDING-13: `resolve_pair` Returns Wrong Types for Some Config Pairs

**Severity:** MEDIUM
**Location:** `src/execution/dex/mod.rs` lines 431-488

**Description:**
The `resolve_pair` function splits on `/` and uppercases both parts. It maps `USD` → `USDC`. But the config pairs are like `"BTC/USD"`, `"ETH/USD"`, etc. When `resolve_pair("BTC/USD", Long)` is called, it returns `(USDC, BTC)` — correct. But what about `"USDC/USD"`? That would resolve to `(USDC, USDC)` — a swap from USDC to USDC with no economic purpose.

The engine pre-filters stablecoins in the main loop (lines 861-870), so "USDC/USD" won't reach `resolve_pair`. But "USDT/USD" would become `(USDC, USDT)` — a valid swap but probably unintended.

**Impact:** Low — mitigated by the stablecoin pre-filter. But if the filter is ever removed or bypassed, weird pairs could be traded.

**Recommendation:** Add a check in `resolve_pair`: if base == quote (e.g., USDC/USDC), return an error.

---

### FINDING-14: PaperTrader and DexTrader Position Desynchronization

**Severity:** MEDIUM
**Location:** `src/engine.rs` lines 751-795, 1232-1325

**Description:**
The engine maintains TWO position trackers:
1. `PaperTrader` (always active, used for circuit breaker + metrics)
2. `DexTrader` (active in live mode, used for actual execution)

When a position is opened via DexTrader, `paper.place_order()` is NOT called. But the circuit breaker check uses `paper.account()`. This means:
- Paper account shows balance unchanged after a real trade.
- Circuit breaker doesn't see the real positions.
- The `executor_position_map` HashMap (line 752) maps PaperTrader IDs to DexTrader IDs, but it's never populated on open (only referenced in close logic).

**Impact:** Circuit breaker is blind to real positions. The phantom position detection at startup (lines 385-395) mitigates this at restart, but during a session, the circuit breaker can't accurately assess portfolio heat.

**Recommendation:** After `ex.place_order()`, also call `paper.place_order()` with the same parameters to keep both in sync. Or use the executor as the single source of truth.

---

## LOW FINDINGS

### FINDING-15: Duplicate `spender` Extraction in `compute_struct_hash`

**Severity:** LOW
**Location:** `src/execution/dex/zero_x.rs` line 336

**Code:**
```rust
let spender = message.get("spender").and_then(|v| v.as_str()).unwrap_or("");
```

This duplicates the same extraction at line 306. Not a bug, just redundant code.

---

### FINDING-16: `parse_timeframe` and `parse_timeframe_minutes` Are Duplicated

**Severity:** LOW
**Location:** `src/engine.rs` lines 39-61

Two functions do the same thing with different return types. Worth consolidating.

---

### FINDING-17: `APE` Address Shows as `0x7f9FBf9bDd1F0e6E2c2c2c2c2c2c2c2c2c2c2c2c`

**Severity:** LOW (logged separately from Finding-04 for emphasis)

If this is intentionally a sentinel/placeholder to detect unfilled addresses, it should be documented. If it's accidental, it needs to be fixed immediately — it's the kind of thing that looks like a test artifact in a production system.

---

## SUMMARY TABLE

| ID | Severity | Finding | Impact |
|----|----------|---------|--------|
| F-01 | CRITICAL | No ERC-20 `approve()` for Permit2 | Every swap reverts |
| F-02 | CRITICAL | `amount_to_wei` uses `f64` | Precision loss / silent overflow |
| F-03 | CRITICAL | Exchange Proxy address hardcoded | Opaque trust assumption |
| F-04 | HIGH | APE address is fake/placeholder | Wrong-contract tx or loss of funds |
| F-05 | HIGH | AUSDT decimals wrong (18 vs 6) | 10^12× wrong amount |
| F-06 | HIGH | `eth_call` dry-run ≠ real tx state | False OK, real revert |
| F-07 | HIGH | Retry queue returns value never used | Failed swaps silently dropped |
| F-08 | HIGH | Close skips fee deduction | Balance drift |
| F-09 | HIGH | SHORT order amount_wei wrong formula | Sell amount × entry_price too large |
| F-10 | MEDIUM | JSON normalization incomplete | Parse failures |
| F-11 | MEDIUM | partial_extract default masks errors | Wrong parameters executed silently |
| F-12 | MEDIUM | No on-chain balance check before trade | Insufficient funds revert |
| F-13 | MEDIUM | USDC/USDC pair possible | Self-swap (no-op) |
| F-14 | MEDIUM | PaperTrader/DexTrader desync | Circuit breaker blind to real positions |
| F-15 | LOW | Duplicate spender extraction | Redundant code |
| F-16 | LOW | Duplicate timeframe parsers | Consolidation opportunity |
| F-17 | LOW | APE address appears intentional | Document or fix |

---

## AUDIT METHODOLOGY

1. Read full README for architecture overview
2. Read all 4 DEX module files (mod.rs, trader.rs, zero_x.rs, inch.rs) — line by line
3. Read execution engine trait, decision parser, error types
4. Read main engine loop (3833 lines) — focusing on execution path
5. Traced data flow: AI decision → parse → execute_swap → sign_and_send → broadcast
6. Verified Permit2 signature format against 0x docs (correct: `calldata || sig_len || sig`)
7. Checked token database for obvious errors (fake addresses, wrong decimals)
8. Identified missing steps in the 0x Permit2 flow (ERC-20 approval)

**Not reviewed in this session:** Kraken CEX backend, memory module, insight aggregator, vault integration, backtesting engine, training pipeline, TUI, REST API server. These are out of scope for the "why no TX succeeded" question.

---

*End of audit. 17 findings. 3 Critical. 6 High. 5 Medium. 3 Low.*
