# FID-2026-0609-106: Agent Conversation & Query System

**ID:** FID-2026-0609-106
**Created:** 2026-06-09 22:10
**Severity:** high
**Status:** open
**Scope:** src/agent/, src/api/, dashboard/src/components/

---

## Problem

The agent operates in a black box. The operator can send commands (FID-093) but cannot:

1. **Ask the agent questions** — "Why did you close WETH?", "What's your current strategy for LINK?"
2. **Have a freeform conversation** — back-and-forth dialogue with the agent about market conditions, decisions, or strategy
3. **Get explanations** — the `explain` command only returns raw decision log data, not a natural language explanation
4. **Inject context that gets a response** — `inject_context` queues a message for the next LLM evaluation but the operator never sees the agent's reaction

The `query` command is stubbed: `"LLM query not yet wired"`.

---

## Design

### 1. Wire the `query` Command (One-Shot LLM)

The operator types a question in the command terminal. The agent responds with a natural language answer using current portfolio context.

**Protocol:**
```
> Why did you close WETH/USD?
< I closed WETH/USD because: (1) RSI hit 75 (overbought), (2) volume dried up,
< (3) the position was down $0.34 (-1.47%). The thesis was invalidated by
< persistent lower highs in a ranging regime (ADX 18.7).
```

**Implementation:**
- Build a context snapshot from shared state (positions, recent decisions, market data)
- Make a one-shot LLM call with the question + context
- Return the response to the operator via the command WS
- Rate limit: 3 queries per 5 minutes (already in FID-093 design)
- Timeout: 30 seconds

### 2. Conversation Mode (Persistent Back-and-Forth)

A toggle in the command terminal that switches from "command mode" to "conversation mode". In conversation mode, the operator and agent have a persistent dialogue with memory.

**Protocol:**
```
> [conversation mode ON]
> What's your current market outlook?
< I'm seeing Extreme Fear (F&G 9) across the board. BTC is ranging $61,500-$62,000
< with weak volume. I'm not seeing any high-conviction long setups right now.
> What would make you bullish on BTC?
< I'd need to see: (1) ADX > 25 confirming a trend, (2) EMA fast crossing above
< EMA slow, (3) volume confirmation on a breakout above $62,200. Currently 0/3 triggers.
> Set a watch on BTC and let me know when you see 2/3 triggers
< Watching BTC/USD. I'll notify you when 2+ triggers align.
```

**Implementation:**
- Toggle button in CommandTerminal: "Command" / "Conversation"
- Conversation history stored in memory (last 20 messages)
- Each message includes the conversation context when calling the LLM
- Agent responses appear in the command terminal with the `agent_msg` type
- Conversation history cleared on engine restart (ephemeral)

### 3. Enhanced `explain` Command

Currently `explain` returns raw JSON from the decision log. Upgrade it to return a natural language explanation.

**Before:**
```json
{"pair": "WETH/USD", "action": "Close", "confidence": 0.62, "reasoning": "..."}
```

**After:**
```
I decided to close WETH/USD with 62% confidence. Here's why:

Market conditions: Ranging regime (ADX 18.7), RSI 37 (approaching oversold),
price at $1642.69 near recent troughs.

Position state: Entry at $1644.00, currently down $0.13 (-0.08%). Stop at $1512.00
is 8% below entry — too wide for the current volatility.

Decision: The position thesis (momentum long) was invalidated by persistent lower
highs in a ranging market. Zero momentum triggers fired. This is dead capital.

Alternatives considered: HOLD (rejected — 0/3 momentum triggers, ranging regime
suspends momentum entries, position is losing).
```

**Implementation:**
- After retrieving the decision log entry, make a one-shot LLM call to generate the explanation
- Include: market conditions, position state, decision reasoning, alternatives considered
- Same rate limit as query (3/5min shared bucket)

---

## Files to Modify

| File | Change |
|------|--------|
| `src/api/mod.rs` | Wire `query` command handler to make LLM call; upgrade `explain` to return NL |
| `src/agent/orchestrator.rs` | Add `query()` method for one-shot LLM calls with context |
| `dashboard/src/components/CommandTerminal.tsx` | Add conversation mode toggle, conversation history display |

## New Files

| File | Purpose |
|------|---------|
| `src/agent/conversation.rs` | Conversation context builder, LLM query handler |

## Estimate

| Component | Effort |
|-----------|--------|
| Wire `query` command | 1 session |
| Conversation mode | 1-2 sessions |
| Enhanced `explain` | 0.5 session |
| **Total** | 2-3 sessions |

## Dependencies

- FID-093 (command bridge) — ✅ complete
- FID-095 (advanced terminal features) — can run in parallel

---

## Risks

1. **LLM cost**: Each query/explain makes an LLM call. Mitigated by rate limiting (3/5min shared).
2. **Context size**: Building a full context snapshot for each query could be large. Mitigated by using a condensed summary (positions + recent decisions + market regime).
3. **Conversation memory**: Ephemeral (cleared on restart). Acceptable for v1 — can persist later.
