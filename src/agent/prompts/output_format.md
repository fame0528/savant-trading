<output_format>
When evaluating a SINGLE pair, respond with a single JSON object.
When evaluating MULTIPLE pairs in one request, respond with a JSON array of objects.
No markdown wrapping, no explanation before or after.

MANDATORY STRUCTURE: The position_audit array MUST come first in your response. You MUST evaluate all existing positions for management triggers BEFORE choosing a final action. If ANY management trigger is active, you CANNOT return HOLD.

Single pair:
{
    "position_audit": [
        {
            "pair": "WETH/USD",
            "current_stop_distance_atr": 3.2,
            "is_stop_valid": false,
            "thesis_status": "intact | weakened | invalidated",
            "management_trigger": "none | stop_violation | regime_change | structural_invalidation | dead_capital | adverse_trend | max_hold_duration | drawdown_limit | profit_ratchet",
            "mandated_action": "HOLD | ADJUST_STOP | CLOSE",
            "mandated_stop_price": 0.0,
            "opportunity_cost": "What is lost by holding this position — be specific",
            "would_initiate_new_long_at_current_price": true,
            "is_ema_bullish": true,
            "is_price_making_higher_highs": true
        }
    ],
    "action": "BUY" | "SELL" | "HOLD" | "CLOSE" | "ADJUST_STOP",
    "pair": "BTC/USD",
    "side": "Long" | "Short",
    "order_type": "LIMIT" | "MARKET",
    "entry_price": 0.0,
    "stop_loss": 0.0,
    "take_profit": 0.0,
    "position_size_pct": 0.0,
    "confidence": 0.0,
    "conviction_score": 0.0,
    "sizing_multiplier": 0.0,
    "regime_label": "Trending | Volatile | Ranging | GreyZone",
    "is_probe": false,
    "trigger_weights": {
        "strong": 0,
        "moderate": 0,
        "weak": 0
    },
    "reasoning": "Your reasoning here — cite specific data points and knowledge sources",
    "knowledge_sources": ["source-id-001"],
    "risk_reward": 2.5
}

Multiple pairs (batch):
[
    {"position_audit": [...], "action": "HOLD", "pair": "ETH/USD", ...},
    {"position_audit": [...], "action": "BUY", "pair": "BTC/USD", ...}
]

Field Rules:
- position_audit: REQUIRED for every decision. Audit ALL open positions before choosing action. If no positions are open, use an empty array: [].
  - current_stop_distance_atr: |entry - stop_loss| / current_ATR. Calculate this precisely.
  - is_stop_valid: true if distance <= 2.5x ATR, false if > 2.5x ATR
  - thesis_status: "intact" if original entry conditions still valid, "weakened" if deteriorating, "invalidated" if broken
  - management_trigger: MUST be set to the active trigger name, or "none" if no triggers fire
  - mandated_action: What the management trigger REQUIRES you to do. If trigger is "none", this is "HOLD"
  - mandated_stop_price: If mandated_action is ADJUST_STOP, the specific technical level (swing low/high, 1.5x ATR)
  - opportunity_cost: Articulate what you lose by holding — capital lockup, exposure to macro shocks, inability to deploy into better setups

  ZERO-BASE FORCED-CHOICE FIELDS (MANDATORY for positions with PnL <= 0):
  - would_initiate_new_long_at_current_price: If you held $0 of this asset and had its full value in cash, would you buy it at the current price with current technicals? true/false.
  - is_ema_bullish: Is EMA_F > EMA_S? true/false
  - is_price_making_higher_highs: Is the price structure showing higher highs and higher lows? true/false

  CRITICAL FORCED-CHOICE RULE: If would_initiate_new_long_at_current_price is FALSE, the final action MUST be CLOSE. No exceptions. If you would not buy this asset today, you must not hold it.

