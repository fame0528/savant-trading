//! Context builder — aggregates all data sources into a structured prompt for the LLM.
//!
//! Combines candles, indicators, market insight, positions, account state,
//! and selected knowledge units into the system prompt and user message.

use crate::agent::knowledge::{KnowledgeBase, MarketCondition};
use crate::agent::prompts::PromptComposer;
use crate::core::session;
use crate::core::types::{
    AccountState, Candle, IndicatorValues, MarketRegime, Position, TradeRecord, VolumeProfile,
};
use crate::insight::rss;
use crate::insight::MarketContext;

/// Full context for a single AI evaluation tick.
pub struct FullContext<'a> {
    pub candles: &'a [Candle],
    pub indicators: &'a IndicatorValues,
    pub regime: MarketRegime,
    pub volume_profile: Option<&'a VolumeProfile>,
    pub market_context: &'a MarketContext,
    pub positions: &'a [Position],
    pub account: &'a AccountState,
    pub pair: &'a str,
    pub recent_trades: Option<&'a [TradeRecord]>,
    pub order_book_imbalance: Option<f64>,
    pub session: crate::core::session::Session,
    pub memory_context: Option<String>,
    pub higher_tf_candles: Vec<(String, Vec<Candle>)>,
    /// Tags derived from current market context for precise knowledge matching
    pub context_tags: Vec<String>,
}

/// Build the system prompt and user message for the LLM.
///
/// Returns (system_prompt, user_message).
pub fn build_context(
    ctx: &FullContext,
    knowledge_base: &KnowledgeBase,
    composer: &PromptComposer,
    token_budget: usize,
) -> (String, String) {
    // 1. Determine current market conditions
    let conditions = determine_conditions(ctx);

    // 2. Select relevant knowledge units
    let knowledge_units =
        knowledge_base.select_with_tags(&conditions, &ctx.context_tags, token_budget);

    // 3. Compose system prompt with selected knowledge
    let system_prompt = composer.compose(&knowledge_units);

    // 4. Build user message with current market data
    let user_message = build_user_message_static(ctx);

    (system_prompt, user_message)
}

/// Determine current market conditions from context.
fn determine_conditions(ctx: &FullContext) -> Vec<MarketCondition> {
    let mut conditions = determine_conditions_static(
        ctx.regime,
        ctx.market_context.sentiment.fear_greed_index,
        ctx.market_context.funding.funding_rate,
    );

    // M12: Merge on-chain conditions
    let onchain_conditions =
        crate::insight::onchain::derive_conditions(&ctx.market_context.onchain);
    conditions.extend(onchain_conditions);

    // Indicator-derived conditions — map RSI/ADX/EMA/volume to knowledge conditions
    conditions.extend(derive_indicator_conditions(ctx.indicators, ctx.candles));

    conditions
}

/// Derive market conditions from indicator values.
///
/// Maps RSI, ADX, EMA, and volume readings to knowledge conditions so the
/// MMR selector can match units that are relevant to the current chart state.
fn derive_indicator_conditions(
    indicators: &IndicatorValues,
    candles: &[Candle],
) -> Vec<MarketCondition> {
    let mut conditions = Vec::new();

    // RSI extremes
    if let Some(rsi) = indicators.rsi {
        if rsi < 30.0 {
            conditions.push(MarketCondition::Oversold);
        } else if rsi > 70.0 {
            conditions.push(MarketCondition::Overbought);
        }
    }

    // ADX trend strength
    if let Some(adx) = indicators.adx {
        if adx > 25.0 {
            conditions.push(MarketCondition::StrongTrend);
        } else if adx < 20.0 {
            conditions.push(MarketCondition::WeakTrend);
        }
    }

    // EMA alignment
    if let (Some(fast), Some(slow)) = (indicators.ema_fast, indicators.ema_slow) {
        if fast > slow {
            conditions.push(MarketCondition::TrendAlignment);
        }
    }

    // Volume expansion (current volume > 1.5x 20-period SMA)
    if let Some(last) = candles.last() {
        if candles.len() >= 20 {
            let vol_sma: f64 = candles[candles.len() - 20..]
                .iter()
                .map(|c| c.volume)
                .sum::<f64>()
                / 20.0;
            if vol_sma > 0.0 && last.volume > vol_sma * 1.5 {
                conditions.push(MarketCondition::VolumeExpansion);
            }
        }
    }

    conditions
}

