use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::core::error::ExecutionError;
use crate::core::types::{
    AccountState, Order, OrderStatus, OrderType, Position, ScaleLevel, Side, TradeRecord,
};
use crate::execution::engine::ExecutionEngine;

pub struct PaperTrader {
    positions: HashMap<String, Position>,
    closed_trades: Vec<TradeRecord>,
    account: AccountState,
    order_counter: u64,
    fee_rate: f64,
    slippage_pct: f64,
    last_reset_date: NaiveDate,
    /// Current ATR for dynamic slippage calculation
    current_atr: f64,
    /// Average ATR over lookback period (for scaling slippage)
    avg_atr: f64,
    /// Top-of-book depth (sum of top 5 bid/ask volumes) per pair
    book_depth: HashMap<String, f64>,
    /// Kraken maker fee rate (0.16% vs 0.26% taker)
    maker_fee_rate: f64,
    /// Current best bid per pair (for maker routing)
    best_bid: HashMap<String, f64>,
    /// Current best ask per pair (for maker routing)
    best_ask: HashMap<String, f64>,
}

impl PaperTrader {
    pub fn new(starting_balance: f64, fee_rate: f64, slippage_pct: f64) -> Self {
        Self {
            positions: HashMap::new(),
            closed_trades: Vec::new(),
            account: AccountState::new(starting_balance),
            order_counter: 0,
            fee_rate,
            slippage_pct,
            last_reset_date: Utc::now().date_naive(),
            current_atr: 0.0,
            avg_atr: 0.0,
            book_depth: HashMap::new(),
            maker_fee_rate: 0.0025, // Kraken maker fee: 0.25%
            best_bid: HashMap::new(),
            best_ask: HashMap::new(),
        }
    }

    /// Update ATR values for dynamic slippage calculation.
    pub fn update_atr(&mut self, current_atr: f64, avg_atr: f64) {
        self.current_atr = current_atr;
        self.avg_atr = avg_atr;
    }

    /// Update order book depth for a pair (sum of top 5 bid/ask volumes).
    pub fn update_book_depth(&mut self, pair: &str, depth: f64) {
        self.book_depth.insert(pair.to_string(), depth);
    }

    /// Update best bid/ask for maker-fee routing.
    pub fn update_book_prices(&mut self, pair: &str, bid: f64, ask: f64) {
        self.best_bid.insert(pair.to_string(), bid);
        self.best_ask.insert(pair.to_string(), ask);
    }

    /// Get current spread in basis points for a pair.
    pub fn spread_bps(&self, pair: &str) -> f64 {
        if let (Some(&bid), Some(&ask)) = (self.best_bid.get(pair), self.best_ask.get(pair)) {
            if bid > 0.0 {
                return (ask - bid) / bid * 10000.0;
            }
        }
        0.0
    }

    /// Should we use a limit order (maker) instead of market (taker)?
    ///
    /// If spread > fee differential (taker - maker = 0.10%), posting a limit
    /// order at bid/ask saves more than the spread costs.
    fn should_use_maker(&self, pair: &str) -> bool {
        let spread = self.spread_bps(pair);
        let fee_diff_bps = (self.fee_rate - self.maker_fee_rate) * 10000.0;
        spread > fee_diff_bps
    }

    /// Calculate dynamic slippage for an order.
    ///
    /// Slippage increases with:
    /// - Order size relative to book depth (larger orders move the market)
    /// - Current ATR vs average ATR (volatile markets have wider spreads)
    fn dynamic_slippage(&self, pair: &str, order_value: f64) -> f64 {
        let base = self.slippage_pct;

        // ATR multiplier: scales from 1.0 (calm) to 3.0 (very volatile)
        let atr_mult = if self.avg_atr > 0.0 {
            1.0 + (self.current_atr / self.avg_atr).min(3.0)
        } else {
            1.0
        };

        // Book depth impact: order_value / depth
        let depth_impact = if let Some(&depth) = self.book_depth.get(pair) {
            if depth > 0.0 {
                (order_value / depth).min(0.01) // Cap at 1% additional slippage
            } else {
                0.0
            }
        } else {
            0.0
        };

        base * atr_mult + depth_impact
    }

    pub fn update_prices(&mut self, prices: &HashMap<String, f64>) {
        // Reset daily PnL on new UTC day
        let today = Utc::now().date_naive();
        if today != self.last_reset_date {
            self.account.daily_pnl = 0.0;
            self.account.trades_today = 0;
            self.last_reset_date = today;
        }

        let mut total_unrealized = 0.0;
        for pos in self.positions.values_mut() {
            if let Some(&price) = prices.get(&pos.pair) {
                pos.current_price = price;
                pos.unrealized_pnl = match pos.side {
                    Side::Long => (price - pos.entry_price) * pos.quantity,
                    Side::Short => (pos.entry_price - price) * pos.quantity,
                };
                total_unrealized += pos.unrealized_pnl;
            }
        }
        self.account.update_equity(total_unrealized);
    }

