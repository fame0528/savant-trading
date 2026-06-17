# Savant Trading - Repository Backup Script
# Mirrors GitHub to a local bare clone. GitHub is the only remote; this is just
# local insurance against accidental local damage (rm -rf, .git corruption, etc).
#
# Schedule: Run daily at 3 AM (Windows Task Scheduler)
# Register: schtasks /create /tn "SavantRepositoryBackup" /tr "pwsh.exe -NoProfile -File C:\Users\spenc\dev\savant-trading\scripts\Backup-Repository.ps1" /sc daily /st 03:00

$ErrorActionPreference = "Stop"

$ProjectRoot = "C:\Users\spenc\dev\savant-trading"
$BackupPath = "$env:USERPROFILE\backups\savant-trading.git"
$PrimarySource = "git@github.com:fame0528/savant-trading.git"

Write-Host "Initializing backup mirror cycle..." -ForegroundColor Cyan

# Update local bare mirror
if (Test-Path -Path $BackupPath) {
    Set-Location $BackupPath
    git remote update
} else {
    New-Item -ItemType Directory -Path (Split-Path $BackupPath) -Force | Out-Null
    git clone --mirror $PrimarySource $BackupPath
    Set-Location $BackupPath
}

if ($LASTEXITCODE -ne 0) {
    Write-Error "Local mirror update failed. Backup aborted."
    exit 1
}

Write-Host ""
Write-Host "Backup verification complete." -ForegroundColor Green
Write-Host "  Local mirror: $BackupPath"
Write-Host "  Last mirror:  $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz')"
