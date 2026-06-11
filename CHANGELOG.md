# Changelog

All notable changes to Savant Trading will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
n## [0.13.3] — 2026-06-11

### Fixed

- **FID-117 Timing Fix:** Fixed brace mismatch in engine — FID-117 starting equity snapshot was inside `if sync_balance()` where `journal` didn't exist yet. Added missing closing braces for sync_balance and executor blocks. Moved recording code to correct location (after journal init, before wallet recovery). Fixes dashboard doubling portfolio balance ($42.35 instead of $22.72).

### Changed

- **Tailwind v4 Canonical Classes:** Updated 117 lines in dashboard to v4 syntax (`text-[var(--dim)]` → `text-(--dim)`, `bg-gradient-to-r` → `bg-linear-to-r`, etc.)
- **FID Lifecycle Cleanup:** Archived 6 completed FIDs (111, 112, 114, 115, 116, 117) to `dev/fids/archive/`. Created `dev/fids/MASTER-FID.md` tracking file. 3 active FIDs remain.
- **Stale starting_equity deleted** from SQLite journal so engine recalculates correctly on next boot.

## [0.13.2] — 2026-06-10

### Fixed — FID-111: Position-Pair Injection (Held Positions Invisible to LLM)

Positions loaded from journal or wallet-recovery can reference pairs not in the discovery list. These pairs never get evaluated by the LLM and are missing from AI Decisions. After position loading, any pair with an open position not in active_pairs is added to active_pairs + curated_pairs, gets a MarketDataStore, and has historical candles fetched at startup.

### Fixed — FID-112: Final Side Correction (SHORT Positions Surviving Into Portfolio)

The executor-to-portfolio sync path bypasses the wallet-sync side correction, reintroducing SHORT positions into the portfolio and dashboard. Added FINAL SIDE CORRECTION block before shared state sync that forces all remaining SHORT positions to LONG with corrected TP/SL, re-registers in DexTrader, saves to journal, and logs activity.

### Changed — FID-117: Journal as Single Source of Truth (Eliminated JSON Snapshot Files)

The system used JSON snapshot files (`data/starting_equity.json`, `data/starting_balance.json`) to persist starting equity across restarts. These files were stale, error-prone, and the root cause of multiple financial reporting bugs. FID-117 eliminates them entirely.

**Architecture:** Two sources of truth — chain (current state) + SQLite journal (historical). Everything else derived at query time.

**Changes:**
- `src/monitor/journal.rs` — Added `settings` table (key-value SQLite store) + 5 methods: `get_setting`, `set_setting`, `get_starting_equity`, `ensure_starting_equity` (idempotent — only writes on first boot), `get_peak_equity` (MAX from equity_snapshots)
- `src/core/shared.rs` — Added `starting_equity: Arc<RwLock<f64>>` to SharedEngineData
- `src/engine/mod.rs` — Orphaned JSON file cleanup at startup. FID-117 snapshot: calculates equity = USDC + token values, records to journal on first boot, loads into shared state. Peak equity restored from journal MAX(equity_snapshots) on restart. Simplified chain_equity sync. Removed all stale FID-116 JSON file code.
- `src/api/mod.rs` — `get_session` reads starting_equity from `shared.starting_equity` (3 lines) instead of reading JSON file (15 lines of file I/O)

### Fixed — FID-116: Chain-Only Truth (Stale Data Causing $25+ Losses)

The system had 5 competing sources of truth for financial data (config, snapshots, journal, portfolio, chain), producing contradictory numbers that caused incorrect position sizing, wrong profit display, phantom trades, and real money loss. Dashboard showed -$10.27 loss when wallet was actually up +$5.62 (+29%).

**Root causes fixed:**
- `set_balance()` reset `peak_equity` to raw USDC balance, corrupting drawdown tracking
- `starting_balance` hardcoded $30 from config when actual on-chain was $19.35
- FID-115 snapshot captured stale portfolio equity (missing 10.71 STG "ghost capital")
- `sync_balance()` ran every 15 min — stale equity for up to 15 min after swaps
- Wallet sync ignored `on_chain_qty > tracked_qty` gap — loose tokens invisible to equity

**Changes:**
- `src/execution/portfolio.rs` — `set_balance()` preserves `peak_equity`, sets `equity = balance` as safe intermediate
- `src/core/shared.rs` — Added `chain_equity: Arc<RwLock<f64>>` to SharedEngineData for dashboard-wide true equity
- `src/api/mod.rs` — `get_portfolio` uses `chain_equity` when available; `get_session` reads from `data/starting_equity.json` and uses chain_equity for PnL
- `src/engine/mod.rs` — (a) FID-116 snapshot: calculates true on-chain equity (USDC + all token positions × price) at first boot, saves to `data/starting_equity.json`. (b) `chain_equity` updated after periodic `sync_balance`. (c) Wallet sync gap fix: updates position qty when `on_chain_qty > tracked_qty` + calls `refresh_equity()` after
- `.gitignore` — Both `data/starting_balance.json` and `data/starting_equity.json` listed

### Changed

- `src/engine/mod.rs` — Added FID-111 position-pair injection block (~60 lines)
- `src/engine/mod.rs` — Added FID-112 FINAL SIDE CORRECTION block (~40 lines)
- `src/engine/mod.rs` — Made `active_pairs` mutable (line 149)
- `src/engine/mod.rs` — Added FID-116 snapshot, chain_equity sync, and gap fix (~70 lines)

### Build & Test

- 273 tests passing, 0 clippy warnings
- Engine compiled successfully with all fixes

## [0.13.1] — 2026-06-10

### Changed — FID-110: Engine Decomposition (Sessions 1–4)

The 7,214-line monolithic `engine.rs` was decomposed into a structured module with extracted utilities, test harness, and state management.

**Extracted modules:**
- `engine/utils.rs` (200 lines) — `parse_timeframe`, `create_executor`, `derive_address_from_key`, `load_knowledge_base`, `exchange_base`
- `engine/training.rs` (1,846 lines) — 11 functions: `run_training`, `run_sandbox`, `run_action_test`, `run_training_batch`, `run_live_test`, `run_historical`, `run_model_comparison`, and helpers
- `engine/debug.rs` (407 lines) — `dry_run`, `run_live_test` (sandbox dry-run and live test modes)

**EngineState struct (Session 3):**
- 48-field struct encapsulating all mutable engine state (portfolio, agent, insight, metrics, etc.)
- `EngineState::new()` constructor (~1,150 lines of init code extracted from `run()`)
- `run()` is now a thin wrapper: calls `new()`, destructures, delegates to loop

**Result:** `engine/mod.rs` is now 4,581 lines (down from 7,214). Modules are independently testable. Sessions 5–7 (loop body extraction, cycle sub-phases, audit) deferred.

### Changed
- `src/engine.rs` (deleted) → `src/engine/mod.rs` + `utils.rs` + `training.rs` + `debug.rs`
- Root scripts (`run-247.bat`, `run-canary.ps1`, etc.) moved to `scripts/` folder (keep `start.bat` in root)
- Session file `session-ses_14d3.md` moved to `dev/session-summaries/`
- `dashboard/src/app/page.tsx` — Disconnected overlay message updated
- Stale research docs deleted: `dev/AI Crypto Scalping Agent Optimization.md`, `dev/research-prompt-scalping-optimization.md`
- Archived FIDs: FID-107 (scalping conversion), FID-108 (DEX reliability — superseded), FID-109 (chain-first — superseded), MASTER-FID-2026-0609

### Build & Test
- 273 tests passing, 0 clippy warnings
- Engine tested in production: 3 cycles completed, all systems operational

---

## [0.13.0] — 2026-06-10

### Fixed — Position Sizing: Session Multiplier Overflow + Hard Rejection

The position sizer capped at 99% of balance, but the session multiplier (1.2x during US-EU Overlap) was applied AFTER the cap, inflating position size past wallet balance. The concentration check then hard-rejected with no fallback — killing valid signals.

**Root cause:** `$24.117 * 0.99 = $23.88` → session 1.2x → `$28.65` → exceeds `$24.12` → BUY REJECTED. Signal dead, no retry.

1. **PositionSizer overflow cap** (`src/risk/position.rs:173`) — Changed from `0.99` to `0.9999` (99.99%). Prevents rounding `$24.117` up to `$24.12`.
2. **Auto-adjust instead of reject** (`src/engine.rs:3260-3289`) — When order value exceeds concentration cap, quantity is reduced to fit (`safe_max = balance * cap * 0.9999`). Logs `ADJUSTED` instead of `REJECTED`. Signal preserved.
3. **Correct percentage in log** — Hardcoded "33%" replaced with dynamic label (`100%` in full_deploy, `33%` otherwise).
4. **LLM feedback injection** — On auto-adjust, appends a `FEEDBACK` entry to the decision log: "Your BUY signal was correct. Position auto-adjusted." Prevents the LLM from second-guessing valid analysis on the next cycle.

**Before:** Position sized → session inflates → hard reject → signal lost
**After:** Position sized → session inflates → auto-adjust to cap → order placed

### Changed

- `position.rs` overflow cap: `0.99` → `0.9999`
- `engine.rs` concentration check: hard reject → auto-adjust with fallback

## [0.12.9] — 2026-06-10

### Added — FID-108: DEX Execution Reliability (9 Changes)

1. **Enhanced pre-flight diagnostics** — Structured logging for why swaps fail
2. **Error categorization** — Transient/Permanent/UserFixable classification
3. **FailureTracker** — Blacklists tokens after 5 permanent failures in 60 min
4. **execute_with_retry()** — Tries next pair in queue on failure
5. **Gas buffer increase** — 2x→2.5x with 750K floor for Permit2 calldata
6. **Circuit breaker fix** — Only blocks Buy/Sell, not management actions (ADJUST_STOP/CLOSE)
7. **Session penalty** — -10% confidence during deep Asian (02:00-05:59 UTC)
8. **Spread filter** — Configurable bps threshold (default 30)
9. **Token address resolution** — Blockscout API lookup for unknown tokens

### Added — FID-109: Chain-First Architecture

1. **Phantom detection fix** — Checks on-chain token balances before clearing positions
2. **Wallet sync fix** — Includes discovered pairs (not just static config)
3. **Shared state sync** — Forces dashboard update after wallet recovery
4. **Executor-to-portfolio sync** — Copies DexTrader positions to PortfolioManager
5. **Slippage fix** — Increased from 15 bps to 30 bps for 0x Gasless API compatibility

### Fixed

- Circuit breaker blocking ADJUST_STOP when max positions reached
- Phantom detection clearing real positions on restart
- Wallet sync missing discovered pairs (STG/USD etc.)
- Dashboard showing 0 positions despite on-chain holdings
- Slippage too tight for 0x Gasless API (15 bps → 30 bps)

### Changed

- `slippage_pct`: 0.0015 → 0.0030 (30 bps minimum for Gasless API)
- `spread_filter_bps`: 30.0 (new config field)
- `session_penalty_deep_asian`: 0.90 (new config field)

## [0.12.9] — 2026-06-09

### Added — FID-093: Dashboard Tabbed Terminal with Command Bridge

The dashboard terminal is no longer a read-only log viewer. It now has two tabs:

**Logs tab** (existing): read-only stream of engine output — unchanged.
**Command tab** (new): bidirectional channel for sending commands to the agent and receiving responses.

**13 operator commands:**
- `override_close` — Force-close a position by pair name
- `override_stop` — Set stop-loss for a position
- `inject_context` — Inject operator message into next LLM evaluation
- `query` — Ask the agent a question (one-shot LLM call)
- `explain` — Explain last decision for a pair
- `set_autonomy` — Change autonomy level (autonomous/confirm/suggest)
- `approve` — Approve pending action (confirm/suggest mode)
- `pause` — Halt all trading
- `resume` — Resume trading
- `status` — Get current engine/agent state
- `feedback` — Operator verdict on a trade
- `watch` — Add pair to evaluation list for N cycles
- `undo` — Reverse the last command

**Natural language support:**
- `close weth` → override_close for WETH/USD
- `status` → status
- `pause` → pause
- `set stop link 7.50` → override_stop for LINK/USD at 7.50
- `what's happening with btc` → explain for BTC/USD

**Security:**
- inject_context: 500-char limit, 5/cycle rate limit, command injection rejection
- query: 3/5min rate limit, 30s timeout
- All commands: 10-minute TTL, bounded queue (100), auth required

**Frontend:**
- Tabbed terminal with Logs and Command tabs
- Ctrl+L (Logs), Ctrl+K (Command) keyboard shortcuts
- Command history with arrow keys (last 50)
- Color-coded response cards (green/red/yellow/blue)
- Connection status indicator

### Build & Test

- 273 tests passing (9 new from commands module), 0 clippy warnings

## [0.12.8] — 2026-06-09

### Fixed — FID-105: 0x API Swap Direction Reversal (Critical)

The 0x API `/quote` endpoint returned calldata for the opposite swap direction. When closing a LONG AAVE position (selling AAVE for USDC), the API returned a route that **bought AAVE with USDC** instead. The engine signed and broadcast it without checking direction. Result: wallet lost USDC, gained more AAVE.

**Fix:** Added `verify_swap_direction()` that parses ERC-20 Transfer events from the transaction receipt after confirmation. Verifies that `src_token` was sent from the wallet and `dst_token` was received. If direction is reversed, the trade is rejected with an error.

**Also:** `sign_and_send` now returns `(tx_hash, TxReceipt)` tuple so callers can inspect receipt data.

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.7] — 2026-06-09

### Fixed — Master FID: P0-1 Balance + P1-1 Parser + P1-2 Bear Market + P1-3 Gemini Priority 1

**P0-1: Balance Query Zero + Missing Pair Eval + Age Reset**
- Added `balance_cache` to DexTrader — caches last known on-chain balance per token during `sync_balance`, used as fallback when `query_token_balance` returns 0 during close
- Batch pair count validation already implemented (logs missing pairs at engine.rs:2136)
- Wallet recovery `opened_at` now uses epoch-0 sentinel (timestamp 0) instead of `Utc::now()` — dashboard shows "unknown" for recovered positions

**P1-1: Parser Bugs + Token Discovery**
- `partial_extract` now extracts `side` from JSON instead of hardcoding `Side::Long`
- `partial_extract` confidence default changed from `unwrap_or(0.5)` to `unwrap_or(0.0)` — broken JSON no longer bypasses 0.40 confidence floor
- `extract_from_freeform` Pass/Hold confidence changed from 0.5 to 0.0
- `discover_tokens()` wired into engine.rs startup — discovers Arbitrum tokens from Blockscout and adds to token DB

**P1-2: Bear Market Pre-Scoring Filter**
- ADX threshold lowered from 25.0 to 20.0 in pre-scoring filter
- Added volume spike as 4th pre-scoring signal (volume > 1.5x volume_sma)

**P1-3: Gemini Priority 1 — ATR TP, BB Squeeze, Dynamic ADX**
- TP2/TP3 computed from ATR in engine BUY path (TP2 = TP1 + ATR*1.0, TP3 = TP1 + ATR*2.0)
- Bollinger Band Squeeze detection added to pre-scoring (BB inside Keltner Channels)
- Dynamic ADX threshold: scales from 25 (F&G=50) to 18 (F&G≤20) using linear interpolation

### Chores

- Archived all resolved FIDs into `dev/fids/archive/2026-0609-cleanup/`
- `dev/fids/` now contains only `MASTER-FID-2026-0609.md`
- Updated Master FID with all fixes marked complete

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.6] — 2026-06-09

### Fixed — FID-103: DEX Price as Authoritative Source (Structural Plumbing)

The agent and dashboard were using Kraken (CEX) prices for all decisions and PnL, but trades execute on 0x DEX (Arbitrum). This created a systematic price gap between what the agent sees and what actually happens on-chain.

