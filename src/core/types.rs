use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
            trades_today: 0,
        }
    }

    pub fn update_equity(&mut self, unrealized: f64) {
        self.unrealized_pnl = unrealized;
        self.equity = self.balance + unrealized;
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
