//! Agent orchestrator — main decision loop with autonomy level control.
//!
//! Coordinates the LLM provider, context builder, knowledge base, and prompt composer
//! to produce trade decisions on each tick.

use tracing::{info, warn};

use crate::agent::context_builder::{self, FullContext};
use crate::agent::decision_parser::{self, TradeAction, TradeDecision};
use crate::agent::knowledge::KnowledgeBase;
use crate::agent::prompts::PromptComposer;
use crate::agent::provider::{LlmConfig, LlmProvider, Message};

/// Autonomy level for the AI agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomyLevel {
    /// Log decisions only — no execution
    Suggest,
    /// Log decisions and wait for user confirmation
    Confirm,
    /// Execute decisions automatically
    Autonomous,
}

/// Configuration for the agent orchestrator.
pub struct AgentConfig {
    pub autonomy_level: AutonomyLevel,
    pub max_decisions_per_hour: u32,
    pub knowledge_token_budget: usize,
    pub price_tolerance_pct: f64,
    pub max_retries: u32,
}

/// The agent orchestrator — coordinates all AI components.
pub struct AgentOrchestrator {
    provider: LlmProvider,
    knowledge_base: KnowledgeBase,
    composer: PromptComposer,
    config: AgentConfig,
    decisions_this_hour: u32,
    consecutive_failures: u32,
    fallback_active: bool,
}

/// Result of an agent evaluation.
pub enum AgentResult {
    /// AI produced a valid decision
    Decision(TradeDecision),
    /// AI decided to hold — no action needed
    Hold,
    /// AI unavailable — use fallback (rule-based strategies)
    Fallback,
    /// Decision logged for confirmation (Confirm mode)
    PendingConfirmation(TradeDecision),
    /// Error occurred
    Error(String),
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator.
    pub fn new(
        llm_config: LlmConfig,
        agent_config: AgentConfig,
        knowledge_base: KnowledgeBase,
        composer: PromptComposer,
    ) -> Self {
        Self {
            provider: LlmProvider::new(llm_config),
            knowledge_base,
            composer,
            config: agent_config,
            decisions_this_hour: 0,
            consecutive_failures: 0,
            fallback_active: false,
        }
    }

    /// Evaluate the current market context and produce a decision.
    pub async fn evaluate(&mut self, ctx: &FullContext<'_>) -> AgentResult {
        // Check fallback mode
        if self.fallback_active {
            warn!("Agent in fallback mode — using rule-based strategies");
            return AgentResult::Fallback;
        }

        // Build context
        let (system_prompt, user_message) = context_builder::build_context(
            ctx,
            &self.knowledge_base,
            &self.composer,
            self.config.knowledge_token_budget,
        );

        info!(
            "ai_context | prompt_chars={} | knowledge_budget={} | pair={} | regime={:?}",
            system_prompt.len(),
            self.config.knowledge_token_budget,
            ctx.pair,
            ctx.regime
        );

        // Call LLM with retries
        let mut last_error = String::new();
        for attempt in 0..self.config.max_retries {
            let messages = vec![Message {
                role: "user".to_string(),
                content: user_message.clone(),
            }];

            match self.provider.chat(&system_prompt, &messages).await {
                Ok(response) => {
                    // Parse decision
                    let current_price = ctx.candles.last().map(|c| c.close).unwrap_or(0.0);

                    match decision_parser::parse_decision(
                        &response,
                        current_price,
                        self.config.price_tolerance_pct,
                    ) {
                        Ok(decision) => {
                            self.decisions_this_hour += 1;
                            self.consecutive_failures = 0;

                            return match decision.action {
                                TradeAction::Hold => {
                                    info!(
                                        "Agent hold: {} — {} (confidence: {:.0}%)",
                                        decision.pair,
                                        decision.reasoning,
                                        decision.confidence * 100.0
                                    );
                                    AgentResult::Decision(decision)
                                }
                                _ => match self.config.autonomy_level {
                                    AutonomyLevel::Suggest => {
                                        info!(
                                            "Agent suggestion: {:?} {} {} @ {:.2} (confidence: {:.0}%)",
                                            decision.action, decision.pair, decision.side,
                                            decision.entry_price, decision.confidence * 100.0
                                        );
                                        AgentResult::Decision(decision)
                                    }
                                    AutonomyLevel::Confirm => {
                                        AgentResult::PendingConfirmation(decision)
                                    }
                                    AutonomyLevel::Autonomous => {
                                        info!(
                                            "Agent decision: {:?} {} {} @ {:.2} (confidence: {:.0}%)",
                                            decision.action, decision.pair, decision.side,
                                            decision.entry_price, decision.confidence * 100.0
                                        );
                                        AgentResult::Decision(decision)
                                    }
                                },
                            };
                        }
                        Err(e) => {
                            last_error = format!("Parse error (attempt {}): {}", attempt + 1, e);
                            warn!("{}", last_error);
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("LLM error (attempt {}): {}", attempt + 1, e);
                    warn!("{}", last_error);
                }
            }

            // Exponential backoff
            if attempt < self.config.max_retries - 1 {
                let delay_ms = 1000 * 2u64.pow(attempt);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }

        // All retries failed
        self.consecutive_failures += 1;
        if self.consecutive_failures >= 3 {
            warn!(
                "Agent failed {} consecutive times — activating fallback",
                self.consecutive_failures
            );
            self.fallback_active = true;
        }

        AgentResult::Error(last_error)
    }

    /// Reset the hourly decision counter (call at the top of each hour).
    pub fn reset_hourly_counter(&mut self) {
        self.decisions_this_hour = 0;
    }

    /// Deactivate fallback mode (call after LLM becomes available again).
    pub fn deactivate_fallback(&mut self) {
        self.fallback_active = false;
        self.consecutive_failures = 0;
        info!("Agent fallback deactivated — AI brain re-engaged");
    }

    /// Whether the agent is in fallback mode.
    pub fn is_fallback(&self) -> bool {
        self.fallback_active
    }

    pub fn knowledge_base(&self) -> &KnowledgeBase {
        &self.knowledge_base
    }

    pub fn composer(&self) -> &PromptComposer {
        &self.composer
    }

    pub fn provider_clone(&self) -> LlmProvider {
        LlmProvider::new(self.provider.config_clone())
    }
}
