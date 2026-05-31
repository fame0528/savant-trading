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
    pub ws_url: String,
    pub rest_url: String,
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
        if self.risk.max_risk_per_trade <= 0.0 || self.risk.max_risk_per_trade > 0.1 {
            return Err(ConfigError::ValidationError(
                "max_risk_per_trade must be between 0 and 0.1 (10%)".into(),
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
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            exchange: ExchangeConfig {
                name: "kraken".into(),
                ws_url: "wss://ws.kraken.com/v2".into(),
                rest_url: "https://api.kraken.com".into(),
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
                provider: "opengateway".into(),
                endpoint: "https://opengateway.gitlawb.com/v1".into(),
                model: "mimo-v2.5-pro".into(),
                api_key_env: "OPENGATEWAY_API_KEY".into(),
                autonomy_level: 3,
                max_decisions_per_hour: 5,
                context_window_candles: 100,
                knowledge_token_budget: 8000,
                price_tolerance_pct: 10.0,
                max_retries: 3,
                temperature: 0.7,
                max_tokens: 4096,
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
        }
    }
}
