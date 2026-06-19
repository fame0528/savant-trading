use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[derive(Error, Debug)]
pub enum DataError {
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("WebSocket error: {0}")]
    WsError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("No data available: {0}")]
    NoData(String),
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Order rejected: {0}")]
    OrderRejected(String),
    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: f64, available: f64 },
    #[error("Position not found: {0}")]
    PositionNotFound(String),
    // FID-210: duplicate position id detected at open time
    #[error("Duplicate position id: {0}")]
    DuplicatePositionId(String),
    // FID-210: invalid stop-loss ratchet (e.g. trying to lower the SL below
    // the current price for a long, which is a guard against catastrophic fills)
    #[error("Invalid stop ratchet: old {old}, new {new}")]
    InvalidStopRatchet { old: f64, new: f64 },
    #[error("Exchange error: {0}")]
    ExchangeError(String),
    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum StrategyError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

#[derive(Error, Debug)]
pub enum RiskError {
    #[error("Risk limit exceeded: {0}")]
    RiskLimitExceeded(String),
    #[error("Circuit breaker triggered: {0}")]
    CircuitBreakerTriggered(String),
    #[error("Position sizing error: {0}")]
    PositionSizingError(String),
}

#[derive(Error, Debug)]
pub enum SavantError {
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    #[error("Data error: {0}")]
    Data(#[from] DataError),
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),
    #[error("Strategy error: {0}")]
    Strategy(#[from] StrategyError),
    #[error("Risk error: {0}")]
    Risk(#[from] RiskError),
}
