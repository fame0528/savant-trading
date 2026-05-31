//! Walk-forward optimization with rolling windows.

use tracing::info;

use crate::backtest::engine::{run_backtest, BacktestConfig, BacktestResult};
use crate::backtest::metrics::BacktestMetrics;
use crate::core::types::Candle;
use crate::strategy::base::Strategy;

/// Walk-forward optimization configuration.
#[derive(Debug, Clone)]
pub struct WalkForwardConfig {
    /// In-sample window size (number of candles)
    pub in_sample_size: usize,
    /// Out-of-sample window size (number of candles)
    pub out_of_sample_size: usize,
    /// Step size for rolling window
    pub step_size: usize,
    /// Backtest configuration
    pub backtest_config: BacktestConfig,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            in_sample_size: 5000,
            out_of_sample_size: 1000,
            step_size: 500,
            backtest_config: BacktestConfig::default(),
        }
    }
}

/// Result of walk-forward optimization.
#[derive(Debug)]
pub struct WalkForwardResult {
    /// Metrics from stitching all OOS segments together
    pub aggregate_metrics: BacktestMetrics,
    /// Individual OOS segment results
    pub segments: Vec<WalkForwardSegment>,
    /// Total number of OOS trades
    pub total_oos_trades: usize,
}

/// A single walk-forward segment.
#[derive(Debug)]
pub struct WalkForwardSegment {
    pub segment_index: usize,
    pub in_sample_start: usize,
    pub in_sample_end: usize,
    pub oos_start: usize,
    pub oos_end: usize,
    pub oos_result: BacktestResult,
}

/// Run walk-forward optimization on historical candles.
pub fn run_walk_forward(
    candles: &[Candle],
    strategy: &dyn Strategy,
    config: &WalkForwardConfig,
) -> WalkForwardResult {
    let mut segments = Vec::new();
    let mut all_oos_trades = Vec::new();
    let mut all_equity_curves = Vec::new();
    let mut segment_index = 0;

    // M18: Track cumulative balance across segments
    let mut cumulative_balance = config.backtest_config.starting_balance;

    let mut start = 0;
    while start + config.in_sample_size + config.out_of_sample_size <= candles.len() {
        let is_start = start;
        let is_end = start + config.in_sample_size;
        let oos_start = is_end;
        let oos_end = (oos_start + config.out_of_sample_size).min(candles.len());

        info!(
            "Walk-forward segment {}: IS [{}..{}], OOS [{}..{}], balance=${:.2}",
            segment_index, is_start, is_end, oos_start, oos_end, cumulative_balance
        );

        // Run backtest on OOS segment with cumulative balance
        let oos_candles = &candles[oos_start..oos_end];
        let mut segment_config = config.backtest_config.clone();
        segment_config.starting_balance = cumulative_balance;
        let oos_result = run_backtest(oos_candles, strategy, &segment_config);

        // Update cumulative balance from segment result
        cumulative_balance = oos_result.metrics.final_balance;

        all_oos_trades.extend(oos_result.trades.clone());
        for (_, equity) in &oos_result.equity_curve {
            all_equity_curves.push((chrono::Utc::now(), *equity));
        }

        segments.push(WalkForwardSegment {
            segment_index,
            in_sample_start: is_start,
            in_sample_end: is_end,
            oos_start,
            oos_end,
            oos_result,
        });

        segment_index += 1;
        start += config.step_size;
    }

    let aggregate_metrics = BacktestMetrics::calculate(
        &all_oos_trades,
        config.backtest_config.starting_balance,
        &all_equity_curves,
    );

    info!(
        "Walk-forward complete: {} segments, {} OOS trades, final balance=${:.2}, {}",
        segments.len(),
        all_oos_trades.len(),
        cumulative_balance,
        aggregate_metrics
    );

    WalkForwardResult {
        aggregate_metrics,
        segments,
        total_oos_trades: all_oos_trades.len(),
    }
}