**Fixes:**
- **Added `dex_price: Option<f64>` to `FullContext`** — The agent prompt now shows DEX price as authoritative when available, with spread warning vs Kraken price. Falls back to Kraken-only display when DEX price unavailable.
- **Added `dex_prices` to `SharedEngineData`** — Thread-safe shared map for DEX prices, mirrors existing `ws_ticker_prices` pattern.
- **DEX price in positions API** — `/api/positions` now includes `dex_price` object with price and age for each open position.
- **DEX price parsing from 0x `/price` response** — `buy_token_price_usd` extracted from 0x API and stored in `LiquidityCheck`.
- **Dashboard spread indicator** — Color-coded spread badge (green/amber/red) shows DEX vs Kraken price divergence. DEX price row added to position metrics.
- **Better PnL calculation** — TradeRecord exit price now uses actual DEX execution proceeds (`verified_proceeds / actual_close_qty`) instead of stale Kraken price.
- **Balance query fallback warning** — `warn!()` instead of silent `unwrap_or(close_qty)` when on-chain balance query fails during close.
- **Restored doc comments** on `LiquidityCheck` struct fields (stripped by bad session).

**Note:** Structural plumbing is complete. Live DEX price population in the main evaluation loop (calling 0x `/price` per cycle and wiring into `FullContext::dex_price`) is planned future work (FID-103 remaining items).

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.5] — 2026-06-09

### Fixed — FID-098: Episodic Memory Feedback Loop (Model Never Learns)

The LLM was making decisions every 5 minutes but never learning from outcomes. `EpisodicMemory::update_outcome()` was never called when trades closed — the feedback loop was completely broken.

**Fixes:**
- **Wired `update_outcome()` in 3 trade close paths** — AI-initiated close, stop-loss/TP close, and external close (reconciliation). Episodes now get actual PnL, win/loss, and achieved R:R when trades close.
- **Wired `DecisionLog::context_for_pair()` into prompt** — The model now sees its recent same-pair decisions with outcomes in a new `## Recent Decision Log` section.
- **Episode store** — Maps pair-action-tick → episode_id so outcomes can be matched to decisions at close time.

**Before:** `## Dynamic Memory Context` showed 0 win rates, all episodes as "HELD"/"OPEN". Model flew blind.
**After:** Win rates populate after first closed trades. Recent analogs show "WIN (+1.2R)" or "LOSS (-0.5R)". Decision log shows past reasoning with outcomes.

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.4] — 2026-06-09

### Fixed — FID-097: Circuit breaker baseline corruption + position resurrection

Five fixes from FID-097 after v0.12.3 exposed downstream issues:

- **peak_equity reset after reconciliation** — After phantom positions are removed (both at startup and per-cycle), `peak_equity` and `drawdown_pct` are reset to current equity. Fixes the circuit breaker being permanently stuck at 50%+ drawdown.
- **Position resurrection guard** — Added `reconciliation_removed: HashSet<String>` that tracks all positions removed by on-chain reconciliation. Both FID-074 revert paths check this set before restoring positions — prevents phantom positions from reappearing.
- **Batch deduplication** — LLM batch responses are now deduplicated by pair name (keep last decision). Duplicates are logged with a warning.
- **Wallet address masking (Law 12)** — Wallet address in logs is now masked to first 6 + last 4 characters.
- **Batch size validation** — When LLM returns fewer decisions than requested, missing pair names are logged as a warning for observability.

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.3] — 2026-06-09

### Fixed — Reconciliation queried wrong token (USDC instead of WETH/LINK)

The on-chain reconciliation used `resolve_pair_on_chain(pair, Side::Long)` which returns `(USDC, WETH)` — so it queried USDC balance ($25.97) instead of WETH balance (0). Since USDC was non-zero, the reconciliation never detected the phantom positions.

Changed to `Side::Short` which returns `(WETH, USDC)` — now correctly queries the base token (WETH/LINK) balance. Same class of bug as FID-094 (wrong token resolution for side).

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.2] — 2026-06-09

### Fixed — Version integrity: external close trade record separated from v0.12.1

- **External close trade record** — When reconciliation detects tokens sold externally (on-chain balance = 0), now records a `TradeRecord` with estimated exit price from market data. `on_chain_verified: false`, notes: "External close — tokens sold outside engine." Previously the position was silently removed with no trade history.

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.1] — 2026-06-09

### Fixed — FID-096: On-Chain Reconciliation + Zero-Base Enforcement

**The problem:** Engine operated on stale portfolio data. `sync_balance()` only synced USDC every 3 cycles — token balances (WETH, LINK) were checked once at startup and never again. If tokens were sold externally, the engine continued tracking phantom positions, making decisions on non-existent holdings, and showing incorrect portfolio values.

Additionally, the Zero-Base Review ("Would you buy at current price?") was correctly performed by the LLM but had no parser enforcement. The AI said "No" and chose HOLD anyway.

**Fixes:**
- **On-chain token reconciliation** — Every 2 cycles (10 min), queries on-chain balance for all held positions. If balance is 0 but position exists → external close detected → removes position from PortfolioManager, DexTrader, journal, and executor_position_map. Logs equity correction and dashboard notification. (`engine.rs`)
- **Zero-Base Review enforcement** — Added `would_initiate_new_long` field to TradeDecision struct. Parsed from nested `position_audit[0]` in LLM JSON. If `false` + action is HOLD → parser overrides to CLOSE. (`decision_parser.rs`)
- **ExecutionEngine trait** — Added `query_token_balance()` and `chain_id()` to the trait so the engine can query on-chain balances via the trait object. (`engine.rs`, `trader.rs`)
- **HOLD confidence prompt** — Updated output_format.md: "For HOLD decisions, set confidence to your conviction in the HOLD thesis, NOT 0.0." (`output_format.md`)

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.12.0] — 2026-06-08

### Fixed — FID-093: A-Z Logic Cleanup (Partial) + FID-094: Close Death Loop

**FID-093 (Partial — 9 of 28 items):**
- C9: eval_in_progress flag stuck on LLM timeout → reset flag before continue
- C2: Version string hardcoded v0.10.5 → use env!("CARGO_PKG_VERSION")
- C5: Permanent dead tokens list (never cleared)
- C6: Ethereum DAI address typo fixed
- C7: Duplicate USDT0/USDTE removed
- C8: Price tolerance widened from 0.5% to 1.0%
- C10: Base chain placeholder addresses marked as unsupported
- D1: Midnight UTC reset independent of price updates
- D4: Removed unwired max_spread_bps from config

**FID-094: Close Execution Death Loop (critical):**
- **Root cause:** Side correction at startup updated PortfolioManager but NOT DexTrader's internal positions map. close_position_internal read stale SHORT from DexTrader, resolved USDC as src_token instead of WETH, queried USDC balance (= $0), close failed. SL fired again next cycle. 45+ minutes of phantom SL events.
- **Fix 1:** Sync corrected positions to DexTrader after side correction via register_position()
- **Fix 2:** Close retry cooldown (30 min) — skip close if recently failed, breaks death loop
- **Fix 3:** FID-088 trigger guard — don't fire ADJUST_STOP when close is on cooldown
- **Fix 4:** Death loop detection — 3+ consecutive SL fires → halt close attempts for 1 hour
- **Fix 5:** Zero-amount swap guard — return error if close qty < 0.0001 instead of calling 0x

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.9] — 2026-06-08

### Fixed — FID-092: Dead Capital Trap (Parabolic SAR, Zero-Base Review, Adverse Trend Exit)

**The problem:** Agent held two LONG positions for 48+ hours, losing $3.73 of $30 (12.4%) with no exit mechanism. The agent evaluated every 5 minutes, said "hold and monitor," and never exited. Root cause: ADX logic error (FID-088's Dead Capital trigger suppressed exit when ADX was high), LLM cognitive biases (status quo, sunk cost, default effect), no time-based exit, no Parabolic SAR.

**Gemini research identified 3 key insights:**
- ADX measures trend STRENGTH, not direction. ADX 26-54 + underwater LONG = strong bearish trend AGAINST the position. The trigger should fire MORE urgently.
- Parabolic SAR accelerates toward price as time passes — prevents indefinite holding.
- Zero-Base Review ("would you buy this asset today?") eliminates sunk cost bias.

**Engine-side forcing functions (4 new triggers):**
- **Parabolic SAR exit:** Dynamic trailing stop that accelerates toward price. If price crosses SAR → engine executes CLOSE automatically, bypassing LLM. (`indicators.rs`, `engine.rs`)
- **Adverse trend exit:** ADX > 25 AND position underwater AND EMA bearish → CLOSE. Fixes FID-088's backwards ADX logic. (`engine.rs`)
- **Maximum hold duration:** Position open > 24 hours AND PnL <= 0 → CLOSE. Winning positions exempt. (`engine.rs`)
- **Per-position drawdown limit:** Position loss > 5% of portfolio equity → CLOSE. Fires BEFORE hard stop loss. (`engine.rs`)
- **Full scan:** All 10 pairs evaluated even when fully deployed (MONITORING mode). Agent now sees all charts for opportunity cost awareness. (`engine.rs`)

**Prompt architecture redesign (4 prompt files):**
- **Zero-Base Review** (`base_identity.md`, `strategy_knowledge.md`): "If you held $0 of this asset, would you buy it today?" If no → CLOSE. Eliminates sunk cost fallacy.
- **Forced-choice Boolean schema** (`output_format.md`): `would_initiate_new_long_at_current_price`, `is_ema_bullish`, `is_price_making_higher_highs`. If would_initiate is FALSE → action MUST be CLOSE.
- **Debiasing directive** (`risk_constraints.md`): Explicit prompt about sunk cost fallacy, status quo bias, and opportunity cost decay.
- **Cash Conversion Mode** (`risk_constraints.md`): When $0 USDC, close unless asset justifies consuming 100% of liquidity. Cash is a strategic position.
- **New management triggers** (`risk_constraints.md`): Adverse trend, max hold duration, drawdown limit added to trigger list.

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.8] — 2026-06-08

### Fixed — FID-089: Engine Trigger Stale Price + Balance Query Zero

**The problem:** FID-088's engine-side management trigger used `pos.current_price` (stale entry price from wallet recovery) instead of the actual market price from candle data. This set the stop ABOVE market price, causing an immediate false stop loss. Additionally, `query_token_balance` returned 0 despite tokens existing on-chain, and `sync_balance` still used the old `unwrap_or(U256::ZERO)` pattern.

**4 fixes + 3 guards:**

- **Bug 1 — Stale price:** Engine trigger now uses `market_stores.get(&pair).last().close` for actual market price instead of `pos.current_price`. (`engine.rs`)
- **Bug 3 — Balance query:** Added debug logging to `query_token_balance` (logs token address, hex response, parsed value). Fixed `sync_balance` to use `match` instead of `.unwrap_or(U256::ZERO)`. (`trader.rs`)
- **Guard 1 — Stale price detection:** If `pos.current_price` is within 0.1% of `pos.entry_price`, skip trigger entirely (price hasn't been refreshed yet). (`engine.rs`)
- **Guard 2 — ATR sanity check:** If ATR > 10% of price, ATR data is unreliable, skip trigger. (`engine.rs`)
- **Guard 3 — Effective price fallback:** Use actual market price if available, else fall back to `pos.current_price`. (`engine.rs`)

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.7] — 2026-06-08

### Fixed — FID-088: Agent Action Paralysis (Cognitive Forcing Functions)

**The problem:** The AI agent correctly diagnosed market patterns and position issues (wide stops, invalidated theses, dead capital, ranging regimes) but defaulted to PASS/HOLD instead of executing the actions its own reasoning demanded. Root cause: LLM status quo bias + asymmetric action thresholds (entries require 3+ triggers, management required none).

**5 architectural changes implemented:**

- **Identity rewrite** (`base_identity.md`): Agent is now a "ruthless autonomous executioner" with absolute authority for position management. No permission needed to fix legacy errors or tighten risk.
- **Mandatory stop audit** (`stop_loss_behavior.md`): Stop distance >2.5x ATR → MUST ADJUST_STOP. No legacy deference. Trailing ratchet at 1R. Quantized adjustments (≥0.5R improvement).
- **Management triggers** (`risk_constraints.md`): 5 triggers that PROHOLD when active: stop distance violation, regime incompatibility, structural invalidation, dead capital tolerance, profit protection ratchet.
- **Regime translation matrix** (`strategy_knowledge.md`): ADX >25 = trend-following (3+ momentum triggers). ADX <20 = range-trading (support/resistance ARE triggers, momentum suspended). Transition handling for both directions.
- **Position audit schema** (`output_format.md`): Mandatory `position_audit` array in JSON output. Forces mathematical evaluation (stop distance / ATR, thesis status, management trigger, mandated action, opportunity cost) BEFORE action selection.

**3 enforcement layers:**

1. **Prompt-level**: Structured JSON schema forces evaluation before action (exploits autoregressive token generation)
2. **Parser-level**: If `management_trigger_active=true` but action is HOLD, parser overrides to mandated action. If `thesis_invalidated=true` but action is HOLD, parser overrides to CLOSE.
3. **Engine-level**: If LLM returns Pass but position has stop >2.5x ATR or regime incompatibility, engine overrides to ADJUST_STOP. This is the weak-model fallback.

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.6] — 2026-06-08

### Fixed — FID-087: Position Lifecycle Failures (8 interconnected bugs, critical)

**The problem:** On restart, the engine loaded stale SHORT positions from SQLite, applied LONG-only stop-losses, ignored the AI's close recommendation, reported 0 on-chain balance, and fired fabricated stop-loss exits — recording $2.62 in phantom PnL while real tokens remained untouched on-chain.

**8 bugs fixed atomically:**

- **Bug F — Journal cleanup:** Added `delete_position()` in on-chain close success paths (both main and fallback). Positions are now removed from SQLite when closed, preventing resurrection on restart. (`engine.rs`)
- **Bug G — Auto-stop side-aware:** Auto-stop override now computes SL based on position side. LONG: 8% below entry. SHORT: 8% above entry. Default SL check also side-aware (15% below for LONG, 15% above for SHORT). (`engine.rs`)
- **Bug A — Stale position detection:** Wallet recovery now checks if on-chain has tokens but journal says SHORT — forces LONG with corrected SL and take-profits. (`engine.rs`)
- **Bug C — Wallet recovery SL:** SL calculation now uses actual position side instead of hardcoded LONG. (`engine.rs`)
- **Bug E — SL direction validation:** On journal load, validates SL direction matches side. SHORT with below-entry SL (or LONG with above-entry SL) is corrected to 8% buffer. (`engine.rs`)
- **Bug D — Balance query:** `query_token_balance()` now returns `None` on hex parse failure instead of `Some(0.0)`. Caller's `unwrap_or(close_qty)` correctly falls back to requested quantity. (`trader.rs`)
- **Bug B — Action consistency:** Updated `output_format.md` with explicit CLOSE vs HOLD guidance. Added post-parse safety net: if reasoning contains "close"/"exit" without "hold"/"keep", overrides HOLD to CLOSE. (`decision_parser.rs`, `output_format.md`)
- **Bug H — Phantom trade prevention:** Added reverted trade tracking. When on-chain close fails and FID-074 reverts the portfolio state, the journal save is skipped. Prevents phantom trades from being persisted to SQLite. (`engine.rs`)

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.5] — 2026-06-08

### Fixed — Entry Price Drift + Wallet Recovery Duplication (Root Cause)

Three bugs in the startup sequence caused positions to get wrong entry prices on every restart:

1. **Stale position filter ran before journal load** — PortfolioManager was empty when the filter ran, so it never caught stale pair names (e.g. "ETH/USD" from before the rename). Moved filter to run AFTER journal positions are loaded. Also deletes stale positions from SQLite so they don't come back. (`engine.rs`)

2. **Wallet recovery checked DexTrader.positions (always 0 after restart)** — DexTrader doesn't persist positions across restarts. `sync_wallet_positions()` always saw 0 tracked quantity, creating duplicate wallet-recovery positions on every restart. Now: journal positions are registered in DexTrader during load, AND wallet recovery checks PortfolioManager as a second source of truth. (`engine.rs`)

3. **Wallet recovery overwrote journal entry price with current market price** — `INSERT OR REPLACE` + `entry_price = market_price` meant the real entry was lost on every restart. Now: if PortfolioManager already has a position for that pair (from journal), wallet recovery updates the quantity to on-chain but KEEPS the journal entry price. Only creates a new recovery position if no journal entry exists. (`engine.rs`)

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.4] — 2026-06-08

