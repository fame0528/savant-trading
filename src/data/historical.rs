//! Historical data fetcher and cache for training on real market data.
//!
//! Fetches OHLCV candles from Kraken API and caches to local JSON files.
//! Training on real historical data (2021 blow-off, 2022 capitulation, etc.)
//! produces better calibration than synthetic scenarios.

use crate::core::types::Candle;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

/// Metadata about a cached historical dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMeta {
    pub pair: String,
    pub interval_minutes: u32,
    pub start_ts: i64,
    pub end_ts: i64,
    pub candle_count: usize,
    pub fetched_at: String,
}

/// A cached historical dataset with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataset {
    pub meta: HistoricalMeta,
    pub candles: Vec<Candle>,
}

impl HistoricalDataset {
    /// Load from cache file if it exists and is fresh.
    pub fn load_from_cache(pair: &str, interval_minutes: u32) -> Option<Self> {
        let path = cache_path(pair, interval_minutes);
        if !Path::new(&path).exists() {
            return None;
        }

        match std::fs::read_to_string(&path) {
            Ok(data) => match serde_json::from_str::<HistoricalDataset>(&data) {
                Ok(dataset) => {
                    info!(
                        "Loaded {} cached candles for {} ({}m)",
                        dataset.candles.len(),
                        pair,
                        interval_minutes
                    );
                    Some(dataset)
                }
                Err(e) => {
                    warn!("Failed to parse cached historical data: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read cached historical data: {}", e);
                None
            }
        }
    }

    /// Save to cache file.
    pub fn save_to_cache(&self) {
        let path = cache_path(&self.meta.pair, self.meta.interval_minutes);
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    warn!("Failed to write historical cache: {}", e);
                } else {
                    info!(
                        "Cached {} candles for {} to {}",
                        self.candles.len(),
                        self.meta.pair,
                        path
                    );
                }
            }
            Err(e) => warn!("Failed to serialize historical data: {}", e),
        }
    }
}

fn cache_path(pair: &str, interval_minutes: u32) -> String {
    let safe_pair = pair.replace('/', "_");
    format!("data/historical_{}_{}m.json", safe_pair, interval_minutes)
}

