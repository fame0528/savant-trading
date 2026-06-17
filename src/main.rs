use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::prelude::*;

use savant_trading::backtest::engine::{run_backtest, BacktestConfig};
use savant_trading::core::config::AppConfig;
use savant_trading::core::shared::SharedEngineData;
use savant_trading::data::candle_client::CandleClient;
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
    model: Option<String>,
    managed_keys: bool,
    endpoint: Option<String>,
    api_key_env: Option<String>,
    save_responses: bool,
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
            "--model" | "-m" => {
                i += 1;
                ta.model = args.get(i).cloned();
            }
            "--managed-keys" => {
                ta.managed_keys = true;
            }
            "--endpoint" => {
                i += 1;
                ta.endpoint = args.get(i).cloned();
            }
            "--api-key-env" => {
                i += 1;
                ta.api_key_env = args.get(i).cloned();
            }
            "--save-responses" => {
                ta.save_responses = true;
            }
            _ => {}
        }
        i += 1;
    }
    ta
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // FID-176: log the dotenvy result so silent parse failures (e.g., a var
    // name starting with a digit) are visible at startup. Without this, a
    // malformed .env line aborts loading of ALL vars, and the engine runs
    // with empty API keys — leading to 401 "Invalid token" errors that look
    // like credential issues but are actually .env parser failures.
    match dotenvy::dotenv() {
        Ok(path) => eprintln!("[Savant] Loaded .env from {}", path.display()),
        Err(e) => eprintln!(
            "[Savant] WARNING: .env load FAILED: {}. \
             All API keys may be empty. Check for malformed entries \
             (e.g., var names starting with digits).",
            e
        ),
    }

    // Catch panics and log them before crashing — without this, panics
    // from reqwest/tokio kill the engine silently with exit code 0xffffffff
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else {
            "unknown panic".to_string()
        };
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        eprintln!(
            "{}[Savant Trading]{} {}[PANIC]{} {}{} at {}{}",
            savant_trading::core::console::CYAN_BOLD,
            savant_trading::core::console::RESET,
            savant_trading::core::console::RED_BOLD,
            savant_trading::core::console::RESET,
            savant_trading::core::console::RED_FG,
            msg,
            location,
            savant_trading::core::console::RESET,
        );
    }));

    // Initialize log broadcast channel for dashboard terminal streaming.
    // Must happen BEFORE tracing subscriber so first log lines are captured.
    savant_trading::core::console::init_log_broadcast();

    // Uniform console output — both tracing and savant_log use the same format:
    // [Savant Trading] [MM-DD-YYYY HH:mm AM/PM] [ACTION] [RESULT]
    tracing_subscriber::registry()
        .with(savant_trading::core::console::SavantLayer)
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("savant_trading=info")),
        )
        .init();

    // Set console window title + icon
    savant_trading::core::console::init_console(env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().collect();

    // Parse --config flag early and strip it from args so it doesn't
    // interfere with subcommand matching (serve, --test, --dry-run, etc.)
    let config_path = args
        .iter()
        .position(|a| a == "--config")
        .and_then(|i| args.get(i + 1).map(|p| (i, p.clone())))
        .map(|(_, path)| path)
        .unwrap_or_else(|| "config/default.toml".to_string());

    let args: Vec<String> = args
        .into_iter()
        .scan(false, |skip, a| {
            if *skip {
                *skip = false;
                return Some(None); // skip this arg (the path value)
            }
            if a == "--config" {
                *skip = true;
                return Some(None); // skip the flag itself
            }
            Some(Some(a))
        })
        .flatten()
        .collect();

    let config = AppConfig::load(Path::new(&config_path)).unwrap_or_else(|e| {
        warn!("Config load failed from '{}' ({}), using defaults", config_path, e);
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
        Some("--liquidate") => {
            return emergency_liquidate().await;
        }
        Some("recover") => {
            info!("=== SAVANT RECOVER — Scan wallet, restore positions to DB ===");
            return recover_positions(&config).await;
        }
        Some("close-all") => {
            info!("=== SAVANT CLOSE ALL — Sell all positions to USDC ===");
            return close_all_positions(&config).await;
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
                return engine::run_sandbox(
                    config,
                    ta.model,
                    ta.managed_keys,
                    ta.endpoint,
                    ta.api_key_env,
                    ta.save_responses,
                )
                .await;
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
                    ta.model,
                    ta.managed_keys,
                    ta.save_responses,
                )
                .await;
            }
            info!("=== SAVANT ACTION TEST ===");
            return engine::run_action_test(
                config,
                ta.category,
                ta.action_only,
                ta.count,
                ta.model,
                ta.managed_keys,
            )
            .await;
        }
        Some("--live-test") => {
            let ta = parse_test_args(&args);
            let pairs_override: Vec<String> = args
                .iter()
                .position(|a| a == "--pairs")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
                .unwrap_or_default();
            let show_prompt = args.iter().any(|a| a == "--show-prompt");
            info!("=== SAVANT LIVE SITUATION TEST ===");
            return engine::run_live_test(config, ta.model, pairs_override, show_prompt).await;
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
        Some("serve") => {
            // Clear circuit breaker block file on fresh start
            if std::path::Path::new("savant.blocked").exists() {
                let _ = std::fs::remove_file("savant.blocked");
                info!("Cleared savant.blocked from previous session");
            }
            info!("=== SAVANT — Engine + Dashboard ===");
            let shared = SharedEngineData::new();
            let engine_running = Arc::new(AtomicBool::new(true));

            // On Windows, npm is a .ps1 script — use cmd /c to invoke it
            #[cfg(target_os = "windows")]
            fn npm_cmd() -> std::process::Command {
                let mut cmd = std::process::Command::new("cmd");
                cmd.arg("/c").arg("npm");
                cmd
            }
            #[cfg(not(target_os = "windows"))]
            fn npm_cmd() -> std::process::Command {
                std::process::Command::new("npm")
            }

            // 1. Build dashboard if needed (blocking, before engine starts)
            let dashboard_dir = std::path::Path::new("dashboard");
            if !dashboard_dir.join(".next").exists() {
                info!("Dashboard not built — running npm run build...");
                match npm_cmd()
                    .args(["run", "build"])
                    .current_dir(dashboard_dir)
                    .status()
                {
                    Ok(s) if s.success() => info!("Dashboard built successfully"),
                    Ok(s) => warn!("Dashboard build exited with: {}", s),
                    Err(e) => warn!("Failed to build dashboard: {} (is Node.js installed?)", e),
                }
            }

            // 2. Start dashboard in background with crash detection (FID-093 C3)
            info!("Starting dashboard on http://localhost:3000");
            let dashboard_dir_clone = dashboard_dir.to_path_buf();
            tokio::spawn(async move {
                loop {
                    info!("Spawning dashboard process...");
                    let mut child = match npm_cmd()
                        .args(["start"])
                        .current_dir(&dashboard_dir_clone)
                        .spawn()
                    {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Failed to start dashboard: {} — trying `npm run dev`", e);
                            match npm_cmd()
                                .args(["run", "dev"])
                                .current_dir(&dashboard_dir_clone)
                                .spawn()
                            {
                                Ok(c) => c,
                                Err(e2) => {
                                    error!("Failed to start dashboard: {} — retrying in 30s", e2);
                                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                                    continue;
                                }
                            }
                        }
                    };
                    // Monitor: wait for process to exit
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                warn!("Dashboard process exited with status: {} — restarting in 5s", status);
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                break; // Break inner loop to restart
                            }
                            Ok(None) => { /* still running */ }
                            Err(e) => {
                                error!("Error checking dashboard status: {} — restarting in 10s", e);
                                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                                break;
                            }
                        }
                    }
                }
            });

            // 3. Start API server in background
            let api_config = config.clone();
            let api_shared = shared.clone();
            let api_running = engine_running.clone();
            tokio::spawn(async move {
                if let Err(e) = api::start_server(api_config, api_shared, api_running).await {
                    warn!("API server error: {}", e);
                }
            });

            info!("API server on http://localhost:8080");
            info!("Press Ctrl+C to stop everything");

            // 4. Run engine in FOREGROUND — panics are visible, not swallowed
            let engine_config = config.clone();
            let engine_shared = shared.clone();
            let engine_flag = engine_running.clone();
            engine::run(engine_config, engine_shared, engine_flag).await?;

            info!("SAVANT stopped.");
            return Ok(());
        }
        _ => {}
    }

    info!("=== SAVANT TRADING ENGINE v{} ===", env!("CARGO_PKG_VERSION"));
    info!(
        "Mode: {}",
        if !config.mode.live_execution {
            "SIMULATED"
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
    let candle_api = CandleClient::new(&config.exchange.rest_url);
    let default_pair = "BTC/USD".to_string();
    let pair = config.trading.pairs.first().unwrap_or(&default_pair);

    info!("Fetching historical candles for {}...", pair);
    let interval = engine::parse_timeframe_minutes(&config.trading.timeframe);
    let candles = candle_api
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
    println!("SAVANT TRADING ENGINE v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("  savant                    Start trading engine + API server");
    println!("  savant serve              Start engine + API + dashboard (single command)");
    println!("  savant --tui              Start with full-screen multi-tab TUI");
    println!("  savant --dry-run          One AI decision cycle, full pipeline");
    println!("  savant --api-only         REST API server only");
    println!("  savant backtest           Backtest on historical data");
    println!("  savant report             Performance report");
    println!();
    println!("OPTIONS:");
    println!("  --config <path>           Use custom config file (default: config/default.toml)");
    println!();
    println!("DASHBOARD: http://localhost:3000  (requires `savant serve`)");
    println!("API:       http://localhost:8080/api/");
    println!("  /status /portfolio /positions /trades /decisions");
    println!("  /insight /knowledge /risk /session /activity /memory");
}

/// Emergency liquidation — reads dex_state.json and closes all open positions.
/// Used when the engine crashes while holding positions.
async fn emergency_liquidate() -> anyhow::Result<()> {
    use savant_trading::execution::dex::zero_x::ZeroXBackend;
    use savant_trading::execution::dex::DexTrader;
    use savant_trading::execution::engine::ExecutionEngine;

    let state_path = std::path::Path::new("data/dex_state.json");
    if !state_path.exists() {
        println!("No dex_state.json found — nothing to liquidate.");
        return Ok(());
    }

    let json = std::fs::read_to_string(state_path)?;
    let state: serde_json::Value = serde_json::from_str(&json)?;
    let positions = state["positions"].as_array();

    match positions {
        Some(pos) if pos.is_empty() => {
            println!("No open positions in dex_state.json — nothing to liquidate.");
            return Ok(());
        }
        Some(pos) => {
            println!("Found {} open position(s). Liquidating...", pos.len());
        }
        None => {
            println!("No positions field in dex_state.json — nothing to liquidate.");
            return Ok(());
        }
    }

    // Load config and create executor
    let config_path = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--config")
        .and_then(|w| w.get(1))
        .cloned()
        .unwrap_or_else(|| "config/default.toml".to_string());
    let config =
        savant_trading::core::config::AppConfig::load(std::path::Path::new(&config_path))?;
    let wallet_key = std::env::var(&config.exchange.dex.wallet_key_env)?;
    let api_key = std::env::var(&config.exchange.dex.api_key_env)?;

    let signing_key = {
        let key_hex = wallet_key.trim_start_matches("0x");
        let key_bytes = alloy_core::primitives::hex::decode(key_hex)
            .map_err(|e| anyhow::anyhow!("Invalid wallet key hex: {}", e))?;
        k256::ecdsa::SigningKey::from_bytes(key_bytes.as_slice().into())
            .map_err(|e| anyhow::anyhow!("Invalid wallet key for signing: {}", e))?
    };
    let backend = ZeroXBackend::new(api_key, signing_key);
    let mut trader = DexTrader::new(
        backend,
        &wallet_key,
        &config.exchange.dex.rpc_url,
        config.exchange.dex.chain_id,
        config.exchange.dex.slippage_pct,
        config.trading.starting_balance,
    )
    .await?;

    // Close all positions
    let positions: Vec<String> = trader
        .open_positions()
        .iter()
        .map(|p| p.id.clone())
        .collect();
    for pos_id in &positions {
        println!("Closing position {}...", pos_id);
        match trader.close_position(pos_id).await {
            Ok(order) => {
                println!(
                    "  Closed: {} @ {:.4}",
                    pos_id,
                    order.filled_price.unwrap_or(0.0)
                );
            }
            Err(e) => {
                println!("  Failed to close {}: {}", pos_id, e);
            }
        }
    }

    println!(
        "Liquidation complete. Final balance: ${:.2}",
        trader.balance()
    );
    Ok(())
}

/// Recover on-chain positions into the engine's SQLite DB.
/// Scans wallet for non-zero token balances and creates Position records
/// so the engine can see and manage them on startup.
async fn recover_positions(config: &savant_trading::core::config::AppConfig) -> anyhow::Result<()> {
    use alloy_core::primitives::{hex, Address, Keccak256, U256};
    use k256::ecdsa::SigningKey;
    use savant_trading::core::types::{Position, ScaleLevel, Side};
    use savant_trading::monitor::journal::TradeJournal;

    // Derive wallet address from private key
    let private_key = std::env::var(&config.exchange.dex.wallet_key_env)
        .map_err(|_| anyhow::anyhow!("WALLET_PRIVATE_KEY not set"))?;

    let key_bytes = hex::decode(private_key.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("Invalid key hex: {}", e))?;
    let signing_key =
        SigningKey::from_slice(&key_bytes).map_err(|e| anyhow::anyhow!("Invalid key: {}", e))?;
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false).to_bytes().to_vec();
    let hash = {
        let mut h = Keccak256::new();
        h.update(&encoded[1..]);
        h.finalize()
    };
    let addr_bytes: [u8; 20] = hash[12..32].try_into()?;
    let wallet_address = Address::from(addr_bytes);
    let wallet_hex = format!("{:#x}", wallet_address);
    println!("Wallet: {}", wallet_hex);

    // Known Arbitrum token holdings to check
    struct TokenInfo {
        symbol: &'static str,
        address: &'static str,
        decimals: u64,
    }

    let tokens = vec![
        TokenInfo {
            symbol: "AAVE",
            address: "0xba5ddd1f9d7f570dc94a51479a000e3bce967196",
            decimals: 18,
        },
        TokenInfo {
            symbol: "FLUID",
            address: "0x61E030A56D33e8260FdD81f03B162A79Fe3449Cd",
            decimals: 18,
        },
        TokenInfo {
            symbol: "VANA",
            address: "0x7FF7Fa94b8b66Ef313f7970d4EEbd2CB3103a2C0",
            decimals: 18,
        },
        TokenInfo {
            symbol: "WETH",
            address: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
            decimals: 18,
        },
        TokenInfo {
            symbol: "ARB",
            address: "0x912CE59144191C1204E64559FE8253a0e49E6548",
            decimals: 18,
        },
    ];

    let rpc_url = &config.exchange.dex.rpc_url;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let wallet_addr_clean = wallet_hex.trim_start_matches("0x");
    let balance_of_data = format!("0x70a08231{:0>64}", wallet_addr_clean);

    let mut recovered = Vec::new();

    for token in &tokens {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_call",
            "params": [{"to": token.address, "data": &balance_of_data}, "latest"]
        });

        let resp: serde_json::Value = client
            .post(rpc_url)
            .header("User-Agent", "Mozilla/5.0")
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if let Some(hex_val) = resp.get("result").and_then(|r| r.as_str()) {
            let raw =
                U256::from_str_radix(hex_val.trim_start_matches("0x"), 16).unwrap_or(U256::ZERO);
            if raw.is_zero() {
                continue;
            }
            let divisor = 10f64.powi(token.decimals as i32);
            let amount: f64 = raw.to_string().parse().unwrap_or(0.0) / divisor;
            if amount < 0.0001 {
                continue;
            }

            // Get price from CoinGecko
            let cg_id = match token.symbol {
                "AAVE" => "aave",
                "FLUID" => "instadapp",
                "VANA" => "vana",
                "WETH" => "weth",
                "ARB" => "arbitrum",
                _ => continue,
            };
            let price = match client
                .get(format!(
                    "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
                    cg_id
                ))
                .header("User-Agent", "Mozilla/5.0")
                .send()
                .await
            {
                Ok(r) => r
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|d| d.get(cg_id)?.get("usd")?.as_f64())
                    .unwrap_or(0.0),
                Err(_) => 0.0,
            };

            let value = amount * price;
            println!(
                "  {} {}: {:.6} @ ${:.4} = ${:.2}",
                token.symbol, token.address, amount, price, value
            );

            if value < 0.01 {
                println!("    Skipping — dust value");
                continue;
            }

            let pos = Position {
                id: format!(
                    "recover-{}-{}",
                    token.symbol.to_lowercase(),
                    chrono::Utc::now().timestamp()
                ),
                pair: format!("{}/USD", token.symbol),
                side: Side::Long,
                entry_price: price, // Use current price as entry (conservative)
                current_price: price,
                quantity: amount,
                stop_loss: 0.0, // No stop — user needs to set manually or engine will set
                take_profit_1: 0.0,
                take_profit_2: 0.0,
                take_profit_3: 0.0,
                unrealized_pnl: 0.0,
                risk_amount: 0.0,
                strategy_name: "recovered".to_string(),
                opened_at: chrono::Utc::now(),
                scale_level: ScaleLevel::Full,
                token_address: savant_trading::execution::dex::lookup_token(token.symbol, config.exchange.dex.chain_id).map(|(addr, _)| addr).unwrap_or_default(),
            };
            recovered.push((pos, value));
        }
    }

    if recovered.is_empty() {
        println!("No token holdings found to recover.");
        return Ok(());
    }

    // Save to DB
    let journal = TradeJournal::new(&config.trading.database_url).await?;
    let total: f64 = recovered.iter().map(|(_, v)| *v).sum();

    println!(
        "\nRecovering {} positions (${:.2} total) to DB...",
        recovered.len(),
        total
    );
    for (pos, value) in &recovered {
        journal.save_position(pos).await?;
        let _ = journal
            .record_activity(
                "Trade",
                &pos.pair,
                &format!(
                    "RECOVERED {} {:.6} @ ${:.4} = ${:.2}",
                    pos.side, pos.quantity, pos.current_price, value
                ),
            )
            .await;
        println!("  Saved: {} {} {:.6}", pos.pair, pos.side, pos.quantity);
    }

    println!(
        "\nDone. {} positions saved to DB. Start the engine to manage them.",
        recovered.len()
    );
    println!("  The engine will load these positions on startup.");
    println!("  To sell everything to USDC: cargo run --release -- close-all");
    println!("  To start the engine: cargo run --release");
    Ok(())
}

