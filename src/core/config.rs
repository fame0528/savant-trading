use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub context: ContextConfig,
    pub insight: InsightConfig,
    #[serde(default)]
    pub training: TrainingConfig,
    #[serde(default)]
    pub chains: HashMap<String, ChainEntry>,
}

/// Per-chain configuration for multi-chain support (FID-045).
#[derive(Debug, Clone, Deserialize)]
pub struct ChainEntry {
    pub chain_id: u64,
    pub name: String,
    pub rpc_url: String,
    pub native_token: String,
    #[serde(default = "default_min_gas")]
    pub min_gas_native: f64,
    #[serde(default = "default_dex_slippage")]
    pub slippage_pct: f64,
    #[serde(default)]
    pub enabled: bool,
}

fn default_min_gas() -> f64 {
    0.002
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

fn default_nvidia_endpoint() -> String {
    "https://integrate.api.nvidia.com/v1".into()
}
fn default_nvidia_api_key_env() -> String {
    "NVIDIA_API_KEY".into()
}
fn default_nvidia_model() -> String {
    "deepseek-ai/deepseek-v4-flash".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct NvidiaConfig {
    #[serde(default = "default_nvidia_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_nvidia_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_nvidia_model")]
    pub model: String,
}

impl Default for NvidiaConfig {
    fn default() -> Self {
        Self {
            endpoint: default_nvidia_endpoint(),
            api_key_env: default_nvidia_api_key_env(),
            model: default_nvidia_model(),
        }
    }
}

fn default_tokenrouter_endpoint() -> String {
    "https://api.tokenrouter.com/v1".into()
}
fn default_tokenrouter_api_key_env() -> String {
    "TOKEN_ROUTER_API_KEY".into()
}
fn default_tokenrouter_model() -> String {
    "MiniMax-M3".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenRouterConfig {
    #[serde(default = "default_tokenrouter_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_tokenrouter_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_tokenrouter_model")]
    pub model: String,
}

impl Default for TokenRouterConfig {
    fn default() -> Self {
        Self {
            endpoint: default_tokenrouter_endpoint(),
            api_key_env: default_tokenrouter_api_key_env(),
            model: default_tokenrouter_model(),
        }
    }
}

/// Regime-specific jury sizes (FID-114 Phase 6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeSizes {
    #[serde(default = "default_regime_size_trending")]
    pub trending: usize,
    #[serde(default = "default_regime_size_ranging")]
    pub ranging: usize,
    #[serde(default = "default_regime_size_volatile")]
    pub volatile: usize,
}

impl Default for RegimeSizes {
    fn default() -> Self {
        Self {
            trending: default_regime_size_trending(),
            ranging: default_regime_size_ranging(),
            volatile: default_regime_size_volatile(),
        }
    }
}

/// Jury system configuration (FID-114: Model Jury).
///
/// **FID-143 (MS-3):** `model` replaced with `models` (Vec) to assign a specific
/// model slug to each jury member. This guarantees provider diversity — each
/// juror gets a different free model (Gemma, Llama, Nemotron, Qwen, etc.) plus
/// one M3 control group member. Falls back to `model` string (legacy compat).
#[derive(Debug, Clone, Deserialize)]
pub struct JuryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_jury_size")]
    pub jury_size: usize,
    /// Deprecated: use `models` instead. Kept for backward compat.
    #[serde(default = "default_jury_model")]
    pub model: String,
    /// Per-juror model slugs. Juror i uses models[i % models.len()].
    /// When empty, falls back to `model` for all jurors (legacy behavior).
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default = "default_quorum_pct")]
    pub quorum_pct: f64,
    #[serde(default = "default_jury_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_consecutive_failures")]
    pub max_consecutive_failures: u32,
    #[serde(default = "default_key_prefix")]
    pub key_prefix: String,
    #[serde(default = "default_true")]
    pub cleanup_keys_on_shutdown: bool,
    #[serde(default)]
    pub regime_sizes: RegimeSizes,
    /// FID-146: If true, jury can veto primary model's Buy/Sell when 70%+ disagrees.
    #[serde(default = "default_true")]
    pub jury_veto_enabled: bool,
    /// FID-146: Fraction of jury that must disagree to trigger veto (0.0-1.0).
    #[serde(default = "default_jury_veto_threshold")]
    pub jury_veto_threshold: f64,
}

fn default_jury_veto_threshold() -> f64 {
    0.70
}

