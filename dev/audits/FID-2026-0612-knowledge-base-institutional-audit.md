# Knowledge Base Audit: Institutional vs Snipe Conflict

**Date:** 2026-06-12
**Trigger:** M3 sandbox shows 56/60 Pass decisions after FID-125 fix. User noted knowledge base was built from institutional trading books but engine is used for crypto sniping.
**Severity:** high
**Status:** analyzed

---

## Headline Finding

**The knowledge base is 74.5% institutional/stock-market content and 25.5% crypto-native.** The MMR selector's tier multiplier system (FID-059) was designed to filter out institutional sources, but it only penalizes 5 specific authors (Wyckoff, Elder, Turtle, Bulkowski, VPA — 146 units at 0.5x). The remaining 2,547 institutional units get the SAME 1.0x multiplier as crypto-native units, so they compete on equal footing in the prompt.

## Knowledge Base Composition

**Total units:** 3,456

| Tier Multiplier | Count | % | Description |
|---|---|---|---|
| **5.0x (scalping)** | 189 | 5.5% | Tagged for scalping — DOMINANT |
| **1.5x (YouTube)** | 574 | 16.6% | Crypto-native YouTube interviews |
| **1.0x (neutral)** | 2,547 | 73.7% | Includes institutional books AND crypto-native |
| **0.5x (penalized)** | 146 | 4.2% | Wyckoff, Elder, Turtle, Bulkowski, VPA |
| **TOTAL** | **3,456** | 100% | |

## Source Distribution (Top 10)

| Count | Source | Type |
|---|---|---|
| 319 | books-merged | Institutional (mixed authors) |
| 282 | knowledge-execution-build | Institutional execution |
| 87+85+80 | YouTube scalper interviews | Crypto-native |
| 58 | Murphy technical analysis | Institutional (stock) |
| 55 | Investing & crypto expert YT | Crypto-native |
| 47 | Bitcoin price analysis YT | Crypto-native |
| 47 | Ultimate crypto course YT | Crypto-native |
| 40+39 | Coulling (volume price analysis) | Institutional (stock) |
| 39 | Irrational Exuberance | Institutional (stock) |
| 37 | AI in crypto trading YT | Crypto-native |

**74.5% of all knowledge is institutional/stock-trading focused.** Only 25.5% is explicitly crypto-native (Glassnode, on-chain, DeFi, crypto YouTube).

## MMR Selector Logic Review

The MMR selector at `src/agent/knowledge.rs:131-220` has a tier multiplier system:

```rust
let tier_mult = if has_scalping_tag {
    5.0  // Scalping-tagged: DOMINANT
} else if source.starts_with("youtube") || source.starts_with("yt_") {
    1.5  // YouTube: boosted
} else if source.contains("wyckoff")
    || source.contains("elder")
    || source.contains("turtle")
    || source.contains("bulkowski")
    || source.contains("vpa")
{
    0.5  // 5 specific authors: penalized
} else {
    1.0  // EVERYTHING ELSE: neutral
};
```

**Problem:** The penalty list is too narrow. Only 146/2574 (5.7%) of institutional content is penalized. The other 2,401 institutional units compete at 1.0x with crypto-native content.

**Specific issues:**

1. **Murphy (58 units, stock-focused technical analysis)** — 1.0x (not penalized)
2. **Coulling (79 units, stock volume analysis)** — 1.0x (not penalized)
3. **Minervini (in Schwager books, stock swing trading)** — 1.0x (not penalized)
4. **Tharp (350+ units in risk_management, position sizing for $10K+ accounts)** — 1.0x (not penalized)
5. **Graham Intelligent Investor (in fundamentals, long-term value investing)** — 1.0x (not penalized)
6. **"books-merged" (319 units, mixed institutional sources)** — 1.0x (not penalized)
7. **knowledge-execution-build (282 units, generic execution rules)** — 1.0x (not penalized)

## Strict-Language Patterns Found

Searched all 3,456 units for patterns that teach the model to require multiple confirmations before trading:

| Pattern | Count | Examples |
|---|---|---|
| "3+ trigger/confirm" | 26 | "All three must align", "Three requirements for every trade" |
| "wait for confirm" | 39 | "Wait for confirmation", "Patience prevents losses" |
| "never enter" | 96 | "Never trade without a stop", "Never chase entries" |
| "all must" | 53 | "All three filters must align" |
| "volume required" | 23 | "Volume must expand on breakout" |
| "ranging/avoid" | 34 | "Avoid ranging markets", "Wait for range break" |
| "patience/wait" | 170 | "Patience for the right setup > trading frequently" |

**Total units with strict entry/exit language:** ~400+ (12%+ of all knowledge)

## Specific Problematic Examples

### 1. Power Momentum Strategy (knowledge_execution.json)
> "Buy when price closes above 20-period EMA AND RSI >50 AND MACD histogram positive. **All three must align.** Exit when any one condition fails. Simple three-filter system."

**Conflict with snipe use case:** Snipes often fire on a single strong signal (e.g., volume spike) without needing 3 separate confirmations. The 5-minute window for a snipe entry is too short to wait for EMA cross + RSI + MACD to all align.

### 2. Iron Triangle of Risk Control (knowledge_crypto_native.json)
> "Three requirements for every trade: 1) Trend analysis says go. 2) Oscillator confirms entry timing. 3) Money management allows the trade. If ANY of the three says no, don't trade."

