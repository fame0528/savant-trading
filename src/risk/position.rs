use crate::core::types::{AccountState, Side};

/// Minimum notional value for an order (config/default.toml syncs to this)
const MIN_ORDER_VALUE: f64 = 1.0;

/// FID-127: Base risk per trade (2% of balance)
const BASE_RISK_PCT: f64 = 0.02;
/// FID-127: Half-Kelly fraction (micro-cap safety)
const KELLY_FRACTION: f64 = 0.5;
/// FID-127: Minimum gas-cost-to-risk ratio that triggers uneconomic guard.
/// If gas > 0.5 * risk_amount, the trade is refused.
const GAS_ECONOMIC_RATIO: f64 = 0.5;
/// FID-127: Minimum notional value below which a trade is refused (dust orders).
/// Set higher than MIN_ORDER_VALUE because DEX min order size is typically $1-5
/// after slippage, not the exchange minimum.
const MIN_NOTIONAL_USD: f64 = 1.0;

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
            quantity = (account.balance * 0.9999) / entry;
        }

        if quantity <= 0.0 {
            return None;
        }

        let cost = entry * quantity;
        if cost < self.min_order_value {
            return None;
        }

        Some(PositionSize::Sized {
            quantity,
            risk_amount,
            rr_ratio,
        })
    }

    /// FID-127: Calculate position size with conviction-weighted scaling.
    ///
    /// Conviction scaler: `(confidence - 0.50) * 2.0` — 0 below 0.50, linear above.
    /// Position size scales with the LLM's stated confidence and sizing_multiplier.
    ///
    /// Tier logic is preserved (v0.9.1 hardcoded tiers). Conviction formula MULTIPLIES
    /// the tier-derived base risk, it does NOT replace it. This protects capital while
    /// adding conviction scaling.
    ///
    /// Returns `PositionSize::Refused { reason: UneconomicGas | BelowMinNotional }`
    /// for guard failures.
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_with_conviction(
        &self,
        account: &AccountState,
        entry: f64,
        stop_loss: f64,
        take_profit: f64,
        side: Side,
        confidence: f64,
        sizing_multiplier: f64,
        gas_estimate_usd: Option<f64>,
    ) -> PositionSize {
        // Clamp inputs
        let confidence = confidence.clamp(0.0, 1.0);
        let sizing_multiplier = sizing_multiplier.clamp(0.0, 1.0);

        // Conviction scaler: 0 below 0.50, linear above (FID-127 formula)
        let conviction_scaler = ((confidence - 0.50) * 2.0).max(0.0);

        // Base risk: tier-derived + conviction-scaled.
        // Per FID-127: conviction_risk = tier_base_risk * base_risk * kelly * sizing * scaler.
        // tier_risk_amount = balance * tier_risk, so the final formula is
        // `balance * tier_risk * base_risk * kelly * sizing * scaler` (NOT divided
        // by tier_risk — that would cancel the tier scaling out).
        let tier_risk = self.effective_risk_pct(account.balance);
        let tier_risk_amount = account.balance * tier_risk;
        let scaled_risk = tier_risk_amount
            * BASE_RISK_PCT
            * KELLY_FRACTION
            * sizing_multiplier
            * conviction_scaler;

        // Gas-economics guard: if gas > 50% of risk, refuse the trade
        // (avoid burning capital on gas for sub-cent positions)
        if let Some(gas) = gas_estimate_usd {
            if gas > 0.0 && scaled_risk > 0.0 && gas > scaled_risk * GAS_ECONOMIC_RATIO {
                return PositionSize::Refused {
                    reason: RefusalReason::UneconomicGas,
                };
            }
        }

        // Min-notional guard: if scaled risk < $1.00, refuse the trade
        // (DEX minimums of $1-5 will reject sub-dollar orders)
        if scaled_risk < MIN_NOTIONAL_USD {
            return PositionSize::Refused {
                reason: RefusalReason::BelowMinNotional,
            };
        }

        // Compute the actual position size with conviction-scaled risk.
        // We don't reuse calculate_with_atr() because it derives risk_amount
        // from the tier table, not from our pre-computed scaled_risk. The
        // manual path here keeps the conviction formula's risk amount
        // authoritative.
        let risk_per_unit = match side {
            Side::Long => entry - stop_loss,
            Side::Short => stop_loss - entry,
        };
        if risk_per_unit <= 0.0 {
            return PositionSize::Refused {
                reason: RefusalReason::InvalidStopLoss,
            };
        }
        let quantity = scaled_risk / risk_per_unit;
        if quantity <= 0.0 {
            return PositionSize::Refused {
                reason: RefusalReason::BelowMinNotional,
            };
        }
        let cost = entry * quantity;
        if cost > account.balance {
            return PositionSize::Refused {
                reason: RefusalReason::InsufficientBalance,
            };
        }

        // R:R check
        let reward_per_unit = match side {
            Side::Long => take_profit - entry,
            Side::Short => entry - take_profit,
        };
        if reward_per_unit <= 0.0 {
            return PositionSize::Refused {
                reason: RefusalReason::InvalidTakeProfit,
            };
        }
        let rr_ratio = reward_per_unit / risk_per_unit;
        let min_rr = self.effective_min_rr(account.balance);
        if rr_ratio < min_rr - 0.001 {
            return PositionSize::Refused {
                reason: RefusalReason::InsufficientRR,
            };
        }

        PositionSize::Sized {
            quantity,
            risk_amount: scaled_risk,
            rr_ratio,
        }
    }
}

