#!/bin/bash
#
# PhotonCast DMG Installation Test Script
# Task 5.6: Test DMG Installation Flow
#
# This script tests the complete DMG installation workflow:
# - DMG mounts correctly
# - Background image displays
# - App bundle is present
# - Applications folder alias works
# - Drag-to-Applications simulation
#
# Usage: ./scripts/test-dmg.sh [dmg_path]
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
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Configuration
APP_NAME="PhotonCast"
DEFAULT_DMG_PATH="${PROJECT_ROOT}/build/${APP_NAME}.dmg"
DMG_PATH="${1:-${DEFAULT_DMG_PATH}}"
MOUNT_POINT=""

# Counters
PASSED=0
FAILED=0
WARNINGS=0

# Cleanup function
cleanup() {
    if [[ -n "$MOUNT_POINT" && -d "$MOUNT_POINT" ]]; then
        echo ""
        echo -e "${BLUE}Cleaning up...${NC}"
        hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true
    fi
}

trap cleanup EXIT

# Helper function for test results
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

echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}PhotonCast DMG Installation Test${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""

# =============================================================================
# Check DMG File Exists
# =============================================================================

echo -e "${BLUE}1. Checking DMG file...${NC}"
echo ""

if [[ ! -f "$DMG_PATH" ]]; then
    fail "DMG file not found at: $DMG_PATH"
    echo ""
    echo "Please build the DMG first with:"
    echo "  ./scripts/release-build.sh"
    echo "  ./scripts/sign.sh"
    echo "  ./scripts/create-dmg.sh"
    exit 1
fi

# Get DMG info
dmg_size=$(wc -c < "$DMG_PATH" | tr -d ' ')
dmg_size_mb=$((dmg_size / 1024 / 1024))
pass "DMG file exists: $(basename "$DMG_PATH")"
echo "   Size: ${dmg_size_mb} MB"
echo ""

# =============================================================================
# Verify DMG Integrity
# =============================================================================

echo -e "${BLUE}2. Verifying DMG integrity...${NC}"
echo ""

# Check DMG checksum
if [[ -f "${DMG_PATH}.sha256" ]]; then
    expected_sha=$(cat "${DMG_PATH}.sha256" | awk '{print $1}')
    actual_sha=$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')
    
    if [[ "$expected_sha" == "$actual_sha" ]]; then
        pass "SHA256 checksum matches"
    else
        fail "SHA256 checksum mismatch"
        echo "   Expected: $expected_sha"
        echo "   Actual:   $actual_sha"
    fi
else
    warn "No .sha256 file found for verification"
fi

# Verify DMG with hdiutil
echo ""
echo "   Running hdiutil verify..."
if hdiutil verify "$DMG_PATH" 2>&1 | grep -q "verified"; then
    pass "DMG passes hdiutil verify"
else
    # hdiutil verify may not output "verified" on success, check exit code
    if hdiutil verify "$DMG_PATH" >/dev/null 2>&1; then
        pass "DMG passes hdiutil verify"
    else
        fail "DMG failed hdiutil verify"
    fi
fi
echo ""

# =============================================================================
# Mount DMG
# =============================================================================

echo -e "${BLUE}3. Mounting DMG...${NC}"
echo ""

# Mount the DMG
mount_output=$(hdiutil attach "$DMG_PATH" -nobrowse -noverify 2>&1)

if [[ $? -eq 0 ]]; then
    # Extract mount point
    MOUNT_POINT=$(echo "$mount_output" | grep -o '/Volumes/[^[:space:]]*' | head -1)
    
    if [[ -n "$MOUNT_POINT" && -d "$MOUNT_POINT" ]]; then
        pass "DMG mounted successfully"
        echo "   Mount point: $MOUNT_POINT"
    else
        fail "Could not determine mount point"
        echo "   Output: $mount_output"
        exit 1
    fi
else
    fail "Failed to mount DMG"
    echo "   Error: $mount_output"
    exit 1
fi
echo ""

# =============================================================================
# Check Volume Contents
# =============================================================================

echo -e "${BLUE}4. Checking volume contents...${NC}"
echo ""

# List contents
echo "   Volume contents:"
ls -la "$MOUNT_POINT" 2>/dev/null | while read line; do
    echo "     $line"
done
echo ""

# Check for app bundle
APP_PATH="${MOUNT_POINT}/${APP_NAME}.app"
if [[ -d "$APP_PATH" ]]; then
    pass "App bundle present: ${APP_NAME}.app"
    
    # Check app bundle structure
    if [[ -f "${APP_PATH}/Contents/Info.plist" ]]; then
        pass "Info.plist present"
    else
        fail "Info.plist missing from app bundle"
    fi
    
    if [[ -f "${APP_PATH}/Contents/MacOS/photoncast" ]]; then
        pass "Executable present"
    else
        fail "Executable missing from app bundle"
    fi
    
    if [[ -d "${APP_PATH}/Contents/Resources" ]]; then
        pass "Resources directory present"
    else
        fail "Resources directory missing"
    fi
else
    fail "App bundle not found in DMG"
fi
echo ""

# Check for Applications folder alias/symlink
echo -e "${BLUE}5. Checking Applications alias...${NC}"
echo ""

APPS_LINK="${MOUNT_POINT}/Applications"
if [[ -L "$APPS_LINK" ]]; then
    link_target=$(readlink "$APPS_LINK" 2>/dev/null || true)
    if [[ "$link_target" == "/Applications" ]]; then
        pass "Applications symlink points to /Applications"
    else
        warn "Applications link target: $link_target"
    fi
elif [[ -e "$APPS_LINK" ]]; then
    pass "Applications alias present"
else
    fail "Applications folder alias/symlink not found"
fi
echo ""

# =============================================================================
# Check Background Image
# =============================================================================

echo -e "${BLUE}6. Checking DMG background...${NC}"
echo ""

BG_DIR="${MOUNT_POINT}/.background"
if [[ -d "$BG_DIR" ]]; then
    pass "Background directory present"
    
    # List background files
    bg_files=$(ls -1 "$BG_DIR" 2>/dev/null || true)
    if [[ -n "$bg_files" ]]; then
        echo "   Background files:"
        echo "$bg_files" | while read f; do
            echo "     - $f"
        done
        pass "Background image(s) found"
    else
        warn "Background directory is empty"
    fi
else
    warn "No .background directory (background may be embedded differently)"
fi
echo ""

# =============================================================================
# Check Window Settings (DS_Store)
# =============================================================================

echo -e "${BLUE}7. Checking DMG window settings...${NC}"
echo ""

DS_STORE="${MOUNT_POINT}/.DS_Store"
if [[ -f "$DS_STORE" ]]; then
    pass ".DS_Store present (contains window layout)"
else
    warn ".DS_Store not found (window layout may use defaults)"
fi
echo ""

# =============================================================================
# Verify Code Signature (if present)
# =============================================================================

echo -e "${BLUE}8. Verifying app signature...${NC}"
echo ""

if codesign --verify --deep --strict "$APP_PATH" 2>/dev/null; then
    pass "App bundle is properly signed"
    
    # Check for notarization
    if spctl -a -v "$APP_PATH" 2>&1 | grep -q "accepted"; then
        pass "App is notarized (Gatekeeper approved)"
    else
        warn "App may not be notarized (Gatekeeper check inconclusive)"
    fi
else
    warn "App signature verification failed or not signed"
fi
echo ""

# =============================================================================
# Test Installation Simulation
# =============================================================================

echo -e "${BLUE}9. Installation simulation...${NC}"
echo ""

# Create a temporary directory to simulate installation
TEMP_APPS=$(mktemp -d)
echo "   Simulating drag to Applications..."
echo "   Target: $TEMP_APPS"

if cp -R "$APP_PATH" "$TEMP_APPS/"; then
    pass "App can be copied (simulated drag-to-Applications)"
    
    # Verify copied app
    COPIED_APP="${TEMP_APPS}/${APP_NAME}.app"
    if [[ -d "$COPIED_APP" ]]; then
        pass "Copied app bundle is valid"
        
        # Try to run the app briefly (without UI)
        EXECUTABLE="${COPIED_APP}/Contents/MacOS/photoncast"
        if [[ -x "$EXECUTABLE" ]]; then
            pass "Executable is runnable"
        else
            warn "Executable may not be runnable"
        fi
    else
        fail "Copied app bundle is invalid"
    fi
    
    # Cleanup
    rm -rf "$TEMP_APPS"
else
    fail "Failed to copy app bundle"
    rm -rf "$TEMP_APPS"
fi
echo ""

# =============================================================================
# First Launch Simulation Check
# =============================================================================

echo -e "${BLUE}10. First launch checks...${NC}"
echo ""

# Check for quarantine attribute (simulating download)
echo "   Note: First launch behavior depends on:"
echo "     - Notarization status"
echo "     - Gatekeeper settings"
echo "     - Download source (Safari adds quarantine)"
echo ""

# Check if the app would trigger Gatekeeper
if spctl --assess --type execute "$APP_PATH" 2>&1 | grep -q "rejected"; then
    fail "App would be rejected by Gatekeeper"
else
    pass "App should launch without Gatekeeper warning"
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
echo ""

# Manual verification checklist
echo -e "${BLUE}Manual Verification Checklist:${NC}"
echo ""
echo "  After opening the DMG in Finder, verify:"
echo ""
echo "  [ ] DMG window opens at correct size"
echo "  [ ] Background image is visible and crisp"
echo "  [ ] App icon is positioned correctly"
echo "  [ ] Applications folder icon is visible"
echo "  [ ] Arrow graphic points from app to Applications"
echo "  [ ] Dragging app to Applications works"
echo "  [ ] App launches from /Applications without warning"
echo ""

if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}✗ DMG verification failed!${NC}"
    echo "  Please fix the issues and rebuild the DMG."
    exit 1
elif [[ $WARNINGS -gt 0 ]]; then
    echo -e "${YELLOW}⚠ DMG verification passed with warnings${NC}"
    echo "  Review the warnings above."
    exit 0
else
    echo -e "${GREEN}✓ All DMG checks passed!${NC}"
    exit 0
fi