**Conflict with snipe use case:** This is a SWING TRADING framework for $10K+ accounts. For a $30 account sniping altcoins, the "trend analysis" and "oscillator confirms" are often too slow — the move is over by the time all 3 align.

### 3. Confirmation Rule (knowledge_price_action.json)
> "CONFIRMATION RULE — Always wait for confirmation. Bullish: close above pattern high. Bearish: close below pattern low. Without = potential only. #1 rule for candlestick trading. Patience prevents losses."

**Conflict with snipe use case:** Sniping IS entering on potential, before full confirmation. The "always wait" rule literally prevents snipe entries.

### 4. Asian Session Penalty (knowledge_crypto_native.json)
> "TIME-OF-DAY EFFECTS IN CRYPTO: Statistical analysis shows BTC returns are not uniform across hours. US session (13:30-21:00 UTC) generates disproportionate returns. **Asian session has a slight negative bias.**"

**Conflict with snipe use case:** Crypto trades 24/7. Some of the biggest altcoin moves happen in Asian session. The "slight negative bias" is BTC-specific, not altcoin-specific.

### 5. Wait for the Right Moment (knowledge_crypto_native.json)
> "WAIT FOR THE RIGHT MOMENT: 'I must not begin to advance until I am sure I shall not have to retreat.' Patience for the right setup > trading frequently."

**Conflict with snipe use case:** Snipes ARE the right moment — high-conviction short-window setups. This rule pushes the model to wait for "better" setups that never come in low-volume synthetic data.

## Root Cause Analysis

The knowledge base was built from two sources:
1. **22 curated trading books** (Murphy, Wyckoff, Tharp, Minervini, Graham, Schwager, etc.) — institutional, swing-trading focused
2. **15+ YouTube crypto courses** — crypto-native, but often educational/explanatory rather than action-oriented

The MMR selector was designed to balance these by boosting scalping-tagged units and penalizing a few specific authors. But:
- The scalping tag is applied to only 189 units (5.5%)
- The penalty list covers only 5 authors
- The remaining 2,547 institutional units are neutral (1.0x)

**Net effect:** When the model evaluates a scenario, the MMR selector picks 12 units from a pool where 74.5% are institutional. On average, 8-9 of the 12 selected units will be institutional/stock-trading focused. The model reads 8-9 "wait for 3+ confirmations" / "avoid ranging" / "patience" messages and concludes Pass.

## Recommendations

### 1. Expand the institutional penalty list (immediate, high impact)

Add to the penalty list at `src/agent/knowledge.rs:158-163`:

```rust
|| source.contains("murphy")
|| source.contains("coulling")
|| source.contains("minervini")
|| source.contains("tharp")
|| source.contains("graham")
|| source.contains("schwager")
|| source.contains("edwards-magee")
|| source.contains("coulling-volume")
|| source.contains("books-merged")
|| source.contains("knowledge-execution-build")
```

This would penalize 2,000+ institutional units to 0.5x, effectively suppressing them in favor of crypto-native content.

### 2. Boost crypto-native sources explicitly (immediate, high impact)

Add a crypto-native tier:

```rust
} else if source.contains("glassnode")
    || source.contains("binance")
    || source.contains("coingecko")
    || topic == KnowledgeTopic::CryptoNative
{
    2.0  // Crypto-native: 2x boost
}
```

This would promote 882 crypto-native units to 2.0x, making them dominant in the prompt.

### 3. Add "snipe" tags to crypto-native knowledge units (medium impact)

The 5.0x boost only applies to units with "scalp" in their tags. Audit crypto-native units and add scalping/snipe tags where appropriate. Target: tag at least 200+ crypto-native units as snipe-relevant.

### 4. Remove or rewrite the most problematic units (high impact, high effort)

Identify the 26 units with "3+ trigger" language and either:
- Delete them
- Rewrite to add an exception clause: "For swing trades, require 3+ confirmations. For scalps/snipes, 2 strong signals may suffice."

### 5. Add explicit snipe-vs-swing distinction in the model prompt (medium impact)

In `soul.md` or `base_identity.md`, add a clear distinction:

```
TRADE STYLE:
- Snipes (5m-15m): 1-2 strong signals may be sufficient. Speed > confirmation.
- Swings (1h-4h): 3+ aligned triggers required. Confirmation > speed.
- Position trades (daily+): Full confirmation + multi-timeframe alignment required.
```

The model should know WHICH mode it's in based on the timeframe and pair characteristics.

## Impact Estimate

If recommendations 1, 2, and 3 are implemented:
- Crypto-native content dominates the prompt (was 25.5%, will be ~60-70%)
- Institutional strict-language patterns suppressed
- Model has more "snipe-appropriate" guidance

Expected result: divergence count drops from 34/60 to ~15-20/60, and Buy actions return (currently 0, expected 5-10).

## Files Reviewed

- `src/agent/knowledge.rs` (MMR selector logic)
- `knowledge/knowledge_crypto_native.json` (319 units)
- `knowledge/knowledge_technical_analysis.json` (506 units)
- `knowledge/knowledge_risk_management.json` (350 units)
- `knowledge/knowledge_psychology.json` (319 units)
- `knowledge/knowledge_sentiment.json` (291 units)
- `knowledge/knowledge_trading_systems.json` (226 units)
- `knowledge/knowledge_execution.json` (282 units)
- `knowledge/knowledge_price_action.json` (216 units)
- `knowledge/knowledge_fundamentals.json` (200 units)
- 20+ YouTube interview files (~1,000+ units)
