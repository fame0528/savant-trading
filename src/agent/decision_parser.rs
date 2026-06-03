//! Decision parser — extracts structured TradeDecision from LLM JSON responses.

use serde::{Deserialize, Serialize};

use crate::core::types::Side;

/// The action the AI wants to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TradeAction {
    #[serde(alias = "BUY", alias = "buy")]
    Buy,
    #[serde(alias = "SELL", alias = "sell")]
    Sell,
    #[serde(alias = "HOLD", alias = "hold")]
    Hold,
    #[serde(alias = "CLOSE", alias = "close")]
    Close,
    #[serde(alias = "ADJUST_STOP", alias = "adjust_stop", alias = "ADJUSTSTOP", alias = "Adjust_Stop")]
    AdjustStop,
}

/// A structured trade decision from the AI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDecision {
    pub action: TradeAction,
    pub pair: String,
    pub side: Side,
    #[serde(default = "default_order_type")]
    pub order_type: String,
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

fn default_order_type() -> String {
    "LIMIT".to_string()
}

impl TradeDecision {
    /// Calculate Expected Value: EV = (p × reward) - ((1-p) × risk)
    /// Where p = confidence, reward = distance to TP1, risk = distance to stop.
    /// Returns Some(ev) for Buy/Sell, None for Hold.
    pub fn expected_value(&self) -> Option<f64> {
        if self.action == TradeAction::Hold {
            return None;
        }

        let p = self.confidence;
        let risk = match self.side {
            Side::Long => (self.entry_price - self.stop_loss).abs(),
            Side::Short => (self.stop_loss - self.entry_price).abs(),
        };
        let reward = match self.side {
            Side::Long => (self.take_profit_1 - self.entry_price).abs(),
            Side::Short => (self.entry_price - self.take_profit_1).abs(),
        };

        if risk <= 0.0 || reward <= 0.0 {
            return None;
        }

        let ev = (p * reward) - ((1.0 - p) * risk);
        Some(ev)
    }

    /// Returns true if the trade has positive expected value.
    pub fn is_positive_ev(&self) -> bool {
        self.expected_value().map(|ev| ev > 0.0).unwrap_or(false)
    }
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
/// Uses multi-pass repair: strict parse → json-repair crate → partial extraction.
pub fn parse_decision(
    response: &str,
    current_price: f64,
    price_tolerance_pct: f64,
) -> Result<TradeDecision, ParseError> {
    // Extract JSON from response (may be wrapped in markdown code blocks)
    let json_str = extract_json(response);

    // Normalize action field — LLMs sometimes return UPPERCASE
    let normalized = normalize_llm_json(json_str);

    // Try strict parse first, then repair, then partial extraction
    let decision = match serde_json::from_str::<TradeDecision>(&normalized) {
        Ok(d) => d,
        Err(strict_err) => {
            // Pass 2: manual repair — fix truncated strings, unclosed brackets
            let repaired = repair_json_string(&normalized);
            match serde_json::from_str::<TradeDecision>(&repaired) {
                Ok(d) => d,
                Err(repair_err) => {
                    // Pass 3: partial extraction — salvage what we can
                    // Try partial on repaired first (may have fixed truncation), then original
                    match partial_extract(&repaired).or_else(|| partial_extract(&normalized)) {
                        Some(d) => d,
                        None => {
                            tracing::debug!(
                                "JSON parse failed after all passes. strict={}, repair={}",
                                strict_err,
                                repair_err
                            );
                            return Err(ParseError::InvalidJson(strict_err));
                        }
                    }
                }
            }
        }
    };

    // Validate required fields
    if decision.pair.is_empty() {
        return Err(ParseError::MissingField("pair".to_string()));
    }

    // Validate prices are within tolerance of current price
    let tolerance = current_price * price_tolerance_pct / 100.0;
    let min_price = current_price - tolerance;
    let max_price = current_price + tolerance;

    // AdjustStop only moves a stop on an existing position — it needs a new
    // stop_loss, not an entry/TP. Validate that separately and skip entry checks.
    if decision.action == TradeAction::AdjustStop && decision.stop_loss <= 0.0 {
        return Err(ParseError::MissingField("stop_loss".to_string()));
    }

    // Only validate entry prices for actionable entry decisions (Buy/Sell/Close).
    if decision.action != TradeAction::Hold && decision.action != TradeAction::AdjustStop {
        if decision.entry_price <= 0.0 {
            return Err(ParseError::MissingField("entry_price".to_string()));
        }
        // Price tolerance only constrains entry_price (prevents hallucinated entries)
        validate_price("entry_price", decision.entry_price, min_price, max_price)?;

        // Stop loss and take profits use directional validation, not distance
        if decision.side == Side::Long {
            if decision.stop_loss >= decision.entry_price {
                return Err(ParseError::InvalidValue(
                    "Long stop_loss must be below entry_price".into(),
                ));
            }
            if decision.take_profit_1 > 0.0 && decision.take_profit_1 <= decision.entry_price {
                return Err(ParseError::InvalidValue(
                    "Long take_profit_1 must be above entry_price".into(),
                ));
            }
        } else if decision.side == Side::Short {
            if decision.stop_loss <= decision.entry_price {
                return Err(ParseError::InvalidValue(
                    "Short stop_loss must be above entry_price".into(),
                ));
            }
            if decision.take_profit_1 > 0.0 && decision.take_profit_1 >= decision.entry_price {
                return Err(ParseError::InvalidValue(
                    "Short take_profit_1 must be below entry_price".into(),
                ));
            }
        }
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

    // Confidence floor: downgrade low-confidence ENTRIES to Hold.
    // Removes the worst trades (0-25% bucket at 18% accuracy). Only applies to
    // new entries (Buy/Sell) — Close and AdjustStop are position management and
    // must not be blocked by a low confidence score.
    let mut decision = decision;
    const CONFIDENCE_FLOOR: f64 = 0.40;
    let is_entry = matches!(decision.action, TradeAction::Buy | TradeAction::Sell);
    if decision.confidence < CONFIDENCE_FLOOR && is_entry {
        tracing::info!(
            "Confidence floor: {:.0}% < {:.0}% — downgrading {:?} to Hold",
            decision.confidence * 100.0,
            CONFIDENCE_FLOOR * 100.0,
            decision.action
        );
        decision.action = TradeAction::Hold;
    }

    Ok(decision)
}

/// Extract JSON from a response that may contain markdown code blocks.
fn extract_json(response: &str) -> &str {
    let trimmed = response.trim();

    // Check for ```json ... ``` wrapper
    if trimmed.starts_with("```json") || trimmed.starts_with("```JSON") {
        // Find the opening newline (after ```json)
        let start = trimmed.find('\n').map(|i| i + 1).unwrap_or(7);
        // Find the CLOSING ``` (must be after start position)
        let end = trimmed[start..]
            .rfind("```")
            .map(|i| start + i)
            .unwrap_or(trimmed.len());
        if start < end {
            return trimmed[start..end].trim();
        }
    }

    // Check for ``` ... ``` wrapper
    if trimmed.starts_with("```") {
        let start = trimmed.find('\n').map(|i| i + 1).unwrap_or(3);
        let end = trimmed[start..]
            .rfind("```")
            .map(|i| start + i)
            .unwrap_or(trimmed.len());
        if start < end {
            return trimmed[start..end].trim();
        }
    }

    // Try to find JSON object boundaries
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if start <= end {
                return &trimmed[start..=end];
            }
        }
    }

