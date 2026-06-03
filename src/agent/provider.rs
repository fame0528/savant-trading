//! OpenAI-compatible LLM HTTP client.
//!
//! Supports both non-streaming and SSE streaming modes.
//! Streaming keeps the connection alive during long reasoning (mimo v2.5 pro)
//! and provides real-time visibility into the model's thinking.

use crate::core::config::AiConfig;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub top_p: f64,
    pub timeout_secs: u64,
    pub extra_headers: Vec<(String, String)>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://openrouter.ai/api/v1".to_string(),
            model: "openrouter/owl-alpha".to_string(),
            api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
            max_tokens: 131072,
            temperature: 0.6,
            top_p: 0.95,
            timeout_secs: 300,
            extra_headers: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct LlmProvider {
    client: reqwest::Client,
    config: LlmConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Rate limited, retry after {0}s")]
    RateLimited(u64),
}

/// Create an [`LlmProvider`] based on the configured AI provider.
///
/// Reads `config.ai.provider` to select the correct defaults:
/// - `"opengateway"` → uses `endpoint`, `model`, `api_key_env` directly from `AiConfig`
/// - `"openrouter"` → uses `AiConfig.openrouter` sub-config with provider-specific headers
/// - any other value → falls back to the top-level AiConfig fields (graceful)
pub fn create_provider(ai_cfg: &AiConfig) -> LlmProvider {
    let (mut base, extra_headers) = match ai_cfg.provider.as_str() {
        "openrouter" => {
            let or = &ai_cfg.openrouter;
            (
                LlmConfig {
                    endpoint: or.endpoint.clone(),
                    model: or.model.clone(),
                    api_key: std::env::var(&or.api_key_env).unwrap_or_default(),
                    max_tokens: ai_cfg.max_tokens,
                    temperature: ai_cfg.temperature,
                    top_p: ai_cfg.top_p,
                    timeout_secs: ai_cfg.timeout_secs,
                    extra_headers: vec![],
                },
                vec![
                    ("HTTP-Referer".to_string(), or.referer.clone()),
                    ("X-OpenRouter-Title".to_string(), or.title.clone()),
                ],
            )
        }
        "nvidia" => {
            let nv = &ai_cfg.nvidia;
            (
                LlmConfig {
                    endpoint: nv.endpoint.clone(),
                    model: nv.model.clone(),
                    api_key: std::env::var(&nv.api_key_env).unwrap_or_default(),
                    max_tokens: ai_cfg.max_tokens.min(16384),
                    temperature: ai_cfg.temperature,
                    top_p: ai_cfg.top_p,
                    timeout_secs: ai_cfg.timeout_secs,
                    extra_headers: vec![],
                },
                vec![],
            )
        }
        _ => (
            LlmConfig {
                endpoint: ai_cfg.endpoint.clone(),
                model: ai_cfg.model.clone(),
                api_key: std::env::var(&ai_cfg.api_key_env).unwrap_or_default(),
                max_tokens: ai_cfg.max_tokens,
                temperature: ai_cfg.temperature,
                top_p: ai_cfg.top_p,
                timeout_secs: ai_cfg.timeout_secs,
                extra_headers: vec![],
            },
            vec![],
        ),
    };
    base.extra_headers = extra_headers;

    // OPENROUTER_MODEL env var overrides config file for quick model switching
    // without editing config/default.toml. Only applies to the OpenRouter provider.
    if ai_cfg.provider.as_str() == "openrouter" {
        if let Ok(env_model) = std::env::var("OPENROUTER_MODEL") {
            if !env_model.is_empty() {
                base.model = env_model;
            }
        }
    }

    LlmProvider::new(base)
}

