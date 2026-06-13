//! Jury Pool — parallel evaluation of market data by N independent models.
//!
//! Spawns N concurrent LLM calls via JoinSet, each with its own API key
//! from the JuryKeyManager. Collects results, checks quorum, and returns
//! a JuryResult for the Judge to synthesize.

use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::agent::jury::verdict_parser;
use crate::agent::jury::{JuryKeyManager, JuryResult};
use crate::agent::provider::{LlmConfig, LlmProvider, Message};
use crate::core::config::JuryConfig;
use crate::core::types::MarketRegime;

/// Jury Pool — manages parallel jury member evaluations.
///
/// **FID-147 (jury dual-path):** The jury uses TWO distinct API paths:
///
/// - **Juror 0 (M3 control):** `provider_config_m3` + `m3_api_key`. Endpoint
///   points to the M3/TokenRouter proxy. Same path as the main agent. Does
///   NOT participate in management-key rotation.
///
/// - **Jurors 1..N (free):** `provider_config_openrouter` + management keys
///   from `key_manager`. Endpoint MUST be `https://openrouter.ai/api/v1`.
///   Each juror gets a separate management-provisioned key so they can hit
///   a different model (Gemma, Llama, Nemotron, Qwen, etc.).
pub struct JuryPool {
    /// LlmConfig for the M3 control juror (juror 0). Endpoint = M3/TokenRouter.
    provider_config_m3: LlmConfig,
    /// LlmConfig for OpenRouter free jurors (1..N). Endpoint = openrouter.ai.
    provider_config_openrouter: LlmConfig,
    /// API key for the M3 control juror — from `TOKEN_ROUTER_API_KEY`.
    m3_api_key: String,
    /// Key manager for OpenRouter free jurors (acquires management keys).
    key_manager: JuryKeyManager,
    /// Jury configuration.
    config: JuryConfig,
    /// System prompt for jury members (loaded from jury_member.md).
    jury_system_prompt: String,
    /// Accumulated metrics.
    metrics: JuryPoolMetrics,
}

/// Lightweight metrics tracking for the jury pool.
#[derive(Debug, Default)]
pub struct JuryPoolMetrics {
    pub total_evaluations: u64,
    pub quorum_failures: u64,
    pub total_verdicts: u64,
    pub total_failures: u64,
    pub total_latency_ms: u64,
}

impl JuryPool {
    /// Create a new jury pool. Does NOT create keys — call `initialize()` first.
    ///
    /// **FID-147:** Pass TWO LlmConfigs + the M3 API key:
    /// - `provider_config_m3` — for juror 0 (M3 control). Endpoint = M3/TokenRouter.
    /// - `provider_config_openrouter` — for jurors 1..N. Endpoint = openrouter.ai.
    /// - `m3_api_key` — TOKEN_ROUTER_API_KEY value, used to authenticate juror 0.
    pub fn new(
        provider_config_m3: LlmConfig,
        provider_config_openrouter: LlmConfig,
        m3_api_key: String,
        key_manager: JuryKeyManager,
        config: JuryConfig,
    ) -> Self {
        let jury_system_prompt =
            include_str!("../prompts/jury_member.md").to_string();

        // Ensure dev/logs/ exists for metrics flush (once at init, not every cycle)
        let _ = std::fs::create_dir_all("dev/logs");

        Self {
            provider_config_m3,
            provider_config_openrouter,
            m3_api_key,
            key_manager,
            config,
            jury_system_prompt,
            metrics: JuryPoolMetrics::default(),
        }
    }

    /// Initialize the jury key manager. Call once at startup.
    pub async fn initialize(&self) -> Result<usize, crate::agent::jury::JuryKeyError> {
        self.key_manager.initialize().await
    }

    /// Get the key manager for shutdown cleanup.
    pub fn key_manager(&self) -> &JuryKeyManager {
        &self.key_manager
    }

    /// Get accumulated metrics.
    pub fn metrics(&self) -> &JuryPoolMetrics {
        &self.metrics
    }

