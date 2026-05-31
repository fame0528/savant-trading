use crate::core::types::OrderBook;

pub struct OrderBookManager {
    current: Option<OrderBook>,
    _pair: String,
}

impl OrderBookManager {
    pub fn new(pair: &str) -> Self {
        Self {
            current: None,
            _pair: pair.to_string(),
        }
    }

    pub fn update(&mut self, book: OrderBook) {
        self.current = Some(book);
    }

    pub fn current(&self) -> Option<&OrderBook> {
        self.current.as_ref()
    }

    pub fn mid_price(&self) -> Option<f64> {
        self.current.as_ref().and_then(|b| b.mid_price())
    }

    pub fn spread(&self) -> Option<f64> {
        self.current.as_ref().and_then(|b| b.spread())
    }

    pub fn bid_depth(&self, levels: usize) -> f64 {
        self.current
            .as_ref()
            .map(|b| b.bids.iter().take(levels).map(|l| l.volume).sum())
            .unwrap_or(0.0)
    }

    pub fn ask_depth(&self, levels: usize) -> f64 {
        self.current
            .as_ref()
            .map(|b| b.asks.iter().take(levels).map(|l| l.volume).sum())
            .unwrap_or(0.0)
    }

    pub fn imbalance(&self, levels: usize) -> f64 {
        let bid = self.bid_depth(levels);
        let ask = self.ask_depth(levels);
        let total = bid + ask;
        if total == 0.0 {
            0.0
        } else {
            (bid - ask) / total
        }
    }
}
