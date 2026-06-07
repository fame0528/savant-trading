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
    #[serde(
        alias = "HOLD",
        alias = "hold",
        alias = "PASS",
        alias = "pass",
        alias = "SKIP",
        alias = "skip"
    )]
    Pass,
    #[serde(alias = "CLOSE", alias = "close")]
    Close,
    #[serde(
        alias = "ADJUST_STOP",
        alias = "adjust_stop",
        alias = "ADJUSTSTOP",
        alias = "Adjust_Stop"
    )]
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
        if self.action == TradeAction::Pass {
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
    // Strip thinking/reasoning tags that local models (Qwen, DeepSeek, etc.) produce
    let stripped = strip_thinking_tags(response);

    // Extract JSON from response (may be wrapped in markdown code blocks)
    let json_str = extract_json(&stripped);

    // Normalize action field — LLMs sometimes return UPPERCASE
    let normalized = normalize_llm_json(json_str);

    // Try strict parse first, then repair, then partial extraction, then freeform NLP
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
                            // Pass 4: freeform NLP extraction — for local models that
                            // produce natural language instead of JSON
                            match extract_from_freeform(&stripped) {
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
    if decision.action != TradeAction::Pass && decision.action != TradeAction::AdjustStop {
        if decision.entry_price <= 0.0 {
            return Err(ParseError::MissingField("entry_price".to_string()));
        }
        // FID-072: Reject BUY/SELL with no stop loss — naked positions are dangerous
        if decision.stop_loss <= 0.0 {
            return Err(ParseError::InvalidValue(
                "STOP_LOSS_REQUIRED: Buy/Sell must have stop_loss > 0".to_string(),
            ));
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
        decision.action = TradeAction::Pass;
    }

    Ok(decision)
}

/// Strip thinking/reasoning tags that some local models produce.
/// Removes `<think>...</think>` and `</think>` tags that local models (Qwen, DeepSeek, etc.) produce.
/// Extracts any JSON block that appears after the thinking tags.
pub fn strip_thinking_tags(response: &str) -> String {
    let mut result = response.to_string();

    // Remove <think>...</think> blocks (Qwen, DeepSeek)
    while let Some(start) = result.find("<think>") {
        let end = result[start..]
            .find("</think>")
            .map(|i| start + i + 8)
            .unwrap_or(result.len());
        result = format!("{}{}", &result[..start], &result[end..]);
    }

    // Remove <tool_call>...</think> blocks (Qwen tool-calling)
    while let Some(start) = result.find("<tool_call>") {
        let end = result[start..]
            .find("</think>")
            .map(|i| start + i + 8)
            .unwrap_or(result.len());
        result = format!("{}{}", &result[..start], &result[end..]);
    }

    result.trim().to_string()
}

/// Extract a JSON array from a response that may contain surrounding text.
/// MiMo v2.5 Pro often returns individual JSON objects with text between them
/// instead of a clean JSON array. This function handles all cases:
///
/// 1. Fast path: entire string is a valid JSON array
/// 2. Find all balanced `{...}` blocks using brace counting
/// 3. Parse each block as a JSON Value
/// 4. Return as Vec<Value>
pub fn extract_json_array(text: &str) -> Result<Vec<serde_json::Value>, serde_json::Error> {
    let trimmed = text.trim();

    // Fast path: try direct parse as JSON array
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
        return Ok(arr);
    }

    // Slow path: extract individual JSON objects from surrounding text
    let mut objects = Vec::new();
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            // Found start of potential JSON object — count braces to find end
            let mut depth = 0;
            let mut in_string = false;
            let mut escape_next = false;
            let start = i;

            for j in i..chars.len() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                if chars[j] == '\\' && in_string {
                    escape_next = true;
                    continue;
                }
                if chars[j] == '"' {
                    in_string = !in_string;
                    continue;
                }
                if in_string {
                    continue;
                }
                if chars[j] == '{' {
                    depth += 1;
                } else if chars[j] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        // Found balanced object
                        let obj_str: String = chars[start..=j].iter().collect();
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&obj_str) {
                            objects.push(val);
                        }
                        i = j + 1;
                        break;
                    }
                }
            }
            if depth != 0 {
                // Unbalanced braces — skip this `{`
                i = start + 1;
            }
        } else {
            i += 1;
        }
    }

    if objects.is_empty() {
        // Last resort: try parsing as a single object (batch of 1)
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return Ok(vec![val]);
        }
        // Return error — no JSON found
        return Err(serde_json::from_str::<serde_json::Value>("").unwrap_err());
    }

    Ok(objects)
}