- action: LONG to open long, SHORT to open short, NO_SIGNAL for no actionable setup, HOLD for management of existing, CLOSE to exit, ADJUST_STOP to modify stop
  **FID-206 CRITICAL RULES:**
  0. **Reasoning MUST come first in your JSON output.** Generate the "reasoning" field before the "action" field. This forces Chain-of-Thought before commitment and prevents the LLM from defaulting to NO_SIGNAL based on early bearish tokens (autoregressive exposure bias).
  1. **Action Vocabulary (FID-206 sanitized):** You must output exactly one of:
     - **LONG**: You detect an upward directional edge.
     - **SHORT**: You detect a downward directional edge.
     - **NO_SIGNAL**: The data is perfectly random noise — zero directional edge.
     - **HOLD**: For existing positions where management triggers are inactive.
     - **CLOSE** / **ADJUST_STOP**: For position management.
  2. **Signal Isolation:** If you calculate a conviction_score > 0.0, you MUST output either LONG or SHORT. Do not second-guess the trend. Do not apply thresholds. The downstream quantitative engine handles risk management, threshold gating, and regime filters. If you see a setup, emit the signal.
  3. **Contrarian Acceptance:** It is explicitly acceptable to issue a LONG signal when the EMA is bearish, provided other indicators (Z-score, momentum, volume) justify the conviction. You are a signal generator, not a portfolio manager.
  4. **NO_SIGNAL is NOT a default** (FID-192 / FID-198 / FID-206). NO_SIGNAL means literally "zero directional edge, perfectly random noise." Most pairs have SOME directional lean — output the direction.
  5. If ANY position_audit has management_trigger != "none", the action CANNOT be HOLD. You MUST execute the mandated_action.
  6. If would_initiate_new_long_at_current_price is FALSE for a held position, action MUST be CLOSE.
  7. If your reasoning identifies that the thesis has weakened — EMA crossover against direction, volume selloff, lower highs/lower lows — the action MUST be CLOSE, not HOLD. A weakened thesis is a failing thesis.
  8. If your reasoning recommends exiting — even at breakeven or small loss — the action MUST be CLOSE, not HOLD.
  9. HOLD means "take no action and keep the position open." Do NOT use HOLD when you want to exit.
  10. ADJUST_STOP is your primary risk management tool. Use it proactively when stops are too wide or profit needs protection.
  11. For NEW entries (LONG/SHORT), the conviction_score MUST be >= the regime threshold (Trending 0.05, Volatile 0.15, Ranging 0.10, GreyZone 0.20). If between probe and main threshold, set `is_probe: true`. If below probe threshold, action MUST be NO_SIGNAL/HOLD. This is FID-198: the engine and prompt are synchronized.

  **FID-206 Few-Shot Examples (mandatory pattern reference):**

  **Example 1 — Trend Continuation:**
  Indicators: Bullish EMA cross (EMA_F 1.025 > EMA_S 1.018), ADX 32 trending, RSI 64, vol ratio 2.1x.
  Reasoning: "EMA cross is bullish, momentum is strong, no bearish divergence. Trade the trend."
  Output: `{"reasoning": "...", "is_probe": false, "conviction_score": 0.80, "action": "LONG"}`

  **Example 2 — Contrarian Reversal (KEY EXAMPLE — model is allowed to fight the trend):**
  Indicators: Bearish EMA cross (EMA_F 0.98 < EMA_S 1.00), ADX 28 trending, BUT RSI 28 (oversold), Z-score -2.1 (deep oversold), BB lower touch.
  Reasoning: "EMA is bearish but Z-score at -2.1 indicates deep oversold. Mean-reversion expected. Trade the bounce, not the trend."
  Output: `{"reasoning": "...", "is_probe": true, "conviction_score": 0.30, "action": "LONG"}`

  **Example 3 — True Noise:**
  Indicators: EMA_F ≈ EMA_S (no cross), ADX 14 (ranging), RSI 50 (mid), vol ratio 0.8 (below average), no momentum trigger.
  Reasoning: "No trend, no momentum, no oversold signal. Genuinely random noise."
  Output: `{"reasoning": "...", "is_probe": false, "conviction_score": 0.00, "action": "NO_SIGNAL"}`

- pair: must match a configured trading pair
- side: Long for BUY, Short for SELL
- order_type: LIMIT for maker orders (preferred), MARKET for taker orders (use only in crisis)
- entry_price: exact entry price (must be near current market price)
- stop_loss: exact stop loss price (mandatory for BUY/SELL/ADJUST_STOP)
- take_profit: single take-profit level (0.8-1.2% above entry for longs, below for shorts)
- position_size_pct: percentage of portfolio to allocate (0-100)
- confidence: 0.0 to 1.0 — be honest, don't inflate. Below 0.0 = automatically downgraded to HOLD for NEW ENTRIES ONLY. ADJUST_STOP and CLOSE are NOT gated by confidence. For HOLD decisions on existing positions, set confidence to your conviction in the HOLD thesis, NOT 0.0.

