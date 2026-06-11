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
pub struct JuryPool {
    /// LlmConfig cloned for creating per-spawn providers.
    provider_config: LlmConfig,
    /// Key manager for acquiring jury API keys.
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
    pub fn new(
        provider_config: LlmConfig,
        key_manager: JuryKeyManager,
        config: JuryConfig,
    ) -> Self {
        let jury_system_prompt =
            include_str!("../prompts/jury_member.md").to_string();

        Self {
            provider_config,
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

        // Spawn N parallel jury member calls
        let mut join_set = tokio::task::JoinSet::new();
        for i in 0..jury_size {
            let key = match self.key_manager.acquire_key().await {
                Some(k) => k,
                None => {
                    warn!("Jury: no key available for member {}, skipping", i);
                    continue;
                }
            };
            let provider = LlmProvider::new(self.provider_config.clone());
            let system = self.jury_system_prompt.clone();
            let user = user_message.to_string();
            let model = self.config.model.clone();
            let timeout_secs = self.config.timeout_secs;
            let key_label = key.label.clone();

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
                        &key.api_key,
                        timeout_secs,
                        true, // no_cache for free models
                    ),
                )
                .await;

                (key_label, key.hash.clone(), result)
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
                            let _ = self.key_manager.record_success(&hash).await;
                        }
                        Err(e) => {
                            warn!("Jury member '{}': parse failed: {}", label, e);
                            failed += 1;
                            let _ = self.key_manager.record_failure(&hash).await;
                        }
                    }
                }
                Ok((label, hash, Ok(Err(e)))) => {
                    warn!("Jury member '{}': LLM error: {}", label, e);
                    failed += 1;
                    let _ = self.key_manager.record_failure(&hash).await;
                }
                Ok((label, hash, Err(_))) => {
                    warn!("Jury member '{}': timed out", label);
                    failed += 1;
                    let _ = self.key_manager.record_failure(&hash).await;
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
