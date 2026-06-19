use tracing::{info, warn};

use savant_trading::agent::knowledge::KnowledgeBase;
use savant_trading::core::config::AppConfig;
use savant_trading::execution::dex::inch::InchBackend;
use savant_trading::execution::dex::zero_x::ZeroXBackend;
use savant_trading::execution::dex::DexTrader;
use savant_trading::execution::engine::ExecutionEngine;

pub fn parse_timeframe(tf: &str) -> u64 {
    match tf {
        "1m" => 60,
        "5m" => 300,
        "15m" => 900,
        "1h" => 3600,
        "4h" => 14400,
        "1d" => 86400,
        _ => 300,
    }
}

pub fn parse_timeframe_minutes(tf: &str) -> u32 {
    match tf {
        "1m" => 1,
        "5m" => 5,
        "15m" => 15,
        "1h" => 60,
        "4h" => 240,
        "1d" => 1440,
        _ => 5,
    }
}

/// FID-093 C1: Derive wallet address from private key hex.
/// Used to cache the address at startup for API serving.
pub(super) fn derive_address_from_key(private_key_hex: &str) -> Result<String, String> {
    use alloy_core::primitives::{hex, Address, Keccak256};
    use k256::ecdsa::SigningKey;

    let hex_key = private_key_hex.trim_start_matches("0x");
    let key_bytes = hex::decode(hex_key).map_err(|e| format!("Invalid hex: {}", e))?;
    let signing_key =
        SigningKey::from_slice(&key_bytes).map_err(|e| format!("Invalid key: {}", e))?;
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false).to_bytes().to_vec();
    let mut hasher = Keccak256::new();
    hasher.update(&encoded[1..]);
    let hash = hasher.finalize();
    let addr_bytes: [u8; 20] = hash[12..32]
        .try_into()
        .map_err(|_| "Failed to derive address".to_string())?;
    let address = Address::from(addr_bytes);
    Ok(format!("{:#x}", address))
}

/// Create a live execution engine based on config mode + backend.
///
/// Returns `None` for simulated mode (`live_execution: false`).
/// Otherwise creates the appropriate backend:
///   - `"0x"`     → [`DexTrader<ZeroXBackend>`] (requires WALLET_PRIVATE_KEY + ZEROEX_API_KEY)
///   - `"1inch"`  → [`DexTrader<InchBackend>`] (requires WALLET_PRIVATE_KEY + 1INCH_API_KEY)
pub(super) async fn create_executor(
    config: &AppConfig,
) -> Result<Option<Box<dyn ExecutionEngine>>, anyhow::Error> {
    if !config.mode.live_execution {
        info!("portfolio trading mode: using PortfolioManager");
        return Ok(None);
    }

    match config.exchange.backend.as_str() {
        "0x" => {
            let wallet_key = std::env::var(&config.exchange.dex.wallet_key_env).map_err(|_| {
                anyhow::anyhow!(
                    "{} not set — required for 0x DEX trading",
                    config.exchange.dex.wallet_key_env
                )
            })?;
            let api_key = std::env::var(&config.exchange.dex.api_key_env).map_err(|_| {
                anyhow::anyhow!(
                    "{} not set — required for 0x API",
                    config.exchange.dex.api_key_env
                )
            })?;

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
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create DexTrader (0x): {}", e))?;

            // FID-209: Enable Anvil spread filter bypass if configured.
            // MUST be called before the first execute_swap, after DexTrader::new.
            trader.set_is_anvil(config.mode.is_anvil);

            // Register additional chains from config (FID-045)
            for chain_cfg in config.chains.values() {
                if chain_cfg.enabled && chain_cfg.chain_id != config.exchange.dex.chain_id {
                    info!(
                        "Registering chain: {} (id={})",
                        chain_cfg.name, chain_cfg.chain_id
                    );
                    let chain_config = savant_trading::execution::dex::ChainConfig {
                        chain_id: chain_cfg.chain_id,
                        name: Box::leak(chain_cfg.name.clone().into_boxed_str()),
                        rpc_url: chain_cfg.rpc_url.clone(),
                        native_token: Box::leak(chain_cfg.native_token.clone().into_boxed_str()),
                        min_gas_native: chain_cfg.min_gas_native,
                        slippage_pct: chain_cfg.slippage_pct,
                    };
                    trader.add_chain(chain_config);
                }
            }

            info!(
                "LIVE trading mode: DexTrader (0x) initialized on chain {} ({} total chains)",
                config.exchange.dex.chain_id,
                trader.chain_ids().len()
            );
            Ok(Some(Box::new(trader)))
        }
        "1inch" => {
            let wallet_key = std::env::var(&config.exchange.dex.wallet_key_env).map_err(|_| {
                anyhow::anyhow!(
                    "{} not set — required for 1inch DEX trading",
                    config.exchange.dex.wallet_key_env
                )
            })?;
            let api_key = std::env::var(&config.exchange.dex.api_key_env).map_err(|_| {
                anyhow::anyhow!(
                    "{} not set — required for 1inch API",
                    config.exchange.dex.api_key_env
                )
            })?;

            let backend = InchBackend::new(api_key);
            let mut trader = DexTrader::new(
                backend,
                &wallet_key,
                &config.exchange.dex.rpc_url,
                config.exchange.dex.chain_id,
                config.exchange.dex.slippage_pct,
                config.trading.starting_balance,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create DexTrader (1inch): {}", e))?;

            // FID-209: Enable Anvil spread filter bypass if configured.
            trader.set_is_anvil(config.mode.is_anvil);

            info!(
                "LIVE trading mode: DexTrader (1inch) initialized on chain {}",
                config.exchange.dex.chain_id
            );
            Ok(Some(Box::new(trader)))
        }
        other => Err(anyhow::anyhow!("Unknown exchange backend '{}'", other)),
    }
}

pub(super) fn load_knowledge_base() -> KnowledgeBase {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let knowledge_root = manifest_dir.join("knowledge");
    let knowledge_src = manifest_dir.join("src").join("agent").join("knowledge");

    let knowledge_dir = if knowledge_root.exists() {
        knowledge_root
    } else {
        warn!(
            "knowledge/ not found at {:?}, falling back to src/agent/knowledge/",
            manifest_dir
        );
        knowledge_src
    };

    info!("Loading knowledge from {:?}", knowledge_dir);

    let mut all_units = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&knowledge_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match std::fs::read_to_string(&path) {
                    Ok(json) => match KnowledgeBase::from_json(&json) {
                        Ok(kb) => {
                            let count = kb.len();
                            all_units.extend_from_slice(kb.all());
                            info!(
                                "Loaded {} knowledge units from {:?}",
                                count,
                                path.file_name()
                            );
                        }
                        Err(e) => warn!("Failed to parse {:?}: {}", path.file_name(), e),
                    },
                    Err(e) => warn!("Failed to read {:?}: {}", path.file_name(), e),
                }
            }
        }
    }

    info!("Knowledge base loaded: {} total units", all_units.len());
    let mut kb = KnowledgeBase::new(all_units);

    // Load persisted utility scores if available
    let scores_path = std::path::Path::new("data/knowledge_utility.json");
    if let Err(e) = kb.load_utility_scores(scores_path) {
        warn!("Failed to load utility scores: {}", e);
    } else if scores_path.exists() {
        info!("Loaded utility scores from {:?}", scores_path);
    }

    kb
}
