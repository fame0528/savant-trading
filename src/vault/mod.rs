//! Glass House — Obsidian vault integration for transparent trading state.
//!
//! Projects engine state (trades, decisions, portfolio, insight, knowledge,
//! sessions, risk) into an Obsidian vault as structured markdown.
//! Monitors vault for user edits and ingests them as ground truth (Lessons/).

pub mod config;
pub mod watcher;
pub mod writer;

pub use config::VaultConfig;
pub use watcher::VaultWatcher;
pub use writer::VaultWriter;
