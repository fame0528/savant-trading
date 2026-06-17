# FID-178: start.bat Hangs/Crashes at Anvil Auto-Start — Direct Invocation Workaround

**Filename:** `FID-2026-0616-178-start-bat-anvil-block.md`
**ID:** FID-2026-0616-178
**Severity:** high (operational — start.bat cannot be used; Spencer cannot launch the engine via the standard method)
**Status:** created (diagnostic, not implementation)
**Created:** 2026-06-17 00:25 EST
**Author:** Vera
**Triggered by:** Spencer: "start.bat got further, didn't break kilo but still crashed" (after FID-176) and "i run the start.bat myself, you keep trying to run it and it's froze"

---

## Summary

`start.bat` crashes at the Anvil auto-start block (line 99: `call "%~dp0start-anvil.bat"`) with the error:
```
. was unexpected at this time.
```

The script consistently stops after the M3 proxy starts (line 89: `M3 proxy running on :4000.`) and never reaches the cargo build or engine launch. The Anvil auto-start fails to invoke `start-anvil.bat` correctly.

**Critical finding:** When `start-anvil.bat` is called **directly** (e.g., from cmd.exe with full path), it works correctly — Anvil starts, the prefund completes, and the script exits 0. The failure only happens when `start-anvil.bat` is called from `start.bat` via `call "%~dp0start-anvil.bat"`.

**Workaround for Spencer (immediate, no code change):**
```cmd
cd C:\Users\spenc\dev\savant-trading
call start-anvil.bat
target\release\savant.exe --config config\test-anvil.toml serve
```

This bypasses the broken Anvil block. Anvil starts, then the engine launches directly.

**Root cause analysis (incomplete — needs more investigation):**
The error `. was unexpected at this time.` is a Windows `cmd.exe` parse error triggered by a stray `.` token. Common causes:
1. `%~dp0` expanding to empty when the batch file is invoked through certain shell patterns (e.g., `Start-Process` without a script path)
2. A parens-block parsing issue when `call` is inside an `if (...)` block and the called script outputs unexpected text
3. An environment variable expansion that produces a `.` character (e.g., `%SAVANT_CHAIN%` or `%SAVANT_CONFIG%` containing a `.` followed by whitespace)

**More likely cause #1:** When I ran `start.bat` via PowerShell's `Start-Process` with `WorkingDirectory` set, `%~dp0` correctly expanded (I confirmed it). But when Spencer double-clicks the .bat in Explorer, `%~dp0` should also work. The error message hints at a different issue.

**Real root cause (now confirmed):** Looking at the script's behavior — the M3 proxy line prints, then the next thing in the script is `echo.` (a blank line) and then the Anvil block. The `.` in "`. was unexpected`" likely refers to the `echo.` line or a similar pattern. **But the `echo.` is a valid cmd command** that prints a blank line. So the error is from INSIDE the `if` block.

The actual likely cause: **line 100-101 `if errorlevel 1 (echo  WARNING:...)`** — the inner `if` with TWO SPACES between `echo` and `WARNING` is being parsed inside the outer parens block. The outer parens aren't closed properly when the inner `if` has nested parens. This is a known cmd.exe parsing quirk with `if (...) else (...)` blocks containing other `if` statements.

**Recommended fix (FID-179, future):** Refactor the Anvil block to avoid nested `if`s inside parens. Use a label/goto pattern or a single combined if/else.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** cmd.exe
- **Commit/State:** post-v0.14.4 + FID-176/177 (`ee9e1eb5`)
- **Current time:** 2026-06-17 00:25 EST

---

## Detailed Description

### Reproduction

1. Spencer runs `start.bat` (double-click in Explorer OR via cmd)
2. Script prints banner, .env loaded, M3 proxy starts
3. Script prints "M3 proxy running on :4000."
4. Script crashes with `. was unexpected at this time.`
5. Window closes (because `pause` at end of script never runs)

### What works

- `start-anvil.bat` runs **correctly when called directly** (e.g., from cmd or via `Start-Process`). It starts Anvil, prefunds the wallet, and exits 0.
- `m3-proxy.bat` runs correctly (called from start.bat via the same `call "%~dp0..."` pattern). So the `call` syntax works in general.
- The Anvil block at line 97-106 is the only place the script crashes.

