<output_format>
When evaluating a SINGLE pair, respond with a single JSON object.
When evaluating MULTIPLE pairs in one request, respond with a JSON array of objects.
No markdown wrapping, no explanation before or after.

Single pair:
{
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
    {"action": "HOLD", "pair": "ETH/USD", ...},
    {"action": "BUY", "pair": "BTC/USD", ...}
]

Field Rules:
- action: BUY to open long, SELL to open short, HOLD for no action, CLOSE to exit existing, ADJUST_STOP to modify stop
- pair: must match a configured trading pair
- side: Long for BUY, Short for SELL
- order_type: LIMIT for maker orders (preferred), MARKET for taker orders (use only in crisis)
- entry_price: exact entry price (must be near current market price)
- stop_loss: exact stop loss price (mandatory for BUY/SELL)
- take_profit_1/2/3: three take-profit levels (TP1 nearest, TP3 farthest)
- position_size_pct: percentage of portfolio to allocate (0-100)
- confidence: 0.0 to 1.0 — be honest, don't inflate. Below 0.40 = automatically downgraded to HOLD.
- reasoning: cite specific data, indicators, and knowledge sources
- knowledge_sources: list of knowledge unit IDs that informed your decision
- risk_reward: calculated R:R ratio from your entry, stop_loss, and take_profit_1 values. Formula: |take_profit_1 - entry_price| / |entry_price - stop_loss|. Do NOT leave at 0.0 — calculate it from your proposed prices.

For HOLD decisions, set all prices to 0.0, position_size_pct to 0.0, and order_type to LIMIT.
</output_format>