    /// Flush jury metrics to `dev/logs/jury-metrics.json`.
    /// Synchronous file I/O — safe to call from Drop or periodic flush.
    pub fn flush_metrics(&self) {
        let metrics_json = serde_json::json!({
            "total_evaluations": self.metrics.total_evaluations,
            "quorum_failures": self.metrics.quorum_failures,
            "total_verdicts": self.metrics.total_verdicts,
            "total_failures": self.metrics.total_failures,
            "total_latency_ms": self.metrics.total_latency_ms,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = std::fs::write(
            "dev/logs/jury-metrics.json",
            serde_json::to_string_pretty(&metrics_json).unwrap_or_default(),
        ) {
            warn!("Failed to flush jury metrics: {}", e);
        } else if self.metrics.total_evaluations > 0 {
            info!("Jury metrics flushed (evals={}, quorum_fails={})",
                self.metrics.total_evaluations, self.metrics.quorum_failures);
        }
    }

    /// Evaluate market data with N jury members in parallel.
    ///
    /// Returns a JuryResult with all successful verdicts, quorum status,
    /// and failure counts. The Judge synthesizes this into a final decision.
    pub async fn evaluate(
        &mut self,
        user_message: &str,
        regime: MarketRegime,
    ) -> JuryResult {
        let start = Instant::now();
        let jury_size = self.config.size_for_regime(&regime.to_string());
        let quorum = (jury_size as f64 * self.config.quorum_pct).ceil() as usize;

        self.metrics.total_evaluations += 1;

        info!(
            "Jury: evaluating with {} members (regime: {:?}, quorum: {})",
            jury_size, regime, quorum
        );

        // FID-147: Filter JuryConfig.models to exclude the M3 marker entry.
        // The M3 model is sourced from `provider_config_m3.model` for juror 0.
        let free_models: Vec<String> = self
            .config
            .models
            .iter()
            .filter(|m| {
                let lower = m.to_lowercase();
                !lower.contains("minimax") && lower != "m3" && !lower.ends_with("/m3")
            })
            .cloned()
            .collect();

        // Spawn N parallel jury member calls
        let mut join_set = tokio::task::JoinSet::new();
        for juror_idx in 0..jury_size {
            // FID-147: Juror 0 is the M3 control — uses M3 config + M3 key (no management).
            // Jurors 1..N are free — use OpenRouter config + management-provisioned keys.
            let is_m3_control = juror_idx == 0;

            let (provider, model, api_key, key_label, key_hash) = if is_m3_control {
                let provider = LlmProvider::new(self.provider_config_m3.clone());
                let model = self.provider_config_m3.model.clone();
                let label = format!("m3-control-{}", chrono::Utc::now().timestamp());
                // Empty hash — M3 control doesn't participate in key rotation
                (provider, model, self.m3_api_key.clone(), label, String::new())
            } else {
                let key = match self.key_manager.acquire_key().await {
                    Some(k) => k,
                    None => {
                        warn!(
                            "Jury: no key available for free juror {}, skipping",
                            juror_idx
                        );
                        continue;
                    }
                };
                let provider = LlmProvider::new(self.provider_config_openrouter.clone());
                // Free juror index 0 -> free_models[0], etc.
                let model = if !free_models.is_empty() {
                    free_models[(juror_idx - 1) % free_models.len()].clone()
                } else {
                    // No free models configured (or all filtered as M3 markers).
                    // Legacy fallback: use the single `model` field for all free jurors.
                    self.config.model.clone()
                };
                (provider, model, key.api_key, key.label.clone(), key.hash.clone())
            };

            let system = self.jury_system_prompt.clone();
            let user = user_message.to_string();
            let timeout_secs = self.config.timeout_secs;

            join_set.spawn(async move {
                let result = tokio::time::timeout(
                    Duration::from_secs(timeout_secs),
                    provider.chat_with_override(
                        &system,
                        &[Message {
                            role: "user".to_string(),
                            content: user,
                        }],
                        &model,
                        &api_key,
                        timeout_secs,
                        true, // no_cache for free models
                    ),
                )
                .await;

                (key_label, key_hash, result)
            });
        }

        // Collect results
        let mut verdicts = Vec::new();
        let mut failed = 0usize;
        let mut model_ids = Vec::new();

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((label, hash, Ok(Ok(response)))) => {
                    match verdict_parser::parse_verdict(&response) {
                        Ok(v) => {
                            info!(
                                "Jury member '{}': {} (confidence: {:.0}%)",
                                label,
                                v.verdict,
                                v.confidence * 100.0
                            );
                            model_ids.push(label);
                            verdicts.push(v);
                            // FID-147: M3 control has empty hash (no management) — skip rotation.
                            if !hash.is_empty() {
                                let _ = self.key_manager.record_success(&hash).await;
                            }
                        }
                        Err(e) => {
                            warn!("Jury member '{}': parse failed: {}", label, e);
                            failed += 1;
                            if !hash.is_empty() {
                                let _ = self.key_manager.record_failure(&hash).await;
                            }
                        }
                    }
                }
                Ok((label, hash, Ok(Err(e)))) => {
                    warn!("Jury member '{}': LLM error: {}", label, e);
                    failed += 1;
                    if !hash.is_empty() {
                        let _ = self.key_manager.record_failure(&hash).await;
                    }
                }
                Ok((label, hash, Err(_))) => {
                    warn!("Jury member '{}': timed out", label);
                    failed += 1;
                    if !hash.is_empty() {
                        let _ = self.key_manager.record_failure(&hash).await;
                    }
                }
                Err(e) => {
                    warn!("Jury member: join error: {}", e);
                    failed += 1;
                }
            }
        }

        let latency = start.elapsed().as_millis() as u64;
        let quorum_met = verdicts.len() >= quorum;

        if !quorum_met {
            self.metrics.quorum_failures += 1;
            warn!(
                "Jury quorum NOT met: {}/{} verdicts (need {})",
                verdicts.len(),
                jury_size,
                quorum
            );
        }

        self.metrics.total_verdicts += verdicts.len() as u64;
        self.metrics.total_failures += failed as u64;
        self.metrics.total_latency_ms += latency;

        info!(
            "Jury: {} verdicts, {} failed, quorum={}, {}ms",
            verdicts.len(),
            failed,
            quorum_met,
            latency
        );

        JuryResult {
            verdicts,
            failed_count: failed,
            model_ids,
            total_latency_ms: latency,
            quorum_met,
        }
    }
}

