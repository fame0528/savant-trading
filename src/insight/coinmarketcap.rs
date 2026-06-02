//! CoinMarketCap API client — discovers high-volume, high-volatility pairs.
//!
//! Uses CMC's free tier (10K calls/month) to:
//! 1. Fetch top coins by market cap
//! 2. Filter for coins available on Kraken with USD pairs
//! 3. Rank by 24h volume and volatility (24h % change)
//! 4. Feed into the trading engine's pair selection

use serde::Deserialize;
use tracing::{info, warn};

const CMC_BASE_URL: &str = "https://pro-api.coinmarketcap.com/v1";

/// CoinMarketCap cryptocurrency listing.
#[derive(Debug, Clone, Deserialize)]
pub struct CmcCoin {
    pub id: u32,
    pub name: String,
    pub symbol: String,
    #[serde(rename = "cmc_rank")]
    pub rank: Option<u32>,
    pub quote: CmcQuote,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmcQuote {
    #[serde(rename = "USD")]
    pub usd: CmcUsdQuote,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmcUsdQuote {
    pub price: f64,
    #[serde(rename = "volume_24h")]
    pub volume_24h: f64,
    #[serde(rename = "percent_change_24h")]
    pub percent_change_24h: f64,
    #[serde(rename = "percent_change_7d")]
    pub percent_change_7d: Option<f64>,
    #[serde(rename = "market_cap")]
    pub market_cap: f64,
}

/// A tradeable pair discovered via CoinMarketCap + Kraken cross-reference.
#[derive(Debug, Clone)]
pub struct DiscoveredPair {
    pub symbol: String,
    pub kraken_pair: String,
    pub name: String,
    pub rank: u32,
    pub price: f64,
    pub volume_24h: f64,
    pub change_24h: f64,
    pub change_7d: f64,
    pub market_cap: f64,
    /// Volatility score: abs(24h change) + abs(7d change) / 7
    pub volatility_score: f64,
}

impl DiscoveredPair {
    /// Format for logging/display.
    pub fn summary(&self) -> String {
        format!(
            "{} ({}) | ${:.2} | 24h: {:+.1}% | Vol: ${:.0}M | Score: {:.1}",
            self.symbol,
            self.kraken_pair,
            self.price,
            self.change_24h,
            self.volume_24h / 1_000_000.0,
            self.volatility_score,
        )
    }
}

/// Fetch top cryptocurrencies from CoinMarketCap.
///
/// `limit`: number of coins to fetch (max 5000, free tier: 100)
pub async fn fetch_top_coins(
    client: &reqwest::Client,
    api_key: &str,
    limit: u32,
) -> Result<Vec<CmcCoin>, String> {
    let url = format!("{}/cryptocurrency/listings/latest", CMC_BASE_URL);

    let resp = client
        .get(&url)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .query(&[
            ("start", "1".to_string()),
            ("limit", limit.to_string()),
            ("convert", "USD".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("CMC request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("CMC API error {}: {}", status, body));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("CMC parse error: {}", e))?;

    let data = json["data"]
        .as_array()
        .ok_or_else(|| "CMC response missing 'data' field".to_string())?;

    let mut coins = Vec::new();
    for item in data {
        if let Ok(coin) = serde_json::from_value::<CmcCoin>(item.clone()) {
            coins.push(coin);
        }
    }

    info!("CMC: fetched {} coins", coins.len());
    Ok(coins)
}

/// Known Kraken USD pair mappings.
/// CMC symbol → Kraken pair name
fn kraken_pair_map(symbol: &str) -> Option<&str> {
    match symbol.to_uppercase().as_str() {
        "BTC" => Some("BTC/USD"),
        "ETH" => Some("ETH/USD"),
        "SOL" => Some("SOL/USD"),
        "XRP" => Some("XRP/USD"),
        "DOGE" => Some("DOGE/USD"),
        "ADA" => Some("ADA/USD"),
        "AVAX" => Some("AVAX/USD"),
        "DOT" => Some("DOT/USD"),
        "LINK" => Some("LINK/USD"),
        "MATIC" => Some("MATIC/USD"),
        "UNI" => Some("UNI/USD"),
        "ATOM" => Some("ATOM/USD"),
        "LTC" => Some("LTC/USD"),
        "BCH" => Some("BCH/USD"),
        "ALGO" => Some("ALGO/USD"),
        "FIL" => Some("FIL/USD"),
        "NEAR" => Some("NEAR/USD"),
        "ARB" => Some("ARB/USD"),
        "OP" => Some("OP/USD"),
        "INJ" => Some("INJ/USD"),
        "TIA" => Some("TIA/USD"),
        "SUI" => Some("SUI/USD"),
        "SEI" => Some("SEI/USD"),
        "PEPE" => Some("PEPE/USD"),
        "SHIB" => Some("SHIB/USD"),
        "WIF" => Some("WIF/USD"),
        "BONK" => Some("BONK/USD"),
        "FLOKI" => Some("FLOKI/USD"),
        "RENDER" => Some("RENDER/USD"),
        "FET" => Some("FET/USD"),
        "GRT" => Some("GRT/USD"),
        "AAVE" => Some("AAVE/USD"),
        "MKR" => Some("MKR/USD"),
        "CRV" => Some("CRV/USD"),
        "SNX" => Some("SNX/USD"),
        "LDO" => Some("LDO/USD"),
        "RPL" => Some("RPL/USD"),
        "ENS" => Some("ENS/USD"),
        "COMP" => Some("COMP/USD"),
        "SUSHI" => Some("SUSHI/USD"),
        "YFI" => Some("YFI/USD"),
        "BAL" => Some("BAL/USD"),
        "KNC" => Some("KNC/USD"),
        "ZRX" => Some("ZRX/USD"),
        "MANA" => Some("MANA/USD"),
        "SAND" => Some("SAND/USD"),
        "AXS" => Some("AXS/USD"),
        "GALA" => Some("GALA/USD"),
        "ENJ" => Some("ENJ/USD"),
        "CHZ" => Some("CHZ/USD"),
        "FLOW" => Some("FLOW/USD"),
        "MINA" => Some("MINA/USD"),
        "EGLD" => Some("EGLD/USD"),
        "HBAR" => Some("HBAR/USD"),
        "VET" => Some("VET/USD"),
        "ICP" => Some("ICP/USD"),
        "APT" => Some("APT/USD"),
        "DYDX" => Some("DYDX/USD"),
        "1INCH" => Some("1INCH/USD"),
        "ANKR" => Some("ANKR/USD"),
        "STORJ" => Some("STORJ/USD"),
        "BAND" => Some("BAND/USD"),
        "OCEAN" => Some("OCEAN/USD"),
        "CTSI" => Some("CTSI/USD"),
        "TRX" => Some("TRX/USD"),
        "XLM" => Some("XLM/USD"),
        "EOS" => Some("EOS/USD"),
        "QTUM" => Some("QTUM/USD"),
        "SC" => Some("SC/USD"),
        "OXT" => Some("OXT/USD"),
        "MASK" => Some("MASK/USD"),
        "IMX" => Some("IMX/USD"),
        "BLUR" => Some("BLUR/USD"),
        "JUP" => Some("JUP/USD"),
        "PYTH" => Some("PYTH/USD"),
        "W" => Some("W/USD"),
        "ENA" => Some("ENA/USD"),
        "PENDLE" => Some("PENDLE/USD"),
        "WLD" => Some("WLD/USD"),
        "BTT" => Some("BTT/USD"),
        "XMR" => Some("XMR/USD"),
        "ZEC" => Some("ZEC/USD"),
        "DASH" => Some("DASH/USD"),
        "KAVA" => Some("KAVA/USD"),
        "KSM" => Some("KSM/USD"),
        "RUNE" => Some("RUNE/USD"),
        "OSMO" => Some("OSMO/USD"),
        "STX" => Some("STX/USD"),
        _ => None,
    }
}

/// Discover tradeable pairs by cross-referencing CMC data with Kraken.
///
/// Returns pairs sorted by volatility score (highest first).
pub async fn discover_pairs(
    client: &reqwest::Client,
    api_key: &str,
    limit: u32,
) -> Vec<DiscoveredPair> {
    let coins = match fetch_top_coins(client, api_key, limit).await {
        Ok(c) => c,
        Err(e) => {
            warn!("CMC fetch failed: {}", e);
            return vec![];
        }
    };

    let mut pairs: Vec<DiscoveredPair> = coins
        .iter()
        .filter_map(|coin| {
            let kraken_pair = kraken_pair_map(&coin.symbol)?;
            let change_24h = coin.quote.usd.percent_change_24h;
            let change_7d = coin.quote.usd.percent_change_7d.unwrap_or(0.0);
            let volatility_score = change_24h.abs() + change_7d.abs() / 7.0;

            Some(DiscoveredPair {
                symbol: coin.symbol.clone(),
                kraken_pair: kraken_pair.to_string(),
                name: coin.name.clone(),
                rank: coin.rank.unwrap_or(999),
                price: coin.quote.usd.price,
                volume_24h: coin.quote.usd.volume_24h,
                change_24h,
                change_7d,
                market_cap: coin.quote.usd.market_cap,
                volatility_score,
            })
        })
        .collect();

    // Sort by volatility score descending
    pairs.sort_by(|a, b| b.volatility_score.partial_cmp(&a.volatility_score).unwrap());

    info!(
        "Discovered {} tradeable pairs from {} CMC coins",
        pairs.len(),
        coins.len()
    );

    pairs
}

/// Get the top N most volatile pairs as Kraken pair strings.
pub async fn get_top_volatile_pairs(
    client: &reqwest::Client,
    api_key: &str,
    top_n: usize,
) -> Vec<String> {
    let pairs = discover_pairs(client, api_key, 100).await;
    let result: Vec<String> = pairs
        .iter()
        .take(top_n)
        .map(|p| p.kraken_pair.clone())
        .collect();

    info!("Top {} volatile pairs: {:?}", result.len(), result);
    result
}

// ============================================================
// DEX API endpoints (v4) — on-chain DEX data
// ============================================================

/// DEX pair OHLCV data from CoinMarketCap.
#[derive(Debug, Clone, Deserialize)]
pub struct DexOhlcv {
    pub time_open: String,
    pub time_close: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    #[serde(rename = "market_cap")]
    pub market_cap: Option<f64>,
}

/// DEX trade data from CoinMarketCap.
#[derive(Debug, Clone, Deserialize)]
pub struct DexTrade {
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<String>,
    pub price: f64,
    pub amount: f64,
    pub side: Option<String>,
    pub timestamp: String,
}

/// Fetch historical OHLCV data for a DEX pair.
///
/// This gives us candle data for ANY on-chain pair — including meme coins
/// that aren't on centralized exchanges yet.
///
/// `pair_address`: The contract address of the DEX pair
/// `network`: The blockchain network (e.g., "ethereum", "solana", "base")
/// `interval`: Candle interval ("1m", "5m", "1h", "1d")
/// `count`: Number of candles to fetch
pub async fn fetch_dex_ohlcv(
    client: &reqwest::Client,
    api_key: &str,
    pair_address: &str,
    network: &str,
    interval: &str,
    count: u32,
) -> Result<Vec<DexOhlcv>, String> {
    let url = format!("{}/v4/dex/pairs/ohlcv/historical", CMC_BASE_URL);

    let resp = client
        .get(&url)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .query(&[
            ("address", pair_address),
            ("network", network),
            ("interval", interval),
            ("count", &count.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("CMC DEX OHLCV request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("CMC DEX OHLCV error {}: {}", status, body));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("CMC DEX OHLCV parse error: {}", e))?;

    let data = json["data"]
        .as_array()
        .ok_or_else(|| "CMC DEX OHLCV missing 'data' field".to_string())?;

    let mut candles = Vec::new();
    for item in data {
        if let Ok(candle) = serde_json::from_value::<DexOhlcv>(item.clone()) {
            candles.push(candle);
        }
    }

    info!(
        "CMC DEX OHLCV: fetched {} candles for {} on {}",
        candles.len(),
        pair_address,
        network
    );
    Ok(candles)
}

/// Fetch the latest trades for a DEX pair.
///
/// Returns up to 100 most recent trades with transaction hashes.
/// Useful for real-time on-chain trade analysis.
pub async fn fetch_dex_trades(
    client: &reqwest::Client,
    api_key: &str,
    pair_address: &str,
    network: &str,
) -> Result<Vec<DexTrade>, String> {
    let url = format!("{}/v4/dex/pairs/trade/latest", CMC_BASE_URL);

    let resp = client
        .get(&url)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .query(&[("address", pair_address), ("network", network)])
        .send()
        .await
        .map_err(|e| format!("CMC DEX trades request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("CMC DEX trades error {}: {}", status, body));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("CMC DEX trades parse error: {}", e))?;

    let data = json["data"]
        .as_array()
        .ok_or_else(|| "CMC DEX trades missing 'data' field".to_string())?;

    let mut trades = Vec::new();
    for item in data {
        if let Ok(trade) = serde_json::from_value::<DexTrade>(item.clone()) {
            trades.push(trade);
        }
    }

    info!(
        "CMC DEX trades: fetched {} trades for {} on {}",
        trades.len(),
        pair_address,
        network
    );
    Ok(trades)
}

/// DEX listing with market data.
#[derive(Debug, Clone, Deserialize)]
pub struct DexListing {
    pub id: u32,
    pub name: String,
    pub slug: String,
    #[serde(rename = "num_market_pairs")]
    pub num_market_pairs: Option<u32>,
}

/// Fetch top DEX exchanges ranked by volume.
pub async fn fetch_dex_listings(
    client: &reqwest::Client,
    api_key: &str,
    limit: u32,
) -> Result<Vec<DexListing>, String> {
    let url = format!("{}/v4/dex/listings/quotes", CMC_BASE_URL);

    let resp = client
        .get(&url)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .query(&[("limit", limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("CMC DEX listings request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("CMC DEX listings error {}: {}", status, body));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("CMC DEX listings parse error: {}", e))?;

    let data = json["data"]
        .as_array()
        .ok_or_else(|| "CMC DEX listings missing 'data' field".to_string())?;

    let mut listings = Vec::new();
    for item in data {
        if let Ok(listing) = serde_json::from_value::<DexListing>(item.clone()) {
            listings.push(listing);
        }
    }

    info!("CMC DEX listings: fetched {} exchanges", listings.len());
    Ok(listings)
}
