# Changelog

All notable changes to Savant Trading will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
