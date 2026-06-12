<strategy_knowledge>
Scalping Execution Protocol (FID-126 — Conviction-Weighted, replaces 3+ Boolean gate):

OPERATIONAL DEFAULTS (FID-126 — preserved from v0.13.8, do not relax):

- Stop loss: 0.5% (or structure-based, whichever is tighter)
- Take profit: 0.8-1.2% (ATR-based)
- Spread filter: < 0.25% on the pair
- Preferred session window: 08:00-17:00 UTC (peak liquidity, US-EU overlap optimal)
- These are HARD CONSTRAINTS that apply to every entry, regardless of regime. They sit above the conviction-weighted framework.

CONVICTION-WEIGHTED DECISION FRAMEWORK:
The previous "3+ Action Triggers aligned" rule was a rigid Boolean gate that caused the model to Pass on setups with partial but valuable trigger alignment. This framework replaces it with a probabilistic conviction score that allows granular decision-making.

# Regime-Dependent Threshold Matrix

| Regime | ADX | ATR | Conviction Threshold | Trigger Equivalence | Risk Veto |
|---|---|---|---|---|---|
| Trending | ADX > 25 | ATR <= 2x baseline | 0.50 | 2+ Triggers | Standard invalidations |
| Volatile | any | ATR > 2x baseline | 0.60 | 2.5+ Triggers | Mandatory bear veto |
| Ranging | ADX < 20 | any | 0.75 | 3+ Mean-reversion triggers | Tight stop mandate |
| Grey Zone | 20 <= ADX <= 25 | ATR <= 2x baseline | 0.65 | 2.5+ Triggers, must include regime-disambiguating trigger | Standard invalidations |

Why Volatile needs MORE triggers than Trending (0.60 vs 0.50): Volatile regimes have higher noise-to-signal; requiring more aligned triggers + higher conviction prevents over-trading whipsaws. Risk veto is mandatory bear-only (no counter-trend longs in vol).

Why Ranging needs 0.75 + 3 mean-reversion triggers: Mean-reversion requires statistical confluence (Bollinger touch + RSI extreme + range position). One trigger = noise; three = signal.

Grey zone handling: ADX 20-25 is a "regime uncertainty" window. Require 2.5+ triggers AND a disambiguating trigger that explicitly resolves the regime (range-boundary break or trend-continuation higher-high). Default to Volatile's risk veto if no disambiguator.

# Trigger-to-Conviction Mapping

Triggers are weighted, not counted. Each trigger contributes a partial conviction score based on its quality:

Strong trigger (full weight 1.0): EMA cross with body > 50% of candle, VWAP bounce with volume, breakout above 20-period high with volume > 1.5x
Moderate trigger (weight 0.7): VWAP support hold, MACD cross, RSI oversold (<30) without divergence
Weak trigger (weight 0.4): Partial EMA alignment, BB touch, low-volume cross

conviction_score = clamp(sum(trigger_weights) / 3.0, 0.0, 1.0)

Example: 1 strong + 1 moderate + 1 weak = 1.0 + 0.7 + 0.4 = 2.1 / 3.0 = 0.70 — passes Trending (0.50) and Grey Zone (0.65) thresholds, fails Ranging (0.75).

# Fuzzy Volume Membership

Trapezoidal function: 0.25x avg = 0, 1.1x avg = 0.6, 1.5x+ avg = 1.0. Below-threshold volume contributes partial credit instead of failing Boolean. The volume trigger is itself a fuzzy input to the conviction calculation; it does not gate independently.

# ANTI-PATTERN BLOCK — MANDATORY

DO NOT default to conviction_score=0.50 or 0.65. Output a granular score based on actual trigger quality. Calibration will be measured by Brier Score (FID-130); defaulting to threshold values yields Brier > 0.30 and is a calibration failure.

Conviction score std dev across 60 scenarios must exceed 0.15. If your outputs cluster at 0.50/0.65 (the threshold values), the calibration gate fails.

# Few-Shot Example

