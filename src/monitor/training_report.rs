//! Training report — comprehensive audit of agent performance across all
//! training episodes.
//!
//! Queries test_memory.db for episode data and produces a full report with
//! P&L simulation, win rates, calibration curves, category edge analysis,
//! and knowledge utility distribution.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::Row;
use std::str::FromStr;
use std::time::Duration;

/// Run the full training report against the test memory database.
pub async fn run_training_report(db_path: &str) -> anyhow::Result<()> {
    let options = SqliteConnectOptions::from_str(db_path)?
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5))
        .read_only(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    println!();
    println!("{}", "=".repeat(80));
    println!("SAVANT TRAINING REPORT");
    println!("{}", "=".repeat(80));

    // Total episodes
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_episodes")
        .fetch_one(&pool)
        .await?;
    println!("\nTotal Episodes: {}", total);

    if total == 0 {
        println!("No episodes recorded yet. Run `savant --test --train` first.");
        return Ok(());
    }

    // Status breakdown
    print_status_breakdown(&pool).await?;

    // P&L simulation
    print_pnl_simulation(&pool).await?;

    // Win rate by category
    print_category_edge(&pool).await?;

    // Win rate by conviction level
    print_conviction_calibration(&pool).await?;

    // Win rate by regime
    print_regime_edge(&pool).await?;

    // Win rate by session
    print_session_edge(&pool).await?;

    // Confidence calibration curve
    print_confidence_curve(&pool).await?;

    // Top winning patterns
    print_top_patterns(&pool).await?;

    // Top losing patterns (anti-patterns)
    print_anti_patterns(&pool).await?;

    // Knowledge utility distribution
    print_knowledge_utility().await?;

    // Auto-lessons summary
    print_lessons_summary(&pool).await?;

    // Semantic patterns
    print_semantic_patterns(&pool).await?;

    // Recent episodes (last 20)
    print_recent_episodes(&pool).await?;

    println!("\n{}", "=".repeat(80));
    println!("END OF REPORT");
    println!("{}\n", "=".repeat(80));

    pool.close().await;
    Ok(())
}

async fn print_status_breakdown(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*) as cnt FROM agent_episodes GROUP BY status ORDER BY cnt DESC",
    )
    .fetch_all(pool)
    .await?;

    println!("\n--- STATUS BREAKDOWN ---");
    for row in &rows {
        let status: String = row.get("status");
        let cnt: i64 = row.get("cnt");
        println!("  {:<20} {:>6}", status, cnt);
    }
    Ok(())
}

