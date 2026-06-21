@echo off
setlocal
title M3 Proxy Controller
:: ============================================================
:: CRLF LINE ENDINGS REQUIRED
:: ============================================================
:: This file uses Windows CRLF line endings. LF-only or CR-only
:: lines break cmd.exe's backward-label scan, which fires when a
:: :label is invoked by call/goto from below. Symptom:
::   "The system cannot find the batch label specified - <label>"
::
:: If you see that error after editing, run:
::     powershell -ExecutionPolicy Bypass -File scripts\normalize_crlf.ps1
:: or (the cmd wrapper):
::     scripts\normalize_crlf.bat
:: The .gitattributes at project root also forces CRLF for .bat
:: and .ps1 files so fresh checkouts stay clean.
:: ============================================================

:: NOTE: deliberately NOT `cd /d "%~dp0"`. cmd.exe's `call`
:: inherits the child's cwd UPWARD into the parent process —
:: so if we cd to scripts/, start.bat's later `cd dashboard`
:: (which would resolve relative to PROJECT root, not scripts/)
:: would fail with "The system cannot find the path specified."
:: All subroutines below use absolute paths via %~dp0 so no cd
:: is needed.

:: ============================================================
:: M3 PROXY CONTROLLER (AIO)
:: ============================================================
:: Companion to m3-proxy.bat. One entry point for the most
:: common operations: ensure / status / start / stop / restart
:: / watch / logs / help. Adding a command? Drop a new label
:: below and register it in the DISPATCHER block at the top.
::
:: DEFAULT ACTION (no args): "ensure running" — starts if the
:: proxy is down, reports status if it's already up. Exits 0
:: in either case so it can sit in a `call` chain without
:: exploding the next step.
::
:: start.bat already kills m3-proxy node processes via a
:: cmdline-substring match (`*m3-proxy*`). The launch line below
:: keeps that substring intact so the cleanup still works.
:: ============================================================

:: --- DISPATCHER ---
if "%~1"=="" goto ensure
if /i "%~1"=="start"    goto start_proxy
if /i "%~1"=="stop"     goto stop_proxy
if /i "%~1"=="restart"  goto restart_proxy
if /i "%~1"=="status"   goto status_proxy
if /i "%~1"=="watch"    goto watch_proxy
if /i "%~1"=="install-247"     goto install_247_proxy
if /i "%~1"=="uninstall-247"   goto uninstall_247_proxy
if /i "%~1"=="logs"     goto logs_proxy
if /i "%~1"=="help"     goto help_proxy
if /i "%~1"=="/?"       goto help_proxy

echo Unknown command: %~1
goto help_proxy


:: ============== SUBROUTINES ==============

:ensure
call :status_proxy silent
if errorlevel 1 (
    echo [ensure] DOWN on :4000. Starting...
    call :start_proxy
    rem exit /b WITHOUT an argument inherits the CURRENT errorlevel
    rem at execution time. This is NOT subject to cmd parse-time
    rem variable capture. The errorlevel inside this IF-block was
    rem being substituted at parse time with the stale value 1
    rem the errorlevel that put us into the IF-block in the first
    rem place AND even when call start_proxy later ran successfully
    rem and dropped errorlevel to 0. The compiled exit /b 1 then
    rem propagated to start.bat which printed the misleading
    rem WARNING M3 proxy failed to start banner even though
    rem the proxy had actually bound and was up.
    rem Bare exit /b uses the LIVE errorlevel propagating the
    rem real outcome of call start_proxy up the call chain.
    exit /b
)
echo [ensure] UP on :4000.
exit /b 0


:status_proxy
:: Args: [silent] — suppresses the "Querying :4000" header.
set "STATUS_HEADER=1"
if /i "%~1"=="silent" set "STATUS_HEADER=0"
if "%STATUS_HEADER%"=="1" echo [status] Querying :4000...

:: netstat -aon line format:
::   Proto  Local-Address       Foreign-Address  State    PID
::   TCP    0.0.0.0:4000        0.0.0.0:0        LISTENING  1234
:: The trailing space after ":4000" prevents matching :40001 etc.
:: %RANDOM% on the temp filename prevents concurrent invocations
:: from racing on the same scratch file (e.g. two `call`s in the
:: same script block, or a watch + status piped together).
set "STATUS_TMP=%TEMP%\m3proxy_status_%RANDOM%.txt"
netstat -aon 2>nul | findstr ":4000 " | findstr "LISTENING" > "%STATUS_TMP%"
if %errorlevel% neq 0 (
    if "%STATUS_HEADER%"=="1" echo [status] DOWN: nothing listening on :4000.
    del "%STATUS_TMP%" >nul 2>&1
    exit /b 1
)

