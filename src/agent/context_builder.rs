//! Context builder — aggregates all data sources into a structured prompt for the LLM.
//!
//! Combines candles, indicators, market insight, positions, account state,
//! and selected knowledge units into the system prompt and user message.

use crate::agent::knowledge::{KnowledgeBase, MarketCondition};
use crate::agent::prompts::PromptComposer;
use crate::core::types::{
    AccountState, Candle, IndicatorValues, MarketRegime, Position, VolumeProfile,
};
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
    let knowledge_units = knowledge_base.select(&conditions, token_budget);

    // 3. Compose system prompt with selected knowledge
    let system_prompt = composer.compose(&knowledge_units);

    // 4. Build user message with current market data
    let user_message = build_user_message_static(ctx);

    (system_prompt, user_message)
}

/// Determine current market conditions from context.
fn determine_conditions(ctx: &FullContext) -> Vec<MarketCondition> {
    determine_conditions_static(
        ctx.regime,
        ctx.market_context.sentiment.fear_greed_index,
        ctx.market_context.funding.funding_rate,
    )
}

/// Static version of determine_conditions for use outside FullContext.
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
        if fr.abs() > 0.01 {
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
    }

    // Indicators
    msg.push_str(&format!(
        "Indicators: EMA_FAST={:?} EMA_SLOW={:?} RSI={:?} ATR={:?} ADX={:?} VWAP={:?}\n",
        ctx.indicators.ema_fast,
        ctx.indicators.ema_slow,
        ctx.indicators.rsi,
        ctx.indicators.atr,
        ctx.indicators.adx,
        ctx.indicators.vwap,
    ));

    // Regime
    msg.push_str(&format!("Regime: {:?}\n", ctx.regime));

    // Volume profile
    if let Some(vp) = ctx.volume_profile {
        msg.push_str(&format!(
            "Volume Profile: POC={:.2} VAH={:.2} VAL={:.2}\n",
            vp.poc_price, vp.value_area_high, vp.value_area_low
        ));
    }

    // Market context
    msg.push_str(&format!(
        "\n## Market Insight\n{}\n",
        ctx.market_context.summary()
    ));

    // Open positions
    if !ctx.positions.is_empty() {
        msg.push_str("\n## Open Positions\n");
        for pos in ctx.positions {
            msg.push_str(&format!(
                "- {} {} @ {:.2} | SL: {:.2} | TP1: {:.2} | PnL: {:.2}\n",
                pos.pair,
                pos.side as u8,
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

    msg.push_str("\n## Decision Required\n");
    msg.push_str(
        "Analyze the above data and provide your trade decision in the specified JSON format.\n",
    );

    msg
}
