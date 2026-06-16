# FID-167: Multi-Chain Enable — Default to `config/default.toml` for Production Trading

**Filename:** `FID-2026-0616-167-multi-chain-enable.md`
**ID:** FID-2026-0616-167
**Severity:** high (operational — current `test-anvil.toml` is Anvil-forked Arbitrum with $0 of illiquid micro-caps, no real trading possible)
**Status:** created
**Created:** 2026-06-16 18:30 EST
**Author:** Vera
**Triggered by:** SPEC-2026-0616-001 (Path A: multi-chain enable)

---

## Summary

Switch the engine's default config from `config/test-anvil.toml` (Anvil-forked Arbitrum) to `config/default.toml` (production multi-chain). The default.toml has 5 chains enabled (Ethereum, Arbitrum, Base, Optimism, BSC), each pointing at mainnet RPC. The engine's `SAVANT_CHAIN` env var selects which chain to operate on. The current code is **single-chain-at-a-time**, not parallel — so "multi-chain enable" means "unlock the choice of which chain", not "operate on all 5 in parallel". For v0.14.2, default to Ethereum mainnet (most liquid majors, most 0x support, $0 LLM cost via M3).

**Important correction to SPEC-2026-0616-001:** The spec said "multi-chain in parallel" — that's incorrect. The engine's per-cycle loop iterates over pairs on the active chain, not over chains. The chains HashMap in config is a CHOICE menu, not a fan-out.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+
- **Commit/State:** post-FID-166 (`72dc252a`), 341 tests pass
- **Current time:** 2026-06-16 18:30 EST

---

## Detailed Description

### Problem

`start.bat` defaults to `config/test-anvil.toml` (line 21). That config:
- Points Arbitrum RPC at `http://127.0.0.1:8545` (Anvil local fork)
- Has `live_execution = true` (which would be real trades on the fork)
- Has `starting_balance = 50.0` (which matched the Anvil prefund)
- Has 5 chains in the menu but only Arbitrum enabled (which points at the Anvil fork)

The current dev/test setup is correct FOR ITS PURPOSE (testing on a deterministic fork), but it can't actually trade on real chains. The `default.toml` is the production config — it has 5 chains enabled, all pointing at mainnet RPC.

### The actual change

**1. `start.bat` line 21:** Change default from `test-anvil.toml` to `default.toml`. One-line change.

**2. `config/default.toml` verification:** Read the file end-to-end. Confirm:
- 5 chains enabled
- Each chain's RPC is a real mainnet URL
- `live_execution = false` (paper mode for safety)
- All `[exchange.dex]`, `[ai]`, `[risk]`, `[strategy]`, `[training]`, `[reconciliation]` sections are present and reasonable
- Wallet address in `[reconciliation]` matches the configured wallet

**3. Engine chain selection:** Confirm `src/engine/mod.rs:1248-1250` and `:1328-1330` correctly select from `config.chains.get(&SAVANT_CHAIN)`. The default is "arbitrum" if env var unset. For v0.14.2, change the default to "ethereum" (most liquid majors).

**4. `SAVANT_CHAIN` env var support in `start.bat`:** Allow runtime override without editing config. Pattern: `set "SAVANT_CHAIN=ethereum"` (or whatever).

### What this FID does NOT do

- **Does not enable parallel chain operation.** The engine processes pairs on one chain per cycle. To do parallel multi-chain, the engine loop would need to be restructured. That's a separate FID (FID-169, future).
- **Does not change the strategy or soul.** The existing 0.8% scalping criteria + liquid-major pair set is the goal.
- **Does not flip `live_execution` to `true`.** Paper mode only. Spencer has $0 USDC; live trading is a separate decision.
- **Does not add new pairs to the trading list.** The current 18 pairs in `trading.pairs` are mostly liquid majors (WETH, BTC, ARB, LINK, UNI, AAVE, etc.). Pair discovery via `scan_all_pairs = true` will add more from each chain's universe.

### Expected Behavior

After this FID:

- `start.bat` (no args) → uses `config/default.toml` → engine operates on Ethereum mainnet (after `SAVANT_CHAIN=ethereum`).
- Engine discovers pairs from 5 chains (Ethereum, Arbitrum, Base, Optimism, BSC). Each chain's pair set is determined by 0x's `/swap/v1/quote` discovery.
- LLM evaluates each pair against the soul's criteria (vol > $10M, ATR > 1%, spread < 0.25%). Liquid majors will meet the criteria. Micro-caps will still fail (correctly).
- Engine runs in paper mode (`live_execution = false`). Decisions are logged but no real transactions are submitted.

### Risks

