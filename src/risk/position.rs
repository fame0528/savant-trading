use crate::core::types::{AccountState, Side};

/// Minimum notional value for a Kraken order (config/default.toml syncs to this)
const MIN_ORDER_VALUE: f64 = 1.0;

pub struct RiskTier {
    pub balance: f64,
    pub risk_pct: f64,
}

pub struct PositionSizer {
    max_risk_per_trade: f64,
    min_rr_ratio: f64,
    min_order_value: f64,
    dynamic_risk_tiers: Vec<RiskTier>,
    max_position_pct: f64,
    full_deploy: bool,
    min_rr_ratio_low_balance: f64,
    low_balance_threshold: f64,
}

impl PositionSizer {
    pub fn new(max_risk_per_trade: f64, min_rr_ratio: f64) -> Self {
        Self {
            max_risk_per_trade,
            min_rr_ratio,
            min_order_value: MIN_ORDER_VALUE,
            max_position_pct: 0.30,
            full_deploy: false,
            min_rr_ratio_low_balance: 1.2,
            low_balance_threshold: 50.0,
            dynamic_risk_tiers: vec![
                RiskTier {
                    balance: 500.0,
                    risk_pct: 1.00,
                },
                RiskTier {
                    balance: 5000.0,
                    risk_pct: 0.10,
                },
                RiskTier {
                    balance: 50000.0,
                    risk_pct: 0.05,
                },
                RiskTier {
                    balance: 999999.0,
                    risk_pct: 0.02,
                },
            ],
        }
    }

    pub fn with_tiers(mut self, tiers: Vec<RiskTier>) -> Self {
        self.dynamic_risk_tiers = tiers;
        self
    }

    pub fn with_full_deploy(mut self, full_deploy: bool) -> Self {
        self.full_deploy = full_deploy;
        self
    }

    pub fn with_low_balance_rr(mut self, min_rr: f64, threshold: f64) -> Self {
        self.min_rr_ratio_low_balance = min_rr;
        self.low_balance_threshold = threshold;
        self
    }

    /// Get the effective risk % for the current balance.
    /// At small balances, uses higher risk to overcome fee friction.
    pub fn effective_risk_pct(&self, balance: f64) -> f64 {
        // In full_deploy mode at low balance, use 100% of capital
        if self.full_deploy && balance < self.low_balance_threshold {
            return 1.00;
        }
        for tier in &self.dynamic_risk_tiers {
            if balance <= tier.balance {
                return tier.risk_pct;
            }
        }
        self.max_risk_per_trade
    }

    /// Get the effective min R:R for the current balance.
    /// At very low balances, relax slightly to allow first trade.
    fn effective_min_rr(&self, balance: f64) -> f64 {
        if balance < self.low_balance_threshold {
            self.min_rr_ratio_low_balance
        } else {
            self.min_rr_ratio
        }
    }

    pub fn calculate(
        &self,
        account: &AccountState,
        entry: f64,
        stop_loss: f64,
        take_profit: f64,
        side: Side,
    ) -> Option<PositionSize> {
        self.calculate_with_atr(account, entry, stop_loss, take_profit, side, None)
    }

    /// Calculate position size with optional ATR-based risk adjustment (FID-035).
    ///
    /// For meme coins with high ATR (5%+), limits risk to prevent oversized positions.
    /// `atr` is the current ATR value. If provided, risk is capped at `atr * quantity * 0.5`.
    pub fn calculate_with_atr(
        &self,
        account: &AccountState,
        entry: f64,
        stop_loss: f64,
        take_profit: f64,
        side: Side,
        atr: Option<f64>,
    ) -> Option<PositionSize> {
        let risk_pct = self.effective_risk_pct(account.balance);
        let risk_amount = account.balance * risk_pct;
        let min_rr = self.effective_min_rr(account.balance);

        // Dynamic max_position_pct: in full deploy at low balance, use 100%
        let max_pct = if self.full_deploy && account.balance < self.low_balance_threshold {
            1.00
        } else {
            self.max_position_pct
        };

        let risk_per_unit = match side {
            Side::Long => entry - stop_loss,
            Side::Short => stop_loss - entry,
        };

        if risk_per_unit <= 0.0 {
            return None;
        }

        let reward_per_unit = match side {
            Side::Long => take_profit - entry,
            Side::Short => entry - take_profit,
        };

        if reward_per_unit <= 0.0 {
            return None;
        }

        let rr_ratio = reward_per_unit / risk_per_unit;
        tracing::debug!(
            "PositionSizer: entry={:.6} stop={:.6} tp={:.6} risk={:.6} reward={:.6} rr={:.4} min={:.4} balance=${:.2} risk_pct={:.2}%",
            entry, stop_loss, take_profit, risk_per_unit, reward_per_unit, rr_ratio, min_rr,
            account.balance, risk_pct * 100.0
        );
        if rr_ratio < min_rr - 0.001 {
            return None;
        }

        let mut quantity = risk_amount / risk_per_unit;

        // ATR-based risk cap (FID-035): for high-volatility assets,
        // limit position size so risk doesn't exceed ATR * 0.5
        if let Some(atr_val) = atr {
            let atr_risk_cap = atr_val * quantity * 0.5;
            if risk_amount > atr_risk_cap {
                quantity = atr_risk_cap / risk_per_unit;
            }
        }

        let max_qty = (account.balance * max_pct) / entry;
        if quantity > max_qty {
            quantity = max_qty;
        }
        let cost = entry * quantity;
        if cost > account.balance {
            quantity = (account.balance * 0.99) / entry;
        }

        if quantity <= 0.0 {
            return None;
        }

        let cost = entry * quantity;
        if cost < self.min_order_value {
            return None;
        }

        Some(PositionSize {
            quantity,
            risk_amount,
            rr_ratio,
        })
    }
}

pub struct PositionSize {
    pub quantity: f64,
    pub risk_amount: f64,
    pub rr_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account(balance: f64) -> AccountState {
        AccountState::new(balance)
    }

    #[test]
    fn position_sizer_basic() {
        let sizer = PositionSizer::new(0.20, 1.5);
        let account = make_account(5000.0);
        let result = sizer.calculate(&account, 100.0, 95.0, 110.0, Side::Long);
        assert!(result.is_some());
        let ps = result.unwrap();
        assert_eq!(ps.risk_amount, 500.0); // 10% of 5000
                                           // Risk-based size = 500/5 = 100 units, but max_position_pct (30% of $5000=$1500) caps at 15
        assert_eq!(ps.quantity, 15.0);
    }

    #[test]
    fn position_sizer_rr_too_low() {
        let sizer = PositionSizer::new(0.20, 1.5);
        let account = make_account(5000.0);
        let result = sizer.calculate(&account, 100.0, 95.0, 102.0, Side::Long);
        assert!(result.is_none());
    }

    #[test]
    fn position_sizer_short() {
        let sizer = PositionSizer::new(0.20, 1.5);
        let account = make_account(5000.0);
        let result = sizer.calculate(&account, 100.0, 105.0, 90.0, Side::Short);
        assert!(result.is_some());
        let ps = result.unwrap();
        assert_eq!(ps.risk_amount, 500.0); // 10% of 5000
    }

    #[test]
    fn position_sizer_invalid_stop() {
        let sizer = PositionSizer::new(0.01, 1.5);
        let account = make_account(1000.0);
        let result = sizer.calculate(&account, 100.0, 105.0, 110.0, Side::Long);
        assert!(result.is_none());
    }
}
