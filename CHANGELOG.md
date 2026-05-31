# Changelog

All notable changes to Savant Trading will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),

## [0.4.2] — 2026-05-31

### Added

- **Persistent memory system** — 4-tier architecture based on Gemini Deep Research (40 citations)
  - Episodic capture: SQLite WAL (6 tables, 7 indices), every decision stored with full market context
  - 6th prompt layer: Dynamic Memory Context injected into AI prompt (win rates, recent analogs, CUSUM alerts, operator rules)
  - Brier Score calibration: confidence penalty calculation from trade history
  - CUSUM control chart: edge decay detection per pair, persisted to SQLite
  - Experience Replay: generates lessons from HIGH conviction losses on startup
  - Operator rules: loaded from vault Lessons/ directory, injected as "OPERATOR RULES (override all AI reasoning)"
  - Progressive confidence: 1-25 trades=LOW, 26-50=MEDIUM, 50+=HIGH
  - TUI Memory panel: Brier Score, confidence cap, CUSUM status per pair, replay lesson count

- **Sandbox & stress testing system** — 4-phase "trading dojo" based on Gemini Deep Research (50 citations)
  - GARCH(1,1) OHLCV generator with configurable trend, volatility regime, market events
  - Order book simulator with bid/ask depth, imbalance calculation, slippage simulation
  - 50 curated scenarios across 11 categories (Trend Bull/Bear, Range, Volatility, Catalyst, Microstructure, Session, Correlation, Sentiment, On-Chain, Edge Cases)
  - 3-tier grading rubric: binary compliance, R:R scoring, reasoning quality
  - GEPA-style feedback loop: failure analysis, SOUL.md mutation proposals
  - Report card generator with category breakdown and critical failures
  - Vault/Sandbox/ integration for report output

- **SOUL.md persona** — 560-line enterprise trading identity
  - 12 sections: Identity, Creed, Cognitive Style, Communication, Emotional Architecture, Crypto Philosophy, Risk Management, Decision Framework, Operational Constraints, Operator Relationship, Technical Values, Identity Invariants
  - Resolves all knowledge base contradictions (circuit breakers, R:R, position sizing)
  - 10-point pre-trade checklist, 8 identity invariants, quick reference card

- **Knowledge base expansion** — 141 → 254 units, 11 → 22 JSON files
  - 11 new files: on-chain, risk math, derivatives, Wyckoff, macro, DeFi, backtesting, execution engineering, prop firms, psychology, compliance
  - 7 new MarketCondition variants

- **Deep research documents** — Memory system design (40 citations), Sandbox design (50 citations), SOUL design, 155 research questions

### Changed

- **All dead code wired** — EventBus, VaultWriter, VaultWatcher, StopLossCalculator, OrderBookManager, format_for_context, fetch_funding_multi
- **API + engine merged** — API spawns as background task alongside engine
- **Parallel AI evaluation** — all pairs evaluated simultaneously via JoinSet
- **Crypto-native sessions** — removed stock market "off-hours", all sessions tradeable
- **Decision parser hardened** — normalizes UPPERCASE/empty action/side fields

### Fixed

- Parse crash on markdown-wrapped LLM responses
- Drawdown kill switch was non-functional (update_equity never called)
- Paper trader could open unlimited positions (entry cost not deducted)
- Daily PnL cumulative from engine start (now resets at midnight UTC)
- Rate limiter permanent lockout (now resets every second)
- .gitignore was ignoring src/data/ (anchored to repo root)
- RSS UTF-8 crash on Bulgarian text (floor_char_boundary)
- .env never loaded (added dotenvy)

### Tests

- 112 total tests (was 13)
- Sandbox: 28 tests (generator, grader, harness, scenarios, feedback, report)
- Memory: 9 tests (calibration, cusum)
- Indicators: 13 tests
- Risk: 13 tests
- Paper: 6 tests
- Insight: 10 tests
- API: 4 tests
- Strategy: 4 tests
- Vault: 3 tests
- Agent: 7 tests

## [0.4.1] — 2026-05-30

### Added