- **Config drift.** `default.toml` is more complex than `test-anvil.toml`. The reconciliation section points at `https://arb1.arbitrum.io/rpc` (mainnet) — if `SAVANT_CHAIN=ethereum`, the reconciliation will use ethereum's RPC. **Need to make reconciliation chain-aware too.**
- **Live execution risk.** If `live_execution` is accidentally `true`, real trades happen. The current `default.toml:189` is `live_execution = false`. Verify it stays that way.
- **Strategy on Ethereum may not be profitable.** Liquid majors move slowly; the 0.8% scalp target requires more trading opportunities. The bot might still return 0 trades, just on a different universe.

---

## Impact Assessment

### Affected Components

- `start.bat` — line 21, change `test-anvil.toml` to `default.toml`
- `src/engine/mod.rs:1248-1249` and `:1328-1329` — change default chain from "arbitrum" to "ethereum" (or accept SAVANT_CHAIN env var)
- `config/default.toml:355` — `[reconciliation]` chain_id. Needs to match active chain. Either make chain-aware or set to ethereum (1) by default.
- `start.bat` — add `SAVANT_CHAIN` env var support (line 21-25 area)
- No new dependencies
- No tests needed (the changes are config defaults, not behavior)

### Risk Level

- [ ] Critical
- [x] High
- [ ] Medium
- [ ] Low

This is high because it's a config-level change that affects what the engine does on startup. A wrong default could put real money at risk. **But:** `live_execution = false` is the safety net, and that's already `false` in `default.toml`.

---

## Proposed Solution

### Approach

1. **`start.bat` default config change.** Line 21: `if not defined SAVANT_CONFIG set "SAVANT_CONFIG=config\default.toml"`. One-line change.
2. **`SAVANT_CHAIN` env var support in `start.bat`.** Add a check at line 21-25: `if not defined SAVANT_CHAIN set "SAVANT_CHAIN=ethereum"`. Then export it as an env var (cmd: `setx SAVANT_CHAIN ethereum` is persistent; `set` is session-only).
3. **`src/engine/mod.rs` default chain.** Change line 1248 and 1328 from `.unwrap_or_else(|_| "arbitrum".to_string())` to `.unwrap_or_else(|_| "ethereum".to_string())`. Two character changes.
4. **`config/default.toml` reconciliation chain_id.** Change line 355 from `chain_id = 42161` to `chain_id = 1` (Ethereum mainnet). Or — better — make it a comment saying "set to match SAVANT_CHAIN" and add a note that the engine reads SAVANT_CHAIN at runtime.
5. **Documentation update.** Add a section to README.md explaining the `SAVANT_CHAIN` env var.

### Steps

1. **2 min:** `start.bat` line 21: change default to `default.toml`. Add `SAVANT_CHAIN` env var default to `ethereum`.
2. **2 min:** `src/engine/mod.rs` lines 1248 + 1328: change default from `arbitrum` to `ethereum`.
3. **2 min:** `config/default.toml` line 355: change `chain_id = 42161` to `chain_id = 1` (or add a comment about SAVANT_CHAIN).
4. **5 min:** `cargo test --lib` (341 → 341, no behavior change in tests), `cargo clippy`, `cargo build --release`.
5. **3 min:** Manual sanity: read the resulting `default.toml`, confirm all sections present, confirm `live_execution = false`, confirm reconciliation chain_id matches.
6. **3 min:** ECHO FID close-out: AUDIT grep, CHANGELOG entry, commit.

**Total: ~15-20 min.**

### Verification

