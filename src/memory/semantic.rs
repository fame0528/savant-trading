//! Semantic consolidation — converts raw episodic data into actionable patterns.
//!
//! Runs SQL aggregations against the episodic ledger to identify statistical
//! edges, conviction calibration, and knowledge unit efficacy. Populates the
//! semantic_patterns table for injection into the AI prompt (6th layer).

use chrono::Utc;
use sqlx::{Row, SqlitePool};
use tracing::{info, warn};

use crate::memory::episodic::EpisodicMemory;

/// A consolidated semantic pattern extracted from episodic memory.
#[derive(Debug, Clone)]
pub struct SemanticPattern {
    pub pattern_id: String,
    pub category: String,
    pub condition_value: String,
    pub sample_size: i64,
    pub win_rate: f64,
    pub avg_pnl: f64,
    pub avg_rr: f64,
    pub profit_factor: f64,
    pub is_valid: bool,
    pub confidence_penalty: f64,
}

/// Minimum sample size for a pattern to be considered valid.
const MIN_SAMPLE_SIZE: i64 = 5;

/// Run all consolidation queries against the episodic memory database.
///
/// This is the primary entry point for the semantic consolidation pipeline.
/// Should be called daily (00:00 UTC) or after every 10 closed trades.
pub async fn consolidate(memory: &EpisodicMemory) -> Result<usize, sqlx::Error> {
    let pool = memory.pool();
    let mut total_inserted = 0;

    // 1. Regime/Session/Pair edge matrix
    match consolidate_regime_session_pair(pool).await {
        Ok(n) => {
            info!("Semantic: regime/session/pair patterns: {}", n);
            total_inserted += n;
        }
        Err(e) => warn!("Semantic: regime/session/pair failed: {}", e),
    }

    // 2. Conviction calibration matrix
    match consolidate_conviction_calibration(pool).await {
        Ok(n) => {
            info!("Semantic: conviction calibration patterns: {}", n);
            total_inserted += n;
        }
        Err(e) => warn!("Semantic: conviction calibration failed: {}", e),
    }

    // 3. Category edge (from test scenarios)
    match consolidate_category_edge(pool).await {
        Ok(n) => {
            info!("Semantic: category edge patterns: {}", n);
            total_inserted += n;
        }
        Err(e) => warn!("Semantic: category edge failed: {}", e),
    }

    // 4. Evict stale patterns (rolling 90-day window)
    match evict_stale_patterns(pool).await {
        Ok(n) => {
            if n > 0 {
                info!("Semantic: evicted {} stale patterns", n);
            }
        }
        Err(e) => warn!("Semantic: eviction failed: {}", e),
    }

    info!(
        "Semantic consolidation complete: {} total patterns inserted/updated",
        total_inserted
    );
    Ok(total_inserted)
}

/// Extract performance patterns by regime, session, and pair.
///
/// Groups closed episodes by market regime + trading session + pair and
/// calculates win rate, average R:R, and profit factor. Only promotes
/// patterns with sufficient sample size.
async fn consolidate_regime_session_pair(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            ep.regime,
            ep.session,
            ep.pair,
            COUNT(ep.episode_id) as sample_size,
            CAST(SUM(CASE WHEN ep.is_win = 1 THEN 1 ELSE 0 END) AS REAL) / COUNT(ep.episode_id) as win_rate,
            AVG(CASE WHEN ep.pnl IS NOT NULL THEN ep.pnL ELSE NULL END) as avg_pnl,
            AVG(CASE WHEN ep.achieved_rr IS NOT NULL THEN ep.achieved_rr ELSE NULL END) as avg_rr,
            CASE
                WHEN SUM(CASE WHEN ep.is_win = 0 THEN 1 ELSE 0 END) = 0 THEN 99.0
                ELSE CAST(SUM(CASE WHEN ep.is_win = 1 THEN 1 ELSE 0 END) AS REAL) /
                     SUM(CASE WHEN ep.is_win = 0 THEN 1 ELSE 0 END)
            END as profit_factor
        FROM agent_episodes ep
        WHERE ep.status IN ('closed', 'executed', 'held', 'test_action', 'test_hold')
          AND ep.is_win IS NOT NULL
        GROUP BY ep.regime, ep.session, ep.pair
        HAVING sample_size >= ?
        "#,
    )
    .bind(MIN_SAMPLE_SIZE)
    .fetch_all(pool)
    .await?;

    let mut count = 0;
    for row in &rows {
        let regime: String = row.get("regime");
        let session: String = row.get("session");
        let pair: String = row.get("pair");
        let sample_size: i64 = row.get("sample_size");
        let win_rate: f64 = row.get("win_rate");
        let avg_pnl: f64 = row.try_get("avg_pnl").unwrap_or(0.0);
        let avg_rr: f64 = row.try_get("avg_rr").unwrap_or(0.0);
        let profit_factor: f64 = row.try_get("profit_factor").unwrap_or(0.0);

        let condition = format!("{}_{}_{}", regime, session, pair);
        let pattern_id = format!("regime_session_pair_{}", condition);

        upsert_pattern(
            pool,
            &pattern_id,
            "REGIME_SESSION_PAIR",
            &condition,
            sample_size,
            win_rate,
            avg_pnl,
            avg_rr,
            profit_factor,
        )
        .await?;
        count += 1;
    }

    Ok(count)
}

