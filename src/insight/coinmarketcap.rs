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
