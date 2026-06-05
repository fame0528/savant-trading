$ErrorActionPreference = "Continue"
$env:GOOGLE_API_KEY = "AQ.Ab8RN6LJHjsdJ46zwhFI50h-iNtR-JQM8k3lze780OZu9crFpg"

$TIMEOUT_SECONDS = 180
$models = @(
    @{ name = "nvidia/nemotron-3-nano-30b-a3b:free"; slug = "nemotron-nano"; concurrency = 10 },
    @{ name = "nvidia/nemotron-3-super-120b-a12b:free"; slug = "nemotron-super"; concurrency = 5 },
    @{ name = "google/gemma-4-31b-it:free"; slug = "gemma-4-31b"; concurrency = 5 },
    @{ name = "google/gemma-4-26b-a4b-it:free"; slug = "gemma-4-26b"; concurrency = 5 },
    @{ name = "moonshotai/kimi-k2.6:free"; slug = "kimi-k2.6"; concurrency = 1 }
)

$results = @{}

foreach ($model in $models) {
    $slug = $model.slug
    $modelName = $model.name
    $concurrency = $model.concurrency

    Write-Host ""
    Write-Host "================================================================"
    Write-Host "  RUNNING: $modelName (concurrency=$concurrency, timeout=${TIMEOUT_SECONDS}s)"
    Write-Host "================================================================"
    Write-Host ""

    $env:SANDBOX_CONCURRENCY = $concurrency.ToString()

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "--test", "--sandbox", "--model", $modelName `
        -NoNewWindow -PassThru -RedirectStandardOutput "data/sandbox_stdout_$slug.txt" `
        -RedirectStandardError "data/sandbox_stderr_$slug.txt" `
        -WorkingDirectory (Get-Location).Path

    $completed = $proc.WaitForExit($TIMEOUT_SECONDS * 1000)
    $sw.Stop()

    if (-not $completed) {
        Write-Host "  TIMEOUT after ${TIMEOUT_SECONDS}s — DISQUALIFIED"
        $proc | Stop-Process -Force -ErrorAction SilentlyContinue
        $proc | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue

        $results[$slug] = @{
            model = $modelName
            status = "DISQUALIFIED"
            reason = "Timeout after ${TIMEOUT_SECONDS}s"
            avg_score = "DQ"
            passed = "DQ"
            compliance = "DQ"
            t2 = "DQ"
            t3 = "DQ"
            parse_errors = 0
            llm_errors = 0
            retried = 0
            elapsed = $sw.Elapsed.TotalSeconds
        }
        continue
    }

    # Read output
    $output = ""
    if (Test-Path "data/sandbox_stdout_$slug.txt") {
        $output = Get-Content "data/sandbox_stdout_$slug.txt" -Raw -ErrorAction SilentlyContinue
    }

    $exitCode = $proc.ExitCode
    if ($exitCode -ne 0 -and -not ($output -match 'SANDBOX COMPLETE')) {
        Write-Host "  CRASHED (exit code $exitCode) — DISQUALIFIED"
        $results[$slug] = @{
            model = $modelName
            status = "DISQUALIFIED"
            reason = "Process crashed (exit code $exitCode)"
            avg_score = "DQ"
            passed = "DQ"
            compliance = "DQ"
            t2 = "DQ"
            t3 = "DQ"
            parse_errors = 0
            llm_errors = 0
            retried = 0
            elapsed = $sw.Elapsed.TotalSeconds
        }
        continue
    }

    $output | Out-File -FilePath "data/sandbox_output_$slug.txt" -Encoding utf8

    $avgScore = if ($output -match 'Average Score:\s+([\d.]+)') { $Matches[1] } else { "N/A" }
    $passed = if ($output -match 'Passed:\s+(\d+)\s*/\s*(\d+)') { "$($Matches[1])/$($Matches[2])" } else { "N/A" }
    $compliance = if ($output -match 'Compliance Ratio:\s+(\d+)%') { "$($Matches[1])%" } else { "N/A" }
    $t2 = if ($output -match 'R:R Score:\s+([\d.]+)') { $Matches[1] } else { "N/A" }
    $t3 = if ($output -match 'Reasoning:\s+([\d.]+)') { $Matches[1] } else { "N/A" }
    $parseErrors = ([regex]::Matches($output, 'ParseError')).Count
    $llmErrors = ([regex]::Matches($output, 'LLMError')).Count
    $retried = if ($output -match 'Retrying (\d+) rate-limited') { $Matches[1] } else { "0" }

    $status = "COMPLETED"
    if ($avgScore -eq "N/A") { $status = "FAILED" }

    $results[$slug] = @{
        model = $modelName
        status = $status
        reason = ""
        avg_score = $avgScore
        passed = $passed
        compliance = $compliance
        t2 = $t2
        t3 = $t3
        parse_errors = $parseErrors
        llm_errors = $llmErrors
        retried = $retried
        elapsed = $sw.Elapsed.TotalSeconds
    }

    Write-Host ""
    Write-Host "  RESULT: $modelName — $status"
    Write-Host "  Score: $avgScore | Passed: $passed | Compliance: $compliance"
    Write-Host "  T2: $t2 | T3: $t3 | ParseErr: $parseErrors | LLMErr: $llmErrors | Retried: $retried"
    Write-Host "  Time: $([Math]::Round($sw.Elapsed.TotalSeconds, 1))s"
    Write-Host ""

    if (Test-Path "data/sandbox_report.md") {
        Copy-Item "data/sandbox_report.md" "data/sandbox_report_$slug.md" -Force
    }
    if (Test-Path "data/sandbox_reports/latest.md") {
        Copy-Item "data/sandbox_reports/latest.md" "data/sandbox_reports/${slug}-latest.md" -Force
    }
}

# Generate comparison report
Write-Host ""
Write-Host "================================================================"
Write-Host "  MODEL COMPARISON REPORT"
Write-Host "================================================================"
Write-Host ""

$report = "# Model Comparison Report — $(Get-Date -Format 'yyyy-MM-dd HH:mm')`n`n"
$report += "**Test:** 60-scenario sandbox | **Timeout:** ${TIMEOUT_SECONDS}s per model`n"
$report += "**Grading:** 3-tier rubric (Compliance + R:R + Reasoning)`n`n"
$report += "## Results`n`n"
$report += "| # | Model | Status | Score | Passed | Compliance | T2 (R:R) | T3 (Reason) | Retried | Time |`n"
$report += "|---|-------|--------|-------|--------|------------|----------|-------------|---------|------|`n"

$rank = 1
foreach ($slug in ($results.Keys | Sort-Object {
    $r = $results[$_]
    if ($r.status -eq "DISQUALIFIED") { -2 }
    elseif ($r.avg_score -eq "N/A" -or $r.avg_score -eq "DQ") { -1 }
    else { [double]$r.avg_score }
} -Descending)) {
    $r = $results[$slug]
    $statusCol = if ($r.status -eq "DISQUALIFIED") { "DQ: $($r.reason)" } else { $r.status }
    $report += "| $rank | $($r.model) | $statusCol | $($r.avg_score) | $($r.passed) | $($r.compliance) | $($r.t2) | $($r.t3) | $($r.retried) | $([Math]::Round($r.elapsed, 0))s |`n"
    $rank++
}

$report += "`n## Disqualification Log`n`n"
$dqCount = 0
foreach ($slug in $results.Keys) {
    $r = $results[$slug]
    if ($r.status -eq "DISQUALIFIED") {
        $dqCount++
        $report += "- **$($r.model)**: $($r.reason)`n"
    }
}
if ($dqCount -eq 0) { $report += "None.`n" }

$report | Out-File -FilePath "data/model-comparison-$(Get-Date -Format 'yyyy-MM-dd').md" -Encoding utf8
Write-Host $report
Write-Host ""
Write-Host "=== ALL RUNS COMPLETE ==="
