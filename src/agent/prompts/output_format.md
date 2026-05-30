## Required Output Format

Respond with ONLY a JSON object — no markdown wrapping, no explanation before or after:

{
    "action": "BUY" | "SELL" | "HOLD" | "CLOSE" | "ADJUST_STOP",
    "pair": "BTC/USD",
    "side": "Long" | "Short",
    "entry_price": 0.0,
    "stop_loss": 0.0,
    "take_profit_1": 0.0,
    "take_profit_2": 0.0,
    "take_profit_3": 0.0,
    "position_size_pct": 0.0,
    "confidence": 0.0,
    "reasoning": "Your reasoning here — cite specific data points and knowledge sources",
    "knowledge_sources": ["source-id-001"],
    "risk_reward": 0.0
}

## Field Rules
- action: BUY to open long, SELL to open short, HOLD for no action, CLOSE to exit existing, ADJUST_STOP to modify stop
- pair: must match a configured trading pair
- side: Long for BUY, Short for SELL
- entry_price: exact entry price (must be near current market price)
- stop_loss: exact stop loss price (mandatory for BUY/SELL)
- take_profit_1/2/3: three take-profit levels (TP1 nearest, TP3 farthest)
- position_size_pct: percentage of portfolio to allocate (0-100)
- confidence: 0.0 to 1.0 — be honest, don't inflate
- reasoning: cite specific data, indicators, and knowledge sources
- knowledge_sources: list of knowledge unit IDs that informed your decision
- risk_reward: calculated R:R ratio for the trade

For HOLD decisions, set all prices to 0.0 and position_size_pct to 0.0.
