@echo off
title Anvil Fork Manager
cd /d "%~dp0"

:: ============================================================
:: Auto-start Anvil fork of Arbitrum One if not already running.
:: Idempotent: if Anvil is already healthy, exits 0 in ~1 second.
::
:: FID-161 (2026-06-15): Rewritten from WSL to native Windows.
:: WSL was not available (HCS_E_SERVICE_NOT_AVAILABLE).
:: Installed Foundry v1.7.1 native binaries to C:\Users\spenc\foundry\bin\.
:: ============================================================

set WALLET=0x543CA0434B84aD38c858D2D178D2082521711fBC
set USDC=0xaf88d065e77c8cC2239327C5EDb3A432268e5831
set RPC=http://127.0.0.1:8545
set FORK_URL=https://arb1.arbitrum.io/rpc
set ANVIL_LOG=%TEMP%\anvil.log
set ANVIL=%USERPROFILE%\foundry\bin\anvil.exe
set CAST=%USERPROFILE%\foundry\bin\cast.exe

echo [Anvil] Checking port 8545...

:: 1) Health check: is an existing Anvil responsive?
%CAST% chain-id --rpc-url %RPC% >nul 2>&1
if %errorlevel% equ 0 (
    echo [Anvil] Already running and responsive on port 8545.
    goto :prefund
)

:: 2) Kill any stale listener on 8545
echo [Anvil] Killing stale listener on port 8545...
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":8545 " ^| findstr "LISTENING"') do (
    echo [Anvil] Killing PID %%a...
    taskkill /F /PID %%a >nul 2>&1
)
timeout /t 3 /nobreak >nul

:: 3) Start Anvil as a detached background process.
:: No nested quotes: FORK_URL and ANVIL_LOG have no spaces.
echo [Anvil] Starting fork of Arbitrum One at latest block...
start "" /b cmd /c "%ANVIL% --fork-url %FORK_URL% --port 8545 --silent >%ANVIL_LOG% 2>&1"

:: 4) Poll for readiness (up to 60 seconds)
echo [Anvil] Waiting for readiness...
set COUNT=0

:wait_loop
if %COUNT% geq 60 (
    echo [Anvil] ERROR: did not respond within 60 seconds.
    echo [Anvil] Last lines of log:
    powershell -Command "Get-Content '%ANVIL_LOG%' -Tail 10" 2>nul
    exit /b 1
)

%CAST% chain-id --rpc-url %RPC% >nul 2>&1
if %errorlevel% equ 0 (
    echo [Anvil] Ready after %COUNT% seconds.
    goto :prefund
)

timeout /t 1 /nobreak >nul
set /a COUNT+=1
goto :wait_loop

:prefund
:: 5) Prefund wallet: 10 ETH + 50 USDC
echo [Anvil] Prefunding wallet (10 ETH + 50 USDC)...

:: ETH: 10 ETH = 0x8AC7230489E80000 (wei)
%CAST% rpc anvil_setBalance %WALLET% 0x8AC7230489E80000 --rpc-url %RPC% >nul 2>&1

:: USDC: storage slot for balances[wallet] = keccak256(wallet . uint256(9))
for /f "delims=" %%s in ('%CAST% index address %WALLET% 9') do set SLOT=%%s
%CAST% rpc anvil_setStorageAt %USDC% %SLOT% 0x0000000000000000000000000000000000000000000000000000000002FAF080 --rpc-url %RPC% >nul 2>&1

:: 6) Verify prefund
echo [Anvil] Verifying prefund...
for /f "delims=" %%e in ('%CAST% balance %WALLET% --rpc-url %RPC% --ether 2^>nul') do set ETH_BAL=%%e
for /f "delims=" %%u in ('%CAST% call %USDC% "balanceOf(address)(uint256)" %WALLET% --rpc-url %RPC% 2^>nul') do set USDC_BAL=%%u

echo [Anvil] ETH: %ETH_BAL%  USDC (raw): %USDC_BAL%

:: Warn if USDC doesn't look right (expected: 50000000 = 50e6)
echo %USDC_BAL% | findstr "50000000" >nul 2>&1
if %errorlevel% neq 0 (
    echo [Anvil] WARNING: USDC balance unexpected. Prefund may have failed.
)

echo [Anvil] Ready for engine startup.
exit /b 0
