# Gemini Deep Research: Pair Expansion Strategy

## Research Question

We have an AI-driven DEX trading engine on Arbitrum with $35 USDC capital. It currently trades 8 high-liquidity pairs (BTC, ETH, SOL, XRP, DOGE, ADA, LINK, AVAX) but the AI can't find setups because all pairs are ranging with compressed ATR. We want to expand to include meme coins and higher-volatility tokens where the AI's speed advantage matters.

## Context

- **Engine:** Rust, AI makes decisions via mimo-v2.5-pro, executes on Arbitrum via 0x API
- **Capital:** $35 USDC (small account, ~$11 per position)
- **Risk:** 1% per trade ($0.35), 3 max positions, 5% daily loss limit, 10% drawdown kill switch
- **AI triggers:** Requires 3+ action triggers (EMA crossover, ADX>25, SOPR<1.0, MVRV<1.0, Fear<20, etc.)
- **Current pairs:** BTC/USD, ETH/USD, SOL/USD, XRP/USD, DOGE/USD, ADA/USD, LINK/USD, AVAX/USD
- **DEX:** 0x API on Arbitrum, Permit2 approval, EIP-1559 signing
- **Token resolution:** Any token symbol works via 0x API lookup (no hardcoded addresses needed)

## Questions for Research

### 1. Which meme coins have sufficient liquidity on Arbitrum DEXes?

We need tokens that:
- Have $100K+ daily volume on Arbitrum DEXes (Uniswap V3, SushiSwap, Camelot)
- Can be swapped via 0x API (it aggregates DEX liquidity)
- Have tight enough spreads (under 1%) for $11 position sizes
- Are available on Kraken for price data (our candle source)

Candidates:
- PEPE (already on Arbitrum)
- WIF (dogwifhat)
- BONK
- SHIB
- FLOKI
- TURBO
- MOG
- SPX6900

### 2. What's the optimal pair count for a $35 account?

With 8 pairs, the AI evaluates all every 2 minutes. With 20+ pairs:
- LLM evaluation time increases (each call takes 20-60s)
- More opportunities but slower cycles
- Need to balance coverage vs speed

### 3. Should we use different timeframes for meme coins?

Current: 5m candles for all pairs. Meme coins might need:
- 1m for faster entries (but more noise)
- 15m for cleaner structure
- 1h for trend confirmation

### 4. How to handle the spread problem on DEX?

Meme coins have wider spreads. Options:
- Add a spread filter (skip pairs with >1% spread)
- Use limit orders instead of market orders
- Only trade during high-volume periods
- Use the 0x API's `slippageBps` parameter more aggressively

### 5. What's the risk of getting rugged on Arbitrum meme coins?

- Check for honeypot contracts
- Verify liquidity lock status
- Check holder concentration
- Use 0x API's liquidity routing (it avoids thin pools)

### 6. Should we add a correlation filter for meme coins?

Meme coins tend to move together. If we add PEPE, WIF, and BONK:
- They're all highly correlated with BTC
- Opening positions in all 3 is effectively one bet
- Need a correlation cap (max 2 correlated positions)

## Expected Output

1. List of 10-15 recommended pairs with rationale
2. Optimal pair count recommendation
3. Timeframe recommendation for meme coins
4. Spread filter threshold
5. Risk management adjustments needed
6. Correlation filter recommendation

## Constraints

- Must work with 0x API on Arbitrum
- Must work with Kraken candle data
- Must not break existing 8-pair configuration
- Must maintain 1% risk per trade
- Must not exceed $35 capital constraints
