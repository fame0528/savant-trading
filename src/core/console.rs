// Enterprise console logging — single source of truth for ALL output.
//
// Format: `[Savant Trading] [MM-DD-YYYY HH:mm AM/PM] [ACTION] Result`
//
// Both `savant_log()` macros AND `tracing` events use the same format.
// This file provides:
//   - `savant_log()` — direct styled logging for engine events
//   - `SavantLayer` — custom tracing Layer for library/framework events
//   - ANSI color constants
// ── ANSI codes ──────────────────────────────────────────────────────────

pub const CYAN_FG: &str = "\x1b[36m";
pub const GREEN_FG: &str = "\x1b[32m";
pub const ORANGE_FG: &str = "\x1b[33m";
pub const RED_FG: &str = "\x1b[31m";
pub const WHITE_FG: &str = "\x1b[97m";
pub const GREY_FG: &str = "\x1b[90m";

pub const CYAN_BOLD: &str = "\x1b[1;36m";
pub const GREEN_BOLD: &str = "\x1b[1;32m";
pub const ORANGE_BOLD: &str = "\x1b[1;33m";
pub const RED_BOLD: &str = "\x1b[1;31m";
pub const WHITE_BOLD: &str = "\x1b[1;97m";

pub const GREY_DIM: &str = "\x1b[2;37m";
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
    DecisionBuy,
    DecisionSell,
    DecisionPass,
    DecisionClose,
    DecisionAdjust,
    Trade,
    Swap,
    SwapOk,
    SwapFail,
    Vault,
    Circuit,
    Warn,
}

/// Print a styled log line.
///
/// Format: `[Savant Trading] [TIME] [ACTION] Result`
/// Colors flow continuously — no RESET between segments.
pub fn savant_log(level: LogLevel, action: &str, result: &str) {
    let (action_color, result_color) = match level {
        LogLevel::Phase => (CYAN_BOLD, WHITE_FG),
        LogLevel::Llm => (CYAN_BOLD, WHITE_FG),
        LogLevel::LlmDone => (GREEN_BOLD, WHITE_FG),
        LogLevel::Decision => (CYAN_BOLD, WHITE_FG),
        LogLevel::DecisionBuy => (GREEN_BOLD, GREEN_FG),
        LogLevel::DecisionSell => (RED_BOLD, RED_FG),
        LogLevel::DecisionPass => (GREY_FG, GREY_FG),
        LogLevel::DecisionClose => (ORANGE_BOLD, ORANGE_FG),
        LogLevel::DecisionAdjust => (ORANGE_FG, ORANGE_FG),
        LogLevel::Trade => (ORANGE_BOLD, ORANGE_FG),
        LogLevel::Swap => (CYAN_FG, WHITE_FG),
        LogLevel::SwapOk => (GREEN_BOLD, GREEN_FG),
        LogLevel::SwapFail => (RED_BOLD, RED_FG),
        LogLevel::Vault => ("\x1b[34m", GREY_FG),
        LogLevel::Circuit => (RED_BOLD, RED_FG),
        LogLevel::Warn => (ORANGE_BOLD, ORANGE_FG),
    };

    let ts = est_now();

    // Colors flow continuously — RESET only before result to apply result_color
    eprintln!(
        "\x1b[1;36m[Savant Trading] \x1b[90m[{}] {}[{}]\x1b[0m {}{}",
        ts, action_color, action, result_color, result
    );
}

// ── SavantLayer — custom tracing Layer ───────────────────────────────────

use tracing_subscriber::Layer;
use std::fmt::Write as FmtWrite;

pub struct SavantLayer;

/// Capitalize module name: `funding_rates` → `FundingRates`, `kraken` → `Kraken`
fn capitalize_module(name: &str) -> String {
    // Special cases for well-known module names
    match name {
        "onchain" => return "On Chain".to_string(),
        "websocket" => return "WebSocket".to_string(),
        "funding_rates" => return "Funding Rates".to_string(),
        "coinmarketcap" => return "CoinMarketCap".to_string(),
        "goplus" => return "GoPlus".to_string(),
        "kraken" => return "Kraken Data".to_string(),
        "trader" => return "DEX Trader".to_string(),
        "paper" => return "Balance".to_string(),
        "episodic" => return "Episodic Memory".to_string(),
        "aggregator" => return "Insight".to_string(),
        "liquidation" => return "Liquidation".to_string(),
        "rss" => return "RSS".to_string(),
        "news" => return "News".to_string(),
        "watcher" => return "Vault Watcher".to_string(),
        "market_data" => return "Market Data".to_string(),
        _ => {}
    }
    // Default: capitalize each word, join with space
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

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

        // Map tracing level to label and colors
        let (action, action_color, msg_color) = match *level {
            tracing::Level::ERROR => ("ERROR", RED_BOLD, RED_FG),
            tracing::Level::WARN => ("WARN", ORANGE_BOLD, ORANGE_FG),
            tracing::Level::INFO => ("INFO", WHITE_BOLD, WHITE_FG),
            tracing::Level::DEBUG => ("DEBUG", GREY_FG, GREY_FG),
            tracing::Level::TRACE => ("TRACE", GREY_DIM, GREY_DIM),
        };

        // Extract the message from the event
        let mut message = String::new();
        let mut visitor = MessageVisitor(&mut message);
        event.record(&mut visitor);

        // Extract target (module path) as a short, capitalized label
        let target = metadata.target();
        let short_target = target
            .rsplit("::")
            .next()
            .unwrap_or(target);
        let formatted_target = capitalize_module(short_target);

        let ts = est_now();

        // Colors flow continuously — RESET only before message to apply msg_color
        let line = format!(
            "\x1b[1;36m[Savant Trading] \x1b[90m[{}] {}[{}] \x1b[90m[{}]\x1b[0m {}{}",
            ts, action_color, action, formatted_target, msg_color, message
        );
        eprintln!("{}", line);
    }
}

/// Visitor to extract the message from a tracing event.
struct MessageVisitor<'a>(&'a mut String);

impl tracing::field::Visit for MessageVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            // Remove surrounding quotes from Debug format
            let debug_str = format!("{:?}", value);
            if debug_str.starts_with('"') && debug_str.ends_with('"') {
                self.0.push_str(&debug_str[1..debug_str.len()-1]);
            } else {
                self.0.push_str(&debug_str);
            }
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
        let level = match $action {
            "BUY" => $crate::core::console::LogLevel::DecisionBuy,
            "SELL" => $crate::core::console::LogLevel::DecisionSell,
            "PASS" => $crate::core::console::LogLevel::DecisionPass,
            "CLOSE" => $crate::core::console::LogLevel::DecisionClose,
            "ADJUST" => $crate::core::console::LogLevel::DecisionAdjust,
            _ => $crate::core::console::LogLevel::Decision,
        };
        $crate::core::console::savant_log(level, $action, &format!($($arg)*));
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
