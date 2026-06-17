# ============================================================================
#  SAVANT TRADING ENGINE — Pre-Push Validation
#  scripts/pre-push-validation.ps1
#
#  Runs cargo fmt, clippy, and tests before any push to remote.
#  Blocks the push if any check fails.
#
#  Called from .git/hooks/pre-push (Bash wrapper).
#
#  Performance: ~60-90s on a 354-test lib suite. Use cargo-nextest if slower.
# ============================================================================

$ErrorActionPreference = "Stop"
$ProjectRoot = (Get-Item $PSScriptRoot).Parent.FullName

Write-Host "━━━ Pre-push validation ━━━" -ForegroundColor Cyan

# ── Check 1: cargo fmt --check ─────────────────────────────────────
Write-Host "[1/3] Checking code formatting..." -ForegroundColor Cyan
cargo fmt --manifest-path "$ProjectRoot/Cargo.toml" --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "Code formatting violations detected. Run 'cargo fmt' and re-stage."
    exit 1
}
Write-Host "  [OK] Formatting clean" -ForegroundColor Green

# ── Check 2: cargo clippy --all-targets ────────────────────────────
Write-Host "[2/3] Running clippy..." -ForegroundColor Cyan
cargo clippy --manifest-path "$ProjectRoot/Cargo.toml" --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Error "Clippy warnings or compiler lints detected. Fix code issues."
    exit 1
}
Write-Host "  [OK] Clippy clean" -ForegroundColor Green

# ── Check 3: cargo test (workspace) ────────────────────────────────
# Use cargo-nextest if available (60% faster), fall back to cargo test.
$nextest = Get-Command cargo-nextest -ErrorAction SilentlyContinue
if ($nextest) {
    Write-Host "[3/3] Running test suite via cargo-nextest..." -ForegroundColor Cyan
    cargo nextest run --manifest-path "$ProjectRoot/Cargo.toml" --workspace --all-targets
} else {
    Write-Host "[3/3] Running test suite via cargo test..." -ForegroundColor Cyan
    cargo test --manifest-path "$ProjectRoot/Cargo.toml" --workspace --all-targets
}
if ($LASTEXITCODE -ne 0) {
    Write-Error "Tests failed. Build is unstable. Do not push."
    exit 1
}
Write-Host "  [OK] All tests pass" -ForegroundColor Green

Write-Host ""
Write-Host "━━━ All pre-push validations passed. Push approved. ━━━" -ForegroundColor Green
exit 0
