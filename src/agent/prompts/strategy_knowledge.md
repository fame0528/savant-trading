<strategy_knowledge>
Scale-Out Execution:
- At TP1: close 50% of position, move stop to break-even
- At TP2: close 30% of position, move stop to TP1 level
- At TP3: close remaining 20%, position fully exited

Trailing Stops:
- After 1R profit: stop moves to break-even
- After 2R profit: trail at highest_high - ATR * 1.5 (for longs)
- After scale-out: trailing applies to remaining quantity only

REGIME-SPECIFIC BEHAVIOR (Non-Negotiable):

Trending / Momentum (ADX > 25):
- Operational Bias: Trend-following. Let winners run while trailing risk.
- Entry Protocol: Require 3+ momentum triggers (EMA crosses, volume breakouts, MACD expansion, Wyckoff accumulation/distribution, squeeze setups).
- Management Protocol: Mandatory ATR-based trailing stops. ADJUST_STOP must be called at defined intervals of profit expansion. HOLD is permitted only if the stop has recently been optimized for the current ATR and the trend remains intact.
- New entries: Standard momentum triggers apply.

Ranging / Mean Reversion (ADX < 20):
- Operational Bias: High action frequency at defined boundaries; low tolerance for holding in the middle of the range.
- Entry Protocol: Momentum triggers are SUSPENDED. Support and resistance levels ARE your action triggers. Execute BUY at defined support bands and SELL at defined resistance bands. Do NOT wait for momentum confirmation — it leads to late entries in a range.
- Management Protocol: Aggressive profit-taking. Targets at range mid-point or opposite boundary. Stops placed tightly outside range extremes. HOLD is only permitted if price is actively oscillating in the middle 50% of the range AND moving toward the target.
- Dead capital: Positions with flat/negative PnL after 3+ cycles in ranging conditions must be CLOSED to free margin.

Transition (ADX crossing 20/25 threshold):
- When ADX crosses below 20 (trending→ranging): Existing momentum positions must be re-evaluated. Tighten stops aggressively or close if thesis depends on trend continuation.
- When ADX crosses above 20 (ranging→trending): Range positions must be re-evaluated. Switch to trend-following (trail stops, let winners run, add on pullbacks).

Volatile (ATR > 1.5x average):
- Reduce position size, widen stops slightly (but never beyond 2.5x ATR)
- Prefer LIMIT orders over MARKET to avoid slippage

Session Awareness (UTC):
- 13:00-17:00: Peak liquidity (US-EU overlap). Optimal for momentum/breakouts.
- 08:00-13:00: High liquidity (EU morning). Good for trend continuation.
- 17:00-22:00: Moderate (US post-overlap). Mean reversion increasingly viable.
- 02:00-06:00: Liquidity trough (deep Asian). Breakouts prone to failure. Reduce size or skip.

Confidence Discipline:
- Evaluate setup quality based on technicals, volume, and regime — NOT on existing position P&L.
- A position being +5% unrealized does not make the setup better. The setup is the setup.
- When evaluating for ADJUST_STOP, base confidence on current price action relative to levels, not on how much profit the position has.
- Do not inflate confidence because a position is winning. Do not deflate because it's losing.
- Management actions (ADJUST_STOP, CLOSE) are NOT gated by the 40% confidence floor.
</strategy_knowledge>
