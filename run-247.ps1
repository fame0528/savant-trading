# ============================================================
# SAVANT TRADING — 24/7 Training + Paper Trading Launcher
# ============================================================
# Runs training on real Kraken tick data overnight, then
# switches to live paper trading. Auto-restarts on crash.
# Logs everything to logs/ with timestamped files.
#
# Usage: Right-click > Run with PowerShell, or:
#   powershell -ExecutionPolicy Bypass -File run-247.ps1
#
# Stop: Close the window or Ctrl+C.
# ============================================================

$ErrorActionPreference = "Continue"
$env:RUST_LOG = "info"

# Setup
$logDir = "logs"
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$trainLog = "$logDir\training_$timestamp.log"
$engineLog = "$logDir\engine_$timestamp.log"

if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }

function Log($msg, $file) {
    $line = "[$(Get-Date -Format 'HH:mm:ss')] $msg"
    Write-Host $line
    Add-Content -Path $file -Value $line
}

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  SAVANT TRADING ENGINE v0.4.4" -ForegroundColor Cyan
Write-Host "  24/7 Operation" -ForegroundColor Cyan
Write-Host "  Started: $(Get-Date)" -ForegroundColor Cyan
Write-Host "" -ForegroundColor Cyan
Write-Host "  Training log: $trainLog" -ForegroundColor Cyan
Write-Host "  Engine log:   $engineLog" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# ============================================================
# PHASE 1: Training on real Kraken tick data
# ============================================================
# Runs full 20-run training with 60 scenarios each using historical data.
# Uses cached tick candles (224M ticks, 4.25M candles across 8 pairs).
# Takes ~4-6 hours for full training.

Log "=== PHASE 1: TRAINING ON REAL DATA ===" $trainLog
Log "Starting historical training (20 runs x 60 scenarios)..." $trainLog
Log "This will take several hours. All output logged to $trainLog" $trainLog
Write-Host ""

try {
    & cargo run --release -- --test --train --historical --full -n 60 2>&1 | ForEach-Object {
        $line = "[$(Get-Date -Format 'HH:mm:ss')] $_"
        Write-Host $line
        Add-Content -Path $trainLog -Value $line
    }
    Log "Training phase completed successfully" $trainLog
} catch {
    Log "Training phase error: $_" $trainLog
}

Write-Host ""

# ============================================================
# PHASE 2: Run training report
# ============================================================

Log "=== PHASE 2: TRAINING REPORT ===" $trainLog
Log "Generating training report..." $trainLog

try {
    & cargo run --release -- report --test 2>&1 | ForEach-Object {
        $line = "[$(Get-Date -Format 'HH:mm:ss')] $_"
        Write-Host $line
        Add-Content -Path $trainLog -Value $line
    }
} catch {
    Log "Report generation error: $_" $trainLog
}

Write-Host ""

# ============================================================
# PHASE 3: Live paper trading (runs 24/7)
# ============================================================

Log "=== PHASE 3: LIVE PAPER TRADING ===" $trainLog
Log "Switching to live paper trading..." $trainLog
Log "Engine log: $engineLog" $trainLog

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "  Training complete. Starting live engine." -ForegroundColor Green
Write-Host "  Engine log: $engineLog" -ForegroundColor Green
Write-Host "  Press Ctrl+C to stop." -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""

while ($true) {
    Log "Starting live engine..." $engineLog
    
    try {
        & cargo run --release 2>&1 | ForEach-Object {
            $line = "[$(Get-Date -Format 'HH:mm:ss')] $_"
            Write-Host $line
            Add-Content -Path $engineLog -Value $line
        }
    } catch {
        Log "Engine crash: $_" $engineLog
    }
    
    Log "Engine exited. Restarting in 10 seconds..." $engineLog
    Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Engine exited. Restarting in 10s..." -ForegroundColor Yellow
    Start-Sleep -Seconds 10
}
