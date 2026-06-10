# SAVANT TRADING ENGINE — 24/7 Paper Trading Launcher
# Run this in a separate terminal window. Close the window to stop.
# Auto-restarts on crash with 10-second delay.

$ErrorActionPreference = "Continue"
$env:RUST_LOG = "info"
$logDir = "logs"
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$logFile = "$logDir\engine_$timestamp.log"

if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  SAVANT TRADING ENGINE v0.5.0" -ForegroundColor Cyan
Write-Host "  Paper Trading Mode — 24/7 Operation" -ForegroundColor Cyan
Write-Host "  Started: $(Get-Date)" -ForegroundColor Cyan
Write-Host "  Log: $logFile" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

while ($true) {
    Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Starting engine..." -ForegroundColor Green
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
