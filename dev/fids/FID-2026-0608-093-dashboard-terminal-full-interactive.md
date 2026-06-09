# FID-2026-0608-093 — Dashboard Terminal: Agent Communication Bridge

**Created:** 2026-0608
**Severity:** high
**Status:** Open
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

### What's missing
| Gap | Impact |
|---|---|
| No command input path from dashboard to agent | Operator can't steer the agent |
| No agent response channel back to dashboard | Operator can't see command results |
| No command protocol defined | No structured way to express intent |
| No way to inject context mid-cycle | Agent can't receive real-time operator input |
| No override mechanism | Can't halt, pause, or redirect the agent |

### Confirmed: zero agent communication surface
- `AgentOrchestrator.evaluate()` is called only from the engine main loop
- No API endpoint accepts operator text and routes it to the agent
- No shared state field for pending operator commands
- The terminal WS handler explicitly discards all input (line 978)

---

## Reference: Kilocode CLI Architecture

Kilo's terminal system was reviewed for patterns. Key takeaways relevant to our use case:

### What we can reuse (patterns, not code)
- **Tabbed terminal UI**: Kilocode's `state.ts` manages per-context terminal lists with add/remove/reorder — we adapt this for our 2-tab model (logs + command)
- **Persistent xterm layer**: Kilocode renders all terminals with `opacity`/`pointer-events` stacking (never `display: none`) to keep canvases alive — we use the same pattern
- **ResizeObserver → debounced resize**: Standard pattern we already partially implement
- **Theme sync via CSS vars**: Kilocode reads `--vscode-terminal-*` custom properties — we define our own `--term-*` vars

### What we don't need
- **PTY/shell allocation**: We're not running a shell — we're sending structured commands to the agent
- **Binary frame protocol**: Our command channel is text-based JSON, not PTY byte streams
- **node-pty / portable-pty**: No pseudo-terminal needed
- **Multiple worktree contexts**: We have one engine, one agent

### What we should note for future use (Kilocode patterns)
- **SQLite storage**: Kilocode uses `drizzle-orm/bun-sqlite` + `drizzle-orm/node-sqlite` with a central `kilo.db` database. Schema defined via Drizzle ORM with migrations. Their storage system has both raw SQLite and ORM layers with a JSON-to-SQLite migration path. Savant already uses SQLite via `rusqlite` for the journal (`src/monitor/journal.rs`) — this pattern is consistent. If Savant ever needs structured storage beyond the journal (e.g., session history, command audit log), Kilocode's Drizzle-on-SQLite pattern is the reference.

---

## Design: Tabbed Terminal with Command Bridge

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Dashboard Terminal Panel                                │
│  ┌──────────┬──────────┬──────────────────────────────┐ │
│  │ [Logs]   │ [Cmd]    │  +  │  ← Tab bar              │ │
│  └──────────┴──────────┴──────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │                                                     │ │
│  │  xterm.js instance (shared, visibility-toggled)     │ │
│  │                                                     │ │
│  │  Logs tab:    read-only stream of engine output     │ │
│  │  Command tab: bidirectional command/response        │ │
│  │                                                     │ │
│  └─────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ $ ________________________________________ [send]   │ │
│  └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
         │                              │
         │  WS: /api/terminal/logs     │  WS: /api/terminal/command
         │  (broadcast subscribe)      │  (bidirectional JSON)
         ▼                              ▼
┌─────────────────┐          ┌──────────────────────┐
│  Log broadcast  │          │  Command handler     │
│  (existing)     │          │  (new)               │
└─────────────────┘          └──────────────────────┘
                                      │
                                      ▼
                              ┌──────────────────┐
                              │  Agent command   │
                              │  processor       │
                              │  (new)           │
                              └──────────────────┘
                                      │
                         ┌────────────┼────────────┐
                         ▼            ▼            ▼
                    ┌─────────┐ ┌─────────┐ ┌──────────┐
                    │ Override│ │ Context │ │  Query   │
                    │ actions │ │ inject  │ │  agent   │
                    └─────────┘ └─────────┘ └──────────┘
