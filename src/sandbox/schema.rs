//! SQLite schema for sandbox evaluation storage.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info};

use crate::sandbox::harness::ScenarioResult;

/// Sandbox database store.
pub struct SandboxDb {
    pool: SqlitePool,
}

impl SandboxDb {
    /// Create or open the sandbox database.
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        info!("Sandbox database initialized");
        Ok(db)
    }

    async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS soul_versions (
                version_hash TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                content TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS scenario_catalog (
                scenario_id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                scenario_name TEXT NOT NULL,
                difficulty TEXT NOT NULL,
                trigger_condition TEXT NOT NULL,
                expected_action TEXT NOT NULL,
                target_rule TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS evaluation_runs (
                run_id TEXT PRIMARY KEY,
                version_hash TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT,
                total_scenarios INTEGER,
                passed INTEGER,
                failed INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_decisions (
                decision_id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                scenario_id TEXT NOT NULL,
                pair TEXT NOT NULL,
                json_payload TEXT NOT NULL,
                reasoning TEXT,
                latency_ms INTEGER,
                token_count INTEGER
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rubric_scores (
                score_id TEXT PRIMARY KEY,
                decision_id TEXT NOT NULL,
                tier_1_compliance INTEGER,
                tier_2_rr_score REAL,
                tier_3_reasoning_score REAL,
                total_score REAL,
                judge_rationale TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Indices
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_decisions_run ON agent_decisions(run_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_decisions_scenario ON agent_decisions(scenario_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_scores_decision ON rubric_scores(decision_id)")
            .execute(&self.pool)
            .await?;

        debug!("Sandbox migrations complete (5 tables, 3 indices)");
        Ok(())
    }

    /// Store a scenario result.
    pub async fn store_result(
        &self,
        run_id: &str,
        result: &ScenarioResult,
    ) -> Result<(), sqlx::Error> {
        let decision_id = uuid::Uuid::new_v4().to_string();
        let score_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO agent_decisions
            (decision_id, run_id, scenario_id, pair, json_payload, reasoning, latency_ms, token_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, 0)
            "#,
        )
        .bind(&decision_id)
        .bind(run_id)
        .bind(&result.scenario_id)
        .bind("BTC/USD")
        .bind(&result.action_taken)
        .bind(&result.grade.tier_3_rationale)
        .bind(result.latency_ms as i64)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO rubric_scores
            (score_id, decision_id, tier_1_compliance, tier_2_rr_score, tier_3_reasoning_score, total_score, judge_rationale)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&score_id)
        .bind(&decision_id)
        .bind(if result.grade.tier_1_compliance { 1 } else { 0 })
        .bind(result.grade.tier_2_rr_score)
        .bind(result.grade.tier_3_reasoning_score)
        .bind(result.grade.total_score)
        .bind(&result.grade.tier_3_rationale)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store a run summary.
    pub async fn store_run(
        &self,
        run_id: &str,
        version_hash: &str,
        total: usize,
        passed: usize,
        failed: usize,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO evaluation_runs
            (run_id, version_hash, start_time, end_time, total_scenarios, passed, failed)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(run_id)
        .bind(version_hash)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(total as i64)
        .bind(passed as i64)
        .bind(failed as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the database pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
