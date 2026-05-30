//! Live market insight — external data sources for AI context.
//!
//! All data sources are free, no API keys required (Kraken Futures, CoinGecko,
//! alternative.me, blockchain.info, RSS feeds).
//!
//! - `sentiment` — Fear & Greed Index, BTC Dominance (CoinGecko, alternative.me)
//! - `funding_rates` — Derivatives funding rates, open interest (Kraken Futures)
//! - `liquidation` — Liquidation risk derived from futures data (Kraken Futures)
//! - `flows` — On-chain data: block height, mempool, tx count (blockchain.info)
//! - `news` — RSS feeds (8 sources) + CoinGecko trending
//! - `rss` — RSS feed fetcher and parser (quick-xml)
//! - `aggregator` — Unified MarketContext combining all sources

pub mod aggregator;
pub mod flows;
pub mod funding_rates;
pub mod liquidation;
pub mod news;
pub mod rss;
pub mod sentiment;

pub use aggregator::{InsightAggregator, MarketContext};
