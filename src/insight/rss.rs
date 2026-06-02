//! RSS feed fetcher and parser for crypto news.
//!
//! Fetches from 8 free RSS feeds concurrently, parses XML with quick-xml,
//! deduplicates by URL, and scores relevance to current trading pairs.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// An RSS feed source.
#[derive(Debug, Clone)]
pub struct RssFeed {
    pub url: &'static str,
    pub name: &'static str,
    pub category: RssCategory,
}

/// Category of RSS feed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RssCategory {
    Macro,
    Defi,
    Institutional,
    Bitcoin,
    Ethereum,
    Mainstream,
}

/// A parsed RSS item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub pub_date: Option<String>,
    pub description: String,
    pub categories: Vec<String>,
    pub source: String,
    pub relevance_score: f64,
}

/// All configured RSS feeds (15 sources).
const FEEDS: &[RssFeed] = &[
    // === Crypto-native macro & sentiment ===
    RssFeed {
        url: "https://cointelegraph.com/rss",
        name: "Cointelegraph",
        category: RssCategory::Macro,
    },
    RssFeed {
        url: "https://www.coindesk.com/arc/outboundfeeds/rss/?outputType=xml",
        name: "CoinDesk",
        category: RssCategory::Macro,
    },
    RssFeed {
        url: "https://cryptoslate.com/feed/",
        name: "CryptoSlate",
        category: RssCategory::Macro,
    },
    RssFeed {
        url: "https://decrypt.co/feed",
        name: "Decrypt",
        category: RssCategory::Macro,
    },
    RssFeed {
        url: "https://cryptonews.com/news/feed/",
        name: "CryptoNews",
        category: RssCategory::Macro,
    },
    RssFeed {
        url: "https://cryptopotato.com/feed/",
        name: "CryptoPotato",
        category: RssCategory::Macro,
    },
    RssFeed {
        url: "https://www.cryptobreaking.com/feed/",
        name: "CryptoBreaking",
        category: RssCategory::Macro,
    },
    // === DeFi & institutional ===
    RssFeed {
        url: "https://thedefiant.io/feed/",
        name: "The Defiant",
        category: RssCategory::Defi,
    },
    RssFeed {
        url: "https://smartliquidity.info/feed/",
        name: "SmartLiquidity",
        category: RssCategory::Defi,
    },
    RssFeed {
        url: "https://blockworks.co/feed",
        name: "Blockworks",
        category: RssCategory::Institutional,
    },
    // === Bitcoin-specific ===
    RssFeed {
        url: "https://bitcoinmagazine.com/feed",
        name: "Bitcoin Magazine",
        category: RssCategory::Bitcoin,
    },
    // === Ethereum-specific ===
    RssFeed {
        url: "https://benjaminion.xyz/newineth2/rss_feed.xml",
        name: "Ethereum 2.0",
        category: RssCategory::Ethereum,
    },
    // === Mainstream finance (macro context) ===
    RssFeed {
        url: "https://finance.yahoo.com/news/rssindex",
        name: "Yahoo Finance",
        category: RssCategory::Mainstream,
    },
    RssFeed {
        url: "https://www.cnbc.com/id/10000664/device/rss/rss.html",
        name: "CNBC",
        category: RssCategory::Mainstream,
    },
    // === Regional ===
    RssFeed {
        url: "https://kriptonovini.bg/rss.xml",
        name: "KriptoNovini",
        category: RssCategory::Macro,
    },
];