### Fixed — Stale Position Cleanup + OKX Funding Rate

- **Stale position filter** — Old pair names (e.g. "ETH/USD" before rename to "WETH/USD") were creating phantom positions from SQLite journal history. Added config-pair filter that drops positions whose pair names don't match current config on startup. (`engine.rs`)
- **OKX funding rate for WETH** — `fetch_okx_funding()` wasn't using `exchange_base()` mapping. "WETH/USD" was being sent to OKX as "WETH-USDT-SWAP" (doesn't exist). Fixed to use "ETH-USDT-SWAP". (`insight/funding_rates.rs`)

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.3] — 2026-06-08

### Fixed — ETH/WETH Data Integrity

- **Renamed ETH/USD → WETH/USD** — On Arbitrum, ETH is the native gas token ($1.99). WETH is the wrapped ERC-20 token used for trading ($13.60). The engine was labeling positions as "ETH/USD" when the actual holdings are WETH. This is a logic issue, not cosmetic — if the engine conflates ETH (gas) with WETH (trade), position valuations and LLM decisions break.
- **`display_pair()` / `exchange_pair()` functions** — Maps "ETH/USD" → "WETH/USD" for on-chain display, "WETH/USD" → "ETH/USD" for exchange APIs (Kraken/OKX/Binance all use "ETH"). (`core/types.rs`)
- **`exchange_base()` normalization** — All 6 data sources (OKX, KuCoin, Gate, CryptoCompare, Bybit, Binance) + Kraken candle client + WebSocket + funding rates now normalize "WETH" → "ETH" before calling exchange APIs.
- **Decision parser normalization** — LLM output "ETH/USD" is normalized to "WETH/USD" in `extract_pair()` so decisions match config pair names.
- **Config updated** — `default.toml` and `canary.toml` now use "WETH/USD" instead of "ETH/USD".

### Build & Test

- 264 tests passing, 0 clippy warnings

## [0.11.2] — 2026-06-08

### Changed

- **FID-057 deferred** — Liquidation Cascade Strategy moved to `dev/fids/deferred/`. Coinglass API costs $29/mo (entire account equity). Engine just stabilized after FID-085 overhaul — not the right time to add complexity. Revisit when equity grows or free OI data sources become available.
- **FID-082 archived** — Engine Freeze Deadlock. Fix verified in production (watchdog at engine.rs:3546). Released in v0.10.4.
- **FID-084 archived** — Live Situation Sandbox. Implementation verified: `--live-test` CLI flag at main.rs:233, `run_live_test()` at engine.rs:3779.
- **FID-057 Perfection Loop** — 5 iterations completed before deferral. 10 RED issues identified, 3 design refinements, 2 audit findings, 1 self-correction. Leverage references removed (execution is spot-only via 0x API).

### Build & Test

- 264 tests passing, 0 clippy warnings
- Engine running stable: 10+ cycles with fresh candle data, delta-compression logging, context engine active

## [0.11.1] — 2026-06-08

### Fixed — Post-Release Pipeline Fixes

- **Per-cycle candle refresh** — Candles were fetched once at startup and never refreshed. 199 of 200 candles were frozen, causing stale indicators and flat PnL. Added candle refresh at the start of every cycle loop: fetches fresh 200 candles from API for all active pairs, re-applies WS ticker prices on top. (`engine.rs`)
- **Delta-compression was stripping LLM context** — `DeltaResult::Delta` was sending ONLY the changed lines to the LLM instead of the full prompt. This caused the model to make decisions without position info, knowledge units, or market regime context. Fixed: always send the full prompt. Delta-compression is now observability-only. (`engine.rs`)
- **Noisy log suppression** — Delta-compression logged "92% change — regime shift" at INFO level every cycle because TSLN character-level diff cascades through all data rows when any price changes. Anti-thrashing WARN fired every cycle for the same reason. Changed both to DEBUG level. (`engine.rs`)
- **FID-086 archived** — Stale price pipeline fix verified in production. (`dev/fids/archive/`)
- **FID-085 original archived** — Superseded by v2 implementation. (`dev/fids/archive/`)

## [0.11.0] — 2026-06-08

### Added — FID-085: Context Window Overhaul (28 items across 8 phases)

**Context Engine Pipeline — 90%+ token reduction per evaluation cycle.**

The engine previously sent ~31K raw tokens per pair per cycle (9K Brain + 22K Eyes). This release implements a complete context management system synthesized from 4 research sources (Gemini Deep Research, Hermes Agent, OpenClaw, TradingAgents) with 264 tests and zero clippy warnings.

#### New Modules (8 files)

- **`agent::context_engine`** — Orchestrator for the 6-phase context lifecycle. Assembles prompts with TSLN/ZigZag/KBar encoding, adaptive candle counts by regime, SGDR cosine annealing budget, cache observability, context window guard, tool result summarization, deduplication, and deterministic fallback. (`context_engine.rs`)
- **`agent::context_state`** — Cross-cycle state management: delta-compression (skip unchanged data), anti-thrashing (skip compression when savings < 10%), soft trim (30% threshold), hard clear (50% threshold), TTL-based data pruning, historical data stripping. (`context_state.rs`)
- **`agent::decision_log`** — Append-only JSON log with atomic writes (temp-file + rename), auto-rotation, outcome wiring (PnL + reflection at trade close), and dual-format context injection (same-pair full + cross-pair reflection-only). (`decision_log.rs`)
- **`agent::provider_caps`** — Declarative per-model capabilities table. Handles DeepSeek (no tool_choice), MiniMax (reasoning_split), Anthropic (cache_control), Gemini quirks. Conditional cache_control based on model support. (`provider_caps.rs`)
- **`agent::token_budget`** — Exact BPE token counting via tiktoken-rs (cl100k_base_singleton). Replaces chars/4 heuristic with BPE-accurate counts. 6 tests. (`token_budget.rs`)
- **`core::tsln`** — TSLN (Time-Series Lean Notation): schema-first time-series format with delta-of-delta timestamps and differential pricing. 72% token reduction vs JSON. Lossless round-trip. 3 tests. (`tsln.rs`)
- **`core::time`** — Time utility functions for TSLN: parse_rfc3339_to_secs, secs_to_rfc3339, secs_to_datetime. 3 tests. (`time.rs`)

#### New Features

- **TSLN encoding** — Schema-first time-series format replaces JSON candle arrays. Delta-of-delta timestamps + differential prices. Default active (`encoding_mode: "tsln"`). Fallback to JSON via config. (`config/default.toml`)
- **ZigZag pivot extraction** — ATR-based threshold with 1.5% fallback. Extracts confirmed peak/trough pivots from candle data. (`indicators.rs`)
- **KBar feature extraction** — Pre-computes z-score, annualized volatility, trend score, volume ratio. Cold-start guard (min 20 candles). (`indicators.rs`)
- **Adaptive candle count by regime** — Ranging: 50, Trending: 100, Volatile: 200 candles. Per-regime optimization. (`context_engine.rs`)
- **BPE token counting** — tiktoken-rs with cl100k_base_singleton. Exact counts replace chars/4 heuristic. (`token_budget.rs`)
- **SGDR cosine annealing** — Token budget varies from max (scanning) to min (monitoring) over 288-cycle epoch. Smooth cosine curve. (`context_engine.rs`)
- **Brain caching with mutable/immutable partitioning** — Immutable brain (identity, constraints) cached permanently. Mutable knowledge section cached via SHA-256 digest. (`prompts.rs`)
- **Decision log with atomic rotation** — Append-only JSON, temp-file + rename pattern, auto-rotation after 500 entries. (`decision_log.rs`)
- **Trade outcome wiring** — Decision log entries updated with PnL + reflection when trades close. (`engine.rs`)
- **Context window guard** — Validates model context window against hard minimum (4K) and warn minimum (8K). Actionable messages. (`context_engine.rs`)
- **Cache stability observability** — SHA-256 digests of prompt components. Cache break detection with change codes. (`context_engine.rs`)
- **Deprecation warning for legacy JSON path** — Logs warning if encoding_mode is not "tsln". Verifies old path is dead code. (`context_engine.rs`)

#### Changed

- **`PromptComposer`** now caches immutable brain portion. Added `compose_mutable()` with digest tracking. (`prompts.rs`)
- **`AgentOrchestrator`** added `composer_mut()` accessor for split-call borrow pattern. (`orchestrator.rs`)
- **`Candle`** added `timestamp_unix()` and `timestamp_rfc3339()` methods. (`types.rs`)
- **`IndicatorEngine`** extended with `zigzag_pivots()` and `kbar_features()`. (`indicators.rs`)
- **`provider.rs`** `build_body()` now checks `ModelCapabilities` before adding `cache_control`. (`provider.rs`)
- **`engine.rs`** main evaluation loop now uses ContextEngine for prompt assembly, ContextState for delta-compression, and DecisionLog for logging. (`engine.rs`)
- **`config/default.toml`** — New `[context]` section with 19 fields for encoding, caching, SGDR, adaptive candles, microcompaction, TTL. (`config.rs`)

#### Dependencies

- **Added:** `tiktoken-rs v0.6` — BPE token counting (`Cargo.toml`)

#### Test Results

- **264 tests passing** (217 original + 47 new)
- **0 clippy warnings** (`-D warnings`)
- **All 8 new modules verified wired into production call graph (Law 4)**

## [0.10.5] — 2026-06-07

### Changed — Cycle Interval: 15m → 5m

- **Timeframe changed from 15m to 5m** — With owl-alpha (free model), API cost is $0. More frequent scanning catches moves faster. Trailing stops update every 5 minutes instead of 15. AI Decisions countdown updated to match. (`config/default.toml`, `page.tsx`)

## [0.10.4] — 2026-06-07

### Added — FID-084: Live Situation Sandbox

- **`--live-test` CLI flag** — Test any model against current live market data without starting the engine. Uses the exact same prompt pipeline as production (soul.md, knowledge base, context builder). Fetches live candles, insight, and positions from `dex_state.json`. Read-only, no state mutation, can run alongside active engine. (`main.rs`, `engine.rs`)
- **Usage:** `cargo run --release -- --live-test --model openrouter/owl-alpha --pairs ETH/USD,LINK/USD --show-prompt`

### Changed — Model Switch: MiMo → owl-alpha

- **Default model switched from `xiaomi/mimo-v2.5-pro` to `openrouter/owl-alpha`** — Free model, comparable quality on live test (same actions, similar R:R, 0 parse errors). Saves ~$0.50/day in API costs. Preserves $11 OpenRouter credit for when account grows past $500. (`config/default.toml`)

### Fixed — Terminal Timestamps + AI Decisions Countdown

- **Engine timestamps 1 hour behind** — `est_now()` hardcoded `UTC - 5 hours` (EST) instead of using system timezone. EDT is UTC-4. Now uses `chrono::Local::now()` which respects DST automatically. (`console.rs`)
- **Duplicate timestamps in terminal** — Terminal component added client-side prefix (correct time, wrong position) on top of engine timestamps (wrong time, right position). Removed client-side prefix — engine timestamps are the single source of truth. (`Terminal.tsx`)
- **Terminal placeholder misleading** — "type command..." implied an interactive command system that doesn't exist. Changed to "Engine log viewer — click terminal to scroll, Ctrl+C to interrupt". (`Terminal.tsx`)
- **AI Decisions no countdown** — Section header showed "4m ago" but not when the next cycle fires. Now shows "4m ago · next in 11m" with live countdown based on 15-min cycle interval. (`page.tsx`)

### Fixed — FID-083: AI Decisions Panel — Stale Decision Display

- **No timestamps on decisions** — User couldn't tell WHEN the decision was made. Same decision text shown across cycles looked like the agent was repeating itself. Added relative timestamp ("2m ago") next to each decision pair name and in the section header tag. (`page.tsx`)
- **Stale decisions not visually distinct** — Decisions older than 30 minutes now dimmed with `opacity-50`. (`page.tsx`)

### Fixed — FID-082: Engine Freeze — Deadlock in Shared State Lock Chains (critical)

- **Engine hung after 3 cycles for 1+ hours** — Two sync chains acquired the SAME `RwLock`s in OPPOSITE order while the API server read them every 4 seconds. Classic deadlock. Broke all 3 lock chains: each `write()` now in its own `{}` block so the lock releases before the next acquire. (`engine.rs:3117`, `3214`, `3313`)
- **`tokio::select!` with `ctrl_c()` interfering with sleep on Windows** — Replaced with plain `time::sleep()`. Ctrl+C handled by OS. (`engine.rs:3433`)
- **No watchdog for hung cycles** — Added cycle watchdog: logs CRITICAL if any cycle takes > 5 minutes. (`engine.rs`)

### Fixed — soul.md: LLM Prompt Cleanup

- **Removed all leverage/GMX content** — LLM was told it had 5-8x leverage and could turn $26 into $50 in 2 days. All leverage references stripped — now spot-only DEX via 0x API.
- **Honest cost math** — Added API cost context ($0.01-0.02/eval, ~$0.50/day when scanning, $0 when monitoring).
- **Survival framing with guard rails** — "Inaction is death at $26" balanced with "never skip 3+ triggers" and "if no setup, save API cost."
- **5 contradictions resolved** — "Take profits fast" vs scale-out strategy, "Move fast" vs entry criteria, "Trade actively" vs monitoring mode, "Hesitation" vs discipline.

## [0.10.3] — 2026-06-07

### Fixed — FID-081: Price Feed Staleness Protection (critical)

- **LLM told it had 5-8x leverage** — `soul.md` contained full leverage strategy (GMX V2, collateral buffers, liquidation rules) that was compiled into every LLM prompt via `include_str!()`. The LLM sized positions and set R:R targets assuming leverage that doesn't exist. Removed all leverage/GMX content — now reflects DEX-only spot reality. Stop math updated from leverage-adjusted to spot percentages. (`soul.md`)
- **Engine making decisions on 3-hour-old prices** — `ws_ticker_prices` stored prices with no timestamp. When WebSocket disconnected silently, stale prices were used indefinitely. Now tracks `(price, Instant)` per pair, skips WS prices > 5 min old, and falls back to candle data. (`engine.rs`)
- **Price sanity check** — Rejects price moves > 10% in a single tick to prevent flash-crash triggers on bad data. Logs warning but doesn't block (lets risk layer decide). (`engine.rs`)
- **Candle staleness warning** — Logs warning when last candle > 20 min old (15m candle interval + 5 min buffer). (`engine.rs`)
- **REST fallback** — When ALL WS prices are stale, fires a REST price fetch once per event (10 min cooldown). (`engine.rs`)
- **WS reconnect detection** — Sets `ws_just_reconnected` flag on `StateChange::Connected`, logs warning that prices may be stale until fresh data arrives. (`engine.rs`)
- **Per-pair staleness tracking** — Tracks seconds since last WS update per pair, exposes worst-case to dashboard. (`engine.rs`, `shared.rs`, `api/mod.rs`)
- **Dashboard "STALE PRICES" indicator** — Red pulsing chip with `fa-triangle-exclamation` icon shows when prices > 5 min old. Displays minutes since last update. (`page.tsx`, `api.ts`)
- **Stale-recovery re-eval** — When prices were stale (> 5 min) and positions are open, engine forces LLM re-evaluation of held positions on next cycle. Only evaluates pairs with open positions. Ensures hold thesis is still valid after price feed recovery. (`engine.rs`)

### Fixed — FID-079: Gas Check Only on Active Chain

