//! Vault configuration.

use serde::{Deserialize, Serialize};

/// Configuration for the Glass House vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Whether vault projection is enabled.
    pub enabled: bool,
    /// Path to the Obsidian vault directory.
    pub vault_path: String,
    /// How often to project state (seconds).
    pub sync_interval_secs: u64,
    /// Maximum number of files before cold storage.
    pub max_files: usize,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vault_path: "./savant-vault".to_string(),
            sync_interval_secs: 60,
            max_files: 15000,
        }
    }
}