impl Default for JuryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            jury_size: default_jury_size(),
            model: default_jury_model(),
            models: default_jury_models(),
            quorum_pct: default_quorum_pct(),
            timeout_secs: default_jury_timeout_secs(),
            max_consecutive_failures: default_max_consecutive_failures(),
            key_prefix: default_key_prefix(),
            cleanup_keys_on_shutdown: true,
            regime_sizes: RegimeSizes::default(),
            jury_veto_enabled: true,
            jury_veto_threshold: default_jury_veto_threshold(),
        }
    }
}

impl JuryConfig {
    /// Get the model slug for juror index `i`.
    /// Uses `models[i % models.len()]` when populated; falls back to `model`.
    pub fn model_for_juror(&self, i: usize) -> String {
        if self.models.is_empty() {
            self.model.clone()
        } else {
            self.models[i % self.models.len()].clone()
        }
    }
}

impl JuryConfig {
    /// Get the jury size for a specific market regime.
    pub fn size_for_regime(&self, regime: &str) -> usize {
        match regime.to_lowercase().as_str() {
            "trending" => self.regime_sizes.trending,
            "ranging" => self.regime_sizes.ranging,
            "volatile" | "highvol" | "high_vol" => self.regime_sizes.volatile,
            _ => self.jury_size,
        }
    }
}

fn default_regime_size_trending() -> usize {
    6
}
fn default_regime_size_ranging() -> usize {
    10
}
fn default_regime_size_volatile() -> usize {
    10
}

fn default_jury_size() -> usize {
    10
}
fn default_jury_model() -> String {
    "openrouter/free".into()
}

/// FID-143 (MS-3): 9 diverse free models + 1 M3 control group.
/// Ensures jury diversity — each juror gets a different architecture/provider.
fn default_jury_models() -> Vec<String> {
    vec![
        "google/gemma-4-26b-a4b-it:free".into(),
        "google/gemma-4-31b-it:free".into(),
        "meta-llama/llama-3.2-3b-instruct:free".into(),
        "meta-llama/llama-3.3-70b-instruct:free".into(),
        "nvidia/nemotron-3-super-120b-a12b:free".into(),
        "nvidia/nemotron-3-ultra-550b-a55b:free".into(),
        "qwen/qwen3-coder:free".into(),
        "qwen/qwen3-next-80b-a3b-instruct:free".into(),
        "openai/gpt-oss-120b:free".into(),
        "minimax/minimax-m3".into(),
    ]
}
fn default_quorum_pct() -> f64 {
    0.6
}
fn default_jury_timeout_secs() -> u64 {
    45
}
fn default_max_consecutive_failures() -> u32 {
    3
}
fn default_key_prefix() -> String {
    "savant-jury".into()
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
    /// FID-138: Disable chain-of-thought reasoning for models that support it.
    /// When true, adds `"thinking": {"type": "disabled"}` to the request body.
    /// Only effective for reasoning models (MiniMax M3, DeepSeek R1, etc.).
    #[serde(default)]
    pub disable_thinking: bool,
    #[serde(default)]
    pub openrouter: OpenRouterConfig,
    #[serde(default)]
    pub nvidia: NvidiaConfig,
    #[serde(default)]
    pub tokenrouter: TokenRouterConfig,
    #[serde(default)]
    pub jury: JuryConfig,
}

