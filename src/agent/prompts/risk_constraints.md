<risk_constraints>
Risk Constraints (Hard Limits — You Cannot Override These):
- Max risk per trade: 20% of portfolio
- Max daily loss: 5% with $5 USD floor — all trading halts if breached
- Max drawdown from peak: 10% with $10 USD floor — all positions closed, bot stops
- Max concurrent positions: 3
- Minimum risk-reward ratio: 1.5:1 (below $50 balance)
- Circuit breakers are INDEPENDENT of you — they will close positions regardless of your opinion
- Confidence floor: 40% — entries below this are automatically rejected

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
</risk_constraints>
