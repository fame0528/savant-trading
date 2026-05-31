//! Session detection — crypto-native 24/7 trading sessions.
//!
//! Crypto never closes. Instead of "off-hours", sessions reflect
//! volatility patterns, liquidity windows, and funding rate resets.
//!
//! Key crypto session patterns:
//! - Asian (00:00-08:00 UTC): Lower volume, range-bound, good for mean reversion
//! - European (08:00-16:00 UTC): Higher volume, trend continuations
//! - US (14:00-22:00 UTC): Highest volume, major moves, liquidation cascades
//! - Weekend: Lower volume but still tradeable, often mean-reverting
//! - Funding Reset (every 8h at 00:00, 08:00, 16:00 UTC): Volatility spike

use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Trading session — crypto-native, no "off-hours".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Session {
    /// 00:00-08:00 UTC — Lower volume, range-bound, mean reversion favored
    Asian,
    /// 08:00-14:00 UTC — European session, higher volume, trend continuations
    European,
    /// 14:00-22:00 UTC — US session, highest volume, major moves
    UsSession,
    /// 22:00-00:00 UTC — Late US / pre-Asian transition
    LateUs,
    /// Saturday/Sunday — Lower volume but still active
    Weekend,
}

impl Session {
    pub fn name(&self) -> &str {
        match self {
            Session::Asian => "Asian",
            Session::European => "European",
            Session::UsSession => "US",
            Session::LateUs => "Late US",
            Session::Weekend => "Weekend",
        }
    }

    pub fn behavior(&self) -> &str {
        match self {
            Session::Asian => "Lower volume, range-bound. Mean reversion setups favored. BTC often consolidates. Good for scalping range extremes.",
            Session::European => "Rising volume, trend continuations from Asian session. London open often sets daily direction. Breakout setups favored.",
            Session::UsSession => "Highest volume and volatility. Major moves, liquidation cascades, and trend extensions. Best for momentum trades. FOMC/CPI impact here.",
            Session::LateUs => "Volume declining, position squaring. Good for mean reversion as traders close positions before Asian open.",
            Session::Weekend => "Lower volume but still tradeable. Often mean-reverting. Reduced position size. BTC weekend moves often reverse Monday.",
        }
    }

    pub fn position_size_multiplier(&self) -> f64 {
        match self {
            Session::Asian => 0.8,
            Session::European => 1.0,
            Session::UsSession => 1.2,
            Session::LateUs => 0.9,
            Session::Weekend => 0.7,
        }
    }

    /// All crypto sessions are tradeable — no "off-hours" in 24/7 markets.
    pub fn is_kill_zone(&self) -> bool {
        matches!(self, Session::European | Session::UsSession)
    }

    /// Check if we're near a funding rate reset (00:00, 08:00, 16:00 UTC).
    /// Funding resets often cause volatility spikes.
    pub fn near_funding_reset(&self) -> bool {
        let hour = Utc::now().hour();
        // Within 30 min of funding reset
        matches!(hour, 0 | 7 | 8 | 15 | 16)
    }
}

/// Detect current crypto trading session based on UTC time.
pub fn current_session() -> Session {
    let now = Utc::now();
    let hour = now.hour();
    let weekday = now.weekday();

    use chrono::Weekday;
    if weekday == Weekday::Sat || weekday == Weekday::Sun {
        return Session::Weekend;
    }

    match hour {
        0..=7 => Session::Asian,
        8..=13 => Session::European,
        14..=21 => Session::UsSession,
        _ => Session::LateUs,
    }
}

/// Get session info as a formatted string for AI context.
pub fn session_context() -> String {
    let session = current_session();
    let funding_note = if session.near_funding_reset() {
        " | NEAR FUNDING RESET — expect volatility"
    } else {
        ""
    };
    format!(
        "Session: {} | Behavior: {} | Size Multiplier: {:.1}x | Kill Zone: {}{}",
        session.name(),
        session.behavior(),
        session.position_size_multiplier(),
        if session.is_kill_zone() { "YES" } else { "NO" },
        funding_note,
    )
}
