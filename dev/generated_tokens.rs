//! DEX aggregator backends for no-KYC on-chain trading.
//!
//! Provides an abstract [DexBackend] trait that the 0x API and 1inch API
//! both implement.  [super::DexTrader] uses this trait, making the trader
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
    /// Decimals of the destination token (for wei → human conversion).
    pub buy_decimals: u32,
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
/// exposing a uniform quote + uild_swap_tx surface.
#[async_trait]
pub trait DexBackend: Send + Sync {
    /// Fetch a price quote for the given swap parameters (no calldata).
    async fn quote(&self, params: &SwapParams) -> Result<Quote, ExecutionError>;

    /// Build a full swap-transaction SwapTx that the caller must sign and
    /// broadcast.
    async fn build_swap_tx(&self, params: &SwapParams) -> Result<SwapTx, ExecutionError>;

    /// Human-readable backend name, e.g. "0x" or "1inch".
    fn name(&self) -> &'static str;
}

/// Fallback backend — tries primary first, falls back to secondary on failure.
pub struct FallbackBackend {
    primary: Box<dyn DexBackend>,
    secondary: Box<dyn DexBackend>,
}

impl FallbackBackend {
    pub fn new(primary: Box<dyn DexBackend>, secondary: Box<dyn DexBackend>) -> Self {
        Self { primary, secondary }
    }
}

