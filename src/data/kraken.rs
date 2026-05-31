use reqwest::Client;
use tracing::{debug, info};

use crate::core::error::DataError;
use crate::core::types::{Candle, OrderBook, OrderBookLevel};
use chrono::{TimeZone, Utc};

pub struct KrakenClient {
    client: Client,
    rest_url: String,
}

impl KrakenClient {
    pub fn new(rest_url: &str) -> Self {
        Self {
            client: Client::new(),
            rest_url: rest_url.to_string(),
        }
    }

    pub async fn get_ohlc(
        &self,
        pair: &str,
        interval_minutes: u32,
        since: Option<i64>,
    ) -> Result<Vec<Candle>, DataError> {
        let kraken_pair = Self::to_kraken_pair(pair);
        let url = format!("{}/0/public/OHLC", self.rest_url);

        let mut params = vec![
            ("pair".to_string(), kraken_pair),
            ("interval".to_string(), interval_minutes.to_string()),
        ];
        if let Some(s) = since {
            params.push(("since".to_string(), s.to_string()));
        }

        debug!("Fetching OHLC for {} interval {}m", pair, interval_minutes);

        let resp = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| DataError::HttpError(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(e.to_string()))?;

        let errors = body["error"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !errors.is_empty() {
            return Err(DataError::HttpError(format!("Kraken errors: {:?}", errors)));
        }

        let result = body["result"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("Missing result field".into()))?;

        let mut candles = Vec::new();
        for (key, value) in result {
            if key == "last" {
                continue;
            }
            if let Some(arr) = value.as_array() {
                for item in arr {
                    if let Some(candle_arr) = item.as_array() {
                        if candle_arr.len() >= 7 {
                            let ts = candle_arr[0]
                                .as_f64()
                                .ok_or_else(|| DataError::ParseError("Invalid timestamp".into()))?;
                            candles.push(Candle {
                                timestamp: Utc.timestamp_opt(ts as i64, 0).single().ok_or_else(
                                    || DataError::ParseError("Invalid timestamp".into()),
                                )?,
                                open: candle_arr[1]
                                    .as_str()
                                    .ok_or_else(|| DataError::ParseError("Invalid open".into()))?
                                    .parse()
                                    .map_err(|_| DataError::ParseError("Invalid open".into()))?,
                                high: candle_arr[2]
                                    .as_str()
                                    .ok_or_else(|| DataError::ParseError("Invalid high".into()))?
                                    .parse()
                                    .map_err(|_| DataError::ParseError("Invalid high".into()))?,
                                low: candle_arr[3]
                                    .as_str()
                                    .ok_or_else(|| DataError::ParseError("Invalid low".into()))?
                                    .parse()
                                    .map_err(|_| DataError::ParseError("Invalid low".into()))?,
                                close: candle_arr[4]
                                    .as_str()
                                    .ok_or_else(|| DataError::ParseError("Invalid close".into()))?
                                    .parse()
                                    .map_err(|_| DataError::ParseError("Invalid close".into()))?,
                                volume: candle_arr[6]
                                    .as_str()
                                    .ok_or_else(|| DataError::ParseError("Invalid volume".into()))?
                                    .parse()
                                    .map_err(|_| DataError::ParseError("Invalid volume".into()))?,
                                pair: pair.to_string(),
                            });
                        }
                    }
                }
            }
        }

        candles.sort_by_key(|c| c.timestamp);
        info!("Fetched {} candles for {}", candles.len(), pair);
        Ok(candles)
    }

    pub async fn get_ticker(&self, pair: &str) -> Result<TickerData, DataError> {
        let kraken_pair = Self::to_kraken_pair(pair);
        let url = format!("{}/0/public/Ticker", self.rest_url);

        let resp = self
            .client
            .get(&url)
            .query(&[("pair", &kraken_pair)])
            .send()
            .await
            .map_err(|e| DataError::HttpError(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(e.to_string()))?;

        let result = body["result"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("Missing result".into()))?;

        let (_, ticker_val) = result
            .iter()
            .next()
            .ok_or_else(|| DataError::NoData("No ticker data".into()))?;

        let ask = ticker_val["a"][0]
            .as_str()
            .ok_or_else(|| DataError::ParseError("Invalid ask".into()))?
            .parse::<f64>()
            .map_err(|_| DataError::ParseError("Invalid ask".into()))?;
        let bid = ticker_val["b"][0]
            .as_str()
            .ok_or_else(|| DataError::ParseError("Invalid bid".into()))?
            .parse::<f64>()
            .map_err(|_| DataError::ParseError("Invalid bid".into()))?;
        let last = ticker_val["c"][0]
            .as_str()
            .ok_or_else(|| DataError::ParseError("Invalid last".into()))?
            .parse::<f64>()
            .map_err(|_| DataError::ParseError("Invalid last".into()))?;
        let volume: f64 = ticker_val["v"][1]
            .as_str()
            .ok_or_else(|| DataError::ParseError("Invalid volume".into()))?
            .parse()
            .map_err(|_| DataError::ParseError("Invalid volume".into()))?;

        Ok(TickerData {
            pair: pair.to_string(),
            ask,
            bid,
            last,
            volume,
        })
    }

    fn to_kraken_pair(pair: &str) -> String {
        pair.replace("/", "")
    }

    pub async fn get_order_book(&self, pair: &str, depth: u32) -> Result<OrderBook, DataError> {
        let kraken_pair = Self::to_kraken_pair(pair);
        let url = format!("{}/0/public/Depth", self.rest_url);

        let resp = self
            .client
            .get(&url)
            .query(&[
                ("pair", kraken_pair.as_str()),
                ("count", &depth.to_string()),
            ])
            .send()
            .await
            .map_err(|e| DataError::HttpError(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(e.to_string()))?;

        let result = body["result"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("Missing result field".into()))?;

        let (_, book_val) = result
            .iter()
            .next()
            .ok_or_else(|| DataError::NoData("No order book data".into()))?;

        let asks_arr = book_val["asks"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("Missing asks".into()))?;
        let bids_arr = book_val["bids"]
            .as_array()
            .ok_or_else(|| DataError::ParseError("Missing bids".into()))?;

        let parse_level = |arr: &serde_json::Value| -> Option<OrderBookLevel> {
            let items = arr.as_array()?;
            if items.len() < 2 {
                return None;
            }
            let price = items[0].as_str()?.parse::<f64>().ok()?;
            let volume = items[1].as_str()?.parse::<f64>().ok()?;
            Some(OrderBookLevel { price, volume })
        };

        let asks: Vec<OrderBookLevel> = asks_arr.iter().filter_map(parse_level).collect();
        let bids: Vec<OrderBookLevel> = bids_arr.iter().filter_map(parse_level).collect();

        debug!(
            "Order book for {}: {} bids, {} asks",
            pair,
            bids.len(),
            asks.len()
        );

        Ok(OrderBook {
            pair: pair.to_string(),
            bids,
            asks,
            timestamp: Utc::now(),
        })
    }

    /// Discover all USD pairs available on Kraken.
    /// Returns pairs in "BTC/USD" format, filtered to active spot pairs only.
    pub async fn discover_usd_pairs(&self) -> Result<Vec<String>, DataError> {
        let url = format!("{}/0/public/AssetPairs", self.rest_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("quote", "ZUSD")])
            .send()
            .await
            .map_err(|e| DataError::HttpError(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(e.to_string()))?;

        let result = body["result"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("Missing result".into()))?;

        let mut pairs: Vec<String> = Vec::new();
        for (_key, val) in result {
            let base = match val["base"].as_str() {
                Some(b) => b,
                None => continue,
            };
            let quote = match val["quote"].as_str() {
                Some(q) => q,
                None => continue,
            };
            // Skip leveraged tokens, staking derivatives, and .d dark pool pairs
            if base.contains('.') || quote.contains('.') {
                continue;
            }
            // Skip pairs with "XX" or "XZ" Kraken prefix artifacts
            let clean_base = base.trim_start_matches("X").trim_start_matches("Z");
            let clean_quote = quote.trim_start_matches("X").trim_start_matches("Z");
            if clean_quote == "USD" || quote == "ZUSD" {
                pairs.push(format!("{}/USD", clean_base));
            }
        }

        pairs.sort();
        pairs.dedup();
        info!("Discovered {} USD pairs on Kraken", pairs.len());
        Ok(pairs)
    }
}

#[derive(Debug, Clone)]
pub struct TickerData {
    pub pair: String,
    pub ask: f64,
    pub bid: f64,
    pub last: f64,
    pub volume: f64,
}
