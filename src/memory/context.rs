//! Memory context — 6th prompt layer for Dynamic Memory Context.
//!
//! Queries episodic memory at decision time and formats results
//! for injection into the AI prompt.

use crate::memory::cusum::CusumChart;
use crate::memory::episodic::EpisodicMemory;

/// Memory context formatted for AI prompt injection.
#[derive(Debug, Clone, Default)]
pub struct MemoryContext {
    /// Win rate for current regime + session combination.
    pub regime_session_win_rate: Option<f64>,
    /// Win rate for current pair.
    pub pair_win_rate: Option<f64>,
    /// Total closed trades.
    pub total_trades: i64,
    /// Recent episode summaries (last 3).
    pub recent_episodes: Vec<String>,
    /// Operator rules from Lessons/ vault.
    pub operator_rules: Vec<String>,
    /// Brier Score (if enough data).
    pub brier_score: Option<f64>,
    /// CUSUM alerts.
    pub cusum_alerts: Vec<String>,
    /// Confidence penalty applied.
    pub confidence_penalty: f64,
}

/// Query memory context for a specific pair/regime/session combination.
///
/// The optional `cusum` parameter, when provided, populates `cusum_alerts`
/// with the CUSUM chart's status string so the LLM sees edge decay alerts.
/// FID-163 Part D.
pub async fn query_memory_context(
    memory: &EpisodicMemory,
    pair: &str,
    regime: &str,
    _session: &str,
    cusum: Option<&CusumChart>,
) -> MemoryContext {
    let total_trades = memory.total_trades().await.unwrap_or(0);

    let mut ctx = MemoryContext {
        total_trades,
        ..Default::default()
    };

    // Win rate by regime
    if let Ok(Some(wr)) = memory.win_rate_by_regime(regime).await {
        ctx.regime_session_win_rate = Some(wr);
    }

    // Win rate by pair
    if let Ok(Some(wr)) = memory.win_rate_by_pair(pair).await {
        ctx.pair_win_rate = Some(wr);
    }

    // CUSUM edge-decay alert (FID-163 Part D)
    if let Some(chart) = cusum {
        ctx.cusum_alerts.push(chart.status());
    }

    // Recent episodes
    if let Ok(episodes) = memory.recent_episodes(pair, 3).await {
        for ep in &episodes {
            let result = if ep.status == "closed" {
                match ep.is_win {
                    Some(true) => format!("WIN (+{}R)", ep.achieved_rr.unwrap_or(0.0)),
                    Some(false) => format!("LOSS (-{}R)", ep.achieved_rr.unwrap_or(0.0).abs()),
                    None => "OPEN".to_string(),
                }
            } else {
                "HELD".to_string()
            };
            ctx.recent_episodes.push(format!(
                "{}: {} {} → {}",
                ep.session, ep.action, ep.pair, result
            ));
        }
    }

    ctx
}

/// Format memory context as a prompt section.
pub fn format_memory_prompt(ctx: &MemoryContext) -> String {
    if ctx.total_trades < 5 {
        return String::new();
    }

    let mut msg = String::from("\n## Dynamic Memory Context\n\n");

    // Overall stats
    msg.push_str(&format!("Total closed trades: {}\n", ctx.total_trades));

    // Win rates
    if let Some(wr) = ctx.regime_session_win_rate {
        let label = if wr >= 0.6 {
            "STRONG EDGE"
        } else if wr >= 0.5 {
            "SLIGHT EDGE"
        } else {
            "NEGATIVE EDGE — reduce conviction"
        };
        msg.push_str(&format!(
            "Win rate in this regime: {}% ({})\n",
            wr * 100.0,
            label
        ));
    }

    if let Some(wr) = ctx.pair_win_rate {
        msg.push_str(&format!("Win rate on this pair: {}%\n", wr * 100.0));
    }

    // Confidence penalty
    if ctx.confidence_penalty > 0.0 {
        msg.push_str(&format!(
            "Confidence penalty: -{}% (calibration adjustment)\n",
            ctx.confidence_penalty * 100.0
        ));
    }

    // CUSUM alerts
    if !ctx.cusum_alerts.is_empty() {
        msg.push_str("\nEDGE DECAY ALERTS:\n");
        for alert in &ctx.cusum_alerts {
            msg.push_str(&format!("- {}\n", alert));
        }
    }

    // Recent episodes
    if !ctx.recent_episodes.is_empty() {
        msg.push_str("\nRecent analogs:\n");
        for ep in &ctx.recent_episodes {
            msg.push_str(&format!("- {}\n", ep));
        }
    }

    // Operator rules
    if !ctx.operator_rules.is_empty() {
        msg.push_str("\nOPERATOR RULES (override all AI reasoning):\n");
        for rule in &ctx.operator_rules {
            msg.push_str(&format!("- {}\n", rule));
        }
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_prompt_preserves_win_rate_precision() {
        // wr = 0.54321 should render as 54.321% (not 54% with old {:.0}).
        let ctx = MemoryContext {
            total_trades: 100,
            regime_session_win_rate: Some(0.54321),
            ..Default::default()
        };
        let prompt = format_memory_prompt(&ctx);
        assert!(prompt.contains("54.321"),
                "win rate 0.54321 should render as 54.321%, got prompt:\n{}", prompt);
    }
}
