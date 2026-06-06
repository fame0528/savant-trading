# FID-062: Remove Kraken Execution Backend + Rename Data Pipeline

**Status:** closed
**Severity:** medium
**Created:** 2026-06-06
**Closed:** 2026-06-06
**Author:** Kilo

---

## Problem

The codebase contains a full Kraken execution backend (`KrakenTrader`, `KrakenTraderConfig`, Kraken WebSocket order book streaming, Kraken fee rates, Kraken balance sync) that is dead code. The system pivoted to DEX-only execution via 0x API on Arbitrum. Kraken has never been used for live execution.

The name "Kraken" appears 255 times across the codebase, implying CEX execution support that doesn't exist. The Kraken REST API is used as a candle data source (OHLCV), but the naming is misleading.

## Goal

1. Remove all Kraken **execution** code
2. Rename `KrakenClient` → `MarketDataClient` and `KrakenSource` → `KrakenFeed` to make it clear this is a data feed, not an execution backend
3. Update all references to remove execution-adjacent claims

## Scope

### DELETE
- `src/execution/kraken.rs` (569 lines — full KrakenTrader implementation)
- `src/execution/mod.rs` line 9: `pub mod kraken;`
- `src/execution/mod.rs` line 5: doc comment mentioning kraken
- `src/engine.rs:24` — `use KrakenTrader, KrakenTraderConfig` import
- `src/engine.rs:70-109` — `"kraken" => { ... }` match arm in `create_execution_engine()`
- `src/engine.rs:2967-2976` — Kraken balance sync block
- `src/engine.rs:307,1254` — `config.exchange.backend != "kraken"` guards (simplify to always DEX)
- `src/engine.rs:340` — comment mentioning KrakenTrader
- README lines 26, 106, 198-200 (execution claims)

### RENAME
| Old | New | File |
|-----|-----|------|
| `src/data/kraken.rs` | `src/data/market_data.rs` | filesystem |
| `src/data/sources/kraken.rs` | `src/data/sources/kraken_feed.rs` | filesystem |
| `KrakenClient` | `MarketDataClient` | src/data/market_data.rs |
| `KrakenSource` | `KrakenFeed` | src/data/sources/kraken_feed.rs |
| `pub mod kraken` | `pub mod market_data` | src/data/mod.rs |
| `pub mod kraken` | `pub mod kraken_feed` | src/data/sources/mod.rs |

### UPDATE (references to renamed types)
- `src/engine.rs` — all `KrakenClient` → `MarketDataClient`, all `KrakenSource` → `KrakenFeed`
- `src/main.rs` — all `KrakenClient` → `MarketDataClient`
- `src/data/sources/kraken_feed.rs` — internal references
- `src/core/config.rs` — remove `"kraken"` from allowed `exchange.backend` values (keep candle config)
- `src/core/console.rs` — `"kraken"` → `"kraken_feed"` or keep as display label
- `README.md` — remove execution claims, update data source references
- `protocol.config.yaml` — update description

### KEEP AS-IS
- `src/data/kraken.rs` content — just renamed, code preserved
- `src/data/sources/kraken.rs` content — just renamed, code preserved
- Kraken REST URL in config — still the data source
- Kraken WebSocket URL in config — still used for candle streaming

## Verification

- `cargo clippy -- -D warnings` passes
- `cargo test` passes
- Engine starts with `exchange.backend = "0x"`
- Startup log shows candle fetching from Kraken API (via MarketDataClient)
- `exchange.backend = "kraken"` produces config validation error
- No execution-related Kraken references remain (grep confirms)

## Risk

- **Low:** Kraken execution was never used in production
- **Low:** Renaming is mechanical — no logic changes
- **Data pipeline is safe:** MarketDataClient preserves all existing functionality
