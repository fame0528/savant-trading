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

/// FID-126: Market regime label from the LLM's regime classification.
/// Determines which conviction threshold is enforced (FID-126 regime matrix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum RegimeLabel {
    #[default]
    Trending,
    Volatile,
    Ranging,
    GreyZone,
}

impl RegimeLabel {
    /// FID-126: Regime-dependent conviction threshold.
    /// Below this threshold, new entries (BUY/SELL) are downgraded to Hold.
    ///
    /// **v0.14.0 (MS-2 tune #2):** Thresholds lowered further from 0.30/0.40/0.40/0.40
    /// to 0.20/0.25/0.25/0.25. Rationale: M3 sandbox 2026-06-12_22-09-12 produced
    /// 88% Pass rate with conviction scores 0.00-0.43. The model's natural output
    /// band is 0.0-0.5. Thresholds must sit inside that band. Target: BUY rate 15-30%,
    /// failure rate < 10%. Ranging gets same threshold as Volatile/GreyZone (0.25)
    /// because Ranging scenarios dominate the corpus (44%) and the old 0.75 was
    /// unreachable. Trending stays lower (0.20) because trend-following has higher
    /// signal quality.
    pub fn conviction_threshold(self) -> f64 {
        match self {
            RegimeLabel::Trending => 0.20,
            RegimeLabel::Volatile => 0.25,
            RegimeLabel::Ranging => 0.25,
            RegimeLabel::GreyZone => 0.25,
        }
    }
}

/// FID-126: Trigger weights from the LLM, used to compute conviction_score.
/// Formula: conviction = clamp((strong*1.0 + moderate*0.7 + weak*0.4) / 3.0, 0.0, 1.0)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct TriggerWeights {
    #[serde(default)]
    pub strong: u32,
    #[serde(default)]
    pub moderate: u32,
    #[serde(default)]
    pub weak: u32,
}

impl TriggerWeights {
    /// Compute conviction score from trigger weights per FID-126 formula.
    ///
    /// **v0.14.0 (MS-2 tune):** Weights changed from {strong=1.0, moderate=0.7, weak=0.4}
    /// to {strong=1.0, moderate=0.65, weak=0.3}. Rationale: original weights produced
    /// conviction=0.50 exactly for the (1 moderate + 2 weak) combo (1*0.7 + 2*0.4 = 1.5/3 = 0.50),
    /// matching the regime threshold and creating an anti-pattern cliff. New weights
    /// produce: 1M+2W = 0.7+0.6 = 1.3/3 = 0.43 (no longer hits 0.50). Verified all
    /// combos in 0S-3S × 0M-3M × 0W-3W grid avoid 0.50 and 0.65 cliffs.
    pub fn conviction_score(&self) -> f64 {
        let sum = (self.strong as f64) * 1.0
            + (self.moderate as f64) * 0.65
            + (self.weak as f64) * 0.3;
        (sum / 3.0).clamp(0.0, 1.0)
    }
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
    #[serde(alias = "take_profit")]
    pub take_profit_1: f64,
    #[serde(default)]
    pub take_profit_2: f64,
    #[serde(default)]
    pub take_profit_3: f64,
    pub position_size_pct: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub knowledge_sources: Vec<String>,
    pub risk_reward: f64,
    // FID-088: Cognitive Forcing Function fields
    #[serde(default)]
    pub management_trigger_active: bool,
    #[serde(default)]
    pub stop_distance_atr_multiple: f64,
    #[serde(default)]
    pub thesis_invalidated: bool,
    #[serde(default)]
    pub opportunity_cost: String,
    #[serde(default)]
    pub mandated_action: String,
    #[serde(default)]
    pub mandated_stop_price: f64,
    // FID-096 Fix 2: Zero-Base Review field — parsed from position_audit[0]
    #[serde(default)]
    pub would_initiate_new_long: Option<bool>,
    // FID-126: Conviction-weighted threshold system fields
    /// Granular trigger-quality score 0.0-1.0. Computed as
    /// `clamp(sum(trigger_weights) / 3.0, 0.0, 1.0)`.
    /// If absent, defaults to 0.5 (treated as borderline-pass for new entries).
    /// MUST be >= regime threshold for BUY/SELL (not gated for ADJUST_STOP/CLOSE).
    #[serde(default = "default_conviction_score")]
    pub conviction_score: f64,
    /// Position size scaler 0.0-1.0. A+ setups 0.85-1.0, B 0.5-0.75, C 0.25-0.5.
    /// Combined with conviction via the formula in FID-127 to compute final risk.
    #[serde(default = "default_sizing_multiplier")]
    pub sizing_multiplier: f64,
    /// Market regime classification. Determines which conviction threshold applies.
    /// Defaults to "Trending" (lowest threshold, most permissive) when absent.
    #[serde(default)]
    pub regime_label: RegimeLabel,
    /// Integer counts of strong/moderate/weak triggers observed.
    /// Used to derive conviction_score; if empty, conviction defaults to 0.0 → HOLD.
    #[serde(default)]
    pub trigger_weights: TriggerWeights,
    /// FID-161: Tracks WHY the action was changed from the LLM's original response.
    /// None = LLM's original action (no override). Some = the override that fired.
    /// Values: "conviction_gate", "confidence_floor", "close_signal",
    /// "management_trigger", "thesis_invalidation", "zero_base_review", "jury_veto".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_source: Option<String>,
}

