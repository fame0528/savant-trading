# Gemini Deep Research Prompt: Generate Snipe-Specific Crypto Knowledge Units

---

**Copy everything below this line into Gemini Deep Research:**

---

I'm building an autonomous crypto trading bot called "Savant Trading" that runs on Arbitrum DEX via the 0x API. It has a $30 micro-capital account and makes decisions every 5-minute candle using an LLM.

## The Problem

My current knowledge base is 74.5% institutional/swing-trading content (from books like Murphy, Wyckoff, Tharp, Minervini) and only 25.5% crypto-native. The institutional content teaches the model to require "3+ aligned triggers", "wait for confirmation", and "avoid ranging markets" — which is exactly backwards for crypto sniping, where speed beats confirmation and ranging/quiet markets are where snipes set up.

I've already fixed the FILTER (MMR selector now penalizes institutional content at 0.5x and boosts crypto-native at 2.0x). Now I need to fill the CONTENT gap with snipe-specific knowledge units.

## What I Need From You

Generate **50-100 high-quality knowledge units** specifically for **crypto altcoin sniping** — entering small-cap, high-volatility positions on short timeframes (5m-15m candles) with the goal of catching quick moves (5-30% in minutes to hours).

## Knowledge Unit Format (CRITICAL — must match exactly)

Each knowledge unit must be a JSON object with this exact structure:

```json
{
  "id": "snipe-001",
  "source": "snipe-deep-research-2026-06-12",
  "topic": "CryptoNative",
  "conditions": ["Trending", "HighVolatility"],
  "content": "Knowledge text here — 1-3 sentences, actionable, specific",
  "priority": 4,
  "tags": ["scalp", "altcoin", "5m-candle", "volume-spike"]
}
```

**Field requirements:**
- `id`: `snipe-XXX` (sequential 001-100)
- `source`: always `"snipe-deep-research-2026-06-12"`
- `topic`: always `"CryptoNative"`
- `conditions`: array from `["Trending", "Ranging", "HighVolatility", "LowVolatility", "ExtremeFear", "ExtremeGreed", "VolumeExpansion"]`
- `content`: 1-3 sentences, actionable, specific to snipe use case. Include the trigger, the action, and the risk.
- `priority`: 3-5 (5 = most important)
- `tags`: include `"scalp"` on EVERY unit. Add specific tags like `"volume-spike"`, `"5m-candle"`, `"altcoin"`, `"liquidity-grab"`, `"breakout"`, `"mean-reversion"`, `"dex-execution"`, `"micro-cap"`, `"order-flow"`, `"stop-hunt"`, etc.

## Output Format

Return ONLY a valid JSON array of knowledge unit objects. No prose, no markdown code blocks, no explanation. Just the array.

Example of 2 units:

```json
[
  {
    "id": "snipe-001",
    "source": "snipe-deep-research-2026-06-12",
    "topic": "CryptoNative",
    "conditions": ["HighVolatility", "VolumeExpansion"],
    "content": "VOLUME SPIKE ENTRY: When 1m volume exceeds 3x the 20-period average within a 5m candle, enter on the next candle open. Stop = wick of the spike candle. Target = 1.5-2x the spike candle's range. Works on low-cap altcoins during US-EU overlap when liquidity sweeps Asian session highs/lows.",
    "priority": 5,
    "tags": ["scalp", "volume-spike", "5m-candle", "altcoin", "liquidity-sweep"]
  },
  {
    "id": "snipe-002",
    "source": "snipe-deep-research-2026-06-12",
    "topic": "CryptoNative",
    "conditions": ["Ranging", "LowVolatility"],
    "content": "RANGING MARKET SNIPE: Tight 15-30 minute range on low-cap altcoin = coiled spring. Enter on breakout of range high/low with volume confirmation (1.5x+ average). Stop = opposite side of range. Target = range width projected from breakout. Size up because stop is tight and win rate is high.",
    "priority": 5,
    "tags": ["scalp", "breakout", "range-compression", "altcoin", "micro-cap"]
  }
]
```

## Required Topic Coverage

Generate units across these 7 categories (distribute roughly evenly):

### Category 1: Entry Signals (15-20 units)
- Volume spike patterns (1m, 5m, 15m)
- Liquidity grabs / stop hunts
- Order book imbalance signals
- RSI divergence on 5m
- EMA cross on 5m/15m
- Break of structure on 1m/5m
- CVD (Cumulative Volume Delta) divergences
- Funding rate flips on perps (for context)
- Open interest spikes
- Whale wallet activity (on-chain)

