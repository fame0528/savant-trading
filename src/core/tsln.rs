//! TSLN: Time-Series Lean Notation (FID-085, Phase 2)
//!
//! Schema-first time-series serialization optimized for LLM token efficiency.
//! Replaces JSON OHLC arrays with a compact, lossless format.
//!
//! Format:
//!   # Schema: t:timestamp o:open h:high l:low c:close v:volume
//!   Base: 2026-01-15T10:00:00Z
//!   +0.00 +0.00 +0.00 +0.00 +1234.5
//!   +5.00 +125.50 +0.00 -1.20 +0.00 +567.8
//!
//! Delta-of-delta timestamps: for regular 5-min intervals, the second-order
//! difference is 0 → encoded as a single space or `.` character.
//! Differential prices: encode change from previous close, not absolute values.

use crate::core::types::Candle;

/// TSLN serializer for OHLC candle data.
pub struct TslnSerializer {
    base_timestamp: Option<String>,
    last_close: Option<f64>,
    last_volume: Option<f64>,
}

impl Default for TslnSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl TslnSerializer {
    pub fn new() -> Self {
        Self {
            base_timestamp: None,
            last_close: None,
            last_volume: None,
        }
    }

    /// Reset state for a new serialization session.
    pub fn reset(&mut self) {
        self.base_timestamp = None;
        self.last_close = None;
        self.last_volume = None;
    }

    /// Serialize a slice of candles into TSLN format.
    ///
    /// Output includes schema header + base timestamp + data rows.
    /// For per-cycle use, the schema header and base timestamp are cached
    /// in the Brain and only the data rows are injected into the volatile section.
    /// Use `serialize_data_only()` for that case.
    pub fn serialize(&mut self, candles: &[Candle]) -> String {
        if candles.is_empty() {
            return String::new();
        }

        let mut output = String::new();

        // Schema header (cached in Brain after first use)
        output.push_str(&Self::schema_header());
        output.push('\n');

        // Set base timestamp from first candle
        if self.base_timestamp.is_none() {
            self.base_timestamp = Some(candles[0].timestamp_rfc3339());
        }

        // Base timestamp line
        output.push_str(&self.base_header());
        output.push('\n');

        for candle in candles {
            self.serialize_candle(&mut output, candle);
        }

        output
    }

    /// Serialize only the data rows (no schema header or base timestamp).
    /// Used when schema+base are already cached in the Brain.
    pub fn serialize_data_only(&mut self, candles: &[Candle]) -> String {
        if candles.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        for candle in candles {
            self.serialize_candle(&mut output, candle);
        }
        output
    }

    /// Generate the schema header (cached in Brain, not per-cycle).
    pub fn schema_header() -> String {
        "# Schema: t:timestamp o:open h:high l:low c:close v:volume".to_string()
    }

    /// Generate the base timestamp line (cached in Brain until timeframe changes).
    pub fn base_header(&self) -> String {
        format!(
            "Base: {}",
            self.base_timestamp.as_deref().unwrap_or("unknown")
        )
    }

    fn serialize_candle(&mut self, output: &mut String, candle: &Candle) {
        // Timestamp: delta-of-delta encoding
        // For regular 5-min candles, this is always "+300" seconds
        // We encode as relative offset from base
        let ts_line = if let Some(ref base) = self.base_timestamp {
            let base_secs = crate::core::time::parse_rfc3339_to_secs(base).unwrap_or(0);
            let candle_secs = candle.timestamp_unix();
            let delta = candle_secs - base_secs;
            format!("+{}", delta)
        } else {
            candle.timestamp_rfc3339()
        };

        // OHLCV: differential encoding (change from previous close for O,H,L;
        // absolute for C; delta for V)
        let prev_close = self.last_close.unwrap_or(candle.close);
        let o_diff = candle.open - prev_close;
        let h_diff = candle.high - prev_close;
        let l_diff = candle.low - prev_close;

        let v_str = if let Some(prev_vol) = self.last_volume {
            let v_diff = candle.volume - prev_vol;
            format_diff(v_diff)
        } else {
            format!("{}", candle.volume)
        };

        output.push_str(&format!(
            "{} {} {} {} {} {}\n",
            ts_line,
            format_diff(o_diff),
            format_diff(h_diff),
            format_diff(l_diff),
            candle.close,
            v_str,
        ));

        self.last_close = Some(candle.close);
        self.last_volume = Some(candle.volume);
    }
}

/// Format a differential value: omit if zero, sign prefix otherwise.
fn format_diff(v: f64) -> String {
    if v == 0.0 {
        "+0".to_string()
    } else if v > 0.0 {
        format!("+{}", v)
    } else {
        format!("{}", v)
    }
}

