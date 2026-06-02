@echo off
title SAVANT TRADING ENGINE v0.4.4
echo ============================================
echo  SAVANT TRADING ENGINE v0.4.4
echo  Paper Trading Mode — 24/7 Operation
echo  Started: %date% %time%
echo ============================================
echo.

:loop
echo [%date% %time%] Starting engine...
cargo run --release 2>&1 | tee -a logs\engine_%date:~-4%%date:~4,2%%date:~7,2%.log
echo [%date% %time%] Engine exited. Restarting in 10 seconds...
timeout /t 10 /nobreak >nul
goto loop
