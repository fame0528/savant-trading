//! Data layer — market data fetching, storage, and indicator calculations.
//!
//! - `kraken` — Kraken exchange REST API client for OHLCV and ticker data
//! - `market_data` — Sliding-window candle storage per trading pair
//! - `orderbook` — Order book depth processing and imbalance detection
//! - `indicators` — Technical indicators (EMA, SMA, RSI, ATR, ADX, VWAP, Volume Profile)
//! - `websocket` — Kraken WebSocket v2 client for real-time data

pub mod cache;
pub mod historical;
pub mod indicators;
pub mod kraken;
pub mod market_data;
pub mod orderbook;
pub mod sources;
pub mod tick_data;
pub mod websocket;