async fn print_pnl_simulation(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    // Simulate P&L from test episodes
    // For trades: pnl = entry * (exit_pct) based on is_win
    // For holds: pnl = 0
    let trades = sqlx::query(
        r#"
        SELECT
            action, side, entry_price, confidence, is_win, planned_rr, status,
            (SELECT condition_tags FROM episode_market_context WHERE episode_id = agent_episodes.episode_id) as category
        FROM agent_episodes
        WHERE status IN ('test_action', 'executed')
        ORDER BY timestamp
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut total_pnl = 0.0f64;
    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut trade_count = 0u32;
    let mut gross_profit = 0.0f64;
    let mut gross_loss = 0.0f64;
    let mut max_drawdown = 0.0f64;
    let mut peak_equity = 50.0f64;
    let mut running_balance = 50.0f64;
    let mut equity_curve = vec![50.0f64];
    let mut winning_trades = Vec::new();
    let mut losing_trades = Vec::new();

    for row in &trades {
        let _entry: f64 = row.get("entry_price");
        let confidence: f64 = row.get("confidence");
        let is_win: Option<bool> = row.try_get("is_win").ok().flatten();
        let planned_rr: f64 = row.try_get("planned_rr").unwrap_or(1.5);
        let category: Option<String> = row.try_get("category").ok().flatten();
        let category = category.unwrap_or_else(|| "Unknown".to_string());

        if let Some(win) = is_win {
            trade_count += 1;
            // Simulate: fixed $5 risk per trade (10% of starting $50)
            let risk_amount = 5.0;
            let pnl = if win {
                let p = risk_amount * planned_rr;
                wins += 1;
                gross_profit += p;
                winning_trades.push((category.clone(), confidence, p));
                p
            } else {
                let p = -risk_amount;
                losses += 1;
                gross_loss += p.abs();
                losing_trades.push((category.clone(), confidence, p));
                p
            };

            total_pnl += pnl;
            running_balance += pnl;
            equity_curve.push(running_balance);
            if running_balance > peak_equity {
                peak_equity = running_balance;
            }
            let dd = (peak_equity - running_balance) / peak_equity;
            if dd > max_drawdown {
                max_drawdown = dd;
            }
        }
    }

    let win_rate = if trade_count > 0 {
        wins as f64 / trade_count as f64
    } else {
        0.0
    };
    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else {
        f64::INFINITY
    };
    let avg_win = if wins > 0 {
        gross_profit / wins as f64
    } else {
        0.0
    };
    let avg_loss = if losses > 0 {
        gross_loss / losses as f64
    } else {
        0.0
    };
    let expectancy = if trade_count > 0 {
        total_pnl / trade_count as f64
    } else {
        0.0
    };
    let final_equity = *equity_curve.last().unwrap_or(&50.0);

    println!("\n--- P&L SIMULATION ---");
    println!("  Starting Balance:  $50.00");
    println!("  Final Balance:     ${:.2}", final_equity);
    println!(
        "  Total P&L:         ${:.2} ({:.1}%)",
        total_pnl,
        (total_pnl / 50.0) * 100.0
    );
    println!("  Total Trades:      {}", trade_count);
    println!("  Wins:              {} ({:.1}%)", wins, win_rate * 100.0);
    println!(
        "  Losses:            {} ({:.1}%)",
        losses,
        (1.0 - win_rate) * 100.0
    );
    println!("  Profit Factor:     {:.2}", profit_factor);
    println!("  Avg Win:           ${:.2}", avg_win);
    println!("  Avg Loss:          ${:.2}", avg_loss);
    println!("  Expectancy:        ${:.2}", expectancy);
    println!("  Max Drawdown:      {:.1}%", max_drawdown * 100.0);

    // Equity curve (simplified)
    if equity_curve.len() > 1 {
        println!("\n  Equity Curve (first 10 → last 10):");
        let show = if equity_curve.len() > 20 {
            let first: Vec<_> = equity_curve[..10].iter().collect();
            let last: Vec<_> = equity_curve[equity_curve.len() - 10..].iter().collect();
            [first, vec![&0.0], last].concat()
        } else {
            equity_curve.iter().collect()
        };
        for (i, eq) in show.iter().enumerate() {
            if **eq == 0.0 && i == 10 {
                println!("    ...");
            } else {
                println!("    Trade {:>3}: ${:.2}", i, eq);
            }
        }
    }

    Ok(())
}

async fn print_category_edge(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            ctx.condition_tags as category,
            COUNT(*) as total,
            SUM(CASE WHEN ep.is_win = 1 THEN 1 ELSE 0 END) as wins,
            AVG(ep.confidence) as avg_conf
        FROM agent_episodes ep
        JOIN episode_market_context ctx ON ep.episode_id = ctx.episode_id
        WHERE ep.is_win IS NOT NULL
          AND ctx.condition_tags IS NOT NULL
          AND ctx.condition_tags != '[]'
        GROUP BY ctx.condition_tags
        ORDER BY total DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\n--- CATEGORY EDGE ---");
    println!(
        "  {:<30} {:>6} {:>6} {:>8} {:>10}",
        "Category", "Total", "Wins", "Win%", "Avg Conf"
    );
    println!("  {}", "-".repeat(64));
    for row in &rows {
        let raw: String = row.get("category");
        let category = serde_json::from_str::<Vec<String>>(&raw)
            .ok()
            .and_then(|v| v.into_iter().next())
            .unwrap_or_else(|| raw.clone());
        let total: i64 = row.get("total");
        let wins: i64 = row.get("wins");
        let avg_conf: f64 = row.try_get("avg_conf").unwrap_or(0.0);
        let win_rate = if total > 0 {
            wins as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  {:<30} {:>6} {:>6} {:>7.1}% {:>9.0}%",
            category,
            total,
            wins,
            win_rate,
            avg_conf * 100.0
        );
    }
    Ok(())
}

async fn print_conviction_calibration(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            conviction_level,
            COUNT(*) as total,
            SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins,
            AVG(confidence) as avg_conf
        FROM agent_episodes
        WHERE is_win IS NOT NULL AND conviction_level != 'NONE'
        GROUP BY conviction_level
        ORDER BY
            CASE conviction_level
                WHEN 'HIGH' THEN 1
                WHEN 'MEDIUM' THEN 2
                WHEN 'LOW' THEN 3
            END
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\n--- CONVICTION CALIBRATION ---");
    println!(
        "  {:<10} {:>6} {:>6} {:>8} {:>10}",
        "Level", "Total", "Wins", "Win%", "Avg Conf"
    );
    println!("  {}", "-".repeat(44));
    for row in &rows {
        let level: String = row.get("conviction_level");
        let total: i64 = row.get("total");
        let wins: i64 = row.get("wins");
        let avg_conf: f64 = row.try_get("avg_conf").unwrap_or(0.0);
        let win_rate = if total > 0 {
            wins as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  {:<10} {:>6} {:>6} {:>7.1}% {:>9.0}%",
            level,
            total,
            wins,
            win_rate,
            avg_conf * 100.0
        );
    }
    Ok(())
}

async fn print_regime_edge(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            regime,
            COUNT(*) as total,
            SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins
        FROM agent_episodes
        WHERE is_win IS NOT NULL
        GROUP BY regime
        ORDER BY total DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\n--- REGIME EDGE ---");
    println!(
        "  {:<20} {:>6} {:>6} {:>8}",
        "Regime", "Total", "Wins", "Win%"
    );
    println!("  {}", "-".repeat(44));
    for row in &rows {
        let regime: String = row.get("regime");
        let total: i64 = row.get("total");
        let wins: i64 = row.get("wins");
        let win_rate = if total > 0 {
            wins as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  {:<20} {:>6} {:>6} {:>7.1}%",
            regime, total, wins, win_rate
        );
    }
    Ok(())
}

async fn print_session_edge(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            session,
            COUNT(*) as total,
            SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins
        FROM agent_episodes
        WHERE is_win IS NOT NULL
        GROUP BY session
        ORDER BY total DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\n--- SESSION EDGE ---");
    println!(
        "  {:<20} {:>6} {:>6} {:>8}",
        "Session", "Total", "Wins", "Win%"
    );
    println!("  {}", "-".repeat(44));
    for row in &rows {
        let session: String = row.get("session");
        let total: i64 = row.get("total");
        let wins: i64 = row.get("wins");
        let win_rate = if total > 0 {
            wins as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  {:<20} {:>6} {:>6} {:>7.1}%",
            session, total, wins, win_rate
        );
    }
    Ok(())
}

async fn print_confidence_curve(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let buckets = [
        ("0-20%", 0.0, 0.20),
        ("20-40%", 0.20, 0.40),
        ("40-60%", 0.40, 0.60),
        ("60-80%", 0.60, 0.80),
        ("80-100%", 0.80, 1.01),
    ];

    println!("\n--- CONFIDENCE CALIBRATION CURVE ---");
    println!(
        "  {:<10} {:>6} {:>6} {:>8} {:>10} {:>10}",
        "Bucket", "Total", "Wins", "Win%", "Avg Conf", "Cal. Err"
    );
    println!("  {}", "-".repeat(56));

    for (label, lo, hi) in &buckets {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins,
                AVG(confidence) as avg_conf
            FROM agent_episodes
            WHERE is_win IS NOT NULL
              AND confidence >= ? AND confidence < ?
            "#,
        )
        .bind(lo)
        .bind(hi)
        .fetch_one(pool)
        .await?;

        let total: i64 = row.get("total");
        let wins: i64 = row.try_get("wins").unwrap_or(0);
        let avg_conf: f64 = row.try_get("avg_conf").unwrap_or(0.0);
        let win_rate = if total > 0 {
            wins as f64 / total as f64
        } else {
            0.0
        };
        let cal_error = (avg_conf - win_rate).abs();

        if total > 0 {
            println!(
                "  {:<10} {:>6} {:>6} {:>7.1}% {:>9.0}% {:>9.2}",
                label,
                total,
                wins,
                win_rate * 100.0,
                avg_conf * 100.0,
                cal_error
            );
        }
    }
    Ok(())
}

