//! Risk management — position sizing, stop losses, and circuit breakers.
//!
//! - `position` — Fixed-fractional position sizing with R:R filtering
//! - `stop_loss` — ATR-based and structure-based stop loss calculation
//! - `circuit_breaker` — Daily loss limit, max drawdown, and max position guards

pub mod circuit_breaker;
pub mod correlation;
pub mod position;
pub mod stop_loss;
