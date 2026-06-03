use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tracing::{info, warn};

use savant_trading::backtest::engine::{run_backtest, BacktestConfig};
use savant_trading::core::config::AppConfig;
use savant_trading::core::shared::SharedEngineData;
use savant_trading::data::kraken::KrakenClient;
use savant_trading::tui::TuiApp;

mod api;
mod engine;

#[derive(Debug, Default)]
struct TestArgs {
    category: Option<String>,
    action_only: bool,
    count: Option<usize>,
    sandbox: bool,
    full: bool,
    historical: bool,
}

fn parse_test_args(args: &[String]) -> TestArgs {
    let mut ta = TestArgs::default();
    let mut i = 2; // skip "savant" and "--test"
    while i < args.len() {
        match args[i].as_str() {
            "--category" | "-c" => {
                i += 1;
                ta.category = args.get(i).cloned();
            }
            "--action-only" | "-a" => {
                ta.action_only = true;
            }
            "--count" | "-n" => {
                i += 1;
                ta.count = args.get(i).and_then(|s| s.parse().ok());
            }
            "--sandbox" | "-s" => {
                ta.sandbox = true;
            }
            "--full" => {
                ta.full = true;
            }
            "--historical" => {
                ta.historical = true;
            }
            _ => {}
        }
        i += 1;
    }
    ta
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_timer(savant_trading::core::console::SavantTimer)
        .with_ansi(true)
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
            if args.get(2).map(|s| s.as_str()) == Some("--test") {
                return savant_trading::monitor::training_report::run_training_report(
                    "sqlite:data/test_memory.db",
                )
                .await;
            }
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
        Some("tick-data") => {
            let data_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "data/kraken_ticks".to_string());
            let pair = args
                .get(3)
                .cloned()
                .unwrap_or_else(|| "BTC/USD".to_string());
            info!(
                "=== PROCESSING TICK DATA ===\nDir: {}\nPair: {}",
                data_dir, pair
            );
            match savant_trading::data::tick_data::process_tick_data(&data_dir, &pair, 5) {
                Ok(dataset) => {
                    println!("Tick data processed successfully:");
                    println!("  Pair: {}", dataset.pair);
                    println!("  Ticks: {}", dataset.tick_count);
                    println!(
                        "  Candles: {} ({}m)",
                        dataset.candle_count, dataset.interval_minutes
                    );
                    println!(
                        "  Date range: {} to {}",
                        chrono::DateTime::from_timestamp(dataset.start_ts, 0)
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        chrono::DateTime::from_timestamp(dataset.end_ts, 0)
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                    );
                    println!(
                        "  Cached to: data/tick_candles_{}_5m.json",
                        pair.replace('/', "_")
                    );
                }
                Err(e) => {
                    eprintln!("Tick data processing failed: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        Some("--test") => {
            let ta = parse_test_args(&args);
            if ta.sandbox {
                info!("=== SAVANT SANDBOX ===");
                return engine::run_sandbox(config).await;
            }
            // Check for --train flag
            if args.iter().any(|a| a == "--train") {
                info!("=== SAVANT TRAINING ===");
                return engine::run_training(
                    config,
                    ta.category,
                    ta.action_only,
                    ta.count,
                    ta.full,
                    ta.historical,
                )
                .await;
            }
            info!("=== SAVANT ACTION TEST ===");
            return engine::run_action_test(config, ta.category, ta.action_only, ta.count).await;
        }
        Some("--tui") => {
            info!("=== SAVANT TRADING ENGINE (TUI MODE) ===");

            let shared = SharedEngineData::new();
            let shared_for_tui = shared.clone();
            let engine_running = Arc::new(AtomicBool::new(true));
            let engine_running_clone = engine_running.clone();

            // Start API server in background
            let api_config = config.clone();
            let api_handle = tokio::spawn(async move {
                if let Err(e) = api::start_server(api_config, shared, engine_running_clone).await {
                    warn!("API server error: {}", e);
                }
            });

            // Run engine in background — store handle for clean shutdown
            let engine_config = config.clone();
            let engine_shared = shared_for_tui.clone();
            let engine_handle = tokio::spawn(async move {
                if let Err(e) = engine::run(engine_config, engine_shared, engine_running).await {
                    warn!("Engine error: {}", e);
                }
            });

            // Run TUI in foreground
            let mut tui = TuiApp::new(shared_for_tui, &config);
            if let Err(e) = tui.run().await {
                warn!("TUI error: {}", e);
            }

            // Shut down engine + API server when TUI exits
            engine_handle.abort();
            api_handle.abort();
            info!("SAVANT TUI closed — engine + API stopped.");
            return Ok(());
        }
        Some("--help") | Some("-h") => {
            print_help();
            return Ok(());
        }
        _ => {}
    }

    info!("=== SAVANT TRADING ENGINE v0.5.0 ===");
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

    let shared = SharedEngineData::new();
    let shared_clone = shared.clone();
    let api_config = config.clone();
    let engine_running = Arc::new(AtomicBool::new(true));
    let engine_running_clone = engine_running.clone();

    tokio::spawn(async move {
        if let Err(e) = api::start_server(api_config, shared_clone, engine_running_clone).await {
            warn!("API server error: {}", e);
        }
    });

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
    println!("SAVANT TRADING ENGINE v0.5.0");
    println!();
    println!("USAGE:");
    println!("  savant                    Start trading engine + API server");
    println!("  savant --tui              Start with full-screen multi-tab TUI");
    println!("  savant --dry-run          One AI decision cycle, full pipeline");
    println!("  savant --api-only         REST API server only");
    println!("  savant backtest           Backtest on historical data");
    println!("  savant report             Performance report");
    println!();
    println!("TESTING:");
    println!("  savant --test                         Run all scenarios (action test)");
    println!("  savant --test -c \"Trend Bull\"        Filter by category");
    println!("  savant --test -a                      Only scenarios expecting Buy/Sell");
    println!("  savant --test -n 20                   Run first N scenarios");
    println!("  savant --test -c \"Crash\" -a -n 10    Combine filters");
    println!("  savant --test --train                 Training mode (5 runs by default)");
    println!("  savant --test --train --full           Full training mode (20 runs)");
    println!("  savant --test --train -a -n 20        Training with filters");
    println!("  savant --test --train --historical     Train on real Kraken historical data");
    println!("  savant --test --sandbox               Legacy sandbox with grading");
    println!();
    println!("API: http://localhost:8080/api/");
    println!("  /status /portfolio /positions /trades /decisions");
    println!("  /insight /knowledge /risk /session /activity /memory");
}
