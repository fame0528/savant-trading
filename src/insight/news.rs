//! Breaking news and social sentiment data.
//!
//! Uses CoinGecko trending (free, no key) as primary source.
//! CryptoPanic integration available with API key.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// A single news item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub source: String,
    pub url: Option<String>,
    pub sentiment: Option<f64>, // -1.0 to 1.0
    pub timestamp: Option<String>,
    pub coins: Vec<String>,
}

/// News and social sentiment data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewsData {
    /// Overall sentiment score (-1.0 to 1.0)
    pub sentiment_score: Option<f64>,
    /// Trending coins by social volume
    pub trending_coins: Vec<TrendingCoin>,
    /// Recent breaking news
    pub breaking_news: Vec<NewsItem>,
    /// Trending topics/hashtags
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
/// Uses CoinGecko trending (free, no key) as primary.
/// Uses CryptoPanic (requires key) for news if available.
pub async fn fetch_news(client: &reqwest::Client, api_key: Option<&str>) -> NewsData {
    let mut data = NewsData::default();

    // Always fetch CoinGecko trending (free)
    fetch_coingecko_trending(client, &mut data).await;

    // Try CryptoPanic if key is available
    if let Some(key) = api_key {
        if !key.is_empty() {
            fetch_cryptopanic(client, key, &mut data).await;
        }
    } else {
        debug!("CryptoPanic API key not configured — using CoinGecko trending only");
    }

    // Derive sentiment from trending data
    if !data.trending_coins.is_empty() {
        let avg_change: f64 = data
            .trending_coins
            .iter()
            .filter_map(|c| c.price_change_24h)
            .sum::<f64>()
            / data.trending_coins.len() as f64;

        // Normalize to -1.0 to 1.0 range (clamp at ±10%)
        data.sentiment_score = Some((avg_change / 10.0).clamp(-1.0, 1.0));
    }

    data
}

/// Fetch trending coins from CoinGecko (free, no key).
async fn fetch_coingecko_trending(client: &reqwest::Client, data: &mut NewsData) {
    match client
        .get("https://api.coingecko.com/api/v3/search/trending")
        .send()
        .await
    {
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

/// Fetch news from CryptoPanic (requires API key).
async fn fetch_cryptopanic(client: &reqwest::Client, api_key: &str, data: &mut NewsData) {
    let url = format!(
        "https://cryptopanic.com/api/v1/posts/?auth_token={}&public=true&kind=news&filter=hot",
        api_key
    );

    match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                warn!("CryptoPanic API returned HTTP {}", resp.status());
                return;
            }

            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
                    for item in results.iter().take(10) {
                        let title = item
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();

                        let source = item
                            .get("source")
                            .and_then(|s| s.get("title"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let url = item
                            .get("url")
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string());

                        let published = item
                            .get("published_at")
                            .and_then(|p| p.as_str())
                            .map(|s| s.to_string());

                        let coins: Vec<String> = item
                            .get("currencies")
                            .and_then(|c| c.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|c| c.get("code").and_then(|c| c.as_str()))
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        // Parse sentiment from votes
                        let votes = item.get("votes");
                        let positive = votes
                            .and_then(|v| v.get("positive"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let negative = votes
                            .and_then(|v| v.get("negative"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let total = positive + negative;
                        let sentiment = if total > 0 {
                            Some((positive as f64 - negative as f64) / total as f64)
                        } else {
                            None
                        };

                        data.breaking_news.push(NewsItem {
                            title,
                            source,
                            url,
                            sentiment,
                            timestamp: published,
                            coins,
                        });
                    }

                    info!(
                        "CryptoPanic: {} news items fetched",
                        data.breaking_news.len()
                    );
                }
            }
        }
        Err(e) => warn!("CryptoPanic API request failed: {}", e),
    }
}
