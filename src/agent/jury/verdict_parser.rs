//! Verdict parser — extracts structured JuryVerdict from jury member LLM responses.
//!
//! Uses multi-pass parsing similar to decision_parser:
//! 1. Strip thinking tags
//! 2. Extract JSON
//! 3. Strict parse
//! 4. Repair + reparse
//! 5. Partial extraction (salvage verdict + confidence)
//! 6. Freeform NLP extraction

use super::JuryVerdict;
use crate::agent::decision_parser::{extract_json, repair_json_string, strip_thinking_tags};

/// Errors from verdict parsing.
#[derive(Debug, thiserror::Error)]
pub enum VerdictParseError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("No JSON found in response")]
    NoJsonFound,
    #[error("Could not extract verdict from response")]
    NoVerdict,
}

/// Parse a jury member's response into a JuryVerdict.
///
/// Uses 6-pass parsing to handle the wildly varying output quality
/// of free models (some return clean JSON, some return prose, some
/// return markdown-wrapped JSON, some return truncated responses).
pub fn parse_verdict(response: &str) -> Result<JuryVerdict, VerdictParseError> {
    // Pass 1: Strip thinking tags
    let stripped = strip_thinking_tags(response);

    // Pass 2: Extract JSON from markdown wrappers / surrounding text
    let json_str = extract_json(&stripped);

    // Pass 3: Strict parse
    if let Ok(v) = serde_json::from_str::<JuryVerdict>(json_str) {
        return Ok(v);
    }

    // Pass 4: Repair JSON (truncated strings, unclosed brackets, trailing commas)
    let repaired = repair_json_string(json_str);
    if let Ok(v) = serde_json::from_str::<JuryVerdict>(&repaired) {
        return Ok(v);
    }

    // Pass 5: Partial extraction — salvage what we can from broken JSON
    if let Some(v) = partial_verdict_extract(json_str) {
        return Ok(v);
    }
    if let Some(v) = partial_verdict_extract(&repaired) {
        return Ok(v);
    }

    // Pass 6: Freeform NLP — extract verdict from natural language
    if let Some(v) = extract_verdict_from_freeform(&stripped) {
        return Ok(v);
    }

    // Pass 7: Last resort — try parsing the entire stripped response as JSON
    // (some models return JSON without markdown wrapping)
    if let Ok(v) = serde_json::from_str::<JuryVerdict>(&stripped) {
        return Ok(v);
    }

    Err(VerdictParseError::NoVerdict)
}