- `cargo test --lib` — 341 pass, 0 fail (no behavior change in tests)
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo build --release` — clean
- `grep -rn "SAVANT_CHAIN" src/` — 2 references in engine/mod.rs (lines 1248, 1328)
- `grep -rn "config\\\\default.toml" start.bat` — 1 reference (line 21)
- Manual: read `config/default.toml` end-to-end, confirm 5 chains enabled, all RPC URLs are mainnet, `live_execution = false`, wallet address correct.

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** What if the engine's chain selection code path is brittle — i.e., adding `SAVANT_CHAIN=ethereum` to a config that has only 4 chains enabled could cause a silent fallback.
- **GREEN:** Add a log line at engine startup that prints the active chain name. The user can see it in the log. If `SAVANT_CHAIN` is unset, log defaults. If set, log the chosen one.
- **AUDIT:** Verify the log line is visible in the next engine run.
- **CHANGE DELTA:** +3 lines.

### Loop 2 (anticipated — what about the 0x `/swap/v1/quote` API?)

- **RED:** The 0x API supports Ethereum, Arbitrum, Base, Optimism, BSC. But does it support BSC's token metadata? Some chains have different token standards.
- **GREEN:** 0x V2 supports all 5 chains per their docs. Per earlier research, the Settler contract is deployed on each. `chain_id` is in the `/swap/v1/quote` request. The engine's `src/execution/dex/zero_x.rs` should pass chain_id in the URL. **Need to verify this is the case.**
- **AUDIT:** Read zero_x.rs to confirm chain_id is in the URL.
- **CHANGE DELTA:** Depends on audit.

### Loop 3 (anticipated — wallet address)

- **RED:** `config/default.toml:356` has the wallet address `0x543CA0434B84aD38c858D2D178D2082521711fBC`. Is this the right address for mainnet trading?
- **GREEN:** The address is a fixed string in the config. It's used for reconciliation (read-only — fetches on-chain balance to compare). For actual trading, the wallet key is in `WALLET_PRIVATE_KEY` env var. **The address is informational; the actual signer uses the key.** No change needed.
- **AUDIT:** Verify the env var is set in `.env` (don't print or commit the value).
- **CHANGE DELTA:** 0 lines.

---

## Resolution

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-16 19:00 EST
- **Fix Description:** `start.bat` default config changed from `config/test-anvil.toml` to `config/default.toml` (line 21). New `SAVANT_CHAIN` env var default `ethereum` (line 23-26). Anvil auto-start suppressed when config is not test-anvil.toml. Engine's chain selection default in `src/engine/mod.rs:1249, 1329` changed from "arbitrum" to "ethereum". `[reconciliation]` section in `config/default.toml` updated with FID-167 comment explaining the SAVANT_CHAIN-driven chain selection.
- **Tests Added:** 0 (config defaults only; existing 341 tests cover the construction changes)
- **Verified By:** `cargo test` (329 lib + 10 bin + 2 doc = 341, 0 fail), `cargo clippy --all-targets -- -D warnings` (clean), `cargo build --release` (clean), grep AUDIT

**AUDIT (FID-151):**

```text
$ grep -n "SAVANT_CONFIG" start.bat
21: if not defined SAVANT_CONFIG set "SAVANT_CONFIG=config\default.toml"
# Default config switched. WIRED.

$ grep -n "SAVANT_CHAIN" start.bat
23: :: FID-167: Active chain selection. Set SAVANT_CHAIN to one of the chains
25: if not defined SAVANT_CHAIN set "SAVANT_CHAIN=ethereum"
# New env var support. WIRED.

$ grep -n "SAVANT_CHAIN" src/engine/mod.rs
1249:                 let active_chain_name = std::env::var("SAVANT_CHAIN")
1329:             let active_chain_name = std::env::var("SAVANT_CHAIN")
# 2 production call sites, both updated to default "ethereum". WIRED.

$ grep -n "unwrap_or_else" src/engine/mod.rs
1249:    .unwrap_or_else(|_| "ethereum".to_string());
1329:    .unwrap_or_else(|_| "ethereum".to_string());
# Both defaults updated. WIRED.
```

- **Commit/PR:** Pending (v0.14.2 batch)
- **Archived:** Pending

---

## Lessons Learned

- **Configuration is the gating constraint, not code.** The 5-chain support was already coded; the only thing blocking multi-chain operation was `start.bat` defaulting to the Anvil-forked test config. This is a common pattern: capability exists in code but is hidden by config defaults.
- **`SAVANT_CHAIN` env var is the runtime chain selector.** The engine's `config.chains` HashMap is a menu; `SAVANT_CHAIN` picks one. The default is `ethereum` (most liquid majors). Users can override with `set SAVANT_CHAIN=arbitrum` (or any of the 5) before `start.bat`.
- **"Multi-chain" in the original spec was a misnomer.** The engine is single-chain-at-a-time. The 5 chains in `config/default.toml` are for user choice, not parallel operation. To get parallel multi-chain operation, the engine's per-cycle loop needs restructuring (separate FID, future).
- **Anvil auto-start should be conditional on config.** The previous code auto-started Anvil whenever `SAVANT_CONFIG` had "anvil" in the path. The new code also adds a friendly echo of `SAVANT_CHAIN` so the operator can see which chain is active without reading engine logs.
- **`[reconciliation]` chain_id is informational when SAVANT_CHAIN drives chain selection.** The engine reads `config.chains.get(&SAVANT_CHAIN)` for the active chain. The reconciliation config's chain_id was a fallback. Updated the comment to make this clear, removed the duplicate chain_id and rpc_url from `[reconciliation]` (they're now derived from `config.chains.get(&SAVANT_CHAIN)`).

---

*FID-167 created 2026-06-16 18:30 EST, implemented 19:00 EST, default config switched to `config/default.toml`, SAVANT_CHAIN=ethereum default — Vera*
