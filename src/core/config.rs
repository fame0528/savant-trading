use serde::Deserialize;
use std::path::Path;

use crate::core::error::ConfigError;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub exchange: ExchangeConfig,
    pub trading: TradingConfig,
    pub risk: RiskConfig,
    pub strategy: StrategyConfig,
    pub mode: ModeConfig,
    pub ai: AiConfig,
    pub insight: InsightConfig,
    #[serde(default)]
    pub training: TrainingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrainingConfig {
    #[serde(default = "default_min_sample_size")]
    pub min_sample_size: i64,
    #[serde(default = "default_failure_win_rate")]
    pub failure_win_rate: f64,
    #[serde(default = "default_max_portfolio_heat")]
    pub max_portfolio_heat: f64,
    #[serde(default = "default_backup_interval_hours")]
    pub backup_interval_hours: u64,
    #[serde(default = "default_max_backups")]
    pub max_backups: u32,
    #[serde(default = "default_utility_learning_rate")]
    pub utility_learning_rate: f64,
    #[serde(default = "default_utility_archive_threshold")]
    pub utility_archive_threshold: f64,
    #[serde(default = "default_max_active_lessons")]
    pub max_active_lessons: i64,
    #[serde(default = "default_brier_cap_threshold")]
    pub brier_cap_threshold: f64,
    #[serde(default = "default_memory_context_min_trades")]
    pub memory_context_min_trades: i64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            min_sample_size: default_min_sample_size(),
            failure_win_rate: default_failure_win_rate(),
            max_portfolio_heat: default_max_portfolio_heat(),
            backup_interval_hours: default_backup_interval_hours(),
            max_backups: default_max_backups(),
            utility_learning_rate: default_utility_learning_rate(),
            utility_archive_threshold: default_utility_archive_threshold(),
            max_active_lessons: default_max_active_lessons(),
            brier_cap_threshold: default_brier_cap_threshold(),
            memory_context_min_trades: default_memory_context_min_trades(),
        }
    }
}

fn default_min_sample_size() -> i64 {
    5
}
fn default_failure_win_rate() -> f64 {
    0.30
}
fn default_max_portfolio_heat() -> f64 {
    0.40
}
fn default_backup_interval_hours() -> u64 {
    6
}
fn default_max_backups() -> u32 {
    7
}
fn default_utility_learning_rate() -> f64 {
    0.05
}
fn default_utility_archive_threshold() -> f64 {
    0.30
}
fn default_max_active_lessons() -> i64 {
    50
}
fn default_brier_cap_threshold() -> f64 {
    0.25
}
fn default_memory_context_min_trades() -> i64 {
    5
}
fn default_top_p() -> f64 {
    0.95
}
fn default_timeout_secs() -> u64 {
    300
}

