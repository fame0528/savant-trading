//! SAVANT TUI — Cyberpunk Multi-Tab Trading Terminal
//!
//! Full-screen interactive terminal with 10 tabs, keyboard navigation,
//! real-time data snapshots, and neon-cyberpunk aesthetic.
//!
//! ## Tabs
//!
//! | Key | Tab          | Content                                |
//! |-----|--------------|----------------------------------------|
//! | `1` | Overview     | Portfolio summary, sparkline, positions |
//! | `2` | Portfolio    | Equity/balance charts, drawdown         |
//! | `3` | Positions    | Full position table with detail         |
//! | `4` | Trades       | Closed trade history                    |
//! | `5` | Insight      | F&G, funding, on-chain, news            |
//! | `6` | Decisions    | AI decision log                         |
//! | `7` | Risk         | Circuit breaker, limits, thresholds     |
//! | `8` | Memory       | Brier score, CUSUM, lessons             |
//! | `9` | Config       | Read-only config display                |
//! | `0` | Activity     | Full scrollable activity feed           |
//!
//! ## Globals
//!
//! - `q` / `Esc` / `Ctrl+C` — Quit
//! - `Tab` / `Shift+Tab` — Next/prev tab
//! - `/` — Search (in supported tabs)
//! - `r` — Force refresh
//! - `?` / `F1` — Help overlay

mod keyboard;
pub mod state;
pub mod tabs;
mod widgets;

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

use crate::core::config::AppConfig;
use crate::core::shared::SharedEngineData;

use self::keyboard::{dispatch_key, KeyAction};
use self::state::TuiState;
use self::widgets::*;
use self::tabs::draw_tab;

// ── Tab label constants ──

const TAB_NAMES: &[(&str, Color)] = &[
    (" Act ", NEON_CYAN),    // 0
    (" Ovrv ", NEON_CYAN),   // 1
    (" Port ", NEON_GREEN),  // 2
    (" Pos ", NEON_CYAN),    // 3
    (" Trad ", NEON_MAGENTA),// 4
    (" Mkt ", NEON_YELLOW),  // 5
    (" AI ", NEON_YELLOW),   // 6
    (" Risk ", NEON_RED),    // 7
    (" Mem ", NEON_MAGENTA), // 8
    (" Cfg ", ACCENT_BLUE),  // 9
];

// ── TuiApp ──────────────────────────────────────────────────────

pub struct TuiApp {
    shared: SharedEngineData,
    state: TuiState,
    last_tick: Instant,
    tick_rate: Duration,
}

impl TuiApp {
    pub fn new(shared: SharedEngineData, config: &AppConfig) -> Self {
        Self {
            shared,
            state: TuiState::new(config),
            last_tick: Instant::now(),
            tick_rate: Duration::from_secs(1),
        }
    }

    // ── Main loop ──────────────────────────────────────────────

    pub async fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            // Refresh snapshot from shared state
            self.state.refresh_from(&self.shared);

            // Render
            terminal.draw(|f| self.draw(f))?;

