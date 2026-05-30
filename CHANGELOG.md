# Changelog

All notable changes to Savant Trading will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-05-30

### Added

- **Dry-run test mode** — `savant --dry-run` runs one AI decision cycle and prints full pipeline output
  - Market data (candles, indicators, regime)
  - Insight (Fear & Greed, BTC Dominance, funding rates, RSS)
  - Knowledge selection (conditions → matched units)
  - System prompt (composed with knowledge injection)
  - LLM response (raw JSON from mimo v2.5 pro)
  - Parsed decision (action, entry, stop, targets, confidence, reasoning)
- **REST API server** — `savant --api` starts axum server on localhost:8080
  - 13 endpoints: status, config, portfolio, positions, trades, decisions, insight, knowledge, risk, session, engine control
  - All responses use `{data, error, timestamp}` envelope
  - Localhost-only binding (no external access)
- **External knowledge loading** — `knowledge/` directory at project root
  - Engine loads from `knowledge/` first, falls back to embedded
  - 11 JSON files with 141 knowledge units
  - Editable without recompiling
- **Help command** — `savant --help` shows usage

### Changed

- Version bumped to 0.2.0
- CLI now supports: `savant` (trade), `savant --dry-run`, `savant --api`, `savant report`, `savant --help`
- `context_builder.rs` — `determine_conditions` and `build_user_message` now have static versions for dry-run use
- `axum = "0.8"` and `tower-http = "0.6"` added for REST API

### Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| `axum` | NEW | REST API server |
| `tower-http` | NEW | CORS support |

## [0.1.2] — 2026-05-30

### Added

- **RSS News Pipeline** — 15 free RSS feeds parsed with `quick-xml`, scored by relevance to trading pairs
  - Crypto-native: Cointelegraph, CoinDesk, CryptoSlate, Decrypt, CryptoNews, CryptoPotato, CryptoBreaking
  - DeFi: The Defiant, SmartLiquidity
  - Institutional: Blockworks, Bitcoin Magazine, Ethereum 2.0
  - Mainstream: Yahoo Finance, CNBC (macro context)
  - Regional: KriptoNovini
- **Kraken Futures Integration** — Funding rates, open interest, mark prices from `futures.kraken.com` (free, no key)
- **Liquidation Risk Assessment** — Derived from futures data: mark/index spread, funding extremes, OI concentration
- **On-chain Data** — Block height, mempool size, 24h tx count from blockchain.info (free, no key)
- **CoinGecko Trending** — Trending coins with price changes, used as social sentiment proxy
- **API-KEYS.md** — Reference document with all endpoints, signup links, and env var names

### Changed

- All insight modules now use **free APIs only** — no paid API keys required
- CoinGlass replaced with Kraken Futures (free, no geo-block)
- All insight sources enabled by default in config
- 15 RSS feeds (up from 0)

### Removed

- CoinGlass API dependency (not free)
- CryptoQuant API dependency (not free — blockchain.info used instead)
- API key fields from InsightConfig (all sources free now)

### Fixed

- FID-015 Perfection Loop: `quick-xml` dependency validated, all API endpoints verified
- Cargo.toml: `quick-xml = "0.37"` added for RSS/XML parsing

## [0.1.1] — 2026-05-30

### Changed

- **Knowledge Base Expansion** — 88 → 141 knowledge units (+60%)
  - `ai_claude_bot.json`: 5 → 20 units (HMM math, feature engineering, walk-forward, circuit breakers, dashboard)
  - `tjr_smc.json`: 15 → 19 units (partial FVG fills, rejection candles, engulfing, trailing stop models)
  - `crypto_fcb.json`: 13 → 18 units (MACD, Bollinger Bands, Fibonacci, altcoin selection, market cycles)
  - `pradeep_ep.json`: 12 → 15 units (delayed EP, continuation EP, sector rotation)
  - `juvier_daytrading.json`: 6 → 12 units (displacement candles, breaker blocks, Asian range, London vs NY)
  - `warrior_trading.json`: 6 → 10 units (gap fill probability, short selling, morning vs afternoon)
  - `brian_jung.json`: 5 → 8 units (stablecoin strategy, ETF impact, global liquidity)
  - `ai_competition.json`: 3 → 10 units (all 15 bot strategies, risk tier analysis, evolution methodology)
  - `hybrid_scalping.json`: 4 → 8 units (multi-TF scalping, session-specific, fee optimization)
  - `cathie_wood.json`: 4 → 6 units (Tesla robotaxi, humanoid robots, demographics)
  - `fabio_amt.json`: 15 → 15 units (refined with more specific execution details)