/// Generate context tags from indicator values for knowledge matching.
///
/// Tags use the SAME prefixed format as knowledge unit tags (`namespace:value`)
/// so that `select_with_tags()` score matching actually fires. Only tags that
/// exist in the knowledge vocabulary are generated — verified against the
/// actual tag distribution in the 2,959 knowledge units.
pub fn generate_context_tags(indicators: &IndicatorValues) -> Vec<String> {
    let mut tags = Vec::new();

    // Regime subtype from RSI — maps to existing regime_subtype tags
    if let Some(rsi) = indicators.rsi {
        if rsi < 30.0 {
            tags.push("regime_subtype:capitulation".to_string());
            tags.push("setup_type:reversal".to_string());
            tags.push("trigger:confirmation".to_string());
        } else if rsi > 70.0 {
            tags.push("regime_subtype:extreme_greed".to_string());
            tags.push("setup_type:distribution".to_string());
            tags.push("trigger:confirmation".to_string());
        } else if rsi < 40.0 {
            tags.push("regime_subtype:ranging".to_string());
            tags.push("setup_type:analysis".to_string());
        } else if rsi > 60.0 {
            tags.push("regime_subtype:trending".to_string());
            tags.push("setup_type:analysis".to_string());
        }
    }

    // Regime from ADX — maps to existing regime tags
    if let Some(adx) = indicators.adx {
        if adx > 25.0 {
            tags.push("regime:trending".to_string());
            tags.push("regime_subtype:trending".to_string());
            tags.push("trigger:confirmation".to_string());
        } else if adx < 20.0 {
            tags.push("regime:ranging".to_string());
            tags.push("regime_subtype:ranging".to_string());
            tags.push("regime_subtype:neutral".to_string());
        }
    }

    // Setup type from EMA alignment — maps to existing setup_type tags
    if let (Some(fast), Some(slow)) = (indicators.ema_fast, indicators.ema_slow) {
        let spread_pct = ((fast - slow) / slow).abs() * 100.0;
        if fast > slow {
            tags.push("setup_type:breakout".to_string());
            tags.push("regime_subtype:trending".to_string());
        } else {
            tags.push("setup_type:reversal".to_string());
        }
        if spread_pct > 0.5 {
            tags.push("trigger:confirmation".to_string());
        }
    }

    // Deduplicate (some branches push the same tag)
    tags.sort();
    tags.dedup();
    tags
}

pub fn determine_conditions_static(
    regime: MarketRegime,
    fear_greed: Option<u32>,
    funding_rate: Option<f64>,
) -> Vec<MarketCondition> {
    let mut conditions = Vec::new();

    match regime {
        MarketRegime::Trending => conditions.push(MarketCondition::Trending),
        MarketRegime::Ranging => conditions.push(MarketCondition::Ranging),
        MarketRegime::Volatile => conditions.push(MarketCondition::HighVolatility),
    }

    if let Some(fg) = fear_greed {
        if fg < 25 {
            conditions.push(MarketCondition::ExtremeFear);
        } else if fg > 75 {
            conditions.push(MarketCondition::ExtremeGreed);
        }
    }

    if let Some(fr) = funding_rate {
        if fr.abs() > 0.0005 {
            conditions.push(MarketCondition::FundingRateExtreme);
        }
    }

    conditions
}

