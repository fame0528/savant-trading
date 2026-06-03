//! Custom Ratatui widgets and rendering helpers.
//!
//! Shared building blocks used by the tab renderers in [`super::tabs`].

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::core::types::Side;

// ---------------------------------------------------------------------------
// Neon palette (shared across all tabs)
// ---------------------------------------------------------------------------

pub const NEON_CYAN: Color = Color::Cyan;
pub const NEON_GREEN: Color = Color::Green;
pub const NEON_RED: Color = Color::Red;
pub const NEON_YELLOW: Color = Color::Yellow;
pub const NEON_MAGENTA: Color = Color::Magenta;
pub const DIM_CYAN: Color = Color::DarkGray;
pub const ACCENT_BLUE: Color = Color::Blue;
pub const TEXT_WHITE: Color = Color::White;

// ---------------------------------------------------------------------------
// Side colors
// ---------------------------------------------------------------------------

pub fn side_color(side: Side) -> Color {
    match side {
        Side::Long => NEON_GREEN,
        Side::Short => NEON_RED,
    }
}

pub fn pnl_color(pnl: f64) -> Color {
    if pnl >= 0.0 {
        NEON_GREEN
    } else {
        NEON_RED
    }
}

// ---------------------------------------------------------------------------
// Block / border shortcuts
// ---------------------------------------------------------------------------

/// Create a bordered block with a title and neon cyan border.
pub fn titled_block(title: &str, border_color: Color) -> Block<'static> {
    Block::default()
        .title(format!(" ◆ {} ", title))
        .title_style(
            Style::default()
                .fg(border_color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
}

/// Empty-state message (shown when a panel has no data).
pub fn empty_message(msg: &str) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ◇ ", Style::default().fg(DIM_CYAN)),
            Span::styled(
                msg.to_string(),
                Style::default()
                    .fg(DIM_CYAN)
                    .add_modifier(ratatui::style::Modifier::ITALIC),
            ),
        ]),
    ])
}

// ---------------------------------------------------------------------------
// ASCII gauge bar
// ---------------------------------------------------------------------------

/// Render a horizontal ASCII gauge: `██████░░░░░░` with color.
pub fn ascii_gauge(value_pct: f64, width: usize, color: Color) -> Line<'static> {
    let filled = ((value_pct / 100.0).clamp(0.0, 1.0) * width as f64) as usize;
    let empty = width.saturating_sub(filled);
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("░".repeat(empty), Style::default().fg(DIM_CYAN)),
    ])
}

// ---------------------------------------------------------------------------
// Search bar overlay
// ---------------------------------------------------------------------------

/// Render a search bar at the top of an area.
#[allow(dead_code)]
pub fn search_bar(query: &str) -> Paragraph<'static> {
    let prompt = if query.is_empty() {
        " Search: (type to filter, Esc to cancel, Enter to confirm)".to_string()
    } else {
        format!(" Search: {}█", query)
    };
    Paragraph::new(Line::from(Span::styled(
        prompt,
        Style::default()
            .fg(NEON_YELLOW)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(NEON_YELLOW)),
    )
}

// ---------------------------------------------------------------------------
// Color intensity helpers
// ---------------------------------------------------------------------------

/// Map a percentage (0-100) to a traffic-light color.
pub fn traffic_color(pct: f64) -> Color {
    if pct >= 75.0 {
        NEON_RED
    } else if pct >= 50.0 {
        NEON_YELLOW
    } else {
        NEON_GREEN
    }
}

/// Map a funding rate to an appropriate color.
pub fn funding_color(rate: Option<f64>) -> Color {
    match rate {
        Some(r) if r > 0.01 => NEON_RED,
        Some(r) if r < -0.01 => NEON_GREEN,
        Some(_) => NEON_YELLOW,
        None => DIM_CYAN,
    }
}

/// Map a fear-greed index to color.
pub fn fear_greed_color(index: u32) -> Color {
    if index < 25 {
        NEON_RED
    } else if index < 50 {
        NEON_YELLOW
    } else if index < 75 {
        NEON_GREEN
    } else {
        NEON_CYAN
    }
}

// ---------------------------------------------------------------------------
// Stat line helper
// ---------------------------------------------------------------------------

/// Build a single stat row like: `BAL $47.23  │  EQUITY $48.50`
pub fn stat_row(items: Vec<(&str, String, Color)>) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (label, value, color)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  │  ", Style::default().fg(DIM_CYAN)));
        }
        spans.push(Span::styled(
            format!("{}{}", label, value),
            Style::default()
                .fg(*color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }
    Line::from(spans)
}