/// Context management configuration (FID-085).
///
/// Controls prompt assembly, encoding, caching, and compaction.
#[derive(Debug, Clone, Deserialize)]
pub struct ContextConfig {
    /// Encoding mode for OHLC data: "json" | "tsln" | "zigzag"
    #[serde(default = "default_encoding_mode")]
    pub encoding_mode: String,
    /// Brain cache TTL in seconds (how long the static prefix stays cached)
    #[serde(default = "default_brain_cache_ttl")]
    pub brain_cache_ttl: u64,
    /// Minimum tokens that delta-compression must save to be worthwhile (FID-164).
    /// Adaptive threshold: 1.0 - (min_token_savings / current_tokens). Smaller prompts
    /// get a more lenient threshold, larger prompts get a stricter one. Also drives
    /// per-pair anti-thrashing (skip if last 2 cycles saved < this many tokens each).
    #[serde(default = "default_delta_compression_min_token_savings")]
    pub delta_compression_min_token_savings: usize,
    /// Hard minimum context window (tokens) — warn if model is below this
    #[serde(default = "default_min_context_guard")]
    pub min_context_guard: u32,
    /// Warn if context window is below this many tokens
    #[serde(default = "default_warn_context_guard")]
    pub warn_context_guard: u32,
    /// Max decision log entries before rotation
    #[serde(default = "default_decision_log_max_entries")]
    pub decision_log_max_entries: usize,
    /// Budget for knowledge tokens
    #[serde(default = "default_knowledge_token_budget")]
    pub knowledge_token_budget: usize,
    /// SGDR epoch length in cycles
    #[serde(default = "default_sgdr_epoch_length")]
    pub sgdr_epoch_length: usize,
    /// SGDR max token budget (scanning phase)
    #[serde(default = "default_sgdr_max_budget")]
    pub sgdr_max_budget: u32,
    /// SGDR min token budget (monitoring phase)
    #[serde(default = "default_sgdr_min_budget")]
    pub sgdr_min_budget: u32,
    /// Candle count for ranging markets (ADX < 15)
    #[serde(default = "default_candles_ranging")]
    pub adaptive_candles_ranging: usize,
    /// Candle count for trending markets (ADX > 25)
    #[serde(default = "default_candles_trending")]
    pub adaptive_candles_trending: usize,
    /// Candle count for volatile markets (GK > 2x avg)
    #[serde(default = "default_candles_volatile")]
    pub adaptive_candles_volatile: usize,
    /// Microcompaction soft trim ratio
    #[serde(default = "default_microcompact_soft_ratio")]
    pub microcompact_soft_ratio: f64,
    /// Microcompaction hard clear ratio
    #[serde(default = "default_microcompact_hard_ratio")]
    pub microcompact_hard_ratio: f64,
    /// Price data TTL in milliseconds
    #[serde(default = "default_ttl_prices_ms")]
    pub ttl_prices_ms: u64,
    /// Indicator data TTL in milliseconds
    #[serde(default = "default_ttl_indicators_ms")]
    pub ttl_indicators_ms: u64,
    /// Enable session-sticky routing for cache affinity
    #[serde(default = "default_true")]
    pub provider_session_sticky: bool,
}

fn default_knowledge_token_budget() -> usize {
    20000
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            encoding_mode: default_encoding_mode(),
            brain_cache_ttl: default_brain_cache_ttl(),
            delta_compression_min_token_savings: default_delta_compression_min_token_savings(),
            min_context_guard: default_min_context_guard(),
            warn_context_guard: default_warn_context_guard(),
            decision_log_max_entries: default_decision_log_max_entries(),
            sgdr_epoch_length: default_sgdr_epoch_length(),
            sgdr_max_budget: default_sgdr_max_budget(),
            sgdr_min_budget: default_sgdr_min_budget(),
            adaptive_candles_ranging: default_candles_ranging(),
            adaptive_candles_trending: default_candles_trending(),
            adaptive_candles_volatile: default_candles_volatile(),
            microcompact_soft_ratio: default_microcompact_soft_ratio(),
            microcompact_hard_ratio: default_microcompact_hard_ratio(),
            ttl_prices_ms: default_ttl_prices_ms(),
            ttl_indicators_ms: default_ttl_indicators_ms(),
            provider_session_sticky: default_true(),
            knowledge_token_budget: default_knowledge_token_budget(),
        }
    }
}

fn default_encoding_mode() -> String {
    "tsln".to_string()
}
fn default_brain_cache_ttl() -> u64 {
    3600
}
fn default_delta_compression_min_token_savings() -> usize {
    50
}
fn default_min_context_guard() -> u32 {
    4096
}
fn default_warn_context_guard() -> u32 {
    8192
}
fn default_decision_log_max_entries() -> usize {
    500
}
fn default_min_volume_24h() -> f64 {
    1_500_000.0
}
fn default_min_price_usd() -> f64 {
    0.001
}
fn default_spread_filter_bps() -> f64 {
    30.0
}
fn default_session_penalty() -> f64 {
    0.90
}
fn default_sgdr_epoch_length() -> usize {
    288
}
fn default_sgdr_max_budget() -> u32 {
    8000
}
fn default_sgdr_min_budget() -> u32 {
    3000
}
fn default_candles_ranging() -> usize {
    50
}
fn default_candles_trending() -> usize {
    100
}
fn default_candles_volatile() -> usize {
    200
}
fn default_microcompact_soft_ratio() -> f64 {
    0.30
}
fn default_microcompact_hard_ratio() -> f64 {
    0.50
}
fn default_ttl_prices_ms() -> u64 {
    300_000
}
fn default_ttl_indicators_ms() -> u64 {
    3_600_000
}

