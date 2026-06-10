#!/usr/bin/env bash
# ===========================================================================
#  SAVANT TRADING ENGINE — Enterprise Build Pipeline
#  scripts/build.sh
#
#  A-Z build pipeline: validate → test → lint → release → stage → package
#
#  Usage:
#    ./scripts/build.sh               # Debug build + validate only (fast)
#    ./scripts/build.sh --release     # Full release build + NSIS installer
#    ./scripts/build.sh --package     # Stage files + NSIS installer only
#    ./scripts/build.sh --ci          # CI mode: strict, exit on first failure
#    ./scripts/build.sh --help        # Print usage
#
#  Requirements:
#    - Rust toolchain (rustc, cargo)         https://rustup.rs
#    - NSIS (makensis)                        https://nsis.sourceforge.io
#    - Optional: UPX (binary compression)     https://upx.github.io
# ===========================================================================

set -euo pipefail

# ── Project metadata ──────────────────────────────────────────────
PROJECT_NAME="savant-trading"
BINARY_NAME="savant"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep '^version' "${PROJECT_ROOT}/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')"
BUILD_DIR="${PROJECT_ROOT}/target"
STAGING_DIR="${PROJECT_ROOT}/dist/staging"
OUTPUT_DIR="${PROJECT_ROOT}/dist"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
# Auto-detect architecture from rustc target
ARCH_RAW="$(rustc -vV | grep 'host:' | sed 's/.*: //' | sed 's/-.*//')"
case "${ARCH_RAW}" in
    x86_64) ARCH="x64" ;;
    aarch64) ARCH="arm64" ;;
    i686) ARCH="x86" ;;
    *) ARCH="${ARCH_RAW}" ;;
esac

# ── Color output helpers ──────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'  # No Color

info()    { echo -e "${CYAN}[INFO]${NC}  $*"; }
ok()      { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()    { echo -e "${RED}[FAIL]${NC}  $*"; }
header()  { echo -e "\n${BOLD}━━━ $* ━━━${NC}"; }

# ── Flags ─────────────────────────────────────────────────────────
DO_RELEASE=false
DO_PACKAGE=false
CI_MODE=false
SKIP_TESTS=false
SKIP_LINT=false
SKIP_CHECKSUM=false
UPX_COMPRESS=false
NATIVE_BUILD=false

# ── Argument parsing ──────────────────────────────────────────────
usage() {
    cat << EOF
SAVANT TRADING ENGINE — Enterprise Build Pipeline  v${VERSION}

Usage:
  ./scripts/build.sh [options]

Options:
  --release       Full release build with LTO + NSIS package
  --package       Stage files and build NSIS installer only (skip compile)
  --ci            CI mode: strict, exit on first failure
  --skip-tests    Skip test execution
  --skip-lint     Skip clippy linting
  --skip-checksum Skip checksum generation
  --upx           Compress binary with UPX (requires upx installed)
  --native        Build with -C target-cpu=native (max perf, CPU-specific)
  --help, -h      Show this help message

Examples:
  ./scripts/build.sh                          # Fast debug validation
  ./scripts/build.sh --release                # Full release pipeline
  ./scripts/build.sh --release --upx          # Release + UPX compress
  ./scripts/build.sh --ci --release           # CI pipeline
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)       DO_RELEASE=true ;;
        --package)       DO_PACKAGE=true ;;
        --ci)            CI_MODE=true ;;
        --skip-tests)    SKIP_TESTS=true ;;
        --skip-lint)     SKIP_LINT=true ;;
        --skip-checksum) SKIP_CHECKSUM=true ;;
        --upx)           UPX_COMPRESS=true ;;
        --native)        NATIVE_BUILD=true ;;
        --help|-h)       usage ;;
        *)               fail "Unknown option: $1"; usage ;;
    esac
    shift
done

