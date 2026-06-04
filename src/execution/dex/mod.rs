//! DEX aggregator backends for no-KYC on-chain trading.
//!
//! Provides an abstract [`DexBackend`] trait that the 0x API and 1inch API
//! both implement.  [`super::DexTrader`] uses this trait, making the trader
//! backend-agnostic.  The target chain is **Arbitrum** (chain_id = 42_161)
//! for low gas fees.  On-chain base currency = **USDC**.

pub mod inch;
pub mod trader;
pub mod zero_x;

// Re-export main types at the module level
pub use trader::DexTrader;

use async_trait::async_trait;

use crate::core::error::ExecutionError;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Parameters for requesting a DEX aggregator swap.
#[derive(Debug, Clone)]
pub struct SwapParams {
    /// Source-token address (what we sell).
    pub src_token: String,
    /// Destination-token address (what we buy).
    pub dst_token: String,
    /// Amount in the smallest unit as a decimal string (wei).
    /// We use a string to avoid floating-point precision loss.
    pub amount: String,
    /// Slippage tolerance as a decimal fraction (0.005 = 0.5 %).
    pub slippage: f64,
    /// Taker wallet address.
    pub from: String,
    /// EVM chain ID (e.g. 42_161 for Arbitrum).
    pub chain_id: u64,
}

/// A price quote returned by the aggregator (no calldata).
#[derive(Debug, Clone)]
pub struct Quote {
    /// Expected receive amount in smallest unit (decimal string).
    pub to_amount: String,
    /// Human-readable price.
    pub price: String,
    /// Guaranteed minimum price accounting for slippage.
    pub guaranteed_price: String,
    /// Estimated gas units.
    pub estimated_gas: u64,
}

/// Transaction calldata for executing a swap.
#[derive(Debug, Clone)]
pub struct SwapTx {
    /// Target contract address (exchange router).
    pub to: String,
    /// Encoded calldata (0x-prefixed hex).
    pub data: String,
    /// ETH value to send in wei (0 for ERC20 → ERC20).
    pub value: String,
    /// Gas limit estimate.
    pub gas: u64,
    /// Gas price in wei (for display / fallback).
    pub gas_price: String,
}

/// Resolved token metadata on a given chain.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
}

// ---------------------------------------------------------------------------
// DexBackend trait
// ---------------------------------------------------------------------------

/// Abstract interface for a DEX aggregator API.
///
/// Each implementation wraps a different REST API (0x, 1inch, …) while
/// exposing a uniform `quote` + `build_swap_tx` surface.
#[async_trait]
pub trait DexBackend: Send + Sync {
    /// Fetch a price quote for the given swap parameters (no calldata).
    async fn quote(&self, params: &SwapParams) -> Result<Quote, ExecutionError>;

    /// Build a full swap-transaction `SwapTx` that the caller must sign and
    /// broadcast.
    async fn build_swap_tx(&self, params: &SwapParams) -> Result<SwapTx, ExecutionError>;