```

### Command Protocol (JSON over WebSocket)

**Client → Server (operator commands):**
```json
{"type": "cmd", "action": "override_close", "pair": "WETH/USD", "reason": "Manual exit"}
{"type": "cmd", "action": "override_stop", "pair": "WETH/USD", "stop_loss": 1800.0}
{"type": "cmd", "action": "inject_context", "message": "Fed just announced rate cut — bullish for crypto"}
{"type": "cmd", "action": "query", "message": "Why did you close LINK/USD?"}
{"type": "cmd", "action": "set_autonomy", "level": "confirm"}
{"type": "cmd", "action": "pause"}
{"type": "cmd", "action": "resume"}
{"type": "cmd", "action": "status"}
```

**Server → Client (agent responses):**
```json
{"type": "response", "ok": true, "message": "Closing WETH/USD position on next cycle"}
{"type": "response", "ok": true, "message": "Autonomy set to Confirm — trades will wait for approval"}
{"type": "response", "ok": false, "error": "No open position for LINK/USD"}
{"type": "response", "ok": true, "data": {"balance": 42.50, "equity": 45.20, "positions": 2}}
{"type": "agent_msg", "message": "I closed LINK/USD because RSI hit 75 and volume dried up"}
```

### WebSocket Protocol Specification

**Connection:**
- Logs: `ws://localhost:8080/api/terminal` (unchanged)
- Commands: `ws://localhost:8080/api/terminal/cmd` (new)

**Client → Server (operator commands):**
```json
{"type": "cmd", "action": "override_close", "pair": "WETH/USD", "reason": "Manual exit"}
{"type": "cmd", "action": "override_stop", "pair": "WETH/USD", "stop_loss": 1800.0}
{"type": "cmd", "action": "inject_context", "message": "Fed just announced rate cut — bullish for crypto"}
{"type": "cmd", "action": "query", "message": "Why did you close LINK/USD?"}
{"type": "cmd", "action": "explain", "pair": "LINK/USD"}
{"type": "cmd", "action": "set_autonomy", "level": "confirm"}
{"type": "cmd", "action": "pause"}
{"type": "cmd", "action": "resume"}
{"type": "cmd", "action": "status"}
```

**Server → Client (agent responses):**
```json
{"type": "response", "ok": true, "message": "Closing WETH/USD position on next cycle"}
{"type": "response", "ok": true, "message": "Autonomy set to Confirm — trades will wait for approval"}
{"type": "response", "ok": false, "error": "No open position for LINK/USD"}
{"type": "response", "ok": true, "data": {"balance": 42.50, "equity": 45.20, "positions": 2, "autonomy": "confirm", "hunt_mode": false, "price_staleness_secs": 12}}
{"type": "agent_msg", "message": "I closed LINK/USD because RSI hit 75 and volume dried up"}
```

**Error response (unknown command):**
```json
{"type": "response", "ok": false, "error": "Unknown command: foo"}
```

### Command Actions

| Action | Description | Implementation | Latency |
|---|---|---|---|
| `override_close` | Force-close a position by pair | Write to `close_overrides` shared state | Next cycle |
| `override_stop` | Set stop-loss for a position | Write to `stop_overrides` shared state | Next cycle |
| `inject_context` | Inject operator message into next LLM evaluation | Queue in shared state, prepend to next prompt with `[OPERATOR MESSAGE]` delimiter | Next cycle |
| `query` | Ask the agent a question | One-shot LLM call with current portfolio context + question | 2-5s (async) |
| `explain` | Explain last decision for a pair | Read decision log for pair + reasoning, return text | Immediate |
| `set_autonomy` | Change autonomy level (suggest/confirm/autonomous) | Write to `autonomy_override` shared state | Next cycle |
| `pause` | Halt all trading | Set `engine_running` AtomicBool to false | Next iteration |
| `resume` | Resume trading | Set `engine_running` AtomicBool to true | Immediate |
| `status` | Get current engine/agent state | Read shared state, return JSON | Immediate |

### Security Notes
- Command endpoint reuses existing `auth_middleware` (Bearer token via `SAVANT_API_TOKEN`)
- `pause`/`resume` and `set_autonomy` are privileged — same auth applies
- No shell/PTY access — commands are strictly validated JSON, bound to the 9 defined actions
- Bounded command queue (max 100) prevents memory exhaustion from rapid input