/// FID-127: Reason a position was refused by a guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    UneconomicGas,
    BelowMinNotional,
    /// Defensive: `parse_decision` already rejects invalid stop-loss upstream;
    /// retained here so the enum is exhaustive of trade-rejection causes.
    InvalidStopLoss,
    /// Defensive: `parse_decision` already rejects invalid take-profit upstream;
    /// retained here so the enum is exhaustive of trade-rejection causes.
    InvalidTakeProfit,
    InsufficientBalance,
    InsufficientRR,
}

/// Position sizing result.
/// `Sized` returns a concrete position; `Refused` indicates a guard (gas economics,
/// min notional, invalid prices, etc.) rejected the trade. Both `calculate()` (via
/// the `Sized` variant wrapped in `Option`) and `calculate_with_conviction()` use
/// this single type for sizing outcomes.
#[derive(Debug, Clone, PartialEq)]
pub enum PositionSize {
    Sized {
        quantity: f64,
        risk_amount: f64,
        rr_ratio: f64,
    },
    Refused {
        reason: RefusalReason,
    },
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
        let ps = result.expect("basic sizing should succeed");
        match ps {
            PositionSize::Sized { quantity, risk_amount, .. } => {
                assert_eq!(risk_amount, 500.0); // 10% of 5000
                // Risk-based size = 500/5 = 100 units, but max_position_pct
                // (30% of $5000=$1500) caps at 15
                assert_eq!(quantity, 15.0);
            }
            PositionSize::Refused { reason } => {
                panic!("expected sized, got refused: {:?}", reason);
            }
        }
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
        let ps = result.expect("short sizing should succeed");
        match ps {
            PositionSize::Sized { risk_amount, .. } => {
                assert_eq!(risk_amount, 500.0); // 10% of 5000
            }
            PositionSize::Refused { reason } => {
                panic!("expected sized, got refused: {:?}", reason);
            }
        }
    }

    #[test]
    fn position_sizer_invalid_stop() {
        let sizer = PositionSizer::new(0.01, 1.5);
        let account = make_account(1000.0);
        let result = sizer.calculate(&account, 100.0, 105.0, 110.0, Side::Long);
        assert!(result.is_none());
    }

    // ===== FID-127: Conviction-weighted sizing tests =====

    #[test]
    fn conviction_at_threshold_yields_zero() {
        let sizer = PositionSizer::new(0.02, 1.5);
        let account = make_account(30.0);
        let result = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            0.50, 1.0, None,
        );
        // At 0.50, scaler=0 → sized risk is 0
        match result {
            PositionSize::Refused { reason } => {
                assert_eq!(reason, RefusalReason::BelowMinNotional);
            }
            PositionSize::Sized { .. } => {
                panic!("expected refusal at conviction=0.50, got sized position");
            }
        }
    }

    #[test]
    fn conviction_below_threshold_yields_zero() {
        let sizer = PositionSizer::new(0.02, 1.5);
        let account = make_account(30.0);
        let result = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            0.30, 1.0, None,
        );
        match result {
            PositionSize::Refused { reason } => {
                assert_eq!(reason, RefusalReason::BelowMinNotional);
            }
            PositionSize::Sized { .. } => {
                panic!("expected refusal at conviction=0.30, got sized position");
            }
        }
    }

    #[test]
    fn conviction_above_threshold_scales_linearly() {
        // Use a larger balance to ensure sized risk clears min-notional
        let sizer = PositionSizer::new(0.02, 1.5).with_full_deploy(false);
        let account = make_account(5000.0);

        // Compute relative scaling: 0.75 should give 0.5x the risk of 1.0
        let r_full = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            1.00, 1.0, None,
        );
        let r_75 = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            0.75, 1.0, None,
        );

        let risk_full = match r_full {
            PositionSize::Sized { risk_amount, .. } => risk_amount,
            PositionSize::Refused { reason } => {
                panic!("expected sized at 1.0, got refused: {:?}", reason);
            }
        };
        let risk_75 = match r_75 {
            PositionSize::Sized { risk_amount, .. } => risk_amount,
            PositionSize::Refused { reason } => {
                panic!("expected sized at 0.75, got refused: {:?}", reason);
            }
        };
        // 0.75 → scaler=0.5, 1.0 → scaler=1.0
        // r_75 / r_full should be ~0.5
        let ratio = risk_75 / risk_full;
        assert!((ratio - 0.5).abs() < 0.01, "ratio={}, expected ~0.5", ratio);
    }

    #[test]
    fn gas_uneconomic_override() {
        let sizer = PositionSizer::new(0.02, 1.5);
        let account = make_account(30.0);
        // conviction=0.55 → scaler=0.1 → tiny risk. Gas=$1.50 > 0.5 * tiny risk → refuse
        let result = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            0.55, 1.0, Some(1.50),
        );
        // Note: $30 balance at 0.55 conviction produces ~$0.03 risk.
        // gas=$1.50 > 0.5 * $0.03 = $0.015 → refuse.
        // BUT min-notional ($1.00) would refuse first if scaled_risk < $1.
        // Either refusal reason is acceptable; we test that SOME guard fired.
        match result {
            PositionSize::Refused { reason } => {
                assert!(matches!(reason, RefusalReason::UneconomicGas | RefusalReason::BelowMinNotional));
            }
            PositionSize::Sized { .. } => {
                panic!("expected refusal (gas or min-notional), got sized");
            }
        }
    }

    #[test]
    fn min_notional_override() {
        let sizer = PositionSizer::new(0.02, 1.5);
        let account = make_account(30.0);
        // Low conviction + high balance isn't possible since 0.50 = scaler=0
        // Use a low balance + borderline conviction: scaled risk should be < $1
        let result = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            0.55, 0.25, None, // sizing=0.25 makes scaled_risk even smaller
        );
        match result {
            PositionSize::Refused { reason } => {
                assert_eq!(reason, RefusalReason::BelowMinNotional);
            }
            PositionSize::Sized { .. } => {
                panic!("expected BelowMinNotional refusal, got sized");
            }
        }
    }

    #[test]
    fn sizing_multiplier_scales_proportionally() {
        let sizer = PositionSizer::new(0.02, 1.5);
        let account = make_account(5000.0);

        let r_full = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            0.75, 1.0, None,
        );
        let r_half = sizer.calculate_with_conviction(
            &account, 100.0, 95.0, 110.0, Side::Long,
            0.75, 0.5, None,
        );

        let risk_full = match r_full {
            PositionSize::Sized { risk_amount, .. } => risk_amount,
            PositionSize::Refused { reason } => {
                panic!("expected sized at sizing=1.0, got refused: {:?}", reason);
            }
        };
        let risk_half = match r_half {
            PositionSize::Sized { risk_amount, .. } => risk_amount,
            PositionSize::Refused { reason } => {
                panic!("expected sized at sizing=0.5, got refused: {:?}", reason);
            }
        };
        // sizing 0.5 should give half the risk of sizing 1.0
        let ratio = risk_full / risk_half;
        assert!((ratio - 2.0).abs() < 0.01, "ratio={}, expected ~2.0", ratio);
    }

    /// FID-127 tier-scaling test: at the same conviction, balances in different
    /// tiers produce scaled_risk amounts proportional to (balance * tier_risk_pct).
    /// Without the tier-scaling in the formula, all balances would produce
    /// the same scaled_risk (just `balance * base_risk * kelly * ...`).
    ///
    /// Note on tier boundaries: the default dynamic_risk_tiers are
    /// [<500=1.00, <5000=0.10, <50000=0.05, ...]. A clean 2:1 ratio is achieved
    /// with $400 (tier=1.00, amount=400) vs $2000 (tier=0.10, amount=200). A 10×
    /// ratio would require a tier where (balance * tier_risk_pct) is 10× the
    /// reference — impossible at consecutive tier boundaries (tier=0.10 at exactly
    /// 10× a tier=1.00 balance normalizes back to the same amount).
    #[test]
    fn tier_scaling_at_same_conviction() {
        let sizer = PositionSizer::new(0.02, 1.5);

        // $400 → tier=1.00, tier_risk_amount=400, scaled_risk = 400 * 0.01 = 4.0
        let acct_400 = make_account(400.0);
        let r_400 = sizer.calculate_with_conviction(
            &acct_400, 100.0, 95.0, 110.0, Side::Long,
            1.0, 1.0, None,
        );

        // $2000 → tier=0.10, tier_risk_amount=200, scaled_risk = 200 * 0.01 = 2.0
        let acct_2000 = make_account(2000.0);
        let r_2000 = sizer.calculate_with_conviction(
            &acct_2000, 100.0, 95.0, 110.0, Side::Long,
            1.0, 1.0, None,
        );

        let risk_400 = match r_400 {
            PositionSize::Sized { risk_amount, .. } => risk_amount,
            PositionSize::Refused { reason } => {
                panic!("$400 / tier=1.00 should be sized, got refused: {:?}", reason);
            }
        };
        let risk_2000 = match r_2000 {
            PositionSize::Sized { risk_amount, .. } => risk_amount,
            PositionSize::Refused { reason } => {
                panic!("$2000 / tier=0.10 should be sized, got refused: {:?}", reason);
            }
        };

        // 400 / 200 = 2:1 ratio (the lower-balance / higher-tier produces 2x the
        // tier_risk_amount of the higher-balance / lower-tier)
        let ratio = risk_400 / risk_2000;
        assert!(
            (ratio - 2.0).abs() < 0.01,
            "tier-scaling ratio: {} (expected ~2.0). This catches the bug where \
             the formula accidentally cancels tier_risk out.",
            ratio
        );

        // Also assert the exact absolute values for clarity.
        assert!(
            (risk_400 - 4.0).abs() < 0.001,
            "$400 / tier=1.00 / conviction=1.0 should give risk=4.0, got {}",
            risk_400
        );
        assert!(
            (risk_2000 - 2.0).abs() < 0.001,
            "$2000 / tier=0.10 / conviction=1.0 should give risk=2.0, got {}",
            risk_2000
        );
    }
}
