use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;
use tracing::info;

use crate::core::types::{Position, ScaleLevel, Side, TradeRecord};

pub struct TradeJournal {
    pool: sqlx::SqlitePool,
}

impl TradeJournal {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        // Resolve to absolute path for logging — relative paths can create
        // new empty DBs if working directory differs.
        let abs_path = std::path::Path::new(database_url.trim_start_matches("sqlite:"));
        let abs_display = abs_path
            .canonicalize()
            .unwrap_or_else(|_| abs_path.to_path_buf());
        info!("Trade journal connecting to: {}", abs_display.display());

        let options = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trades (
                id TEXT PRIMARY KEY,
                pair TEXT NOT NULL,
                side TEXT NOT NULL,
                entry_price REAL NOT NULL,
                exit_price REAL NOT NULL,
                quantity REAL NOT NULL,
                pnl REAL NOT NULL,
                pnl_pct REAL NOT NULL,
                fees REAL NOT NULL DEFAULT 0.0,
                strategy_name TEXT NOT NULL,
                opened_at TEXT NOT NULL,
                closed_at TEXT NOT NULL,
                notes TEXT
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS equity_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                balance REAL NOT NULL,
                equity REAL NOT NULL,
                drawdown_pct REAL NOT NULL,
                open_positions INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS positions (
                id TEXT PRIMARY KEY,
                pair TEXT NOT NULL,
                side TEXT NOT NULL,
                entry_price REAL NOT NULL,
                current_price REAL NOT NULL,
                quantity REAL NOT NULL,
                stop_loss REAL NOT NULL,
                take_profit_1 REAL NOT NULL,
                take_profit_2 REAL NOT NULL,
                take_profit_3 REAL NOT NULL,
                unrealized_pnl REAL NOT NULL,
                risk_amount REAL NOT NULL,
                strategy_name TEXT NOT NULL,
                scale_level TEXT NOT NULL,
                opened_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS activity_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                pair TEXT NOT NULL,
                message TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        info!("Trade journal initialized");
        Ok(Self { pool })
    }

    // ── Positions (instant persistence) ────────────────────────────────

    pub async fn save_position(&self, pos: &Position) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO positions
                (id, pair, side, entry_price, current_price, quantity, stop_loss,
                 take_profit_1, take_profit_2, take_profit_3, unrealized_pnl,
                 risk_amount, strategy_name, scale_level, opened_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&pos.id)
        .bind(&pos.pair)
        .bind(format!("{}", pos.side))
        .bind(pos.entry_price)
        .bind(pos.current_price)
        .bind(pos.quantity)
        .bind(pos.stop_loss)
        .bind(pos.take_profit_1)
        .bind(pos.take_profit_2)
        .bind(pos.take_profit_3)
        .bind(pos.unrealized_pnl)
        .bind(pos.risk_amount)
        .bind(&pos.strategy_name)
        .bind(format!("{:?}", pos.scale_level))
        .bind(pos.opened_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_position(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM positions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn load_positions(&self) -> Result<Vec<Position>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, pair, side, entry_price, current_price, quantity, stop_loss, \
             take_profit_1, take_profit_2, take_profit_3, unrealized_pnl, risk_amount, \
             strategy_name, scale_level, opened_at FROM positions",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut positions = Vec::with_capacity(rows.len());
        for row in rows {
            let side_str: String = row.get("side");
            let scale_str: String = row.get("scale_level");
            let opened_at_str: String = row.get("opened_at");

            let opened_at = chrono::DateTime::parse_from_rfc3339(&opened_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .with_timezone(&chrono::Utc);

            positions.push(Position {
                id: row.get("id"),
                pair: row.get("pair"),
                side: if side_str == "Long" {
                    Side::Long
                } else {
                    Side::Short
                },
                entry_price: row.get("entry_price"),
                current_price: row.get("current_price"),
                quantity: row.get("quantity"),
                stop_loss: row.get("stop_loss"),
                take_profit_1: row.get("take_profit_1"),
                take_profit_2: row.get("take_profit_2"),
                take_profit_3: row.get("take_profit_3"),
                unrealized_pnl: row.get("unrealized_pnl"),
                risk_amount: row.get("risk_amount"),
                strategy_name: row.get("strategy_name"),
                scale_level: match scale_str.as_str() {
                    "Scaled50" => ScaleLevel::Scaled50,
                    "Scaled80" => ScaleLevel::Scaled80,
                    "Closed" => ScaleLevel::Closed,
                    _ => ScaleLevel::Full,
                },
                opened_at,
            });
        }

        Ok(positions)
    }

    // ── Activity Log ───────────────────────────────────────────────────

    pub async fn record_activity(
        &self,
        level: &str,
        pair: &str,
        message: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO activity_log (timestamp, level, pair, message) VALUES (?, ?, ?, ?)",
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(level)
        .bind(pair)
        .bind(message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_activity(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, String, String, String)>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT timestamp, level, pair, message FROM activity_log ORDER BY id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<String, _>("timestamp"),
                    r.get::<String, _>("level"),
                    r.get::<String, _>("pair"),
                    r.get::<String, _>("message"),
                )
            })
            .collect())
    }