    pub fn check_stops(&mut self, prices: &HashMap<String, f64>) -> Vec<TradeRecord> {
        let mut closed = Vec::new();
        let mut to_remove = Vec::new();
        let mut to_update: Vec<(String, f64, ScaleLevel)> = Vec::new();

        for (id, pos) in &self.positions {
            if let Some(&price) = prices.get(&pos.pair) {
                // Check stop loss — full close
                let hit_stop = match pos.side {
                    Side::Long => price <= pos.stop_loss,
                    Side::Short => price >= pos.stop_loss,
                };

                if hit_stop {
                    let raw_exit = pos.stop_loss;
                    let slippage = raw_exit * self.slippage_pct;
                    let exit_price = match pos.side {
                        Side::Long => raw_exit - slippage,
                        Side::Short => raw_exit + slippage,
                    };
                    let exit_fee = exit_price * pos.quantity * self.fee_rate;
                    let pnl = match pos.side {
                        Side::Long => (exit_price - pos.entry_price) * pos.quantity - exit_fee,
                        Side::Short => (pos.entry_price - exit_price) * pos.quantity - exit_fee,
                    };
                    let pnl_pct = pnl / (pos.entry_price * pos.quantity) * 100.0;

                    let trade = TradeRecord {
                        id: uuid(),
                        pair: pos.pair.clone(),
                        side: pos.side,
                        entry_price: pos.entry_price,
                        exit_price,
                        quantity: pos.quantity,
                        pnl,
                        pnl_pct,
                        strategy_name: pos.strategy_name.clone(),
                        opened_at: pos.opened_at,
                        closed_at: Utc::now(),
                        notes: format!("Stop loss hit ({:?})", pos.scale_level),
                    };

                    self.account.balance += pnl;
                    self.account.daily_pnl += pnl;
                    self.closed_trades.push(trade.clone());
                    closed.push(trade);
                    to_remove.push(id.clone());
                    continue;
                }

                // Check take-profit levels with scale-out
                match pos.scale_level {
                    ScaleLevel::Full => {
                        // Check TP1: close 50%, move SL to break-even
                        let hit_tp1 = match pos.side {
                            Side::Long => price >= pos.take_profit_1,
                            Side::Short => price <= pos.take_profit_1,
                        };

                        if hit_tp1 {
                            let scale_qty = pos.quantity * 0.5;
                            let raw_exit = pos.take_profit_1;
                            let slippage = raw_exit * self.slippage_pct;
                            let exit_price = match pos.side {
                                Side::Long => raw_exit - slippage,
                                Side::Short => raw_exit + slippage,
                            };
                            let exit_fee = exit_price * scale_qty * self.fee_rate;
                            let pnl = match pos.side {
                                Side::Long => (exit_price - pos.entry_price) * scale_qty - exit_fee,
                                Side::Short => {
                                    (pos.entry_price - exit_price) * scale_qty - exit_fee
                                }
                            };
                            let pnl_pct = pnl / (pos.entry_price * scale_qty) * 100.0;

                            let trade = TradeRecord {
                                id: uuid(),
                                pair: pos.pair.clone(),
                                side: pos.side,
                                entry_price: pos.entry_price,
                                exit_price,
                                quantity: scale_qty,
                                pnl,
                                pnl_pct,
                                strategy_name: pos.strategy_name.clone(),
                                opened_at: pos.opened_at,
                                closed_at: Utc::now(),
                                notes: "TP1 hit — scale out 50%, SL → break-even".to_string(),
                            };

                            self.account.balance += pnl;
                            self.account.daily_pnl += pnl;
                            self.closed_trades.push(trade.clone());
                            closed.push(trade);

                            // Remaining 50%: move SL to break-even, advance scale level
                            to_update.push((id.clone(), pos.entry_price, ScaleLevel::Scaled50));
                        }
                    }
                    ScaleLevel::Scaled50 => {
                        // Check TP2: close 60% of remaining (30% of original)
                        let hit_tp2 = match pos.side {
                            Side::Long => price >= pos.take_profit_2,
                            Side::Short => price <= pos.take_profit_2,
                        };

                        if hit_tp2 {
                            let scale_qty = pos.quantity * 0.6;
                            let raw_exit = pos.take_profit_2;
                            let slippage = raw_exit * self.slippage_pct;
                            let exit_price = match pos.side {
                                Side::Long => raw_exit - slippage,
                                Side::Short => raw_exit + slippage,
                            };
                            let exit_fee = exit_price * scale_qty * self.fee_rate;
                            let pnl = match pos.side {
                                Side::Long => (exit_price - pos.entry_price) * scale_qty - exit_fee,
                                Side::Short => {
                                    (pos.entry_price - exit_price) * scale_qty - exit_fee
                                }
                            };
                            let pnl_pct = pnl / (pos.entry_price * scale_qty) * 100.0;

                            let trade = TradeRecord {
                                id: uuid(),
                                pair: pos.pair.clone(),
                                side: pos.side,
                                entry_price: pos.entry_price,
                                exit_price,
                                quantity: scale_qty,
                                pnl,
                                pnl_pct,
                                strategy_name: pos.strategy_name.clone(),
                                opened_at: pos.opened_at,
                                closed_at: Utc::now(),
                                notes: "TP2 hit — scale out 60% of remaining".to_string(),
                            };

                            self.account.balance += pnl;
                            self.account.daily_pnl += pnl;
                            self.closed_trades.push(trade.clone());
                            closed.push(trade);

                            // Remaining 40%: advance to Scaled80
                            to_update.push((id.clone(), pos.stop_loss, ScaleLevel::Scaled80));
                        }
                    }
                    ScaleLevel::Scaled80 => {
                        // Check TP3: close remaining 100%
                        let hit_tp3 = match pos.side {
                            Side::Long => price >= pos.take_profit_3,
                            Side::Short => price <= pos.take_profit_3,
                        };

                        if hit_tp3 {
                            let raw_exit = pos.take_profit_3;
                            let slippage = raw_exit * self.slippage_pct;
                            let exit_price = match pos.side {
                                Side::Long => raw_exit - slippage,
                                Side::Short => raw_exit + slippage,
                            };
                            let exit_fee = exit_price * pos.quantity * self.fee_rate;
                            let pnl = match pos.side {
                                Side::Long => {
                                    (exit_price - pos.entry_price) * pos.quantity - exit_fee
                                }
                                Side::Short => {
                                    (pos.entry_price - exit_price) * pos.quantity - exit_fee
                                }
                            };
                            let pnl_pct = pnl / (pos.entry_price * pos.quantity) * 100.0;

                            let trade = TradeRecord {
                                id: uuid(),
                                pair: pos.pair.clone(),
                                side: pos.side,
                                entry_price: pos.entry_price,
                                exit_price,
                                quantity: pos.quantity,
                                pnl,
                                pnl_pct,
                                strategy_name: pos.strategy_name.clone(),
                                opened_at: pos.opened_at,
                                closed_at: Utc::now(),
                                notes: "TP3 hit — full close".to_string(),
                            };

                            self.account.balance += pnl;
                            self.account.daily_pnl += pnl;
                            self.closed_trades.push(trade.clone());
                            closed.push(trade);
                            to_remove.push(id.clone());
                        }
                    }
                    ScaleLevel::Closed => {}
                }
            }
        }

        // Apply scale-level and stop-loss updates
        for (id, new_sl, new_scale) in to_update {
            if let Some(pos) = self.positions.get_mut(&id) {
                pos.stop_loss = new_sl;
                pos.scale_level = new_scale;
            }
        }

        for id in to_remove {
            self.positions.remove(&id);
            self.account.open_positions = self.positions.len();
        }

        closed
    }

