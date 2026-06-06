# FID: GMX V2 Native Rust Execution — Replace Python Sidecar

**Filename:** `FID-2026-0606-060-gmx-native-rust.md`
**ID:** FID-2026-0606-060
**Severity:** critical
**Status:** created
**Created:** 2026-06-06 04:03
**Author:** Kilo (mimo-v2.5-pro)

---

## Summary

The Python GMX sidecar failed repeatedly (broken SDK, memory hog, web3 version conflicts). Our Rust engine already has EIP-1559 transaction signing, Arbitrum RPC, wallet management, and gas management. Building GMX execution natively in Rust using direct contract calls is the correct path.

---

## Detailed Description

### Problem

- Python `web3` library consumes 2-4GB RAM, freezing the machine
- GMX Python SDK has web3 version mismatch (built for 6.10.0, installed 7.16.0)
- GMX TypeScript SDK has broken ESM build
- Both SDKs add complexity without proportional value
- Our Rust engine already handles everything needed: signing, RPC, gas

### Solution

Build GMX V2 execution as a new module in `src/execution/dex/` alongside the existing 0x integration. Direct contract calls using our existing Arbitrum infrastructure.

### Architecture

```
src/execution/dex/gmx.rs (NEW)
    ├── GMX V2 contract ABIs (ExchangeRouter, DataStore)
    ├── Oracle price fetching (REST: arbitrum-api.gmxinfra.io)
    ├── Order creation (createIncreaseOrder, createDecreaseOrder)
    ├── Position querying (getPositions)
    └── Uses existing DexTrader signing + RPC infrastructure
```

### GMX V2 Order Flow

1. Fetch oracle prices from `https://arbitrum-api.gmxinfra.io/prices`
2. Build multicall transaction:
   - `setOraclePrice` for each token
   - `createIncreaseOrder` with market, collateral, size, leverage
3. Sign with existing wallet (EIP-1559)
4. Broadcast via existing Arbitrum RPC
5. Keepers execute the order asynchronously

### Key Contracts (Arbitrum)

| Contract | Address |
|----------|---------|
| ExchangeRouter | `0x7C68C7866A64FA2160F78EEeE18b26d8c1B7e6d1` |
| DataStore | `0xFD70de6b91282D8017aA4E741e9Ae325CAb992d8` |
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |

### Phases

1. **FID-060a:** GMX oracle price fetcher (REST API, no contract calls)
2. **FID-060b:** GMX contract ABIs + order builder
3. **FID-060c:** GMX position query + close
4. **FID-060d:** Integration with engine (swap execution backend)

---

## Perfection Loop

### Loop 1

- **RED:** —
- **GREEN:** —
- **AUDIT:** —
- **CHANGE DELTA:** —

---

## Resolution

- **Fixed By:** —
- **Fixed Date:** —
- **Fix Description:** —
- **Tests Added:** —
- **Verified By:** —
- **Commit/PR:** —
- **Archived:** —
