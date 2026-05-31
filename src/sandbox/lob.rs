//! Order book simulator using simplified Hawkes process.
//!
//! Generates realistic bid/ask depth with dynamic liquidity
//! that decays during high volatility.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::core::types::OrderBookLevel;

/// Configuration for order book simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobConfig {
    /// Number of levels per side
    pub depth: usize,
    /// Base spread as fraction of price (e.g., 0.0001 = 1 bps)
    pub base_spread: f64,
    /// Base volume per level
    pub base_volume: f64,
    /// Volatility multiplier for spread widening
    pub volatility_spread_mult: f64,
}

impl Default for LobConfig {
    fn default() -> Self {
        Self {
            depth: 10,
            base_spread: 0.0001,
            base_volume: 1.0,
            volatility_spread_mult: 1.0,
        }
    }
}

/// Generate a simulated order book around a mid price.
pub fn generate_order_book(
    mid_price: f64,
    config: &LobConfig,
    volatility: f64,
) -> (Vec<OrderBookLevel>, Vec<OrderBookLevel>) {
    let mut rng = rand::thread_rng();

    let spread =
        mid_price * config.base_spread * (1.0 + volatility * config.volatility_spread_mult);
    let half_spread = spread / 2.0;

    let mut bids = Vec::with_capacity(config.depth);
    let mut asks = Vec::with_capacity(config.depth);

    for i in 0..config.depth {
        let level_offset = mid_price * config.base_spread * (i + 1) as f64;

        // Bid side
        let bid_price = mid_price - half_spread - level_offset;
        let bid_volume = config.base_volume * rng.gen_range(0.3..2.0);
        bids.push(OrderBookLevel {
            price: bid_price,
            volume: bid_volume,
        });

        // Ask side
        let ask_price = mid_price + half_spread + level_offset;
        let ask_volume = config.base_volume * rng.gen_range(0.3..2.0);
        asks.push(OrderBookLevel {
            price: ask_price,
            volume: ask_volume,
        });
    }

    (bids, asks)
}

/// Calculate order book imbalance (-1.0 to 1.0).
/// Positive = bid-heavy (buying pressure).
/// Negative = ask-heavy (selling pressure).
pub fn calculate_imbalance(bids: &[OrderBookLevel], asks: &[OrderBookLevel], depth: usize) -> f64 {
    let bid_vol: f64 = bids.iter().take(depth).map(|l| l.volume).sum();
    let ask_vol: f64 = asks.iter().take(depth).map(|l| l.volume).sum();
    let total = bid_vol + ask_vol;
    if total == 0.0 {
        0.0
    } else {
        (bid_vol - ask_vol) / total
    }
}

/// Simulate slippage for a market order of given size.
pub fn simulate_slippage(
    _side: crate::core::types::Side,
    size: f64,
    book: &[OrderBookLevel],
) -> f64 {
    let mut remaining = size;
    let mut total_cost = 0.0;

    for level in book {
        let fill = remaining.min(level.volume);
        total_cost += fill * level.price;
        remaining -= fill;
        if remaining <= 0.0 {
            break;
        }
    }

    if size > 0.0 {
        total_cost / size
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_order_book_valid() {
        let config = LobConfig::default();
        let (bids, asks) = generate_order_book(100000.0, &config, 0.5);
        assert_eq!(bids.len(), 10);
        assert_eq!(asks.len(), 10);
        // Bids should be below mid, asks above
        assert!(bids[0].price < 100000.0);
        assert!(asks[0].price > 100000.0);
        // All volumes positive
        for level in bids.iter().chain(asks.iter()) {
            assert!(level.volume > 0.0);
        }
    }

    #[test]
    fn imbalance_balanced() {
        let bids = vec![
            OrderBookLevel {
                price: 99.0,
                volume: 1.0,
            },
            OrderBookLevel {
                price: 98.0,
                volume: 1.0,
            },
        ];
        let asks = vec![
            OrderBookLevel {
                price: 101.0,
                volume: 1.0,
            },
            OrderBookLevel {
                price: 102.0,
                volume: 1.0,
            },
        ];
        let imb = calculate_imbalance(&bids, &asks, 2);
        assert!((imb).abs() < 0.01);
    }

    #[test]
    fn imbalance_bid_heavy() {
        let bids = vec![OrderBookLevel {
            price: 99.0,
            volume: 4.0,
        }];
        let asks = vec![OrderBookLevel {
            price: 101.0,
            volume: 1.0,
        }];
        let imb = calculate_imbalance(&bids, &asks, 1);
        assert!(imb > 0.5);
    }
}
