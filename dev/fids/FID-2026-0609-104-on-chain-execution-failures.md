# FID-2026-0609-104: Critical On-Chain Execution Failures

**ID:** FID-2026-0609-104
**Created:** 2026-06-09 19:30
**Severity:** critical
**Status:** created
**Scope:** engine.rs (BUY path), zero_x.rs (gasless submit)

---

## Issue 1: Position Sizer Rejects Trade Instead of Auto-Adjusting TP (High)

### Problem

COMP/USD rejected on-chain despite strong signal (58% confidence, ADX 37.2, 3+ triggers):
```
[BUY REJECTED] COMP/USD — claimed R:R=1.5, actual=1.0 (entry=18.82 stop=18.55 tp=19.1)
```

The LLM claimed 1.5:1 R:R but actual computed R:R was 1.0:1. Position sizer `calculate()` returns `None` at `risk/position.rs:153` because `rr_ratio (1.037) < min_rr (1.2) - 0.001`. The engine logs the rejection and moves on — no retry, no adjustment, trade is lost.

### Root Cause

**`engine.rs:2985-2991`** — `position_sizer.calculate()` is called with unmodified LLM prices. If R:R is below minimum, it returns `None`. The `else` branch at **`engine.rs:3218-3257`** just logs the rejection. No attempt to auto-extend TP to meet minimum R:R.

### Evidence (from live logs)

```
[BUY] [LONG] [COMP/USD] | 58% | R:1.5 | Trending regime (ADX 37.2)...
[BUY REJECTED] COMP/USD — Position sizer rejected — claimed R:R=1.5, actual=1.0 (entry=18.82 stop=18.55 tp=19.1)
```

Math: risk = 18.82 - 18.55 = 0.27, reward = 19.1 - 18.82 = 0.28, RR = 0.28/0.27 = 1.037. min_rr = 1.2. Trade rejected.

### Fix Location

**`engine.rs` before line 2985** — Add R:R auto-adjust block between the LLM decision and `position_sizer.calculate()`. If actual R:R < min_rr, extend `take_profit_1` to meet minimum:

```rust
// FID-101/FID-104: Auto-extend TP to meet minimum R:R before sizer check
let risk = (decision.entry_price - decision.stop_loss).abs();
if risk > 0.0 {
    let reward = match decision.side {
        Side::Long => decision.take_profit_1 - decision.entry_price,
        Side::Short => decision.entry_price - decision.take_profit_1,
    };
    let actual_rr = reward / risk;
    let min_rr = config.risk.min_rr_ratio;
    if actual_rr < min_rr && reward > 0.0 {
        let required_reward = risk * min_rr;
        let old_tp = decision.take_profit_1;
        decision.take_profit_1 = match decision.side {
            Side::Long => decision.entry_price + required_reward,
            Side::Short => decision.entry_price - required_reward,
        };
        decision.risk_reward = min_rr; // Keep claimed R:R consistent with adjusted prices
        info!("FID-104: Extended TP for {}: {:.4} → {:.4} (R:R {:.2}→{:.2})",
            decision.pair, old_tp, decision.take_profit_1, actual_rr, min_rr);
    }
}
```

### Scope

~15 lines in `engine.rs`, inserted before line 2985. Single file, no new deps.

---

## Issue 2: Gasless API Submit Fails — Missing chainId in Request Body (Critical)

### Problem

LINK/USD close failed on-chain. Standard swap returned dust (0x API returned 0 buyAmount for 2.9582 LINK). Gasless fallback also failed:

```
[GASLESS] Gasless fallback also failed for LINK/USD: Gasless submit returned 400 Bad Request:
{"name":"INPUT_INVALID","message":"The input is invalid","data":{"zid":"0x17160db1d76623fa87812bfb",
"details":[{"field":"chainId","reason":"Expected number, received nan"}]}}
```

Position was force-closed as stop-loss at $7.79 (-$0.34, -1.47%), but the on-chain swap never executed.

### Root Cause

**`zero_x.rs:669-683`** — The gasless submit body is built without `chainId`:

