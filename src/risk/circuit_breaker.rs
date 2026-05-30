use tracing::warn;

use crate::core::types::AccountState;

pub struct CircuitBreaker {
    max_daily_loss_pct: f64,
    max_drawdown_pct: f64,
    max_positions: usize,
}

impl CircuitBreaker {
    pub fn new(max_daily_loss_pct: f64, max_drawdown_pct: f64, max_positions: usize) -> Self {
        Self {
            max_daily_loss_pct,
            max_drawdown_pct,
            max_positions,
        }
    }

    pub fn check(&self, account: &AccountState) -> CircuitBreakerResult {
        let daily_loss_pct = if account.equity > 0.0 {
            -account.daily_pnl / account.equity
        } else {
            0.0
        };

        if daily_loss_pct >= self.max_daily_loss_pct {
            warn!(
                "Circuit breaker: daily loss {:.2}% exceeds limit {:.2}%",
                daily_loss_pct * 100.0,
                self.max_daily_loss_pct * 100.0
            );
            return CircuitBreakerResult::Triggered(format!(
                "Daily loss limit reached: {:.2}%",
                daily_loss_pct * 100.0
            ));
        }

        if account.drawdown_pct >= self.max_drawdown_pct {
            warn!(
                "Circuit breaker: drawdown {:.2}% exceeds limit {:.2}%",
                account.drawdown_pct * 100.0,
                self.max_drawdown_pct * 100.0
            );
            return CircuitBreakerResult::Triggered(format!(
                "Max drawdown reached: {:.2}%",
                account.drawdown_pct * 100.0
            ));
        }

        if account.open_positions >= self.max_positions {
            return CircuitBreakerResult::Triggered(format!(
                "Max positions reached: {}",
                self.max_positions
            ));
        }

        CircuitBreakerResult::Ok
    }
}

pub enum CircuitBreakerResult {
    Ok,
    Triggered(String),
}

impl CircuitBreakerResult {
    pub fn is_triggered(&self) -> bool {
        matches!(self, Self::Triggered(_))
    }
}