    trimmed
}

/// Normalize LLM JSON output — fix common casing issues.
fn normalize_llm_json(json: &str) -> String {
    json.replace("\"action\": \"HOLD\"", "\"action\": \"Hold\"")
        .replace("\"action\": \"BUY\"", "\"action\": \"Buy\"")
        .replace("\"action\": \"SELL\"", "\"action\": \"Sell\"")
        .replace("\"action\": \"CLOSE\"", "\"action\": \"Close\"")
        .replace("\"action\": \"ADJUSTSTOP\"", "\"action\": \"AdjustStop\"")
        .replace("\"action\":\"HOLD\"", "\"action\":\"Hold\"")
        .replace("\"action\":\"BUY\"", "\"action\":\"Buy\"")
        .replace("\"action\":\"SELL\"", "\"action\":\"Sell\"")
        .replace("\"action\":\"CLOSE\"", "\"action\":\"Close\"")
        .replace("\"action\":\"ADJUSTSTOP\"", "\"action\":\"AdjustStop\"")
        .replace("\"side\": \"LONG\"", "\"side\": \"Long\"")
        .replace("\"side\": \"SHORT\"", "\"side\": \"Short\"")
        .replace("\"side\":\"LONG\"", "\"side\":\"Long\"")
        .replace("\"side\":\"SHORT\"", "\"side\":\"Short\"")
        .replace("\"side\": \"long\"", "\"side\": \"Long\"")
        .replace("\"side\": \"short\"", "\"side\": \"Short\"")
        .replace("\"side\": \"\"", "\"side\": \"Long\"")
        .replace("\"side\":\"\"", "\"side\": \"Long\"")
}

