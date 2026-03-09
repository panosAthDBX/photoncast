#!/bin/bash
#
# PhotonCast Local Install Script
#
# Installs the built app bundle into /Applications (or a custom destination),
# replacing the previous install in one step and optionally reopening the app.
#
# Usage:
#   ./scripts/install-app.sh
#   ./scripts/install-app.sh --reset-accessibility
#   ./scripts/install-app.sh --source /path/to/PhotonCast.app --dest ~/Applications/PhotonCast.app
#

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

APP_NAME="PhotonCast"
BUNDLE_ID="com.photoncast.app"
SOURCE_APP="${PROJECT_ROOT}/build/${APP_NAME}.app"
DEST_APP="/Applications/${APP_NAME}.app"
LAUNCH_AFTER_INSTALL=true
RESET_ACCESSIBILITY=false
OPEN_ACCESSIBILITY_SETTINGS=false
WAIT_SECONDS=20

usage() {
    cat <<EOF
Usage: ./scripts/install-app.sh [options]

Options:
  --source PATH                  Source app bundle (default: build/PhotonCast.app)
  --dest PATH                    Destination app bundle (default: /Applications/PhotonCast.app)
  --no-launch                    Do not relaunch PhotonCast after install
  --reset-accessibility          Reset the Accessibility TCC entry for PhotonCast
  --open-accessibility-settings  Open the Accessibility settings pane after install
  --help                         Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --source)
            SOURCE_APP="$2"
            shift 2
            ;;
        --dest)
            DEST_APP="$2"
            shift 2
            ;;
        --no-launch)
            LAUNCH_AFTER_INSTALL=false
            shift
            ;;
        --reset-accessibility)
            RESET_ACCESSIBILITY=true
            OPEN_ACCESSIBILITY_SETTINGS=true
            shift
            ;;
        --open-accessibility-settings)
            OPEN_ACCESSIBILITY_SETTINGS=true
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ ! -d "$SOURCE_APP" ]]; then
    echo -e "${RED}Error: Source app bundle not found at ${SOURCE_APP}${NC}" >&2
    echo "Run ./scripts/release-build.sh first." >&2
    exit 1
fi

DEST_PARENT="$(dirname "$DEST_APP")"
TMP_APP="${DEST_APP}.installing.$$"
BACKUP_APP="${DEST_APP}.backup.$$"

cleanup() {
    rm -rf "$TMP_APP"
}
trap cleanup EXIT

echo -e "${BLUE}Source:${NC} ${SOURCE_APP}"
echo -e "${BLUE}Destination:${NC} ${DEST_APP}"

DESIGNATED_REQUIREMENT=$(codesign -d -r- "$SOURCE_APP" 2>&1 | sed -n 's/^# designated => //p')
if [[ -n "$DESIGNATED_REQUIREMENT" ]]; then
    echo -e "${BLUE}Designated requirement:${NC} ${DESIGNATED_REQUIREMENT}"
fi

if [[ "$DESIGNATED_REQUIREMENT" == cdhash* ]]; then
    echo -e "${YELLOW}Warning: this build is ad-hoc signed.${NC}"
    echo -e "${YELLOW}macOS may require Accessibility/Calendar permissions to be re-granted after install.${NC}"
fi

quit_running_app() {
    if ! pgrep -x photoncast >/dev/null 2>&1; then
        return
    fi

    echo -e "${BLUE}Quitting running PhotonCast instance...${NC}"
    osascript -e 'tell application id "com.photoncast.app" to quit' >/dev/null 2>&1 || true

    for ((i = 0; i < WAIT_SECONDS; i++)); do
        if ! pgrep -x photoncast >/dev/null 2>&1; then
            return
        fi
        sleep 1
    done

    if pgrep -x photoncast >/dev/null 2>&1; then
        echo -e "${YELLOW}PhotonCast did not exit in time; sending TERM...${NC}"
        pkill -x photoncast >/dev/null 2>&1 || true
        sleep 1
    fi

    if pgrep -x photoncast >/dev/null 2>&1; then
        echo -e "${RED}Error: PhotonCast is still running. Quit it and retry.${NC}" >&2
        exit 1
    fi
}

quit_running_app

mkdir -p "$DEST_PARENT"

echo -e "${BLUE}Staging app bundle...${NC}"
rm -rf "$TMP_APP"
ditto "$SOURCE_APP" "$TMP_APP"

echo -e "${BLUE}Installing app bundle...${NC}"
if [[ -e "$DEST_APP" ]]; then
    rm -rf "$BACKUP_APP"
    mv "$DEST_APP" "$BACKUP_APP"
fi

if ! mv "$TMP_APP" "$DEST_APP"; then
    echo -e "${RED}Install failed while moving the new app bundle into place.${NC}" >&2
    if [[ -e "$BACKUP_APP" && ! -e "$DEST_APP" ]]; then
        mv "$BACKUP_APP" "$DEST_APP"
    fi
    exit 1
fi

rm -rf "$BACKUP_APP"

echo -e "${BLUE}Verifying installed bundle...${NC}"
codesign --verify --deep --strict "$DEST_APP"
echo -e "${GREEN}✓ Installed ${DEST_APP}${NC}"

if [[ "$RESET_ACCESSIBILITY" == true ]]; then
    echo -e "${BLUE}Resetting Accessibility permission entry...${NC}"
    tccutil reset Accessibility "$BUNDLE_ID" >/dev/null 2>&1 || true
fi

if [[ "$OPEN_ACCESSIBILITY_SETTINGS" == true ]]; then
    echo -e "${BLUE}Opening Accessibility settings...${NC}"
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility" >/dev/null 2>&1 || true
fi

if [[ "$LAUNCH_AFTER_INSTALL" == true ]]; then
    echo -e "${BLUE}Launching PhotonCast...${NC}"
    open "$DEST_APP"
fi

echo ""
echo -e "${GREEN}Install complete.${NC}"
if [[ "$DESIGNATED_REQUIREMENT" == cdhash* ]]; then
    echo -e "${YELLOW}Because this build is ad-hoc signed, macOS may still ask for permissions again after code changes.${NC}"
fi