async fn print_top_patterns(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            regime || ' | ' || session || ' | ' || COALESCE(
                (SELECT condition_tags FROM episode_market_context WHERE episode_id = agent_episodes.episode_id),
                'no-cat'
            ) as pattern,
            COUNT(*) as total,
            SUM(CASE WHEN is_win = 1 THEN 1 ELSE 0 END) as wins,
            AVG(confidence) as avg_conf
        FROM agent_episodes
        WHERE is_win IS NOT NULL
        GROUP BY pattern
        HAVING total >= 3
        ORDER BY CAST(wins AS REAL) / total DESC, total DESC
        LIMIT 15
        "#
    )
    .fetch_all(pool)
    .await?;

    if !rows.is_empty() {
        println!("\n--- TOP PATTERNS BY WIN RATE (N>=3) ---");
        println!(
            "  {:<50} {:>6} {:>6} {:>8}",
            "Pattern", "Total", "Wins", "Win%"
        );
        println!("  {}", "-".repeat(74));
        for row in &rows {
            let pattern: String = row.get("pattern");
            let total: i64 = row.get("total");
            let wins: i64 = row.get("wins");
            let win_rate = if total > 0 {
                wins as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            let display = if pattern.len() > 48 {
                &pattern[..48]
            } else {
                &pattern
            };
            println!(
                "  {:<50} {:>6} {:>6} {:>7.1}%",
                display, total, wins, win_rate
            );
        }
    }
    Ok(())
}