/// Extract conviction calibration patterns.
///
/// Groups by confidence bucket (HIGH/MEDIUM/LOW) and calculates actual
/// win rate. This reveals whether the agent's confidence is well-calibrated.
async fn consolidate_conviction_calibration(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            conviction_level,
            COUNT(episode_id) as sample_size,
            CAST(SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) AS REAL) / COUNT(episode_id) as win_rate,
            AVG(CASE WHEN pnl IS NOT NULL THEN pnl ELSE NULL END) as avg_pnl,
            AVG(CASE WHEN achieved_rr IS NOT NULL THEN achieved_rr ELSE NULL END) as avg_rr,
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
        HAVING sample_size >= ?
        "#,
    )
    .bind(MIN_SAMPLE_SIZE)
    .fetch_all(pool)
    .await?;

    let mut count = 0;
    for row in &rows {
        let conviction: String = row.get("conviction_level");
        let sample_size: i64 = row.get("sample_size");
        let win_rate: f64 = row.get("win_rate");
        let avg_pnl: f64 = row.try_get("avg_pnl").unwrap_or(0.0);
        let avg_rr: f64 = row.try_get("avg_rr").unwrap_or(0.0);
        let profit_factor: f64 = row.try_get("profit_factor").unwrap_or(0.0);

        let pattern_id = format!("conviction_{}", conviction);

        upsert_pattern(
            pool,
            &pattern_id,
            "CONVICTION_CALIBRATION",
            &conviction,
            sample_size,
            win_rate,
            avg_pnl,
            avg_rr,
            profit_factor,
        )
        .await?;
        count += 1;
    }

    Ok(count)
}

/// Extract category edge patterns from test scenario results.
///
/// Groups by scenario category (condition_tags from episode_market_context)
/// to identify which categories the agent excels at and which need improvement.
async fn consolidate_category_edge(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            ctx.condition_tags as category,
            COUNT(ep.episode_id) as sample_size,
            CAST(SUM(CASE WHEN ep.is_win = 1 THEN 1 ELSE 0 END) AS REAL) / COUNT(ep.episode_id) as win_rate,
            AVG(CASE WHEN ep.pnl IS NOT NULL THEN ep.pnl ELSE NULL END) as avg_pnl,
            AVG(CASE WHEN ep.achieved_rr IS NOT NULL THEN ep.achieved_rr ELSE NULL END) as avg_rr,
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
        GROUP BY ctx.condition_tags
        HAVING sample_size >= ?
        "#,
    )
    .bind(MIN_SAMPLE_SIZE)
    .fetch_all(pool)
    .await?;

    let mut count = 0;
    for row in &rows {
        let raw_category: String = row.get("category");
        // condition_tags is stored as JSON array e.g. '["Trend Bull"]'
        let category = serde_json::from_str::<Vec<String>>(&raw_category)
            .ok()
            .and_then(|v| v.into_iter().next())
            .unwrap_or_else(|| raw_category.clone());
        let sample_size: i64 = row.get("sample_size");
        let win_rate: f64 = row.get("win_rate");
        let avg_pnl: f64 = row.try_get("avg_pnl").unwrap_or(0.0);
        let avg_rr: f64 = row.try_get("avg_rr").unwrap_or(0.0);
        let profit_factor: f64 = row.try_get("profit_factor").unwrap_or(0.0);

        let pattern_id = format!("category_edge_{}", category);

        upsert_pattern(
            pool,
            &pattern_id,
            "CATEGORY_EDGE",
            &category,
            sample_size,
            win_rate,
            avg_pnl,
            avg_rr,
            profit_factor,
        )
        .await?;
        count += 1;
    }

    Ok(count)
}