    // ── Trades ─────────────────────────────────────────────────────────

    pub async fn record_trade(&self, trade: &TradeRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO trades (id, pair, side, entry_price, exit_price, quantity, pnl, pnl_pct, fees, strategy_name, opened_at, closed_at, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&trade.id)
        .bind(&trade.pair)
        .bind(format!("{:?}", trade.side))
        .bind(trade.entry_price)
        .bind(trade.exit_price)
        .bind(trade.quantity)
        .bind(trade.pnl)
        .bind(trade.pnl_pct)
        .bind(trade.fees)
        .bind(&trade.strategy_name)
        .bind(trade.opened_at.to_rfc3339())
        .bind(trade.closed_at.to_rfc3339())
        .bind(&trade.notes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_snapshot(
        &self,
        balance: f64,
        equity: f64,
        drawdown_pct: f64,
        open_positions: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO equity_snapshots (timestamp, balance, equity, drawdown_pct, open_positions)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(balance)
        .bind(equity)
        .bind(drawdown_pct)
        .bind(open_positions)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_snapshots(&self, limit: i64) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT timestamp, balance, equity, drawdown_pct, open_positions FROM equity_snapshots ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut snapshots = Vec::with_capacity(rows.len());
        for row in rows {
            snapshots.push(serde_json::json!({
                "timestamp": row.get::<String, _>(0),
                "balance": row.get::<f64, _>(1),
                "equity": row.get::<f64, _>(2),
                "drawdown_pct": row.get::<f64, _>(3),
                "open_positions": row.get::<i32, _>(4),
            }));
        }
        Ok(snapshots)
    }

    pub async fn get_trades(&self, limit: i64) -> Result<Vec<TradeRecord>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, pair, side, entry_price, exit_price, quantity, pnl, pnl_pct, fees, strategy_name, opened_at, closed_at, notes FROM trades ORDER BY closed_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut trades = Vec::with_capacity(rows.len());
        for row in rows {
            let opened_at_str: String = row.get("opened_at");
            let closed_at_str: String = row.get("closed_at");
            let side_str: String = row.get("side");
            let notes: Option<String> = row.get("notes");

            let opened_at = chrono::DateTime::parse_from_rfc3339(&opened_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .with_timezone(&chrono::Utc);
            let closed_at = chrono::DateTime::parse_from_rfc3339(&closed_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .with_timezone(&chrono::Utc);

            trades.push(TradeRecord {
                id: row.get("id"),
                pair: row.get("pair"),
                side: if side_str == "Long" {
                    Side::Long
                } else {
                    Side::Short
                },
                entry_price: row.get("entry_price"),
                exit_price: row.get("exit_price"),
                quantity: row.get("quantity"),
                pnl: row.get("pnl"),
                pnl_pct: row.get("pnl_pct"),
                fees: row.get::<f64, _>("fees"),
                strategy_name: row.get("strategy_name"),
                opened_at,
                closed_at,
                notes: notes.unwrap_or_default(),
                on_chain_verified: false,
                tx_hash: None,
            });
        }

        Ok(trades)
    }

    pub async fn daily_summary(&self) -> Result<Vec<DailySummary>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                date(closed_at) as day,
                COUNT(*) as trades,
                SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
                SUM(pnl) as total_pnl,
                MIN(pnl) as worst_trade,
                MAX(pnl) as best_trade
            FROM trades
            GROUP BY date(closed_at)
            ORDER BY day DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let day: String = row.get("day");
            let trades: i64 = row.get("trades");
            let wins: i64 = row.get("wins");
            let total_pnl: f64 = row.get("total_pnl");
            let worst_trade: f64 = row.get("worst_trade");
            let best_trade: f64 = row.get("best_trade");

            summaries.push(DailySummary {
                day,
                trades: trades as usize,
                wins: wins as usize,
                losses: (trades - wins) as usize,
                win_rate: if trades > 0 {
                    wins as f64 / trades as f64
                } else {
                    0.0
                },
                total_pnl,
                worst_trade,
                best_trade,
            });
        }

        Ok(summaries)
    }
}

pub struct DailySummary {
    pub day: String,
    pub trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub worst_trade: f64,
    pub best_trade: f64,
}
