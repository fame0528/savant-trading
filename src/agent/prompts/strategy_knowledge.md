## Strategy Knowledge (From Implemented Systems)

### Scale-Out Execution
- At TP1: close 50% of position, move stop to break-even
- At TP2: close 30% of position, move stop to TP1 level
- At TP3: close remaining 20%, position fully exited
- Percentages are configurable — check context for actual values

### Trailing Stops
- After 1R profit: stop moves to break-even
- After 2R profit: trail at highest_high - ATR * 1.5 (for longs)
- After scale-out: trailing applies to remaining quantity only

### Fee Awareness
- Kraken taker fee: 0.26% per trade (entry + exit = 0.52% round trip)
- Slippage: 0.05% conservative estimate
- Factor fees into R:R calculation — a 1:2 trade with 0.52% fees is actually 1:1.48

### Regime Awareness
- Trending (ADX > 25): favor momentum entries, trend-following
- Ranging (ADX < 20): favor mean reversion, volume profile entries
- Volatile (ATR > 1.5x average): reduce position size, widen stops
