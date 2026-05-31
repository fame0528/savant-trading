use std::collections::VecDeque;

use crate::core::types::Candle;

pub struct MarketDataStore {
    candles: VecDeque<Candle>,
    max_size: usize,
    pair: String,
}

impl MarketDataStore {
    pub fn new(pair: &str, max_size: usize) -> Self {
        Self {
            candles: VecDeque::with_capacity(max_size),
            max_size,
            pair: pair.to_string(),
        }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        if self.candles.len() >= self.max_size {
            self.candles.pop_front();
        }
        self.candles.push_back(candle);
    }

    pub fn add_candles(&mut self, new_candles: Vec<Candle>) {
        for candle in new_candles {
            self.add_candle(candle);
        }
    }

    pub fn candles(&self) -> &VecDeque<Candle> {
        &self.candles
    }

    pub fn last(&self) -> Option<&Candle> {
        self.candles.back()
    }

    pub fn last_n(&self, n: usize) -> Vec<&Candle> {
        let start = self.candles.len().saturating_sub(n);
        self.candles.range(start..).collect()
    }

    pub fn closes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.close).collect()
    }

    pub fn highs(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.high).collect()
    }

    pub fn lows(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.low).collect()
    }

    pub fn volumes(&self) -> Vec<f64> {
        self.candles.iter().map(|c| c.volume).collect()
    }

    pub fn len(&self) -> usize {
        self.candles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    pub fn pair(&self) -> &str {
        &self.pair
    }
}
