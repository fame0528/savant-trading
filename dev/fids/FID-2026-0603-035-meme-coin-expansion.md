# FID: Meme coin expansion + safety rails for DEX trading

**Filename:** `FID-2026-0603-035-meme-coin-expansion.md`
**ID:** FID-2026-0603-035
**Severity:** critical
**Status:** analyzed
**Created:** 2026-06-03 17:15
**Author:** Agent

---

## Summary

The AI engine is correctly disciplined but starved of opportunities — all 8 current pairs are ranging with compressed ATR. Expanding to 13 pairs (adding PEPE, SHIB, FLOKI, TURBO, MOG) gives the AI more volatile setups. But meme coins require new safety rails: spread filtering, honeypot detection, correlation caps, and dual-timeframe analysis.

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.94, tokio
- **Commit:** `8249185` (pre-implementation)
- **Chain:** Arbitrum (chain_id 42161)
- **Capital:** $35.34 USDC
- **Source:** Gemini Deep Research report (68 pages, 69 citations)

## Detailed Description

### Problem

The AI has fired 4 Buy signals in 12 hours, but:
- 2 rejected by position sizer (R:R invalid)
- 1 failed on gas (fixed with buffer)
- 1 hung on 0x API (still debugging)

The root cause: all 8 pairs are ranging with ATR < 0.5%. The AI's 3+ trigger requirement can't be met in compressed markets. Meme coins have 2-10x higher ATR = more setups.

### Root Cause

Large-cap pairs in low-volatility environments have compressed ATR. The AI needs 3+ action triggers (EMA crossover, ADX>25, SOPR<1.0, MVRV<1.0, Fear<20) but these rarely align in ranging markets. Meme coins provide the volatility the AI needs.

### Evidence

```text
# Current state:
Pairs: 8 (BTC, ETH, SOL, XRP, DOGE, ADA, LINK, AVAX)
Buy signals in 12 hours: 4
Rejected by position sizer: 2 (R:R invalid)
Failed on gas: 1 (fixed with buffer)
Hung on 0x API: 1 (still debugging)
Successful swaps: 0

# Market conditions:
Fear & Greed: 11 (Extreme Fear)
MVRV: 1.25 (neutral)
SOPR: 0.9741 (capitulation)
ATR compression: all 8 pairs < 0.5%

# Existing code that can be reused:
circuit_breaker.max_spread_bps = 50 (needs lowering to 30)
config.ai.price_tolerance_pct = 10.0 (needs tightening to 0.5)
CorrelationMatrix exists in src/risk/correlation.rs
PaperTrader.spread_bps() exists in src/execution/paper.rs

# Gemini Research recommendation:
Expand to 13 pairs: +PEPE, SHIB, FLOKI, TURBO, MOG
Exclude from Arbitrum: WIF, BONK (Solana-native, thin liquidity)
Spread filter: 30bps max
Security: GoPlus API for honeypot/tax detection
```

### What Needs Building

#### 1. Pair Expansion (config/default.toml)

Add 5 meme pairs to the existing 8:

```toml
pairs = [
    # Core macro
    "BTC/USD", "ETH/USD", "SOL/USD", "XRP/USD",
    "DOGE/USD", "ADA/USD", "LINK/USD", "AVAX/USD",
    # Meme/high-volatility
    "PEPE/USD", "SHIB/USD", "FLOKI/USD", "TURBO/USD", "MOG/USD",
]
```

**Exclude from Arbitrum:** WIF, BONK — native Solana tokens, bridged liquidity too thin. Reserve for Solana fallback (FID-036).

**Token database:** PEPE already has Arbitrum address. SHIB, FLOKI, TURBO, MOG use enterprise token resolution (symbol-based 0x API lookup).

#### 2. Spread Filter (NEW — src/risk/spread_filter.rs)

The circuit breaker already has `max_spread_bps = 50.0` (src/risk/circuit_breaker.rs:24). Need a pre-execution spread check that rejects individual trades:

```
Spread > 30bps → reject trade (too expensive for $11 positions)
Spread 15-30bps → warn but allow
Spread < 15bps → ideal
```