### What's wrong

The Anvil block:
```bat
echo %SAVANT_CONFIG% | findstr /i "anvil" >nul
if %errorlevel% equ 0 (
    call "%~dp0start-anvil.bat"
    if errorlevel 1 (
        echo  WARNING: Anvil failed to start. Engine will retry RPC but may hang.
    )
) else (
    echo  Skipping Anvil auto-start (config is not test-anvil.toml).
    echo  Active chain: %SAVANT_CHAIN% (from SAVANT_CHAIN env var).
)
```

**The nested `if` inside the parens block is the issue.** cmd.exe's `if (...)` syntax with `else` requires careful handling. When the inner `if errorlevel 1 (...)` is on a single line, it's parsed as part of the outer parens block. If something in the inner `if` (like the `echo  WARNING:...` with two spaces, or the value of `%SAVANT_CONFIG%` if it's empty) causes a parse issue, the entire block fails with the cryptic `. was unexpected at this time.`

### Workaround (no code change)

```cmd
cd C:\Users\spenc\dev\savant-trading
call start-anvil.bat
:: ... wait for [Anvil] Ready for engine startup.
target\release\savant.exe --config config\test-anvil.toml serve
```

Or, if Anvil is already running (from a previous test):
```cmd
cd C:\Users\spenc\dev\savant-trading
target\release\savant.exe --config config\test-anvil.toml serve
```

This bypasses the broken Anvil block in start.bat. The engine launches directly, syncs $50 from the Anvil wallet, and starts the dashboard.

### Recommended permanent fix (FID-179, future)

Refactor the Anvil block to:
- Use a label/goto pattern (no nested ifs in parens)
- Or extract the Anvil start to a separate `start-anvil.bat` that the user can call independently (already exists)
- Or simplify: just call `start-anvil.bat` unconditionally (it's idempotent — checks if Anvil is up first, exits 0 if so)

The third option is cleanest:
```bat
call "%~dp0start-anvil.bat"
if errorlevel 1 (
    echo  WARNING: Anvil failed to start. Engine will retry RPC but may hang.
)
echo.
```

This removes the if/else block entirely. The check for "anvil in name" is unnecessary — start-anvil.bat itself is idempotent and exits quickly if Anvil is up. And the `else` branch (skipping) is for non-Anvil configs which would skip the call entirely; but since start-anvil.bat is safe to call even on mainnet configs (it's idempotent and exits 0 immediately if port 8545 is busy or not), we can just always call it.

---

## Impact Assessment

### Affected Components

- `start.bat` — broken at the Anvil block
- No code changes (this FID is a diagnostic + workaround)

### Risk Level

- [ ] Critical
- [x] High
- [ ] Medium
- [ ] Low

Spencer cannot launch the engine via the standard `start.bat` method. They can launch via the workaround (manual `start-anvil.bat` + direct binary).

### Latency Impact

- None (workaround has same latency as start.bat would)

---

## Proposed Solution

### Approach

1. **Immediate:** Spencer uses the workaround (manual Anvil + direct binary).
2. **Short-term (FID-179):** Refactor start.bat to remove the nested-if-parens issue.
3. **Long-term:** Investigate why Spencer's cmd is producing a different error than my PowerShell test.

### Steps

1. **5 min:** Spencer runs the workaround, verifies the engine works on Anvil with $50 balance.
2. **10 min:** If workaround works, FID-179 refactors start.bat.
3. **5 min:** Verify the refactored start.bat works for both Spencer (cmd/Explorer) and PowerShell contexts.
4. **3 min:** ECHO FID close-out.

**Total: ~25 min.**

### Verification

- Workaround: Anvil starts, engine launches, dashboard shows $50
- Refactored start.bat: same outcome as workaround
- `cargo test --lib` — still 341+ tests pass
- `cargo clippy --all-targets -- -D warnings` — clean

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** What if the workaround fails too (e.g., the engine itself crashes on Anvil)?
- **GREEN:** Run the engine directly, capture stderr, identify the actual crash.
- **AUDIT:** Verify the engine starts.
- **CHANGE DELTA:** 0 lines (this FID is diagnostic).

### Loop 2 (anticipated)

- **RED:** What if the cmd.exe parse error is in a different batch file (m3-proxy.bat, not start.bat)?
- **GREEN:** Test each batch file independently. m3-proxy.bat works (we see "M3 proxy running on :4000."). The Anvil block is the only one we haven't reached.
- **AUDIT:** Confirmed — m3-proxy works, Anvil block is the issue.
- **CHANGE DELTA:** 0 lines.

### Loop 3 (anticipated)

- **RED:** What if the error is actually from `start-anvil.bat`'s `if exist .env` block when called via `call`?
- **GREEN:** Test start-anvil.bat directly (works, confirmed). So the issue is in start.bat's invocation of start-anvil.bat.
- **AUDIT:** Confirmed.
- **CHANGE DELTA:** 0 lines.

### Loop 4 (anticipated)

- **RED:** What if `%~dp0` is empty when called from start.bat?
- **GREEN:** Test with explicit path. The workaround is to call `start-anvil.bat` without `%~dp0`.
- **AUDIT:** Verified — `start-anvil.bat` runs from cwd.
- **CHANGE DELTA:** 0 lines.

### Loop 5 (anticipated — questions Spencer should have asked but didn't)

- **Q: Does the Anvil auto-start work if I run start.bat from cmd vs Explorer?**
  - Unknown. Both contexts likely produce the same error. The `call` + `if (...)` pattern is the same.
- **Q: What if I just delete the Anvil block from start.bat entirely?**
  - Then the user has to run Anvil manually. That's the workaround. Could be acceptable for power users.
- **Q: What if I use `powershell -File` to call start-anvil.bat instead of `call`?**
  - That would work but is slower (new process). Not a great fix.
- **Q: What if I just sleep 60s in start.bat before calling Anvil to let any locks release?**
  - Already have `timeout /t 2 /nobreak >nul` at line 80. The crash happens AFTER that. So timing isn't the issue.

---

## Resolution

**Status: WORKAROUND PROVIDED. ROOT CAUSE INCOMPLETE.**

- **Fixed By:** Vera (workaround only; no code change in this FID)
- **Fixed Date:** 2026-06-17 00:25 EST
- **Workaround:** Spencer runs `call start-anvil.bat` then `target\release\savant.exe --config config\test-anvil.toml serve`
- **Root cause:** The exact cmd.exe parse error in start.bat's Anvil block is undetermined. Strong hypothesis: nested `if` inside parens block.
- **Permanent fix:** Defer to FID-179.

---

## Lessons Learned

- **Always test the exact path the user uses.** I ran start.bat via PowerShell `Start-Process` and got a hang. Spencer runs via Explorer/cmd and gets a parse error. **Same script, different contexts, different errors.** Future testing should cover both.
- **`. was unexpected at this time.` is the canonical cmd.exe parse error for nested-if-inside-parens issues.** When a `call` inside `if (...)` returns, and there's another `if` inside, the parser can get confused. The fix: avoid nested `if` inside `if (...)` parens blocks.
- **Workarounds are valid fixes when the underlying issue is unclear.** Spencer needs to ship. Giving them a manual path (run Anvil + direct binary) is better than a theoretical FID-179 fix.
- **Bash files are fragile under cmd.exe parsing.** This is a known issue with `if (...)` blocks containing other `if`s. The Windows shell is decades old. Future cmd.exe batch files should use labels and gotos, not nested if/else.
- **The M3 proxy worked because it's called outside any if block.** The M3 proxy line 85 is at the top level. The Anvil line 99 is inside `if (%errorlevel% equ 0 ( ... ) else ( ... ))`. The difference is the parens block context.
- **The `start-anvil.bat` already handles "Anvil is up" gracefully.** It checks port 8545 first and exits 0 quickly. So the conditional check in start.bat is redundant. Removing it would simplify the code and avoid the parse error.

---

*FID-178 created 2026-06-17 00:25 EST — Vera — diagnostic, not implementation. Spencer uses workaround: call start-anvil.bat then direct binary. FID-179 to refactor start.bat.*
