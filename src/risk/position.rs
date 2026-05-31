use crate::core::types::{AccountState, Side};

pub struct PositionSizer {
    max_risk_per_trade: f64,
    min_rr_ratio: f64,
}

impl PositionSizer {
    pub fn new(max_risk_per_trade: f64, min_rr_ratio: f64) -> Self {
        Self {
            max_risk_per_trade,
            min_rr_ratio,
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
        let risk_amount = account.balance * self.max_risk_per_trade;

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
        if rr_ratio < self.min_rr_ratio {
            return None;
        }

        let quantity = risk_amount / risk_per_unit;

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
        let sizer = PositionSizer::new(0.01, 1.5);
        let account = make_account(1000.0);
        let result = sizer.calculate(&account, 100.0, 95.0, 110.0, Side::Long);
        assert!(result.is_some());
        let ps = result.unwrap();
        assert_eq!(ps.risk_amount, 10.0);
        assert_eq!(ps.quantity, 2.0);
    }

    #[test]
    fn position_sizer_rr_too_low() {
        let sizer = PositionSizer::new(0.01, 1.5);
        let account = make_account(1000.0);
        let result = sizer.calculate(&account, 100.0, 95.0, 102.0, Side::Long);
        assert!(result.is_none());
    }

    #[test]
    fn position_sizer_short() {
        let sizer = PositionSizer::new(0.01, 1.5);
        let account = make_account(1000.0);
        let result = sizer.calculate(&account, 100.0, 105.0, 90.0, Side::Short);
        assert!(result.is_some());
        let ps = result.unwrap();
        assert_eq!(ps.risk_amount, 10.0);
    }

    #[test]
    fn position_sizer_invalid_stop() {
        let sizer = PositionSizer::new(0.01, 1.5);
        let account = make_account(1000.0);
        let result = sizer.calculate(&account, 100.0, 105.0, 110.0, Side::Long);
        assert!(result.is_none());
    }
}
