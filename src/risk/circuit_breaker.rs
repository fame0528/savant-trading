use tracing::warn;

use crate::core::types::AccountState;
use crate::risk::correlation::CorrelationMatrix;

pub struct CircuitBreaker {
    max_daily_loss_pct: f64,
    max_drawdown_pct: f64,
    max_positions: usize,
    max_portfolio_heat: f64,
    max_spread_bps: f64,
    /// Dollar floor for daily loss — don't halt if loss is below this amount.
    /// Prevents circuit breaker from firing on $0.50 loss at $10 balance.
    daily_loss_floor_usd: f64,
    /// Dollar floor for drawdown — don't halt if drawdown is below this amount.
    drawdown_floor_usd: f64,
}

impl CircuitBreaker {
    pub fn new(max_daily_loss_pct: f64, max_drawdown_pct: f64, max_positions: usize) -> Self {
        Self {
            max_daily_loss_pct,
            max_drawdown_pct,
            max_positions,
            max_portfolio_heat: 0.40,
            max_spread_bps: 50.0,
            daily_loss_floor_usd: 5.0,
            drawdown_floor_usd: 10.0,
        }
    }

    pub fn with_daily_loss_floor(mut self, floor: f64) -> Self {
        self.daily_loss_floor_usd = floor;
        self
    }

    pub fn with_drawdown_floor(mut self, floor: f64) -> Self {
        self.drawdown_floor_usd = floor;
        self
    }

    pub fn check(&self, account: &AccountState) -> CircuitBreakerResult {
        self.check_full(account, 0.0, None, 0.0)
    }

    pub fn check_with_heat(
        &self,
        account: &AccountState,
        total_risk_amount: f64,
        correlation: Option<&CorrelationMatrix>,
    ) -> CircuitBreakerResult {
        self.check_full(account, total_risk_amount, correlation, 0.0)
    }

    /// Full circuit breaker check including spread width.
    ///
    /// `spread_bps` is the current bid-ask spread in basis points.
    /// If 0.0, spread check is skipped (no book data available).
    pub fn check_full(
        &self,
        account: &AccountState,
        total_risk_amount: f64,
        correlation: Option<&CorrelationMatrix>,
        spread_bps: f64,
    ) -> CircuitBreakerResult {
        let _ = correlation; // Used for effective position counting (future)
        let daily_loss_pct = if account.equity > 0.0 {
            -account.daily_pnl / account.equity
        } else {
            0.0
        };

        // Dynamic daily loss check: respect both % and $ floor.
        // At $25 balance with 5% limit = $1.25. Floor of $5 means we don't
        // halt until loss exceeds $5 (20% of $25). This allows trading.
        let daily_loss_dollars = -account.daily_pnl;
        if daily_loss_pct >= self.max_daily_loss_pct
            && daily_loss_dollars >= self.daily_loss_floor_usd
        {
            warn!(
                "Circuit breaker: daily loss ${:.2} ({:.2}%) exceeds ${:.2} floor and {:.2}% limit",
                daily_loss_dollars,
                daily_loss_pct * 100.0,
                self.daily_loss_floor_usd,
                self.max_daily_loss_pct * 100.0
            );
            tracing::info!(
                "CIRCUIT_BREAKER_TRIPPED daily_loss=${:.2} ({:.2}%) floor=${:.2} max_pct={:.2}%",
                daily_loss_dollars, daily_loss_pct * 100.0, self.daily_loss_floor_usd, self.max_daily_loss_pct * 100.0
            );
            return CircuitBreakerResult::Triggered(format!(
                "Daily loss limit: ${:.2} ({:.2}%)",
                daily_loss_dollars,
                daily_loss_pct * 100.0
            ));
        }

        // Dynamic drawdown check: same pattern.
        let drawdown_dollars = if account.peak_equity > 0.0 {
            account.peak_equity - account.equity
        } else {
            0.0
        };
        if account.drawdown_pct >= self.max_drawdown_pct
            && drawdown_dollars >= self.drawdown_floor_usd
        {
            warn!(
                "Circuit breaker: drawdown ${:.2} ({:.2}%) exceeds ${:.2} floor and {:.2}% limit",
                drawdown_dollars,
                account.drawdown_pct * 100.0,
                self.drawdown_floor_usd,
                self.max_drawdown_pct * 100.0
            );
            return CircuitBreakerResult::Triggered(format!(
                "Max drawdown: ${:.2} ({:.2}%)",
                drawdown_dollars,
                account.drawdown_pct * 100.0
            ));
        }

        if account.open_positions >= self.max_positions {
            return CircuitBreakerResult::Triggered(format!(
                "Max positions reached: {}",
                self.max_positions
            ));
        }

        // Portfolio heat check — total risk exposure relative to equity
        if account.equity > 0.0 {
            let heat = total_risk_amount / account.equity;
            if heat >= self.max_portfolio_heat {
                warn!(
                    "Circuit breaker: portfolio heat {:.1}% exceeds limit {:.1}%",
                    heat * 100.0,
                    self.max_portfolio_heat * 100.0
                );
                return CircuitBreakerResult::Triggered(format!(
                    "Portfolio heat limit reached: {:.1}% (max {:.1}%)",
                    heat * 100.0,
                    self.max_portfolio_heat * 100.0
                ));
            }
        }

        // Spread width check — halt if market makers have pulled liquidity
        if spread_bps > 0.0 && spread_bps >= self.max_spread_bps {
            warn!(
                "Circuit breaker: spread {:.0}bps exceeds limit {:.0}bps — illiquidity void detected",
                spread_bps, self.max_spread_bps
            );
            return CircuitBreakerResult::Triggered(format!(
                "Spread too wide: {:.0}bps (max {:.0}bps) — market illiquid, halting execution",
                spread_bps, self.max_spread_bps
            ));
        }

        CircuitBreakerResult::Ok
    }