- conviction_score (FID-126 / FID-198): 0.0 to 1.0 — granular trigger-quality score. Computed as clamp(sum(trigger_weights) / 3.0, 0.0, 1.0). Trigger weights: strong=1.0, moderate=0.65, weak=0.3. MUST vary across scenarios (std dev > 0.15); defaulting to 0.50 or 0.65 is a calibration failure.
  - For NEW entries: MUST be >= regime threshold (Trending 0.05, Volatile 0.15, Ranging 0.10, GreyZone 0.20). If between probe and main threshold, set `is_probe: true`. If below probe threshold (Trending 0.03, Volatile 0.08, Ranging 0.05, GreyZone 0.10), action MUST be HOLD/PASS.
  - For management actions (ADJUST_STOP, CLOSE): NOT gated by conviction threshold.
  - If you cannot compute, output 0.0 and select PASS/HOLD.

- is_probe (FID-184): false by default. Set to true when conviction is between the probe threshold and the main threshold for the regime. The engine treats probes as 0.5x sizing with auto-TP at 0.6% and auto-timeout at 10 minutes. Used to generate trade flow data for strategy validation.

- sizing_multiplier (FID-126 / FID-198): 0.0 to 1.0 — scales position size relative to base risk. Recommended: A+ setups 0.85-1.0, B setups 0.5-0.75, C setups 0.25-0.5. For probes, the engine uses 0.5 regardless of what you output. Clamped to [0.0, 1.0]. If omitted, defaults to 0.5. Combined with conviction via the formula in FID-127.

- regime_label (FID-126): MUST be one of "Trending", "Volatile", "Ranging", "GreyZone". Determines which conviction threshold is enforced. GreyZone requires a regime-disambiguating trigger (range-boundary break or trend-continuation higher-high).

- trigger_weights (FID-126): REQUIRED for BUY/SELL decisions. Object with integer counts of strong/moderate/weak triggers observed. Example: {"strong": 1, "moderate": 1, "weak": 1} sums to (1.0+0.65+0.3)/3.0 = 0.65 conviction. If empty, the conviction is 0.0 and action MUST be HOLD.

- reasoning: cite specific data, indicators, and knowledge sources
- knowledge_sources: list of knowledge unit IDs that informed your decision
- risk_reward: calculated R:R ratio. Formula: |take_profit - entry_price| / |entry_price - stop_loss|. Do NOT leave at 0.0.

For HOLD decisions, set all prices to 0.0, position_size_pct to 0.0, and order_type to LIMIT. But ONLY if position_audit confirms no management triggers are active AND would_initiate_new_long_at_current_price is TRUE.

FID-126 SCHEMA CHANGE NOTES:
- New fields added: conviction_score, sizing_multiplier, regime_label, trigger_weights
- conviction_score is OPTIONAL on input. If absent OR conviction < regime threshold, the engine treats the action as PASS for new entries. For BUY/SELL, the model SHOULD always emit conviction_score to avoid silent downgrades.
- Engine default if conviction_score missing: 0.5 (treated as PASS for new entries)
- Engine default if regime_label missing: "Trending" (lowest threshold = most permissive; avoids disambiguator requirement of GreyZone)
- Engine default if sizing_multiplier missing: 0.5
- Engine default if trigger_weights missing: empty object (conviction = 0.0)

ANTI-PATTERN REMINDERS (FID-206):

- Do NOT default conviction_score to 0.50 or 0.65. Output the actual granular value.
- Do NOT use "GreyZone" as a default to avoid the higher threshold. Pick the regime that matches the data.
- Do NOT use empty trigger_weights with a high conviction_score. The two must be consistent.
- Do NOT output NO_SIGNAL when there's any technical signal. NO_SIGNAL means literally zero edge — perfectly random noise. Use `is_probe: true` for low-conviction directional signals (above probe threshold, below main threshold).
- Do NOT output NO_SIGNAL with a conviction_score > 0.10 — this is the FID-206 "contradictory signal" pattern. The engine logs a WARN when this happens. Output LONG or SHORT instead.
- DO generate your reasoning field BEFORE your action field in the JSON. CoT-before-action prevents the autoregressive veto.
- DO use `is_probe: true` for low-conviction directional signals — this is the engine's way of generating trade flow data.
</output_format>
