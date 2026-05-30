//! VaultWriter — projects engine state into the Obsidian vault.
//!
//! Writes structured markdown files for each directory:
//! - Trades/ — Daily trade logs
//! - Decisions/ — AI decision logs
//! - Portfolio/ — Balance history, equity curve
//! - Insight/ — Market context snapshots
//! - Knowledge/ — Knowledge unit index
//! - Sessions/ — Session-specific notes
//! - Risk/ — Circuit breaker events
//! - Lessons/ — User-editable ground truth

use chrono::Utc;
use std::fs;
use std::path::Path;
use tracing::{debug, info};

use crate::vault::config::VaultConfig;

/// Projects engine state into the Obsidian vault.
pub struct VaultWriter {
    config: VaultConfig,
}

impl VaultWriter {
    /// Create a new vault writer.
    pub fn new(config: VaultConfig) -> Self {
        Self { config }
    }

    /// Initialize the vault directory structure.
    pub fn scaffold(&self) -> std::io::Result<()> {
        let root = Path::new(&self.config.vault_path);

        let dirs = [
            ".obsidian",
            "Trades",
            "Decisions",
            "Portfolio",
            "Insight",
            "Knowledge",
            "Sessions",
            "Risk",
            "Lessons",
        ];

        for dir in &dirs {
            let path = root.join(dir);
            if !path.exists() {
                fs::create_dir_all(&path)?;
                debug!("Created vault directory: {:?}", path);
            }
        }

        // Write .obsidian/appearance.json
        let appearance = r##"{"accentColor":"#00d5ff","theme":"obsidian-dark","cssTheme":""}"##;
        let appearance_path = root.join(".obsidian").join("appearance.json");
        if !appearance_path.exists() {
            fs::write(&appearance_path, appearance)?;
        }

        // Write INDEX.md
        let index = self.generate_index();
        fs::write(root.join("INDEX.md"), index)?;

        info!("Vault scaffolded at {:?}", root);
        Ok(())
    }

    /// Project a trade into the vault.
    pub fn project_trade(&self, trade: &crate::core::types::TradeRecord) -> std::io::Result<()> {
        let root = Path::new(&self.config.vault_path);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let trades_dir = root.join("Trades");

        let file_path = trades_dir.join(format!("{}.md", today));
        let entry = format!(
            "- **{}** {} @ {:.2} → {:.2} | PnL: ${:.2} ({:.1}%) | {}\n",
            trade.pair,
            if trade.pnl > 0.0 { "WIN" } else { "LOSS" },
            trade.entry_price,
            trade.exit_price,
            trade.pnl,
            trade.pnl_pct,
            trade.closed_at.format("%H:%M:%S UTC")
        );

        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?;
            content.push_str(&entry);
            fs::write(&file_path, content)?;
        } else {
            let header = format!("# Trades — {}\n\n", today);
            fs::write(&file_path, format!("{}{}", header, entry))?;
        }

        debug!("Projected trade to vault: {}", trade.pair);
        Ok(())
    }

    /// Project an AI decision into the vault.
    pub fn project_decision(
        &self,
        pair: &str,
        action: &str,
        confidence: f64,
        reasoning: &str,
    ) -> std::io::Result<()> {
        let root = Path::new(&self.config.vault_path);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let decisions_dir = root.join("Decisions");

        let file_path = decisions_dir.join(format!("{}.md", today));
        let entry = format!(
            "- **[{}]** {} {} | Conf: {:.0}% | {}\n",
            Utc::now().format("%H:%M:%S"),
            action,
            pair,
            confidence * 100.0,
            reasoning
        );

        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?;
            content.push_str(&entry);
            fs::write(&file_path, content)?;
        } else {
            let header = format!("# AI Decisions — {}\n\n", today);
            fs::write(&file_path, format!("{}{}", header, entry))?;
        }

        debug!("Projected decision to vault: {} {}", action, pair);
        Ok(())
    }

    /// Project portfolio state into the vault.
    pub fn project_portfolio(
        &self,
        balance: f64,
        equity: f64,
        drawdown: f64,
    ) -> std::io::Result<()> {
        let root = Path::new(&self.config.vault_path);
        let portfolio_dir = root.join("Portfolio");

        let content = format!(
            "# Portfolio Snapshot\n\n\
             **Updated:** {}\n\n\
             | Metric | Value |\n\
             |--------|-------|\n\
             | Balance | ${:.2} |\n\
             | Equity | ${:.2} |\n\
             | Drawdown | {:.1}% |\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            balance,
            equity,
            drawdown * 100.0
        );

        fs::write(portfolio_dir.join("Balance-History.md"), content)?;
        Ok(())
    }

    /// Generate the vault INDEX.md with wiki-links.
    fn generate_index(&self) -> String {
        format!(
            "# SAVANT TRADING VAULT\n\n\
             **Last Updated:** {}\n\n\
             ## Directories\n\n\
             - [[Trades/|Trades]] — Daily trade logs\n\
             - [[Decisions/|Decisions]] — AI decision logs\n\
             - [[Portfolio/|Portfolio]] — Balance history, equity curve\n\
             - [[Insight/|Insight]] — Market context snapshots\n\
             - [[Knowledge/|Knowledge]] — Knowledge unit index\n\
             - [[Sessions/|Sessions]] — Session-specific notes\n\
             - [[Risk/|Risk]] — Circuit breaker events\n\
             - [[Lessons/|Lessons]] — User-editable ground truth\n\n\
             ## How It Works\n\n\
             This vault is auto-projected by the Savant Trading engine. \
             Most directories are **read-only** (engine → vault). \
             The **Lessons/** directory is **editable** (vault → engine) — \
             your edits are ingested as ground truth.\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// Ensure the vault directory exists and is scaffolded.
    pub fn ensure_scaffolded(&self) -> std::io::Result<()> {
        let root = Path::new(&self.config.vault_path);
        if !root.exists() || !root.join("INDEX.md").exists() {
            self.scaffold()?;
        }
        Ok(())
    }
}
