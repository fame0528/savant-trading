//! GEPA-style feedback loop — SOUL.md self-correction engine.
//!
//! Identifies systemic failures across scenarios, generates specific
//! SOUL.md mutation proposals via Teacher LLM, validates against sandbox,
//! and manages version control with automated rollback.

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tracing::{info, warn};

use crate::sandbox::harness::{SandboxSummary, ScenarioResult};

/// A proposed mutation to SOUL.md based on sandbox failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulMutation {
    pub mutation_id: String,
    pub timestamp: String,
    pub trigger: String,
    pub category: String,
    pub current_rule: String,
    pub proposed_change: String,
    pub expected_improvement: String,
    pub scenarios_affected: Vec<String>,
}

/// Analysis of systemic failures across a sandbox run.
#[derive(Debug, Clone)]
pub struct FailureAnalysis {
    /// Category with worst performance
    pub worst_category: String,
    /// Average score in worst category
    pub worst_score: f64,
    /// Specific rules being violated
    pub violated_rules: Vec<RuleViolation>,
    /// Patterns detected across failures
    pub patterns: Vec<FailurePattern>,
}

#[derive(Debug, Clone)]
pub struct RuleViolation {
    pub rule: String,
    pub violation_count: usize,
    pub scenarios: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FailurePattern {
    pub pattern: String,
    pub frequency: usize,
    pub suggestion: String,
}

/// A versioned snapshot of SOUL.md with performance scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulVersion {
    pub version_id: String,
    pub timestamp: String,
    pub content_hash: String,
    pub parent_version_id: Option<String>,
    pub pareto_scores: serde_json::Value,
    pub status: String, // active, rejected, rolled_back
    pub trigger_reason: String,
}

/// Extract mutable sections from SOUL.md content.
///
/// Returns the content between <!-- MUTABLE --> and <!-- END MUTABLE --> markers.
/// Returns None if markers are not found (fail-safe: no mutations allowed).
pub fn extract_mutable_sections(soul_content: &str) -> Option<String> {
    let start_marker = "<!-- MUTABLE:";
    let end_marker = "<!-- END MUTABLE -->";

    let start_pos = soul_content.find(start_marker)?;
    let content_start = soul_content[start_pos..].find('>')? + start_pos + 1;
    let end_pos = soul_content.find(end_marker)?;

    if content_start >= end_pos {
        return None;
    }

    Some(soul_content[content_start..end_pos].trim().to_string())
}

/// Replace mutable sections in SOUL.md with mutated content.
///
/// Preserves everything before <!-- MUTABLE --> and after <!-- END MUTABLE -->.
/// Returns None if markers are not found.
pub fn apply_mutation_to_soul(soul_content: &str, new_mutable_content: &str) -> Option<String> {
    let start_marker = "<!-- MUTABLE:";
    let end_marker = "<!-- END MUTABLE -->";

    let start_pos = soul_content.find(start_marker)?;
    let tag_end = soul_content[start_pos..].find('>')? + start_pos + 1;
    let end_pos = soul_content.find(end_marker)?;
    let end_of_end = end_pos + end_marker.len();

    let mut result = String::new();
    result.push_str(&soul_content[..tag_end]);
    result.push('\n');
    result.push_str(new_mutable_content);
    result.push('\n');
    result.push_str(&soul_content[end_of_end..]);

    Some(result)
}

/// Initialize the soul_versions table in the database.
pub async fn init_soul_versions_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS soul_versions (
            version_id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            parent_version_id TEXT,
            pareto_scores TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            trigger_reason TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Store a new SOUL.md version in the database.
pub async fn store_soul_version(
    pool: &SqlitePool,
    version: &SoulVersion,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO soul_versions (version_id, timestamp, content_hash, parent_version_id, pareto_scores, status, trigger_reason)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&version.version_id)
    .bind(&version.timestamp)
    .bind(&version.content_hash)
    .bind(&version.parent_version_id)
    .bind(serde_json::to_string(&version.pareto_scores).unwrap_or_default())
    .bind(&version.status)
    .bind(&version.trigger_reason)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the currently active SOUL.md version.
pub async fn get_active_soul_version(
    pool: &SqlitePool,
) -> Result<Option<SoulVersion>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT version_id, timestamp, content_hash, parent_version_id, pareto_scores, status, trigger_reason
        FROM soul_versions
        WHERE status = 'active'
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| SoulVersion {
        version_id: r.get("version_id"),
        timestamp: r.get("timestamp"),
        content_hash: r.get("content_hash"),
        parent_version_id: r.get("parent_version_id"),
        pareto_scores: serde_json::from_str(r.get::<String, _>("pareto_scores").as_str())
            .unwrap_or(serde_json::Value::Null),
        status: r.get("status"),
        trigger_reason: r.get("trigger_reason"),
    }))
}

