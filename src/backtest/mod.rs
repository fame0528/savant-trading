//! Backtesting engine — historical strategy validation.
//!
//! Replays historical candles through strategies using the existing `Strategy` trait.
//! Supports walk-forward optimization with rolling windows.

pub mod engine;
pub mod metrics;
pub mod walk_forward;