---

## Implementation Plan

### Phase 1: Backend Command Channel (Rust)

**1.1 Add command channel to `SharedEngineData`**
- `pending_commands: Arc<RwLock<VecDeque<OperatorCommand>>>` — bounded queue (max 100) of operator commands
- `autonomy_override: Arc<RwLock<Option<AutonomyLevel>>>` — runtime autonomy override (None = use config)
- Commands are drained at the START of each engine cycle (line ~1210 in engine.rs) — worst-case latency is one cycle interval (5 min). For urgent commands, add a wake-up signal.

**1.2 Define command types (`src/agent/commands.rs`)**
```rust
pub enum OperatorCommand {
    OverrideClose { pair: String, reason: String },
    OverrideStop { pair: String, stop_loss: f64 },
    InjectContext { message: String },
    Query { message: String },
    SetAutonomy { level: AutonomyLevel },
    Pause,
    Resume,
    Status,
    Explain { pair: String },
}
```

**1.3 Add command API endpoints (`src/api/mod.rs`)**
- `WS /api/terminal/cmd` — bidirectional JSON command channel (distinct from `/api/terminal` log stream)
- `POST /api/agent/command` — REST fallback for single commands (same handler, HTTP transport)
- Both endpoints require auth (existing `auth_middleware` applies)

**1.4 Add `command_log` table to the existing SQLite DB (`src/monitor/journal.rs`)**
- New table in the same `TradeJournal` database — reuses the existing `SqlitePool` and WAL mode setup
- Schema:
```sql
CREATE TABLE IF NOT EXISTS command_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL,
    payload TEXT NOT NULL,        -- JSON of the full command
    response TEXT,                -- JSON of the agent response
    ok INTEGER NOT NULL DEFAULT 1, -- 1 = success, 0 = error
    source TEXT NOT NULL DEFAULT 'terminal' -- 'terminal' or 'api'
)
CREATE INDEX IF NOT EXISTS idx_command_log_timestamp ON command_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_command_log_action ON command_log(action);
```
- Methods on `TradeJournal`:
  - `record_command(action, payload, response, ok, source)` — insert after each command is processed
  - `load_commands(limit)` — retrieve recent commands for audit/replay
- Pruning: delete entries older than 30 days (same pattern as `prune_old_snapshots`)

**1.5 Command handler (`src/api/command_handler.rs`)**
- Parse JSON commands, validate schema
- Route to appropriate handler
- Generate response JSON, send back over WS
- For `query`/`explain`: build a minimal system prompt + current portfolio context, call `LlmProvider.chat()`, stream response back
- For `override_close`/`override_stop`: write to existing shared state overrides (already consumed by engine)
- For `pause`/`resume`: toggle `engine_running` AtomicBool
- For `set_autonomy`: write to `autonomy_override` shared state
- For `status`: read shared state, serialize to JSON
- For unknown commands: return `{"type": "response", "ok": false, "error": "Unknown command: <action>"}`
- After every command: call `TradeJournal.record_command()` for audit trail

**1.6 Integrate into engine main loop (engine.rs ~line 1210)**
- At start of each cycle, before Phase 1, drain `pending_commands`
- For `InjectContext`: prepend to next LLM user message with delimiter: `"\n\n[OPERATOR MESSAGE]: {message}\n\n"`
- For `OverrideClose`/`OverrideStop`: already handled by existing override system
- For `SetAutonomy`: engine checks `autonomy_override` before using config autonomy
- Register `commands` module in `src/agent/mod.rs`: `pub mod commands;`
- Re-export `AutonomyLevel` from `src/agent/mod.rs` so `commands.rs` and `command_handler.rs` can reference it

### Phase 2: Frontend Tabbed Terminal (Next.js)

**2.1 Create `TerminalContainer.tsx` — tabbed container**
- Two tabs: "Logs" and "Command"
- Tab state: `activeTab: 'logs' | 'command'`
- Separate xterm.js instances per tab (Logs = read-only log stream, Command = bidirectional command channel)
- Input box only visible on Command tab

**2.2 Create `CommandInput.tsx` — command input with history**
- Text input with Enter to send
- Up/down arrow for command history (last 50 commands)
- Syntax highlighting for JSON responses
- Auto-scroll to bottom on new output
- Loading indicator while waiting for agent response (query/explain can take 2-5s)

