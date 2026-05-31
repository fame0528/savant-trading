//! Knowledge unit types and knowledge base for transcript-derived trading knowledge.
//!
//! Knowledge is loaded at runtime from JSON files in the `knowledge/` directory.
//! Units are selected based on market conditions, topics, and tags, then injected
//! into the AI agent's system prompt.

use serde::{Deserialize, Serialize};

/// A discrete unit of trading knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeUnit {
    pub id: String,
    pub source: String,
    pub topic: KnowledgeTopic,
    pub conditions: Vec<MarketCondition>,
    pub content: String,
    pub priority: u8,
    /// Granular tags for precise matching (setup_type, timeframe, trigger, etc.)
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Topic categories for knowledge units — one per trading function.
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
    PriceAction,
    MarketRegime,
    CryptoNative,
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
    LiquidityExpansion,
    LiquidityContraction,
    MvrvExtreme,
    SoprReset,
    OIDivergence,
    WyckoffSpring,
    DeltaDivergence,
}

/// The knowledge base — holds all loaded knowledge units and provides
/// condition-based selection with tag-aware scoring.
#[derive(Debug, Clone)]
pub struct KnowledgeBase {
    units: Vec<KnowledgeUnit>,
}

impl KnowledgeBase {
    pub fn empty() -> Self {
        Self { units: Vec::new() }
    }

    pub fn new(units: Vec<KnowledgeUnit>) -> Self {
        Self { units }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let units: Vec<KnowledgeUnit> = serde_json::from_str(json)?;
        Ok(Self { units })
    }

    /// Select knowledge units matching the given conditions, scored and sorted.
    ///
    /// Scoring: conditions_match × 3 + priority × 2 + tag_bonus
    /// `context_tags` are optional tags from the current market context
    /// (e.g., ["breakout", "fomc", "bull-strong"]) for finer matching.
    pub fn select(
        &self,
        conditions: &[MarketCondition],
        token_budget: usize,
    ) -> Vec<&KnowledgeUnit> {
        self.select_with_tags(conditions, &[], token_budget)
    }

    /// Select with both conditions and context tags for precise matching.
    pub fn select_with_tags(
        &self,
        conditions: &[MarketCondition],
        context_tags: &[String],
        token_budget: usize,
    ) -> Vec<&KnowledgeUnit> {
        let mut scored: Vec<(f64, &KnowledgeUnit)> = self
            .units
            .iter()
            .filter(|unit| unit.conditions.iter().any(|c| conditions.contains(c)))
            .map(|unit| {
                let condition_score = unit
                    .conditions
                    .iter()
                    .filter(|c| conditions.contains(c))
                    .count() as f64
                    * 3.0;
                let priority_score = unit.priority as f64 * 2.0;
                let tag_score = if context_tags.is_empty() {
                    0.0
                } else {
                    unit.tags
                        .iter()
                        .filter(|t| context_tags.contains(t))
                        .count() as f64
                };
                let total = condition_score + priority_score + tag_score;
                (total, unit)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut selected = Vec::new();
        let mut used_tokens = 0;
        for (_score, unit) in scored {
            let unit_tokens = unit.content.len().div_ceil(4);
            if used_tokens + unit_tokens <= token_budget {
                used_tokens += unit_tokens;
                selected.push(unit);
            }
        }

        selected
    }

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
        let mut used_tokens = 0;
        for unit in matching {
            let unit_tokens = unit.content.len().div_ceil(4);
            if used_tokens + unit_tokens <= token_budget {
                used_tokens += unit_tokens;
                selected.push(unit);
            }
        }

        selected
    }

    pub fn all(&self) -> &[KnowledgeUnit] {
        &self.units
    }

    pub fn len(&self) -> usize {
        self.units.len()
    }

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
            tags: vec![],
        }
    }

    fn sample_unit_with_tags(
        id: &str,
        priority: u8,
        conditions: Vec<MarketCondition>,
        tags: Vec<&str>,
    ) -> KnowledgeUnit {
        KnowledgeUnit {
            id: id.to_string(),
            source: "test".to_string(),
            topic: KnowledgeTopic::TechnicalAnalysis,
            conditions,
            content: format!("Knowledge from {}", id),
            priority,
            tags: tags.into_iter().map(String::from).collect(),
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
        assert_eq!(selected[0].id, "c");
        assert_eq!(selected[1].id, "a");
    }

    #[test]
    fn select_respects_token_budget() {
        let base = KnowledgeBase::new(vec![
            sample_unit("a", 5, vec![MarketCondition::Trending]),
            sample_unit("b", 4, vec![MarketCondition::Trending]),
            sample_unit("c", 3, vec![MarketCondition::Trending]),
        ]);

        // Each unit content is ~16 chars = ~4 tokens. Budget of 10 tokens fits 2.
        let selected = base.select(&[MarketCondition::Trending], 10);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_with_tags_boosts_matching() {
        let base = KnowledgeBase::new(vec![
            sample_unit_with_tags("generic", 5, vec![MarketCondition::Trending], vec![]),
            sample_unit_with_tags(
                "specific",
                5,
                vec![MarketCondition::Trending],
                vec!["breakout", "fomc"],
            ),
        ]);

        let context_tags = vec!["breakout".to_string(), "fomc".to_string()];
        let selected = base.select_with_tags(&[MarketCondition::Trending], &context_tags, 10000);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].id, "specific");
    }

    #[test]
    fn from_json_parses_with_tags() {
        let json = r#"[
            {
                "id": "test-001",
                "source": "test",
                "topic": "TechnicalAnalysis",
                "conditions": ["Trending"],
                "content": "Test content",
                "priority": 5,
                "tags": ["breakout", "intraday"]
            }
        ]"#;

        let base = KnowledgeBase::from_json(json).unwrap();
        assert_eq!(base.len(), 1);
        assert_eq!(base.all()[0].tags, vec!["breakout", "intraday"]);
    }

    #[test]
    fn from_json_parses_without_tags() {
        let json = r#"[
            {
                "id": "test-002",
                "source": "test",
                "topic": "OrderFlow",
                "conditions": ["Trending"],
                "content": "Legacy content",
                "priority": 3
            }
        ]"#;

        let base = KnowledgeBase::from_json(json).unwrap();
        assert_eq!(base.len(), 1);
        assert!(base.all()[0].tags.is_empty());
    }
}
