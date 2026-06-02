//! Monitoring and journaling — trade recording and performance metrics.
//!
//! - `journal` — SQLite-backed trade journal for persistence
//! - `metrics` — Win rate, profit factor, expectancy, and drawdown calculation
//! - `report` — CLI report generation for historical performance
//! - `training_report` — Full training audit report from test_memory.db

pub mod journal;
pub mod metrics;
pub mod report;
pub mod training_report;