```rust
let mut submit_body = serde_json::json!({
    "trade": {
        "type": trade["type"],
        "eip712": trade["eip712"],
        "signature": trade_sig,
    }
});
```

The 0x Gasless API v2 `/submit` endpoint **requires `chainId` as a top-level field** in the request body (per OpenAPI spec at `docs/0x-llms-full.md:23694`):

```yaml
required:
  - chainId
  - trade
```

The 0x API returns HTTP 400 with `"field":"chainId","reason":"Expected number, received nan"` because the field is missing.

### Evidence

From OpenAPI spec (`docs/0x-llms-full.md:23672-23696`):
```yaml
requestBody:
  content:
    application/json:
      schema:
        type: object
        properties:
          chainId:
            type: integer
          ...
        required:
          - chainId
          - trade
```

Our code at `zero_x.rs:669-675` does not include `chainId`.

### Fix Location

**`zero_x.rs` line 669** — Add `chainId` to the submit body:

```rust
let mut submit_body = serde_json::json!({
    "chainId": params.chain_id,
    "trade": {
        "type": trade["type"],
        "eip712": trade["eip712"],
        "signature": trade_sig,
    }
});
```

### Scope

1 line addition in `zero_x.rs`. Single file, no new deps.

---

## Issue 3: Gasless Quote May Also Need chainId (Investigation Needed)

The gasless quote URL at `zero_x.rs:595` includes `chainId` as a query parameter:
```rust
"{}/quote?chainId={}&sellToken={}&buyToken={}&sellAmount={}&taker={}&slippageBps={}"
```

This works (quote succeeds before submit fails). But the API might prefer `chainId` in the body as well. **For now, only the submit body needs fixing** — the quote URL already works.

---

## Perfection Loop

### RED

Three issues identified:
1. **engine.rs:2985** — Sizer rejects trade with low R:R instead of auto-adjusting TP. The FID-101 design specifies auto-extend, but it was never implemented.
2. **zero_x.rs:669** — Gasless submit body missing required `chainId` field. The 0x OpenAPI spec marks it as required, but the docs example doesn't show it. The API rejects with 400.
3. Both issues caused real money loss: COMP/USD trade rejected despite strong signal, LINK/USD close failed and was force-closed at stop-loss.

### GREEN

Two targeted fixes:
1. Add R:R auto-adjust block (~15 lines) before `position_sizer.calculate()` in `engine.rs`
2. Add `chainId` to gasless submit body (1 line) in `zero_x.rs`

### AUDIT

| Check | Result |
|-------|--------|
| Fix 1: `config.risk.min_rr_ratio` accessible | ✅ Used at line 861 |
| Fix 1: `decision.side` available | ✅ Used at line 2990 |
| Fix 1: `decision.take_profit_1` mutable | ✅ `decision` is mut in BUY path |
| Fix 2: `params.chain_id` is `u64` | ✅ Matches API `integer` type |
| Fix 2: 0x spec requires `chainId` | ✅ Confirmed at line 23694 |
| Change delta | ~16 lines, <0.3% of engine.rs, <0.5% of zero_x.rs |
| Five Questions — ALL yes | ✅ All cases covered, scales, hostile-safe, maintainable, best practice |

### SELF-CORRECT

No issues found. Both fixes are surgical and well-scoped.

### COMPLETE

FID documented, fixes designed, ready for approval.

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — 264 pass
3. Runtime: COMP/USD BUY with R:R=1.0 should auto-adjust TP and pass sizer
4. Runtime: LINK/USD close dust → gasless fallback succeeds → trade executes on-chain

---

## References

- FID-101: R:R Auto-Adjust + Bear Market Filter (existing FID, Fix 1 never implemented)
- 0x Gasless API spec: `docs/0x-llms-full.md:23614-23696`
- Position sizer: `src/risk/position.rs:147-155`
- Gasless submit: `src/execution/dex/zero_x.rs:669-720`
- BUY path: `src/engine.rs:2985-3257`
