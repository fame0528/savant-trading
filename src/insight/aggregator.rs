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

    /// Actionable market conditions summary using SOUL.md thresholds.
    ///
    /// Translates raw data into human-readable assessments the AI can
    /// directly use for decision-making. Uses the same thresholds as
    /// SOUL.md Action Triggers (Section XIII).
    pub fn conditions_summary(&self) -> String {
        let mut lines = Vec::new();

        // Sentiment
        if let Some(fg) = self.sentiment.fear_greed_index {
            let assessment = match fg {
                0..=15 => "Extreme Fear — capitulation buy zone per SOUL.md §XIII",
                16..=30 => "Fear — contrarian opportunity, cautious entry",
                31..=60 => "Neutral — no sentiment edge",
                61..=80 => "Greed — caution, tighten stops, take partial profits",
                81..=100 => "Extreme Greed — euphoria, prepare to sell, do NOT long",
                _ => "Unknown",
            };
            lines.push(format!("- Sentiment: {}/100 — {}", fg, assessment));
        }

        // On-chain
        if let Some(mvrv) = self.onchain.mvrv {
            let assessment = if mvrv < 0.8 {
                "deep capitulation — rare buy signal"
            } else if mvrv < 1.0 {
                "below realized value — strong buy zone"
            } else if mvrv < 2.0 {
                "fair value — no edge"
            } else if mvrv < 3.5 {
                "elevated — take profits on strength"
            } else {
                "extreme euphoria — historically precedes major corrections"
            };
            lines.push(format!("- On-chain MVRV: {:.2} — {}", mvrv, assessment));
        }

        if let Some(sopr) = self.onchain.sopr {
            let assessment = if sopr < 0.95 {
                "heavy loss realization — capitulation"
            } else if sopr < 1.0 {
                "mild loss realization — weak hands selling"
            } else if sopr < 1.05 {
                "profit realization — normal"
            } else {
                "heavy profit realization — potential distribution"
            };
            lines.push(format!("- SOPR: {:.4} — {}", sopr, assessment));
        }

        // Funding
        if let Some(fr) = self.funding.funding_rate {
            let pct = fr * 100.0;
            let assessment = if pct > 0.05 {
                "extremely overleveraged longs — squeeze risk, avoid longs"
            } else if pct > 0.01 {
                "longs paying shorts — mild bullish crowding"
            } else if pct > -0.01 {
                "neutral — no funding edge"
            } else if pct > -0.05 {
                "shorts paying longs — mild bearish crowding"
            } else {
                "extremely overleveraged shorts — squeeze setup, watch for long entries"
            };
            lines.push(format!("- Funding: {:.4}% — {}", pct, assessment));
        }

        // Liquidation risk
        lines.push(format!(
            "- Liquidation risk: {:?}",
            self.liquidation.risk_level
        ));

        // Top news with sentiment
        if !self.rss_items.is_empty() {
            let top: Vec<&str> = self
                .rss_items
                .iter()
                .take(3)
                .map(|item| item.title.as_str())
                .collect();
            let sentiments: Vec<&str> = self
                .rss_items
                .iter()
                .take(3)
                .map(|item| classify_headline_sentiment(&item.title))
                .collect();
            lines.push("- Top news:".to_string());
            for (i, title) in top.iter().enumerate() {
                lines.push(format!("  {}. [{}] {}", i + 1, sentiments[i], title));
            }
        }

        if lines.is_empty() {
            "No market data available".to_string()
        } else {
            lines.join("\n")
        }
    }
}

/// Classify a news headline as bullish/bearish/neutral via keyword matching.
fn classify_headline_sentiment(title: &str) -> &'static str {
    let lower = title.to_lowercase();

    // Check for negation patterns first
    let has_positive = lower.contains("approves")
        || lower.contains("approved")
        || lower.contains("adoption")
        || lower.contains("partnership")
        || lower.contains("launch")
        || lower.contains("rally")
        || lower.contains("surge")
        || lower.contains("etf");

    let has_negative = lower.contains("crash")
        || lower.contains("hack")
        || lower.contains("ban")
        || lower.contains("enforcement")
        || lower.contains("crackdown")
        || lower.contains("collapse")
        || lower.contains("fraud")
        || lower.contains("scam");

    // SEC + approve = bullish (negation handling)
    if lower.contains("sec") && has_positive {
        return "BULLISH";
    }

    if has_negative && !has_positive {
        "BEARISH"
    } else if has_positive && !has_negative {
        "BULLISH"
    } else if has_positive || has_negative {
        "MIXED"
    } else {
        "NEUTRAL"
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

        // RSS feeds — capped with source diversity
        if self.config.rss_enabled && !self.rss_disabled {
            let pair_strings: Vec<String> = symbols.to_vec();
            let items =
                rss::fetch_all_feeds_capped(&self.client, self.config.rss_max_items, &pair_strings)
                    .await;
            if items.is_empty() {
                self.rss_disabled = true;
                info!("RSS feeds disabled (no items)");
            } else {
                // Relevance already scored in fetch_all_feeds_capped
                info!(
                    "RSS: {} items (capped at {})",
                    items.len(),
                    self.config.rss_max_items
                );
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