for /f "tokens=5" %%P in ('type "%STATUS_TMP%"') do (set "STATUS_PID=%%P")
del "%STATUS_TMP%" >nul 2>&1

if "%STATUS_HEADER%"=="1" echo [status] UP: PID %STATUS_PID% listening on :4000
exit /b 0


:stop_proxy
echo [stop] Killing node processes running m3-proxy...

:: PowerShell: find any node.exe whose cmdline mentions m3-proxy,
:: then taskkill /F /T  (the /T flag kills the child node AND the
:: cmd wrapper that start spawned as a parent).
set "PS_TEMP=%TEMP%\m3proxy_stop_%RANDOM%.ps1"
>  "%PS_TEMP%" echo $procs = Get-Process -Name node -ErrorAction SilentlyContinue
>> "%PS_TEMP%" echo foreach ($p in $procs) {
>> "%PS_TEMP%" echo     $cmd = (Get-CimInstance Win32_Process -Filter "ProcessId = $($p.Id)").CommandLine
>> "%PS_TEMP%" echo     if ($cmd -like '*m3-proxy*') {
>> "%PS_TEMP%" echo         & taskkill /F /T /PID $p.Id 2^>$null
>> "%PS_TEMP%" echo         if ($LASTEXITCODE -eq 0) { Write-Output ("Killed PID " + $p.Id) } else { Write-Output ("WARN PID " + $p.Id + " exit=" + $LASTEXITCODE) }
>> "%PS_TEMP%" echo     }
>> "%PS_TEMP%" echo }
powershell -NoProfile -ExecutionPolicy Bypass -File "%PS_TEMP%" 2>nul
del "%PS_TEMP%" >nul 2>&1

:: Failsafe: anything still holding :4000 after the PS sweep gets
:: force-killed. Catches the case where a foreign process took the
:: port (e.g., if the proxy auto-restarted into a stuck listener).
for /f "tokens=5" %%P in ('netstat -aon 2^>nul ^| findstr ":4000 " ^| findstr "LISTENING"') do (
    echo [stop] Failsafe: killing PID %%P still on :4000
    taskkill /F /PID %%P >nul 2>&1
)

:: Windows releases loopback sockets well under 2s. Give it room.
ping -n 3 127.0.0.1 >nul

call :status_proxy silent
if errorlevel 1 (
    echo [stop] Downing confirmed: :4000 is free.
) else (
    echo [stop] WARN: :4000 still occupied after kill.
)
exit /b 0


:start_proxy
:: Idempotency: refuse if already running.
call :status_proxy silent
if not errorlevel 1 (
    echo [start] Already running on :4000. Nothing to do.
    exit /b 0
)

:: Resolve TOKEN_ROUTER_API_KEY with two-tier fallback:
::   1. Inherited env (start.bat pre-loads .env into its own
::      environment before calling us — preferred path).
::   2. Read directly from project-root .env via ABSOLUTE path
::      `"%~dp0..\.env"` (works when called standalone; .env
::      lives at the project root, NOT at scripts/).
:: The old `if exist ".env"` lookup was relative-path because
:: this script used to `cd /d "%~dp0"` at the top — once the
:: audit moved this controller into scripts/, the relative
:: `.env` lookup could never find the project-root file.
if "%TOKEN_ROUTER_API_KEY%"=="" (
    if exist "%~dp0..\.env" (
        for /f "tokens=1,* delims==" %%a in ('findstr /b /c:"TOKEN_ROUTER_API_KEY=" "%~dp0..\.env"') do (
            set "%%a=%%b"
        )
    ) else (
        echo [start] ERROR: TOKEN_ROUTER_API_KEY not in env and no .env at %~dp0..\
        exit /b 1
    )
)
if "%TOKEN_ROUTER_API_KEY%"=="" (
    echo [start] ERROR: TOKEN_ROUTER_API_KEY missing or empty.
    exit /b 1
)
echo [start] Using TOKEN_ROUTER_API_KEY=%TOKEN_ROUTER_API_KEY:~0,8%...

