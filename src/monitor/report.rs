use crate::core::types::Side;
use crate::monitor::journal::TradeJournal;
use crate::monitor::metrics::PerformanceMetrics;

pub async fn print_report(database_url: &str) -> anyhow::Result<()> {
    let journal = TradeJournal::new(database_url).await?;

    let trades = journal.get_trades(10000).await?;

    if trades.is_empty() {
        println!("No trades recorded yet.");
        return Ok(());
    }

    let metrics = PerformanceMetrics::calculate(&trades);

    println!("=== SAVANT TRADING REPORT ===");
    println!();
    println!("Overall Performance:");
    println!("  Total Trades:  {}", metrics.total_trades);
    println!("  Wins:          {}", metrics.wins);
    println!("  Losses:        {}", metrics.losses);
    println!("  Win Rate:      {:.1}%", metrics.win_rate * 100.0);
    println!("  Total PnL:     ${:.2}", metrics.total_pnl);
    println!("  Avg Win:       ${:.2}", metrics.avg_win);
    println!("  Avg Loss:      ${:.2}", metrics.avg_loss);
    println!("  Profit Factor: {:.2}", metrics.profit_factor);
    println!("  Expectancy:    ${:.2}", metrics.expectancy);
    println!("  Max Drawdown:  {:.1}%", metrics.max_drawdown * 100.0);
    println!();

    let summaries = journal.daily_summary().await?;

    if !summaries.is_empty() {
        println!("Daily Breakdown:");
        println!(
            "  {:<12} {:>6} {:>6} {:>6} {:>8} {:>10}",
            "Day", "Trades", "Wins", "Losses", "Win%", "PnL"
        );
        println!("  {}", "-".repeat(56));
        for s in &summaries {
            println!(
                "  {:<12} {:>6} {:>6} {:>6} {:>7.1}% {:>10.2}",
                s.day,
                s.trades,
                s.wins,
                s.losses,
                s.win_rate * 100.0,
                s.total_pnl
            );
        }
    }

    println!();
    println!("Recent Trades (last 20):");
    println!(
        "  {:<12} {:>6} {:>10} {:>10} {:>10} {:>8}",
        "Pair", "Side", "Entry", "Exit", "PnL", "PnL%"
    );
    println!("  {}", "-".repeat(60));
    for t in trades.iter().take(20) {
        println!(
            "  {:<12} {:>6} {:>10.2} {:>10.2} {:>10.2} {:>7.2}%",
            t.pair,
            if matches!(t.side, Side::Long) {
                "LONG"
            } else {
                "SHORT"
            },
            t.entry_price,
            t.exit_price,
            t.pnl,
            t.pnl_pct
        );
    }

    Ok(())
}
