//! Backtest performance metrics.

use chrono::{DateTime, Utc};

use crate::core::types::Side;

/// A completed backtest trade.
#[derive(Debug, Clone)]
pub struct BacktestTrade {
    pub pair: String,
    pub side: Side,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub is_win: bool,
}

/// Performance metrics from a backtest run.
#[derive(Debug, Default)]
pub struct BacktestMetrics {
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub expectancy: f64,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub final_balance: f64,
    pub return_pct: f64,
}

impl BacktestMetrics {
    /// Calculate metrics from trades and equity curve.
    pub fn calculate(
        trades: &[BacktestTrade],
        starting_balance: f64,
        equity_curve: &[(DateTime<Utc>, f64)],
    ) -> Self {
        if trades.is_empty() {
            return Self {
                final_balance: starting_balance,
                ..Default::default()
            };
        }

        let wins: Vec<&BacktestTrade> = trades.iter().filter(|t| t.is_win).collect();
        let losses: Vec<&BacktestTrade> = trades.iter().filter(|t| !t.is_win).collect();

        let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
        let win_rate = wins.len() as f64 / trades.len() as f64;

        let avg_win = if wins.is_empty() {
            0.0
        } else {
            wins.iter().map(|t| t.pnl).sum::<f64>() / wins.len() as f64
        };

        let avg_loss = if losses.is_empty() {
            0.0
        } else {
            losses.iter().map(|t| t.pnl.abs()).sum::<f64>() / losses.len() as f64
        };

        let profit_factor = if avg_loss == 0.0 {
            if avg_win > 0.0 {
                f64::INFINITY
            } else {
                0.0
            }
        } else {
            (avg_win * wins.len() as f64) / (avg_loss * losses.len() as f64)
        };

        let expectancy = total_pnl / trades.len() as f64;

        // Max drawdown from equity curve
        let mut peak = starting_balance;
        let mut max_dd = 0.0_f64;
        for &(_, equity) in equity_curve {
            if equity > peak {
                peak = equity;
            }
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        // Sharpe ratio (simplified — annualized from daily returns)
        let returns: Vec<f64> = equity_curve
            .windows(2)
            .map(|w| (w[1].1 - w[0].1) / w[0].1)
            .filter(|r| r.is_finite())
            .collect();

        let sharpe = if returns.len() > 1 {
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance =
                returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
            let std_dev = variance.sqrt();
            if std_dev > 0.0 {
                (mean / std_dev) * (252.0_f64).sqrt() // Annualized
            } else {
                0.0
            }
        } else {
            0.0
        };

        let final_balance = starting_balance + total_pnl;
        let return_pct = total_pnl / starting_balance * 100.0;

        Self {
            total_trades: trades.len(),
            wins: wins.len(),
            losses: losses.len(),
            win_rate,
            total_pnl,
            avg_win,
            avg_loss,
            profit_factor,
            expectancy,
            max_drawdown_pct: max_dd,
            sharpe_ratio: sharpe,
            final_balance,
            return_pct,
        }
    }
}

impl std::fmt::Display for BacktestMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Trades: {} | WR: {:.1}% | PnL: ${:.2} ({:.1}%) | PF: {:.2} | Sharpe: {:.2} | MaxDD: {:.1}%",
            self.total_trades,
            self.win_rate * 100.0,
            self.total_pnl,
            self.return_pct,
            self.profit_factor,
            self.sharpe_ratio,
            self.max_drawdown_pct * 100.0
        )
    }
}
