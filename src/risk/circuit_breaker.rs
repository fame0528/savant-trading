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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account(
        balance: f64,
        equity: f64,
        daily_pnl: f64,
        dd_pct: f64,
        positions: usize,
    ) -> AccountState {
        AccountState {
            balance,
            equity,
            unrealized_pnl: equity - balance,
            daily_pnl,
            peak_equity: equity / (1.0 - dd_pct),
            drawdown_pct: dd_pct,
            open_positions: positions,
            trades_today: 0,
        }
    }

    #[test]
    fn circuit_breaker_ok() {
        let cb = CircuitBreaker::new(0.03, 0.10, 3);
        let account = make_account(100.0, 100.0, -1.0, 0.01, 1);
        assert!(!cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_daily_loss() {
        let cb = CircuitBreaker::new(0.03, 0.10, 3);
        let account = make_account(100.0, 100.0, -5.0, 0.01, 1);
        assert!(cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_drawdown() {
        let cb = CircuitBreaker::new(0.03, 0.10, 3);
        let account = make_account(90.0, 90.0, -1.0, 0.15, 1);
        assert!(cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_max_positions() {
        let cb = CircuitBreaker::new(0.03, 0.10, 3);
        let account = make_account(100.0, 100.0, 0.0, 0.0, 3);
        assert!(cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_at_limit() {
        let cb = CircuitBreaker::new(0.03, 0.10, 3);
        let account = make_account(100.0, 100.0, -3.0, 0.0, 2);
        assert!(cb.check(&account).is_triggered());
    }
}
