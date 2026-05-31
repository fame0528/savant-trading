use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tracing::{info, warn};

use savant_trading::backtest::engine::{run_backtest, BacktestConfig};
use savant_trading::core::config::AppConfig;
use savant_trading::core::shared::SharedEngineData;
use savant_trading::data::kraken::KrakenClient;

mod api;
mod engine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("savant_trading=info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();

    let config = AppConfig::load(Path::new("config/default.toml")).unwrap_or_else(|e| {
        warn!("Config load failed ({}), using defaults", e);
        AppConfig::default()
    });

    match args.get(1).map(|s| s.as_str()) {
        Some("report") => {
            return savant_trading::monitor::report::print_report(&config.trading.database_url)
                .await;
        }
        Some("--dry-run") => {
            info!("=== SAVANT DRY RUN ===");
            return engine::dry_run(config).await;
        }
        Some("--api-only") => {
            info!("=== SAVANT API SERVER (standalone) ===");
            let shared = SharedEngineData::new();
            let engine_running = Arc::new(AtomicBool::new(false));
            return api::start_server(config, shared, engine_running).await;
        }
        Some("backtest") => {
            info!("=== SAVANT BACKTEST ===");
            return run_backtest_cmd(&config).await;
        }
        Some("sandbox") => {
            info!("=== SAVANT SANDBOX ===");
            return engine::run_sandbox(config).await;
        }
        Some("--help") | Some("-h") => {
            print_help();
            return Ok(());
        }
        _ => {}
    }

    info!("=== SAVANT TRADING ENGINE v0.4.2 ===");
    info!(
        "Mode: {}",
        if config.mode.paper_trading {
            "PAPER"
        } else {
            "LIVE"
        }
    );
    info!("Pairs: {:?}", config.trading.pairs);
    info!("Starting balance: ${:.2}", config.trading.starting_balance);

    // Shared state between engine and API
    let shared = SharedEngineData::new();
    let shared_clone = shared.clone();
    let api_config = config.clone();
    let engine_running = Arc::new(AtomicBool::new(true));
    let engine_running_clone = engine_running.clone();

    // Spawn API server as background task
    tokio::spawn(async move {
        if let Err(e) = api::start_server(api_config, shared_clone, engine_running_clone).await {
            warn!("API server error: {}", e);
        }
    });

    // Run engine as primary task (engine populates SharedEngineData)
    engine::run(config, shared, engine_running).await
}

async fn run_backtest_cmd(config: &AppConfig) -> anyhow::Result<()> {
    let kraken = KrakenClient::new(&config.exchange.rest_url);
    let default_pair = "BTC/USD".to_string();
    let pair = config.trading.pairs.first().unwrap_or(&default_pair);

    info!("Fetching historical candles for {}...", pair);
    let interval = engine::parse_timeframe_minutes(&config.trading.timeframe);
    let candles = kraken
        .get_ohlc(pair, interval, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch candles: {}", e))?;

    info!("Fetched {} candles. Running backtest...", candles.len());

    let bt_config = BacktestConfig {
        starting_balance: config.trading.starting_balance,
        risk_per_trade: config.risk.max_risk_per_trade,
        min_rr_ratio: config.risk.min_rr_ratio,
        fee_rate: config.trading.fee_rate,
        slippage_pct: config.trading.slippage_pct,
        ..Default::default()
    };

    // Use momentum strategy as default for backtesting
    let strategy = savant_trading::strategy::momentum::MomentumStrategy::new(
        config.strategy.momentum.ema_period,
        config.strategy.momentum.volume_spike_multiplier,
        config.strategy.momentum.atr_compression_threshold,
    );

    let result = run_backtest(&candles, &strategy, &bt_config);

    println!("\n=== BACKTEST RESULTS ===");
    println!("{}", result.metrics);
    println!();
    println!("Trades: {}", result.metrics.total_trades);
    println!(
        "Wins: {} | Losses: {}",
        result.metrics.wins, result.metrics.losses
    );
    println!("Win Rate: {:.1}%", result.metrics.win_rate * 100.0);
    println!(
        "Total PnL: ${:.2} ({:.1}%)",
        result.metrics.total_pnl, result.metrics.return_pct
    );
    println!("Profit Factor: {:.2}", result.metrics.profit_factor);
    println!("Sharpe Ratio: {:.2}", result.metrics.sharpe_ratio);
    println!(
        "Max Drawdown: {:.1}%",
        result.metrics.max_drawdown_pct * 100.0
    );
    println!("Final Balance: ${:.2}", result.metrics.final_balance);

    Ok(())
}

fn print_help() {
    println!("SAVANT TRADING ENGINE v0.4.2");
    println!();
    println!("USAGE:");
    println!("  savant                 Start trading engine + API server");
    println!("  savant --dry-run       Run one AI decision cycle and print full pipeline");
    println!("  savant --api-only      Start REST API server only (no engine)");
    println!("  savant backtest        Run backtest on historical data");
    println!("  savant sandbox         Run 50 scenarios through AI brain and grade");
    println!("  savant report          Print performance report");
    println!("  savant --help          Show this help");
    println!();
    println!("API: http://localhost:8080/api/");
    println!("  /api/status           Engine status");
    println!("  /api/portfolio        Account balance and equity");
    println!("  /api/positions        Open positions");
    println!("  /api/trades           Closed trade history");
    println!("  /api/decisions        AI decision log");
    println!("  /api/insight          Market insight data");
    println!("  /api/knowledge        Knowledge base units");
    println!("  /api/risk             Risk metrics");
    println!("  /api/session          Session statistics");
}
