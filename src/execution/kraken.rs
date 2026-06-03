use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use crate::core::error::ExecutionError;
use crate::core::types::{AccountState, Order, OrderStatus, OrderType, Position, Side, TradeRecord};
use crate::execution::engine::ExecutionEngine;

/// Per-pair trading metadata fetched from Kraken's public AssetPairs endpoint.
/// Without this, orders for sub-penny coins format to "0.00" and small orders
/// silently violate Kraken's per-pair minimums.
#[derive(Debug, Clone)]
pub struct PairMeta {
    /// Kraken order endpoint name, e.g. "XBTUSD".
    pub altname: String,
    /// Decimal places allowed in the price field.
    pub price_decimals: u32,
    /// Decimal places allowed in the volume (lot) field.
    pub lot_decimals: u32,
    /// Minimum order size in base units.
    pub ordermin: f64,
    /// Minimum order cost (notional) in quote currency. 0.0 if unspecified.
    pub costmin: f64,
}

pub struct KrakenTrader {
    client: Client,
    rest_url: String,
    api_key: String,
    api_secret: Vec<u8>,
    positions: HashMap<String, Position>,
    closed_trades: Vec<TradeRecord>,
    account: AccountState,
    order_counter: u64,
    fee_rate: f64,
    /// "BTC/USD" -> PairMeta. Populated by `load_pair_meta`.
    pair_meta: HashMap<String, PairMeta>,
    /// Spot accounts cannot short. Hard-disabled by default.
    allow_shorts: bool,
    /// position_id -> resting stop-loss order txid. Used to cancel the exchange
    /// stop when a position exits any other way (TP, AI close, kill-switch).
    stop_orders: HashMap<String, String>,
    /// position_id -> resting take-profit order txid (bracket sibling of the stop).
    tp_orders: HashMap<String, String>,
    /// "post_only" or "marketable" — see TradingConfig::entry_mode.
    entry_mode: String,
    /// Max slippage fraction for marketable entries (e.g. 0.0015 = 0.15%).
    max_entry_slippage: f64,
}

/// Map Kraken's internal base-asset symbols to the canonical ticker used in config.
fn canonical_base(sym: &str) -> String {
    match sym {
        "XBT" | "XXBT" => "BTC".to_string(),
        "XDG" | "XXDG" => "DOGE".to_string(),
        other => other.to_string(),
    }
}

impl KrakenTrader {
    pub fn new(
        rest_url: &str,
        api_key: &str,
        api_secret: &str,
        starting_balance: f64,
        fee_rate: f64,
    ) -> Self {
        Self {
            client: Client::new(),
            rest_url: rest_url.to_string(),
            api_key: api_key.to_string(),
            api_secret: general_purpose::STANDARD
                .decode(api_secret)
                .unwrap_or_default(),
            positions: HashMap::new(),
            closed_trades: Vec::new(),
            account: AccountState::new(starting_balance),
            order_counter: 0,
            fee_rate,
            pair_meta: HashMap::new(),
            allow_shorts: false,
            stop_orders: HashMap::new(),
            tp_orders: HashMap::new(),
            entry_mode: "post_only".to_string(),
            max_entry_slippage: 0.0015,
        }
    }

    pub fn set_allow_shorts(&mut self, allow: bool) {
        self.allow_shorts = allow;
    }

    pub fn set_entry_mode(&mut self, mode: &str, max_slippage_pct: f64) {
        self.entry_mode = mode.to_string();
        self.max_entry_slippage = (max_slippage_pct / 100.0).max(0.0);
    }