:: Sweep port 4000 first. A leftover listener from a prior
:: unfinished run will block node from binding :4000 here.
:: Mirrors :stop_proxy's ":4000" failsafe pattern.
for /f "tokens=5" %%P in ('netstat -aon 2^>nul ^| findstr ":4000 " ^| findstr "LISTENING"') do (
    echo [start] Killing stale listener on :4000 [PID %%P]...
    taskkill /F /PID %%P >nul 2>&1
)
ping -n 2 127.0.0.1 >nul

:: Launch detached + minimized + log-captured.
::
:: cmdline includes "m3-proxy" so start.bat's pre-build cleanup
:: (filter `cmd -like '*m3-proxy*'`) still picks it up.
::
:: CRITICAL: do NOT route through `cmd /c`. The nested-quoting
:: pattern `start ... cmd /c "node \"x\" > \"y\" 2>&1"` is cmd's
:: classic trap — outer `start` consumes part of the inner
:: quotes, leaving the inner cmd with malformed tokens like a
:: bare `\` adjacent to a drive letter (→ "The system cannot
:: find the drive specified." twice per spawn). Launch `node`
:: directly so we have a SINGLE layer of quoting. The `> log
:: 2>&1` redirects attach to start's stdio, which node inherits,
:: so node's console.log/console.error still reach the log.
::
:: Also: confirm `node` is on PATH first. If absent, surface a
:: clean error instead of letting the start window vanish
:: silently (start.bat's pre-build cleanup will still find no
:: m3-proxy to kill, and start.bat will print the misleading
:: "M3 proxy running on :4000" line).
where node >nul 2>&1
if errorlevel 1 (
    echo [start] ERROR: 'node' not on PATH. Install Node.js or add it to PATH and try again.
    exit /b 1
)
start "M3 Proxy" /min node "%~dp0m3-proxy.js" > "%~dp0m3-proxy.log" 2>&1

:: Give Node a moment to bind :4000. 5s is generous for first-shot
:: module resolution / DNS lookup when the port was just freed.
ping -n 6 127.0.0.1 >nul

call :status_proxy silent
if not errorlevel 1 (
    echo [start] UP: proxy listening on :4000.
    echo [start] Logs:  %~dp0m3-proxy.log
    exit /b 0
) else (
    echo [start] WARN: didn't bind :4000 in 5s. Check log + key.
    exit /b 1
)


:restart_proxy
echo [restart] Stopping...
call :stop_proxy
echo [restart] Starting...
call :start_proxy
exit /b 0


:watch_proxy
set "WATCH_RESTART_COUNT=0"
set "WATCH_MAX_RESTARTS=10"
echo [watch] Daemon mode: polling every 15s. Ctrl+C to exit.
echo [watch] Exits 1 after %WATCH_MAX_RESTARTS% consecutive failed restarts.
:watch_loop
ping -n 16 127.0.0.1 >nul
call :status_proxy silent
if errorlevel 1 goto :watch_down
if not "%WATCH_RESTART_COUNT%"=="0" set "WATCH_RESTART_COUNT=0"
echo [%date% %time%] [watch] Proxy UP. PID=%STATUS_PID%.
goto :watch_loop

:watch_down
set /a WATCH_RESTART_COUNT+=1
echo [%date% %time%] [watch] Proxy DOWN. Restarting (%WATCH_RESTART_COUNT%/%WATCH_MAX_RESTARTS%)...
call :start_proxy >nul
if errorlevel 1 goto :watch_fail
set "WATCH_RESTART_COUNT=0"
echo [%date% %time%] [watch] Restored.
goto :watch_loop

:watch_fail
if "%WATCH_RESTART_COUNT%" GEQ "%WATCH_MAX_RESTARTS%" (
    echo [%date% %time%] [watch] FATAL: %WATCH_MAX_RESTARTS% consecutive failures. Exiting 1.
    exit /b 1
)
goto :watch_loop


:logs_proxy
if not exist "%~dp0m3-proxy.log" (
    echo [logs] No log file yet. Run 'start' or 'restart' first.
    exit /b 0
)
echo [logs] Tailing %~dp0m3-proxy.log  (Ctrl+C to exit)...
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-Content '%~dp0m3-proxy.log' -Wait -Tail 30"
exit /b 0


:install_247_proxy
echo [install-247] Registering 'M3ProxyWatch' as a 24/7 scheduled task...
echo [install-247] Auto-starts at logon, restarts on failure (1-min cooldown, up to 999 times).

set "PS_TEMP=%TEMP%\m3proxy_install_%RANDOM%.ps1"
>  "%PS_TEMP%" echo $ErrorActionPreference = 'Stop'
>> "%PS_TEMP%" echo $batPath  = "%~dp0m3-proxy-controller.bat"
>> "%PS_TEMP%" echo $user     = "$env:USERDOMAIN\$env:USERNAME"
>> "%PS_TEMP%" echo $action   = New-ScheduledTaskAction -Execute $batPath -Argument "watch" -WorkingDirectory (Split-Path -Path $batPath -Parent)
>> "%PS_TEMP%" echo $trigger  = New-ScheduledTaskTrigger -AtLogOn -User $user
>> "%PS_TEMP%" echo $settings = New-ScheduledTaskSettingsSet -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
>> "%PS_TEMP%" echo Register-ScheduledTask -TaskName "M3ProxyWatch" -Action $action -Trigger $trigger -Settings $settings -Force -Description "M3 thinking-killer proxy watch daemon. m3-proxy-controller.bat watch polls :4000 every 15s, exits 1 after 10 consecutive failed restarts (~150s); Task Scheduler restarts after a 1-min cooldown."
powershell -NoProfile -ExecutionPolicy Bypass -File "%PS_TEMP%"
set "RC=%ERRORLEVEL%"
del "%PS_TEMP%" >nul 2>&1

if not "%RC%"=="0" (
    echo [install-247] ERROR: PowerShell exited %RC%. Admin rights may be required.
    exit /b %RC%
)
echo [install-247] Done.
echo [install-247] Trigger now:    schtasks /run /tn M3ProxyWatch
echo [install-247] Inspect:        schtasks /query /tn M3ProxyWatch /v /fo list
exit /b 0


:uninstall_247_proxy
echo [uninstall-247] Removing 'M3ProxyWatch' scheduled task...

set "PS_TEMP=%TEMP%\m3proxy_uninstall_%RANDOM%.ps1"
>  "%PS_TEMP%" echo $exists = Get-ScheduledTask -TaskName "M3ProxyWatch" -ErrorAction SilentlyContinue
>> "%PS_TEMP%" echo if ($exists) { Unregister-ScheduledTask -TaskName "M3ProxyWatch" -Confirm:$false ; Write-Output ("Removed 'M3ProxyWatch'.") } else { Write-Output ("Not installed; nothing to do.") }
powershell -NoProfile -ExecutionPolicy Bypass -File "%PS_TEMP%"
set "RC=%ERRORLEVEL%"
del "%PS_TEMP%" >nul 2>&1

if not "%RC%"=="0" (
    echo [uninstall-247] ERROR: PowerShell exited %RC%. Admin rights may be required.
    exit /b %RC%
)
echo [uninstall-247] Done.
exit /b 0


:help_proxy
echo.
echo Usage: m3-proxy-controller.bat [command]
echo.
echo Commands:
echo   (no args)       Ensure running: start if down, report if up.
echo   status          Report whether proxy is listening on :4000.
echo   start           Launch the proxy minimized, idempotent.
echo   stop            Kill the proxy and free :4000.
echo   restart         Stop then start.
echo   watch           Daemon mode: poll every 15s, auto-restart.
echo   install-247     Register 'M3ProxyWatch' scheduled task for 24/7.
echo   uninstall-247   Remove the 'M3ProxyWatch' scheduled task.
echo   logs            Tail the live proxy log (Ctrl+C to exit).
echo   help            This message.
echo.
echo start.bat calls m3-proxy-controller.bat directly (this is
echo the canonical entry point used by the build pipeline).
echo m3-proxy.bat (legacy launcher) is still on disk for direct
echo invocation but is no longer wired into start.bat. Either
echo way the launched cmdline contains "m3-proxy", so start.bat's
echo pre-build cleanup still matches. Logs land in m3-proxy.log
echo (already gitignored).
echo.
exit /b 0