async fn print_anti_patterns(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            regime || ' | ' || session || ' | conf=' || CAST(CAST(confidence * 100 AS INT) AS TEXT) || '%' as pattern,
            COUNT(*) as total,
            SUM(CASE WHEN is_win = 0 THEN 1 ELSE 0 END) as losses,
            AVG(confidence) as avg_conf
        FROM agent_episodes
        WHERE is_win = 0 AND confidence > 0.6
        GROUP BY pattern
        HAVING total >= 2
        ORDER BY losses DESC
        LIMIT 10
        "#
    )
    .fetch_all(pool)
    .await?;

    if !rows.is_empty() {
        println!("\n--- ANTI-PATTERNS (High Confidence Failures, N>=2) ---");
        println!(
            "  {:<50} {:>6} {:>6} {:>8}",
            "Pattern", "Total", "Losses", "Avg Conf"
        );
        println!("  {}", "-".repeat(74));
        for row in &rows {
            let pattern: String = row.get("pattern");
            let total: i64 = row.get("total");
            let losses: i64 = row.get("losses");
            let avg_conf: f64 = row.try_get("avg_conf").unwrap_or(0.0);
            let display = if pattern.len() > 48 {
                &pattern[..48]
            } else {
                &pattern
            };
            println!(
                "  {:<50} {:>6} {:>6} {:>7.0}%",
                display,
                total,
                losses,
                avg_conf * 100.0
            );
        }
    }
    Ok(())
}

async fn print_knowledge_utility() -> anyhow::Result<()> {
    let scores_path = std::path::Path::new("data/knowledge_utility.json");
    if !scores_path.exists() {
        println!("\n--- KNOWLEDGE UTILITY ---");
        println!("  No utility scores persisted yet.");
        return Ok(());
    }

    let json = std::fs::read_to_string(scores_path)?;
    let scores: std::collections::HashMap<String, f64> = serde_json::from_str(&json)?;

    let mut boosted: Vec<(&String, &f64)> = scores.iter().filter(|(_, v)| **v > 1.01).collect();
    let mut suppressed: Vec<(&String, &f64)> = scores.iter().filter(|(_, v)| **v < 0.99).collect();
    boosted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    suppressed.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

    println!("\n--- KNOWLEDGE UTILITY ---");
    println!("  Total tracked: {}", scores.len());
    println!("  Boosted (>1.0): {}", boosted.len());
    println!("  Suppressed (<1.0): {}", suppressed.len());

    if !boosted.is_empty() {
        println!("\n  Top Boosted Units:");
        for (id, score) in boosted.iter().take(10) {
            println!("    {:<20} {:.3}", id, score);
        }
    }
    if !suppressed.is_empty() {
        println!("\n  Most Suppressed Units:");
        for (id, score) in suppressed.iter().take(10) {
            println!("    {:<20} {:.3}", id, score);
        }
    }
    Ok(())
}