- **Gas balance warnings for Base/Optimism when trading on Arbitrum** — `sync_balance()` iterated over ALL registered chain clients and logged CRITICAL errors for chains with zero gas. Now only checks the primary trading chain (`self.chain_id`). Eliminates false alarms for unused chains. (`trader.rs:sync_balance`)
- **Closed trades showing empty** — `get_trades` API filtered by `on_chain_verified` flag, but journal DB has no column for it (all load as `false`). Removed filter from trades list — flag is metadata, not a gate. Session stats also count all trades. (`api/mod.rs`)
- **AI Decisions "Waiting for first AI cycle…" when monitoring** — Misleading message when engine is in monitoring mode (fully deployed, no LLM calls). Now shows "Monitoring — LLM not active while fully deployed" with `fa-eye` icon. (`page.tsx`)
- **Equity curve showing stale/$0 data** — Historical snapshots from journal DB had stale values from before chain-first fix. Equity curve seed ran before wallet sync (captured $0). Now skips historical snapshots and seeds equity curve AFTER wallet sync so it captures recovered positions + correct balance. (`engine.rs`)
- **Position PnL stuck at $0.00** — `refresh_from_positions()` summed stale `p.unrealized_pnl` fields instead of recalculating from `entry_price` vs `current_price`. Now recalculates live PnL every cycle. Also: engine binary was locked by running process — `start.bat` couldn't overwrite. Must stop engine before rebuilding. (`types.rs:refresh_from_positions`)

### Added — FID-077: Sound Effects System

- **Win/loss audio clips** — Custom `.mp3` files in `dashboard/public/sounds/wins/` and `dashboard/public/sounds/losses/`. Random selection on trade close. Falls back to synthesized Web Audio sounds when no clips exist. (`sounds.ts`)
- **Sound files included** — 2 win clips (Here Comes the Money, Money SFX), 3 loss clips (Arms of the Angel, Oh God No, Price is Right Losing Horn)

### Added — FID-078: Enterprise Panel Redesign

- **HeroUI v3 migration** — Performance, Market Insight, Risk Controls, Open Positions, AI Decisions panels migrated to v3 components. (`page.tsx`)
- **`MetricRow` component** — Extracted reusable label+value row. Eliminated 11x repeated inline pattern across 4 panels.
- **`RiskBar` component** — Extracted labeled progress bar with threshold colors using v3 `ProgressBar` API. Eliminated 3x copy-pasted risk bar pattern.
- **AI Decisions confidence bars** — Migrated from stale `ProgressBarRoot`/`ProgressBarFill` to v3 `ProgressBar` with `color` prop.
- **`Separator` between sections** — Added visual dividers between logical sections in Performance, Market Insight, and Risk Controls panels.
- **13px values / 10px labels** — Data hierarchy across all panels.

### Added — Disconnected Overlay

- **Full-screen overlay when engine offline** — Replaced small red banner with transparent black overlay centered on screen. Savant logo at 64px 50% opacity, gradient text, red pulsing dot, disconnection message. `z-50` covers all content. Auto-disappears when engine reconnects. (`page.tsx`)

### Changed — Toast Notification System Overhaul

- **Position: top-right → bottom-right** — Less congested area, below Closed Trades panel. (`page.tsx`)
- **`gutter: 12`** — Better visual separation between stacked toasts (was 8px default).
- **`removeDelay: 500`** — Snappier toast dismissal (was 1000ms default).
- **`iconTheme`** — Success/error icons now match dashboard `--green`/`--red` CSS variables.
- **Error duration: 5000ms** — Stop loss toasts display longer for readability (was 4000ms).
- **Duplicate prevention** — Copy and CSV download toasts use unique IDs to prevent stacking.

### Fixed — FID-076: Chain-First Verification System

- **All data verified on-chain prior to display** — Fundamental principle enforced: chain is the single source of truth. Every number shown to the user is derived from on-chain state, not journal/stale data.
- **Profit KPI uses on-chain equity** — `total_pnl = equity - starting_balance` instead of summing journal closed trades. Eliminates phantom P&L from failed swaps. (`api/mod.rs`)
- **Closed trades filtered by `on_chain_verified`** — Added `on_chain_verified: bool` and `tx_hash: Option<String>` to `TradeRecord`. Only trades with on-chain tx confirmation are shown to the user. Phantom trades from `check_stops()` are marked unverified. (`types.rs`, `api/mod.rs`)
- **Win/loss stats from verified trades only** — Session endpoint filters to `on_chain_verified` trades before counting wins/losses. Win rate reflects actual on-chain performance. (`api/mod.rs`)
- **Phantom trade cleanup on failed close** — When executor close fails, the phantom TradeRecord is now removed from `closed_trades` via `retain()`, preventing phantom P&L accumulation. (`engine.rs`, `portfolio.rs`)
- **Wallet recovery uses market price** — Recovered positions use current candle close as entry price instead of stale journal trade entry. Eliminates incorrect P&L on recovered positions. (`engine.rs`)
- **Balance sync always runs** — Removed `if on_chain_balance > 0.0` guard that skipped sync when USDC was $0. Chain is always the source of truth, even at $0. (`engine.rs`)
- **Periodic balance sync every 3 ticks** — Was 10 ticks (150 min). Now 3 ticks (45 min). Dashboard stays accurate. (`engine.rs`)

## [0.10.2] — 2026-06-07

### Added — FID-075: Monitoring Mode Dashboard Badge

- **"LIVE · MONITORING" amber badge** — When fully deployed ($0 USDC), dashboard shows amber "MONITORING" badge instead of generic "RUNNING". Same visual pattern as hunt mode (neon glow, border, icon). Uses `fa-eye` icon.
- **`monitoring_mode` API field** — `/api/portfolio` returns `monitoring_mode: true` when USDC < $1. Dashboard shows badge only when monitoring AND not in hunt mode.
- **`--neon-amber` CSS variable** — `#ffb347` with matching glow text-shadow for the monitoring badge.

### Fixed — FID-073: Overnight Issues (5 of 8 items)

- **Stop override allows backward move** — LLM's ADJUST_STOP could set a stop lower than the current trailing stop for LONG positions. Added directional guard: for LONG, `new_stop > old_stop` required; for SHORT, `new_stop < old_stop`. Rejects invalid overrides with warning. (`engine.rs:2601`)
- **Double LLM evaluation overlap** — 15-minute cycle interval shorter than 100-160s LLM response time. Added `eval_in_progress` AtomicBool flag — set before batch call, cleared after. Next cycle skips Phase 2 if flag is still set. (`engine.rs`)
- **R:R always 0.0** — MiMo v2.5 Pro copied the example value `"risk_reward": 0.0` literally. Changed example to `2.5` and added explicit calculation instruction: "Formula: |TP1 - entry| / |entry - SL|". (`output_format.md`)
- **resolve_pair hardcoded to Arbitrum** — All 4 production callers in `trader.rs` now use `resolve_pair_on_chain(pair, side, self.chain_id)` instead of `resolve_pair()` which hardcoded chain 42161. Tests still use `resolve_pair()`. (`trader.rs`)

### Deferred from FID-073

- **5b: amount_to_wei uses f64** — Precision loss at scale. Needs `rust_decimal` crate. Separate FID.
- **5d: Gasless API not wired into main execute_swap** — Only used as fallback in close path. Separate FID.

### Fixed — FID-074: Overnight Execution Bugs (3 critical)

- **TP1 scale-out sent full position qty instead of 50%** — `close_position()` always used `pos.quantity` for the swap amount, ignoring the PortfolioManager's 50% scale calculation. Added `close_position_partial(position_id, quantity)` to `ExecutionEngine` trait. Engine now passes `trade.quantity` from stop results. Partial closes reduce position qty instead of removing it. (`trader.rs`, `engine.rs`, `engine.rs` trait)
- **Balance not reverted after failed close** — PortfolioManager's `check_stops()` unconditionally added PnL to `account.balance`. When executor close failed (dust), position was restored but balance was not reverted. Now subtracts `trade.pnl` from balance on failure. (`engine.rs:2844`)
- **Close swap dust failure — qty_wei > on-chain balance** — `amount_to_wei(pos.quantity)` rounded differently than actual on-chain balance. Now queries on-chain token balance via `query_token_balance()` before swap and uses `min(requested, on_chain)` as swap amount. (`trader.rs:close_position_internal`)

### Fixed — FID-072: Comprehensive Audit Remediation (29 findings)

- **`drain_retry_queue` — failed swaps silently dropped** — `kept` was always empty, `self.retry_queue = kept` cleared the queue every drain. Now clears queue explicitly. (F-07)
- **`usdc_address_for_chain` defaulted to Arbitrum for unknown chains** — Changed return type to `Option<&str>`. All 5 callers updated to handle `None` explicitly. Silent wrong-chain bug eliminated. (NF-01)
- **`TradeAction::Pass => unreachable!()` — engine panic risk** — Changed to `continue`. Pass decisions no longer crash the engine. (NF-11)
- **`TradeAction::AdjustStop` was a no-op** — Now wires to `stop_overrides` shared state. LLM stop adjustments are actually executed. (NF-12)
- **BUY/SELL accepted with `stop_loss=0.0`** — Added validation: Buy/Sell decisions must have `stop_loss > 0`. Naked positions blocked. (F-11)
- **No pre-trade balance sync** — `sync_balance()` now called before opening new position. Prevents trading with stale paper balance. (F-12)
- **Stablecoin pairs accepted** — USDC/USDT/DAI base pairs now rejected in `resolve_pair`. (F-13)
- **`normalize_llm_json` missed spaces** — Replaced string replace with regex: handles `"action" : "BUY"` (space before colon) and `"action": "ADJUST_STOP"` (underscore variant). (F-10)
- **Poisoned mutex cascade** — All 12 `.lock().unwrap()` calls replaced with `.lock().unwrap_or_else(|e| e.into_inner())`. Thread panic no longer cascades through mutex locks.

### Changed — FID-072: Behavioral Fixes

- **R:R logging** — `actual_rr` (calculated from prices) now logged alongside LLM's `claimed risk_reward` for debugging calibration. (B-01)
- **Max positions context** — LLM told "AT MAX POSITIONS — Do not propose new entries" when at capacity. Prevents wasted evaluation cycles. (B-02)
- **DeepAsian session penalty reduced** — Position size multiplier 0.5→0.7, breakout confidence penalty 0.6→0.75. Crypto markets don't respect traditional session boundaries. (B-03)
- **Volume filter active in all modes** — Previously skipped in live/DEX mode. Now active with lowered threshold ($10). Dead tokens filtered regardless of mode. (B-04)
- **Confidence discipline instruction** — Added to `strategy_knowledge.md`: "Evaluate setup quality independent of position P&L." Prevents confidence inflation from winning positions. (B-05)
- **`AccountState` now includes `max_positions`** — Previously only tracked `open_positions`. Now both available for context injection and validation.

### Lifecycle

- **FID-061 (Stop-Swap Bridge)** — Closed. Implemented via gasless fallback + `register_position()`.
- **FID-068 (HeroUI Color Migration)** — Cancelled. Not worth the risk.
- **FID-069 (Batch Fix + Dashboard Overhaul)** — Closed. All items complete.
- **FID-070 (Full HeroUI Conversion)** — Cancelled. User agreed current code is clean.
- **FID-071 (Batch Parse Fix)** — Closed. Implemented.
- **FID-072 (Audit Remediation)** — Closed. 13 items implemented.
- **FID-058 (GMX Sidecar POC)** — Deferred until $500+.
- **FID-060 (GMX Native Rust)** — Deferred until $500+.

## [0.10.1] — 2026-06-07

### Fixed

- **Batch evaluation JSON parse failure** — MiMo v2.5 Pro returns individual JSON objects with text between them instead of a clean JSON array. Added `extract_json_array()` with balanced brace counting to extract objects from surrounding text. Eliminates 9 sequential fallback calls in normal operation. (FID-071)
- **Ticker wrap-around** — CSS `translateX(-33.333%)` animation didn't work because `display:contents` spans return `offsetWidth=0`. Replaced with `requestAnimationFrame`-based Ticker component that measures `track.scrollWidth/3` and snaps by one copy width. Pauses on hover.
- **Dashboard crash: toLowerCase on non-string** — `memory?.cusum_status` could be a number. `String()` coercion added.

### Changed

- **HeroUI redesign: Performance, Market Insight, Risk Controls** — W/L uses HeroUI Chip (success/danger), sentiment uses Chip, trending coins use Chip (accent). Brier/CUSUM/funding rate have Tooltips explaining meaning. Circuit breaker uses Chip. Metrics in 2-column grid layout.
- **Batch evaluation debug logging** — Raw and thinking-stripped response logged at INFO level for debugging parse failures.
- **Version** — 0.10.0 → 0.10.1

## [0.10.0] — 2026-06-06

### Added

- **Ollama local model support** — `"ollama"` added as a valid AI provider. Points to `localhost:11434/v1` with no API key required. Enables testing local models (Gemma, Qwen, DeepSeek, frankenstein merges like Qwopus) through the existing sandbox harness.
- **Universal output parser (4-pass)** — `decision_parser.rs` now handles any LLM output format:
  - Pass 0: Strip ``/`` tags (Qwen, DeepSeek reasoning models)
  - Pass 1: Strict JSON parse
  - Pass 2: Manual JSON repair (truncated strings, unclosed brackets)
  - Pass 3: Partial extraction (salvage whatever fields are available)
  - Pass 4: Regex-based freeform NLP extraction — extracts pair, prices, confidence, R:R from natural language text when JSON parsing fails entirely