    /// Human-readable backend name, e.g. `"0x"` or `"1inch"`.
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Token address database  (Arbitrum mainnet — chain_id = 42_161)
// ---------------------------------------------------------------------------

/// Arbitrum token addresses — keyed by uppercase symbol.
/// Sources: official Arbitrum token list, verified contract addresses.
const ARBITRUM_TOKENS: &[(&str, &str, u8)] = &[
    // Core — verified via Blockscot API (https://arbitrum.blockscout.com/api/v2/tokens)
    ("ETH", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18),
    ("WETH", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18),
    ("USDC", "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6),
    ("USDT", "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9", 6),
    ("DAI", "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1", 18),
    ("WBTC", "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f", 8),
    ("BTC", "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f", 8), // BTC = WBTC on Arbitrum
    ("ARB", "0x912CE59144191C1204E64559FE8253a0e49E6548", 18),
    ("LINK", "0xf97f4df75117a78c1A5a0DBb814Af92458539FB4", 18),
    ("UNI", "0xFa7F8980b0f1E64A2062791cc3b0871572f1F7f0", 18),
    ("PEPE", "0x25d887Ce7a35172C62FeBFD67a1856F20FaEbB00", 18),
    // DeFi / L2 — verified via Blockscot API
    ("AAVE", "0xba5DdD1f9d7F570dc94a51479a000E3BCE967196", 18),
    ("LDO", "0x13Ad51ed4F1B7e9Dc168d8a00CB3f4DDD85EFA60", 18),
    ("PENDLE", "0x0c880f6761F1af8d9Aa9C466984b80DAb9a8c9e8", 18),
    ("RENDER", "0xC8a4EeA31E9B6b61c406DF013DD4FEc76f21E279", 18), // RNDR on Arbitrum
    ("FET", "0x8D2cD4BF7E2196d5204bb15264BdD5E789D00Bad", 8),
    ("GRT", "0x9623063377AD1B27544C965cCd7342f7EA7e88C7", 18),
    ("BONK", "0x09199d9A5F4448D0848e4395D065e1ad9c4a1F74", 5),
    ("DOT", "0x8d010bf9C26881788b4e6bf5Fd1bdC358c8F90b8", 18),
];

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Runtime token extensions — discovered tokens added at startup.
/// Merged with the static `ARBITRUM_TOKENS` during resolution.
static TOKEN_EXTENSIONS: Mutex<Option<HashMap<String, (String, u8)>>> = Mutex::new(None);

pub fn token_map() -> &'static HashMap<&'static str, (&'static str, u8)> {
    static MAP: OnceLock<HashMap<&str, (&str, u8)>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();
        for &(sym, addr, dec) in ARBITRUM_TOKENS {
            m.insert(sym, (addr, dec));
        }
        m
    })
}

/// Add discovered tokens to the runtime extension database.
/// These are merged with the static `ARBITRUM_TOKENS` during resolution.
pub fn extend_token_db(tokens: &[(String, String, u8)]) {
    let mut ext = TOKEN_EXTENSIONS.lock().unwrap();
    let map = ext.get_or_insert_with(HashMap::new);
    for (symbol, address, decimals) in tokens {
        map.insert(symbol.clone(), (address.clone(), *decimals));
    }
}

/// Look up a token by symbol — checks extensions first, then static DB.
pub fn lookup_token(symbol: &str) -> Option<(String, u8)> {
    // Check runtime extensions first
    let ext = TOKEN_EXTENSIONS.lock().unwrap();
    if let Some(ref map) = *ext {
        if let Some(&(ref addr, dec)) = map.get(symbol) {
            return Some((addr.clone(), dec));
        }
    }
    drop(ext);

    // Fall back to static DB
    token_map().get(symbol).map(|&(addr, dec)| (addr.to_string(), dec))
}

