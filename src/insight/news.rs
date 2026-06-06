//! Breaking news and social sentiment data.
//!
//! Uses CoinGecko trending (free, no key) for social sentiment.
//! RSS feeds (in rss.rs) provide actual news coverage.

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// News and social sentiment data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewsData {
    /// Overall sentiment score (-1.0 to 1.0) derived from trending coin price changes
    pub sentiment_score: Option<f64>,
    /// Trending coins by social volume
    pub trending_coins: Vec<TrendingCoin>,
    /// Trending topics/hashtags (top symbols)
    pub trending_topics: Vec<String>,
}

/// A trending coin from CoinGecko.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingCoin {
    pub name: String,
    pub symbol: String,
    pub market_cap_rank: Option<u64>,
    pub price_usd: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub score: Option<u64>,
}

/// CoinGecko trending response.
#[derive(Debug, Deserialize)]
struct CoinGeckoTrendingResponse {
    coins: Option<Vec<CoinGeckoTrendingItem>>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoTrendingItem {
    item: Option<CoinGeckoTrendingCoin>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoTrendingCoin {
    name: Option<String>,
    symbol: Option<String>,
    market_cap_rank: Option<u64>,
    score: Option<u64>,
    data: Option<CoinGeckoTrendingData>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoTrendingData {
    price: Option<f64>,
    price_change_percentage_24h: Option<serde_json::Value>,
}

/// Fetch news and social sentiment data.
///
/// Uses CoinGecko trending (free, no key) to derive social sentiment.
/// Actual news coverage comes from RSS feeds (see rss.rs).
pub async fn fetch_news(client: &reqwest::Client) -> NewsData {
    let mut data = NewsData::default();

    // Fetch CoinGecko trending (free, no key)
    fetch_coingecko_trending(client, &mut data).await;

    // Derive sentiment from trending coin price changes
    if !data.trending_coins.is_empty() {
        let avg_change: f64 = data
            .trending_coins
            .iter()
            .filter_map(|c| c.price_change_24h)
            .sum::<f64>()
            / data.trending_coins.len() as f64;

        // Normalize to -1.0 to 1.0 range (clamp at ±10%)
        data.sentiment_score = Some((avg_change / 10.0).clamp(-1.0, 1.0));

        info!(
            "News sentiment: {:.2} (avg trending change: {:.2}%)",
            data.sentiment_score.unwrap_or(0.0),
            avg_change
        );
    }

    data
}

/// Fetch trending coins from CoinGecko (Demo API key supported).
async fn fetch_coingecko_trending(client: &reqwest::Client, data: &mut NewsData) {
    let mut req = client.get("https://api.coingecko.com/api/v3/search/trending");
    if let Ok(key) = std::env::var("COINGECKO_API_KEY") {
        req = req.header("x-cg-demo-api-key", key);
    }
    match req.send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("CoinGecko trending API returned HTTP {}", resp.status());
                return;
            }

            match resp.json::<CoinGeckoTrendingResponse>().await {
                Ok(trending) => {
                    if let Some(coins) = trending.coins {
                        for item in coins {
                            if let Some(coin) = item.item {
                                let price_change = coin
                                    .data
                                    .as_ref()
                                    .and_then(|d| d.price_change_percentage_24h.as_ref())
                                    .and_then(|v| v.get("usd"))
                                    .and_then(|v| v.as_f64());

                                let price = coin.data.as_ref().and_then(|d| d.price);

                                data.trending_coins.push(TrendingCoin {
                                    name: coin.name.unwrap_or_default(),
                                    symbol: coin.symbol.unwrap_or_default(),
                                    market_cap_rank: coin.market_cap_rank,
                                    price_usd: price,
                                    price_change_24h: price_change,
                                    score: coin.score,
                                });
                            }
                        }

                        info!(
                            "CoinGecko trending: {} coins, top: {}",
                            data.trending_coins.len(),
                            data.trending_coins
                                .first()
                                .map(|c| c.symbol.as_str())
                                .unwrap_or("none")
                        );

                        // Extract trending topics from coin names
                        data.trending_topics = data
                            .trending_coins
                            .iter()
                            .take(5)
                            .map(|c| c.symbol.clone())
                            .collect();
                    }
                }
                Err(e) => warn!("CoinGecko trending response parse error: {}", e),
            }
        }
        Err(e) => warn!("CoinGecko trending API request failed: {}", e),
    }
}
