//! OpenAI-compatible LLM HTTP client.
//!
//! Connects to any OpenAI-compatible endpoint (OpenGateway, OpenAI, Anthropic via proxy, etc.)
//! and sends chat completion requests.

use serde::{Deserialize, Serialize};

/// Configuration for the LLM provider.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub max_tokens: u32,
    pub temperature: f64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://opengateway.gitlawb.com/v1".to_string(),
            model: "mimo-v2.5-pro".to_string(),
            api_key: std::env::var("OPENGATEWAY_API_KEY").unwrap_or_default(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// A message in the chat completion format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// The LLM provider — sends requests to an OpenAI-compatible endpoint.
pub struct LlmProvider {
    client: reqwest::Client,
    config: LlmConfig,
}

/// Errors from the LLM provider.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Rate limited, retry after {0}s")]
    RateLimited(u64),
}

impl LlmProvider {
    /// Create a new LLM provider from config.
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Send a chat completion request.
    ///
    /// `system` is the system prompt. `messages` is the conversation history.
    /// Returns the assistant's response text.
    pub async fn chat(&self, system: &str, messages: &[Message]) -> Result<String, LlmError> {
        let mut all_messages = vec![Message {
            role: "system".to_string(),
            content: system.to_string(),
        }];
        all_messages.extend_from_slice(messages);

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": all_messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        let url = format!("{}/chat/completions", self.config.endpoint);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);
            return Err(LlmError::RateLimited(retry_after));
        }

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let json: serde_json::Value = response.json().await?;

        let content = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                LlmError::InvalidResponse("Missing choices[0].message.content".to_string())
            })?;

        Ok(content.to_string())
    }
}