const PERMIT2: &str = "0x000000000022d473030f116ddee9f6b43ac78ba3";

/// Check ERC-20 allowance to Permit2 for a given token and owner.
/// Returns U256 to handle max approvals correctly.
async fn check_allowance(
    rpc_url: &str,
    token: &alloy_core::primitives::Address,
    owner: &alloy_core::primitives::Address,
) -> alloy_core::primitives::U256 {
    let owner_hex = format!("{:x}", owner);
    let spender_hex = PERMIT2.trim_start_matches("0x");
    let data = format!("0xdd62ed3e{:0>64}{:0>64}", owner_hex, spender_hex);
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "eth_call",
        "params": [{"to": format!("{:x}", token), "data": data}, "latest"]
    });
    let client = reqwest::Client::new();
    if let Ok(resp) = client
        .post(rpc_url)
        .header("User-Agent", "Mozilla/5.0")
        .json(&body)
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(hex) = json.get("result").and_then(|r| r.as_str()) {
                return alloy_core::primitives::U256::from_str_radix(
                    hex.trim_start_matches("0x"),
                    16,
                )
                .unwrap_or(alloy_core::primitives::U256::ZERO);
            }
        }
    }
    alloy_core::primitives::U256::ZERO
}

/// Check transaction receipt status. Returns Some("0x1") if confirmed, Some("0x0") if reverted.
async fn check_tx_status(rpc_url: &str, tx_hash: &str) -> Option<String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash]
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(rpc_url)
        .header("User-Agent", "Mozilla/5.0")
        .json(&body)
        .send()
        .await
        .ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("result")
        .and_then(|r| r.get("status"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

/// Close all positions — sell all token holdings to USDC via 0x API.
/// Uses ZeroXBackend::build_swap_tx for proper Permit2 signing.
async fn close_all_positions(
    config: &savant_trading::core::config::AppConfig,
) -> anyhow::Result<()> {
    use alloy_core::primitives::{Address, U256};
    use k256::ecdsa::SigningKey;
    use savant_trading::execution::dex::zero_x::ZeroXBackend;
    use savant_trading::execution::dex::{DexTrader, SwapParams};
    use savant_trading::monitor::journal::TradeJournal;

    let wallet_key = std::env::var(&config.exchange.dex.wallet_key_env)
        .map_err(|_| anyhow::anyhow!("WALLET_PRIVATE_KEY not set"))?;
    let api_key = std::env::var(&config.exchange.dex.api_key_env)
        .map_err(|_| anyhow::anyhow!("ZEROEX_API_KEY not set"))?;

    let signing_key = {
        let key_hex = wallet_key.trim_start_matches("0x");
        let key_bytes = alloy_core::primitives::hex::decode(key_hex)
            .map_err(|e| anyhow::anyhow!("Invalid wallet key hex: {}", e))?;
        SigningKey::from_bytes(key_bytes.as_slice().into())
            .map_err(|e| anyhow::anyhow!("Invalid wallet key for signing: {}", e))?
    };
    let backend = ZeroXBackend::new(api_key.clone(), signing_key);
    let trader = DexTrader::new(
        backend,
        &wallet_key,
        &config.exchange.dex.rpc_url,
        config.exchange.dex.chain_id,
        config.exchange.dex.slippage_pct,
        config.trading.starting_balance,
    )
    .await?;

    let wallet_hex = format!("{:#x}", trader.wallet_address());
    let usdc_address = savant_trading::execution::dex::usdc_address_for_chain(config.exchange.dex.chain_id)
        .ok_or_else(|| anyhow::anyhow!("No USDC address for chain {}", config.exchange.dex.chain_id))?;

    let journal = TradeJournal::new(&config.trading.database_url).await?;
    let positions = journal.load_positions().await?;

    if positions.is_empty() {
        println!("No positions in DB. Run 'savant recover' first.");
        return Ok(());
    }

    // Deduplicate by pair (recover may have created duplicates)
    let mut seen = std::collections::HashSet::new();
    let positions: Vec<_> = positions
        .into_iter()
        .filter(|p| seen.insert(p.pair.clone()))
        .collect();

    println!("Wallet: {}", wallet_hex);
    println!("Closing {} unique positions to USDC...", positions.len());
    let mut total_received: f64 = 0.0;

    for pos in &positions {
        if pos.quantity <= 0.0001 {
            println!("  Skipping {} — dust", pos.pair);
            continue;
        }

        let token_symbol = pos.pair.split('/').next().unwrap_or("");

        // Look up token address directly
        let (token_address, decimals) = match savant_trading::execution::dex::lookup_token(
            token_symbol,
            config.exchange.dex.chain_id,
        ) {
            Some((addr, dec)) => (addr, dec as u32),
            None => {
                println!("  Skipping {} — not in token DB", token_symbol);
                continue;
            }
        };

        if token_address.to_lowercase() == usdc_address.to_lowercase() {
            println!("  Skipping {} — already USDC", token_symbol);
            continue;
        }

        let sell_usd = pos.quantity * pos.current_price;

        println!(
            "\n  {} {:.6} {} (~${:.2})",
            pos.side, pos.quantity, token_symbol, sell_usd
        );
        println!("    sellToken: {}", token_address);
        println!("    buyToken:  {}", usdc_address);

        // Split large sells into chunks of max 5 tokens
        let max_chunk = 5.0f64;
        let chunks = if pos.quantity > max_chunk {
            let mut c = Vec::new();
            let mut remaining = pos.quantity;
            while remaining > 0.001 {
                let chunk = remaining.min(max_chunk);
                c.push(chunk);
                remaining -= chunk;
            }
            c
        } else {
            vec![pos.quantity]
        };

        for (i, chunk_qty) in chunks.iter().enumerate() {
            let chunk_wei =
                savant_trading::execution::dex::amount_to_wei(*chunk_qty, decimals as u8);
            let chunk_label = if chunks.len() > 1 {
                format!(" ({}/{})", i + 1, chunks.len())
            } else {
                String::new()
            };

            println!(
                "    Chunk{}: {:.6} {} (wei={})",
                chunk_label, chunk_qty, token_symbol, chunk_wei
            );

            // Ensure Permit2 approval is valid before each swap attempt
            let token_addr: Address = token_address.parse().unwrap_or_default();
            let allowance = check_allowance(
                &config.exchange.dex.rpc_url,
                &token_addr,
                &trader.wallet_address(),
            )
            .await;
            let needed = alloy_core::primitives::U256::from_str_radix(&chunk_wei, 10)
                .unwrap_or(alloy_core::primitives::U256::ZERO);
            if allowance < needed {
                println!("    Approving for Permit2 (allowance < needed)...");
                let approve_data = format!(
                    "0x095ea7b3{:0>64}{:0>64}",
                    PERMIT2.trim_start_matches("0x"),
                    "f".repeat(64)
                );
                let approve_bytes =
                    alloy_core::primitives::hex::decode(approve_data.trim_start_matches("0x"))
                        .unwrap_or_default();
                match trader
                    .sign_and_send(token_addr, &approve_bytes, U256::ZERO, 60000)
                    .await
                {
                    Ok((hash, _receipt)) => {
                        println!("    Approve TX: {}", hash);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    Err(e) => println!("    Approve error (may be OK): {}", e),
                }
            } else {
                println!("    Permit2 allowance OK");
            }

            let swap_params = SwapParams {
                src_token: token_address.clone(),
                dst_token: usdc_address.to_string(),
                amount: chunk_wei.clone(),
                slippage: config.exchange.dex.slippage_pct,
                from: wallet_hex.clone(),
                chain_id: config.exchange.dex.chain_id,
                sell_entire_balance: false,
            };

            match trader.build_swap_tx(&swap_params).await {
                Ok(swap_tx) => {
                    println!("    Quote OK — gas={}", swap_tx.gas);

                    let swap_to: Address = swap_tx.to.parse().unwrap_or_default();
                    let swap_data =
                        alloy_core::primitives::hex::decode(swap_tx.data.trim_start_matches("0x"))
                            .unwrap_or_default();
                    let swap_value =
                        U256::from_str_radix(swap_tx.value.trim_start_matches("0x"), 16)
                            .unwrap_or(U256::ZERO);

                    println!("    Sending swap...");
                    match trader
                        .sign_and_send(swap_to, &swap_data, swap_value, swap_tx.gas)
                        .await
                    {
                        Ok((tx_hash, _receipt)) => {
                            println!("    TX: {}", tx_hash);
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

                            if let Some(status) =
                                check_tx_status(&config.exchange.dex.rpc_url, &tx_hash).await
                            {
                                if status == "0x1" {
                                    println!("    CONFIRMED");
                                    total_received += sell_usd * (*chunk_qty / pos.quantity);
                                } else {
                                    println!("    REVERTED (status={})", status);
                                }
                            }
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            if err_str.contains("TRANSFER_FROM_FAILED")
                                || err_str.contains("reverted")
                            {
                                println!("    Swap reverted — re-approving and retrying...");
                                // Force re-approval
                                let approve_data = format!(
                                    "0x095ea7b3{:0>64}{:0>64}",
                                    PERMIT2.trim_start_matches("0x"),
                                    "f".repeat(64)
                                );
                                let approve_bytes = alloy_core::primitives::hex::decode(
                                    approve_data.trim_start_matches("0x"),
                                )
                                .unwrap_or_default();
                                if let Ok((hash, _receipt)) = trader
                                    .sign_and_send(token_addr, &approve_bytes, U256::ZERO, 60000)
                                    .await
                                {
                                    println!("    Re-approve TX: {}", hash);
                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                    // Retry swap
                                    match trader.build_swap_tx(&swap_params).await {
                                        Ok(retry_tx) => {
                                            let retry_to: Address =
                                                retry_tx.to.parse().unwrap_or_default();
                                            let retry_data = alloy_core::primitives::hex::decode(
                                                retry_tx.data.trim_start_matches("0x"),
                                            )
                                            .unwrap_or_default();
                                            let retry_value = U256::from_str_radix(
                                                retry_tx.value.trim_start_matches("0x"),
                                                16,
                                            )
                                            .unwrap_or(U256::ZERO);
                                            match trader
                                                .sign_and_send(
                                                    retry_to,
                                                    &retry_data,
                                                    retry_value,
                                                    retry_tx.gas,
                                                )
                                                .await
                                            {
                                                Ok((retry_hash, _receipt)) => {
                                                    println!("    Retry TX: {}", retry_hash);
                                                    tokio::time::sleep(
                                                        std::time::Duration::from_secs(10),
                                                    )
                                                    .await;
                                                    if let Some(s) = check_tx_status(
                                                        &config.exchange.dex.rpc_url,
                                                        &retry_hash,
                                                    )
                                                    .await
                                                    {
                                                        if s == "0x1" {
                                                            println!("    CONFIRMED (retry)");
                                                            total_received += sell_usd
                                                                * (*chunk_qty / pos.quantity);
                                                        } else {
                                                            println!(
                                                                "    REVERTED (retry, status={})",
                                                                s
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(e2) => println!("    Retry failed: {}", e2),
                                            }
                                        }
                                        Err(e2) => println!("    Retry quote failed: {}", e2),
                                    }
                                }
                            } else {
                                println!("    Send failed: {}", e);
                            }
                        }
                    }
                }
                Err(e) => println!("    Quote failed: {}", e),
            }

            // Brief pause between chunks
            if i < chunks.len() - 1 {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }

        // Delete from DB after all chunks complete
        let _ = journal.delete_position(&pos.id).await;
    }

    println!("\n=== DONE ===");
    println!("Total sold: ~${:.2}", total_received);

    // Final USDC balance
    let usdc_data = format!(
        "0x70a08231{:0>64}",
        wallet_hex.trim_start_matches("0x").to_lowercase()
    );
    let client = reqwest::Client::new();
    let usdc_body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "eth_call",
        "params": [{"to": usdc_address, "data": usdc_data}, "latest"]
    });
    if let Ok(resp) = client
        .post(&config.exchange.dex.rpc_url)
        .header("User-Agent", "Mozilla/5.0")
        .json(&usdc_body)
        .send()
        .await
    {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(hex_val) = json.get("result").and_then(|r| r.as_str()) {
                let bal = U256::from_str_radix(hex_val.trim_start_matches("0x"), 16)
                    .unwrap_or(U256::ZERO);
                let usdc_bal: f64 = bal.to_string().parse().unwrap_or(0.0) / 1e6;
                println!("USDC balance: ${:.2}", usdc_bal);
            }
        }
    }

    println!("\nNext: cargo run --release (clean start)");
    Ok(())
}
