@echo off
:: ============================================================
:: SAVANT TRADING — 24/7 Launcher
:: ============================================================
:: Double-click this file to start training + live trading.
:: Training runs first (several hours), then switches to live.
:: Logs go to logs/ directory.
:: Close the window to stop.
:: ============================================================

echo Starting Savant Trading Engine v0.4.4...
echo.
echo Phase 1: Training on real Kraken tick data (224M ticks, 8 pairs)
echo Phase 2: Training report
echo Phase 3: Live paper trading (24/7)
echo.
echo Logs will be saved to logs/ directory.
echo Close this window to stop.
echo.

powershell -ExecutionPolicy Bypass -File "%~dp0run-247.ps1"
pause
