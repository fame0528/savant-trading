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
            "management_trigger": "none | stop_violation | regime_change | structural_invalidation | dead_capital | profit_ratchet",
            "mandated_action": "HOLD | ADJUST_STOP | CLOSE",
            "mandated_stop_price": 0.0,
            "opportunity_cost": "What is lost by holding this position — be specific"
        }
    ],
    "action": "BUY" | "SELL" | "HOLD" | "CLOSE" | "ADJUST_STOP",
    "pair": "BTC/USD",
    "side": "Long" | "Short",
    "order_type": "LIMIT" | "MARKET",
    "entry_price": 0.0,
    "stop_loss": 0.0,
    "take_profit_1": 0.0,
    "take_profit_2": 0.0,
    "take_profit_3": 0.0,
    "position_size_pct": 0.0,
    "confidence": 0.0,
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

- action: BUY to open long, SELL to open short, HOLD for no action, CLOSE to exit existing, ADJUST_STOP to modify stop
  **CRITICAL RULES:**
  1. If ANY position_audit has management_trigger != "none", the action CANNOT be HOLD. You MUST execute the mandated_action.
  2. If your reasoning recommends exiting — even at breakeven or small loss — the action MUST be CLOSE, not HOLD.
  3. HOLD means "take no action and keep the position open." Do NOT use HOLD when you want to exit.
  4. ADJUST_STOP is your primary risk management tool. Use it proactively when stops are too wide or profit needs protection.

- pair: must match a configured trading pair
- side: Long for BUY, Short for SELL
- order_type: LIMIT for maker orders (preferred), MARKET for taker orders (use only in crisis)
- entry_price: exact entry price (must be near current market price)
- stop_loss: exact stop loss price (mandatory for BUY/SELL/ADJUST_STOP)
- take_profit_1/2/3: three take-profit levels (TP1 nearest, TP3 farthest)
- position_size_pct: percentage of portfolio to allocate (0-100)
- confidence: 0.0 to 1.0 — be honest, don't inflate. Below 0.40 = automatically downgraded to HOLD for NEW ENTRIES ONLY. ADJUST_STOP and CLOSE are NOT gated by confidence.
- reasoning: cite specific data, indicators, and knowledge sources
- knowledge_sources: list of knowledge unit IDs that informed your decision
- risk_reward: calculated R:R ratio. Formula: |take_profit_1 - entry_price| / |entry_price - stop_loss|. Do NOT leave at 0.0.

For HOLD decisions, set all prices to 0.0, position_size_pct to 0.0, and order_type to LIMIT. But ONLY if position_audit confirms no management triggers are active.
</output_format>
