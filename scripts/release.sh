#!/usr/bin/env bash
# ===========================================================================
#  SAVANT TRADING ENGINE — Release Automation
#  scripts/release.sh
#
#  One-command release: tag → push → create GitHub release with changelog notes.
#
#  Usage:
#    ./scripts/release.sh                    # Release current version
#    ./scripts/release.sh 0.14.0             # Release specific version
#    ./scripts/release.sh --dry-run          # Show what would happen
#    ./scripts/release.sh --help             # Show usage
#
#  Requirements:
#    - gh CLI (GitHub CLI)                    https://cli.github.com
#    - GITHUB_TOKEN in .env or environment
# ===========================================================================

set -euo pipefail

# ── Project metadata ──────────────────────────────────────────────
PROJECT_NAME="savant-trading"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CURRENT_VERSION="$(cat "${PROJECT_ROOT}/VERSION" 2>/dev/null || grep '^version' "${PROJECT_ROOT}/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')"
REPO_SLUG="fame0528/savant-trading"

# ── Color output helpers ──────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${CYAN}[INFO]${NC}  $*"; }
ok()      { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()    { echo -e "${RED}[FAIL]${NC}  $*"; exit 1; }
header()  { echo -e "\n${BOLD}━━━ $* ━━━${NC}"; }

# ── Load .env ──────────────────────────────────────────────────────
if [[ -f "${PROJECT_ROOT}/.env" ]]; then
    set -a
    source <(grep -v '^#' "${PROJECT_ROOT}/.env" | grep '=')
    set +a
fi

# ── Argument parsing ──────────────────────────────────────────────
DRY_RUN=false
NEW_VERSION=""

usage() {
    cat << EOF
SAVANT TRADING ENGINE — Release Automation  v${CURRENT_VERSION}

Usage:
  ./scripts/release.sh [VERSION] [options]

Arguments:
  VERSION           New version number (e.g. 0.14.0). Defaults to CURRENT_VERSION.

Options:
  --dry-run         Show what would happen without executing
  --help, -h        Show this help message

Examples:
  ./scripts/release.sh                    # Release current version (${CURRENT_VERSION})
  ./scripts/release.sh 0.14.0             # Release v0.14.0
  ./scripts/release.sh --dry-run          # Preview without executing

Steps performed:
  1. Validate working tree is clean
  2. Run clippy + tests (pre-flight gate)
  3. Update VERSION file if new version provided
  4. Create git tag v{VERSION}
  5. Push tag to origin
  6. Extract release notes from CHANGELOG.md
  7. Create GitHub release via gh CLI
EOF
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)   DRY_RUN=true; shift ;;
        --help|-h)   usage ;;
        [0-9]*)      NEW_VERSION="$1"; shift ;;
        *)           fail "Unknown argument: $1. Use --help for usage." ;;
    esac
done

VERSION="${NEW_VERSION:-$CURRENT_VERSION}"

# ── Pre-flight: working tree clean ────────────────────────────────
header "Pre-flight Checks"

if ! command -v gh &>/dev/null; then
    fail "gh CLI not found. Install: https://cli.github.com"
fi
ok "gh CLI found: $(gh --version | head -1)"

if [[ -z "${GITHUB_TOKEN:-}" ]]; then
    fail "GITHUB_TOKEN not set. Add it to .env or export it."
fi
ok "GITHUB_TOKEN is set"

if ! git -C "${PROJECT_ROOT}" diff --quiet 2>/dev/null || ! git -C "${PROJECT_ROOT}" diff --cached --quiet 2>/dev/null; then
    fail "Working tree has uncommitted changes. Commit or stash first."
fi
ok "Working tree is clean"

# Check tag doesn't already exist
if git -C "${PROJECT_ROOT}" tag -l "v${VERSION}" | grep -q "v${VERSION}"; then
    fail "Tag v${VERSION} already exists. Delete it first or use a different version."
fi
ok "Tag v${VERSION} does not exist yet"

# ── Pre-flight: clippy + tests ───────────────────────────────────
header "Pre-flight: Clippy + Tests"

info "Running clippy..."
if ! cargo clippy --manifest-path "${PROJECT_ROOT}/Cargo.toml" -- -D warnings 2>&1; then
    fail "Clippy found issues. Fix before releasing."