- **On-chain analytics** — Live MVRV, SOPR, NVT from CoinMetrics (CoinGecko fallback). On-chain conditions (`MvrvExtreme`, `SoprReset`) injected into knowledge selection.
- **Unit tests** — 73 total tests across 11 modules (was 13). API, insight, vault, indicators, circuit breaker, position, regime, metrics, stop loss, onchain, websocket.
- **Kraken WebSocket v2 client** — `connect()` with auto-reconnection, `parse_message()` for ticker/book/trade channels.
- **Backtesting engine** — Candle replay via `evaluate_sync()`, Sharpe/drawdown/profit factor metrics, walk-forward optimization with cumulative balance.
- **TUI dashboard** — Ratatui 0.30 real-time terminal UI. Snapshot-based rendering (no `block_on` deadlock). Portfolio, positions, decisions, insight panels.
- **Knowledge base expansion** — 141 → 254 units, 11 → 22 JSON files. Added: on-chain, risk math, derivatives, Wyckoff, macro, DeFi, backtesting, execution engineering, prop firms, psychology, compliance.
- **7 new MarketCondition variants** — `LiquidityExpansion`, `LiquidityContraction`, `MvrvExtreme`, `SoprReset`, `OIDivergence`, `WyckoffSpring`, `DeltaDivergence`.
- **REST API** — All 13 endpoints return real engine state via `SharedEngineData`. Rate limiter (sliding window). Knowledge by topic endpoint (`/api/knowledge/:topic`).
- **Production safety** — Graceful shutdown (ctrl_c saves state). Block file mechanism (`savant.blocked`). State persistence (`data/paper_state.json`).
- **Scale-out execution** — TP1 → 50% close + break-even stop, TP2 → 60% of remainder, TP3 → full close.
- **Structure stop validation** — AI-proposed stops validated against 3x ATR bounds. `structure_stop()` fallback.
- **Session multiplier wired** — `position_size_multiplier()` applied to both AI and fallback paths. PreMarket session (5-7 AM EST, 0.7x).
- **Configurable volume profile** — `volume_profile_with_pct()` accepts `value_area_pct` parameter.
- **Deep research documents** — `docs/DEEP-RESEARCH-QUESTIONS.md`, `docs/Crypto Trading Knowledge Expansion Roadmap.md`, `docs/KNOWLEDGE-EXPANSION-EXECUTION.md`.

### Changed

- **All dead code wired** — EventBus, VaultWriter, VaultWatcher, StopLossCalculator, OrderBookManager, `format_for_context`, `fetch_funding_multi`.
- **API + engine merged** — API spawns as background task alongside engine. Both share `SharedEngineData`.
- **Insight aggregator** — `refresh_multi()` batches all pairs in single funding API call.
- **WebSocket refactored** — Removed unused `KrakenWebSocket` struct. Kept `connect()`, `parse_message()`, `create_channel()`.
- **Vault writer guards** — `project_trade()`, `project_decision()`, `project_portfolio()` check `config.enabled`.
- **Ratatui bumped** — 0.29 → 0.30 (fixes `lru` GHSA-rhfx-m35p-ff5j vulnerability).

### Fixed

- `parse_wrapped_json` test — Hold decisions with `entry_price: 0.0` no longer rejected.
- `vault/writer.rs` — Raw string `#` parsing error (Rust 2021 reserved prefix).
- `update_equity()` now called — Drawdown kill switch was non-functional.
- Entry cost + fee deducted from balance — Paper trader could open unlimited positions.
- `daily_pnl` resets at midnight UTC — Daily loss limit was cumulative.
- `Display` for `Side` — Logs show `LONG`/`SHORT` instead of `0`/`1`.
- Rate limiter resets every second — Was permanent lockout after 1000 requests.
- Duplicate `parse_timeframe` removed — Engine and main had different return types.
- RSS UTF-8 crash — `floor_char_boundary` for Bulgarian text.
- `.env` loading — Added `dotenvy::dotenv().ok()` to `main()`.

### Hygiene

- Moved misplaced files (yt.md, SESSION-SUMMARY.md, overview.jpg).
- Deleted Claude Code leftovers (dashboard/AGENTS.md, CLAUDE.md).
- Added `savant-vault/` to `.gitignore`.
- Removed duplicate knowledge files from `src/agent/knowledge/`.
- Removed GitHub Actions CI (billing issue).
- MSRV set to 1.91. Added `uuid` crate. Removed unused `tower` dep.
- Version aligned: Cargo.toml, README, protocol.config.yaml, CHANGELOG.

## [0.4.0] — 2026-05-30

### Added in 0.4.0

