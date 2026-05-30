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
