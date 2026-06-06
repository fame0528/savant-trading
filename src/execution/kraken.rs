//! KrakenTrader — live execution engine for Kraken exchange.
//!
//! Implements the ExecutionEngine trait with real order placement,
//! fill tracking, stop-loss management, and safety rails.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::core::error::ExecutionError;
use crate::core::types::{Order, OrderStatus, OrderType, Position, Side, TradeRecord};
use crate::data::kraken::KrakenClient;
use crate::execution::engine::ExecutionEngine;

/// Configuration for the KrakenTrader.
#[derive(Debug, Clone)]
pub struct KrakenTraderConfig {
    /// Starting balance for P&L tracking
    pub starting_balance: f64,
    /// Fee rate (taker)
    pub fee_rate: f64,
    /// Fee rate (maker)
    pub maker_fee_rate: f64,
    /// Maximum order size as fraction of balance
    pub max_order_pct: f64,
    /// Maximum daily loss as fraction of starting balance
    pub max_daily_loss_pct: f64,
    /// Slippage alert threshold (fraction)
    pub slippage_alert_pct: f64,
    /// Discord webhook URL for notifications
    pub webhook_url: Option<String>,
}

impl Default for KrakenTraderConfig {
    fn default() -> Self {
        Self {
            starting_balance: 50.0,
            fee_rate: 0.004,
            maker_fee_rate: 0.0025,
            max_order_pct: 0.20,
            max_daily_loss_pct: 0.10,
            slippage_alert_pct: 0.005,
            webhook_url: None,
        }
    }
}

/// Live execution engine for Kraken exchange.
pub struct KrakenTrader {
    client: KrakenClient,
    config: KrakenTraderConfig,
    positions: HashMap<String, Position>,
    closed_trades: Vec<TradeRecord>,
    balance: f64,
    daily_pnl: f64,
    daily_start_balance: f64,
    last_reset_date: chrono::NaiveDate,
    order_counter: u64,
    /// Minimum order sizes per pair (fetched from Kraken)
    min_order_sizes: HashMap<String, f64>,
    /// Stop order IDs per position (position_id → stop_txid)
    stop_orders: HashMap<String, String>,
    /// Whether the trader is halted (daily loss exceeded)
    halted: bool,
}

impl KrakenTrader {
    /// Create a new KrakenTrader with real API credentials.
    pub async fn new(
        rest_url: &str,
        api_key: &str,
        api_secret: &str,
        config: KrakenTraderConfig,
    ) -> Result<Self, ExecutionError> {
        let client = KrakenClient::with_credentials(rest_url, api_key, api_secret);

        // Verify API credentials by fetching balance
        let balance_map =
            client
                .get_balance()
                .await
                .map_err(|_e| ExecutionError::InsufficientBalance {
                    needed: 0.0,
                    available: 0.0,
                })?;

        let usd_balance = balance_map
            .get("ZUSD")
            .or(balance_map.get("USD"))
            .copied()
            .unwrap_or(0.0);

        info!("KrakenTrader initialized. USD balance: ${:.2}", usd_balance);

        let balance = if usd_balance > 0.0 {
            usd_balance
        } else {
            config.starting_balance
        };

        Ok(Self {
            client,
            config,
            positions: HashMap::new(),
            closed_trades: Vec::new(),
            balance,
            daily_pnl: 0.0,
            daily_start_balance: balance,
            last_reset_date: Utc::now().date_naive(),
            order_counter: 0,
            min_order_sizes: HashMap::new(),
            stop_orders: HashMap::new(),
            halted: false,
        })
    }

    /// Check if daily loss limit has been exceeded.
    fn check_daily_loss(&mut self) {
        let today = Utc::now().date_naive();
        if today != self.last_reset_date {
            self.daily_pnl = 0.0;
            self.daily_start_balance = self.balance;
            self.last_reset_date = today;
            if self.halted {
                info!("New day — resuming trading");
                self.halted = false;
            }
        }

        let max_loss = self.daily_start_balance * self.config.max_daily_loss_pct;
        if self.daily_pnl < -max_loss && !self.halted {
            error!(
                "DAILY LOSS HALT: P&L ${:.2} exceeds max ${:.2}. Cancelling all orders.",
                self.daily_pnl, -max_loss
            );
            self.halted = true;
            // Cancel all open orders
            let client = self.client.clone();
            tokio::spawn(async move {
                let _ = client.cancel_all_orders().await;
            });
        }
    }

