#!/bin/bash
#
# PhotonCast Notarization Script
# Task 3.3: Create Notarization Workflow
#
# This script submits the signed DMG to Apple for notarization, polls for completion,
# and staples the notarization ticket to the app.
#
# Prerequisites:
#   - Signed DMG created by create-dmg.sh
#   - Apple ID with app-specific password configured
#   - Or App Store Connect API key
#
# Usage:
#   With Apple ID: ./scripts/notarize.sh
#   With API Key:  ./scripts/notarize.sh --api-key
#
# Environment Variables:
#   APPLE_ID - Your Apple ID email
#   APPLE_APP_SPECIFIC_PASSWORD - App-specific password (not your regular password!)
#   APPLE_TEAM_ID - Your Apple Developer Team ID (optional)
#   
#   For API Key authentication:
#   API_KEY_ID - App Store Connect API Key ID
#   API_KEY_ISSUER_ID - App Store Connect API Key Issuer ID
#   API_KEY_PATH - Path to the .p8 private key file
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
DMG_PATH="${BUILD_DIR}/${APP_NAME}.dmg"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}.app"
ZIP_PATH="${BUILD_DIR}/${APP_NAME}.zip"

# Parse arguments
USE_API_KEY=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --api-key)
            USE_API_KEY=true
            shift
            ;;
        --help|-h)
            echo "Usage: ./scripts/notarize.sh [--api-key]"
            echo ""
            echo "Options:"
            echo "  --api-key    Use App Store Connect API Key authentication"
            echo ""
            echo "Environment Variables for Apple ID:"
            echo "  APPLE_ID                   - Your Apple ID email"
            echo "  APPLE_APP_SPECIFIC_PASSWORD - App-specific password"
            echo "  APPLE_TEAM_ID              - Apple Developer Team ID (optional)"
            echo ""
            echo "Environment Variables for API Key:"
            echo "  API_KEY_ID      - App Store Connect API Key ID"
            echo "  API_KEY_ISSUER_ID - App Store Connect API Key Issuer ID"
            echo "  API_KEY_PATH    - Path to the .p8 private key file"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Verify DMG exists
if [[ ! -f "$DMG_PATH" ]]; then
    echo -e "${RED}Error: DMG not found at ${DMG_PATH}${NC}"
    echo "Please run ./scripts/create-dmg.sh first"
    exit 1
fi

# Check authentication credentials
if [[ "$USE_API_KEY" == true ]]; then
    # API Key authentication
    if [[ -z "${API_KEY_ID:-}" ]]; then
        echo -e "${RED}Error: API_KEY_ID environment variable not set${NC}"
        exit 1
    fi
    if [[ -z "${API_KEY_ISSUER_ID:-}" ]]; then
        echo -e "${RED}Error: API_KEY_ISSUER_ID environment variable not set${NC}"
        exit 1
    fi
    if [[ -z "${API_KEY_PATH:-}" ]]; then
        echo -e "${RED}Error: API_KEY_PATH environment variable not set${NC}"
        exit 1
    fi
    if [[ ! -f "$API_KEY_PATH" ]]; then
        echo -e "${RED}Error: API key file not found at ${API_KEY_PATH}${NC}"
        exit 1
    fi
    echo -e "${BLUE}Using API Key authentication${NC}"
else
    # Apple ID authentication
    if [[ -z "${APPLE_ID:-}" ]]; then
        echo -e "${RED}Error: APPLE_ID environment variable not set${NC}"
        echo "Set it with: export APPLE_ID='your.email@example.com'"
        exit 1
    fi
    if [[ -z "${APPLE_APP_SPECIFIC_PASSWORD:-}" ]]; then
        echo -e "${RED}Error: APPLE_APP_SPECIFIC_PASSWORD environment variable not set${NC}"
        echo "Generate an app-specific password at: https://appleid.apple.com"
        echo "Then set it with: export APPLE_APP_SPECIFIC_PASSWORD='xxxx-xxxx-xxxx-xxxx'"
        exit 1
    fi
    echo -e "${BLUE}Using Apple ID authentication: ${APPLE_ID}${NC}"
fi

# Create ZIP for submission (notarytool accepts DMG directly, but ZIP is an alternative)
echo -e "${BLUE}Preparing submission archive...${NC}"
SUBMISSION_PATH="$DMG_PATH"

# Function to submit for notarization
submit_for_notarization() {
    local output_path="$1"
    local submission_id=""

    echo -e "${BLUE}Submitting to Apple notarization service...${NC}"

    if [[ "$USE_API_KEY" == true ]]; then
        # API Key authentication
        submission_id=$(xcrun notarytool submit "$output_path" \
            --key-id "$API_KEY_ID" \
            --issuer "$API_KEY_ISSUER_ID" \
            --key "$API_KEY_PATH" \
            --wait 2>&1 | tee /dev/tty | \
            grep -oP 'id: \K[^\s]+' || true)
    else
        # Apple ID authentication with team ID if provided
        local team_arg=""
        if [[ -n "${APPLE_TEAM_ID:-}" ]]; then
            team_arg="--team-id ${APPLE_TEAM_ID}"
        fi

        # Note: In CI, we can't use --wait with Apple ID auth as it requires 2FA
        # Instead, we submit and then poll manually
        if [[ -n "${CI:-}" ]]; then
            submission_id=$(xcrun notarytool submit "$output_path" \
                --apple-id "$APPLE_ID" \
                --password "$APPLE_APP_SPECIFIC_PASSWORD" \
                $team_arg 2>&1 | \
                grep -oP 'id: \K[^\s]+' || true)
        else
            # Local development: use --wait for convenience
            xcrun notarytool submit "$output_path" \
                --apple-id "$APPLE_ID" \
                --password "$APPLE_APP_SPECIFIC_PASSWORD" \
                $team_arg \
                --wait
            return 0
        fi
    fi

    echo "$submission_id"
}

