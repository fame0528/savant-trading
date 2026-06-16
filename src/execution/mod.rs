//! Execution layer — order placement and position management.
//!
//! - `engine` — ExecutionEngine trait defining the order interface
//! - `portfolio` — Portfolio state manager with stop/TP monitoring
//! - `reconciliation` — Wallet reconciliation heartbeat (FID-147) — periodic
//!   on-chain balance check that surfaces in-memory / on-chain state drift
//! - `wallet_recovery` — Chain-driven position recovery (FID-155 / DECISION-015).
//!   The on-chain state is the source of truth; this module queries the chain
//!   and rebuilds the in-memory position map from on-chain reality. Runs on
//!   engine startup and every 5 minutes for periodic reconciliation.

pub mod dex;
pub mod engine;
pub mod portfolio;
pub mod reconciliation;
pub mod wallet_recovery;