**Math:** At $11 positions, 30bps spread = $0.033 friction. Round-trip gas = $0.06. Total friction = $0.093. With 3% stop distance ($0.33), friction consumes 28% of stop margin. Above 30bps, friction exceeds 40% — unacceptable.

**Implementation:** Query 0x API quote, calculate `spread = (buyAmount - sellAmount) / sellAmount`. If > 0.003, reject. **Reuse existing:** `circuit_breaker.max_spread_bps` — lower from 50 to 30 for meme coins.

#### 3. GoPlus Security API (NEW — src/security/goplus.rs)

Meme coins can be honeypots (buy but can't sell) or have hidden taxes. GoPlus API detects these:

```
GET https://api.gopluslabs.io/api/v1/token_security/42161?contract_addresses={address}
```

**Hard reject if:**
- `is_honeypot` = "1"
- `buy_tax` > 0.01 (1%)
- `sell_tax` > 0.01 (1%)
- `transfer_pausable` = "1"
- `cannot_sell_all` = "1"

**Implementation:** Cache results (tokens don't change contracts). Check before LLM evaluation. If security check fails, skip pair entirely.

#### 4. Dual Timeframe (src/engine.rs, src/agent/context_builder.rs)

Current: 5m candles only. Meme coins need 15m for structure + 5m for execution.

**Implementation:**
- Fetch 15m candles in addition to 5m
- Inject 15m trend context into LLM prompt (ADX, EMA direction)
- AI only authorizes 5m EMA crossover if 15m trend agrees
- Reject 1m timeframe entirely (too much HFT noise)

#### 5. Correlation Cap (src/risk/correlation.rs)

Meme coins are highly correlated. Opening PEPE + SHIB + MOG = one bet, not three. **Existing:** `CorrelationMatrix` already tracks pairwise Pearson correlation (src/risk/correlation.rs). Circuit breaker already uses it via `check_with_heat()`.

**Implementation:**
- Define risk buckets: A (Macro L1/L2), B (Legacy/Utility), C (High-Beta Meme)
- Max 1 open position from Bucket C at any time
- If AI detects setups in multiple meme coins, rank by ATR/ADX and pick best one
- **Reuse existing:** `CorrelationMatrix` — add bucket categorization and cap enforcement

#### 6. KV Cache Optimization (src/agent/provider.rs)

The LLM prompt is 90% identical across pairs (system prompt, trading rules, risk params). Only the candle data and indicators change.

**Implementation:**
- Bundle static content (rules, risk, knowledge) into prefix
- Append dynamic content (candles, indicators) per pair
- OpenRouter/mimo-v2.5-pro supports prefix caching
- Expected: 40-60% reduction in TTFT (time-to-first-token)

#### 7. 0x API Retry Queue (src/execution/dex/trader.rs)

The 0x API intermittently hangs. Current: 15s timeout + 3 retries. But if ALL retries fail, the trade is lost.

**Implementation:**
- Failed swaps go into a retry queue
- Next cycle re-evaluates: if AI still says Buy, retry with fresh quote
- Queue expires after 3 cycles (stale quotes)

#### 8. Price Tolerance Check (src/engine.rs)

The AI's entry price might be stale by the time the swap executes. Config already has `price_tolerance_pct = 10.0` — tighten to 0.5% for meme coins.

**Implementation:**
- After AI returns Buy decision, check current price vs entry_price
- If drift > 0.5%, log warning and skip
- Prevents buying into pumps that happened during LLM evaluation
- **Reuse existing:** `config.ai.price_tolerance_pct` — just tighten the value

#### 9. Emergency Liquidation (src/execution/dex/trader.rs)

If the engine crashes while holding a position, there's no way to exit.

**Implementation:**
- `--liquidate` CLI flag
- Reads dex_state.json, finds open positions
- Calls `close_position()` for each
- Exits cleanly

#### 10. ATR-Based Position Sizing (src/risk/position.rs)

Current: fixed 1% risk per trade. Meme coins with 5% ATR need different sizing than BTC with 1% ATR.

**Implementation:**
- `risk_amount = min(1% of equity, ATR * quantity * 0.5)`
- Prevents over-sizing on high-volatility assets
- Preserves the 3% stop margin

## Impact Assessment

### Affected Components

- `config/default.toml` — New pairs, spread filter config
- `src/risk/` — Spread filter, correlation cap, ATR sizing
- `src/security/` — NEW: GoPlus API integration
- `src/execution/dex/trader.rs` — Retry queue, price tolerance
- `src/agent/context_builder.rs` — Dual timeframe context
- `src/agent/provider.rs` — KV cache optimization
- `src/engine.rs` — Price tolerance check, retry queue drain

### Risk Level

- [x] Critical: Engine can't trade without more volatile pairs
- [ ] High: Major feature broken, no workaround
- [ ] Medium: Feature degraded, workaround exists
- [ ] Low: Minor issue, cosmetic, or edge case

## Proposed Solution

### Approach

Implement in 3 phases: (1) immediate pair expansion + spread filter, (2) safety rails (GoPlus, correlation, ATR sizing), (3) optimization (dual timeframe, KV cache, retry queue). Each phase tested independently.

### Steps

**Phase 1: Immediate**
1. Add 5 meme pairs to `config/default.toml`
2. Add spread filter check in `execute_swap()` — reject if > 30bps
3. Add price tolerance check in engine.rs — reject if drift > 0.5%
4. Build, test, restart

**Phase 2: Safety Rails**
5. Create `src/security/goplus.rs` — GoPlus API client
6. Add honeypot check before LLM evaluation
7. Add correlation cap in `src/risk/correlation.rs` — max 1 meme position
8. Add ATR-based position sizing in `src/risk/position.rs`

**Phase 3: Optimization**
9. Add dual timeframe fetch in `src/engine.rs` — 15m + 5m
10. Inject 15m context into LLM prompt
11. Add KV cache prefix optimization in `src/agent/provider.rs`
12. Add retry queue for failed swaps in `src/execution/dex/trader.rs`
13. Add `--liquidate` CLI flag for emergency exit

### Verification

- `cargo build --release` — zero errors
- `cargo test` — 187+ tests pass
- `cargo clippy` — zero warnings
- Manual test: restart engine with 13 pairs, verify AI evaluates all

## Perfection Loop

### Loop 1

- **RED:** Engine starved of setups — 8 pairs ranging, ATR < 0.5%. No spread filter, no honeypot detection, no correlation cap. Gemini Deep Research identified 5 meme coins with Arbitrum liquidity (PEPE, SHIB, FLOKI, TURBO, MOG). WIF/BONK excluded (Solana-native, thin Arbitrum liquidity).
- **GREEN:** (pending — implementation not started)
- **AUDIT:** FID verified against template. All sections present: Summary, Environment, Detailed Description, Problem, Root Cause, Evidence, Impact Assessment, Proposed Solution, Approach, Steps, Verification, Perfection Loop, Resolution, Lessons Learned. Config already has `price_tolerance_pct = 10.0` — needs tightening to 0.5% for meme coins. Circuit breaker already has `max_spread_bps = 50.0` — needs lowering to 30bps for meme coins.
- **CHANGE DELTA:** 0% (FID creation only, no code changes yet)

## Resolution

- **Fixed By:** (pending)
- **Fixed Date:** (pending)
- **Fix Description:** (pending)
- **Tests Added:** (pending)
- **Verified By:** (pending)
- **Commit/PR:** (pending)

## Lessons Learned

- 8 pairs in a ranging market = no trades. Need 13+ for coverage.
- Meme coins have 2-10x higher ATR = more setups for the AI.
- Spread filter is critical — 30bps max for $11 positions.
- GoPlus API prevents honeypot/rug pull exposure.
- Dual timeframe (15m + 5m) filters HFT noise on meme coins.
- Correlation cap prevents stacking correlated meme positions.
- KV cache optimization cuts LLM latency 40-60%.
- WIF/BONK are Solana-native — bridged Arbitrum liquidity too thin for 30bps spread filter.
- Existing code (CorrelationMatrix, spread_bps, price_tolerance_pct) can be reused — don't duplicate.
- Price tolerance check prevents buying into pumps that happened during LLM evaluation (20-60s window).
- Retry queue prevents losing trades when 0x API intermittently fails.
- Emergency liquidation CLI is critical safety net for DEX positions (no exchange kill switch).