#[async_trait]
impl DexBackend for FallbackBackend {
    fn name(&self) -> &'static str {
        "fallback"
    }

    async fn quote(&self, params: &SwapParams) -> Result<Quote, ExecutionError> {
        match self.primary.quote(params).await {
            Ok(quote) => Ok(quote),
            Err(primary_err) => {
                tracing::warn!(
                    "Primary backend ({}) quote failed: {} — trying fallback ({})",
                    self.primary.name(),
                    primary_err,
                    self.secondary.name()
                );
                self.secondary.quote(params).await
            }
        }
    }

    async fn build_swap_tx(&self, params: &SwapParams) -> Result<SwapTx, ExecutionError> {
        match self.primary.build_swap_tx(params).await {
            Ok(tx) => Ok(tx),
            Err(primary_err) => {
                tracing::warn!(
                    "Primary backend ({}) build_swap_tx failed: {} — trying fallback ({})",
                    self.primary.name(),
                    primary_err,
                    self.secondary.name()
                );
                self.secondary.build_swap_tx(params).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Token address database  (Arbitrum mainnet — chain_id = 42_161)
// ---------------------------------------------------------------------------

/// Arbitrum token addresses — keyed by uppercase symbol.
/// Source: CoinGecko API (arbitrum-one platform), verified contract addresses.
/// Total: 201 tokens with > daily volume.
pub const ARBITRUM_TOKENS: &[(&str, &str, u8)] = &[
    // Quote currency
    ("USDC", "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6),

    // Core routing assets
    ("WETH", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18),
    ("ETH", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18),
    ("WBTC", "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f", 8),
    ("BTC", "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f", 8),

    // Top volume tokens (CoinGecko + Arbiscan verified)
    ("1INCH", "0x6314c31a7a1652ce482cffe247e9cb7c3f4bb9af", 18),
    ("AAVE", "0xba5DdD1f9d7F570dc94a51479a000E3BCE967196", 18),
    ("ACX", "0x53691596d1bce8cea565b84d4915e69e03d9c99d", 18),
    ("AIDOGE", "0x09e18590e8f76b6cf471b3cd75fe1a1a9d2b2c2b", 18),
    ("ALETH", "0x17573150d67d820542efb24210371545a4868b03", 18),
    ("ANIME", "0x37a645648df29205c6261289983fb04ecd70b4b3", 18),
    ("ANKR", "0xaeaeed23478c3a4b798e4ed40d8b7f41366ae861", 18),
    ("ANON", "0x79bbf4508b1391af3a0f4b30bb5fc4aa9ab0e07c", 18),
    ("ANT", "0xa78d8321b20c4ef90ecd72f2588aa985a4bdb684", 18),
    ("APE", "0x7f9fbf9bdd3f4105c478b996b648fe6e828a1e98", 18),
    ("APEX", "0x61a1ff55c5216b636a294a07d77c6f4df10d3b56", 18),
    ("APU", "0x1f53b7aa6f4b36b7dfdedb4bc4a14747a19cf7b1", 18),
    ("ARB", "0x912CE59144191C1204E64559FE8253a0e49E6548", 18),
    ("ATH", "0xc87B37a581ec3257B734886d9d3a581F5A9d056c", 18),
    ("AWETH", "0xe50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8", 18),
    ("AXL", "0x23ee2343b892b1bb63503a4fabc840e0e2c6810f", 18),
    ("BADGER", "0xbfa641051ba0a0ad1b0acf549a89536a0d76472e", 18),
    ("BAL", "0x040d1edc9569d4bab2d15287dc5a4f10f56a56b8", 18),
    ("BICO", "0xa68ec98d7ca870cf1dd0b00ebbb7c4bf60a8e74d", 18),
    ("BONK", "0x09199d9A5F4448D0848e4395D065e1ad9c4a1F74", 5),
    ("BOSON", "0x54b334d68cf5382fee7fbbe496fcf1e76d9ba000", 18),
    ("BOUNTY", "0x6a9896837021ea3ed83f623f655c119c54abe02c", 18),
    ("BZR", "0xa0a675d08ca63066f48408136f8a71fc65be4afc", 18),
    ("CAKE", "0x1b896893dfc86bb67Cf57767298b9073D2c1bA2c", 18),
    ("CAPX", "0x97e66d3c4d5bcd7c64e3e55af28544c9addf9281", 18),
    ("CARV", "0xc08cd26474722ce93f4d0c34d16201461c10aa8c", 18),
    ("CBBTC", "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf", 8),
    ("CBETH", "0x1DEBd73E752bEaF79865Fd6446b0c970EaE7732f", 18),
    ("CELR", "0x3a8b787f78d775aecfeea15706d4221b40f345ab", 18),
    ("CHIP", "0x0c1c1c109fe34733fca54b82d7b46b75cfb71f6e", 18),
    ("COMP", "0x354a6da3fcde098f8389cad84b0182725c6c91de", 18),
    ("CORN", "0x44f49ff0da2498bcb1d3dc7c0f999578f67fd8c6", 18),
    ("COW", "0xcb8b5cd20bdcaea9a010ac1f8d835824f5c87a04", 18),
    ("CRV", "0x11cDb42B0EB46D95f990BeDD4695A6e3fA034978", 18),
    ("CTSI", "0x319f865b287fcc10b30d8ce6144e8b6d1b476999", 18),
    ("DAO", "0xcaa38bcc8fb3077975bbe217acfaa449e6596a84", 18),
    ("DOC", "0x2ad62eb9744c720364f6ac856360a43e8a2229b5", 18),
    ("DODO", "0x69eb4fa4a2fbd498c257c57ea8b7655a2559a581", 18),
    ("DOT", "0x8d010bf9C26881788b4e6bf5Fd1bdC358c8F90b8", 18),
    ("DRV", "0x77b7787a09818502305c95d68a2571f090abb135", 18),
    ("EDGE", "0x70f2eadf1ca1969ff42b0c78e9da519e8937cbaf", 18),
    ("ENA", "0x58538e6A46E07434d7E7375Bc268D3cb839C0133", 18),
    ("ERA", "0x00312400303d02c323295f6e8b7309bc30fb6bce", 18),
    ("ESP", "0x3b8db18e69d6686ad9371a423afe3dd1065c94f1", 18),
    ("ETH+", "0x18c14c2d707b2212e17d1579789fc06010cfca23", 18),
    ("ETHFI", "0x7189fb5B6504bbfF6a852B13B7B82a3c118fDc27", 18),
    ("EUL", "0x462cd9e0247b2e63831c3189ae738e5e9a5a4b64", 18),
    ("EURAU", "0x4933a85b5b5466fbaf179f72d3de273c287ec2c2", 18),
    ("EURE", "0x0c06ccf38114ddfc35e07427b9424adcca9f44f8", 18),
    ("EV", "0xe7e7e741c23a4767831a56a8c99f522c5ac1e7e7", 18),
    ("EVA", "0x45d9831d8751b2325f3dbf48db748723726e1c8c", 18),
    ("EYWA", "0x7a10f506e4c7658e6ad15fdf0443d450b7fa80d7", 18),
    ("EZETH", "0x2416092f143378750bb29b79ed961ab195cceea5", 18),
    ("FBTC", "0xc96de26018a54d51c097160568752c4e3bd6c364", 18),
    ("FET", "0x8D2cD4BF7E2196d5204bb15264BdD5E789D00Bad", 8),
    ("FLUID", "0x61e030a56d33e8260fdd81f03b162a79fe3449cd", 18),
    ("FOLKS", "0xff7f8f301f7a706e3cfd3d2275f5dc0b9ee8009b", 18),
    ("FORT", "0x3a1429d50e0cbbc45c997af600541fe1cc3d2923", 18),
    ("FRAX", "0x17fc002b466eec40dae837fc4be5c67993ddbd6f", 18),
    ("FRXETH", "0x178412e79c25968a32e89b11f63b33f733770c2a", 18),
    ("FUSE", "0x6b021b3f68491974be6d4009fee61a4e3c708fd6", 18),
    ("FXS", "0x9d2f299715d94d8a7e6f5eaa8e654e8c74a988a7", 18),
    ("GHO", "0x7dff72693f6a4149b17e7c6314655f6a9f7c8b33", 18),
    ("GIZA", "0xa4eaec0b1d564061a4951816fd5b1ba8cfbc425c", 18),
    ("GMX", "0xfc5A1A6EB076a2C7aD06eD22C90d7E710E35ad0a", 18),
    ("GNO", "0xa0b862F60edEf4452F25B4160F177db44DeB6Cf1", 18),
    ("GNS", "0x18c11FD286C5EC11c3b683Caa813B77f5163A122", 18),
    ("GRANT", "0x7ce42e8a5a42eb15f0c9a08ee9a079d99b1d83cf", 18),
    ("GRND", "0x3b58a4c865b568a2f6a957c264f6b50cba35d8ce", 18),
    ("GRT", "0x9623063377AD1B27544C965cCd7342f7EA7e88C7", 18),
    ("HOT", "0x17e1e5c6bc9ebb11647c94e1c5e3ba619f2781ea", 18),
    ("HYPER", "0xc9d23ed2adb0f551369946bd377f8644ce1ca5c4", 18),
    ("IDOS", "0x68731d6f14b827bbcffbebb62b19daa18de1d79c", 18),
    ("JOE", "0x371c7ec6d8039ff7933a2aa28eb827ffe1f52f07", 18),
    ("JONES", "0x10393c20975cf177a3513071bc110f7962cd67da", 18),
    ("KERNEL", "0x6e401189c8a68d05562c9bab7f674f910821eacf", 18),
    ("KNC", "0xe4dddfe67e7164b0fe14e218d80dc4c08edc01cb", 18),
    ("KOM", "0xa58663faef461761e44066ea26c1fcddf2927b80", 18),
    ("KRL", "0xf75ee6d319741057a82a88eeff1dbafab7307b69", 18),
    ("L3", "0x46777c76dbbe40fabb2aab99e33ce20058e76c59", 18),
    ("LADYS", "0x3b60ff35d3f7f62d636b067dd0dc0dfdad670e4e", 18),
    ("LAVA", "0x11e969e9b3f89cb16d686a03cd8508c9fc0361af", 18),
    ("LDO", "0x13Ad51ed4F1B7e9Dc168d8a00CB3f4DDD85EFA60", 18),
    ("LINK", "0xf97f4df75117a78c1A5a0DBb814Af92458539FB4", 18),
    ("LION", "0x527e8d368298dea5a53be257e5300f4dbafb7a97", 18),
    ("LPT", "0x289ba1701c2f088cf0faf8b3705246331cb8a839", 18),
    ("LQTY", "0xfb9e5d956d889d91a82737b9bfcdac1dce3e1449", 18),
    ("LRC", "0x46d0ce7de6247b0a95f67b43b589b4041bae7fbe", 18),
    ("LUNC", "0x1A4dA80967373fd929961e976b4b53ceeC063a15", 18),
    ("MAGIC", "0x539bdE0d7Dbd336b79148AA742883198BBF60342", 18),
    ("MATH", "0x99f40b01ba9c469193b360f72740e416b17ac332", 18),
    ("MBOX", "0xda661fa59320b808c5a6d23579fcfedf1fd3cf36", 18),
    ("MLK", "0x374c5fb7979d5fdbaad2d95409e235e5cbdfd43c", 18),
    ("MLN", "0x8f5c1a99b1df736ad685006cb6adca7b7ae4b514", 18),
    ("MORPHO", "0x40BD670A58238e6E230c430BBb5cE6ec0d40df48", 18),
    ("NOX", "0xf34450d1f23902657cffb2636153677be7d38750", 18),
    ("NRN", "0xdadeca1167fe47499e53eb50f261103630974905", 18),
    ("NST", "0x88a269df8fe7f53e590c561954c52fccc8ec0cfb", 18),
    ("NUT", "0x8697841b82c71fcbd9e58c15f6de68cd1c63fd02", 18),
    ("OBT", "0x1cd9a56c8c2ea913c70319a44da75e99255aa46f", 18),
    ("OHM", "0xf0cb2dc0db5e6c66B9a70Ac27B06b878da017028", 9),
    ("OLAS", "0x064f8b858c2a603e1b106a2039f5446d32dc81c1", 18),
    ("ORBS", "0xf3c091ed43de9c270593445163a41a876a0bb3dd", 18),
    ("ORDER", "0x4e200fe2f3efb977d5fd9c430a41531fb04d97b8", 18),
    ("OSAK", "0xbfd5206962267c7b4b4a8b3d76ac2e1b2a5c4d5e", 18),
    ("OSETH", "0xf7d4e7273E5015C96728A6b02f31C505eE184603", 18),
    ("PENDLE", "0x0c880f6761F1af8d9Aa9C466984b80DAb9a8c9e8", 18),
    ("PEPE", "0x25d887Ce7a35172C62FeBFD67a1856F20FaEbB00", 18),
    ("PERP", "0x753d224bcf9aafacd81558c32341416df61d3dac", 18),
    ("POKT", "0x764a726d9ced0433a8d7643335919deb03a9a935", 18),
    ("POND", "0xda0a57b710768ae17941a9fa33f8b720c8bd9ddd", 18),
    ("PYTH", "0xE4D5c6aE46ADFAF04313081e8C0052A30b6Dd724", 18),
    ("RAIN", "0x25118290e6A5f4139381D072181157035864099d", 18),
    ("RBTC", "0x441fcb23dfe8289cf572126fedcf450974adc891", 18),
    ("RDNT", "0x3082cc23568eA640225c2467653dB90e9250AaA0", 18),
    ("RENDER", "0xC8a4EeA31E9B6b61c406DF013DD4FEc76f21E279", 18),
    ("RETH", "0xEC70Dcb4A1EFa46b8F2D97C310C9c4790ba5ffA8", 18),
    ("RIF", "0xe5e851b01dd3eda24fde709a407db44555b6d1e0", 18),
    ("RIZ", "0x083fb956333f9c1568f66fc0d0be451f31f8c46c", 18),
    ("RLC", "0xe649e6a1f2afc63ca268c2363691cecaf75cf47c", 18),
    ("ROSA", "0xee0a242f28034fce0bdfac33c0ad2a58ec35fd38", 18),
    ("RPL", "0xb766039cc6db368759c1e56b79affe831d0cc507", 18),
    ("RSETH", "0x4186BFC76E2E237523CBC30FD220FE055156b41F", 18),
    ("RSR", "0xca5ca9083702c56b481d1eec86f1776fdbd2e594", 18),
    ("SDEX", "0xabd587f2607542723b17f14d00d99b987c29b074", 18),
    ("SFRXUSD", "0x5bff88ca1442c2496f7e475e9e7786383bc070c0", 18),
    ("SIS", "0x9e758b8a98a42d612b3d38b66a22074dc03d7370", 18),
    ("SKATE", "0x61dbbbb552dc893ab3aad09f289f811e67cef285", 18),
    ("SMURFCAT", "0x06e90a57d1ece8752d6ce92d1ad348ead5eae4f4", 18),
    ("SOPH", "0x31dba3c96481fde3cd81c2aaf51f2d8bf618c742", 18),
    ("SPA", "0x5575552988a3a80504bbaeb1311674fcfd40ad4b", 18),
    ("SPELL", "0x3e6648c5a70a150a88bce65f4ad4d506fe15d2af", 18),
    ("SQD", "0x1337420ded5adb9980cfc35f8f2b054ea86f8ab1", 18),
    ("STG", "0x6694340fc020c5E6B96567843da2df01b2CE1eb6", 18),
    ("SUSHI", "0xd4d42f0b6def4ce0383636770ef773390d85c61a", 18),
    ("SWEAT", "0xca7dec8550f43a5e46e3dfb95801f64280e75b27", 18),
    ("SYN", "0x080f6aed32fc474dd5717105dba5ea57268f46eb", 18),
    ("TBTC", "0x6c84a8f1c29108F47a79964b5Fe888D4f4D0dE40", 18),
    ("TEL", "0x0419e8bfbbb2623728c3a6129090da4ff4e48113", 18),
    ("TETH", "0xd09acb80c1e8f2291862c4978a008791c9167003", 18),
    ("TLOS", "0x193f4a4a6ea24102f49b931deeeb931f6e32405d", 18),
    ("TRADE", "0xe22c452bd2ade15dfc8ad98286bc6bdf0c9219b7", 18),
    ("TRB", "0xd58d345fd9c82262e087d2d0607624b410d88242", 18),
    ("UNI", "0xFa7F8980b0f1E64A2062791cc3b0871572f1F7f0", 18),
    ("UNITE", "0xb14448b48452d7ba076abeb3c505fc044deaf4e9", 18),
    ("USOL", "0x9b8df6e244526ab5f6e6400d331db28c8fdddb55", 18),
    ("UXRP", "0x2615a94df961278dcbc41fb0a54fec5f10a693ae", 18),
    ("VANA", "0x7ff7fa94b8b66ef313f7970d4eebd2cb3103a2c0", 18),
    ("VCHF", "0x02cea97794d2cfb5f560e1ff4e9c59d1bec75969", 18),
    ("VCNT", "0x60bf4e7cf16ff34513514b968483b54beff42a81", 18),
    ("VSN", "0x6fbbbd8bfb1cd3986b1d05e7861a0f62f87db74b", 18),
    ("W", "0xb0ffa8000886e57f86dd5264b9582b2ad87b2b91", 18),
    ("WAARBGHO", "0xd089b4cb88dacf4e27be869a00e9f7e2e3c18193", 18),
    ("WAARBUSDCN", "0x7f6501d3b98ee91f9b9535e4b0ac710fb0f9e0bc", 18),
    ("WAARBUSDT", "0xa6d12574efb239fc1d2099732bd8b5dc6306897f", 18),
    ("WAARBWETH", "0x4ce13a79f45c1be00bdabd38b764ac28c082704e", 18),
    ("WEETH", "0x35751007a407ca6feffe80b3cb397736d2cf4dbe", 18),
    ("WFRAX", "0x64445f0aecc51e94ad52d8ac56b7190e764e561a", 18),
    ("WM", "0x437cc33344a0b27a429f795ff6b469c72698b291", 18),
    ("WMTX", "0x7aefc9965699fbea943e03264d96e50cd4a97b21", 18),
    ("WOO", "0xcafcd85d8ca7ad1e1c6f82f651fa15e33aefd07b", 18),
    ("WSOL", "0x2bcc6d6cdbbdc0a4071e48bb3b969b06b3330c07", 18),
    ("WSTETH", "0x5979D7b546E38E414F7E9822514be443A4800529", 18),
    ("XAI", "0x4cb9a7ae498cedcbb5eae9f25736ae7d428c9d66", 18),
    ("XAUT0", "0x40461291347e1ecbb09499f3371d3f17f10d7159", 18),
    ("XSGD", "0xe333e7754a2dc1e020a162ecab019254b9dab653", 18),
    ("XVS", "0xc1eb7689147c81ac840d4ff0d298489fc7986d52", 18),
    ("YFI", "0x82e3a8f066a6989666b031d916c43672085b1582", 18),
    ("ZCHF", "0xd4dd9e2f021bb459d5a5f6c24c12fe09c5d45553", 18),
    ("ZRO", "0x6985884C4392D348587B19cb9eAAf157F13271cd", 18),
    ("ZTX", "0x1c43d05be7e5b54d506e3ddb6f0305e8a66cd04e", 18),
];

