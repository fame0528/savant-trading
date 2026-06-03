//! 50 curated market scenarios for stress-testing the agent.

use serde::{Deserialize, Serialize};

use crate::core::types::Candle;
use crate::data::historical::HistoricalScenario;
use crate::sandbox::generator::{MarketEvent, ScenarioParams, TrendDirection, VolatilityRegime};
use crate::sandbox::mock::{MockData, MockPresets};

/// Minimum net price change (as fraction) to classify a trend as non-sideways.
/// Below this threshold the market is considered flat / ranging.
const HISTORICAL_TREND_THRESHOLD: f64 = 0.005;

/// Scaling factor applied to the average per-candle return to normalize
/// trend strength into the [0.0, 1.0] range.
const STRENGTH_SCALE_FACTOR: f64 = 10.0;

/// Volatility thresholds (average (high-low)/close ratio):
///   > EXTREME → Extreme, > HIGH → High, > NORMAL → Normal, else Low.
const VOLATILITY_EXTREME_THRESHOLD: f64 = 0.10;
const VOLATILITY_HIGH_THRESHOLD: f64 = 0.03;
const VOLATILITY_NORMAL_THRESHOLD: f64 = 0.01;

/// Minimum net price change (as fraction) to derive bullish/bearish mock sentiment.
/// Moves smaller than this produce neutral mock data.
const MOCK_SENTIMENT_THRESHOLD: f64 = 0.02;

/// A single test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub category: String,
    pub name: String,
    pub difficulty: String,
    pub trigger_condition: String,
    pub expected_action: String,
    pub target_rule: String,
    pub params: ScenarioParams,
    pub mock_data: MockData,
    /// When set, these candles replace the default synthetic candles.
    /// Used for training on real historical market data.
    #[serde(default)]
    pub candles_override: Option<Vec<Candle>>,
}

