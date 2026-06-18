# Test NVIDIA NIM integration with DeepSeek-V4-Flash
# Verifies the API key works and we can get a structured JSON response.

$ErrorActionPreference = "Stop"

$envPath = "C:\Users\spenc\dev\savant-trading\.env"
$apiKey = (Get-Content $envPath | Select-String -Pattern 'NVIDIA_API_KEY=(.+)' | ForEach-Object { $_.Matches[0].Groups[1].Value }).Trim()
$endpoint = "https://integrate.api.nvidia.com/v1"
$model = "deepseek-ai/deepseek-v4-flash"

if (-not $apiKey) {
    Write-Error "NVIDIA_API_KEY not found in .env"
    exit 1
}

Write-Host "Testing NVIDIA NIM integration..." -ForegroundColor Cyan
Write-Host "  Endpoint: $endpoint"
Write-Host "  Model: $model"
Write-Host "  Key prefix: $($apiKey.Substring(0, 12))..."
Write-Host ""

$body = @{
    model = $model
    messages = @(
        @{
            role = "user"
            content = 'You are a crypto trading agent. Output ONLY a JSON object with: action (Buy/Sell/Hold), pair, conviction_score (0.0-1.0), reasoning. The market is ranging with ADX 12, RSI 50, ATR compressed. Decide.'
        }
    )
    temperature = 0.6
    max_tokens = 500
} | ConvertTo-Json -Depth 5

try {
    $response = Invoke-RestMethod -Uri "$endpoint/chat/completions" -Method Post -Headers @{
        "Authorization" = "Bearer $apiKey"
        "Content-Type" = "application/json"
    } -Body $body -TimeoutSec 60

    Write-Host "Response received:" -ForegroundColor Green
    Write-Host "  Model used: $($response.model)"
    Write-Host "  Tokens: prompt=$($response.usage.prompt_tokens) completion=$($response.usage.completion_tokens)"
    Write-Host "  Content:" -ForegroundColor Green
    Write-Host $response.choices[0].message.content
}
catch {
    Write-Error "NVIDIA NIM call failed: $_"
    Write-Host "Response body:" -ForegroundColor Yellow
    if ($_.Exception.Response) {
        $reader = [System.IO.StreamReader]::new($_.Exception.Response.GetResponseStream())
        Write-Host $reader.ReadToEnd()
    }
    exit 1
}