    /// Check slippage and alert if too high.
    #[allow(dead_code)]
    fn check_slippage(&self, expected: f64, actual: f64, pair: &str) {
        if expected > 0.0 {
            let slippage = ((actual - expected) / expected).abs();
            if slippage > self.config.slippage_alert_pct {
                warn!(
                    "SLIPPAGE ALERT: {} expected {:.2} got {:.2} ({:.2}%)",
                    pair,
                    expected,
                    actual,
                    slippage * 100.0
                );
                // Send webhook notification
                if let Some(ref url) = self.config.webhook_url {
                    let msg = format!(
                        "⚠️ SLIPPAGE ALERT: {} expected ${:.2} got ${:.2} ({:.2}%)",
                        pair,
                        expected,
                        actual,
                        slippage * 100.0
                    );
                    let url = url.clone();
                    tokio::spawn(async move {
                        let _ = send_webhook(&url, &msg).await;
                    });
                }
            }
        }
    }

    /// Sync positions with Kraken's open orders.
    pub async fn sync_positions(&mut self) -> Result<(), ExecutionError> {
        let open_orders = self
            .client
            .get_open_orders()
            .await
            .map_err(|e| ExecutionError::Other(format!("Failed to sync positions: {}", e)))?;

        info!(
            "Synced with Kraken: {} open orders, {} local positions",
            open_orders.len(),
            self.positions.len()
        );

        // Reconcile stop orders
        for order in &open_orders {
            if order.order_type == "stop-loss" || order.order_type == "take-profit" {
                // Find the position this stop belongs to
                for (pos_id, pos) in &self.positions {
                    if pos.pair == order.pair && !self.stop_orders.contains_key(pos_id) {
                        self.stop_orders.insert(pos_id.clone(), order.txid.clone());
                        info!(
                            "Reconciled stop order {} for position {}",
                            order.txid, pos_id
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Place a stop-loss order on Kraken for a position.
    pub async fn place_stop_loss(&mut self, position_id: &str) -> Result<(), ExecutionError> {
        let pos = self
            .positions
            .get(position_id)
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?
            .clone();

        let stop_side = match pos.side {
            Side::Long => "sell",
            Side::Short => "buy",
        };

        match self
            .client
            .add_order(
                &pos.pair,
                stop_side,
                "stop-loss",
                pos.quantity,
                None,
                Some(pos.stop_loss),
            )
            .await
        {
            Ok(txids) => {
                if let Some(txid) = txids.first() {
                    self.stop_orders
                        .insert(position_id.to_string(), txid.clone());
                    info!(
                        "Stop-loss placed for {}: {} @ {:.2} (txid: {})",
                        position_id, stop_side, pos.stop_loss, txid
                    );
                }
                Ok(())
            }
            Err(e) => {
                warn!("Failed to place stop-loss for {}: {}", position_id, e);
                Err(ExecutionError::Other(format!("Stop-loss failed: {}", e)))
            }
        }
    }

    /// Get the current balance.
    pub async fn sync_balance(&mut self) -> Result<(), ExecutionError> {
        match self.client.get_balance().await {
            Ok(balance_map) => {
                let usd = balance_map
                    .get("ZUSD")
                    .or(balance_map.get("USD"))
                    .copied()
                    .unwrap_or(self.balance);
                self.balance = usd;
                debug!("Balance synced: ${:.2}", usd);
                Ok(())
            }
            Err(e) => {
                warn!("Balance sync failed: {}", e);
                Ok(())
            }
        }
    }

    /// Cancel all orders and close all positions (kill switch).
    pub async fn kill(&mut self) -> Result<(), ExecutionError> {
        error!("KILL SWITCH ACTIVATED — cancelling all orders");

        // Cancel all open orders
        let count = self
            .client
            .cancel_all_orders()
            .await
            .map_err(|e| ExecutionError::Other(format!("Cancel all failed: {}", e)))?;

        info!("Cancelled {} orders", count);

        // Close all positions
        let position_ids: Vec<String> = self.positions.keys().cloned().collect();
        for pos_id in position_ids {
            if let Err(e) = self.close_position(&pos_id).await {
                warn!("Failed to close position {}: {}", pos_id, e);
            }
        }

        self.halted = true;
        Ok(())
    }

    /// Check if the trader is halted.
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Get daily P&L.
    pub fn daily_pnl(&self) -> f64 {
        self.daily_pnl
    }
}

#[async_trait]
impl ExecutionEngine for KrakenTrader {
    async fn place_order(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        price: Option<f64>,
    ) -> Result<Order, ExecutionError> {
        // Safety checks
        if self.halted {
            return Err(ExecutionError::Other("Trading halted".into()));
        }

        self.check_daily_loss();
        if self.halted {
            return Err(ExecutionError::Other("Daily loss limit exceeded".into()));
        }

        // Check minimum order size
        let min_size = self.min_order_sizes.get(pair).copied().unwrap_or(0.0001);
        if quantity < min_size {
            return Err(ExecutionError::Other(format!(
                "Order size {} below minimum {} for {}",
                quantity, min_size, pair
            )));
        }

        // Check max order size
        let max_value = self.balance * self.config.max_order_pct;
        let order_value = price.unwrap_or(0.0) * quantity;
        if order_value > max_value && order_value > 0.0 {
            return Err(ExecutionError::Other(format!(
                "Order value ${:.2} exceeds max ${:.2} ({:.0}% of balance)",
                order_value,
                max_value,
                self.config.max_order_pct * 100.0
            )));
        }

        let side_str = match side {
            Side::Long => "buy",
            Side::Short => "sell",
        };
        let order_type = if price.is_some() { "limit" } else { "market" };

        // Place order on Kraken
        let txids = self
            .client
            .add_order(pair, side_str, order_type, quantity, price, None)
            .await
            .map_err(|e| ExecutionError::Other(format!("Order failed: {}", e)))?;

        let txid = txids.first().cloned().unwrap_or_default();

        // For market orders, fetch ticker price as fill price (C4 fix)
        let fill_price = match price {
            Some(p) => p,
            None => {
                // Fetch current market price from Kraken ticker
                match self.client.get_ticker(pair).await {
                    Ok(ticker) => match side {
                        Side::Long => ticker.ask,  // Buy at ask
                        Side::Short => ticker.bid, // Sell at bid
                    },
                    Err(_) => {
                        return Err(ExecutionError::Other(
                            "Market order placed but cannot determine fill price".into(),
                        ));
                    }
                }
            }
        };

        let fee = fill_price * quantity * self.config.fee_rate;
        let order_value = fill_price * quantity;

        self.order_counter += 1;
        let position_id = format!("{}-{}", txid, self.order_counter);

        let order = Order {
            id: txid.clone(),
            pair: pair.to_string(),
            side,
            order_type: if price.is_some() {
                OrderType::Limit
            } else {
                OrderType::Market
            },
            price: Some(fill_price),
            quantity,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(fill_price),
            tx_hash: None,
        };

        // Create position and track it (C1 fix)
        let position = Position {
            id: position_id.clone(),
            pair: pair.to_string(),
            side,
            entry_price: fill_price,
            current_price: fill_price,
            stop_loss: 0.0, // Will be set by caller
            take_profit_1: 0.0,
            take_profit_2: 0.0,
            take_profit_3: 0.0,
            quantity,
            unrealized_pnl: 0.0,
            risk_amount: 0.0,
            strategy_name: "kraken_live".to_string(),
            opened_at: Utc::now(),
            scale_level: crate::core::types::ScaleLevel::Full,
        };
        self.positions.insert(position_id, position);

        // Deduct from balance
        self.balance -= order_value + fee;

        // Send webhook notification
        if let Some(ref url) = self.config.webhook_url {
            let msg = format!(
                "🟢 TRADE OPENED: {} {} {} @ ${:.2} | Qty: {:.4} | Fee: ${:.2}",
                side_str, quantity, pair, fill_price, quantity, fee
            );
            let url = url.clone();
            tokio::spawn(async move {
                let _ = send_webhook(&url, &msg).await;
            });
        }

        info!(
            "Kraken order filled: {} {} {} @ {:.2} (txid: {})",
            side_str, quantity, pair, fill_price, txid
        );

        Ok(order)
    }

    async fn close_position(&mut self, position_id: &str) -> Result<Order, ExecutionError> {
        let pos = self
            .positions
            .remove(position_id)
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?;

        // Cancel stop order if exists
        if let Some(stop_txid) = self.stop_orders.remove(position_id) {
            let _ = self.client.cancel_order(&stop_txid).await;
        }

        let close_side = match pos.side {
            Side::Long => "sell",
            Side::Short => "buy",
        };

        // Place market close order
        let txids = self
            .client
            .add_order(&pos.pair, close_side, "market", pos.quantity, None, None)
            .await
            .map_err(|e| ExecutionError::Other(format!("Close failed: {}", e)))?;

        let txid = txids.first().cloned().unwrap_or_default();

        // Fetch current market price for accurate P&L (C5 fix)
        let exit_price = match self.client.get_ticker(&pos.pair).await {
            Ok(ticker) => match pos.side {
                Side::Long => ticker.bid,  // Sell at bid
                Side::Short => ticker.ask, // Buy at ask
            },
            Err(_) => pos.current_price, // Fallback to last known price
        };

        let pnl = match pos.side {
            Side::Long => (exit_price - pos.entry_price) * pos.quantity,
            Side::Short => (pos.entry_price - exit_price) * pos.quantity,
        };
        let fee = exit_price * pos.quantity * self.config.fee_rate;
        let net_pnl = pnl - fee;

        self.balance += net_pnl;
        self.daily_pnl += net_pnl;

        let trade = TradeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            pair: pos.pair.clone(),
            side: pos.side,
            entry_price: pos.entry_price,
            exit_price,
            quantity: pos.quantity,
            pnl: net_pnl,
            pnl_pct: net_pnl / (pos.entry_price * pos.quantity) * 100.0,
            fees: fee,
            opened_at: pos.opened_at,
            closed_at: Utc::now(),
            strategy_name: pos.strategy_name.clone(),
            notes: format!("Closed via Kraken (txid: {})", txid),
        };
        let trade_pnl_pct = trade.pnl_pct;
        self.closed_trades.push(trade);

        // Send webhook notification
        if let Some(ref url) = self.config.webhook_url {
            let emoji = if net_pnl >= 0.0 { "🟢" } else { "🔴" };
            let msg = format!(
                "{} TRADE CLOSED: {} {} | PnL: ${:.2} ({:.2}%) | Fee: ${:.2}",
                emoji, pos.pair, close_side, net_pnl, trade_pnl_pct, fee
            );
            let url = url.clone();
            tokio::spawn(async move {
                let _ = send_webhook(&url, &msg).await;
            });
        }

        info!(
            "Position closed: {} {} PnL: ${:.2}",
            pos.pair, close_side, net_pnl
        );

        self.check_daily_loss();

        Ok(Order {
            id: txid,
            pair: pos.pair,
            side: pos.side,
            order_type: OrderType::Market,
            price: Some(pos.current_price),
            quantity: pos.quantity,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(pos.current_price),
            tx_hash: None,
        })
    }

    fn open_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    fn balance(&self) -> f64 {
        self.balance
    }
}

/// Send a Discord webhook notification.
async fn send_webhook(url: &str, content: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "content": content });

    client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Webhook error: {}", e))?;

    Ok(())
}
