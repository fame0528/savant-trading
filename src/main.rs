use std::path::Path;
use tracing::{info, warn};

use savant_trading::core::config::AppConfig;

mod api;
mod engine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        Some("--api") => {
            info!("=== SAVANT API SERVER ===");
            let shared = api::SharedEngineData::new();
            return api::start_server(config, shared).await;
        }
        Some("--help") | Some("-h") => {
            print_help();
            return Ok(());
        }
        _ => {}
    }

    info!("=== SAVANT TRADING ENGINE v0.2.0 ===");
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

    engine::run(config).await
}

fn print_help() {
    println!("SAVANT TRADING ENGINE v0.2.0");
    println!();
    println!("USAGE:");
    println!("  savant                 Start trading engine");
    println!("  savant --dry-run       Run one AI decision cycle and print full pipeline");
    println!("  savant --api           Start REST API server for dashboard");
    println!("  savant report          Print performance report");
    println!("  savant --help          Show this help");
}
