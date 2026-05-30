//! Savant Trading Engine — automated crypto trading system for Kraken exchange.
//!
//! Built in Rust with tokio async runtime. Synthesized from expert trader
//! knowledge covering momentum, volume profile, order flow, risk management,
//! and market regime detection.
//!
//! # Architecture
//!
//! - `core` — Configuration, types, errors, events
//! - `data` — Market data fetching and indicator calculations
//! - `strategy` — Signal generation (momentum, mean reversion, regime)
//! - `risk` — Position sizing, stops, circuit breakers
//! - `execution` — Paper trading and live execution
//! - `monitor` — Trade journal and performance metrics

pub mod core;
pub mod data;
pub mod execution;
pub mod monitor;
pub mod risk;
pub mod strategy;
