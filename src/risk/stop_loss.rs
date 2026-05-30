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
