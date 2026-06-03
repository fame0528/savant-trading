//! Knowledge unit types and knowledge base for transcript-derived trading knowledge.
//!
//! Knowledge is loaded at runtime from JSON files in the `knowledge/` directory.
//! Units are selected based on market conditions, topics, and tags, then injected
//! into the AI agent's system prompt.

use serde::{Deserialize, Serialize};

/// Maximum number of knowledge units selected per prompt.
/// Prevents token bloat while keeping MMR diversity.
const MAX_SELECTED_UNITS: usize = 20;

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
    /// Empirical utility score — tracks how well this unit correlates with
    /// successful trades. Adjusted by knowledge lifecycle batch job.
    /// 1.0 = neutral, > 1.0 = correlates with wins, < 1.0 = correlates with losses.
    #[serde(default = "default_utility_score")]
    pub utility_score: f64,
}

fn default_utility_score() -> f64 {
    1.0
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
    TradingSystems,
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
    Oversold,
    Overbought,
    StrongTrend,
    WeakTrend,
    VolumeExpansion,
    TrendAlignment,
    Crypto24h,
    TradingSystems,
    VolumeProfile,
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

    /// Select with both conditions and context tags using MMR (Maximal Marginal Relevance).
    ///
    /// MMR balances relevance vs diversity: high-scoring units that are similar to
    /// already-selected units get penalized, preventing echo chambers.
    /// λ=0.5 balances relevance and diversity equally.
    pub fn select_with_tags(
        &self,
        conditions: &[MarketCondition],
        context_tags: &[String],
        token_budget: usize,
    ) -> Vec<&KnowledgeUnit> {
        // Score all candidates
        let mut candidates: Vec<(f64, usize)> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.conditions.iter().any(|c| conditions.contains(c)))
            .map(|(idx, unit)| {
                let condition_score = unit
                    .conditions
                    .iter()
                    .filter(|c| conditions.contains(c))
                    .count() as f64
                    * 2.0;
                let priority_score = unit.priority as f64;
                let tag_score = if context_tags.is_empty() {
                    0.0
                } else {
                    unit.tags
                        .iter()
                        .filter(|t| context_tags.contains(t))
                        .count() as f64
                        * 3.0
                };
                (condition_score + priority_score + tag_score, idx)
            })
            .map(|(base_score, idx)| {
                // Apply utility_score: units that correlate with wins get promoted,
                // units that correlate with losses get suppressed.
                let utility_mult = 1.0 + self.units[idx].utility_score.max(0.01).log2();
                (base_score * utility_mult, idx)
            })
            .collect();

        // Sort by initial score descending
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let lambda = 0.5f64;
        let mut selected_indices: Vec<usize> = Vec::new();
        let mut selected: Vec<&KnowledgeUnit> = Vec::new();
        let mut used_tokens = 0;

        while !candidates.is_empty() {
            // Find the candidate with highest MMR score
            let mut best_mmr = f64::NEG_INFINITY;
            let mut best_pos = 0;

            for (pos, &(rel_score, idx)) in candidates.iter().enumerate() {
                // Diversity penalty: max similarity to any already-selected unit
                let max_sim = if selected_indices.is_empty() {
                    0.0
                } else {
                    selected_indices
                        .iter()
                        .map(|&sel_idx| {
                            topic_tag_similarity(&self.units[idx], &self.units[sel_idx])
                        })
                        .fold(0.0f64, f64::max)
                };

                let mmr = lambda * rel_score - (1.0 - lambda) * max_sim;
                if mmr > best_mmr {
                    best_mmr = mmr;
                    best_pos = pos;
                }
            }

            let (_, idx) = candidates.remove(best_pos);
            let unit = &self.units[idx];
            let unit_tokens = unit.content.len().div_ceil(4);

            if used_tokens + unit_tokens <= token_budget && selected.len() < MAX_SELECTED_UNITS {
                used_tokens += unit_tokens;
                selected_indices.push(idx);
                selected.push(unit);
            } else {
                break;
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

    /// Get a mutable reference to all units (for utility score updates).
    pub fn units_mut(&mut self) -> &mut Vec<KnowledgeUnit> {
        &mut self.units
    }

    /// Get a unit by ID.
    pub fn get_by_id(&self, id: &str) -> Option<&KnowledgeUnit> {
        self.units.iter().find(|u| u.id == id)
    }

    /// Save utility scores to a JSON sidecar file.
    /// Only saves id → utility_score mapping, not full content.
    pub fn save_utility_scores(&self, path: &std::path::Path) -> std::io::Result<()> {
        let scores: std::collections::HashMap<&str, f64> = self
            .units
            .iter()
            .map(|u| (u.id.as_str(), u.utility_score))
            .collect();
        let json = serde_json::to_string_pretty(&scores)?;
        std::fs::write(path, json)
    }

    /// Load utility scores from a JSON sidecar file and apply to units.
    pub fn load_utility_scores(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let json = std::fs::read_to_string(path)?;
        let scores: std::collections::HashMap<String, f64> = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        for unit in &mut self.units {
            if let Some(&score) = scores.get(&unit.id) {
                unit.utility_score = score;
            }
        }
        Ok(())
    }
}

/// Similarity between two knowledge units based on topic and tag overlap.
/// Returns 0.0 (completely different) to 1.0 (identical).
fn topic_tag_similarity(a: &KnowledgeUnit, b: &KnowledgeUnit) -> f64 {
    let topic_match = if a.topic == b.topic { 1.0 } else { 0.0 };

    let tag_overlap = if a.tags.is_empty() && b.tags.is_empty() {
        0.0
    } else {
        let common = a.tags.iter().filter(|t| b.tags.contains(t)).count();
        let total = (a.tags.len() + b.tags.len()).max(1);
        (common * 2) as f64 / total as f64
    };

    // Weighted: topic match is strong signal, tag overlap refines
    0.6 * topic_match + 0.4 * tag_overlap
}

/// Adjust utility_score for knowledge units based on episode outcomes.
///
/// For each episode: if the agent won, boost utility of units that were in the
/// prompt. If it lost, suppress them. Learning rate controls the magnitude.
pub fn update_utility_scores(
    knowledge: &mut KnowledgeBase,
    episodes: &[(Vec<String>, bool)], // (knowledge_unit_ids, is_win)
    learning_rate: f64,
) -> u32 {
    let mut updates = 0u32;
    for (unit_ids, is_win) in episodes {
        for unit_id in unit_ids {
            if let Some(unit) = knowledge.units.iter_mut().find(|u| u.id == *unit_id) {
                let delta = if *is_win {
                    learning_rate
                } else {
                    -learning_rate * 0.5 // Penalize losses less than reward wins
                };
                unit.utility_score = (unit.utility_score + delta).clamp(0.1, 5.0);
                updates += 1;
            }
        }
    }
    updates
}

/// Load all knowledge JSON files from a directory and return a KnowledgeBase.
pub fn load_knowledge_from_dir(dir: &std::path::Path) -> KnowledgeBase {
    use std::fs;
    let mut all_units = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && !path.to_string_lossy().contains("_backup")
                && path.file_name().and_then(|n| n.to_str()) != Some("package.json")
            {
                if let Ok(content) = fs::read_to_string(&path) {
                    match KnowledgeBase::from_json(&content) {
                        Ok(base) => {
                            all_units.extend(base.units);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    tracing::info!(
        "Loaded {} knowledge units from {}",
        all_units.len(),
        dir.display()
    );
    KnowledgeBase::new(all_units)
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
            utility_score: 1.0,
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
            utility_score: 1.0,
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

    #[test]
    fn utility_score_affects_ranking() {
        let base = KnowledgeBase::new(vec![
            KnowledgeUnit {
                id: "a".into(),
                source: "test".into(),
                topic: KnowledgeTopic::TechnicalAnalysis,
                conditions: vec![MarketCondition::Trending],
                content: "Unit A".into(),
                priority: 3,
                tags: vec![],
                utility_score: 2.0, // high utility
            },
            KnowledgeUnit {
                id: "b".into(),
                source: "test".into(),
                topic: KnowledgeTopic::TechnicalAnalysis,
                conditions: vec![MarketCondition::Trending],
                content: "Unit B".into(),
                priority: 3,
                tags: vec![],
                utility_score: 0.5, // low utility
            },
        ]);
        let selected = base.select(&[MarketCondition::Trending], 10000);
        assert_eq!(selected[0].id, "a"); // higher utility wins
    }
}