/// Fetch all RSS feeds concurrently with per-feed timeout, source diversity,
/// and max items cap.
///
/// - Per-feed timeout: 5s (prevents slow feeds blocking)
/// - Source diversity: top 2 per source, then sort all by relevance, take top N
/// - Cap: max_items from config (default 10)
pub async fn fetch_all_feeds_capped(
    client: &reqwest::Client,
    max_items: usize,
    pairs: &[String],
) -> Vec<RssItem> {
    let mut handles = Vec::new();

    for feed in FEEDS {
        let url = feed.url;
        let name = feed.name;
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            // Per-feed timeout: 5 seconds
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                fetch_single_feed(&client, url, name),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(format!("{}: timed out after 5s", name)),
            }
        }));
    }

    let mut all_items = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(items)) => all_items.extend(items),
            Ok(Err(e)) => warn!("RSS feed task error: {}", e),
            Err(e) => warn!("RSS feed task panicked: {}", e),
        }
    }

    // Deduplicate by link
    all_items.sort_by(|a, b| a.link.cmp(&b.link));
    all_items.dedup_by(|a, b| a.link == b.link);

    // Score relevance
    score_relevance(&mut all_items, pairs);

    // Source diversity: group by source, take top 2 per source
    let mut by_source: std::collections::HashMap<String, Vec<RssItem>> =
        std::collections::HashMap::new();
    for item in all_items {
        by_source.entry(item.source.clone()).or_default().push(item);
    }
    let mut diverse_items: Vec<RssItem> = Vec::new();
    for (_source, mut items) in by_source {
        items.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        diverse_items.extend(items.into_iter().take(2));
    }

    // Sort all by relevance, take top max_items
    diverse_items.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    diverse_items.truncate(max_items);

    info!(
        "RSS: {} items from {} feeds (capped at {}, source-diverse)",
        diverse_items.len(),
        FEEDS.len(),
        max_items
    );

    diverse_items
}

/// Fetch all RSS feeds concurrently and return parsed items (legacy, uncapped).
pub async fn fetch_all_feeds(client: &reqwest::Client) -> Vec<RssItem> {
    let mut handles = Vec::new();

    for feed in FEEDS {
        let url = feed.url;
        let name = feed.name;
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            fetch_single_feed(&client, url, name).await
        }));
    }

    let mut all_items = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(items)) => all_items.extend(items),
            Ok(Err(e)) => warn!("RSS feed task error: {}", e),
            Err(e) => warn!("RSS feed task panicked: {}", e),
        }
    }

    // Deduplicate by link
    all_items.sort_by(|a, b| a.link.cmp(&b.link));
    all_items.dedup_by(|a, b| a.link == b.link);

    // Sort by date (newest first)
    all_items.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));

    info!("RSS: {} items from {} feeds", all_items.len(), FEEDS.len());

    all_items
}

/// Fetch and parse a single RSS feed.
async fn fetch_single_feed(
    client: &reqwest::Client,
    url: &str,
    name: &str,
) -> Result<Vec<RssItem>, String> {
    let resp = client
        .get(url)
        .header("User-Agent", "SavantTrading/0.1")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("{}: HTTP error: {}", name, e))?;

    if !resp.status().is_success() {
        return Err(format!("{}: HTTP {}", name, resp.status()));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("{}: body read error: {}", name, e))?;

    let items = parse_rss_xml(&body, name);
    debug!("{}: parsed {} items", name, items.len());
    Ok(items)
}

