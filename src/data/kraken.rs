use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use tracing::{debug, info};

use crate::core::error::DataError;
use crate::core::types::{Candle, OrderBook, OrderBookLevel};
use chrono::{TimeZone, Utc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct KrakenClient {
    client: Client,
    rest_url: String,
    api_key: Option<String>,
    api_secret: Option<String>,
    nonce: Arc<AtomicU64>,
}

impl KrakenClient {
    pub fn new(rest_url: &str) -> Self {
        Self {
            client: Client::new(),
            rest_url: rest_url.to_string(),
            api_key: None,
            api_secret: None,
            nonce: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            )),
        }
    }

    /// Create a client with private API credentials.
    pub fn with_credentials(rest_url: &str, api_key: &str, api_secret: &str) -> Self {
        Self {
            client: Client::new(),
            rest_url: rest_url.to_string(),
            api_key: Some(api_key.to_string()),
            api_secret: Some(api_secret.to_string()),
            nonce: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            )),
        }
    }

    /// Generate HMAC-SHA512 signature for Kraken private API.
    ///
    /// Kraken's签名 scheme:
    /// 1. SHA256(nonce + POST data) → sha256_digest
    /// 2. HMAC-SHA512(url_path, sha256_digest) using base64-decoded secret
    /// 3. Base64 encode the result
    fn sign(&self, url_path: &str, post_data: &str) -> Result<String, DataError> {
        let secret = self
            .api_secret
            .as_ref()
            .ok_or_else(|| DataError::HttpError("No API secret configured".into()))?;

        let nonce_str = self.nonce.fetch_add(1, Ordering::SeqCst).to_string();

        // Step 1: SHA256(nonce + post_data)
        let mut sha256 = Sha256::new();
        sha256.update(format!("{}{}", nonce_str, post_data).as_bytes());
        let sha256_digest = sha256.finalize();

        // Step 2: HMAC-SHA512(url_path, sha256_digest)
        let secret_bytes = general_purpose::STANDARD
            .decode(secret)
            .map_err(|e| DataError::HttpError(format!("Invalid API secret: {}", e)))?;
        let mut hmac = Hmac::<Sha512>::new_from_slice(&secret_bytes)
            .map_err(|e| DataError::HttpError(format!("HMAC error: {}", e)))?;
        hmac.update(url_path.as_bytes());
        hmac.update(&sha256_digest);
        let hmac_result = hmac.finalize();

        // Step 3: Base64 encode
        Ok(general_purpose::STANDARD.encode(hmac_result.into_bytes()))
    }

    /// Make a private API request to Kraken.
    ///
    /// Adds nonce, signs the request, and sends it.
    async fn private_request(
        &self,
        endpoint: &str,
        mut params: HashMap<String, String>,
    ) -> Result<serde_json::Value, DataError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| DataError::HttpError("No API key configured".into()))?;

        let nonce = self.nonce.load(Ordering::SeqCst).to_string();
        params.insert("nonce".to_string(), nonce.clone());

        let url_path = format!("/0/private/{}", endpoint);
        let url = format!("{}{}", self.rest_url, url_path);

        // Build POST body
        let post_body: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Sign
        let signature = self.sign(&url_path, &post_body)?;

        debug!("Kraken private request: {} (nonce: {})", endpoint, nonce);

        let resp = self
            .client
            .post(&url)
            .header("API-Key", api_key)
            .header("API-Sign", signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_body)
            .send()
            .await
            .map_err(|e| DataError::HttpError(format!("Kraken API error: {}", e)))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DataError::ParseError(format!("Kraken parse error: {}", e)))?;

        // Check for API errors
        let errors = body["error"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !errors.is_empty() {
            return Err(DataError::HttpError(format!(
                "Kraken API errors: {:?}",
                errors
            )));
        }

        Ok(body)
    }

    /// Place an order on Kraken.
    ///
    /// Returns the order ID(s) from Kraken.
    ///
    /// - `pair`: Trading pair (e.g., "BTC/USD")
    /// - `side`: "buy" or "sell"
    /// - `order_type`: "market", "limit", "stop-loss", "take-profit"
    /// - `volume`: Order volume in base currency
    /// - `price`: Price for limit/stop orders (None for market)
    /// - `stop_price`: Stop price for stop orders
    pub async fn add_order(
        &self,
        pair: &str,
        side: &str,
        order_type: &str,
        volume: f64,
        price: Option<f64>,
        stop_price: Option<f64>,
    ) -> Result<Vec<String>, DataError> {
        let kraken_pair = Self::to_kraken_pair(pair);
        let mut params = HashMap::new();
        params.insert("pair".to_string(), kraken_pair);
        params.insert("type".to_string(), side.to_string());
        params.insert("ordertype".to_string(), order_type.to_string());
        params.insert("volume".to_string(), format!("{}", volume));

        if let Some(p) = price {
            params.insert("price".to_string(), format!("{}", p));
        }
        if let Some(sp) = stop_price {
            params.insert("price2".to_string(), format!("{}", sp));
        }

        // Validate: userref for tracking
        let userref = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        params.insert("userref".to_string(), userref.to_string());

        // Post-only for limit orders (maker fee)
        if order_type == "limit" {
            params.insert("oflags".to_string(), "post".to_string());
        }

        let body = self.private_request("AddOrder", params).await?;

        let txids = body["result"]["txid"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        info!(
            "Kraken order placed: {} {} {} {} → txid={:?}",
            side, volume, pair, order_type, txids
        );

        Ok(txids)
    }

    /// Cancel an order by txid.
    pub async fn cancel_order(&self, txid: &str) -> Result<bool, DataError> {
        let mut params = HashMap::new();
        params.insert("txid".to_string(), txid.to_string());

        let body = self.private_request("CancelOrder", params).await?;

        let count = body["result"]["count"].as_u64().unwrap_or(0);

        info!("Kraken cancel order {}: count={}", txid, count);
        Ok(count > 0)
    }

    /// Cancel all open orders.
    pub async fn cancel_all_orders(&self) -> Result<u64, DataError> {
        let body = self.private_request("CancelAll", HashMap::new()).await?;

        let count = body["result"]["count"].as_u64().unwrap_or(0);

        info!("Kraken cancel all orders: count={}", count);
        Ok(count)
    }

    /// Get account balance.
    ///
    /// Returns a map of currency → balance.
    pub async fn get_balance(&self) -> Result<HashMap<String, f64>, DataError> {
        let body = self.private_request("Balance", HashMap::new()).await?;

        let result = body["result"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("Missing result".into()))?;

        let mut balances = HashMap::new();
        for (currency, value) in result {
            if let Some(balance) = value.as_str().and_then(|s| s.parse::<f64>().ok()) {
                if balance > 0.0 {
                    balances.insert(currency.clone(), balance);
                }
            }
        }

        debug!("Kraken balance: {:?}", balances);
        Ok(balances)
    }

    /// Get trade balance (equity, margin, etc.).
    ///
    /// Returns (equity, trade_balance, margin_used, unrealized_pnl).
    pub async fn get_trade_balance(&self, asset: &str) -> Result<(f64, f64, f64, f64), DataError> {
        let mut params = HashMap::new();
        params.insert("asset".to_string(), asset.to_string());

        let body = self.private_request("TradeBalance", params).await?;

        let result = &body["result"];
        let equity = result["eb"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let trade_balance = result["tb"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let margin = result["m"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let unrealized = result["n"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        debug!(
            "Kraken trade balance: equity={:.2} balance={:.2} margin={:.2} unrealized={:.2}",
            equity, trade_balance, margin, unrealized
        );

        Ok((equity, trade_balance, margin, unrealized))
    }

    /// Get open orders.
    ///
    /// Returns a map of txid → order info.
    pub async fn get_open_orders(&self) -> Result<Vec<KrakenOrder>, DataError> {
        let body = self.private_request("OpenOrders", HashMap::new()).await?;

        let result = body["result"]["open"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("Missing result".into()))?;

        let mut orders = Vec::new();
        for (txid, order_val) in result {
            let descr = &order_val["descr"];
            orders.push(KrakenOrder {
                txid: txid.clone(),
                pair: descr["pair"].as_str().unwrap_or("").to_string(),
                side: descr["type"].as_str().unwrap_or("").to_string(),
                order_type: descr["ordertype"].as_str().unwrap_or("").to_string(),
                price: descr["price"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                volume: order_val["vol"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                filled: order_val["vol_exec"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                status: order_val["status"].as_str().unwrap_or("").to_string(),
            });
        }

        debug!("Kraken open orders: {}", orders.len());
        Ok(orders)
    }

    /// Get closed/orders history.
    pub async fn get_closed_orders(&self) -> Result<Vec<KrakenOrder>, DataError> {
        let body = self.private_request("ClosedOrders", HashMap::new()).await?;

        let result = body["result"]["closed"]
            .as_object()
            .ok_or_else(|| DataError::ParseError("Missing result".into()))?;

        let mut orders = Vec::new();
        for (txid, order_val) in result {
            let descr = &order_val["descr"];
            orders.push(KrakenOrder {
                txid: txid.clone(),
                pair: descr["pair"].as_str().unwrap_or("").to_string(),
                side: descr["type"].as_str().unwrap_or("").to_string(),
                order_type: descr["ordertype"].as_str().unwrap_or("").to_string(),
                price: descr["price"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                volume: order_val["vol"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                filled: order_val["vol_exec"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                status: order_val["status"].as_str().unwrap_or("").to_string(),
            });
        }

        info!("Kraken closed orders: {}", orders.len());
        Ok(orders)
    }

    /// Get minimum order size for a pair.
    pub async fn get_min_order_size(&self, pair: &str) -> Result<f64, DataError> {
        let kraken_pair = Self::to_kraken_pair(pair);
        let url = format!("{}/0/public/AssetPairs", self.rest_url);

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

        let (_, pair_val) = result
            .iter()
            .next()
            .ok_or_else(|| DataError::NoData(format!("No data for {}", pair)))?;

        let min_size = pair_val["ordermin"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0001);

        debug!("Min order size for {}: {}", pair, min_size);
        Ok(min_size)
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

#[derive(Debug, Clone)]
pub struct KrakenOrder {
    pub txid: String,
    pub pair: String,
    pub side: String,
    pub order_type: String,
    pub price: f64,
    pub volume: f64,
    pub filled: f64,
    pub status: String,
}
