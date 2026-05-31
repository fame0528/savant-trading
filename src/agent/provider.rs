//! OpenAI-compatible LLM HTTP client.
//!
//! Uses curl for HTTP requests (reqwest has TLS issues on some Windows configs).

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Configuration for the LLM provider.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://opengateway.gitlawb.com/v1".to_string(),
            model: "mimo-v2.5-pro".to_string(),
            api_key: std::env::var("OPENGATEWAY_API_KEY").unwrap_or_default(),
            max_tokens: 4096,
            temperature: 0.7,
            timeout_secs: 120,
        }
    }
}

/// A message in the chat completion format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// The LLM provider — sends requests via curl subprocess.
pub struct LlmProvider {
    config: LlmConfig,
}

/// Errors from the LLM provider.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Rate limited, retry after {0}s")]
    RateLimited(u64),
}

impl LlmProvider {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub fn config_clone(&self) -> LlmConfig {
        self.config.clone()
    }

    pub async fn chat(&self, system: &str, messages: &[Message]) -> Result<String, LlmError> {
        let config = self.config.clone();
        let system = system.to_string();
        let messages = messages.to_vec();

        tokio::task::spawn_blocking(move || chat_blocking(&config, &system, &messages))
            .await
            .map_err(|e| LlmError::Http(format!("Task join error: {}", e)))?
    }
}

fn chat_blocking(
    config: &LlmConfig,
    system: &str,
    messages: &[Message],
) -> Result<String, LlmError> {
    let mut all_messages = vec![Message {
        role: "system".to_string(),
        content: system.to_string(),
    }];
    all_messages.extend_from_slice(messages);

    let body = serde_json::json!({
        "model": config.model,
        "messages": all_messages,
        "max_tokens": config.max_tokens,
        "temperature": config.temperature,
    });

    let url = format!("{}/chat/completions", config.endpoint);
    let body_str = serde_json::to_string(&body)
        .map_err(|e| LlmError::Http(format!("JSON serialize error: {}", e)))?;

    // Write body to temp file (unique per request to avoid race conditions)
    let req_id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path =
        std::env::temp_dir().join(format!("savant_llm_{}_{}.json", std::process::id(), req_id));
    std::fs::write(&tmp_path, &body_str)
        .map_err(|e| LlmError::Http(format!("Failed to write temp file: {}", e)))?;

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            &url,
            "-H",
            "Content-Type: application/json",
            "-H",
            &format!("Authorization: Bearer {}", config.api_key),
            "-d",
            &format!("@{}", tmp_path.display()),
            "--connect-timeout",
            "15",
            "--max-time",
            &config.timeout_secs.to_string(),
        ])
        .output()
        .map_err(|e| LlmError::Http(format!("curl exec error: {}", e)))?;

    let _ = std::fs::remove_file(&tmp_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LlmError::Http(format!(
            "curl exit code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }

    let response_text = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
        LlmError::InvalidResponse(format!(
            "JSON parse error: {} — body: {}",
            e,
            &response_text[..response_text.len().min(200)]
        ))
    })?;

    // Check for API error
    if let Some(error) = json.get("error") {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        if msg.contains("rate") || msg.contains("limit") {
            return Err(LlmError::RateLimited(60));
        }
        return Err(LlmError::InvalidResponse(format!("API error: {}", msg)));
    }

    let content = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| {
            // mimo v2.5 pro returns reasoning in "reasoning" field, content may be null
            m.get("content")
                .and_then(|c| c.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| m.get("reasoning").and_then(|r| r.as_str()))
        })
        .ok_or_else(|| {
            LlmError::InvalidResponse(format!(
                "Missing choices[0].message.content/reasoning — body: {}",
                &response_text[..response_text.len().min(300)]
            ))
        })?;

    Ok(content.to_string())
}
