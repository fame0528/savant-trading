//! Run report generator — creates persistent, timestamped sandbox run logs.
//!
//! Each sandbox run generates a comprehensive markdown report at
//! `data/sandbox_reports/YYYY-MM-DD_HH-MM-SS.md` with full decision logs,
//! trade logs, wallet metrics, category breakdowns, and failure analysis.

use std::path::Path;

use chrono::Utc;

use super::harness::ScenarioResult;
use super::simulator::{SimTrade, WalletMetrics};

/// A complete sandbox run report.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub run_id: String,
    pub timestamp: String,
    pub config_snapshot: ConfigSnapshot,
    pub wallet: WalletMetrics,
    pub trades: Vec<SimTrade>,
    pub decisions: Vec<DecisionEntry>,
    pub category_breakdown: Vec<CategoryEntry>,
    pub failures: Vec<FailureEntry>,
    pub knowledge_stats: KnowledgeStats,
    pub perf_stats: PerfStats,
}

#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    pub pairs: Vec<String>,
    pub timeframe: String,
    pub model: String,
    pub concurrency: usize,
    pub starting_balance: f64,
}

#[derive(Debug, Clone)]
pub struct DecisionEntry {
    pub scenario_id: String,
    pub scenario_name: String,
    pub category: String,
    pub difficulty: String,
    pub action_taken: String,
    pub expected_action: String,
    pub score: f64,
    pub tier1_pass: bool,
    pub tier1_reason: Option<String>,
    pub tier2_score: f64,
    pub tier3_score: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub latency_ms: u64,
    pub pass_fail: String,
}

#[derive(Debug, Clone)]
pub struct CategoryEntry {
    pub category: String,
    pub total: usize,
    pub passed: usize,
    pub avg_score: f64,
}

#[derive(Debug, Clone)]
pub struct FailureEntry {
    pub scenario_id: String,
    pub scenario_name: String,
    pub reason: String,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct KnowledgeStats {
    pub total_units: usize,
    pub files_loaded: usize,
}

impl RunReport {
    pub fn generate(
        results: &[ScenarioResult],
        wallet: &WalletMetrics,
        trades: &[SimTrade],
        config: ConfigSnapshot,
        knowledge_stats: KnowledgeStats,
    ) -> Self {
        let run_id = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let timestamp = Utc::now().to_rfc3339();

        // Build decision entries
        let decisions: Vec<DecisionEntry> = results
            .iter()
            .map(|r| DecisionEntry {
                scenario_id: r.scenario_id.clone(),
                scenario_name: r.scenario_name.clone(),
                category: r.category.clone(),
                difficulty: r.difficulty.clone(),
                action_taken: r.action_taken.clone(),
                expected_action: String::new(),
                score: r.grade.total_score,
                tier1_pass: r.grade.tier_1_compliance,
                tier1_reason: r.grade.tier_1_reason.clone(),
                tier2_score: r.grade.tier_2_rr_score,
                tier3_score: r.grade.tier_3_reasoning_score,
                confidence: 0.0,
                reasoning: r.grade.tier_3_rationale.clone(),
                latency_ms: r.latency_ms,
                pass_fail: if r.grade.total_score >= 0.6 {
                    "PASS".into()
                } else {
                    "FAIL".into()
                },
            })
            .collect();

        // Category breakdown
        let mut categories: std::collections::HashMap<String, (usize, usize, f64)> =
            std::collections::HashMap::new();
        for r in results {
            let entry = categories.entry(r.category.clone()).or_insert((0, 0, 0.0));
            entry.0 += 1;
            if r.grade.total_score >= 0.6 {
                entry.1 += 1;
            }
            entry.2 += r.grade.total_score;
        }
        let category_breakdown: Vec<CategoryEntry> = categories
            .into_iter()
            .map(|(cat, (total, passed, score_sum))| CategoryEntry {
                category: cat,
                total,
                passed,
                avg_score: if total > 0 {
                    score_sum / total as f64
                } else {
                    0.0
                },
            })
            .collect();

        // Failures
        let failures: Vec<FailureEntry> = results
            .iter()
            .filter(|r| r.grade.total_score < 0.6)
            .map(|r| FailureEntry {
                scenario_id: r.scenario_id.clone(),
                scenario_name: r.scenario_name.clone(),
                reason: r
                    .grade
                    .tier_1_reason
                    .clone()
                    .unwrap_or_else(|| format!("Score: {:.2}", r.grade.total_score)),
                score: r.grade.total_score,
            })
            .collect();

        let perf_stats = PerfStats {
            total_llm_calls: results.len(),
            avg_latency_ms: if results.is_empty() {
                0
            } else {
                results.iter().map(|r| r.latency_ms).sum::<u64>() / results.len() as u64
            },
            timeout_count: results.iter().filter(|r| r.latency_ms > 30000).count(),
            error_count: results
                .iter()
                .filter(|r| r.action_taken.contains("Error") || r.action_taken.contains("Parse"))
                .count(),
        };

        Self {
            run_id,
            timestamp,
            config_snapshot: config,
            wallet: wallet.clone(),
            trades: trades.to_vec(),
            decisions,
            category_breakdown,
            failures,
            knowledge_stats,
            perf_stats,
        }
    }

