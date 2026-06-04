//! Multi-source candle architecture (FID-038).
//!
//! Abstracts candle data fetching behind a `CandleSource` trait.
//! Multiple sources can be tried in priority order via `SourceRouter`.
//!
//! Sources:
//!   - KrakenSource — highest quality, 5m candles, ~20 pairs
//!   - CoinGeckoSource — broader coverage, thousands of tokens
//!   - DeFiLlamaSource — DEX-native, Arbitrum pairs

pub mod binance;
pub mod bybit;
pub mod cmc;
pub mod coingecko;
pub mod cryptocompare;
pub mod dexscreener;
pub mod gate;
pub mod geckoterminal;
pub mod kraken;
pub mod kucoin;
pub mod okx;

use async_trait::async_trait;
use crate::core::types::Candle;
use crate::core::error::ExecutionError;

/// Abstract candle data source.
///
/// Implementations fetch OHLCV candle data from various providers.
/// The `SourceRouter` tries sources in priority order.
#[async_trait]
pub trait CandleSource: Send + Sync {
    /// Human-readable source name.
    fn name(&self) -> &str;

    /// Fetch candles for a pair at a given timeframe.
    ///
    /// Returns `Ok(candles)` on success, `Err` if the source doesn't have
    /// the pair or the request failed.
    async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError>;

    /// Check if this source likely has data for a given pair.
    ///
    /// Used by SourceRouter to skip sources that definitely don't have the pair.
    /// Returns `true` if the source might have data, `false` if it definitely doesn't.
    fn might_have(&self, pair: &str) -> bool;
}

/// Router that tries multiple candle sources in priority order.
///
/// Usage:
/// ```rust,no_run
/// use savant_trading::data::sources::{SourceRouter, CandleSource};
/// // Sources are tried in order until one succeeds
/// ```
pub struct SourceRouter {
    sources: Vec<Box<dyn CandleSource>>,
}

impl SourceRouter {
    pub fn new(sources: Vec<Box<dyn CandleSource>>) -> Self {
        Self { sources }
    }

    /// Fetch candles from the first source that has them.
    ///
    /// Tries each source in order. If a source returns an error, empty data,
    /// or all-zero candles (e.g. Kraken returning zeros for unsupported tokens),
    /// tries the next source. Returns an error only if ALL sources fail.
    pub async fn fetch_candles(
        &self,
        pair: &str,
        timeframe_minutes: u32,
        count: u32,
    ) -> Result<Vec<Candle>, ExecutionError> {
        for source in &self.sources {
            if !source.might_have(pair) {
                continue;
            }
            match source.fetch_candles(pair, timeframe_minutes, count).await {
                Ok(candles) if !candles.is_empty() => {
                    // FID-044: Reject all-zero candle responses (e.g. Kraken returning
                    // zeros for tokens it doesn't support). These are not real data.
                    let nonzero = candles.iter().filter(|c| c.close > 0.0).count();
                    if nonzero == 0 {
                        tracing::debug!(
                            "{}: All-zero candles for \x1b[90m[{}]\x1b[36m",
                            source.name(),
                            pair
                        );
                        continue;
                    }
                    tracing::info!(
                        "{}: \x1b[90m[{}]\x1b[36m {} candles ({} non-zero)",
                        source.name(),
                        pair,
                        candles.len(),
                        nonzero
                    );
                    return Ok(candles);
                }
                Ok(_) => {
                    tracing::info!(
                        "{}: No data for \x1b[90m[{}]\x1b[36m",
                        source.name(),
                        pair
                    );
                }
                Err(_) => {
                    tracing::info!(
                        "{}: No data for \x1b[90m[{}]\x1b[36m",
                        source.name(),
                        pair
                    );
                }
            }
        }
        Err(ExecutionError::Other(format!(
            "No candle source has data for {}",
            pair
        )))
    }
}