    pub fn account(&self) -> &AccountState {
        &self.account
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

    pub fn account_mut(&mut self) -> &mut AccountState {
        &mut self.account
    }

    pub fn set_balance(&mut self, balance: f64) {
        self.account.balance = balance;
        self.account.equity = balance;
        self.account.peak_equity = balance;
    }

    /// Save engine state to disk for crash recovery (PROD-3).
    pub fn save_state(&self, path: &str) -> Result<(), String> {
        let state = serde_json::json!({
            "account": self.account,
            "positions": self.positions,
            "closed_trades": self.closed_trades,
            "order_counter": self.order_counter,
            "last_reset_date": self.last_reset_date.to_string(),
            "saved_at": Utc::now().to_rfc3339(),
        });

        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Create dir: {}", e))?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&state).unwrap())
            .map_err(|e| format!("Write state: {}", e))?;
        Ok(())
    }

    /// Load engine state from disk (PROD-3).
    pub fn load_state(&mut self, path: &str) -> Result<(), String> {
        let data = std::fs::read_to_string(path).map_err(|e| format!("Read state: {}", e))?;
        let state: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("Parse state: {}", e))?;

        if let Some(account) = state.get("account") {
            if let Ok(acct) = serde_json::from_value::<AccountState>(account.clone()) {
                self.account = acct;
            }
        }
        if let Some(positions) = state.get("positions") {
            if let Ok(pos) = serde_json::from_value::<HashMap<String, Position>>(positions.clone())
            {
                self.positions = pos;
                self.account.open_positions = self.positions.len();
            }
        }
        if let Some(trades) = state.get("closed_trades") {
            if let Ok(t) = serde_json::from_value::<Vec<TradeRecord>>(trades.clone()) {
                self.closed_trades = t;
            }
        }
        if let Some(counter) = state.get("order_counter") {
            if let Some(c) = counter.as_u64() {
                self.order_counter = c;
            }
        }
        if let Some(date) = state.get("last_reset_date") {
            if let Some(d) = date.as_str() {
                if let Ok(nd) = d.parse::<NaiveDate>() {
                    self.last_reset_date = nd;
                }
            }
        }

        info!(
            "State loaded: balance=${:.2}, positions={}, trades={}",
            self.account.balance,
            self.positions.len(),
            self.closed_trades.len()
        );
        Ok(())
    }
}

