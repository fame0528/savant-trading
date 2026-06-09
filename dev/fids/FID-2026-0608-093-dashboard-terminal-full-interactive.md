# FID-2026-0608-093 — Dashboard Terminal: Agent Communication Bridge

**Created:** 2026-06-08
**Severity:** high
**Status:** analyzed
**Scope:** Dashboard frontend, Rust API, agent command surface

---

## Problem Statement

The dashboard terminal is a **read-only log viewer**. User input is silently discarded (`api/mod.rs:978`). There is **zero communication channel** from the operator to the agent — no way to send commands, override decisions, inject context, or steer behavior in real-time. The agent runs fully autonomously with no human-in-the-loop interface.

This FID defines a **tabbed terminal** with two modes:
1. **Logs tab** (existing): read-only stream of engine output — unchanged
2. **Command tab** (new): bidirectional channel for sending commands to the agent and receiving responses

---

## Current State Audit

### What exists
| Component | File | Status |
|---|---|---|
| WS endpoint | `src/api/mod.rs:928-980` | Works for output, discards input |
| xterm.js component | `dashboard/src/components/Terminal.tsx` | Renders output, captures input, sends via WS |
| Log broadcast | `src/core/console.rs` | `tokio::broadcast` channel for tracing output |
| API routes | `src/api/mod.rs:118-145` | 15 REST routes + 1 WS route, no agent command surface |
| Agent orchestrator | `src/agent/orchestrator.rs` | `evaluate()` is internal only — no external call path |
| Shared state | `src/core/shared.rs` | RwLock data, no command channel |
| Close overrides | `shared.close_overrides` | Already exists — consumed by engine each cycle |
| Stop overrides | `shared.stop_overrides` | Already exists — consumed by engine each cycle |

### What's missing
| Gap | Impact |
|---|---|
| No command input path from dashboard to agent | Operator can't steer the agent |
| No agent response channel back to dashboard | Operator can't see command results |
| No command protocol defined | No structured way to express intent |
| No way to inject context mid-cycle | Agent can't receive real-time operator input |
| No override mechanism (beyond existing overrides) | Can't halt, pause, or redirect the agent |
| No autonomy level control | Can't switch between autonomous/confirm/suggest modes |

---

## Design: Tabbed Terminal with Command Bridge

### Architecture

```
Dashboard Terminal Panel
+----------+----------+--------------------+
| [Logs]   | [Cmd]    |                    |   <- Tab bar
+----------+----------+--------------------+
|                                              |
|  Logs tab:    xterm.js read-only stream      |
|  Command tab: plain input/output (no xterm)  |
|                                              |
+----------------------------------------------+
| $ __________________________________ [send]  |   <- Input (Cmd tab only)
+----------------------------------------------+
         |                              |
         |  WS: /api/terminal          |  WS: /api/terminal/cmd
         |  (broadcast subscribe)      |  (bidirectional JSON)
         v                              v
+-----------------+          +---------------------+
|  Log broadcast  |          |  Command handler    |
|  (existing)     |          |  (new)              |
+-----------------+          +---------------------+
                                      |
                         +------------+------------+
                         v            v            v
                    +---------+ +---------+ +---------+
                    | Override| | Context | |  Query  |
                    | actions | | inject  | |  agent  |
                    +---------+ +---------+ +---------+
```

### Autonomy Modes

```
                  set_autonomy("confirm")
    +--------------+<----------------------------+
    |  AUTONOMOUS  |                             |
    |  (Level 3)   |----+ set_autonomy("suggest")|
    +--------------+    |                        |
         |              v                        |
         |        +-----------+                  |
         +------->|  CONFIRM  |  approve --------+
                  |  (Level 2)|  reject ---------> back to confirm
                  +-----------+
                       |
                       | set_autonomy("suggest")
                       v
                  +-----------+
                  |  SUGGEST  |  approve --------> execute once
                  |  (Level 1)|                    (stays in suggest)
                  +-----------+
```

