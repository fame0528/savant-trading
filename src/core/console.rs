// Enterprise console logging — single source of truth for ALL output.
//
// Format: `[Savant Trading] [MM-DD-YYYY HH:mm AM/PM] [ACTION] [RESULT]`
//
// Both `savant_log()` macros AND `tracing` events use the same format.
// This file provides:
//   - `savant_log()` — direct styled logging for engine events
//   - `SavantLayer` — custom tracing Layer for library/framework events
//   - ANSI color constants
//   - Pair name highlighting
// ── ANSI codes ──────────────────────────────────────────────────────────

pub const CYAN_FG: &str = "\x1b[36m";
pub const GREEN_FG: &str = "\x1b[32m";
pub const ORANGE_FG: &str = "\x1b[33m";
pub const RED_FG: &str = "\x1b[31m";
pub const WHITE_FG: &str = "\x1b[97m";
pub const GREY_FG: &str = "\x1b[37m";

pub const CYAN_BOLD: &str = "\x1b[1;36m";
pub const GREEN_BOLD: &str = "\x1b[1;32m";
pub const ORANGE_BOLD: &str = "\x1b[1;33m";
pub const RED_BOLD: &str = "\x1b[1;31m";
pub const WHITE_BOLD: &str = "\x1b[1;97m";

pub const GREY_DIM: &str = "\x1b[90m";
pub const RESET: &str = "\x1b[0m";

// Legacy aliases
pub const CYAN: &str = CYAN_FG;
pub const GREEN: &str = GREEN_FG;
pub const ORANGE: &str = ORANGE_FG;
pub const RED: &str = RED_FG;
pub const WHITE: &str = WHITE_FG;
pub const GREY: &str = GREY_FG;
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

// ── Trading pairs for highlighting ───────────────────────────────────────

const TRADING_PAIRS: &[&str] = &[
    "BTC/USD", "ETH/USD", "SOL/USD", "XRP/USD",
    "DOGE/USD", "ADA/USD", "LINK/USD", "AVAX/USD",
    "BTC/USDC", "ETH/USDC", "SOL/USDC", "XRP/USDC",
];

/// Highlight trading pair names with cyan bold brackets.
fn highlight_pairs(text: &str) -> String {
    let mut result = text.to_string();
    for pair in TRADING_PAIRS {
        let bracketed = format!("[{}]", pair);
        if result.contains(pair) && !result.contains(&bracketed) {
            result = result.replace(pair, &format!("{}[{}]{}", CYAN_BOLD, pair, RESET));
        }
    }
    result
}

/// Returns current EST time formatted as `MM-DD-YYYY H:MM AM/PM`.
pub fn est_now() -> String {
    let now = chrono::Utc::now();
    let est = now - chrono::Duration::hours(5);
    let hour = est.format("%I").to_string();
    let hour = hour.trim_start_matches('0');
    let minute = est.format("%M").to_string();
    let ampm = est.format("%p").to_string();
    format!("{} {}:{} {}", est.format("%m-%d-%Y"), hour, minute, ampm)
}

// ── savant_log — direct styled logging ───────────────────────────────────

pub enum LogLevel {
    Phase,
    Llm,
    LlmDone,
    Decision,
    Trade,
    Swap,
    SwapOk,
    SwapFail,
    Vault,
    Circuit,
    Warn,
}

pub fn savant_log(level: LogLevel, action: &str, result: &str) {
    let (action_style, result_style) = match level {
        LogLevel::Phase => (WHITE_BOLD, WHITE_FG),
        LogLevel::Llm => (GREY_FG, WHITE_FG),
        LogLevel::LlmDone => (GREY_FG, GREEN_FG),
        LogLevel::Decision => (CYAN_BOLD, WHITE_FG),
        LogLevel::Trade => (ORANGE_BOLD, ORANGE_FG),
        LogLevel::Swap => (CYAN_BOLD, GREY_DIM),
        LogLevel::SwapOk => (GREEN_BOLD, GREEN_FG),
        LogLevel::SwapFail => (RED_BOLD, RED_FG),
        LogLevel::Vault => (GREY_DIM, GREY_DIM),
        LogLevel::Circuit => (RED_BOLD, RED_FG),
        LogLevel::Warn => (ORANGE_BOLD, ORANGE_FG),
    };

    let ts = est_now();
    let highlighted = highlight_pairs(result);

    eprintln!(
        "{}[Savant Trading]{} {}[{}]{} {}[{}]{} {}{}{}",
        CYAN_BOLD, RESET,
        GREY_FG, ts, RESET,
        action_style, action, RESET,
        result_style, highlighted, RESET,
    );
}

// ── SavantLayer — custom tracing Layer ───────────────────────────────────
//
// Formats ALL tracing events in the same `[Savant Trading] [TIME] [LEVEL] msg`
// format as savant_log. This makes tracing output uniform with our styled logs.

use tracing_subscriber::Layer;
use std::fmt::Write as FmtWrite;

pub struct SavantLayer;

impl<S> Layer<S> for SavantLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = metadata.level();

        // Map tracing level to action label and color
        let (action, action_color, result_color) = match *level {
            tracing::Level::ERROR => ("ERROR", RED_BOLD, RED_FG),
            tracing::Level::WARN => ("WARN", ORANGE_BOLD, ORANGE_FG),
            tracing::Level::INFO => ("INFO", GREY_FG, WHITE_FG),
            tracing::Level::DEBUG => ("DEBUG", GREY_DIM, GREY_DIM),
            tracing::Level::TRACE => ("TRACE", GREY_DIM, GREY_DIM),
        };

        // Extract the message from the event
        let mut message = String::new();
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);

        // Extract target (module path) as a short label
        let target = metadata.target();
        let short_target = target
            .rsplit("::")
            .next()
            .unwrap_or(target);

        let ts = est_now();
        let highlighted = highlight_pairs(&message);

        // Format: [Savant Trading] [TIME] [LEVEL] [module] message
        eprintln!(
            "{}[Savant Trading]{} {}[{}]{} {}[{}]{} {}[{}]{} {}{}{}",
            CYAN_BOLD, RESET,
            GREY_FG, ts, RESET,
            action_color, action, RESET,
            GREY_DIM, short_target, RESET,
            result_color, highlighted, RESET,
        );
    }
}

/// Visitor to extract the message from a tracing event.
struct MessageVisitor<'a>(&'a mut String);

impl tracing::field::Visit for MessageVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.0, "{:?}", value);
        } else {
            let _ = write!(self.0, " {}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            let _ = write!(self.0, " {}={}", field.name(), value);
        }
    }
}

// ── Timer for tracing subscriber ─────────────────────────────────────────

pub struct SavantTimer;

impl tracing_subscriber::fmt::time::FormatTime for SavantTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", est_now())
    }
}

// ── Thin macros ──────────────────────────────────────────────────────────

#[macro_export]
macro_rules! log_phase {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Phase, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_llm {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Llm, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_llm_done {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::LlmDone, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_decision {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Decision, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_trade {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Trade, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_swap {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Swap, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_swap_ok {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::SwapOk, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_swap_fail {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::SwapFail, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_vault {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Vault, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_circuit {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Circuit, $action, &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log($crate::core::console::LogLevel::Warn, $action, &format!($($arg)*));
    }};
}
