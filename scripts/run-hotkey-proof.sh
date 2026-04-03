#!/bin/bash
#
# run-hotkey-proof.sh - Run the current manual hotkey latency proof path
#
# This script runs the ignored hotkey callback latency tests in the photoncast
# app shell target. These tests require a macOS app-shell / main-run-loop
# environment and are not expected to pass in every command-line unit-test
# context.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "$PROJECT_ROOT"

echo -e "${GREEN}PhotonCast hotkey proof path${NC}"
echo
echo -e "${YELLOW}Note:${NC} these ignored tests need a macOS app-shell/main-run-loop context."
echo "They are a manual proof path, not a guaranteed CI-safe check."
echo

echo -e "${YELLOW}==>${NC} Hotkey callback dispatch snapshot"
cargo test -p photoncast test_hotkey_callback_dispatch_latency_snapshot -- --ignored --nocapture || {
  echo
  echo -e "${RED}Hotkey proof path did not pass in this shell context.${NC}"
  echo "This is expected when the unit-test process does not service the app-shell/main run loop."
  exit 1
}

echo
echo -e "${YELLOW}==>${NC} Hotkey callback strict baseline (<50ms)"
cargo test -p photoncast test_hotkey_callback_dispatch_under_50ms_strict -- --ignored --nocapture

echo
echo -e "${GREEN}Hotkey proof path completed.${NC}"
