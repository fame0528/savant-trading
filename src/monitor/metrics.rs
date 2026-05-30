use crate::core::types::TradeRecord;

pub struct PerformanceMetrics;

impl PerformanceMetrics {
    pub fn calculate(trades: &[TradeRecord]) -> Metrics {
        if trades.is_empty() {
            return Metrics::default();
        }

        let wins: Vec<&TradeRecord> = trades.iter().filter(|t| t.pnl > 0.0).collect();
        let losses: Vec<&TradeRecord> = trades.iter().filter(|t| t.pnl <= 0.0).collect();

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

        let expectancy = if trades.is_empty() {
            0.0
        } else {
            total_pnl / trades.len() as f64
        };

        let mut peak = 0.0_f64;
        let mut max_drawdown = 0.0_f64;
        let mut running = 0.0_f64;
        for trade in trades {
            running += trade.pnl;
            if running > peak {
                peak = running;
            }
            let dd = (peak - running) / peak.max(1.0);
            if dd > max_drawdown {
                max_drawdown = dd;
            }
        }

        Metrics {
            total_trades: trades.len(),
            wins: wins.len(),
            losses: losses.len(),
            win_rate,
            total_pnl,
            avg_win,
            avg_loss,
            profit_factor,
            expectancy,
            max_drawdown,
        }
    }
}

#[derive(Debug, Default)]
pub struct Metrics {
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    pub expectancy: f64,
    pub max_drawdown: f64,
}

impl std::fmt::Display for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Trades: {} | Win Rate: {:.1}% | PnL: ${:.2} | PF: {:.2} | Max DD: {:.1}%",
            self.total_trades,
            self.win_rate * 100.0,
            self.total_pnl,
            self.profit_factor,
            self.max_drawdown * 100.0
        )
    }
}