/// Deserialize TSLN back to candles (for round-trip testing).
pub fn deserialize(tsln: &str) -> Vec<Candle> {
    let mut candles = Vec::new();
    let mut base_ts: Option<i64> = None;
    let mut last_close: Option<f64> = None;
    let mut last_volume: f64 = 0.0;
    let mut schema_seen = false;

    for line in tsln.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("# Schema:") {
            schema_seen = true;
            continue;
        }
        if line.starts_with("Base:") {
            let ts_str = line.trim_start_matches("Base:").trim();
            base_ts = crate::core::time::parse_rfc3339_to_secs(ts_str);
            continue;
        }
        if !schema_seen {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        // Parse timestamp
        let ts_secs = if parts[0].starts_with('+') {
            let offset: i64 = parts[0][1..].parse().unwrap_or(0);
            base_ts.unwrap_or(0) + offset
        } else {
            crate::core::time::parse_rfc3339_to_secs(parts[0]).unwrap_or(0)
        };

        // Parse OHLCV
        let close: f64 = parts[4].parse().unwrap_or(0.0);
        let vol: f64 = parts[5].parse().unwrap_or(0.0);

        let (open, high, low) = if let Some(prev) = last_close {
            let o_diff: f64 = parts[1].parse().unwrap_or(0.0);
            let h_diff: f64 = parts[2].parse().unwrap_or(0.0);
            let l_diff: f64 = parts[3].parse().unwrap_or(0.0);
            (prev + o_diff, prev + h_diff, prev + l_diff)
        } else {
            // First candle: OHL are stored as diffs from close, but prev is unknown
            // Reconstruct: assume the candle's open≈close for the very first tick
            // This is an approximation; the exact OHLC is only preserved from candle 2+
            let o_diff: f64 = parts[1].parse().unwrap_or(0.0);
            let h_diff: f64 = parts[2].parse().unwrap_or(0.0);
            let l_diff: f64 = parts[3].parse().unwrap_or(0.0);
            // For first candle, use close as the reference point
            (close + o_diff, close + h_diff, close + l_diff)
        };

        candles.push(Candle {
            timestamp: crate::core::time::secs_to_datetime(ts_secs),
            open,
            high,
            low,
            close,
            volume: if last_close.is_some() {
                last_volume + vol
            } else {
                vol // First candle: absolute volume
            },
            pair: String::new(),
        });

        last_close = Some(close);
        if last_close.is_some() && candles.len() > 1 {
            last_volume += vol;
        } else {
            last_volume = vol;
        }
    }

    candles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candle(ts: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> Candle {
        Candle {
            timestamp: crate::core::time::secs_to_datetime(ts),
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
            pair: "TEST/USD".to_string(),
        }
    }

    #[test]
    fn tsln_round_trip_5_candles() {
        let candles = vec![
            make_candle(1705312800, 2450.0, 2455.0, 2448.0, 2452.0, 1000.0),
            make_candle(1705313100, 2452.0, 2458.0, 2451.0, 2456.0, 1200.0),
            make_candle(1705313400, 2456.0, 2456.0, 2440.0, 2442.0, 1500.0),
            make_candle(1705313700, 2442.0, 2448.0, 2441.0, 2445.0, 900.0),
            make_candle(1705314000, 2445.0, 2450.0, 2444.0, 2449.0, 1100.0),
        ];

        let mut ser = TslnSerializer::new();
        let encoded = ser.serialize(&candles);
        let decoded = deserialize(&encoded);

        assert_eq!(decoded.len(), candles.len());
        for (orig, dec) in candles.iter().zip(decoded.iter()) {
            assert!(
                (orig.close - dec.close).abs() < 0.01,
                "close mismatch: {} vs {}",
                orig.close,
                dec.close
            );
            assert!(
                (orig.high - dec.high).abs() < 0.01,
                "high mismatch: {} vs {}",
                orig.high,
                dec.high
            );
            assert!(
                (orig.low - dec.low).abs() < 0.01,
                "low mismatch: {} vs {}",
                orig.low,
                dec.low
            );
            assert!(
                (orig.volume - dec.volume).abs() < 0.1,
                "volume mismatch: {} vs {}",
                orig.volume,
                dec.volume
            );
        }
    }

    #[test]
    fn tsln_token_savings_vs_json() {
        let mut candles = Vec::new();
        let mut ts = 1705312800i64;
        let mut price = 2450.0f64;
        for _ in 0..100 {
            let c = make_candle(ts, price, price + 5.0, price - 2.0, price + 3.0, 1000.0);
            candles.push(c);
            ts += 300;
            price += 1.0;
        }

        // JSON encoding (current)
        let json_tokens = candles.len() * 6 * 8; // ~6 fields * ~8 tokens each
                                                 // TSLN encoding
        let mut ser = TslnSerializer::new();
        let tsln = ser.serialize(&candles);
        let tsln_chars = tsln.chars().count();
        let tsln_tokens = tsln_chars / 4; // rough estimate

        assert!(
            tsln_tokens < json_tokens / 2,
            "TSLN should use <50% of JSON tokens. JSON: {}, TSLN: {}",
            json_tokens,
            tsln_tokens
        );
    }

    #[test]
    fn schema_header_is_constant() {
        let h = TslnSerializer::schema_header();
        assert_eq!(
            h,
            "# Schema: t:timestamp o:open h:high l:low c:close v:volume"
        );
    }

    // === FID-163: Precision preservation tests ===

    #[test]
    fn tsln_preserves_sub_cent_precision() {
        // Close=0.009123456 has 6 decimal places of meaningful data.
        // With {:.2}, it would round to 0.01 — losing all sub-cent info.
        // With {} Display, it should round-trip exactly.
        let candle = make_candle(
            1705312800,
            0.009123456,
            0.009223456,
            0.009023456,
            0.009123456,
            1000.0,
        );
        let mut ser = TslnSerializer::new();
        let encoded = ser.serialize(std::slice::from_ref(&candle));
        let decoded = deserialize(&encoded);
        assert_eq!(decoded.len(), 1);
        assert!(
            (decoded[0].close - candle.close).abs() < 1e-10,
            "close should round-trip exact: expected {}, got {}",
            candle.close,
            decoded[0].close
        );
    }

    #[test]
    fn tsln_preserves_tiny_diffs() {
        // Candle has close=1.0000001234, but O/H/L differ by 0.0000000001
        // (diff = ±1e-10). Old code with abs<0.001 would collapse these to "+0".
        // New code must preserve them as non-zero strings.
        let candle = make_candle(
            1705312800,
            1.0000001235, // open: +1e-10 from close
            1.0000001236, // high: +2e-10 from close
            1.0000001233, // low: -1e-10 from close
            1.0000001234, // close
            100.0,
        );
        let mut ser = TslnSerializer::new();
        let encoded = ser.serialize(&[candle]);
        // The encoded line should contain non-zero diffs for O/H/L.
        // A collapsed "+0" appears exactly 4 times consecutively for the OHLV slots only
        // when ALL diffs are exactly zero. Here they are tiny, so they should render.
        // Check that we have at least one non-zero diff string.
        let non_zero_count = encoded
            .lines()
            .nth(2) // data row
            .map(|line| {
                line.split_whitespace()
                    .take(4) // O, H, L, V diffs
                    .filter(|tok| *tok != "+0" && *tok != "0")
                    .count()
            })
            .unwrap_or(0);
        assert!(
            non_zero_count >= 1,
            "Tiny diffs should not collapse to +0; encoded was:\n{}",
            encoded
        );
    }

    #[test]
    fn tsln_preserves_volume_precision() {
        let candle = make_candle(1705312800, 1.0, 1.0, 1.0, 1.0, 1234.5678);
        let mut ser = TslnSerializer::new();
        let encoded = ser.serialize(std::slice::from_ref(&candle));
        let decoded = deserialize(&encoded);
        assert!(
            (decoded[0].volume - candle.volume).abs() < 1e-9,
            "volume should round-trip exact: expected {}, got {}",
            candle.volume,
            decoded[0].volume
        );
    }

    #[test]
    fn tsln_preserves_high_price_precision() {
        let candle = make_candle(
            1705312800,
            42123.456789,
            42124.456789,
            42122.456789,
            42123.456789,
            500.0,
        );
        let mut ser = TslnSerializer::new();
        let encoded = ser.serialize(std::slice::from_ref(&candle));
        let decoded = deserialize(&encoded);
        assert!(
            (decoded[0].close - candle.close).abs() < 1e-9,
            "high price should round-trip exact: expected {}, got {}",
            candle.close,
            decoded[0].close
        );
    }

    #[test]
    fn tsln_reset_clears_state() {
        // After reset, base_timestamp/last_close/last_volume should be None.
        let mut ser = TslnSerializer::new();
        let _ = ser.serialize(&[make_candle(1705312800, 1.0, 1.0, 1.0, 1.0, 100.0)]);
        assert!(ser.base_timestamp.is_some());
        assert!(ser.last_close.is_some());
        ser.reset();
        assert!(ser.base_timestamp.is_none());
        assert!(ser.last_close.is_none());
        assert!(ser.last_volume.is_none());
    }

    #[test]
    fn tsln_isolates_state_across_pairs() {
        // FID-163 Part B: the state-bleed bug.
        // Without reset, pair B's first candle's O/H/L diffs would be
        // (pair_B_open - pair_A_close), producing nonsense.
        let pair_a = vec![make_candle(
            1705312800, 50000.0, 50001.0, 49999.0, 50000.0, 100.0,
        )];
        let pair_b = vec![make_candle(1705312800, 0.01, 0.011, 0.009, 0.01, 50.0)];

        let mut ser = TslnSerializer::new();
        let _ = ser.serialize(&pair_a);
        // Reset between pairs (this is what build_tsln_message now does)
        ser.reset();
        let encoded_b = ser.serialize(&pair_b);

        // Pair B's first candle is the first after reset, so O/H/L diffs should be 0
        // (or very small due to floating-point repr of the candle itself).
        // Crucially, they should NOT contain "-49999" (which is what state-bleed would produce).
        assert!(
            !encoded_b.contains("-49999"),
            "Pair B O/H/L diffs bled from pair A close. encoded_b was:\n{}",
            encoded_b
        );
        assert!(
            !encoded_b.contains("-50000"),
            "Pair B O/H/L diffs bled from pair A close. encoded_b was:\n{}",
            encoded_b
        );
    }
}