/// Manual JSON repair — fix truncated strings, unclosed brackets, trailing commas.
fn repair_json_string(json: &str) -> String {
    let mut s = json.to_string();

    // Fix: unclosed string at end (truncated response)
    // Count quotes, but also check if we're inside a string at EOF
    let quote_count = s.matches('"').count();
    let mut in_str = false;
    let mut prev = ' ';
    for c in s.chars() {
        if c == '"' && prev != '\\' {
            in_str = !in_str;
        }
        prev = c;
    }
    let in_string_at_eof = in_str;

    if !quote_count.is_multiple_of(2) || in_string_at_eof {
        s.push('"');
    }

    // Fix: missing closing braces
    let open_braces = s.matches('{').count();
    let close_braces = s.matches('}').count();
    for _ in 0..(open_braces.saturating_sub(close_braces)) {
        s.push('}');
    }

    // Fix: missing closing brackets
    let open_brackets = s.matches('[').count();
    let close_brackets = s.matches(']').count();
    for _ in 0..(open_brackets.saturating_sub(close_brackets)) {
        s.push(']');
    }

    // Fix: trailing comma before closing brace
    s = s.replace(",}", "}").replace(",]", "]");

    // Fix: single quotes to double quotes (but not inside strings)
    // Simple heuristic: replace ' with " if not inside a string context
    let mut result = String::with_capacity(s.len());
    let mut in_string = false;
    let mut prev_char = ' ';
    for c in s.chars() {
        if c == '"' && prev_char != '\\' {
            in_string = !in_string;
            result.push(c);
        } else if c == '\'' && !in_string {
            result.push('"');
        } else {
            result.push(c);
        }
        prev_char = c;
    }

    // Fix: extra text after JSON object (reasoning/thinking leaking into content)
    // Find the first '{' and track brace depth to find the matching '}'
    if let Some(start) = result.find('{') {
        let mut depth = 0i32;
        let mut end = start;
        let mut in_str = false;
        let mut prev = ' ';
        for (i, c) in result[start..].char_indices() {
            if c == '"' && prev != '\\' {
                in_str = !in_str;
            }
            if !in_str {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i + 1;
                        break;
                    }
                }
            }
            prev = c;
        }
        if end > start {
            result = result[start..end].to_string();
        }
    }

    result
}

/// Partial extraction — salvage valid fields from broken JSON.
/// Builds a TradeDecision from whatever fields we can extract.
fn partial_extract(json: &str) -> Option<TradeDecision> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;

    let action_str = value.get("action")?.as_str()?;
    let action = match action_str {
        "Buy" | "BUY" | "buy" => TradeAction::Buy,
        "Sell" | "SELL" | "sell" => TradeAction::Sell,
        "Hold" | "HOLD" | "hold" => TradeAction::Hold,
        "Close" | "CLOSE" | "close" => TradeAction::Close,
        _ => return None,
    };

    Some(TradeDecision {
        action,
        pair: value
            .get("pair")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC/USD")
            .to_string(),
        side: Side::Long,
        order_type: value
            .get("order_type")
            .and_then(|v| v.as_str())
            .unwrap_or("LIMIT")
            .to_string(),
        entry_price: value
            .get("entry_price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        stop_loss: value
            .get("stop_loss")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        take_profit_1: value
            .get("take_profit_1")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        take_profit_2: value
            .get("take_profit_2")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        take_profit_3: value
            .get("take_profit_3")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        position_size_pct: value
            .get("position_size_pct")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        confidence: value
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5),
        reasoning: value
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("Partial extraction")
            .to_string(),
        knowledge_sources: vec![],
        risk_reward: value
            .get("risk_reward")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    })
}

/// Validate a price is within the acceptable range.
fn validate_price(field: &str, value: f64, min: f64, max: f64) -> Result<(), ParseError> {
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
    fn confidence_floor_downgrades_to_hold() {
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
            "confidence": 0.25,
            "reasoning": "Weak setup",
            "knowledge_sources": [],
            "risk_reward": 2.5
        }"#;

        let decision = parse_decision(json, 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Hold);
    }

    #[test]
    fn confidence_floor_allows_high_confidence() {
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
            "confidence": 0.75,
            "reasoning": "Strong setup",
            "knowledge_sources": [],
            "risk_reward": 2.5
        }"#;

        let decision = parse_decision(json, 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Buy);
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

    #[test]
    fn repair_extra_text_after_json() {
        // Simulates reasoning leaking into content field
        let response = r#"{"action":"Hold","pair":"BTC/USD","side":"Long","entry_price":0.0,"stop_loss":0.0,"take_profit_1":0.0,"take_profit_2":0.0,"take_profit_3":0.0,"position_size_pct":0.0,"confidence":0.0,"reasoning":"No setup","knowledge_sources":[],"risk_reward":0.0}
Some extra reasoning text that leaked into the response..."#;

        let decision = parse_decision(response, 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Hold);
    }

    #[test]
    fn repair_truncated_string() {
        // Simulates a response cut mid-string — the repair should handle this
        // via partial_extract (3rd pass) if repair_json_string can't fix it
        let response = r#"{"action":"Hold","pair":"BTC/USD","side":"Long","entry_price":0.0,"stop_loss":0.0,"take_profit_1":0.0,"take_profit_2":0.0,"take_profit_3":0.0,"position_size_pct":0.0,"confidence":0.0,"reasoning":"No actionable setup due to conf"#;

        let result = parse_decision(response, 65000.0, 10.0);
        // Should succeed via either repair or partial_extract
        assert!(
            result.is_ok(),
            "Truncated string should be salvageable: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().action, TradeAction::Hold);
    }
}
