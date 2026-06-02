//! Session detection — crypto-native 24/7 trading sessions.
//!
//! Crypto never closes. Instead of "off-hours", sessions reflect
//! volatility patterns, liquidity windows, and funding rate resets.
//!
//! Key crypto session patterns (FID-015 updated with UTC liquidity profiles):
//! - Asian (00:00-08:00 UTC): Lower volume, range-bound, good for mean reversion
//!   - 02:00-06:00 UTC: DEEP ASIAN — liquidity trough, 42% less depth, breakouts fail
//! - European (08:00-14:00 UTC): Higher volume, trend continuations
//! - US-EU Overlap (13:00-17:00 UTC): PEAK LIQUIDITY — optimal for momentum/breakouts
//! - US Post-Overlap (17:00-22:00 UTC): Moderate volume, mean reversion increasing
//! - Weekend: Lower volume, often mean-reverting. Sunday 18:00-23:00 watch for gap opens.
//! - Funding Reset (every 8h at 00:00, 08:00, 16:00 UTC): Volatility spike

use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Trading session — crypto-native, no "off-hours".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Session {
    /// 00:00-02:00 UTC — Early Asian, moderate volume
    Asian,
    /// 02:00-06:00 UTC — Deep Asian, liquidity trough, breakouts prone to failure
    DeepAsian,
    /// 06:00-08:00 UTC — Late Asian / pre-European, volume picking up
    LateAsian,
    /// 08:00-13:00 UTC — European session, higher volume, trend continuations
    European,
    /// 13:00-17:00 UTC — US-EU overlap, PEAK LIQUIDITY, optimal for momentum
    UsEuOverlap,
    /// 17:00-22:00 UTC — US post-overlap, volume declining, mean reversion viable
    UsPostOverlap,
    /// 22:00-00:00 UTC — Late US / pre-Asian transition
    LateUs,
    /// Saturday — Lower volume, often range-bound
    Weekend,
    /// Sunday 18:00-23:00 UTC — Pre-Monday, watch for institutional gap opens
    SundayPreOpen,
}

impl Session {
    pub fn name(&self) -> &str {
        match self {
            Session::Asian => "Asian",
            Session::DeepAsian => "Deep Asian",
            Session::LateAsian => "Late Asian",
            Session::European => "European",
            Session::UsEuOverlap => "US-EU Overlap",
            Session::UsPostOverlap => "US Post-Overlap",
            Session::LateUs => "Late US",
            Session::Weekend => "Weekend",
            Session::SundayPreOpen => "Sunday Pre-Open",
        }
    }

    pub fn behavior(&self) -> &str {
        match self {
            Session::Asian => "Moderate volume. Range-bound setups favored.",
            Session::DeepAsian => "LIQUIDITY TROUGH — 42% less order book depth. Breakouts statistically fail. Reduce size or skip.",
            Session::LateAsian => "Volume picking up as European traders come online. Watch for early direction.",
            Session::European => "Higher volume, trend continuations from Asian session. London open often sets daily direction.",
            Session::UsEuOverlap => "PEAK LIQUIDITY — highest global volume, tightest spreads. Optimal for momentum/breakouts. Full position sizing.",
            Session::UsPostOverlap => "Volume declining. Mean reversion increasingly viable as momentum stalls.",
            Session::LateUs => "Volume declining, position squaring. Mean reversion as traders close before Asian open.",
            Session::Weekend => "Lower volume, often mean-reverting. Reduced position size.",
            Session::SundayPreOpen => "Watch for institutional gap opens. Front-running CME futures launch.",
        }
    }

    pub fn position_size_multiplier(&self) -> f64 {
        match self {
            Session::Asian => 0.8,
            Session::DeepAsian => 0.5,
            Session::LateAsian => 0.7,
            Session::European => 1.0,
            Session::UsEuOverlap => 1.2,
            Session::UsPostOverlap => 0.9,
            Session::LateUs => 0.8,
            Session::Weekend => 0.6,
            Session::SundayPreOpen => 0.7,
        }
    }

    /// Confidence penalty for breakout trades during low-liquidity sessions.
    /// Returns a multiplier to apply to the agent's confidence score.
    pub fn breakout_confidence_penalty(&self) -> f64 {
        match self {
            Session::DeepAsian => 0.6,     // 40% penalty — breakouts fail here
            Session::LateAsian => 0.85,    // 15% penalty
            Session::LateUs => 0.85,       // 15% penalty
            Session::Weekend => 0.7,       // 30% penalty
            Session::SundayPreOpen => 0.8, // 20% penalty
            _ => 1.0,                      // No penalty during liquid sessions
        }
    }

    /// All crypto sessions are tradeable — no "off-hours" in 24/7 markets.
    /// But Deep Asian and Weekend require extra caution.
    pub fn is_kill_zone(&self) -> bool {
        matches!(
            self,
            Session::European | Session::UsEuOverlap | Session::UsPostOverlap
        )
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
    if weekday == Weekday::Sat {
        return Session::Weekend;
    }
    if weekday == Weekday::Sun {
        return if hour >= 18 {
            Session::SundayPreOpen
        } else {
            Session::Weekend
        };
    }

    match hour {
        0..=1 => Session::Asian,
        2..=5 => Session::DeepAsian,
        6..=7 => Session::LateAsian,
        8..=12 => Session::European,
        13..=16 => Session::UsEuOverlap,
        17..=21 => Session::UsPostOverlap,
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
    let penalty = session.breakout_confidence_penalty();
    let penalty_note = if penalty < 1.0 {
        format!(" | Breakout confidence penalty: {:.0}%", penalty * 100.0)
    } else {
        String::new()
    };
    format!(
        "Session: {} | Behavior: {} | Size Multiplier: {:.1}x | Kill Zone: {}{}{}",
        session.name(),
        session.behavior(),
        session.position_size_multiplier(),
        if session.is_kill_zone() { "YES" } else { "NO" },
        funding_note,
        penalty_note,
    )
}
