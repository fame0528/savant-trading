//! Mock API layer for sandbox — intercepts all external HTTP requests.
//!
//! Returns scenario-specific data instead of live API responses.

use serde::{Deserialize, Serialize};

/// Mock response data for a single scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockData {
    pub fear_greed_index: i32,
    pub fear_greed_label: String,
    pub btc_dominance: f64,
    pub funding_rate: f64,
    pub open_interest: f64,
    pub mvrv: f64,
    pub sopr: f64,
    pub nvt_signal: f64,
    pub block_height: u64,
    pub hashrate: f64,
    pub news_headlines: Vec<String>,
    pub session_override: Option<String>,
}

impl Default for MockData {
    fn default() -> Self {
        Self {
            fear_greed_index: 50,
            fear_greed_label: "Neutral".to_string(),
            btc_dominance: 55.0,
            funding_rate: 0.01,
            open_interest: 1000.0,
            mvrv: 1.5,
            sopr: 1.0,
            nvt_signal: 50.0,
            block_height: 900000,
            hashrate: 600.0,
            news_headlines: Vec::new(),
            session_override: None,
        }
    }
}

/// Predefined mock data sets for common market conditions.
pub struct MockPresets;

impl MockPresets {
    pub fn extreme_fear() -> MockData {
        MockData {
            fear_greed_index: 10,
            fear_greed_label: "Extreme Fear".to_string(),
            funding_rate: -0.0005,
            mvrv: 0.8,
            sopr: 0.95,
            ..Default::default()
        }
    }

    pub fn extreme_greed() -> MockData {
        MockData {
            fear_greed_index: 90,
            fear_greed_label: "Extreme Greed".to_string(),
            funding_rate: 0.0015,
            mvrv: 3.8,
            sopr: 1.08,
            ..Default::default()
        }
    }

    pub fn funding_spike() -> MockData {
        MockData {
            fear_greed_index: 70,
            fear_greed_label: "Greed".to_string(),
            funding_rate: 0.0012,
            open_interest: 5000.0,
            ..Default::default()
        }
    }

    pub fn capitulation() -> MockData {
        MockData {
            fear_greed_index: 5,
            fear_greed_label: "Extreme Fear".to_string(),
            funding_rate: -0.002,
            mvrv: 0.6,
            sopr: 0.88,
            open_interest: 200.0,
            ..Default::default()
        }
    }

    pub fn neutral() -> MockData {
        MockData::default()
    }

    pub fn exchange_hack() -> MockData {
        MockData {
            fear_greed_index: 15,
            fear_greed_label: "Extreme Fear".to_string(),
            funding_rate: -0.003,
            news_headlines: vec![
                "BREAKING: Major exchange reports security breach".to_string(),
                "Users report missing funds on exchange".to_string(),
            ],
            ..Default::default()
        }
    }

    pub fn etf_approval() -> MockData {
        MockData {
            fear_greed_index: 75,
            fear_greed_label: "Greed".to_string(),
            funding_rate: 0.0008,
            news_headlines: vec![
                "SEC approves spot Bitcoin ETF".to_string(),
                "Institutional inflows expected to surge".to_string(),
            ],
            ..Default::default()
        }
    }

    pub fn fomc_rate_hike() -> MockData {
        MockData {
            fear_greed_index: 25,
            fear_greed_label: "Fear".to_string(),
            funding_rate: -0.0003,
            news_headlines: vec![
                "Federal Reserve raises interest rates by 25 basis points".to_string(),
                "Risk assets sell off on hawkish Fed commentary".to_string(),
            ],
            ..Default::default()
        }
    }
}