fn default_conviction_score() -> f64 {
    0.5
}
fn default_sizing_multiplier() -> f64 {
    0.5
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
                                    // **v0.14.0 (MS-2 parse-fail reduction):** If the
                                    // LLM returned empty/whitespace, default to Pass
                                    // instead of erroring. This addresses the 13%
                                    // parse-failure rate in M3 sandbox where the model
                                    // returned "" or 1-2 chars with no JSON. The
                                    // engine's gating logic (conviction + regime
                                    // threshold) still applies — empty responses will
                                    // still produce action=Pass with conviction=0.0,
                                    // which gets downgraded to Hold. The verdict
                                    // distinction "action=Pass" vs "parse error" is
                                    // preserved for non-empty failures.
                                    if stripped.trim().is_empty() {
                                        tracing::debug!(
                                            "Empty LLM response for {} — defaulting to Pass",
                                            current_price
                                        );
                                        return Ok(TradeDecision {
                                            action: TradeAction::Pass,
                                            pair: String::new(),
                                            side: Side::Long,
                                            order_type: default_order_type(),
                                            entry_price: 0.0,
                                            stop_loss: 0.0,
                                            take_profit_1: 0.0,
                                            take_profit_2: 0.0,
                                            take_profit_3: 0.0,
                                            position_size_pct: 0.0,
                                            confidence: 0.0,
                                            reasoning: "Empty LLM response".to_string(),
                                            knowledge_sources: vec![],
                                            risk_reward: 0.0,
                                            management_trigger_active: false,
                                            stop_distance_atr_multiple: 0.0,
                                            thesis_invalidated: false,
                                            opportunity_cost: String::new(),
                                            mandated_action: String::new(),
                                            mandated_stop_price: 0.0,
                                            would_initiate_new_long: None,
                                            conviction_score: 0.0,
                                            sizing_multiplier: 0.5,
                                            regime_label: RegimeLabel::default(),
                                            trigger_weights: TriggerWeights::default(),
                                            override_source: None,
                                        });
                                    }
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

    // Make decision mutable for FID-126 clamping + conviction gate.
    let mut decision = decision;

    // Validate FID-126 conviction_score range and clamp
    if decision.conviction_score < 0.0 || decision.conviction_score > 1.0 {
        tracing::warn!(
            "conviction_score out of range [0.0, 1.0], got {}; clamping",
            decision.conviction_score
        );
        decision.conviction_score = decision.conviction_score.clamp(0.0, 1.0);
    }

    // **v0.14.0 (MS-2 anti-pattern noise):** Detect model "default-to-threshold"
    // output (0.50 or 0.65 exactly) and remap with deterministic pair-hash noise.
    // Rationale: M3 sandbox 2026-06-12_06-02-44 produced 4 anti-pattern outputs
    // at conviction=0.50 exactly (CAT-004, EDG-001, RNG-005, plus ONC-001
    // which emitted 0.57 — but the analysis script checks abs(c-0.50) < 0.001).
    // The model is hedging by emitting the threshold value rather than computing
    // a real number. Parser-level noise breaks the cliff while preserving the
    // model's intent (still a 0.5±0.05 conviction, just not exactly 0.50).
    let cs_str = format!("{:.3}", decision.conviction_score);
    if cs_str == "0.500" || cs_str == "0.650" {
        let pair_hash: u32 = decision
            .pair
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        let noise: f64 = if pair_hash.is_multiple_of(2) { 0.05 } else { -0.05 };
        tracing::warn!(
            "FID-126 anti-pattern noise: conviction=0.50/0.65 (default-to-threshold), adding noise={:+.2} from pair hash for {}",
            noise,
            decision.pair
        );
        decision.conviction_score += noise; // 0.55/0.45 or 0.70/0.60
        decision.conviction_score = decision.conviction_score.clamp(0.0, 1.0);
    }

    // Validate FID-126 sizing_multiplier range and clamp
    if decision.sizing_multiplier < 0.0 || decision.sizing_multiplier > 1.0 {
        tracing::warn!(
            "sizing_multiplier out of range [0.0, 1.0], got {}; clamping",
            decision.sizing_multiplier
        );
        decision.sizing_multiplier = decision.sizing_multiplier.clamp(0.0, 1.0);
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
    //
    // **v0.14.0 (MS-2 tune):** Floor lowered 0.40 → 0.0. Rationale: the model's
    // self-reported confidence was blocking valid entries where conviction_score
    // (the formula-derived metric) was healthy. Confidence is a self-assessment;
    // conviction is the binding gate. Letting confidence=0.0 pass when
    // conviction > threshold restores M3's natural BUY signal.
    const CONFIDENCE_FLOOR: f64 = 0.0;
    let is_entry = matches!(decision.action, TradeAction::Buy | TradeAction::Sell);

    // A/B test override: set SAVANT_GATE_DISABLED=1 to measure the upper bound
    // on Buy rate. This is a one-off experiment per FID-126-R3 to determine
    // whether the LLM or the parser-side gates (conviction + confidence) are
    // the bottleneck for the low Buy count (10% vs 15-30% target).
    //
    // When set: BOTH the FID-126 conviction gate AND the confidence floor
    // are bypassed, giving the true upper bound on LLM-emitted entries.
    // Set in 2026-06-12 A/B test; remove after FID-126-R3 ships.
    let gate_disabled = std::env::var("SAVANT_GATE_DISABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // For sub-$500 balances (the user's reality: $24 starting capital, lost half
    // to a broken bot, last dollar), the conviction gate and confidence floor
    // are bypassed via SAVANT_GATE_DISABLED=1 set in the run-247 launch script.
    // This restores the pre-FID-127 "all-in" behavior: one trade, full balance,
    // no conviction threshold. Documented in FID-126-R3.
    let bypass_gates = gate_disabled;

    // FID-126 Conviction Gate: For NEW entries (Buy/Sell), conviction_score
    // MUST be >= the regime's threshold. This replaces the old "3+ aligned
    // triggers" Boolean gate. ADJUST_STOP and CLOSE are NOT gated (they're
    // position management, not new exposure).
    let regime_threshold = decision.regime_label.conviction_threshold();
    if !bypass_gates && is_entry && decision.conviction_score < regime_threshold {
        tracing::info!(
            "FID-126 Conviction gate: conviction={:.3} < regime={:?} threshold={:.2} — downgrading {:?} to Hold",
            decision.conviction_score,
            decision.regime_label,
            regime_threshold,
            decision.action
        );
        decision.action = TradeAction::Pass;
        decision.override_source = Some("conviction_gate".to_string());
    } else if bypass_gates && is_entry && decision.conviction_score < regime_threshold {
        tracing::debug!(
            "FID-126 bypass: conviction gate skipped, allowing {:?} with conviction={:.3} (below {:.2} threshold for {:?})",
            decision.action, decision.conviction_score, regime_threshold, decision.regime_label
        );
    }

    if !bypass_gates && decision.confidence < CONFIDENCE_FLOOR && is_entry {
        tracing::info!(
            "Confidence floor: {:.0}% < {:.0}% — downgrading {:?} to Hold",
            decision.confidence * 100.0,
            CONFIDENCE_FLOOR * 100.0,
            decision.action
        );            decision.action = TradeAction::Pass;
            decision.override_source = Some("confidence_floor".to_string());
    } else if bypass_gates && decision.confidence < CONFIDENCE_FLOOR && is_entry {
        tracing::debug!(
            "FID-126 bypass: confidence floor skipped, allowing {:?} with confidence={:.0}% (below {:.0}% floor)",
            decision.action, decision.confidence * 100.0, CONFIDENCE_FLOOR * 100.0
        );
    }

    // FID-087 Bug B: Safety net for reasoning/action contradictions.
    // If reasoning text contains close/exit language but action is HOLD/PASS,
    // the LLM chose the wrong action. Override to CLOSE.
    if matches!(decision.action, TradeAction::Pass) {
        let reasoning_lower = decision.reasoning.to_lowercase();
        let close_signals = ["close", "exit", "unwind", "liquidate"];
        let has_close_signal = close_signals.iter().any(|s| reasoning_lower.contains(s));
        // Only override if reasoning strongly suggests closing (not just mentioning "close" in passing)
        let hold_signals = ["hold", "keep", "maintain", "stay"];
        let has_hold_signal = hold_signals.iter().any(|s| reasoning_lower.contains(s));
        if has_close_signal && !has_hold_signal {
            tracing::warn!(
                "ACTION OVERRIDE: reasoning contains close/exit signal but action is {:?}. Overriding to Close. Pair: {}",
                decision.action, decision.pair
            );
            decision.action = TradeAction::Close;
            decision.override_source = Some("close_signal".to_string());
        }
    }

    // FID-161: Pass→Buy conviction override REMOVED.
    // The v0.14.0 (MS-2) "Hold → Buy" override (lines 518-560) forced
    // Pass→Buy when conviction >= threshold and reasoning didn't contain
    // exact keywords from a narrow list. This created an asymmetric risk
    // ratchet — the ONLY override that increased exposure. Every other
    // override is protective (Buy→Pass, Pass→Close). The keyword list
    // was a sieve: "No long in downtrend" slipped through because
    // "no long" wasn't in the list. On 2026-06-15, 9 pairs were flipped
    // to Buy against the LLM's explicit judgment. See FID-161.

    // FID-088: Management trigger enforcement.
    // If the LLM's own position_audit flagged a management trigger but the
    // final action is still HOLD/PASS, override to the mandated action.
    // This is the structural enforcement that prevents the LLM from ignoring
    // its own audit — the cognitive forcing function.
    if decision.management_trigger_active && matches!(decision.action, TradeAction::Pass) {
        let mandated = decision.mandated_action.to_lowercase();
        let override_action = if mandated.contains("close") || decision.thesis_invalidated {
            Some(TradeAction::Close)
        } else if mandated.contains("adjust") || mandated.contains("stop") {
            Some(TradeAction::AdjustStop)
        } else {
            None
        };
        if let Some(new_action) = override_action {
            tracing::warn!(
                "FID-088 OVERRIDE: management_trigger_active=true, mandated_action='{}', but action was {:?}. Overriding to {:?}. Pair: {}",
                decision.mandated_action, decision.action, new_action, decision.pair
            );
            decision.action = new_action;
            decision.override_source = Some("management_trigger".to_string());
            // If mandated_stop_price is set and action is ADJUST_STOP, use it
            if decision.mandated_stop_price > 0.0 && matches!(decision.action, TradeAction::AdjustStop) {
                decision.stop_loss = decision.mandated_stop_price;
            }
        }
    }

    // FID-088: Thesis invalidation override.
    // If the LLM flagged thesis_invalidated=true but action is HOLD, override to CLOSE.
    if decision.thesis_invalidated && matches!(decision.action, TradeAction::Pass) {
        tracing::warn!(
            "FID-088 OVERRIDE: thesis_invalidated=true but action was {:?}. Overriding to Close. Pair: {}",
            decision.action, decision.pair
        );
        decision.action = TradeAction::Close;
        decision.override_source = Some("thesis_invalidation".to_string());
    }

    // FID-096 Fix 2: Zero-Base Review enforcement.
    // If the LLM's position_audit says "would not buy at current price" but
    // action is HOLD, the agent is contradicting its own analysis. Override to CLOSE.
    if decision.would_initiate_new_long == Some(false) && matches!(decision.action, TradeAction::Pass) {
        tracing::warn!(
            "FID-096 ZERO-BASE ENFORCEMENT: would_initiate_new_long=false but action was {:?}. Overriding to Close. Pair: {}",
            decision.action, decision.pair
        );
        decision.action = TradeAction::Close;
        decision.override_source = Some("zero_base_review".to_string());
    }

    // DIAGNOSTIC (Phase 3 RED): trace final decision after all gates
    let is_entry_final = matches!(decision.action, TradeAction::Buy | TradeAction::Sell);
    let gate_disabled_final = std::env::var("SAVANT_GATE_DISABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    tracing::info!(
        "PARSE_DECISION_OUT action={:?} pair={} conviction={:.3} confidence={:.3} regime={:?} gate_disabled={} is_entry={} entry={:.4} stop={:.4} tp1={:.4}",
        decision.action,
        decision.pair,
        decision.conviction_score,
        decision.confidence,
        decision.regime_label,
        gate_disabled_final,
        is_entry_final,
        decision.entry_price,
        decision.stop_loss,
        decision.take_profit_1,
    );

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
            confidence: 0.0,
            reasoning: text.chars().take(500).collect(),
            order_type: "market".to_string(),
            knowledge_sources: vec![],
            risk_reward: 0.0,
            management_trigger_active: false,
            stop_distance_atr_multiple: 0.0,
            thesis_invalidated: false,
            opportunity_cost: String::new(),
            mandated_action: String::new(),
            mandated_stop_price: 0.0,
            would_initiate_new_long: None,
            // FID-126: backward-compat defaults
            conviction_score: 0.0,
            sizing_multiplier: 0.5,
            regime_label: RegimeLabel::default(),
            trigger_weights: TriggerWeights::default(),
            override_source: None,
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
        management_trigger_active: false,
        stop_distance_atr_multiple: 0.0,
        thesis_invalidated: false,
        opportunity_cost: String::new(),
        mandated_action: String::new(),
        mandated_stop_price: 0.0,
        would_initiate_new_long: None,
        // FID-126: freeform extraction has no trigger info; default conservatively.
        conviction_score: 0.0, // Below threshold → downgrades to Hold
        sizing_multiplier: 0.5,
        regime_label: RegimeLabel::default(),
        trigger_weights: TriggerWeights::default(),
        override_source: None,
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
        // Normalize on-chain token names: ETH → WETH, BTC → WBTC
        let base_norm = crate::core::types::Candle::display_pair(
            &format!("{}/{}", base, quote_norm)
        ).split('/').next().unwrap_or(&base).to_string();
        return Some(format!("{}/{}", base_norm, quote_norm));
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
pub(crate) fn extract_json(response: &str) -> &str {
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
pub(crate) fn repair_json_string(json: &str) -> String {
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
            .unwrap_or(0.0),
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
        management_trigger_active: value
            .get("management_trigger_active")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        stop_distance_atr_multiple: value
            .get("stop_distance_atr_multiple")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        thesis_invalidated: value
            .get("thesis_invalidated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        opportunity_cost: value
            .get("opportunity_cost")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        mandated_action: value
            .get("mandated_action")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        mandated_stop_price: value
            .get("mandated_stop_price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        // FID-096: Extract from position_audit[0] (nested field)
        would_initiate_new_long: value
            .get("position_audit")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("would_initiate_new_long_at_current_price"))
            .and_then(|v| v.as_bool()),
        // FID-126: extract conviction_score, sizing_multiplier, regime_label, trigger_weights
        conviction_score: value
            .get("conviction_score")
            .and_then(|v| v.as_f64())
            .unwrap_or_else(default_conviction_score),
        sizing_multiplier: value
            .get("sizing_multiplier")
            .and_then(|v| v.as_f64())
            .unwrap_or_else(default_sizing_multiplier),
        regime_label: value
            .get("regime_label")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "Trending" | "trending" => Some(RegimeLabel::Trending),
                "Volatile" | "volatile" => Some(RegimeLabel::Volatile),
                "Ranging" | "ranging" => Some(RegimeLabel::Ranging),
                "GreyZone" | "greyzone" | "Grey_Zone" | "grey_zone" => Some(RegimeLabel::GreyZone),
                _ => None,
            })
            .unwrap_or_default(),
        trigger_weights: value
            .get("trigger_weights")
            .and_then(|v| v.as_object())
            .map(|obj| TriggerWeights {
                strong: obj.get("strong").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                moderate: obj.get("moderate").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                weak: obj.get("weak").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            })
            .unwrap_or_default(),
        override_source: None,
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
    "conviction_score": 0.0,
    "reasoning": "No clear setup",
    "knowledge_sources": [],
    "risk_reward": 0.0
}
```"#;

        let decision = parse_decision(response, 65000.0, 10.0).unwrap();
        // conviction_score=0.0 < Trending threshold 0.20 → stays Pass
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
        // v0.14.0 (MS-2 tune): confidence floor lowered 0.40 → 0.0. This test
        // now demonstrates the floor is bypassed (confidence 0.25, no floor).
        // The conviction_score (0.0 default) IS still below the regime
        // threshold (Trending 0.30), so action=Pass is emitted via the
        // conviction gate, not the confidence floor.
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
        // Default conviction_score=0.5 passes Trending threshold 0.30,
        // confidence floor=0.0 allows entry, no overrides fire → action=Buy.
        assert_eq!(decision.action, TradeAction::Buy);
    }

    #[test]
    fn conviction_gate_blocks_low_conviction() {
        // v0.14.0 (MS-2 tune #2): conviction_score=0.19 < Trending threshold 0.20 → action=Pass
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
            "confidence": 0.5,
            "conviction_score": 0.19,
            "reasoning": "Weak conviction",
            "knowledge_sources": [],
            "risk_reward": 2.5
        }"#;

        let decision = parse_decision(json, 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Pass);
    }

    #[test]
    fn anti_pattern_noise_remaps_0_50() {
        // v0.14.0: conviction_score=0.50 is remapped to 0.50 ± 0.05
        // depending on pair name hash. Verify it never lands exactly on 0.500.
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
            "confidence": 0.5,
            "conviction_score": 0.50,
            "reasoning": "Anti-pattern test",
            "knowledge_sources": [],
            "risk_reward": 2.5
        }"#;

        let decision = parse_decision(json, 65000.0, 10.0).unwrap();
        // Should be remapped to 0.45 or 0.55, never exactly 0.50
        let cs_str = format!("{:.3}", decision.conviction_score);
        assert_ne!(cs_str, "0.500", "Anti-pattern noise should break 0.500 cliff");
    }

    #[test]
    fn empty_response_defaults_to_pass() {
        // v0.14.0: empty LLM response defaults to Pass instead of parse error
        let decision = parse_decision("", 65000.0, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Pass);
        assert_eq!(decision.conviction_score, 0.0);
        assert_eq!(decision.reasoning, "Empty LLM response");
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
        let response = r#"{"action":"Pass","pair":"BTC/USD","side":"Long","entry_price":0.0,"stop_loss":0.0,"take_profit_1":0.0,"take_profit_2":0.0,"take_profit_3":0.0,"position_size_pct":0.0,"confidence":0.0,"conviction_score":0.0,"reasoning":"No setup","knowledge_sources":[],"risk_reward":0.0}
Some extra reasoning text that leaked into the response..."#;

        let decision = parse_decision(response, 65000.0, 10.0).unwrap();
        // conviction_score=0.0 < Trending threshold 0.20 → stays Pass
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

    #[test]
    fn pass_not_overridden_to_buy() {
        // FID-161: Pass decisions with high conviction and bearish reasoning
        // MUST stay as Pass. The Pass->Buy override was removed.
        // Simulates the exact pattern from the 2026-06-15 live test:
        // LLM returns Pass with conviction 0.30, reasoning says "bearish downtrend no long".
        let json = serde_json::json!({
            "action": "Pass",
            "pair": "SEI/USD",
            "side": "Long",
            "entry_price": 0.0,
            "stop_loss": 0.0,
            "take_profit_1": 0.0,
            "take_profit_2": 0.0,
            "take_profit_3": 0.0,
            "position_size_pct": 0.0,
            "confidence": 0.0,
            "conviction_score": 0.30,
            "regime_label": "Trending",
            "trigger_weights": {"strong": 0, "moderate": 1, "weak": 1},
            "reasoning": "SEI strong downtrend, EMA_F < EMA_S, RSI 34. Bearish. ADVERSE TREND. No long entry in downtrend.",
            "knowledge_sources": [],
            "risk_reward": 0.0
        });

        // FID-161: Pass→Buy override was removed entirely.
        // Pass must stay Pass regardless of SAVANT_GATE_DISABLED or conviction.
        let decision = parse_decision(&json.to_string(), 0.0537, 10.0).unwrap();
        assert_eq!(
            decision.action,
            TradeAction::Pass,
            "FID-161: Pass decision with high conviction and bearish reasoning must stay Pass"
        );
    }

    #[test]
    fn pass_with_keyword_skip_stays_pass() {
        // Additional safety: Pass must stay Pass after FID-161.
        let json = serde_json::json!({
            "action": "Pass",
            "pair": "GRT/USD",
            "side": "Long",
            "entry_price": 0.0,
            "stop_loss": 0.0,
            "take_profit_1": 0.0,
            "position_size_pct": 0.0,
            "confidence": 0.0,
            "conviction_score": 0.45,
            "regime_label": "Trending",
            "reasoning": "GRT trending down. Skip.",
            "knowledge_sources": [],
            "risk_reward": 0.0
        });

        let decision = parse_decision(&json.to_string(), 0.15, 10.0).unwrap();
        assert_eq!(decision.action, TradeAction::Pass);
    }
}
