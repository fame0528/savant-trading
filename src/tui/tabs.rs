//! Tab implementations for the multi-tab TUI.
//!
//! Each tab is a free function: `draw_tab_N(f, area, state)`.
//! The main loop in [`super::TuiApp`] dispatches to the active tab.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Sparkline, Table, Wrap},
    Frame,
};

use crate::core::types::Side;

use super::state::TuiState;
use super::widgets::*;

// ---------------------------------------------------------------------------
// Tab rendering — called by TuiApp::draw()
// ---------------------------------------------------------------------------

pub fn draw_tab(f: &mut Frame, area: Rect, state: &TuiState) {
    match state.active_tab {
        0 => draw_tab_activity(f, area, state),
        1 => draw_tab_overview(f, area, state),
        2 => draw_tab_portfolio(f, area, state),
        3 => draw_tab_positions(f, area, state),
        4 => draw_tab_trades(f, area, state),
        5 => draw_tab_insight(f, area, state),
        6 => draw_tab_decisions(f, area, state),
        7 => draw_tab_risk(f, area, state),
        8 => draw_tab_memory(f, area, state),
        9 => draw_tab_config(f, area, state),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tab 1 — Overview
// ---------------------------------------------------------------------------

fn draw_tab_overview(f: &mut Frame, area: Rect, state: &TuiState) {
    let s = &state.snapshot;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Portfolio summary
            Constraint::Length(6), // Equity sparkline + stats
            Constraint::Length(6), // Open positions summary
            Constraint::Min(5),    // Recent trades
        ])
        .split(area);

    // ── Portfolio summary ──
    let a = &s.account;
    let dd_pct = a.drawdown_pct * 100.0;
    let (dd_color, dd_label) = {
        let blocked = s.max_drawdown.min(s.max_daily_loss);
        let review = s.max_drawdown * 0.50;
        if dd_pct >= blocked {
            (NEON_RED, "⚠ BLOCKED")
        } else if dd_pct >= review {
            (NEON_YELLOW, "⚠ REVIEW")
        } else {
            (NEON_GREEN, "✓ OK")
        }
    };

    let summary = vec![
        stat_row(vec![
            ("BAL ", format!("${:.2}", a.balance), NEON_GREEN),
            ("EQUITY ", format!("${:.2}", a.equity), NEON_CYAN),
            (
                "P&L ",
                format!(
                    "{}$ {:.2}",
                    if a.daily_pnl >= 0.0 { "▲" } else { "▼" },
                    a.daily_pnl.abs()
                ),
                pnl_color(a.daily_pnl),
            ),
            ("DD ", format!("{:.1}% ({})", dd_pct, dd_label), dd_color),
        ]),
        stat_row(vec![
            ("POS ", format!("{}", s.positions.len()), NEON_CYAN),
            ("TRADES ", format!("{}", s.closed_trades.len()), NEON_CYAN),
            ("MODEL ", s.model_name.clone(), ACCENT_BLUE),
            (
                "MODE ",
                s.mode_label.clone(),
                if s.mode_label == "LIVE" {
                    NEON_RED
                } else {
                    NEON_GREEN
                },
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(summary).block(titled_block("PORTFOLIO", NEON_CYAN)),
        chunks[0],
    );

    // ── Equity sparkline + stats ──
    let perf_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    let spark_block = titled_block("EQUITY CURVE", NEON_GREEN);
    if state.equity_history.len() >= 2 {
        let inner = spark_block.inner(perf_chunks[0]);
        f.render_widget(spark_block, perf_chunks[0]);
        f.render_widget(
            Sparkline::default()
                .data(state.equity_history.data())
                .style(Style::default().fg(NEON_CYAN)),
            inner,
        );
    } else {
        f.render_widget(
            Paragraph::new(vec![Line::from(vec![Span::styled(
                "  Collecting data...",
                Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
            )])])
            .block(spark_block),
            perf_chunks[0],
        );
    }

    // ── Quick metrics ──
    let trades = &s.closed_trades;
    let mut metrics = Vec::new();
    if trades.is_empty() {
        metrics.push(Line::from(vec![Span::styled(
            "    No closed trades yet",
            Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
        )]));
    } else {
        let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
        let losses = trades.len().saturating_sub(wins);
        let wr = if trades.is_empty() {
            0.0
        } else {
            wins as f64 / trades.len() as f64
        };
        metrics.push(stat_row(vec![
            ("Trades: ", format!("{}", trades.len()), TEXT_WHITE),
            ("W/L: ", format!("{}/{}", wins, losses), NEON_GREEN),
            (
                "WR: ",
                format!("{:.1}%", wr * 100.0),
                if wr >= 0.5 { NEON_GREEN } else { NEON_YELLOW },
            ),
        ]));
    }
    f.render_widget(
        Paragraph::new(metrics).block(titled_block("PERFORMANCE", NEON_MAGENTA)),
        perf_chunks[1],
    );

    // ── Open positions summary ──
    if s.positions.is_empty() {
        f.render_widget(
            empty_message("No open positions — waiting for setup..."),
            chunks[2],
        );
    } else {
        let pos_lines: Vec<Line> = s
            .positions
            .iter()
            .map(|p| {
                let p_color = pnl_color(p.unrealized_pnl);
                Line::from(vec![
                    Span::styled(
                        format!("  {} ", p.pair),
                        Style::default().fg(TEXT_WHITE).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}", p.side),
                        Style::default()
                            .fg(side_color(p.side))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {:.2}", p.entry_price),
                        Style::default().fg(TEXT_WHITE),
                    ),
                    Span::styled(
                        format!(" → {:.2}", p.current_price),
                        Style::default().fg(NEON_CYAN),
                    ),
                    Span::styled(
                        format!("  P&L: ${:.2}", p.unrealized_pnl),
                        Style::default().fg(p_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  SL: {:.2}", p.stop_loss),
                        Style::default().fg(NEON_RED),
                    ),
                ])
            })
            .collect();
        f.render_widget(
            Paragraph::new(pos_lines).block(titled_block("OPEN POSITIONS", NEON_CYAN)),
            chunks[2],
        );
    }

    // ── Recent trades ──
    let recent: Vec<Line> = s
        .closed_trades
        .iter()
        .rev()
        .take(10)
        .map(|t| {
            let t_color = pnl_color(t.pnl);
            Line::from(vec![
                Span::styled(
                    format!("  {}", t.pair),
                    Style::default().fg(TEXT_WHITE).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}", t.side),
                    Style::default().fg(side_color(t.side)),
                ),
                Span::styled(
                    format!(" PnL: ${:.2}", t.pnl),
                    Style::default().fg(t_color).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect();
    f.render_widget(
        Paragraph::new(recent).block(titled_block("RECENT TRADES", NEON_MAGENTA)),
        chunks[3],
    );
}

// ---------------------------------------------------------------------------
// Tab 2 — Portfolio  (equity curve, balance history, daily P&L)
// ---------------------------------------------------------------------------

fn draw_tab_portfolio(f: &mut Frame, area: Rect, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Min(4),
        ])
        .split(area);

    // Equity sparkline (bigger)
    let eq_block = titled_block("EQUITY CURVE (last 120)", NEON_GREEN);
    if state.equity_history.len() >= 2 {
        let inner = eq_block.inner(chunks[0]);
        f.render_widget(eq_block, chunks[0]);
        f.render_widget(
            Sparkline::default()
                .data(state.equity_history.data())
                .style(Style::default().fg(NEON_CYAN)),
            inner,
        );
    } else {
        f.render_widget(
            Paragraph::new(vec![Line::from(vec![Span::styled(
                "  Collecting equity data...",
                Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
            )])])
            .block(eq_block),
            chunks[0],
        );
    }

    // Balance history sparkline
    let bal_block = titled_block("BALANCE HISTORY", NEON_YELLOW);
    if state.balance_history.len() >= 2 {
        let inner = bal_block.inner(chunks[1]);
        f.render_widget(bal_block, chunks[1]);
        f.render_widget(
            Sparkline::default()
                .data(state.balance_history.data())
                .style(Style::default().fg(NEON_YELLOW)),
            inner,
        );
    } else {
        f.render_widget(
            Paragraph::new(vec![Line::from(vec![Span::styled(
                "  Collecting balance data...",
                Style::default().fg(DIM_CYAN).add_modifier(Modifier::ITALIC),
            )])])
            .block(bal_block),
            chunks[1],
        );
    }

    // Drawdown chart
    let a = &state.snapshot.account;
    let dd_pct = a.drawdown_pct * 100.0;
    let dd_block = titled_block("DRAWDOWN", NEON_RED);
    let dd_lines = vec![
        Line::from(vec![
            Span::styled("  Current: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{:.1}%", dd_pct),
                Style::default()
                    .fg(traffic_color(dd_pct))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" / {:.0}% max", state.snapshot.max_drawdown),
                Style::default().fg(DIM_CYAN),
            ),
        ]),
        ascii_gauge(
            dd_pct / state.snapshot.max_drawdown.max(1.0) * 100.0,
            30,
            traffic_color(dd_pct),
        ),
    ];
    f.render_widget(Paragraph::new(dd_lines).block(dd_block), chunks[2]);

    // Session stats
    let session = crate::core::session::current_session();
    let sess_lines = vec![stat_row(vec![
        ("Session: ", session.name().to_string(), NEON_MAGENTA),
        (
            "Mult: ",
            format!("{:.1}x", session.position_size_multiplier()),
            NEON_CYAN,
        ),
        (
            "Uptime: ",
            format!("{:?}", state.start_time.elapsed().as_secs() / 60),
            DIM_CYAN,
        ),
    ])];
    f.render_widget(
        Paragraph::new(sess_lines).block(titled_block("SESSION", ACCENT_BLUE)),
        chunks[3],
    );
}

// ---------------------------------------------------------------------------
// Tab 3 — Positions  (full table with detail)
// ---------------------------------------------------------------------------

fn draw_tab_positions(f: &mut Frame, area: Rect, state: &TuiState) {
    let ts = &state.snapshot;
    if ts.positions.is_empty() {
        f.render_widget(empty_message("No open positions"), area);
        return;
    }

    let block = titled_block("OPEN POSITIONS", NEON_CYAN);
    let header = Row::new(vec![
        "PAIR", "SIDE", "ENTRY", "CURRENT", "P&L", "P&L%", "STOP", "TP1", "DURATION",
    ])
    .style(Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD))
    .bottom_margin(0);

    let rows: Vec<Row> = ts
        .positions
        .iter()
        .map(|p| {
            let duration = chrono::Utc::now() - p.opened_at;
            let hours = duration.num_hours();
            let mins = duration.num_minutes() % 60;
            let pnl_color_s = pnl_color(p.unrealized_pnl);
            let pnl_pct = if p.entry_price > 0.0 {
                (p.current_price - p.entry_price) / p.entry_price
                    * 100.0
                    * if p.side == Side::Long { 1.0 } else { -1.0 }
            } else {
                0.0
            };

            Row::new(vec![
                Cell::from(p.pair.clone())
                    .style(Style::default().fg(TEXT_WHITE).add_modifier(Modifier::BOLD)),
                Cell::from(format!("{}", p.side)).style(
                    Style::default()
                        .fg(side_color(p.side))
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("${:.2}", p.entry_price)).style(Style::default().fg(TEXT_WHITE)),
                Cell::from(format!("${:.2}", p.current_price))
                    .style(Style::default().fg(NEON_CYAN)),
                Cell::from(format!("${:.2}", p.unrealized_pnl)).style(
                    Style::default()
                        .fg(pnl_color_s)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{:.1}%", pnl_pct)).style(Style::default().fg(pnl_color_s)),
                Cell::from(format!("${:.2}", p.stop_loss)).style(Style::default().fg(NEON_RED)),
                Cell::from(format!("${:.2}", p.take_profit_1))
                    .style(Style::default().fg(NEON_GREEN)),
                Cell::from(format!("{}h{}m", hours, mins)).style(Style::default().fg(DIM_CYAN)),
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
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
}

// ---------------------------------------------------------------------------
// Tab 4 — Trades  (closed trade history)
// ---------------------------------------------------------------------------

fn draw_tab_trades(f: &mut Frame, area: Rect, state: &TuiState) {
    let trades = &state.snapshot.closed_trades;
    if trades.is_empty() {
        f.render_widget(empty_message("No closed trades yet"), area);
        return;
    }

    let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
    let losses = trades.len().saturating_sub(wins);
    let wr = wins as f64 / trades.len() as f64;
    let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Stats header
    let stats_line = stat_row(vec![
        ("Total: ", format!("{}", trades.len()), TEXT_WHITE),
        ("Wins: ", format!("{}", wins), NEON_GREEN),
        ("Losses: ", format!("{}", losses), NEON_RED),
        (
            "WR: ",
            format!("{:.1}%", wr * 100.0),
            if wr >= 0.5 { NEON_GREEN } else { NEON_YELLOW },
        ),
        (
            "Total PnL: ",
            format!("${:.2}", total_pnl),
            pnl_color(total_pnl),
        ),
    ]);
    f.render_widget(
        Paragraph::new(stats_line).block(titled_block("TRADE HISTORY", NEON_MAGENTA)),
        chunks[0],
    );

    // Trade table (last 50, newest first)
    let header = Row::new(vec![
        "PAIR", "SIDE", "ENTRY", "EXIT", "PnL", "PnL%", "FEE", "STRATEGY",
    ])
    .style(Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = trades
        .iter()
        .rev()
        .take(50)
        .map(|t| {
            Row::new(vec![
                Cell::from(t.pair.clone())
                    .style(Style::default().fg(TEXT_WHITE).add_modifier(Modifier::BOLD)),
                Cell::from(format!("{}", t.side)).style(Style::default().fg(side_color(t.side))),
                Cell::from(format!("${:.2}", t.entry_price)).style(Style::default().fg(TEXT_WHITE)),
                Cell::from(format!("${:.2}", t.exit_price)).style(Style::default().fg(NEON_CYAN)),
                Cell::from(format!("${:.2}", t.pnl)).style(
                    Style::default()
                        .fg(pnl_color(t.pnl))
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{:.1}%", t.pnl_pct))
                    .style(Style::default().fg(pnl_color(t.pnl))),
                Cell::from(format!("${:.4}", t.fees)).style(Style::default().fg(DIM_CYAN)),
                Cell::from(t.strategy_name.clone()).style(Style::default().fg(ACCENT_BLUE)),
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
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(14),
        ],
    )
    .header(header)
    .block(Block::default());

    f.render_widget(table, chunks[1]);
}

// ---------------------------------------------------------------------------
// Tab 5 — Market Insight
// ---------------------------------------------------------------------------

fn draw_tab_insight(f: &mut Frame, area: Rect, state: &TuiState) {
    let insight = &state.snapshot.insight;
    let mut lines: Vec<Line> = Vec::new();

    // Regime
    let regime = insight
        .sentiment
        .fear_greed_label
        .as_deref()
        .unwrap_or("UNKNOWN");
    let regime_color = fear_greed_color(insight.sentiment.fear_greed_index.unwrap_or(50));
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
        lines.push(Line::from(vec![
            Span::styled("    F&G: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{} {}", fg, label),
                Style::default()
                    .fg(fear_greed_color(fg))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        // ASCII gauge
        let gauge = ascii_gauge(fg as f64, 20, fear_greed_color(fg));
        lines.push(gauge);
    }

    // BTC Dominance
    if let Some(btcd) = insight.sentiment.btc_dominance {
        lines.push(Line::from(vec![
            Span::styled("    BTC Dom: ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{:.1}%", btcd), Style::default().fg(NEON_CYAN)),
        ]));
    }

    // Funding rate
    if let Some(fr) = insight.funding.funding_rate {
        let fr_color = funding_color(Some(fr));
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

    // On-chain
    if let Some(mvrv) = insight.onchain.mvrv {
        lines.push(Line::from(vec![
            Span::styled("    MVRV: ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{:.2}", mvrv), Style::default().fg(NEON_CYAN)),
        ]));
    }
    if let Some(sopr) = insight.onchain.sopr {
        lines.push(Line::from(vec![
            Span::styled("    SOPR: ", Style::default().fg(DIM_CYAN)),
            Span::styled(format!("{:.2}", sopr), Style::default().fg(NEON_CYAN)),
        ]));
    }

    // News count
    if !insight.rss_items.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("    News: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{} items", insight.rss_items.len()),
                Style::default().fg(NEON_CYAN),
            ),
        ]));
        // RSS headlines
        for item in insight.rss_items.iter().take(5) {
            lines.push(Line::from(vec![
                Span::styled("      ▸ ", Style::default().fg(DIM_CYAN)),
                Span::styled(
                    item.title.chars().take(60).collect::<String>(),
                    Style::default().fg(TEXT_WHITE),
                ),
            ]));
        }
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

    f.render_widget(
        Paragraph::new(lines).block(titled_block("MARKET INSIGHT", NEON_YELLOW)),
        area,
    );
}

// ---------------------------------------------------------------------------
// Tab 6 — AI Decisions
// ---------------------------------------------------------------------------

fn draw_tab_decisions(f: &mut Frame, area: Rect, state: &TuiState) {
    let decisions = &state.snapshot.decisions;
    if decisions.is_empty() {
        f.render_widget(empty_message("No AI decisions yet"), area);
        return;
    }

    let block = titled_block("AI DECISIONS", NEON_YELLOW);

    let lines: Vec<Line> = decisions
        .iter()
        .rev()
        .take(100)
        .map(|d| {
            let action_color = match d.action.as_str() {
                "Buy" | "Long" => NEON_GREEN,
                "Sell" | "Short" => NEON_RED,
                _ => NEON_YELLOW,
            };
            Line::from(vec![
                Span::styled(
                    format!(" {} ", &d.timestamp[11..19]),
                    Style::default().fg(DIM_CYAN),
                ),
                Span::styled(
                    format!(" {:<7}", d.action),
                    Style::default()
                        .fg(action_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}", d.pair),
                    Style::default().fg(TEXT_WHITE).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}", d.side),
                    Style::default().fg(side_color(if d.side == "LONG" {
                        Side::Long
                    } else {
                        Side::Short
                    })),
                ),
                Span::styled(
                    format!(" @ {:.4}", d.entry_price),
                    Style::default().fg(NEON_CYAN),
                ),
                Span::styled(
                    format!("  [{:.0}%]", d.confidence * 100.0),
                    Style::default().fg(NEON_MAGENTA),
                ),
                Span::styled(
                    format!("  {}", d.reasoning.chars().take(60).collect::<String>()),
                    Style::default().fg(DIM_CYAN),
                ),
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

// ---------------------------------------------------------------------------
// Tab 7 — Risk
// ---------------------------------------------------------------------------

fn draw_tab_risk(f: &mut Frame, area: Rect, state: &TuiState) {
    let a = &state.snapshot.account;
    let dd_pct = a.drawdown_pct * 100.0;
    let max_dd = state.snapshot.max_drawdown;
    let max_dl = state.snapshot.max_daily_loss;
    let blocked_threshold = max_dd.min(max_dl);
    let dd_ratio = if blocked_threshold > 0.0 {
        dd_pct / blocked_threshold
    } else {
        0.0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Min(3),
        ])
        .split(area);

    // Circuit breaker status
    let (cb_label, cb_color) = if dd_pct >= blocked_threshold {
        ("⛔ BLOCKED", NEON_RED)
    } else if dd_pct >= max_dd * 0.50 {
        ("⚠ REVIEW", NEON_YELLOW)
    } else if dd_pct >= max_dd * 0.20 {
        ("◉ CAUTION", NEON_YELLOW)
    } else {
        ("✓ CLEAR", NEON_GREEN)
    };

    let cb_lines = vec![
        Line::from(vec![
            Span::styled("  CIRCUIT BREAKER: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                cb_label,
                Style::default().fg(cb_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Drawdown: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{:.1}%", dd_pct),
                Style::default()
                    .fg(traffic_color(dd_ratio * 100.0))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" / {:.0}% max", blocked_threshold),
                Style::default().fg(DIM_CYAN),
            ),
        ]),
        ascii_gauge(dd_ratio * 100.0, 30, traffic_color(dd_ratio * 100.0)),
    ];
    f.render_widget(
        Paragraph::new(cb_lines).block(titled_block("RISK STATUS", NEON_RED)),
        chunks[0],
    );

    // Risk limits
    let limit_lines = vec![
        stat_row(vec![
            ("Max DD: ", format!("{:.0}%", max_dd), NEON_RED),
            ("Max Daily Loss: ", format!("{:.0}%", max_dl), NEON_YELLOW),
            ("Max Positions: ", format!("{}", 5), NEON_CYAN), // from config
        ]),
        stat_row(vec![
            (
                "Current DD: ",
                format!("{:.1}%", dd_pct),
                traffic_color(dd_ratio * 100.0),
            ),
            (
                "Current Pos: ",
                format!("{}", state.snapshot.positions.len()),
                NEON_CYAN,
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(limit_lines).block(titled_block("LIMITS", NEON_RED)),
        chunks[1],
    );

    // Memory + CUSUM
    let mem = &state.snapshot.memory;
    let mem_lines = vec![stat_row(vec![
        (
            "Brier: ",
            mem.brier_score
                .map(|s| format!("{:.2}", s))
                .unwrap_or_else(|| "N/A".into()),
            NEON_CYAN,
        ),
        ("Cap: ", mem.confidence_cap.to_string(), NEON_MAGENTA),
        ("Trades: ", format!("{}", mem.total_trades), TEXT_WHITE),
    ])];
    f.render_widget(
        Paragraph::new(mem_lines).block(titled_block("MEMORY", NEON_MAGENTA)),
        chunks[2],
    );

    // Thresholds legend
    let caution_t = max_dd * 0.20;
    let review_t = max_dd * 0.50;
    let thresh_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("    {:.0}% ", caution_t),
                Style::default().fg(NEON_YELLOW),
            ),
            Span::styled("→ Caution", Style::default().fg(DIM_CYAN)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("    {:.0}% ", review_t),
                Style::default().fg(NEON_YELLOW),
            ),
            Span::styled("→ Review", Style::default().fg(DIM_CYAN)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("    {:.0}% ", blocked_threshold),
                Style::default().fg(NEON_RED),
            ),
            Span::styled("→ Blocked", Style::default().fg(DIM_CYAN)),
        ]),
    ];
    f.render_widget(
        Paragraph::new(thresh_lines).block(titled_block("THRESHOLDS", DIM_CYAN)),
        chunks[3],
    );
}

// ---------------------------------------------------------------------------
// Tab 8 — Memory
// ---------------------------------------------------------------------------

fn draw_tab_memory(f: &mut Frame, area: Rect, state: &TuiState) {
    let mem = &state.snapshot.memory;
    let mut lines: Vec<Line> = Vec::new();

    // Brier Score
    let (brier_display, brier_color) = match mem.brier_score {
        Some(s) => (
            format!("{:.3} ({})", s, mem.brier_label),
            if s <= 0.15 {
                NEON_GREEN
            } else if s <= 0.25 {
                NEON_YELLOW
            } else if s <= 0.35 {
                NEON_MAGENTA
            } else {
                NEON_RED
            },
        ),
        None => ("Insufficient data".to_string(), DIM_CYAN),
    };

    lines.push(Line::from(vec![
        Span::styled("  Brier Score: ", Style::default().fg(DIM_CYAN)),
        Span::styled(
            brier_display,
            Style::default()
                .fg(brier_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Confidence cap
    lines.push(Line::from(vec![
        Span::styled("  Confidence Cap: ", Style::default().fg(DIM_CYAN)),
        Span::styled(
            &mem.confidence_cap,
            Style::default()
                .fg(NEON_MAGENTA)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({} trades)", mem.total_trades),
            Style::default().fg(DIM_CYAN),
        ),
    ]));

    // CUSUM status
    if !mem.cusum_status.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  CUSUM Edge Detection:",
            Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
        )]));
        for (pair, status) in mem.cusum_status.iter().take(10) {
            let status_color = match status.as_str() {
                "positive" | "improving" => NEON_GREEN,
                "negative" | "decay" => NEON_RED,
                _ => DIM_CYAN,
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {} ", pair), Style::default().fg(TEXT_WHITE)),
                Span::styled(
                    status,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    // Replay lessons
    if mem.replay_lesson_count > 0 {
        lines.push(Line::from(vec![
            Span::styled("  Replay Lessons: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{}", mem.replay_lesson_count),
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines).block(titled_block("MEMORY", NEON_MAGENTA)),
        area,
    );
}

// ---------------------------------------------------------------------------
// Tab 9 — Config (read-only)
// ---------------------------------------------------------------------------

fn draw_tab_config(f: &mut Frame, area: Rect, state: &TuiState) {
    let s = &state.snapshot;
    let lines = vec![
        Line::from(vec![
            Span::styled("  Mode: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                &s.mode_label,
                Style::default()
                    .fg(if s.mode_label == "LIVE" {
                        NEON_RED
                    } else {
                        NEON_GREEN
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Backend: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                &s.backend_name,
                Style::default()
                    .fg(NEON_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Model: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                &s.model_name,
                Style::default()
                    .fg(ACCENT_BLUE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Starting Balance: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("${:.0}", s.starting_balance),
                Style::default()
                    .fg(NEON_MAGENTA)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  RISK LIMITS",
            Style::default().fg(NEON_RED).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    Max Drawdown: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{:.0}%", s.max_drawdown),
                Style::default().fg(NEON_RED),
            ),
        ]),
        Line::from(vec![
            Span::styled("    Max Daily Loss: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                format!("{:.0}%", s.max_daily_loss),
                Style::default().fg(NEON_YELLOW),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Version: ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                env!("CARGO_PKG_VERSION"),
                Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(titled_block("CONFIGURATION", ACCENT_BLUE)),
        area,
    );
}

// ---------------------------------------------------------------------------
// Tab 0 — Activity Log
// ---------------------------------------------------------------------------

fn draw_tab_activity(f: &mut Frame, area: Rect, state: &TuiState) {
    let activity = &state.snapshot.activity;
    if activity.is_empty() {
        f.render_widget(empty_message("Waiting for engine activity..."), area);
        return;
    }

    let block = titled_block("LIVE ACTIVITY", NEON_MAGENTA);

    let lines: Vec<Line> = activity
        .iter()
        .rev()
        .take(30)
        .map(|entry| {
            let (icon, color) = match entry.level {
                crate::core::shared::ActivityLevel::Info => ("INFO", DIM_CYAN),
                crate::core::shared::ActivityLevel::Thinking => ("THINK", NEON_CYAN),
                crate::core::shared::ActivityLevel::Decision => ("DECIDE", NEON_YELLOW),
                crate::core::shared::ActivityLevel::Trade => ("TRADE", NEON_GREEN),
                crate::core::shared::ActivityLevel::Warning => ("WARN", NEON_YELLOW),
                crate::core::shared::ActivityLevel::Error => ("ERROR", NEON_RED),
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
                    Style::default().fg(TEXT_WHITE),
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
