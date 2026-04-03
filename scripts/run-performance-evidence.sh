#!/bin/bash
#
# run-performance-evidence.sh - Run the currently available PhotonCast performance proof checks
#
# This script executes the automated smoke checks that are currently feasible in
# this repository without requiring the full GPUI + Metal runtime harness. It
# also reports the remaining manual checks needed for hotkey latency and GPUI
# rendering/appear-time validation.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "$PROJECT_ROOT"

run_check() {
    local label="$1"
    shift

    echo -e "${YELLOW}==>${NC} ${label}"
    "$@"
    echo -e "${GREEN}PASS${NC} ${label}"
    echo
}

echo -e "${GREEN}PhotonCast performance evidence run${NC}"
echo

run_check \
    "Search latency smoke proof (<30ms target path)" \
    cargo test -p photoncast-core test_search_with_200_apps_under_30ms_smoke -- --nocapture

run_check \
    "App initialization performance snapshot" \
    cargo test -p photoncast-core test_app_initialization_performance_snapshot -- --nocapture

echo -e "${YELLOW}Remaining manual / environment-gated checks${NC}"
echo "1. Hotkey end-to-end response latency (<50ms) still needs a dedicated app-shell proof path."
echo "2. GPUI rendering / launcher appear-time checks remain in tests/integration/gpui_test.rs and"
echo "   require a full Xcode + Metal Toolchain environment."
echo

if command -v xcodebuild >/dev/null 2>&1; then
    echo "Suggested next command for Metal-backed GPUI validation:"
    echo "  xcodebuild -downloadComponent MetalToolchain"
else
    echo -e "${RED}xcodebuild not found${NC} - GPUI/Metal validation cannot be run from this shell yet."
fi
