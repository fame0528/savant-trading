use std::path::Path;
use tracing::{info, warn};

use savant_trading::core::config::AppConfig;

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

    if args.get(1).map(|s| s.as_str()) == Some("report") {
        return savant_trading::monitor::report::print_report(&config.trading.database_url).await;
    }

    info!("=== SAVANT TRADING ENGINE v0.1.0 ===");

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
