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
    /// FID-166: Separate, lower timeout for streaming calls. M3 should respond in
    /// 30s. 60s gives headroom. Stalled streaming upstreams fail fast and fall back
    /// to non-streaming.
    pub streaming_timeout_secs: u64,
    pub extra_headers: Vec<(String, String)>,
    /// FID-138: Disable chain-of-thought reasoning for models that support it.
    /// When true, adds `"thinking": {"type": "disabled"}` to the request body.
    /// Only effective for reasoning models (MiniMax M3, DeepSeek R1, etc.).
    pub disable_thinking: bool,
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
            streaming_timeout_secs: 60,
            extra_headers: vec![],
            disable_thinking: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct LlmProvider {
    /// Non-streaming client (chat, fallback). Uses `timeout_secs` (default 300s).
    client: reqwest::Client,
    /// FID-166: Streaming client (chat_stream). Uses `streaming_timeout_secs`
    /// (default 60s) so stalled upstreams fail fast and fall back to non-streaming.
    streaming_client: reqwest::Client,
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
                    streaming_timeout_secs: ai_cfg.streaming_timeout_secs,
                    extra_headers: vec![],
                    disable_thinking: ai_cfg.disable_thinking,
                },
                vec![
                    ("HTTP-Referer".to_string(), or.referer.clone()),
                    ("X-OpenRouter-Title".to_string(), or.title.clone()),
                ],
            )
        }
        "ollama" => (                LlmConfig {
                    endpoint: "http://localhost:11434/v1".to_string(),
                    model: ai_cfg.model.clone(),
                    api_key: String::new(),
                    max_tokens: ai_cfg.max_tokens,
                    temperature: ai_cfg.temperature,
                    top_p: ai_cfg.top_p,
                    timeout_secs: ai_cfg.timeout_secs.max(300),
                    streaming_timeout_secs: ai_cfg.streaming_timeout_secs.max(300),
                    extra_headers: vec![],
                    disable_thinking: ai_cfg.disable_thinking,
                },
            vec![],
        ),
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
                    streaming_timeout_secs: ai_cfg.streaming_timeout_secs,
                    extra_headers: vec![],
                    disable_thinking: ai_cfg.disable_thinking,
                },
                vec![],
            )
        }
        "tokenrouter" => {
            let tr = &ai_cfg.tokenrouter;
            (
                LlmConfig {
                    endpoint: tr.endpoint.clone(),
                    model: tr.model.clone(),
                    api_key: std::env::var(&tr.api_key_env).unwrap_or_default(),
                    max_tokens: ai_cfg.max_tokens,
                    temperature: ai_cfg.temperature,
                    top_p: ai_cfg.top_p,
                    timeout_secs: ai_cfg.timeout_secs,
                    streaming_timeout_secs: ai_cfg.streaming_timeout_secs,
                    extra_headers: vec![],
                    disable_thinking: ai_cfg.disable_thinking,
                },
                vec![],
            )
        }
        _ => (                LlmConfig {
                    endpoint: ai_cfg.endpoint.clone(),
                    model: ai_cfg.model.clone(),
                    api_key: std::env::var(&ai_cfg.api_key_env).unwrap_or_default(),
                    max_tokens: ai_cfg.max_tokens,
                    temperature: ai_cfg.temperature,
                    top_p: ai_cfg.top_p,
                    timeout_secs: ai_cfg.timeout_secs,
                    streaming_timeout_secs: ai_cfg.streaming_timeout_secs,
                    extra_headers: vec![],
                    disable_thinking: ai_cfg.disable_thinking,
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
        let streaming_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.streaming_timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            streaming_client,
            config,
        }
    }

    pub fn config_clone(&self) -> LlmConfig {
        self.config.clone()
    }

    pub async fn chat(&self, system: &str, messages: &[Message]) -> Result<String, LlmError> {
        let body = self.build_body(system, messages, false);
        let max_attempts = 3;
        let mut last_err = String::new();

        for attempt in 0..max_attempts {
            let resp = match self.send_with_retry(&body, 3, "chat()", false).await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("{}", e);
                    if attempt < max_attempts - 1 {
                        let wait = 2u64.pow(attempt as u32 + 1);
                        tracing::warn!(
                            "chat() parse-retry (attempt {}/{}): {}. Retrying in {}s...",
                            attempt + 1,
                            max_attempts,
                            last_err,
                            wait
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                        continue;
                    }
                    return Err(e);
                }
            };

            match Self::parse_non_streaming(resp).await {
                Ok(content) => return Ok(content),
                Err(e) => {
                    last_err = format!("{}", e);
                    if attempt < max_attempts - 1 {
                        let wait = 2u64.pow(attempt as u32 + 1);
                        tracing::warn!(
                            "chat() parse failed (attempt {}/{}): {}. Retrying in {}s...",
                            attempt + 1,
                            max_attempts,
                            last_err,
                            wait
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(LlmError::Http(format!(
            "All {} attempts failed: {}",
            max_attempts, last_err
        )))
    }

    pub async fn chat_stream(
        &self,
        system: &str,
        messages: &[Message],
    ) -> Result<String, LlmError> {
        let stream_body = self.build_body(system, messages, true);
        let max_retries = 1;
        let mut last_err = String::new();

        for attempt in 0..max_retries {
            let resp = match self.send_with_retry(&stream_body, 2, "Stream", true).await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("{}", e);
                    break;
                }
            };

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

        tracing::warn!(
            "All {} streaming attempts failed ({}). Falling back to non-streaming.",
            max_retries,
            last_err
        );
        let body = self.build_body(system, messages, false);
        let resp = self.send_with_retry(&body, 1, "Fallback", false).await?;
        Self::parse_non_streaming(resp).await
    }

    pub(crate) fn build_body(&self, system: &str, messages: &[Message], stream: bool) -> serde_json::Value {
        // FID-085: Check model capabilities for cache_control support
        let caps = crate::agent::provider_caps::ModelCapabilities::for_model(&self.config.model);

        // KV cache optimization (FID-035): Mark system message with cache_control
        // so OpenRouter can cache the static prefix (rules, risk, knowledge).
        // Dynamic content (candles, indicators) is in user messages only.
        // Expected: 40-60% reduction in TTFT for repeated evaluations.
        let system_msg = if caps.supports_cache_control {
            serde_json::json!({
                "role": "system",
                "content": system,
                "cache_control": { "type": "ephemeral" }
            })
        } else {
            serde_json::json!({
                "role": "system",
                "content": system
            })
        };

        let mut all_messages = vec![system_msg];
        for msg in messages {
            all_messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": all_messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "top_p": self.config.top_p,
            "stream": stream,
        });

        // FID-138: Disable chain-of-thought for reasoning models (MiniMax M3, etc.)
        // that exhaust the token budget on <think> blocks before emitting JSON.
        if self.config.disable_thinking {
            let model_lower = self.config.model.to_lowercase();
            let is_reasoning_model = model_lower.contains("m3")
                || model_lower.contains("minimax")
                || model_lower.contains("m1")
                || model_lower.contains("deepseek-r1")
                || model_lower.contains("qwq");
            if is_reasoning_model {
                body["thinking"] = serde_json::json!({"type": "disabled"});
                tracing::debug!(
                    "FID-138: thinking disabled for reasoning model {}",
                    self.config.model
                );
            }
        }

        body
    }

    async fn send_request(
        &self,
        body: &serde_json::Value,
        use_streaming_client: bool,
    ) -> Result<reqwest::Response, LlmError> {
        let url = format!("{}/chat/completions", self.config.endpoint);
        let client = if use_streaming_client {
            &self.streaming_client
        } else {
            &self.client
        };
        let mut req_builder = client
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

    /// Shared retry logic for HTTP requests. Handles 429 (with retry-after),
    /// 502/503/504/529 (transient, FID-166 added 504), and connection errors
    /// with exponential backoff. Returns the successful response for the caller.
    /// `use_streaming_client` selects between the regular and streaming reqwest
    /// clients (different timeouts).
    async fn send_with_retry(
        &self,
        body: &serde_json::Value,
        max_attempts: u32,
        label: &str,
        use_streaming_client: bool,
    ) -> Result<reqwest::Response, LlmError> {
        let mut last_err = String::new();

        for attempt in 0..max_attempts {
            let resp = match self.send_request(body, use_streaming_client).await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("{}", e);
                    if attempt < max_attempts - 1 {
                        let wait = 2u64.pow(attempt + 1);
                        tracing::warn!(
                            "{} request failed (attempt {}/{}): {}. Retrying in {}s...",
                            label,
                            attempt + 1,
                            max_attempts,
                            last_err,
                            wait
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                        continue;
                    }
                    return Err(e);
                }
            };

            let status = resp.status();
            if status == 429 {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60);
                tracing::warn!(
                    "{} rate limited (attempt {}/{}). Waiting {}s...",
                    label,
                    attempt + 1,
                    max_attempts,
                    retry_after
                );
                tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                last_err = format!("Rate limited (429), waited {}s", retry_after);
                continue;
            }
            if status == 502 || status == 503 || status == 504 || status == 529 {
                last_err = format!("HTTP {} (transient)", status);
                if attempt < max_attempts - 1 {
                    let wait = 2u64.pow(attempt + 1);
                    tracing::warn!(
                        "{} HTTP {} (attempt {}/{}). Retrying in {}s...",
                        label,
                        status,
                        attempt + 1,
                        max_attempts,
                        wait
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                    continue;
                }
                let text = resp.text().await.unwrap_or_default();
                return Err(LlmError::Http(format!("HTTP {}: {}", status, text)));
            }
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(LlmError::Http(format!("HTTP {}: {}", status, text)));
            }

            return Ok(resp);
        }

        Err(LlmError::Http(format!(
            "All {} attempts failed: {}",
            max_attempts, last_err
        )))
    }

    pub(crate) async fn parse_non_streaming(resp: reqwest::Response) -> Result<String, LlmError> {
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
                let content_str = m.get("content")
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty());
                // FID-138: If content is empty but reasoning is present, the model
                // may have exhausted its token budget on chain-of-thought.
                // Log a warning but still fall back to reasoning for backward compat
                // with models (like old mimo v2.5 pro) that output in the reasoning field.
                // The real fix is disable_thinking + reduced max_tokens at the provider level.
                if content_str.is_none() {
                    if let Some(reasoning) = m.get("reasoning").and_then(|r| r.as_str()) {
                        if !reasoning.is_empty() {
                            tracing::warn!(
                                "FID-138: content empty, falling back to reasoning field ({} chars). Consider disable_thinking=true.",
                                reasoning.len()
                            );
                        }
                    }
                }
                content_str
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

    /// Chat with model/key/timeout override — used by the Model Jury system.
    ///
    /// Creates a temporary reqwest::Client with the specified timeout,
    /// overrides the model and API key, and optionally strips cache_control
    /// for free models that don't support it.
    pub async fn chat_with_override(
        &self,
        system: &str,
        messages: &[Message],
        model: &str,
        api_key: &str,
        timeout_secs: u64,
        no_cache: bool,
    ) -> Result<String, LlmError> {
        let mut body = self.build_body(system, messages, false);
        body["model"] = serde_json::Value::String(model.to_string());

        // Strip cache_control for free models (they don't support it)
        if no_cache {
            if let Some(sys_msg) = body["messages"].get_mut(0) {
                if let Some(obj) = sys_msg.as_object_mut() {
                    obj.remove("cache_control");
                }
            }
        }

        // Create temporary client with jury-specific timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let max_attempts = 3u32;
        let mut last_err = String::new();

        for attempt in 0..max_attempts {
            let url = format!("{}/chat/completions", self.config.endpoint);
            let mut req_builder = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key));
            for (name, value) in &self.config.extra_headers {
                req_builder = req_builder.header(name.as_str(), value.as_str());
            }

            let resp = match req_builder.json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("{}", e);
                    if attempt < max_attempts - 1 {
                        let wait = 2u64.pow(attempt + 1);
                        tracing::warn!(
                            "jury request failed (attempt {}/{}): {}. Retrying in {}s...",
                            attempt + 1, max_attempts, last_err, wait
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                        continue;
                    }
                    return Err(LlmError::Http(last_err));
                }
            };

            let status = resp.status();
            if status == 429 || status == 502 || status == 503 || status == 504 || status == 529 {
                last_err = format!("HTTP {} (transient)", status);
                if attempt < max_attempts - 1 {
                    let wait = 2u64.pow(attempt + 1);
                    tracing::warn!(
                        "jury HTTP {} (attempt {}/{}). Retrying in {}s...",
                        status, attempt + 1, max_attempts, wait
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                    continue;
                }
                let text = resp.text().await.unwrap_or_default();
                return Err(LlmError::Http(format!("HTTP {}: {}", status, text)));
            }
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(LlmError::Http(format!("HTTP {}: {}", status, text)));
            }

            return Self::parse_non_streaming(resp).await;
        }

        Err(LlmError::Http(format!(
            "All {} jury attempts failed: {}",
            max_attempts, last_err
        )))
    }
}