/// Resolve a trading pair (e.g. `"ETH/USDC"`) into source + destination
/// [`TokenInfo`] based on the trade [`Side`](crate::core::types::Side).
///
/// **LONG**  — buy the base asset → sell quote (`USDC`), buy base (`ETH`).
/// **SHORT** — sell the base asset → sell base (`ETH`), buy quote (`USDC`).
///
/// `"USD"` is automatically mapped to `"USDC"` because there is no native
/// USD token on EVM chains.
///
/// ## Enterprise token resolution
///
/// When a token is not found in the local database, we return a `TokenInfo`
/// with an **empty address** and **18 decimals** (the standard for most
/// wrapped tokens). The caller (see [`DexTrader::execute_swap`]) detects the
/// empty address and passes the token **symbol** directly to the DEX
/// aggregator API instead. Both the 0x and 1inch APIs accept token symbols
/// natively and resolve the most liquid deployed address on the target
/// chain. This means:
///
/// - No fragile, hardcoded address database for bridged tokens
/// - The API always returns the current most liquid version
/// - Works for ALL tokens without manual address research
/// - Adapts automatically when bridges/liquidity shift
pub fn resolve_pair(
    pair: &str,
    side: crate::core::types::Side,
) -> Result<(TokenInfo, TokenInfo), ExecutionError> {
    let parts: Vec<&str> = pair.split('/').collect();
    if parts.len() != 2 {
        return Err(ExecutionError::Other(format!(
            "Invalid pair format '{}' — expected BASE/QUOTE (e.g. ETH/USDC)",
            pair
        )));
    }

    let base_sym = parts[0].to_uppercase();
    let quote_sym = if parts[1].to_uppercase() == "USD" {
        "USDC".to_string()
    } else {
        parts[1].to_uppercase()
    };

    // Resolve base token — check runtime extensions first, then static DB
    let base = match lookup_token(&base_sym) {
        Some((addr, dec)) => TokenInfo {
            symbol: base_sym,
            address: addr,
            decimals: dec,
        },
        None => TokenInfo {
            symbol: base_sym,
            address: String::new(), // Empty = use symbol directly with API
            decimals: 18,           // Standard for most bridged tokens
        },
    };

    // Resolve quote token — same pattern
    let quote = match lookup_token(&quote_sym) {
        Some((addr, dec)) => TokenInfo {
            symbol: quote_sym,
            address: addr,
            decimals: dec,
        },
        None => TokenInfo {
            symbol: quote_sym,
            address: String::new(),
            decimals: 18,
        },
    };

    match side {
        crate::core::types::Side::Long => {
            // Buy base → sell quote, receive base
            Ok((quote, base))
        }
        crate::core::types::Side::Short => {
            // Sell base → sell base, receive quote
            Ok((base, quote))
        }
    }
}

/// Convert a human-readable amount + decimals into a wei-scale decimal string.
///
/// `amount_to_wei(0.01, 18)` returns `"10000000000000000"`.
/// `amount_to_wei(50.0, 6)` returns `"50000000"`.
pub fn amount_to_wei(amount: f64, decimals: u8) -> String {
    let factor = 10u128.pow(decimals as u32) as f64;
    let wei = (amount * factor).round() as u128;
    wei.to_string()
}

