use sqlx::Row;
use sqlx::SqlitePool;
use tracing::info;

use crate::core::types::TradeRecord;

pub struct TradeJournal {
    pool: SqlitePool,
}

impl TradeJournal {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;

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

        info!("Trade journal initialized");
        Ok(Self { pool })
    }

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
                    crate::core::types::Side::Long
                } else {
                    crate::core::types::Side::Short
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
