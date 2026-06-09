# FID: Dashboard Terminal — Advanced Features (Deferred)

**Filename:** `FID-2026-0608-095-terminal-advanced-features.md`
**ID:** FID-2026-0608-095
**Severity:** medium
**Status:** deferred
**Created:** 2026-06-08 23:55
**Author:** Kilo (ECHO Protocol v0.1.0, Level 3)

---

## Summary

Advanced features for the dashboard terminal command bridge, deferred until FID-093 (core command channel) is complete. These are polish and power-user features that enhance the command bridge but aren't required for the core functionality.

---

## Features

### 1. Command Confirmation for Dangerous Actions

`override_close` and `pause` require confirmation before executing.

**Protocol:**
```json
// Server sends confirmation request
{"type": "confirm_required", "id": "abc123", "message": "Close WETH/USD? This will sell 0.008 WETH at market."}
// Client responds
{"type": "confirm", "id": "abc123"}
// or
{"type": "cancel", "id": "abc123"}
```

**Implementation:** Command handler holds dangerous commands in a pending state. Sends confirmation request over WS. Waits for confirm/cancel (30s timeout = auto-cancel).

### 2. Command Aliases

Short aliases for power users:

| Alias | Expands To |
|-------|-----------|
| `c WETH` | `override_close` for WETH/USD |
| `s WETH 1800` | `override_stop` for WETH/USD at 1800 |
| `q "why link"` | `query` with question |
| `st` | `status` |
| `p` | `pause` |
| `r` | `resume` |
| `u` | `undo` |

**Implementation:** Alias expansion layer in the natural language parser. Check first word against alias table before NLP parsing.

### 3. Command Templates / Presets

Pre-defined multi-step command sequences:

| Template | Commands |
|----------|----------|
| "Tighten all stops to 1.5x ATR" | Multiple `override_stop` commands |
| "Close all losing positions" | Multiple `override_close` for negative PnL |
| "Set all stops to break-even" | Multiple `override_stop` at entry price |
| "Pause trading for 1 hour" | `pause` + schedule `resume` in 1h |

**Implementation:** Template engine that expands a template name into a sequence of commands. Templates defined in a JSON config file.

### 4. Command Analytics

Track operator behavior patterns:

- Most used commands (action frequency histogram)
- Most overridden pairs
- Override frequency over time (are overrides increasing?)
- Divergence between agent decisions and operator overrides
- Win rate of operator-overridden trades vs agent-autonomous trades

**Implementation:** Analytics queries on the `command_log` table. Dashboard widget showing command usage stats.

### 5. Command Auto-Complete

Tab completion for:
- Pair names (from config `trading.pairs`)
- Action names (from the 13 commands)
- Stop prices (from current position data)
- Common values (ATR, entry price, current price)

**Implementation:** Client-side trie populated from a `/api/terminal/autocomplete` endpoint that returns valid completions for the current input context.

### 6. Webhook Notifications

When the agent takes a significant action, optionally send a webhook to an external URL:

- Close/open decisions
- Stop loss hits
- Circuit breaker activation
- Management trigger firings

**Config:**
```toml
[notifications]
webhook_url = "https://hooks.slack.com/..."
webhook_events = ["close", "open", "sl_hit", "circuit_breaker"]
```

**Implementation:** Notification service that subscribes to the `notify_tx` broadcast channel (from FID-093) and sends HTTP POST to configured webhook URL.

### 7. Command Scheduling

Schedule commands for later execution:

```
at 15:00 close WETH
in 30m tighten stops
every 1h status
```

**Implementation:** Scheduler service with a `scheduled_commands` table in SQLite. Engine checks for due commands each cycle. Commands have: scheduled_time, command_payload, recurring (optional interval).

---

## Dependencies

All features depend on FID-093 (core command channel) being complete first.

| Feature | Depends On | Effort |
|---------|-----------|--------|
| Command confirmation | FID-093 Phase 1 | 1 session |
| Command aliases | FID-093 Phase 1 | 0.5 session |
| Command templates | FID-093 Phase 1 | 1 session |
| Command analytics | FID-093 Phase 1 + command_log table | 1 session |
| Auto-complete | FID-093 Phase 2 | 0.5 session |
| Webhook notifications | FID-093 proactive notifications | 1 session |
| Command scheduling | FID-093 Phase 1 | 1 session |

**Total:** 6 sessions (after FID-093 complete)