/// Fetch historical candles from Kraken, paginating backwards from `end`.
///
/// Kraken OHLC API returns max 720 candles per request. For 30 days of 5m data
/// (~8,640 candles), we need ~12 paginated requests.
///
/// Rate limit: 1 request per second for public endpoints.
pub async fn fetch_historical(
    client: &crate::data::candle_client::CandleClient,
    pair: &str,
    interval_minutes: u32,
    days: u32,
) -> Result<HistoricalDataset, String> {
    let end = Utc::now();
    let start = end - Duration::days(days as i64);
    let start_ts = start.timestamp();

    info!(
        "Fetching {} days of {}m candles for {} ({} to {})",
        days,
        interval_minutes,
        pair,
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d")
    );

    let mut all_candles: Vec<Candle> = Vec::new();
    let mut since: Option<i64> = Some(start_ts);
    let mut request_count = 0;
    let max_requests = 50; // Safety limit

    loop {
        if request_count >= max_requests {
            warn!("Hit max requests ({}) for historical fetch", max_requests);
            break;
        }

        let batch = client
            .get_ohlc(pair, interval_minutes, since)
            .await
            .map_err(|e| format!("Kraken OHLC error: {}", e))?;

        if batch.is_empty() {
            break;
        }

        let batch_len = batch.len();
        let last_ts = batch.last().map(|c| c.timestamp.timestamp()).unwrap_or(0);

        all_candles.extend(batch);
        request_count += 1;

        info!(
            "Fetched batch {}: {} candles (total: {})",
            request_count,
            batch_len,
            all_candles.len()
        );

        // If we got fewer than 720 candles (Kraken max), we've reached the end
        if batch_len < 720 {
            break;
        }

        // Move `since` past the last candle we received
        since = Some(last_ts + 1);

        // Rate limit: 1 req/sec for public endpoints
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // Sort by timestamp and deduplicate
    all_candles.sort_by_key(|c| c.timestamp);
    all_candles.dedup_by_key(|c| c.timestamp);

    // Filter to requested time range
    all_candles.retain(|c| c.timestamp.timestamp() >= start_ts);

    info!(
        "Historical fetch complete: {} candles for {} ({} requests)",
        all_candles.len(),
        pair,
        request_count
    );

    let dataset = HistoricalDataset {
        meta: HistoricalMeta {
            pair: pair.to_string(),
            interval_minutes,
            start_ts,
            end_ts: end.timestamp(),
            candle_count: all_candles.len(),
            fetched_at: Utc::now().to_rfc3339(),
        },
        candles: all_candles,
    };

    dataset.save_to_cache();
    Ok(dataset)
}

/// Get historical data — load from cache or fetch fresh.
///
/// Cache is considered fresh if it's less than 24 hours old and covers
/// the requested time range.
pub async fn get_historical(
    client: &crate::data::candle_client::CandleClient,
    pair: &str,
    interval_minutes: u32,
    days: u32,
) -> Result<HistoricalDataset, String> {
    // Try cache first
    if let Some(cached) = HistoricalDataset::load_from_cache(pair, interval_minutes) {
        // Check if cache is fresh (< 24 hours old)
        if let Ok(fetched_at) = DateTime::parse_from_rfc3339(&cached.meta.fetched_at) {
            let age = Utc::now() - fetched_at.with_timezone(&Utc);
            if age < Duration::hours(24) {
                info!(
                    "Using cached historical data (age: {} hours)",
                    age.num_hours()
                );
                return Ok(cached);
            }
        }
        info!("Cache stale, fetching fresh data");
    }

    fetch_historical(client, pair, interval_minutes, days).await
}

/// Get the best available historical data — tick data first, then API.
///
/// Priority order:
/// 1. Tick-derived candles (from 12GB Kraken download) — most comprehensive
/// 2. Cached API candles (from Kraken OHLC endpoint) — limited to 720 candles
/// 3. Fresh API fetch — limited to 720 candles
///
/// This function is the single entry point for training data.
pub async fn get_best_historical(
    client: &crate::data::candle_client::CandleClient,
    pair: &str,
    interval_minutes: u32,
) -> Result<HistoricalDataset, String> {
    // 1. Try tick-derived candles (from processed Kraken tick CSVs)
    let tick_cache = format!(
        "data/tick_candles_{}_{}m.json",
        pair.replace('/', "_"),
        interval_minutes
    );
    if let Some(tick_data) = crate::data::tick_data::TickDerivedCandles::load_cache(&tick_cache) {
        info!(
            "Using tick-derived candles: {} candles from {} ticks ({} to {})",
            tick_data.candle_count,
            tick_data.tick_count,
            chrono::DateTime::from_timestamp(tick_data.start_ts, 0)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "?".into()),
            chrono::DateTime::from_timestamp(tick_data.end_ts, 0)
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "?".into()),
        );
        return Ok(HistoricalDataset {
            meta: HistoricalMeta {
                pair: pair.to_string(),
                interval_minutes,
                start_ts: tick_data.start_ts,
                end_ts: tick_data.end_ts,
                candle_count: tick_data.candle_count,
                fetched_at: chrono::Utc::now().to_rfc3339(),
            },
            candles: tick_data.candles,
        });
    }

    // 2. Try cached API candles
    if let Some(cached) = HistoricalDataset::load_from_cache(pair, interval_minutes) {
        if let Ok(fetched_at) = DateTime::parse_from_rfc3339(&cached.meta.fetched_at) {
            let age = Utc::now() - fetched_at.with_timezone(&Utc);
            if age < Duration::hours(24) {
                info!(
                    "Using cached API candles (age: {} hours, {} candles)",
                    age.num_hours(),
                    cached.meta.candle_count
                );
                return Ok(cached);
            }
        }
    }

    // 3. Fetch fresh from API
    info!("No tick data or cache found — fetching from Kraken API (720 candle limit)");
    fetch_historical(client, pair, interval_minutes, 30).await
}

/// Generate synthetic scenarios from historical candle windows.
///
/// Takes windows of `scenario_len` candles from the historical data.
/// Each window becomes a training scenario with the LAST candle as the
/// decision point (what the agent should predict).
pub fn generate_scenarios_from_history(
    dataset: &HistoricalDataset,
    scenario_len: usize,
    stride: usize,
) -> Vec<HistoricalScenario> {
    let candles = &dataset.candles;
    if candles.len() < scenario_len + 10 {
        return vec![];
    }

    let mut scenarios = Vec::new();
    let mut i = 0;

    while i + scenario_len + 10 < candles.len() {
        let context_candles = candles[i..i + scenario_len].to_vec();
        let future_candles =
            candles[i + scenario_len..std::cmp::min(i + scenario_len + 10, candles.len())].to_vec();

        // Determine what happened: did price go up or down after the decision point?
        let decision_price = context_candles.last().map(|c| c.close).unwrap_or(0.0);
        let future_close = future_candles
            .last()
            .map(|c| c.close)
            .unwrap_or(decision_price);
        let pct_change = (future_close - decision_price) / decision_price;

        let outcome = if pct_change > 0.005 {
            "Buy"
        } else if pct_change < -0.005 {
            "Sell"
        } else {
            "Hold"
        };

        scenarios.push(HistoricalScenario {
            id: format!("HIST-{:04}", scenarios.len()),
            context_candles,
            future_candles,
            expected_action: outcome.to_string(),
            pct_change,
            decision_price,
        });

        i += stride;
    }

    info!(
        "Generated {} historical scenarios from {} candles (stride={})",
        scenarios.len(),
        candles.len(),
        stride
    );

    scenarios
}

/// A training scenario derived from historical market data.
#[derive(Debug, Clone)]
pub struct HistoricalScenario {
    pub id: String,
    pub context_candles: Vec<Candle>,
    pub future_candles: Vec<Candle>,
    pub expected_action: String,
    pub pct_change: f64,
    pub decision_price: f64,
}
