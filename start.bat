@echo off
title SAVANT Trading Engine
echo.
echo  ========================================
echo   SAVANT Trading Engine
echo   Building + starting engine + dashboard...
echo  ========================================
echo.
cd /d "%~dp0"
:: Load environment variables from .env (GITHUB_TOKEN for gh CLI, etc.)
if exist .env (
    for /f "tokens=1,* delims==" %%a in ('findstr /v /b "#" .env') do (
        if not "%%a"=="" set "%%a=%%b"
    )
    echo  .env loaded.
) else (
    echo  WARNING: .env not found. API keys may be missing.
)
:: Config override: set SAVANT_CONFIG env var to use a custom config file.
:: Example: set SAVANT_CONFIG=config\default.toml
:: FID-177: default reverted to test-anvil.toml (Anvil). Spencer's working dev
:: setup is Anvil. FID-167 changed the default to default.toml (Ethereum
:: mainnet) but that broke the Anvil workflow. Use SAVANT_CONFIG env var
:: to override for mainnet/multi-chain testing.
if not defined SAVANT_CONFIG set "SAVANT_CONFIG=config\test-anvil.toml"

:: FID-167: Active chain selection. Set SAVANT_CHAIN to one of the chains
:: declared in [chains.*] in config/test-anvil.toml. Default: arbitrum.
:: The Anvil workflow only enables Arbitrum. For other chains, use
:: config\default.toml and SAVANT_CHAIN=ethereum (or base, optimism, bsc).
if not defined SAVANT_CHAIN set "SAVANT_CHAIN=arbitrum"

:: FID-126-R3: bypass conviction + confidence gates for sub-$500 balances.
:: This restores the pre-FID-127 "all-in" path. Remove if balance > $500.
set "SAVANT_GATE_DISABLED=1"

:: ============================================================
:: PRE-BUILD CLEANUP (FID-163)
:: Kill stale processes that would otherwise hold the release
:: binary open and block `cargo build --release` with
:: "failed to remove file target\release\savant.exe".
:: Scoped by .exe name + path filter so we never kill cargo
:: itself or unrelated processes.
:: ============================================================
echo  Pre-build cleanup...
echo.

:: Helper: run a PowerShell command and capture stdout lines.
:: Avoids cmd caret-escaping nightmares by writing the PS
:: command to a temp .ps1 file, executing it, reading output.
set "PS_TEMP=%TEMP%\savant_prebuild_%RANDOM%.ps1"
set "PS_OUT=%TEMP%\savant_prebuild_%RANDOM%.txt"

:: 1. Kill any running engine binary (the file cargo wants to overwrite).
::    Scoped to this project's target\release path to avoid touching
::    other dev directories that may have a savant.exe of their own.
> "%PS_TEMP%" echo Get-Process -Name savant -ErrorAction SilentlyContinue ^| Where-Object { $_.Path -like '*savant-trading*' } ^| ForEach-Object { taskkill /F /PID $_.Id 2^>$null ; Write-Output ("Killed engine PID " + $_.Id) }
powershell -NoProfile -ExecutionPolicy Bypass -File "%PS_TEMP%" 2>nul
del "%PS_TEMP%" 2>nul

:: 2. Kill stale dashboard dev server AND M3 proxy (both are node.exe).
::    Scoped by command-line filter: only kill node processes whose
::    cmdline contains "savant-trading" or "m3-proxy". This protects
::    kilo CLI and any other unrelated node processes (MCP servers,
::    firebase-tools, playwright, etc.) that kilo spawns.
::    Pattern mirrors line 52's savant.exe path filter.
> "%PS_TEMP%" echo $procs = Get-Process -Name node -ErrorAction SilentlyContinue ; foreach ($p in $procs) { $cmd = (Get-CimInstance Win32_Process -Filter "ProcessId = $($p.Id)").CommandLine ; if ($cmd -like '*savant-trading*' -or $cmd -like '*m3-proxy*') { taskkill /F /PID $p.Id 2^>$null ; Write-Output ("Killed savant node PID " + $p.Id) } }
powershell -NoProfile -ExecutionPolicy Bypass -File "%PS_TEMP%" 2>nul
del "%PS_TEMP%" 2>nul

:: 3. Kill stale Anvil fork. We will restart it via start-anvil.bat
::    below so a fresh prefund tx lands on a known state.
> "%PS_TEMP%" echo Get-Process -Name anvil -ErrorAction SilentlyContinue ^| ForEach-Object { taskkill /F /PID $_.Id 2^>$null ; Write-Output ("Killed Anvil PID " + $_.Id) }
powershell -NoProfile -ExecutionPolicy Bypass -File "%PS_TEMP%" 2>nul
del "%PS_TEMP%" 2>nul
set "PS_TEMP="
set "PS_OUT="

:: Give Windows a moment to release the file locks.
timeout /t 2 /nobreak >nul

:: ============================================================
:: Start M3 Thinking Killer Proxy (required for MiniMax M3 in Kilo)
:: ============================================================
call "%~dp0m3-proxy.bat"
if errorlevel 1 (
    echo  WARNING: M3 proxy failed to start. Kilo will get think tags.
) else (
    echo  M3 proxy running on :4000.
)
echo.

:: ============================================================
:: Auto-start Anvil fork if not running (self-recovery).
:: start-anvil.bat is idempotent — exits quickly if Anvil is up.
:: Unconditional call avoids the cmd.exe parens-block parse error
:: that the previous if/else/findstr pattern produced. (FID-178/179)
:: ============================================================
call "%~dp0start-anvil.bat"
if errorlevel 1 (
    echo  WARNING: Anvil failed to start. Engine will retry RPC but may hang.
)
echo.

:: Kill stale processes holding port 3000 (deduplicated — was
:: printing the same PID twice due to a for-loop bug).
set "KILLED_3000="
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
    if not defined KILLED_3000 (
        echo  Killing stale process on port 3000 [PID %%a]...
        taskkill /F /PID %%a >nul 2>&1
        set "KILLED_3000=1"
    )
)
timeout /t 2 /nobreak >nul

echo  Building Rust engine...
echo.
cargo build --release 2>&1
if errorlevel 1 (
    echo.
    echo  ENGINE BUILD FAILED. Fix errors and restart.
    pause
    exit /b 1
)
echo.
echo  Engine build complete. Building dashboard...
echo.
cd dashboard
call npm run build 2>&1
if errorlevel 1 (
    echo.
    echo  DASHBOARD BUILD FAILED. Fix errors and restart.
    cd ..
    pause
    exit /b 1
)
cd ..
echo.
echo  Both builds complete. Starting engine + dashboard...
echo.
target\release\savant.exe --config "%SAVANT_CONFIG%" serve
echo.
echo  Engine stopped. Press any key to exit.
pause >nul
