# SPEC-2026-0616-001: Strategy/Universe Mismatch — The Engine Is Right, the Pairs Are Wrong

**Filename:** `SPEC-2026-0616-001-strategy-universe-mismatch.md`
**ID:** SPEC-2026-0616-001
**Status:** draft (Spencer's call required before implementation)
**Created:** 2026-06-16 18:00 EST
**Author:** Vera
**Type:** Strategy/universe spec (read-only, no code)

---

## TL;DR

The trading engine is correctly returning 0 trades across 30+ pairs over 2 hours. The strategy (`soul.md`) targets scalping on liquid, high-volume, high-volatility pairs. The engine is pointed at illiquid DEX micro-caps on Arbitrum where vol=0, RSI is at extremes, and the 0.8% scalp targets are mathematically impossible. This is not a bug. This is a strategy/universe mismatch. **Two paths forward; each is a separate FID:**

- **Path A (recommended):** Switch from `test-anvil.toml` (Arbitrum-only via Anvil fork) to `config/default.toml` to enable 0x multi-chain. The same engine code, but with a universe of liquid majors (ETH, WBTC, USDC, ARB, OP, etc.) across Ethereum, Base, Optimism, Arbitrum. Pair selection criteria already match the strategy. **Estimated 2-3 hours.** (FID-167, Workstream 2.)
- **Path B (alternative):** Retune strategy for illiquid DEX micro-caps. Accept 2-5% targets and 1-2% stops. Change the soul, the conviction thresholds, the position sizing, and the circuit breakers. **Estimated 1-2 days.** (Separate FID, not in v0.14.2.)

**Path A is the right answer for now.** It uses the existing engine, the existing strategy, and the existing 0x integration — it just unlocks the chain coverage that's already coded but not configured.

---

## Evidence

From the FID-163 fidelity evidence (verified 2026-06-15, post-fix) and the engine run 2026-06-15 23:50 – 2026-06-16 01:25 (cycles 11-17):

```
Cycle 17 at 1:25 AM (the last cycle before Spencer closed the session):
  FUN/USD: Ranging ADX 13.4, RSI 51.5, EMA_F<EMA_S bearish, vol 1.32
  GUN/USD: Trending ADX 37.2, RSI 18.5 deeply oversold, EMA_F<EMA_S bearish, vol 0 dead
  SEI/USD: Ranging ADX 19.7, RSI 59.2, EMA_F<EMA_S bearish, vol 0.65
  HFT/USD: Trending ADX 34.7, RSI 64.7, EMA_F>EMA_S slightly bullish, vol 0 dead
  GUN/USD: Trending ADX 37.2, RSI 18.5 deeply oversold, vol 0 dead
  PUFFER/USD: Trending ADX 29.9, EMA_F>EMA_S bullish, RSI 29.3, vol 0.37 dead
  PUMP/USD: Trending ADX 20.5, RSI 59.5, EMA_F<EMA_S bearish, vol 0.67
  PYTH/USD: Trending ADX 34.4, RSI 66.2 overbought, EMA_F<EMA_S slightly bearish, vol 0 dead
  RAIN/USD: Trending ADX 29.2, vol 0 dead, EMA_F>EMA_S bullish, RSI 59.1
```

Every pair shows the same pattern: **vol = 0 (or near-zero), price barely moving, RSI at extremes.** The 0.8% scalp target is mathematically impossible when volume is zero. The LLM is correctly returning Pass on every pair.

### Comparison to the strategy

`src/agent/soul.md` lines 134-141:
> **Pair selection criteria:**
> 1. **ATR > 1% daily** — enough movement for scalps
> 2. **Volume > $10M daily** — enough liquidity for clean fills
> 3. **Spread < 0.25%** — tight enough for scalps to be profitable
> 4. **Available on 0x Arbitrum** — must be swappable

**Reality:** None of the 36 active pairs in `test-anvil.toml` have $10M daily volume. Most have $0-100K. The pair discovery at engine startup (`src/engine/mod.rs:166-180`) uses `min_volume_24h_usd: 1_500_000.0` — but that's a $1.5M floor on Kraken-discovered pairs. The Anvil-forked Arbitrum pairs are a static list of 201 tokens from `data/tokens.json`, most of which are illiquid dead micro-caps.

**Result:** The strategy is right. The universe is wrong. The LLM is correctly failing every pair's scalp criteria.

---

## Path A: Multi-Chain (Recommended)

### What it is

Switch the engine's startup config from `test-anvil.toml` (Arbitrum-only via Anvil local fork) to `config/default.toml` (multi-chain via 0x API on mainnet). The 0x V2 API supports 20+ chains including Ethereum, Arbitrum, Base, Optimism, BSC, Polygon. The engine code already has multi-chain support (per `src/core/config.rs` ChainEntry, `src/engine/mod.rs` chain selection). The configuration is the only thing blocking it.

### Why it works

- **0x API provides liquidity for majors.** ETH/USDC, WBTC/USDC, ARB/USDC, OP/USDC, etc. all have $10M+ daily volume. They meet the soul's pair selection criteria.
- **Same engine, same strategy, same soul.** No code change to the strategy or the LLM prompt. The LLM will see real data and start making real decisions.
- **Multi-chain coverage is already coded.** `config/default.toml` has 5 chains declared (Ethereum, Arbitrum, Base, Optimism, Polygon per the ChainEntry struct). The 0x Settler contract is deployed on each.
- **Anvil is not required for mainnet.** The engine doesn't need a local fork when trading on real chains. Simpler operation.

### What's required

1. **Verify `config/default.toml` is fully wired.** (FID-167, Workstream 2)
2. **Update `start.bat` to default to `config/default.toml`.** One-line change. Or pass `--config config/default.toml` to override.
3. **Verify 0x Settler + Permit2 are deployed on each configured chain.** Per `research/repos/0xProject/0x-settler/chain_config.json` (already verified for Ethereum, Arbitrum, Base, Optimism, Polygon in earlier sessions).
4. **Test on a single chain first (Ethereum mainnet) before enabling all 5.** This isolates failures.
5. **Set `live_execution` to `false` for paper-mode validation.** Already false. Engine is OFF.

### Risks

- **Real money at risk.** Switching to mainnet means real trades. Spencer has $0 USDC currently. The engine can't actually trade until capital is restored. Paper mode is the validation path.
- **0x v2 API rate limits.** $0.01 per request at 0x's pricing. With 36 pairs × 5 cycles/hour = 180 requests/hour = $1.80/hour. The M3 proxy uses the free model (M3), so LLM cost is $0. Total cost: $0 LLM + $1.80 0x = $1.80/hour while running.
- **Strategy may not be profitable even on liquid majors.** The 0.8% scalp target with 0.5% stop is tight. May need backtesting before live money.

### Estimated scope

- FID-167 (Workstream 2): 2-3 hours. `cargo test` + `cargo build --release` + engine restart on testnet.
- 0x Settler + Permit2 verification: 1 hour. (Already done for Arbitrum. Just confirm for Ethereum, Base, Optimism.)
- Total: half a day.

### Open question for Spencer

**Do we want to enable ALL 5 chains, or just Ethereum mainnet first?** More chains = more pairs = more potential scalps = more API cost = more LLM latency. I recommend starting with Ethereum mainnet, validating for 24h, then expanding to Base + Optimism + Arbitrum + Polygon one chain at a time.

---

## Path B: Retune for Illiquid Micro-Caps

### What it is

Change the strategy to accept illiquid DEX micro-caps as a valid universe. This means rewriting parts of the soul and adjusting thresholds.

### Why it might be wanted

- **The current test-anvil config is the working development setup.** Switching to mainnet is a bigger change.
- **Micro-caps are where the alpha is for small accounts.** Liquid majors have tight spreads but require larger positions to make meaningful returns. Micro-caps can move 5-10% in hours with $1K of capital.
- **The engine + LLM are tuned for fast-moving price action.** Micro-caps provide that; liquid majors are slower.

### What's required

1. **Soul rewrite.** Targets become 2-5% (instead of 0.8-1.2%), stops become 1-2% (instead of 0.5%), hold time extends to 30-60 minutes (instead of 5-15).
2. **Position sizing change.** $25 in a $50K-volume micro-cap fills the order book. Need to size to <1% of daily volume.
3. **Conviction threshold change.** Currently 0.30 (Trending) / 0.40 (others). Micro-caps have noisy indicators — need to raise to 0.45-0.50 to filter false signals.
4. **Circuit breaker change.** 5% daily loss at $25 is $1.25 — too tight. Micro-cap volatility means larger swings. Adjust to 10-15% daily loss.
5. **Backtest on the current 36 pairs.** Run 1000+ scenarios against historical data to validate the new thresholds.
6. **Risk: this is what killed the $40 in the 2026-06-13 incident.** The engine drained $40 trading micro-caps. The same strategy, the same data integrity, the same soul violation risk. **Lessons 001 and 008 are directly relevant here.** A 2-5% target means a 1-2% stop is a 1:2 R:R. At 60% win rate, expected value is positive. At 50% win rate, it's negative. The bot needs to be honest about PnL (invariant #5) and not fabricate.

### Risks

- **This is the universe that drained $40 last time.** The 2026-06-13 incident was a micro-cap on Arbitrum. The structural fix (FID-147, 148, 149, 150, 151, 152, 161, 162, 163) addressed the data integrity. It did NOT validate the strategy. Path B is essentially saying "go back to the universe that failed, with better tools."
- **Micro-cap liquidity is unreliable.** 0x quotes can be 5-10% off on thin liquidity. Spread filter at 0.25% (soul) is wrong; should be 1-2%.
- **Slippage on close can be huge.** If the entry fills at $X but the close fills at $X-2% (slippage), a 0.8% scalp becomes a 1.2% loss. Path B needs a 2-3% slippage budget per side.

### Estimated scope

- Soul rewrite + 6 conviction/risk threshold changes: 4-6 hours
- Backtest: 2-3 hours (use the existing sandbox, ~30 scenarios)
- Engine re-tuning for slippage/size: 2-4 hours
- Total: 1-2 days

### Open question for Spencer

**Is the goal "make the engine trade" or "make the engine trade profitably"?** Path A unlocks trading on a universe that meets the existing strategy. Path B rewrites the strategy to accept a more dangerous universe. The first question is whether the existing strategy is even profitable on liquid majors — that's a backtest, not a code change.

---

## Recommendation

**Path A.** Here's why:

1. **Lower risk.** Same engine, same strategy, same soul. The only change is the chain coverage. Easier to validate. Easier to roll back.
2. **Lower cost.** ~$0 LLM + $1.80 0x/hour. Path B doesn't change this.
3. **Faster to validate.** Half a day vs 1-2 days. Spencer can see results in one session.
4. **Closer to production-ready.** Liquid majors + tight spreads + real volume = real fills. Micro-caps = phantom fills + surprise slippage.
5. **Doesn't preclude Path B later.** If Path A doesn't show promising backtests, we can always retune for micro-caps. Path B is irreversible in terms of time investment.
6. **Aligns with Spencer's goal of "production ready and running properly."** Path B is a research project.

**Concrete proposal:** FID-167 (Workstream 2) implements Path A. After validation, run a backtest against 4-6 weeks of liquid-major historical data. If the strategy is profitable on majors, ship v0.14.2 with multi-chain. If not, file FID-168 for strategy retune (Path B).

---

## Implementation

This SPEC is read-only. No code change. FID-167 (Workstream 2) is the implementation ticket for Path A. It will:

1. Read `config/default.toml` and verify it loads without errors.
2. Read the multi-chain code paths in `src/engine/mod.rs` and `src/execution/`.
3. Wire chain selection: which chains are enabled, how pairs are selected per chain.
4. Update `start.bat` default config to `config/default.toml` (or accept `--config` arg).
5. Add FID-168 to the queue for backtest against historical data (separate session).

---

## Open questions for Spencer (please respond before FID-167 implementation begins)

1. **Which chains to enable first?** Ethereum mainnet only? Or Ethereum + Base? Or all 5?
2. **Live trading or paper mode for v0.14.2?** Spencer has $0 USDC. Engine is OFF. Path A is structural; live trading is a separate decision.
3. **Backtest before v0.14.2 or after?** I recommend before. ~2 hours of work to run 1000+ historical scenarios against the multi-chain pair set.
4. **Do you want a separate FID-168 for the backtest, or should it be part of FID-167?**

---

*SPEC-2026-0616-001 — Vera — read-only strategy/universe spec, recommendation: Path A (multi-chain), pending Spencer's call on chains/live/backtest scope*