/// FID-120: Dynamic token database configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenStoreConfig {
    /// Enable persistent token store + periodic discovery
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Re-discover token addresses every N cycles (default 10 = ~50 min)
    #[serde(default = "default_token_discovery_interval")]
    pub discovery_interval_cycles: u64,
    /// Minimum 24h volume (USD) for discovered tokens
    #[serde(default = "default_token_min_volume")]
    pub min_volume_usd: f64,
    /// Minimum holder count for discovered tokens
    #[serde(default = "default_token_min_holders")]
    pub min_holders: u64,
    /// Validate new addresses via 0x /price endpoint before adding
    #[serde(default = "default_true")]
    pub validate_via_0x: bool,
    /// Path to persistent token store (JSON)
    #[serde(default = "default_token_persist_path")]
    pub persist_path: String,
}

impl Default for TokenStoreConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            discovery_interval_cycles: default_token_discovery_interval(),
            min_volume_usd: default_token_min_volume(),
            min_holders: default_token_min_holders(),
            validate_via_0x: true,
            persist_path: default_token_persist_path(),
        }
    }
}

fn default_token_discovery_interval() -> u64 {
    10
}
fn default_token_min_volume() -> f64 {
    1_000_000.0
}
fn default_token_min_holders() -> u64 {
    500
}
fn default_token_persist_path() -> String {
    "data/tokens.json".into()
}

/// FID-118: Pair health rotation configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PairRotationConfig {
    /// Re-discover new pairs every N cycles (default 60 = ~3 hours at 5m intervals)
    #[serde(default = "default_rotation_interval")]
    pub interval_cycles: u64,
    /// Permanently evict pair after N consecutive dead cycles (default 5)
    #[serde(default = "default_eviction_threshold")]
    pub eviction_threshold: u32,
    /// Re-check evicted pairs every N cycles (default 300 = ~25 hours)
    #[serde(default = "default_revival_check_cycles")]
    pub revival_check_cycles: u64,
}

impl Default for PairRotationConfig {
    fn default() -> Self {
        Self {
            interval_cycles: default_rotation_interval(),
            eviction_threshold: default_eviction_threshold(),
            revival_check_cycles: default_revival_check_cycles(),
        }
    }
}

fn default_rotation_interval() -> u64 {
    60
}
fn default_eviction_threshold() -> u32 {
    5
}
fn default_revival_check_cycles() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

// ── Sandbox config (FID-123) ─────────────────────────────────────────

fn default_sandbox_endpoint() -> String {
    "https://openrouter.ai/api/v1".into()
}
fn default_sandbox_api_key_env() -> String {
    "OPENROUTER_API_KEY".into()
}
fn default_sandbox_model() -> String {
    "openrouter/owl-alpha".into()
}
fn default_sandbox_max_tokens() -> u32 {
    4096
}
fn default_sandbox_temperature() -> f64 {
    0.6
}
fn default_sandbox_top_p() -> f64 {
    0.95
}

/// Sandbox provider configuration (FID-123).
///
/// Completely independent from `[ai]` — allows testing different
/// models/providers in the sandbox while the engine runs in production.
/// Falls back to `[ai]` fields when not configured.
///
/// **FID-138 (M3 thinking leak):** Added `max_tokens`, `temperature`,
/// `top_p`, `timeout_secs`, and `disable_thinking` fields so the sandbox
/// can constrain reasoning models (MiniMax M3) that exhaust token budgets
/// on chain-of-thought before emitting the action JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct SandboxConfig {
    #[serde(default = "default_sandbox_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_sandbox_api_key_env")]
    pub api_key_env: String,
    #[serde(default = "default_sandbox_model")]
    pub model: String,
    /// Max output tokens (total, including reasoning). Default 4096.
    /// Lower for reasoning models (M3) to force concise output.
    #[serde(default = "default_sandbox_max_tokens")]
    pub max_tokens: u32,
    /// Temperature for sandbox LLM calls.
    #[serde(default = "default_sandbox_temperature")]
    pub temperature: f64,
    /// Top-p for sandbox LLM calls.
    #[serde(default = "default_sandbox_top_p")]
    pub top_p: f64,
    /// Timeout in seconds for sandbox LLM calls.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Disable chain-of-thought reasoning for models that support it (MiniMax M3).
    /// When true, adds `"thinking": {"type": "disabled"}` to the request body.
    #[serde(default)]
    pub disable_thinking: bool,
    #[serde(default)]
    pub management: OpenRouterManagementConfig,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            endpoint: default_sandbox_endpoint(),
            api_key_env: default_sandbox_api_key_env(),
            model: default_sandbox_model(),
            max_tokens: default_sandbox_max_tokens(),
            temperature: default_sandbox_temperature(),
            top_p: default_sandbox_top_p(),
            timeout_secs: default_timeout_secs(),
            disable_thinking: false,
            management: OpenRouterManagementConfig::default(),
        }
    }
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
    "0x".to_string()
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

