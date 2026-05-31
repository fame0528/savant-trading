//! Context tag generator — derives semantic tags from indicators and market state.
//!
//! These tags are injected into FullContext.context_tags and used by the MMR
//! knowledge selector for precise matching.

use crate::core::types::{IndicatorValues, OrderBook, VolumeProfile};

/// Generate context tags from current market state.
pub fn generate_context_tags(
    indicators: &IndicatorValues,
    volume_profile: Option<&VolumeProfile>,
    order_book: Option<&OrderBook>,
    current_price: f64,
) -> Vec<String> {
    let mut tags = Vec::new();

    // RSI-based tags
    if let Some(rsi) = indicators.rsi {
        if rsi < 30.0 {
            tags.push("rsi_oversold".into());
        } else if rsi > 70.0 {
            tags.push("rsi_overbought".into());
        }
        if rsi < 20.0 {
            tags.push("rsi_extreme_oversold".into());
        } else if rsi > 80.0 {
            tags.push("rsi_extreme_overbought".into());
        }
    }

    // EMA relationship tags
    if let (Some(fast), Some(slow)) = (indicators.ema_fast, indicators.ema_slow) {
        if fast > slow {
            tags.push("ema_bullish_cross".into());
        } else {
            tags.push("ema_bearish_cross".into());
        }
    }

    // ADX trend strength
    if let Some(adx) = indicators.adx {
        if adx > 25.0 {
            tags.push("strong_trend".into());
        } else if adx < 20.0 {
            tags.push("weak_trend".into());
        }
    }

    // VWAP relationship
    if let Some(vwap) = indicators.vwap {
        if current_price > vwap {
            tags.push("above_vwap".into());
        } else {
            tags.push("below_vwap".into());
        }
    }

    // Volume tags (using volume_sma as baseline)
    // Volume spike detected if current volume > 2x SMA
    // This is checked from the candle data in the engine, not indicators directly

    // Volume Profile tags
    if let Some(vp) = volume_profile {
        let tolerance = (vp.value_area_high - vp.value_area_low) * 0.02;
        if (current_price - vp.value_area_high).abs() < tolerance {
            tags.push("at_vah".into());
        }
        if (current_price - vp.value_area_low).abs() < tolerance {
            tags.push("at_val".into());
        }
        if (current_price - vp.poc_price).abs() < tolerance {
            tags.push("at_poc".into());
        }
        if current_price > vp.value_area_high {
            tags.push("above_value_area".into());
        }
        if current_price < vp.value_area_low {
            tags.push("below_value_area".into());
        }
    }

    // Order Book Imbalance tags — compute from raw OrderBook
    if let Some(ob) = order_book {
        let bid_vol: f64 = ob.bids.iter().take(5).map(|l| l.volume).sum();
        let ask_vol: f64 = ob.asks.iter().take(5).map(|l| l.volume).sum();
        let total = bid_vol + ask_vol;
        if total > 0.0 {
            let obi = (bid_vol - ask_vol) / total;
            if obi > 0.6 {
                tags.push("strong_buy_imbalance".into());
            } else if obi < -0.6 {
                tags.push("strong_sell_imbalance".into());
            } else if obi > 0.3 {
                tags.push("buy_imbalance".into());
            } else if obi < -0.3 {
                tags.push("sell_imbalance".into());
            }
        }
    }

    tags
}
