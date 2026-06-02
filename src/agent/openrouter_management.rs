//! OpenRouter Management API client.
//!
//! Provides programmatic CRUD operations for API keys via the
//! [Management API](https://openrouter.ai/docs/guides/overview/auth/management-api-keys).
//!
//! A management key (created on the OpenRouter dashboard) is required.
//! Management keys **cannot** be used for LLM completion calls — only
//! for admin operations like key creation, rotation, and usage monitoring.
//!
//! # Usage
//!
//! ```rust,no_run
//! use savant_trading::agent::openrouter_management::{OpenRouterManagementClient, CreateKeyRequest};
//!
//! # async fn example() {
//! let mgmt_key = std::env::var("OPENROUTER_MANAGEMENT_KEY").unwrap();
//! let client = OpenRouterManagementClient::new(mgmt_key);
//!
//! // List existing keys
//! let keys = client.list_keys(None).await.unwrap();
//!
//! // Create a new key with a credit limit
//! let new_key = client.create_key(CreateKeyRequest {
//!     name: "paper-trading-key".into(),
//!     limit: Some(10.0),
//!     ..Default::default()
//! }).await.unwrap();
//! println!("New key: {}", new_key.key);
//! # }
//! ```

use serde::{Deserialize, Serialize};

/// Errors returned by the OpenRouter Management API.
#[derive(Debug, thiserror::Error)]
pub enum ManagementError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("API error: {status} — {body}")]
    Api { status: u16, body: String },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Full key information returned by all management endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiKeyInfo {
    pub created_at: String,
    pub updated_at: String,
    pub hash: String,
    pub label: String,
    pub name: String,
    pub disabled: bool,
    pub limit: f64,
    pub limit_remaining: f64,
    pub limit_reset: Option<String>,
    #[serde(default)]
    pub include_byok_in_limit: bool,
    pub usage: f64,
    pub usage_daily: f64,
    pub usage_weekly: f64,
    pub usage_monthly: f64,
    pub byok_usage: f64,
    pub byok_usage_daily: f64,
    pub byok_usage_weekly: f64,
    pub byok_usage_monthly: f64,
}

/// Response from `POST /api/v1/keys` — includes the raw key string.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatedKey {
    pub data: ApiKeyInfo,
    /// The actual API key string (only returned on creation, never again).
    pub key: String,
}

/// Request body for creating a new API key.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateKeyRequest {
    /// Human-readable label for the key.
    pub name: String,
    /// Optional credit limit (in USD). Key is disabled when limit is reached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    /// Whether to include BYOK (bring-your-own-key) usage in the limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_byok_in_limit: Option<bool>,
    /// How often to reset the limit: "daily", "weekly", "monthly".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_reset: Option<String>,
}

/// Request body for updating an existing API key.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateKeyRequest {
    /// New human-readable label for the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether to disable the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    /// Whether to include BYOK usage in the limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_byok_in_limit: Option<bool>,
    /// How often to reset the limit: "daily", "weekly", "monthly".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_reset: Option<String>,
}

/// List response wrapper.
#[derive(Debug, Clone, Deserialize)]
struct ListKeysResponse {
    data: Vec<ApiKeyInfo>,
}

/// OpenRouter Management API client.
///
/// Requires a management key (not a regular API key). Management keys
/// are created on the [OpenRouter dashboard](https://openrouter.ai/settings/management-keys)
/// and cannot be used for LLM completion calls.
#[derive(Debug, Clone)]
pub struct OpenRouterManagementClient {
    client: reqwest::Client,
    base_url: String,
    management_key: String,
}

impl OpenRouterManagementClient {
    /// Create a new management client with the default endpoint.
    pub fn new(management_key: String) -> Self {
        Self::with_endpoint(management_key, "https://openrouter.ai/api/v1/keys")
    }