/// Extract a trading decision from freeform natural language text.
/// Handles models that don't produce structured JSON output.
/// Returns a TradeDecision if enough fields can be extracted, None otherwise.
fn extract_from_freeform(text: &str) -> Option<TradeDecision> {
    let lower = text.to_lowercase();

    // Extract action
    let action = if lower.contains("buy")
        || lower.contains("go long")
        || lower.contains("enter long")
    {
        TradeAction::Buy
    } else if lower.contains("sell") || lower.contains("go short") || lower.contains("enter short")
    {
        TradeAction::Sell
    } else if lower.contains("close") || lower.contains("exit") {
        TradeAction::Close
    } else if lower.contains("hold")
        || lower.contains("pass")
        || lower.contains("skip")
        || lower.contains("no trade")
    {
        TradeAction::Pass
    } else {
        return None; // Can't determine action
    };

    // If it's a Pass/Hold, return early with minimal fields
    if matches!(action, TradeAction::Pass) {
        return Some(TradeDecision {
            action,
            pair: extract_pair(text).unwrap_or_else(|| "BTC/USD".to_string()),
            side: Side::Long,
            entry_price: 0.0,
            stop_loss: 0.0,
            take_profit_1: 0.0,
            take_profit_2: 0.0,
            take_profit_3: 0.0,
            position_size_pct: 0.0,
            confidence: 0.5,
            reasoning: text.chars().take(500).collect(),
            order_type: "market".to_string(),
            knowledge_sources: vec![],
            risk_reward: 0.0,
        });
    }

    // For Buy/Sell, we need at least a pair and some price info
    let pair = extract_pair(text).unwrap_or_else(|| "BTC/USD".to_string());
    let prices = extract_prices(text);
    let entry = prices.first().copied().unwrap_or(0.0);
    let stop = prices.get(1).copied().unwrap_or(0.0);
    let tp1 = prices.get(2).copied().unwrap_or(0.0);

    // Need at least an entry price to be useful
    if entry <= 0.0 {
        return None;
    }

    let side = if matches!(action, TradeAction::Buy) {
        Side::Long
    } else {
        Side::Short
    };
    let confidence = extract_confidence(text).unwrap_or(0.5);

    // Calculate R:R if we have all three prices
    let rr = if entry > 0.0 && stop > 0.0 && tp1 > 0.0 {
        let risk = (entry - stop).abs();
        let reward = (tp1 - entry).abs();
        if risk > 0.0 {
            reward / risk
        } else {
            0.0
        }
    } else {
        0.0
    };

    Some(TradeDecision {
        action,
        pair,
        side,
        entry_price: entry,
        stop_loss: stop,
        take_profit_1: tp1,
        take_profit_2: 0.0,
        take_profit_3: 0.0,
        position_size_pct: 0.0,
        confidence,
        reasoning: text.chars().take(500).collect(),
        order_type: "market".to_string(),
        knowledge_sources: vec![],
        risk_reward: rr,
    })
}

/// Extract a trading pair from text. Looks for common patterns like "ETH/USD", "BTC/USDC", "ETH-USD".
fn extract_pair(text: &str) -> Option<String> {
    // Pattern: SYMBOL/QUOTE or SYMBOL-QUOTE or SYMBOLUSD
    let pair_re = regex::Regex::new(r"(?i)\b([A-Z]{2,6})\s*/\s*([A-Z]{3,4})\b").ok()?;
    if let Some(caps) = pair_re.captures(text) {
        let base = caps.get(1)?.as_str().to_uppercase();
        let quote = caps.get(2)?.as_str().to_uppercase();
        // Normalize common quote currencies
        let quote_norm = match quote.as_str() {
            "USD" | "USDT" | "USDC" => "USD",
            _ => return None,
        };
        return Some(format!("{}/{}", base, quote_norm));
    }
    None
}

