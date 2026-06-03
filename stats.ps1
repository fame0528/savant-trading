# SAVANT - performance scoreboard (read-only).
# Reads data/alerts.jsonl and prints a clean win/loss + net PnL summary.
#
#   .\stats.ps1                                  # all valid records
#   .\stats.ps1 -Since "2026-06-03T11:00:00"     # only trades after a cutoff (UTC)
#
# Notes:
#  - The single source of truth for "are we up?" is your live Kraken USD balance
#    (the engine logs "Kraken balance: " each sync; or the API /portfolio).
#  - Records with exit_price = 0 or pnl_pct <= -99 are phantom artifacts from the
#    pre-fix code and are excluded automatically.

param(
    [string]$Since = "",
    [string]$File  = "data/alerts.jsonl"
)

if (-not (Test-Path $File)) {
    Write-Host "No alerts file at $File yet - no trades recorded." -ForegroundColor Yellow
    return
}

$rows = Get-Content $File | Where-Object { $_.Trim() -ne "" } | ForEach-Object {
    try { $_ | ConvertFrom-Json } catch { $null }
} | Where-Object { $_ -ne $null }

if ($Since -ne "") {
    $cut = [datetime]::Parse($Since)
    $rows = $rows | Where-Object { ([datetime]$_.timestamp) -ge $cut }
}

$opened = @($rows | Where-Object { $_.type -eq 'TRADE_OPENED' })
$closedAll = @($rows | Where-Object { $_.type -eq 'TRADE_CLOSED' })
$closed = @($closedAll | Where-Object { $_.exit_price -ne 0 -and $_.pnl_pct -gt -99 })
$dropped = $closedAll.Count - $closed.Count

$wins   = @($closed | Where-Object { $_.pnl -gt 0 })
$losses = @($closed | Where-Object { $_.pnl -le 0 })
$net    = ($closed | Measure-Object -Property pnl -Sum).Sum
if ($null -eq $net) { $net = 0 }
$winRate = if ($closed.Count -gt 0) { [math]::Round(100 * $wins.Count / $closed.Count, 1) } else { 0 }
$avgWin  = if ($wins.Count   -gt 0) { [math]::Round(($wins   | Measure-Object -Property pnl -Sum).Sum / $wins.Count, 4) } else { 0 }
$avgLoss = if ($losses.Count -gt 0) { [math]::Round(($losses | Measure-Object -Property pnl -Sum).Sum / $losses.Count, 4) } else { 0 }

Write-Host ""
Write-Host "================ SAVANT SCOREBOARD ================" -ForegroundColor Cyan
if ($Since -ne "") { Write-Host "Since: $Since (UTC)" -ForegroundColor DarkGray }
Write-Host ("Positions opened : {0}" -f $opened.Count)
Write-Host ("Trades closed    : {0}" -f $closed.Count)
Write-Host ("  Wins           : {0}" -f $wins.Count)   -ForegroundColor Green
Write-Host ("  Losses         : {0}" -f $losses.Count) -ForegroundColor Red
Write-Host ("Win rate         : {0}%" -f $winRate)
Write-Host ("Avg win          : {0} USD" -f $avgWin)  -ForegroundColor Green
Write-Host ("Avg loss         : {0} USD" -f $avgLoss) -ForegroundColor Red
$col = if ($net -ge 0) { 'Green' } else { 'Red' }
Write-Host ("Net PnL (approx) : {0} USD" -f ([math]::Round($net,4))) -ForegroundColor $col
if ($dropped -gt 0) { Write-Host ("(excluded {0} pre-fix phantom records)" -f $dropped) -ForegroundColor DarkGray }
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "Truth check: compare your live Kraken USD balance to your starting amount." -ForegroundColor DarkGray
Write-Host ""

if ($closed.Count -gt 0) {
    Write-Host "Last 8 closed:" -ForegroundColor Cyan
    $closed | Select-Object -Last 8 | ForEach-Object {
        $c = if ($_.pnl -ge 0) { 'Green' } else { 'Red' }
        Write-Host ("  {0,-10} {1,6}%  {2,9} USD  {3}" -f $_.pair, [math]::Round($_.pnl_pct,2), [math]::Round($_.pnl,3), $_.notes) -ForegroundColor $c
    }
    Write-Host ""
}