/// Rollback to the previous stable SOUL.md version.
///
/// Marks the current active version as 'rolled_back' and reactivates
/// the parent version.
pub async fn rollback_soul_version(pool: &SqlitePool) -> Result<Option<SoulVersion>, sqlx::Error> {
    let current = get_active_soul_version(pool).await?;
    if let Some(current) = current {
        // Mark current as rolled back
        sqlx::query("UPDATE soul_versions SET status = 'rolled_back' WHERE version_id = ?")
            .bind(&current.version_id)
            .execute(pool)
            .await?;

        // Reactivate parent
        if let Some(parent_id) = &current.parent_version_id {
            sqlx::query("UPDATE soul_versions SET status = 'active' WHERE version_id = ?")
                .bind(parent_id)
                .execute(pool)
                .await?;

            let parent = sqlx::query(
                "SELECT version_id, timestamp, content_hash, parent_version_id, pareto_scores, status, trigger_reason FROM soul_versions WHERE version_id = ?"
            )
            .bind(parent_id)
            .fetch_optional(pool)
            .await?;

            if let Some(r) = parent {
                info!(
                    "Rolled back SOUL.md from {} to {}",
                    current.version_id, parent_id
                );
                return Ok(Some(SoulVersion {
                    version_id: r.get("version_id"),
                    timestamp: r.get("timestamp"),
                    content_hash: r.get("content_hash"),
                    parent_version_id: r.get("parent_version_id"),
                    pareto_scores: serde_json::from_str(
                        r.get::<String, _>("pareto_scores").as_str(),
                    )
                    .unwrap_or(serde_json::Value::Null),
                    status: r.get("status"),
                    trigger_reason: r.get("trigger_reason"),
                }));
            }
        }

        warn!("No parent version found for rollback");
    }
    Ok(None)
}

/// Build a Teacher LLM prompt for critiquing a failed trade.
///
/// The Teacher Agent uses the same mimo v2.5 pro model ("model empathy")
/// to analyze the reasoning trace and generate a corrective heuristic.
pub fn build_teacher_critique_prompt(
    reasoning: &str,
    outcome: &str,
    market_context: &str,
    category: &str,
) -> String {
    format!(
        r#"You are a meta-analyst reviewing a failed trading decision by an autonomous agent.

## Failed Trade Context
- Category: {category}
- Agent's Reasoning: {reasoning}
- Outcome: {outcome}
- Market Context: {market_context}

## Task
Diagnose the analytical blindspot. Why did the agent's thesis fail?
Then formulate a SINGLE-SENTENCE heuristic rule that acts as a mental model to prevent this specific error in the future.

Requirements:
- The rule must be generalizable (not specific to one ticker)
- The rule must be actionable (the agent can check it before trading)
- The rule must NOT contradict risk management fundamentals
- Output ONLY the single-sentence heuristic, nothing else

Example output:
"When ADX > 30 and RSI > 70, avoid long entries even if breakout confirmation exists — the trend is exhausted and mean reversion probability exceeds 60%."

Your heuristic:"#,
        category = category,
        reasoning = reasoning,
        outcome = outcome,
        market_context = market_context,
    )
}

/// Build a GEPA mutation prompt for the Teacher LLM.
///
/// Given the current mutable SOUL.md sections and a failure analysis,
/// the Teacher proposes a targeted textual mutation.
pub fn build_gepa_mutation_prompt(
    mutable_sections: &str,
    failure_analysis: &str,
    worst_category: &str,
) -> String {
    format!(
        r#"You are a prompt engineer optimizing a trading agent's system prompt.

## Current Mutable Sections of SOUL.md
```
{mutable_sections}
```

## Failure Analysis
{failure_analysis}

## Worst Performing Category
{worst_category}

## Task
Generate a TARGETED mutation to the mutable sections above that addresses the failure.
The mutation must:
1. Be minimal — change only what's necessary to fix the specific failure
2. Preserve existing rules that are working well
3. Not introduce contradictions with immutable risk constraints
4. Be specific and actionable (not vague platitudes)

Output ONLY the modified mutable sections. Do not include the markers.
Do not explain your changes — the diff IS the explanation.

Modified mutable sections:"#,
        mutable_sections = mutable_sections,
        failure_analysis = failure_analysis,
        worst_category = worst_category,
    )
}

