<strategy_knowledge>
Scalping Execution Protocol:

Entry Requirements:
- 3+ Action Triggers aligned for direction
- Stop loss defined BEFORE entry (0.5% or structure-based)
- Target defined BEFORE entry (0.8-1.2% based on ATR)
- Spread < 0.25% on the pair
- Session is active (08:00-17:00 UTC preferred)

Exit Protocol:
- Single target: close 100% at 0.8-1.2%
- Breakeven trigger: move stop to breakeven at +0.4%
- Time stop: close if no 0.3% move in 5 minutes
- Hard stop: close at -0.5%

ZERO-BASE PORTFOLIO REVIEW (FID-092 — Mandatory for All Position Evaluations):

The most effective technique for eliminating sunk cost bias. When evaluating an existing position, you MUST ask:

"If I held $0 of this asset and had its full value in cash today, would I initiate a new position at the current price with the current technicals?"

- If YES: HOLD (the setup is still valid regardless of entry price)
- If NO: CLOSE immediately (the market is telling you this is a bad allocation)

The historical entry price is economically irrelevant. The market does not know or care what you paid.

REGIME-SPECIFIC BEHAVIOR (Non-Negotiable):

Trending / Momentum (ADX > 25):
- Entry: Require 3+ momentum triggers (EMA crosses, volume breakouts, MACD expansion).
- Management: Tight stops (0.3-0.5%), single target, quick exits.
- New entries: Standard momentum triggers apply.
- ADVERSE TREND RULE: If ADX > 25 AND position is underwater AND EMA is against direction → CLOSE immediately.

Ranging / Mean Reversion (ADX < 20):
- Entry: Buy at support, sell at resistance. No momentum confirmation needed.
- Management: Tighter stops (0.3%), smaller targets (0.6-0.8%).
- Dead capital: Positions with flat/negative PnL after 5+ minutes → CLOSE.

Transition (ADX crossing 20/25 threshold):
- Existing positions must be re-evaluated. Tighten stops or close if thesis depends on trend continuation.

Volatile (ATR > 1.5x average):
- Reduce position size, widen stops slightly (but never beyond 0.8%)
- Prefer LIMIT orders over MARKET to avoid slippage

Session Awareness (UTC):
- 13:00-17:00: Peak liquidity (US-EU overlap). Optimal for scalps.
- 08:00-13:00: High liquidity (EU morning). Good for scalps.
- 02:00-06:00: Liquidity trough (deep Asian). Scalps prone to failure. Reduce size or skip.

Confidence Discipline:
- Evaluate setup quality based on technicals, volume, and regime — NOT on existing position P&L.
- Management actions (ADJUST_STOP, CLOSE) are NOT gated by the 40% confidence floor.
</strategy_knowledge>
