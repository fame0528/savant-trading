//! Multi-source candle architecture (FID-038).
//!
//! Abstracts candle data fetching behind a `CandleSource` trait.
//! Multiple sources can be tried in priority order via `SourceRouter`.
//!
//! Sources:
//!   - KrakenSource — highest quality, 5m candles, ~20 pairs
//!   - CoinGeckoSource — broader coverage, thousands of tokens
//!   - DeFiLlamaSource — DEX-native, Arbitrum pairs

pub mod coingecko;
pub mod kraken;

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
    /// Tries each source in order. If a source returns an error or empty data,
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
                    tracing::info!(
                        "[{}] Fetched {} candles for {}",
                        source.name(),
                        candles.len(),
                        pair
                    );
                    return Ok(candles);
                }
                Ok(_) => {
                    tracing::debug!("[{}] Empty response for {}", source.name(), pair);
                }
                Err(e) => {
                    tracing::debug!("[{}] Failed for {}: {}", source.name(), pair, e);
                }
            }
        }
        Err(ExecutionError::Other(format!(
            "No candle source has data for {}",
            pair
        )))
    }
}
