# FID-098: Episodic Memory Feedback Loop — Model Never Learns From Outcomes

**ID:** FID-2026-0609-098
**Severity:** critical
**Status:** fixed
**Closed:** 2026-06-09 04:30
**Created:** 2026-06-09 03:55
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

The LLM makes trading decisions every 5 minutes but **never learns from the outcomes**. Episodes are captured at decision time with NULL outcome fields, and `EpisodicMemory::update_outcome()` is **never called** when trades close. The result: win rate queries always return 0 rows, recent episodes show "HELD"/"OPEN" for every decision, and the model has zero feedback on whether its BUY/SELL/PASS decisions were correct. The model is flying blind — making the same mistakes repeatedly.

---

## Evidence

From live engine logs (2026-06-09):
```
[Savant Trading] [06-09-2026 3:47 AM] [BUY] [LONG] [LINK/USD] | 52% | R:1.0
[Savant Trading] [06-09-2026 3:48 AM] [BUY REJECTED] LINK/USD — R:R=1.0, actual=0.6
[Savant Trading] [06-09-2026 3:51 AM] [PASS] [LONG] [LINK/USD] | 0% | R:0.0
[Savant Trading] [06-09-2026 3:57 AM] [PASS] [LONG] [LINK/USD] | 0% | R:0.0
[Savant Trading] [06-09-2026 4:05 AM] [PASS] [LONG] [LINK/USD] | 0% | R:0.0
```

LINK/USD has been PASSed on 20+ consecutive cycles while the model keeps generating detailed reasoning. The model has no memory that it already analyzed LINK 20 times today. The `## Dynamic Memory Context` section in its prompt is effectively empty.

From the Decisions vault (`savant-vault/Decisions/2026-06-09.md`): 4200+ lines of detailed LLM reasoning today, zero outcome feedback.

From the Trades vault (`savant-vault/Trades/2026-06-09.md`): 15+ closed trades with PnL data, never shown to model.

---

## Root Cause Analysis

### Broken Path 1: Episodic Memory Outcome Update

1. `capture_episode()` at `engine.rs:2337` creates episodes with `pnl: None, is_win: None, status: "executed"`
2. When trades close at `engine.rs:2700-2773`, `DecisionLog::update_outcome()` is called (line 2742) — this updates a JSON file
3. `EpisodicMemory::update_outcome()` is **never called** — zero call sites in entire codebase
4. Episodes remain with NULL outcomes forever
5. `win_rate_by_regime()` and `win_rate_by_pair()` filter on `status = 'closed'` → always return `None`
6. `recent_episodes()` returns snapshots with all outcomes as None → formatted as "HELD" or "OPEN"

### Broken Path 2: Decision Log Never Read Back

1. `DecisionLog::update_outcome()` writes to JSON at `engine.rs:2742`
2. `DecisionLog::context_for_pair()` exists to format outcomes for the prompt
3. `context_for_pair()` is **never called** from production code — only from unit tests
4. The decision log is write-only

### Broken Path 3: Vault Data Never Injected

1. `vault_writer.project_decision()` writes decisions to `savant-vault/Decisions/` (line 2273)
2. `vault_writer.project_trade()` writes trades to `savant-vault/Trades/` (line 3780)
3. No code reads from these vault directories back into the prompt
4. 4200+ lines of reasoning and 150+ trade records with PnL are invisible to the model

### What the Model Actually Sees

The `## Dynamic Memory Context` prompt section (injected at `context_builder.rs:495`) contains:
- Total closed trades count (from `total_trades()` — counts all statuses, so this grows)
- Win rate in current regime: **always None** (no closed episodes)
- Win rate on current pair: **always None**
- Recent analogs: **always "HELD" or "OPEN"** (no outcome data)
- Operator rules: loaded from vault at startup (static, not outcome-based)

---

## Impact

- **Model cannot learn** from its own decisions — same reasoning patterns repeat
- **Win rate statistics are always zero** — the model has no calibration signal
- **PASS decisions accumulate** with no counterfactual feedback ("you passed, here's what happened")
- **API costs burn** on 20+ identical PASS evaluations per pair per day with zero learning
- **Dead capital positions** (like LINK at -2.4%) persist because the model has no memory of repeatedly passing on a losing position
- **The "Dynamic Memory Context" prompt section is effectively dead weight** — consuming tokens with no information

---

## Proposed Solution

### Fix 1: Wire EpisodicMemory::update_outcome() on Trade Close

**Scope:** `src/engine.rs` — 3 trade close paths

Store the `episode_id` returned from `capture_episode()` keyed by pair+action. When a trade closes (via AI decision, stop-loss, or TP), call `mem.update_outcome(episode_id, pnl, pnl_pct, is_win, achieved_rr)`.

**Trade close paths:**
1. AI-initiated close at `engine.rs:2700-2773` (after `decision_log.update_outcome()`)
2. Stop-loss/TP close at `engine.rs:3684-3783` (in the `for trade in stop_result.closed` loop)
3. External close (reconciliation) at `engine.rs:3970-4018`

### Fix 2: Wire DecisionLog::context_for_pair() into Prompt

**Scope:** `src/agent/context_builder.rs`, `src/engine.rs`

After building the memory context, also call `decision_log.context_for_pair(pair)` and inject the result into the prompt. This gives the model direct feedback on recent same-pair decisions with outcomes.

### Fix 3: Inject Counterfactual PASS Feedback

**Scope:** `src/memory/context.rs`, `src/engine.rs`

For PASS decisions where a position exists, compute what would have happened if the model had acted. Include a brief "PASS outcome" line in the memory context:
- "PASS on LINK at 8.02 → price dropped to 7.80 (-2.7%) — correct pass"
- "PASS on AAVE at 64.90 → price dropped to 61.70 (-4.9%) — correct pass"

---

## Verification

1. `cargo clippy -- -D warnings` — zero warnings
2. `cargo test` — all pass
3. Law 4 grep: `update_outcome` — 3 production call sites (one per close path)
4. Law 4 grep: `context_for_pair` — 1 production call site in context_builder
5. Runtime: verify `## Dynamic Memory Context` section shows non-null win rates after first trade closes
6. Runtime: verify `recent_episodes` shows "WIN (+1.2R)" or "LOSS (-0.5R)" after trades close

---

## Files Changed

| File | Change |
|------|--------|
| `src/engine.rs` | Fix 1: Wire `update_outcome()` in 3 trade close paths; store episode_id mapping |
| `src/engine.rs` | Fix 3: Compute PASS counterfactuals |
| `src/agent/context_builder.rs` | Fix 2: Inject `decision_log.context_for_pair()` into prompt |
| `src/memory/context.rs` | Fix 3: Add PASS outcome formatting to `format_memory_prompt()` |