- **Gasless swap fallback in close path** — When `close_position()` detects a dust output error from the standard 0x Permit2 swap (0x can't route micro-amounts), it automatically retries via the 0x Gasless API. Gasless handles approvals and gas costs internally — no ETH needed, no Permit2 approval tx. Solves the live issue where stop-losses on small positions ($20-30) couldn't execute.
- **`DexBackend` trait: gasless methods** — `build_gasless_swap_tx()` and `poll_gasless_status()` added to the `DexBackend` trait with default "not supported" implementations. `ZeroXBackend` delegates to existing gasless code. `FallbackBackend` tries primary then secondary.
- **`run-ollama-tests.ps1`** — PowerShell script for benchmarking local Ollama models against the 60-scenario sandbox. Auto-detects available models, runs each with configurable timeout, generates comparison report.
- **`regex` crate** — Added as dependency for the universal parser's freeform text extraction.

### Fixed

- **CRITICAL: Stop-loss execution failure on micro-amounts** — Live incident where 0.01485 WETH (~$23) stop-loss couldn't close via standard 0x swap. API returned 0 output tokens (dust). System retried every cycle for 30+ minutes without success. Gasless fallback now handles this automatically.
- **Duplicate trade closures inflating win/loss count** — Same position closed multiple times across ticks (stop-loss fires, position re-registers via wallet recovery, stop fires again). Added deduplication: same pair+entry+exit+side within 60s = skip recording. (FID-065)
- **Dashboard TypeScript build errors** — `block_number` → `block_height`, `rss_count` → `rss_items`, `trending` → `trending_coins`. Caused by editing dashboard without reading full type definitions (Law 1 violation). (FID-066)
- **Trailing whitespace in engine.rs:5087** — `cargo fmt` internal error caused by trailing space on a `matches!()` line. Fixed manually.
- **Phantom ETH position in journal DB** — After manual WETH→USDC swap, the SQLite journal still contained the old ETH position record. Engine re-registered it on startup as a wallet-recovered position, inflating portfolio value by ~$23. Cleaned via direct DB deletion. (Wallet recovery side=SHORT bug noted for future fix.)
- **Dashboard crash: toLowerCase on non-string** — `memory?.cusum_status` could be a number. `(0 ?? "")` returns `0`, `0.toLowerCase()` throws TypeError. Wrapped with `String()`. (FID-066)

### Changed

- **`cargo fmt` applied project-wide** — All 43 source files reformatted to consistent style. No logic changes.
- **`.gitignore` expanded** — Added entries for: `prompt-results/`, `DEEP-RESEARCH-PROMPT.md`, `MODEL-TRAINING-RESEARCH.md`, `API-KEYS.md`, `data/sandbox_*`, `data/sandbox_reports/`, `data/model-comparison-*.md`, `data/test_memory.db*`, `LLM Crypto Trading Growth Strategy.md`.
- **Sandbox test artifacts cleaned** — Removed 15+ temp files from `data/` (sandbox stdout/stderr/output/report files, model comparison reports, test databases).
- **FID-062: Removed Kraken execution backend** — Deleted `src/execution/kraken.rs` (569 lines of dead code). `KrakenTrader` was never used for live execution. Removed Kraken match arm from engine, removed `KrakenTraderConfig`, removed Kraken balance sync. `exchange.backend = "kraken"` no longer valid — only `"0x"` and `"1inch"` accepted.
- **FID-062: Renamed data pipeline** — `KrakenClient` → `CandleClient` (`src/data/candle_client.rs`), `KrakenSource` → `KrakenFeed` (`src/data/sources/kraken.rs`). Removed 400+ lines of dead private API code (signing, order placement, balance queries). Console label "Kraken Data" → "Market Data". All variable names updated.
- **FID-063: Hunt mode under $500** — When idle capital > $5 and equity < $500, engine bypasses candle hash cache and pre-scoring filter to aggressively scan all pairs for entries. LLM receives explicit "HUNT MODE" instruction with idle capital amount. Hunt mode flag exposed via `/api/portfolio` endpoint.
- **FID-064: Dashboard copy buttons** — Added `CopyButton` component to `SectionHeader`. Copy buttons on Performance, Market Insight, Open Positions, Risk Controls, AI Decisions, Closed Trades sections.
- **FID-064: Hunt mode header tag** — "HUNT MODE" orange badge in dashboard header next to "LIVE · RUNNING". Only visible when active.
- **FID-066: Position re-evaluation for open positions** — Pairs with open positions bypass candle hash cache and pre-scoring filter every cycle. LLM evaluates current price + position state for stop adjustments even when candle data hasn't changed.
- **FID-066: Auto-rebuild in start.bat** — `start.bat` now runs `cargo build --release` AND `npm run build` (dashboard) before starting engine. Prevents stale binary and stale dashboard issues.
- **FID-067: Neon red hunt mode badge** — `--neon-red` CSS variable with glow text-shadow. Header badge and Performance section indicator both use neon red with glow effect.
- **FID-067: Engine fallback timeout** — LLM fallback path (9 individual calls when batch JSON parse fails) now has 5-minute total timeout. Per-call logging added so progress is visible in terminal.
- **Batch evaluation: thinking tag stripping** — MiMo v2.5 Pro wraps responses in `<think>...</think>`. `strip_thinking_tags()` now called on batch response before JSON parse attempt.
- **Batch evaluation: output_format.md updated** — System prompt now includes batch instruction: "When evaluating MULTIPLE pairs, respond with a JSON array." Previously only said "single JSON object."
- **Batch evaluation: 180s timeout** — `tokio::time::timeout(180s)` wraps the batch LLM call to prevent indefinite hang.
- **Batch evaluation: raw response logging** — Both raw and thinking-stripped response logged at INFO level before parse attempt for debugging.
- **Weekend mode removed** — `Session::Weekend` and `Session::SundayPreOpen` variants deleted. All days use time-of-day sessions (Asian/DeepAsian/LateAsian/European/UsEuOverlap/UsPostOverlap/LateUs). Crypto trades 24/7 — no off-hours.
- **Config alignment** — `starting_balance: 50.0 → 30.0`, `fee_rate: 0.0040 → 0.001` (DEX actual), `slippage_pct: 0.0005 → 0.005` (realistic), `exchange.name: "kraken" → "market_data"`.
- **Risk constraints updated** — `risk_constraints.md`: 5% daily loss (was 10%), 10% drawdown (was 20%), DEX fees (was Kraken fees), 1.5:1 R:R (was 2.0:1).
- **Sources label** — `KrakenFeed::name()` returns "Market Data" instead of "Kraken". Console shows `Sources: Market Data: [ETH/USD]` instead of `Sources: Kraken: [ETH/USD]`.
- **Dead dependencies removed** — `howler`, `use-sound`, `lucide-react`, `@types/howler` removed from dashboard. Sound effects use Web Audio API directly (sounds.ts).
- **Dashboard: ADJUST STOP formatting** — `ADJUSTSTOP` → `ADJUST STOP` via `.replace("_", " ")`.
- **Dashboard: 3-tier confidence coloring** — AI Decisions confidence bars and percentages color-coded: red (0-33%), amber (34-66%), green (67-100%).
- **Dashboard: action badge colors** — BUY=green, SELL/CLOSE=red, ADJUST/ADJUSTSTOP=amber with fa-sliders icon.
- **Dashboard: Performance section** — Win rate percentage prominently displayed, Brier score color-coded (green <0.20, amber 0.20-0.30, red >0.30), CUSUM color-coded (green positive, red negative), confidence cap color-coded.
- **Dashboard: Market Insight section** — Sentiment color-coded by Fear & Greed level, funding rate color-coded (green negative/squeeze, red positive/overleveraged).
- **Dashboard: Risk Controls section** — Drawdown/daily loss/positions progress bars color-coded by severity (green <50%, amber 50-80%, red >80%). Values color-coded to match.
- **Dashboard: HeroUI component integration** — Performance, Market Insight, Risk Controls sections now use HeroUI Chip (status badges), Tooltip (hover explanations for Brier, CUSUM, funding rate, drawdown, daily loss), and Spinner. Trending coins use Chip instead of raw spans. Circuit breaker, confidence cap, CUSUM, sentiment all use Chip with semantic colors.
- **Dashboard: scrolling news ticker** — CSS-only infinite scroll between header and KPI bar. Shows trending coins, F&G, funding rate, BTC dominance, block, positions with price + P&L. Pauses on hover. Directional arrows on all metrics.
- **Dashboard: position close button** — X button on each position with confirmation dialog. Calls `POST /api/positions/{pair}/close`. Engine forces stop to current price → triggers on-chain swap.
- **Dashboard: connection error banner** — Red banner when engine API unreachable.
- **Dashboard: CSV export** — Download button on Closed Trades section. `Ctrl+Shift+E` keyboard shortcut.
- **Dashboard: keyboard shortcuts** — `Ctrl+Shift+C` copies all sections, `Ctrl+Shift+E` exports trades CSV.
- **Dashboard: unified copy formatters** — `dashboard/src/lib/copy.ts` with `copyFormatters` for all 7 sections. Replaces all inline copy functions.
- **Dashboard: terminal copy button** — Extracts xterm buffer text for clipboard.
- **Dashboard: hunt mode header tag** — "HUNT MODE" neon red badge with glow, only visible when active.
- **Position close API** — `POST /api/positions/{pair}/close` endpoint. `close_overrides` in SharedEngineData. Engine reads close requests and forces stop to current price.
- **Version** — 0.9.1 → 0.10.0

### Decisions

- **FID-070: HeroUI full conversion — CANCELLED** — Evaluated converting all dashboard components to HeroUI (Card, Chip, Button, Table, ProgressBar). Decided against it: current custom components are clean and consistent, HeroUI conversion adds complexity with className overrides to maintain visual parity, risk of visual regression outweighs benefit. Dashboard uses HeroUI primitives (ProgressBarRoot/ProgressBarFill) where it matters. Will use HeroUI for new components going forward.

## [0.9.1] — 2026-06-05

### Added

- **`savant serve` command** — Single command starts engine + API (port 8080) + Next.js dashboard (port 3000). Dashboard auto-builds if not built. `cmd /c npm` for Windows compatibility.
- **0x `/price` liquidity pre-check** — `LiquidityCheck` struct with `available`, `buy_tax_bps`, `sell_tax_bps`, `buy_amount`, `balance_ok`, `allowance_ok`. Called before every BUY and close. Honeypot detection via buy tax > 1%.
- **`sellEntireBalance` parameter** — 0x API uses actual on-chain balance at execution time. Prevents dust/rounding failures on close swaps.
- **Multi-chain token databases** — Ethereum (19 tokens), Base (14), Optimism (14). `lookup_token()` now chain-aware for 4 chains. All chains enabled in config.
- **Wallet sync (chain-first reconciliation)** — Queries on-chain ERC-20 balances for all curated pairs on startup. Creates recovery positions for untracked tokens (entry price from trade history). Removes ghost positions for tokens no longer in wallet. `entry_price > 0` guard prevents corrupted positions.
- **Equity chart** — Recharts area chart in dashboard showing equity curve over time. API endpoint `/api/equity` querying `equity_snapshots` table.
- **Activity log copy button** — Click to copy full activity log to clipboard.
- **Scrollbar styling** — Custom scrollbar matching dark design system (thin, dark track, subtle thumb).
- **12h AM/PM timestamps** — Activity log uses 12-hour format instead of 24h.
- **Rejection logging to dashboard** — Every BUY rejection (price drift, liquidity, position sizer, concentration cap) logged to activity feed with reason.
- **Log broadcast channel** — `tokio::sync::broadcast` channel captures all tracing/savant_log output. Terminal WebSocket streams to dashboard in real-time.
- **SQLite busy_timeout** — `SqlitePool::connect_with` with `busy_timeout(5s)` + WAL mode. Prevents hanging on stale locks from crashed processes.
- **Dynamic gas management** — Queries `eth_gasPrice` from network, calculates cost per swap, requires enough for 2 swaps + 50% buffer. No hardcoded gas thresholds.
- **Circuit breaker dollar floors** — `min_daily_loss_usd = $5`, `min_drawdown_usd = $10`. Prevents false halts at tiny balances.
- **Full deploy mode** — `full_deploy = true` in config. At <$500 balance, 100% of capital into best-conviction trade.
- **Dynamic R:R** — `min_rr_ratio_low_balance = 1.2` at <$50 balance (was fixed 1.5).
- **Dynamic position sizing** — Risk tiers: <$500 = 100%, <$5000 = 10%, <$50000 = 5%, above = 2%. No hardcoded percentages.
- **Gas buffer increase** — 1.5x multiplier with 800,000 minimum (was raw 0x estimate). Fixes "out of gas" on Permit2 calldata.

### Fixed

- **CRITICAL: Close trade failure** — `close_position()` removed position from map BEFORE swap executed. If swap failed (dust, no liquidity), tokens stranded in wallet with no tracking. Now: position stays in map until on-chain USDC balance verified.
- **CRITICAL: Equity calculation wrong** — Used `balance + unrealized_pnl` (only counts profit, not position value). Now uses `balance + sum(position_values)` (includes deployed capital). All 5 callers fixed.
- **CRITICAL: Balance double-counting** — DB position restore deducted `entry_price * quantity` from balance. But balance was already reduced when trade executed on-chain. Removed deduction.
- **CRITICAL: Wallet desync (3 incidents)** — Engine crashed before saving positions to DB. Wallet sync created ghost positions with `entry_price=0`. Fixed: chain-first reconciliation after candle data loads, `entry_price > 0` guard.
- **CRITICAL: `sell_usd` decimal bug** — Was hardcoding USDC 6-decimal math for ALL tokens (including 18-decimal UNI). Now uses correct decimals per token.
- **Pre-flight "out of gas"** — 0x API returns `gas=600000` but Permit2 needs more. Added 1.5x buffer with 800,000 minimum.
- **Timer 0m** — `engine_started_at` initialized to `Some(Instant::now())` when engine running at API startup.
- **Market insight "No data"** — Shared state not seeded after first `refresh_multi()`. Now seeds immediately and syncs every 5 ticks.
- **DB balance overwrites on-chain** — In live mode, balance comes from on-chain USDC only. DB trades loaded for history but don't override balance.
- **SQLite connection hangs** — URL parameters don't work with sqlx. Switched to `SqliteConnectOptions` with `busy_timeout`.
- **Circuit breaker block file** — Auto-deleted on `savant serve` so stale blocks don't prevent startup.
- **Windows npm not found** — `std::process::Command::new("npm")` fails on Windows. Uses `cmd /c npm` via `#[cfg(target_os = "windows")]`.
- **Terminology: "DRY-RUN" → "PRE-FLIGHT"** — eth_call simulation renamed. "Paper Trading" removed from vault writer. Correct terms for live trading.
- **Dead code: `total_unrealized`** — Unused variable in paper.rs after equity calculation rewrite.
- **Clippy: `map_or` → `is_none_or`** — 0x liquidity check simplified per clippy suggestion.
- **Clippy: `single_match` → `if let`** — Token balance loop in trader.rs.
- **`prepared_for_retry` scope** — Brace fix broke sandbox function scope. Restored correct closing braces.

### Changed

- **10 curated high-liquidity pairs** — ETH, BTC, ARB, LINK, UNI, AAVE, PEPE, PENDLE, COMP, LDO (was 9 pairs including DOGE/BONK with uncertain Arbitrum liquidity).
- **Blockscout non-blocking** — For curated pairs, Blockscout check skipped entirely. For other pairs, logs warning instead of rejecting. 0x quote is the real liquidity gate.
- **`DexBackend::check_liquidity`** — Returns `LiquidityCheck` struct instead of `bool`. All backends updated (0x, 1inch, fallback).
- **`SwapParams` struct** — Added `sell_entire_balance: bool` field. All 8 construction sites updated.
- **`ExecutionEngine::check_liquidity`** — Returns `LiquidityCheck` with `available`, `buy_tax_bps`, etc.
- **Version** — 0.9.0 → 0.9.1

## [0.9.0] — 2026-06-04

### Added

- **First successful on-chain swap** — AAVE bought on Arbitrum for $9.54 (tx: 0x846d...b018e1). FLUID bought for $8.48 (tx: 0x85b0...29e9). Both confirmed on-chain.
- **Trailing stop-loss** — Auto-trails SL as price moves in our favor. Only for Full-scale positions with risk > 0. After TP1 scale-out (break-even SL), stop stays fixed.
- **CoinGecko verification gate** — DEX mode only trades tokens with CoinGecko-verified addresses. Blocks unknown/unverified tokens.
- **Position concentration cap** — Max 33% of portfolio per position. Prevents overconcentration.
- **Dead token cache** — Tokens with all-zero candle data are skipped after first failure. Retried every 10 cycles.
- **CoinGecko token filter** — Tokens not in CoinGecko Arbitrum list rejected in DEX mode.
- **Illiquid token filter** — Tokens with <5 unique close prices across 200 candles rejected.
- **`--model` CLI flag** — `cargo run -- --test --model openrouter/owl-alpha -n 20` to test any model in sandbox. Wired through `run_action_test`, `run_training`, `run_sandbox` → `run_training_batch`.
- **`--managed-keys` CLI flag** — Auto-creates temporary OpenRouter API key with $1 limit per test/training run. Uses existing `OpenRouterManagementClient`.
- **Gas buffer increase** — Gas limit 2x multiplier + 500K floor (was 1.2x). Prevents "out of gas" on Permit2 calldata.
- **Model comparison results** — Tested owl-alpha (free), DeepSeek V4 Flash, MiMo v2.5 Pro on 150 scenarios. MiMo best on Brier (0.47) and P&L (+$80.65). Owl free with 0 parse errors.

### Fixed

- **CRITICAL: Flawed SELL handling** — `TradeAction::Sell` was routed to close-only path. DEX can't SHORT without owning the asset. Now correctly skips with visible log.
- **CRITICAL: Short order amount_wei** — For SHORT orders, was computing `amount_to_wei(entry_price * quantity)` (USD value). Fixed to `amount_to_wei(quantity, src_decimals)` (token amount).
- **Console color system** — 12 distinct log types. LLM tags now CYAN_BOLD (was WHITE). INFO tags CYAN_BOLD (was invisible). PASS body WHITE (was dark grey). VAULT tag dim blue. WARN body matches tag.
- **R:R hallucination logging** — `[BUY REJECTED]` now shows claimed vs actual R:R with per-side computation (LONG vs SHORT).
- **SourceRouter log cleanup** — Error messages truncated to 80 chars. Pair names in dark grey brackets. Source name without redundant brackets.
- **CoinGecko filter: collapsible if** — Fixed clippy warning.

### Removed

- **GeckoTerminal from SourceRouter** — 99% failed requests, 30 req/min rate limit, zero value.
- **Dashboard scaffold** — 397MB Next.js dead weight (never used).

### Archived

- FID-045: Multi-chain 0x swap system
- FID-046: QoL improvements (7 items)
- FID-047: Sandbox model override
- FID-048: OpenRouter management key system

## [0.8.0] — 2026-06-04

### Added

- **Multi-chain 0x system (FID-045)** — Five phases: (1) 201 Arbitrum tokens, (2) Chain-aware database with `ChainConfig`, `ChainToken`, per-chain USDC addresses, (3) Multi-chain execution with `chain_clients` HashMap, per-chain gas monitoring, (4) Gasless API (`/gasless/quote`, `/gasless/submit`, `/gasless/status`), (5) Cross-Chain API (`/cross-chain/quotes`, `/cross-chain/status`)
- **Permit2 signature fix** — Added 32-byte signature length prefix to calldata encoding. 0x API v2 expects `calldata || sig_length (32 bytes, big-endian) || signature (65 bytes)`. Also uses API-provided `permit2.hash` field instead of computing own EIP-712 hash.
- **ERC-20 approve for Permit2** — `ensure_permit2_approval()` checks USDC/token allowance for Permit2 contract and sends approve(max) if insufficient. Called before every swap (place_order + close_position).
- **Multi-source candle architecture** — 6 active sources: Kraken, OKX, KuCoin, Gate.io, CryptoCompare, CoinGecko. 8 total sources including Binance/Bybit (geo-blocked, unused).
- **OKX candle source** — 40 req/2s, broad coverage, no API key required.
- **KuCoin candle source** — 300 req/10s, massive altcoin selection.
- **Gate.io candle source** — 300 req/s, obscure token coverage.
- **CryptoCompare candle source** — 100K calls/month, US-accessible.
- **198 Arbitrum tokens** — Real addresses from CoinGecko API `/coins/list?include_platform=true`.
- **xStock filter** — SPYX, QQQX, GLDX, CRCLX filtered (require 0x opt-in we don't have).
- **`eth_call` dry-run** — Verifies Permit2 calldata before broadcast.
- **test_swap.rs binary** — Dry-run swap verification tool.

### Fixed

- **CRITICAL: ERC-20 approve for Permit2** — Missing token approval for Permit2 contract was the likely root cause of all swap failures. `ensure_permit2_approval()` now checks and sets allowance before every swap.
- **APE token address** — Was `0x7f9FBf9bDd1F0e6E2c2c2c2c2c2c2c2c2c2c2c2c` (truncated placeholder). Fixed to CoinGecko-verified `0x7f9fbf9bdd3f4105c478b996b648fe6e828a1e98`.
- **AUSDT decimals** — Was 18 (wrong). aUSDT on Aave Arbitrum uses 6 decimals (same as USDT). Fixed.
- **SHORT order amount_wei** — Was computing `amount_to_wei(entry_price * quantity, decimals)` for SHORT orders, which sends the USD value instead of the token amount. Fixed: SHORT uses `amount_to_wei(quantity, src_decimals)`.
- **drain_retry_queue** — `kept` was always empty, so retries were lost after drain. Fixed to properly track entries below max_retries.
- **close_position fee accounting** — Closing fee (0.1%) was not deducted from balance on close. Now subtracts `fee_est` from proceeds.
- **FallbackBackend priority** — Was trying secondary (1inch) first, now tries primary (0x) first.
- **Quote failure aborts swap** — Previously "proceeding without spread check" on quote failure.
- **Volume filter relaxed** — Kraken spot volume irrelevant for Arbitrum DEX tokens; filter skipped in DEX mode.
- **Non-zero candle threshold** — Lowered from 50% to 30% for DEX mode.
- **SourceRouter rejects all-zero candle responses** — Kraken returning zeros for unsupported tokens no longer blocks fallback.

### Removed

- **Dead files** — Removed 35+ dead files: dashboard/ scaffold (397MB), API caches (blockscout/cg JSONs), redundant text intermediates, old handoff docs, research prompts, `firebase-debug.log`, `nul` artifact.
- **Version drift** — Fixed protocol.config.yaml version (was 0.7.1, now 0.8.0). Added default-run to Cargo.toml.

### Archived

- FID-041: Spread filter decimals
- FID-042: Permit2 signature missing
- FID-043: Trades reverting on-chain
- FID-044: Scanning under 100 pairs
- FID-045: Multi-chain 0x swap system (all 5 phases implemented)

## [0.7.1] — 2026-06-03

### Added

- **Token discovery** — Blockscot API integration for dynamic Arbitrum token discovery. Top 200 tokens by volume, filtered by $1M+ volume and 500+ holders.
- **Runtime token DB** — `TOKEN_EXTENSIONS` allows discovered addresses to be added at startup. `lookup_token()` checks extensions then static DB.
- **CoinGecko candle fallback** — `market_chart` endpoint gives 5m candles for 1 day (288 candles). SourceRouter tries Kraken first, CoinGecko second.
- **15s 0x API timeout** — `tokio::time::timeout(15s)` around `build_swap_tx()` prevents indefinite hangs.
- **Panic hook** — Logs `[PANIC] message at file.rs:123:45` instead of silent exit code 0xffffffff.

### Fixed

- **Tracing ANSI bleeding** — Disabled tracing colors, only `savant_log()` controls colors.
- **12h clock format** — EST timestamps now show `MM-DD-YYYY H:MM AM/PM`.
- **Pair highlighting** — `highlight_pairs()` skips already-bracketed pairs to avoid `[[BTC/USD]]`.
- **Module names** — `funding_rates` → `Funding Rates`, `onchain` → `On Chain`, `websocket` → `WebSocket`.
- **GoPlus spam** — Core assets (BTC, ETH, etc.) skip security check.
- **Vault verbosity** — Consolidated writing/done into single log line.
- **Watcher spam** — Removed per-pattern logging, only logs unique patterns.

### Archived

- FID-001: Inherited clippy lints
- FID-029: Port Kraken improvements (deferred to Preston)
- FID-030: 0x API hang
- FID-031: 0x API panic crash
- FID-032: Console color inconsistency
- FID-033: Uniform console output
- FID-034: ANSI color placement
- FID-035: Meme coin expansion
- FID-037: Console production ready
- FID-038: Arbitrum tokens + candle sources
- FID-039: Mass pair scanning
- FID-040: Full scan support

## [0.7.0] — 2026-06-03

### Added

- **FID-035 Phase 3** — Dual timeframe (5m→15m aggregation, no extra API calls). KV cache optimization (`cache_control: ephemeral` on system message for OpenRouter prefix caching). Emergency liquidation CLI (`--liquidate` flag). Retry queue for failed swaps (max 3 attempts).
- **FID-035 Phase 2** — GoPlus Security API (honeypot/tax detection for meme coins). Risk buckets (macro/legacy/meme) with correlation cap. ATR-based position sizing (`calculate_with_atr()`).
- **FID-035 Phase 1** — Meme coin expansion: 13 pairs (8 core + PEPE, SHIB, FLOKI, TURBO, MOG). Spread filter (30bps max). Price tolerance (0.5% drift). `highlight_pairs()` updated.
- **FID-034** — ANSI color placement fix.
- **FID-033** — Uniform console output via custom tracing Layer.
- **Merge strategy** — `dev/MERGE-STRATEGY.md` documenting cherry-pick approach.
- **Handoff docs** — `dev/HANDOFF-OTHER-DEV.md`, `dev/AGENT-PROMPT-PRESTON.md` (archived after consumption).
- **12h EST timestamps** — `SavantTimer` for tracing subscriber, `est_now()` shared function.
- **15s timeout on 0x API** — `tokio::time::timeout(15s)` around `build_swap_tx()` prevents indefinite API hangs.

### Fixed

- **ANSI color placement (FID-034)** — Color codes placed after text in format string, so colors applied to nothing. Fixed with named format params. Capitalized module names. Stripped Debug quotes from tracing messages.
- **Uniform console output (FID-033)** — Two formatting systems running (savant_log vs tracing). Created custom `SavantLayer` tracing Layer. All output now uses `[Savant Trading] [TIME] [ACTION] [RESULT]` format with consistent colors.
- **Panic hook** — Added `std::panic::set_hook` to log panics with file:line:col before crashing. Engine now shows `[PANIC] message at file.rs:123:45` instead of silent exit code 0xffffffff.
- **Pair name highlighting** — `highlight_pairs()` now works on both bare (`BTC/USD`) and already-bracketed (`[BTC/USD]`) pairs.
- **GREY_FG color** — Changed from `\x1b[37m` (light grey = white) to `\x1b[90m` (bright black = grey).
- **Vault level brightness** — Changed from `(GREY_DIM, GREY_DIM)` to `(GREY_DIM, GREY_FG)` — result text now readable.
- **Tracing color bleeding** — tracing's ANSI codes were bleeding into `savant_log()` output. Replaced `fmt()` subscriber with custom `SavantLayer`.
- **12h clock format** — `est_timestamp()` now returns `MM-DD-YYYY H:MM AM/PM` instead of 24h format.
- **Decision reasoning truncation** — Console log truncates reasoning to 100 chars (full text in vault/episodic).
- **Clippy warnings** — Fixed 3 warnings (empty line, empty format string, `and_then` → `map`).

### Changed

- **Version bump** — 0.6.0 → 0.7.0 (cherry-picked improvements from other dev's branch)
- **Tracing subscriber** — Uses `SavantTimer` for EST timestamps, `with_ansi(false)` to prevent color bleeding.
- **Preston branch created** — `origin/preston` for other dev's Kraken porting work.

### Archived

- **FID-026** — Sell/Close action handling (resolved → archived)
- **FID-027** — Swap execution hang (resolved → archived)
- **FID-028** — Console logging (resolved → archived)
- **FID-025** — NVIDIA NIM provider (verified → archived)
- **Handoff docs** — MERGE-STRATEGY, HANDOFF-OTHER-DEV, AGENT-PROMPT-PRESTON (consumed → archived)

## [0.6.0] — 2026-06-03

### Added

- **Enterprise console logging** — `src/core/console.rs` with single `savant_log()` function. Format: `[Savant Trading] [MM-DD-YYYY HH:mm] [ACTION] [RESULT]`. Cyan brand prefix, grey timestamps, white/green/orange/red results. EST timezone. 11 thin macros (`log_phase!`, `log_llm!`, `log_decision!`, `log_trade!`, `log_swap!`, `log_swap_ok!`, `log_swap_fail!`, `log_vault!`, `log_circuit!`, `log_warn!`).
- **3-retry logic for swap failures** — Retries on gas spike, nonce collision, network error, timeout. 2s delay between retries. Permanent failures (insufficient balance, invalid params) fail immediately.
- **Phantom position reconciliation** — DexTrader auto-clears positions on startup when balance drift > $1 or positions exist with zero completed trades. PaperTrader auto-reconciles when executor has no positions.
- **Position sizer logging** — Logs reason when returning None (stop/R:R invalid, entry/stop/tp1 values).
- **60s timeout on swap execution** — `tokio::time::timeout(60s)` around `place_order()` and `close_position()` prevents indefinite hangs.
- **50% gas buffer** — `maxFeePerGas = baseFee + baseFee/2 + priority` prevents `max fee per gas less than block base fee` errors on Arbitrum.
- **Kraken rebase prompt** — `dev/KRAKEN-REBASE-PROMPT.md` (385 lines) with ECHO Protocol boot sequence, file map, architecture docs, conflict zones, verification checklist.

### Fixed

- **Swap execution hang (FID-027)** — `place_order()` hung indefinitely when RPC call had no timeout. Added 60s timeout + retry logic.
- **Gas price too low** — 0x API returned stale gas estimate. Added 50% buffer to `maxFeePerGas`.
- **Console logging inconsistency (FID-028)** — Mix of `tracing` and `eprintln!`, no colors, no timestamps. Unified through `savant_log()`.
- **Clippy warnings** — Fixed 3 warnings: empty line after doc comment, empty format string, `and_then` → `map`.

### Changed

- **Version bump** — 0.5.0 → 0.6.0 (DEX execution fixes + console logging)
- **All version references updated** — Cargo.toml, VERSION, README

## [0.5.0] — 2026-06-03

### Fixed

- **FID-026: Sell/Close action handling (critical)** — Engine ignored AI's `Sell` and `Close` decisions. All non-Hold actions (including Sell, Close) fell through to `place_order()`, which always **opened** a new position. The agent could never exit positions except via stop-loss. On DEX, this caused on-chain swap failures when the wallet didn't own the base token (nonce stayed 0). On Kraken CEX, Sell opened a new short instead of closing an existing long. Fixed by adding action-aware branching: `Sell` → finds existing position for pair → `close_position()`; `Close` → finds ALL positions for pair → `close_position()` each; `Buy` → duplicate guard before `place_order()`. Creates proper `TradeRecord` with PnL for event bus. Backend-agnostic fix (both Kraken and DEX). Verified: 187/187 tests pass, clippy clean.

### Changed

- **Version bump** — 0.4.4 → 0.5.0 (critical sell logic fix = minor version bump)
- **All version references updated** — Cargo.toml, VERSION, README, protocol.config.yaml, main.rs, vault writer, scripts, HANDOFF.md, run scripts

## [0.4.4] — 2026-06-02

### Closed (FID-016, FID-017, FID-018, FID-019, FID-020, FID-021, FID-022, FID-023, FID-024 — archived 2026-06-02)

- **FID-016: Kraken Live Trading Execution Engine** (critical) — KrakenTrader implemented with 14/16 proposed fixes. Private API client with HMAC-SHA512 signing, order placement (market/limit/stop), balance sync, daily loss halt, kill switch, slippage alerts, Discord webhook notifications. See commit `d2ab69a`. Minor gaps: partial fill tracking, sandbox mode. Status: closed → archived.

- **FID-017: Multi-Exchange DEX Integration (0x + 1inch)** (critical) — DexTrader with ZeroXBackend and InchBackend. Enterprise token resolution (symbol fallback for non-EVM tokens). EIP-1559 signing with ECHO Law 6 compliance. Docs audit fixed 4 0x API issues + 2 1inch API issues + 2 EIP-1559 critical bugs. 176 tests, clippy clean. Status: verified → closed → archived.

- **FID-023: OpenRouter LLM Provider** (high) — Added OpenRouter as first-class AI provider alongside OpenGateway. Provider factory (`create_provider()`), `extra_headers` support, `OpenRouterConfig` struct with endpoint/model/api_key_env/referer/title, config validation, engine wiring in `run()` and `dry_run()`. Zero protocol changes — same OpenAI-compatible wire format. Status: verified → closed → archived.

- **FID-024: OpenRouter Model Env Var + Management Key System** (medium) — Added `OPENROUTER_MODEL` env var override for model switching without config edits. Created `OpenRouterManagementClient` with full CRUD (list/create/get/update/delete keys) via `/api/v1/keys`. `OpenRouterManagementConfig` struct, optional engine startup wiring for key usage monitoring. Status: verified → closed → archived.

- **FID-018: DEX Production Safety** (critical) — Stop-loss persistence + re-establishment, balance reconciliation via RPC (`eth_getBalance` + USDC `balanceOf`), crash recovery via JSON state persistence (`data/dex_state.json`). ETH gas halt at <0.002 ETH. State saved on every position mutation. Status: analyzed → closed → archived.

- **FID-019: DEX Test Infrastructure** (medium) — `ZeroXBackend::with_client()` and `InchBackend::with_client()` constructor injection. `with_client_and_url()` for wiremock routing. 12 hermetic tests covering happy path, 429, 500, malformed JSON, and missing fields for both backends. All pass without API keys or network. Status: analyzed → closed → archived.

- **FID-020: TUI Code Quality** (low) — Dynamic footer reads backend, mode, budget, model from `TuiSnapshot`. Version uses `env!("CARGO_PKG_VERSION")`. Drawdown thresholds derived from config values. Status: analyzed → closed → archived.

- **FID-021: has_actionable_signal Pre-Filter Review** (medium) — EMA spread threshold 0.1% → 0.5%. VWAP deviation check wired (was dead code). Volume spike gate added (`vol / volume_sma > 1.5`). Trending regime gate removed (redundant with ADX > 25). `current_price` and `current_volume` parameters passed to function. Status: analyzed → closed → archived.

- **FID-022: CLI TUI Overhaul** (high) — Multi-tab Ratatui terminal with 5-file modular architecture (`mod.rs`, `state.rs`, `tabs.rs`, `widgets.rs`, `keyboard.rs`). 10 tabs with keyboard navigation (0-9, Tab, arrows, PgUp/PgDn), search (`/`), help overlay (`?`/`F1`). Snapshot-based rendering from `SharedEngineData`. Status: analyzed → closed → archived.

---

**All 7 FIDs closed this session:** FID-018 (DEX Safety), FID-019 (DEX Tests), FID-020 (TUI Quality), FID-021 (Pre-filter), FID-022 (TUI Overhaul), FID-023 (OpenRouter Provider), FID-024 (OpenRouter Management).

### Added (FID-015 — Gemini Deep Research Optimization Overhaul)

- **Config Overhaul** — Corrected fee rate (0.26% → 0.40% taker), tightened risk (daily loss 20% → 10%, drawdown 40% → 20%), raised R:R (1.5 → 2.0), temperature 0.6 + top_p 0.95, knowledge budget 8K → 20K chars, candles 100 → 500, timeout 180s → 300s, max_tokens 131072 (128K), added 1d timeframe.
- **Maker Order Support** — `order_type` field (LIMIT/MARKET) in TradeDecision. Paper trader maker fee corrected to 0.25%.
- **Prompt Architecture** — XML-tagged prompts (`<identity>`, `<risk_constraints>`, `<strategy_knowledge>`, `<trading_rules>`, `<output_format>`, `<thinking>`). 5-step structured reasoning framework.
- **Session Liquidity Profiles** — 9 UTC-based sessions (Deep Asian 0.5x, US-EU Overlap 1.2x). Breakout confidence penalties for low-liquidity sessions.
- **Garman-Klass Volatility** — OHLC-based volatility estimator in indicator engine. More accurate than ATR for stop-loss sizing.
- **Isotonic Regression** — PAVA confidence calibrator. Maps raw LLM confidence to calibrated probability. Wired into training report.
- **Four-Factor Causal Attribution** — Loss classification (Setup/Process/Market/Trader) in training pipeline.
- **Historical Tick Data** — `data/historical.rs` fetches and caches 30 days of 5m candles from Kraken. `--historical` CLI flag.
- **JSON Repair** — Enhanced `repair_json_string()` with mid-word truncation detection, extra-text-after-JSON stripping, partial_extract fallback on repaired strings.

### Fixed

- **maker_fee_rate** — Paper trader maker fee corrected from 0.16% to 0.25% (actual Kraken base tier).
- **Garman-Klass in context** — Now displayed alongside ATR in market data section.

## [0.4.3] — 2026-06-01

### Added

- **SSE Streaming LLM Provider** — `chat_stream()` for real-time response streaming via Server-Sent Events. Keeps connection alive during long reasoning (mimo v2.5 pro can take 30-90s). Parses both `delta.content` and `delta.reasoning` fields. Streaming fallback to non-streaming on failure. 180s timeout.
- **Semantic Consolidation** — `memory/semantic.rs`: SQL aggregations against episodic memory to extract regime/session/pair edge matrix, conviction calibration, category edge. Populates `semantic_patterns` table. Rolling 90-day pattern decay. PF calculated from wins/losses ratio (not pnl).
- **Anti-Pattern Detection** — `memory/anti_pattern.rs`: SQL queries for conditions where win_rate < 30%. Category-level detection via `episode_market_context.condition_tags`. Narrative constraints for prompt injection. Auto-eviction when conditions recover.
- **Multi-Asset Correlation** — `risk/correlation.rs`: Rolling Pearson correlation matrix between active pairs. Effective position counting (correlated pairs count as 1.5-2x).
- **Portfolio Heat Tracking** — `risk/circuit_breaker.rs`: Total risk exposure / equity calculation. Blocks new trades when heat > 40%. Spread width halt at 50bps.
- **Dynamic Slippage** — `execution/paper.rs`: Slippage scales with ATR volatility and order book depth. `update_atr()` and `update_book_depth()` methods.
- **Maker-Fee Routing** — `execution/paper.rs`: If spread > fee differential (10bps), posts limit order at bid/ask instead of crossing with market. Saves 10bps per trade.
- **Knowledge Lifecycle** — `agent/knowledge.rs`: `utility_score` field on KnowledgeUnit (serde default 1.0). MMR scoring adjusted: `base * (1 + log2(utility))`. `save_utility_scores()` and `load_utility_scores()` for persistence.
- **SOUL.md Evolution** — `sandbox/feedback.rs`: `<!-- MUTABLE -->` markers on Section XIII+. `extract_mutable_sections()`, `apply_mutation_to_soul()`. Teacher LLM prompts (critique + GEPA mutation). `soul_versions` table for version control with auto-rollback.
- **Train/Val Split** — `sandbox/scenarios.rs`: `load_train_scenarios()` (first 40), `load_val_scenarios()` (last 20). `load_scenarios_by_difficulty()`, `worst_category()`.
- **WS Exponential Backoff** — `data/websocket.rs`: Reconnection uses exponential backoff (1s→30s cap) with ±20% jitter. CancelAllOrders signal after 3 consecutive failures.
- **BGeometrics On-Chain** — `insight/onchain.rs`: Replaced dead CoinMetrics/CoinGecko (403) with BGeometrics API. Free, no key, daily MVRV/SOPR/NUPL. Range validation on all values.
- **OKX Funding Primary** — `insight/funding_rates.rs`: OKX as primary funding source. Free, no key, no geo-block. Kraken as fallback with range validation (-2% to +2%).
- **RSS Cap + Source Diversity** — `insight/rss.rs`: `fetch_all_feeds_capped()` with per-feed 5s timeout, source diversity (top 2 per source), relevance scoring, cap enforcement.
- **Conditions Summary** — `insight/aggregator.rs`: `conditions_summary()` with SOUL.md thresholds. Actionable market assessments instead of raw data dump. RSS sentiment classification with negation handling.
- **TTL Cache** — `data/cache.rs`: TTL-based cache with LRU eviction. Graceful degradation (serve stale on API failure). Tests included.
- **Training Pipeline** — `engine.rs`: `run_training_batch()` with memory capture, Brier score, confidence distribution, category edge, auto-lessons, knowledge utility update, semantic consolidation, anti-pattern detection. All phases wrapped in error boundaries.
- **Training CLI** — `main.rs`: `savant --test --train` with filters (-c, -a, -n). `savant report --test` for full audit.
- **Training Report** — `monitor/training_report.rs`: P&L simulation, conviction calibration, confidence curve, category edge, anti-patterns, knowledge utility, lessons summary, semantic patterns, recent episodes.
- **SQLite Backup** — `engine.rs`: `backup_databases()` with rolling timestamped backups. Keeps last 7 copies.

### Fixed (FID-012, FID-013, FID-014 — closed 2026-06-01)

- **Confidence Floor** (FID-014) — `decision_parser.rs`: Trades with confidence < 40% automatically downgraded to Hold. Removes the 0-25% confidence bucket (18% accuracy). Highest-impact one-line fix.
- **Short Bias** (FID-014) — `scenarios.rs`: `derive_expected_action()` rebalanced. Capitulation buy signals boosted (2→3), moderate capitulation added (MVRV<1.2+SOPR<1.0), fear signals boosted (1→2), mild fear added (FG≤45). Buy threshold tightened to require `buy > sell`.
- **Vault Wiring** (FID-012, FID-013, FID-014) — `engine.rs`: Training batch now writes to vault. `project_decision()` per scenario, `project_risk_event()` for anti-patterns, `project_sandbox()` for batch report. 5 empty vault folders populated.
- **Training Default** (FID-014) — `engine.rs`, `main.rs`: `--train` defaults to 5 runs (was 20). `--train --full` for 20 runs. Help text updated.
- **Knowledge Selection Overhaul** — Indicator-derived conditions (RSI/ADX/EMA/volume → MarketCondition). Context tags use prefixed format. Unit cap (20). Scoring: tags×3, conditions×2, priority×1.
- **Knowledge Priority Migration** — All 2,959 units migrated from uniform 5 to differentiated 2-5. Risk catch-alls fixed. Execution units given conditions.
- **Random Scenario Generator** — `sandbox/scenarios.rs`: `generate_random_scenarios()` with weighted categories (weak areas get 3x). Expected actions derived from mock data. Every run is unique.
- **Protocol v0.1.0** — ECHO.md, protocol.config.yaml, templates, coding-standards synced from GitHub.
- **Training Workflow** — `docs/TRAINING-WORKFLOW.md`: Formalized closed-loop TRAIN → AUDIT → IDENTIFY → FIX → RETRAIN cycle.
- **/api/training** — Endpoint returning training metrics, config, episode count.

### Changed

- **Double-sleep bug fixed** — Engine had `time::sleep()` followed by `tokio::select!` with another sleep. Removed extra sleep.
- **Dry-run uses build_context()** — Same path as live engine. No custom prompt building.
- **Debug logging in engine** — Phase 1 and Phase 2 have debug-level logging.
- **Knowledge JSON files** — All 10 files migrated: priorities 2-5, risk catch-alls trimmed, execution units given conditions.
- **Max retries reduced** — 1 streaming + 1 fallback = 2 total per pair (was 3+1=4).
- **LLM timeout increased** — 180s (was 90s). Handles large prompts.
- **Dev folder restructured** — `findings` → `fids`, `archived` → `archive`, removed `baselines`/`plans`.
- **FID lifecycle** — Closed FIDs auto-archived per ECHO Protocol.
- **LEARNINGS.md** — Updated with session lessons.

### Fixed

- Context tag format mismatch — Tags were plain words, knowledge units use prefixed format.
- Risk catch-all conditions — 301/350 risk units always matched. Trimmed by content.
- Execution units invisible — 0 conditions → [Trending, Ranging].
- Kraken funding rate garbage — -45% per 8hr. Replaced with OKX (0.01%).
- Dead on-chain APIs — CoinMetrics/CoinGecko 403. Replaced with BGeometrics.
- RSS bloat — 333 items when config says 10. Cap enforced.
- Format string errors in action test output.
- Byte index panic on multi-byte UTF-8 chars in reasoning truncation.

### Tests

- 136 total tests (was 119)
- Cache: 5 tests
- Correlation: 4 tests
- Circuit breaker: 3 new tests (spread width)
- On-chain: 10 tests
- All tests passing, zero clippy warnings
- **Training Config** — `core/config.rs`: `TrainingConfig` struct with all training parameters. `config/default.toml`: `[training]` section with min_sample_size, failure_win_rate, max_portfolio_heat, backup_interval, utility_learning_rate, etc.
- **SQLite Backup** — `engine.rs`: `backup_databases()` function with rolling timestamped backups. Keeps last 7 copies. Called before training starts.
- **/api/training Endpoint** — Returns total episodes, semantic pattern count, Brier estimate, training config, SOUL.md version.
- **Persistent Training Pipeline** — `engine.rs`: `run_action_test()` and `run_training()` with memory capture, Brier score, confidence distribution, category edge, auto-lesson generation, progressive difficulty, convergence detection.
- **6th Prompt Layer Wiring** — Memory context now includes semantic patterns + anti-patterns alongside win rates and recent episodes.
- **Knowledge Selection Overhaul** — Indicator-derived conditions (RSI/ADX/EMA/volume → MarketCondition). Context tags use prefixed format matching knowledge vocabulary (`regime_subtype:trending` not `strong_trend`). Unit cap (20). Scoring rebalanced: tags × 3, conditions × 2, priority × 1.
- **Knowledge Priority Migration** — All 2,959 units migrated from uniform priority 5 to differentiated 2-5 based on content specificity. Risk catch-all conditions fixed. Execution units given conditions.

### Changed

- **Double-sleep bug fixed** — Engine had `time::sleep()` followed by `tokio::select!` with another sleep, doubling the tick interval. Removed the extra sleep.
- **Dry-run uses build_context()** — Dry-run now calls the exact same `build_context()` as the live engine. No more custom prompt building.
- **Debug logging in engine** — Phase 1 (candle fetch, order book, higher TF, pre-filter) and Phase 2 (LLM streaming) have debug-level logging for hang diagnosis.
- **Knowledge JSON files** — All 10 files migrated: priorities 2-5, risk catch-alls trimmed, execution units given [Trending, Ranging] conditions.

### Fixed

- Context tag format mismatch — Tags were plain words (`oversold`, `strong_trend`) but knowledge units use prefixed format (`regime_subtype:capitulation`, `setup_type:breakout`). Zero overlap meant zero tag matching. Fixed to use matching format.
- Risk catch-all conditions — 301/350 risk units had 5+ conditions (ExtremeFear, ExtremeGreed, HighVolatility, LowVolatility + more), always matching regardless of market state. Trimmed by content relevance.
- Execution units invisible — 282 execution units had zero conditions, never selected by the condition filter. Added [Trending, Ranging].
- Format string errors in action test output.

### Tests

- 119 total tests (was 112)
- Knowledge: tests updated for utility_score field
- All tests passing, zero clippy warnings

## [0.4.2] — 2026-05-31

### Added

- **Persistent memory system** — 4-tier architecture based on Gemini Deep Research (40 citations)
  - Episodic capture: SQLite WAL (6 tables, 7 indices), every decision stored with full market context
  - 6th prompt layer: Dynamic Memory Context injected into AI prompt (win rates, recent analogs, CUSUM alerts, operator rules)
  - Brier Score calibration: confidence penalty calculation from trade history
  - CUSUM control chart: edge decay detection per pair, persisted to SQLite
  - Experience Replay: generates lessons from HIGH conviction losses on startup
  - Operator rules: loaded from vault Lessons/ directory, injected as "OPERATOR RULES (override all AI reasoning)"
  - Progressive confidence: 1-25 trades=LOW, 26-50=MEDIUM, 50+=HIGH
  - TUI Memory panel: Brier Score, confidence cap, CUSUM status per pair, replay lesson count

- **Sandbox & stress testing system** — 4-phase "trading dojo" based on Gemini Deep Research (50 citations)
  - GARCH(1,1) OHLCV generator with configurable trend, volatility regime, market events
  - Order book simulator with bid/ask depth, imbalance calculation, slippage simulation
  - 50 curated scenarios across 11 categories (Trend Bull/Bear, Range, Volatility, Catalyst, Microstructure, Session, Correlation, Sentiment, On-Chain, Edge Cases)
  - 3-tier grading rubric: binary compliance, R:R scoring, reasoning quality
  - GEPA-style feedback loop: failure analysis, SOUL.md mutation proposals
  - Report card generator with category breakdown and critical failures
  - Vault/Sandbox/ integration for report output

- **SOUL.md persona** — 560-line enterprise trading identity
  - 12 sections: Identity, Creed, Cognitive Style, Communication, Emotional Architecture, Crypto Philosophy, Risk Management, Decision Framework, Operational Constraints, Operator Relationship, Technical Values, Identity Invariants
  - Resolves all knowledge base contradictions (circuit breakers, R:R, position sizing)
  - 10-point pre-trade checklist, 8 identity invariants, quick reference card

- **Knowledge base expansion** — 141 → 254 units, 11 → 22 JSON files
  - 11 new files: on-chain, risk math, derivatives, Wyckoff, macro, DeFi, backtesting, execution engineering, prop firms, psychology, compliance
  - 7 new MarketCondition variants

- **Deep research documents** — Memory system design (40 citations), Sandbox design (50 citations), SOUL design, 155 research questions

### Changed

- **All dead code wired** — EventBus, VaultWriter, VaultWatcher, StopLossCalculator, OrderBookManager, format_for_context, fetch_funding_multi
- **API + engine merged** — API spawns as background task alongside engine
- **Parallel AI evaluation** — all pairs evaluated simultaneously via JoinSet
- **Crypto-native sessions** — removed stock market "off-hours", all sessions tradeable
- **Decision parser hardened** — normalizes UPPERCASE/empty action/side fields

### Fixed

- Parse crash on markdown-wrapped LLM responses
- Drawdown kill switch was non-functional (update_equity never called)
- Paper trader could open unlimited positions (entry cost not deducted)
- Daily PnL cumulative from engine start (now resets at midnight UTC)
- Rate limiter permanent lockout (now resets every second)
- .gitignore was ignoring src/data/ (anchored to repo root)
- RSS UTF-8 crash on Bulgarian text (floor_char_boundary)
- .env never loaded (added dotenvy)

### Tests

- 112 total tests (was 13)
- Sandbox: 28 tests (generator, grader, harness, scenarios, feedback, report)
- Memory: 9 tests (calibration, cusum)
- Indicators: 13 tests
- Risk: 13 tests
- Paper: 6 tests
- Insight: 10 tests
- API: 4 tests
- Strategy: 4 tests
- Vault: 3 tests
- Agent: 7 tests

## [0.4.1] — 2026-05-30

### Added

- **On-chain analytics** — Live MVRV, SOPR, NVT from CoinMetrics (CoinGecko fallback). On-chain conditions (`MvrvExtreme`, `SoprReset`) injected into knowledge selection.
- **Unit tests** — 73 total tests across 11 modules (was 13). API, insight, vault, indicators, circuit breaker, position, regime, metrics, stop loss, onchain, websocket.
- **Kraken WebSocket v2 client** — `connect()` with auto-reconnection, `parse_message()` for ticker/book/trade channels.
- **Backtesting engine** — Candle replay via `evaluate_sync()`, Sharpe/drawdown/profit factor metrics, walk-forward optimization with cumulative balance.
- **TUI dashboard** — Ratatui 0.30 real-time terminal UI. Snapshot-based rendering (no `block_on` deadlock). Portfolio, positions, decisions, insight panels.
- **Knowledge base expansion** — 141 → 254 units, 11 → 22 JSON files. Added: on-chain, risk math, derivatives, Wyckoff, macro, DeFi, backtesting, execution engineering, prop firms, psychology, compliance.
- **7 new MarketCondition variants** — `LiquidityExpansion`, `LiquidityContraction`, `MvrvExtreme`, `SoprReset`, `OIDivergence`, `WyckoffSpring`, `DeltaDivergence`.
- **REST API** — All 13 endpoints return real engine state via `SharedEngineData`. Rate limiter (sliding window). Knowledge by topic endpoint (`/api/knowledge/:topic`).
- **Production safety** — Graceful shutdown (ctrl_c saves state). Block file mechanism (`savant.blocked`). State persistence (`data/paper_state.json`).
- **Scale-out execution** — TP1 → 50% close + break-even stop, TP2 → 60% of remainder, TP3 → full close.
- **Structure stop validation** — AI-proposed stops validated against 3x ATR bounds. `structure_stop()` fallback.
- **Session multiplier wired** — `position_size_multiplier()` applied to both AI and fallback paths. PreMarket session (5-7 AM EST, 0.7x).
- **Configurable volume profile** — `volume_profile_with_pct()` accepts `value_area_pct` parameter.
- **Deep research documents** — `docs/DEEP-RESEARCH-QUESTIONS.md`, `docs/Crypto Trading Knowledge Expansion Roadmap.md`, `docs/KNOWLEDGE-EXPANSION-EXECUTION.md`.

### Changed

- **All dead code wired** — EventBus, VaultWriter, VaultWatcher, StopLossCalculator, OrderBookManager, `format_for_context`, `fetch_funding_multi`.
- **API + engine merged** — API spawns as background task alongside engine. Both share `SharedEngineData`.
- **Insight aggregator** — `refresh_multi()` batches all pairs in single funding API call.
- **WebSocket refactored** — Removed unused `KrakenWebSocket` struct. Kept `connect()`, `parse_message()`, `create_channel()`.
- **Vault writer guards** — `project_trade()`, `project_decision()`, `project_portfolio()` check `config.enabled`.
- **Ratatui bumped** — 0.29 → 0.30 (fixes `lru` GHSA-rhfx-m35p-ff5j vulnerability).

### Fixed

- `parse_wrapped_json` test — Hold decisions with `entry_price: 0.0` no longer rejected.
- `vault/writer.rs` — Raw string `#` parsing error (Rust 2021 reserved prefix).
- `update_equity()` now called — Drawdown kill switch was non-functional.
- Entry cost + fee deducted from balance — Paper trader could open unlimited positions.
- `daily_pnl` resets at midnight UTC — Daily loss limit was cumulative.
- `Display` for `Side` — Logs show `LONG`/`SHORT` instead of `0`/`1`.
- Rate limiter resets every second — Was permanent lockout after 1000 requests.
- Duplicate `parse_timeframe` removed — Engine and main had different return types.
- RSS UTF-8 crash — `floor_char_boundary` for Bulgarian text.
- `.env` loading — Added `dotenvy::dotenv().ok()` to `main()`.

### Hygiene

- Moved misplaced files (yt.md, SESSION-SUMMARY.md, overview.jpg).
- Deleted Claude Code leftovers (dashboard/AGENTS.md, CLAUDE.md).
- Added `savant-vault/` to `.gitignore`.
- Removed duplicate knowledge files from `src/agent/knowledge/`.
- Removed GitHub Actions CI (billing issue).
- MSRV set to 1.91. Added `uuid` crate. Removed unused `tower` dep.
- Version aligned: Cargo.toml, README, protocol.config.yaml, CHANGELOG.

## [0.4.0] — 2026-05-30

### Added in 0.4.0

- **Glass House — Obsidian vault integration** — Bidirectional vault sync for transparent trading state
  - `VaultWriter` — Projects engine state into Obsidian vault as structured markdown
    - Trades/ — Daily trade logs with entry/exit/PnL
    - Decisions/ — AI decision logs with reasoning
    - Portfolio/ — Balance history, equity curve
    - Knowledge/ — Knowledge unit index
    - INDEX.md — Master index with wiki-links
  - `VaultWatcher` — Monitors vault for user edits
    - Lessons/ — Editable ground truth ingested by engine
    - Injection defense — Scans for 14 prompt injection patterns
    - Invisible unicode detection
  - `VaultConfig` — Configurable vault path, sync interval, max files
  - `.obsidian/appearance.json` — Dark theme, cyan accent (#00d5ff)

## [0.3.1] — 2026-05-30

### Added in 0.3.1

- **Session-aware trading** — Engine knows what trading session is active
  - Asian (7 PM - 2 AM EST): low volume, ranging, 0.5x position size
  - London (2 AM - 5 AM EST): high volume, reversals, 1.0x
  - New York (7 AM - 10 AM EST): highest volume, continuations, 1.0x
  - London/NY Overlap (8 AM - 10 AM EST): peak volume, 1.2x
  - Off-hours: 0.3x position size, avoid new entries
  - Kill zone detection: London, NY, Overlap are high-probability windows
  - Session context injected into AI prompt

### Changed in 0.3.1

- `core/session.rs` — New module: session detection, behavior, position size multiplier
- `context_builder.rs` — Session info added to AI user message

## [0.3.0] — 2026-05-30

### Added in 0.3.0

- **Dashboard UI** — Next.js 16 + React 19 + TypeScript 5, matching Savant design system
  - 11 pages: Overview, Transactions, AI Decisions, Portfolio, Insight, Knowledge, Risk, Session, Settings
  - CSS Modules with Savant design tokens (`#00d5ff` accent, glass morphism, JetBrains Mono + Inter)
  - 3-column layout: sidebar (280px), main content, right panel
  - Ambient background with radial gradients
  - Custom scrollbars, glass panels, accent glow effects
  - All pages build successfully (`npm run build`)

## [0.2.1] — 2026-05-30

### Added in 0.2.1

- **ECHO Protocol in system prompt** — Trading rules from ECHO.md + transcript-derived rules embedded in AI prompt
  - Sell into strength (80% at 10-20% gain)
  - 3 consecutive losses = stop for the day
  - Don't marry positions
  - Four-factor performance model
  - Session awareness (kill zones)
  - Compound strategy (risk profits on directional days)
- **Trade history in AI context** — Last 10 trades + performance summary injected into prompt
  - Win rate, average win/loss, profit factor
  - Individual trade details with entry/exit/PnL
  - Cold start: omit if no trades yet
- **Multi-pair insight refresh** — Insight now refreshes for ALL configured pairs, not just the first
- **Structured logging** — AI context logged with prompt chars, knowledge budget, pair, regime

### Changed in 0.2.1

- `FullContext` struct now includes `recent_trades: Option<&[TradeRecord]>`
- System prompt includes ECHO rules alongside strategy knowledge
- Knowledge section header: "From 11 Curated Transcripts"

## [0.2.0] — 2026-05-30

### Added in 0.2.0

- **Dry-run test mode** — `savant --dry-run` runs one AI decision cycle and prints full pipeline output
  - Market data (candles, indicators, regime)
  - Insight (Fear & Greed, BTC Dominance, funding rates, RSS)
  - Knowledge selection (conditions → matched units)
  - System prompt (composed with knowledge injection)
  - LLM response (raw JSON from mimo v2.5 pro)
  - Parsed decision (action, entry, stop, targets, confidence, reasoning)
- **REST API server** — `savant --api` starts axum server on localhost:8080
  - 13 endpoints: status, config, portfolio, positions, trades, decisions, insight, knowledge, risk, session, engine control
  - All responses use `{data, error, timestamp}` envelope
  - Localhost-only binding (no external access)
- **External knowledge loading** — `knowledge/` directory at project root
  - Engine loads from `knowledge/` first, falls back to embedded
  - 11 JSON files with 141 knowledge units
  - Editable without recompiling
- **Help command** — `savant --help` shows usage

### Changed in 0.2.0

- Version bumped to 0.2.0
- CLI now supports: `savant` (trade), `savant --dry-run`, `savant --api`, `savant report`, `savant --help`
- `context_builder.rs` — `determine_conditions` and `build_user_message` now have static versions for dry-run use
- `axum = "0.8"` and `tower-http = "0.6"` added for REST API

### Dependencies in 0.2.0

| Dependency  | Status | Notes            |
|-------------|--------|------------------|
| `axum`      | NEW    | REST API server  |
| `tower-http`| NEW    | CORS support     |

## [0.1.2] — 2026-05-30

### Added in 0.1.2

- **RSS News Pipeline** — 15 free RSS feeds parsed with `quick-xml`, scored by relevance to trading pairs
  - Crypto-native: Cointelegraph, CoinDesk, CryptoSlate, Decrypt, CryptoNews, CryptoPotato, CryptoBreaking
  - DeFi: The Defiant, SmartLiquidity
  - Institutional: Blockworks, Bitcoin Magazine, Ethereum 2.0
  - Mainstream: Yahoo Finance, CNBC (macro context)
  - Regional: KriptoNovini
- **Kraken Futures Integration** — Funding rates, open interest, mark prices from `futures.kraken.com` (free, no key)
- **Liquidation Risk Assessment** — Derived from futures data: mark/index spread, funding extremes, OI concentration
- **On-chain Data** — Block height, mempool size, 24h tx count from blockchain.info (free, no key)
- **CoinGecko Trending** — Trending coins with price changes, used as social sentiment proxy
- **API-KEYS.md** — Reference document with all endpoints, signup links, and env var names

### Changed in 0.1.2

- All insight modules now use **free APIs only** — no paid API keys required
- CoinGlass replaced with Kraken Futures (free, no geo-block)
- All insight sources enabled by default in config
- 15 RSS feeds (up from 0)

### Removed in 0.1.2

- CoinGlass API dependency (not free)
- CryptoQuant API dependency (not free — blockchain.info used instead)
- API key fields from InsightConfig (all sources free now)

### Fixed in 0.1.2

- FID-015 Perfection Loop: `quick-xml` dependency validated, all API endpoints verified
- Cargo.toml: `quick-xml = "0.37"` added for RSS/XML parsing

## [0.1.1] — 2026-05-30

### Changed in 0.1.1

- **Knowledge Base Expansion** — 88 → 141 knowledge units (+60%)
  - `ai_claude_bot.json`: 5 → 20 units (HMM math, feature engineering, walk-forward, circuit breakers, dashboard)
  - `tjr_smc.json`: 15 → 19 units (partial FVG fills, rejection candles, engulfing, trailing stop models)
  - `crypto_fcb.json`: 13 → 18 units (MACD, Bollinger Bands, Fibonacci, altcoin selection, market cycles)
  - `pradeep_ep.json`: 12 → 15 units (delayed EP, continuation EP, sector rotation)
  - `juvier_daytrading.json`: 6 → 12 units (displacement candles, breaker blocks, Asian range, London vs NY)
  - `warrior_trading.json`: 6 → 10 units (gap fill probability, short selling, morning vs afternoon)
  - `brian_jung.json`: 5 → 8 units (stablecoin strategy, ETF impact, global liquidity)
  - `ai_competition.json`: 3 → 10 units (all 15 bot strategies, risk tier analysis, evolution methodology)
  - `hybrid_scalping.json`: 4 → 8 units (multi-TF scalping, session-specific, fee optimization)
  - `cathie_wood.json`: 4 → 6 units (Tesla robotaxi, humanoid robots, demographics)
  - `fabio_amt.json`: 15 → 15 units (refined with more specific execution details)

### Fixed in 0.1.1

- FID-001, FID-002, FID-003 status corrected from "analyzed" to "fixed"
- FID-007 status set to "superseded" (absorbed into FID-013)
- All 13 FIDs have proper Perfection Loop audits

### Verified in 0.1.1

- Fear & Greed API: `GET https://api.alternative.me/fng/?limit=1` → `23 (Extreme Fear)` — free, no key
- CoinGecko API: `GET https://api.coingecko.com/api/v3/global` → `BTC.D: 57.44%` — free, no key
- Build: `cargo clippy -- -D warnings` zero warnings, `cargo fmt --check` clean

## [0.1.0] — 2026-05-30

### Added in 0.1.0

- **Agent Module** — AI-powered autonomous trading brain with mimo v2.5 pro via OpenGateway
  - Knowledge injection system — 11 curated transcripts processed into discrete knowledge units
  - Modular 5-layer system prompt composer (identity, risk, strategy, knowledge, output format)
  - OpenAI-compatible LLM provider with retry and rate limiting
  - Context builder — aggregates candles, indicators, insight, positions into LLM context
  - Decision parser — extracts structured TradeDecision from LLM JSON responses
  - Orchestrator — main decision loop with 3 autonomy levels (Suggest, Confirm, Autonomous)
  - Fallback mode — rule-based strategies activate if LLM fails 3 consecutive ticks
- **Insight Module** — Live market context from external data sources
  - Fear & Greed Index (alternative.me, free)
  - BTC Dominance and total market cap (CoinGecko, free)
  - Funding rates, open interest, long/short ratio (stub — CoinGlass)
  - Liquidation clusters (stub — CoinGlass)
  - Exchange inflow/outflow (stub — CryptoQuant)
  - News and social sentiment (stub — LunarCrush)
  - Unified MarketContext aggregator with graceful failure handling
- **Core Engine** — Existing rule-based trading engine
  - Kraken REST + WebSocket integration
  - Technical indicators (EMA, SMA, RSI, ATR, ADX, VWAP, Volume Profile)
  - Paper trading simulator with realistic fees (0.26% Kraken taker) and slippage
  - Risk management: position sizing, stop-loss, break-even, circuit breakers
  - Trade journal with SQLite persistence and equity curve tracking
  - CLI with trade, report, and backtest commands
- **Knowledge Base** — 11 curated transcripts
  - Scalping (Fabio Valentina — order flow, volume profile, AMT)
  - Strategy (Pradeep Bondi — episodic pivots, execution edges)
  - Day Trading (TJR — SMC, FVG, order blocks, liquidity sweeps)
  - Crypto (Full Course — sentiment, BTC dominance, alt season)
  - Crypto (Brian Jung — macro catalysts, halving cycle)
  - Day Trading (Juvier — kill zones, session trading)
  - Day Trading (Warrior Trading — 5 Pillars, pullback entry)
  - Scalping (Hybrid — Heikin Ashi + EMA, prop firms)
  - Macro (Cathie Wood — innovation platforms, Wright's Law)
  - AI Trading (Claude Code bot — HMM regimes, circuit breakers)
  - AI Trading (Competition — natural selection, multi-agent)
- **FID System** — 13 tracked findings with Perfection Loop audits
  - 3 fixed (ECHO violations, paper persistence, fee/slippage)
  - 8 pending (trailing stops, scale-out, FVG, backtesting, WebSocket, multi-TF, tests, rate limiting)
  - 1 superseded (sentiment → absorbed into FID-013)
  - 1 new (AI Agent Brain — critical architectural upgrade)
- **ECHO Protocol** — Universal agent bootstrap with 15 laws and Perfection Loop FSM