/// Analyze a sandbox summary to identify systemic failures.
pub fn analyze_failures(summary: &SandboxSummary) -> FailureAnalysis {
    let mut violated_rules: Vec<RuleViolation> = Vec::new();
    let mut patterns: Vec<FailurePattern> = Vec::new();

    // Collect all failed scenarios
    let failures: Vec<&ScenarioResult> = summary
        .results
        .iter()
        .filter(|r| r.grade.total_score < 0.4)
        .collect();

    // Analyze Tier 1 failures (compliance violations)
    let tier_1_failures: Vec<&ScenarioResult> = summary
        .results
        .iter()
        .filter(|r| !r.grade.tier_1_compliance)
        .collect();

    if !tier_1_failures.is_empty() {
        // Group by failure reason
        let mut reason_counts: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for f in &tier_1_failures {
            if let Some(ref reason) = f.grade.tier_1_reason {
                reason_counts
                    .entry(reason.clone())
                    .or_default()
                    .push(f.scenario_id.clone());
            }
        }

        for (reason, scenarios) in &reason_counts {
            violated_rules.push(RuleViolation {
                rule: reason.clone(),
                violation_count: scenarios.len(),
                scenarios: scenarios.clone(),
            });
        }
    }

    // Analyze Tier 2 failures (poor R:R)
    let poor_rr: Vec<&ScenarioResult> = summary
        .results
        .iter()
        .filter(|r| r.grade.tier_2_rr_score < 0.3 && r.action_taken != "Hold")
        .collect();

    if poor_rr.len() > 3 {
        patterns.push(FailurePattern {
            pattern: "Consistently poor R:R ratios".to_string(),
            frequency: poor_rr.len(),
            suggestion: "Increase minimum R:R requirement or improve entry timing".to_string(),
        });
    }

    // Analyze Tier 3 failures (poor reasoning)
    let poor_reasoning: Vec<&ScenarioResult> = summary
        .results
        .iter()
        .filter(|r| r.grade.tier_3_reasoning_score < 0.3)
        .collect();

    if poor_reasoning.len() > 3 {
        patterns.push(FailurePattern {
            pattern: "Reasoning quality consistently low".to_string(),
            frequency: poor_reasoning.len(),
            suggestion: "Add more specific reasoning requirements to SOUL.md".to_string(),
        });
    }

    // Analyze category-specific failures
    let mut category_failures: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for f in &failures {
        *category_failures.entry(f.category.clone()).or_insert(0) += 1;
    }

    for (category, count) in &category_failures {
        if *count >= 3 {
            patterns.push(FailurePattern {
                pattern: format!("Systemic failure in {} scenarios", category),
                frequency: *count,
                suggestion: format!(
                    "Add specific guidance for {} conditions to SOUL.md",
                    category
                ),
            });
        }
    }

    // Analyze session-specific failures
    let mut session_failures: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for f in &failures {
        if f.category == "Session" {
            *session_failures.entry(f.scenario_name.clone()).or_insert(0) += 1;
        }
    }

    if !session_failures.is_empty() {
        patterns.push(FailurePattern {
            pattern: "Session-specific handling gaps".to_string(),
            frequency: session_failures.values().sum(),
            suggestion: "Add session-specific decision rules to SOUL.md".to_string(),
        });
    }

    FailureAnalysis {
        worst_category: summary.worst_category.clone(),
        worst_score: summary
            .results
            .iter()
            .filter(|r| r.category == summary.worst_category)
            .map(|r| r.grade.total_score)
            .sum::<f64>()
            / summary
                .results
                .iter()
                .filter(|r| r.category == summary.worst_category)
                .count()
                .max(1) as f64,
        violated_rules,
        patterns,
    }
}

/// Generate SOUL.md mutation proposals from failure analysis.
pub fn generate_mutations(analysis: &FailureAnalysis) -> Vec<SoulMutation> {
    let mut mutations = Vec::new();
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Generate mutations for violated rules
    for (i, violation) in analysis.violated_rules.iter().enumerate() {
        mutations.push(SoulMutation {
            mutation_id: format!("MUT-{}-{}", i + 1, chrono::Utc::now().timestamp()),
            timestamp: timestamp.clone(),
            trigger: format!("Rule violated in {} scenarios", violation.violation_count),
            category: "Compliance".to_string(),
            current_rule: violation.rule.clone(),
            proposed_change: format!("Add explicit enforcement for: {}", violation.rule),
            expected_improvement: format!("Reduce {} violations to 0", violation.violation_count),
            scenarios_affected: violation.scenarios.clone(),
        });
    }

    // Generate mutations for patterns
    for (i, pattern) in analysis.patterns.iter().enumerate() {
        mutations.push(SoulMutation {
            mutation_id: format!("MUT-P{}-{}", i + 1, chrono::Utc::now().timestamp()),
            timestamp: timestamp.clone(),
            trigger: pattern.pattern.clone(),
            category: "Pattern".to_string(),
            current_rule: "Current SOUL.md guidance".to_string(),
            proposed_change: pattern.suggestion.clone(),
            expected_improvement: format!("Address {} failure instances", pattern.frequency),
            scenarios_affected: Vec::new(),
        });
    }

    mutations
}

