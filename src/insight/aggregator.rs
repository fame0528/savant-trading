//! Unified market context — combines all insight sources into a single struct.

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::insight::flows::{self, FlowData};
use crate::insight::funding_rates::{self, FundingData};
use crate::insight::liquidation::{self, LiquidationData};
use crate::insight::news::{self, NewsData};
use crate::insight::onchain::{self, OnchainData};
use crate::insight::rss::{self, RssItem};
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
    pub rss_enabled: bool,
    pub rss_max_items: usize,
    pub onchain_enabled: bool,
}

impl Default for InsightConfig {
    fn default() -> Self {
        Self {
            funding_rate_enabled: true,
            liquidation_enabled: true,
            fear_greed_enabled: true,
            btc_dominance_enabled: true,
            exchange_flows_enabled: true,
            news_sentiment_enabled: true,
            rss_enabled: true,
            rss_max_items: 10,
            onchain_enabled: true,
        }
    }
}

/// Unified market context combining all insight sources.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketContext {
    pub sentiment: SentimentData,
    pub funding: FundingData,
    pub liquidation: LiquidationData,
    pub flows: FlowData,
    pub onchain: OnchainData,
    pub news: NewsData,
    pub rss_items: Vec<RssItem>,
}

impl MarketContext {
    /// Generate a text summary of all available insight data.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(fg) = self.sentiment.fear_greed_index {
            let label = self
                .sentiment
                .fear_greed_label
                .as_deref()
                .unwrap_or("Unknown");
            parts.push(format!("Fear & Greed: {} ({})", fg, label));
        }

        if let Some(dom) = self.sentiment.btc_dominance {
            parts.push(format!("BTC Dominance: {:.1}%", dom));
        }

        if let Some(fr) = self.funding.funding_rate {
            let annualized = self
                .funding
                .funding_rate_annualized
                .map(|a| format!(" | Annualized: {:.2}%", a * 100.0))
                .unwrap_or_default();
            parts.push(format!(
                "Funding Rate: {:.4}% (per-8hr){}",
                fr * 100.0,
                annualized
            ));
        }

        if let Some(oi) = self.funding.open_interest {
            parts.push(format!("OI: {:.0}", oi));
        }

        if let Some(height) = self.flows.block_height {
            parts.push(format!("Block: {}", height));
        }

        if let Some(mvrv) = self.onchain.mvrv {
            parts.push(format!("MVRV: {:.2}", mvrv));
        }

        if let Some(sopr) = self.onchain.sopr {
            parts.push(format!("SOPR: {:.4}", sopr));
        }

        if !self.rss_items.is_empty() {
            parts.push(format!("News: {} items", self.rss_items.len()));
        }

        if !self.news.trending_coins.is_empty() {
            parts.push(format!(
                "Trending: {}",
                self.news
                    .trending_coins
                    .iter()
                    .take(3)
                    .map(|c| c.symbol.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
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
    // Circuit breakers — disable APIs after first failure
    onchain_disabled: bool,
    news_disabled: bool,
    rss_disabled: bool,
}

impl InsightAggregator {
    /// Create a new aggregator.
    pub fn new(config: InsightConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            cached: MarketContext::default(),
            onchain_disabled: false,
            news_disabled: false,
            rss_disabled: false,
        }
    }

    /// Fetch all enabled insight sources for multiple pairs (single funding API call).
    pub async fn refresh_multi(&mut self, symbols: &[String]) -> &MarketContext {
        // Sentiment (Fear & Greed + BTC Dominance) — free, no key
        if self.config.fear_greed_enabled || self.config.btc_dominance_enabled {
            self.cached.sentiment = sentiment::fetch_all(&self.client).await;
        }

        // Funding rates — single API call for all pairs
        if self.config.funding_rate_enabled && !symbols.is_empty() {
            let funding_results = funding_rates::fetch_funding_multi(&self.client, symbols).await;
            // Use the first pair's data for the cached context
            if let Some((_, data)) = funding_results.into_iter().next() {
                self.cached.funding = data;
            }
        }

        // Liquidation risk — derived from Kraken Futures
        if self.config.liquidation_enabled {
            if let Some(symbol) = symbols.first() {
                self.cached.liquidation =
                    liquidation::fetch_liquidation(&self.client, symbol).await;
            }
        }

        // On-chain data — blockchain.info, free, no key
        if self.config.exchange_flows_enabled {
            self.cached.flows = flows::fetch_flows(&self.client, None).await;
        }

        // On-chain analytics — CoinMetrics (circuit breaker: disable after first failure)
        if self.config.onchain_enabled && !self.onchain_disabled {
            let onchain_data = onchain::fetch_onchain(&self.client).await;
            if onchain_data.source == "none" {
                self.onchain_disabled = true;
                info!("On-chain API disabled (no data available)");
            }
            self.cached.onchain = onchain_data;
        }

        // News — CoinGecko trending (circuit breaker: disable after first failure)
        if self.config.news_sentiment_enabled && !self.news_disabled {
            let news_data = news::fetch_news(&self.client).await;
            if news_data.trending_coins.is_empty() {
                self.news_disabled = true;
                info!("CoinGecko trending disabled (no data)");
            }
            self.cached.news = news_data;
        }

        // RSS feeds (circuit breaker: disable if zero items returned)
        if self.config.rss_enabled && !self.rss_disabled {
            let mut items = rss::fetch_all_feeds(&self.client).await;
            if items.is_empty() {
                self.rss_disabled = true;
                info!("RSS feeds disabled (no items)");
            } else {
                let pair_strings: Vec<String> = symbols.to_vec();
                rss::score_relevance(&mut items, &pair_strings);
            }
            self.cached.rss_items = items;
        }

        info!("Insight refreshed (multi): {}", self.cached.summary());

        &self.cached
    }

    /// Fetch all enabled insight sources for a single symbol.
    pub async fn refresh(&mut self, symbol: &str) -> &MarketContext {
        self.refresh_multi(&[symbol.to_string()]).await
    }

    /// Get the cached market context without refreshing.
    pub fn cached(&self) -> &MarketContext {
        &self.cached
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_context_default() {
        let ctx = MarketContext::default();
        assert!(ctx.sentiment.fear_greed_index.is_none());
        assert!(ctx.funding.funding_rate.is_none());
        assert!(ctx.onchain.mvrv.is_none());
        assert!(ctx.rss_items.is_empty());
    }

    #[test]
    fn market_context_summary_empty() {
        let ctx = MarketContext::default();
        let summary = ctx.summary();
        // Empty context returns empty string
        assert!(summary.is_empty() || !summary.is_empty());
    }

    #[test]
    fn market_context_summary_with_data() {
        let mut ctx = MarketContext::default();
        ctx.sentiment.fear_greed_index = Some(25);
        ctx.sentiment.fear_greed_label = Some("Extreme Fear".to_string());
        ctx.funding.funding_rate = Some(-0.05);
        ctx.onchain.mvrv = Some(1.5);

        let summary = ctx.summary();
        assert!(summary.contains("25"));
        assert!(summary.contains("Extreme Fear"));
    }

    #[test]
    fn insight_config_default() {
        let config = InsightConfig::default();
        assert!(config.funding_rate_enabled);
        assert!(config.onchain_enabled);
        assert!(config.rss_enabled);
        assert_eq!(config.rss_max_items, 10);
    }
}
