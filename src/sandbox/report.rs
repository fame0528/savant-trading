//! Report card generation for sandbox runs.

use serde::{Deserialize, Serialize};

use crate::sandbox::harness::SandboxSummary;

/// Category-level performance breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryReport {
    pub category: String,
    pub scenario_count: usize,
    pub passed: usize,
    pub failed: usize,
    pub avg_score: f64,
    pub avg_tier_2: f64,
    pub avg_tier_3: f64,
}

/// Full report card for a sandbox run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportCard {
    pub timestamp: String,
    pub total_scenarios: usize,
    pub passed: usize,
    pub failed: usize,
    pub compliance_ratio: f64,
    pub avg_score: f64,
    pub avg_tier_1_pass_rate: f64,
    pub avg_tier_2_score: f64,
    pub avg_tier_3_score: f64,
    pub best_category: String,
    pub worst_category: String,
    pub categories: Vec<CategoryReport>,
    pub critical_failures: Vec<FailureEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEntry {
    pub scenario_id: String,
    pub scenario_name: String,
    pub category: String,
    pub score: f64,
    pub reason: String,
}

/// Generate a full report card from a sandbox summary.
pub fn generate_report_card(summary: &SandboxSummary) -> ReportCard {
    let mut categories: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();
    let mut category_t2: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();
    let mut category_t3: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();
    let mut category_passed: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for r in &summary.results {
        categories
            .entry(r.category.clone())
            .or_default()
            .push(r.grade.total_score);
        category_t2
            .entry(r.category.clone())
            .or_default()
            .push(r.grade.tier_2_rr_score);
        category_t3
            .entry(r.category.clone())
            .or_default()
            .push(r.grade.tier_3_reasoning_score);
        if r.grade.total_score >= 0.6 {
            *category_passed.entry(r.category.clone()).or_insert(0) += 1;
        }
    }

    let category_reports: Vec<CategoryReport> = categories
        .iter()
        .map(|(cat, scores)| {
            let count = scores.len();
            let passed = category_passed.get(cat).copied().unwrap_or(0);
            let avg_t2 = category_t2
                .get(cat)
                .map(|v| v.iter().sum::<f64>() / v.len() as f64)
                .unwrap_or(0.0);
            let avg_t3 = category_t3
                .get(cat)
                .map(|v| v.iter().sum::<f64>() / v.len() as f64)
                .unwrap_or(0.0);
            CategoryReport {
                category: cat.clone(),
                scenario_count: count,
                passed,
                failed: count - passed,
                avg_score: scores.iter().sum::<f64>() / count as f64,
                avg_tier_2: avg_t2,
                avg_tier_3: avg_t3,
            }
        })
        .collect();

    let critical_failures: Vec<FailureEntry> = summary
        .results
        .iter()
        .filter(|r| r.grade.total_score < 0.4)
        .map(|r| FailureEntry {
            scenario_id: r.scenario_id.clone(),
            scenario_name: r.scenario_name.clone(),
            category: r.category.clone(),
            score: r.grade.total_score,
            reason: r
                .grade
                .tier_1_reason
                .clone()
                .unwrap_or_else(|| format!("Low score: {:.2}", r.grade.total_score)),
        })
        .collect();

    ReportCard {
        timestamp: chrono::Utc::now().to_rfc3339(),
        total_scenarios: summary.total_scenarios,
        passed: summary.passed,
        failed: summary.failed,
        compliance_ratio: summary.avg_tier_1_pass_rate,
        avg_score: summary.avg_score,
        avg_tier_1_pass_rate: summary.avg_tier_1_pass_rate,
        avg_tier_2_score: summary.avg_tier_2_score,
        avg_tier_3_score: summary.avg_tier_3_score,
        best_category: summary.best_category.clone(),
        worst_category: summary.worst_category.clone(),
        categories: category_reports,
        critical_failures,
    }
}

/// Format report card as markdown for vault output.
pub fn format_report_markdown(report: &ReportCard) -> String {
    let mut md = String::new();

    md.push_str(&format!(
        "# Sandbox Report — {}\n\n",
        &report.timestamp[..10]
    ));

    md.push_str("## Overall\n\n");
    md.push_str(&format!(
        "| Metric | Value |\n|--------|-------|\n| Compliance Ratio | {:.0}% |\n| Average Score | {:.2} |\n| Passed | {} / {} |\n| Failed | {} / {} |\n\n",
        report.compliance_ratio * 100.0,
        report.avg_score,
        report.passed, report.total_scenarios,
        report.failed, report.total_scenarios,
    ));

    md.push_str("## Tier Breakdown\n\n");
    md.push_str(&format!(
        "| Tier | Score |\n|------|-------|\n| Compliance | {:.0}% |\n| R:R Score | {:.2} |\n| Reasoning | {:.2} |\n\n",
        report.avg_tier_1_pass_rate * 100.0,
        report.avg_tier_2_score,
        report.avg_tier_3_score,
    ));

    md.push_str("## Category Performance\n\n");
    md.push_str("| Category | Scenarios | Passed | Avg Score |\n");
    md.push_str("|----------|-----------|--------|----------|\n");
    for cat in &report.categories {
        md.push_str(&format!(
            "| {} | {} | {} | {:.2} |\n",
            cat.category, cat.scenario_count, cat.passed, cat.avg_score
        ));
    }
    md.push('\n');

    if !report.critical_failures.is_empty() {
        md.push_str("## Critical Failures\n\n");
        md.push_str("| ID | Scenario | Category | Score | Reason |\n");
        md.push_str("|----|----------|----------|-------|--------|\n");
        for f in &report.critical_failures {
            md.push_str(&format!(
                "| {} | {} | {} | {:.2} | {} |\n",
                f.scenario_id, f.scenario_name, f.category, f.score, f.reason
            ));
        }
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::grader::Grade;
    use crate::sandbox::harness::ScenarioResult;

    fn make_result(id: &str, category: &str, score: f64) -> ScenarioResult {
        ScenarioResult {
            scenario_id: id.into(),
            scenario_name: format!("Test {}", id),
            category: category.into(),
            difficulty: "Medium".into(),
            action_taken: "Buy".into(),
            grade: Grade {
                tier_1_compliance: score >= 0.5,
                tier_1_reason: None,
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
    fn test_generate_report_card() {
        let results = vec![
            make_result("T1", "Trend Bull", 0.8),
            make_result("T2", "Trend Bull", 0.7),
            make_result("T3", "Range", 0.3),
        ];
        let summary = SandboxSummary::from_results(results);
        let report = generate_report_card(&summary);
        assert_eq!(report.total_scenarios, 3);
        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn test_format_markdown() {
        let results = vec![make_result("T1", "Trend", 0.8)];
        let summary = SandboxSummary::from_results(results);
        let report = generate_report_card(&summary);
        let md = format_report_markdown(&report);
        assert!(md.contains("Sandbox Report"));
        assert!(md.contains("Compliance Ratio"));
    }
}
