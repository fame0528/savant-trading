//! Episodic memory — immutable ledger of every decision + market snapshot.
//!
//! SQLite WAL mode for concurrent reads (15 pairs) + single writer.
//! Every decision (including Holds) is captured with full context.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info};

// Types used by snapshot struct

/// Minimum viable snapshot captured at every decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimumViableSnapshot {
    // Execution data
    pub pair: String,
    pub action: String,
    pub side: Option<String>,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub planned_rr: f64,

    // Market context
    pub regime: String,
    pub session: String,
    pub funding_rate: Option<f64>,
    pub funding_rate_annualized: Option<f64>,
    pub fear_greed_index: Option<i32>,
    pub fear_greed_label: Option<String>,
    pub order_book_imbalance: Option<f64>,
    pub mvrv: Option<f64>,
    pub sopr: Option<f64>,
    pub nvt_signal: Option<f64>,
    pub atr: Option<f64>,
    pub adx: Option<f64>,
    pub rsi: Option<f64>,
    pub condition_tags: Vec<String>,

    // Cognitive state
    pub knowledge_units_used: Vec<String>,
    pub thesis_summary: String,
    pub invalidation_reasoning: String,

    // Outcome (filled later)
    pub pnl: Option<f64>,
    pub pnl_pct: Option<f64>,
    pub is_win: Option<bool>,
    pub achieved_rr: Option<f64>,
    pub status: String, // executed / held / rejected
}

/// Episodic memory store backed by SQLite WAL.
pub struct EpisodicMemory {
    pool: SqlitePool,
}

impl EpisodicMemory {
    /// Create or open the episodic memory database.
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(16)
            .connect_with(options)
            .await?;