**2.3 Rewrite `Terminal.tsx` → split into `LogTerminal.tsx` + `CommandTerminal.tsx`**
- `LogTerminal.tsx`: existing log streaming logic (broadcast subscribe → xterm write)
- `CommandTerminal.tsx`: new command WS connection, send/receive JSON, render formatted responses
- Both export a shared interface for `TerminalContainer` to manage

**2.4 Add CSS for tab bar**
- Minimal tab styling matching existing dashboard aesthetic
- Active tab indicator
- Connection status dot per tab (green = connected, red = disconnected, amber = reconnecting)

### Phase 3: Polish

**3.1 Command history persistence**
- Store last 50 commands in `localStorage`
- Restore on page reload

**3.2 Response formatting**
- Color-code: green for success, red for errors, yellow for warnings
- Timestamps on all responses
- Collapsible JSON data sections

**3.3 Keyboard shortcuts**
- `Ctrl+L` — switch to Logs tab
- `Ctrl+K` — switch to Command tab
- `Ctrl+C` — interrupt (send to engine, not just WS)

**3.4 Connection status indicator**
- Show WS connection state per tab
- Reconnect with exponential backoff

---

## Files to Create/Modify

### New files
| File | Purpose |
|---|---|
| `src/agent/commands.rs` | Command types, parsing, routing (registered as `pub mod commands` in `agent/mod.rs`) |
| `src/api/command_handler.rs` | WS + REST command endpoints |
| `dashboard/src/components/TerminalContainer.tsx` | Tabbed terminal container |
| `dashboard/src/components/LogTerminal.tsx` | Log stream terminal (refactored from current Terminal.tsx) |
| `dashboard/src/components/CommandTerminal.tsx` | Command channel terminal (new) |
| `dashboard/src/components/CommandInput.tsx` | Command input with history |
| `dashboard/src/hooks/useCommandChannel.ts` | Command WS connection, send/receive |

### Modified files
| File | Change |
|---|---|
| `src/agent/mod.rs` | Add `pub mod commands;`, re-export `AutonomyLevel` |
| `src/core/shared.rs` | Add `pending_commands: VecDeque`, `autonomy_override: Option<AutonomyLevel>` |
| `src/api/mod.rs` | Add `GET /api/terminal/cmd` WS route + `POST /api/agent/command` REST route |
| `src/monitor/journal.rs` | Add `command_log` table + `record_command()`/`load_commands()` methods |
| `dashboard/src/components/Terminal.tsx` | **Replaced** by LogTerminal.tsx + CommandTerminal.tsx |
| `dashboard/src/app/page.tsx` | Add TerminalContainer, tab bar |
| `dashboard/src/lib/api.ts` | Add `sendCommand()`, `onCommandResponse()` |

---

## Estimate

| Phase | Effort | Priority |
|---|---|---|
| Phase 1: Backend command channel | 2-3 sessions | P0 |
| Phase 2: Frontend tabbed terminal | 2 sessions | P0 |
| Phase 3: Polish | 1 session | P1 |

**Total:** 5 sessions

---

## Risks

1. **Command injection into LLM context**: Operator messages become part of the prompt — need clear delimiters so the agent distinguishes operator input from market data
2. **Autonomy level changes at runtime**: Changing from autonomous to confirm mid-trade could leave positions in limbo — need clear state machine
3. **Query latency**: Agent queries require a full LLM call (~2-5s) — need async response handling with loading indicator
4. **Command queue overflow**: If operator sends commands faster than engine can process — need bounded queue with backpressure

---

## References

- Current terminal WS: `src/api/mod.rs:928-980`
- Current terminal UI: `dashboard/src/components/Terminal.tsx`
- Shared state: `src/core/shared.rs`
- Agent orchestrator: `src/agent/orchestrator.rs`
- LLM provider: `src/agent/provider.rs`
- Kilocode terminal state (reference): `packages/kilo-vscode/webview-ui/agent-manager/terminal/state.ts`
- Kilocode terminal tab (reference): `packages/kilo-vscode/webview-ui/agent-manager/terminal/TerminalTab.tsx`
