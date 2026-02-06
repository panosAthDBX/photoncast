#!/bin/bash
#
# PhotonCast Release Build Script
# Task 3.1: Create Release Build Script
#
# This script builds an optimized release binary and creates a macOS app bundle
# with all required resources (Info.plist, entitlements, icons).
#
# Usage: ./scripts/release-build.sh [version]
#   version: Optional version string (defaults to version from Cargo.toml)
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

# Default values
BUNDLE_ID="com.photoncast.app"
APP_NAME="PhotonCast"
EXECUTABLE_NAME="photoncast"

# Parse arguments
VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
    # Extract version from Cargo.toml
    VERSION=$(grep "^version" "${PROJECT_ROOT}/Cargo.toml" | head -1 | sed 's/.*= "\([^"]*\)".*/\1/')
    echo -e "${BLUE}Using version from Cargo.toml: ${VERSION}${NC}"
fi

# Directories
BUILD_DIR="${PROJECT_ROOT}/build"
APP_BUNDLE_DIR="${BUILD_DIR}/${APP_NAME}.app"
CONTENTS_DIR="${APP_BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
FRAMEWORKS_DIR="${CONTENTS_DIR}/Frameworks"

# Clean and create build directory
echo -e "${BLUE}Setting up build directory...${NC}"
rm -rf "${BUILD_DIR}"
mkdir -p "${MACOS_DIR}" "${RESOURCES_DIR}" "${FRAMEWORKS_DIR}"

# Build optimized release binary
echo -e "${BLUE}Building optimized release binary...${NC}"
cd "${PROJECT_ROOT}"
cargo build --release

# Detect target directory (may be customized via CARGO_TARGET_DIR or .cargo/config)
TARGET_DIR=$(cargo metadata --format-version 1 2>/dev/null | jq -r '.target_directory' 2>/dev/null || echo "${PROJECT_ROOT}/target")
BINARY_PATH="${TARGET_DIR}/release/${EXECUTABLE_NAME}"

if [[ ! -f "${BINARY_PATH}" ]]; then
    echo -e "${RED}Error: Release binary not found at ${BINARY_PATH}${NC}"
    exit 1
fi

# Copy executable to bundle
echo -e "${BLUE}Copying executable to bundle...${NC}"
cp "${BINARY_PATH}" "${MACOS_DIR}/${EXECUTABLE_NAME}"
chmod +x "${MACOS_DIR}/${EXECUTABLE_NAME}"

# Copy extension runner if it exists
EXTENSION_RUNNER_PATH="${TARGET_DIR}/release/photoncast-extension-runner"
if [[ -f "${EXTENSION_RUNNER_PATH}" ]]; then
    echo -e "${BLUE}Copying extension runner...${NC}"
    cp "${EXTENSION_RUNNER_PATH}" "${MACOS_DIR}/photoncast-extension-runner"
    chmod +x "${MACOS_DIR}/photoncast-extension-runner"
    echo -e "${GREEN}✓ Extension runner copied${NC}"
else
    echo -e "${YELLOW}Warning: Extension runner not found at ${EXTENSION_RUNNER_PATH}${NC}"
fi

# Copy and update Info.plist
echo -e "${BLUE}Setting up Info.plist...${NC}"
if [[ -f "${PROJECT_ROOT}/resources/Info.plist" ]]; then
    cp "${PROJECT_ROOT}/resources/Info.plist" "${CONTENTS_DIR}/Info.plist"

    # Update version in Info.plist
    /usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${VERSION}" "${CONTENTS_DIR}/Info.plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${VERSION}" "${CONTENTS_DIR}/Info.plist"
else
    echo -e "${YELLOW}Warning: Info.plist not found in resources/, creating minimal version...${NC}"
    cat > "${CONTENTS_DIR}/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>${EXECUTABLE_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSUIElement</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.productivity</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
</dict>
</plist>
EOF
fi

# Copy app icon if it exists
echo -e "${BLUE}Setting up app icon...${NC}"
if [[ -f "${PROJECT_ROOT}/resources/AppIcon.icns" ]]; then
    cp "${PROJECT_ROOT}/resources/AppIcon.icns" "${RESOURCES_DIR}/AppIcon.icns"
    echo -e "${GREEN}✓ AppIcon.icns copied${NC}"
elif [[ -f "${PROJECT_ROOT}/resources/icon.png" ]]; then
    # Try to convert PNG to icns if iconutil is available
    if command -v iconutil &> /dev/null; then
        mkdir -p "${BUILD_DIR}/iconset"
        for size in 16 32 128 256 512; do
            sips -z ${size} ${size} "${PROJECT_ROOT}/resources/icon.png" \
                --out "${BUILD_DIR}/iconset/icon_${size}x${size}.png" 2>/dev/null || true
            sips -z $((size*2)) $((size*2)) "${PROJECT_ROOT}/resources/icon.png" \
                --out "${BUILD_DIR}/iconset/icon_${size}x${size}@2x.png" 2>/dev/null || true
        done
        iconutil -c icns "${BUILD_DIR}/iconset" -o "${RESOURCES_DIR}/AppIcon.icns" 2>/dev/null || true
        rm -rf "${BUILD_DIR}/iconset"
    fi
fi

# Copy entitlements for reference
echo -e "${BLUE}Copying entitlements...${NC}"
if [[ -f "${PROJECT_ROOT}/resources/entitlements.plist" ]]; then
    cp "${PROJECT_ROOT}/resources/entitlements.plist" "${BUILD_DIR}/entitlements.plist"
fi

# Verify bundle structure
echo -e "${BLUE}Verifying bundle structure...${NC}"
if [[ ! -x "${MACOS_DIR}/${EXECUTABLE_NAME}" ]]; then
    echo -e "${RED}Error: Executable is not executable or missing${NC}"
    exit 1
fi

if [[ ! -f "${CONTENTS_DIR}/Info.plist" ]]; then
    echo -e "${RED}Error: Info.plist is missing${NC}"
    exit 1
fi

# Ad-hoc sign the bundle to bind Info.plist and bundle identifier.
# This is required on macOS 26+ where TCC validates code identity for
# accessibility permissions. Without this, each rebuild gets a different
# auto-generated identifier, invalidating previously granted permissions.
echo -e "${BLUE}Ad-hoc signing app bundle...${NC}"
codesign -s - --force --deep "${APP_BUNDLE_DIR}"
echo -e "${GREEN}✓ App bundle signed (ad-hoc)${NC}"

# Print bundle info
echo -e "${GREEN}✓ App bundle created successfully!${NC}"
echo ""
echo -e "${BLUE}Bundle Information:${NC}"
echo "  Name: ${APP_NAME}.app"
echo "  Version: ${VERSION}"
echo "  Bundle ID: ${BUNDLE_ID}"
echo "  Path: ${APP_BUNDLE_DIR}"
echo ""
echo -e "${BLUE}Bundle Structure:${NC}"
find "${APP_BUNDLE_DIR}" -type f -o -type d | head -20

echo ""
echo -e "${GREEN}Build complete. Next steps:${NC}"
echo "  1. Sign the bundle: ./scripts/sign.sh"
echo "  2. Create DMG: ./scripts/create-dmg.sh"
echo "  3. Notarize: ./scripts/notarize.sh"
