//! Experience Replay — weekend retrospective analysis.
//!
//! Queries HIGH conviction losses and missed opportunities,
//! re-prompts LLM to generate single-sentence heuristic lessons.

use chrono::Utc;
use sqlx::{Row, SqlitePool};
use tracing::debug;

use crate::memory::episodic::EpisodicMemory;

/// A lesson generated from experience replay.
#[derive(Debug, Clone)]
pub struct ReplayLesson {
    pub lesson_id: String,
    pub original_episode_id: String,
    pub error_type: String,
    pub heuristic: String,
    pub applied_count: i32,
}

/// Query HIGH conviction trades that resulted in losses.
pub async fn query_high_conviction_losses(
    memory: &EpisodicMemory,
    limit: i64,
) -> Result<Vec<(String, String, String, String)>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT episode_id, pair, regime, reasoning
        FROM agent_episodes
        WHERE conviction_level = 'HIGH'
          AND status = 'closed'
          AND is_win = 0
        ORDER BY timestamp DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(memory.pool())
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            (
                r.get("episode_id"),
                r.get("pair"),
                r.get("regime"),
                r.get("reasoning"),
            )
        })
        .collect())
}

/// Query NONE conviction decisions where price subsequently hit TP1 (missed opportunity).
pub async fn query_missed_opportunities(
    memory: &EpisodicMemory,
    limit: i64,
) -> Result<Vec<(String, String, String, String)>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT episode_id, pair, regime, reasoning
        FROM agent_episodes
        WHERE action = 'Pass'
          AND conviction_level = 'NONE'
        ORDER BY timestamp DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(memory.pool())
    .await?;

    Ok(rows
        .iter()
        .map(|r| {
            (
                r.get("episode_id"),
                r.get("pair"),
                r.get("regime"),
                r.get("reasoning"),
            )
        })
        .collect())
}

/// Store a replay lesson.
pub async fn store_lesson(
    pool: &SqlitePool,
    original_episode_id: &str,
    error_type: &str,
    heuristic: &str,
) -> Result<(), sqlx::Error> {
    let lesson_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO experience_replay_lessons
        (lesson_id, timestamp, original_episode_id, error_type, heuristic, applied_count)
        VALUES (?, ?, ?, ?, ?, 0)
        "#,
    )
    .bind(&lesson_id)
    .bind(Utc::now().to_rfc3339())
    .bind(original_episode_id)
    .bind(error_type)
    .bind(heuristic)
    .execute(pool)
    .await?;

    debug!("Stored replay lesson: {}", heuristic);
    Ok(())
}

/// Get all active replay lessons.
pub async fn get_lessons(pool: &SqlitePool) -> Result<Vec<ReplayLesson>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT lesson_id, original_episode_id, error_type, heuristic, applied_count
        FROM experience_replay_lessons
        ORDER BY timestamp DESC
        LIMIT 50
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| ReplayLesson {
            lesson_id: r.get("lesson_id"),
            original_episode_id: r.get("original_episode_id"),
            error_type: r.get("error_type"),
            heuristic: r.get("heuristic"),
            applied_count: r.get("applied_count"),
        })
        .collect())
}
