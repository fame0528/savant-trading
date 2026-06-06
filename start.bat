@echo off
title SAVANT Trading Engine
echo.
echo  ========================================
echo   SAVANT Trading Engine
echo   Starting engine + dashboard...
echo  ========================================
echo.
cd /d "%~dp0"
:: Kill stale processes holding port 3000
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":3000 " ^| findstr "LISTENING"') do (
    echo  Killing stale process on port 3000 [PID %%a]...
    taskkill /F /PID %%a >nul 2>&1
)
timeout /t 2 /nobreak >nul
echo  Starting engine...
echo.
target\release\savant.exe serve
echo.
echo  Engine stopped. Press any key to exit.
pause >nul