# ── Load .env ──────────────────────────────────────────────────────
if [[ -f "${PROJECT_ROOT}/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source <(grep -v '^#' "${PROJECT_ROOT}/.env" | grep '=')
    set +a
    ok ".env loaded (GITHUB_TOKEN=${GITHUB_TOKEN:+set})"
else
    warn ".env not found — API keys may be missing."
fi

# ── Pre-flight checks ─────────────────────────────────────────────
header "Pre-flight Checks"

# Rust toolchain
if ! command -v cargo &>/dev/null; then
    fail "cargo not found. Install Rust: https://rustup.rs"
    exit 1
fi
ok "cargo found: $(cargo --version | head -1)"

# Detect Rust channel
RUST_CHANNEL=$(rustc --version | awk '{print $2}')
RUST_VERSION=$(rustc --version | awk '{print $2}' | sed 's/-.*//')
info "rustc channel: ${RUST_CHANNEL}, version: ${RUST_VERSION}"

# NSIS (only required for --release or --package)
if [[ "$DO_RELEASE" == true || "$DO_PACKAGE" == true ]]; then
    NSIS_BIN=""
    if command -v makensis &>/dev/null; then
        NSIS_BIN="makensis"
    elif [[ -f "/c/Program Files (x86)/NSIS/makensis.exe" ]]; then
        NSIS_BIN="/c/Program Files (x86)/NSIS/makensis.exe"
    elif [[ -f "/c/Program Files/NSIS/makensis.exe" ]]; then
        NSIS_BIN="/c/Program Files/NSIS/makensis.exe"
    fi

    if [[ -n "$NSIS_BIN" ]]; then
        ok "NSIS found: $($NSIS_BIN //VERSION 2>/dev/null || $NSIS_BIN -version 2>/dev/null || echo 'available')"
    else
        warn "NSIS (makensis) not found — installer will not be built."
        warn "Download: https://nsis.sourceforge.io/Download"
        if [[ "$CI_MODE" == true ]]; then
            fail "NSIS required in CI mode."
            exit 1
        fi
    fi
fi

# UPX
if [[ "$UPX_COMPRESS" == true ]]; then
    if command -v upx &>/dev/null; then
        ok "UPX found: $(upx --version | head -1)"
    else
        warn "UPX not found — skipping compression."
        UPX_COMPRESS=false
    fi
fi

# ── Phase 1: cargo check (fast compile check) ─────────────────────
header "Phase 1: Cargo Check (fast validation)"
if [[ "$DO_PACKAGE" == false ]]; then
    if cargo check --manifest-path "${PROJECT_ROOT}/Cargo.toml" 2>&1; then
        ok "cargo check passed"
    else
        fail "cargo check failed"
        [[ "$CI_MODE" == true ]] && exit 1
    fi
else
    info "Skipping cargo check (--package mode)"
fi

# ── Phase 2: cargo test ───────────────────────────────────────────
header "Phase 2: Unit Tests"
if [[ "$DO_PACKAGE" == false && "$SKIP_TESTS" == false ]]; then
    # Run DEX tests specifically (fastest coverage for core trading)
    if cargo test --lib dex --manifest-path "${PROJECT_ROOT}/Cargo.toml" 2>&1; then
        ok "DEX tests passed"
    else
        fail "DEX tests failed"
        [[ "$CI_MODE" == true ]] && exit 1
    fi

    # Full test suite
    if cargo test --manifest-path "${PROJECT_ROOT}/Cargo.toml" 2>&1; then
        ok "All tests passed"
    else
        fail "Some tests failed"
        [[ "$CI_MODE" == true ]] && exit 1
    fi
else
    info "Skipping tests"
fi

# ── Phase 3: Clippy linting ───────────────────────────────────────
header "Phase 3: Clippy Lint Gate"
if [[ "$DO_PACKAGE" == false && "$SKIP_LINT" == false ]]; then
    if cargo clippy --manifest-path "${PROJECT_ROOT}/Cargo.toml" -- -D warnings 2>&1; then
        ok "Clippy passed with zero warnings"
    else
        fail "Clippy found issues"
        [[ "$CI_MODE" == true ]] && exit 1
    fi
else
    info "Skipping clippy"
fi

# ── Phase 4: Release Build ────────────────────────────────────────
header "Phase 4: Build Binary"

RELEASE_FLAGS=""
BUILD_PROFILE="debug"
BINARY_PATH="${BUILD_DIR}/debug/${BINARY_NAME}.exe"

if [[ "$DO_RELEASE" == true ]]; then
    BUILD_PROFILE="release"
    BINARY_PATH="${BUILD_DIR}/release/${BINARY_NAME}.exe"
    RELEASE_FLAGS="--release"

    info "Building release binary with LTO..."
    # RUSTFLAGS for full LTO and optimization
    # NOTE: -C link-arg=-s omitted — MSVC link.exe doesn't accept -s.
    # CARGO_PROFILE_RELEASE_STRIP handles symbol stripping cross-platform.
    # NOTE: -C target-cpu=native only used with --native flag.
    # For distribution binaries this is OFF by default to ensure
    # compatibility across all x86-64 CPUs (no AVX2/AVX-512 assumptions).
    if [[ "$NATIVE_BUILD" == true ]]; then
        export RUSTFLAGS="-C target-cpu=native ${RUSTFLAGS:-}"
        info "Native CPU optimizations enabled (--native)"
    fi
    # Statically link the MSVC CRT so the binary runs on clean Windows
    # installs without requiring the VC++ Redistributable.
    export RUSTFLAGS="-C target-feature=+crt-static ${RUSTFLAGS:-}"
    export CARGO_PROFILE_RELEASE_LTO="fat"
    export CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1"
    export CARGO_PROFILE_RELEASE_OPT_LEVEL="z"
    export CARGO_PROFILE_RELEASE_STRIP="symbols"
    export CARGO_PROFILE_RELEASE_PANIC="abort"
fi

if [[ "$DO_PACKAGE" == false ]]; then
    if cargo build ${RELEASE_FLAGS} --manifest-path "${PROJECT_ROOT}/Cargo.toml" 2>&1; then
        ok "Build successful (${BUILD_PROFILE})"
        ls -lh "${BINARY_PATH}"
    else
        fail "Build failed"
        exit 1
    fi
else
    info "Skipping build (--package mode), using existing binary"
    if [[ ! -f "${BINARY_PATH}" ]]; then
        fail "Binary not found at ${BINARY_PATH}. Run without --package first."
        exit 1
    fi
    ok "Using existing binary: ${BINARY_PATH}"
fi

# ── UPX Compression ───────────────────────────────────────────────
if [[ "$UPX_COMPRESS" == true && -f "${BINARY_PATH}" ]]; then
    header "UPX Compression"
    if upx --best --lzma "${BINARY_PATH}" 2>&1; then
        ok "UPX compression completed"
        ls -lh "${BINARY_PATH}"
    else
        warn "UPX compression failed (non-fatal)"
    fi
fi

# ── Phase 5: Stage Files for Installer ────────────────────────────
header "Phase 5: Stage Files"

if [[ "$DO_RELEASE" == true || "$DO_PACKAGE" == true ]]; then
    # Clean staging directory
    rm -rf "${STAGING_DIR}"
    mkdir -p "${STAGING_DIR}/config"
    mkdir -p "${STAGING_DIR}/knowledge"
    mkdir -p "${STAGING_DIR}/data"
    mkdir -p "${STAGING_DIR}/docs"

    # Binary
    cp "${BINARY_PATH}" "${STAGING_DIR}/${BINARY_NAME}.exe"
    ok "Staged binary"

    # Config
    cp "${PROJECT_ROOT}/config/default.toml" "${STAGING_DIR}/config/"
    ok "Staged config"

    # Knowledge files
    if ls "${PROJECT_ROOT}/knowledge/"*.json 1>/dev/null 2>&1; then
        cp "${PROJECT_ROOT}/knowledge/"*.json "${STAGING_DIR}/knowledge/"
        ok "Staged knowledge files ($(ls -1 "${PROJECT_ROOT}/knowledge/"*.json | wc -l) files)"
    fi

    # License
    if [[ -f "${PROJECT_ROOT}/LICENSE" ]]; then
        cp "${PROJECT_ROOT}/LICENSE" "${STAGING_DIR}/docs/"
        ok "Staged LICENSE"
    fi

    # Documentation
    for doc in README.md CHANGELOG.md API-KEYS.md DESIGN.md ECHO.md MIGRATION.md STARTER-PROMPT.md; do
        if [[ -f "${PROJECT_ROOT}/${doc}" ]]; then
            cp "${PROJECT_ROOT}/${doc}" "${STAGING_DIR}/docs/"
        fi
    done
    ok "Staged documentation"

    # VERSION file
    echo "${VERSION}" > "${STAGING_DIR}/VERSION"
    ok "Staged VERSION file"

    # Generate env template
    cat > "${STAGING_DIR}/.env.example" << 'ENV_EOF'
# ===========================================================================
#  SAVANT TRADING ENGINE — Environment Variables
# ===========================================================================
# Copy this file to .env and fill in your API keys.
# See API-KEYS.md for details on obtaining each key.

# === REQUIRED: AI Provider ===
# OPENGATEWAY_API_KEY=sk-your-key-here

# === OPTIONAL: Kraken CEX (for live trading with KYC) ===
# KRAKEN_API_KEY=your-kraken-api-key
# KRAKEN_API_SECRET=your-kraken-api-secret

# === OPTIONAL: DEX Trading (no KYC, Arbitrum) ===
# WALLET_PRIVATE_KEY=0x-your-ethereum-private-key
# ZEROEX_API_KEY=your-0x-api-key
# 1INCH_API_KEY=your-1inch-api-key

# === OPTIONAL: Market Data ===
# COINMARKETCAP_API_KEY=your-coinmarketcap-api-key
ENV_EOF
    ok "Staged .env.example"

    # Generate run scripts
    cat > "${STAGING_DIR}/run.bat" << 'RUN_EOF'
@echo off
REM ===========================================================================
REM  SAVANT TRADING ENGINE — Quick Start Script
REM ===========================================================================
REM Usage:
REM   run.bat              Start engine + API server (paper trading default)
REM   run.bat --tui        Start with full-screen multi-tab TUI
REM   run.bat backtest     Run backtest on historical data
REM   run.bat --help       Show all commands
REM ===========================================================================

%~dp0savant.exe %*
RUN_EOF

    cat > "${STAGING_DIR}/run-tui.bat" << 'TUI_EOF'
@echo off
REM SAVANT TRADING ENGINE — TUI Mode (full-screen terminal)
%~dp0savant.exe --tui
TUI_EOF
    ok "Staged run scripts"

    # Summary
    echo ""
    info "Staged files:"
    find "${STAGING_DIR}" -type f | sed "s|${STAGING_DIR}|  dist/staging|" | sort
    echo ""

    # ── Phase 6: Build NSIS Installer ────────────────────────────
    header "Phase 6: NSIS Installer"

    if [[ -n "${NSIS_BIN:-}" ]]; then
        NSIS_SCRIPT="${PROJECT_ROOT}/scripts/savant.nsi"
        INSTALLER_NAME="SavantTrading-${VERSION}-${ARCH}-Setup.exe"
        INSTALLER_PATH="${OUTPUT_DIR}/${INSTALLER_NAME}"

        if [[ -f "$NSIS_SCRIPT" ]]; then
            info "Compiling NSIS installer..."
            info "Script: ${NSIS_SCRIPT}"
            info "Output: ${INSTALLER_PATH}"

            # Pass version as define to NSIS
            if "${NSIS_BIN}" \
                -DVERSION="${VERSION}" \
                -DSRC_DIR="${STAGING_DIR}" \
                -DOUT_DIR="${OUTPUT_DIR}" \
                "${NSIS_SCRIPT}" 2>&1; then

                if [[ -f "${INSTALLER_PATH}" ]]; then
                    ok "Installer created: ${INSTALLER_PATH}"
                    ls -lh "${INSTALLER_PATH}"
                else
                    warn "Installer may have been created with a different name."
                    find "${OUTPUT_DIR}" -name "SavantTrading-*-Setup.exe" -exec ls -lh {} \;
                fi
            else
                fail "NSIS compilation failed"
                warn "Check NSIS script syntax. Ensure all required plugins are installed."
                [[ "$CI_MODE" == true ]] && exit 1
            fi
        else
            fail "NSIS script not found: ${NSIS_SCRIPT}"
            [[ "$CI_MODE" == true ]] && exit 1
        fi
    else
        warn "NSIS not available — skipping installer."
        warn "To create installer manually:"
        warn "  1. Install NSIS from https://nsis.sourceforge.io/Download"
        warn "  2. Run: makensis -DVERSION=${VERSION} -DSRC_DIR=\"${STAGING_DIR}\" -DOUT_DIR=\"${OUTPUT_DIR}\" scripts/savant.nsi"
    fi

    # ── Phase 7: Checksums ───────────────────────────────────────
    header "Phase 7: Checksums"

    if [[ "$SKIP_CHECKSUM" == false ]]; then
        mkdir -p "${OUTPUT_DIR}"

        # SHA-256 checksums for all distributable artifacts
        CHECKSUM_FILE="${OUTPUT_DIR}/SavantTrading-${VERSION}-${ARCH}-checksums.txt"
        > "${CHECKSUM_FILE}"

        # Stage directory checksums (everything that goes into the installer)
        if [[ -d "${STAGING_DIR}" ]]; then
            info "Generating checksums for staged files..."
            find "${STAGING_DIR}" -type f -exec sha256sum {} \; >> "${CHECKSUM_FILE}"
        fi

        # Installer checksum
        if [[ -f "${INSTALLER_PATH:-}" ]]; then
            sha256sum "${INSTALLER_PATH}" >> "${CHECKSUM_FILE}"
        fi

        # Binary checksum
        if [[ -f "${BINARY_PATH}" ]]; then
            sha256sum "${BINARY_PATH}" >> "${CHECKSUM_FILE}"
        fi

        if [[ -s "${CHECKSUM_FILE}" ]]; then
            ok "Checksums written to ${CHECKSUM_FILE}"
            cat "${CHECKSUM_FILE}"
        else
            warn "No checksums generated (no artifacts found)"
        fi
    fi

    # ── Summary ───────────────────────────────────────────────────
    header "Build Summary"
    echo ""
    echo "  Project:    ${PROJECT_NAME} v${VERSION}"
    echo "  Profile:    ${BUILD_PROFILE}"
    echo "  Binary:     ${BINARY_PATH}"
    echo "  Staging:    ${STAGING_DIR} ($(find "${STAGING_DIR}" -type f | wc -l) files)"
    echo "  Output:     ${OUTPUT_DIR}/"
    echo ""

    if [[ -f "${INSTALLER_PATH:-}" ]]; then
        echo -e "  ${GREEN}Installer:   ${INSTALLER_PATH}${NC}"
    fi
    echo ""
    ok "Build pipeline complete."

else
    # Fast mode (no release, no package)
    header "Fast Build Summary"
    echo ""
    echo "  Project:  ${PROJECT_NAME} v${VERSION}"
    echo "  Profile:  ${BUILD_PROFILE}"
    echo "  Binary:   ${BINARY_PATH}"
    echo ""
    ok "Validation complete. Use --release for full package."
fi
