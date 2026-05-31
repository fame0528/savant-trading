//! SAVANT TUI — Cyberpunk Trading Terminal
//!
//! Black background. Cyan neon. Real-time everything.
//! "Nothing behind a blackbox."

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

use crate::core::shared::{
    ActivityEntry, ActivityLevel, DecisionRecord, MemorySnapshot, SharedEngineData,
};
use crate::core::types::AccountState;
use crate::insight::aggregator::MarketContext;

// ── Neon Palette ──────────────────────────────────────────────
const NEON_CYAN: Color = Color::Cyan;
const NEON_GREEN: Color = Color::Green;
const NEON_RED: Color = Color::Red;
const NEON_YELLOW: Color = Color::Yellow;
const NEON_MAGENTA: Color = Color::Magenta;
const DIM_CYAN: Color = Color::DarkGray;
const ACCENT_BLUE: Color = Color::Blue;

// ── TUI State ─────────────────────────────────────────────────

struct TuiSnapshot {
    account: AccountState,
    positions: Vec<crate::core::types::Position>,
    decisions: Vec<DecisionRecord>,
    insight: MarketContext,
    activity: Vec<ActivityEntry>,
    closed_trades: Vec<crate::core::types::TradeRecord>,
    memory: MemorySnapshot,
}

pub struct TuiApp {
    shared: SharedEngineData,
    last_tick: Instant,
    tick_rate: Duration,
    snapshot: TuiSnapshot,
    /// Rolling equity history for sparkline (last 60 samples)
    equity_history: Vec<u64>,
    /// Uptime counter
    start_time: Instant,
}

impl TuiApp {
    pub fn new(shared: SharedEngineData) -> Self {
        Self {
            shared,
            last_tick: Instant::now(),
            tick_rate: Duration::from_secs(1),
            snapshot: TuiSnapshot {
                account: AccountState::new(0.0),
                positions: Vec::new(),
                decisions: Vec::new(),
                insight: MarketContext::default(),
                activity: Vec::new(),
                closed_trades: Vec::new(),
                memory: MemorySnapshot {
                    brier_score: None,
                    brier_label: "No data".to_string(),
                    confidence_cap: "LOW".to_string(),
                    total_trades: 0,
                    cusum_status: Vec::new(),
                    replay_lesson_count: 0,
                },
            },
            equity_history: Vec::with_capacity(60),
            start_time: Instant::now(),
        }
    }

    async fn refresh_snapshot(&mut self) {
        self.snapshot.account = self.shared.account.read().await.clone();
        self.snapshot.positions = self.shared.positions.read().await.clone();
        self.snapshot.decisions = self.shared.decisions.read().await.clone();
        self.snapshot.insight = self.shared.insight.read().await.clone();
        self.snapshot.activity = self.shared.activity_log.read().await.clone();
        self.snapshot.closed_trades = self.shared.closed_trades.read().await.clone();
        self.snapshot.memory = self.shared.memory_snapshot.read().await.clone();

        // Track equity for sparkline (scale to u64 for widget)
        let equity = self.snapshot.account.equity;
        if equity > 0.0 {
            self.equity_history.push((equity * 100.0) as u64);
            if self.equity_history.len() > 60 {
                self.equity_history.remove(0);
            }
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            self.refresh_snapshot().await;
            terminal.draw(|f| self.draw(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                            _ => {}
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

    // ── Main Layout ───────────────────────────────────────────

    fn draw(&self, f: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),  // ASCII art banner
                Constraint::Length(3),  // Stats bar
                Constraint::Length(14), // Positions + Insight + Regime
                Constraint::Length(7),  // Equity sparkline + Performance
                Constraint::Length(4),  // Memory panel
                Constraint::Min(8),     // Activity feed
                Constraint::Length(3),  // Footer
            ])
            .split(f.area());

        self.draw_banner(f, root[0]);
        self.draw_stats_bar(f, root[1]);
        self.draw_panels(f, root[2]);
        self.draw_performance(f, root[3]);
        self.draw_memory(f, root[4]);
        self.draw_activity(f, root[5]);
        self.draw_footer(f, root[6]);
    }

    // ── ASCII Art Banner ──────────────────────────────────────

    fn draw_banner(&self, f: &mut Frame, area: Rect) {
        let uptime = self.start_time.elapsed();
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
                    Style::default()
                        .fg(ACCENT_BLUE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "  TRADING ENGINE",
                    Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  v0.4.2  │  UP {:02}:{:02}:{:02}", hours, mins, secs),
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
                    "  ╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝   ",
                    Style::default().fg(DIM_CYAN),
                ),
                Span::styled(
                    "  PATIENT BY NECESSITY",
                    Style::default()
                        .fg(NEON_CYAN)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]),
        ];

