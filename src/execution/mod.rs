//! Execution layer — order placement and position management.
//!
//! - `engine` — ExecutionEngine trait defining the order interface
//! - `paper` — Paper trading simulator with stop/TP monitoring

pub mod engine;
pub mod kraken;
pub mod paper;
