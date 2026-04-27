#!/bin/bash
set -euo pipefail

APP_BUNDLE="${1:-build/PhotonCast.app}"
HELPER="${APP_BUNDLE}/Contents/Library/LaunchServices/com.photoncast.privileged-uninstall.helper"
HELPER_PLIST="${APP_BUNDLE}/Contents/Library/LaunchServices/com.photoncast.privileged-uninstall.helper.plist"
CLIENT="${APP_BUNDLE}/Contents/MacOS/photoncast-privileged-client"
APP_INFO_PLIST="${APP_BUNDLE}/Contents/Info.plist"

test -x "$HELPER"
test -f "$HELPER_PLIST"
test -x "$CLIENT"
test -f "$APP_INFO_PLIST"

codesign --verify --strict --verbose=2 "$HELPER"
codesign --verify --strict --verbose=2 "$CLIENT"
plutil -lint "$HELPER_PLIST"

if ! otool -l "$HELPER" | grep -q "sectname __info_plist"; then
    echo "Helper is missing embedded __TEXT,__info_plist section" >&2
    exit 1
fi

if ! otool -l "$HELPER" | grep -q "sectname __launchd_plist"; then
    echo "Helper is missing embedded __TEXT,__launchd_plist section" >&2
    exit 1
fi

HELPER_ID=$(codesign -d --verbose=2 "$HELPER" 2>&1 | sed -n 's/^Identifier=//p' | head -1)
if [[ "$HELPER_ID" != "com.photoncast.privileged-uninstall.helper" ]]; then
    echo "Unexpected helper identifier: $HELPER_ID" >&2
    exit 1
fi

HELPER_REQUIREMENT=$(/usr/libexec/PlistBuddy -c 'Print :SMPrivilegedExecutables:com.photoncast.privileged-uninstall.helper' "$APP_INFO_PLIST")
codesign --verify -R="$HELPER_REQUIREMENT" --verbose=2 "$HELPER"

APP_REQUIREMENT=$(/usr/bin/strings "$HELPER" | /usr/bin/grep -m 1 -F 'identifier "com.photoncast.app"' | sed -E 's|.*<string>(.*)</string>.*|\1|')
codesign --verify -R="$APP_REQUIREMENT" --verbose=2 "$APP_BUNDLE"

CLIENT_REQUIREMENT=$(/usr/bin/strings "$HELPER" | /usr/bin/grep -m 1 -F 'identifier "photoncast-privileged-client"' | sed -E 's|.*<string>(.*)</string>.*|\1|')
codesign --verify -R="$CLIENT_REQUIREMENT" --verbose=2 "$CLIENT"

echo "privileged helper bundle verification passed"
