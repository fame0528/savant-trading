//! Time utility functions for TSLN serialization (FID-085).

use chrono::{DateTime, TimeZone, Utc};

/// Parse an RFC 3339 timestamp string to Unix seconds.
pub fn parse_rfc3339_to_secs(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

/// Convert Unix seconds to an RFC 3339 timestamp string.
pub fn secs_to_rfc3339(secs: i64) -> String {
    secs_to_datetime(secs).to_rfc3339()
}

/// Convert Unix seconds to a DateTime<Utc>.
pub fn secs_to_datetime(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trip() {
        let original = "2026-01-15T10:00:00Z";
        let secs = parse_rfc3339_to_secs(original).unwrap();
        let back = secs_to_rfc3339(secs);
        assert!(back.starts_with("2026-01-15T10:00:00"));
    }

    #[test]
    fn secs_to_datetime_round_trip() {
        let secs = 1705312800i64;
        let dt = secs_to_datetime(secs);
        assert_eq!(dt.timestamp(), secs);
    }

    #[test]
    fn parse_invalid_returns_none() {
        assert!(parse_rfc3339_to_secs("not-a-timestamp").is_none());
    }
}
