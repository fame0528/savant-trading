//! Execution layer — order placement and position management.
//!
//! - `engine` — ExecutionEngine trait defining the order interface
//! - `portfolio` — Portfolio state manager with stop/TP monitoring
//! - `kraken` — Live execution engine for Kraken exchange

pub mod dex;
pub mod engine;
pub mod kraken;
pub mod portfolio;