### Fixed

- FID-001, FID-002, FID-003 status corrected from "analyzed" to "fixed"
- FID-007 status set to "superseded" (absorbed into FID-013)
- All 13 FIDs have proper Perfection Loop audits

### Verified

- Fear & Greed API: `GET https://api.alternative.me/fng/?limit=1` → `23 (Extreme Fear)` — free, no key
- CoinGecko API: `GET https://api.coingecko.com/api/v3/global` → `BTC.D: 57.44%` — free, no key
- Build: `cargo clippy -- -D warnings` zero warnings, `cargo fmt --check` clean

## [0.1.0] — 2026-05-30

### Added

- **Agent Module** — AI-powered autonomous trading brain with mimo v2.5 pro via OpenGateway
  - Knowledge injection system — 11 curated transcripts processed into discrete knowledge units
  - Modular 5-layer system prompt composer (identity, risk, strategy, knowledge, output format)
  - OpenAI-compatible LLM provider with retry and rate limiting
  - Context builder — aggregates candles, indicators, insight, positions into LLM context
  - Decision parser — extracts structured TradeDecision from LLM JSON responses
  - Orchestrator — main decision loop with 3 autonomy levels (Suggest, Confirm, Autonomous)
  - Fallback mode — rule-based strategies activate if LLM fails 3 consecutive ticks
- **Insight Module** — Live market context from external data sources
  - Fear & Greed Index (alternative.me, free)
  - BTC Dominance and total market cap (CoinGecko, free)
  - Funding rates, open interest, long/short ratio (stub — CoinGlass)
  - Liquidation clusters (stub — CoinGlass)
  - Exchange inflow/outflow (stub — CryptoQuant)
  - News and social sentiment (stub — LunarCrush)
  - Unified MarketContext aggregator with graceful failure handling
- **Core Engine** — Existing rule-based trading engine
  - Kraken REST + WebSocket integration
  - Technical indicators (EMA, SMA, RSI, ATR, ADX, VWAP, Volume Profile)
  - Paper trading simulator with realistic fees (0.26% Kraken taker) and slippage
  - Risk management: position sizing, stop-loss, break-even, circuit breakers
  - Trade journal with SQLite persistence and equity curve tracking
  - CLI with trade, report, and backtest commands
- **Knowledge Base** — 11 curated transcripts
  - Scalping (Fabio Valentina — order flow, volume profile, AMT)
  - Strategy (Pradeep Bondi — episodic pivots, execution edges)
  - Day Trading (TJR — SMC, FVG, order blocks, liquidity sweeps)
  - Crypto (Full Course — sentiment, BTC dominance, alt season)
  - Crypto (Brian Jung — macro catalysts, halving cycle)
  - Day Trading (Juvier — kill zones, session trading)
  - Day Trading (Warrior Trading — 5 Pillars, pullback entry)
  - Scalping (Hybrid — Heikin Ashi + EMA, prop firms)
  - Macro (Cathie Wood — innovation platforms, Wright's Law)
  - AI Trading (Claude Code bot — HMM regimes, circuit breakers)
  - AI Trading (Competition — natural selection, multi-agent)
- **FID System** — 13 tracked findings with Perfection Loop audits
  - 3 fixed (ECHO violations, paper persistence, fee/slippage)
  - 8 pending (trailing stops, scale-out, FVG, backtesting, WebSocket, multi-TF, tests, rate limiting)
  - 1 superseded (sentiment → absorbed into FID-013)
  - 1 new (AI Agent Brain — critical architectural upgrade)
- **ECHO Protocol** — Universal agent bootstrap with 15 laws and Perfection Loop FSM