### Category 2: Exit Signals (10-15 units)
- Partial profit taking (TP1 at 1R, TP2 at 2R, TP3 at 3R)
- Trailing stop methods (ATR-based, candle-based, structure-based)
- Time-based exits (close if not moving in 15-30 min)
- Volume exhaustion (declining volume on continuation = exit signal)
- Regime change exits (breakout fails, ranging starts)
- MEV/sandwich risk exits
- Gas spike exits (Arbitrum gas > 0.5 gwei = reconsider)

### Category 3: Regime Detection for Small-Caps (8-10 units)
- How to detect trending regime on 5m (ADX > 25, higher highs/lows)
- How to detect ranging regime on 5m (ADX < 20, equal highs/lows)
- How to detect breakout regime (compression then expansion)
- How to detect exhaustion (divergences + volume decline)
- Altcoin-specific: when BTC dominance is rising, alts underperform

### Category 4: DEX Execution for $30 Account (8-10 units)
- 0x API slippage tolerance (default 1%, raise to 2-3% for low-caps)
- Gas optimization (Arbitrum, batch when possible)
- Position sizing for micro-capital (full deploy on A+ setups, 50% on B, 25% on C)
- Max position size (% of portfolio per trade)
- When to skip the trade (gas > potential profit)
- 0x API quote vs actual execution (MEV protection)
- Token approval before swap
- Failed swap recovery (what to do when 0x returns dust)

### Category 5: Risk Management for Micro-Capital (8-10 units)
- Max loss per trade (2-3% of $30 = $0.60-0.90)
- Max daily loss (10% = $3, then stop for the day)
- Max weekly loss (20% = $6, then stop for the week)
- Correlation risk (don't snipe 3 memecoins at once)
- Drawdown halt (after 3 consecutive losses, reduce size 50%)
- Recovery mode (after a loss, size down until next win)
- When to add to a winner (scaling in)
- When to cut a loser (no averaging down on snipes)

### Category 6: Session & Timing (5-8 units)
- Best hours for altcoin snipes (US-EU overlap 13:00-20:00 UTC)
- Worst hours (Asian session 00:00-08:00 UTC has lower volume but also less competition)
- Weekend behavior (crypto trades 24/7, weekends can be slow or volatile)
- News event windows (CPI, FOMC, exchange listings — avoid or trade the reaction)
- Token unlock schedules (avoid sniping right before unlocks)

### Category 7: Token Selection for Snipes (5-8 units)
- Market cap range ($10M-$500M is the sweet spot)
- 24h volume threshold ($500K+ minimum)
- Holder count (1K+ minimum, 10K+ preferred)
- Liquidity depth (check 0x quote before entry)
- Recent listing (first 2 weeks after listing = most volatile)
- Social signals (Twitter mentions, Telegram activity)
- Avoid: tokens with honeypot risk, high sell tax, unverified contracts

## Critical Constraints

- **NO institutional/swing-trading language** — avoid "3+ confirmations", "always wait for", "patience", "avoid ranging markets"
- **Crypto-native only** — examples should be from altcoins (SOL, ARB, PEPE, DOGE, WLD, etc.), not stocks or forex
- **Speed over confirmation** — snipes are about catching the move early, not waiting for full proof
- **Specific numbers** — include exact thresholds (e.g., "ADX > 25", "volume > 1.5x", "stop at 2%")
- **Actionable** — each unit should tell the model WHAT to do, not just describe a concept
- **Include the risk** — each unit should mention what can go wrong and when NOT to use the signal
- **Micro-capital aware** — examples should reference $30-scale accounts, not $10K+ accounts
- **Every unit gets the "scalp" tag** — this activates the 5.0x MMR boost

## Quality Bar

A good knowledge unit:
- Tells the model exactly when to act
- Includes specific numerical thresholds
- Mentions what to do if wrong
- Is 1-3 sentences (concise, not a paragraph)
- Uses crypto-native terminology (altcoin, DEX, gas, liquidity, etc.)

A bad knowledge unit:
- "Markets are complex and require patience" (too vague)
- "Consider all factors before trading" (not actionable)
- "Use proper risk management" (no specifics)
- "Wait for the right setup" (this is the problem we're fixing)

## What I Will Do With Your Output

1. Parse the JSON array
2. Validate each unit has all required fields
3. Merge into `knowledge/knowledge_crypto_native.json`
4. Re-run the sandbox to measure divergence count change
5. Keep units that improve performance, remove or rewrite units that don't

Generate the array now. Aim for 60-80 high-quality units. Better to have 50 excellent units than 100 mediocre ones.
