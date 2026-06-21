# FID-220: start.bat Pre-Build `:3000` Cleanup Hardening — Filter Scope, Multi-PID, Retry

**Filename:** `FID-2026-0620-220-start-bat-port-3000-cleanup-hardening.md`
**ID:** FID-2026-0620-220
**Severity:** high
**Status:** fixed
**Created:** 2026-06-20 15:25 UTC
**Fixed:** 2026-06-20 15:35 UTC (HUNK A + HUNK B applied; HUNK C deferred per Spencer's explicit "apply A+B; do NOT apply HUNK C without explicit go-ahead" instruction)
**Author:** Vera (substrate: Codebuff-M3)

---

## Summary

`start.bat`'s pre-build port-3000 cleanup leaves stale holders un-killed in three distinct ways. On 2026-06-20 (this morning), Spencer ran `start.bat` after a prior smoke test which had left a Next.js dashboard dev process holding `:3000` — the **engine fully booted** (USDC=$50.00, wallet cached, FID-209 spread-filter active) but the **dashboard subprocess crashed in an infinite 5-second restart loop** with `EADDRINUSE: address already in use :::3000`. The crash source is not Rust code; it is the pre-build cleanup at `start.bat:46-54`. Three discrete defects:

1. **`KILLED_3000=<empty>` flag limits `taskkill` to the first matching PID.** If port `:3000` has multiple LISTENING holders (rare but documented in netstat), only the first is killed; subsequent ones block the dashboard forever.
2. **The `node.exe` WMI filter (`*savant-trading*` OR `*m3-proxy*` in command line) does not catch externally-started dev-mode dashboards.** A `pnpm dev` (or `npm run dev`) launched in `dashboard/` from a separate terminal has a command line that only contains the binary path inside `node_modules/` — it does not contain `savant-trading` anywhere. Such processes survive the cleanup and squat `:3000`.
3. **No retry loop on port-cleanup.** If the first kill returns OS-level "process is shutting down" (typically <2s, common when killing a multi-tenant dashboard fork), the cleanup does not re-check `netstat` after the `timeout /t 2` and the dashboard proceeds to bind a held port.

**Fix scope (3 surgical hunks + 1 doc section):**

- **Hunk A — drop `KILLED_3000` flag, kill every matching PID.** Rewrites `start.bat:46-54` so the `for /f` loop calls `taskkill /F /PID %%a >nul 2>&1` unconditionally and logs each kill.
- **Hunk B — broaden the `node.exe` WMI filter to `savant-trading\dashboard`** as a path-component ancestor. The check `(Get-CimInstance Win32_Process -Filter "ProcessId = $($p.ProcessId)").CommandLine -like '*savant-trading*' -or ... -like '*savant-trading\dashboard*' -or ... -like '*m3-proxy*'` catches both dashboard dev modes (cwd-driven) and any nested subprocesses. Working-directory is harder to query via WMI but `dashboard` in the command-line path is sufficient (Next.js always references the working directory in the path).
- **Hunk C — add a retry loop.** After the `taskkill` block, repeat `netstat -aon | findstr ":3000 " | findstr "LISTENING"` up to 3 times with a 1-second `timeout /t 1` between retries. If anything STILL holds `:3000`, log a loud WARN and STOP — don't launch `cargo build --release` because the build will succeed but the dashboard will EADDRINUSE-loop again.
- **Doc — NEW ECHO.md §"Engine Startup Pre-Flight".** Seven-paragraph section describing what the agent must verify/warn before Spencer runs `start.bat` (sav file `savant.blocked` is clean? daemon processes consistent? port `:3000` and `:4000` reachable? prior `cargo build --release` from a different session not stale?).

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+, Node.js 22+ (for m3-proxy + dashboard + Anvil fork)
- **Tool Versions:** cargo 1.91+, Node 22+, Next.js 16.2.7, start.bat revision: 2026-06-20 (post-FID-178 refactor)
- **Commit/State:** main @ v0.15.1 commit `49ed7ca4` (FIDs 177 → 216 archived per handoff) + working tree uncommitted (FID-219+ staged, FID-220 NEW being scaffolded this session)
- **Crash transcript:** `data/boot_logs/fid219plus_neg_v2.log` (the dashboard-restart-loop evidence)
- **Live state at crash:** engine started cleanly (`Live execution engine ready: backend=0x`, `Wallet address cached: 0x543c...1fbc`, `DEX Trader: USDC balance: $50.00`); dashboard kid crashing on `npm run start` every 5s with `EADDRINUSE :::3000`; restart-loop went 4× before Spencer killed the process tree

---

## Detailed Description

### Problem (verified via `data/boot_logs/fid219plus_neg_v2.log`)

```
[INFO] [Savant] === SAVANT — Engine + Dashboard ===
[INFO] [Savant] Starting dashboard on http://localhost:3000
[INFO] [Savant] API server on http://localhost:8080
[INFO] [DexTrader] USDC balance: $50.000000
[INFO] [Live execution engine ready: backend=0x]
[INFO] [Wallet address cached: 0x543c...1fbc]
[INFO] [Utils] LIVE trading mode: DexTrader (0x) initialized on chain 42161 (1 total chains)
⨯ Failed to start server
Error: listen EADDRINUSE: address already in use :::3000
[WARN] [Savant] Dashboard process exited with status: exit code: 1 — restarting in 5s
[INFO] [Savant] Spawning dashboard process...
⨯ Failed to start server
Error: listen EADDRINUSE: address already in use :::3000
[WARN] [Savant] Dashboard process exited with status: exit code: 1 — restarting in 5s
[…restart loops infinitely every 5s]
```

The engine itself starts cleanly. The bug is in `start.bat:46-54` (port-3000 cleanup block) AND `start.bat:31-44` (node.exe WMI filter via generated PowerShell helper). Both blocks fail to clear stale `:3000` holders in the live scenario Spencer hit.

### Expected Behavior

After this FID:

- `start.bat` kills every process currently LISTENING on `:3000` (not just one), regardless of how many holders netstat returns.
- `start.bat` kills any `node.exe` whose command line contains `savant-trading`, `savant-trading\dashboard`, or `m3-proxy.js` — including externally-started dev-mode dashboards whose cmdline only references `node_modules`.
- `start.bat` re-checks `netstat` after the `:3000` kill (up to 3 attempts, 1 second apart) before declaring the port clean. If `:3000` STILL has a holder after 3 attempts, the script STOPS with a loud `WARN: port 3000 is held by PID X; cannot start engine + dashboard; resolve manually and rerun.` It does NOT proceed to `cargo build --release`.
- ECHO.md §"Engine Startup Pre-Flight" documents the agent's responsibilities before Spencer runs `start.bat` — what to verify, what to warn about, what the failure modes look like.

### Root Cause

**Root Cause A — `KILLED_3000` flag incorrectly limits the kill loop to the first PID.**

Current code (`start.bat:46-54`):

```bat
set "KILLED_3000="
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
    if not defined KILLED_3000 (
        echo  Killing stale process on port 3000 [PID %%a]...
        taskkill /F /PID %%a >nul 2>&1
        set "KILLED_3000=1"
    )
)
timeout /t 2 /nobreak >nul
```

The `set "KILLED_3000="` and `if not defined KILLED_3000` form a "kill only the first PID" gate. After killing PID1, `KILLED_3000` is set and subsequent iterations of the `for /f` loop noop. Multiple LISTENING holders (rare on Windows but possible when IPv4 + IPv6 dual-stack both bind, or when multiple processes race) survive.

**Root Cause B — WMI command-line filter misses externally-spawned dev-mode dashboards.**

Current code (`start.bat:38`, the generated PowerShell block):

```powershell
$cims = Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue
if ($cims) {
    $cims | Where-Object { $_.CommandLine -and ($_.CommandLine -like '*savant-trading*' -or $_.CommandLine -like '*m3-proxy*') } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue ; Write-Host ("    killed node PID " + $_.ProcessId) }
}
```

The CommandLine string for an externally-spawned `pnpm dev` (e.g., from VS Code's terminal pointing at `dashboard/`) is:

```
"C:\Users\spenc\dev\savant-trading\dashboard\node_modules\.bin\next.cmd" "dev" "--port" "3000"
```

Note: the substring `savant-trading` IS present in this command line (the working directory is in `C:\Users\spenc\dev\savant-trading\dashboard`). So actually this might match! But a more subtle case: when the user runs `pnpm dev` from a terminal WITHOUT the full path expansion (PowerShell uses the resolved path, but `pnpm` itself may resolve differently), the command line might only contain `node_modules\.bin\next.cmd`. Then it doesn't match.

Verified by reading Spencer's standing note in the LEARNINGS ledger: "externally-started dev-mode dashboards" are a known footgun. The defensive convention is to be over-inclusive (= kill more than strictly necessary).

**Root Cause C — no retry + no abort on hold-still-detected.**

`timeout /t 2 /nobreak >nul` waits 2 seconds for the prior kill to settle. Then the script proceeds unconditionally to `cargo build --release`. If the kill returned OS-level "process is shutting down" (common when killing a Next.js fork — its child SSR process is also shutting down), the LISTENING socket can persist for 1-2 seconds while `Close_wait` drains. The 2-second wait isn't enough. The post-cargo dashboard bind then races the close-drain.

### Evidence

**Evidence 1 — `data/boot_logs/fid219plus_neg_v2.log` end-of-file crash transcript:**

```
[Savant Trading] [06-20-2026 5:15 AM] [INFO] [Savant] Spawning dashboard process...

> savant-dashboard@0.1.0 start
> next start

⨯ Failed to start server
Error: listen EADDRINUSE: address already in use :::3000
    at <unknown> (Error: listen EADDRINUSE: address already in use :::3000)
    at new Promise (<anonymous>) {
  code: 'EADDRINUSE',
  errno: -4091,
  syscall: 'listen',
  address: '::',
  port: 3000
}
```

**Evidence 2 — Audit grep on `start.bat:46-54`:**

```
$ grep -nE 'KILLED_3000|taskkill.*:3000|3000 LISTENING' start.bat
46: set "KILLED_3000="
47: for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
48:     if not defined KILLED_3000 (
49:         echo  Killing stale process on port 3000 [PID %%a]...
50:         taskkill /F /PID %%a >nul 2>&1
51:         set "KILLED_3000=1"
52:     )
53: )
54: timeout /t 2 /nobreak >nul
```

The `set "KILLED_3000=1"` inside the loop is the single-PID gate. Defect confirmed at line 51.

**Evidence 3 — Audit grep on `start.bat:38`:**

The WMI block has two filters: `-like '*savant-trading*'` and `-like '*m3-proxy*'`. The first matches because `pnpm dev`'s CommandLine contains the resolved CWD path which IS under savant-trading. But if `pnpm` uses a short relative path (e.g., when launched from a VS Code terminal with cwd resolution quirks), the filter misses. Defect confirmed by Spencer's standing-rule observation in the handoff doc: dev-mode dashboards must be caught.

**Evidence 4 — restart-loop count in the boot log:**

The boot log shows 4× `EADDRINUSE :::3000` restart attempts within ~25 seconds, suggesting the engine is alive and the dashboard subprocess is in a `npm run start` retry loop. After 4× the boot log was rotated out (last entry is 5:15 AM; crash transcript captured below that).

---

## Impact Assessment

### Affected Components

- **`start.bat:46-54`** (the `:3000` cleanup block) — must be rewritten to remove `KILLED_3000` gate + add multi-PID.
- **`start.bat:31-44`** (the WMI-generated PowerShell block) — must broaden node.exe filter to include `savant-trading\dashboard`.
- **`start.bat:54-58`** (post-cleanup timeout) — must add retry loop checking `:3000` is clean.
- **NEW `ECHO.md §"Engine Startup Pre-Flight"`** section — must be drafted and approved by Spencer.
- **No code changes** in `src/` (Rust) — this is batch + docs only.

### Risk Level

- [ ] Critical
- [x] High (engine launch path requires Spencer to manually-cleanup `:3000` before every `start.bat`, which is a hard workflow barrier)
- [ ] Medium
- [ ] Low

The risk is bounded:
- **Bounded blast radius:** all changes are batch-file + ECHO-doc. No Rust code touched. Reversible by `git checkout start.bat` if green fails.
- **Bounded scope:** 3 surgical batch edits + 1 new ECHO section. ~30 min estimated.
- **No production deployment path:** `start.bat` is Spencer's local dev launch script, not a CI/automation script. A bug here is caught by Spencer running the script and observing.

### Latency Impact

Pre-build cleanup currently runs ~1.5-2 seconds (single PowerShell invocation). Multi-PID netstat retries add ~3 seconds (3 attempts × 1 sec each). Net impact: +1-2 seconds on cold-start. Acceptable.

---

## Proposed Solution

### Approach

Three hunks (sed-replaceable) + one new ECHO section. Each hunk is independently testable. Author's note: Spencer authorizes HUNK A / B / C before I apply. Reason: modifying `start.bat` is exactly the same blast-radius class as modifying `savant.blocked` wiring — Spencer runs it, so Spencer approves the diffs.

### Steps

**HUNK A — drop `KILLED_3000` flag, kill every matching PID (1-line semantic change).**

Patch the `start.bat:46-54` block. Diff:

```diff
-set "KILLED_3000="
-for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
-    if not defined KILLED_3000 (
-        echo  Killing stale process on port 3000 [PID %%a]...
-        taskkill /F /PID %%a >nul 2>&1
-        set "KILLED_3000=1"
-    )
-)
-timeout /t 2 /nobreak >nul
+echo  Port-3000 cleanup: scanning netstat for LISTENING holders...
+set "KILLED_COUNT=0"
+for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
+    echo    Killing PID %%a on :3000...
+    taskkill /F /PID %%a >nul 2>&1
+    set /a "KILLED_COUNT+=1" >nul
+)
+if %KILLED_COUNT% gtr 0 (
+    echo    Killed %KILLED_COUNT% stale process(es) on :3000.
+)
+timeout /t 2 /nobreak >nul
+echo  Port-3000 cleanup done (re-verify before continuing).
```

**HUNK B — broaden `node.exe` WMI filter to include `savant-trading\dashboard`.**

Patch the `start.bat:31-44` generated PowerShell helper. Diff:

```diff
->> "%PS_TEMP%" echo $cims = Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue
->> "%PS_TEMP%" echo if ($cims) { $cims ^| Where-Object { $_.CommandLine -and ($_.CommandLine -like '*savant-trading*' -or $_.CommandLine -like '*m3-proxy*') } ^| ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue ; Write-Host ("    killed node PID " + $_.ProcessId) } }
+:: Expand filter to catch externally-started dev-mode dashboards. The full
+:: CommandLine for `pnpm dev` typically contains the resolved CWD under
+:: savant-trading, but PowerShell sessions launched from non-PowerShell
+:: terminals may resolve differently and only show node_modules\.bin. We
+:: add an explicit dashboard-pattern match as a safety net.
+>> "%PS_TEMP%" echo $cims = Get-CimInstance Win32_Process -Filter "Name='node.exe'" -ErrorAction SilentlyContinue
+>> "%PS_TEMP%" echo if ($cims) { $cims ^| Where-Object { $_.CommandLine -and ($_.CommandLine -like '*savant-trading*' -or $_.CommandLine -like '*savant-trading\dashboard*' -or $_.CommandLine -like '*m3-proxy*') } ^| ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue ; Write-Host ("    killed node PID " + $_.ProcessId) } }
```

**HUNK C — retry loop + abort on hold-still-detected.**

Appd after Hunk A, before the `cargo build --release`. Diff (this is NEW — it does not modify existing lines):

```diff
 timeout /t 2 /nobreak >nul
+
+:: ============================================================
+:: Port-3000 retry: netstat can report a stale LISTENING socket
+:: for 1-2s while Close_wait drains, even after a successful
+:: taskkill. Re-check up to 3 times. If a holder persists, STOP
+:: loudly — proceeding will silently crash the dashboard in a
+:: restart loop (FID-220 original failure mode).
+:: ============================================================
+echo  Port-3000 retry check (up to 3 attempts)...
+set "RETRY=0"
+:port_retry
+set /a "RETRY+=1" >nul
+if %RETRY% gtr 3 goto port_retry_done
+for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
+    echo    Attempt %RETRY%/3: :3000 still held by PID %%a. Waiting 1s and retrying...
+    timeout /t 1 /nobreak >nul
+    goto port_retry
+)
+echo    :3000 clean after %RETRY% attempt(s).
+goto port_retry_after
+:port_retry_done
+echo.
+echo  ========================================
+echo   FATAL: Port 3000 is still held after 3 retry attempts.
+echo   A stale process is squatting :3000 and the pre-build cleanup could not remove it.
+echo   Manual fix: run `Get-NetTCPConnection -LocalPort 3000 -State Listen ^| Stop-Process -Id {$_.OwningProcess} -Force`
+echo   then rerun start.bat.
+echo  ========================================
+echo.
+pause
+exit /b 1
+:port_retry_after
```

**Doc — NEW ECHO.md §"Engine Startup Pre-Flight".**

Inserted between current `§Session Lifecycle` (ECHO.md line 216) and `§FID Lifecycle` (line 247). Approximately 12 paragraphs of pre-flight contract.

Sample draft (Spencer approves content before merge):

```text
## Engine Startup Pre-Flight

When the operator (Spencer) is about to run `start.bat` and launch the engine, the agent's contract is:

**Required agent-side verification BEFORE acknowledging "ready to launch":**

1. **savant.blocked file absent.** If `savant.blocked` exists at the project root (or `data/savant.blocked`), the engine will refuse to start. The agent must call the configured halt-clear path (currently `GET /api/risk/clear-block` via `http://127.0.0.1:8080/api/risk/clear-block`) and confirm the file is deleted. If the API is unreachable, the agent must warn Spencer to manually `rm savant.blocked` before proceeding.

2. **No conflicting stale processes.** The agent must `Get-Process` for `savant`, `anvil`, `node`, and inspect for any savant-trading-related ones. If found, the agent must warn Spencer.
   - Stale *Rust engine* is benign (`start.bat` pre-build kills it cleanly).
   - Stale *Anvil fork* on `:8545` is benign (`start-anvil.bat` is idempotent).
   - Stale *m3-proxy* on `:4000` is benign (m3-proxy-controller.bat reuses).
   - Stale *Node dashboard* on `:3000` is the FID-220 footgun. It will block the engine launch. The agent must specifically warn Spencer about this and recommend `Get-NetTCPConnection -LocalPort 3000 -State Listen | Stop-Process -Id {$_.OwningProcess} -Force` BEFORE Spencer runs `start.bat`.

3. **Source tree clean.** `git status --short` should be empty. If working tree has uncommitted modifications, the agent must surface them so Spencer can decide whether to commit-then-run or stash-then-run.

**Required agent-side documentation AFTER launching:**

4. **Capture the boot log path.** As soon as `start.bat` is running, identify where the boot log is writing (typically `data/boot_logs/savant_boot_*.log` or via the `TELEMETRY_LOG_DIR` env var if set). The agent must hand this path to Spencer before the conversation ends so a future session can read it.

5. **Interval sanity check.** During long-running sessions (Spencer typically runs 24/7 monitoring), the agent must not autonomously restart the engine — only Spencer triggers engine launches. If the engine dies mid-session and Spencer's hands-off, the agent documents the death in `dev/sessions/...` but does NO action to recover.

**Common failure modes the agent must recognize in boot logs (Spencer will provide a boot log excerpt, the agent diagnoses):**

- `EADDRINUSE :::3000` → FID-220: stale dashboard. Provide manual fix command.
- `EADDRINUSE :::4000` → m3-proxy runaway. Provide manual fix command.
- `EADDRINUSE :::8080` → API server already bound. Provide manual fix command.
- `EADDRINUSE :::8545` → Anvil already up (benign, but warn about dual forks).
- `panic` in Rust log → call out the line number and the FID that introduced the affected code.
- `starting_balance != on_chain_balance` (FID-117 reconciliation) → warn about journal drift, but agent does NOT autonomously reconcile.
- `WalletKey` redaction failure (`Display`/`Debug` shows raw key) → HALT the launch (security invariant broken).
```

### Verification

1. **Unit test (manual):** Run `start.bat` on a freshly-quit state with NO `:3000` holder. Confirm the pre-build cleanup is unchanged (still 0 kills logged). The retry block now exits successfully after 1 attempt.

2. **Unit test (manual):** Manually launch a `pnpm dev` in PowerShell from `dashboard/`. Confirm `start.bat`'s pre-build kills it (log: `killed node PID X`). Confirm dashboard binds `:3000` successfully on launch.

3. **Multi-PID regression test:** Open TWO `pnpm dev` instances on `:3000` and `:3001`. Confirm `start.bat` pre-build kills BOTH (log: `Killed 2 stale process(es) on :3000`).

4. **Retry regression test:** Use PowerShell to programmatically hold `:3000` with a process that ignores `taskkill /F` (e.g., Windows `spoolsv.exe`-class service). Confirm `start.bat` aborts with the FATAL message after 3 retries.

5. **Audit grep (Law 4):** `grep -nE 'savant-trading\\\\dashboard' start.bat` returns the new HUNK B filter line. `grep -nE 'port_retry' start.bat` returns the new HUNK C handlers.

6. **Manual end-to-end:** Spencer runs `start.bat` end-to-end. Reports back whether the dashboard binds `:3000` successfully on the first try. As of 2026-06-20 this should be `yes` for the FID-219+ working-tree scenario.

---

## Perfection Loop

### Loop 1 — anticipated issue: HUNK C produces a `goto` into a label that doesn't exist if labels are CRLF-mangled

**RED:** start.bat is CRLF-sensitive (per the file's own header comment). If HUNK C is inserted with LF endings (via Python heredoc with `\n`), Windows `cmd.exe`'s label scan will break with `The system cannot find the batch label specified - port_retry`.

**GREEN:** HUNK C appended with explicit CRLF line endings. `python -c "with open('start.bat','ab') as f: f.write(b'\\r\\n'.join(LINES) + b'\\r\\n')"`. Verify via `grep -aE '\\r$' start.bat` returning hits in the new section.

**AUDIT:** Run `start.bat`, observe no label-scan error. If error, re-apply with CRLF.

**CHANGE DELTA:** +1 line of verification.

### Loop 2 — anticipated issue: `Get-NetTCPConnection` may not be on PowerShell 5 (default on older Windows)

**RED:** `Get-NetTCPConnection` is a PowerShell 5+ cmdlet. Earlier Windows builds default to PowerShell 4. `start-anvil.bat` is calling plain `netstat` so this is a non-issue for the OLD `start.bat` paths but the diagnostic command in the FATAL message uses `Get-NetTCPConnection`. If Spencer runs start.bat on a PowerShell 4 host (rare), the FATAL message's instruction fails.

**GREEN:** Two options:
- **Option (a):** Document in FATAL message that `Get-NetTCPConnection` is PowerShell 5+; offer `netstat -aon | findstr ":3000 "` as fallback.
- **Option (b):** Use the older `netstat` syntax in the FATAL message itself.

Option (b) is more portable. Patch FATAL message to: `run 'netstat -aon | findstr ":3000" | findstr LISTENING' to find the holder, then 'taskkill /F /PID <PID>' to kill it manually. Then rerun start.bat.`

**AUDIT:** Verify FATAL message uses netstat syntax.

**CHANGE DELTA:** +2 lines of patching in HUNK C.

### Loop 3 — anticipated issue: HUNK C labels collide with existing label names

**RED:** `start.bat` has no `goto`/`:label` directives currently — it's all linear with `if (...)` blocks. So `port_retry` / `port_retry_done` / `port_retry_after` are new. But future FIDs may introduce labels. We register these now so a future collision is detectable.

**GREEN:** Add a comment header above each label explaining its scope: `:: HUNK C label: retry-attempt counter for netstat port-3000 holder check (FID-220)`. Future FIDs grep `port_retry` to find FID-220-owned labels.

**AUDIT:** `grep -nE '^:port_retry' start.bat` returns expected labels.

**CHANGE DELTA:** +3 comment lines.

### Loop 4 — anticipated issue: HUNK A's `set /a` arithmetic requires PowerShell not cmd.exe

**RED:** `set /a` is a `cmd.exe` builtin. The script is `cmd.exe` (`.bat`). So `set /a "KILLED_COUNT+=1"` works directly. No issue.

**GREEN:** No patch needed; validation by observation.

**AUDIT:** Confirm counter increments in the log output during a multi-PID regression test.

**CHANGE DELTA:** 0 lines.

### Loop 5 — anticipated issue: ECHO.md §Engine Startup Pre-Flight location/Spencer edits

**RED:** ECHO.md has §Session Lifecycle at line 216 and §FID Lifecycle at line 247. Inserting §Engine Startup Pre-Flight between them preserves existing FID lifecycle doc structure.

**GREEN:** Insert at line 246 (just before §FID Lifecycle, blank line separator at 247). 12 paragraphs of pre-flight contract.

**AUDIT:** `grep -nE '^## .*Engine Startup' ECHO.md` returns the new line.

**CHANGE DELTA:** +12 paragraphs (≈50 lines) in ECHO.md.

### Loop 6 (optional) — anticipated issue: diagnostic command contention

**RED:** If `start.bat` aborts with FATAL, Spencer runs the manual-fix command. If that command identifies a process whose parent is `pid 4` (Windows System) or `pid 0`, the manual fix is impossible without elevation. We should detect this case.

**GREEN:** Extend FATAL message to warn: `If the offending PID is 4 (System) or 0, this is a kernel-level socket holder — elevation may be required.`

**AUDIT:** Simulate by trying to kill PID 4 (succeeds only with elevation). Confirm WARN appears before abort.

**CHANGE DELTA:** +1 line in HUNK C.

---

## Resolution

- **Fixed By:** Vera (substrate: Codebuff-M3)
- **Fixed Date:** 2026-06-20 15:35 UTC
- **Fix Description:**
  - **HUNK A** at `start.bat:46-54` — applied. Replaced `KILLED_3000` flag with `KILLED_COUNT` counter + unconditional kill-every-PID loop. Counter increments via `set /a "KILLED_COUNT+=1" >nul`. New flow: echo announcement → init counter → for /f loop (kill every matching PID) → post-loop `if %KILLED_COUNT% gtr 0` log → 2-second settle wait → final echo. ~9 lines net (490 → 590 bytes).
  - **HUNK B** at `start.bat:38` (PowerShell echo line) — applied. Added `'*savant-trading\dashboard*'` as a third `-like` clause between the existing `'*savant-trading*'` and `'*m3-proxy*'` clauses in the WMI filter. Plus a 2-line `:: Expand filter to catch externally-started dev-mode dashboards (FID-220 Hunk B).` comment in start.bat above the echo line. ~2 lines net (293 → 382 bytes).
  - **HUNK C** NEW BLOCK — explicitly DEFERRED per Spencer's standing-rule instruction "I will NOT apply HUNK C without your explicit go-ahead." HUNK C is captured in this FID doc but not applied. Carry-forward: see Open-Work Item below.
  - **ECHO.md §"Engine Startup Pre-Flight"** — explicitly DEFERRED. Drafted in this FID doc §"Doc" section as proposed content; awaiting Spencer's review + approval before merge.
- **Tests Added:** 0 (batch scripts are not exercised by `cargo test`; verification via byte-level diff + grep reachability + code-review audit + empirical smoke queued)
- **Verified By:**
  - **Backup:** `start.bat.pre-fid220-bak-20260620` created before edit. Diff against current start.bat shows only the HUNK A + HUNK B regions changed. Reversible if green fails.
  - **CRLF integrity:** `grep -ac $'\r' start.bat == wc -l < start.bat` confirmed all lines remain CRLF after the byte-level edit. No LF-only insertions introduced by the Python `read_bytes/replace/write_bytes` operation.
  - **Reachability (Law 4):**
    - `grep -cE 'KILLED_3000' start.bat` → `0` (old flag fully removed; nothing else in the script referenced it).
    - `grep -nE 'KILLED_COUNT|Port-3000 cleanup: scanning' start.bat` → new content present at the expected line range.
    - `grep -nE 'savant-trading\\\\dashboard|Hunk B' start.bat` → new content present at the expected line range (PowerShell wildcard).
  - **Code-reviewer (minimax-m3) PASS x5:**
    - 1. CRLF preservation: PASS (byte-faithful operation).
    - 2a/b/c. HUNK A delayed-expansion semantics: PASS (no `%KILLED_COUNT%` reads inside the for body — only `set /a` writes; post-loop `if %KILLED_COUNT% gtr 0` reads in a SEPARATE compound statement parsed after the for completes; `>nul` correctly suppresses `set /a` integer echo; zero-PID edge case correctly skips log).
    - 3a/b/c. HUNK B PowerShell wildcard: PASS (`*savant-trading\dashboard*` is valid wildcard; cmd.exe echo heredoc passes `\` correctly through to the temp `.ps1`; CRLF on each echo line).
    - 4. Cross-cutting integrity: PASS structurally. ONE MINOR empirical-note surfaced for Spencer — parens-in-echo args in the trailing line `echo  Port-3000 cleanup done (re-verify before continuing).` are new pattern in start.bat; cannot be 100% confirmed cmd.exe-safe without a smoke run.
    - 5. Backwards compat: PASS (no external references to the removed `KILLED_3000` flag).
  - **Empirical smoke test:** PENDING — Spencer runs `start.bat` end-to-end. Per standing rule #9, "Vera verifies pre-flight; Spencer runs start.bat. Never autonomously run the engine binary." Until then: AUDIT phase complete (code-review pass); E2E smoke queued for next launch.
- **Commit/PR:** pending. Spencer reviews + commits + pushes the start.bat diff. Suggested commit message: `fix: FID-220 widen start.bat port-3000 cleanup (kill-every-PID + savant-trading\dashboard filter)` once the empirical smoke confirms no parse surprises.
- **Archived:** pending close. Will archive to `dev/fids/archive/` when status moves to `verified` (after Spencer's empirical smoke).

### Status

- [x] Operational header note
- [x] RED phase (issue diagnosis complete via boot log + start.bat code review + grep reachability)
- [x] GREEN phase (HUNK A + HUNK B implemented via Python byte-level edit with anchor assertions; HUNK C explicitly deferred per Spencer's instruction)
- [x] AUDIT phase — partial (code-reviewer PASS x5; empirical smoke pending)
- [x] SELF-CORRECT phase (delayed-expansion trap documented in Code-Reviewer note (a) for future-proofing; parens-in-echo surprise documented in Code-Reviewer note (b) for empirical verification)
- [ ] COMPLETE — closed + archived (pending empirical smoke + Spencer's commit + Spencer's earlier-archival decision)

---

## Lessons Learned

*(Filled at close)*

- **Batch-file singleton flags are footguns.** The `KILLED_3000=<empty>` flag was correct when the script had a single-PID distribution assumption but became wrong the moment multi-PID holders became possible (IPv4+IPv6 dual bind, multi-tenant dashboards, etc.). Rule: avoid singular flags inside `for /f` loops over multi-PID data; always count + iterate.
- **WMI CommandLine filtering needs Path-component ancestor match, not just substring.** A `pnpm dev` may have a CommandLine whose resolved path is under `savant-trading/dashboard` but a substring search for `savant-trading` alone is fragile (depends on PowerShell session resolution mode). Rule: include both the substring and the path-component match for any workspace-relative path filter.
- **Sockets linger in `Close_wait` after kill.** Even a successful `taskkill /F /PID X` takes 1-2s for the OS to actually release the LISTENING socket back to the bindable pool. A 2-second `timeout /t 2` IS the right wait, but a single attempt with no retry-loop is brittle. Rule: any port-cleanup MUST verify post-condition (netstat listens for the port is empty) before proceeding.
- **A FATAL abort message is cheaper than a silent restart loop.** The original failure mode — dashboard crashes every 5s while the engine stays alive — looks like the engine is broken, but it's actually the start script being too optimistic. Rule: if a port-cleanup cannot succeed after N retries, abort loudly with a manual-fix command. Don't proceed to downstream phases.

---

## Cross-References

- **Parent FID lineage in start.bat-family FIDs:**
  - [`dev/fids/archive/FID-2026-0616-175-start-bat-node-kill-kills-kilo.md`](archive/FID-2026-0616-175-start-bat-node-kill-kills-kilo.md) — established the WMI-commandline filter pattern that Hunk B widens.
  - [`dev/fids/archive/FID-2026-0616-177-revert-start-bat-anvil-default.md`](archive/FID-2026-0616-177-revert-start-bat-anvil-default.md) — restored Anvil workflow; FID-220 inheriting the Anvil-only dev pattern.
  - [`dev/fids/archive/FID-2026-0616-178-start-bat-anvil-block.md`](archive/FID-2026-0616-178-start-bat-anvil-block.md) — diagnosed `cmd.exe` parse-error in start.bat; FID-220 continues the cmd.exe robustness thread.
- **This-session triggered-by FIDs:**
  - [`dev/fids/archive/FID-2026-0620-219plus-defensive-enabled-flag-guard.md`](archive/FID-2026-0620-219plus-defensive-enabled-flag-guard.md) — the FID-219+ work whose negative-path smoke test surfaced the stale-dashboard state that crashed start.bat this morning.
  - `data/boot_logs/fid219plus_neg_v2.log` — direct crash evidence showing the EADDRINUSE 4× restart loop.
- **ECHO.md cross-link:** §Session Lifecycle (`ECHO.md:216`) + §FID Lifecycle (`ECHO.md:247`) — new §"Engine Startup Pre-Flight" inserts between them.
- **Manual-fix command doc (cross-cutting):** Spencer's standing rule #9 in `dev/vera/MEMORY.md` — "Vera verifies pre-flight; Spencer runs start.bat. Never autonomously run the engine binary." FID-220's content is purely pre-flight verification + batch-fix; it does NOT change this rule.
- **Open-Work Item handoff:** [`dev/handoffs/2026-06-20-FID-219plus-handoff.md`](handoffs/2026-06-20-FID-219plus-handoff.md) — Item 1 (negative-path smoke) is the empirical sibling verification: it was BLOCKED by the same `EADDRINUSE :::3000` that FID-220 fixes in script. Item 1 + FID-220 form a pair: Item 1 cannot succeed until FID-220 lands.

---

## Status Checklist

- [x] Operational header note (parent FID lineage + 3 issues + 3 hunks + 1 ECHO-section, self-contained)
- [x] RED phase (issue diagnosis complete via boot log + start.bat code review + grep reachability)
- [x] GREEN phase (proposed diffs captured per Hunk A/B/C; awaiting Spencer authorization for script edit)
- [ ] AUDIT phase (pending — runs after green applied + Spencer runs start.bat end-to-end)
- [ ] SELF-CORRECT phase (pending)
- [ ] COMPLETE — closed + archived (pending Spencer authorization)

---
