//! Modular system prompt composer.
//!
//! Assembles the system prompt from multiple layers:
//! 1. Base identity
//! 2. Risk constraints
//! 3. Strategy knowledge
//! 4. Transcript knowledge (dynamic)
//! 5. Output format

use crate::agent::knowledge::KnowledgeUnit;

/// Prompt layer content.
pub struct PromptLayer {
    pub name: String,
    pub content: String,
}

/// Composes the system prompt from fixed layers + dynamic knowledge.
pub struct PromptComposer {
    layers: Vec<PromptLayer>,
}

impl PromptComposer {
    /// Create a new prompt composer with the fixed layers.
    pub fn new(
        base_identity: &str,
        risk_constraints: &str,
        strategy_knowledge: &str,
        output_format: &str,
    ) -> Self {
        Self {
            layers: vec![
                PromptLayer {
                    name: "base_identity".to_string(),
                    content: base_identity.to_string(),
                },
                PromptLayer {
                    name: "risk_constraints".to_string(),
                    content: risk_constraints.to_string(),
                },
                PromptLayer {
                    name: "strategy_knowledge".to_string(),
                    content: strategy_knowledge.to_string(),
                },
                PromptLayer {
                    name: "output_format".to_string(),
                    content: output_format.to_string(),
                },
            ],
        }
    }

    /// Compose the full system prompt with selected knowledge units injected.
    pub fn compose(&self, knowledge_units: &[&KnowledgeUnit]) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Add fixed layers (base_identity, risk_constraints, strategy_knowledge)
        for layer in &self.layers {
            if layer.name != "output_format" {
                parts.push(layer.content.clone());
            }
        }

        // Add dynamic knowledge layer
        if !knowledge_units.is_empty() {
            let mut knowledge_section = String::from("## Relevant Trading Knowledge\n\n");
            for unit in knowledge_units {
                knowledge_section.push_str(&format!(
                    "### [{}] {}\n{}\n\n",
                    unit.source, unit.topic_as_str(), unit.content
                ));
            }
            parts.push(knowledge_section);
        }

        // Add output format last
        if let Some(output_layer) = self.layers.iter().find(|l| l.name == "output_format") {
            parts.push(output_layer.content.clone());
        }

        parts.join("\n\n---\n\n")
    }
}

/// Default base identity prompt.
pub fn default_base_identity() -> String {
    r#"You are Savant — an autonomous crypto trading agent operating on Kraken exchange.

## Core Principles
- You are a rigorous trading agent. You do not guess. You read data and make decisions.
- Every decision must be backed by data from the provided market context.
- You optimize for mathematical correctness, extreme robustness, and long-term maintainability.
- You never take positions you cannot justify with specific technical or fundamental reasoning.

## Operating Rules
- Always specify exact entry, stop-loss, and take-profit prices.
- Never risk more than the configured max risk per trade.
- Always provide a confidence score (0.0 to 1.0) based on setup quality.
- Always cite which knowledge sources informed your decision.
- If no high-quality setup exists, output a HOLD decision."#.to_string()
}

/// Default output format prompt.
pub fn default_output_format() -> String {
    r#"## Required Output Format

Respond with ONLY a JSON object (no markdown, no explanation before/after):

```json
{
    "action": "BUY" | "SELL" | "HOLD" | "CLOSE" | "ADJUST_STOP",
    "pair": "BTC/USD",
    "side": "Long" | "Short",
    "entry_price": 0.0,
    "stop_loss": 0.0,
    "take_profit_1": 0.0,
    "take_profit_2": 0.0,
    "take_profit_3": 0.0,
    "position_size_pct": 0.0,
    "confidence": 0.0,
    "reasoning": "Your reasoning here",
    "knowledge_sources": ["source-id-001"],
    "risk_reward": 0.0
}
```

For HOLD decisions, set all prices to 0.0 and position_size_pct to 0.0."#.to_string()
}

impl KnowledgeUnit {
    /// Get the topic as a human-readable string.
    pub fn topic_as_str(&self) -> &str {
        match self.topic {
            crate::agent::knowledge::KnowledgeTopic::OrderFlow => "Order Flow",
            crate::agent::knowledge::KnowledgeTopic::VolumeProfile => "Volume Profile",
            crate::agent::knowledge::KnowledgeTopic::RiskManagement => "Risk Management",
            crate::agent::knowledge::KnowledgeTopic::Sentiment => "Sentiment",
            crate::agent::knowledge::KnowledgeTopic::MacroAnalysis => "Macro Analysis",
            crate::agent::knowledge::KnowledgeTopic::TechnicalAnalysis => "Technical Analysis",
            crate::agent::knowledge::KnowledgeTopic::Psychology => "Psychology",
            crate::agent::knowledge::KnowledgeTopic::Execution => "Execution",
            crate::agent::knowledge::KnowledgeTopic::RegimeDetection => "Regime Detection",
            crate::agent::knowledge::KnowledgeTopic::Backtesting => "Backtesting",
            crate::agent::knowledge::KnowledgeTopic::AiStrategy => "AI Strategy",
        }
    }
}