| Mode | Behavior |
|------|----------|
| **Autonomous** | Agent executes all actions independently (current behavior) |
| **Confirm** | Agent evaluates, generates pending action, pauses. Operator approves via `approve` command. |
| **Suggest** | Agent suggests actions but never executes. Operator must approve each via `approve`. |

**Transition rules:**
- autonomous→confirm: current cycle completes, next cycle starts in confirm mode
- confirm→autonomous: pending approvals discarded, next cycle runs autonomous
- Any→suggest: pending approvals preserved, agent stops executing
- Startup default: config value (currently autonomous)

### Command Protocol (JSON over WebSocket)

**Connection:**
- Logs: `ws://localhost:8080/api/terminal` (unchanged)
- Commands: `ws://localhost:8080/api/terminal/cmd` (new)

**Client → Server (12 commands):**

```json
{"type": "cmd", "action": "feedback", "pair": "WETH/USD", "verdict": "good", "note": "Good exit timing"}
{"type": "cmd", "action": "watch", "pair": "SOL/USD", "cycles": 6}
{"type": "cmd", "action": "undo"}
```

**Natural language support:**
The command handler accepts plain English in addition to structured JSON. A thin NLP layer maps natural language to the 13 structured commands:

| Natural Language | Maps To |
|-----------------|---------|
| `close weth` | `override_close` for WETH/USD |
| `tighten stops` | `override_stop` for all positions at 1.5x ATR |
| `what's happening with link` | `explain` for LINK/USD |
| `why did you close eth` | `query` with question |
| `pause trading` | `pause` |
| `set stop weth 1800` | `override_stop` for WETH/USD at 1800 |
| `hold for now` | `inject_context` with operator message |
| `undo` | `undo` |

Implementation: if the incoming message is not valid JSON, pass it through a keyword parser that extracts intent, pair, and value. Fallback: if no match, treat as `inject_context`.

**Server → Client (responses):**

```json
{"type": "response", "ok": true, "message": "Closing WETH/USD position on next cycle"}
{"type": "response", "ok": true, "message": "Autonomy set to Confirm — trades will wait for approval"}
{"type": "response", "ok": false, "error": "No open position for LINK/USD"}
{"type": "response", "ok": true, "data": {"balance": 42.50, "equity": 45.20, "positions": 2, "autonomy": "confirm", "hunt_mode": false, "price_staleness_secs": 12}}
{"type": "agent_msg", "message": "I closed LINK/USD because RSI hit 75 and volume dried up"}
{"type": "response", "ok": true, "data": {"pending_action": {"action": "BUY", "pair": "BTC/USD", "confidence": 0.72, "reasoning": "EMA bullish cross + volume spike"}}}
```

### Command Actions

| Action | Description | Implementation | Latency |
|---|---|---|---|
| `override_close` | Force-close a position by pair | Write to `close_overrides` shared state (existing) | Next cycle |
| `override_stop` | Set stop-loss for a position | Write to `stop_overrides` shared state (existing) | Next cycle |
| `inject_context` | Inject operator message into next LLM evaluation | Queue in `Vec<String>`, prepend to next prompt with `[OPERATOR CONTEXT]` delimiter | Next cycle |
| `query` | Ask the agent a question | One-shot LLM call with cached portfolio context + question | 2-5s (async) |
| `explain` | Explain last decision for a pair | Read decision log for pair + reasoning + triggers + audit | Immediate |
| `set_autonomy` | Change autonomy level | Write to `autonomy_override` shared state | Next cycle |
| `approve` | Approve pending action (confirm/suggest mode) | Execute the pending action from the approval queue | Immediate |
| `pause` | Halt all trading | Set `engine_running` AtomicBool to false | Next iteration |
| `resume` | Resume trading | Set `engine_running` AtomicBool to true | Immediate |
| `status` | Get current engine/agent state | Read shared state, return JSON | Immediate |
| `feedback` | Operator verdict on a trade | Write to episodic memory calibration system | Immediate |
| `watch` | Add pair to evaluation list | Add to `active_pairs` for N cycles (temporary) | Next cycle |
| `undo` | Reverse the last command | Pop command history stack, execute inverse | Immediate |

