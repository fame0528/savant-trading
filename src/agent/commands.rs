//! Operator command system — bidirectional communication between operator and agent.
//!
//! Defines the 13 operator commands, parsing from JSON/natural language,
//! rate limiting, TTL expiration, and command history for undo.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of commands in the pending queue.
#[allow(dead_code)]
const MAX_PENDING_COMMANDS: usize = 100;
/// Maximum length of inject_context messages.
const MAX_INJECT_CONTEXT_LEN: usize = 500;
/// Maximum inject_context messages per cycle.
const MAX_INJECT_CONTEXT_PER_CYCLE: usize = 5;
/// Command TTL — expired commands discarded on drain.
const COMMAND_TTL_SECS: u64 = 600; // 10 minutes
/// Maximum command history entries for undo.
#[allow(dead_code)]
const MAX_COMMAND_HISTORY: usize = 10;
/// Maximum query rate: 3 queries per 5 minutes.
const MAX_QUERIES_PER_5MIN: usize = 3;

/// Autonomy level — controls how much the agent can do without operator approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutonomyLevel {
    /// Level 3: Agent executes all actions independently (current behavior)
    Autonomous,
    /// Level 2: Agent evaluates, generates pending action, pauses for approval
    Confirm,
    /// Level 1: Agent suggests actions but never executes
    Suggest,
}

impl AutonomyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Autonomous => "autonomous",
            Self::Confirm => "confirm",
            Self::Suggest => "suggest",
        }
    }
}

/// 13 operator commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum OperatorCommand {
    /// Force-close a position by pair name.
    OverrideClose {
        pair: String,
        reason: Option<String>,
    },
    /// Set stop-loss for a position.
    OverrideStop { pair: String, stop_loss: f64 },
    /// Inject operator message into next LLM evaluation.
    InjectContext { message: String },
    /// Ask the agent a question (one-shot LLM call).
    Query { message: String },
    /// Change autonomy level.
    SetAutonomy { level: AutonomyLevel },
    /// Approve pending action (confirm/suggest mode).
    Approve,
    /// Halt all trading.
    Pause,
    /// Resume trading.
    Resume,
    /// Get current engine/agent state.
    Status,
    /// Explain last decision for a pair.
    Explain { pair: String },
    /// Operator verdict on a trade.
    Feedback {
        pair: String,
        verdict: String,
        note: Option<String>,
    },
    /// Add pair to evaluation list for N cycles.
    Watch { pair: String, cycles: u32 },
    /// Reverse the last command.
    Undo,
}

/// A command with metadata for the pending queue.
#[derive(Debug, Clone)]
pub struct PendingCommand {
    pub command: OperatorCommand,
    pub created_at: Instant,
    pub source: String,
}

/// Command history entry for undo support.
#[derive(Debug, Clone)]
pub struct CommandHistoryEntry {
    pub original: OperatorCommand,
    pub inverse: OperatorCommand,
    pub created_at: Instant,
}

/// Pending action awaiting operator approval (confirm/suggest mode).
#[derive(Debug, Clone)]
pub struct PendingAction {
    pub action: String,
    pub pair: String,
    pub confidence: f64,
    pub reasoning: String,
    pub created_at: Instant,
}

