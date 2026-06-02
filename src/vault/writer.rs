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
            "Sandbox",
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
        if !self.config.enabled {
            return Ok(());
        }
        let root = Path::new(&self.config.vault_path);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let trades_dir = root.join("Trades");

        let file_path = trades_dir.join(format!("{}.md", today));
        let emoji = if trade.pnl > 0.0 { "🟢" } else { "🔴" };
        let result = if trade.pnl > 0.0 { "WIN" } else { "LOSS" };

        let entry = format!(
            "\n### {} {} {} — {}\n\n\
             | Field | Value |\n\
             |-------|-------|\n\
             | **Pair** | {} |\n\
             | **Side** | {} |\n\
             | **Entry** | ${:.4} |\n\
             | **Exit** | ${:.4} |\n\
             | **Quantity** | {:.4} |\n\
             | **P&L** | ${:.2} ({:.1}%) |\n\
             | **Strategy** | {} |\n\
             | **Opened** | {} |\n\
             | **Closed** | {} |\n\
             | **Notes** | {} |\n\n\
             ---\n",
            emoji,
            trade.pair,
            result,
            trade.closed_at.format("%H:%M:%S UTC"),
            trade.pair,
            trade.side,
            trade.entry_price,
            trade.exit_price,
            trade.quantity,
            trade.pnl,
            trade.pnl_pct,
            trade.strategy_name,
            trade.opened_at.format("%Y-%m-%d %H:%M:%S UTC"),
            trade.closed_at.format("%Y-%m-%d %H:%M:%S UTC"),
            if trade.notes.is_empty() {
                "—".to_string()
            } else {
                trade.notes.clone()
            },
        );

        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?;
            content.push_str(&entry);
            fs::write(&file_path, content)?;
        } else {
            let header = format!(
                "# Trades — {}\n\n\
                 > Savant Trading Engine v0.4.4 | 24/7 Crypto Paper Trading\n\n\
                 | Metric | Value |\n\
                 |--------|-------|\n\
                 | Date | {} |\n\
                 | Engine | Live |\n\
                 | Budget | $50 |\n\n\
                 ---\n",
                today, today
            );
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
        if !self.config.enabled {
            return Ok(());
        }
        let root = Path::new(&self.config.vault_path);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let time = Utc::now().format("%H:%M:%S").to_string();
        let decisions_dir = root.join("Decisions");

        let file_path = decisions_dir.join(format!("{}.md", today));

        // Parse reasoning into structured sections
        let action_emoji = match action {
            "Buy" => "🟢",
            "Sell" => "🔴",
            "Hold" => "⏸️",
            _ => "📊",
        };
        let conf_bar = if confidence >= 0.7 {
            "███████████ HIGH"
        } else if confidence >= 0.4 {
            "███████░░░░ MED"
        } else {
            "████░░░░░░░ LOW"
        };

        let entry = format!(
            "\n### {} {} {} @ {}\n\n\
             | Field | Value |\n\
             |-------|-------|\n\
             | **Action** | {} {} |\n\
             | **Confidence** | {:.0}% ({}) |\n\
             | **Time** | {} UTC |\n\n\
             **Reasoning:**\n> {}\n\n\
             ---\n",
            action_emoji,
            pair,
            action,
            time,
            action_emoji,
            pair,
            confidence * 100.0,
            conf_bar,
            time,
            reasoning.replace(". ", ".\n> ")
        );

        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?;
            content.push_str(&entry);
            fs::write(&file_path, content)?;
        } else {
            let header = format!(
                "# AI Decisions — {}\n\n\
                 > Savant Trading Engine v0.4.4 | 24/7 Crypto Paper Trading\n\n\
                 | Metric | Value |\n\
                 |--------|-------|\n\
                 | Date | {} |\n\
                 | Engine | Live |\n\
                 | Budget | $50 |\n\n\
                 ---\n",
                today, today
            );
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
        if !self.config.enabled {
            return Ok(());
        }
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
             - [[Trades/|Trades]] — Daily trade logs (auto-populated)\n\
             - [[Decisions/|Decisions]] — AI decision logs (auto-populated)\n\
             - [[Portfolio/|Portfolio]] — Balance history, equity curve (auto-populated)\n\
             - [[Insight/|Insight]] — Market context snapshots (auto-populated)\n\
             - [[Knowledge/|Knowledge]] — Knowledge unit index (auto-populated)\n\
             - [[Sessions/|Sessions]] — Session-specific notes (auto-populated)\n\
             - [[Risk/|Risk]] — Circuit breaker events (auto-populated)\n\
             - [[Lessons/|Lessons]] — User-editable ground truth\n\n\
             ## How It Works\n\n\
             This vault is auto-projected by the Savant Trading engine. \
             Most directories are **read-only** (engine → vault). \
             The **Lessons/** directory is **editable** (vault → engine) — \
             your edits are ingested as ground truth.\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// Project market insight snapshot into the vault.
    pub fn project_insight(
        &self,
        fear_greed: Option<i32>,
        fear_greed_label: Option<&str>,
        funding_rate: Option<f64>,
        mvrv: Option<f64>,
        sopr: Option<f64>,
        session_and_rss: (&str, usize),
    ) -> std::io::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        let root = Path::new(&self.config.vault_path);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let time = Utc::now().format("%H:%M:%S").to_string();
        let insight_dir = root.join("Insight");

        let file_path = insight_dir.join(format!("{}.md", today));

        let (session_name, rss_count) = session_and_rss;

        let fg_str = match fear_greed {
            Some(fg) => {
                let label = fear_greed_label.unwrap_or("?");
                let emoji = if fg < 25 {
                    "🔴"
                } else if fg < 50 {
                    "🟡"
                } else {
                    "🟢"
                };
                format!("{} {} ({})", emoji, fg, label)
            }
            None => "N/A".to_string(),
        };

        let funding_str = match funding_rate {
            Some(fr) => {
                let emoji = if fr > 0.01 {
                    "🔴"
                } else if fr < -0.01 {
                    "🟢"
                } else {
                    "🟡"
                };
                format!("{} {:.4}%", emoji, fr * 100.0)
            }
            None => "N/A".to_string(),
        };

        let mvrv_str = match mvrv {
            Some(m) => {
                let emoji = if m > 3.5 {
                    "🔴"
                } else if m < 1.0 {
                    "🟢"
                } else {
                    "🟡"
                };
                format!("{} {:.2}", emoji, m)
            }
            None => "N/A (API blocked)".to_string(),
        };

        let sopr_str = match sopr {
            Some(s) => format!("{:.4}", s),
            None => "N/A".to_string(),
        };

        let entry = format!(
            "\n### Snapshot @ {} UTC\n\n\
             | Metric | Value |\n\
             |--------|-------|\n\
             | **Fear & Greed** | {} |\n\
             | **Funding Rate** | {} |\n\
             | **MVRV** | {} |\n\
             | **SOPR** | {} |\n\
             | **Session** | {} |\n\
             | **RSS Items** | {} |\n\n\
             ---\n",
            time, fg_str, funding_str, mvrv_str, sopr_str, session_name, rss_count,
        );

        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?;
            content.push_str(&entry);
            fs::write(&file_path, content)?;
        } else {
            let header = format!(
                "# Market Insight — {}\n\n\
                 > Auto-updated every 5 ticks by Savant Trading Engine\n\n\
                 ---\n",
                today
            );
            fs::write(&file_path, format!("{}{}", header, entry))?;
        }

        debug!("Projected insight to vault");
        Ok(())
    }

    /// Project knowledge base index into the vault.
    pub fn project_knowledge(&self, units: &[(String, String, String)]) -> std::io::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        let root = Path::new(&self.config.vault_path);
        let knowledge_dir = root.join("Knowledge");

        let mut content = String::from(
            "# Knowledge Base Index\n\n\
             > Auto-generated by Savant Trading Engine\n\n\
             | ID | Topic | Title |\n\
             |----|-------|-------|\n",
        );

        for (id, topic, title) in units {
            content.push_str(&format!("| {} | {} | {} |\n", id, topic, title));
        }

        content.push_str(&format!(
            "\n**Total Units:** {}\n\n**Last Updated:** {}\n",
            units.len(),
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        fs::write(knowledge_dir.join("INDEX.md"), content)?;
        debug!("Projected knowledge index to vault: {} units", units.len());
        Ok(())
    }

    /// Project circuit breaker event into the vault.
    pub fn project_risk_event(&self, event_type: &str, details: &str) -> std::io::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        let root = Path::new(&self.config.vault_path);
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let time = Utc::now().format("%H:%M:%S").to_string();
        let risk_dir = root.join("Risk");

        let file_path = risk_dir.join(format!("{}.md", today));

        let emoji = match event_type {
            "circuit_breaker" => "🚨",
            "block_file" => "🛑",
            "drawdown_warning" => "⚠️",
            _ => "📊",
        };

        let entry = format!(
            "\n### {} {} @ {} UTC\n\n> {}\n\n---\n",
            emoji, event_type, time, details
        );

        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?;
            content.push_str(&entry);
            fs::write(&file_path, content)?;
        } else {
            let header = format!(
                "# Risk Events — {}\n\n\
                 > Circuit breaker triggers, drawdown warnings, block file events\n\n\
                 ---\n",
                today
            );
            fs::write(&file_path, format!("{}{}", header, entry))?;
        }

        debug!("Projected risk event to vault: {}", event_type);
        Ok(())
    }

    /// Project sandbox report card to Vault/Sandbox/.
    pub fn project_sandbox(&self, report_md: &str) -> std::io::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        let root = Path::new(&self.config.vault_path);
        let sandbox_dir = root.join("Sandbox");
        fs::create_dir_all(&sandbox_dir)?;

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let file_path = sandbox_dir.join(format!("Report_{}.md", today));
        fs::write(&file_path, report_md)?;

        debug!("Projected sandbox report to vault");
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::config::VaultConfig;

    #[test]
    fn vault_config_default() {
        let config = VaultConfig::default();
        assert!(config.enabled);
        assert_eq!(config.vault_path, "./savant-vault");
        assert_eq!(config.max_files, 15000);
    }

    #[test]
    fn vault_writer_new() {
        let config = VaultConfig {
            enabled: false,
            ..Default::default()
        };
        let writer = VaultWriter::new(config);
        assert_eq!(writer.config.vault_path, "./savant-vault");
    }

    #[test]
    fn vault_writer_disabled_operations() {
        let config = VaultConfig {
            enabled: false,
            ..Default::default()
        };
        let writer = VaultWriter::new(config);

        // All operations should succeed silently when disabled
        assert!(writer
            .project_trade(&crate::core::types::TradeRecord {
                id: "test".to_string(),
                pair: "BTC/USD".to_string(),
                side: crate::core::types::Side::Long,
                entry_price: 100.0,
                exit_price: 105.0,
                quantity: 1.0,
                pnl: 5.0,
                pnl_pct: 5.0,
                strategy_name: "test".to_string(),
                opened_at: chrono::Utc::now(),
                closed_at: chrono::Utc::now(),
                notes: String::new(),
            })
            .is_ok());

        assert!(writer
            .project_decision("BTC/USD", "Buy", 0.8, "test reason")
            .is_ok());

        assert!(writer.project_portfolio(1000.0, 1000.0, 0.0).is_ok());
    }
}