<few_shot_example>
  <market_state>
    Regime: Trending (ADX 28)
    Triggers: EMA9 > EMA21 (1.0, strong), VWAP Support (0.7, moderate), Volume at 0.9x Average (0.4, weak)
    Session: Deep Asian
  </market_state>
  <reasoning>
    Cumulative trigger weights = 2.1/3.0 = conviction 0.70. Trending regime threshold = 0.50. Passes with margin. Volume is below 1.5x but fuzzy membership = 0.4, contributing partial credit. Deep Asian session no longer penalized (FID-129). RSI = 62, MACD = flat but positive, range position = mid. No risk veto triggers active.
  </reasoning>
  <action>
    Decision: BUY
    Conviction: 0.70
    Sizing_Multiplier: 0.75
    Regime: Trending
  </action>
</few_shot_example>

# Partial Compliance Is Permitted

7/10 checklist points + no critical invalidation = grounds for a low-conviction entry. The 10-point pre-trade checklist is now an EVALUATION MATRIX (FID-132), not a Boolean gate. Modifiers from missing criteria reduce conviction by their sum but do not auto-Pass.

# Out-of-Range Handling

If you cannot compute a conviction score, output 0.0 and select PASS. If conviction > 1.0, clamp to 1.0. If sizing_multiplier > 1.0, clamp to 1.0. The engine will reject (PASS) any decision where conviction < regime threshold.

# Schema Change Risk

The JSON schema in output_format.md now includes <conviction_score>, <sizing_multiplier>, and <regime_label>. Existing response captures (FID-124) are versioned; old captures remain comparable for A/B tests (FID-133).

ZERO-BASE PORTFOLIO REVIEW (FID-092 — Mandatory for All Position Evaluations):

The most effective technique for eliminating sunk cost bias. When evaluating an existing position, you MUST ask:

"If I held $0 of this asset and had its full value in cash today, would I initiate a new position at the current price with the current technicals?"

- If YES: HOLD (the setup is still valid regardless of entry price)
- If NO: CLOSE immediately (the market is telling you this is a bad allocation)

The historical entry price is economically irrelevant. The market does not know or care what you paid.

REGIME-SPECIFIC BEHAVIOR (Non-Negotiable):

Trending / Momentum (ADX > 25):
- Entry: Trigger weights must sum to >= 1.50 (conviction >= 0.50). See regime matrix above.
- Management: Tight stops (0.3-0.5%), single target, quick exits.
- New entries: Conviction-weighted triggers apply (do not require 3+ aligned).
- ADVERSE TREND RULE: If ADX > 25 AND position is underwater AND EMA is against direction → CLOSE immediately.

Volatile (ATR > 2x baseline):
- Conviction threshold 0.60. 2.5+ trigger weights required. Mandatory bear veto (no counter-trend longs).
- Reduce position size, widen stops slightly (but never beyond 0.8%)
- Prefer LIMIT orders over MARKET to avoid slippage

Ranging / Mean Reversion (ADX < 20):
- Conviction threshold 0.75. 3+ mean-reversion trigger weights required.
- Buy at support, sell at resistance. No momentum confirmation needed.
- Management: Tighter stops (0.3%), smaller targets (0.6-0.8%).
- Dead capital: Positions with flat/negative PnL after 5+ minutes → CLOSE.

Transition / Grey Zone (ADX 20-25):
- Conviction threshold 0.65. 2.5+ trigger weights with regime-disambiguating trigger.
- Existing positions must be re-evaluated. Tighten stops or close if thesis depends on trend continuation.

Session Awareness (UTC):
- 13:00-17:00: Peak liquidity (US-EU overlap). Optimal for scalps.
- 08:00-13:00: High liquidity (EU morning). Good for scalps.
- 02:00-06:00: Crypto is 24/7; no penalty applied. Arbitrum liquidity remains sufficient for $30 micro-capital accounts (FID-129). If 24h DEX volume is below 30-day average, consider reducing size by 20% (data-driven, not time-driven).

Confidence Discipline:
- Evaluate setup quality based on technicals, volume, and regime — NOT on existing position P&L.
- conviction_score replaces boolean "is this a good setup?" with continuous quality. Output the actual score, not a binary yes/no.
- Management actions (ADJUST_STOP, CLOSE) are NOT gated by the conviction threshold.
</strategy_knowledge>
