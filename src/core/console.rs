/// Enterprise console logging — single source of truth for all output.
///
/// Format: `[Savant Trading] [MM-DD-YYYY HH:mm] [ACTION] [RESULT]`
///
/// Every log in the system goes through `savant_log()`. Macros are thin wrappers.
pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const ORANGE: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";
pub const WHITE: &str = "\x1b[97m";
pub const GREY: &str = "\x1b[90m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RESET: &str = "\x1b[0m";

/// Log level determines action + result colors.
pub enum LogLevel {
    /// System phase headers — white action, white result
    Phase,
    /// LLM evaluation in progress — grey action, dim result
    Llm,
    /// LLM evaluation complete — grey action, green result
    LlmDone,
    /// AI decision output — white action, white result
    Decision,
    /// Trade opened/closed — orange action, orange result
    Trade,
    /// Swap in progress — cyan action, dim result
    Swap,
    /// Swap success — green action, green result
    SwapOk,
    /// Swap failure — red action, red result
    SwapFail,
    /// Vault/episodic write — dim action, dim result
    Vault,
    /// Circuit breaker — red action, red result
    Circuit,
    /// Warning — orange action, orange result
    Warn,
}

/// Single log function — ALL console output goes through here.
///
/// `[Savant Trading] [MM-DD-YYYY HH:mm] [ACTION] [RESULT]`
pub fn savant_log(level: LogLevel, action: &str, result: &str) {
    let (action_color, result_color) = match level {
        LogLevel::Phase => (WHITE, WHITE),
        LogLevel::Llm => (GREY, DIM),
        LogLevel::LlmDone => (GREY, GREEN),
        LogLevel::Decision => (WHITE, WHITE),
        LogLevel::Trade => (ORANGE, ORANGE),
        LogLevel::Swap => (CYAN, DIM),
        LogLevel::SwapOk => (GREEN, GREEN),
        LogLevel::SwapFail => (RED, RED),
        LogLevel::Vault => (DIM, DIM),
        LogLevel::Circuit => (RED, RED),
        LogLevel::Warn => (ORANGE, ORANGE),
    };

    let now = chrono::Utc::now();
    let est = now - chrono::Duration::hours(5);
    let ts = est.format("%m-%d-%Y %H:%M").to_string();

    eprintln!(
        "{}{}[Savant Trading]{} {}[{}]{} {}{}[{}]{} {}{}",
        BOLD, CYAN, RESET,
        GREY, ts, RESET,
        action_color, BOLD, action, RESET,
        result_color, result,
    );
}

// ── Thin macros ─────────────────────────────────────────────────────────

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