/// Best-effort metrics flush on Drop (Ctrl+C, panic, normal exit).
/// Key cleanup is handled by JuryKeyManager::drop() — we only flush metrics here.
impl Drop for JuryPool {
    fn drop(&mut self) {
        self.flush_metrics();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::jury::JuryVerdict;

    #[test]
    fn jury_metrics_default() {
        let m = JuryPoolMetrics::default();
        assert_eq!(m.total_evaluations, 0);
        assert_eq!(m.quorum_failures, 0);
    }

    #[test]
    fn quorum_calculation() {
        let config = JuryConfig::default();
        // Default: jury_size=10, quorum_pct=0.6 → need 6
        let quorum = (config.jury_size as f64 * config.quorum_pct).ceil() as usize;
        assert_eq!(quorum, 6);
    }

    #[test]
    fn regime_size_trending() {
        let config = JuryConfig::default();
        assert_eq!(config.size_for_regime("Trending"), 6);
    }

    #[test]
    fn regime_size_ranging() {
        let config = JuryConfig::default();
        assert_eq!(config.size_for_regime("Ranging"), 10);
    }

    #[test]
    fn regime_size_volatile() {
        let config = JuryConfig::default();
        assert_eq!(config.size_for_regime("Volatile"), 10);
    }

    #[test]
    fn regime_size_unknown_falls_back() {
        let config = JuryConfig::default();
        assert_eq!(config.size_for_regime("unknown"), 10); // fallback to jury_size
    }

    #[test]
    fn jury_result_verdict_count() {
        let result = JuryResult {
            verdicts: vec![
                JuryVerdict {
                    verdict: "BUY".to_string(),
                    confidence: 0.7,
                    key_argument: "test".to_string(),
                    risk_flag: String::new(),
                    evidence_quality: Some(8.0),
                    reasoning: String::new(),
                },
                JuryVerdict {
                    verdict: "HOLD".to_string(),
                    confidence: 0.3,
                    key_argument: "test".to_string(),
                    risk_flag: String::new(),
                    evidence_quality: Some(5.0),
                    reasoning: String::new(),
                },
            ],
            failed_count: 1,
            model_ids: vec!["model-a".to_string(), "model-b".to_string()],
            total_latency_ms: 1500,
            quorum_met: false,
        };
        assert_eq!(result.verdicts.len(), 2);
        assert_eq!(result.failed_count, 1);
        assert!(!result.quorum_met);
    }
}
