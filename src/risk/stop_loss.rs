use crate::core::types::Candle;

pub struct StopLossCalculator;

impl StopLossCalculator {
    pub fn atr_stop(entry: f64, atr: f64, multiplier: f64, is_long: bool) -> f64 {
        if is_long {
            entry - atr * multiplier
        } else {
            entry + atr * multiplier
        }
    }

    pub fn structure_stop(candles: &[Candle], is_long: bool, lookback: usize) -> Option<f64> {
        if candles.len() < lookback {
            return None;
        }
        let recent = &candles[candles.len() - lookback..];
        if is_long {
            Some(recent.iter().map(|c| c.low).fold(f64::INFINITY, f64::min))
        } else {
            Some(
                recent
                    .iter()
                    .map(|c| c.high)
                    .fold(f64::NEG_INFINITY, f64::max),
            )
        }
    }

    pub fn break_even_trigger(
        entry: f64,
        stop_loss: f64,
        current_price: f64,
        is_long: bool,
    ) -> bool {
        let risk = if is_long {
            entry - stop_loss
        } else {
            stop_loss - entry
        };
        if risk <= 0.0 {
            return false;
        }
        let profit = if is_long {
            current_price - entry
        } else {
            entry - current_price
        };
        profit >= risk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn atr_stop_long() {
        let stop = StopLossCalculator::atr_stop(100.0, 2.0, 1.5, true);
        assert_eq!(stop, 97.0);
    }

    #[test]
    fn atr_stop_short() {
        let stop = StopLossCalculator::atr_stop(100.0, 2.0, 1.5, false);
        assert_eq!(stop, 103.0);
    }

    #[test]
    fn structure_stop_long() {
        let candles = vec![
            Candle {
                timestamp: Utc::now(),
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 102.0,
                volume: 100.0,
                pair: "BTC/USD".to_string(),
            },
            Candle {
                timestamp: Utc::now(),
                open: 102.0,
                high: 106.0,
                low: 97.0,
                close: 104.0,
                volume: 100.0,
                pair: "BTC/USD".to_string(),
            },
            Candle {
                timestamp: Utc::now(),
                open: 104.0,
                high: 108.0,
                low: 99.0,
                close: 103.0,
                volume: 100.0,
                pair: "BTC/USD".to_string(),
            },
        ];
        let stop = StopLossCalculator::structure_stop(&candles, true, 3);
        assert_eq!(stop, Some(95.0));
    }

    #[test]
    fn structure_stop_short() {
        let candles = vec![
            Candle {
                timestamp: Utc::now(),
                open: 100.0,
                high: 105.0,
                low: 95.0,
                close: 102.0,
                volume: 100.0,
                pair: "BTC/USD".to_string(),
            },
            Candle {
                timestamp: Utc::now(),
                open: 102.0,
                high: 106.0,
                low: 97.0,
                close: 104.0,
                volume: 100.0,
                pair: "BTC/USD".to_string(),
            },
        ];
        let stop = StopLossCalculator::structure_stop(&candles, false, 2);
        assert_eq!(stop, Some(106.0));
    }

    #[test]
    fn structure_stop_insufficient_data() {
        let candles = vec![Candle {
            timestamp: Utc::now(),
            open: 100.0,
            high: 105.0,
            low: 95.0,
            close: 102.0,
            volume: 100.0,
            pair: "BTC/USD".to_string(),
        }];
        let stop = StopLossCalculator::structure_stop(&candles, true, 3);
        assert_eq!(stop, None);
    }

    #[test]
    fn break_even_trigger_long_reached() {
        assert!(StopLossCalculator::break_even_trigger(
            100.0, 95.0, 105.0, true
        ));
    }

    #[test]
    fn break_even_trigger_long_not_reached() {
        assert!(!StopLossCalculator::break_even_trigger(
            100.0, 95.0, 103.0, true
        ));
    }

    #[test]
    fn break_even_trigger_short_reached() {
        assert!(StopLossCalculator::break_even_trigger(
            100.0, 105.0, 95.0, false
        ));
    }

    #[test]
    fn break_even_trigger_invalid_risk() {
        assert!(!StopLossCalculator::break_even_trigger(
            100.0, 105.0, 102.0, true
        ));
    }
}