    /// FID-146: Per-trade loss check — fire breaker if a single trade loses > 5% of equity.
    /// Returns Triggered with a descriptive reason. Dollar floor of $1.00 prevents
    /// micro-accounts ($20 equity) from tripping on a $1.10 loss that's only 5.5%.
    pub fn check_per_trade_loss(&self, pnl: f64, equity: f64) -> CircuitBreakerResult {
        // Only check losses (positive PnL = win, no trip)
        if pnl >= 0.0 || equity <= 0.0 {
            return CircuitBreakerResult::Ok;
        }
        let loss_pct = -pnl / equity;
        let loss_dollars = -pnl;
        // 5% loss threshold with $1.00 dollar floor for micro-accounts
        // FID-146: 5% threshold. Floor lowered from $1.00 to $0.50 so it actually trips
        // for micro-accounts (5% of $15 = $0.75; with $1.00 floor, a $0.80 loss wouldn't trip).
        const PER_TRADE_LOSS_PCT: f64 = 0.05;
        const PER_TRADE_LOSS_FLOOR_USD: f64 = 0.50;
        if loss_pct > PER_TRADE_LOSS_PCT && loss_dollars >= PER_TRADE_LOSS_FLOOR_USD {
            warn!(
                "Circuit breaker: per-trade loss ${:.2} ({:.2}%) exceeds 5% limit (equity=${:.2})",
                loss_dollars, loss_pct * 100.0, equity
            );
            return CircuitBreakerResult::Triggered(format!(
                "Per-trade loss: ${:.2} ({:.2}%) exceeds 5% limit (equity=${:.2})",
                loss_dollars,
                loss_pct * 100.0,
                equity
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
            max_positions: 3,
            trades_today: 0,
        }
    }

    #[test]
    fn circuit_breaker_ok() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(100.0, 100.0, -1.0, 0.01, 1);
        assert!(!cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_daily_loss() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(100.0, 100.0, -21.0, 0.01, 1);
        assert!(cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_drawdown() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(59.0, 59.0, -1.0, 0.45, 1);
        assert!(cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_max_positions() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(100.0, 100.0, 0.0, 0.0, 5);
        assert!(cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_at_limit() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(100.0, 100.0, -20.0, 0.0, 2);
        assert!(cb.check(&account).is_triggered());
    }

    #[test]
    fn circuit_breaker_spread_too_wide() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(100.0, 100.0, 0.0, 0.0, 1);
        // Spread 60bps > 50bps limit → trigger
        assert!(cb.check_full(&account, 0.0, None, 60.0).is_triggered());
    }

    #[test]
    fn circuit_breaker_spread_ok() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(100.0, 100.0, 0.0, 0.0, 1);
        // Spread 30bps < 50bps limit → ok
        assert!(!cb.check_full(&account, 0.0, None, 30.0).is_triggered());
    }

    #[test]
    fn circuit_breaker_spread_zero_skips() {
        let cb = CircuitBreaker::new(0.20, 0.40, 5);
        let account = make_account(100.0, 100.0, 0.0, 0.0, 1);
        // Spread 0.0 = no book data → skip check
        assert!(!cb.check_full(&account, 0.0, None, 0.0).is_triggered());
    }
}