fn default_openrouter_endpoint() -> String {
    "https://openrouter.ai/api/v1".into()
}
fn default_openrouter_api_key_env() -> String {
    "OPENROUTER_API_KEY".into()
}
fn default_openrouter_model() -> String {
    "openai/gpt-4o".into()
}
fn default_openrouter_referer() -> String {
    "https://github.com/spencer-thompson/savant-trading".into()
}
fn default_openrouter_title() -> String {
    "Savant Trading Engine".into()
}
fn default_management_key_env() -> String {
    "OPENROUTER_MANAGEMENT_KEY".into()
}
fn default_management_endpoint() -> String {
    "https://openrouter.ai/api/v1/keys".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenRouterManagementConfig {
    #[serde(default = "default_management_key_env")]
    pub management_key_env: String,
    #[serde(default = "default_management_endpoint")]
    pub endpoint: String,
}

impl Default for OpenRouterManagementConfig {
    fn default() -> Self {
        Self {
            management_key_env: default_management_key_env(),
            endpoint: default_management_endpoint(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenRouterConfig {
    #[serde(default = "default_openrouter_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_openrouter_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_openrouter_model")]
    pub model: String,
    #[serde(default = "default_openrouter_referer")]
    pub referer: String,
    #[serde(default = "default_openrouter_title")]
    pub title: String,
    #[serde(default)]
    pub management: OpenRouterManagementConfig,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            endpoint: default_openrouter_endpoint(),
            api_key_env: default_openrouter_api_key_env(),
            model: default_openrouter_model(),
            referer: default_openrouter_referer(),
            title: default_openrouter_title(),
            management: OpenRouterManagementConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub api_key_env: String,
    pub autonomy_level: u8,
    pub max_decisions_per_hour: u32,
    pub context_window_candles: usize,
    pub knowledge_token_budget: usize,
    pub price_tolerance_pct: f64,
    pub max_retries: u32,
    pub temperature: f64,
    pub max_tokens: u32,
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub openrouter: OpenRouterConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InsightConfig {
    pub funding_rate_enabled: bool,
    pub liquidation_enabled: bool,
    pub fear_greed_enabled: bool,
    pub btc_dominance_enabled: bool,
    pub exchange_flows_enabled: bool,
    pub news_sentiment_enabled: bool,
    pub rss_enabled: bool,
    pub rss_max_items: usize,
    pub onchain_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    #[serde(default = "default_backend")]
    pub backend: String,
    pub ws_url: String,
    pub rest_url: String,
    #[serde(default)]
    pub dex: DexConfig,
}

fn default_backend() -> String {
    "kraken".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct DexConfig {
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
    #[serde(default = "default_rpc_url")]
    pub rpc_url: String,
    #[serde(default = "default_wallet_key_env")]
    pub wallet_key_env: String,
    #[serde(default = "default_dex_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_dex_slippage")]
    pub slippage_pct: f64,
}

fn default_chain_id() -> u64 { 42161 }
fn default_rpc_url() -> String { "https://arb1.arbitrum.io/rpc".into() }
fn default_wallet_key_env() -> String { "WALLET_PRIVATE_KEY".into() }
fn default_dex_api_key_env() -> String { "ZEROEX_API_KEY".into() }
fn default_dex_slippage() -> f64 { 0.005 }

impl Default for DexConfig {
    fn default() -> Self {
        Self {
            chain_id: default_chain_id(),
            rpc_url: default_rpc_url(),
            wallet_key_env: default_wallet_key_env(),
            api_key_env: default_dex_api_key_env(),
            slippage_pct: default_dex_slippage(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TradingConfig {
    pub pairs: Vec<String>,
    pub scan_all_pairs: bool,
    pub timeframe: String,
    pub timeframes: Vec<String>,
    pub base_currency: String,
    pub starting_balance: f64,
    pub database_url: String,
    pub fee_rate: f64,
    pub slippage_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskTier {
    pub balance: f64,
    pub risk_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    pub max_risk_per_trade: f64,
    #[serde(default)]
    pub dynamic_risk_tiers: Vec<RiskTier>,
    pub max_daily_loss: f64,
    pub max_drawdown: f64,
    pub max_positions: usize,
    pub min_rr_ratio: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StrategyConfig {
    pub momentum: MomentumConfig,
    pub mean_reversion: MeanReversionConfig,
    pub regime: RegimeConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MomentumConfig {
    pub ema_period: usize,
    pub volume_spike_multiplier: f64,
    pub atr_compression_threshold: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MeanReversionConfig {
    pub profile_periods: usize,
    pub value_area_pct: f64,
    pub volume_spike_multiplier: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegimeConfig {
    pub adx_period: usize,
    pub adx_trending_threshold: f64,
    pub adx_ranging_threshold: f64,
    pub atr_volatility_multiplier: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModeConfig {
    pub paper_trading: bool,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        let config: AppConfig =
            toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.trading.pairs.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one trading pair required".into(),
            ));
        }
        if self.risk.max_risk_per_trade <= 0.0 || self.risk.max_risk_per_trade > 0.25 {
            return Err(ConfigError::ValidationError(
                "max_risk_per_trade must be between 0 and 0.25 (25%)".into(),
            ));
        }
        if self.risk.max_daily_loss <= 0.0 || self.risk.max_daily_loss > 0.5 {
            return Err(ConfigError::ValidationError(
                "max_daily_loss must be between 0 and 0.5 (50%)".into(),
            ));
        }
        if self.trading.starting_balance <= 0.0 {
            return Err(ConfigError::ValidationError(
                "starting_balance must be positive".into(),
            ));
        }
        // Validate exchange backend
        match self.exchange.backend.as_str() {
            "kraken" | "0x" | "1inch" => {}
            other => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid exchange.backend '{}': must be 'kraken', '0x', or '1inch'",
                    other
                )));
            }
        }
        // Validate AI provider
        match self.ai.provider.as_str() {
            "opengateway" | "openrouter" => {}
            other => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid ai.provider '{}': must be 'opengateway' or 'openrouter'",
                    other
                )));
            }
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            exchange: ExchangeConfig {
                name: "kraken".into(),
                backend: default_backend(),
                ws_url: "wss://ws.kraken.com/v2".into(),
                rest_url: "https://api.kraken.com".into(),
                dex: DexConfig::default(),
            },
            trading: TradingConfig {
                pairs: vec!["BTC/USD".into(), "ETH/USD".into()],
                scan_all_pairs: false,
                timeframe: "5m".into(),
                timeframes: vec!["5m".into(), "1h".into(), "4h".into()],
                base_currency: "USD".into(),
                starting_balance: 100.0,
                database_url: "sqlite:data/savant.db".into(),
                fee_rate: 0.0026,
                slippage_pct: 0.0005,
            },
            risk: RiskConfig {
                max_risk_per_trade: 0.20,
                dynamic_risk_tiers: vec![
                    RiskTier {
                        balance: 500.0,
                        risk_pct: 0.20,
                    },
                    RiskTier {
                        balance: 5000.0,
                        risk_pct: 0.10,
                    },
                    RiskTier {
                        balance: 50000.0,
                        risk_pct: 0.05,
                    },
                    RiskTier {
                        balance: 999999.0,
                        risk_pct: 0.02,
                    },
                ],
                max_daily_loss: 0.20,
                max_drawdown: 0.40,
                max_positions: 5,
                min_rr_ratio: 1.5,
            },
            strategy: StrategyConfig {
                momentum: MomentumConfig {
                    ema_period: 100,
                    volume_spike_multiplier: 2.0,
                    atr_compression_threshold: 0.7,
                },
                mean_reversion: MeanReversionConfig {
                    profile_periods: 100,
                    value_area_pct: 0.70,
                    volume_spike_multiplier: 1.5,
                },
                regime: RegimeConfig {
                    adx_period: 14,
                    adx_trending_threshold: 25.0,
                    adx_ranging_threshold: 20.0,
                    atr_volatility_multiplier: 1.5,
                },
            },
            mode: ModeConfig {
                paper_trading: true,
            },
            ai: AiConfig {
                provider: "openrouter".into(),
                endpoint: "https://openrouter.ai/api/v1".into(),
                model: "openrouter/owl-alpha".into(),
                api_key_env: "OPENROUTER_API_KEY".into(),
                autonomy_level: 3,
                max_decisions_per_hour: 5,
                context_window_candles: 500,
                knowledge_token_budget: 20000,
                price_tolerance_pct: 10.0,
                max_retries: 3,
                temperature: 0.6,
                top_p: 0.95,
                max_tokens: 131072,
                timeout_secs: 300,
                openrouter: OpenRouterConfig::default(),
            },
            insight: InsightConfig {
                funding_rate_enabled: true,
                liquidation_enabled: true,
                fear_greed_enabled: true,
                btc_dominance_enabled: true,
                exchange_flows_enabled: true,
                news_sentiment_enabled: true,
                rss_enabled: true,
                rss_max_items: 10,
                onchain_enabled: true,
            },
            training: TrainingConfig::default(),
        }
    }
}
