//! Jury Key Manager — lifecycle management for jury API keys.
//!
//! Creates ephemeral child API keys via the OpenRouter Management API,
//! tracks key health, rotates failed keys, and cleans up on shutdown.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::agent::openrouter_management::{
    CreateKeyRequest, ManagementError, OpenRouterManagementClient,
};
use crate::core::config::JuryConfig;

/// Errors from jury key management.
#[derive(Debug, thiserror::Error)]
pub enum JuryKeyError {
    #[error("Management API error: {0}")]
    Management(#[from] ManagementError),
    #[error("No keys available — all rotated or exhausted")]
    NoKeysAvailable,
    #[error("Management key not configured")]
    NotConfigured,
}

/// A single jury API key with health tracking.
#[derive(Debug, Clone)]
pub struct JuryKey {
    /// The raw API key string (used in Authorization header).
    pub api_key: String,
    /// OpenRouter key hash (used for delete/update operations).
    pub hash: String,
    /// Label assigned to this key.
    pub label: String,
    /// Consecutive failures — rotated after threshold.
    pub consecutive_failures: Arc<AtomicU32>,
}

/// Manages the lifecycle of jury API keys.
///
/// - Creates N ephemeral keys at startup via the Management API
/// - Round-robin key acquisition for jury evaluations
/// - Rotates keys that exceed the failure threshold
/// - Deletes all keys on shutdown
pub struct JuryKeyManager {
    client: OpenRouterManagementClient,
    keys: Arc<Mutex<Vec<JuryKey>>>,
    config: JuryConfig,
    next_index: Arc<AtomicU32>,
}

impl JuryKeyManager {
    /// Create a new key manager. Does NOT create keys yet — call `initialize()`.
    pub fn new(client: OpenRouterManagementClient, config: JuryConfig) -> Self {
        Self {
            client,
            keys: Arc::new(Mutex::new(Vec::new())),
            config,
            next_index: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Initialize jury keys: create N keys via the Management API.
    /// Cleans up any orphaned keys from previous runs first.
    pub async fn initialize(&self) -> Result<usize, JuryKeyError> {
        let prefix = &self.config.key_prefix;

        // Step 1: Clean up orphaned keys from previous runs
        self.cleanup_orphaned_keys(prefix).await;

        // Step 2: Create fresh keys
        let mut keys = Vec::new();
        for i in 0..self.config.jury_size {
            let label = format!("{}-{}-{}", prefix, i, chrono::Utc::now().timestamp());
            match self
                .client
                .create_key(CreateKeyRequest {
                    name: label.clone(),
                    limit: None,        // Free models = $0 cost
                    limit_reset: None,
                    include_byok_in_limit: None,
                })
                .await
            {
                Ok(created) => {
                    let key_preview = if created.key.len() > 12 {
                        format!("{}...{}", &created.key[..8], &created.key[created.key.len()-4..])
                    } else {
                        created.key.clone()
                    };
                    tracing::debug!(
                        "Jury key [{}] created: key_preview={}, hash={}",
                        i, key_preview, created.data.hash.as_deref().unwrap_or("unknown")
                    );
                    keys.push(JuryKey {
                        api_key: created.key,
                        hash: created.data.hash.unwrap_or_default(),
                        label,
                        consecutive_failures: Arc::new(AtomicU32::new(0)),
                    });
                }
                Err(e) => {
                    warn!("Failed to create jury key {}: {}", i, e);
                    // Continue creating remaining keys — partial failure is acceptable
                }
            }
        }

        let count = keys.len();
        info!("Jury keys created: {}/{}", count, self.config.jury_size);
        let mut stored = self.keys.lock().await;
        *stored = keys;
        Ok(count)
    }

    /// Acquire the next available key (round-robin).
    /// Skips keys that have exceeded the failure threshold.
    pub async fn acquire_key(&self) -> Option<JuryKey> {
        let keys = self.keys.lock().await;
        if keys.is_empty() {
            return None;
        }

        let len = keys.len() as u32;
        let start = self.next_index.fetch_add(1, Ordering::Relaxed) % len;

        for i in 0..len {
            let idx = ((start + i) % len) as usize;
            let key = &keys[idx];
            let failures = key.consecutive_failures.load(Ordering::Relaxed);
            if failures < self.config.max_consecutive_failures {
                return Some(key.clone());
            }
        }

        // All keys exceeded failure threshold — return the least-failed one
        keys.iter()
            .min_by_key(|k| k.consecutive_failures.load(Ordering::Relaxed))
            .cloned()
    }

    /// Record a failure for a specific key (by hash).
    pub async fn record_failure(&self, hash: &str) {
        let keys = self.keys.lock().await;
        if let Some(key) = keys.iter().find(|k| k.hash == hash) {
            let failures = key.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
            if failures >= self.config.max_consecutive_failures {
                warn!(
                    "Jury key '{}' exceeded failure threshold ({}/{})",
                    key.label, failures, self.config.max_consecutive_failures
                );
            }
        }
    }

    /// Record a success for a specific key (resets failure counter).
    pub async fn record_success(&self, hash: &str) {
        let keys = self.keys.lock().await;
        if let Some(key) = keys.iter().find(|k| k.hash == hash) {
            key.consecutive_failures.store(0, Ordering::Relaxed);
        }
    }

    /// Delete all jury keys. Called on shutdown (or by Drop impl).
    #[allow(dead_code)] // Called by Drop impl; also available for manual/testing cleanup
    pub async fn cleanup_all(&self) {
        let keys = self.keys.lock().await;
        for key in keys.iter() {
            if let Err(e) = self.client.delete_key(&key.hash).await {
                warn!("Failed to delete jury key '{}': {}", key.label, e);
            } else {
                info!("Deleted jury key: {}", key.label);
            }
        }
        info!("Jury key cleanup complete: {} keys deleted", keys.len());
    }

    /// Clean up orphaned keys from previous runs (matching prefix).
    async fn cleanup_orphaned_keys(&self, prefix: &str) {
        match self.client.list_keys(None).await {
            Ok(existing) => {
                let orphaned: Vec<_> = existing
                    .iter()
                    .filter(|k| k.name.as_deref().unwrap_or("").starts_with(prefix))
                    .collect();
                if !orphaned.is_empty() {
                    info!(
                        "Cleaning up {} orphaned jury keys from previous run",
                        orphaned.len()
                    );
                    for key in &orphaned {
                        if let Err(e) = self.client.delete_key(key.hash.as_deref().unwrap_or("")).await {
                            warn!("Failed to delete orphaned key '{}': {}", key.name.as_deref().unwrap_or("?"), e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to list keys for orphan cleanup: {}", e);
            }
        }
    }

    /// Number of active keys.
    #[allow(dead_code)] // Phase 2: used for metrics reporting
    pub async fn key_count(&self) -> usize {
        self.keys.lock().await.len()
    }

    /// Whether the manager has any keys.
    #[allow(dead_code)] // Phase 2: used for health checks
    pub async fn has_keys(&self) -> bool {
        !self.keys.lock().await.is_empty()
    }
}

/// Best-effort key cleanup on drop (Ctrl+C, panic, normal exit).
/// The runtime may already be shutting down, so we try block_on and
/// fall back to a warning if it fails. Startup orphan cleanup catches
/// any keys we miss here.
impl Drop for JuryKeyManager {
    fn drop(&mut self) {
        // Try to take the keys out of the Arc<Mutex> without async.
        // try_lock() is non-async and will fail if another thread holds the lock.
        let keys = match self.keys.try_lock() {
            Ok(guard) => guard.clone(),
            Err(_) => {
                warn!("JuryKeyManager::drop: mutex locked, cannot cleanup keys");
                return;
            }
        };
        if keys.is_empty() {
            return;
        }
        // Attempt async cleanup via the tokio runtime. If the runtime is
        // already shut down (e.g. Ctrl+C), this fails and we log a warning.
        // Startup orphan cleanup (cleanup_orphaned_keys) catches any misses.
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                let client = &self.client;
                let _ = handle.block_on(async {
                    for key in &keys {
                        if let Err(e) = client.delete_key(&key.hash).await {
                            warn!("Drop: failed to delete jury key '{}': {}", key.label, e);
                        } else {
                            info!("Drop: deleted jury key: {}", key.label);
                        }
                    }
                    info!("Drop: jury key cleanup complete: {} keys deleted", keys.len());
                });
            }
            Err(_) => {
                warn!(
                    "JuryKeyManager::drop: no tokio runtime — {} jury keys will be cleaned up on next startup",
                    keys.len()
                );
            }
        }
    }
}
