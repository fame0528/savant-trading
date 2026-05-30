//! Decision parser — extracts structured TradeDecision from LLM JSON responses.

use serde::{Deserialize, Serialize};

use crate::core::types::Side;

/// The action the AI wants to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
    Close,
    AdjustStop,
}

/// A structured trade decision from the AI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDecision {
    pub action: TradeAction,
    pub pair: String,
    pub side: Side,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    pub take_profit_3: f64,
    pub position_size_pct: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub knowledge_sources: Vec<String>,
    pub risk_reward: f64,
}

/// Errors from decision parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid value: {0}")]
    InvalidValue(String),
    #[error("Hallucinated price: {field}={value} outside market range [{min}, {max}]")]
    HallucinatedPrice {
        field: String,
        value: f64,
        min: f64,
        max: f64,
    },
}

/// Parse a JSON string into a TradeDecision.
///
/// Validates that prices are within reasonable bounds.
pub fn parse_decision(
    response: &str,
    current_price: f64,
    price_tolerance_pct: f64,
) -> Result<TradeDecision, ParseError> {
    // Extract JSON from response (may be wrapped in markdown code blocks)
    let json_str = extract_json(response);

    let decision: TradeDecision = serde_json::from_str(&json_str)?;

    // Validate required fields
    if decision.pair.is_empty() {
        return Err(ParseError::MissingField("pair".to_string()));
    }

    // Validate prices are within tolerance of current price
    let tolerance = current_price * price_tolerance_pct / 100.0;
    let min_price = current_price - tolerance;
    let max_price = current_price + tolerance;

    if decision.entry_price <= 0.0 {
        return Err(ParseError::MissingField("entry_price".to_string()));
    }

    // Only validate prices for non-Hold decisions
    if decision.action != TradeAction::Hold {
        validate_price(
            "entry_price",
            decision.entry_price,
            min_price,
            max_price,
        )?;
        validate_price("stop_loss", decision.stop_loss, min_price, max_price)?;
        validate_price(
            "take_profit_1",
            decision.take_profit_1,
            min_price,
            max_price,
        )?;
    }

    // Validate confidence range
    if decision.confidence < 0.0 || decision.confidence > 1.0 {
        return Err(ParseError::InvalidValue(format!(
            "confidence must be 0.0-1.0, got {}",
            decision.confidence
        )));
    }

    // Validate position size
    if decision.position_size_pct < 0.0 || decision.position_size_pct > 100.0 {
        return Err(ParseError::InvalidValue(format!(
            "position_size_pct must be 0-100, got {}",
            decision.position_size_pct
        )));
    }

    Ok(decision)
}

/// Extract JSON from a response that may contain markdown code blocks.
fn extract_json(response: &str) -> &str {
    let trimmed = response.trim();

    // Check for ```json ... ``` wrapper
    if trimmed.starts_with("```json") {
        let start = trimmed.find('\n').unwrap_or(7);
        let end = trimmed.rfind("```").unwrap_or(trimmed.len());
        return trimmed[start..end].trim();
    }

    // Check for ``` ... ``` wrapper
    if trimmed.starts_with("```") {
        let start = trimmed.find('\n').unwrap_or(3);
        let end = trimmed.rfind("```").unwrap_or(trimmed.len());
        return trimmed[start..end].trim();
    }

    // Try to find JSON object boundaries
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return &trimmed[start..=end];
        }
    }

    trimmed
}

/// Validate a price is within the acceptable range.
fn validate_price(
    field: &str,
    value: f64,
    min: f64,
    max: f64,
) -> Result<(), ParseError> {
    if value < min || value > max {
        Err(ParseError::HallucinatedPrice {
            field: field.to_string(),
            value,
            min,
            max,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_decision() {
        let json = r#"{
            "action": "Buy",
            "pair": "BTC/USD",
            "side": "Long",
            "entry_price": 65000.0,
            "stop_loss": 64000.0,
            "take_profit_1": 66500.0,
            "take_profit_2": 68000.0,
            "take_profit_3": 69500.0,
            "position_size_pct": 50.0,
            "confidence": 0.85,
            "reasoning": "Strong setup",
            "knowledge_sources": ["fabio-amt-001"],
            "risk_reward": 2.5
        }"#;

        let decision = parse_decision(json, 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Buy);
        assert_eq!(decision.pair, "BTC/USD");
        assert_eq!(decision.side, Side::Long);
    }

    #[test]
    fn parse_wrapped_json() {
        let response = r#"Here is my decision:
```json
{
    "action": "Hold",
    "pair": "BTC/USD",
    "side": "Long",
    "entry_price": 0.0,
    "stop_loss": 0.0,
    "take_profit_1": 0.0,
    "take_profit_2": 0.0,
    "take_profit_3": 0.0,
    "position_size_pct": 0.0,
    "confidence": 0.5,
    "reasoning": "No clear setup",
    "knowledge_sources": [],
    "risk_reward": 0.0
}
```"#;

        let decision = parse_decision(response, 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Hold);
    }

    #[test]
    fn reject_hallucinated_price() {
        let json = r#"{
            "action": "Buy",
            "pair": "BTC/USD",
            "side": "Long",
            "entry_price": 999999.0,
            "stop_loss": 64000.0,
            "take_profit_1": 66500.0,
            "take_profit_2": 68000.0,
            "take_profit_3": 69500.0,
            "position_size_pct": 50.0,
            "confidence": 0.85,
            "reasoning": "Test",
            "knowledge_sources": [],
            "risk_reward": 2.5
        }"#;

        let result = parse_decision(json, 65000.0, 10.0);
        assert!(result.is_err());
    }

    #[test]
    fn reject_invalid_confidence() {
        let json = r#"{
            "action": "Buy",
            "pair": "BTC/USD",
            "side": "Long",
            "entry_price": 65000.0,
            "stop_loss": 64000.0,
            "take_profit_1": 66500.0,
            "take_profit_2": 68000.0,
            "take_profit_3": 69500.0,
            "position_size_pct": 50.0,
            "confidence": 1.5,
            "reasoning": "Test",
            "knowledge_sources": [],
            "risk_reward": 2.5
        }"#;

        let result = parse_decision(json, 65000.0, 10.0);
        assert!(result.is_err());
    }
}
