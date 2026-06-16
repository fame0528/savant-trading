//! Jury module — multi-model adversarial decision system (FID-114).
//!
//! Uses the OpenRouter Management API to create ephemeral child API keys.
//! Each jury member gets a distinct model slug (Gemma, Llama, Nemotron, Qwen,
//! etc.) for architectural diversity. Owl Alpha (or the primary trading model)
//! acts as the Judge, synthesizing all jury verdicts.
//!
//! **FID-143 (MS-3):** Jury members always use the OpenRouter endpoint
//! (regardless of primary provider) since keys are OpenRouter-issued.
//! The Judge uses the primary trading model.

pub mod judge;
pub mod key_manager;
pub mod pool;
pub mod verdict_parser;

pub use judge::{JudgeError, JuryJudge};
pub use key_manager::{JuryKey, JuryKeyError, JuryKeyManager};
pub use pool::{
    JurorRecord, JuryCycleRecord, JuryKeyHealth, JuryPool, JuryPoolMetrics, VerdictBreakdown,
};

use serde::{Deserialize, Serialize};

use crate::agent::decision_parser::TradeDecision;

/// A single jury member's verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JuryVerdict {
    /// "BUY" | "SELL" | "HOLD"
    pub verdict: String,
    /// Confidence 0.0-1.0
    #[serde(default)]
    pub confidence: f64,
    /// Primary thesis
    #[serde(default)]
    pub key_argument: String,
    /// Top risk factor
    #[serde(default)]
    pub risk_flag: String,
    /// Self-assessed data utilization 1-10
    #[serde(default)]
    pub evidence_quality: Option<f64>,
    /// Extended reasoning
    #[serde(default)]
    pub reasoning: String,
}

/// Aggregated jury result passed to the Judge.
#[derive(Debug, Clone)]
pub struct JuryResult {
    /// Successful verdicts from jury members
    pub verdicts: Vec<JuryVerdict>,
    /// Number of members that failed to respond
    pub failed_count: usize,
    /// Model IDs that responded (from OpenRouter response metadata)
    pub model_ids: Vec<String>,
    /// Total latency across all jury calls
    pub total_latency_ms: u64,
    /// Whether quorum was met
    pub quorum_met: bool,
}

/// Judge's final decision synthesizing all jury verdicts.
#[derive(Debug, Clone)]
pub struct JuryJudgment {
    /// The final trade decision from the Judge
    pub decision: TradeDecision,
    /// How aligned the jury was (0.0-1.0)
    pub consensus_strength: f64,
    /// Judge's analysis of disagreement
    pub dissent_analysis: String,
    /// Number of jury members that participated
    pub jury_size_used: usize,
}
