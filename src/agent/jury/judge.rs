//! Jury Judge — synthesizes all jury verdicts into a single TradeDecision.
//!
//! The Judge is a dedicated LLM call to Owl Alpha (the main model) that receives
//! all jury verdicts and produces the final decision. It weights verdicts by
//! evidence quality, resolves contradictions, and can override weak majority consensus.

use tracing::{debug, info, warn};

use crate::agent::decision_parser::{self, TradeAction};
use crate::agent::jury::{JuryJudgment, JuryResult};
use crate::agent::provider::{LlmProvider, Message};

/// System prompt for the Judge — loaded from jury_judge.md at compile time.
const JUDGE_SYSTEM_PROMPT: &str = include_str!("../prompts/jury_judge.md");

/// Errors from the Judge.
#[derive(Debug, thiserror::Error)]
pub enum JudgeError {
    #[error("LLM error: {0}")]
    Llm(#[from] crate::agent::provider::LlmError),
    #[error("Parse error: {0}")]
    Parse(#[from] decision_parser::ParseError),
    #[error("Empty jury — no verdicts to judge")]
    EmptyJury,
}

/// The Judge — synthesizes jury verdicts into a final TradeDecision.
pub struct JuryJudge {
    /// The main LLM provider (Owl Alpha).
    provider: LlmProvider,
    /// Price tolerance for decision validation.
    price_tolerance_pct: f64,
}

impl JuryJudge {
    /// Create a new Judge with the main LLM provider.
    pub fn new(provider: LlmProvider, price_tolerance_pct: f64) -> Self {
        Self {
            provider,
            price_tolerance_pct,
        }
    }

    /// Synthesize jury verdicts into a single JuryJudgment.
    ///
    /// Sends all verdicts + condensed summary to Owl Alpha, which produces
    /// a final TradeDecision. Falls back to majority vote if the Judge fails.
    pub async fn judge(
        &self,
        user_message: &str,
        jury_result: &JuryResult,
        current_price: f64,
    ) -> Result<JuryJudgment, JudgeError> {
        if jury_result.verdicts.is_empty() {
            return Err(JudgeError::EmptyJury);
        }

        let judge_message = self.build_judge_message(user_message, jury_result);

        info!(
            "Judge: synthesizing {} verdicts ({} failed, quorum={})",
            jury_result.verdicts.len(),
            jury_result.failed_count,
            jury_result.quorum_met
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: judge_message,
        }];

        // Call Owl Alpha (the main model) as the Judge
        match self.provider.chat(JUDGE_SYSTEM_PROMPT, &messages).await {
            Ok(response) => {
                match decision_parser::parse_decision(
                    &response,
                    current_price,
                    self.price_tolerance_pct,
                ) {
                    Ok(mut decision) => {
                        // FID-148d: Clamp hallucinated entry_price to current_price.
                        // Free models sometimes emit wildly wrong prices (e.g., 95 for BTC at 100K).
                        // Use a wide tolerance (20%) to only catch truly hallucinated prices,
                        // not legitimate limit entries near current price.
                        if decision.action == TradeAction::Buy || decision.action == TradeAction::Sell {
                            let max_deviation = current_price * 0.20; // 20% tolerance
                            if (decision.entry_price - current_price).abs() > max_deviation {
                                warn!(
                                    "Judge: hallucinated entry_price={:.4} vs current={:.4} (>20% deviation). Clamping.",
                                    decision.entry_price, current_price
                                );
                                // Preserve the original SL *distance* and recalculate relative to clamped entry
                                let sl_distance = (decision.stop_loss - decision.entry_price).abs();
                                decision.entry_price = current_price;
                                decision.stop_loss = if decision.action == TradeAction::Buy {
                                    current_price - sl_distance
                                } else {
                                    current_price + sl_distance
                                };
                            }
                        }

                        let consensus = self.calculate_consensus(jury_result);
                        let dissent = self.analyze_dissent(jury_result);

                        info!(
                            "Judge: {:?} {} (consensus: {:.0}%, dissent: {})",
                            decision.action,
                            decision.pair,
                            consensus * 100.0,
                            dissent
                        );

                        Ok(JuryJudgment {
                            decision,
                            consensus_strength: consensus,
                            dissent_analysis: dissent,
                            jury_size_used: jury_result.verdicts.len(),
                        })
                    }
                    Err(e) => {
                        // FID-181: demote parse failure to debug. Free models
                        // return malformed JSON occasionally. The fallback
                        // (majority vote) is the correct behavior — we just
                        // don't need to spam the log every time it happens.
                        debug!(
                            "Judge: parse failed ({}), falling back to majority vote",
                            e
                        );
                        Ok(self.fallback_majority_vote(jury_result, current_price))
                    }
                }
            }
            Err(e) => {
                // FID-181: same demotion. Judge LLM call failures are
                // already logged at the calling site. Don't double-log.
                debug!("Judge: LLM call failed ({}), falling back to majority vote", e);
                Ok(self.fallback_majority_vote(jury_result, current_price))
            }
        }
    }

    /// Build the Judge's user message with all verdicts + summary.
    fn build_judge_message(&self, user_message: &str, jury_result: &JuryResult) -> String {
        let mut msg = String::new();

        // Section 1: Jury Verdicts
        msg.push_str(&format!(
            "## Jury Verdicts ({} responses, {} failed)\n\n",
            jury_result.verdicts.len(),
            jury_result.failed_count
        ));

        for (i, v) in jury_result.verdicts.iter().enumerate() {
            msg.push_str(&format!(
                "### Jury Member {} (confidence: {:.0}%)\n",
                i + 1,
                v.confidence * 100.0
            ));
            msg.push_str(&format!("- Verdict: {}\n", v.verdict));
            msg.push_str(&format!("- Key Argument: {}\n", v.key_argument));
            if !v.risk_flag.is_empty() {
                msg.push_str(&format!("- Risk Flag: {}\n", v.risk_flag));
            }
            if let Some(eq) = v.evidence_quality {
                msg.push_str(&format!("- Evidence Quality: {:.0}/10\n", eq));
            }
            msg.push_str(&format!("- Reasoning: {}\n\n", v.reasoning));
        }

        // Section 2: Jury Summary
        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut hold_count = 0;
        let mut total_confidence = 0.0f64;

        for v in &jury_result.verdicts {
            match v.verdict.to_uppercase().as_str() {
                "BUY" => buy_count += 1,
                "SELL" => sell_count += 1,
                _ => hold_count += 1,
            }
            total_confidence += v.confidence;
        }
        let avg_confidence = if !jury_result.verdicts.is_empty() {
            total_confidence / jury_result.verdicts.len() as f64
        } else {
            0.0
        };

        msg.push_str("## Jury Summary\n");
        msg.push_str(&format!(
            "- BUY: {} | SELL: {} | HOLD: {}\n",
            buy_count, sell_count, hold_count
        ));
        msg.push_str(&format!(
            "- Average Confidence: {:.0}%\n",
            avg_confidence * 100.0
        ));
        msg.push_str(&format!(
            "- Failed Members: {}/{}\n\n",
            jury_result.failed_count,
            jury_result.verdicts.len() + jury_result.failed_count
        ));

        // Section 3: Original Market Data (condensed)
        msg.push_str("## Original Market Data\n");
        msg.push_str(user_message);
        msg.push_str("\n\n## Decision Required\n");
        msg.push_str("Synthesize the jury verdicts into your final TradeDecision JSON.\n");

        msg
    }

    /// Calculate consensus strength — how aligned the jury was (0.0-1.0).
    fn calculate_consensus(&self, jury_result: &JuryResult) -> f64 {
        if jury_result.verdicts.is_empty() {
            return 0.0;
        }

        let mut buy = 0usize;
        let mut sell = 0usize;
        let mut hold = 0usize;

        for v in &jury_result.verdicts {
            match v.verdict.to_uppercase().as_str() {
                "BUY" => buy += 1,
                "SELL" => sell += 1,
                _ => hold += 1,
            }
        }

        let total = jury_result.verdicts.len() as f64;
        let max_agreement = buy.max(sell).max(hold) as f64;
        max_agreement / total
    }

    /// Analyze disagreement patterns for the Judge's context.
    fn analyze_dissent(&self, jury_result: &JuryResult) -> String {
        let mut buy_conf = Vec::new();
        let mut sell_conf = Vec::new();
        let mut hold_conf = Vec::new();

        for v in &jury_result.verdicts {
            match v.verdict.to_uppercase().as_str() {
                "BUY" => buy_conf.push(v.confidence),
                "SELL" => sell_conf.push(v.confidence),
                _ => hold_conf.push(v.confidence),
            }
        }

        let mut parts = Vec::new();

        if !buy_conf.is_empty() {
            let avg = buy_conf.iter().sum::<f64>() / buy_conf.len() as f64;
            parts.push(format!("BUY({} @ {:.0}%)", buy_conf.len(), avg * 100.0));
        }
        if !sell_conf.is_empty() {
            let avg = sell_conf.iter().sum::<f64>() / sell_conf.len() as f64;
            parts.push(format!("SELL({} @ {:.0}%)", sell_conf.len(), avg * 100.0));
        }
        if !hold_conf.is_empty() {
            let avg = hold_conf.iter().sum::<f64>() / hold_conf.len() as f64;
            parts.push(format!("HOLD({} @ {:.0}%)", hold_conf.len(), avg * 100.0));
        }

        parts.join(", ")
    }

    /// Fallback: majority vote when the Judge LLM fails.
    fn fallback_majority_vote(
        &self,
        jury_result: &JuryResult,
        current_price: f64,
    ) -> JuryJudgment {
        let mut buy = 0usize;
        let mut sell = 0usize;
        let mut hold = 0usize;
        let mut total_confidence = 0.0f64;
        let mut best_reasoning = String::new();
        let mut best_confidence = 0.0f64;

        for v in &jury_result.verdicts {
            match v.verdict.to_uppercase().as_str() {
                "BUY" => buy += 1,
                "SELL" => sell += 1,
                _ => hold += 1,
            }
            total_confidence += v.confidence;
            if v.confidence > best_confidence {
                best_confidence = v.confidence;
                best_reasoning = v.reasoning.clone();
            }
        }

        let action = if buy > sell && buy > hold {
            TradeAction::Buy
        } else if sell > buy && sell > hold {
            TradeAction::Sell
        } else {
            TradeAction::Pass
        };

        let avg_confidence = if !jury_result.verdicts.is_empty() {
            total_confidence / jury_result.verdicts.len() as f64
        } else {
            0.0
        };

        let consensus = self.calculate_consensus(jury_result);

        warn!(
            "Judge fallback: majority vote → {:?} (BUY:{}, SELL:{}, HOLD:{}, consensus: {:.0}%)",
            action, buy, sell, hold, consensus * 100.0
        );

        JuryJudgment {
            decision: decision_parser::TradeDecision {
                action,
                pair: "BATCH".to_string(),
                side: crate::core::types::Side::Long,
                order_type: "MARKET".to_string(),
                entry_price: current_price,
                stop_loss: 0.0,
                take_profit_1: 0.0,
                take_profit_2: 0.0,
                take_profit_3: 0.0,
                position_size_pct: 0.0,
                confidence: avg_confidence,
                reasoning: format!(
                    "Jury majority vote (fallback): {} — {}",
                    jury_result.verdicts.len(),
                    best_reasoning
                ),
                knowledge_sources: vec![],
                risk_reward: 0.0,
                management_trigger_active: false,
                stop_distance_atr_multiple: 0.0,
                thesis_invalidated: false,
                opportunity_cost: String::new(),
                mandated_action: String::new(),
                mandated_stop_price: 0.0,
                would_initiate_new_long: None,
                // FID-126: default conviction fields (jury fallback path)
                conviction_score: 0.0,
                sizing_multiplier: 0.5,
                regime_label: decision_parser::RegimeLabel::default(),
                trigger_weights: decision_parser::TriggerWeights::default(),
                override_source: None,
            },
            consensus_strength: consensus,
            dissent_analysis: self.analyze_dissent(jury_result),
            jury_size_used: jury_result.verdicts.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::jury::{JuryResult, JuryVerdict};

    fn make_verdict(verdict: &str, confidence: f64, eq: Option<f64>) -> JuryVerdict {
        JuryVerdict {
            verdict: verdict.to_string(),
            confidence,
            key_argument: "test argument".to_string(),
            risk_flag: "test risk".to_string(),
            evidence_quality: eq,
            reasoning: "test reasoning".to_string(),
        }
    }

    #[test]
    fn consensus_unanimous_buy() {
        let judge = JuryJudge::new(
            LlmProvider::new(crate::agent::provider::LlmConfig::default()),
            10.0,
        );
        let result = JuryResult {
            verdicts: vec![
                make_verdict("BUY", 0.8, Some(8.0)),
                make_verdict("BUY", 0.7, Some(7.0)),
                make_verdict("BUY", 0.6, Some(6.0)),
            ],
            failed_count: 0,
            model_ids: vec![],
            total_latency_ms: 0,
            quorum_met: true,
        };
        let consensus = judge.calculate_consensus(&result);
        assert!((consensus - 1.0).abs() < 0.01);
    }

    #[test]
    fn consensus_split() {
        let judge = JuryJudge::new(
            LlmProvider::new(crate::agent::provider::LlmConfig::default()),
            10.0,
        );
        let result = JuryResult {
            verdicts: vec![
                make_verdict("BUY", 0.7, Some(7.0)),
                make_verdict("SELL", 0.6, Some(6.0)),
                make_verdict("BUY", 0.5, Some(5.0)),
                make_verdict("SELL", 0.4, Some(4.0)),
            ],
            failed_count: 0,
            model_ids: vec![],
            total_latency_ms: 0,
            quorum_met: true,
        };
        let consensus = judge.calculate_consensus(&result);
        assert!((consensus - 0.5).abs() < 0.01); // 2 BUY, 2 SELL → 50%
    }

    #[test]
    fn dissent_analysis_format() {
        let judge = JuryJudge::new(
            LlmProvider::new(crate::agent::provider::LlmConfig::default()),
            10.0,
        );
        let result = JuryResult {
            verdicts: vec![
                make_verdict("BUY", 0.8, Some(8.0)),
                make_verdict("BUY", 0.7, Some(7.0)),
                make_verdict("HOLD", 0.3, Some(3.0)),
            ],
            failed_count: 0,
            model_ids: vec![],
            total_latency_ms: 0,
            quorum_met: true,
        };
        let dissent = judge.analyze_dissent(&result);
        assert!(dissent.contains("BUY(2"));
        assert!(dissent.contains("HOLD(1"));
    }

    #[test]
    fn majority_vote_buy_wins() {
        let judge = JuryJudge::new(
            LlmProvider::new(crate::agent::provider::LlmConfig::default()),
            10.0,
        );
        let result = JuryResult {
            verdicts: vec![
                make_verdict("BUY", 0.8, Some(8.0)),
                make_verdict("BUY", 0.7, Some(7.0)),
                make_verdict("SELL", 0.6, Some(6.0)),
            ],
            failed_count: 0,
            model_ids: vec![],
            total_latency_ms: 0,
            quorum_met: true,
        };
        let judgment = judge.fallback_majority_vote(&result, 50000.0);
        assert_eq!(judgment.decision.action, TradeAction::Buy);
        assert!(judgment.consensus_strength > 0.5);
    }

    #[test]
    fn majority_vote_hold_wins_on_tie() {
        let judge = JuryJudge::new(
            LlmProvider::new(crate::agent::provider::LlmConfig::default()),
            10.0,
        );
        let result = JuryResult {
            verdicts: vec![
                make_verdict("BUY", 0.5, Some(5.0)),
                make_verdict("SELL", 0.5, Some(5.0)),
            ],
            failed_count: 0,
            model_ids: vec![],
            total_latency_ms: 0,
            quorum_met: true,
        };
        let judgment = judge.fallback_majority_vote(&result, 50000.0);
        assert_eq!(judgment.decision.action, TradeAction::Pass);
    }

    #[test]
    fn empty_jury_returns_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let judge = JuryJudge::new(
                LlmProvider::new(crate::agent::provider::LlmConfig::default()),
                10.0,
            );
            let result = JuryResult {
                verdicts: vec![],
                failed_count: 5,
                model_ids: vec![],
                total_latency_ms: 0,
                quorum_met: false,
            };
            let err = judge.judge("test", &result, 50000.0).await;
            assert!(matches!(err, Err(JudgeError::EmptyJury)));
        });
    }
}
