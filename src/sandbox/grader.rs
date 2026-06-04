//! 3-Tier grading rubric for sandbox evaluation.

use serde::{Deserialize, Serialize};

/// Grade for a single decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grade {
    /// Tier 1: Binary compliance (0 or 1)
    pub tier_1_compliance: bool,
    /// Tier 1 failure reason (if any)
    pub tier_1_reason: Option<String>,
    /// Tier 2: R:R score (0.0 to 1.0)
    pub tier_2_rr_score: f64,
    /// Tier 2 details
    pub tier_2_details: String,
    /// Tier 3: Reasoning score (0.0 to 1.0)
    pub tier_3_reasoning_score: f64,
    /// Tier 3 judge rationale
    pub tier_3_rationale: String,
    /// Total score (weighted average)
    pub total_score: f64,
}

/// Tier 1: Binary compliance check.
/// Returns (pass, reason).
pub fn tier_1_compliance(
    action: &str,
    stop_loss: f64,
    entry_price: f64,
    confidence: f64,
    reasoning: &str,
    expected_action: &str,
) -> (bool, Option<String>) {
    // Check: action is not empty
    if action.is_empty() {
        return (false, Some("Empty action".into()));
    }

    // Check: Hold when scenario demands action is a compliance failure
    let expected_lower = expected_action.to_lowercase();
    if action == "Pass" && (expected_lower.contains("buy") || expected_lower.contains("sell")) {
        return (
            false,
            Some(format!("Missed trade: expected {}", expected_action)),
        );
    }

    // Check: Hold decisions are otherwise compliant (no risk taken)
    if action == "Pass" {
        return (true, None);
    }

    // Check: non-Hold must have stop loss
    if stop_loss <= 0.0 {
        return (false, Some("Missing stop loss".into()));
    }

    // Check: non-Hold must have valid entry price
    if entry_price <= 0.0 {
        return (false, Some("Invalid entry price".into()));
    }

    // Check: confidence must be between 0 and 1
    if !(0.0..=1.0).contains(&confidence) {
        return (false, Some(format!("Invalid confidence: {}", confidence)));
    }

    // Check: reasoning must not be empty
    if reasoning.is_empty() {
        return (false, Some("Empty reasoning".into()));
    }

    // Check: reasoning should be substantive (>20 chars)
    if reasoning.len() < 20 {
        return (false, Some("Reasoning too short (<20 chars)".into()));
    }

    (true, None)
}

/// Tier 2: Quantitative R:R scoring.
/// Returns score 0.0 to 1.0.
pub fn tier_2_rr_score(
    entry_price: f64,
    stop_loss: f64,
    take_profit_1: f64,
    action: &str,
    expected_action: &str,
) -> (f64, String) {
    // Hold decisions get neutral score if no action expected, zero if action was expected
    if action == "Pass" {
        let expected_lower = expected_action.to_lowercase();
        if expected_lower.contains("buy") || expected_lower.contains("sell") {
            return (0.0, "Hold when trade expected — zero R:R".into());
        }
        return (0.5, "Hold decision — neutral R:R score".into());
    }

    if entry_price <= 0.0 || stop_loss <= 0.0 || take_profit_1 <= 0.0 {
        return (0.0, "Invalid price parameters".into());
    }

    let risk = (entry_price - stop_loss).abs();
    let reward = (take_profit_1 - entry_price).abs();

    if risk == 0.0 {
        return (0.0, "Zero risk — invalid stop placement".into());
    }

    let rr = reward / risk;

    let score = if rr >= 3.0 {
        1.0
    } else if rr >= 2.0 {
        0.8
    } else if rr >= 1.5 {
        0.6
    } else if rr >= 1.0 {
        0.3
    } else {
        0.0
    };

    let details = format!("R:R = {:.2} (risk={:.2}, reward={:.2})", rr, risk, reward);
    (score, details)
}

