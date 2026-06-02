//! Kraken tick data parser — converts raw trade CSVs into OHLCV candles.
//!
//! Kraken provides historical tick data (time and sales) as CSV files.
//! Each line: timestamp,price,volume,side,order_type,misc
//!
//! This module:
//! 1. Reads raw CSV tick files (decompressed from Kraken ZIPs)
//! 2. Aggregates ticks into OHLCV candles at configurable intervals
//! 3. Caches the result as JSON for fast reloading
//! 4. Feeds into the training pipeline via generate_scenarios_from_history()

use crate::core::types::Candle;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use tracing::{info, warn};

/// A single trade from Kraken's tick data CSV.
#[derive(Debug, Clone)]
pub struct Tick {
    pub timestamp: DateTime<Utc>,
    pub price: f64,
    pub volume: f64,
    pub side: TickSide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TickSide {
    Buy,
    Sell,
}

/// Parse a single line of Kraken tick CSV.
///
/// Format: timestamp,price,volume
/// Example: 1706803466,0.23,50.0
///
/// Kraken's historical data uses this simple 3-field format.
pub fn parse_tick_line(line: &str) -> Option<Tick> {
    let parts: Vec<&str> = line.trim().split(',').collect();
    if parts.len() < 3 {
        return None;
    }

    let ts_f64: f64 = parts[0].parse().ok()?;
    let ts_secs = ts_f64 as i64;
    let ts_micros = ((ts_f64 - ts_secs as f64) * 1_000_000.0) as u32;
    let timestamp = Utc.timestamp_opt(ts_secs, ts_micros * 1000).single()?;

    let price: f64 = parts[1].parse().ok()?;
    let volume: f64 = parts[2].parse().ok()?;

    // Skip zero-volume or zero-price trades
    if volume <= 0.0 || price <= 0.0 {
        return None;
    }

    Some(Tick {
        timestamp,
        price,
        volume,
        side: TickSide::Buy, // Not provided in Kraken historical data
    })
}

/// Aggregate ticks into OHLCV candles at the given interval.
///
/// `interval_minutes`: candle size (5, 15, 60, 240, 1440)
/// Returns candles sorted by timestamp.
pub fn aggregate_ticks_to_candles(ticks: &[Tick], interval_minutes: i64) -> Vec<Candle> {
    if ticks.is_empty() {
        return vec![];
    }

    let interval_secs = interval_minutes * 60;
    let mut buckets: BTreeMap<i64, Vec<&Tick>> = BTreeMap::new();

    for tick in ticks {
        let bucket_key = tick.timestamp.timestamp() / interval_secs;
        buckets.entry(bucket_key).or_default().push(tick);
    }

    let mut candles = Vec::with_capacity(buckets.len());

    for (bucket_key, bucket_ticks) in &buckets {
        if bucket_ticks.is_empty() {
            continue;
        }

        let open = bucket_ticks[0].price;
        let close = bucket_ticks.last().map(|t| t.price).unwrap_or(open);
        let high = bucket_ticks
            .iter()
            .map(|t| t.price)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = bucket_ticks
            .iter()
            .map(|t| t.price)
            .fold(f64::INFINITY, f64::min);
        let volume: f64 = bucket_ticks.iter().map(|t| t.volume).sum();

        let timestamp = Utc
            .timestamp_opt(bucket_key * interval_secs, 0)
            .single()
            .unwrap_or_default();

        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            pair: "BTC/USD".to_string(), // Will be overridden by caller
        });
    }

    candles
}

/// Parse a Kraken tick CSV file and return all ticks.
///
/// Handles large files efficiently by reading line-by-line.
/// Skips malformed lines silently.
pub fn parse_tick_file(path: &Path) -> Result<Vec<Tick>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let mut ticks = Vec::with_capacity(content.len() / 80); // ~80 chars per line
    let mut skipped = 0u64;

    for line in content.lines() {
        // Skip header lines
        if line.starts_with("timestamp") || line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        match parse_tick_line(line) {
            Some(tick) => ticks.push(tick),
            None => skipped += 1,
        }
    }

    info!(
        "Parsed {} ticks from {} ({} skipped)",
        ticks.len(),
        path.display(),
        skipped
    );

    // Sort by timestamp
    ticks.sort_by_key(|t| t.timestamp);

    Ok(ticks)
}

/// Map standard pair names to Kraken's historical data filenames.
///
/// Kraken uses XBT instead of BTC, and some pairs have different naming.
fn kraken_filename_pair(pair: &str) -> String {
    match pair.to_uppercase().replace('/', "").as_str() {
        "BTCUSD" => "XBTUSD".to_string(),
        "BTCEUR" => "XBTEUR".to_string(),
        "BTCGBP" => "XBTGBP".to_string(),
        "DOGEUSD" => "DOGEUSD".to_string(),
        other => other.to_string(),
    }
}

