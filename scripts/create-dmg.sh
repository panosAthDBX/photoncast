#!/bin/bash
#
# PhotonCast DMG Creation Script
# Task 3.4: Create DMG Creation Script
#
# This script creates a polished DMG with custom background, Applications folder alias,
# and proper window layout for drag-to-install experience.
#
# Prerequisites:
#   - Signed app bundle (from sign.sh)
#   - create-dmg tool installed (brew install create-dmg)
#   - Background image at resources/dmg-background.png (optional)
#
# Usage: ./scripts/create-dmg.sh
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
BUILD_DIR="${PROJECT_ROOT}/build"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}.app"
DMG_PATH="${BUILD_DIR}/${APP_NAME}.dmg"
RESOURCES_DIR="${PROJECT_ROOT}/resources"
BACKGROUND_IMAGE="${RESOURCES_DIR}/dmg-background.png"

# DMG settings
DMG_VOLUME_NAME="${APP_NAME} Installer"
DMG_WINDOW_SIZE="800x500"
DMG_ICON_SIZE=128
DMG_APP_POSITION="200,250"
DMG_APPS_POSITION="600,250"

# Check if app bundle exists
if [[ ! -d "$APP_BUNDLE" ]]; then
    echo -e "${RED}Error: App bundle not found at ${APP_BUNDLE}${NC}"
    echo "Please run ./scripts/release-build.sh and ./scripts/sign.sh first"
    exit 1
fi

# Check if create-dmg is installed
USE_CREATE_DMG=false
if command -v create-dmg &> /dev/null; then
    USE_CREATE_DMG=true
    echo -e "${GREEN}✓ create-dmg tool found${NC}"
else
    echo -e "${YELLOW}⚠ create-dmg not found, using hdiutil fallback${NC}"
    echo "For best results, install create-dmg: brew install create-dmg"
fi

# Remove old DMG if it exists
if [[ -f "$DMG_PATH" ]]; then
    echo -e "${BLUE}Removing existing DMG...${NC}"
    rm -f "$DMG_PATH"
fi

# Create a temporary directory for DMG contents
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Copy app bundle to temp directory
echo -e "${BLUE}Preparing DMG contents...${NC}"
cp -R "$APP_BUNDLE" "${TEMP_DIR}/"

# Create Applications folder alias
ln -s /Applications "${TEMP_DIR}/Applications"

if [[ "$USE_CREATE_DMG" == true ]]; then
    echo -e "${BLUE}Creating DMG with create-dmg...${NC}"

    # Build create-dmg arguments
    CREATE_DMG_ARGS=(
        "--volname" "${DMG_VOLUME_NAME}"
        "--window-size" "$DMG_WINDOW_SIZE"
        "--icon-size" "$DMG_ICON_SIZE"
        "--app-drop-link" "$DMG_APPS_POSITION"
        "--no-internet-enable"
    )

    # Add background image if it exists
    if [[ -f "$BACKGROUND_IMAGE" ]]; then
        echo -e "${BLUE}Using background image: ${BACKGROUND_IMAGE}${NC}"
        CREATE_DMG_ARGS+=("--background" "$BACKGROUND_IMAGE")
    else
        echo -e "${YELLOW}⚠ Background image not found, using default${NC}"
        echo "  Expected at: ${BACKGROUND_IMAGE}"
    fi

    # Add license if it exists
    if [[ -f "${PROJECT_ROOT}/LICENSE" ]]; then
        CREATE_DMG_ARGS+=("--eula" "${PROJECT_ROOT}/LICENSE")
    fi

    # Create the DMG
    create-dmg "${CREATE_DMG_ARGS[@]}" "$DMG_PATH" "$TEMP_DIR"

    echo -e "${GREEN}✓ DMG created with create-dmg${NC}"
else
    echo -e "${BLUE}Creating DMG with hdiutil...${NC}"

    # Fallback method using hdiutil
    TEMP_DMG="${TEMP_DIR}/temp.dmg"
    MOUNT_POINT="${TEMP_DIR}/mount"

    # Calculate DMG size (app size + 20MB padding)
    APP_SIZE=$(du -sm "${APP_BUNDLE}" | cut -f1)
    DMG_SIZE=$((APP_SIZE + 50))

    # Create temporary DMG
    hdiutil create -size "${DMG_SIZE}m" -fs HFS+ -volname "${DMG_VOLUME_NAME}" -o "$TEMP_DMG"

    # Mount the DMG
    mkdir -p "$MOUNT_POINT"
    hdiutil attach "$TEMP_DMG" -mountpoint "$MOUNT_POINT" -nobrowse

    # Copy contents
    cp -R "${TEMP_DIR}/${APP_NAME}.app" "$MOUNT_POINT/"
    ln -s /Applications "$MOUNT_POINT/Applications"

    # Copy background image if it exists (styling is best-effort, may fail in non-interactive environments)
    if [[ -f "$BACKGROUND_IMAGE" ]]; then
        mkdir -p "$MOUNT_POINT/.background"
        cp "$BACKGROUND_IMAGE" "$MOUNT_POINT/.background/background.png"
        echo -e "${BLUE}Background image copied${NC}"
    fi

    # Unmount
    hdiutil detach "$MOUNT_POINT" -force || hdiutil detach "$MOUNT_POINT" 2>/dev/null || true
    sleep 1

    # Convert to compressed read-only DMG
    hdiutil convert "$TEMP_DMG" -format UDZO -o "$DMG_PATH"

    echo -e "${GREEN}✓ DMG created with hdiutil${NC}"
fi

# Verify DMG
echo -e "${BLUE}Verifying DMG...${NC}"
if [[ -f "$DMG_PATH" ]]; then
    DMG_SIZE=$(du -h "$DMG_PATH" | cut -f1)
    echo -e "${GREEN}✓ DMG created: ${DMG_PATH} (${DMG_SIZE})${NC}"

    # Test mounting
    echo -e "${BLUE}Testing DMG mount...${NC}"
    MOUNT_TEST=$(hdiutil attach "$DMG_PATH" -readonly -nobrowse | grep "Volumes" | head -1)
    if [[ -n "$MOUNT_TEST" ]]; then
        MOUNT_PATH=$(echo "$MOUNT_TEST" | sed -E 's/.*(\/Volumes.*)/\1/')
        echo -e "${GREEN}✓ DMG mounts successfully at: ${MOUNT_PATH}${NC}"
        hdiutil detach "$MOUNT_PATH" -force || true
    fi
else
    echo -e "${RED}✗ DMG creation failed${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ DMG creation completed successfully!${NC}"
echo ""
echo -e "${BLUE}Output:${NC}"
echo "  DMG: ${DMG_PATH}"
echo "  Size: ${DMG_SIZE}"
echo ""
echo -e "${BLUE}Next step: Notarize with ./scripts/notarize.sh${NC}"
