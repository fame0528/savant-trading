# Pair Expansion Analysis — Practical Recommendations

## Current State

| Metric | Value |
|--------|-------|
| Pairs | 8 |
| Cycle time | ~2 minutes |
| LLM calls per cycle | 8 parallel (20-60s each) |
| Buy signals fired | 2 in 12 hours (both rejected by position sizer) |
| Problem | All 8 pairs ranging, compressed ATR, no setups |

## Why Expand

The AI is correctly disciplined — it requires 3+ action triggers and valid R:R. But with only 8 pairs in a ranging market, there's nothing to trade. Meme coins offer:
- Higher volatility → more setups
- Faster momentum → AI speed advantage
- Dumb money → contrarian opportunities

## Recommended Pairs to Add

### Tier 1: High Confidence (add immediately)

| Pair | Why | Risk |
|------|-----|------|
| PEPE/USD | On Arbitrum, high volume, 0x supported | Wide spreads |
| WIF/USD | High volatility, trending often | Newer, less data |
| BONK/USD | Solana-bridged to Arbitrum, high volume | Bridge risk |
| SHIB/USD | Massive volume, tight spreads on 0x | Correlated with DOGE |

### Tier 2: Medium Confidence (add after testing)

| Pair | Why | Risk |
|------|-----|------|
| ARB/USD | L2 native, high volume, tight spreads | Correlated with ETH |
| UNI/USD | DeFi blue chip, good liquidity | Lower volatility |
| AAVE/USD | DeFi blue chip, good liquidity | Lower volatility |
| PENDLE/USD | DeFi narrative, moderate volume | Moderate volatility |

### Tier 3: Experimental (add if Tier 1 works)

| Pair | Why | Risk |
|------|-----|------|
| FLOKI/USD | Meme coin, moderate volume | Wide spreads |
| TURBO/USD | AI meme coin, high volatility | Very wide spreads |
| MOG/USD | Meme coin, moderate volume | Wide spreads |

## Implementation Plan

### Phase 1: Add 4 pairs (PEPE, WIF, BONK, SHIB)

```toml
pairs = [
    "BTC/USD", "ETH/USD", "SOL/USD", "XRP/USD",
    "DOGE/USD", "ADA/USD", "LINK/USD", "AVAX/USD",
    "PEPE/USD", "WIF/USD", "BONK/USD", "SHIB/USD",
]
```

- 12 pairs total
- Cycle time: ~3 minutes (12 parallel LLM calls)
- More opportunities, still manageable

### Phase 2: Add spread filter

```toml
[risk]
max_spread_bps = 100  # Skip pairs with >1% spread
```

### Phase 3: Add correlation cap

```toml
[risk]
max_position_correlation = 0.85  # Don't stack correlated positions
```

### Phase 4: Add timeframe support for meme coins

```toml
[trading.pairs.PEPE/USD]
timeframe = "1m"  # Faster entries for meme coins
```

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Wider spreads on meme coins | Spread filter (max_spread_bps) |
| Higher volatility = bigger losses | 1% risk per trade, 3 max positions |
| Correlation (meme coins move together) | Correlation cap (0.85) |
| Honeypot contracts | 0x API liquidity routing avoids thin pools |
| Bridge risk (BONK) | 0x handles routing, we just swap |

## Expected Impact

| Metric | Current | After Phase 1 |
|--------|---------|---------------|
| Pairs | 8 | 12 |
| Cycle time | 2 min | 3 min |
| Buy signals per day | 2 | 5-10 (estimated) |
| Avg volatility (ATR) | 0.5-1% | 2-5% |

## Gemini Research Needed

- Optimal pair count for $35 account
- Timeframe optimization for meme coins
- Spread threshold analysis
- Correlation matrix for proposed pairs
- Liquidity depth analysis on Arbitrum DEXes
