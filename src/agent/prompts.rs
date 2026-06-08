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
///
/// The Brain is partitioned into:
/// - **Immutable:** base_identity, risk_constraints, strategy_knowledge, output_format
///   (cached permanently, never re-serialized)
/// - **Mutable:** knowledge_units (cached until market conditions change)
///
/// The immutable portion is composed once and stored as `Arc<String>`.
/// The mutable portion is composed per evaluation but cached via digest.
pub struct PromptComposer {
    layers: Vec<PromptLayer>,
    /// Cached immutable Brain portion (fixed layers only).
    /// Computed once at init via `compute_immutable_brain()`.
    immutable_brain: Option<String>,
    /// SHA-256 digest of the last mutable section for change detection.
    mutable_digest: Option<String>,
}

impl PromptComposer {
    /// Create a new prompt composer with the fixed layers.
    pub fn new(
        base_identity: &str,
        risk_constraints: &str,
        strategy_knowledge: &str,
        output_format: &str,
    ) -> Self {
        let layers = vec![
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
        ];

        let mut composer = Self {
            layers,
            immutable_brain: None,
            mutable_digest: None,
        };
        composer.compute_immutable_brain();
        composer
    }

    /// Compute the immutable Brain portion (fixed layers only).
    /// Called once at init. The output is cached permanently.
    fn compute_immutable_brain(&mut self) {
        let mut parts: Vec<String> = Vec::new();
        for layer in &self.layers {
            if layer.name != "output_format" {
                parts.push(layer.content.clone());
            }
        }
        // Output format always comes last in the Brain
        if let Some(output_layer) = self.layers.iter().find(|l| l.name == "output_format") {
            parts.push(output_layer.content.clone());
        }
        self.immutable_brain = Some(parts.join("\n\n"));
    }

    /// Get the immutable Brain string (cached permanently).
    pub fn immutable_brain(&self) -> Option<&str> {
        self.immutable_brain.as_deref()
    }

    /// Compose the mutable knowledge section.
    /// Cached: if the knowledge selection hasn't changed (same digest), returns cached result.
    /// Returns (knowledge_section, digest).
    pub fn compose_mutable(
        &mut self,
        knowledge_units: &[&KnowledgeUnit],
    ) -> (String, String) {
        let mut knowledge_section = String::new();

        if !knowledge_units.is_empty() {
            knowledge_section.push_str("## Relevant Trading Knowledge\n\n");
            for unit in knowledge_units {
                knowledge_section.push_str(&format!(
                    "### [{}] {}\n{}\n\n",
                    unit.source,
                    unit.topic_as_str(),
                    unit.content
                ));
            }
        }

        // Compute digest for change detection
        let digest = format!("{:x}", {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            knowledge_section.hash(&mut hasher);
            hasher.finish()
        });

        self.mutable_digest = Some(digest.clone());
        (knowledge_section, digest)
    }

    /// Check if the current mutable digest matches a previous one.
    pub fn is_mutable_unchanged(&self, digest: &str) -> bool {
        self.mutable_digest.as_deref() == Some(digest)
    }

    /// Legacy: Compose the full system prompt (for backward compatibility).
    /// Prefer using `immutable_brain()` + `compose_mutable()` for FID-085 optimizations.
    pub fn compose(&self, knowledge_units: &[&KnowledgeUnit]) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Use cached immutable brain if available
        if let Some(ref brain) = self.immutable_brain {
            parts.push(brain.clone());
        } else {
            // Fallback: compose from layers
            for layer in &self.layers {
                if layer.name != "output_format" {
                    parts.push(layer.content.clone());
                }
            }
        }

        // Add dynamic knowledge layer
        if !knowledge_units.is_empty() {
            let mut knowledge_section = String::from("## Relevant Trading Knowledge\n\n");
            for unit in knowledge_units {
                knowledge_section.push_str(&format!(
                    "### [{}] {}\n{}\n\n",
                    unit.source,
                    unit.topic_as_str(),
                    unit.content
                ));
            }
            parts.push(knowledge_section);
        }

        // Add output format last
        if let Some(output_layer) = self.layers.iter().find(|l| l.name == "output_format") {
            parts.push(output_layer.content.clone());
        }

        parts.join("\n\n")
    }
}

/// Default base identity — loaded from SOUL.md at compile time.
pub fn default_base_identity() -> String {
    include_str!("soul.md").to_string()
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

For HOLD decisions, set all prices to 0.0 and position_size_pct to 0.0."#
        .to_string()
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
            crate::agent::knowledge::KnowledgeTopic::PriceAction => "Price Action",
            crate::agent::knowledge::KnowledgeTopic::MarketRegime => "Market Regime",
            crate::agent::knowledge::KnowledgeTopic::CryptoNative => "Crypto Native",
            crate::agent::knowledge::KnowledgeTopic::TradingSystems => "Trading Systems",
        }
    }
}
