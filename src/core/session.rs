//! Session detection — knows what trading session is active and adjusts behavior.
//!
//! Sessions from transcripts:
//! - Asian: 7 PM - 2 AM EST (low volume, ranging)
//! - London: 2 AM - 5 AM EST (high volume, reversals)
//! - NY: 7 AM - 10 AM EST (highest volume, continuations)
//! - London/NY Overlap: 8 AM - 10 AM EST (peak volume)
//! - Off-hours: reduced position size, wider stops

use chrono::{Datelike, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};

/// Trading session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Session {
    Asian,
    London,
    NewYork,
    LondonNyOverlap,
    OffHours,
}

impl Session {
    /// Human-readable name.
    pub fn name(&self) -> &str {
        match self {
            Session::Asian => "Asian",
            Session::London => "London",
            Session::NewYork => "New York",
            Session::LondonNyOverlap => "London/NY Overlap",
            Session::OffHours => "Off-Hours",
        }
    }

    /// Expected behavior during this session.
    pub fn behavior(&self) -> &str {
        match self {
            Session::Asian => "Low volume, ranging. Reduce position size. Look for range extremes.",
            Session::London => {
                "High volume, reversals. Sweep of Asian range common. Enter on reversal."
            }
            Session::NewYork => {
                "Highest volume, continuations. Look for continuation of London move."
            }
            Session::LondonNyOverlap => {
                "Peak volume. Best time for breakouts. Highest probability window."
            }
            Session::OffHours => {
                "Low activity. Reduce position size. Widen stops. Avoid new entries."
            }
        }
    }

    /// Position size multiplier for this session.
    pub fn position_size_multiplier(&self) -> f64 {
        match self {
            Session::Asian => 0.5,
            Session::London => 1.0,
            Session::NewYork => 1.0,
            Session::LondonNyOverlap => 1.2,
            Session::OffHours => 0.3,
        }
    }

    /// Whether this session is a "kill zone" (high probability).
    pub fn is_kill_zone(&self) -> bool {
        matches!(
            self,
            Session::London | Session::NewYork | Session::LondonNyOverlap
        )
    }
}

/// Detect the current trading session based on UTC time.
///
/// All times converted from EST to UTC at startup:
/// - EST = UTC-5 (winter) or UTC-4 (summer/EDT)
/// - Using EDT (UTC-4) for current dates
pub fn current_session() -> Session {
    let now = Utc::now();
    let hour = now.hour();

    // Convert UTC to EST (UTC-4 during EDT)
    let est_hour = if hour >= 4 { hour - 4 } else { hour + 20 };

    // Weekend check
    let weekday = now.weekday();
    if weekday == Weekday::Sat || weekday == Weekday::Sun {
        return Session::OffHours;
    }

    match est_hour {
        // Asian: 7 PM - 2 AM EST
        19..=23 | 0..=1 => Session::Asian,
        // London: 2 AM - 5 AM EST
        2..=4 => Session::London,
        // London/NY Overlap: 8 AM - 10 AM EST
        8..=9 => Session::LondonNyOverlap,
        // NY: 7 AM - 10 AM EST (excluding overlap)
        7 | 10 => Session::NewYork,
        // Everything else is off-hours
        _ => Session::OffHours,
    }
}

/// Get session info as a formatted string for the AI context.
pub fn session_context() -> String {
    let session = current_session();
    format!(
        "Session: {} | Behavior: {} | Size Multiplier: {:.1}x | Kill Zone: {}",
        session.name(),
        session.behavior(),
        session.position_size_multiplier(),
        if session.is_kill_zone() { "YES" } else { "NO" }
    )
}
