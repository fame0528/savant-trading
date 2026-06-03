# SAVANT TRADING — LIVE CANARY Launcher
# Small, long-only live validation run on config/canary.toml (BTC/ETH/SOL).
#
#   *** THIS TRADES REAL MONEY on your live Kraken account. ***
#
# Before running:
#   1. Flatten any leftover holdings to USD on Kraken (start clean ~$45 USD).
#   2. Confirm KRAKEN_API_KEY / KRAKEN_API_SECRET are set in .env with
#      "Create & modify orders" permission.
# Watch:  data/alerts.jsonl  and  http://localhost:8080/api/status
# Auto-restarts on crash with 10-second delay. Close the window to stop.

$ErrorActionPreference = "Continue"
$env:RUST_LOG = "info"
$env:SAVANT_CONFIG = "config/canary.toml"
# Leave leftover assets alone on boot (we flatten manually). Set to "1" only if
# you want the engine to market-sell non-USD balances on startup.
$env:SAVANT_LIQUIDATE_ON_START = "0"

$logDir = "logs"
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$logFile = "$logDir\canary_$timestamp.log"
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }

Write-Host "============================================" -ForegroundColor Magenta
Write-Host "  SAVANT TRADING — LIVE CANARY" -ForegroundColor Magenta
Write-Host "  Config: config/canary.toml (BTC/ETH/SOL)" -ForegroundColor Magenta
Write-Host "  LONG-ONLY — REAL MONEY" -ForegroundColor Yellow
Write-Host "  Started: $(Get-Date)" -ForegroundColor Magenta
Write-Host "  Log: $logFile" -ForegroundColor Magenta
Write-Host "============================================" -ForegroundColor Magenta
Write-Host ""

while ($true) {
    Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Starting canary..." -ForegroundColor Green
    try {
        & cargo run --release 2>&1 | ForEach-Object {
            $line = "[$(Get-Date -Format 'HH:mm:ss')] $_"
            Write-Host $line
            Add-Content -Path $logFile -Value $line
        }
    } catch {
        Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Crash: $_" -ForegroundColor Red
        Add-Content -Path $logFile -Value "[$(Get-Date -Format 'HH:mm:ss')] CRASH: $_"
    }
    Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Engine exited. Restarting in 10s..." -ForegroundColor Yellow
    Start-Sleep -Seconds 10
}