    /// Create a new management client with a custom endpoint.
    pub fn with_endpoint(management_key: String, endpoint: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: endpoint.trim_end_matches('/').to_string(),
            management_key,
        }
    }

    /// List the most recent API keys, with optional pagination offset.
    pub async fn list_keys(&self, offset: Option<usize>) -> Result<Vec<ApiKeyInfo>, ManagementError> {
        let mut url = self.base_url.clone();
        if let Some(off) = offset {
            url.push_str(&format!("?offset={}", off));
        }
        let resp = self.get(&url).await?;
        let data: ListKeysResponse = serde_json::from_value(resp)
            .map_err(|e| ManagementError::InvalidResponse(format!("list_keys parse: {}", e)))?;
        Ok(data.data)
    }

    /// Create a new API key with the given parameters.
    pub async fn create_key(&self, req: CreateKeyRequest) -> Result<CreatedKey, ManagementError> {
        let body = serde_json::to_value(&req)
            .map_err(|e| ManagementError::InvalidResponse(format!("create_key serialize: {}", e)))?;
        let resp = self.post(&self.base_url, body).await?;
        serde_json::from_value(resp)
            .map_err(|e| ManagementError::InvalidResponse(format!("create_key parse: {}", e)))
    }

    /// Get detailed information about a specific key by its hash.
    pub async fn get_key(&self, hash: &str) -> Result<ApiKeyInfo, ManagementError> {
        let url = format!("{}/{}", self.base_url, hash);
        let resp = self.get(&url).await?;
        // GET /keys/{hash} returns `{ "data": { ... } }` (single object, not array)
        let data: SingleKeyResponse = serde_json::from_value(resp)
            .map_err(|e| ManagementError::InvalidResponse(format!("get_key parse: {}", e)))?;
        Ok(data.data)
    }

    /// Update an existing API key (name, disabled, limit_reset, etc.).
    pub async fn update_key(&self, hash: &str, req: UpdateKeyRequest) -> Result<ApiKeyInfo, ManagementError> {
        let url = format!("{}/{}", self.base_url, hash);
        let body = serde_json::to_value(&req)
            .map_err(|e| ManagementError::InvalidResponse(format!("update_key serialize: {}", e)))?;
        let resp = self.patch(&url, body).await?;
        let data: SingleKeyResponse = serde_json::from_value(resp)
            .map_err(|e| ManagementError::InvalidResponse(format!("update_key parse: {}", e)))?;
        Ok(data.data)
    }

    /// Delete an API key permanently.
    pub async fn delete_key(&self, hash: &str) -> Result<(), ManagementError> {
        let url = format!("{}/{}", self.base_url, hash);
        self.delete(&url).await
    }

    // --- HTTP helpers ---

    async fn get(&self, url: &str) -> Result<serde_json::Value, ManagementError> {
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.management_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| ManagementError::Http(format!("GET failed: {}", e)))?;
        self.check_response(resp).await
    }

    async fn post(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value, ManagementError> {
        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.management_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ManagementError::Http(format!("POST failed: {}", e)))?;
        self.check_response(resp).await
    }

    async fn patch(&self, url: &str, body: serde_json::Value) -> Result<serde_json::Value, ManagementError> {
        let resp = self
            .client
            .patch(url)
            .header("Authorization", format!("Bearer {}", self.management_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ManagementError::Http(format!("PATCH failed: {}", e)))?;
        self.check_response(resp).await
    }

    async fn delete(&self, url: &str) -> Result<(), ManagementError> {
        let resp = self
            .client
            .delete(url)
            .header("Authorization", format!("Bearer {}", self.management_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| ManagementError::Http(format!("DELETE failed: {}", e)))?;
        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ManagementError::Api {
                status: status.as_u16(),
                body,
            })
        }
    }

    async fn check_response(&self, resp: reqwest::Response) -> Result<serde_json::Value, ManagementError> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ManagementError::Api {
                status: status.as_u16(),
                body,
            });
        }
        resp.json()
            .await
            .map_err(|e| ManagementError::InvalidResponse(format!("JSON parse: {}", e)))
    }
}

/// Helper for single-key responses (GET /keys/{hash}, PATCH /keys/{hash}).
#[derive(Debug, Clone, Deserialize)]
struct SingleKeyResponse {
    data: ApiKeyInfo,
}
