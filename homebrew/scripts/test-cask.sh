#!/bin/bash
#
# PhotonCast Homebrew Cask Test Script
# Task 5.7: Test Homebrew Cask Installation
#
# This script verifies the Homebrew cask formula:
# - Formula syntax validation (brew audit)
# - Formula style check (brew style)
# - Local installation test
# - Uninstall verification
#
# Usage: ./homebrew/scripts/test-cask.sh
#
# Environment Variables:
#   SKIP_INSTALL_TEST - Set to 1 to skip actual install/uninstall tests
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOMEBREW_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROJECT_ROOT="$(cd "${HOMEBREW_DIR}/.." && pwd)"

# Configuration
FORMULA_FILE="${HOMEBREW_DIR}/photoncast.rb"
APP_NAME="PhotonCast"

# Counters
PASSED=0
FAILED=0
WARNINGS=0
SKIPPED=0

# Helper functions
pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((PASSED++))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    ((FAILED++))
}

warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((WARNINGS++))
}

skip() {
    echo -e "${BLUE}○${NC} $1 (skipped)"
    ((SKIPPED++))
}

echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}PhotonCast Homebrew Cask Test${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""

# =============================================================================
# Check Prerequisites
# =============================================================================

echo -e "${BLUE}1. Checking prerequisites...${NC}"
echo ""

# Check if Homebrew is installed
if command -v brew &> /dev/null; then
    brew_version=$(brew --version | head -1)
    pass "Homebrew is installed: $brew_version"
else
    fail "Homebrew is not installed"
    echo "   Install from: https://brew.sh"
    exit 1
fi

# Check if formula file exists
if [[ -f "$FORMULA_FILE" ]]; then
    pass "Formula file exists: $(basename "$FORMULA_FILE")"
else
    fail "Formula file not found: $FORMULA_FILE"
    exit 1
fi
echo ""

# =============================================================================
# Formula Syntax Check
# =============================================================================

echo -e "${BLUE}2. Validating formula syntax...${NC}"
echo ""

# Read the formula to check basic structure
echo "   Checking formula structure..."

# Check for required stanzas
if grep -q "cask \"photoncast\"" "$FORMULA_FILE"; then
    pass "Cask name defined correctly"
else
    fail "Cask name not found or incorrect"
fi

if grep -q "version" "$FORMULA_FILE"; then
    version=$(grep -m1 "version" "$FORMULA_FILE" | sed 's/.*"\(.*\)".*/\1/')
    pass "Version defined: $version"
else
    fail "Version not defined"
fi

if grep -q "sha256" "$FORMULA_FILE"; then
    pass "SHA256 checksum defined"
else
    fail "SHA256 checksum not defined"
fi

if grep -q "url" "$FORMULA_FILE"; then
    pass "Download URL defined"
else
    fail "Download URL not defined"
fi

if grep -q "name" "$FORMULA_FILE"; then
    pass "App name defined"
else
    warn "App name not explicitly defined"
fi

if grep -q "homepage" "$FORMULA_FILE"; then
    pass "Homepage defined"
else
    warn "Homepage not defined"
fi

if grep -q "app \"" "$FORMULA_FILE"; then
    pass "App artifact defined"
else
    fail "App artifact not defined"
fi
echo ""

# =============================================================================
# Brew Audit
# =============================================================================

echo -e "${BLUE}3. Running brew audit...${NC}"
echo ""

echo "   Running: brew audit --cask $FORMULA_FILE"
audit_output=$(brew audit --cask "$FORMULA_FILE" 2>&1) || true

if [[ -z "$audit_output" ]] || echo "$audit_output" | grep -q "passed"; then
    pass "brew audit passed"
else
    # Check if there are only warnings vs errors
    if echo "$audit_output" | grep -q "Error:"; then
        fail "brew audit found errors"
        echo "   $audit_output" | head -20
    else
        warn "brew audit has warnings"
        echo "   $audit_output" | head -10
    fi
fi
echo ""

# =============================================================================
# Brew Style Check
# =============================================================================

echo -e "${BLUE}4. Running brew style check...${NC}"
echo ""

echo "   Running: brew style --fix $FORMULA_FILE"
style_output=$(brew style "$FORMULA_FILE" 2>&1) || true

if [[ -z "$style_output" ]] || echo "$style_output" | grep -q "no offenses"; then
    pass "brew style passed (no offenses)"
else
    if echo "$style_output" | grep -q "offenses detected"; then
        warn "brew style found style issues"
        echo "   Run 'brew style --fix $FORMULA_FILE' to auto-fix"
        echo "   $style_output" | head -10
    else
        pass "brew style check completed"
    fi
fi
echo ""

# =============================================================================
# Check Zap Stanza
# =============================================================================

echo -e "${BLUE}5. Checking zap stanza...${NC}"
echo ""

if grep -q "zap" "$FORMULA_FILE"; then
    pass "Zap stanza defined for cleanup"
    
    # Check what gets cleaned up
    if grep -A5 "zap" "$FORMULA_FILE" | grep -q "trash"; then
        pass "Zap includes trash paths"
    else
        warn "Zap may not include all cleanup paths"
    fi
else
    warn "Zap stanza not defined (optional but recommended)"
fi
echo ""

# =============================================================================
# Check Caveats
# =============================================================================

echo -e "${BLUE}6. Checking caveats...${NC}"
echo ""

if grep -q "caveats" "$FORMULA_FILE"; then
    pass "Caveats defined"
    echo "   Caveats content:"
    grep -A10 "caveats" "$FORMULA_FILE" | head -10 | while read line; do
        echo "     $line"
    done
else
    warn "No caveats defined (optional)"
fi
echo ""

# =============================================================================
# Installation Test (Optional)
# =============================================================================

echo -e "${BLUE}7. Installation test...${NC}"
echo ""

if [[ "${SKIP_INSTALL_TEST:-0}" == "1" ]]; then
    skip "Installation test (SKIP_INSTALL_TEST=1)"
else
    # Check if PhotonCast is already installed
    if brew list --cask 2>/dev/null | grep -q "photoncast"; then
        warn "PhotonCast is already installed via Homebrew"
        echo "   Skipping installation test to avoid conflicts"
        skip "Installation test (already installed)"
    elif [[ -d "/Applications/${APP_NAME}.app" ]]; then
        warn "PhotonCast.app exists in /Applications"
        echo "   Skipping installation test to avoid conflicts"
        skip "Installation test (app exists)"
    else
        echo "   Attempting local cask installation..."
        echo "   Running: brew install --cask $FORMULA_FILE"
        
        if brew install --cask "$FORMULA_FILE" 2>&1; then
            pass "Installation successful"
            
            # Verify app was installed
            if [[ -d "/Applications/${APP_NAME}.app" ]]; then
                pass "App installed to /Applications"
            else
                fail "App not found in /Applications after install"
            fi
            
            # Test uninstall
            echo ""
            echo "   Testing uninstall..."
            if brew uninstall --cask photoncast 2>&1; then
                pass "Uninstall successful"
                
                # Verify app was removed
                if [[ ! -d "/Applications/${APP_NAME}.app" ]]; then
                    pass "App removed from /Applications"
                else
                    fail "App still exists after uninstall"
                fi
            else
                fail "Uninstall failed"
            fi
        else
            fail "Installation failed"
            echo "   This may be expected if:"
            echo "   - DMG is not available at the URL"
            echo "   - SHA256 doesn't match"
            echo "   - Network issues"
        fi
    fi
fi
echo ""

# =============================================================================
# URL Availability Check
# =============================================================================

echo -e "${BLUE}8. Checking download URL...${NC}"
echo ""

# Extract URL from formula
download_url=$(grep -m1 "url" "$FORMULA_FILE" | sed 's/.*"\(http[^"]*\)".*/\1/' | sed 's/#{version}/'"$version"'/g')

if [[ -n "$download_url" ]]; then
    echo "   URL: $download_url"
    
    # Try HEAD request to check if URL is accessible
    if curl --output /dev/null --silent --head --fail "$download_url" 2>/dev/null; then
        pass "Download URL is accessible"
    else
        warn "Download URL may not be accessible yet"
        echo "   This is expected if the release hasn't been published"
    fi
else
    fail "Could not extract download URL from formula"
fi
echo ""

# =============================================================================
# SHA256 Verification
# =============================================================================

echo -e "${BLUE}9. Checking SHA256...${NC}"
echo ""

sha256=$(grep -m1 "sha256" "$FORMULA_FILE" | sed 's/.*"\(.*\)".*/\1/')
if [[ -n "$sha256" && "$sha256" != "no_check" ]]; then
    echo "   SHA256: $sha256"
    
    if [[ ${#sha256} -eq 64 ]]; then
        pass "SHA256 format is valid (64 hex characters)"
    else
        fail "SHA256 format is invalid (got ${#sha256} chars, expected 64)"
    fi
else
    warn "SHA256 is 'no_check' or not set"
    echo "   Update with actual checksum before submission"
fi
echo ""

# =============================================================================
# Compare with Local DMG
# =============================================================================

echo -e "${BLUE}10. Comparing with local DMG...${NC}"
echo ""

LOCAL_DMG="${PROJECT_ROOT}/build/${APP_NAME}.dmg"
if [[ -f "$LOCAL_DMG" ]]; then
    local_sha256=$(shasum -a 256 "$LOCAL_DMG" | awk '{print $1}')
    echo "   Local DMG SHA256: $local_sha256"
    
    if [[ "$sha256" == "$local_sha256" ]]; then
        pass "Formula SHA256 matches local DMG"
    elif [[ "$sha256" == "no_check" ]]; then
        warn "Formula SHA256 not set, local DMG checksum: $local_sha256"
    else
        warn "Formula SHA256 doesn't match local DMG"
        echo "   Formula: $sha256"
        echo "   Local:   $local_sha256"
        echo "   Update formula with: $local_sha256"
    fi
else
    skip "Local DMG comparison (no local DMG found)"
fi
echo ""

# =============================================================================
# Summary
# =============================================================================

echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}Summary${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""
echo -e "  ${GREEN}Passed:${NC}   $PASSED"
echo -e "  ${RED}Failed:${NC}   $FAILED"
echo -e "  ${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "  ${BLUE}Skipped:${NC}  $SKIPPED"
echo ""

# Submission checklist
echo -e "${BLUE}Homebrew Submission Checklist:${NC}"
echo ""
echo "  Before submitting to homebrew-cask:"
echo ""
echo "  [ ] App has 50+ GitHub stars (or notable/popular)"
echo "  [ ] App is notarized and signed"
echo "  [ ] Download URL is stable and accessible"
echo "  [ ] SHA256 matches the downloadable DMG"
echo "  [ ] Version matches the release"
echo "  [ ] Formula passes 'brew audit --cask'"
echo "  [ ] Formula passes 'brew style'"
echo "  [ ] Tested local installation works"
echo ""

if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}✗ Cask verification failed!${NC}"
    echo "  Please fix the issues before submission."
    exit 1
elif [[ $WARNINGS -gt 0 ]]; then
    echo -e "${YELLOW}⚠ Cask verification passed with warnings${NC}"
    echo "  Review the warnings above."
    exit 0
else
    echo -e "${GREEN}✓ All cask checks passed!${NC}"
    exit 0
fi
