//! Monitoring and journaling — trade recording and performance metrics.
//!
//! - `journal` — SQLite-backed trade journal for persistence
//! - `metrics` — Win rate, profit factor, expectancy, and drawdown calculation
//! - `report` — CLI report generation for historical performance

pub mod journal;
pub mod metrics;
pub mod report;
