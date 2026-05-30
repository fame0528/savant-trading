//! Unified market context — combines all insight sources into a single struct.

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::insight::flows::{self, FlowData};
use crate::insight::funding_rates::{self, FundingData};
use crate::insight::liquidation::{self, LiquidationData};
use crate::insight::news::{self, NewsData};
use crate::insight::sentiment::{self, SentimentData};

/// Configuration for which insight sources are enabled.
#[derive(Debug, Clone)]
pub struct InsightConfig {
    pub funding_rate_enabled: bool,
    pub liquidation_enabled: bool,
    pub fear_greed_enabled: bool,
    pub btc_dominance_enabled: bool,
    pub exchange_flows_enabled: bool,
    pub news_sentiment_enabled: bool,
    pub funding_api_key: Option<String>,
    pub liquidation_api_key: Option<String>,
    pub news_api_key: Option<String>,
}

/// Unified market context combining all insight sources.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketContext {
    pub sentiment: SentimentData,
    pub funding: FundingData,
    pub liquidation: LiquidationData,
    pub flows: FlowData,
    pub news: NewsData,
}

impl MarketContext {
    /// Generate a text summary of all available insight data.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(fg) = self.sentiment.fear_greed_index {
            let label = self.sentiment.fear_greed_label.as_deref().unwrap_or("Unknown");
            parts.push(format!("Fear & Greed: {} ({})", fg, label));
        }

        if let Some(dom) = self.sentiment.btc_dominance {
            parts.push(format!("BTC Dominance: {:.1}%", dom));
        }

        if let Some(fr) = self.funding.funding_rate {
            parts.push(format!("Funding Rate: {:.4}%", fr * 100.0));
        }

        if let Some(ls) = self.funding.long_short_ratio {
            parts.push(format!("Long/Short Ratio: {:.2}", ls));
        }

        if parts.is_empty() {
            "No insight data available".to_string()
        } else {
            parts.join(" | ")
        }
    }
}

/// Aggregates all insight sources with caching and graceful failure handling.
pub struct InsightAggregator {
    client: reqwest::Client,
    config: InsightConfig,
    cached: MarketContext,
}

impl InsightAggregator {
    /// Create a new aggregator.
    pub fn new(config: InsightConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            cached: MarketContext::default(),
        }
    }

    /// Fetch all enabled insight sources and update the cache.
    ///
    /// Each source is fetched independently — failures are logged but don't
    /// prevent other sources from being fetched.
    pub async fn refresh(&mut self, symbol: &str) -> &MarketContext {
        // Sentiment (Fear & Greed + BTC Dominance)
        if self.config.fear_greed_enabled || self.config.btc_dominance_enabled {
            self.cached.sentiment = sentiment::fetch_all(&self.client).await;
        }

        // Funding rates
        if self.config.funding_rate_enabled {
            self.cached.funding = funding_rates::fetch_funding(
                &self.client,
                symbol,
                self.config.funding_api_key.as_deref(),
            )
            .await;
        }

        // Liquidation
        if self.config.liquidation_enabled {
            self.cached.liquidation = liquidation::fetch_liquidation(
                &self.client,
                symbol,
                self.config.liquidation_api_key.as_deref(),
            )
            .await;
        }

        // Exchange flows
        if self.config.exchange_flows_enabled {
            self.cached.flows = flows::fetch_flows(
                &self.client,
                None, // TODO: API key
            )
            .await;
        }

        // News
        if self.config.news_sentiment_enabled {
            self.cached.news = news::fetch_news(
                &self.client,
                self.config.news_api_key.as_deref(),
            )
            .await;
        }

        &self.cached
    }

    /// Get the cached market context without refreshing.
    pub fn cached(&self) -> &MarketContext {
        &self.cached
    }
}
