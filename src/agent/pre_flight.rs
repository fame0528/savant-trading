//! FID-194: Pre-flight Guard Against Phantom Management
//!
//! The LLM/jury may emit `AdjustStop` or `Close` for a pair that was never
//! actually opened on-chain (e.g., a spread-rejected BUY from a prior cycle).
//! Without this guard, the engine would try to manage a phantom position.
//!
//! This module exports a single function `apply_pre_flight_guard` that the
//! engine calls after `parse_decision` returns. Per ECHO Law 13
//! (utility-first), this is the ONE function that handles phantom management
//! detection — call sites should not re-implement this logic.

use crate::agent::decision_parser::{TradeAction, TradeDecision};
use crate::core::types::Position;
use crate::execution::engine::ExecutionEngine;

/// FID-194: Pre-flight guard against phantom management decisions.
///
/// If the LLM/jury says `AdjustStop` or `Close` but no position exists for
/// this pair in the executor's state, downgrade to `Pass` with
/// `override_source="no_position_to_manage"`.
///
/// The executor is the source of truth for live mode. In dry mode (no
/// executor), fall back to the in-memory portfolio.
///
/// Returns true if the decision was downgraded, false if it passed through.
///
/// Per ECHO Law 4 (call-graph reachability), this function must be called
/// from the single `parse_decision` call site in `engine/mod.rs:2844`.
pub fn apply_pre_flight_guard(
    decision: &mut TradeDecision,
    executor: Option<&dyn ExecutionEngine>,
    portfolio_positions: &[Position],
) -> bool {
    if !matches!(
        decision.action,
        TradeAction::AdjustStop | TradeAction::Close
    ) {
        return false;
    }

    let has_position = if let Some(ex) = executor {
        ex.open_positions().iter().any(|p| p.pair == decision.pair)
    } else {
        portfolio_positions.iter().any(|p| p.pair == decision.pair)
    };

    if !has_position {
        let action_label = match decision.action {
            TradeAction::AdjustStop => "AdjustStop",
            TradeAction::Close => "Close",
            _ => "unknown",
        };
        tracing::info!(
            "FID-194: {} for {} but no position exists. Downgrading to Pass (override: no_position_to_manage).",
            action_label,
            decision.pair
        );
        decision.action = TradeAction::Pass;
        decision.override_source = Some("no_position_to_manage".to_string());
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::decision_parser::{RegimeLabel, TriggerWeights};
    use crate::core::error::ExecutionError;
    use crate::core::types::{Order, ScaleLevel, Side};
    use async_trait::async_trait;
    use chrono::Utc;

    fn make_decision(action: TradeAction, pair: &str) -> TradeDecision {
        TradeDecision {
            action,
            pair: pair.to_string(),
            side: Side::Long,
            order_type: "market".to_string(),
            entry_price: 0.0953,
            stop_loss: 0.0938,
            take_profit_1: 0.0965,
            take_profit_2: 0.0,
            take_profit_3: 0.0,
            position_size_pct: 50.0,
            confidence: 0.62,
            conviction_score: 0.52,
            risk_reward: 2.0,
            reasoning: "test reasoning".to_string(),
            knowledge_sources: vec![],
            trigger_weights: TriggerWeights {
                strong: 1,
                moderate: 1,
                weak: 0,
            },
            regime_label: RegimeLabel::Trending,
            override_source: None,
            management_trigger_active: false,
            stop_distance_atr_multiple: 0.0,
            thesis_invalidated: false,
            opportunity_cost: String::new(),
            mandated_action: String::new(),
            mandated_stop_price: 0.0,
            would_initiate_new_long: None,
            sizing_multiplier: 1.0,
        }
    }

    fn make_position(pair: &str) -> Position {
        Position {
            id: format!("test-{}", pair),
            pair: pair.to_string(),
            side: Side::Long,
            entry_price: 0.0953,
            current_price: 0.0953,
            quantity: 100.0,
            stop_loss: 0.0938,
            take_profit_1: 0.0965,
            take_profit_2: 0.0,
            take_profit_3: 0.0,
            unrealized_pnl: 0.0,
            risk_amount: 5.0,
            strategy_name: "test".to_string(),
            opened_at: Utc::now(),
            scale_level: ScaleLevel::Full,
            token_address: String::new(),
        }
    }

    struct MockExecutor {
        positions: Vec<Position>,
    }

    #[async_trait]
    impl ExecutionEngine for MockExecutor {
        fn balance(&self) -> f64 {
            100.0
        }
        fn open_positions(&self) -> Vec<&Position> {
            self.positions.iter().collect()
        }
        async fn place_order(
            &mut self,
            _pair: &str,
            _side: Side,
            _quantity: f64,
            _price: Option<f64>,
        ) -> Result<Order, ExecutionError> {
            Ok(Order {
                id: "mock".to_string(),
                pair: _pair.to_string(),
                side: _side,
                order_type: crate::core::types::OrderType::Market,
                price: _price,
                quantity: _quantity,
                status: crate::core::types::OrderStatus::Filled,
                created_at: Utc::now(),
                filled_at: Some(Utc::now()),
                filled_price: _price,
                tx_hash: None,
            })
        }
        async fn close_position(&mut self, _position_id: &str) -> Result<Order, ExecutionError> {
            Ok(Order {
                id: "mock-close".to_string(),
                pair: String::new(),
                side: Side::Long,
                order_type: crate::core::types::OrderType::Market,
                price: None,
                quantity: 0.0,
                status: crate::core::types::OrderStatus::Filled,
                created_at: Utc::now(),
                filled_at: Some(Utc::now()),
                filled_price: None,
                tx_hash: None,
            })
        }
    }

    #[test]
    fn downgrades_adjuststop_for_phantom_position() {
        let mut decision = make_decision(TradeAction::AdjustStop, "ENA/USD");
        let executor = MockExecutor { positions: vec![] };
        let portfolio = vec![];

        let downgraded = apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);

        assert!(downgraded);
        assert_eq!(decision.action, TradeAction::Pass);
        assert_eq!(
            decision.override_source,
            Some("no_position_to_manage".to_string())
        );
    }

    #[test]
    fn downgrades_close_for_phantom_position() {
        let mut decision = make_decision(TradeAction::Close, "ENA/USD");
        let executor = MockExecutor { positions: vec![] };
        let portfolio = vec![];

        let downgraded = apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);

        assert!(downgraded);
        assert_eq!(decision.action, TradeAction::Pass);
    }

    #[test]
    fn keeps_adjuststop_when_executor_has_position() {
        let mut decision = make_decision(TradeAction::AdjustStop, "ENA/USD");
        let ena = make_position("ENA/USD");
        let executor = MockExecutor {
            positions: vec![ena],
        };
        let portfolio = vec![];

        let downgraded = apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);

        assert!(!downgraded);
        assert_eq!(decision.action, TradeAction::AdjustStop);
        assert!(decision.override_source.is_none());
    }

    #[test]
    fn falls_back_to_portfolio_when_no_executor() {
        let mut decision = make_decision(TradeAction::Close, "ENA/USD");
        let portfolio = vec![make_position("ENA/USD")];

        let downgraded = apply_pre_flight_guard(&mut decision, None, &portfolio);

        assert!(!downgraded);
        assert_eq!(decision.action, TradeAction::Close);
    }

    #[test]
    fn downgrades_when_only_portfolio_has_other_position() {
        let mut decision = make_decision(TradeAction::AdjustStop, "BTC/USD");
        let portfolio = vec![make_position("ENA/USD")]; // ENA, not BTC

        let downgraded = apply_pre_flight_guard(&mut decision, None, &portfolio);

        assert!(downgraded);
        assert_eq!(decision.action, TradeAction::Pass);
    }

    #[test]
    fn ignores_buy_action() {
        let mut decision = make_decision(TradeAction::Buy, "ENA/USD");
        let executor = MockExecutor { positions: vec![] };
        let portfolio = vec![];

        let downgraded = apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);

        assert!(!downgraded);
        assert_eq!(decision.action, TradeAction::Buy);
    }

    #[test]
    fn ignores_sell_action() {
        let mut decision = make_decision(TradeAction::Sell, "ENA/USD");
        let executor = MockExecutor { positions: vec![] };
        let portfolio = vec![];

        let downgraded = apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);

        assert!(!downgraded);
        assert_eq!(decision.action, TradeAction::Sell);
    }

    #[test]
    fn ignores_pass_action() {
        let mut decision = make_decision(TradeAction::Pass, "ENA/USD");
        let executor = MockExecutor { positions: vec![] };
        let portfolio = vec![];

        let downgraded = apply_pre_flight_guard(&mut decision, Some(&executor), &portfolio);

        assert!(!downgraded);
        assert_eq!(decision.action, TradeAction::Pass);
    }
}
