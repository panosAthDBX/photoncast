#!/bin/bash
#
# PhotonCast Code Signing Script
# Task 3.2: Implement Code Signing
#
# This script signs the app bundle with a stable code-signing identity
# and verifies the signature with strict validation.
#
# Prerequisites:
#   - A signing identity such as:
#       - Developer ID Application
#       - Apple Development
#       - PhotonCast Local Dev (from create-dev-signing-identity.sh)
#   - App bundle created by release-build.sh
#
# Usage: ./scripts/sign.sh [certificate_name]
#   certificate_name: Optional. If omitted, the script tries:
#     1. $PHOTONCAST_SIGNING_IDENTITY
#     2. Developer ID Application
#     3. Apple Development
#     4. PhotonCast Local Dev
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

# shellcheck source=./lib/signing_env.sh
source "${PROJECT_ROOT}/scripts/lib/signing_env.sh"
load_photoncast_signing_env
unlock_photoncast_signing_keychain

# Configuration
APP_NAME="PhotonCast"
BUILD_DIR="${PROJECT_ROOT}/build"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}.app"
ENTITLEMENTS_FILE="${BUILD_DIR}/entitlements.plist"
SIGNING_KEYCHAIN="${PHOTONCAST_SIGNING_KEYCHAIN:-}"

# Certificate name (can be overridden by argument or environment variable)
CERT_NAME="${1:-${PHOTONCAST_SIGNING_IDENTITY:-${APPLE_DEVELOPER_ID:-}}}"
if [[ -z "$CERT_NAME" ]]; then
    IDENTITY_ARGS=(-v -p codesigning)
    if [[ -n "$SIGNING_KEYCHAIN" ]]; then
        IDENTITY_ARGS+=("$SIGNING_KEYCHAIN")
    fi

    IDENTITIES=$(security find-identity "${IDENTITY_ARGS[@]}" 2>/dev/null || true)

    for pattern in "Developer ID Application" "Apple Development" "PhotonCast Local Dev"; do
        CERT_NAME=$(printf '%s\n' "$IDENTITIES" | \
            grep "$pattern" | \
            head -1 | \
            sed -E 's/.*"([^"]+)".*/\1/' || true)
        if [[ -n "$CERT_NAME" ]]; then
            break
        fi
    done

    if [[ -z "$CERT_NAME" ]]; then
        echo -e "${RED}Error: No compatible signing identity found${NC}"
        echo ""
        echo "Please provide the certificate name as an argument:"
        echo "  ./scripts/sign.sh 'Developer ID Application: Your Name (TEAM_ID)'"
        echo ""
        echo "Or set the PHOTONCAST_SIGNING_IDENTITY environment variable:"
        echo "  export PHOTONCAST_SIGNING_IDENTITY='Developer ID Application: Your Name (TEAM_ID)'"
        echo ""
        echo "Available certificates:"
        printf '%s\n' "$IDENTITIES"
        exit 1
    fi
fi

echo -e "${BLUE}Using certificate: ${CERT_NAME}${NC}"
if [[ -n "$SIGNING_KEYCHAIN" ]]; then
    echo -e "${BLUE}Using keychain: ${SIGNING_KEYCHAIN}${NC}"
fi

SIGNING_REF="$CERT_NAME"
IDENTITY_QUERY_ARGS=(-v -p codesigning)
if [[ -n "$SIGNING_KEYCHAIN" ]]; then
    IDENTITY_QUERY_ARGS+=("$SIGNING_KEYCHAIN")
fi

IDENTITY_LINE=$(security find-identity "${IDENTITY_QUERY_ARGS[@]}" 2>/dev/null | \
    grep "\"${CERT_NAME}\"" | \
    head -1 || true)
if [[ -n "$IDENTITY_LINE" ]]; then
    SIGNING_REF=$(printf '%s\n' "$IDENTITY_LINE" | awk '{print $2}')
fi

echo -e "${BLUE}Using signing reference: ${SIGNING_REF}${NC}"

# Verify app bundle exists
if [[ ! -d "$APP_BUNDLE" ]]; then
    echo -e "${RED}Error: App bundle not found at ${APP_BUNDLE}${NC}"
    echo "Please run ./scripts/release-build.sh first"
    exit 1
