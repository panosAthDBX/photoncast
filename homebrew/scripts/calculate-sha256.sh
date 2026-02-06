#!/bin/bash
#
# Calculate SHA256 hash for PhotonCast DMG
# Usage: ./calculate-sha256.sh [path-to-dmg]
#
# If no path is provided, searches for PhotonCast-*.dmg in current directory
# and common build locations.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to calculate SHA256
calculate_sha256() {
    local dmg_path="$1"

    if [[ ! -f "$dmg_path" ]]; then
        print_error "File not found: $dmg_path"
        return 1
    fi

    print_info "Calculating SHA256 for: $dmg_path"

    # Calculate SHA256 using shasum (macOS native) or sha256sum (Linux)
    local sha256
    if command -v shasum &> /dev/null; then
        sha256=$(shasum -a 256 "$dmg_path" | awk '{print $1}')
    elif command -v sha256sum &> /dev/null; then
        sha256=$(sha256sum "$dmg_path" | awk '{print $1}')
    else
        print_error "Neither shasum nor sha256sum found"
        return 1
    fi

    echo ""
    print_info "SHA256 hash:"
    echo "  $sha256"
    echo ""
    print_info "File size:"
    ls -lh "$dmg_path" | awk '{print "  " $5}'
    echo ""
    print_info "Formula snippet:"
    echo "  sha256 \"$sha256\""

    return 0
}

# Function to find DMG file
find_dmg() {
    local search_paths=(
        "."
        "./dist"
        "./build"
        "./target/release"
        "./target/debug"
        "../dist"
        "../build"
    )

    for path in "${search_paths[@]}"; do
        local found
        found=$(find "$path" -maxdepth 1 -name "PhotonCast-*.dmg" -type f 2>/dev/null | head -1)
        if [[ -n "$found" ]]; then
            echo "$found"
            return 0
        fi
    done

    return 1
}

# Main script
main() {
    local dmg_path=""

    # Check if path was provided as argument
    if [[ $# -ge 1 ]]; then
        dmg_path="$1"
    else
        print_info "No DMG path provided, searching for PhotonCast-*.dmg..."
        dmg_path=$(find_dmg) || true
    fi

    # If still no DMG found, show help
    if [[ -z "$dmg_path" ]]; then
        print_warning "No PhotonCast DMG file found automatically"
        echo ""
        echo "Usage: $0 [path-to-dmg]"
        echo ""
        echo "Examples:"
        echo "  $0"
        echo "  $0 ./dist/PhotonCast-0.1.0-alpha.dmg"
        echo "  $0 ~/Downloads/PhotonCast-0.1.0.dmg"
        echo ""
        print_info "Searched in:"
        echo "  - Current directory"
        echo "  - ./dist"
        echo "  - ./build"
        echo "  - ./target/release"
        echo "  - ./target/debug"
        exit 1
    fi

    # Calculate SHA256
    calculate_sha256 "$dmg_path"
}

main "$@"
