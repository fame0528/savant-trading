//! Core module — foundational types, configuration, error handling, and event bus.
//!
//! - `config` — Application configuration loaded from TOML
//! - `error` — Typed error hierarchy using thiserror
//! - `events` — Channel-based event bus for decoupled component communication
//! - `types` — Shared data structures (Candle, Signal, Position, Order, etc.)

pub mod config;
pub mod console;
pub mod error;
pub mod events;
pub mod session;
pub mod shared;
pub mod types;