async fn print_lessons_summary(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM experience_replay_lessons")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    println!("\n--- EXPERIENCE REPLAY LESSONS ---");
    println!("  Total lessons: {}", count);

    if count > 0 {
        let rows = sqlx::query(
            "SELECT error_type, heuristic FROM experience_replay_lessons ORDER BY timestamp DESC LIMIT 10"
        )
        .fetch_all(pool)
        .await?;

        println!("\n  Recent Lessons:");
        for row in &rows {
            let error_type: String = row.get("error_type");
            let heuristic: String = row.get("heuristic");
            let display = if heuristic.len() > 100 {
                &heuristic[..100]
            } else {
                &heuristic
            };
            println!("    [{}] {}", error_type, display);
        }
    }
    Ok(())
}

async fn print_semantic_patterns(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT category, condition_value, sample_size, win_rate, profit_factor, is_valid
        FROM semantic_patterns
        ORDER BY profit_factor DESC
        LIMIT 15
        "#,
    )
    .fetch_all(pool)
    .await?;

    if !rows.is_empty() {
        println!("\n--- SEMANTIC PATTERNS ---");
        println!(
            "  {:<20} {:<30} {:>6} {:>8} {:>8} {:>6}",
            "Category", "Condition", "N", "Win%", "PF", "Valid"
        );
        println!("  {}", "-".repeat(82));
        for row in &rows {
            let category: String = row.get("category");
            let condition: String = row.get("condition_value");
            let n: i64 = row.get("sample_size");
            let win_rate: f64 = row.try_get("win_rate").unwrap_or(0.0);
            let pf: f64 = row.try_get("profit_factor").unwrap_or(0.0);
            let valid: bool = row.try_get("is_valid").unwrap_or(false);
            let cond_display = if condition.len() > 28 {
                &condition[..28]
            } else {
                &condition
            };
            println!(
                "  {:<20} {:<30} {:>6} {:>7.1}% {:>7.2} {:>6}",
                category,
                cond_display,
                n,
                win_rate * 100.0,
                pf,
                if valid { "YES" } else { "no" }
            );
        }
    }
    Ok(())
}

async fn print_recent_episodes(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT
            ep.timestamp, ep.action, ep.side, ep.entry_price, ep.confidence,
            ep.is_win, ep.status, ep.regime, ep.session,
            (SELECT condition_tags FROM episode_market_context WHERE episode_id = ep.episode_id) as category
        FROM agent_episodes ep
        ORDER BY ep.timestamp DESC
        LIMIT 20
        "#
    )
    .fetch_all(pool)
    .await?;

    println!("\n--- RECENT EPISODES (last 20) ---");
    println!(
        "  {:<20} {:<6} {:<6} {:>10} {:>6} {:>5} {:<12} {:<10}",
        "Timestamp", "Action", "Side", "Entry", "Conf", "Win?", "Status", "Category"
    );
    println!("  {}", "-".repeat(80));
    for row in &rows {
        let ts: String = row.get("timestamp");
        let action: String = row.get("action");
        let side: Option<String> = row.try_get("side").ok().flatten();
        let entry: f64 = row.try_get("entry_price").unwrap_or(0.0);
        let conf: f64 = row.try_get("confidence").unwrap_or(0.0);
        let is_win: Option<bool> = row.try_get("is_win").ok().flatten();
        let status: String = row.get("status");
        let category: Option<String> = row.try_get("category").ok().flatten();

        let ts_short = if ts.len() > 19 { &ts[..19] } else { &ts };
        let side_str = side.unwrap_or_else(|| "-".to_string());
        let win_str = match is_win {
            Some(true) => "WIN",
            Some(false) => "LOSS",
            None => "-",
        };
        let cat_raw = category.unwrap_or_else(|| "-".to_string());
        let cat_str = serde_json::from_str::<Vec<String>>(&cat_raw)
            .ok()
            .and_then(|v| v.into_iter().next())
            .unwrap_or_else(|| cat_raw.clone());
        let cat_display = if cat_str.len() > 10 {
            &cat_str[..10]
        } else {
            &cat_str
        };

        println!(
            "  {:<20} {:<6} {:<6} {:>10.2} {:>5.0}% {:>5} {:<12} {:<10}",
            ts_short,
            action,
            side_str,
            entry,
            conf * 100.0,
            win_str,
            status,
            cat_display
        );
    }
    Ok(())
}
