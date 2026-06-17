# Savant Trading - Repository Backup Script
# Mirrors GitHub to a local bare clone, pushes to a secondary remote (GitLab or Gitea)
#
# Setup (one-time):
#   1. Create a private repo on GitLab: https://gitlab.com/projects/new
#      (or self-hosted Gitea)
#   2. Add the local SSH public key ($env:USERPROFILE\.ssh\id_ed25519.pub)
#      to GitLab: https://gitlab.com/-/profile/keys
#   3. Add the secondary remote: git remote add backup git@gitlab.com:YOUR_USER/savant-trading.git
#   4. Test: git remote update backup
#
# Schedule: Run daily at 3 AM (Windows Task Scheduler)
# Register: schtasks /create /tn "SavantRepositoryBackup" /tr "pwsh.exe -NoProfile -File C:\Users\spenc\dev\savant-trading\scripts\Backup-Repository.ps1" /sc daily /st 03:00

$ErrorActionPreference = "Stop"

$ProjectRoot = "C:\Users\spenc\dev\savant-trading"
$BackupPath = "$env:USERPROFILE\backups\savant-trading.git"
$PrimarySource = "git@github.com:fame0528/savant-trading.git"
$SecondaryRemoteName = "backup"

Write-Host "Initializing backup mirror cycle..." -ForegroundColor Cyan

# Step 1: Update local bare mirror
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

# Step 2: Push to secondary remote (if configured)
$remotes = git remote
if ($remotes -contains $SecondaryRemoteName) {
    Write-Host "Mirroring backup state to secondary target ($SecondaryRemoteName)..." -ForegroundColor Cyan
    git push --mirror $SecondaryRemoteName
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Secondary push failed. Check remote URL and SSH key."
        exit 1
    }
    Write-Host "  [OK] Pushed to $SecondaryRemoteName" -ForegroundColor Green
} else {
    Write-Warning "Secondary remote '$SecondaryRemoteName' not configured."
    Write-Warning "Add with: git remote add $SecondaryRemoteName git@gitlab.com:YOUR_USER/savant-trading.git"
    Write-Warning "Local mirror updated but no remote backup performed."
}

Write-Host ""
Write-Host "Backup verification complete." -ForegroundColor Green
Write-Host "  Local mirror: $BackupPath"
Write-Host "  Last mirror:  $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz')"
