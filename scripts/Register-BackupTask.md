# Savant Trading - Task Scheduler Registration
#
# Run these commands in an elevated PowerShell prompt to register the daily backup.
#
# 1. One-time setup: create the secondary remote (GitLab or self-hosted Gitea)
#    git remote add backup git@gitlab.com:YOUR_USER/savant-trading.git
#    git remote update backup  # test
#
# 2. Register the daily backup task (run as Administrator):
#    $action = New-ScheduledTaskAction -Execute 'pwsh.exe' -Argument '-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File C:\Users\spenc\dev\savant-trading\scripts\Backup-Repository.ps1'
#    $trigger = New-ScheduledTaskTrigger -Daily -At 3:00AM
#    Register-ScheduledTask -TaskName "SavantRepositoryBackup" -Action $action -Trigger $trigger -Description "Daily mirror of Savant Trading repo to secondary host"
#
# 3. Verify:
#    Get-ScheduledTask -TaskName "SavantRepositoryBackup"
#
# 4. Run manually to test:
#    Start-ScheduledTask -TaskName "SavantRepositoryBackup"
#    Get-ScheduledTask -TaskName "SavantRepositoryBackup" | Select-Object LastRunTime, LastTaskResult

# Or via schtasks (one-liner, also requires elevation):
# schtasks /create /tn "SavantRepositoryBackup" /tr "pwsh.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File C:\Users\spenc\dev\savant-trading\scripts\Backup-Repository.ps1" /sc daily /st 03:00 /rl highest /f