    /// Render the full report as markdown.
    pub fn to_markdown(&self) -> String {
        let mut md = String::with_capacity(8192);

        // Header
        md.push_str(&format!("# Sandbox Run Report — {}\n\n", self.run_id));
        md.push_str(&format!("**Timestamp:** {}\n\n", self.timestamp));

        // Config
        md.push_str("## Configuration\n\n");
        md.push_str(&format!(
            "- **Pairs:** {}\n",
            self.config_snapshot.pairs.join(", ")
        ));
        md.push_str(&format!(
            "- **Timeframe:** {}\n",
            self.config_snapshot.timeframe
        ));
        md.push_str(&format!("- **Model:** {}\n", self.config_snapshot.model));
        md.push_str(&format!(
            "- **Concurrency:** {}\n",
            self.config_snapshot.concurrency
        ));
        md.push_str(&format!(
            "- **Starting Balance:** ${:.2}\n\n",
            self.config_snapshot.starting_balance
        ));

        // Wallet
        md.push_str("## Wallet Summary\n\n");
        md.push_str(&self.wallet.report_card());
        md.push_str("\n\n");

        // Trade Log
        if !self.trades.is_empty() {
            md.push_str("## Trade Log\n\n");
            md.push_str("| Scenario | Action | Side | Entry | Stop | Exit | P&L | R-Multiple | Fees | Exit Reason |\n");
            md.push_str("|----------|--------|------|-------|------|------|-----|-----------|------|-------------|\n");
            for t in &self.trades {
                md.push_str(&format!(
                    "| {} | {} | {} | {:.2} | {:.2} | {:.2} | ${:+.2} | {:.2} | ${:.2} | {} |\n",
                    t.scenario_id,
                    t.action,
                    t.side,
                    t.entry_price,
                    t.stop_loss,
                    t.exit_price,
                    t.pnl,
                    t.r_multiple,
                    t.fees_paid,
                    t.exit_reason
                ));
            }
            md.push('\n');
        }

        // Decision Log
        md.push_str("## Decision Log\n\n");
        for d in &self.decisions {
            md.push_str(&format!(
                "### {} ({}) — {}\n",
                d.scenario_name, d.scenario_id, d.pass_fail
            ));
            md.push_str(&format!(
                "- **Category:** {} | **Difficulty:** {}\n",
                d.category, d.difficulty
            ));
            md.push_str(&format!(
                "- **Action:** {} | **Expected:** {}\n",
                d.action_taken, d.expected_action
            ));
            md.push_str(&format!(
                "- **Score:** {:.2} | T1: {} | T2: {:.2} | T3: {:.2}\n",
                d.score, d.tier1_pass, d.tier2_score, d.tier3_score
            ));
            if let Some(ref reason) = d.tier1_reason {
                md.push_str(&format!("- **T1 Reason:** {}\n", reason));
            }
            md.push_str(&format!("- **Latency:** {}ms\n", d.latency_ms));
            if !d.reasoning.is_empty() {
                let truncated = if d.reasoning.len() > 500 {
                    format!("{}...", &d.reasoning[..500])
                } else {
                    d.reasoning.clone()
                };
                md.push_str(&format!("- **Reasoning:** {}\n", truncated));
            }
            md.push('\n');
        }

        // Category Breakdown
        md.push_str("## Category Breakdown\n\n");
        md.push_str("| Category | Passed | Total | Avg Score |\n");
        md.push_str("|----------|--------|-------|----------|\n");
        for c in &self.category_breakdown {
            md.push_str(&format!(
                "| {} | {}/{} | {:.2} |\n",
                c.category, c.passed, c.total, c.avg_score
            ));
        }
        md.push('\n');

        // Failures
        if !self.failures.is_empty() {
            md.push_str("## Failure Analysis\n\n");
            for f in &self.failures {
                md.push_str(&format!(
                    "- **{}** ({}) — Score: {:.2} — {}\n",
                    f.scenario_name, f.scenario_id, f.score, f.reason
                ));
            }
            md.push('\n');
        }

        // Knowledge Stats
        md.push_str("## Knowledge Stats\n\n");
        md.push_str(&format!(
            "- **Units loaded:** {}\n",
            self.knowledge_stats.total_units
        ));
        md.push_str(&format!(
            "- **Files loaded:** {}\n\n",
            self.knowledge_stats.files_loaded
        ));

        // Performance
        md.push_str("## Performance\n\n");
        md.push_str(&format!(
            "- **Total LLM calls:** {}\n",
            self.perf_stats.total_llm_calls
        ));
        md.push_str(&format!(
            "- **Avg latency:** {}ms\n",
            self.perf_stats.avg_latency_ms
        ));
        md.push_str(&format!(
            "- **Timeouts:** {}\n",
            self.perf_stats.timeout_count
        ));
        md.push_str(&format!("- **Errors:** {}\n", self.perf_stats.error_count));

        md
    }

    /// Write the report to disk.
    pub fn write_to_disk(&self, base_dir: &str) -> std::io::Result<String> {
        let dir = Path::new(base_dir).join("sandbox_reports");
        std::fs::create_dir_all(&dir)?;

        let filename = format!("{}.md", self.run_id);
        let path = dir.join(&filename);
        let content = self.to_markdown();
        std::fs::write(&path, &content)?;

        // Write latest.md copy
        let latest_path = dir.join("latest.md");
        std::fs::write(&latest_path, &content)?;

        // Write equity curve JSON
        let curve_path = dir.join("equity_curve.json");
        let curve_json =
            serde_json::to_string(&self.wallet.equity_curve).unwrap_or_else(|_| "[]".to_string());
        std::fs::write(&curve_path, &curve_json)?;

        Ok(path.to_string_lossy().to_string())
    }
}

#[derive(Debug, Clone)]
pub struct PerfStats {
    pub total_llm_calls: usize,
    pub avg_latency_ms: u64,
    pub timeout_count: usize,
    pub error_count: usize,
}
