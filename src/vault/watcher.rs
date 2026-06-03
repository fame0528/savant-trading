//! VaultWatcher — monitors the vault for user edits and ingests them.
//!
//! Watches the Lessons/ directory for new or modified files.
//! User edits in Lessons/ are ingested as ground truth into the engine.
//!
//! Injection defense: scans file content for prompt injection patterns
//! before ingesting.

use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Injection patterns to scan for (from Savant's scan_prompt).
const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous",
    "ignore all",
    "disregard",
    "forget your instructions",
    "new instructions",
    "system prompt",
    "you are now",
    "act as",
    "pretend to be",
    "override",
    "\u{200b}", // zero-width space
    "\u{200c}", // zero-width non-joiner
    "\u{200d}", // zero-width joiner
    "\u{feff}", // BOM
];

/// Monitors the vault for user edits.
pub struct VaultWatcher {
    vault_path: PathBuf,
    lessons_path: PathBuf,
}

impl VaultWatcher {
    /// Create a new vault watcher.
    pub fn new(vault_path: &str) -> Self {
        let path = PathBuf::from(vault_path);
        Self {
            lessons_path: path.join("Lessons"),
            vault_path: path,
        }
    }

    /// Scan a file for injection patterns.
    /// Returns true if any pattern is found. Logs only the first matched pattern.
    pub fn scan_for_injection(&self, content: &str) -> bool {
        let lower = content.to_lowercase();
        for pattern in INJECTION_PATTERNS {
            if lower.contains(pattern) {
                // Only log the first pattern found per file to reduce spam
                return true;
            }
        }
        false
    }

    /// Read all lesson files from the Lessons/ directory.
    pub fn read_lessons(&self) -> Vec<(String, String)> {
        let mut lessons = Vec::new();

        if !self.lessons_path.exists() {
            return lessons;
        }

        if let Ok(entries) = std::fs::read_dir(&self.lessons_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if !self.scan_for_injection(&content) {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            lessons.push((name, content));
                            debug!("Read lesson: {:?}", path.file_name());
                        } else {
                            warn!(
                                "Skipping lesson with injection patterns: {:?}",
                                path.file_name()
                            );
                        }
                    }
                }
            }
        }

        lessons
    }

    /// Get the vault path.
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }
}
