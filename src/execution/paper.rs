use async_trait::async_trait;
use chrono::Utc;
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
        }
    }

    pub fn update_prices(&mut self, prices: &HashMap<String, f64>) {
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
        let fill_price = price.unwrap_or(0.0);
        let cost = fill_price * quantity;
        let fee = cost * self.fee_rate;

        if cost + fee > self.account.balance {
            return Err(ExecutionError::InsufficientBalance {
                needed: cost + fee,
                available: self.account.balance,
            });
        }

        self.account.balance -= fee;

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
            "Paper order filled: {} {} {} @ {:.2}",
            order.side as u8, quantity, pair, fill_price
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