/// Build the user message containing current market data.
pub fn build_user_message_static(ctx: &FullContext) -> String {
    let mut msg = String::new();

    msg.push_str(&format!("## Current Market Data — {}\n\n", ctx.pair));

    // Latest candle
    if let Some(last) = ctx.candles.last() {
        msg.push_str(&format!(
            "Latest Candle: O={:.2} H={:.2} L={:.2} C={:.2} V={:.2}\n",
            last.open, last.high, last.low, last.close, last.volume
        ));
        msg.push_str(
            "Note: Latest candle may still be forming. Use 20-period volume SMA for volume analysis, not latest candle volume alone.\n",
        );
    }

    // Multi-timeframe candles (SPRINT-1)
    for (tf, tf_candles) in &ctx.higher_tf_candles {
        if tf_candles.is_empty() {
            continue;
        }
        msg.push_str(&format!("\n### {} Timeframe ({})\n", tf, ctx.pair));
        if let Some(last) = tf_candles.last() {
            msg.push_str(&format!(
                "Latest {} Candle: O={:.2} H={:.2} L={:.2} C={:.2} V={:.2}\n",
                tf, last.open, last.high, last.low, last.close, last.volume
            ));
        }
        let len = tf_candles.len();
        if len >= 20 {
            let recent20 = &tf_candles[len - 20..];
            let avg_vol: f64 = recent20.iter().map(|c| c.volume).sum::<f64>() / 20.0;
            let high20 = recent20
                .iter()
                .map(|c| c.high)
                .fold(f64::NEG_INFINITY, f64::max);
            let low20 = recent20.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
            msg.push_str(&format!(
                "{} 20-bar range: {:.2}–{:.2} | Avg Vol: {:.2}\n",
                tf, low20, high20, avg_vol
            ));
        }
    }

    // Indicators
    let gk_str = ctx
        .indicators
        .garman_klass
        .map(|g| format!("{:.2}", g))
        .unwrap_or_else(|| "N/A".to_string());
    msg.push_str(&format!(
        "Indicators: EMA_FAST={:?} EMA_SLOW={:?} RSI={:?} ATR={:?} ADX={:?} VWAP={:?} GarmanKlass={}\n",
        ctx.indicators.ema_fast,
        ctx.indicators.ema_slow,
        ctx.indicators.rsi,
        ctx.indicators.atr,
        ctx.indicators.adx,
        ctx.indicators.vwap,
        gk_str,
    ));

    // Regime
    msg.push_str(&format!("Regime: {:?}\n", ctx.regime));

    // Session
    msg.push_str(&format!("{}\n", session::session_context()));

    // Volume profile
    if let Some(vp) = ctx.volume_profile {
        msg.push_str(&format!(
            "Volume Profile: POC={:.2} VAH={:.2} VAL={:.2}\n",
            vp.poc_price, vp.value_area_high, vp.value_area_low
        ));
    }

    // Order book imbalance (CRIT-4)
    if let Some(imbalance) = ctx.order_book_imbalance {
        let pressure = if imbalance > 0.3 {
            "bid-heavy — buying pressure"
        } else if imbalance < -0.3 {
            "ask-heavy — selling pressure"
        } else {
            "balanced"
        };
        msg.push_str(&format!(
            "Order Book Imbalance: {:+.2} ({})\n",
            imbalance, pressure
        ));
    }

    // Session info (CRIT-5)
    msg.push_str(&format!(
        "Current Session: {} — {}\n",
        ctx.session.name(),
        ctx.session.behavior()
    ));
    if ctx.session.is_kill_zone() {
        msg.push_str("STATUS: KILL ZONE ACTIVE — high probability window\n");
    }

    // Market context
    msg.push_str(&format!(
        "\n## Market Insight\n{}\n",
        ctx.market_context.summary()
    ));

    // On-chain analytics
    let oc = &ctx.market_context.onchain;
    if oc.mvrv.is_some() || oc.sopr.is_some() || oc.nvt_signal.is_some() {
        msg.push_str("\n## On-Chain Analytics\n");
        if let Some(mvrv) = oc.mvrv {
            let state = if mvrv > 3.5 {
                "EUPHORIA (sell signal)"
            } else if mvrv > 2.0 {
                "Warming up"
            } else if mvrv > 1.0 {
                "Neutral/Undervalued"
            } else {
                "CAPITULATION (strong buy)"
            };
            msg.push_str(&format!("MVRV: {:.2} — {}\n", mvrv, state));
        }
        if let Some(sopr) = oc.sopr {
            let state = if sopr > 1.0 {
                "Profit realization"
            } else {
                "Loss realization (capitulation)"
            };
            msg.push_str(&format!("SOPR: {:.4} — {}\n", sopr, state));
        }
        if let Some(nvt) = oc.nvt_signal {
            msg.push_str(&format!("NVT Signal: {:.2}\n", nvt));
        }
    }

    // RSS news (top 5 relevant items)
    if !ctx.market_context.rss_items.is_empty() {
        msg.push_str(&format!(
            "\n## Recent News\n{}\n",
            rss::format_for_context(&ctx.market_context.rss_items, 5)
        ));
    }

    // Open positions
    if !ctx.positions.is_empty() {
        msg.push_str("\n## Open Positions\n");
        for pos in ctx.positions {
            msg.push_str(&format!(
                "- {} {} @ {:.2} | SL: {:.2} | TP1: {:.2} | PnL: {:.2}\n",
                pos.pair,
                pos.side,
                pos.entry_price,
                pos.stop_loss,
                pos.take_profit_1,
                pos.unrealized_pnl
            ));
        }
    }

    // Account state
    msg.push_str(&format!(
        "\n## Account\nBalance: ${:.2} | Equity: ${:.2} | DD: {:.1}% | Open: {}\n",
        ctx.account.balance,
        ctx.account.equity,
        ctx.account.drawdown_pct * 100.0,
        ctx.account.open_positions
    ));

    // Trade history (if available from journal)
    if let Some(trades) = ctx.recent_trades {
        if !trades.is_empty() {
            msg.push_str("\n## Recent Trade History\n");
            let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
            let losses = trades.iter().filter(|t| t.pnl <= 0.0).count();
            let avg_win = if wins > 0 {
                trades
                    .iter()
                    .filter(|t| t.pnl > 0.0)
                    .map(|t| t.pnl)
                    .sum::<f64>()
                    / wins as f64
            } else {
                0.0
            };
            let avg_loss = if losses > 0 {
                trades
                    .iter()
                    .filter(|t| t.pnl <= 0.0)
                    .map(|t| t.pnl)
                    .sum::<f64>()
                    / losses as f64
            } else {
                0.0
            };
            let profit_factor = if avg_loss != 0.0 {
                (avg_win * wins as f64) / (avg_loss.abs() * losses as f64)
            } else {
                f64::INFINITY
            };

            for (i, trade) in trades.iter().take(10).enumerate() {
                msg.push_str(&format!(
                    "{}. {} {} @ {:.2} → {:.2} | PnL: ${:.2} ({:.1}%) | {}\n",
                    i + 1,
                    trade.pair,
                    if trade.pnl > 0.0 { "WIN" } else { "LOSS" },
                    trade.entry_price,
                    trade.exit_price,
                    trade.pnl,
                    trade.pnl_pct,
                    trade.closed_at.format("%Y-%m-%d")
                ));
            }
            msg.push_str(&format!(
                "\nSummary: {}W/{}L ({:.0}% WR) | Avg Win: ${:.2} | Avg Loss: ${:.2} | PF: {:.2}\n",
                wins,
                losses,
                if wins + losses > 0 {
                    wins as f64 / (wins + losses) as f64 * 100.0
                } else {
                    0.0
                },
                avg_win,
                avg_loss,
                profit_factor
            ));
        }
    }

    // Memory context (6th prompt layer — WIRE-1)
    if let Some(ref memory) = ctx.memory_context {
        if !memory.is_empty() {
            msg.push_str(memory);
        }
    }

    msg.push_str("\n## Decision Required\n");
    msg.push_str(
        "Analyze the above data and provide your trade decision in the specified JSON format.\n",
    );

    msg
}