### Security Controls

**inject_context hardening:**
- Max length: 500 chars per message
- Rate limit: max 5 messages per cycle
- TTL: expire after 10 minutes (2 cycles)
- Queue: `Vec<String>` (accumulate all messages, don't overwrite)
- Sanitization: reject messages containing `"action":` or `"type": "cmd"` (prevent command injection)
- Delimiter: `[OPERATOR CONTEXT]` with explicit start/end markers

**query rate limiting:**
- Max 3 queries per 5 minutes
- 30s timeout per query
- Cached portfolio context (build once per cycle, reuse)

**General:**
- Command endpoint reuses existing `auth_middleware` (Bearer token)
- `pause`/`resume`, `set_autonomy`, `approve` are privileged — same auth applies
- No shell/PTY access — commands are strictly validated JSON, bound to the 13 defined actions
- Bounded command queue (max 100) with backpressure
- Command TTL: 10 minutes — expired commands discarded on drain

### Proactive Agent Notifications

The agent can push unsolicited messages to the command terminal — not just responses to commands. This gives the operator real-time visibility into agent reasoning.

**Server → Client (proactive):**
```json
{"type": "agent_notify", "severity": "info", "message": "WETH/USD: EMA bearish cross detected. Considering close."}
{"type": "agent_notify", "severity": "warning", "message": "LINK/USD: Stop distance 4.2x ATR exceeds threshold. Will adjust next cycle."}
{"type": "agent_notify", "severity": "critical", "message": "WETH/USD: Parabolic SAR triggered. Closing position."}
```

**When notifications fire:**
- Management trigger evaluation (before action)
- Stop adjustment decisions
- Close/open decisions
- Regime change detection
- Circuit breaker activation
- Any action that affects capital

**Implementation:** Add a `notify_tx: broadcast::Sender<AgentNotification>` channel. Engine sends notifications before executing actions. Command terminal subscribes and renders as formatted cards with severity coloring (info=blue, warning=yellow, critical=red).

### Undo Command

`undo` reverses the last command. Maintains a command history stack (last 10 commands with their inverse).

| Command | Undo Action |
|---------|-------------|
| `override_stop` | Restore previous stop value |
| `override_close` | Cancel pending close |
| `inject_context` | Remove last injected message from queue |
| `set_autonomy` | Restore previous autonomy level |
| `pause` | Resume |
| `resume` | Pause |
| `watch` | Remove pair from watch list |

**Implementation:**
- `command_history: VecDeque<CommandHistoryEntry>` in shared state (max 10)
- Each entry stores: original command + inverse command + timestamp
- `undo` pops the last entry and executes the inverse
- If history is empty, return error "Nothing to undo"

---

## Implementation Plan

### Phase 1: Backend Command Channel (Rust)

**1.1 Add command channel to `SharedEngineData`**
- `pending_commands: Arc<RwLock<VecDeque<OperatorCommand>>>` — bounded queue (max 100)
- `autonomy_override: Arc<RwLock<Option<AutonomyLevel>>>` — runtime override
- `pending_approval: Arc<RwLock<Option<PendingAction>>>` — confirm/suggest mode queue
- `inject_context_queue: Arc<RwLock<Vec<String>>>` — accumulated context messages
- `active_management_triggers: Arc<RwLock<HashMap<String, Vec<String>>>>` — pair → active triggers (for `status` command)
- `command_history: Arc<RwLock<VecDeque<CommandHistoryEntry>>>` — last 10 commands for undo (max 10)
- `notify_tx: broadcast::Sender<AgentNotification>` — proactive agent notification channel
- Commands drained at START of each engine cycle

**1.2 Define command types (`src/agent/commands.rs`)**
```rust
pub enum OperatorCommand {
    OverrideClose { pair: String, reason: String },
    OverrideStop { pair: String, stop_loss: f64 },
    InjectContext { message: String },
    Query { message: String },
    SetAutonomy { level: AutonomyLevel },
    Approve,
    Pause,
    Resume,
    Status,
    Explain { pair: String },
    Feedback { pair: String, verdict: String, note: String },
    Watch { pair: String, cycles: u32 },
    Undo,
}

pub struct CommandHistoryEntry {
    pub original: OperatorCommand,
    pub inverse: OperatorCommand,
    pub created_at: std::time::Instant,
}

pub enum AutonomyLevel {
    Autonomous, // Level 3: agent executes independently
    Confirm,    // Level 2: agent pauses for approval
    Suggest,    // Level 1: agent suggests only
}

pub struct PendingAction {
    pub action: TradeAction,
    pub pair: String,
    pub confidence: f64,
    pub reasoning: String,
    pub created_at: std::time::Instant,
}
```

**1.3 Add command API endpoints (`src/api/mod.rs`)**
- `WS /api/terminal/cmd` — bidirectional JSON command channel
- `POST /api/agent/command` — REST fallback
- Both require auth (existing `auth_middleware`)

**1.4 Add `command_log` table (`src/monitor/journal.rs`)**
```sql
CREATE TABLE IF NOT EXISTS command_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL,
    payload TEXT NOT NULL,
    response TEXT,
    ok INTEGER NOT NULL DEFAULT 1,
    source TEXT NOT NULL DEFAULT 'terminal'
);
CREATE INDEX IF NOT EXISTS idx_command_log_timestamp ON command_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_command_log_action ON command_log(action);
```
Methods: `record_command()`, `load_commands()`, prune after 30 days.

**1.5 Command handler (`src/api/command_handler.rs`)**
- Parse JSON, validate schema, route to handler
- inject_context: sanitize (reject if contains `"action":`), check length (500 max), check rate (5/cycle), push to `inject_context_queue`
- query: rate limit (3/5min), 30s timeout, cached portfolio context
- explain: read from decision log, return full chain (reasoning, confidence, triggers, audit, alternatives considered)
- override_close: write to `close_overrides` (existing), append reason to TradeRecord.notes
- override_stop: write to `stop_overrides` (existing)
- set_autonomy: write to `autonomy_override`
- approve: execute `pending_approval` action
- pause/resume: toggle `engine_running`
- status: read all shared state, return comprehensive JSON
- feedback: write to episodic memory calibration
- watch: add pair to temporary evaluation list
- After every command: `record_command()` for audit trail

**1.6 Engine integration (engine.rs ~line 1210)**
- At start of each cycle, drain `pending_commands`
- Expire commands older than 10 minutes
- For inject_context: collect all messages from queue, prepend to next LLM prompt with `[OPERATOR CONTEXT]` delimiter
- For override_close/override_stop: already handled by existing override system
- For set_autonomy: check `autonomy_override` before using config
- For confirm mode: if LLM produces an action, queue as `pending_approval` instead of executing
- Register `commands` module in `src/agent/mod.rs`

### Phase 2: Frontend Tabbed Terminal (Next.js)

**2.1 Create `TerminalContainer.tsx`**
- Two tabs: "Logs" and "Command"
- Tab state: `activeTab: 'logs' | 'command'`
- Logs tab: existing xterm.js (read-only)
- Command tab: plain `<input>` + scrollable `<div>` (NOT xterm.js)

**2.2 Create `CommandInput.tsx`**
- Text input with Enter to send
- Up/down arrow for command history (last 50, localStorage)
- Auto-scroll to bottom on new output
- Loading indicator for async responses

**2.3 Create `CommandOutput.tsx`**
- Scrollable div with formatted response cards
- Color-coded: green (success), red (error), yellow (warning), blue (agent_msg)
- Timestamps on all responses
- Collapsible JSON data sections

**2.4 Rewrite `Terminal.tsx` → split into `LogTerminal.tsx` + `CommandTerminal.tsx`**
- `LogTerminal.tsx`: existing log streaming logic
- `CommandTerminal.tsx`: new command WS, send/receive, render formatted responses
- Both export shared interface for `TerminalContainer`

**2.5 CSS for tab bar**
- Minimal styling matching existing dashboard aesthetic
- Active tab indicator
- Connection status dot per tab

### Phase 3: Polish

**3.1 Command history persistence**
- Last 50 commands in `localStorage`
- Restore on page reload

**3.2 Response formatting**
- Color-coded response cards
- Timestamps
- Collapsible JSON sections

**3.3 Keyboard shortcuts**
- `Ctrl+L` — Logs tab
- `Ctrl+K` — Command tab
- `Ctrl+C` — interrupt

**3.4 Connection status indicator**
- WS connection state per tab
- Reconnect with exponential backoff

---

## Files to Create/Modify

### New files
| File | Purpose |
|---|---|
| `src/agent/commands.rs` | Command types, parsing, routing |
| `src/api/command_handler.rs` | WS + REST command endpoints |
| `dashboard/src/components/TerminalContainer.tsx` | Tabbed terminal container |
| `dashboard/src/components/LogTerminal.tsx` | Log stream terminal (refactored) |
| `dashboard/src/components/CommandTerminal.tsx` | Command channel terminal |
| `dashboard/src/components/CommandInput.tsx` | Command input with history |
| `dashboard/src/components/CommandOutput.tsx` | Formatted response cards |
| `dashboard/src/hooks/useCommandChannel.ts` | Command WS connection |

### Modified files
| File | Change |
|---|---|
| `src/agent/mod.rs` | Add `pub mod commands;`, re-export `AutonomyLevel` |
| `src/core/shared.rs` | Add `pending_commands`, `autonomy_override`, `pending_approval`, `inject_context_queue`, `active_management_triggers` |
| `src/api/mod.rs` | Add WS + REST command routes |
| `src/monitor/journal.rs` | Add `command_log` table + methods |
| `dashboard/src/components/Terminal.tsx` | **Replaced** by LogTerminal + CommandTerminal |
| `dashboard/src/app/page.tsx` | Add TerminalContainer |
| `dashboard/src/lib/api.ts` | Add `sendCommand()`, `onCommandResponse()` |

---

## Testing Strategy

| Test | Coverage |
|------|----------|
| Unit: command parsing | All 12 commands, valid + invalid JSON |
| Unit: rate limiting | inject_context (5/cycle), query (3/5min) |
| Unit: TTL expiration | Commands older than 10 min discarded |
| Unit: inject_context sanitization | Reject messages containing command-like JSON |
| Integration: WS command channel | Send command, verify response over WS |
| Integration: override_close → engine | Command writes to close_overrides, engine consumes next cycle |
| Integration: set_autonomy state machine | Transition between autonomous/confirm/suggest |
| Manual: all 12 commands | Each command tested with real engine |
| Manual: query action | Mock LLM for unit tests, real LLM for integration |

---

## Estimate

| Phase | Effort | Priority |
|---|---|---|
| Phase 1: Backend command channel | 3-4 sessions | P0 |
| Phase 2: Frontend tabbed terminal | 2 sessions | P0 |
| Phase 3: Polish | 1 session | P1 |

**Total:** 6-7 sessions

---

## Risks

1. **Prompt injection via inject_context**: Mitigated by 500-char limit, sanitization, rate limiting, and explicit delimiter markers
2. **Autonomy state machine complexity**: confirm mode requires pending action queue and approval flow — most complex part
3. **Query latency**: 30s timeout + rate limiting mitigates, but still blocks a WS connection
4. **Command queue overflow**: Bounded at 100 with backpressure — rejects excess with error
5. **owl-alpha rate limits**: query action makes LLM calls — rate limit of 3/5min prevents exhaustion

---

## References

- Current terminal WS: `src/api/mod.rs:928-980`
- Current terminal UI: `dashboard/src/components/Terminal.tsx`
- Shared state: `src/core/shared.rs`
- Existing close overrides: `shared.close_overrides`
- Existing stop overrides: `shared.stop_overrides`
- Agent orchestrator: `src/agent/orchestrator.rs`
- LLM provider: `src/agent/provider.rs`
- Decision log: `src/agent/decision_log.rs`