fi
ok "Clippy passed"

info "Running tests..."
if ! cargo test --manifest-path "${PROJECT_ROOT}/Cargo.toml" 2>&1; then
    fail "Tests failed. Fix before releasing."
fi
ok "All tests passed"

# ── Update VERSION if new version ────────────────────────────────
header "Update VERSION"

if [[ -n "$NEW_VERSION" && "$NEW_VERSION" != "$CURRENT_VERSION" ]]; then
    info "Updating VERSION: ${CURRENT_VERSION} → ${NEW_VERSION}"
    if [[ "$DRY_RUN" == true ]]; then
        info "[DRY RUN] Would update VERSION file and Cargo.toml"
    else
        echo "${NEW_VERSION}" > "${PROJECT_ROOT}/VERSION"
        # Update Cargo.toml version
        sed -i "s/^version = \".*\"/version = \"${NEW_VERSION}\"/" "${PROJECT_ROOT}/Cargo.toml"
        ok "VERSION updated to ${NEW_VERSION}"
    fi
else
    ok "Releasing current version: ${VERSION}"
fi

# ── Extract release notes from CHANGELOG ──────────────────────────
header "Extract Release Notes"

# Grab everything between the version header and the next ## heading
RELEASE_NOTES=$(awk "/^## \\[${VERSION}\\]/{found=1; next} /^## \\[/{if(found) exit} found{print}" "${PROJECT_ROOT}/CHANGELOG.md")

if [[ -z "$RELEASE_NOTES" ]]; then
    warn "No release notes found in CHANGELOG.md for v${VERSION}"
    RELEASE_NOTES="Release v${VERSION}"
fi

# Trim leading/trailing whitespace
RELEASE_NOTES=$(echo "$RELEASE_NOTES" | sed '/^[[:space:]]*$/d')

info "Release notes ($(echo "$RELEASE_NOTES" | wc -l) lines):"
echo "$RELEASE_NOTES" | head -5
if [[ $(echo "$RELEASE_NOTES" | wc -l) -gt 5 ]]; then
    info "  ... ($(($(echo "$RELEASE_NOTES" | wc -l) - 5)) more lines)"
fi

# ── Execute release ───────────────────────────────────────────────
header "Release: v${VERSION}"

# Step 1: Commit VERSION/Cargo.toml changes (if any)
if [[ -n "$NEW_VERSION" && "$NEW_VERSION" != "$CURRENT_VERSION" ]]; then
    if [[ "$DRY_RUN" == true ]]; then
        info "[DRY RUN] Would commit VERSION + Cargo.toml changes"
    else
        git -C "${PROJECT_ROOT}" add VERSION Cargo.toml
        git -C "${PROJECT_ROOT}" commit -m "chore: bump version to ${NEW_VERSION}"
        ok "Committed version bump"
    fi
fi

# Step 2: Create tag
if [[ "$DRY_RUN" == true ]]; then
    info "[DRY RUN] Would create tag v${VERSION}"
else
    git -C "${PROJECT_ROOT}" tag -a "v${VERSION}" -m "Release v${VERSION}"
    ok "Created tag v${VERSION}"
fi

# Step 3: Push tag
if [[ "$DRY_RUN" == true ]]; then
    info "[DRY RUN] Would push tag to origin"
else
    git -C "${PROJECT_ROOT}" push origin "v${VERSION}"
    ok "Pushed tag v${VERSION} to origin"
fi

# Step 4: Create GitHub release
if [[ "$DRY_RUN" == true ]]; then
    info "[DRY RUN] Would create GitHub release v${VERSION}"
else
    echo "$RELEASE_NOTES" | gh release create "v${VERSION}" \
        --repo "${REPO_SLUG}" \
        --title "v${VERSION}" \
        --notes-file - \
        --latest
    ok "GitHub release created: https://github.com/${REPO_SLUG}/releases/tag/v${VERSION}"
fi

# ── Summary ───────────────────────────────────────────────────────
header "Release Complete"
echo ""
echo "  Version:   v${VERSION}"
echo "  Tag:       v${VERSION}"
echo "  Release:   https://github.com/${REPO_SLUG}/releases/tag/v${VERSION}"
echo ""
ok "Release v${VERSION} is live."
