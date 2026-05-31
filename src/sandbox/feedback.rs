//! GEPA-style feedback loop — SOUL.md self-correction engine.
//!
//! Identifies systemic failures across scenarios, generates specific
//! SOUL.md mutation proposals, and tracks improvement across versions.

use serde::{Deserialize, Serialize};

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