/// Parse RSS XML into RssItem structs using quick-xml.
fn parse_rss_xml(xml: &str, source: &str) -> Vec<RssItem> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut items = Vec::new();
    let mut in_item = false;
    let mut current_tag = String::new();
    let mut title = String::new();
    let mut link = String::new();
    let mut pub_date = String::new();
    let mut description = String::new();
    let mut categories = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "item" {
                    in_item = true;
                    title.clear();
                    link.clear();
                    pub_date.clear();
                    description.clear();
                    categories.clear();
                } else if in_item {
                    current_tag = tag;
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "item" && in_item {
                    in_item = false;
                    if !title.is_empty() {
                        items.push(RssItem {
                            title: title.trim().to_string(),
                            link: if link.is_empty() {
                                String::new()
                            } else {
                                link.trim().to_string()
                            },
                            pub_date: if pub_date.is_empty() {
                                None
                            } else {
                                Some(pub_date.trim().to_string())
                            },
                            description: truncate_description(description.trim()),
                            categories: categories.clone(),
                            source: source.to_string(),
                            relevance_score: 0.0,
                        });
                    }
                }
                current_tag.clear();
            }
            Ok(Event::Text(ref e)) => {
                if in_item {
                    let text = e.unescape().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "title" => title.push_str(&text),
                        "link" => link.push_str(&text),
                        "pubDate" => pub_date.push_str(&text),
                        "description" => description.push_str(&text),
                        "category" => categories.push(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::CData(ref e)) => {
                if in_item && current_tag == "description" {
                    if let Ok(text) = std::str::from_utf8(e.as_ref()) {
                        description.push_str(text);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => continue,
            _ => {}
        }
        buf.clear();
    }

    items
}

/// Truncate description to max 200 chars, strip HTML tags.
fn truncate_description(desc: &str) -> String {
    // Strip HTML tags
    let stripped: String = desc
        .chars()
        .fold((String::new(), false), |(mut s, in_tag), c| match c {
            '<' => (s, true),
            '>' => (s, false),
            _ if in_tag => (s, in_tag),
            _ => {
                s.push(c);
                (s, in_tag)
            }
        })
        .0;

    if stripped.len() > 200 {
        let boundary = stripped.floor_char_boundary(197);
        format!("{}...", &stripped[..boundary])
    } else {
        stripped
    }
}

/// Score relevance of RSS items to current trading pairs.
pub fn score_relevance(items: &mut [RssItem], pairs: &[String]) {
    // Extract keywords from pairs (e.g., "BTC/USD" → ["btc", "bitcoin"])
    let mut keywords: Vec<String> = Vec::new();
    for pair in pairs {
        let base = pair.split('/').next().unwrap_or(pair).to_lowercase();
        keywords.push(base.clone());
        // Map common symbols to full names
        match base.as_str() {
            "btc" => keywords.push("bitcoin".to_string()),
            "eth" => keywords.push("ethereum".to_string()),
            "sol" => keywords.push("solana".to_string()),
            "xrp" => keywords.push("ripple".to_string()),
            "ada" => keywords.push("cardano".to_string()),
            "doge" => keywords.push("dogecoin".to_string()),
            "dot" => keywords.push("polkadot".to_string()),
            "avax" => keywords.push("avalanche".to_string()),
            "matic" | "pol" => keywords.push("polygon".to_string()),
            "link" => keywords.push("chainlink".to_string()),
            _ => {}
        }
    }

    // General high-signal keywords
    keywords.extend([
        "bitcoin".to_string(),
        "crypto".to_string(),
        "etf".to_string(),
        "regulation".to_string(),
        "fed".to_string(),
        "fomc".to_string(),
        "sec".to_string(),
        "halving".to_string(),
        "defi".to_string(),
    ]);

    for item in items.iter_mut() {
        let text = format!(
            "{} {} {}",
            item.title.to_lowercase(),
            item.description.to_lowercase(),
            item.categories
                .iter()
                .map(|c| c.to_lowercase())
                .collect::<Vec<_>>()
                .join(" ")
        );

        let matches = keywords
            .iter()
            .filter(|kw| text.contains(kw.as_str()))
            .count();

        item.relevance_score = matches as f64;
    }

    // Sort by relevance (highest first), stable sort preserves date order for ties
    items.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Get top N most relevant items as a formatted string for AI context.
pub fn format_for_context(items: &[RssItem], max_items: usize) -> String {
    if items.is_empty() {
        return "No recent news available.".to_string();
    }

    let mut output = String::new();
    for (i, item) in items.iter().take(max_items).enumerate() {
        output.push_str(&format!(
            "{}. [{}] {}\n   {}\n",
            i + 1,
            item.source,
            item.title,
            item.description,
        ));
    }

    output
}
