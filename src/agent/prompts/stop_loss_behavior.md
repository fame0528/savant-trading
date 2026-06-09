# Stop-Loss Consistency and Mandatory Management

## Primary Rule

Treat any position with `stop_loss == 0.0` as **invalid and unguarded** until proven otherwise.

## Fallback Hierarchy (What the engine does when a stop is missing)

1. **DB restore** (`engine.rs:420-430`): if `stop_loss <= 0.0`, auto-set `entry_price * 0.95` (5% default) and persist the fix.
2. **Fresh DEX fill** (`execution/dex/trader.rs:1111`): position is created with `stop_loss: 0.0` intentionally. The engine/outer loop must patch this before the next candle tick.
3. **Wallet recovery** (`engine.rs:927`): if no real stop is known, hardcode `entry_price * 0.85` (15%) with TP at `+10/20/30%`. This is a recovery-only path for orphaned on-chain positions.

## MANDATORY STOP MANAGEMENT PROTOCOL

### Rule 1: The Absurdity Check (Non-Negotiable)

During every evaluation cycle, you MUST calculate the distance of the existing Stop Loss (SL) as a multiple of the current 14-period ATR.

**Formula:** `stop_distance_atr = |entry_price - stop_loss| / current_atr`

- If `stop_distance_atr > 2.5`: The SL is **Structurally Invalid**. You MUST immediately output the ADJUST_STOP action to move the SL to a technically sound level (recent swing low/high or 1.5x ATR from current price).
- If `stop_distance_atr > 2.0`: The SL is **Too Wide**. You SHOULD output ADJUST_STOP unless a specific technical level justifies the distance.

### Rule 2: No Legacy Deference (Absolute Authorization)

If you identify a stop as a "legacy error," "survival stop," "absurdly wide," or "wallet recovery default," you have **absolute and explicit authorization** to fix it immediately.

Returning HOLD on an invalid stop is a **catastrophic failure** of your directive. You do NOT need permission. You do NOT need explicit instruction. The ADJUST_STOP tool exists for exactly this purpose.

### Rule 3: The Trailing Ratchet (Profit Protection)

- If a position achieves profit ≥ 1R (where R = |entry - original_stop|), you MUST execute ADJUST_STOP to move the stop to break-even plus fees.
- You are forbidden from allowing a 1R winner to turn into a loss.
- After 2R profit: trail at highest_high - ATR * 1.5 (for longs) or lowest_low + ATR * 1.5 (for shorts).

### Rule 4: Quantized Adjustments (Anti-Overtreading)

Do not execute ADJUST_STOP for micro-movements. The new stop must improve the risk profile by at least 0.5R to justify the execution cost.

**Minimum improvement formula:** `|current_stop - new_stop| >= 0.5 * |entry - current_stop|`

If the improvement is less than 0.5R, HOLD — the gas cost of the adjustment exceeds the risk reduction.

## What This Means For You

- When you submit a **Buy/Sell/Short** decision with `stop_loss: 0.0`, the system will ingest it but the position is naked until a later tick fixes it.
- When you submit an **AdjustStop** action, `stop_loss: 0.0` is rejected by the decision parser (`decision_parser.rs:152`).
- A `Pass` decision is the right call ONLY if all management triggers evaluate to false AND no new entry triggers are met.

## Auditing Signal

If you see a position survive more than one lifecycle tick with `stop_loss == 0.0`, treat it as a bug trace: either the decision payload was malformed, or the engine failed to patch it after fill.
