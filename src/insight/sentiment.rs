//! Fear & Greed Index and BTC Dominance — free APIs, no auth required.

use serde::{Deserialize, Serialize};

/// Sentiment data from external sources.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SentimentData {
    /// Fear & Greed Index (0-100)
    pub fear_greed_index: Option<u32>,
    /// Human-readable label
    pub fear_greed_label: Option<String>,
    /// BTC dominance as percentage of total crypto market cap
    pub btc_dominance: Option<f64>,
    /// Total crypto market cap in USD
    pub total_market_cap: Option<f64>,
}

/// Fetch Fear & Greed Index from alternative.me (free, no auth).
pub async fn fetch_fear_greed(
    client: &reqwest::Client,
) -> Option<(u32, String)> {
    let url = "https://api.alternative.me/fng/?limit=1";
    let resp = client.get(url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    let data = json.get("data")?.get(0)?;
    let value: u32 = data.get("value")?.as_str()?.parse().ok()?;
    let label = data
        .get("value_classification")?
        .as_str()?
        .to_string();
    Some((value, label))
}

/// Fetch BTC dominance from CoinGecko (free, no auth).
pub async fn fetch_btc_dominance(
    client: &reqwest::Client,
) -> Option<(f64, f64)> {
    let url = "https://api.coingecko.com/api/v3/global";
    let resp = client.get(url).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    let data = json.get("data")?;
    let btc_dom = data.get("market_cap_percentage")?.get("btc")?.as_f64()?;
    let total_mcap = data.get("total_market_cap")?.get("usd")?.as_f64()?;
    Some((btc_dom, total_mcap))
}

/// Fetch all sentiment data.
pub async fn fetch_all(client: &reqwest::Client) -> SentimentData {
    let mut data = SentimentData::default();

    if let Some((fg, label)) = fetch_fear_greed(client).await {
        data.fear_greed_index = Some(fg);
        data.fear_greed_label = Some(label);
    }

    if let Some((dom, mcap)) = fetch_btc_dominance(client).await {
        data.btc_dominance = Some(dom);
        data.total_market_cap = Some(mcap);
    }

    data
}