/// Evict patterns whose rolling performance has degraded.
///
/// Patterns with profit_factor < 1.0 over the last 90 days are marked invalid.
/// They remain in the database for historical analysis but are excluded from
/// context injection.
async fn evict_stale_patterns(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE semantic_patterns
        SET is_valid = 0
        WHERE is_valid = 1
          AND profit_factor < 1.0
          AND last_updated < datetime('now', '-90 days')
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as usize)
}

/// Insert or update a semantic pattern.
#[allow(clippy::too_many_arguments)]
async fn upsert_pattern(
    pool: &SqlitePool,
    pattern_id: &str,
    category: &str,
    condition_value: &str,
    sample_size: i64,
    win_rate: f64,
    avg_pnl: f64,
    avg_rr: f64,
    profit_factor: f64,
) -> Result<(), sqlx::Error> {
    let is_valid = sample_size >= MIN_SAMPLE_SIZE && profit_factor > 0.0;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO semantic_patterns
            (pattern_id, category, condition_value, sample_size, win_rate,
             avg_pnl, avg_rr, profit_factor, last_updated, is_valid, confidence_penalty)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0.0)
        ON CONFLICT(pattern_id) DO UPDATE SET
            sample_size = excluded.sample_size,
            win_rate = excluded.win_rate,
            avg_pnl = excluded.avg_pnl,
            avg_rr = excluded.avg_rr,
            profit_factor = excluded.profit_factor,
            last_updated = excluded.last_updated,
            is_valid = excluded.is_valid
        "#,
    )
    .bind(pattern_id)
    .bind(category)
    .bind(condition_value)
    .bind(sample_size)
    .bind(win_rate)
    .bind(avg_pnl)
    .bind(avg_rr)
    .bind(profit_factor)
    .bind(&now)
    .bind(is_valid)
    .execute(pool)
    .await?;

    Ok(())
}

/// Query valid semantic patterns for prompt injection.
///
/// Returns patterns where is_valid = 1, ordered by profit_factor descending.
/// Limited to top N patterns to avoid context dilution.
pub async fn query_active_patterns(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<SemanticPattern>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT pattern_id, category, condition_value, sample_size,
               win_rate, avg_pnl, avg_rr, profit_factor, is_valid, confidence_penalty
        FROM semantic_patterns
        WHERE is_valid = 1
        ORDER BY profit_factor DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| SemanticPattern {
            pattern_id: r.get("pattern_id"),
            category: r.get("category"),
            condition_value: r.get("condition_value"),
            sample_size: r.get("sample_size"),
            win_rate: r.get("win_rate"),
            avg_pnl: r.try_get("avg_pnl").unwrap_or(0.0),
            avg_rr: r.try_get("avg_rr").unwrap_or(0.0),
            profit_factor: r.try_get("profit_factor").unwrap_or(0.0),
            is_valid: r.get::<bool, _>("is_valid"),
            confidence_penalty: r.try_get("confidence_penalty").unwrap_or(0.0),
        })
        .collect())
}

/// Format semantic patterns for prompt injection.
///
/// Converts patterns into a narrative format optimized for LLM consumption.
/// Only includes patterns with meaningful edges (profit_factor > 1.2).
pub fn format_patterns_for_prompt(patterns: &[SemanticPattern]) -> String {
    let meaningful: Vec<&SemanticPattern> = patterns
        .iter()
        .filter(|p| p.profit_factor > 1.2 && p.win_rate > 0.5)
        .collect();

    if meaningful.is_empty() {
        return String::new();
    }

    let mut msg = String::from("\n## Dynamic Memory Context\n\n");
    msg.push_str(&format!(
        "Based on {} historical patterns (N={} total observations):\n\n",
        meaningful.len(),
        meaningful.iter().map(|p| p.sample_size).sum::<i64>()
    ));

    for pattern in &meaningful {
        let label = if pattern.profit_factor > 2.0 {
            "STRONG EDGE"
        } else if pattern.profit_factor > 1.5 {
            "MODERATE EDGE"
        } else {
            "SLIGHT EDGE"
        };

        msg.push_str(&format!(
            "- **{}** ({})**: {}% win rate, PF {}, avg R:R {} (N={})\n",
            pattern.category,
            label,
            pattern.win_rate * 100.0,
            pattern.profit_factor,
            pattern.avg_rr,
            pattern.sample_size,
        ));
    }

    msg
}