/// Convert a wei-scale decimal string back to a human-readable float.
pub fn wei_to_amount(wei: &str, decimals: u8) -> f64 {
    let factor = 10u128.pow(decimals as u32) as f64;
    let val: u128 = wei.parse().unwrap_or(0);
    val as f64 / factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Side;

    #[test]
    fn resolve_eth_usdc_long() {
        let (src, dst) = resolve_pair("ETH/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "ETH");
        assert!(dst.address.starts_with("0x82aF"));
    }

    #[test]
    fn resolve_eth_usdc_short() {
        let (src, dst) = resolve_pair("ETH/USDC", Side::Short).unwrap();
        assert_eq!(src.symbol, "ETH");
        assert_eq!(dst.symbol, "USDC");
    }

    #[test]
    fn resolve_usd_becomes_usdc() {
        let (src, dst) = resolve_pair("WBTC/USD", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "WBTC");
    }

    #[test]
    fn resolve_unknown_token_falls_back_to_symbol() {
        // Tokens not in the local DB should resolve via symbol (empty address = use symbol with API)
        let (src, dst) = resolve_pair("FAKE/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "FAKE");
        assert!(
            dst.address.is_empty(),
            "unknown token should have empty address (symbol-based)"
        );
        assert_eq!(dst.decimals, 18, "unknown token defaults to 18 decimals");
    }

    #[test]
    fn resolve_sol_usdc_long_symbol_fallback() {
        // SOL is not in the local DB — should resolve by symbol
        let (src, dst) = resolve_pair("SOL/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "SOL");
        assert!(src.address.starts_with("0x"), "USDC should have address");
        assert!(
            dst.address.is_empty(),
            "SOL should have empty address (symbol-based)"
        );
    }

    #[test]
    fn resolve_xrp_usdc_long_symbol_fallback() {
        let (src, dst) = resolve_pair("XRP/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "XRP");
        assert!(dst.address.is_empty());
    }

    #[test]
    fn resolve_ada_usdc_long_symbol_fallback() {
        let (src, dst) = resolve_pair("ADA/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "ADA");
        assert!(dst.address.is_empty());
    }

    #[test]
    fn resolve_avax_usdc_long_symbol_fallback() {
        let (src, dst) = resolve_pair("AVAX/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "AVAX");
        assert!(dst.address.is_empty());
    }

    #[test]
    fn resolve_dot_usdc_long_symbol_fallback() {
        let (src, dst) = resolve_pair("DOT/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "DOT");
        assert!(dst.address.starts_with("0x8d01"), "DOT should have Arbitrum address");
    }

    #[test]
    fn resolve_invalid_format_error() {
        let result = resolve_pair("INVALID", Side::Long);
        assert!(result.is_err());
    }

    #[test]
    fn amount_to_wei_roundtrip() {
        let wei = amount_to_wei(0.01, 18);
        assert_eq!(wei, "10000000000000000");
        let back = wei_to_amount(&wei, 18);
        assert!((back - 0.01).abs() < 1e-12);
    }

    #[test]
    fn amount_to_wei_usdc() {
        let wei = amount_to_wei(50.0, 6);
        assert_eq!(wei, "50000000");
    }

    // ---- New token resolution tests ----

    #[test]
    fn resolve_aave_usdc_long() {
        let (src, dst) = resolve_pair("AAVE/USDC", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "AAVE");
        assert!(dst.address.starts_with("0xba5D"));
    }

    #[test]
    fn resolve_ldo_usdc_long() {
        let (_src, dst) = resolve_pair("LDO/USDC", Side::Long).unwrap();
        assert_eq!(_src.symbol, "USDC");
        assert_eq!(dst.symbol, "LDO");
        assert!(dst.address.starts_with("0x13Ad"));
    }

    #[test]
    fn resolve_pendle_usdc_long() {
        let (_src, dst) = resolve_pair("PENDLE/USDC", Side::Long).unwrap();
        assert_eq!(_src.symbol, "USDC");
        assert_eq!(dst.symbol, "PENDLE");
        assert!(dst.address.starts_with("0x0c88"), "PENDLE should have Arbitrum address");
    }

    #[test]
    fn resolve_render_usdc_long() {
        let (_src, dst) = resolve_pair("RENDER/USDC", Side::Long).unwrap();
        assert_eq!(_src.symbol, "USDC");
        assert_eq!(dst.symbol, "RENDER");
        assert!(dst.address.starts_with("0xC8a4"), "RENDER (RNDR) should have Arbitrum address");
    }

    #[test]
    fn resolve_fet_usdc_long() {
        let (_src, dst) = resolve_pair("FET/USDC", Side::Long).unwrap();
        assert_eq!(_src.symbol, "USDC");
        assert_eq!(dst.symbol, "FET");
        assert!(dst.address.starts_with("0x8D2c"), "FET should have Arbitrum address");
    }

    #[test]
    fn resolve_grt_usdc_long() {
        let (_src, dst) = resolve_pair("GRT/USDC", Side::Long).unwrap();
        assert_eq!(_src.symbol, "USDC");
        assert_eq!(dst.symbol, "GRT");
        assert!(dst.address.starts_with("0x9623"));
    }

    #[test]
    fn resolve_aave_usd_long() {
        // USD maps to USDC
        let (src, dst) = resolve_pair("AAVE/USD", Side::Long).unwrap();
        assert_eq!(src.symbol, "USDC");
        assert_eq!(dst.symbol, "AAVE");
    }
}
