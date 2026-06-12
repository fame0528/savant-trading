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
    for /f "usebackq tokens=1,* delims==" %%a in ('findstr /v /b "#" .env') do (
        if not "%%a"=="" set "%%a=%%b"
    )
    echo  .env loaded.
) else (
    echo  WARNING: .env not found. API keys may be missing.
)
:: FID-126-R3: bypass conviction + confidence gates for sub-$500 balances.
:: This restores the pre-FID-127 "all-in" path. Remove if balance > $500.
set "SAVANT_GATE_DISABLED=1"

:: Kill stale processes holding port 3000
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
    echo  Killing stale process on port 3000 [PID %%a]...
    taskkill /F /PID %%a >nul 2>&1
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
target\release\savant.exe serve
echo.
echo  Engine stopped. Press any key to exit.
pause >nul
