//! Execution harness — parallel scenario evaluation with rate limiting.

use crate::sandbox::grader::{self, Grade};
use crate::sandbox::scenarios::Scenario;

/// Result of evaluating a single scenario.
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub scenario_name: String,
    pub category: String,
    pub difficulty: String,
    pub action_taken: String,
    pub grade: Grade,
    pub latency_ms: u64,
}

/// Run a single scenario through the grading pipeline.
/// This is the deterministic grading path (Tier 1 + Tier 2).
/// Tier 3 (LLM-as-judge) requires the actual LLM response.
pub fn grade_scenario_deterministic(
    scenario: &Scenario,
    action: &str,
    entry_price: f64,
    stop_loss: f64,
    take_profit_1: f64,
    confidence: f64,
    reasoning: &str,
) -> Grade {
    // Tier 1: Binary compliance
    let (tier_1_pass, tier_1_reason) = grader::tier_1_compliance(
        action,
        stop_loss,
        entry_price,
        confidence,
        reasoning,
        &scenario.expected_action,
    );

    // Tier 2: R:R scoring
    let (tier_2_score, tier_2_details) = grader::tier_2_rr_score(
        entry_price,
        stop_loss,
        take_profit_1,
        action,
        &scenario.expected_action,
    );

    // Tier 3: Reasoning quality (deterministic heuristic)
    let (tier_3_score, tier_3_rationale) =
        grader::tier_3_reasoning_score(reasoning, &scenario.expected_action);

    // Total
    let total = grader::calculate_total(tier_1_pass, tier_2_score, tier_3_score);

    Grade {
        tier_1_compliance: tier_1_pass,
        tier_1_reason,
        tier_2_rr_score: tier_2_score,
        tier_2_details,
        tier_3_reasoning_score: tier_3_score,
        tier_3_rationale,
        total_score: total,
    }
}

/// Summary of a full sandbox run.
#[derive(Debug, Clone)]
pub struct SandboxSummary {
    pub total_scenarios: usize,
    pub passed: usize,
    pub failed: usize,
    pub avg_score: f64,
    pub weighted_avg_score: f64,
    pub avg_tier_1_pass_rate: f64,
    pub avg_tier_2_score: f64,
    pub avg_tier_3_score: f64,
    pub worst_category: String,
    pub best_category: String,
    pub results: Vec<ScenarioResult>,
}

/// Generate an always-hold benchmark — grades every scenario as "Hold" to prove
/// the AI adds value vs doing nothing.
pub fn always_hold_benchmark(scenarios: &[super::scenarios::Scenario]) -> Vec<ScenarioResult> {
    scenarios
        .iter()
        .map(|s| {
            let grade = super::harness::grade_scenario_deterministic(
                s,
                "Hold",
                0.0,
                0.0,
                0.0,
                0.0,
                "Always-hold benchmark — no trade taken",
            );
            ScenarioResult {
                scenario_id: s.id.clone(),
                scenario_name: s.name.clone(),
                category: s.category.clone(),
                difficulty: s.difficulty.clone(),
                action_taken: "Hold".into(),
                grade,
                latency_ms: 0,
            }
        })
        .collect()
}

/// Check if score dropped >threshold vs last N runs.
/// Returns Some(warning_message) if regression detected.
pub fn check_regression(
    current_score: f64,
    recent_scores: &[f64],
    threshold_pct: f64,
) -> Option<String> {
    if recent_scores.is_empty() {
        return None;
    }
    let avg: f64 = recent_scores.iter().sum::<f64>() / recent_scores.len() as f64;
    if avg > 0.0 {
        let drop_pct = (avg - current_score) / avg;
        if drop_pct > threshold_pct {
            return Some(format!(
                "REGRESSION WARNING: Score dropped {:.1}% vs rolling avg ({:.2} → {:.2})",
                drop_pct * 100.0,
                avg,
                current_score
            ));
        }
    }
    None
}
pub fn difficulty_weight(difficulty: &str) -> f64 {
    match difficulty {
        "Easy" => 1.0,
        "Medium" => 1.5,
        "Hard" => 2.0,
        "Extreme" => 3.0,
        _ => 1.0,
    }
}

/// α-trimmed mean: sort scores, drop k from each end, average the rest.
/// With n=5 and k=1 (20% trim), drops min and max, averages middle 3.
/// Resistant to outlier hallucinations and lucky runs.
pub fn alpha_trimmed_mean(scores: &[f64], trim_pct: f64) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    if scores.len() <= 2 {
        return scores.iter().sum::<f64>() / scores.len() as f64;
    }

    let mut sorted = scores.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let k = ((scores.len() as f64) * trim_pct).floor() as usize;
    let k = k.min((scores.len() - 1) / 2);

    if k == 0 {
        return sorted.iter().sum::<f64>() / sorted.len() as f64;
    }

    let trimmed = &sorted[k..sorted.len() - k];
    trimmed.iter().sum::<f64>() / trimmed.len() as f64
}