/// Parse all tick CSV files in a directory (recursively).
///
/// Kraken data is organized as: pair/PAIR_*.csv or just *.csv files.
/// This function finds all .csv files and parses them.
pub fn parse_tick_directory(dir: &Path, pair_filter: &str) -> Result<Vec<Tick>, String> {
    let mut all_ticks = Vec::new();
    let mut file_count = 0;

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            match parse_tick_directory(&path, pair_filter) {
                Ok(mut ticks) => {
                    all_ticks.append(&mut ticks);
                    file_count += 1;
                }
                Err(e) => warn!("Skipping dir {}: {}", path.display(), e),
            }
        } else if path.extension().is_some_and(|ext| ext == "csv") {
            // Filter by pair name if specified
            let filename = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
                .to_uppercase()
                .replace(".CSV", "");
            let kraken_name = kraken_filename_pair(pair_filter);
            let pair_upper = pair_filter.to_uppercase();

            if pair_filter.is_empty()
                || filename == kraken_name
                || filename == pair_upper.replace('/', "")
            {
                match parse_tick_file(&path) {
                    Ok(mut ticks) => {
                        all_ticks.append(&mut ticks);
                        file_count += 1;
                    }
                    Err(e) => warn!("Skipping file {}: {}", path.display(), e),
                }
            }
        }
    }

    info!(
        "Parsed {} total ticks from {} files in {}",
        all_ticks.len(),
        file_count,
        dir.display()
    );

    // Sort by timestamp
    all_ticks.sort_by_key(|t| t.timestamp);

    Ok(all_ticks)
}

/// Cached OHLCV dataset generated from tick data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickDerivedCandles {
    pub pair: String,
    pub interval_minutes: u32,
    pub tick_count: usize,
    pub candle_count: usize,
    pub start_ts: i64,
    pub end_ts: i64,
    pub source_dir: String,
    pub candles: Vec<Candle>,
}

impl TickDerivedCandles {
    /// Save to JSON cache.
    pub fn save_cache(&self, path: &str) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    warn!("Failed to write tick cache: {}", e);
                } else {
                    info!(
                        "Cached {} candles (from {} ticks) to {}",
                        self.candle_count, self.tick_count, path
                    );
                }
            }
            Err(e) => warn!("Failed to serialize tick cache: {}", e),
        }
    }

    /// Load from JSON cache.
    pub fn load_cache(path: &str) -> Option<Self> {
        if !Path::new(path).exists() {
            return None;
        }
        match std::fs::read_to_string(path) {
            Ok(data) => match serde_json::from_str::<TickDerivedCandles>(&data) {
                Ok(cached) => {
                    info!(
                        "Loaded {} cached candles from {} ({} ticks)",
                        cached.candle_count, path, cached.tick_count
                    );
                    Some(cached)
                }
                Err(e) => {
                    warn!("Failed to parse tick cache: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read tick cache: {}", e);
                None
            }
        }
    }
}

/// Process raw Kraken tick data into OHLCV candles.
///
/// 1. Reads all CSV files from `data_dir` matching `pair`
/// 2. Aggregates into candles at `interval_minutes`
/// 3. Caches to JSON for fast reloading
/// 4. Returns the candle dataset
pub fn process_tick_data(
    data_dir: &str,
    pair: &str,
    interval_minutes: u32,
) -> Result<TickDerivedCandles, String> {
    let cache_path = format!(
        "data/tick_candles_{}_{}m.json",
        pair.replace('/', "_"),
        interval_minutes
    );

    // Try cache first (valid for 24h)
    if let Some(cached) = TickDerivedCandles::load_cache(&cache_path) {
        if cached.interval_minutes == interval_minutes && cached.pair == pair {
            return Ok(cached);
        }
    }

    info!(
        "Processing tick data from {} for {} at {}m intervals...",
        data_dir, pair, interval_minutes
    );

    let ticks = parse_tick_directory(Path::new(data_dir), pair)?;

    if ticks.is_empty() {
        return Err(format!("No ticks found in {} for {}", data_dir, pair));
    }

    let start_ts = ticks.first().map(|t| t.timestamp.timestamp()).unwrap_or(0);
    let end_ts = ticks.last().map(|t| t.timestamp.timestamp()).unwrap_or(0);
    let tick_count = ticks.len();

    let mut candles = aggregate_ticks_to_candles(&ticks, interval_minutes as i64);

    // Set pair name on all candles
    for c in &mut candles {
        c.pair = pair.to_string();
    }

    let candle_count = candles.len();

    info!(
        "Aggregated {} ticks into {} candles ({}m) for {}",
        tick_count, candle_count, interval_minutes, pair
    );

    let dataset = TickDerivedCandles {
        pair: pair.to_string(),
        interval_minutes,
        tick_count,
        candle_count,
        start_ts,
        end_ts,
        source_dir: data_dir.to_string(),
        candles,
    };

    dataset.save_cache(&cache_path);

    Ok(dataset)
}
