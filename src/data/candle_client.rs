use reqwest::Client;
use tracing::{debug, info};

use crate::core::error::DataError;
use crate::core::types::{Candle, OrderBook, OrderBookLevel};
use chrono::{TimeZone, Utc};

#[derive(Clone)]
pub struct CandleClient {
    client: Client,
    rest_url: String,
}

impl CandleClient {
    pub fn new(rest_url: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            rest_url: rest_url.to_string(),
        }
    }

    pub async fn get_ohlc(
        &self,
        pair: &str,
        interval_minutes: u32,
        since: Option<i64>,
    ) -> Result<Vec<Candle>, DataError> {
        let api_pair = Self::to_api_pair(pair);
        let url = format!("{}/0/public/OHLC", self.rest_url);

        let mut params = vec![
            ("pair".to_string(), api_pair),
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
            return Err(DataError::HttpError(format!("API errors: {:?}", errors)));
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
        let api_pair = Self::to_api_pair(pair);
        let url = format!("{}/0/public/Ticker", self.rest_url);

        let resp = self
            .client
            .get(&url)
            .query(&[("pair", &api_pair)])
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

    fn to_api_pair(pair: &str) -> String {
        // Map on-chain token names back to exchange API names
        // WETH → ETH, WBTC → BTC (exchanges use native names)
        let exchange_pair = match pair {
            "WETH/USD" => "ETH/USD",
            "WBTC/USD" => "BTC/USD",
            _ => pair,
        };
        exchange_pair.replace("/", "")
    }

    pub async fn get_order_book(&self, pair: &str, depth: u32) -> Result<OrderBook, DataError> {
        let api_pair = Self::to_api_pair(pair);
        let url = format!("{}/0/public/Depth", self.rest_url);

        let resp = self
            .client
            .get(&url)
            .query(&[("pair", api_pair.as_str()), ("count", &depth.to_string())])
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

    pub async fn discover_usd_pairs(&self) -> Result<Vec<String>, DataError> {
        self.discover_safe_usd_pairs(500_000.0, 0.001, &[]).await
    }

    pub async fn discover_safe_usd_pairs(
        &self,
        min_volume_24h: f64,
        min_price: f64,
        blacklisted: &[String],
    ) -> Result<Vec<String>, DataError> {
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

        let mut raw_pairs: Vec<String> = Vec::new();
        for (_key, val) in result {
            let base = match val["base"].as_str() {
                Some(b) => b,
                None => continue,
            };
            let quote = match val["quote"].as_str() {
                Some(q) => q,
                None => continue,
            };
            if base.contains('.') || quote.contains('.') {
                continue;
            }
            let clean_base = base.trim_start_matches('X').trim_start_matches('Z');
            let clean_quote = quote.trim_start_matches('X').trim_start_matches('Z');
            if clean_quote == "USD" || quote == "ZUSD" {
                raw_pairs.push(clean_base.to_string());
            }
        }

        raw_pairs.sort();
        raw_pairs.dedup();

        if raw_pairs.is_empty() {
            return Ok(Vec::new());
        }

        // Batch ticker fetch — Kraken accepts up to 20 pairs per call
        let blacklist_lower: Vec<String> = blacklisted.iter().map(|s| s.to_lowercase()).collect();
        let mut safe_pairs = Vec::new();

        for chunk in raw_pairs.chunks(20) {
            let api_names: Vec<String> = chunk
                .iter()
                .map(|s| Self::to_api_pair(&format!("{}/USD", s)))
                .collect();
            let ticker_url = format!("{}/0/public/Ticker", self.rest_url);
            let ticker_resp = self
                .client
                .get(&ticker_url)
                .query(&[("pair", api_names.join(","))])
                .send()
                .await
                .map_err(|e| DataError::HttpError(e.to_string()))?;

            let ticker_body: serde_json::Value = ticker_resp
                .json()
                .await
                .map_err(|e| DataError::ParseError(e.to_string()))?;

            let ticker_result = match ticker_body["result"].as_object() {
                Some(r) => r,
                None => continue,
            };

            for (api_name, val) in ticker_result {
                // v[1] = 24h volume, c[0] = last price
                let volume = val["v"][1]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let price = val["c"][0]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                // Find the matching base symbol from raw_pairs
                let base_sym = chunk.iter().find(|s| {
                    let api_pair = Self::to_api_pair(&format!("{}/USD", s));
                    api_pair == *api_name
                });
                let base_sym = match base_sym {
                    Some(s) => s,
                    None => continue,
                };

                // Filter: blacklist
                if blacklist_lower
                    .iter()
                    .any(|b| b == &base_sym.to_lowercase())
                {
                    continue;
                }
                // Filter: volume
                if volume < min_volume_24h {
                    continue;
                }
                // Filter: price
                if price < min_price {
                    continue;
                }

                safe_pairs.push(format!("{}/USD", base_sym));
            }
        }

        safe_pairs.sort();
        info!(
            "Safe pair discovery: {} pairs passed (min_vol=${:.0}, min_price=${:.4}, {} blacklisted)",
            safe_pairs.len(),
            min_volume_24h,
            min_price,
            blacklisted.len()
        );
        Ok(safe_pairs)
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
