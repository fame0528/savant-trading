//! Knowledge unit types and knowledge base for transcript-derived trading knowledge.
//!
//! Transcripts are processed into discrete `KnowledgeUnit`s that can be
//! dynamically selected based on current market conditions and injected
//! into the AI agent's system prompt.

use serde::{Deserialize, Serialize};

/// A discrete unit of trading knowledge extracted from a transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeUnit {
    /// Unique identifier (e.g., "fabio-amt-001")
    pub id: String,
    /// Source transcript filename (e.g., "scalping-fabio-valentina-amt")
    pub source: String,
    /// Topic category for filtering
    pub topic: KnowledgeTopic,
    /// Market conditions under which this knowledge is relevant
    pub conditions: Vec<MarketCondition>,
    /// The actual knowledge content to inject into the prompt
    pub content: String,
    /// Priority 1-5 (higher = more likely to be selected when token budget is tight)
    pub priority: u8,
}

/// Topic categories for knowledge units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeTopic {
    OrderFlow,
    VolumeProfile,
    RiskManagement,
    Sentiment,
    MacroAnalysis,
    TechnicalAnalysis,
    Psychology,
    Execution,
    RegimeDetection,
    Backtesting,
    AiStrategy,
}

/// Market conditions that trigger knowledge injection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketCondition {
    Trending,
    Ranging,
    HighVolatility,
    LowVolatility,
    ExtremeFear,
    ExtremeGreed,
    BreakingNews,
    SessionOpen,
    SessionClose,
    AltSeason,
    BtcDominant,
    HalvingProximity,
    FomcDate,
    FundingRateExtreme,
    LiquidationCluster,
}

/// The knowledge base — holds all loaded knowledge units and provides
/// condition-based selection.
#[derive(Debug, Clone)]
pub struct KnowledgeBase {
    units: Vec<KnowledgeUnit>,
}

impl KnowledgeBase {
    /// Create an empty knowledge base.
    pub fn empty() -> Self {
        Self { units: Vec::new() }
    }

    /// Create a knowledge base from a vector of units.
    pub fn new(units: Vec<KnowledgeUnit>) -> Self {
        Self { units }
    }

    /// Load knowledge units from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let units: Vec<KnowledgeUnit> = serde_json::from_str(json)?;
        Ok(Self { units })
    }

    /// Select knowledge units matching the given conditions, sorted by priority
    /// (highest first), fitting within the token budget.
    ///
    /// `token_budget` is an approximate character count limit (tokens ≈ chars / 4).
    pub fn select(
        &self,
        conditions: &[MarketCondition],
        token_budget: usize,
    ) -> Vec<&KnowledgeUnit> {
        let mut matching: Vec<&KnowledgeUnit> = self
            .units
            .iter()
            .filter(|unit| unit.conditions.iter().any(|c| conditions.contains(c)))
            .collect();

        // Sort by priority descending
        matching.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Fill within token budget
        let mut selected = Vec::new();
        let mut used_chars = 0;
        for unit in matching {
            let unit_chars = unit.content.len();
            if used_chars + unit_chars <= token_budget {
                used_chars += unit_chars;
                selected.push(unit);
            }
        }

        selected
    }

    /// Select knowledge units by topic, regardless of market conditions.
    pub fn select_by_topic(
        &self,
        topic: KnowledgeTopic,
        token_budget: usize,
    ) -> Vec<&KnowledgeUnit> {
        let mut matching: Vec<&KnowledgeUnit> = self
            .units
            .iter()
            .filter(|unit| unit.topic == topic)
            .collect();

        matching.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut selected = Vec::new();
        let mut used_chars = 0;
        for unit in matching {
            let unit_chars = unit.content.len();
            if used_chars + unit_chars <= token_budget {
                used_chars += unit_chars;
                selected.push(unit);
            }
        }

        selected
    }

    /// Return all units (for inspection/debugging).
    pub fn all(&self) -> &[KnowledgeUnit] {
        &self.units
    }

    /// Number of loaded units.
    pub fn len(&self) -> usize {
        self.units.len()
    }

    /// Whether the knowledge base is empty.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_unit(id: &str, priority: u8, conditions: Vec<MarketCondition>) -> KnowledgeUnit {
        KnowledgeUnit {
            id: id.to_string(),
            source: "test".to_string(),
            topic: KnowledgeTopic::OrderFlow,
            conditions,
            content: format!("Knowledge from {}", id),
            priority,
        }
    }

    #[test]
    fn select_filters_by_condition() {
        let base = KnowledgeBase::new(vec![
            sample_unit("a", 3, vec![MarketCondition::Trending]),
            sample_unit("b", 5, vec![MarketCondition::Ranging]),
            sample_unit("c", 4, vec![MarketCondition::Trending]),
        ]);

        let selected = base.select(&[MarketCondition::Trending], 10000);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].id, "c"); // higher priority first
        assert_eq!(selected[1].id, "a");
    }

    #[test]
    fn select_respects_token_budget() {
        let base = KnowledgeBase::new(vec![
            sample_unit("a", 5, vec![MarketCondition::Trending]),
            sample_unit("b", 4, vec![MarketCondition::Trending]),
            sample_unit("c", 3, vec![MarketCondition::Trending]),
        ]);

        // Each unit content is ~18 chars, budget fits 2
        let selected = base.select(&[MarketCondition::Trending], 40);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn from_json_parses_correctly() {
        let json = r#"[
            {
                "id": "test-001",
                "source": "test-transcript",
                "topic": "OrderFlow",
                "conditions": ["Trending", "HighVolatility"],
                "content": "Test knowledge content",
                "priority": 5
            }
        ]"#;

        let base = KnowledgeBase::from_json(json).unwrap();
        assert_eq!(base.len(), 1);
        assert_eq!(base.all()[0].id, "test-001");
    }
}