- **Glass House — Obsidian vault integration** — Bidirectional vault sync for transparent trading state
  - `VaultWriter` — Projects engine state into Obsidian vault as structured markdown
    - Trades/ — Daily trade logs with entry/exit/PnL
    - Decisions/ — AI decision logs with reasoning
    - Portfolio/ — Balance history, equity curve
    - Knowledge/ — Knowledge unit index
    - INDEX.md — Master index with wiki-links
  - `VaultWatcher` — Monitors vault for user edits
    - Lessons/ — Editable ground truth ingested by engine
    - Injection defense — Scans for 14 prompt injection patterns
    - Invisible unicode detection
  - `VaultConfig` — Configurable vault path, sync interval, max files
  - `.obsidian/appearance.json` — Dark theme, cyan accent (#00d5ff)

## [0.3.1] — 2026-05-30

### Added in 0.3.1

- **Session-aware trading** — Engine knows what trading session is active
  - Asian (7 PM - 2 AM EST): low volume, ranging, 0.5x position size
  - London (2 AM - 5 AM EST): high volume, reversals, 1.0x
  - New York (7 AM - 10 AM EST): highest volume, continuations, 1.0x
  - London/NY Overlap (8 AM - 10 AM EST): peak volume, 1.2x
  - Off-hours: 0.3x position size, avoid new entries
  - Kill zone detection: London, NY, Overlap are high-probability windows
  - Session context injected into AI prompt

### Changed in 0.3.1

- `core/session.rs` — New module: session detection, behavior, position size multiplier
- `context_builder.rs` — Session info added to AI user message

## [0.3.0] — 2026-05-30

### Added in 0.3.0

- **Dashboard UI** — Next.js 16 + React 19 + TypeScript 5, matching Savant design system
  - 11 pages: Overview, Transactions, AI Decisions, Portfolio, Insight, Knowledge, Risk, Session, Settings
  - CSS Modules with Savant design tokens (`#00d5ff` accent, glass morphism, JetBrains Mono + Inter)
  - 3-column layout: sidebar (280px), main content, right panel
  - Ambient background with radial gradients
  - Custom scrollbars, glass panels, accent glow effects
  - All pages build successfully (`npm run build`)

## [0.2.1] — 2026-05-30

### Added in 0.2.1

- **ECHO Protocol in system prompt** — Trading rules from ECHO.md + transcript-derived rules embedded in AI prompt
  - Sell into strength (80% at 10-20% gain)
  - 3 consecutive losses = stop for the day
  - Don't marry positions
  - Four-factor performance model
  - Session awareness (kill zones)
  - Compound strategy (risk profits on directional days)
- **Trade history in AI context** — Last 10 trades + performance summary injected into prompt
  - Win rate, average win/loss, profit factor
  - Individual trade details with entry/exit/PnL
  - Cold start: omit if no trades yet
- **Multi-pair insight refresh** — Insight now refreshes for ALL configured pairs, not just the first
- **Structured logging** — AI context logged with prompt chars, knowledge budget, pair, regime

### Changed in 0.2.1

- `FullContext` struct now includes `recent_trades: Option<&[TradeRecord]>`
- System prompt includes ECHO rules alongside strategy knowledge
- Knowledge section header: "From 11 Curated Transcripts"

## [0.2.0] — 2026-05-30

### Added in 0.2.0

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

### Changed in 0.2.0

- Version bumped to 0.2.0
- CLI now supports: `savant` (trade), `savant --dry-run`, `savant --api`, `savant report`, `savant --help`
- `context_builder.rs` — `determine_conditions` and `build_user_message` now have static versions for dry-run use
- `axum = "0.8"` and `tower-http = "0.6"` added for REST API

### Dependencies in 0.2.0

| Dependency  | Status | Notes            |
|-------------|--------|------------------|
| `axum`      | NEW    | REST API server  |
| `tower-http`| NEW    | CORS support     |

## [0.1.2] — 2026-05-30

### Added in 0.1.2

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

### Changed in 0.1.2

- All insight modules now use **free APIs only** — no paid API keys required
- CoinGlass replaced with Kraken Futures (free, no geo-block)
- All insight sources enabled by default in config
- 15 RSS feeds (up from 0)

### Removed in 0.1.2

- CoinGlass API dependency (not free)
- CryptoQuant API dependency (not free — blockchain.info used instead)
- API key fields from InsightConfig (all sources free now)

### Fixed in 0.1.2

- FID-015 Perfection Loop: `quick-xml` dependency validated, all API endpoints verified
- Cargo.toml: `quick-xml = "0.37"` added for RSS/XML parsing

## [0.1.1] — 2026-05-30

### Changed in 0.1.1

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

### Fixed in 0.1.1

- FID-001, FID-002, FID-003 status corrected from "analyzed" to "fixed"
- FID-007 status set to "superseded" (absorbed into FID-013)
- All 13 FIDs have proper Perfection Loop audits

### Verified in 0.1.1

- Fear & Greed API: `GET https://api.alternative.me/fng/?limit=1` → `23 (Extreme Fear)` — free, no key
- CoinGecko API: `GET https://api.coingecko.com/api/v3/global` → `BTC.D: 57.44%` — free, no key
- Build: `cargo clippy -- -D warnings` zero warnings, `cargo fmt --check` clean

## [0.1.0] — 2026-05-30

### Added in 0.1.0

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