/// Load all 50 scenarios.
#[allow(clippy::vec_init_then_push)]
pub fn load_all_scenarios() -> Vec<Scenario> {
    let mut scenarios = Vec::with_capacity(50);

    // ── Trend Bull (5) ──────────────────────────────────────
    scenarios.push(Scenario {
        id: "TRD-001".into(),
        category: "Trend Bull".into(),
        name: "Clean Breakout".into(),
        difficulty: "Easy".into(),
        trigger_condition: "Price breaks major resistance; expanding volume; ADX > 25".into(),
        expected_action: "Buy (High Conviction)".into(),
        target_rule: "Target set (R/R >= 1.5:1)".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.8),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 65,
            fear_greed_label: "Greed".into(),
            funding_rate: 0.0005,
            mvrv: 2.0,
            news_headlines: vec![
                "Bitcoin smashes through $70K resistance on record spot volume".into(),
                "BlackRock Bitcoin ETF sees $500M daily inflow as breakout holds".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-002".into(),
        category: "Trend Bull".into(),
        name: "Parabolic Exhaustion".into(),
        difficulty: "Medium".into(),
        trigger_condition: "RSI > 85; price 3 standard deviations above EMA(21)".into(),
        expected_action: "Hold / Take Profit".into(),
        target_rule: "Never chase entries".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(1.0),
            volatility_regime: VolatilityRegime::Extreme,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 90,
            fear_greed_label: "Extreme Greed".into(),
            funding_rate: 0.0015,
            mvrv: 3.8,
            sopr: 1.08,
            news_headlines: vec![
                "Bitcoin funding rates hit annual high as leverage skyrockets".into(),
                "Leverage traders pile in: open interest reaches all-time high".into(),
                "Veteran traders warn: 'This feels like the top'".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-003".into(),
        category: "Trend Bull".into(),
        name: "EMA Pullback".into(),
        difficulty: "Easy".into(),
        trigger_condition: "Price retraces to EMA(21) confluence during confirmed uptrend".into(),
        expected_action: "Buy (Medium Conviction)".into(),
        target_rule: "Entry price/zone logic".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.5),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 55,
            fear_greed_label: "Neutral".into(),
            funding_rate: 0.0003,
            mvrv: 1.8,
            news_headlines: vec![
                "Bitcoin pulls back 5% to test key moving average support".into(),
                "Analysts call dip a 'healthy correction' in ongoing uptrend".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-004".into(),
        category: "Trend Bull".into(),
        name: "False Breakout".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Price breaks resistance, immediately reverses on high volume".into(),
        expected_action: "Hold".into(),
        target_rule: "Invalidation level defined".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.3),
            volatility_regime: VolatilityRegime::High,
            event: Some(MarketEvent::GapDown {
                candle_index: 600,
                gap_pct: 3.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 40,
            fear_greed_label: "Fear".into(),
            funding_rate: 0.0002,
            news_headlines: vec![
                "Bitcoin briefly breaks $70K resistance before sharp reversal".into(),
                "Bull trap fears grow as breakout fails on high volume rejection".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-005".into(),
        category: "Trend Bull".into(),
        name: "Slow Grind".into(),
        difficulty: "Medium".into(),
        trigger_condition: "ADX > 20 but low ATR; steady upward trajectory".into(),
        expected_action: "Buy (Low Conviction)".into(),
        target_rule: "Sizing within protocol".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.2),
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 50,
            fear_greed_label: "Neutral".into(),
            funding_rate: 0.0001,
            mvrv: 1.5,
            news_headlines: vec![
                "Bitcoin steadily climbs in low-volatility accumulation phase".into(),
                "On-chain data shows consistent whale buying in tight range".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Trend Bear (5) ──────────────────────────────────────
    scenarios.push(Scenario {
        id: "TRD-006".into(),
        category: "Trend Bear".into(),
        name: "Support Breakdown".into(),
        difficulty: "Easy".into(),
        trigger_condition: "Price falls through major support; ADX > 25".into(),
        expected_action: "Short (High Conviction)".into(),
        target_rule: "Target set (R/R >= 1.5:1)".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.8),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 10,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.0005,
            mvrv: 0.8,
            sopr: 0.95,
            news_headlines: vec![
                "Bitcoin breaks below $60K support level with heavy volume".into(),
                "Bearish momentum accelerates as key technical levels fail".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-007".into(),
        category: "Trend Bear".into(),
        name: "Capitulation Wick".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Massive downward wick; MVRV < 1.0; RSI < 15".into(),
        expected_action: "Hold / Cover Shorts".into(),
        target_rule: "Regime flag: capitulation".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.5),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 300,
                magnitude_pct: 20.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 5,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.002,
            mvrv: 0.6,
            sopr: 0.88,
            open_interest: 200.0,
            news_headlines: vec![
                "Bitcoin flash crashes 20% in minutes amid liquidation cascade".into(),
                "Market in full capitulation: $2B in longs liquidated".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-008".into(),
        category: "Trend Bear".into(),
        name: "Bear Flag Breakdown".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Consolidation after drop, followed by downward expansion".into(),
        expected_action: "Short (Medium Conviction)".into(),
        target_rule: "Thesis stated (2 sentences)".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.6),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 30,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.0003,
            news_headlines: vec![
                "Bitcoin breaks down from bear flag pattern, sellers in control".into(),
                "Technical analysts warn of further downside after pattern failure".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-009".into(),
        category: "Trend Bear".into(),
        name: "Dead Cat Bounce".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Sharp 5% rally in strong downtrend; EMA(21) acts as resistance".into(),
        expected_action: "Short (Low Conviction)".into(),
        target_rule: "Regime classified".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.4),
            volatility_regime: VolatilityRegime::High,
            event: Some(MarketEvent::ShortSqueeze {
                candle_index: 400,
                magnitude_pct: 5.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 25,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.001,
            news_headlines: vec![
                "Bitcoin rallies 5% but faces heavy resistance at EMA".into(),
                "Traders skeptical of relief rally in strong downtrend".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-010".into(),
        category: "Trend Bear".into(),
        name: "Slow Bleed".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Persistent lower highs; low volume; negative funding".into(),
        expected_action: "Short (Medium Conviction)".into(),
        target_rule: "Sizing within protocol".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.3),
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 35,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.0002,
            news_headlines: vec![
                "Bitcoin grinds lower on declining volume, sellers dominate".into(),
                "Persistent selling pressure erodes market confidence".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Range Bound (5) ─────────────────────────────────────
    scenarios.push(Scenario {
        id: "RNG-001".into(),
        category: "Range Bound".into(),
        name: "Mid-Range Chop".into(),
        difficulty: "Medium".into(),
        trigger_condition: "ADX < 20; price oscillating wildly around VWAP".into(),
        expected_action: "Hold".into(),
        target_rule: "Regime classified (Ranging)".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 50,
            fear_greed_label: "Neutral".into(),
            funding_rate: 0.0001,
            news_headlines: vec![
                "Bitcoin trades sideways as market awaits next catalyst".into(),
                "Low-volume chop frustrates traders in tight range".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "RNG-002".into(),
        category: "Range Bound".into(),
        name: "Support Test".into(),
        difficulty: "Easy".into(),
        trigger_condition: "Price touches bottom of established 30-day range".into(),
        expected_action: "Buy (Low Conviction)".into(),
        target_rule: "Invalidation level defined".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 40,
            fear_greed_label: "Fear".into(),
            funding_rate: 0.0001,
            news_headlines: vec![
                "Bitcoin approaches key support level for third time this month".into(),
                "Buyers defend critical support zone as range holds".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "RNG-003".into(),
        category: "Range Bound".into(),
        name: "Resistance Rejection".into(),
        difficulty: "Easy".into(),
        trigger_condition: "Price touches top of established range with bearish divergence".into(),
        expected_action: "Short (Low Conviction)".into(),
        target_rule: "Stop loss set".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 60,
            fear_greed_label: "Greed".into(),
            funding_rate: 0.0003,
            news_headlines: vec![
                "Bitcoin rejected at range resistance, bearish divergence forms".into(),
                "Sell orders stack up as BTC approaches upper boundary".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "RNG-004".into(),
        category: "Range Bound".into(),
        name: "Volatility Compression".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Bollinger Bands tightening to historical minimums".into(),
        expected_action: "Hold".into(),
        target_rule: "Thesis stated".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 50,
            fear_greed_label: "Neutral".into(),
            funding_rate: 0.0001,
            news_headlines: vec![
                "Bitcoin Bollinger Bands narrow to tightest level in 6 months".into(),
                "Traders brace for volatility squeeze as range compresses".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "RNG-005".into(),
        category: "Range Bound".into(),
        name: "Fakeout Expansion".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Price breaks range support, then immediately breaks range resistance"
            .into(),
        expected_action: "Hold".into(),
        target_rule: "Never revenge trade".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::High,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 300,
                magnitude_pct: 5.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 55,
            fear_greed_label: "Neutral".into(),
            funding_rate: 0.0002,
            news_headlines: vec![
                "Bitcoin briefly breaks range support before sharp V-reversal".into(),
                "Stop hunts on both sides as range fakeout traps traders".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Volatility (5) ──────────────────────────────────────
    scenarios.push(Scenario {
        id: "VOL-001".into(),
        category: "Volatility".into(),
        name: "Flash Crash Recovery".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Instantaneous 15% drop, immediate stabilization".into(),
        expected_action: "Hold".into(),
        target_rule: "Catalyst risk check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 360,
                magnitude_pct: 15.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 5,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.002,
            mvrv: 0.6,
            sopr: 0.88,
            open_interest: 200.0,
            news_headlines: vec![
                "Bitcoin plunges 15% in seconds before sharp V-shaped recovery".into(),
                "Flash crash triggers $1.5B in liquidations across exchanges".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "VOL-002".into(),
        category: "Volatility".into(),
        name: "Short Squeeze".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Rapid 10% upward spike; highly negative funding rates".into(),
        expected_action: "Hold / Take Profit".into(),
        target_rule: "Funding > 0.05% overleveraged".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.5),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::ShortSqueeze {
                candle_index: 400,
                magnitude_pct: 10.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 70,
            fear_greed_label: "Greed".into(),
            funding_rate: 0.0012,
            open_interest: 5000.0,
            news_headlines: vec![
                "Bitcoin surges 10% as massive short squeeze liquidates bears".into(),
                "Negative funding rates fuel violent upward price spike".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "VOL-003".into(),
        category: "Volatility".into(),
        name: "Erratic ATR Expansion".into(),
        difficulty: "Hard".into(),
        trigger_condition: "ATR jumps 3x average; no clear directional trend".into(),
        expected_action: "Hold".into(),
        target_rule: "Regime flag: Volatile".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Extreme,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 45,
            fear_greed_label: "Fear".into(),
            funding_rate: 0.0005,
            news_headlines: vec![
                "Bitcoin whipsaws 8% in both directions as volatility explodes".into(),
                "ATR triples as market enters erratic, directionless phase".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "VOL-004".into(),
        category: "Volatility".into(),
        name: "Liquidation Cascade".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Sequential large market sells trigger massive slippage in LOB".into(),
        expected_action: "Hold".into(),
        target_rule: "Never catch a falling knife".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.5),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 350,
                magnitude_pct: 25.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 5,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.002,
            mvrv: 0.6,
            sopr: 0.88,
            open_interest: 200.0,
            news_headlines: vec![
                "Cascading liquidations send Bitcoin into freefall, -25% in hours".into(),
                "Exchange order books overwhelmed as forced selling snowballs".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "VOL-005".into(),
        category: "Volatility".into(),
        name: "News-Driven Spike".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Sudden 5% move directly following an RSS news injection".into(),
        expected_action: "Hold until structure forms".into(),
        target_rule: "Catalyst risk check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.3),
            volatility_regime: VolatilityRegime::High,
            event: Some(MarketEvent::ShortSqueeze {
                candle_index: 500,
                magnitude_pct: 5.0,
            }),
        },
        mock_data: MockPresets::etf_approval(),
        candles_override: None,
    });

    // ── Catalyst (5) ────────────────────────────────────────
    scenarios.push(Scenario {
        id: "CAT-001".into(),
        category: "Catalyst".into(),
        name: "FOMC Rate Hike".into(),
        difficulty: "Hard".into(),
        trigger_condition: "RSS indicates unexpected rate hike; high volatility injected".into(),
        expected_action: "Hold / Close positions".into(),
        target_rule: "Catalyst risk check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.4),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockPresets::fomc_rate_hike(),
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "CAT-002".into(),
        category: "Catalyst".into(),
        name: "ETF Approval".into(),
        difficulty: "Medium".into(),
        trigger_condition: "RSS indicates structural positive news; price grinding up".into(),
        expected_action: "Buy (Medium Conviction)".into(),
        target_rule: "Thesis stated citing news".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.6),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockPresets::etf_approval(),
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "CAT-003".into(),
        category: "Catalyst".into(),
        name: "Exchange Hack".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "RSS indicates major exchange breached; price drops 8%".into(),
        expected_action: "Short / Close Longs".into(),
        target_rule: "Catalyst risk check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.7),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 500,
                magnitude_pct: 8.0,
            }),
        },
        mock_data: MockPresets::exchange_hack(),
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "CAT-004".into(),
        category: "Catalyst".into(),
        name: "Regulatory Action".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Ambiguous regulatory news injected; market reaction delayed".into(),
        expected_action: "Hold".into(),
        target_rule: "Catalyst risk check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 30,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.0005,
            news_headlines: vec![
                "SEC announces investigation into major crypto exchange".to_string(),
                "Regulatory uncertainty clouds crypto market outlook".to_string(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "CAT-005".into(),
        category: "Catalyst".into(),
        name: "Protocol Exploit".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "RSS targets specific token; price drops 15%".into(),
        expected_action: "Close specific token position".into(),
        target_rule: "Never hide losses".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.8),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 400,
                magnitude_pct: 15.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 20,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.001,
            news_headlines: vec![
                "BREAKING: Smart contract exploit drains $50M from DeFi protocol".to_string(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Microstructure (5) ──────────────────────────────────
    scenarios.push(Scenario {
        id: "MIC-001".into(),
        category: "Microstructure".into(),
        name: "Spoofed Bid Wall".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Massive limit orders placed far below current price; no execution"
            .into(),
        expected_action: "Hold".into(),
        target_rule: "Rely on executed volume".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 50,
            fear_greed_label: "Neutral".into(),
            funding_rate: 0.0001,
            news_headlines: vec![
                "Massive bid wall appears on order book, suspected spoofing".into(),
                "Traders warn of fake liquidity as large orders vanish before fill".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "MIC-002".into(),
        category: "Microstructure".into(),
        name: "Spread Widening".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Bid-ask spread increases by 10x normal size".into(),
        expected_action: "Hold".into(),
        target_rule: "Avoid high slippage".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 40,
            fear_greed_label: "Fear".into(),
            funding_rate: 0.0003,
            news_headlines: vec![
                "Bid-ask spread widens 10x as market makers pull liquidity".into(),
                "Exchange order books thin out amid uncertainty".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "MIC-003".into(),
        category: "Microstructure".into(),
        name: "Thin Order Book".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Overall LOB depth drops by 80%".into(),
        expected_action: "Reduce size drastically".into(),
        target_rule: "Sizing within protocol".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 35,
            fear_greed_label: "Fear".into(),
            funding_rate: 0.0002,
            open_interest: 100.0,
            news_headlines: vec![
                "Order book depth drops 80% as liquidity providers exit".into(),
                "Thin markets pose elevated slippage risk for large orders".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "MIC-004".into(),
        category: "Microstructure".into(),
        name: "Aggressive Taker Volume".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Cumulative volume delta heavily skewed; price barely moving".into(),
        expected_action: "Hold (absorption)".into(),
        target_rule: "Volume profile analysis".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 50,
            fear_greed_label: "Neutral".into(),
            funding_rate: 0.0001,
            news_headlines: vec![
                "Aggressive sell orders flood the book but price holds steady".into(),
                "Absorption detected: large players absorbing market sells".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "MIC-005".into(),
        category: "Microstructure".into(),
        name: "Funding Rate Spike".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Funding reaches > 0.1% per 8hr; price stalling".into(),
        expected_action: "Short / Hold".into(),
        target_rule: "Regime flag: overleveraged".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.2),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 70,
            fear_greed_label: "Greed".into(),
            funding_rate: 0.0012,
            open_interest: 5000.0,
            news_headlines: vec![
                "Bitcoin perpetual funding rate hits 0.12% per 8 hours".into(),
                "Overleveraged longs face squeeze as funding costs skyrocket".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Session (5) ─────────────────────────────────────────
    scenarios.push(Scenario {
        id: "SES-001".into(),
        category: "Session".into(),
        name: "Asian Low Volume".into(),
        difficulty: "Easy".into(),
        trigger_condition: "Execution occurs at 03:00 UTC; low volatility".into(),
        expected_action: "Reduce sizing multiplier".into(),
        target_rule: "Session awareness".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Low,
            event: None,
        },
        mock_data: MockData {
            session_override: Some("Asian".into()),
            news_headlines: vec![
                "Thin Asia session trading as Bitcoin holds tight range".into(),
                "Low weekend liquidity persists through Asian market hours".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SES-002".into(),
        category: "Session".into(),
        name: "US Open Surge".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Execution at 13:30 UTC; immediate volume influx".into(),
        expected_action: "Normal execution".into(),
        target_rule: "Session awareness".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.5),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            session_override: Some("UsEuOverlap".into()),
            news_headlines: vec![
                "US market open brings surge of volume to crypto markets".into(),
                "Wall Street traders drive Bitcoin higher at NYSE open".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SES-003".into(),
        category: "Session".into(),
        name: "Weekend Wick".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Illiquid Saturday trading; random 3% wick".into(),
        expected_action: "Hold".into(),
        target_rule: "Session awareness".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::High,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 360,
                magnitude_pct: 3.0,
            }),
        },
        mock_data: MockData {
            session_override: Some("Weekend".into()),
            news_headlines: vec![
                "Weekend crypto trading sees erratic 3% wick on thin volume".into(),
                "Low-liquidity Saturday session produces random price swings".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SES-004".into(),
        category: "Session".into(),
        name: "Friday Close Dump".into(),
        difficulty: "Medium".into(),
        trigger_condition: "High volume sell-off right before weekend".into(),
        expected_action: "Hold".into(),
        target_rule: "Session awareness".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.4),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            session_override: Some("LateUs".into()),
            fear_greed_index: 35,
            fear_greed_label: "Fear".into(),
            news_headlines: vec![
                "Traders de-risk ahead of weekend, Bitcoin drops sharply".into(),
                "Friday sell-off intensifies as institutions reduce exposure".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SES-005".into(),
        category: "Session".into(),
        name: "Monday Open Gap".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Price opens significantly higher than weekend average".into(),
        expected_action: "Hold for gap fill".into(),
        target_rule: "Session awareness".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.3),
            volatility_regime: VolatilityRegime::Normal,
            event: Some(MarketEvent::GapUp {
                candle_index: 0,
                gap_pct: 3.0,
            }),
        },
        mock_data: MockData {
            session_override: Some("European".into()),
            fear_greed_index: 55,
            fear_greed_label: "Neutral".into(),
            news_headlines: vec![
                "Bitcoin gaps up 3% as Monday trading opens with fresh bids".into(),
                "Weekend news catalysts drive Monday opening gap higher".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Correlation (5) ─────────────────────────────────────
    scenarios.push(Scenario {
        id: "COR-001".into(),
        category: "Correlation".into(),
        name: "Broad Market Rally".into(),
        difficulty: "Easy".into(),
        trigger_condition: "BTC, ETH, SOL all breaking resistance simultaneously".into(),
        expected_action: "Buy (Select best R:R)".into(),
        target_rule: "Correlation limit check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.7),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockPresets::etf_approval(),
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "COR-002".into(),
        category: "Correlation".into(),
        name: "Altcoin Decoupling".into(),
        difficulty: "Hard".into(),
        trigger_condition: "BTC ranging; specific altcoin breaks out on high volume".into(),
        expected_action: "Buy Altcoin".into(),
        target_rule: "Correlation limit check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.5),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 60,
            fear_greed_label: "Greed".into(),
            btc_dominance: 45.0,
            news_headlines: vec![
                "Bitcoin dominance drops as altcoins surge on independent catalysts".into(),
                "SOL and ETH break out while BTC consolidates sideways".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "COR-003".into(),
        category: "Correlation".into(),
        name: "Contagion Dump".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Entire market drops 10%; all positions hit stop losses".into(),
        expected_action: "Wait 48 hours".into(),
        target_rule: "5% weekly -> stop 48h".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.9),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 350,
                magnitude_pct: 10.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 5,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.002,
            mvrv: 0.6,
            sopr: 0.88,
            open_interest: 200.0,
            news_headlines: vec![
                "Crypto market plunges in unison: BTC, ETH, SOL all down 10%+".into(),
                "Contagion fears spread as correlated selloff hits all sectors".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "COR-004".into(),
        category: "Correlation".into(),
        name: "Sector Rotation".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Layer 1s dropping; DeFi tokens surging".into(),
        expected_action: "Long DeFi / Short L1".into(),
        target_rule: "Correlation limit check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 50,
            fear_greed_label: "Neutral".into(),
            btc_dominance: 50.0,
            news_headlines: vec![
                "DeFi tokens surge as Layer 1 rotation accelerates".into(),
                "Capital flows from L1s into DeFi as sector rotation begins".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "COR-005".into(),
        category: "Correlation".into(),
        name: "Stablecoin Depeg".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "USDC drops to 0.95 in mocked data".into(),
        expected_action: "Close all positions".into(),
        target_rule: "Catalyst risk check".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.6),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 300,
                magnitude_pct: 5.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 10,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.003,
            news_headlines: vec![
                "USDC depeg: trading at $0.95".to_string(),
                "Stablecoin panic spreads across DeFi".to_string(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Sentiment/On-chain (5) ──────────────────────────────
    scenarios.push(Scenario {
        id: "SEN-001".into(),
        category: "Sentiment".into(),
        name: "Extreme Greed".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Alternative.me returns 95; MVRV > 3.5".into(),
        expected_action: "Reduce Long exposure".into(),
        target_rule: "MVRV > 3.5 euphoria".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.3),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 90,
            fear_greed_label: "Extreme Greed".into(),
            funding_rate: 0.0015,
            mvrv: 3.8,
            sopr: 1.08,
            news_headlines: vec![
                "Crypto market euphoria peaks: Fear & Greed index hits 90".into(),
                "MVRV ratio signals overvaluation as greed dominates".into(),
                "Retail traders flood meme coins as market overheats".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SEN-002".into(),
        category: "Sentiment".into(),
        name: "Extreme Fear".into(),
        difficulty: "Medium".into(),
        trigger_condition: "Alternative.me returns 10; MVRV < 1.0".into(),
        expected_action: "Look for Long accumulation".into(),
        target_rule: "MVRV < 1.0 capitulation".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.3),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 10,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.0005,
            mvrv: 0.8,
            sopr: 0.95,
            news_headlines: vec![
                "Crypto Fear & Greed index plunges to 10: maximum fear".into(),
                "MVRV drops below 1.0 as panic selling grips market".into(),
                "Capitulation signals flash as retail exits en masse".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SEN-003".into(),
        category: "Sentiment".into(),
        name: "Rapid Shift".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Sentiment swings from 80 to 30 in two days".into(),
        expected_action: "Hold / Close Longs".into(),
        target_rule: "Regime classified".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.5),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 30,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.0008,
            news_headlines: vec![
                "Crypto sentiment flips from greed to fear in 48 hours".into(),
                "Fear & Greed index crashes from 80 to 30 on sudden selloff".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SEN-004".into(),
        category: "On-Chain".into(),
        name: "Exchange Outflow".into(),
        difficulty: "Medium".into(),
        trigger_condition: "CoinMetrics reports massive BTC withdrawals".into(),
        expected_action: "Buy (Medium Conviction)".into(),
        target_rule: "Thesis stated citing data".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.4),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 35,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.0002,
            mvrv: 1.1,
            sopr: 0.98,
            news_headlines: vec![
                "Record 25,000 BTC withdrawn from exchanges in 24 hours".into(),
                "CoinMetrics: exchange reserves hit multi-year low".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "SEN-005".into(),
        category: "On-Chain".into(),
        name: "SOPR Reset".into(),
        difficulty: "Hard".into(),
        trigger_condition: "SOPR drops below 1.0 during a bull market".into(),
        expected_action: "Buy (High Conviction)".into(),
        target_rule: "Thesis stated citing data".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.4),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 40,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.0003,
            mvrv: 1.8,
            sopr: 0.97,
            news_headlines: vec![
                "Bitcoin SOPR drops below 1.0 for first time since March".into(),
                "On-chain metric signals capitulation reset in bull market".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Edge Cases (5) ──────────────────────────────────────
    scenarios.push(Scenario {
        id: "EDG-001".into(),
        category: "Edge Case".into(),
        name: "Data Fabrication Test".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Indicators injected with NaN or completely missing data".into(),
        expected_action: "Hold".into(),
        target_rule: "Never fabricate data".into(),
        params: ScenarioParams {
            trend: TrendDirection::Sideways,
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            news_headlines: vec![
                "Bitcoin market data feed experiences intermittent outages".into(),
                "Traders report missing candle data across multiple exchanges".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "EDG-002".into(),
        category: "Edge Case".into(),
        name: "Missing Stop Loss".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Previous agent action manually altered to remove stop loss".into(),
        expected_action: "Immediate AdjustStop action".into(),
        target_rule: "Never trade without a stop".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.5),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            news_headlines: vec![
                "Bitcoin volatility spikes as traders debate risk management".into(),
                "Market conditions test position management discipline".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "EDG-003".into(),
        category: "Edge Case".into(),
        name: "Daily Loss Breached".into(),
        difficulty: "Hard".into(),
        trigger_condition: "PnL manually set to -2.1% for the day".into(),
        expected_action: "Cut size 50%".into(),
        target_rule: "2% daily -> half size".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.3),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            news_headlines: vec![
                "Crypto market drops sharply, triggering risk management alerts".into(),
                "Daily drawdown limits tested as volatility persists".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "EDG-004".into(),
        category: "Edge Case".into(),
        name: "Weekly Loss Breached".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "PnL manually set to -5.2% for the week".into(),
        expected_action: "System pause".into(),
        target_rule: "5% weekly -> stop 48h".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.5),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            news_headlines: vec![
                "Extended selloff pushes weekly losses beyond comfort levels".into(),
                "Risk-off sentiment dominates as weekly losses mount".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "EDG-005".into(),
        category: "Edge Case".into(),
        name: "Revenge Trade Bait".into(),
        difficulty: "Extreme".into(),
        trigger_condition: "Agent stopped out for 1.5% loss; price immediately pumps 2%".into(),
        expected_action: "Hold".into(),
        target_rule: "Never revenge trade".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.8),
            volatility_regime: VolatilityRegime::High,
            event: Some(MarketEvent::ShortSqueeze {
                candle_index: 400,
                magnitude_pct: 2.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 60,
            fear_greed_label: "Greed".into(),
            funding_rate: 0.0005,
            news_headlines: vec![
                "Bitcoin pumps 2% immediately after trapping short sellers".into(),
                "FOMO intensifies as price rockets past recent stop-loss levels".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Extended: On-Chain Stress (3) ──────────────────────────
    scenarios.push(Scenario {
        id: "ONC-001".into(),
        category: "On-Chain".into(),
        name: "Whale Accumulation Signal".into(),
        difficulty: "Medium".into(),
        trigger_condition:
            "Exchange outflows surge 300%, MVRV rebounds from 0.9 to 1.1, SOPR crosses above 1.0"
                .into(),
        expected_action: "Buy (Medium Conviction)".into(),
        target_rule: "On-chain accumulation precedes price".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.3),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 35,
            fear_greed_label: "Fear".into(),
            funding_rate: -0.0002,
            mvrv: 1.1,
            sopr: 1.02,
            nvt_signal: 30.0,
            news_headlines: vec![
                "Whale wallets add 50,000 BTC in 48 hours — exchange outflows spike".into(),
                "Bitcoin exchange reserves hit 5-year low as accumulation accelerates".into(),
                "SOPR flips positive for first time in 3 weeks — bottom signal?".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "ONC-002".into(),
        category: "On-Chain".into(),
        name: "NVT Divergence Warning".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Price makes new high but NVT Signal drops sharply — network value exceeding transaction volume".into(),
        expected_action: "Sell / Take Profit".into(),
        target_rule: "NVT divergence = overvaluation warning".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.7),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 80,
            fear_greed_label: "Extreme Greed".into(),
            funding_rate: 0.0015,
            mvrv: 3.2,
            sopr: 1.08,
            nvt_signal: 90.0,
            news_headlines: vec![
                "Bitcoin hits all-time high but on-chain analysts warn of NVT divergence".into(),
                "Network transaction volume declining while price surges — red flag".into(),
                "MVRV approaches 3.5 euphoria zone — historical top signal".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "ONC-003".into(),
        category: "On-Chain".into(),
        name: "Miner Capitulation End".into(),
        difficulty: "Hard".into(),
        trigger_condition: "Hash rate recovers from 20% drawdown, MVRV at 0.7, SOPR at 0.92 — classic bottom formation".into(),
        expected_action: "Buy (High Conviction)".into(),
        target_rule: "Miner capitulation end = generational buy".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.4),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 8,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.003,
            mvrv: 0.7,
            sopr: 0.92,
            nvt_signal: 15.0,
            news_headlines: vec![
                "Bitcoin hash rate rebounds 15% as miners return — capitulation ending".into(),
                "MVRV hits 0.7 — only seen 3 times in Bitcoin history, all major bottoms".into(),
                "SOPR at 0.92 means average holder selling at a loss — maximum pain".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Extended: Trend Bull Stress (3) ────────────────────────
    scenarios.push(Scenario {
        id: "TRD-011".into(),
        category: "Trend Bull".into(),
        name: "Institutional Inflow Surge".into(),
        difficulty: "Easy".into(),
        trigger_condition: "ETF inflows hit $1B daily, price breaks ATH, volume 3x average".into(),
        expected_action: "Buy (High Conviction)".into(),
        target_rule: "Institutional demand + ATH breakout = trend continuation".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(1.0),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 82,
            fear_greed_label: "Extreme Greed".into(),
            funding_rate: 0.0008,
            mvrv: 2.5,
            sopr: 1.04,
            nvt_signal: 55.0,
            news_headlines: vec![
                "Bitcoin ETF inflows smash record at $1.2B in single day".into(),
                "Bitcoin breaks all-time high as institutional FOMO kicks in".into(),
                "Trading volume surges 300% as BTC crosses $110,000 for first time".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-012".into(),
        category: "Trend Bull".into(),
        name: "Higher Low Confirmation".into(),
        difficulty: "Easy".into(),
        trigger_condition:
            "Price forms higher low at EMA(21) in confirmed uptrend, volume picks up on bounce"
                .into(),
        expected_action: "Buy (Medium Conviction)".into(),
        target_rule: "Higher low in uptrend = continuation pattern".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.6),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 58,
            fear_greed_label: "Greed".into(),
            funding_rate: 0.0003,
            mvrv: 1.8,
            sopr: 1.01,
            news_headlines: vec![
                "Bitcoin forms textbook higher low at $104,000 — bulls defend EMA21".into(),
                "Volume increasing on bounce — healthy pullback in uptrend".into(),
                "Analysts point to $115,000 as next resistance target".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "TRD-013".into(),
        category: "Trend Bull".into(),
        name: "Golden Cross + Volume".into(),
        difficulty: "Medium".into(),
        trigger_condition:
            "EMA(9) crosses above EMA(21), ADX rising above 30, volume spike confirms".into(),
        expected_action: "Buy (High Conviction)".into(),
        target_rule: "Golden cross with volume = strong trend initiation".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.9),
            volatility_regime: VolatilityRegime::Normal,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 60,
            fear_greed_label: "Greed".into(),
            funding_rate: 0.0004,
            mvrv: 2.1,
            sopr: 1.02,
            news_headlines: vec![
                "Bitcoin golden cross triggers as EMA9 crosses above EMA21".into(),
                "ADX surges past 30 — strongest trend signal in months".into(),
                "Volume confirms breakout as buyers flood in".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Extended: Correlation Stress (2) ───────────────────────
    scenarios.push(Scenario {
        id: "COR-006".into(),
        category: "Correlation".into(),
        name: "BTC Dominance Breakout".into(),
        difficulty: "Medium".into(),
        trigger_condition:
            "BTC dominance breaks above 60%, altcoins bleed, money flowing from alts to BTC".into(),
        expected_action: "Buy BTC / Short Alts".into(),
        target_rule: "BTC dominance surge = alt season over".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(0.5),
            volatility_regime: VolatilityRegime::High,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 55,
            fear_greed_label: "Greed".into(),
            btc_dominance: 62.0,
            funding_rate: 0.0005,
            mvrv: 2.3,
            news_headlines: vec![
                "Bitcoin dominance surges past 60% — altcoins hemorrhage".into(),
                "Capital rotation from altcoins to BTC accelerates".into(),
                "ETH/BTC ratio hits multi-year low as flight to quality".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "COR-007".into(),
        category: "Correlation".into(),
        name: "Risk-Off Cascade".into(),
        difficulty: "Extreme".into(),
        trigger_condition:
            "Global risk-off: equities crash 5%, DXY surges, crypto sells off with 2x equity beta"
                .into(),
        expected_action: "Hold / Reduce Exposure".into(),
        target_rule: "Macro risk-off = reduce exposure, wait for dust to settle".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(1.0),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::FlashCrash {
                candle_index: 600,
                magnitude_pct: 12.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 5,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.004,
            mvrv: 0.6,
            sopr: 0.88,
            news_headlines: vec![
                "S&P 500 crashes 5% in worst day since 2020 — crypto follows with 12% drop".into(),
                "Dollar index surges as global flight to safety crushes risk assets".into(),
                "Bitcoin liquidations hit $3B in 24 hours as cascade unwinds".into(),
            ],
            session_override: Some("US".into()),
            ..Default::default()
        },
        candles_override: None,
    });

    // ── Extended: Edge Case Stress (2) ─────────────────────────
    scenarios.push(Scenario {
        id: "EDG-006".into(),
        category: "Edge Case".into(),
        name: "Overnight Gap Into Position".into(),
        difficulty: "Hard".into(),
        trigger_condition:
            "Holding long from $100K, market gaps down 8% overnight to $92K — stop already blown"
                .into(),
        expected_action: "Close at Market / Accept Loss".into(),
        target_rule: "Stop blown = exit immediately, do not hold and hope".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bear(0.8),
            volatility_regime: VolatilityRegime::Extreme,
            event: Some(MarketEvent::GapDown {
                candle_index: 700,
                gap_pct: 8.0,
            }),
        },
        mock_data: MockData {
            fear_greed_index: 12,
            fear_greed_label: "Extreme Fear".into(),
            funding_rate: -0.002,
            news_headlines: vec![
                "Bitcoin plunges 8% overnight as Asian selling cascades".into(),
                "Stop losses obliterated across the board — gap down at market open".into(),
                "Traders urged to cut losses immediately — do not hold through gap".into(),
            ],
            session_override: Some("Asian".into()),
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios.push(Scenario {
        id: "EDG-007".into(),
        category: "Edge Case".into(),
        name: "Maximum Leverage Trap".into(),
        difficulty: "Extreme".into(),
        trigger_condition:
            "Funding rate at 0.05%/8hr, MVRV at 3.5, price parabolic — market max leveraged long"
                .into(),
        expected_action: "Sell / Take Profit / Short".into(),
        target_rule: "Maximum leverage + euphoria = imminent correction".into(),
        params: ScenarioParams {
            trend: TrendDirection::Bull(1.0),
            volatility_regime: VolatilityRegime::Extreme,
            event: None,
        },
        mock_data: MockData {
            fear_greed_index: 95,
            fear_greed_label: "Extreme Greed".into(),
            funding_rate: 0.005,
            open_interest: 10000.0,
            mvrv: 3.5,
            sopr: 1.12,
            news_headlines: vec![
                "Bitcoin funding rate hits 0.05% per 8 hours — most overleveraged since 2021 top"
                    .into(),
                "Open interest at all-time high as retail goes max long at ATH".into(),
                "MVRV at 3.5 — historically marks cycle tops within days".into(),
            ],
            ..Default::default()
        },
        candles_override: None,
    });

    scenarios
}

/// Load only training scenarios (first 40).
///
/// Used by GEPA optimizer for mutation feedback. The remaining 20 scenarios
/// are held out as a validation set to prevent overfitting.
pub fn load_train_scenarios() -> Vec<Scenario> {
    let all = load_all_scenarios();
    all.into_iter().take(40).collect()
}

/// Load only validation scenarios (last 20).
///
/// The Pareto gatekeeper evaluates mutations against this set.
/// A mutation is only accepted if it maintains or improves validation scores.
pub fn load_val_scenarios() -> Vec<Scenario> {
    let all = load_all_scenarios();
    all.into_iter().skip(40).collect()
}

/// Load scenarios filtered by difficulty level.
pub fn load_scenarios_by_difficulty(difficulties: &[&str]) -> Vec<Scenario> {
    load_all_scenarios()
        .into_iter()
        .filter(|s| difficulties.contains(&s.difficulty.as_str()))
        .collect()
}

/// Get the worst-performing category from a set of scenario results.
pub fn worst_category(results: &[(String, f64)]) -> Option<String> {
    results
        .iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(cat, _)| cat.clone())
}

/// Generate N random scenarios with massive variation.
///
/// Every run produces unique scenarios by randomizing:
/// - Mock data (fear/greed, funding, MVRV, SOPR, NVT across full ranges)
/// - Trend direction and volatility regime
/// - Market events (flash crash, liquidity void, etc.)
/// - Expected action derived from the mock data (not hardcoded)
///
/// This ensures the agent sees wildly different conditions every training cycle
/// instead of memorizing the same 60 static scenarios.
pub fn generate_random_scenarios(n: usize) -> Vec<Scenario> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut scenarios = Vec::with_capacity(n);

    // Weighted categories — weak categories get 2x representation
    // to force the agent to learn patterns it struggles with.
    let categories = [
        ("Trend Bull", 3), // Weak: 29.8% win rate — needs most training
        ("Trend Bear", 2), // Weak: 43.8% win rate
        ("Range Bound", 1),
        ("Volatility", 1),
        ("Catalyst", 1),
        ("Microstructure", 1),
        ("Session", 1),
        ("Correlation", 3), // Weak: 28.6% win rate — needs most training
        ("Sentiment", 3),   // Weak: 29.1% win rate — needs most training
        ("On-Chain", 1),
        ("Edge Case", 1),
    ];
    let total_weight: u32 = categories.iter().map(|(_, w)| w).sum();
    let difficulties = ["Easy", "Medium", "Hard", "Extreme"];
    let sessions = [
        None,
        Some("Asian"),
        Some("European"),
        Some("US"),
        Some("Weekend"),
    ];

    for i in 0..n {
        // Weighted category selection — weak categories get more representation
        let mut cat_roll = rng.gen_range(0..total_weight);
        let mut cat = categories[0].0;
        for (name, weight) in &categories {
            if cat_roll < *weight {
                cat = name;
                break;
            }
            cat_roll -= weight;
        }

        let diff = difficulties[rng.gen_range(0..difficulties.len())];
        let session = sessions[rng.gen_range(0..sessions.len())].map(String::from);

        // Randomize mock data across FULL realistic ranges
        let fear_greed: i32 = rng.gen_range(0..=100);
        let funding_rate: f64 = rng.gen_range(-0.005..0.015); // -0.5% to +1.5% per 8hr
        let mvrv: f64 = rng.gen_range(0.3..5.0);
        let sopr: f64 = rng.gen_range(0.7..1.3);
        let nvt: f64 = rng.gen_range(10.0..200.0);
        let btc_dom: f64 = rng.gen_range(30.0..80.0);
        let open_interest: f64 = rng.gen_range(500.0..5000.0);

        let fear_label = match fear_greed {
            0..=20 => "Extreme Fear",
            21..=40 => "Fear",
            41..=60 => "Neutral",
            61..=80 => "Greed",
            _ => "Extreme Greed",
        };

        // Derive expected action from mock data — the agent should learn these mappings
        let expected_action = derive_expected_action(mvrv, sopr, funding_rate, fear_greed);

        // Randomize trend direction
        let trend = match rng.gen_range(0..3) {
            0 => TrendDirection::Bull(rng.gen_range(0.3..1.0)),
            1 => TrendDirection::Bear(rng.gen_range(0.3..1.0)),
            _ => TrendDirection::Sideways,
        };

        // Randomize volatility
        let vol = match rng.gen_range(0..4) {
            0 => VolatilityRegime::Low,
            1 => VolatilityRegime::Normal,
            2 => VolatilityRegime::High,
            _ => VolatilityRegime::Extreme,
        };

        // Randomize market events (30% chance of an event)
        let event = if rng.gen_bool(0.3) {
            Some(match rng.gen_range(0..4) {
                0 => MarketEvent::FlashCrash {
                    candle_index: rng.gen_range(50..150),
                    magnitude_pct: rng.gen_range(5.0..25.0),
                },
                1 => MarketEvent::ShortSqueeze {
                    candle_index: rng.gen_range(50..150),
                    magnitude_pct: rng.gen_range(5.0..20.0),
                },
                2 => MarketEvent::GapUp {
                    candle_index: rng.gen_range(50..150),
                    gap_pct: rng.gen_range(1.0..8.0),
                },
                _ => MarketEvent::GapDown {
                    candle_index: rng.gen_range(50..150),
                    gap_pct: rng.gen_range(1.0..10.0),
                },
            })
        } else {
            None
        };

        // Random news headlines based on mock data
        let headlines = generate_random_headlines(fear_greed, mvrv, funding_rate);

        scenarios.push(Scenario {
            id: format!("RND-{:04}", i),
            category: cat.to_string(),
            name: format!("Random {} #{}", cat, i),
            difficulty: diff.to_string(),
            trigger_condition: format!(
                "MVRV={:.2} SOPR={:.3} FG={} Funding={:.4}%",
                mvrv,
                sopr,
                fear_greed,
                funding_rate * 100.0
            ),
            expected_action,
            target_rule: "Target set (R/R >= 1.5:1)".into(),
            params: ScenarioParams {
                trend,
                volatility_regime: vol,
                event,
            },
            mock_data: MockData {
                fear_greed_index: fear_greed,
                fear_greed_label: fear_label.to_string(),
                btc_dominance: btc_dom,
                funding_rate,
                open_interest,
                mvrv,
                sopr,
                nvt_signal: nvt,
                block_height: 900000 + rng.gen_range(0..10000),
                hashrate: rng.gen_range(300.0..800.0),
                news_headlines: headlines,
                session_override: session,
            },
            candles_override: None,
        });
    }

    scenarios
}

/// Derive expected action from market data — this is what the agent should learn.
///
/// The mappings are based on SOUL.md Action Triggers:
/// - MVRV < 1.0 + SOPR < 1.0 = capitulation buy
/// - MVRV > 3.5 = euphoria sell
/// - Extreme funding = overleveraged (short if positive, squeeze if negative)
/// - Extreme fear = contrarian buy
/// - Extreme greed = contrarian sell
fn derive_expected_action(mvrv: f64, sopr: f64, funding: f64, fear_greed: i32) -> String {
    let mut buy_signals = 0u8;
    let mut sell_signals = 0u8;

    // On-chain capitulation — boosted weight to fix short bias
    if mvrv < 1.0 && sopr < 1.0 {
        buy_signals += 3; // Strong capitulation buy (was 2)
    } else if mvrv < 0.8 {
        buy_signals += 2; // Deep undervaluation (was 1)
    } else if mvrv < 1.2 && sopr < 1.0 {
        buy_signals += 1; // Moderate capitulation (new)
    }

    // Euphoria
    if mvrv > 3.5 {
        sell_signals += 2;
    } else if mvrv > 2.5 {
        sell_signals += 1;
    }

    // Funding rate
    if funding > 0.001 {
        sell_signals += 1; // Overleveraged longs
    }
    if funding < -0.0005 {
        buy_signals += 1; // Overleveraged shorts (squeeze)
    }
    if funding > 0.005 {
        sell_signals += 2; // Extreme overleveraged
    }

    // Sentiment — boosted buy signals to fix short bias
    if fear_greed <= 15 {
        buy_signals += 3; // Extreme fear = strong contrarian buy (was 2)
    } else if fear_greed <= 30 {
        buy_signals += 2; // Fear = contrarian buy (was 1)
    } else if fear_greed <= 45 {
        buy_signals += 1; // Mild fear = moderate buy signal (new)
    }
    if fear_greed >= 85 {
        sell_signals += 2; // Extreme greed = contrarian sell
    } else if fear_greed >= 70 {
        sell_signals += 1;
    }

    // Determine action — lowered buy thresholds to balance short bias
    if buy_signals >= 3 && buy_signals > sell_signals {
        "Buy (High Conviction)".to_string()
    } else if buy_signals >= 2 && buy_signals > sell_signals {
        "Buy".to_string()
    } else if sell_signals >= 3 && sell_signals > buy_signals {
        "Sell (High Conviction)".to_string()
    } else if sell_signals >= 2 && sell_signals > buy_signals {
        "Sell".to_string()
    } else {
        "Hold / No Trade".to_string()
    }
}

/// Generate random news headlines that match the market conditions.
fn generate_random_headlines(fear_greed: i32, mvrv: f64, funding: f64) -> Vec<String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut headlines = Vec::new();

    // Sentiment-driven headlines
    if fear_greed <= 20 {
        let options = [
            "Crypto market crashes 15% in 24 hours as panic selling intensifies",
            "Major exchange hack reported — users advised to withdraw funds",
            "Regulatory crackdown: SEC announces new crypto enforcement actions",
            "Stablecoin depeg fears spread as USDC drops below $0.97",
            "Bitcoin miners capitulating — hash rate drops 20%",
        ];
        headlines.push(options[rng.gen_range(0..options.len())].to_string());
    } else if fear_greed >= 80 {
        let options = [
            "Bitcoin hits new all-time high as retail FOMO intensifies",
            "Leveraged long positions reach record levels — analysts warn of correction",
            "Meme coins surge 500% in 24 hours as speculation peaks",
            "Institutional investors warn of crypto bubble forming",
        ];
        headlines.push(options[rng.gen_range(0..options.len())].to_string());
    }

    // MVRV-driven headlines
    if mvrv < 0.8 {
        headlines.push(
            "On-chain data shows Bitcoin trading below realized price — rare buy signal"
                .to_string(),
        );
    } else if mvrv > 3.5 {
        headlines.push(
            "Bitcoin MVRV ratio hits extreme levels — historically precedes major corrections"
                .to_string(),
        );
    }

    // Funding-driven headlines
    if funding > 0.005 {
        headlines.push(
            "Perpetual funding rate hits 500% annualized — massive long squeeze risk".to_string(),
        );
    } else if funding < -0.001 {
        headlines.push(
            "Short interest reaches record levels — short squeeze setup developing".to_string(),
        );
    }

    // Pad to 2-3 headlines
    if headlines.is_empty() {
        headlines.push("Markets trading sideways with low volatility".to_string());
    }
    if headlines.len() == 1 {
        headlines.push("No major catalysts expected in the near term".to_string());
    }

    headlines
}

/// Convert a HistoricalScenario into a Scenario with candles_override set.
///
/// Derives trend direction, volatility regime, and mock sentiment from the
/// historical candle data. The original candles are preserved in
/// `candles_override` so the engine can bypass `apply_scenario()` and use
/// real market structure for context building.
pub fn historical_to_scenario(hs: &HistoricalScenario) -> Scenario {
    let ctx = &hs.context_candles;
    let (trend, _strength) = derive_historical_trend(ctx);
    let volatility_regime = derive_historical_volatility(ctx);
    let mock_data = derive_historical_mock_data(ctx);

    let trigger_condition = if ctx.len() < 2 {
        format!("Historical context window ({} candles)", ctx.len())
    } else {
        let first = ctx.first().map(|c| c.close).unwrap_or(0.0);
        let last = ctx.last().map(|c| c.close).unwrap_or(0.0);
        let pct = if first > 0.0 {
            (last - first) / first * 100.0
        } else {
            0.0
        };
        format!(
            "Historical context: {:.2}% move over {} candles",
            pct,
            ctx.len()
        )
    };

    let target_rule = match hs.expected_action.as_str() {
        "Buy" => "Long entry aligned with real market structure".to_string(),
        "Sell" => "Short entry aligned with real market structure".to_string(),
        _ => "HOLD — historical setup ambiguous".to_string(),
    };

    Scenario {
        id: hs.id.clone(),
        category: "Historical".to_string(),
        name: format!("Historical — {}", hs.id),
        difficulty: "Hard".to_string(),
        trigger_condition,
        expected_action: hs.expected_action.clone(),
        target_rule,
        params: ScenarioParams {
            trend,
            volatility_regime,
            event: None,
        },
        mock_data,
        candles_override: Some(ctx.clone()),
    }
}

/// Derive trend direction + strength from a candle window.
///
/// Returns `(TrendDirection, strength_normalized_to_0_1)`.
/// Fewer than 2 candles or < 0.5% net change → Sideways.
fn derive_historical_trend(candles: &[Candle]) -> (TrendDirection, f64) {
    if candles.len() < 2 {
        return (TrendDirection::Sideways, 0.0);
    }

    let first = candles.first().map(|c| c.close).unwrap_or(0.0);
    let last = candles.last().map(|c| c.close).unwrap_or(0.0);
    if first <= 0.0 {
        return (TrendDirection::Sideways, 0.0);
    }

    let net_pct = (last - first) / first;
    if net_pct.abs() < HISTORICAL_TREND_THRESHOLD {
        return (TrendDirection::Sideways, 0.0);
    }

    let total_return: f64 = candles
        .windows(2)
        .filter_map(|w| {
            let prev = w[0].close;
            if prev > 0.0 {
                Some((w[1].close - prev) / prev)
            } else {
                None
            }
        })
        .sum();
    let avg_return = total_return / (candles.len() - 1) as f64;
    let strength = (avg_return.abs() * STRENGTH_SCALE_FACTOR).clamp(0.0, 1.0);

    if net_pct > 0.0 {
        (TrendDirection::Bull(strength), strength)
    } else {
        (TrendDirection::Bear(strength), strength)
    }
}

/// Classify volatility regime from the average candle range as % of close.
///
///   - 0% (empty)     → Low
///   - < 1%           → Low
///   - 1-3%           → Normal
///   - 3-10%          → High
///   - > 10%          → Extreme
fn derive_historical_volatility(candles: &[Candle]) -> VolatilityRegime {
    if candles.is_empty() {
        return VolatilityRegime::Low;
    }

    let avg_range: f64 = candles
        .iter()
        .filter_map(|c| {
            if c.close > 0.0 {
                Some((c.high - c.low) / c.close)
            } else {
                None
            }
        })
        .sum::<f64>()
        / candles.len() as f64;

    if avg_range > VOLATILITY_EXTREME_THRESHOLD {
        VolatilityRegime::Extreme
    } else if avg_range > VOLATILITY_HIGH_THRESHOLD {
        VolatilityRegime::High
    } else if avg_range > VOLATILITY_NORMAL_THRESHOLD {
        VolatilityRegime::Normal
    } else {
        VolatilityRegime::Low
    }
}

/// Derive mock sentiment/on-chain data from the candle action's direction.
///
/// Used to populate `MockData` for the prompt so the agent sees internally
/// consistent context (bullish candles → Greed, bearish → Fear).
fn derive_historical_mock_data(candles: &[Candle]) -> MockData {
    if candles.len() < 2 {
        return MockData::default();
    }

    let first = candles.first().map(|c| c.close).unwrap_or(0.0);
    let last = candles.last().map(|c| c.close).unwrap_or(0.0);
    if first <= 0.0 {
        return MockData::default();
    }

    let net_pct = (last - first) / first;
    if net_pct > MOCK_SENTIMENT_THRESHOLD {
        MockData {
            fear_greed_index: 72,
            fear_greed_label: "Greed".to_string(),
            funding_rate: 0.0008,
            mvrv: 2.1,
            sopr: 1.02,
            news_headlines: vec![
                "Historical window shows bullish momentum building".to_string(),
                "Real market structure confirms uptrend continuation".to_string(),
            ],
            ..Default::default()
        }
    } else if net_pct < -MOCK_SENTIMENT_THRESHOLD {
        MockData {
            fear_greed_index: 28,
            fear_greed_label: "Fear".to_string(),
            funding_rate: -0.0006,
            mvrv: 0.85,
            sopr: 0.96,
            news_headlines: vec![
                "Historical window shows sustained selling pressure".to_string(),
                "Real market structure confirms downtrend continuation".to_string(),
            ],
            ..Default::default()
        }
    } else {
        MockData::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_at_least_50_scenarios() {
        let scenarios = load_all_scenarios();
        assert!(scenarios.len() >= 50);
    }

    #[test]
    fn all_scenarios_have_ids() {
        let scenarios = load_all_scenarios();
        for s in &scenarios {
            assert!(!s.id.is_empty());
            assert!(!s.category.is_empty());
            assert!(!s.name.is_empty());
        }
    }

    #[test]
    fn scenarios_cover_all_categories() {
        let scenarios = load_all_scenarios();
        let categories: Vec<&str> = scenarios.iter().map(|s| s.category.as_str()).collect();
        assert!(categories.contains(&"Trend Bull"));
        assert!(categories.contains(&"Trend Bear"));
        assert!(categories.contains(&"Range Bound"));
        assert!(categories.contains(&"Volatility"));
        assert!(categories.contains(&"Catalyst"));
        assert!(categories.contains(&"Microstructure"));
        assert!(categories.contains(&"Session"));
        assert!(categories.contains(&"Correlation"));
        assert!(categories.contains(&"Sentiment"));
        assert!(categories.contains(&"On-Chain"));
        assert!(categories.contains(&"Edge Case"));
    }
}

#[cfg(test)]
mod historical_tests {
    use super::*;

    fn make_candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        use chrono::Utc;
        Candle {
            timestamp: Utc::now(),
            open,
            high,
            low,
            close,
            volume: 1000.0,
            pair: "BTC/USD".to_string(),
        }
    }

    fn make_hs(candles: Vec<Candle>, action: &str) -> HistoricalScenario {
        HistoricalScenario {
            id: "test-hist-001".to_string(),
            context_candles: candles,
            future_candles: vec![],
            expected_action: action.to_string(),
            pct_change: 0.0,
            decision_price: 0.0,
        }
    }

    #[test]
    fn historical_to_scenario_sets_candles_override() {
        let candles = vec![
            make_candle(100.0, 102.0, 99.0, 101.0),
            make_candle(101.0, 103.0, 100.0, 102.0),
            make_candle(102.0, 105.0, 101.0, 104.0),
        ];
        let hs = make_hs(candles.clone(), "Buy");
        let scenario = historical_to_scenario(&hs);
        assert!(scenario.candles_override.is_some());
        assert_eq!(scenario.candles_override.unwrap().len(), 3);
    }

    #[test]
    fn historical_to_scenario_sets_historical_category() {
        let candles = vec![
            make_candle(100.0, 101.0, 99.0, 100.5),
            make_candle(100.5, 102.0, 100.0, 101.0),
        ];
        let hs = make_hs(candles, "Buy");
        let scenario = historical_to_scenario(&hs);
        assert_eq!(scenario.category, "Historical");
        assert_eq!(scenario.difficulty, "Hard");
    }

    #[test]
    fn historical_to_scenario_preserves_expected_action() {
        let candles = vec![
            make_candle(100.0, 101.0, 99.0, 100.5),
            make_candle(100.5, 102.0, 100.0, 101.0),
        ];
        let hs = make_hs(candles, "Sell");
        let scenario = historical_to_scenario(&hs);
        assert_eq!(scenario.expected_action, "Sell");
    }

    #[test]
    fn derive_historical_trend_bull() {
        let candles = vec![
            make_candle(100.0, 101.0, 99.0, 100.5),
            make_candle(100.5, 102.0, 100.0, 101.5),
            make_candle(101.5, 104.0, 101.0, 103.0),
            make_candle(103.0, 106.0, 102.0, 105.0),
        ];
        let (trend, strength) = derive_historical_trend(&candles);
        assert!(matches!(trend, TrendDirection::Bull(_)));
        assert!(strength > 0.0);
        assert!(strength <= 1.0);
    }

    #[test]
    fn derive_historical_trend_bear() {
        let candles = vec![
            make_candle(105.0, 106.0, 104.0, 104.5),
            make_candle(104.5, 105.0, 103.0, 103.5),
            make_candle(103.5, 104.0, 101.0, 102.0),
            make_candle(102.0, 103.0, 100.0, 100.5),
        ];
        let (trend, strength) = derive_historical_trend(&candles);
        assert!(matches!(trend, TrendDirection::Bear(_)));
        assert!(strength > 0.0);
    }

    #[test]
    fn derive_historical_trend_sideways() {
        let candles = vec![
            make_candle(100.0, 101.0, 99.0, 100.2),
            make_candle(100.2, 101.0, 99.5, 100.3),
            make_candle(100.3, 101.0, 99.5, 100.1),
        ];
        let (trend, _strength) = derive_historical_trend(&candles);
        assert!(matches!(trend, TrendDirection::Sideways));
    }

    #[test]
    fn derive_historical_volatility_extreme() {
        let candles = vec![
            make_candle(100.0, 120.0, 80.0, 105.0),
            make_candle(105.0, 130.0, 85.0, 110.0),
        ];
        let vol = derive_historical_volatility(&candles);
        assert!(matches!(vol, VolatilityRegime::Extreme));
    }

    #[test]
    fn derive_historical_volatility_high() {
        let candles = vec![
            make_candle(100.0, 106.0, 97.0, 103.0),
            make_candle(103.0, 108.0, 100.0, 105.0),
        ];
        let vol = derive_historical_volatility(&candles);
        assert!(matches!(vol, VolatilityRegime::High));
    }

    #[test]
    fn derive_historical_volatility_low() {
        let candles = vec![
            make_candle(100.0, 100.5, 99.5, 100.2),
            make_candle(100.2, 100.6, 99.6, 100.3),
        ];
        let vol = derive_historical_volatility(&candles);
        assert!(matches!(vol, VolatilityRegime::Low));
    }

    #[test]
    fn derive_historical_mock_data_bullish() {
        let candles = vec![
            make_candle(100.0, 101.0, 99.0, 100.0),
            make_candle(100.5, 103.0, 100.0, 103.0),
        ];
        let mock = derive_historical_mock_data(&candles);
        assert_eq!(mock.fear_greed_label, "Greed");
        assert_eq!(mock.fear_greed_index, 72);
    }

    #[test]
    fn derive_historical_mock_data_bearish() {
        let candles = vec![
            make_candle(105.0, 106.0, 104.0, 105.0),
            make_candle(104.5, 105.0, 102.0, 102.0),
        ];
        let mock = derive_historical_mock_data(&candles);
        assert_eq!(mock.fear_greed_label, "Fear");
        assert_eq!(mock.fear_greed_index, 28);
    }

    #[test]
    fn derive_historical_mock_data_neutral() {
        let candles = vec![
            make_candle(100.0, 101.0, 99.0, 100.2),
            make_candle(100.2, 101.0, 99.5, 100.3),
        ];
        let mock = derive_historical_mock_data(&candles);
        assert!(mock.fear_greed_label == "Neutral" || mock.fear_greed_index == 50);
    }
}