            // Handle input (non-blocking)
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if let Some(action) = dispatch_key(key) {
                            if self.handle_action(action) {
                                break; // Quit
                            }
                        }
                    }
                }
            }

            if self.last_tick.elapsed() >= self.tick_rate {
                self.last_tick = Instant::now();
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    // ── Action dispatch ────────────────────────────────────────

    /// Handle a key action. Returns `true` if the app should quit.
    fn handle_action(&mut self, action: KeyAction) -> bool {
        match action {
            KeyAction::Quit => return true,

            KeyAction::SwitchTab(n) => {
                self.state.active_tab = n;
                // Reset scroll offset when switching tabs
                if let Some(ts) = self.state.tab_states.get_mut(n) {
                    ts.scroll_offset = 0;
                    ts.selected_row = None;
                }
            }
            KeyAction::NextTab => {
                self.state.active_tab = (self.state.active_tab + 1) % 10;
            }
            KeyAction::PrevTab => {
                self.state.active_tab = if self.state.active_tab == 0 {
                    9
                } else {
                    self.state.active_tab - 1
                };
            }

            KeyAction::ScrollUp => {
                if let Some(ts) = self.state.tab_states.get_mut(self.state.active_tab) {
                    ts.scroll_offset = ts.scroll_offset.saturating_sub(1);
                }
            }
            KeyAction::ScrollDown => {
                if let Some(ts) = self.state.tab_states.get_mut(self.state.active_tab) {
                    ts.scroll_offset = ts.scroll_offset.saturating_add(1);
                }
            }
            KeyAction::PageUp => {
                if let Some(ts) = self.state.tab_states.get_mut(self.state.active_tab) {
                    ts.scroll_offset = ts.scroll_offset.saturating_sub(10);
                }
            }
            KeyAction::PageDown => {
                if let Some(ts) = self.state.tab_states.get_mut(self.state.active_tab) {
                    ts.scroll_offset = ts.scroll_offset.saturating_add(10);
                }
            }

            KeyAction::Help => {
                self.state.show_help = !self.state.show_help;
            }
            KeyAction::Refresh => {
                self.state.refresh_from(&self.shared);
            }

            _ => {} // Noop for unmapped actions
        }
        false
    }

    // ── Render ─────────────────────────────────────────────────

    fn draw(&self, f: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),  // Banner
                Constraint::Length(3),  // Stats bar
                Constraint::Length(3),  // Tab bar
                Constraint::Min(6),     // Tab content
                Constraint::Length(3),  // Footer
            ])
            .split(f.area());

        self.draw_banner(f, root[0]);
        self.draw_stats_bar(f, root[1]);
        self.draw_tab_bar(f, root[2]);
        draw_tab(f, root[3], &self.state);
        self.draw_footer(f, root[4]);
    }

    // ── Banner ─────────────────────────────────────────────────

    fn draw_banner(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let uptime = self.state.start_time.elapsed();
        let hours = uptime.as_secs() / 3600;
        let mins = (uptime.as_secs() % 3600) / 60;
        let secs = uptime.as_secs() % 60;

        let banner_lines = vec![
            Line::from(vec![Span::styled(
                "  ███████╗ █████╗ ██╗   ██╗ █████╗ ███╗   ██╗████████╗",
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  ██╔════╝██╔══██╗██║   ██║██╔══██╗████╗  ██║╚══██╔══╝",
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "  ███████╗███████║██║   ██║███████║██╔██╗ ██║   ██║   ",
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "  ╚════██║██╔══██║╚██╗ ██╔╝██╔══██║██║╚██╗██║   ██║   ",
                    Style::default().fg(ACCENT_BLUE).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  TRADING ENGINE", Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("  v{}  │  UP {:02}:{:02}:{:02}", env!("CARGO_PKG_VERSION"), hours, mins, secs),
                    Style::default().fg(DIM_CYAN),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  ███████║██║  ██║ ╚████╔╝ ██║  ██║██║ ╚████║   ██║   ",
                    Style::default().fg(ACCENT_BLUE),
                ),
                Span::styled(
                    "  ─── DISCIPLINED BY DESIGN ─── TRANSPARENT BY DEFAULT ───",
                    Style::default().fg(DIM_CYAN),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  ╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ",
                    Style::default().fg(DIM_CYAN),
                ),
                Span::styled("  PATIENT BY NECESSITY", Style::default().fg(NEON_CYAN).add_modifier(Modifier::ITALIC)),
            ]),
        ];

        f.render_widget(
            Paragraph::new(banner_lines)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(NEON_CYAN))),
            area,
        );
    }

    // ── Stats bar ──────────────────────────────────────────────

    fn draw_stats_bar(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let a = &self.state.snapshot.account;
        let pos_count = self.state.snapshot.positions.len();
        let trade_count = self.state.snapshot.closed_trades.len();
        let dd_pct = a.drawdown_pct * 100.0;
        let session = crate::core::session::current_session();

        let pnl_color_s = if a.daily_pnl >= 0.0 { NEON_GREEN } else { NEON_RED };
        let pnl_arrow = if a.daily_pnl >= 0.0 { "▲" } else { "▼" };
        let dd_color_s = if dd_pct > 10.0 { NEON_RED } else if dd_pct > 5.0 { NEON_YELLOW } else { NEON_GREEN };

        let stats = Line::from(vec![
            Span::styled("  BAL ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("${:.2}  ", a.balance), Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD)),
            Span::styled("│  EQUITY ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("${:.2}  ", a.equity), Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("│  P&L ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{}${:.2}  ", pnl_arrow, a.daily_pnl.abs()), Style::default().fg(pnl_color_s).add_modifier(Modifier::BOLD)),
            Span::styled("│  DD ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{:.1}%  ", dd_pct), Style::default().fg(dd_color_s).add_modifier(Modifier::BOLD)),
            Span::styled("│  POS ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{}  ", pos_count), Style::default().fg(NEON_CYAN)),
            Span::styled("│  TRADES ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{}  ", trade_count), Style::default().fg(NEON_CYAN)),
            Span::styled("│  SESSION ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{}  ", session.name()), Style::default().fg(NEON_MAGENTA)),
            Span::styled("│  ", Style::default().fg(DIM_CYAN)),
            Span::styled(chrono::Local::now().format("%H:%M:%S").to_string(), Style::default().fg(DIM_CYAN)),
        ]);

        f.render_widget(
            Paragraph::new(stats)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM_CYAN))),
            area,
        );
    }

    // ── Tab bar ────────────────────────────────────────────────

    fn draw_tab_bar(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mut spans = Vec::with_capacity(20);
        spans.push(Span::styled("  ", Style::default()));

        for (i, &(name, color)) in TAB_NAMES.iter().enumerate() {
            let is_active = i == self.state.active_tab;
            let style = if is_active {
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            };
            spans.push(Span::styled(format!("[{}]{}", i, name), style));
            if i < 9 {
                spans.push(Span::styled("  ", Style::default()));
            }
        }

        let help_hint = if self.state.show_help {
            "  [?] Hide help".to_string()
        } else {
            "  [?] Help".to_string()
        };
        spans.push(Span::styled(help_hint, Style::default().fg(DIM_CYAN)));

        f.render_widget(
            Paragraph::new(Line::from(spans))
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(NEON_CYAN))),
            area,
        );
    }

    // ── Footer ─────────────────────────────────────────────────

    fn draw_footer(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let s = &self.state.snapshot;
        let mode_color = if s.mode_label == "LIVE" { NEON_RED } else { NEON_GREEN };

        let footer = Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[Q]uit", Style::default().fg(NEON_RED).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled("SAVANT TRADING ENGINE", Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled(&s.backend_name, Style::default().fg(NEON_YELLOW)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled(&s.mode_label, Style::default().fg(mode_color)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("${:.0} Budget", s.starting_balance),
                Style::default().fg(NEON_MAGENTA),
            ),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled(&s.model_name, Style::default().fg(ACCENT_BLUE)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled("24/7", Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD)),
        ]);

        if self.state.show_help {
            let help = vec![
                Line::from(Span::styled(
                    "  [0-9] Tabs  [Tab/Shift+Tab] Next/Prev  [↑↓] Scroll  [PgUp/PgDn] Page  [Home/End] Top/Bottom",
                    Style::default().fg(NEON_YELLOW),
                )),
                Line::from(Span::styled(
                    "  [/] Search  [n/N] Next/Prev match  [Enter] Detail  [r] Refresh  [q/Esc] Quit",
                    Style::default().fg(NEON_YELLOW),
                )),
            ];

            f.render_widget(
                Paragraph::new(help)
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(NEON_YELLOW))),
                area,
            );
            return;
        }

        f.render_widget(
            Paragraph::new(footer)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(DIM_CYAN))),
            area,
        );
    }
}
