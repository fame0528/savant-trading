# Savant Trading - Task Scheduler Registration
#
# Local backup only. GitHub is the source of truth; the bare clone at
# $env:USERPROFILE\backups\savant-trading.git is local insurance against
# accidental local damage. No secondary remote.
#
# Register the daily backup task (run as Administrator):
#   $action = New-ScheduledTaskAction -Execute 'pwsh.exe' -Argument '-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File C:\Users\spenc\dev\savant-trading\scripts\Backup-Repository.ps1'
#   $trigger = New-ScheduledTaskTrigger -Daily -At 3:00AM
#   Register-ScheduledTask -TaskName "SavantRepositoryBackup" -Action $action -Trigger $trigger -Description "Daily local mirror of Savant Trading repo"
#
# Or via schtasks (one-liner, also requires elevation):
#   schtasks /create /tn "SavantRepositoryBackup" /tr "pwsh.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File C:\Users\spenc\dev\savant-trading\scripts\Backup-Repository.ps1" /sc daily /st 03:00 /rl highest /f
