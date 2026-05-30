use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::core::error::ExecutionError;
use crate::core::types::{
    AccountState, Order, OrderStatus, OrderType, Position, Side, TradeRecord,
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

        for (id, pos) in &self.positions {
            if let Some(&price) = prices.get(&pos.pair) {
                let hit_stop = match pos.side {
                    Side::Long => price <= pos.stop_loss,
                    Side::Short => price >= pos.stop_loss,
                };

                let hit_tp1 = match pos.side {
                    Side::Long => price >= pos.take_profit_1,
                    Side::Short => price <= pos.take_profit_1,
                };

                if hit_stop || hit_tp1 {
                    let raw_exit = if hit_stop {
                        pos.stop_loss
                    } else {
                        pos.take_profit_1
                    };
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
                        notes: if hit_stop {
                            "Stop loss hit".to_string()
                        } else {
                            "Take profit 1 hit".to_string()
                        },
                    };

                    if self.account.balance + pnl < 0.0 {
                        warn!(
                            "Trade would push balance negative: balance={:.2}, pnl={:.2}",
                            self.account.balance, pnl
                        );
                    }
                    self.account.balance += pnl;
                    self.account.daily_pnl += pnl;
                    self.closed_trades.push(trade.clone());
                    closed.push(trade);
                    to_remove.push(id.clone());
                }
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
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", t)
}
