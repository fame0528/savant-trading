//! Live market insight — external data sources for AI context.
//!
//! - `sentiment` — Fear & Greed Index, BTC Dominance
//! - `funding_rates` — Derivatives funding rates, open interest, long/short ratio
//! - `liquidation` — Liquidation clusters and heatmaps
//! - `flows` — Exchange inflow/outflow
//! - `news` — Breaking news and social sentiment
//! - `aggregator` — Unified MarketContext combining all sources

pub mod aggregator;
pub mod flows;
pub mod funding_rates;
pub mod liquidation;
pub mod news;
pub mod sentiment;

pub use aggregator::{InsightAggregator, MarketContext};