fi

# Verify entitlements file exists
if [[ ! -f "$ENTITLEMENTS_FILE" ]]; then
    echo -e "${YELLOW}Warning: Entitlements file not found at ${ENTITLEMENTS_FILE}${NC}"
    echo -e "${YELLOW}Signing without entitlements...${NC}"
    ENTITLEMENTS_FILE=""
fi

# Function to sign a binary
sign_binary() {
    local target="$1"
    local options="--sign \"${SIGNING_REF}\" --force --timestamp"

    if [[ -n "$SIGNING_KEYCHAIN" ]]; then
        options="${options} --keychain \"${SIGNING_KEYCHAIN}\""
    fi

    # Add entitlements for the main app bundle
    if [[ -n "$ENTITLEMENTS_FILE" && "$target" == "$APP_BUNDLE" ]]; then
        options="${options} --entitlements \"${ENTITLEMENTS_FILE}\""
    fi

    # Enable hardened runtime for the main app bundle
    if [[ "$target" == "$APP_BUNDLE" ]]; then
        options="${options} --options runtime"
    fi

    echo -e "${BLUE}Signing: ${target}${NC}"
    eval codesign ${options} "${target}"
}

# Sign all embedded frameworks and libraries
echo -e "${BLUE}Signing embedded frameworks and libraries...${NC}"
FRAMEWORKS_DIR="${APP_BUNDLE}/Contents/Frameworks"
if [[ -d "$FRAMEWORKS_DIR" ]]; then
    find "$FRAMEWORKS_DIR" -type f \( -name "*.framework" -o -name "*.dylib" -o -name "*.so" \) | while read -r file; do
        sign_binary "$file"
    done
fi

# Sign all embedded extensions
EXTENSIONS_DIR="${APP_BUNDLE}/Contents/Extensions"
if [[ -d "$EXTENSIONS_DIR" ]]; then
    find "$EXTENSIONS_DIR" -type f -name "*.dylib" | while read -r file; do
        sign_binary "$file"
    done
fi

# Sign every embedded executable in Contents/MacOS
MACOS_DIR="${APP_BUNDLE}/Contents/MacOS"
if [[ -d "$MACOS_DIR" ]]; then
    find "$MACOS_DIR" -type f | while read -r file; do
        sign_binary "$file"
    done
fi

# Sign the app bundle itself (with entitlements and hardened runtime)
echo -e "${BLUE}Signing app bundle with hardened runtime...${NC}"
sign_binary "$APP_BUNDLE"

# Verify signature
echo -e "${BLUE}Verifying signature...${NC}"

# Basic verification
echo -e "${BLUE}Running codesign --verify...${NC}"
if codesign --verify --verbose "${APP_BUNDLE}"; then
    echo -e "${GREEN}✓ Basic signature verification passed${NC}"
else
    echo -e "${RED}✗ Basic signature verification failed${NC}"
    exit 1
fi

# Deep verification
echo -e "${BLUE}Running deep verification...${NC}"
if codesign --verify --deep --strict --verbose=2 "${APP_BUNDLE}" 2>&1 | head -50; then
    echo -e "${GREEN}✓ Deep signature verification passed${NC}"
else
    echo -e "${RED}✗ Deep signature verification failed${NC}"
    exit 1
fi

# Display signature info
echo -e "${BLUE}Signature details:${NC}"
codesign --display --verbose=4 "${APP_BUNDLE}" 2>&1 | head -30

# Check for hardened runtime
echo -e "${BLUE}Checking for hardened runtime...${NC}"
if codesign --display --verbose=4 "${APP_BUNDLE}" 2>&1 | grep -q "Runtime Version"; then
    echo -e "${GREEN}✓ Hardened runtime is enabled${NC}"
else
    echo -e "${YELLOW}⚠ Hardened runtime may not be enabled${NC}"
fi

echo ""
echo -e "${GREEN}✓ Code signing completed successfully!${NC}"
echo ""
echo -e "${BLUE}Next step: Create DMG with ./scripts/create-dmg.sh${NC}"