impl LlmProvider {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client, config }
    }

    pub fn config_clone(&self) -> LlmConfig {
        self.config.clone()
    }

    pub async fn chat(&self, system: &str, messages: &[Message]) -> Result<String, LlmError> {
        let body = self.build_body(system, messages, false);
        let resp = self.send_request(&body).await?;
        let status = resp.status();
        if status == 429 {
            return Err(LlmError::RateLimited(60));
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::Http(format!("HTTP {}: {}", status, text)));
        }
        Self::parse_non_streaming(resp).await
    }

    pub async fn chat_stream(
        &self,
        system: &str,
        messages: &[Message],
    ) -> Result<String, LlmError> {
        let max_retries = 2;
        let mut last_err = String::new();

        for attempt in 0..max_retries {
            let body = self.build_body(system, messages, true);
            let resp = match self.send_request(&body).await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("{}", e);
                    if attempt < max_retries - 1 {
                        let wait = 2u64.pow(attempt as u32 + 1);
                        tracing::warn!(
                            "Stream request failed (attempt {}/{}): {}. Retrying in {}s...",
                            attempt + 1,
                            max_retries,
                            last_err,
                            wait
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                        continue;
                    }
                    break;
                }
            };

            let status = resp.status();
            if status == 429 {
                return Err(LlmError::RateLimited(60));
            }
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(LlmError::Http(format!("HTTP {}: {}", status, text)));
            }

            match Self::parse_streaming(resp).await {
                Ok(content) => return Ok(content),
                Err(e) => {
                    last_err = format!("{}", e);
                    if attempt < max_retries - 1 {
                        let wait = 2u64.pow(attempt as u32 + 1);
                        tracing::warn!(
                            "Stream parse failed (attempt {}/{}): {}. Retrying in {}s...",
                            attempt + 1,
                            max_retries,
                            last_err,
                            wait
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                        continue;
                    }
                    break;
                }
            }
        }

        // Streaming failed all retries — fall back to non-streaming.
        // Non-streaming reads the full response body at once, avoiding
        // chunked transfer encoding issues that cause "error decoding
        // response body" on large prompts.
        tracing::warn!(
            "All {} streaming attempts failed ({}). Falling back to non-streaming.",
            max_retries,
            last_err
        );
        let body = self.build_body(system, messages, false);
        let resp = self.send_request(&body).await?;
        let status = resp.status();
        if status == 429 {
            return Err(LlmError::RateLimited(60));
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::Http(format!("HTTP {}: {}", status, text)));
        }
        Self::parse_non_streaming(resp).await
    }

    fn build_body(&self, system: &str, messages: &[Message], stream: bool) -> serde_json::Value {
        let mut all_messages = vec![Message {
            role: "system".to_string(),
            content: system.to_string(),
        }];
        all_messages.extend_from_slice(messages);

        serde_json::json!({
            "model": self.config.model,
            "messages": all_messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "top_p": self.config.top_p,
            "stream": stream,
        })
    }

    async fn send_request(&self, body: &serde_json::Value) -> Result<reqwest::Response, LlmError> {
        let url = format!("{}/chat/completions", self.config.endpoint);
        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key));
        for (name, value) in &self.config.extra_headers {
            req_builder = req_builder.header(name.as_str(), value.as_str());
        }
        req_builder
            .json(body)
            .send()
            .await
            .map_err(|e| LlmError::Http(format!("{}", e)))
    }

    async fn parse_non_streaming(resp: reqwest::Response) -> Result<String, LlmError> {
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(format!("JSON parse error: {}", e)))?;

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
                m.get("content")
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| m.get("reasoning").and_then(|r| r.as_str()))
            })
            .ok_or_else(|| {
                let body_str = serde_json::to_string(&json).unwrap_or_default();
                let truncated = &body_str[..300.min(body_str.len())];
                LlmError::InvalidResponse(format!(
                    "Missing choices[0].message.content/reasoning — body: {}",
                    truncated
                ))
            })?;

        Ok(content.to_string())
    }

    async fn parse_streaming(resp: reqwest::Response) -> Result<String, LlmError> {
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut full_content = String::new();
        let mut full_reasoning = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LlmError::Http(format!("Stream error: {}", e)))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if line.is_empty() || !line.starts_with("data: ") {
                    continue;
                }

                let data = &line[6..];
                if data == "[DONE]" {
                    break;
                }

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                        for choice in choices {
                            if let Some(delta) = choice.get("delta") {
                                if let Some(c) = delta.get("content").and_then(|v| v.as_str()) {
                                    full_content.push_str(c);
                                }
                                if let Some(r) = delta.get("reasoning").and_then(|v| v.as_str()) {
                                    full_reasoning.push_str(r);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(
            "Stream complete: reasoning={} chars, content={} chars",
            full_reasoning.len(),
            full_content.len()
        );

        if !full_content.is_empty() {
            Ok(full_content)
        } else if !full_reasoning.is_empty() {
            Ok(full_reasoning)
        } else {
            Err(LlmError::InvalidResponse(
                "Empty stream response".to_string(),
            ))
        }
    }
}