/// Aggregate multiple run results using α-trimmed mean on scores.
/// Each scenario gets `runs_per_scenario` results, trimmed mean applied.
pub fn aggregate_multi_run(
    all_results: Vec<Vec<ScenarioResult>>,
    trim_pct: f64,
) -> Vec<ScenarioResult> {
    if all_results.is_empty() {
        return vec![];
    }
    if all_results.len() == 1 {
        return all_results.into_iter().next().unwrap_or_default();
    }

    let scenario_count = all_results[0].len();
    let mut aggregated = Vec::with_capacity(scenario_count);

    for i in 0..scenario_count {
        let scores: Vec<f64> = all_results
            .iter()
            .filter_map(|run| run.get(i))
            .map(|r| r.grade.total_score)
            .collect();

        let t1_scores: Vec<f64> = all_results
            .iter()
            .filter_map(|run| run.get(i))
            .map(|r| if r.grade.tier_1_compliance { 1.0 } else { 0.0 })
            .collect();

        let t2_scores: Vec<f64> = all_results
            .iter()
            .filter_map(|run| run.get(i))
            .map(|r| r.grade.tier_2_rr_score)
            .collect();

        let t3_scores: Vec<f64> = all_results
            .iter()
            .filter_map(|run| run.get(i))
            .map(|r| r.grade.tier_3_reasoning_score)
            .collect();

        let trimmed_total = alpha_trimmed_mean(&scores, trim_pct);
        let trimmed_t1 = alpha_trimmed_mean(&t1_scores, trim_pct);
        let trimmed_t2 = alpha_trimmed_mean(&t2_scores, trim_pct);
        let trimmed_t3 = alpha_trimmed_mean(&t3_scores, trim_pct);

        // Use the first run's metadata, replace scores with trimmed means
        let mut result = all_results[0][i].clone();
        result.grade.total_score = trimmed_total;
        result.grade.tier_1_compliance = trimmed_t1 >= 0.5;
        result.grade.tier_2_rr_score = trimmed_t2;
        result.grade.tier_3_reasoning_score = trimmed_t3;

        aggregated.push(result);
    }

    aggregated
}

impl SandboxSummary {
    /// Calculate summary from results.
    pub fn from_results(results: Vec<ScenarioResult>) -> Self {
        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| r.grade.total_score >= 0.6)
            .count();
        let failed = total - passed;

        let avg_score = if total > 0 {
            results.iter().map(|r| r.grade.total_score).sum::<f64>() / total as f64
        } else {
            0.0
        };

        let avg_tier_1 = if total > 0 {
            results.iter().filter(|r| r.grade.tier_1_compliance).count() as f64 / total as f64
        } else {
            0.0
        };

        let avg_tier_2 = if total > 0 {
            results.iter().map(|r| r.grade.tier_2_rr_score).sum::<f64>() / total as f64
        } else {
            0.0
        };

        let avg_tier_3 = if total > 0 {
            results
                .iter()
                .map(|r| r.grade.tier_3_reasoning_score)
                .sum::<f64>()
                / total as f64
        } else {
            0.0
        };

