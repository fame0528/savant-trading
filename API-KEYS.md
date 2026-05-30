# API Keys Reference

All data sources are **free, no keys required**. Optional keys unlock additional features.

---

## All Free (No Keys Needed)

| Service | Endpoint | Data | Used For |
|---------|----------|------|----------|
| Kraken Futures | `https://futures.kraken.com/derivatives/api/v3/tickers` | Funding rates, OI, mark prices | Derivatives positioning |
| Fear & Greed | `https://api.alternative.me/fng/?limit=1` | Market sentiment 0-100 | Contrarian signals |
| CoinGecko Global | `https://api.coingecko.com/api/v3/global` | BTC dominance, total market cap | Macro context |
| CoinGecko Price | `https://api.coingecko.com/api/v3/simple/price` | Per-coin price + volume | Price data |
| CoinGecko Trending | `https://api.coingecko.com/api/v3/search/trending` | Trending coins | Social sentiment |
| blockchain.info | `https://blockchain.info/q/getblockcount` | Block height, mempool, tx count | On-chain activity |
| Cointelegraph RSS | `https://cointelegraph.com/rss` | Crypto news | Breaking news |
| CoinDesk RSS | `https://www.coindesk.com/arc/outboundfeeds/rss/?outputType=xml` | Crypto news | Breaking news |
| CryptoSlate RSS | `https://cryptoslate.com/feed/` | Crypto news | Breaking news |
| Decrypt RSS | `https://decrypt.co/feed` | Crypto news | Breaking news |
| CryptoNews RSS | `https://cryptonews.com/news/feed/` | Crypto news | Breaking news |
| CryptoPotato RSS | `https://cryptopotato.com/feed/` | Crypto news | Breaking news |
| CryptoBreaking RSS | `https://www.cryptobreaking.com/feed/` | Crypto news | Breaking news |
| The Defiant RSS | `https://thedefiant.io/feed/` | DeFi news | DeFi signals |
| SmartLiquidity RSS | `https://smartliquidity.info/feed/` | DeFi news | DeFi signals |
| Blockworks RSS | `https://blockworks.co/feed` | Institutional news | Institutional flows |
| Bitcoin Magazine RSS | `https://bitcoinmagazine.com/feed` | Bitcoin news | BTC-specific |
| Ethereum 2.0 RSS | `https://benjaminion.xyz/newineth2/rss_feed.xml` | Ethereum news | ETH-specific |
| Yahoo Finance RSS | `https://finance.yahoo.com/news/rssindex` | Mainstream finance | Macro context |
| CNBC RSS | `https://www.cnbc.com/id/10000664/device/rss/rss.html` | Mainstream finance | Macro context |
| KriptoNovini RSS | `https://kriptonovini.bg/rss.xml` | Regional crypto news | Sentiment breadth |

---

## Optional (Enhanced Features)

| Service | What It Adds | Signup | Env Var |
|---------|-------------|--------|---------|
| CryptoPanic | Aggregated news with sentiment scoring | https://cryptopanic.com/developers/ | `CRYPTOPANIC_API_KEY` |

---

## .env Template

```bash
# === Optional (enhanced features) ===
CRYPTOPANIC_API_KEY=

# === OpenGateway (AI provider — has built-in defaults) ===
OPENGATEWAY_API_KEY=

# === Kraken (required for live trading only) ===
KRAKEN_API_KEY=
KRAKEN_API_SECRET=
```
