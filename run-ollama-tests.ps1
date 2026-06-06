# Ollama Local Model Testing for Savant Trading
#
# Prerequisites:
# 1. Install Ollama: https://ollama.ai
# 2. Start Ollama: ollama serve
# 3. Pull models (examples):
#    ollama pull qwen3:8b
#    ollama pull qwen3:30b-a3b
#    ollama pull gemma3:27b
#    ollama pull llama3.3:70b
#    ollama pull deepseek-r1:14b
#
#    For GGUF frankenstein models from HuggingFace (e.g. Jackrong/Qwopus):
#    1. Download the GGUF file from HuggingFace
#    2. Create a Modelfile: FROM ./Qwopus3.5-9B-Coder-Q4_K_M.gguf
#    3. Build: ollama create qwopus9b -f Modelfile
#
# Usage:
#   powershell -File run-ollama-tests.ps1

$ErrorActionPreference = "Continue"

# Models to test — add/remove based on what you've pulled
$models = @(
    @{ name = "qwen3:8b"; slug = "qwen3-8b"; concurrency = 1 },
    @{ name = "qwen3:30b-a3b"; slug = "qwen3-30b-a3b"; concurrency = 1 },
    @{ name = "gemma3:27b"; slug = "gemma3-27b"; concurrency = 1 },
    @{ name = "deepseek-r1:14b"; slug = "deepseek-r1-14b"; concurrency = 1 }
    # Add frankenstein models here after creating them:
    # @{ name = "qwopus9b"; slug = "qwopus-9b"; concurrency = 1 },
    # @{ name = "qwopus27b"; slug = "qwopus-27b"; concurrency = 1 },
)

$TIMEOUT_SECONDS = 600  # Local models are slower — 10 min timeout
$OLLAMA_ENDPOINT = "http://localhost:11434/v1"

# Check if Ollama is running
try {
    $null = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -Method Get -TimeoutSec 5
    Write-Host "Ollama is running."
} catch {
    Write-Host "ERROR: Ollama is not running. Start it with: ollama serve"
    exit 1
}

# List available models
$available = Invoke-RestMethod -Uri "http://localhost:11434/api/tags" -Method Get
Write-Host "Available models:"
foreach ($m in $available.models) {
    Write-Host "  - $($m.name) ($([math]::Round($m.size / 1GB, 1)) GB)"
}
Write-Host ""

$results = @{}

foreach ($model in $models) {
    $slug = $model.slug
    $modelName = $model.name
    $concurrency = $model.concurrency

    # Check if model is available
    $found = $available.models | Where-Object { $_.name -eq $modelName -or $_.name -like "$modelName*" }
    if (-not $found) {
        Write-Host "SKIP: $modelName not found. Pull it with: ollama pull $modelName"
        continue
    }

    Write-Host ""
    Write-Host "================================================================"
    Write-Host "  RUNNING: $modelName (concurrency=$concurrency, timeout=${TIMEOUT_SECONDS}s)"
    Write-Host "================================================================"
    Write-Host ""

    # Set env vars for the sandbox
    $env:SANDBOX_CONCURRENCY = $concurrency.ToString()
    $env:OLLAMA_MODEL = $modelName
    $env:OLLAMA_ENDPOINT = $OLLAMA_ENDPOINT

    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    # Run sandbox with Ollama provider
    # The engine reads config.ai.provider = "ollama" and config.ai.model = env:OLLAMA_MODEL
    $proc = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "--", "--test", "--sandbox", "--model", $modelName, "--provider", "ollama" `
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
            elapsed = $sw.Elapsed.TotalSeconds
        }
        continue
    }

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
    $freeformHits = ([regex]::Matches($output, 'FreeformExtraction')).Count

    $status = "COMPLETED"
    if ($avgScore -eq "N/A") { $status = "FAILED" }

    $results[$slug] = @{
        model = $modelName
        status = $status
        avg_score = $avgScore
        passed = $passed
        compliance = $compliance
        t2 = $t2
        t3 = $t3
        parse_errors = $parseErrors
        freeform_hits = $freeformHits
        elapsed = $sw.Elapsed.TotalSeconds
    }

    Write-Host ""
    Write-Host "  RESULT: $modelName — $status"
    Write-Host "  Score: $avgScore | Passed: $passed | Compliance: $compliance"
    Write-Host "  T2: $t2 | T3: $t3 | ParseErr: $parseErrors | Freeform: $freeformHits"
    Write-Host "  Time: $([Math]::Round($sw.Elapsed.TotalSeconds, 1))s"
    Write-Host ""

    if (Test-Path "data/sandbox_report.md") {
        Copy-Item "data/sandbox_report.md" "data/sandbox_report_$slug.md" -Force
    }
}

# Generate comparison report
Write-Host ""
Write-Host "================================================================"
Write-Host "  OLLAMA MODEL COMPARISON REPORT"
Write-Host "================================================================"
Write-Host ""

$report = "# Ollama Model Comparison — $(Get-Date -Format 'yyyy-MM-dd HH:mm')`n`n"
$report += "**Test:** 60-scenario sandbox | **Timeout:** ${TIMEOUT_SECONDS}s per model`n"
$report += "**Grading:** 3-tier rubric (Compliance + R:R + Reasoning)`n"
$report += "**Baseline:** MiMo v2.5 Pro via OpenRouter (0.70+ avg score)`n`n"
$report += "## Results`n`n"
$report += "| # | Model | Status | Score | Passed | T2 (R:R) | T3 (Reason) | ParseErr | Freeform | Time |`n"
$report += "|---|-------|--------|-------|--------|----------|-------------|----------|----------|------|`n"

$rank = 1
foreach ($slug in ($results.Keys | Sort-Object {
    $r = $results[$_]
    if ($r.status -eq "DISQUALIFIED") { -2 }
    elseif ($r.avg_score -eq "N/A" -or $r.avg_score -eq "DQ") { -1 }
    else { [double]$r.avg_score }
} -Descending)) {
    $r = $results[$slug]
    $statusCol = if ($r.status -eq "DISQUALIFIED") { "DQ: $($r.reason)" } else { $r.status }
    $report += "| $rank | $($r.model) | $statusCol | $($r.avg_score) | $($r.passed) | $($r.t2) | $($r.t3) | $($r.parse_errors) | $($r.freeform_hits) | $([Math]::Round($r.elapsed, 0))s |`n"
    $rank++
}

$report += "`n## Notes`n`n"
$report += "- **ParseErr**: Number of scenarios where JSON parsing failed (higher = model struggles with structured output)`n"
$report += "- **Freeform**: Number of scenarios where the freeform NLP parser was used as fallback`n"
$report += "- **MiMo baseline**: 0.70+ score, 0 parse errors, 0 freeform hits`n"
$report += "- A model scoring >0.50 with <5 parse errors is a viable local alternative`n"

$report | Out-File -FilePath "data/ollama-comparison-$(Get-Date -Format 'yyyy-MM-dd').md" -Encoding utf8
Write-Host $report
Write-Host ""
Write-Host "=== ALL OLLAMA RUNS COMPLETE ==="