#[async_trait]
impl ExecutionEngine for PaperTrader {
    async fn place_order(
        &mut self,
        pair: &str,
        side: Side,
        quantity: f64,
        price: Option<f64>,
    ) -> Result<Order, ExecutionError> {
        let raw_price = price.unwrap_or(0.0);
        let order_value = raw_price * quantity;

        // Maker-fee routing: if spread > fee differential, use limit at bid/ask
        let use_maker = self.should_use_maker(pair);
        let effective_fee = if use_maker {
            self.maker_fee_rate
        } else {
            self.fee_rate
        };

        let slippage = if use_maker {
            0.0 // Limit order at bid/ask — no slippage
        } else {
            self.dynamic_slippage(pair, order_value)
        };

        let fill_price = if use_maker {
            // Post at bid (buy) or ask (sell) — fill at the passive side
            match side {
                Side::Long => self.best_bid.get(pair).copied().unwrap_or(raw_price),
                Side::Short => self.best_ask.get(pair).copied().unwrap_or(raw_price),
            }
        } else {
            match side {
                Side::Long => raw_price * (1.0 + slippage),
                Side::Short => raw_price * (1.0 - slippage),
            }
        };

        let cost = fill_price * quantity;
        let fee = cost * effective_fee;

        if cost + fee > self.account.balance {
            return Err(ExecutionError::InsufficientBalance {
                needed: cost + fee,
                available: self.account.balance,
            });
        }

        self.account.balance -= cost + fee;

        self.order_counter += 1;
        let order = Order {
            id: format!("paper-{}", self.order_counter),
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
        };

        info!(
            "Paper order filled: {} {} {} @ {:.2} ({})",
            order.side,
            quantity,
            pair,
            fill_price,
            if use_maker { "MAKER" } else { "TAKER" }
        );

        Ok(order)
    }

    async fn close_position(&mut self, position_id: &str) -> Result<Order, ExecutionError> {
        let pos = self
            .positions
            .remove(position_id)
            .ok_or_else(|| ExecutionError::PositionNotFound(position_id.to_string()))?;

        let pnl = match pos.side {
            Side::Long => (pos.current_price - pos.entry_price) * pos.quantity,
            Side::Short => (pos.entry_price - pos.current_price) * pos.quantity,
        };

        if self.account.balance + pnl < 0.0 {
            warn!(
                "Close would push balance negative: balance={:.2}, pnl={:.2}",
                self.account.balance, pnl
            );
        }
        self.account.balance += pnl;
        self.account.daily_pnl += pnl;
        self.account.open_positions = self.positions.len();

        self.order_counter += 1;
        Ok(Order {
            id: format!("paper-close-{}", self.order_counter),
            pair: pos.pair.clone(),
            side: match pos.side {
                Side::Long => Side::Short,
                Side::Short => Side::Long,
            },
            order_type: OrderType::Market,
            price: Some(pos.current_price),
            quantity: pos.quantity,
            status: OrderStatus::Filled,
            created_at: Utc::now(),
            filled_at: Some(Utc::now()),
            filled_price: Some(pos.current_price),
        })
    }