        f.render_widget(
            Paragraph::new(banner_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(NEON_CYAN)),
            ),
            area,
        );
    }

    // ── Stats Bar ─────────────────────────────────────────────

    fn draw_stats_bar(&self, f: &mut Frame, area: Rect) {
        let a = &self.snapshot.account;
        let pos_count = self.snapshot.positions.len();
        let trade_count = self.snapshot.closed_trades.len();

        let pnl_color = if a.daily_pnl >= 0.0 {
            NEON_GREEN
        } else {
            NEON_RED
        };
        let pnl_arrow = if a.daily_pnl >= 0.0 { "▲" } else { "▼" };

        let dd_color = if a.drawdown_pct > 0.10 {
            NEON_RED
        } else if a.drawdown_pct > 0.05 {
            NEON_YELLOW
        } else {
            NEON_GREEN
        };

        let session = crate::core::session::current_session();

        let stats = Line::from(vec![
            Span::styled("  BAL ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("${:.2}  ", a.balance),
                Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled("│  EQUITY ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("${:.2}  ", a.equity),
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled("│  P&L ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{}${:.2}  ", pnl_arrow, a.daily_pnl.abs()),
                Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("│  DD ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{:.1}%  ", a.drawdown_pct * 100.0),
                Style::default().fg(dd_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled("│  POS ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{}  ", pos_count), Style::default().fg(NEON_CYAN)),
            Span::styled("│  TRADES ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{}  ", trade_count), Style::default().fg(NEON_CYAN)),
            Span::styled("│  SESSION ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{}  ", session.name()),
                Style::default().fg(NEON_MAGENTA),
            ),
            Span::styled("│  ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                chrono::Local::now().format("%H:%M:%S").to_string(),
                Style::default().fg(DIM_CYAN),
            ),
        ]);

        f.render_widget(
            Paragraph::new(stats).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DIM_CYAN)),
            ),
            area,
        );
    }

    // ── Three-Panel Row ───────────────────────────────────────

    fn draw_panels(&self, f: &mut Frame, area: Rect) {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Positions
                Constraint::Percentage(35), // Insight / Regime
                Constraint::Percentage(25), // Circuit Breaker / Risk
            ])
            .split(area);

        self.draw_positions(f, h_chunks[0]);
        self.draw_insight(f, h_chunks[1]);
        self.draw_risk_panel(f, h_chunks[2]);
    }

    // ── Positions Table ───────────────────────────────────────

    fn draw_positions(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" ◆ OPEN POSITIONS ")
            .title_style(Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_CYAN));

        if self.snapshot.positions.is_empty() {
            let empty_msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ◇ ", Style::default().fg(DIM_CYAN)),
                    Span::styled(
                        "No open positions",
                        Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ◇ ", Style::default().fg(DIM_CYAN)),
                    Span::styled(
                        "Waiting for setup...",
                        Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
                    ),
                ]),
            ])
            .block(block);
            f.render_widget(empty_msg, area);
            return;
        }

        let header = Row::new(vec![
            "PAIR", "SIDE", "ENTRY", "CURRENT", "P&L", "STOP", "TP1",
        ])
        .style(Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD))
        .bottom_margin(0);

        let rows: Vec<Row> = self
            .snapshot
            .positions
            .iter()
            .map(|p| {
                let pnl_color = if p.unrealized_pnl >= 0.0 {
                    NEON_GREEN
                } else {
                    NEON_RED
                };
                let side_color = if p.side == crate::core::types::Side::Long {
                    NEON_GREEN
                } else {
                    NEON_RED
                };
                Row::new(vec![
                    Cell::from(p.pair.clone()).style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Cell::from(format!("{}", p.side))
                        .style(Style::default().fg(side_color).add_modifier(Modifier::BOLD)),
                    Cell::from(format!("${:.2}", p.entry_price))
                        .style(Style::default().fg(Color::White)),
                    Cell::from(format!("${:.2}", p.current_price))
                        .style(Style::default().fg(NEON_CYAN)),
                    Cell::from(format!("${:.2}", p.unrealized_pnl))
                        .style(Style::default().fg(pnl_color).add_modifier(Modifier::BOLD)),
                    Cell::from(format!("${:.2}", p.stop_loss)).style(Style::default().fg(NEON_RED)),
                    Cell::from(format!("${:.2}", p.take_profit_1))
                        .style(Style::default().fg(NEON_GREEN)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(9),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(block);

        f.render_widget(table, area);
    }

    // ── Insight / Regime Panel ────────────────────────────────

    fn draw_insight(&self, f: &mut Frame, area: Rect) {
        let insight = &self.snapshot.insight;
        let mut lines: Vec<Line> = vec![];

        // Regime header
        let regime = insight
            .sentiment
            .fear_greed_label
            .as_deref()
            .unwrap_or("UNKNOWN");
        let regime_color = match regime {
            "Extreme Fear" | "Fear" => NEON_RED,
            "Extreme Greed" | "Greed" => NEON_GREEN,
            _ => NEON_YELLOW,
        };
        lines.push(Line::from(vec![
            Span::styled("  ◆ ", Style::default().fg(NEON_CYAN)),
            Span::styled(
                "REGIME",
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    Market: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                regime,
                Style::default()
                    .fg(regime_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Fear & Greed
        if let Some(fg) = insight.sentiment.fear_greed_index {
            let label = insight.sentiment.fear_greed_label.as_deref().unwrap_or("?");
            let fg_color = if fg < 25 {
                NEON_RED
            } else if fg < 50 {
                NEON_YELLOW
            } else if fg < 75 {
                NEON_GREEN
            } else {
                NEON_CYAN
            };
            lines.push(Line::from(vec![
                Span::styled("    F&G:    ", Style::default().fg(DIM_CYAN)),
                Span::styled(
                    format!("{} {}", fg, label),
                    Style::default().fg(fg_color).add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Funding
        if let Some(fr) = insight.funding.funding_rate {
            let fr_color = if fr > 0.01 {
                NEON_RED
            } else if fr < -0.01 {
                NEON_GREEN
            } else {
                NEON_YELLOW
            };
            let fr_label = if fr > 0.05 {
                " ⚠ OVERLEVERAGED"
            } else if fr < -0.03 {
                " ⚠ SHORT SQUEEZE RISK"
            } else {
                ""
            };
            lines.push(Line::from(vec![
                Span::styled("    Funding: ", Style::default().fg(DIM_CYAN)),
                Span::styled(format!("{:.4}%", fr * 100.0), Style::default().fg(fr_color)),
                Span::styled(fr_label, Style::default().fg(NEON_YELLOW)),
            ]));
        }

        // News count
        if !insight.rss_items.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("    News:   ", Style::default().fg(DIM_CYAN)),
                Span::styled(
                    format!("{} items", insight.rss_items.len()),
                    Style::default().fg(NEON_CYAN),
                ),
            ]));
        }

        // Session
        let session = crate::core::session::current_session();
        lines.push(Line::from(vec![
            Span::styled("    Session: ", Style::default().fg(DIM_CYAN)),
            Span::styled(session.name(), Style::default().fg(NEON_MAGENTA)),
            Span::styled(
                format!(" ({:.1}x)", session.position_size_multiplier()),
                Style::default().fg(DIM_CYAN),
            ),
        ]));

        let block = Block::default()
            .title(" ◆ MARKET INSIGHT ")
            .title_style(
                Style::default()
                    .fg(NEON_YELLOW)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_YELLOW));

        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    // ── Risk / Circuit Breaker Panel ──────────────────────────

    fn draw_risk_panel(&self, f: &mut Frame, area: Rect) {
        let a = &self.snapshot.account;
        let mut lines: Vec<Line> = vec![];

        // Circuit breaker status
        let dd_pct = a.drawdown_pct * 100.0;
        let (cb_label, cb_color) = if dd_pct >= 10.0 {
            ("⛔ BLOCKED", NEON_RED)
        } else if dd_pct >= 5.0 {
            ("⚠ REVIEW", NEON_YELLOW)
        } else if dd_pct >= 2.0 {
            ("◉ CAUTION", NEON_YELLOW)
        } else {
            ("✓ CLEAR", NEON_GREEN)
        };

        lines.push(Line::from(vec![
            Span::styled("  ◆ ", Style::default().fg(NEON_CYAN)),
            Span::styled(
                "RISK STATUS",
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    CB:  ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                cb_label,
                Style::default().fg(cb_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        // Drawdown gauge
        lines.push(Line::from(vec![
            Span::styled("    DD:  ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{:.1}%", dd_pct), Style::default().fg(cb_color)),
            Span::styled(" / 10.0% max", Style::default().fg(DIM_CYAN)),
        ]));

        // Drawdown bar (ASCII)
        let dd_bar_len = 20;
        let dd_filled = ((dd_pct / 10.0) * dd_bar_len as f64).min(dd_bar_len as f64) as usize;
        let dd_empty = dd_bar_len - dd_filled;
        let dd_bar_color = if dd_pct >= 10.0 {
            NEON_RED
        } else if dd_pct >= 5.0 {
            NEON_YELLOW
        } else {
            NEON_GREEN
        };
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default().fg(DIM_CYAN)),
            Span::styled("█".repeat(dd_filled), Style::default().fg(dd_bar_color)),
            Span::styled("░".repeat(dd_empty), Style::default().fg(DIM_CYAN)),
        ]));

        // Thresholds
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ◆ ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                "THRESHOLDS",
                Style::default().fg(DIM_CYAN).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    2% ", Style::default().fg(NEON_YELLOW)),
            Span::styled("→ Cut size 50%", Style::default().fg(DIM_CYAN)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    3% ", Style::default().fg(NEON_YELLOW)),
            Span::styled("→ Close all", Style::default().fg(DIM_CYAN)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    5% ", Style::default().fg(NEON_RED)),
            Span::styled("→ Stop 48hrs", Style::default().fg(DIM_CYAN)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   10% ", Style::default().fg(NEON_RED)),
            Span::styled("→ Full stop", Style::default().fg(DIM_CYAN)),
        ]));

        let block = Block::default()
            .title(" ◆ RISK ENGINE ")
            .title_style(Style::default().fg(NEON_RED).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_RED));

        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    // ── Performance Row (Sparkline + Metrics) ─────────────────

    fn draw_performance(&self, f: &mut Frame, area: Rect) {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        self.draw_equity_sparkline(f, h_chunks[0]);
        self.draw_metrics(f, h_chunks[1]);
    }

    fn draw_equity_sparkline(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" ◆ EQUITY CURVE ")
            .title_style(Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_GREEN));

        if self.equity_history.len() < 2 {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "  Collecting data points...",
                        Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
                    )]),
                ])
                .block(block),
                area,
            );
            return;
        }

        let inner = block.inner(area);
        let sparkline = Sparkline::default()
            .data(&self.equity_history)
            .style(Style::default().fg(NEON_CYAN));

        f.render_widget(block, area);
        f.render_widget(sparkline, inner);
    }

    fn draw_metrics(&self, f: &mut Frame, area: Rect) {
        let trades = &self.snapshot.closed_trades;
        let mut lines: Vec<Line> = vec![];

        lines.push(Line::from(vec![
            Span::styled("  ◆ ", Style::default().fg(NEON_CYAN)),
            Span::styled(
                "PERFORMANCE",
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            ),
        ]));

        if trades.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "    No closed trades yet",
                Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
            )]));
        } else {
            let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
            let losses = trades.iter().filter(|t| t.pnl <= 0.0).count();
            let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
            let win_rate = wins as f64 / trades.len() as f64;
            let avg_win = if wins > 0 {
                trades
                    .iter()
                    .filter(|t| t.pnl > 0.0)
                    .map(|t| t.pnl)
                    .sum::<f64>()
                    / wins as f64
            } else {
                0.0
            };
            let avg_loss = if losses > 0 {
                trades
                    .iter()
                    .filter(|t| t.pnl <= 0.0)
                    .map(|t| t.pnl.abs())
                    .sum::<f64>()
                    / losses as f64
            } else {
                0.0
            };
            let profit_factor = if avg_loss > 0.0 {
                (avg_win * wins as f64) / (avg_loss * losses as f64)
            } else if avg_win > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };

            let pnl_color = if total_pnl >= 0.0 {
                NEON_GREEN
            } else {
                NEON_RED
            };
            let wr_color = if win_rate >= 0.5 {
                NEON_GREEN
            } else {
                NEON_YELLOW
            };

            lines.push(Line::from(vec![
                Span::styled("    Trades: ", Style::default().fg(DIM_CYAN)),
                Span::styled(
                    format!("{}", trades.len()),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  W/L: ", Style::default().fg(DIM_CYAN)),
                Span::styled(format!("{}", wins), Style::default().fg(NEON_GREEN)),
                Span::styled("/", Style::default().fg(DIM_CYAN)),
                Span::styled(format!("{}", losses), Style::default().fg(NEON_RED)),
            ]));

            lines.push(Line::from(vec![
                Span::styled("    WR:     ", Style::default().fg(DIM_CYAN)),
                Span::styled(
                    format!("{:.1}%", win_rate * 100.0),
                    Style::default().fg(wr_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  PF: ", Style::default().fg(DIM_CYAN)),
                Span::styled(
                    format!("{:.2}", profit_factor),
                    Style::default().fg(NEON_CYAN),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::styled("    P&L:    ", Style::default().fg(DIM_CYAN)),
                Span::styled(
                    format!("${:.2}", total_pnl),
                    Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  Avg W: ", Style::default().fg(DIM_CYAN)),
                Span::styled(format!("${:.2}", avg_win), Style::default().fg(NEON_GREEN)),
                Span::styled("  Avg L: ", Style::default().fg(DIM_CYAN)),
                Span::styled(format!("-${:.2}", avg_loss), Style::default().fg(NEON_RED)),
            ]));
        }

        let block = Block::default()
            .title(" ◆ METRICS ")
            .title_style(
                Style::default()
                    .fg(NEON_MAGENTA)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_MAGENTA));

        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    // ── Memory Panel ──────────────────────────────────────────

    fn draw_memory(&self, f: &mut Frame, area: Rect) {
        let mem = &self.snapshot.memory;

        let brier_text = match mem.brier_score {
            Some(score) => format!("{:.2} ({})", score, mem.brier_label),
            None => "No data".to_string(),
        };

        let cusum_text = if mem.cusum_status.is_empty() {
            "No data".to_string()
        } else {
            mem.cusum_status
                .iter()
                .take(5)
                .map(|(pair, status)| format!("{} {}", pair, status))
                .collect::<Vec<_>>()
                .join(" | ")
        };

        let line = Line::from(vec![
            Span::styled("  Calibration: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                &brier_text,
                Style::default().fg(if mem.brier_score.is_some_and(|s| s <= 0.25) {
                    NEON_GREEN
                } else if mem.brier_score.is_some_and(|s| s <= 0.35) {
                    NEON_YELLOW
                } else {
                    NEON_RED
                }),
            ),
            Span::styled("  │  Cap: ", Style::default().fg(DIM_CYAN)),
            Span::styled(&mem.confidence_cap, Style::default().fg(NEON_MAGENTA)),
            Span::styled(
                format!(" ({} trades)", mem.total_trades),
                Style::default().fg(DIM_CYAN),
            ),
            Span::styled("  │  CUSUM: ", Style::default().fg(DIM_CYAN)),
            Span::styled(cusum_text, Style::default().fg(NEON_CYAN)),
            Span::styled(
                format!("  │  Lessons: {}", mem.replay_lesson_count),
                Style::default().fg(DIM_CYAN),
            ),
        ]);

        let block = Block::default()
            .title(" ◆ MEMORY ")
            .title_style(
                Style::default()
                    .fg(NEON_MAGENTA)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM_CYAN));

        f.render_widget(Paragraph::new(line).block(block), area);
    }

    // ── Activity Feed ─────────────────────────────────────────

    fn draw_activity(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" ◆ LIVE ACTIVITY ")
            .title_style(
                Style::default()
                    .fg(NEON_MAGENTA)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_MAGENTA));

        if self.snapshot.activity.is_empty() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "  ◇ Waiting for engine activity...",
                        Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
                    )]),
                ])
                .block(block),
                area,
            );
            return;
        }

        let lines: Vec<Line> = self
            .snapshot
            .activity
            .iter()
            .rev()
            .take(30)
            .map(|entry| {
                let (icon, color) = match entry.level {
                    ActivityLevel::Info => ("INFO", DIM_CYAN),
                    ActivityLevel::Thinking => ("THINK", NEON_CYAN),
                    ActivityLevel::Decision => ("DECIDE", NEON_YELLOW),
                    ActivityLevel::Trade => ("TRADE", NEON_GREEN),
                    ActivityLevel::Warning => ("WARN", NEON_YELLOW),
                    ActivityLevel::Error => ("ERROR", NEON_RED),
                };

                Line::from(vec![
                    Span::styled(
                        format!("{} ", entry.timestamp),
                        Style::default().fg(DIM_CYAN),
                    ),
                    Span::styled(
                        format!("{:<7}", icon),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<12}", entry.pair),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw(&entry.message),
                ])
            })
            .collect();

        f.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    // ── Footer ────────────────────────────────────────────────

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                " [Q]uit",
                Style::default().fg(NEON_RED).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                "SAVANT TRADING ENGINE",
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled("Kraken Exchange", Style::default().fg(NEON_YELLOW)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled("Paper Trading", Style::default().fg(NEON_GREEN)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled("$50 Budget", Style::default().fg(NEON_MAGENTA)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled("MiMo v2.5 Pro", Style::default().fg(ACCENT_BLUE)),
            Span::styled("  │  ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                "24/7",
                Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD),
            ),
        ]);

        f.render_widget(
            Paragraph::new(footer).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(DIM_CYAN)),
            ),
            area,
        );
    }
}