        // Difficulty-weighted average score
        let (weighted_sum, weight_total) = results.iter().fold((0.0, 0.0), |(ws, wt), r| {
            let w = difficulty_weight(&r.difficulty);
            (ws + r.grade.total_score * w, wt + w)
        });
        let weighted_avg_score = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            0.0
        };

        // Find worst/best category
        let mut category_scores: std::collections::HashMap<String, Vec<f64>> =
            std::collections::HashMap::new();
        for r in &results {
            category_scores
                .entry(r.category.clone())
                .or_default()
                .push(r.grade.total_score);
        }

        let worst_category = category_scores
            .iter()
            .map(|(cat, scores)| {
                let avg = scores.iter().sum::<f64>() / scores.len() as f64;
                (cat.clone(), avg)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(cat, _)| cat)
            .unwrap_or_else(|| "N/A".into());

        let best_category = category_scores
            .iter()
            .map(|(cat, scores)| {
                let avg = scores.iter().sum::<f64>() / scores.len() as f64;
                (cat.clone(), avg)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(cat, _)| cat)
            .unwrap_or_else(|| "N/A".into());

        Self {
            total_scenarios: total,
            passed,
            failed,
            avg_score,
            weighted_avg_score,
            avg_tier_1_pass_rate: avg_tier_1,
            avg_tier_2_score: avg_tier_2,
            avg_tier_3_score: avg_tier_3,
            worst_category,
            best_category,
            results,
        }
    }

    /// Format as a report card string.
    pub fn report_card(&self) -> String {
        let mut report = String::new();
        report.push_str("═══════════════════════════════════════════\n");
        report.push_str("         SAVANT SANDBOX REPORT CARD\n");
        report.push_str("═══════════════════════════════════════════\n\n");

        report.push_str(&format!(
            "Overall Compliance Ratio: {:.0}%\n",
            self.avg_tier_1_pass_rate * 100.0
        ));
        report.push_str(&format!("Average Score: {:.2} / 1.00\n", self.avg_score));
        report.push_str(&format!(
            "Weighted Score: {:.2} / 1.00 (difficulty-weighted)\n",
            self.weighted_avg_score
        ));
        report.push_str(&format!(
            "Passed: {} / {}\n",
            self.passed, self.total_scenarios
        ));
        report.push_str(&format!(
            "Failed: {} / {}\n\n",
            self.failed, self.total_scenarios
        ));

        report.push_str("─── Tier Breakdown ──────────────────────\n");
        report.push_str(&format!(
            "Tier 1 (Compliance): {:.0}%\n",
            self.avg_tier_1_pass_rate * 100.0
        ));
        report.push_str(&format!(
            "Tier 2 (R:R Score):  {:.2}\n",
            self.avg_tier_2_score
        ));
        report.push_str(&format!(
            "Tier 3 (Reasoning):  {:.2}\n\n",
            self.avg_tier_3_score
        ));

        report.push_str("─── Category Analysis ───────────────────\n");
        report.push_str(&format!("Best:  {}\n", self.best_category));
        report.push_str(&format!("Worst: {}\n\n", self.worst_category));

        // Top failures
        let failures: Vec<&ScenarioResult> = self
            .results
            .iter()
            .filter(|r| r.grade.total_score < 0.4)
            .collect();

        if !failures.is_empty() {
            report.push_str("─── Critical Failures ───────────────────\n");
            for f in failures.iter().take(10) {
                report.push_str(&format!(
                    "  {} ({}) — Score: {:.2} — {}\n",
                    f.scenario_name,
                    f.scenario_id,
                    f.grade.total_score,
                    f.grade.tier_1_reason.as_deref().unwrap_or("Low score")
                ));
            }
        }

        report.push_str("\n═══════════════════════════════════════════\n");
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::scenarios::load_all_scenarios;

    #[test]
    fn grade_hold_scenario() {
        let scenarios = load_all_scenarios();
        let scenario = &scenarios[0]; // TRD-001 — expected "Buy (High Conviction)"

        let grade = grade_scenario_deterministic(
            scenario,
            "Hold",
            0.0,
            0.0,
            0.0,
            0.0,
            "No clear setup in current conditions",
        );

        // Hold when Buy expected = compliance failure now
        assert!(!grade.tier_1_compliance);
        assert_eq!(grade.tier_2_rr_score, 0.0); // Zero for missed trade
    }

    #[test]
    fn grade_buy_with_good_rr() {
        let scenarios = load_all_scenarios();
        let scenario = &scenarios[0]; // TRD-001

        let grade = grade_scenario_deterministic(
            scenario,
            "Buy",
            100.0,
            95.0,
            110.0,
            0.75,
            "BTC trending above EMA21 with volume breakout in US session. Support at $95, R:R 2:1.",
        );

        assert!(grade.tier_1_compliance);
        assert!(grade.tier_2_rr_score >= 0.6);
        assert!(grade.tier_3_reasoning_score >= 0.4);
        assert!(grade.total_score > 0.0);
    }

    #[test]
    fn summary_from_results() {
        let scenarios = load_all_scenarios();
        let mut results = Vec::new();

        for scenario in &scenarios[..5] {
            let grade =
                grade_scenario_deterministic(scenario, "Hold", 0.0, 0.0, 0.0, 0.0, "No setup");
            results.push(ScenarioResult {
                scenario_id: scenario.id.clone(),
                scenario_name: scenario.name.clone(),
                category: scenario.category.clone(),
                difficulty: scenario.difficulty.clone(),
                action_taken: "Hold".into(),
                grade,
                latency_ms: 100,
            });
        }

        let summary = SandboxSummary::from_results(results);
        assert_eq!(summary.total_scenarios, 5);
        assert!(summary.avg_tier_1_pass_rate > 0.0);
    }

    #[test]
    fn report_card_format() {
        let scenarios = load_all_scenarios();
        let mut results = Vec::new();

        for scenario in &scenarios[..3] {
            let grade =
                grade_scenario_deterministic(scenario, "Hold", 0.0, 0.0, 0.0, 0.0, "No setup");
            results.push(ScenarioResult {
                scenario_id: scenario.id.clone(),
                scenario_name: scenario.name.clone(),
                category: scenario.category.clone(),
                difficulty: scenario.difficulty.clone(),
                action_taken: "Hold".into(),
                grade,
                latency_ms: 100,
            });
        }

        let summary = SandboxSummary::from_results(results);
        let report = summary.report_card();
        assert!(report.contains("REPORT CARD"));
        assert!(report.contains("Compliance Ratio"));
    }
}
