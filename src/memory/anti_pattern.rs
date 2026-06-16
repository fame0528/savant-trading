//! Anti-pattern detection — identifies market conditions where the agent
//! consistently fails.
//!
//! Queries episodic memory for multidimensional failure conditions and
//! generates narrative constraints for prompt injection.

use sqlx::{Row, SqlitePool};
use tracing::{info, warn};

/// A detected anti-pattern — a condition where the agent has a negative edge.
#[derive(Debug, Clone)]
pub struct AntiPattern {
    pub pattern_id: String,
    pub condition: String,
    pub sample_size: i64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub narrative: String,
}

const MIN_SAMPLE_SIZE: i64 = 5;
const FAILURE_WIN_RATE: f64 = 0.30;

pub async fn detect_anti_patterns(pool: &SqlitePool) -> Result<Vec<AntiPattern>, sqlx::Error> {
    let mut anti_patterns = Vec::new();

    if let Ok(ap) = detect_regime_anti_patterns(pool).await {
        anti_patterns.extend(ap);
    }
    if let Ok(ap) = detect_conviction_anti_patterns(pool).await {
        anti_patterns.extend(ap);
    }
    if let Ok(ap) = detect_category_anti_patterns(pool).await {
        anti_patterns.extend(ap);
    }

    match evict_recovered_anti_patterns(pool).await {
        Ok(n) => {
            if n > 0 {
                info!("Anti-patterns: {} conditions recovered (removed)", n);
            }
        }
        Err(e) => warn!("Anti-pattern eviction failed: {}", e),
    }

    info!("Anti-patterns detected: {}", anti_patterns.len());
    Ok(anti_patterns)
}

async fn detect_regime_anti_patterns(pool: &SqlitePool) -> Result<Vec<AntiPattern>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            regime,
            COUNT(episode_id) as sample_size,
            CAST(SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) AS REAL) / COUNT(episode_id) as win_rate,
            CASE
                WHEN SUM(CASE WHEN is_win = 0 THEN 1 ELSE 0 END) = 0 THEN 99.0
                ELSE CAST(SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) AS REAL) /
                     SUM(CASE WHEN is_win = 0 THEN 1 ELSE 0 END)
            END as profit_factor
        FROM agent_episodes
        WHERE status IN ('closed', 'executed', 'test_action', 'test_hold')
          AND is_win IS NOT NULL
        GROUP BY regime
        HAVING sample_size >= ? AND win_rate < ?
        "#,
    )
    .bind(MIN_SAMPLE_SIZE)
    .bind(FAILURE_WIN_RATE)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let regime: String = r.get("regime");
            let sample_size: i64 = r.get("sample_size");
            let win_rate: f64 = r.get("win_rate");
            let profit_factor: f64 = r.try_get("profit_factor").unwrap_or(0.0);

            AntiPattern {
                pattern_id: format!("anti_regime_{}", regime),
                condition: format!("regime={}", regime),
                sample_size,
                win_rate,
                profit_factor,
                narrative: format!(
                    "ANTI-PATTERN: In {} regime, your historical win rate is {}% (PF: {}, N={}). \
                     Reduce conviction or avoid entries unless offset by strong confirming signals.",
                    regime, win_rate * 100.0, profit_factor, sample_size
                ),
            }
        })
        .collect())
}

