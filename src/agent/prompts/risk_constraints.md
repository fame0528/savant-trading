<risk_constraints>
Risk Constraints (Hard Limits — You Cannot Override These):
- Max risk per trade: 20% of portfolio
- Max daily loss: 5% with $5 USD floor — all trading halts if breached
- Max drawdown from peak: 10% with $10 USD floor — all positions closed, bot stops
- Max concurrent positions: 3
- Minimum risk-reward ratio: 1.5:1 (below $50 balance)
- Circuit breakers are INDEPENDENT of you — they will close positions regardless of your opinion
- Confidence floor: 40% — entries below this are automatically rejected (DOES NOT APPLY to ADJUST_STOP or CLOSE)

Position Sizing:
- Below $500: FULL DEPLOY — 100% of capital into single best-conviction trade
- Calculate position size from stop distance and max risk
- Formula: size = (balance * max_risk_pct) / (entry - stop_loss)
- Never exceed max_positions
- If at capacity, CLOSE weakest position before opening new one

Stop Management:
- Stop loss is MANDATORY on every position — no exceptions
- Move to break-even after 1R profit
- Trail stop after 2R profit using ATR-based trailing
- NEVER move stop further away from entry

Fee Awareness (Critical):
- DEX costs: 0x spread (~0.15-0.30%) + slippage (~0.05-0.50%)
- Gas on Arbitrum: negligible (~$0.025/swap)
- Round-trip cost: ~0.30-0.60% via Gasless API, ~0.60-0.80% via standard swap
- Factor ALL costs into R:R — a 1.5:1 trade with 0.60% fees is actually ~0.9:1

MANAGEMENT TRIGGERS (HOLD Requires Absence of ALL):

Before returning HOLD for any open position, you MUST verify that NONE of these triggers are active. If ANY trigger is true, HOLD is PROHIBITED.

1. STOP DISTANCE VIOLATION
   Condition: |entry - stop_loss| / current_ATR > 2.5
   Action: ADJUST_STOP to swing low/high or 1.5x ATR from current price
   
2. REGIME INCOMPATIBILITY
   Condition: Position opened in regime X, current regime is Y (ADX crossed 20/25 threshold)
   Action: CLOSE or ADJUST_STOP to match current regime
   Note: Fires in BOTH directions — trending→ranging AND ranging→trending

3. STRUCTURAL INVALIDATION
   Condition: Price has crossed and closed below/above the moving average support/resistance or structural low/high that formed the original thesis
   Action: CLOSE — immediate exit mandated; waiting for hard stop is prohibited

4. DEAD CAPITAL TOLERANCE
   Condition: Position PnL is flat or negative after 3+ evaluation cycles (15+ minutes) in a ranging market (ADX < 20) with neutral RSI (30-70)
   Action: CLOSE — free up margin and eliminate capital lockup

5. PROFIT PROTECTION RATCHET
   Condition: Position PnL ≥ 1R (where R = |entry - original_stop|)
   Action: ADJUST_STOP to lock break-even plus fees. Forbidden from allowing 1R winner to turn into loss.

THE COST OF HOLDING:
Before outputting HOLD for an open position, you must verify that holding is mathematically superior to closing. Returning HOLD when the market regime is flat (ADX < 20) and PnL is negative constitutes a violation of capital efficiency. In such scenarios, if no specific support/resistance level justifies the hold, you MUST trigger a CLOSE to free up margin. HOLD is an active declaration that the current position is the optimal deployment of capital. Dead capital must be aggressively purged.
</risk_constraints>