    fn nonce(&self) -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string()
    }

    fn sign(&self, url_path: &str, nonce: &str, post_data: &str) -> String {
        let mut sha = Sha256::new();
        sha.update(nonce.as_bytes());
        sha.update(post_data.as_bytes());
        let sha_hash = sha.finalize();

        let mut mac = Hmac::<Sha512>::new_from_slice(&self.api_secret)
            .expect("HMAC can take key of any size");
        mac.update(url_path.as_bytes());
        mac.update(&sha_hash);
        let result = mac.finalize();
        general_purpose::STANDARD.encode(result.into_bytes())
    }

    async fn private_request(
        &self,
        endpoint: &str,
        mut params: HashMap<String, String>,
    ) -> Result<serde_json::Value, ExecutionError> {
        let nonce = self.nonce();
        params.insert("nonce".to_string(), nonce.clone());

        let post_data: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url_path = format!("/0/private/{}", endpoint);
        let url = format!("{}{}", self.rest_url, url_path);
        let signature = self.sign(&url_path, &nonce, &post_data);

        let resp = self
            .client
            .post(&url)
            .header("API-Key", &self.api_key)
            .header("API-Sign", &signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_data)
            .send()
            .await
            .map_err(|e| ExecutionError::ExchangeError(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::ExchangeError(e.to_string()))?;

        let errors = body["error"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !errors.is_empty() {
            return Err(ExecutionError::ExchangeError(format!("Kraken: {:?}", errors)));
        }

        Ok(body)
    }

    /// Fetch per-pair trading metadata (decimals, minimums, altname) from
    /// Kraken's public AssetPairs endpoint. Best-effort: failures leave the
    /// cache empty and callers fall back to conservative defaults.
    pub async fn load_pair_meta(&mut self) -> Result<usize, ExecutionError> {
        let url = format!("{}/0/public/AssetPairs", self.rest_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecutionError::ExchangeError(e.to_string()))?;
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::ExchangeError(e.to_string()))?;
        let result = body["result"]
            .as_object()
            .ok_or_else(|| ExecutionError::ExchangeError("AssetPairs missing result".into()))?;

        let mut count = 0;
        for (_key, val) in result {
            let altname = match val["altname"].as_str() {
                Some(a) => a.to_string(),
                None => continue,
            };
            // wsname is "XBT/USD" form; fall back to base/quote if absent.
            let normalized = if let Some(ws) = val["wsname"].as_str() {
                let parts: Vec<&str> = ws.split('/').collect();
                if parts.len() == 2 {
                    format!("{}/{}", canonical_base(parts[0]), parts[1])
                } else {
                    continue;
                }
            } else {
                let base = val["base"].as_str().unwrap_or("");
                let quote = val["quote"].as_str().unwrap_or("");
                let q = quote.trim_start_matches('Z');
                format!("{}/{}", canonical_base(base), q)
            };

            let meta = PairMeta {
                altname,
                price_decimals: val["pair_decimals"].as_u64().unwrap_or(5) as u32,
                lot_decimals: val["lot_decimals"].as_u64().unwrap_or(8) as u32,
                ordermin: val["ordermin"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                costmin: val["costmin"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
            };
            self.pair_meta.insert(normalized, meta);
            count += 1;
        }
        info!("Loaded Kraken metadata for {} pairs", count);
        Ok(count)
    }

    fn meta(&self, pair: &str) -> Option<&PairMeta> {
        self.pair_meta.get(pair)
    }

    /// Kraken order-endpoint pair name. Prefers the official altname; falls
    /// back to stripping the slash (works for most modern symbols).
    fn kraken_pair(&self, pair: &str) -> String {
        self.meta(pair)
            .map(|m| m.altname.clone())
            .unwrap_or_else(|| pair.replace('/', ""))
    }

    fn price_decimals(&self, pair: &str) -> u32 {
        self.meta(pair).map(|m| m.price_decimals).unwrap_or(5)
    }

    fn lot_decimals(&self, pair: &str) -> u32 {
        self.meta(pair).map(|m| m.lot_decimals).unwrap_or(8)
    }

    fn fmt_price(&self, pair: &str, price: f64) -> String {
        format!("{:.*}", self.price_decimals(pair) as usize, price)
    }

    /// Round volume DOWN to the pair's lot precision so we never exceed
    /// available balance/inventory due to rounding up.
    fn round_volume(&self, pair: &str, vol: f64) -> f64 {
        let dp = self.lot_decimals(pair);
        let factor = 10f64.powi(dp as i32);
        (vol * factor).floor() / factor
    }

    fn fmt_volume(&self, pair: &str, vol: f64) -> String {
        format!("{:.*}", self.lot_decimals(pair) as usize, vol)
    }

    /// Validate an order against Kraken's per-pair minimums. Returns the
    /// lot-rounded volume on success.
    fn validate_order(&self, pair: &str, vol: f64, price: f64) -> Result<f64, ExecutionError> {
        let rounded = self.round_volume(pair, vol);
        if rounded <= 0.0 {
            return Err(ExecutionError::ExchangeError(format!(
                "{}: volume {} rounds to zero at {} lot decimals",
                pair,
                vol,
                self.lot_decimals(pair)
            )));
        }
        if let Some(m) = self.meta(pair) {
            if m.ordermin > 0.0 && rounded < m.ordermin {
                return Err(ExecutionError::ExchangeError(format!(
                    "{}: volume {} below ordermin {}",
                    pair, rounded, m.ordermin
                )));
            }
            let cost = rounded * price;
            if m.costmin > 0.0 && cost < m.costmin {
                return Err(ExecutionError::ExchangeError(format!(
                    "{}: cost ${:.2} below costmin ${:.2}",
                    pair, cost, m.costmin
                )));
            }
        }
        Ok(rounded)
    }

    pub async fn fetch_balance_raw(&mut self) -> Result<HashMap<String, f64>, ExecutionError> {
        let body = self.private_request("Balance", HashMap::new()).await?;
        let result = body["result"]
            .as_object()
            .ok_or_else(|| ExecutionError::ExchangeError("Missing result".into()))?;
        let mut assets = HashMap::new();
        for (asset, amount_str) in result {
            let amount: f64 = amount_str.as_str().unwrap_or("0").parse().unwrap_or(0.0);
            if amount > 0.0 {
                assets.insert(asset.clone(), amount);
            }
        }
        Ok(assets)
    }

    pub async fn fetch_balance(&mut self) -> Result<f64, ExecutionError> {
        let body = self.private_request("Balance", HashMap::new()).await?;
        let result = body["result"]
            .as_object()
            .ok_or_else(|| ExecutionError::ExchangeError("Missing result".into()))?;

        let mut total_usd = 0.0;
        for (asset, amount_str) in result {
            let amount: f64 = amount_str.as_str().unwrap_or("0").parse().unwrap_or(0.0);
            if amount <= 0.0 {
                continue;
            }
            match asset.as_str() {
                "ZUSD" | "USD" => total_usd += amount,
                "XXBT" | "XBT" => {
                    if let Ok(ticker) = self.get_ticker_price("BTC/USD").await {
                        total_usd += amount * ticker;
                    }
                }
                "XETH" | "ETH" => {
                    if let Ok(ticker) = self.get_ticker_price("ETH/USD").await {
                        total_usd += amount * ticker;
                    }
                }
                _ => {
                    let clean = canonical_base(asset.trim_start_matches('X').trim_start_matches('Z'));
                    if let Ok(ticker) = self.get_ticker_price(&format!("{}/USD", clean)).await {
                        total_usd += amount * ticker;
                    }
                }
            }
        }

        self.account.balance = total_usd;
        self.account.equity = total_usd;
        info!("Kraken balance: ${:.2}", total_usd);
        Ok(total_usd)
    }

    pub async fn get_ticker_price(&self, pair: &str) -> Result<f64, ExecutionError> {
        let kraken_pair = self.kraken_pair(pair);
        let url = format!("{}/0/public/Ticker", self.rest_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("pair", &kraken_pair)])
            .send()
            .await
            .map_err(|e| ExecutionError::ExchangeError(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ExecutionError::ExchangeError(e.to_string()))?;

        let result = body["result"]
            .as_object()
            .ok_or_else(|| ExecutionError::ExchangeError("Missing result".into()))?;

        let (_, ticker_val) = result
            .iter()
            .next()
            .ok_or_else(|| ExecutionError::ExchangeError("No ticker".into()))?;

        ticker_val["c"][0]
            .as_str()
            .unwrap_or("0")
            .parse::<f64>()
            .map_err(|_| ExecutionError::ExchangeError("Invalid price".into()))
    }

    /// Poll QueryOrders until the order closes (or times out), returning the
    /// real average fill price, executed volume, and fee paid. This replaces
    /// the previous fire-and-forget behaviour that recorded a fill regardless
    /// of whether the exchange actually executed.
    async fn confirm_fill(
        &self,
        txid: &str,
    ) -> Result<(f64, f64, f64), ExecutionError> {
        for attempt in 0..12 {
            let mut params = HashMap::new();
            params.insert("txid".to_string(), txid.to_string());
            let body = match self.private_request("QueryOrders", params).await {
                Ok(b) => b,
                Err(e) => {
                    warn!("QueryOrders failed (attempt {}): {}", attempt, e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            if let Some(order) = body["result"].get(txid) {
                let status = order["status"].as_str().unwrap_or("");
                let vol_exec = order["vol_exec"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let avg_price = order["price"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let fee = order["fee"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                if status == "closed" && vol_exec > 0.0 {
                    return Ok((avg_price, vol_exec, fee));
                }
                if status == "canceled" || status == "expired" {
                    if vol_exec > 0.0 {
                        return Ok((avg_price, vol_exec, fee));
                    }
                    return Err(ExecutionError::ExchangeError(format!(
                        "Order {} {} with no fill",
                        txid, status
                    )));
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(ExecutionError::ExchangeError(format!(
            "Order {} not confirmed filled within timeout",
            txid
        )))
    }

    /// Wait up to `max_secs` for a (limit) order to fill, polling once per
    /// second. Returns the real (avg_price, vol_exec, fee) if anything filled,
    /// else None. Used for post-only maker entries that must either fill at the
    /// planned price or be cancelled (no adverse slippage).
    async fn wait_for_limit_fill(
        &self,
        txid: &str,
        max_secs: u64,
    ) -> Option<(f64, f64, f64)> {
        let parse = |order: &serde_json::Value| -> (String, f64, f64, f64) {
            let status = order["status"].as_str().unwrap_or("").to_string();
            let vol = order["vol_exec"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let price = order["price"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let fee = order["fee"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            (status, vol, price, fee)
        };
        for _ in 0..max_secs {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            let mut params = HashMap::new();
            params.insert("txid".to_string(), txid.to_string());
            if let Ok(body) = self.private_request("QueryOrders", params).await {
                if let Some(order) = body["result"].get(txid) {
                    let (status, vol, price, fee) = parse(order);
                    if status == "closed" && vol > 0.0 {
                        return Some((price, vol, fee));
                    }
                    if status == "canceled" || status == "expired" {
                        return if vol > 0.0 { Some((price, vol, fee)) } else { None };
                    }
                }
            }
        }
        // Timeout — return any partial fill so the caller can keep it.
        let mut params = HashMap::new();
        params.insert("txid".to_string(), txid.to_string());
        if let Ok(body) = self.private_request("QueryOrders", params).await {
            if let Some(order) = body["result"].get(txid) {
                let (_s, vol, price, fee) = parse(order);
                if vol > 0.0 {
                    return Some((price, vol, fee));
                }
            }
        }
        None
    }

    /// Cancel a single order by txid (used to clean up resting stop orders).
    pub async fn cancel_order(&self, txid: &str) -> Result<(), ExecutionError> {
        let mut params = HashMap::new();
        params.insert("txid".to_string(), txid.to_string());
        self.private_request("CancelOrder", params).await?;
        Ok(())
    }

    /// Real kill-switch: cancel every open order on the account.
    pub async fn cancel_all(&mut self) -> Result<(), ExecutionError> {
        self.private_request("CancelAll", HashMap::new()).await?;
        self.stop_orders.clear();
        self.tp_orders.clear();
        info!("Kraken CancelAll executed — all open orders cancelled");
        Ok(())
    }

    /// Place a resting stop-loss order on the exchange as a downtime safety net
    /// and remember its txid against the position so we can cancel it if the
    /// position exits some other way. Best-effort: a failure is logged, not fatal
    /// (the engine also manages stops locally while it is running).
    pub async fn place_stop(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        stop_price: f64,
        position_id: &str,
    ) {
        if stop_price <= 0.0 {
            return;
        }
        // Closing side: a long position is protected by a sell-stop.
        let stop_side = match side {
            Side::Long => "sell",
            Side::Short => "buy",
        };
        let vol = self.round_volume(pair, quantity);
        if vol <= 0.0 {
            return;
        }
        let mut params = HashMap::new();
        params.insert("pair".to_string(), self.kraken_pair(pair));
        params.insert("type".to_string(), stop_side.to_string());
        params.insert("ordertype".to_string(), "stop-loss".to_string());
        params.insert("price".to_string(), self.fmt_price(pair, stop_price));
        params.insert("volume".to_string(), self.fmt_volume(pair, vol));

        match self.private_request("AddOrder", params).await {
            Ok(body) => {
                let txid = body["result"]["txid"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !txid.is_empty() {
                    self.stop_orders.insert(position_id.to_string(), txid.clone());
                    info!(
                        "Resting stop-loss placed for {} @ {} (txid={})",
                        pair,
                        self.fmt_price(pair, stop_price),
                        txid
                    );
                }
            }
            Err(e) => warn!("Failed to place resting stop for {}: {}", pair, e),
        }
    }

    /// Place a resting take-profit (trigger) order at TP as the bracket sibling
    /// of the stop. Both are TRIGGER orders (untriggered until price reaches the
    /// level), so neither reserves spot balance — they coexist safely. When one
    /// fills, the OCO logic cancels the other. Captures winners 24/7, even
    /// between ticks or while the bot is down.
    pub async fn place_tp(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        tp_price: f64,
        position_id: &str,
    ) {
        if tp_price <= 0.0 {
            return;
        }
        let tp_side = match side {
            Side::Long => "sell",
            Side::Short => "buy",
        };
        let vol = self.round_volume(pair, quantity);
        if vol <= 0.0 {
            return;
        }
        let mut params = HashMap::new();
        params.insert("pair".to_string(), self.kraken_pair(pair));
        params.insert("type".to_string(), tp_side.to_string());
        params.insert("ordertype".to_string(), "take-profit".to_string());
        params.insert("price".to_string(), self.fmt_price(pair, tp_price));
        params.insert("volume".to_string(), self.fmt_volume(pair, vol));

        match self.private_request("AddOrder", params).await {
            Ok(body) => {
                let txid = body["result"]["txid"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !txid.is_empty() {
                    self.tp_orders.insert(position_id.to_string(), txid.clone());
                    info!(
                        "Resting take-profit placed for {} @ {} (txid={})",
                        pair, self.fmt_price(pair, tp_price), txid
                    );
                }
            }
            Err(e) => warn!("Failed to place resting take-profit for {}: {}", pair, e),
        }
    }

    /// Move the stop-loss on an open position: cancel the old resting exchange
    /// stop, update the local stop, and place a fresh resting stop at the new
    /// price. If the re-place fails, the engine's local stop check still covers
    /// the position (no resting stop => managed locally), so it's never naked.
    pub async fn adjust_stop(&mut self, position_id: &str, new_stop: f64) {
        let (pair, side, qty) = match self.positions.get(position_id) {
            Some(p) => (p.pair.clone(), p.side, p.quantity),
            None => return,
        };
        if let Some(txid) = self.stop_orders.remove(position_id) {
            if let Err(e) = self.cancel_order(&txid).await {
                warn!("adjust_stop: failed to cancel old stop {}: {}", txid, e);
            }
        }
        if let Some(p) = self.positions.get_mut(position_id) {
            p.stop_loss = new_stop;
        }
        self.place_stop(&pair, side, qty, new_stop, position_id).await;
    }

    /// Market-sell an entire asset balance (used by reconciliation cleanup).
    pub async fn market_sell_all(&self, pair: &str, volume: f64) -> Result<(), ExecutionError> {
        let vol = self.round_volume(pair, volume);
        if vol <= 0.0 {
            return Ok(());
        }
        let mut params = HashMap::new();
        params.insert("pair".to_string(), self.kraken_pair(pair));
        params.insert("type".to_string(), "sell".to_string());
        params.insert("ordertype".to_string(), "market".to_string());
        params.insert("volume".to_string(), self.fmt_volume(pair, vol));
        self.private_request("AddOrder", params).await?;
        Ok(())
    }

    pub fn account(&self) -> &AccountState {
        &self.account
    }

    pub fn set_balance(&mut self, balance: f64) {
        self.account.balance = balance;
        self.account.equity = balance;
    }

    pub fn closed_trades(&self) -> &[TradeRecord] {
        &self.closed_trades
    }

    pub fn positions(&self) -> &HashMap<String, Position> {
        &self.positions
    }

    pub fn positions_mut(&mut self) -> &mut HashMap<String, Position> {
        &mut self.positions
    }

    pub fn update_prices(&mut self, prices: &HashMap<String, f64>) {
        let mut total_unrealized = 0.0;
        let mut total_cost_basis = 0.0;
        for pos in self.positions.values_mut() {
            if let Some(&price) = prices.get(&pos.pair) {
                pos.current_price = price;
                pos.unrealized_pnl = match pos.side {
                    Side::Long => (price - pos.entry_price) * pos.quantity,
                    Side::Short => (pos.entry_price - price) * pos.quantity,
                };
                total_unrealized += pos.unrealized_pnl;
                total_cost_basis += pos.entry_price * pos.quantity;
            }
        }
        self.account.update_equity(total_unrealized, total_cost_basis);
    }

    /// Reconcile resting exchange stop orders that have already filled (e.g. the
    /// stop fired between ticks, or while the bot was briefly down). For each
    /// filled stop we record the real fill, credit PnL, and drop the position —
    /// so the local book never double-sells a position the exchange already exited.
    pub async fn reconcile_filled_stops(&mut self) -> Vec<TradeRecord> {
        let mut closed = Vec::new();
        let tracked: Vec<(String, String)> = self
            .stop_orders
            .iter()
            .map(|(pid, txid)| (pid.clone(), txid.clone()))
            .collect();

        for (pid, txid) in tracked {
            let mut params = HashMap::new();
            params.insert("txid".to_string(), txid.clone());
            let body = match self.private_request("QueryOrders", params).await {
                Ok(b) => b,
                Err(_) => continue,
            };
            let order = match body["result"].get(&txid) {
                Some(o) => o,
                None => continue,
            };
            let status = order["status"].as_str().unwrap_or("");
            let vol_exec = order["vol_exec"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            if status == "closed" && vol_exec > 0.0 {
                let exit_price = order["price"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let fee = order["fee"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                self.stop_orders.remove(&pid);
                // OCO: stop filled -> cancel the sibling take-profit.
                if let Some(tp_txid) = self.tp_orders.remove(&pid) {
                    let _ = self.cancel_order(&tp_txid).await;
                }
                if let Some(pos) = self.positions.remove(&pid) {
                    let pnl = match pos.side {
                        Side::Long => (exit_price - pos.entry_price) * vol_exec - fee,
                        Side::Short => (pos.entry_price - exit_price) * vol_exec - fee,
                    };
                    let pnl_pct = if pos.entry_price * vol_exec > 0.0 {
                        pnl / (pos.entry_price * vol_exec) * 100.0
                    } else {
                        0.0
                    };
                    self.account.balance += pnl;
                    self.account.daily_pnl += pnl;
                    self.account.open_positions = self.positions.len();
                    let trade = TradeRecord {
                        id: format!("live-{}", self.order_counter),
                        pair: pos.pair.clone(),
                        side: pos.side,
                        entry_price: pos.entry_price,
                        exit_price,
                        quantity: vol_exec,
                        pnl,
                        pnl_pct,
                        strategy_name: pos.strategy_name.clone(),
                        opened_at: pos.opened_at,
                        closed_at: Utc::now(),
                        notes: "Resting stop-loss filled".to_string(),
                    };
                    self.order_counter += 1;
                    self.closed_trades.push(trade.clone());
                    info!("Reconciled filled stop for {} @ {}", pos.pair, exit_price);
                    closed.push(trade);
                }
            } else if status == "canceled" || status == "expired" {
                // Stop no longer resting and didn't fill — stop owning it.
                self.stop_orders.remove(&pid);
            }
        }
        closed
    }

    /// Reconcile resting take-profit orders that have filled (the winning
    /// bracket leg). Books the profit, cancels the sibling stop (OCO), and drops
    /// the position. Captures winners even between ticks / during downtime.
    pub async fn reconcile_filled_tps(&mut self) -> Vec<TradeRecord> {
        let mut closed = Vec::new();
        let tracked: Vec<(String, String)> = self
            .tp_orders
            .iter()
            .map(|(pid, txid)| (pid.clone(), txid.clone()))
            .collect();

        for (pid, txid) in tracked {
            let mut params = HashMap::new();
            params.insert("txid".to_string(), txid.clone());
            let body = match self.private_request("QueryOrders", params).await {
                Ok(b) => b,
                Err(_) => continue,
            };
            let order = match body["result"].get(&txid) {
                Some(o) => o,
                None => continue,
            };
            let status = order["status"].as_str().unwrap_or("");
            let vol_exec = order["vol_exec"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
            if status == "closed" && vol_exec > 0.0 {
                let exit_price = order["price"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let fee = order["fee"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                self.tp_orders.remove(&pid);
                // OCO: take-profit filled -> cancel the sibling stop.
                if let Some(stop_txid) = self.stop_orders.remove(&pid) {
                    let _ = self.cancel_order(&stop_txid).await;
                }
                if let Some(pos) = self.positions.remove(&pid) {
                    let pnl = match pos.side {
                        Side::Long => (exit_price - pos.entry_price) * vol_exec - fee,
                        Side::Short => (pos.entry_price - exit_price) * vol_exec - fee,
                    };
                    let pnl_pct = if pos.entry_price * vol_exec > 0.0 {
                        pnl / (pos.entry_price * vol_exec) * 100.0
                    } else {
                        0.0
                    };
                    self.account.balance += pnl;
                    self.account.daily_pnl += pnl;
                    self.account.open_positions = self.positions.len();
                    let trade = TradeRecord {
                        id: format!("live-{}", self.order_counter),
                        pair: pos.pair.clone(),
                        side: pos.side,
                        entry_price: pos.entry_price,
                        exit_price,
                        quantity: vol_exec,
                        pnl,
                        pnl_pct,
                        strategy_name: pos.strategy_name.clone(),
                        opened_at: pos.opened_at,
                        closed_at: Utc::now(),
                        notes: "Take-profit filled".to_string(),
                    };
                    self.order_counter += 1;
                    self.closed_trades.push(trade.clone());
                    info!("Reconciled filled take-profit for {} @ {}", pos.pair, exit_price);
                    closed.push(trade);
                }
            } else if status == "canceled" || status == "expired" {
                self.tp_orders.remove(&pid);
            }
        }
        closed
    }

    /// Drive exits while the bot is running:
    ///   - Take-profit-1 breach -> local market close (and cancels the resting stop).
    ///   - Stop-loss is owned by the resting EXCHANGE order when one exists; we
    ///     only close locally for positions that have NO resting stop (e.g.
    ///     reconciled holdings). This avoids the local book and the exchange both
    ///     selling the same position.
    ///
    /// Always reconciles already-filled stops first.
    pub async fn check_stops_live(
        &mut self,
        prices: &HashMap<String, f64>,
    ) -> Vec<TradeRecord> {
        let mut closed = self.reconcile_filled_stops().await;
        closed.extend(self.reconcile_filled_tps().await);

        // Local fallback only for legs that have NO resting exchange order
        // (e.g. reconciled holdings). Bracketed positions are owned by the
        // exchange orders + the reconcile passes above, so we don't double-act.
        let triggers: Vec<(String, String)> = self
            .positions
            .iter()
            .filter_map(|(id, pos)| {
                let price = *prices.get(&pos.pair)?;
                let has_resting_stop = self.stop_orders.contains_key(id);
                let has_resting_tp = self.tp_orders.contains_key(id);
                let hit_stop = pos.stop_loss > 0.0
                    && match pos.side {
                        Side::Long => price <= pos.stop_loss,
                        Side::Short => price >= pos.stop_loss,
                    };
                let hit_tp = pos.take_profit_1 > 0.0
                    && match pos.side {
                        Side::Long => price >= pos.take_profit_1,
                        Side::Short => price <= pos.take_profit_1,
                    };
                if hit_tp && !has_resting_tp {
                    Some((id.clone(), "Take profit 1 hit".to_string()))
                } else if hit_stop && !has_resting_stop {
                    Some((id.clone(), "Stop loss hit".to_string()))
                } else {
                    None
                }
            })
            .collect();

        for (id, reason) in triggers {
            match self.close_position_with_note(&id, &reason).await {
                Ok(trade) => closed.push(trade),
                Err(e) => warn!("Live stop/TP close failed for {}: {}", id, e),
            }
        }
        closed
    }

    async fn close_position_with_note(
        &mut self,
        position_id: &str,
        note: &str,
    ) -> Result<TradeRecord, ExecutionError> {
        let pos = self
            .positions
            .get(position_id)
            .cloned()
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?;

        // Cancel any resting bracket orders FIRST (stop + take-profit) so they
        // can't also fire and double-sell once we place the market close below.
        if let Some(txid) = self.stop_orders.remove(position_id) {
            if let Err(e) = self.cancel_order(&txid).await {
                warn!("Failed to cancel resting stop {} for {}: {}", txid, pos.pair, e);
            }
        }
        if let Some(txid) = self.tp_orders.remove(position_id) {
            if let Err(e) = self.cancel_order(&txid).await {
                warn!("Failed to cancel resting take-profit {} for {}: {}", txid, pos.pair, e);
            }
        }

        // Spot: only longs are held, so closing means selling the base asset.
        let close_side = match pos.side {
            Side::Long => "sell",
            Side::Short => "buy",
        };
        let vol = self.validate_order(&pos.pair, pos.quantity, pos.current_price)?;

        let mut params = HashMap::new();
        params.insert("pair".to_string(), self.kraken_pair(&pos.pair));
        params.insert("type".to_string(), close_side.to_string());
        params.insert("ordertype".to_string(), "market".to_string());
        params.insert("volume".to_string(), self.fmt_volume(&pos.pair, vol));

        let body = self.private_request("AddOrder", params).await?;
        let txid = body["result"]["txid"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let (exit_price, exit_vol, exit_fee) = self
            .confirm_fill(&txid)
            .await
            .unwrap_or((pos.current_price, vol, pos.current_price * vol * self.fee_rate));

        let pnl = match pos.side {
            Side::Long => (exit_price - pos.entry_price) * exit_vol - exit_fee,
            Side::Short => (pos.entry_price - exit_price) * exit_vol - exit_fee,
        };
        let pnl_pct = if pos.entry_price * exit_vol > 0.0 {
            pnl / (pos.entry_price * exit_vol) * 100.0
        } else {
            0.0
        };

        let trade = TradeRecord {
            id: format!("live-{}", self.order_counter),
            pair: pos.pair.clone(),
            side: pos.side,
            entry_price: pos.entry_price,
            exit_price,
            quantity: exit_vol,
            pnl,
            pnl_pct,
            strategy_name: pos.strategy_name.clone(),
            opened_at: pos.opened_at,
            closed_at: Utc::now(),
            notes: note.to_string(),
        };
        self.order_counter += 1;
        self.account.balance += pnl;
        self.account.daily_pnl += pnl;
        self.positions.remove(position_id);
        self.account.open_positions = self.positions.len();
        self.closed_trades.push(trade.clone());
        Ok(trade)
    }
}

#[async_trait]
impl ExecutionEngine for KrakenTrader {
    /// Place a LIVE **post-only limit** entry at the AI's planned price and
    /// confirm the real fill before returning. Post-only (maker) means:
    ///   - we pay the maker fee (~0.25%) instead of taker (~0.40%), and
    ///   - the order can ONLY fill at the planned price or better — never worse —
    ///     so adverse slippage can't silently wreck the planned R:R.
    /// If it doesn't fill within the window it is cancelled and an error is
    /// returned (no position recorded). A market fallback is used only if no
    /// price is supplied. `stop_loss` is placed separately via `place_stop`.
    async fn place_order(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        price: Option<f64>,
        _stop_loss: Option<f64>,
    ) -> Result<Order, ExecutionError> {
        if side == Side::Short && !self.allow_shorts {
            return Err(ExecutionError::ExchangeError(
                "Short orders disabled (spot account is long-only)".into(),
            ));
        }

        let kraken_side = match side {
            Side::Long => "buy",
            Side::Short => "sell",
        };
        let ref_price = price.unwrap_or_else(|| {
            self.positions
                .get(pair)
                .map(|p| p.current_price)
                .unwrap_or(0.0)
        });
        let vol = self.validate_order(pair, quantity, ref_price.max(0.000001))?;

        let mut params = HashMap::new();
        params.insert("pair".to_string(), self.kraken_pair(pair));
        params.insert("type".to_string(), kraken_side.to_string());
        params.insert("volume".to_string(), self.fmt_volume(pair, vol));

        let is_limit = price.is_some() && ref_price > 0.0;
        let marketable = is_limit && self.entry_mode == "marketable";
        let mut limit_px = ref_price;

        if is_limit && marketable {
            // Marketable limit: anchor to CURRENT market and allow crossing up to
            // the slippage cap, so it fills now (taker) but never chases too far.
            let mkt = self.get_ticker_price(pair).await.unwrap_or(ref_price);
            limit_px = match side {
                Side::Long => mkt * (1.0 + self.max_entry_slippage),
                Side::Short => mkt * (1.0 - self.max_entry_slippage),
            };
            params.insert("ordertype".to_string(), "limit".to_string());
            params.insert("price".to_string(), self.fmt_price(pair, limit_px));
            info!(
                "Placing Kraken MARKETABLE LIMIT {}: {} {} @ {} (mkt {}, cap {:.2}%)",
                kraken_side, self.fmt_volume(pair, vol), pair,
                self.fmt_price(pair, limit_px), self.fmt_price(pair, mkt),
                self.max_entry_slippage * 100.0
            );
        } else if is_limit {
            // Post-only maker at the AI's planned price.
            params.insert("ordertype".to_string(), "limit".to_string());
            params.insert("price".to_string(), self.fmt_price(pair, ref_price));
            params.insert("oflags".to_string(), "post".to_string());
            info!(
                "Placing Kraken POST-ONLY LIMIT {}: {} {} @ {}",
                kraken_side, self.fmt_volume(pair, vol), pair, self.fmt_price(pair, ref_price)
            );
        } else {
            params.insert("ordertype".to_string(), "market".to_string());
            info!("Placing Kraken MARKET {}: {} {}", kraken_side, self.fmt_volume(pair, vol), pair);
        }

        let body = match self.private_request("AddOrder", params).await {
            Ok(b) => b,
            Err(e) => {
                return Err(ExecutionError::ExchangeError(format!(
                    "Entry not placed: {}",
                    e
                )));
            }
        };
        let txid = body["result"]["txid"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Confirm the real fill. Marketable crosses immediately (short window);
        // post-only may rest (longer fill-or-cancel window); market confirms fast.
        let fill = if is_limit {
            let window = if marketable { 8 } else { 25 };
            match self.wait_for_limit_fill(&txid, window).await {
                Some(f) => f,
                None => {
                    let _ = self.cancel_order(&txid).await;
                    return Err(ExecutionError::ExchangeError(format!(
                        "Limit entry for {} did not fill at {} within window — cancelled",
                        pair,
                        self.fmt_price(pair, limit_px)
                    )));
                }
            }
        } else {
            self.confirm_fill(&txid).await?
        };
        let (fill_price, fill_vol, _fee) = fill;
        self.order_counter += 1;

        info!(
            "Kraken order FILLED: txid={} @ {} vol {} (maker={})",
            txid,
            self.fmt_price(pair, fill_price),
            fill_vol,
            is_limit
        );

        Ok(Order {
            id: txid,
            pair: pair.to_string(),
            side,
            order_type: if is_limit { OrderType::Limit } else { OrderType::Market },
            price: Some(fill_price),
            quantity: fill_vol,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(fill_price),
        })
    }

    async fn close_position(&mut self, position_id: &str) -> Result<Order, ExecutionError> {
        let trade = self
            .close_position_with_note(position_id, "Closed")
            .await?;
        Ok(Order {
            id: format!("live-close-{}", self.order_counter),
            pair: trade.pair.clone(),
            side: match trade.side {
                Side::Long => Side::Short,
                Side::Short => Side::Long,
            },
            order_type: OrderType::Market,
            price: Some(trade.exit_price),
            quantity: trade.quantity,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(trade.exit_price),
        })
    }

    fn open_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    fn balance(&self) -> f64 {
        self.account.balance
    }
}