/// Agent notification — proactive messages pushed to the command terminal.
#[derive(Debug, Clone, Serialize)]
pub struct AgentNotification {
    pub severity: String,
    pub message: String,
    pub pair: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Command response sent back to the client.
#[derive(Debug, Clone, Serialize)]
pub struct CommandResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub ok: bool,
    pub message: Option<String>,
    pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl CommandResponse {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            msg_type: "response".into(),
            ok: true,
            message: Some(message.into()),
            error: None,
            data: None,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            msg_type: "response".into(),
            ok: false,
            message: None,
            error: Some(error.into()),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Rate limiter for inject_context messages.
#[derive(Debug, Default)]
pub struct InjectContextRateLimiter {
    pub count_this_cycle: usize,
}

impl InjectContextRateLimiter {
    pub fn new() -> Self {
        Self {
            count_this_cycle: 0,
        }
    }

    pub fn reset(&mut self) {
        self.count_this_cycle = 0;
    }

    pub fn check(&mut self) -> bool {
        if self.count_this_cycle >= MAX_INJECT_CONTEXT_PER_CYCLE {
            return false;
        }
        self.count_this_cycle += 1;
        true
    }
}

/// Rate limiter for query commands.
#[derive(Debug, Default)]
pub struct QueryRateLimiter {
    pub timestamps: VecDeque<Instant>,
}

impl QueryRateLimiter {
    pub fn new() -> Self {
        Self {
            timestamps: VecDeque::new(),
        }
    }

    pub fn check(&mut self) -> bool {
        let now = Instant::now();
        // Remove timestamps older than 5 minutes
        while let Some(front) = self.timestamps.front() {
            if now.duration_since(*front).as_secs() > 300 {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }
        if self.timestamps.len() >= MAX_QUERIES_PER_5MIN {
            return false;
        }
        self.timestamps.push_back(now);
        true
    }
}

/// Parse an incoming WebSocket message into an OperatorCommand.
/// Supports both structured JSON and natural language.
pub fn parse_command(input: &str) -> Result<OperatorCommand, String> {
    let trimmed = input.trim();

    // Try JSON first
    if trimmed.starts_with('{') {
        return serde_json::from_str::<OperatorCommand>(trimmed)
            .map_err(|e| format!("Invalid command JSON: {}", e));
    }

    // Natural language parsing
    parse_natural_language(trimmed)
}

/// Parse natural language into a structured command.
fn parse_natural_language(input: &str) -> Result<OperatorCommand, String> {
    let lower = input.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.is_empty() {
        return Err("Empty command".into());
    }

    // close <pair>
    if words[0] == "close" && words.len() >= 2 {
        let pair = extract_pair_name(&words[1..]);
        return Ok(OperatorCommand::OverrideClose {
            pair,
            reason: Some("Operator manual close".into()),
        });
    }

    // set stop <pair> <price>
    if words.len() >= 4 && words[0] == "set" && words[1] == "stop" {
        let pair = extract_pair_name(&[words[2]]);
        if let Ok(stop_loss) = words[3].parse::<f64>() {
            return Ok(OperatorCommand::OverrideStop { pair, stop_loss });
        }
    }

    // tighten stops
    if lower.contains("tighten") && lower.contains("stop") {
        return Ok(OperatorCommand::InjectContext {
            message: "Operator directive: Tighten all stops to 1.5x ATR".into(),
        });
    }

    // pause / stop trading
    if words[0] == "pause" || (words.len() >= 2 && words[0] == "stop" && words[1] == "trading") {
        return Ok(OperatorCommand::Pause);
    }

    // resume / start
    if words[0] == "resume" || words[0] == "start" {
        return Ok(OperatorCommand::Resume);
    }

    // status
    if words[0] == "status" {
        return Ok(OperatorCommand::Status);
    }

    // undo
    if words[0] == "undo" {
        return Ok(OperatorCommand::Undo);
    }

    // what's happening with <pair> / explain <pair>
    if (words[0] == "explain" || lower.starts_with("what")) && words.len() >= 2 {
        let pair_start = if words[0] == "explain" {
            1
        } else {
            words
                .iter()
                .position(|w| w.contains("with") || w.contains("about"))
                .map(|i| i + 1)
                .unwrap_or(words.len() - 1)
        };
        if pair_start < words.len() {
            let pair = extract_pair_name(&words[pair_start..]);
            return Ok(OperatorCommand::Explain { pair });
        }
    }

    // set autonomy <level>
    if words.len() >= 3 && words[0] == "set" && words[1] == "autonomy" {
        let level = match words[2] {
            "autonomous" | "auto" | "3" => AutonomyLevel::Autonomous,
            "confirm" | "2" => AutonomyLevel::Confirm,
            "suggest" | "1" => AutonomyLevel::Suggest,
            _ => return Err(format!("Unknown autonomy level: {}", words[2])),
        };
        return Ok(OperatorCommand::SetAutonomy { level });
    }

    // approve
    if words[0] == "approve" || words[0] == "yes" || words[0] == "ok" {
        return Ok(OperatorCommand::Approve);
    }

    // watch <pair>
    if words[0] == "watch" && words.len() >= 2 {
        let pair = extract_pair_name(&words[1..]);
        return Ok(OperatorCommand::Watch { pair, cycles: 6 });
    }

    // Default: treat as inject_context
    Ok(OperatorCommand::InjectContext {
        message: input.to_string(),
    })
}

/// Extract a pair name from word tokens. Handles "WETH", "WETH/USD", "weth", etc.
fn extract_pair_name(words: &[&str]) -> String {
    let joined = words.join(" ").to_uppercase();
    let symbol = joined.split_whitespace().next().unwrap_or("BTC");

    // Normalize common variants
    let base = match symbol {
        "ETH" | "WETH" | "ETHEREUM" => "WETH",
        "BTC" | "BITCOIN" => "BTC",
        "LINK" | "CHAINLINK" => "LINK",
        "UNI" | "UNISWAP" => "UNI",
        "AAVE" => "AAVE",
        "ARB" | "ARBITRUM" => "ARB",
        "PENDLE" => "PENDLE",
        "COMP" | "COMPOUND" => "COMP",
        "LDO" | "LIDO" => "LDO",
        "PEPE" => "PEPE",
        "SOL" | "SOLANA" => "SOL",
        _ => symbol,
    };

    if base.contains('/') {
        base.to_string()
    } else {
        format!("{}/USD", base)
    }
}

/// Sanitize an inject_context message.
/// Rejects messages that look like command injection attempts.
pub fn sanitize_inject_context(message: &str) -> Result<(), String> {
    if message.len() > MAX_INJECT_CONTEXT_LEN {
        return Err(format!(
            "Message too long: {} chars (max {})",
            message.len(),
            MAX_INJECT_CONTEXT_LEN
        ));
    }

    // Reject messages containing command-like JSON
    let lower = message.to_lowercase();
    if lower.contains("\"action\":") || lower.contains("\"type\": \"cmd\"") {
        return Err("Message contains command-like JSON — rejected for safety".into());
    }

    Ok(())
}

/// Expire commands older than TTL.
pub fn expire_commands(queue: &mut VecDeque<PendingCommand>) {
    let now = Instant::now();
    queue.retain(|cmd| now.duration_since(cmd.created_at).as_secs() < COMMAND_TTL_SECS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_close() {
        let input = r#"{"action": "override_close", "pair": "WETH/USD", "reason": "test"}"#;
        let cmd = parse_command(input).unwrap();
        match cmd {
            OperatorCommand::OverrideClose { pair, .. } => assert_eq!(pair, "WETH/USD"),
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_natural_close() {
        let cmd = parse_command("close weth").unwrap();
        match cmd {
            OperatorCommand::OverrideClose { pair, .. } => assert_eq!(pair, "WETH/USD"),
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_pause() {
        let cmd = parse_command("pause").unwrap();
        assert!(matches!(cmd, OperatorCommand::Pause));
    }

    #[test]
    fn test_parse_set_stop() {
        let cmd = parse_command("set stop weth 1800").unwrap();
        match cmd {
            OperatorCommand::OverrideStop { pair, stop_loss } => {
                assert_eq!(pair, "WETH/USD");
                assert_eq!(stop_loss, 1800.0);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_autonomy() {
        let cmd = parse_command("set autonomy confirm").unwrap();
        match cmd {
            OperatorCommand::SetAutonomy { level } => {
                assert_eq!(level, AutonomyLevel::Confirm);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_inject_context_fallback() {
        let cmd = parse_command("hold for now, market looks shaky").unwrap();
        match cmd {
            OperatorCommand::InjectContext { message } => {
                assert_eq!(message, "hold for now, market looks shaky");
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_sanitize_inject_context() {
        assert!(sanitize_inject_context("normal message").is_ok());
        assert!(sanitize_inject_context(r#"{"action": "pause"}"#).is_err());
        assert!(sanitize_inject_context(&"x".repeat(501)).is_err());
    }

    #[test]
    fn test_extract_pair_name() {
        assert_eq!(extract_pair_name(&["weth"]), "WETH/USD");
        assert_eq!(extract_pair_name(&["WETH/USD"]), "WETH/USD");
        assert_eq!(extract_pair_name(&["bitcoin"]), "BTC/USD");
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = QueryRateLimiter::new();
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(!limiter.check()); // 4th should fail
    }
}
