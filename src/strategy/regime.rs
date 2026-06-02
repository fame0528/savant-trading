use crate::core::types::{Candle, IndicatorValues, MarketRegime};

pub struct RegimeDetector {
    pub adx_period: usize,
    pub adx_trending: f64,
    pub adx_ranging: f64,
    pub atr_volatility_mult: f64,
}

impl RegimeDetector {
    pub fn new(
        adx_period: usize,
        adx_trending: f64,
        adx_ranging: f64,
        atr_volatility_mult: f64,
    ) -> Self {
        Self {
            adx_period,
            adx_trending,
            adx_ranging,
            atr_volatility_mult,
        }
    }

    pub fn detect(&self, indicators: &IndicatorValues, candles: &[Candle]) -> MarketRegime {
        let atr = match indicators.atr {
            Some(a) => a,
            None => return MarketRegime::Ranging,
        };

        let adx = indicators.adx.unwrap_or(0.0);

        if candles.len() < 20 {
            return MarketRegime::Ranging;
        }

        let recent_atrs: Vec<f64> = candles
            .windows(2)
            .skip(candles.len().saturating_sub(20))
            .map(|w| {
                let c = &w[1];
                let p = &w[0];
                let hl = c.high - c.low;
                let hc = (c.high - p.close).abs();
                let lc = (c.low - p.close).abs();
                hl.max(hc).max(lc)
            })
            .collect();

        let avg_atr = recent_atrs.iter().sum::<f64>() / recent_atrs.len().max(1) as f64;

        if atr > avg_atr * self.atr_volatility_mult {
            return MarketRegime::Volatile;
        }

        if adx >= self.adx_trending {
            MarketRegime::Trending
        } else if adx <= self.adx_ranging {
            MarketRegime::Ranging
        } else {
            MarketRegime::Trending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_indicators(atr: Option<f64>, adx: Option<f64>) -> IndicatorValues {
        IndicatorValues {
            ema_fast: None,
            ema_slow: None,
            rsi: None,
            atr,
            adx,
            vwap: None,
            volume_sma: None,
            garman_klass: None,
        }
    }

    fn make_candles(n: usize, base: f64) -> Vec<Candle> {
        (0..n)
            .map(|i| Candle {
                timestamp: Utc::now() + chrono::Duration::minutes(i as i64),
                open: base + i as f64 * 0.1,
                high: base + i as f64 * 0.1 + 0.5,
                low: base + i as f64 * 0.1 - 0.5,
                close: base + i as f64 * 0.1 + 0.2,
                volume: 100.0,
                pair: "BTC/USD".to_string(),
            })
            .collect()
    }

    #[test]
    fn detect_trending() {
        let detector = RegimeDetector::new(14, 25.0, 20.0, 1.5);
        let indicators = make_indicators(Some(0.5), Some(30.0));
        let candles = make_candles(25, 50000.0);
        assert_eq!(
            detector.detect(&indicators, &candles),
            MarketRegime::Trending
        );
    }

    #[test]
    fn detect_ranging() {
        let detector = RegimeDetector::new(14, 25.0, 20.0, 1.5);
        let indicators = make_indicators(Some(0.5), Some(15.0));
        let candles = make_candles(25, 50000.0);
        assert_eq!(
            detector.detect(&indicators, &candles),
            MarketRegime::Ranging
        );
    }

    #[test]
    fn detect_no_atr() {
        let detector = RegimeDetector::new(14, 25.0, 20.0, 1.5);
        let indicators = make_indicators(None, Some(30.0));
        let candles = make_candles(25, 50000.0);
        assert_eq!(
            detector.detect(&indicators, &candles),
            MarketRegime::Ranging
        );
    }

    #[test]
    fn detect_insufficient_candles() {
        let detector = RegimeDetector::new(14, 25.0, 20.0, 1.5);
        let indicators = make_indicators(Some(50.0), Some(30.0));
        let candles = make_candles(5, 50000.0);
        assert_eq!(
            detector.detect(&indicators, &candles),
            MarketRegime::Ranging
        );
    }
}