/// Format mutations as a report string.
pub fn format_mutations_report(mutations: &[SoulMutation]) -> String {
    if mutations.is_empty() {
        return "No mutations proposed — all scenarios passed.".to_string();
    }

    let mut report = String::new();
    report.push_str("═══════════════════════════════════════════\n");
    report.push_str("       SOUL.md MUTATION PROPOSALS\n");
    report.push_str("═══════════════════════════════════════════\n\n");

    for mutation in mutations {
        report.push_str(&format!(
            "[{}] {}\n",
            mutation.mutation_id, mutation.trigger
        ));
        report.push_str(&format!("  Category: {}\n", mutation.category));
        report.push_str(&format!("  Current:  {}\n", mutation.current_rule));
        report.push_str(&format!("  Proposed: {}\n", mutation.proposed_change));
        report.push_str(&format!("  Expected: {}\n", mutation.expected_improvement));
        if !mutation.scenarios_affected.is_empty() {
            report.push_str(&format!(
                "  Scenarios: {}\n",
                mutation.scenarios_affected.join(", ")
            ));
        }
        report.push('\n');
    }

    report.push_str("═══════════════════════════════════════════\n");
    report
}

/// Four-Factor Causal Attribution (Fabio Valentina / Pradeep Bondi).
///
/// When a trade loses, classify the failure into one of four categories:
/// - Setup: Wrong setup selection (bad entry criteria)
/// - Process: Execution error (wrong size, wrong entry, missed stop)
/// - Market: Market not in our favor (correlation, regime shift)
/// - Trader: Emotional/revenge trading (FOMO, tilt)
///
/// This attribution is injected into the memory context for the next 5 episodes,
/// allowing the agent to self-correct based on WHY it failed, not just THAT it failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAttribution {
    pub episode_id: String,
    pub factor: LossFactor,
    pub explanation: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LossFactor {
    /// Wrong setup selection — the entry criteria themselves were flawed
    Setup,
    /// Execution error — right setup, wrong execution (size, timing, stop)
    Process,
    /// Market factor — regime shift, correlation spike, black swan
    Market,
    /// Emotional — revenge trading, FOMO, tilt
    Trader,
}

impl std::fmt::Display for LossFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LossFactor::Setup => write!(f, "Setup"),
            LossFactor::Process => write!(f, "Process"),
            LossFactor::Market => write!(f, "Market"),
            LossFactor::Trader => write!(f, "Trader"),
        }
    }
}

impl CausalAttribution {
    pub fn format_for_memory(&self) -> String {
        format!(
            "[{}] {}: {}",
            self.factor, self.episode_id, self.explanation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::grader::Grade;
    use crate::sandbox::harness::ScenarioResult;

    fn make_result(id: &str, category: &str, score: f64, tier_1: bool) -> ScenarioResult {
        ScenarioResult {
            scenario_id: id.into(),
            scenario_name: format!("Test {}", id),
            category: category.into(),
            difficulty: "Medium".into(),
            action_taken: "Buy".into(),
            grade: Grade {
                tier_1_compliance: tier_1,
                tier_1_reason: if !tier_1 {
                    Some("Missing stop loss".into())
                } else {
                    None
                },
                tier_2_rr_score: score,
                tier_2_details: String::new(),
                tier_3_reasoning_score: score,
                tier_3_rationale: String::new(),
                total_score: score,
            },
            latency_ms: 100,
        }
    }

    #[test]
    fn analyze_no_failures() {
        let results = vec![
            make_result("T1", "Trend", 0.8, true),
            make_result("T2", "Trend", 0.7, true),
        ];
        let summary = SandboxSummary::from_results(results);
        let analysis = analyze_failures(&summary);
        assert!(analysis.violated_rules.is_empty());
    }

    #[test]
    fn analyze_with_failures() {
        let results = vec![
            make_result("T1", "Trend", 0.2, false),
            make_result("T2", "Trend", 0.1, false),
            make_result("T3", "Trend", 0.15, false),
            make_result("T4", "Range", 0.8, true),
        ];
        let summary = SandboxSummary::from_results(results);
        let analysis = analyze_failures(&summary);
        assert!(!analysis.violated_rules.is_empty());
    }

    #[test]
    fn generate_mutations_from_analysis() {
        let results = vec![
            make_result("T1", "Trend", 0.2, false),
            make_result("T2", "Trend", 0.1, false),
            make_result("T3", "Trend", 0.15, false),
            make_result("T4", "Trend", 0.1, false),
        ];
        let summary = SandboxSummary::from_results(results);
        let analysis = analyze_failures(&summary);
        let mutations = generate_mutations(&analysis);
        assert!(!mutations.is_empty());
    }

    #[test]
    fn format_empty_mutations() {
        let report = format_mutations_report(&[]);
        assert!(report.contains("No mutations"));
    }
}
