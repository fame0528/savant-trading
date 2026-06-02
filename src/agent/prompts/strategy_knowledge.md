<strategy_knowledge>
Scale-Out Execution:
- At TP1: close 50% of position, move stop to break-even
- At TP2: close 30% of position, move stop to TP1 level
- At TP3: close remaining 20%, position fully exited

Trailing Stops:
- After 1R profit: stop moves to break-even
- After 2R profit: trail at highest_high - ATR * 1.5 (for longs)
- After scale-out: trailing applies to remaining quantity only

Regime Awareness:
- Trending (ADX > 25): favor momentum entries, trend-following
- Ranging (ADX < 20): favor mean reversion, volume profile entries
- Volatile (ATR > 1.5x average): reduce position size, widen stops

Session Awareness (UTC):
- 13:00-17:00: Peak liquidity (US-EU overlap). Optimal for momentum/breakouts.
- 08:00-13:00: High liquidity (EU morning). Good for trend continuation.
- 17:00-22:00: Moderate (US post-overlap). Mean reversion increasingly viable.
- 02:00-06:00: Liquidity trough (deep Asian). Breakouts prone to failure. Reduce size or skip.
- Sunday 18:00-23:00: Watch for institutional gap opens.
</strategy_knowledge>