        let memory = Self { pool };
        memory.run_migrations().await?;
        info!("Episodic memory initialized (WAL mode, 16 connections)");
        Ok(memory)
    }

    async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_episodes (
                episode_id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                pair TEXT NOT NULL,
                session TEXT NOT NULL,
                regime TEXT NOT NULL,
                action TEXT NOT NULL,
                side TEXT,
                conviction_level TEXT NOT NULL,
                planned_rr REAL,
                achieved_rr REAL,
                pnl REAL,
                pnl_pct REAL,
                is_win INTEGER,
                status TEXT NOT NULL,
                entry_price REAL,
                stop_loss REAL,
                take_profit_1 REAL,
                confidence REAL,
                reasoning TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS episode_market_context (
                episode_id TEXT PRIMARY KEY REFERENCES agent_episodes(episode_id),
                funding_rate REAL,
                funding_rate_annualized REAL,
                fear_greed_index INTEGER,
                fear_greed_label TEXT,
                order_book_imbalance REAL,
                mvrv_ratio REAL,
                sopr_ratio REAL,
                nvt_signal REAL,
                volatility_atr REAL,
                adx REAL,
                rsi REAL,
                volume_sma REAL,
                condition_tags TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS episode_cognitive_state (
                episode_id TEXT PRIMARY KEY REFERENCES agent_episodes(episode_id),
                knowledge_units_used TEXT,
                thesis_summary TEXT,
                invalidation_reasoning TEXT,
                confidence_score REAL,
                system_prompt_version TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS semantic_patterns (
                pattern_id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                condition_value TEXT NOT NULL,
                sample_size INTEGER NOT NULL,
                win_rate REAL,
                avg_pnl REAL,
                avg_rr REAL,
                profit_factor REAL,
                last_updated TEXT NOT NULL,
                is_valid INTEGER NOT NULL,
                confidence_penalty REAL DEFAULT 0.0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS edge_decay_alerts (
                alert_id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                category TEXT NOT NULL,
                condition_value TEXT NOT NULL,
                cusum_upper REAL,
                cusum_lower REAL,
                threshold REAL,
                action_taken TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS experience_replay_lessons (
                lesson_id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                original_episode_id TEXT NOT NULL,
                error_type TEXT NOT NULL,
                heuristic TEXT NOT NULL,
                applied_count INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indices for fast pattern queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_episodes_regime ON agent_episodes(regime)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_episodes_session ON agent_episodes(session)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_episodes_pair ON agent_episodes(pair)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_episodes_action ON agent_episodes(action)")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_episodes_timestamp ON agent_episodes(timestamp)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_episodes_status ON agent_episodes(status)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_patterns_category ON semantic_patterns(category, condition_value)")
            .execute(&self.pool)
            .await?;

        debug!("Episodic memory migrations complete (6 tables, 7 indices)");
        Ok(())
    }

    /// Capture a decision episode with full market context.
    pub async fn capture_episode(
        &self,
        snapshot: &MinimumViableSnapshot,
    ) -> Result<String, sqlx::Error> {
        let episode_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        // Determine conviction level from confidence
        let conviction = if snapshot.confidence >= 0.7 {
            "HIGH"
        } else if snapshot.confidence >= 0.4 {
            "MEDIUM"
        } else if snapshot.confidence > 0.0 {
            "LOW"
        } else {
            "NONE"
        };

        // Insert episode
        sqlx::query(
            r#"
            INSERT INTO agent_episodes
            (episode_id, timestamp, pair, session, regime, action, side,
             conviction_level, planned_rr, achieved_rr, pnl, pnl_pct,
             is_win, status, entry_price, stop_loss, take_profit_1,
             confidence, reasoning)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&episode_id)
        .bind(&timestamp)
        .bind(&snapshot.pair)
        .bind(&snapshot.session)
        .bind(&snapshot.regime)
        .bind(&snapshot.action)
        .bind(&snapshot.side)
        .bind(conviction)
        .bind(snapshot.planned_rr)
        .bind(snapshot.achieved_rr)
        .bind(snapshot.pnl)
        .bind(snapshot.pnl_pct)
        .bind(snapshot.is_win.map(|b| if b { 1 } else { 0 }))
        .bind(&snapshot.status)
        .bind(snapshot.entry_price)
        .bind(snapshot.stop_loss)
        .bind(snapshot.take_profit_1)
        .bind(snapshot.confidence)
        .bind(&snapshot.reasoning)
        .execute(&self.pool)
        .await?;

        // Insert market context
        let tags_json =
            serde_json::to_string(&snapshot.condition_tags).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            r#"
            INSERT INTO episode_market_context
            (episode_id, funding_rate, funding_rate_annualized,
             fear_greed_index, fear_greed_label, order_book_imbalance,
             mvrv_ratio, sopr_ratio, nvt_signal, volatility_atr,
             adx, rsi, volume_sma, condition_tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&episode_id)
        .bind(snapshot.funding_rate)
        .bind(snapshot.funding_rate_annualized)
        .bind(snapshot.fear_greed_index)
        .bind(&snapshot.fear_greed_label)
        .bind(snapshot.order_book_imbalance)
        .bind(snapshot.mvrv)
        .bind(snapshot.sopr)
        .bind(snapshot.nvt_signal)
        .bind(snapshot.atr)
        .bind(snapshot.adx)
        .bind(snapshot.rsi)
        .bind(None::<f64>) // volume_sma not in snapshot yet
        .bind(&tags_json)
        .execute(&self.pool)
        .await?;

        // Insert cognitive state
        let units_json = serde_json::to_string(&snapshot.knowledge_units_used)
            .unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            r#"
            INSERT INTO episode_cognitive_state
            (episode_id, knowledge_units_used, thesis_summary,
             invalidation_reasoning, confidence_score, system_prompt_version)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&episode_id)
        .bind(&units_json)
        .bind(&snapshot.thesis_summary)
        .bind(&snapshot.invalidation_reasoning)
        .bind(snapshot.confidence)
        .bind("1.0.0") // SOUL.md version
        .execute(&self.pool)
        .await?;

        debug!("Captured episode {} for {}", episode_id, snapshot.pair);
        Ok(episode_id)
    }

    /// Update episode outcome after trade closes.
    pub async fn update_outcome(
        &self,
        episode_id: &str,
        pnl: f64,
        pnl_pct: f64,
        is_win: bool,
        achieved_rr: f64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE agent_episodes
            SET pnl = ?, pnl_pct = ?, is_win = ?, achieved_rr = ?, status = 'closed'
            WHERE episode_id = ?
            "#,
        )
        .bind(pnl)
        .bind(pnl_pct)
        .bind(if is_win { 1 } else { 0 })
        .bind(achieved_rr)
        .bind(episode_id)
        .execute(&self.pool)
        .await?;

        debug!(
            "Updated episode {} outcome: pnl={:.2}, win={}",
            episode_id, pnl, is_win
        );
        Ok(())
    }

    /// Query win rate by regime.
    pub async fn win_rate_by_regime(&self, regime: &str) -> Result<Option<f64>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins
            FROM agent_episodes
            WHERE regime = ? AND status = 'closed'
            "#,
        )
        .bind(regime)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let total: i64 = r.get("total");
                let wins: i64 = r.get("wins");
                if total > 0 {
                    Ok(Some(wins as f64 / total as f64))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Query win rate by session.
    pub async fn win_rate_by_session(&self, session: &str) -> Result<Option<f64>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins
            FROM agent_episodes
            WHERE session = ? AND status = 'closed'
            "#,
        )
        .bind(session)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let total: i64 = r.get("total");
                let wins: i64 = r.get("wins");
                if total > 0 {
                    Ok(Some(wins as f64 / total as f64))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Query win rate by pair.
    pub async fn win_rate_by_pair(&self, pair: &str) -> Result<Option<f64>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins
            FROM agent_episodes
            WHERE pair = ? AND status = 'closed'
            "#,
        )
        .bind(pair)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let total: i64 = r.get("total");
                let wins: i64 = r.get("wins");
                if total > 0 {
                    Ok(Some(wins as f64 / total as f64))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Query win rate by conviction level.
    pub async fn win_rate_by_conviction(
        &self,
        conviction: &str,
    ) -> Result<Option<f64>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins
            FROM agent_episodes
            WHERE conviction_level = ? AND status = 'closed'
            "#,
        )
        .bind(conviction)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let total: i64 = r.get("total");
                let wins: i64 = r.get("wins");
                if total > 0 {
                    Ok(Some(wins as f64 / total as f64))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Get total trade count.
    pub async fn total_trades(&self) -> Result<i64, sqlx::Error> {
        let row =
            sqlx::query("SELECT COUNT(*) as count FROM agent_episodes WHERE status IN ('closed', 'executed', 'test_action', 'test_hold')")
                .fetch_one(&self.pool)
                .await?;
        Ok(row.get("count"))
    }

    /// Get total episode count (including holds).
    pub async fn total_episodes(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM agent_episodes")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }

    /// Get recent episodes for a pair.
    pub async fn recent_episodes(
        &self,
        pair: &str,
        limit: i64,
    ) -> Result<Vec<MinimumViableSnapshot>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT e.*, mc.funding_rate, mc.fear_greed_index, mc.order_book_imbalance,
                   mc.mvrv_ratio, mc.sopr_ratio, mc.volatility_atr, mc.adx, mc.rsi,
                   mc.condition_tags, cs.knowledge_units_used, cs.thesis_summary,
                   cs.invalidation_reasoning
            FROM agent_episodes e
            LEFT JOIN episode_market_context mc ON e.episode_id = mc.episode_id
            LEFT JOIN episode_cognitive_state cs ON e.episode_id = cs.episode_id
            WHERE e.pair = ?
            ORDER BY e.timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(pair)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut snapshots = Vec::with_capacity(rows.len());
        for row in rows {
            let condition_tags: Vec<String> = row
                .get::<Option<String>, _>("condition_tags")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let knowledge_units: Vec<String> = row
                .get::<Option<String>, _>("knowledge_units_used")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            snapshots.push(MinimumViableSnapshot {
                pair: row.get("pair"),
                action: row.get("action"),
                side: row.get("side"),
                entry_price: row.get::<f64, _>("entry_price"),
                stop_loss: row.get::<f64, _>("stop_loss"),
                take_profit_1: row.get::<f64, _>("take_profit_1"),
                confidence: row.get::<f64, _>("confidence"),
                reasoning: row.get("reasoning"),
                planned_rr: row.get::<f64, _>("planned_rr"),
                regime: row.get("regime"),
                session: row.get("session"),
                funding_rate: row.get("funding_rate"),
                funding_rate_annualized: None,
                fear_greed_index: row.get("fear_greed_index"),
                fear_greed_label: None,
                order_book_imbalance: row.get("order_book_imbalance"),
                mvrv: row.get("mvrv_ratio"),
                sopr: row.get("sopr_ratio"),
                nvt_signal: None,
                atr: row.get("volatility_atr"),
                adx: row.get("adx"),
                rsi: row.get("rsi"),
                condition_tags,
                knowledge_units_used: knowledge_units,
                thesis_summary: row.get("thesis_summary"),
                invalidation_reasoning: row.get("invalidation_reasoning"),
                pnl: row.get("pnl"),
                pnl_pct: row.get("pnl_pct"),
                is_win: row.get::<Option<i32>, _>("is_win").map(|v| v == 1),
                achieved_rr: row.get("achieved_rr"),
                status: row.get("status"),
            });
        }

        Ok(snapshots)
    }

    /// Get the database pool for advanced queries.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
