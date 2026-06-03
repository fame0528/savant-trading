/// Enterprise console logging — single source of truth for all output.
///
/// Format: `[Savant Trading] [MM-DD-YYYY HH:mm AM/PM] [ACTION] [RESULT]`
///
/// Every log in the system goes through `savant_log()`. Macros are thin wrappers.
///
/// Color schema:
///   - Cyan bold — brand prefix, decisions, swap actions
///   - Green — success, LLM complete
///   - Orange — warnings, trade actions
///   - Red — errors, failures, circuit breaker
///   - White — emphasis, results, data
///   - Grey — timestamps, LLM evaluation, background info
///
/// Rules:
///   - Action and result MUST have different colors (contrast)
///   - Pair names wrapped in brackets and highlighted: `[BTC/USD]`
///   - Single RESET at end of line prevents bleeding
// ── ANSI codes ──────────────────────────────────────────────────────────
// Foreground only
pub const CYAN_FG: &str = "\x1b[36m";
pub const GREEN_FG: &str = "\x1b[32m";
pub const ORANGE_FG: &str = "\x1b[33m";
pub const RED_FG: &str = "\x1b[31m";
pub const WHITE_FG: &str = "\x1b[97m";
pub const GREY_FG: &str = "\x1b[37m";       // Light grey (readable on black bg)

// Bold + foreground (compound — preserves bold across color changes)
pub const CYAN_BOLD: &str = "\x1b[1;36m";
pub const GREEN_BOLD: &str = "\x1b[1;32m";
pub const ORANGE_BOLD: &str = "\x1b[1;33m";
pub const RED_BOLD: &str = "\x1b[1;31m";
pub const WHITE_BOLD: &str = "\x1b[1;97m";

// Dim — used only for background info (vault, episodic)
pub const GREY_DIM: &str = "\x1b[90m";       // Grey (no dim modifier — readable on black)

// Reset — use ONLY at end of line
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

/// Highlight trading pair names in result text with cyan bold brackets.
/// `BTC/USD` → `[BTC/USD]` in cyan bold
fn highlight_pairs(text: &str) -> String {
    let mut result = text.to_string();
    for pair in TRADING_PAIRS {
        // Only highlight if not already in brackets
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

/// Custom tracing timer — outputs EST timestamps matching the console format.
pub struct SavantTimer;

impl tracing_subscriber::fmt::time::FormatTime for SavantTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", est_now())
    }
}

/// Log level determines action + result colors.
///
/// KEY RULE: action and result MUST have different colors for contrast.
pub enum LogLevel {
    /// Bold white action, white result — system phase headers
    Phase,
    /// Grey action, white result — LLM evaluation in progress
    Llm,
    /// Grey action, green result — LLM evaluation complete
    LlmDone,
    /// Bold cyan action, white result — AI decisions
    Decision,
    /// Bold orange action, orange result — trade opened/closed
    Trade,
    /// Bold cyan action, dim result — swap in progress
    Swap,
    /// Bold green action, green result — swap success
    SwapOk,
    /// Bold red action, red result — swap failure
    SwapFail,
    /// Dim grey action, dim result — vault/episodic
    Vault,
    /// Bold red action, red result — circuit breaker
    Circuit,
    /// Bold orange action, orange result — warnings
    Warn,
}

/// Single log function — ALL console output goes through here.
///
/// Format: `[Savant Trading] [MM-DD-YYYY HH:mm AM/PM] [ACTION] RESULT`
///
/// Uses compound ANSI codes to preserve bold across sections.
/// Single RESET at end prevents color bleeding to next line.
pub fn savant_log(level: LogLevel, action: &str, result: &str) {
    let (action_style, result_style) = match level {
        LogLevel::Phase => (WHITE_BOLD, WHITE_FG),
        LogLevel::Llm => (GREY_FG, WHITE_FG),       // Grey action, white result
        LogLevel::LlmDone => (GREY_FG, GREEN_FG),    // Grey action, green result
        LogLevel::Decision => (CYAN_BOLD, WHITE_FG),  // Cyan action, white result
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

    // Format: [Savant Trading] [TIME] [ACTION] RESULT
    // Each section uses compound codes — no RESET between sections
    // Single RESET at end prevents bleeding
    eprintln!(
        "{}[Savant Trading]{} {}[{}]{} {}[{}]{} {}{}{}",
        CYAN_BOLD, RESET,
        GREY_FG, ts, RESET,
        action_style, action, RESET,
        result_style, highlighted, RESET,
    );
}

// ── Thin macros ──────────────────────────────────────────────────────────

#[macro_export]
macro_rules! log_phase {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Phase,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_llm {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Llm,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_llm_done {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::LlmDone,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_decision {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Decision,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_trade {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Trade,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_swap {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Swap,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_swap_ok {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::SwapOk,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_swap_fail {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::SwapFail,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_vault {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Vault,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_circuit {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Circuit,
            $action,
            &format!($($arg)*),
        );
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($action:expr, $($arg:tt)*) => {{
        $crate::core::console::savant_log(
            $crate::core::console::LogLevel::Warn,
            $action,
            &format!($($arg)*),
        );
    }};
}
