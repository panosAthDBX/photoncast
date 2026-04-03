#!/bin/bash
#
# run-launcher-appear-proof.sh - Manual launcher appear-time proof harness
#
# Builds PhotonCast, launches it with a test-only env var that opens the
# launcher window on startup, and measures how long it takes for a window to
# become visible via System Events polling.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "$PROJECT_ROOT"

echo -e "${YELLOW}Building photoncast binary...${NC}"
cargo build -p photoncast -q

TARGET_DIR="$(cargo metadata --format-version 1 --no-deps | python -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
BINARY_PATH="${TARGET_DIR}/debug/photoncast"

if [[ ! -x "$BINARY_PATH" ]]; then
  echo -e "${RED}Built binary not found: ${BINARY_PATH}${NC}"
  exit 1
fi

echo -e "${GREEN}Launching PhotonCast appear-time harness...${NC}"
START_NS="$(python - <<'PY'
import time
print(time.time_ns())
PY
)"
MARKERS_PATH="/tmp/photoncast-appear-proof-markers.csv"
rm -f "$MARKERS_PATH"

PHOTONCAST_PERF_MARKERS_PATH="$MARKERS_PATH" "$BINARY_PATH" >/tmp/photoncast-appear-proof.log 2>&1 &
APP_PID=$!

cleanup() {
  kill "$APP_PID" >/dev/null 2>&1 || true
}
trap cleanup EXIT

deadline=$((SECONDS + 10))
VISIBLE_MS=""
if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo -e "${RED}PhotonCast exited before a window became visible.${NC}"
  echo "See /tmp/photoncast-appear-proof.log"
  exit 1
fi

VISIBLE_MS="$(swift -e "
import CoreGraphics
import Foundation

let pid = Int32(CommandLine.arguments[1])!
let startNs = UInt64(CommandLine.arguments[2])!
let deadline = Date().addingTimeInterval(10)
let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]

while Date() < deadline {
    let info = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] ?? []
    let visible = info.contains { entry in
        (entry[kCGWindowOwnerPID as String] as? Int32) == pid
    }
    if visible {
        let nowNs = UInt64(Date().timeIntervalSince1970 * 1_000_000_000)
        let elapsedMs = Double(nowNs - startNs) / 1_000_000
        print(elapsedMs)
        exit(0)
    }
    usleep(50_000)
}

exit(1)
" "$APP_PID" "$START_NS" 2>/dev/null || true)"

if [[ -z "$VISIBLE_MS" ]]; then
  echo -e "${RED}Timed out waiting for launcher window visibility.${NC}"
  echo "See /tmp/photoncast-appear-proof.log"
  exit 1
fi

echo -e "${GREEN}Launcher window became visible in ${VISIBLE_MS} ms${NC}"
if [[ -f "$MARKERS_PATH" ]]; then
  echo
  echo "Internal markers:"
  cat "$MARKERS_PATH"
fi
echo
echo "Interpretation:"
echo "- Compare this value against the documented '<50ms' window-appear target."
echo "- This is a manual app-shell measurement, not a CI-safe automated proof."
echo "- Log file: /tmp/photoncast-appear-proof.log"