    fn open_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    fn balance(&self) -> f64 {
        self.account.balance
    }
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = t.as_nanos();
    let pid = std::process::id();
    format!(
        "{:08x}-{:08x}-{:08x}-{:08x}",
        (nanos >> 96) as u32,
        (nanos >> 64) as u32,
        (nanos >> 32) as u32,
        (nanos as u32) ^ pid
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position(id: &str, entry: f64, sl: f64, tp1: f64, tp2: f64, tp3: f64) -> Position {
        Position {
            id: id.to_string(),
            pair: "BTC/USD".to_string(),
            side: Side::Long,
            entry_price: entry,
            current_price: entry,
            quantity: 1.0,
            stop_loss: sl,
            take_profit_1: tp1,
            take_profit_2: tp2,
            take_profit_3: tp3,
            unrealized_pnl: 0.0,
            risk_amount: entry - sl,
            strategy_name: "test".to_string(),
            opened_at: Utc::now(),
            scale_level: ScaleLevel::Full,
        }
    }

    #[test]
    fn stop_loss_full_close() {
        let mut trader = PaperTrader::new(1000.0, 0.001, 0.0005);
        let pos = make_position("p1", 100.0, 95.0, 105.0, 110.0, 115.0);
        trader.positions.insert(pos.id.clone(), pos);

        let mut prices = HashMap::new();
        prices.insert("BTC/USD".to_string(), 94.0);

        let closed = trader.check_stops(&prices);
        assert_eq!(closed.len(), 1);
        assert!(closed[0].notes.contains("Stop loss"));
        assert!(trader.positions.is_empty());
    }

    #[test]
    fn tp1_scales_out_50_percent() {
        let mut trader = PaperTrader::new(1000.0, 0.001, 0.0005);
        let pos = make_position("p1", 100.0, 95.0, 105.0, 110.0, 115.0);
        trader.positions.insert(pos.id.clone(), pos);

        let mut prices = HashMap::new();
        prices.insert("BTC/USD".to_string(), 106.0);

        let closed = trader.check_stops(&prices);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].quantity, 0.5);
        assert!(closed[0].notes.contains("TP1"));

        // Remaining position should be at Scaled50 with SL moved to break-even
        let remaining = trader.positions.get("p1").unwrap();
        assert_eq!(remaining.scale_level, ScaleLevel::Scaled50);
        assert_eq!(remaining.stop_loss, 100.0); // break-even
        assert_eq!(remaining.quantity, 1.0); // quantity unchanged on original
    }

    #[test]
    fn tp2_scales_out_after_tp1() {
        let mut trader = PaperTrader::new(1000.0, 0.001, 0.0005);
        let mut pos = make_position("p1", 100.0, 95.0, 105.0, 110.0, 115.0);
        pos.scale_level = ScaleLevel::Scaled50;
        pos.stop_loss = 100.0; // already at break-even
        trader.positions.insert(pos.id.clone(), pos);

        let mut prices = HashMap::new();
        prices.insert("BTC/USD".to_string(), 111.0);

        let closed = trader.check_stops(&prices);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].quantity, 0.6); // 60% of remaining
        assert!(closed[0].notes.contains("TP2"));

        let remaining = trader.positions.get("p1").unwrap();
        assert_eq!(remaining.scale_level, ScaleLevel::Scaled80);
    }

    #[test]
    fn tp3_full_close_after_scale_out() {
        let mut trader = PaperTrader::new(1000.0, 0.001, 0.0005);
        let mut pos = make_position("p1", 100.0, 95.0, 105.0, 110.0, 115.0);
        pos.scale_level = ScaleLevel::Scaled80;
        pos.stop_loss = 100.0;
        trader.positions.insert(pos.id.clone(), pos);

        let mut prices = HashMap::new();
        prices.insert("BTC/USD".to_string(), 116.0);

        let closed = trader.check_stops(&prices);
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].quantity, 1.0);
        assert!(closed[0].notes.contains("TP3"));
        assert!(trader.positions.is_empty());
    }

    #[test]
    fn stop_at_break_even_after_tp1() {
        let mut trader = PaperTrader::new(1000.0, 0.001, 0.0005);
        let mut pos = make_position("p1", 100.0, 95.0, 105.0, 110.0, 115.0);
        pos.scale_level = ScaleLevel::Scaled50;
        pos.stop_loss = 100.0; // break-even
        trader.positions.insert(pos.id.clone(), pos);

        let mut prices = HashMap::new();
        prices.insert("BTC/USD".to_string(), 99.0);

        let closed = trader.check_stops(&prices);
        assert_eq!(closed.len(), 1);
        assert!(closed[0].notes.contains("Stop loss"));
        assert!(trader.positions.is_empty());
    }

    #[test]
    fn uuid_format_is_valid() {
        let id = uuid();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 4);
        for part in parts {
            assert_eq!(part.len(), 8);
            assert!(u32::from_str_radix(part, 16).is_ok());
        }
    }
}
