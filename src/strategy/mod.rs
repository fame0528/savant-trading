//! Strategy module — signal generation based on technical analysis.
//!
//! - `base` — Strategy trait defining the async evaluation interface
//! - `momentum` — Break of structure + volume spike breakout strategy
//! - `mean_reversion` — Volume profile mean reversion to point of control
//! - `regime` — ADX-based market regime detection (trending/ranging/volatile)
//! - `pre_scorer` — FID-222 Funnel v1 momentum pre-scorer + top-K selector

pub mod base;
pub mod mean_reversion;
pub mod momentum;
pub mod pre_scorer;
pub mod regime;