async fn detect_conviction_anti_patterns(
    pool: &SqlitePool,
) -> Result<Vec<AntiPattern>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            conviction_level,
            COUNT(episode_id) as sample_size,
            CAST(SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) AS REAL) / COUNT(episode_id) as win_rate,
            CASE
                WHEN SUM(CASE WHEN is_win = 0 THEN 1 ELSE 0 END) = 0 THEN 99.0
                ELSE CAST(SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) AS REAL) /
                     SUM(CASE WHEN is_win = 0 THEN 1 ELSE 0 END)
            END as profit_factor
        FROM agent_episodes
        WHERE status IN ('closed', 'executed', 'test_action')
          AND is_win IS NOT NULL
          AND conviction_level != 'NONE'
        GROUP BY conviction_level
        HAVING sample_size >= ? AND win_rate < ?
        "#,
    )
    .bind(MIN_SAMPLE_SIZE)
    .bind(FAILURE_WIN_RATE)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let conviction: String = r.get("conviction_level");
            let sample_size: i64 = r.get("sample_size");
            let win_rate: f64 = r.get("win_rate");
            let profit_factor: f64 = r.try_get("profit_factor").unwrap_or(0.0);

            AntiPattern {
                pattern_id: format!("anti_conviction_{}", conviction),
                condition: format!("conviction={}", conviction),
                sample_size,
                win_rate,
                profit_factor,
                narrative: format!(
                    "ANTI-PATTERN: {} conviction trades have {}% win rate (PF: {}, N={}). \
                     Reduce position sizing at this conviction level.",
                    conviction,
                    win_rate * 100.0,
                    profit_factor,
                    sample_size
                ),
            }
        })
        .collect())
}

/// Detect anti-patterns by scenario category (condition_tags).
///
/// This is the most important dimension — it identifies which trading
/// categories the agent consistently fails at (e.g., Trend Bull, Sentiment).
async fn detect_category_anti_patterns(pool: &SqlitePool) -> Result<Vec<AntiPattern>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            json_extract(ctx.condition_tags, '$[0]') as category,
            COUNT(ep.episode_id) as sample_size,
            CAST(SUM(CASE WHEN ep.is_win = 1 THEN 1 ELSE 0 END) AS REAL) / COUNT(ep.episode_id) as win_rate,
            CASE
                WHEN SUM(CASE WHEN ep.is_win = 0 THEN 1 ELSE 0 END) = 0 THEN 99.0
                ELSE CAST(SUM(CASE WHEN ep.is_win = 1 THEN 1 ELSE 0 END) AS REAL) /
                     SUM(CASE WHEN ep.is_win = 0 THEN 1 ELSE 0 END)
            END as profit_factor
        FROM agent_episodes ep
        JOIN episode_market_context ctx ON ep.episode_id = ctx.episode_id
        WHERE ep.status IN ('test_action', 'test_hold')
          AND ep.is_win IS NOT NULL
          AND ctx.condition_tags IS NOT NULL
          AND ctx.condition_tags != '[]'
        GROUP BY category
        HAVING sample_size >= ? AND win_rate < ?
        "#,
    )
    .bind(MIN_SAMPLE_SIZE)
    .bind(FAILURE_WIN_RATE)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            let category: String = r.get("category");
            let sample_size: i64 = r.get("sample_size");
            let win_rate: f64 = r.get("win_rate");
            let profit_factor: f64 = r.try_get("profit_factor").unwrap_or(0.0);

            AntiPattern {
                pattern_id: format!("anti_category_{}", category),
                condition: format!("category={}", category),
                sample_size,
                win_rate,
                profit_factor,
                narrative: format!(
                    "ANTI-PATTERN: {} scenarios yield {}% win rate (PF: {}, N={}). \
                     Apply stricter filters or avoid entries in {} setups.",
                    category,
                    win_rate * 100.0,
                    profit_factor,
                    sample_size,
                    category
                ),
            }
        })
        .collect())
}

async fn evict_recovered_anti_patterns(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE semantic_patterns
        SET is_valid = 0
        WHERE category = 'ANTI_PATTERN'
          AND is_valid = 1
          AND win_rate >= 0.40
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as usize)
}

pub fn format_anti_patterns_for_prompt(anti_patterns: &[AntiPattern]) -> String {
    if anti_patterns.is_empty() {
        return String::new();
    }

    let mut msg = String::from("\n## Active Anti-Pattern Constraints\n\n");
    msg.push_str("The following conditions have historically produced negative results. ");
    msg.push_str(
        "Standard entry triggers are invalidated unless offset by extreme confirming signals.\n\n",
    );

    for ap in anti_patterns {
        msg.push_str(&format!("- {}\n", ap.narrative));
    }

    msg
}