/// Extract numeric prices from text. Returns them in order of appearance.
fn extract_prices(text: &str) -> Vec<f64> {
    // Match numbers that look like prices (with optional $ prefix, commas, decimals)
    let price_re = regex::Regex::new(r"(?i)\$?\s*(\d{1,3}(?:,\d{3})*(?:\.\d+)?)\b").ok();
    let re = match price_re {
        Some(r) => r,
        None => return vec![],
    };
    re.captures_iter(text)
        .filter_map(|caps| {
            let s = caps.get(1)?.as_str().replace(',', "");
            s.parse::<f64>().ok()
        })
        .filter(|&p| p > 0.001) // Filter out tiny numbers
        .collect()
}

/// Extract a confidence value from text. Looks for patterns like "confidence: 0.7" or "70%".
fn extract_confidence(text: &str) -> Option<f64> {
    // Pattern: confidence: 0.XX or confidence: XX%
    let conf_re = regex::Regex::new(r"(?i)confidence[:\s]+(\d*\.?\d+)").ok()?;
    if let Some(caps) = conf_re.captures(text) {
        let val: f64 = caps.get(1)?.as_str().parse().ok()?;
        if val > 1.0 {
            return Some(val / 100.0);
        } // Handle percentage format
        if (0.0..=1.0).contains(&val) {
            return Some(val);
        }
    }
    // Pattern: XX% confidence
    let pct_re = regex::Regex::new(r"(\d{1,3})%\s*(?:confident|confidence)").ok()?;
    if let Some(caps) = pct_re.captures(text) {
        let val: f64 = caps.get(1)?.as_str().parse().ok()?;
        return Some(val / 100.0);
    }
    None
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

/// Normalize LLM JSON output — fix common casing and spacing issues.
/// Uses regex to handle arbitrary whitespace between key, colon, and value.
fn normalize_llm_json(json: &str) -> String {
    let action_re = regex::Regex::new(r#""action"\s*:\s*"([^"]+)""#).unwrap();
    let result = action_re.replace_all(json, |caps: &regex::Captures| {
        let val = caps.get(1).unwrap().as_str();
        let upper = val.to_uppercase();
        let normalized = match upper.as_str() {
            "HOLD" | "PASS" => "Pass",
            "BUY" | "LONG" => "Buy",
            "SELL" | "SHORT" => "Sell",
            "CLOSE" | "EXIT" => "Close",
            "ADJUSTSTOP" | "ADJUST_STOP" | "ADJUST" => "AdjustStop",
            _ => val,
        };
        format!("\"action\": \"{}\"", normalized)
    });

    let side_re = regex::Regex::new(r#""side"\s*:\s*"([^"]+)""#).unwrap();
    let result = side_re.replace_all(result.as_ref(), |caps: &regex::Captures| {
        let val = caps.get(1).unwrap().as_str();
        let upper = val.to_uppercase();
        let normalized = match upper.as_str() {
            "LONG" | "BUY" => "Long",
            "SHORT" | "SELL" => "Short",
            "" => "Long",
            _ => val,
        };
        format!("\"side\": \"{}\"", normalized)
    });

    result.into_owned()
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
        "Hold" | "HOLD" | "hold" | "Pass" | "PASS" | "pass" | "Skip" | "SKIP" | "skip" => {
            TradeAction::Pass
        }
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
    "action": "Pass",
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
        assert_eq!(decision.action, TradeAction::Pass);
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
        assert_eq!(decision.action, TradeAction::Pass);
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
        let response = r#"{"action":"Pass","pair":"BTC/USD","side":"Long","entry_price":0.0,"stop_loss":0.0,"take_profit_1":0.0,"take_profit_2":0.0,"take_profit_3":0.0,"position_size_pct":0.0,"confidence":0.0,"reasoning":"No setup","knowledge_sources":[],"risk_reward":0.0}
Some extra reasoning text that leaked into the response..."#;

        let decision = parse_decision(response, 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Pass);
    }

    #[test]
    fn repair_truncated_string() {
        // Simulates a response cut mid-string — the repair should handle this
        // via partial_extract (3rd pass) if repair_json_string can't fix it
        let response = r#"{"action":"Pass","pair":"BTC/USD","side":"Long","entry_price":0.0,"stop_loss":0.0,"take_profit_1":0.0,"take_profit_2":0.0,"take_profit_3":0.0,"position_size_pct":0.0,"confidence":0.0,"reasoning":"No actionable setup due to conf"#;

        let result = parse_decision(response, 65000.0, 10.0);
        // Should succeed via either repair or partial_extract
        assert!(
            result.is_ok(),
            "Truncated string should be salvageable: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().action, TradeAction::Pass);
    }
}