/// Tier 3: Reasoning quality scoring (deterministic heuristic).
/// Returns score 0.0 to 1.0.
pub fn tier_3_reasoning_score(reasoning: &str, expected_action: &str) -> (f64, String) {
    let mut score: f64 = 0.0;
    let mut evidence = Vec::new();

    let lower = reasoning.to_lowercase();

    // Check for regime classification
    if lower.contains("trending")
        || lower.contains("ranging")
        || lower.contains("volatile")
        || lower.contains("bull")
        || lower.contains("bear")
    {
        score += 0.15;
        evidence.push("Regime classified");
    }

    // Check for thesis
    if reasoning.len() > 50 {
        score += 0.15;
        evidence.push("Substantive thesis (>50 chars)");
    }

    // Check for specific price levels
    if reasoning.contains('$')
        || lower.contains("support")
        || lower.contains("resistance")
        || lower.contains("entry")
        || lower.contains("stop")
    {
        score += 0.15;
        evidence.push("Specific price levels cited");
    }

    // Check for risk management
    if lower.contains("risk")
        || lower.contains("stop loss")
        || lower.contains("rr")
        || lower.contains("r:r")
        || lower.contains("reward")
    {
        score += 0.15;
        evidence.push("Risk management considered");
    }

    // Check for session awareness
    if lower.contains("session")
        || lower.contains("asian")
        || lower.contains("london")
        || lower.contains("us session")
        || lower.contains("weekend")
    {
        score += 0.1;
        evidence.push("Session awareness");
    }

    // Check for on-chain/sentiment reference
    if lower.contains("fear")
        || lower.contains("greed")
        || lower.contains("funding")
        || lower.contains("mvrv")
        || lower.contains("sopr")
    {
        score += 0.1;
        evidence.push("Sentiment/on-chain referenced");
    }

    // Check for volume reference
    if lower.contains("volume") || lower.contains("oi") || lower.contains("open interest") {
        score += 0.1;
        evidence.push("Volume/OI referenced");
    }

    // Bonus for Hold decisions that correctly identify unfavorable conditions
    let expected_lower = expected_action.to_lowercase();
    let expects_action = expected_lower.contains("buy") || expected_lower.contains("sell");

    if expected_action == "Pass" && score >= 0.3 {
        score += 0.1;
        evidence.push("Correctly identified unfavorable conditions");
    }

    // Penalty: Hold with generic reasoning when action was expected
    if expects_action && score < 0.4 {
        score = (score * 0.5).max(0.0);
        evidence.push("PENALTY: Generic reasoning when trade expected");
    }

    // Penalty: Reasoning doesn't reference ANY data from the context
    // (indicators, sentiment, on-chain, volume, session)
    let has_data_ref = lower.contains("ema")
        || lower.contains("rsi")
        || lower.contains("atr")
        || lower.contains("adx")
        || lower.contains("vwap")
        || lower.contains("fear")
        || lower.contains("greed")
        || lower.contains("funding")
        || lower.contains("mvrv")
        || lower.contains("volume")
        || lower.contains("session")
        || lower.contains("support")
        || lower.contains("resistance");
    if !has_data_ref && reasoning.len() > 20 {
        score = (score - 0.15).max(0.0);
        evidence.push("PENALTY: No data references in reasoning");
    }

    score = score.min(1.0);

    let rationale = if evidence.is_empty() {
        "No evidence found in reasoning".to_string()
    } else {
        format!("Evidence: {}", evidence.join(", "))
    };

    (score, rationale)
}

/// Calculate total weighted score.
pub fn calculate_total(tier_1: bool, tier_2: f64, tier_3: f64) -> f64 {
    if !tier_1 {
        return 0.0; // Hard fail on compliance
    }
    // Weighted: Tier 1 is pass/fail, Tier 2 = 40%, Tier 3 = 60%
    tier_2 * 0.4 + tier_3 * 0.6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_1_hold_always_passes() {
        let (pass, _) = tier_1_compliance("Pass", 0.0, 0.0, 0.0, "No setup", "Pass");
        assert!(pass);
    }

    #[test]
    fn tier_1_buy_needs_stop() {
        let (pass, reason) = tier_1_compliance("Buy", 0.0, 100.0, 0.7, "Good setup", "Buy");
        assert!(!pass);
        assert!(reason.unwrap().contains("stop loss"));
    }

    #[test]
    fn tier_1_buy_passes() {
        let (pass, _) = tier_1_compliance(
            "Buy",
            95.0,
            100.0,
            0.7,
            "Strong support at 95 with volume",
            "Buy",
        );
        assert!(pass);
    }

    #[test]
    fn tier_2_good_rr() {
        let (score, _) = tier_2_rr_score(100.0, 95.0, 110.0, "Buy", "Buy (High Conviction)");
        assert!(score >= 0.6); // R:R = 2.0
    }

    #[test]
    fn tier_2_bad_rr() {
        let (score, _) = tier_2_rr_score(100.0, 99.0, 100.5, "Buy", "Buy (High Conviction)");
        assert!(score <= 0.3); // R:R = 0.5
    }

    #[test]
    fn tier_2_hold_neutral() {
        let (score, _) = tier_2_rr_score(0.0, 0.0, 0.0, "Pass", "Pass");
        assert_eq!(score, 0.5);
    }

    #[test]
    fn tier_3_rich_reasoning() {
        let reasoning = "BTC trending above EMA21 in US session. Support at $100,000 with volume confirmation. Fear & Greed at 25 = extreme fear = buying zone. Stop at $99,000, R:R 2:1.";
        let (score, _) = tier_3_reasoning_score(reasoning, "Buy");
        assert!(score >= 0.6);
    }

    #[test]
    fn tier_3_poor_reasoning() {
        let (score, _) = tier_3_reasoning_score("buy", "Buy");
        assert!(score <= 0.2);
    }

    #[test]
    fn total_score_compliance_fail() {
        let total = calculate_total(false, 0.8, 0.9);
        assert_eq!(total, 0.0);
    }

    #[test]
    fn total_score_weighted() {
        let total = calculate_total(true, 0.8, 0.9);
        assert!((total - 0.86).abs() < 0.01);
    }
}