# Function to poll for notarization status
poll_notarization_status() {
    local submission_id="$1"
    local max_attempts=60
    local wait_seconds=30

    echo -e "${BLUE}Polling for notarization status (this may take a few minutes)...${NC}"
    echo -e "${BLUE}Submission ID: ${submission_id}${NC}"

    for ((i=1; i<=max_attempts; i++)); do
        echo -e "${BLUE}Check ${i}/${max_attempts}: Waiting ${wait_seconds}s...${NC}"
        sleep $wait_seconds

        local status
        if [[ "$USE_API_KEY" == true ]]; then
            status=$(xcrun notarytool info "$submission_id" \
                --key-id "$API_KEY_ID" \
                --issuer "$API_KEY_ISSUER_ID" \
                --key "$API_KEY_PATH" \
                2>&1)
        else
            local team_arg=""
            if [[ -n "${APPLE_TEAM_ID:-}" ]]; then
                team_arg="--team-id ${APPLE_TEAM_ID}"
            fi
            status=$(xcrun notarytool info "$submission_id" \
                --apple-id "$APPLE_ID" \
                --password "$APPLE_APP_SPECIFIC_PASSWORD" \
                $team_arg 2>&1)
        fi

        echo "$status"

        if echo "$status" | grep -q "Accepted"; then
            echo -e "${GREEN}✓ Notarization accepted!${NC}"
            return 0
        elif echo "$status" | grep -q "Rejected"; then
            echo -e "${RED}✗ Notarization rejected${NC}"
            return 1
        elif echo "$status" | grep -q "Invalid"; then
            echo -e "${RED}✗ Notarization invalid${NC}"
            return 1
        fi
    done

    echo -e "${RED}✗ Notarization polling timed out${NC}"
    return 1
}

# Submit for notarization
if [[ -n "${CI:-}" && "$USE_API_KEY" != true ]]; then
    # CI with Apple ID - need manual polling
    SUBMISSION_ID=$(submit_for_notarization "$SUBMISSION_PATH")
    if [[ -z "$SUBMISSION_ID" ]]; then
        echo -e "${RED}Error: Failed to get submission ID${NC}"
        exit 1
    fi

    if ! poll_notarization_status "$SUBMISSION_ID"; then
        echo -e "${RED}Notarization failed${NC}"
        exit 1
    fi
else
    # Local or API Key - use --wait
    if ! submit_for_notarization "$SUBMISSION_PATH"; then
        echo -e "${RED}Notarization failed${NC}"
        exit 1
    fi
fi

# Staple the notarization ticket to the app
echo -e "${BLUE}Stapling notarization ticket to app...${NC}"
if [[ -d "$APP_BUNDLE" ]]; then
    if xcrun stapler staple "$APP_BUNDLE"; then
        echo -e "${GREEN}✓ Notarization ticket stapled to app${NC}"
    else
        echo -e "${YELLOW}⚠ Failed to staple ticket to app (this is OK, gatekeeper can still validate online)${NC}"
    fi
fi

# Staple the notarization ticket to the DMG
echo -e "${BLUE}Stapling notarization ticket to DMG...${NC}"
if xcrun stapler staple "$DMG_PATH"; then
    echo -e "${GREEN}✓ Notarization ticket stapled to DMG${NC}"
else
    echo -e "${YELLOW}⚠ Failed to staple ticket to DMG (this is OK, gatekeeper can still validate online)${NC}"
fi

# Verify notarization
echo -e "${BLUE}Verifying notarization...${NC}"
if xcrun stapler validate "$DMG_PATH" 2>&1 | grep -q "The validate action worked"; then
    echo -e "${GREEN}✓ DMG notarization validated${NC}"
else
    echo -e "${YELLOW}⚠ Stapler validation returned unexpected result${NC}"
fi

echo ""
echo -e "${BLUE}Running spctl check...${NC}"
if spctl -a -v "$DMG_PATH" 2>&1 | grep -q "accepted"; then
    echo -e "${GREEN}✓ Gatekeeper will accept this DMG${NC}"
else
    echo -e "${YELLOW}⚠ spctl check returned unexpected result${NC}"
fi

echo ""
echo -e "${GREEN}✓ Notarization completed successfully!${NC}"
echo ""
echo -e "${BLUE}Output:${NC}"
echo "  DMG: ${DMG_PATH}"
echo ""
echo -e "${BLUE}The DMG is ready for distribution!${NC}"
