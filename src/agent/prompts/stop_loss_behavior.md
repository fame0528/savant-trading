# Stop-Loss Consistency and Fallback Behavior

## Primary Rule

Treat any position with `stop_loss == 0.0` as **invalid and unguarded** until proven otherwise.

## Fallback Hierarchy (What the engine does when a stop is missing)

1. **DB restore** (`engine.rs:420-430`): if `stop_loss <= 0.0`, auto-set `entry_price * 0.95` (5% default) and persist the fix.
2. **Fresh DEX fill** (`execution/dex/trader.rs:1111`): position is created with `stop_loss: 0.0` intentionally. The engine/outer loop must patch this before the next candle tick.
3. **Wallet recovery** (`engine.rs:927`): if no real stop is known, hardcode `entry_price * 0.85` (15%) with TP at `+10/20/30%`. This is a recovery-only path for orphaned on-chain positions.

## What This Means For You

- When you submit a **Buy/Sell/Short** decision with `stop_loss: 0.0`, the system will ingest it but the position is naked until a later tick fixes it.
- When you submit an **AdjustStop** action, `stop_loss: 0.0` is rejected by the decision parser (`decision_parser.rs:152`).
- A `Pass` decision is the right call if you cannot compute a valid stop — do not invent a zero stop to satisfy the schema.

## Preferred Behavior

1. Always compute a real stop (ATR, structure, or fixed percent) before emitting a BUY/SELL/Short decision.
2. If you cannot derive a stop from current data, return `Pass` and explain why in `reasoning`.
3. Do not rely on the engine fallback as your primary risk management — it is a safety net, not a substitute for proper analysis.

## Auditing Signal

If you see a position survive more than one lifecycle tick with `stop_loss == 0.0`, treat it as a bug trace: either the decision payload was malformed, or the engine failed to patch it after fill.