fn default_chain_id() -> u64 {
    42161
}
fn default_rpc_url() -> String {
    "https://arb1.arbitrum.io/rpc".into()
}
fn default_wallet_key_env() -> String {
    "WALLET_PRIVATE_KEY".into()
}
fn default_dex_api_key_env() -> String {
    "ZEROEX_API_KEY".into()
}
fn default_dex_slippage() -> f64 {
    0.005
}

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
    #[serde(default = "default_min_volume_24h")]
    pub min_volume_24h_usd: f64,
    #[serde(default = "default_min_price_usd")]
    pub min_price_usd: f64,
    #[serde(default)]
    pub blacklisted_symbols: Vec<String>,
    pub timeframe: String,
    pub timeframes: Vec<String>,
    pub base_currency: String,
    pub starting_balance: f64,
    pub database_url: String,
    pub fee_rate: f64,
    pub slippage_pct: f64,
    #[serde(default)]
    pub full_deploy: bool,
    #[serde(default = "default_spread_filter_bps")]
    pub spread_filter_bps: f64,
    #[serde(default = "default_session_penalty")]
    pub session_penalty_deep_asian: f64,
    #[serde(default)]
    pub pair_rotation: PairRotationConfig,
    /// FID-120: Dynamic token database
    #[serde(default)]
    pub token_store: TokenStoreConfig,
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
    #[serde(default = "default_min_rr_low_balance")]
    pub min_rr_ratio_low_balance: f64,
    #[serde(default = "default_low_balance_threshold")]
    pub low_balance_threshold: f64,
    #[serde(default = "default_min_daily_loss_usd")]
    pub min_daily_loss_usd: f64,
    #[serde(default = "default_min_drawdown_usd")]
    pub min_drawdown_usd: f64,
}

fn default_min_rr_low_balance() -> f64 {
    1.2
}
fn default_low_balance_threshold() -> f64 {
    50.0
}
fn default_min_daily_loss_usd() -> f64 {
    5.0
}
fn default_min_drawdown_usd() -> f64 {
    10.0
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
    pub live_execution: bool,
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
            "0x" | "1inch" => {}
            other => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid exchange.backend '{}': must be '0x' or '1inch'",
                    other
                )));
            }
        }
        // Validate AI provider
        match self.ai.provider.as_str() {
            "opengateway" | "openrouter" | "nvidia" | "ollama" | "tokenrouter" => {}
            other => {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid ai.provider '{}': must be 'opengateway', 'openrouter', 'nvidia', 'ollama', or 'tokenrouter'",
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
                name: "candle_feed".into(),
                backend: default_backend(),
                ws_url: "wss://ws.kraken.com/v2".into(),
                rest_url: "https://api.kraken.com".into(),
                dex: DexConfig::default(),
            },
            trading: TradingConfig {
                pairs: vec!["BTC/USD".into(), "ETH/USD".into()],
                scan_all_pairs: false,
                min_volume_24h_usd: 1_500_000.0,
                min_price_usd: 0.001,
                blacklisted_symbols: vec![],
                timeframe: "5m".into(),
                timeframes: vec!["5m".into(), "1h".into(), "4h".into()],
                base_currency: "USD".into(),
                starting_balance: 100.0,
                database_url: "sqlite:data/savant.db".into(),
                fee_rate: 0.0026,
                slippage_pct: 0.0005,
                full_deploy: false,
                spread_filter_bps: 30.0,
                session_penalty_deep_asian: 0.90,
                pair_rotation: PairRotationConfig::default(),
                token_store: TokenStoreConfig::default(),
            },
            risk: RiskConfig {
                max_risk_per_trade: 0.20,
                dynamic_risk_tiers: vec![
                    RiskTier {
                        balance: 500.0,
                        risk_pct: 1.00,
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
                min_rr_ratio_low_balance: 1.2,
                low_balance_threshold: 50.0,
                min_daily_loss_usd: 5.0,
                min_drawdown_usd: 10.0,
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
                live_execution: false,
            },
            ai: AiConfig {
                provider: "nvidia".into(),
                endpoint: "https://integrate.api.nvidia.com/v1".into(),
                model: "deepseek-ai/deepseek-v4-flash".into(),
                api_key_env: "NVIDIA_API_KEY".into(),
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
                disable_thinking: false,
                openrouter: OpenRouterConfig::default(),
                nvidia: NvidiaConfig::default(),
                tokenrouter: TokenRouterConfig::default(),
                jury: JuryConfig::default(),
            },
            sandbox: SandboxConfig::default(),
            context: ContextConfig::default(),
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
            chains: HashMap::new(),
        }
    }
}