/// Extract verdict + confidence from broken/partial JSON.
fn partial_verdict_extract(json: &str) -> Option<JuryVerdict> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;

    let verdict = value
        .get("verdict")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("action").and_then(|v| v.as_str()))
        .or_else(|| value.get("decision").and_then(|v| v.as_str()))?;

    let normalized = normalize_verdict(verdict);

    Some(JuryVerdict {
        verdict: normalized,
        confidence: value
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5),
        key_argument: value
            .get("key_argument")
            .or_else(|| value.get("reasoning"))
            .or_else(|| value.get("thesis"))
            .and_then(|v| v.as_str())
            .unwrap_or("Partial extraction")
            .to_string(),
        risk_flag: value
            .get("risk_flag")
            .or_else(|| value.get("risk"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        evidence_quality: value
            .get("evidence_quality")
            .and_then(|v| v.as_f64())
            .or_else(|| {
                // Handle "7/10" format
                value
                    .get("evidence_quality")
                    .and_then(|v| v.as_str())
                    .and_then(parse_fraction)
            }),
        reasoning: value
            .get("reasoning")
            .or_else(|| value.get("analysis"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Extract verdict from freeform natural language text.
fn extract_verdict_from_freeform(text: &str) -> Option<JuryVerdict> {
    let lower = text.to_lowercase();

    // Extract verdict
    let verdict = if lower.contains("buy") || lower.contains("go long") || lower.contains("bullish")
    {
        "BUY"
    } else if lower.contains("sell") || lower.contains("go short") || lower.contains("bearish") {
        "SELL"
    } else if lower.contains("hold")
        || lower.contains("pass")
        || lower.contains("skip")
        || lower.contains("no trade")
    {
        "HOLD"
    } else {
        return None;
    };

    let confidence = extract_confidence_from_text(&lower).unwrap_or(0.5);

    // Extract key argument — first sentence that contains reasoning keywords
    let key_argument = extract_key_argument(&lower);

    Some(JuryVerdict {
        verdict: verdict.to_string(),
        confidence,
        key_argument,
        risk_flag: String::new(),
        evidence_quality: None,
        reasoning: text.chars().take(500).collect(),
    })
}

/// Normalize verdict string to uppercase standard form.
fn normalize_verdict(verdict: &str) -> String {
    let upper = verdict.to_uppercase();
    match upper.as_str() {
        "BUY" | "LONG" | "BULLISH" => "BUY".to_string(),
        "SELL" | "SHORT" | "BEARISH" => "SELL".to_string(),
        "HOLD" | "PASS" | "SKIP" | "NEUTRAL" | "NO TRADE" => "HOLD".to_string(),
        _ => upper,
    }
}

/// Parse a fraction string like "7/10" into a float.
fn parse_fraction(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].trim().parse().ok()?;
        let den: f64 = parts[1].trim().parse().ok()?;
        if den > 0.0 {
            return Some(num / den);
        }
    }
    None
}

/// Extract confidence from text. Looks for patterns like "confidence: 0.7" or "70%".
fn extract_confidence_from_text(text: &str) -> Option<f64> {
    let conf_re = regex::Regex::new(r"confidence[:\s]+(\d*\.?\d+)").ok()?;
    if let Some(caps) = conf_re.captures(text) {
        let val: f64 = caps.get(1)?.as_str().parse().ok()?;
        if val > 1.0 {
            return Some(val / 100.0);
        }
        if (0.0..=1.0).contains(&val) {
            return Some(val);
        }
    }
    let pct_re = regex::Regex::new(r"(\d{1,3})%\s*(?:confident|confidence)").ok()?;
    if let Some(caps) = pct_re.captures(text) {
        let val: f64 = caps.get(1)?.as_str().parse().ok()?;
        return Some(val / 100.0);
    }
    None
}

/// Extract a key argument from the first meaningful sentence.
fn extract_key_argument(text: &str) -> String {
    // Look for sentences containing reasoning keywords
    let keywords = [
        "because",
        "due to",
        "given",
        "suggests",
        "indicates",
        "shows",
    ];
    for sentence in text.split(&['.', '!', '?'][..]) {
        let trimmed = sentence.trim();
        if keywords.iter().any(|k| trimmed.contains(k)) && trimmed.len() > 10 {
            return trimmed.chars().take(200).collect();
        }
    }
    // Fallback: first non-empty sentence
    text.split(&['.', '!', '?'][..])
        .find(|s| s.trim().len() > 10)
        .map(|s| s.trim().chars().take(200).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_verdict() {
        let json = r#"{
            "verdict": "BUY",
            "confidence": 0.72,
            "key_argument": "Strong EMA crossover with volume confirmation",
            "risk_flag": "RSI approaching overbought at 68",
            "evidence_quality": 8.0,
            "reasoning": "The 5m chart shows a clear bullish engulfing pattern."
        }"#;
        let v = parse_verdict(json).unwrap();
        assert_eq!(v.verdict, "BUY");
        assert_eq!(v.confidence, 0.72);
        assert_eq!(v.evidence_quality, Some(8.0));
    }

    #[test]
    fn parse_verdict_with_thinking_tags() {
        let response = r#"<think>Let me analyze this...</think>
{"verdict": "HOLD", "confidence": 0.35, "key_argument": "Price at resistance", "risk_flag": "Bearish divergence", "reasoning": "Mixed signals"}"#;
        let v = parse_verdict(response).unwrap();
        assert_eq!(v.verdict, "HOLD");
        assert_eq!(v.confidence, 0.35);
    }

    #[test]
    fn parse_verdict_with_markdown_wrapper() {
        let response = r#"Here is my analysis:
```json
{"verdict": "SELL", "confidence": 0.65, "key_argument": "Breakdown below support", "risk_flag": "Volume declining", "reasoning": "Bearish structure"}
```"#;
        let v = parse_verdict(response).unwrap();
        assert_eq!(v.verdict, "SELL");
    }

    #[test]
    fn parse_freeform_buy() {
        let response = "I think we should BUY here because the EMA crossover is strong with 70% confidence. Volume is confirming the move.";
        let v = parse_verdict(response).unwrap();
        assert_eq!(v.verdict, "BUY");
        assert!((v.confidence - 0.70).abs() < 0.01);
    }

    #[test]
    fn parse_freeform_hold() {
        let response = "No clear setup. I recommend HOLD — mixed signals across timeframes.";
        let v = parse_verdict(response).unwrap();
        assert_eq!(v.verdict, "HOLD");
    }

    #[test]
    fn parse_action_field_alias() {
        // Some models use "action" instead of "verdict"
        let json = r#"{"action": "Buy", "confidence": 0.6, "reasoning": "Strong momentum"}"#;
        let v = parse_verdict(json).unwrap();
        assert_eq!(v.verdict, "BUY");
    }

    #[test]
    fn parse_evidence_quality_fraction() {
        let json = r#"{"verdict": "BUY", "confidence": 0.8, "evidence_quality": "7/10", "reasoning": "test"}"#;
        let v = parse_verdict(json).unwrap();
        assert!((v.evidence_quality.unwrap() - 0.7).abs() < 0.01);
    }

    #[test]
    fn parse_truncated_verdict() {
        let json =
            r#"{"verdict": "BUY", "confidence": 0.65, "key_argument": "Strong setup due to vol"#;
        let v = parse_verdict(json).unwrap();
        assert_eq!(v.verdict, "BUY");
    }

    #[test]
    fn normalize_verdict_variants() {
        assert_eq!(normalize_verdict("buy"), "BUY");
        assert_eq!(normalize_verdict("LONG"), "BUY");
        assert_eq!(normalize_verdict("bullish"), "BUY");
        assert_eq!(normalize_verdict("short"), "SELL");
        assert_eq!(normalize_verdict("pass"), "HOLD");
        assert_eq!(normalize_verdict("skip"), "HOLD");
    }

    #[test]
    fn parse_garbled_text_fails_gracefully() {
        let response = "asdkjfh asdkjh faskdjfh askdjfh";
        let result = parse_verdict(response);
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_response() {
        let result = parse_verdict("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_verdict_with_decision_alias() {
        let json = r#"{"decision": "SELL", "confidence": 0.55, "reasoning": "Breakdown"}"#;
        let v = parse_verdict(json).unwrap();
        assert_eq!(v.verdict, "SELL");
    }
}
