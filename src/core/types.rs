use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Long,
    Short,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Long => write!(f, "LONG"),
            Side::Short => write!(f, "SHORT"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Filled,
    PartiallyFilled,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timeframe {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
}

impl Timeframe {
    pub fn seconds(&self) -> u64 {
        match self {
            Self::OneMinute => 60,
            Self::FiveMinutes => 300,
            Self::FifteenMinutes => 900,
            Self::OneHour => 3600,
            Self::FourHours => 14400,
            Self::OneDay => 86400,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "1m" | "1min" => Some(Self::OneMinute),
            "5m" | "5min" => Some(Self::FiveMinutes),
            "15m" | "15min" => Some(Self::FifteenMinutes),
            "1h" | "1hr" => Some(Self::OneHour),
            "4h" | "4hr" => Some(Self::FourHours),
            "1d" | "daily" => Some(Self::OneDay),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub pair: String,
}

impl Candle {
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// On-chain display name for a trading pair.
    /// Maps exchange pair names to their actual on-chain token names.
    /// "ETH/USD" → "WETH/USD" (on Arbitrum, ETH is gas, WETH is the trade)
    /// "BTC/USD" → "WBTC/USD" (on Arbitrum, BTC is bridged as WBTC)
    /// All other pairs pass through unchanged.
    pub fn display_pair(pair: &str) -> &str {
        match pair {
            "ETH/USD" => "WETH/USD",
            "BTC/USD" => "WBTC/USD",
            _ => pair,
        }
    }

    /// Exchange API pair name (reverse of display_pair).
    /// "WETH/USD" → "ETH/USD" for Kraken/OKX/Binance APIs.
    pub fn exchange_pair(pair: &str) -> &str {
        match pair {
            "WETH/USD" => "ETH/USD",
            "WBTC/USD" => "BTC/USD",
            _ => pair,
        }
    }

    /// Normalize base token for exchange APIs.
    /// "WETH" → "ETH", "WBTC" → "BTC". All others pass through.
    pub fn exchange_base(base: &str) -> &str {
        match base {
            "WETH" => "ETH",
            "WBTC" => "BTC",
            _ => base,
        }
    }

    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    pub fn range(&self) -> f64 {
        self.high - self.low
    }

    pub fn upper_wick(&self) -> f64 {
        self.high - self.open.max(self.close)
    }

    pub fn lower_wick(&self) -> f64 {
        self.open.min(self.close) - self.low
    }

    /// Return timestamp as Unix seconds.
    pub fn timestamp_unix(&self) -> i64 {
        self.timestamp.timestamp()
    }

    /// Return timestamp as RFC 3339 string.
    pub fn timestamp_rfc3339(&self) -> String {
        self.timestamp.to_rfc3339()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub pair: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: DateTime<Utc>,
}

impl OrderBook {
    pub fn best_bid(&self) -> Option<f64> {
        self.bids.first().map(|l| l.price)
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks.first().map(|l| l.price)
    }

    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub pair: String,
    pub side: Side,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    pub take_profit_3: f64,
    pub strategy_name: String,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub metadata: SignalMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMetadata {
    pub regime: Option<MarketRegime>,
    pub atr: Option<f64>,
    pub volume_ratio: Option<f64>,
    pub adx: Option<f64>,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketRegime {
    Trending,
    Ranging,
    Volatile,
}

impl std::fmt::Display for MarketRegime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketRegime::Trending => write!(f, "Trending"),
            MarketRegime::Ranging => write!(f, "Ranging"),
            MarketRegime::Volatile => write!(f, "Volatile"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub pair: String,
    pub side: Side,
    pub entry_price: f64,
    pub current_price: f64,
    pub quantity: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    pub take_profit_3: f64,
    pub unrealized_pnl: f64,
    pub risk_amount: f64,
    pub strategy_name: String,
    pub opened_at: DateTime<Utc>,
    pub scale_level: ScaleLevel,
    /// On-chain ERC-20 token address for this position's base asset.
    /// Used by reconciliation heartbeat to verify on-chain balance matches
    /// in-memory tracking. Empty string if unknown (legacy positions).
    #[serde(default)]
    pub token_address: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScaleLevel {
    Full,
    Scaled50,
    Scaled80,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub pair: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub quantity: f64,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub filled_at: Option<DateTime<Utc>>,
    pub filled_price: Option<f64>,
    /// On-chain transaction hash (DEX swaps only). None for paper trading.
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: String,
    pub pair: String,
    pub side: Side,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub fees: f64,
    pub strategy_name: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
    pub notes: String,
    #[serde(default)]
    pub on_chain_verified: bool,
    #[serde(default)]
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub balance: f64,
    pub equity: f64,
    pub unrealized_pnl: f64,
    pub daily_pnl: f64,
    pub peak_equity: f64,
    pub drawdown_pct: f64,
    pub open_positions: usize,
    pub max_positions: usize,
    pub trades_today: usize,
}

impl AccountState {
    pub fn new(balance: f64) -> Self {
        Self {
            balance,
            equity: balance,
            unrealized_pnl: 0.0,
            daily_pnl: 0.0,
            peak_equity: balance,
            drawdown_pct: 0.0,
            open_positions: 0,
            max_positions: 3,
            trades_today: 0,
        }
    }

    /// Recompute equity, unrealized P&L, drawdown from current positions.
    /// This is the SINGLE source of truth for account metrics.
    /// equity = cash balance + sum(position market values)
    /// unrealized_pnl = sum(per-position unrealized P&L)
    /// Recalculates per-position PnL from entry_price vs current_price
    /// so it's always live — not dependent on stale fields.
    pub fn refresh_from_positions(&mut self, positions: &HashMap<String, Position>) {
        let mut position_values: f64 = 0.0;
        let mut total_pnl: f64 = 0.0;
        for p in positions.values() {
            position_values += p.current_price * p.quantity;
            let pnl = match p.side {
                Side::Long => (p.current_price - p.entry_price) * p.quantity,
                Side::Short => (p.entry_price - p.current_price) * p.quantity,
            };
            total_pnl += pnl;
        }
        self.unrealized_pnl = total_pnl;
        self.equity = self.balance + position_values;
        self.open_positions = positions.len();
        if self.equity > self.peak_equity {
            self.peak_equity = self.equity;
        }
        if self.peak_equity > 0.0 {
            self.drawdown_pct = (self.peak_equity - self.equity) / self.peak_equity;
        }
    }
}

#[derive(Debug, Clone)]
pub struct VolumeProfile {
    pub poc_price: f64,
    pub poc_volume: f64,
    pub value_area_high: f64,
    pub value_area_low: f64,
    pub levels: Vec<VolumeLevel>,
}

#[derive(Debug, Clone)]
pub struct VolumeLevel {
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug, Clone)]
pub struct IndicatorValues {
    pub ema_fast: Option<f64>,
    pub ema_slow: Option<f64>,
    pub rsi: Option<f64>,
    pub atr: Option<f64>,
    pub adx: Option<f64>,
    pub vwap: Option<f64>,
    pub volume_sma: Option<f64>,
    pub garman_klass: Option<f64>,
    pub parabolic_sar: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum TradingEvent {
    CandleClosed(Candle),
    SignalGenerated(Signal),
    OrderPlaced(Order),
    OrderFilled(Order),
    PositionOpened(Position),
    PositionClosed(TradeRecord),
    RiskAlert(String),
    CircuitBreakerTriggered(String),
    RegimeChanged(MarketRegime),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position(pair: &str, side: Side, entry: f64, qty: f64, current: f64) -> Position {
        Position {
            id: format!("test-{}", pair),
            pair: pair.to_string(),
            side,
            entry_price: entry,
            current_price: current,
            quantity: qty,
            stop_loss: 0.0,
            take_profit_1: 0.0,
            take_profit_2: 0.0,
            take_profit_3: 0.0,
            unrealized_pnl: match side {
                Side::Long => (current - entry) * qty,
                Side::Short => (entry - current) * qty,
            },
            risk_amount: 0.0,
            strategy_name: "test".into(),
            scale_level: ScaleLevel::Full,
            opened_at: Utc::now(),
            token_address: String::new(),
        }
    }

    #[test]
    fn refresh_empty_positions_equity_equals_balance() {
        let mut acct = AccountState::new(100.0);
        let positions = HashMap::new();
        acct.refresh_from_positions(&positions);
        assert_eq!(acct.equity, 100.0);
        assert_eq!(acct.unrealized_pnl, 0.0);
        assert_eq!(acct.open_positions, 0);
    }

    #[test]
    fn refresh_with_long_position_in_profit() {
        let mut acct = AccountState::new(50.0);
        let mut positions = HashMap::new();
        positions.insert(
            "ETH/USD".into(),
            make_position("ETH/USD", Side::Long, 100.0, 1.0, 110.0),
        );
        acct.refresh_from_positions(&positions);
        // equity = balance + position_market_value = 50 + (110 * 1) = 160
        assert_eq!(acct.equity, 160.0);
        assert_eq!(acct.unrealized_pnl, 10.0);
        assert_eq!(acct.open_positions, 1);
    }

    #[test]
    fn refresh_peak_equity_tracks_highs() {
        let mut acct = AccountState::new(100.0);
        let mut positions = HashMap::new();
        acct.refresh_from_positions(&positions);
        assert_eq!(acct.peak_equity, 100.0);

        positions.insert(
            "ETH/USD".into(),
            make_position("ETH/USD", Side::Long, 100.0, 1.0, 120.0),
        );
        acct.refresh_from_positions(&positions);
        assert_eq!(acct.peak_equity, 220.0); // 100 + 120

        // Price drops — peak should NOT decrease
        positions.insert(
            "ETH/USD".into(),
            make_position("ETH/USD", Side::Long, 100.0, 1.0, 105.0),
        );
        acct.refresh_from_positions(&positions);
        assert_eq!(acct.peak_equity, 220.0); // unchanged
        assert!(acct.drawdown_pct > 0.0);
    }

    #[test]
    fn refresh_drawdown_calculation() {
        let mut acct = AccountState::new(100.0);
        let mut positions = HashMap::new();
        positions.insert(
            "ETH/USD".into(),
            make_position("ETH/USD", Side::Long, 100.0, 1.0, 200.0),
        );
        acct.refresh_from_positions(&positions);
        assert_eq!(acct.peak_equity, 300.0); // 100 + 200

        // Price drops 50%: equity = 100 + 100 = 200, DD = (300-200)/300 = 33.3%
        positions.insert(
            "ETH/USD".into(),
            make_position("ETH/USD", Side::Long, 100.0, 1.0, 100.0),
        );
        acct.refresh_from_positions(&positions);
        assert!((acct.drawdown_pct - 0.333).abs() < 0.01);
    }

    #[test]
    fn refresh_short_position_pnl() {
        let mut acct = AccountState::new(50.0);
        let mut positions = HashMap::new();
        positions.insert(
            "BTC/USD".into(),
            make_position("BTC/USD", Side::Short, 1000.0, 0.01, 900.0),
        );
        acct.refresh_from_positions(&positions);
        // Short profit: (1000 - 900) * 0.01 = 1.0
        assert_eq!(acct.unrealized_pnl, 1.0);
        // equity = 50 + (900 * 0.01) = 59
        assert_eq!(acct.equity, 59.0);
    }
}
