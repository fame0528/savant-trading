# Savant Trading — Agent Knowledge Base Reference

> **Version:** 1.1 | **Generated:** 2026-06-04 | **Status:** ACTIVE
>
> This document provides a complete inventory and reference for all data in the Savant Trading agent's knowledge base (brain). It covers structured knowledge units (JSON), source book libraries, extraction scripts, backup knowledge, and generated summaries.
>
> **Context:** Savant is an **AI-native autonomous crypto trading engine** (Kraken CEX + DEX). The knowledge base is purpose-built for crypto trading. The 168 book files in `knowledge/books/` — many covering forex, stocks, and general trading — were parsed, deduplicated, and optimized into ~6,676 structured knowledge units. Forex and stock books provide foundational knowledge (candlestick patterns, chart analysis, price action, market structure, risk management, trading psychology) that transfers directly to crypto markets. The agent's primary edge comes from crypto-native knowledge: on-chain analytics, DeFi mechanics, funding rates, whale tracking, and crypto-specific market structure.

---

## Table of Contents

1. [Knowledge Base Architecture](#1-knowledge-base-architecture)
2. [Structured Knowledge Units (JSON)](#2-structured-knowledge-units-json)
3. [Book Library](#3-book-library)
4. [Knowledge Extraction Pipeline](#4-knowledge-extraction-pipeline)
5. [Backup Knowledge Files](#5-backup-knowledge-files)
6. [Generated Summaries & Indexes](#6-generated-summaries--indexes)
7. [Topic Coverage Matrix](#7-topic-coverage-matrix)
8. [Knowledge Unit ID Ranges](#8-knowledge-unit-id-ranges)
9. [File Manifest](#9-file-manifest)

---

## 1. Knowledge Base Architecture

The knowledge base lives under `/knowledge/` and uses a multi-layered architecture:

```
knowledge/
├── books/                     # Source book files (PDF, EPUB, MOBI, AZW, TXT)
│   ├── _info.text             # Rated book inventory with Goodreads-style scores
│   ├── INVESTMENT_BOOKS_SUMMARY.md  # Synthesized summary of 150+ books
│   ├── Trading.txt            # Curated book list organized by skill level
│   └── [168 book files]       # PDFs, EPUBs, and other formats
├── extracted/                  # Plain-text extraction output (currently empty)
├── _backup/                    # Supplementary knowledge JSON files (33 files, 3,717 units)
├── knowledge_*.json            # Primary structured knowledge units (10 files, 2,959 units)
├── extract_books.py            # Python script for plain-text PDF extraction
└── generate_forex_knowledge.py # Python script generating forex knowledge units
```

**Design Principles:**
- The 168 book files were **parsed, deduplicated, and optimized** into ~6,676 structured knowledge units across 43 JSON files
- Books provide foundational TA/risk/psychology knowledge; crypto-native knowledge comes from on-chain data, DeFi research, exchange mechanics, and AI-generated strategy work
- Knowledge units are JSON objects with `id`, `source`, `topic`, `conditions`, `content`, `priority`, and optional `tags`
- `conditions` link units to market contexts (Trending, Ranging, HighVolatility, LowVolatility, ExtremeFear, ExtremeGreed, etc.)
- `priority` is 1-5 (5 = most critical)
- `tags` provide additional metadata (setup_type, timeframe, indicator, regime_subtype, risk_context)
- 548 unique source strings: ~183 book-derived, ~365 from external/online resources, AI bots, exchange APIs, on-chain analytics, and synthetic merges

---

## 2. Structured Knowledge Units (JSON)

The primary knowledge base consists of **10 JSON files** containing **2,959 total knowledge units**. These are the actively loaded files used by the agent.

| File | Units | Topics Covered |
|------|-------|----------------|
| `knowledge_technical_analysis.json` | 506 | Dow Theory, volume analysis, chart patterns, indicator theory, MA systems, support/resistance |
| `knowledge_risk_management.json` | 350 | Position sizing (2%, 6%, Kelly, fixed fractional), drawdown math, risk-reward, portfolio management |
| `knowledge_crypto_native.json` | 319 | MVRV, NUPL, SOPR, NVT ratio, on-chain metrics, exchange flows, whale movements |
| `knowledge_psychology.json` | 319 | Tilt, discipline, probabilistic thinking, fear/greed management, journaling, recovery |
| `knowledge_sentiment.json` | 291 | COT data, put/call ratios, VIX, Fear & Greed Index, Minervini Stage 2, sentiment extremes |
| `knowledge_execution.json` | 282 | 2% rule, 6% rule, order types, slippage, execution tactics, session timing, breakouts |
| `knowledge_market_regimes.json` | 250 | Trending vs ranging detection, ADX, Bollinger, MA stacking, regime filters |
| `knowledge_trading_systems.json` | 226 | Turtle system, Donchian channels, trend following, system rules, pyramiding |
| `knowledge_price_action.json` | 216 | Wyckoff phases, FVG, order blocks, engulfing, pin bars, inside bars, fakey |
| `knowledge_fundamentals.json` | 200 | Mr. Market, margin of safety, Fisher 15 points, Buffett principles, Lynch categories |

### Topic Distribution

| Topic | Files Using It | Description |
|-------|---------------|-------------|
| `TechnicalAnalysis` | ta, pa, regimes, systems | Core TA: patterns, indicators, price action |
| `RiskManagement` | risk, execution, systems | Position sizing, stops, drawdown, portfolio |
| `Execution` | execution, price_action, systems | Order management, timing, slippage, sessions |
| `Psychology` | psychology | Mindset, discipline, emotional control |
| `CryptoNative` | crypto_native | On-chain metrics specific to crypto |
| `Sentiment` | sentiment, fundamentals | Market sentiment indicators and interpretation |
| `MarketRegime` | market_regimes | Regime identification and classification |
| `PriceAction` | price_action | Raw price action setups and patterns |
| `Fundamentals` | fundamentals | Value investing, fundamental analysis |
| `TradingSystems` | trading_systems | Systematic trading rules and methodologies |
| `MacroAnalysis` | (backup files) | Global liquidity, macro catalysts, cycles |
| `OrderFlow` | (backup files) | Order flow, market profile, auction theory |
| `AiStrategy` | (backup files) | AI-specific strategy concepts |
| `Backtesting` | (backup files) | Walk-forward optimization, backtesting methodology |
| `Sentiment` | (backup files) | Fear & Greed, COT, sentiment interpretation |

### Condition Tags (Market Contexts)

Knowledge units are activatable under these market conditions:

| Condition | Meaning |
|-----------|---------|
| `Trending` | Market exhibiting directional trend |
| `Ranging` | Market moving sideways in a range |
| `HighVolatility` | Elevated volatility environment |
| `LowVolatility` | Compressed volatility environment |
| `ExtremeFear` | Fear & Greed Index 0-25 or equivalent |
| `ExtremeGreed` | Fear & Greed Index 76-100 or equivalent |
| `SessionOpen` | At session open (London/NY/Asian) |
| `SessionClose` | At session close |
| `BreakingNews` | Major news event in progress |
| `AltSeason` | Altcoin season conditions |
| `BtcDominance` | Bitcoin dominance regime active |
| `HalvingProximity` | Near Bitcoin halving event |
| `FundingRateExtreme` | Perpetual funding rates at extremes |

---

## 3. Book Library

### Overview

The `knowledge/books/` directory contains **168 book files** spanning ~128 unique titles (some in multiple formats: PDF, EPUB, MOBI). These are the **source material** that was parsed, deduplicated, and optimized into the structured knowledge units in the JSON files above.

**How books map to crypto trading:**
- **Forex books** → candlestick reading, chart patterns, price action, support/resistance, session structure, order flow concepts (forex and crypto share 24/7 market microstructure)
- **Stock/fundamental books** → risk management frameworks, position sizing, portfolio theory, market psychology, behavioral discipline
- **Crypto-native books** → token economics, blockchain fundamentals, crypto market cycles
- **Market structure books** (Wyckoff, auction theory) → accumulation/distribution detection applicable to BTC dominance and altcoin rotation
- The forex and stock knowledge is **not the end goal** — it's the foundation the agent uses to read and trade crypto markets

### Complete Book Inventory

Below is the full inventory organized by category. Formats: `[PDF]`, `[EPUB]`, `[MOBI]`, `[AZW]`. Ratings from `_info.text` (Goodreads-style: rating/review_count).

#### Japanese Candlestick & Chart Pattern Analysis

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Japanese Candlestick Charting Techniques (Nison) | 1991 | 4.5 | 236 | PDF |
| Japanese Candlestick Charting Techniques 2nd ed. (Nison) | 2001 | 4.5 | 236 | PDF (×3) |
| Beyond Candlesticks: New Japanese Charting Techniques Revealed | 1994 | 4.2 | 30 | PDF (×3, 1 annotated) |
| The Candlestick Course (Nison) | 2003 | 4.5 | 106 | PDF |
| Candlestick Charting Explained 3rd ed. (Morris) | 2006 | 4.4 | 84 | PDF |
| Candlestick Charting For Dummies (Linton) | 2008 | 4.3 | 80 | PDF |
| Getting Started in Candlestick Charting (Wright) | 2008 | 4.4 | 36 | PDF |
| Candlestick and Pivot Point Trading Triggers (Person) | 2006 | 4 | 33 | PDF |
| Encyclopedia of Chart Patterns 2nd ed. (Bulkowski) | 2005 | 4.3 | 90 | PDF |
| Visual Guide to Chart Patterns (Bulkowski) | 2012 | 4 | 29 | PDF |

#### Technical Analysis

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Technical Analysis of the Financial Markets (Murphy) | 1999 | 4.6 | 227 | PDF |
| Technical Analysis: The Complete Resource for Financial Market Technicians (Edwards/Magee) | 2006 | 4.4 | 39 | PDF |
| Technical Analysis: The Complete Resource 2nd ed. (Edwards/Magee) | 2010 | 4.1 | 55 | PDF (×2) |
| Technical Analysis: The Complete Resource 2nd ed. (Edwards/Magee) | 2011 | 4.1 | 55 | PDF |
| Technical Analysis Explained 4th ed. (Pring) | 2002 | 4.1 | 50 | PDF |
| Technical Analysis and Stock Market Profits (Schabacker) | 2005 | 4.4 | 6 | PDF |
| Technical Analysis of Stock Trends 9th ed. (Edwards/Magee) | 2007 | 4.1 | 102 | PDF |
| Technical Analysis of Stock Trends Explained (Bulkowski) | 2013 | 3.6 | 6 | EPUB, MOBI |
| Technical Analysis For Dummies 2nd ed. (Rockefeller) | 2011 | 4.1 | 95 | PDF |
| A Complete Guide to Technical Trading Tactics (Person) | 2004 | 3.5 | 19 | PDF |
| Chart Your Way To Profits 2nd ed. (Person) | 2010 | 4.2 | 8 | PDF |
| The Visual Investor 2nd ed. (Murphy) | 2009 | 4.4 | 53 | PDF |
| The Alchemy of Finance 2nd ed. (Soros) | 1994 | 4.3 | 6 | PDF |
| Technical Analysis for the Trading Professional 2nd ed. (Brown) | 2012 | 3.6 | 74 | PDF |
| New Frontiers in Technical Analysis (DeMark) | 2011 | 4 | 1 | PDF |
| Evidence-Based Technical Analysis (Aronson) | 2007 | 4 | 69 | PDF |
| The Handbook of Technical Analysis | 2016 | 5 | 5 | PDF |
| The Art and Science of Technical Analysis (Grimes) | 2012 | 4.6 | 79 | PDF (×1, 1 annotated) |

#### Volume Price Analysis

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| A Complete Guide To Volume Price Analysis (Coulling) | 2013 | 4.5 | 383 | EPUB, PDF |

#### Trading Psychology & Discipline

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Trading in the Zone (Douglas) | 2000 | 4.5 | 541 | PDF (×2), EPUB |
| The Disciplined Trader (Douglas) | 1990 | 4.4 | 205 | PDF (×2, 1 annotated) |
| Fooled by Randomness (Taleb) | 2005 | 4.1 | 718 | PDF, EPUB |
| How to Day Trade for a Living 4th ed. (Aaziz) | 2015 | 4.7 | 880 | EPUB |
| Sentiment in the Forex Market (Saettele) | 2008 | 4 | 17 | PDF |

#### Market Wizards & Trader Interviews

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Market Wizards (Schwager) | 2012 | 4.5 | 340 | PDF (×2) |
| The New Market Wizards (Schwager) | 1992 | 4.5 | 162 | PDF |
| Stock Market Wizards (Schwager) | 2001 | 4.2 | 68 | PDF (×2), EPUB |
| 41775536-Market-Wizards (Schwager) | — | — | PDF |
| Millionaire Traders (Schwager) | 2007 | — | PDF |
| Adventures of a Currency Trader (Lein) | 2007 | 4 | 52 | PDF |
| The Complete Turtle Trader (Faith) | 2007 | — | PDF |
| Way of the Turtle (Faith) | 2007 | 4 | 165 | PDF |

#### Risk Management & Money Management

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Trade Your Way to Financial Freedom 2nd ed. (Van Tharp) | 2006 | 4.3 | 266 | EPUB, PDF (×2) |
| Trading for a Living (Elder) | 1993 | 4.2 | 296 | PDF |
| The New Trading for a Living (Elder) | 2014 | 4.4 | 99 | PDF |
| The New Trading for a Living (Study Guide) (Elder) | 2014 | — | PDF |
| Building Reliable Trading Systems (Schutt) | 2013 | 4.2 | 29 | PDF |
| High Probability Trading Strategies (Minervini) | 2008 | 4.1 | 147 | PDF |
| Trade Like a Pro (Whistler) | 2009 | 4 | 1 | PDF |
| The Commitments of Traders Bible (Briese) | 2008 | 4.6 | 15 | PDF |

#### Position Sizing & Portfolio Theory

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| The Encyclopedia of Trading Strategies (Saluka) | 2000 | 3 | 33 | PDF |

#### Trend Following

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Trend Following (Covel) | 2009 | 4.1 | 389 | PDF |
| Trend Trading for a Living (Crabel) | 2007 | 4.2 | 71 | PDF |
| Long-Term Secrets to Short-Term Trading (Williams) | 1999 | 3.4 | 47 | PDF |
| Attacking Currency Trends (Dao) | 2011 | 4.6 | 61 | PDF |
| The Trend Following Bible (Abraham) | 2012 | 3.5 | 33 | PDF |
| Dual Momentum Investing (Antonacci) | 2015 | 4.7 | 340 | EPUB |

#### Swing Trading

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Swing Trading For Dummies (Basset) | 2008 | 4.4 | 75 | PDF |
| Dave Landry on Swing Trading | 2001 | 4.1 | 21 | PDF |
| The Master Swing Trader (Crabel) | 2001 | 3.2 | 248 | PDF |
| The Master Swing Trader Toolkit (Crabel) | 2010 | 3.4 | 20 | PDF |
| Timing Solutions for Swing Traders (Yates) | 2012 | 4.5 | 2 | PDF |

#### Day Trading

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Day Trading For Dummies (Logan) | 2008 | 3.9 | 28 | PDF |
| Day Trading For Dummies 3rd ed. (Logan) | 2014 | 3.6 | 36 | PDF, EPUB |
| Mastering the Trade (Carter) | 2006 | 4.3 | 327 | PDF (×3) |
| Mastering the Trade 2nd ed. (Carter) | 2012 | 4.3 | 327 | PDF (×3, 1 annotated) |
| The Complete Trading Course (Carter) | 2011 | 4.1 | 24 | PDF |

#### Price Action & Order Flow

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Understanding Price Action (Lunden) | 2014 | 4.2 | 47 | PDF |
| Naked Forex (Nekritin) | 2012 | 4.2 | 95 | PDF |
| Trade What You See (Littleton) | 2007 | 4 | 46 | PDF |
| Trading Price Action Trends (Brooks) | 2011 | 4.1 | 67 | PDF |

#### Algorithmic & Systematic Trading

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Algorithmic Trading (Chan) | 2013 | 4.1 | 37 | PDF (×2, 1 annotated) |
| Intermarket Trading Strategies (Henderson) | 2009 | 4.6 | 15 | PDF |
| The Complete Guide to Price Action Trading (Brooks) | — | — | — |

#### Trading Systems & Methodologies

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Come Into My Trading Room (Elder) | 2002 | 4.5 | 170 | PDF |
| Come Into My Trading Room (Study Guide) (Elder) | 2002 | — | PDF |
| The 10 Essentials of Forex Trading (Norris) | 2007 | 3.9 | 44 | PDF |
| The Market Guys' Five Points for Trading Success | 2007 | 4.4 | 52 | PDF |

#### Harmonic Trading

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Harmonic Trading, Volume One (Carney) | 2010 | 3.6 | 43 | PDF |
| Harmonic Trading, Volume Two (Carney) | 2010 | 4.2 | 12 | PDF |

#### Options & Derivatives

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Option Volatility & Pricing (Natenberg) | 1994 | 4.3 | 155 | PDF |
| Option Spread Strategies (Harley) | 2009 | 3.3 | 15 | PDF |
| Trading Binary Options (2nd ed.) | 2016 | 3 | 2 | PDF |
| Top Binary Options Trading Strategies | — | — | PDF |

#### Forex & Currency Trading (Foundational Market Structure)

These books teach market mechanics — session timing, price action, order flow, and multi-timeframe analysis — that transfer directly to crypto's 24/7 markets. Crypto lacks traditional session structure, but the concepts of liquidity, volatility cycles, institutional vs. retail behavior, and intermarket relationships apply.

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Day Trading and Swing Trading the Currency Market 2nd ed. (Lien) | 2008 | 4 | 29 | PDF |
| Day Trading the Currency Market (Lien) | 2005 | 3.8 | 47 | PDF |
| Currency Trading and Intermarket Analysis (Lien) | 2008 | 4 | 26 | PDF |
| Inside the Currency Market (Dunbar) | 2011 | 4 | 2 | PDF |
| Currency Strategy (Callan) | 2002 | 3.5 | 7 | PDF |
| Essentials of Foreign Exchange Trading (Haskill) | 2009 | 4.1 | 26 | PDF |
| Currency Trading For Dummies 2nd ed. (Dolan) | 2011 | 4.1 | 66 | PDF |
| The Little Book of Currency Trading (Lien) | 2010 | 3.8 | 42 | PDF |
| How to Make a Living Trading Foreign Exchange (Cook) | 2010 | 3.6 | 29 | PDF |
| Forex Patterns and Probabilities (Edwards) | 2007 | 4.1 | 62 | PDF |
| Forex Trading Secrets (McGee) | 2010 | 3.3 | 9 | PDF |
| Forex for Beginners (Dolan) | 2012 | 4 | 10 | PDF |
| The Sensible Guide to Forex (Loyd) | 2012 | 3.8 | 20 | PDF |
| 17 Proven Currency Trading Strategies (Lien) | 2013 | 2.4 | 10 | PDF |
| Forex Trading Basics & Secrets Vol 3.0 | — | — | PDF |

#### Ichimoku

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Trading with Ichimoku Clouds (Patel) | 2010 | 3 | 33 | PDF |

#### ETF Trading

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Trading ETFs 2nd ed. (Duguid) | 2012 | 4.2 | 3 | PDF |

#### Fundamental Analysis & Value Investing

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| The Intelligent Investor (Graham) | 2003 | 4.5 | 1881 | PDF, MOBI |
| Common Stocks and Uncommon Profits (Fisher) | 2003 | 4.3 | 206 | PDF, AZW |
| One Up On Wall Street (Lynch) | 2000 | 4.5 | 673 | EPUB |
| How to Make Money in Stocks 4th ed. (O'Neil) | 2009 | 4.3 | 577 | PDF, EPUB, MOBI |
| The Five Rules for Successful Stock Investing (Dorsey) | 2004 | 4.6 | 131 | PDF |
| The Little Book of Common Sense Investing (Bogle) | 2007 | 4.6 | 681 | PDF, EPUB |
| The Little Book That Beats the Market (Greenblatt) | 2006 | 4.1 | 466 | PDF |
| Fundamental Analysis For Dummies (Krantz) | 2009 | 4.3 | 52 | PDF |
| The Essays of Warren Buffett (Buffett/Cunningham) | 2015 | 4.7 | 260 | PDF |

#### Warren Buffett & Charlie Munger

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| University of Berkshire Hathaway (Pecaut) | 2017 | 4.6 | 172 | EPUB, MOBI |
| The Warren Buffett Way 2nd ed. (Hagstrom) | 2005 | 4 | 122 | PDF |
| Buffett: The Making of an American Capitalist (Lowenstein) | 2008 | 4.7 | 244 | PDF |
| The Snowball (Schroeder) | 2009 | 4.4 | 522 | PDF |
| The Essays of Warren Buffett (Buffett/Cunningham) | 2015 | 4.7 | 260 | PDF |

#### Trading Classics (Pre-2000)

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Reminiscences of a Stock Operator (Lefèvre) | 2006 | 4.5 | 569 | PDF (×3), EPUB (×3) |
| Jesse Livermore: World's Greatest Stock Trader (Smitten) | 2001 | 4.3 | 52 | PDF |

#### Behavioral Finance & Market Psychology

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Irrational Exuberance (Shiller) | 2000 | 4.2 | 229 | PDF |
| Irrational Exuberance 2nd ed. (Shiller) | 2006 | 4.2 | 229 | EPUB |
| Irrational Exuberance 3rd ed. (Shiller) | 2016 | 4.2 | 229 | PDF |
| Extraordinary Popular Delusions and The Madness of Crowds (Mackay) | 2001 | 3.9 | 313 | PDF |
| Principles - Life and Work (Dalio) | 2017 | 4.4 | 604 | EPUB |

#### Market History & Wall Street Narratives

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Liar's Poker (Lewis) | 2014 | 4.4 | 983 | EPUB |
| Flash Boys (Lewis) | 2015 | 4.5 | 3359 | EPUB |
| The Big Short (Lewis) | 2011 | 4.6 | 2442 | EPUB |
| When Genius Failed (Lowenstein) | 2001 | 4.6 | 404 | PDF, EPUB |
| Boomerang (Lewis) | 2012 | 4.3 | 1154 | PDF |
| The Broker (Raghavan) | 2005 | 4.1 | 1310 | PDF |
| Moneyball (Lewis) | 2004 | 4.6 | 1323 | MOBI |

#### Crypto-Specific

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Cryptoassets (Burniske/Tatar) | 2017 | 4.5 | 70 | PDF |
| Cryptocurrency-Trading-101 | — | — | PDF |
| Crypto Full Course Beginners | — | — | (via backup) |
| GoodCrypto Patterns Presentation | — | — | PDF |
| schueffelgroenewegbaldegger2019 Crypto Encyclopedia | 2019 | — | PDF |

#### Business & Biography (Non-Trading)

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| The Blind Side (Lewis) | 2007 | 4.4 | 615 | MOBI |
| The New New Thing (Lewis) | 2014 | 4.1 | 328 | EPUB |

#### For Dummies Series (General)

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| Trading For Dummies 2nd ed. (Rochester) | 2009 | 4.3 | 65 | PDF |
| Day Trading For Dummies (Logan) | 2008 | 3.9 | 28 | PDF |
| Day Trading For Dummies 3rd ed. (Logan) | 2014 | 3.6 | 36 | PDF, EPUB |
| Swing Trading For Dummies (Basset) | 2008 | 4.4 | 75 | PDF |
| Currency Trading For Dummies 2nd ed. (Dolan) | 2011 | 4.1 | 66 | PDF |
| Technical Analysis For Dummies 2nd ed. (Rockefeller) | 2011 | 4.1 | 95 | PDF |
| Fundamental Analysis For Dummies (Krantz) | 2009 | 4.3 | 52 | PDF |

#### A Random Walk Down Wall Street

| Title | Year | Rating | Reviews | Formats |
|-------|------|--------|---------|
| A Random Walk Down Wall Street 10th ed. (Malkiel) | 2012 | 4.6 | 807 | EPUB, MOBI |
| A Random Walk Down Wall Street 11th ed. (Malkiel) | 2015 | 4.6 | 807 | EPUB |

---

## 3b. Complete Book Title List

All 128 unique titles from `_info.text`, sorted alphabetically:

1. 17 Proven Currency Trading Strategies (Lien, 2013)
2. Adventures of a Currency Trader (Lein, 2007)
3. Algorithmic Trading (Chan, 2013)
4. Attacking Currency Trends (Dao, 2011)
5. A Random Walk Down Wall Street 10th ed. (Malkiel, 2012)
6. A Random Walk Down Wall Street 11th ed. (Malkiel, 2015)
7. A Complete Guide to Technical Trading Tactics (Person, 2004)
8. A Complete Guide to Volume Price Analysis (Coulling, 2013)
9. Buffett: The Making of an American Capitalist (Lowenstein, 2008)
10. Building Reliable Trading Systems (Schutt, 2013)
11. Candlestick and Pivot Point Trading Triggers (Person, 2006)
12. Candlestick Charting Explained 3rd ed. (Morris, 2006)
13. Candlestick Charting For Dummies (Linton, 2008)
14. Chart Your Way to Profits 2nd ed. (Person, 2010)
15. Come Into My Trading Room (Elder, 2002)
16. Common Stocks and Uncommon Profits 2nd ed. (Fisher, 2003)
17. Currency Strategy (Callan, 2002)
18. Currency Trading and Intermarket Analysis (Lien, 2008)
19. Currency Trading for Dummies 2nd ed. (Dolan, 2011)
20. Cryptoassets (Burniske/Tatar, 2017)
21. Cryptocurrency-Trading-101
22. Day Trading and Swing Trading the Currency Market 2nd ed. (Lien, 2008)
23. Day Trading for Dummies (Logan, 2008)
24. Day Trading for Dummies 3rd ed. (Logan, 2014)
25. Day Trading the Currency Market (Lien, 2005)
26. Dual Momentum Investing (Antonacci, 2015)
27. Encyclopedia of Chart Patterns 2nd ed. (Bulkowski, 2005)
28. Essentials of Foreign Exchange Trading (Haskill, 2009)
29. Evidence-Based Technical Analysis (Aronson, 2007)
30. Extraordinary Popular Delusions and the Madness of Crowds (Mackay, 2016)
31. Flash Boys (Lewis, 2015)
32. Fooled by Randomness (Taleb, 2005)
33. Forex for Beginners (Dolan, 2012)
34. Forex Patterns and Probabilities (Edwards, 2007)
35. Forex Trading Basics & Secrets Vol 3.0
36. Forex Trading Secrets (McGee, 2010)
37. Fundamental Analysis for Dummies (Krantz, 2009)
38. Getting Started in Candlestick Charting (Wright, 2008)
39. Harmonic Trading Volume One (Carney, 2010)
40. Harmonic Trading Volume Two (Carney, 2010)
41. High Probability Trading Strategies (Minervini, 2008)
42. How to Day Trade for a Living 4th ed. (Aaziz, 2015)
43. How to Make a Living Trading Foreign Exchange (Cook, 2010)
44. How to Make Money in Stocks 4th ed. (O'Neil, 2009)
45. Inside the Currency Market (Dunbar, 2011)
46. Intermarket Trading Strategies (Henderson, 2009)
47. Irrational Exuberance (Shiller, 2000)
48. Irrational Exuberance 2nd ed. (Shiller, 2006)
49. Irrational Exuberance 3rd ed. (Shiller, 2016)
50. Japanese Candlestick Charting Techniques (Nison, 1991)
51. Japanese Candlestick Charting Techniques 2nd ed. (Nison, 2001)
52. Beyond Candlesticks (Nison, 1994)
53. Jesse Livermore: World's Greatest Stock Trader (Smitten, 2001)
54. Liar's Poker (Lewis, 2014)
55. Long-Term Secrets to Short-Term Trading (Williams, 1999)
56. Mastering the Trade 2nd ed. (Carter, 2012)
57. Millionaire Traders (Schwager, 2007)
58. Moneyball (Lewis, 2004)
59. Naked Forex (Nekritin, 2012)
60. New Frontiers in Technical Analysis (DeMark, 2011)
61. One Up on Wall Street (Lynch, 2000)
62. Option Volatility & Pricing (Natenberg, 1994)
63. Option Spread Strategies (Harley, 2009)
64. Principles — Life and Work (Dalio, 2017)
65. Reminiscences of a Stock Operator (Lefèvre, 2006)
66. Sentiment in the Forex Market (Saettele, 2008)
67. Stock Market Wizards (Schwager, 2003)
68. Swing Trading for Dummies (Basset, 2008)
69. Technical Analysis and Stock Market Profits (Schabacker, 2005)
70. Technical Analysis Explained 4th ed. (Pring, 2002)
71. Technical Analysis for Dummies 2nd ed. (Rockefeller, 2011)
72. Technical Analysis for the Trading Professional 2nd ed. (Brown, 2012)
73. Technical Analysis of Stock Trends 9th ed. (Edwards/Magee, 2007)
74. Technical Analysis of Stock Trends Explained (Bulkowski, 2013)
75. Technical Analysis of the Financial Markets (Murphy, 1999)
76. Technical Analysis: The Complete Resource 2nd ed. (Edwards/Magee, 2011)
77. The Alchemy of Finance 2nd ed. (Soros, 1994)
78. The Art and Science of Technical Analysis (Grimes, 2012)
79. The Big Short (Lewis, 2011)
80. The Blind Side (Lewis, 2007)
81. The Broker (Raghavan, 2005)
82. The Candlestick Course (Nison, 2003)
83. The Commitments of Traders Bible (Briese, 2008)
84. The Complete Turtle Trader (Faith, 2007)
85. The Disciplined Trader (Douglas, 1990)
86. The Encyclopedia of Trading Strategies (Saluka, 2000)
87. The Essays of Warren Buffett (Buffett/Cunningham, 2015)
88. The Five Rules for Successful Stock Investing (Dorsey, 2004)
89. The Little Book of Common Sense Investing (Bogle, 2007)
90. The Little Book of Currency Trading (Lien, 2010)
91. The Little Book That Beats the Market (Greenblatt, 2006)
92. The Market Guys' Five Points for Trading Success (2007)
93. The Master Swing Trader (Crabel, 2001)
94. The Master Swing Trader Toolkit (Crabel, 2010)
95. The New Market Wizards (Schwager, 1992)
96. The New New Thing (Lewis, 2014)
97. The New Trading for a Living (Elder, 2014)
98. The Sensible Guide to Forex (Loyd, 2012)
99. The Snowball (Schroeder, 2009)
100. The Warren Buffett Way 2nd ed. (Hagstrom, 2005)
101. Timing Solutions for Swing Traders (Yates, 2012)
102. Trade Like a Pro (Whistler, 2009)
103. Trade What You See (Littleton, 2007)
104. Trade Your Way to Financial Freedom 2nd ed. (Van Tharp, 2006)
105. Trading Binary Options 2nd ed. (2016)
106. Trading ETFs 2nd ed. (Duguid, 2012)
107. Trading for a Living (Elder, 1993)
108. Trading in the Zone (Douglas, 2000)
109. Trading Price Action Trends (Brooks, 2011)
110. Trading with Ichimoku Clouds (Patel, 2010)
111. Trend Following (Covel, 2009)
112. Trend Trading for a Living (Crabel, 2007)
113. University of Berkshire Hathaway (Pecaut, 2017)
114. Visual Guide to Chart Patterns (Bulkowski, 2012)
115. Way of the Turtle (Faith, 2007)
116. When Genius Failed (Lowenstein, 2001)

---

## 3c. Interview, Video & Course Sources

These are transcripts, video courses, and interview transcripts that were parsed into knowledge units.

### YouTube Video Courses & Tutorials

| Title | Source | Format |
|-------|--------|--------|
| How To Actually Build a Trading Bot With Claude Code (Fully Automated) | YouTube — Full step-by-step tutorial on building an automated trading bot using Claude Code, Hidden Markov Models, and the Alpaca brokerage API | Transcript |
| I Made AI Trading Bots Compete To Make Money — OpenClaw Trading Olympics | YouTube — Experiment running autonomous AI trading bots on OpenClaw (Claude-powered) competing on Hyperliquid in a survival-of-the-fittest tournament | Transcript |
| How To Start Day Trading in 2026 — A Complete 9-Step Framework | Warrior Trading (Ross Cameron) — Full-length training | Transcript |
| How To Start Day Trading as a Beginner in 2025 — The Complete Guide | TJR (Tyler) — YouTube Compilation, Full Day Trading Transformation Series | Transcript |
| How To Start Day Trading for Beginners in 2026 — Full Course | Juvier — YouTube Full Course, Day Trading Complete Beginner Guide | Transcript |
| The Simplest Way to Start Day Trading in 2026 — Step-by-Step Beginner's Blueprint | Unnamed Trading Educator — YouTube Video | Transcript |

### Podcast & Interview Transcripts

| Title | Source | Format |
|-------|--------|--------|
| Cathie Wood — The Future of AI, Tesla, Bitcoin, and Disruptive Innovation | Diary of a CEO Podcast — Cathie Wood, CEO and CIO of ARK Invest, discusses five major innovation platforms, her highest-conviction stock picks, Bitcoin's path to $1.5M, and why AI is the biggest technological disruption in history | Transcript |
| Pradeep Bondi — Episodic Pivot Strategy | Words of Wisdom Podcast — Pradeep Bondi, founder of StockBy and creator of the Episodic Pivot (EP) trading playbook | Transcript |
| Fabio Valentina — World-Class Scalping: Order Flow, Volume Profile, and Auction Market Theory | Chart Fanatics Podcast — Fabio Valentina, world-class futures scalper ranked top 3 globally at Robins Club (500%+ returns in 12 months, futures division) | Transcript |

### Crypto Strategy & Education

| Title | Source | Format |
|-------|--------|--------|
| How I Plan to Make Millions Investing in Crypto 2026 (Again) | Brian Jung — Crypto investing strategy for 2026, covering BTC price predictions, macro catalysts, market cycle analysis, altcoin picks, and a complete framework for building wealth in the next cycle | Transcript |
| Crypto Trading Full Course for Beginners | Pakistani Crypto Educator — Comprehensive free crypto trading course | Transcript |

### External / On-Chain / AI-Derived Sources

These sources were incorporated into knowledge units but don't have standalone transcript files:

| Source | Description | Units |
|--------|-------------|-------|
| Glassnode | On-chain analytics: MVRV, NUPL, SOPR, NVT ratio, exchange flows | 15 |
| CryptoQuant | Exchange reserves, whale movements, miner flows | — |
| BGeometrics | Fear & Greed Index historical data | — |
| CoinGecko | Token data, market cap rankings | — |
| Blockscout | Arbitrum contract verification | — |
| OpenRouter | LLM model comparisons, benchmarks | — |
| Alpaca API | Brokerage API documentation | — |
| Hyperliquid | On-chain perpetual futures documentation | — |
| Velotrade | Prop firm drawdown models | 7 |
| IRS/Form 1099-DA | Crypto tax compliance | 5 |
| QuantInsti | Walk-forward optimization methodology | 10 |
| Jared Tintle | Tilt detection, psychology | 10 |
| Michael Howell | Global liquidity cycle macro research | 10 |

---

## 4. Knowledge Extraction Pipeline

### `extract_books.py`

A Python script using PyMuPDF (`fitz`) to extract plain text from 17 forex PDFs. Outputs numbered `book_01.txt` through `book_17.txt` in `knowledge/extracted/`. The output directory is currently empty — the script may have been run and cleaned up, or serves as a utility for future re-extraction. The actual knowledge from these books was extracted programmatically into JSON knowledge units rather than plain text.

**Books targeted for extraction (17 forex titles):**
1. Day Trading and Swing Trading the Currency Market — Kathy Lien
2. Currency Trading and Intermarket Analysis — Kathy Lien
3. Inside the Currency Market — Dunbar
4. Sentiment in the Forex Market — Saettele
5. Forex Patterns and Probabilities — Edwards
6. Attacking Currency Trends — Dao
7. Naked Forex — Nekritin
8. The Little Book of Currency Trading — Lien
9. The Sensible Guide to Forex — Loyd
10. Essentials of Foreign Exchange Trading — Haskill
11. How to Make a Living Trading Foreign Exchange — Cook
12. Currency Strategy — Callan
13. 17 Proven Currency Trading Strategies — Lien
14. Forex Trading Basics & Secrets Vol 3.0
15. Forex for Beginners — Dolan
16. The 10 Essentials of Forex Trading — Norris
17. Forex Trading Secrets — McGee

### `generate_forex_knowledge.py`

A Python script that programmatically generates forex-derived knowledge units from manual curation. Adds knowledge units with IDs `book-fx-001` through `book-fx-335+` covering session timing, London/NY overlap, fade strategies, MA analysis, and more. Sources attributed to specific books like Kathy Lien's works. The generated output was merged into the backup `book_forex_complete.json` file. While the source material is forex-focused, the knowledge (session behavior, price action, order flow) is applied by the agent to crypto market analysis.

---

## 5. Backup Knowledge Files

The `knowledge/_backup/` directory contains **33 JSON files** with **3,717 total knowledge units**. These provide supplementary and specialized coverage that extends the primary knowledge base. Some overlap with primary files; others provide unique crypto-native, macro, DeFi, and trader-interview coverage not in the primary 10 files.

### Book-Derived Collections

| File | Units | Source Books |
|------|-------|-------------|
| `book_forex_complete.json` | 335 | Multiple forex books |
| `book_technical_analysis.json` | 340 | Murphy, Edwards/Magee, Pring, Kirkpatrick/Dahlquist |
| `book_complete_systems.json` | 200 | Elder, Carter, various systems books |
| `book_patterns_candles.json` | 253 | Nison, Coulling, Morris, Bulkowski |
| `book_volume_advanced_ta.json` | 250 | Volume analysis, advanced TA |
| `book_investing_fundamentals.json` | 291 | Graham, Fisher, Buffett, Lynch |
| `book_market_wizards.json` | 220 | All Market Wizards series |
| `book_systems_algo.json` | 250 | Algorithmic/systematic trading books |
| `book_trading_psychology.json` | 180 | Douglas, Elder, Taleb |
| `book_behavioral_crypto_options.json` | 285 | Crypto Trading 101, options books |
| `book_daytrading_options_misc.json` | 200 | Day trading, options, miscellaneous |

### Interview & Trader-Specific

| File | Units | Source |
|------|-------|--------|
| `brian_jung.json` | 8 | Brian Jung — macro catalysts, trading strategy |
| `cathie_wood.json` | 6 | Cathie Wood / ARK Invest — innovation platforms |
| `fabio_amt.json` | 15 | Fabio & Valentina — scalping, AMT |
| `pradeep_ep.json` | 15 | Pradeep — episodic pivot strategy |
| `juvier_daytrading.json` | 14 | Juvier — kill zones, day trading |
| `warrior_trading.json` | 10 | Warrior Trading — 5 pillars stock selection |
| `tjr_smc.json` | 24 | TJR — smart money concepts, FVG |

### Strategy & Execution

| File | Units | Source |
|------|-------|--------|
| `wyckoff_orderflow.json` | 12 | Altrady — Wyckoff accumulation/distribution |
| `hybrid_scalping.json` | 8 | Heikin Ashi + 100 EMA scalping |
| `execution_engineering.json` | 8 | CoinAPI — REST vs WebSocket, latency |
| `crypto_derivatives.json` | 12 | BitMEX — funding rate arbitrage |
| `defi_execution.json` | 10 | Hyperliquid — on-chain perpetual futures |
| `backtesting_deployment.json` | 10 | QuantInsti — walk-forward optimization |

### Macro & On-Chain

| File | Units | Source |
|------|-------|--------|
| `macro_liquidity.json` | 10 | Michael Howell — global liquidity cycle |
| `onchain_analytics.json` | 15 | Glassnode — MVRV, NUPL, SOPR, NVT |
| `crypto_fcb.json` | 22 | Crypto Full Course — Fear & Greed, basics |

### Risk & Compliance

| File | Units | Source |
|------|-------|--------|
| `risk_management.json` | 14 | Quantitative risk mathematics |
| `prop_firm.json` | 7 | Velotrade — prop firm drawdown models |
| `compliance.json` | 5 | IRS — Form 1099-DA, wash sale rules |

### AI & Psychology

| File | Units | Source |
|------|-------|--------|
| `ai_claude_bot.json` | 20 | Claude Code trading bot — HMM regime detection |
| `ai_competition.json` | 10 | AI trading competition — natural selection concept |
| `trading_psychology.json` | 10 | Jared Tintle — tilt detection |

---

## 6. Generated Summaries & Indexes

### `INVESTMENT_BOOKS_SUMMARY.md` (917 lines)

A comprehensive synthesized summary of 150+ investment and trading books. Covers:
- Trading psychology & discipline (Douglas, Elder)
- Risk management & money management (2% rule, Kelly, position sizing)
- Loss recovery strategies (stop digging, reduce size, Fresh Start Protocol)
- Efficient trading strategies (checklists, scaling, trailing stops)
- Technical analysis principles (candlesticks, S/R, trend analysis, volume)
- Fundamental analysis (Graham, Fisher, Buffett, Lynch)
- Market structure & price action (naked Forex, Wyckoff, auction theory)
- Trading systems (trend following, mean reversion, swing trading, algo)
- Position sizing & portfolio management (diversification, rebalancing)
- Notable quotes from Livermore, Graham, Buffett, Jones, Seykota, Elder, Douglas
- Reading order recommendations

### `_info.text` (128 lines)

Complete rated inventory of all books with Goodreads-style ratings (rating/review_count). Covers 128 unique entries. Highest-rated: The Handbook of Technical Analysis (5/5), Buffett (4.7/244), Essays of Warren Buffett (4.7/260), How to Day Trade for a Living (4.7/880), Dual Momentum Investing (4.7/340), Evidence-Based TA (4.6/404), Technical Analysis of the Financial Markets (4.6/227).

### `Trading.txt` (170 lines)

Skill-level organized book list:
- **Developing Style** (3 books): Principles, The Disciplined Trader, Trading in the Zone
- **Beginner** (11 books): Candlestick foundations, TA Encyclopedia, Technical Analysis textbooks
- **Intermediate** (30+ books): Day trading, swing trading, forex, systems, momentum
- **Experienced or General View** (25+ books): Market Wizards, Buffett, behavioral finance, classics
- Full duplicate list included at end

---

## 7. Topic Coverage Matrix

| Area | Primary JSON | Backup JSON | Books | Total Units |
|------|-------------|-------------|-------|-------------|
| Technical Analysis | 506 (`ta`) | 593 (`book_technical_analysis`, `book_patterns_candles`, `book_volume_advanced_ta`) | ~35 | ~1,099 |
| Risk Management | 350 (`risk`) | 21 (`risk_management`, `prop_firm`) | ~12 | ~371 |
| Crypto On-Chain | 319 (`crypto_native`) | 37 (`onchain_analytics`, `crypto_derivatives`, `crypto_fcb`) | ~5 | ~356 |
| Psychology | 319 (`psych`) | 190 (`book_trading_psychology`, `trading_psychology`) | ~8 | ~509 |
| Sentiment | 291 (`sentiment`) | 22 (`crypto_fcb`) | ~5 | ~313 |
| Execution | 282 (`execution`) | 18 (`execution_engineering`) | ~20 | ~300 |
| Market Regimes | 250 (`market_regimes`) | — | — | ~250 |
| Trading Systems | 226 (`trading_systems`) | 450 (`book_systems_algo`, `book_complete_systems`) | ~15 | ~676 |
| Price Action | 216 (`price_action`) | 36 (`wyckoff_orderflow`, `hybrid_scalping`, `tjr_smc`) | ~8 | ~252 |
| Forex (foundational TA for crypto) | — | 335 (`book_forex_complete`) | ~17 | ~335 |
| Fundamentals | 200 (`fundamentals`) | 291 (`book_investing_fundamentals`) | ~10 | ~491 |
| Candlestick Analysis | — | 253 (`book_patterns_candles`) | ~10 | ~253 |
| Trader Interviews | — | 263 (`book_market_wizards` + trader-specific) | ~8 | ~263 |
| Macro Analysis | — | 31 (`macro_liquidity`, `brian_jung`, `cathie_wood`) | — | ~31 |
| Options/Derivatives | — | 285 (`book_behavioral_crypto_options`) | ~4 | ~285 |
| Algorithmic Trading | — | 250 (`book_systems_algo`) | ~3 | ~250 |
| Backtesting | — | 10 (`backtesting_deployment`) | — | ~10 |
| AI/ML | — | 30 (`ai_claude_bot`, `ai_competition`) | — | ~30 |
| Compliance | — | 5 (`compliance`) | — | ~5 |
| Scalping | — | 23 (`fabio_amt`, `hybrid_scalping`, `juvier_daytrading`) | — | ~23 |
| DeFi | — | 10 (`defi_execution`) | — | ~10 |
| **TOTALS** | **2,959** | **3,717** | **~169** | **~6,676+** |

---

## 8. Knowledge Unit ID Ranges

### Primary Files

| File | ID Prefix | Range |
|------|-----------|-------|
| `knowledge_technical_analysis.json` | `ta-` | 001–506 |
| `knowledge_risk_management.json` | `risk-` | 001–350 |
| `knowledge_crypto_native.json` | `cn-` | 001–319 |
| `knowledge_psychology.json` | `psych-` | 001–319 |
| `knowledge_sentiment.json` | `sent-` | 012–302 |
| `knowledge_execution.json` | `exec-` | 001–282 |
| `knowledge_market_regimes.json` | `reg-` | 001–250 |
| `knowledge_trading_systems.json` | `ts-` | 001–226 |
| `knowledge_price_action.json` | `pa-` | 001–216 |
| `knowledge_fundamentals.json` | `fund-` | 001–200 |

### Backup Files (Selected)

| File | ID Prefix | Range |
|------|-----------|-------|
| `book_market_wizards.json` | `book-wiz-` | 001–220 |
| `book_technical_analysis.json` | `book-ta-` | 001–340 |
| `book_forex_complete.json` | `book-fx-` | 001–335+ |
| `book_patterns_candles.json` | `book-pc-` | 001–253 |
| `book_investing_fundamentals.json` | `book-if-` | 001–291 |
| `book_complete_systems.json` | `book-cs-` | 001–200 |
| `wyckoff_orderflow.json` | `wyckoff-` | 001–12 |
| `macro_liquidity.json` | `macro-` | 001–10 |
| `tjr_smc.json` | `tjr-smc-` | 001–24 |
| `crypto_derivatives.json` | `deriv-` | 001–12 |

---

## 9. File Manifest

### Primary Knowledge (Active)

```
knowledge/
├── knowledge_crypto_native.json          (6,318 lines, 319 units)
├── knowledge_execution.json              (4,498 lines, 282 units)
├── knowledge_fundamentals.json           (4,020 lines, 200 units)
├── knowledge_market_regimes.json         (3,914 lines, 250 units)
├── knowledge_price_action.json           (3,132 lines, 216 units)
├── knowledge_psychology.json             (4,816 lines, 319 units)
├── knowledge_risk_management.json        (7,057 lines, 350 units)
├── knowledge_sentiment.json              (5,977 lines, 291 units)
├── knowledge_technical_analysis.json     (7,499 lines, 506 units)
├── knowledge_trading_systems.json        (4,454 lines, 226 units)
├── extract_books.py                      (55 lines)
└── generate_forex_knowledge.py           (1,158 lines)
```

### Book Library (Raw Sources)

```
knowledge/books/
├── _info.text                            (128 lines, rated inventory)
├── INVESTMENT_BOOKS_SUMMARY.md           (917 lines, synthesized learnings)
├── Trading.txt                           (170 lines, skill-level index)
├── [~130 PDF files]                      (varying formats)
├── [~25 EPUB files]                      (varying formats)
├── [~8 MOBI files]                       (varying formats)
├── [~1 AZW file]                         (varying format)
└── [~3 TXT/other files]                  (varying formats)
```

### Backup Knowledge (Supplementary)

```
knowledge/_backup/
├── ai_claude_bot.json                    (20 units)
├── ai_competition.json                   (10 units)
├── backtesting_deployment.json           (10 units)
├── book_behavioral_crypto_options.json   (285 units)
├── book_complete_systems.json            (200 units)
├── book_daytrading_options_misc.json     (200 units)
├── book_forex_complete.json              (335 units)
├── book_investing_fundamentals.json      (291 units)
├── book_market_wizards.json              (220 units)
├── book_patterns_candles.json            (253 units)
├── book_systems_algo.json                (250 units)
├── book_technical_analysis.json          (340 units)
├── book_trading_psychology.json          (180 units)
├── book_volume_advanced_ta.json          (250 units)
├── brian_jung.json                       (8 units)
├── cathie_wood.json                      (6 units)
├── compliance.json                       (5 units)
├── crypto_derivatives.json               (12 units)
├── crypto_fcb.json                       (22 units)
├── defi_execution.json                   (10 units)
├── execution_engineering.json            (8 units)
├── fabio_amt.json                        (15 units)
├── hybrid_scalping.json                  (8 units)
├── juvier_daytrading.json                (14 units)
├── macro_liquidity.json                  (10 units)
├── onchain_analytics.json                (15 units)
├── pradeep_ep.json                       (15 units)
├── prop_firm.json                        (7 units)
├── risk_management.json                  (14 units)
├── tjr_smc.json                          (24 units)
├── trading_psychology.json               (10 units)
├── warrior_trading.json                  (10 units)
└── wyckoff_orderflow.json                (12 units)
```

### Extraction Output

```
knowledge/extracted/
├── (empty — extraction pipeline output directory)
```

---

## Summary Statistics

| Metric | Count |
|--------|-------|
| **Total unique book titles (source material)** | ~128 |
| **Total book files** (all formats) | 168 |
| **Primary knowledge units** (10 active JSON files) | 2,959 |
| **Backup knowledge units** (33 supplementary JSON files) | 3,717 |
| **Total knowledge units** | 6,676 |
| **Unique source strings** | 548 |
| **Book-derived sources** (forex, TA, stocks, psychology, crypto) | ~183 |
| **External/online/AI-derived sources** (on-chain, DeFi, exchanges, bots) | ~365 |
| **Knowledge topics** | 15+ |
| **Market condition tags** | 11 |
| **Generation/extraction scripts** | 2 |
| **Summary/index documents** | 3 |

---

> **Note:** The 168 book files (forex, stocks, TA, psychology, systems, crypto) have already been parsed and distilled into ~6,676 structured knowledge units across 43 JSON files (10 primary + 33 backup). The `extracted/` directory is empty because extraction was done programmatically into JSON rather than plain-text dumps. The `extract_books.py` script targets 17 forex PDFs for plain-text output; `generate_forex_knowledge.py` generates forex-derived knowledge units. The knowledge base also incorporates ~365 non-book sources (AI bot research, exchange API docs, on-chain analytics from Glassnode/CryptoQuant, DeFi protocols, online courses, trader interviews) that provide the crypto-native edge beyond traditional trading literature.
