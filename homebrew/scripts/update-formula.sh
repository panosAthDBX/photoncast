#!/bin/bash
#
# Update Homebrew Cask formula for new PhotonCast releases
# Usage: ./update-formula.sh <version> [dmg-path]
#
# Example:
#   ./update-formula.sh 0.1.0-alpha ./dist/PhotonCast-0.1.0-alpha.dmg
#   ./update-formula.sh 0.2.0  # Will search for DMG automatically
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
HOMEBREW_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$HOMEBREW_DIR")"
FORMULA_FILE="$HOMEBREW_DIR/photoncast.rb"

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

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Function to display usage
usage() {
    echo "Update Homebrew Cask formula for PhotonCast"
    echo ""
    echo "Usage: $0 <version> [dmg-path]"
    echo ""
    echo "Arguments:"
    echo "  version   - New version number (e.g., 0.1.0-alpha, 0.2.0)"
    echo "  dmg-path  - Path to the DMG file (optional, will search if not provided)"
    echo ""
    echo "Examples:"
    echo "  $0 0.1.0-alpha ./dist/PhotonCast-0.1.0-alpha.dmg"
    echo "  $0 0.2.0"
    echo ""
    echo "Environment variables:"
    echo "  DRY_RUN=1    - Preview changes without modifying the formula"
    echo "  SKIP_TESTS=1 - Skip audit and style checks"
}

# Function to find DMG file
find_dmg() {
    local version="$1"
    local search_paths=(
        "$PROJECT_ROOT/dist"
        "$PROJECT_ROOT/build"
        "$PROJECT_ROOT/target/release"
        "$PROJECT_ROOT/target/debug"
        "."
        "./dist"
        "./build"
    )

    for path in "${search_paths[@]}"; do
        # Try exact match first
        local exact_match="$path/PhotonCast-$version.dmg"
        if [[ -f "$exact_match" ]]; then
            echo "$exact_match"
            return 0
        fi

        # Try wildcard match
        local found
        found=$(find "$path" -maxdepth 1 -name "PhotonCast-*.dmg" -type f 2>/dev/null | head -1)
        if [[ -n "$found" ]]; then
            echo "$found"
            return 0
        fi
    done

    return 1
}

# Function to calculate SHA256
calculate_sha256() {
    local dmg_path="$1"

    if [[ ! -f "$dmg_path" ]]; then
        print_error "File not found: $dmg_path"
        return 1
    fi

    # Calculate SHA256
    local sha256
    if command -v shasum &> /dev/null; then
        sha256=$(shasum -a 256 "$dmg_path" | awk '{print $1}')
    elif command -v sha256sum &> /dev/null; then
        sha256=$(sha256sum "$dmg_path" | awk '{print $1}')
    else
        print_error "Neither shasum nor sha256sum found"
        return 1
    fi

    echo "$sha256"
}

# Function to update formula file
update_formula() {
    local version="$1"
    local sha256="$2"
    local formula_path="$3"

    print_step "Updating formula: $formula_path"

    # Create backup
    cp "$formula_path" "${formula_path}.backup"

    # Update version
    sed -i.bak "s/version \"[^\"]*\"/version \"$version\"/" "$formula_path"

    # Update sha256
    sed -i.bak "s/sha256 :no_check/sha256 \"$sha256\"/" "$formula_path"
    sed -i.bak "s/sha256 \"[^\"]*\"/sha256 \"$sha256\"/" "$formula_path"

    # Remove .bak files created by sed
    rm -f "${formula_path}.backup" "${formula_path}.bak"

    print_info "Formula updated successfully!"
}

# Function to verify formula with brew
verify_formula() {
    local formula_path="$1"

    if ! command -v brew &> /dev/null; then
        print_warning "Homebrew not found, skipping verification"
        return 0
    fi

    print_step "Running Homebrew audit..."
    if brew audit --cask "$formula_path" 2>&1; then
        print_info "Audit passed!"
    else
        print_warning "Audit found issues (see above)"
    fi

    print_step "Running Homebrew style check..."
    if brew style --fix "$formula_path" 2>&1; then
        print_info "Style check passed!"
    else
        print_warning "Style issues found (see above)"
    fi
}

# Function to show diff
show_diff() {
    local formula_path="$1"

    if command -v git &> /dev/null && git rev-parse --git-dir > /dev/null 2>&1; then
        echo ""
        print_step "Changes made to formula:"
        git diff "$formula_path" || true
    fi
}

# Main script
main() {
    # Check arguments
    if [[ $# -lt 1 ]]; then
        usage
        exit 1
    fi

    local version="$1"
    local dmg_path="${2:-}"

    # Validate version format (basic check)
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+ ]]; then
        print_warning "Version '$version' doesn't match expected semver format (e.g., 0.1.0-alpha)"
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi

    print_info "Updating formula for PhotonCast v$version"

    # Check if formula file exists
    if [[ ! -f "$FORMULA_FILE" ]]; then
        print_error "Formula file not found: $FORMULA_FILE"
        exit 1
    fi

    # Find DMG if not provided
    if [[ -z "$dmg_path" ]]; then
        print_step "Searching for DMG file..."
        dmg_path=$(find_dmg "$version") || true

        if [[ -z "$dmg_path" ]]; then
            print_error "Could not find PhotonCast DMG file automatically"
            echo ""
            echo "Searched in:"
            echo "  - $PROJECT_ROOT/dist"
            echo "  - $PROJECT_ROOT/build"
            echo "  - $PROJECT_ROOT/target/release"
            echo "  - ./dist"
            echo "  - ./build"
            echo ""
            echo "Please provide the DMG path as the second argument:"
            echo "  $0 $version ./path/to/PhotonCast-$version.dmg"
            exit 1
        fi
    fi

    print_info "Using DMG: $dmg_path"

    # Calculate SHA256
    print_step "Calculating SHA256 hash..."
    local sha256
    sha256=$(calculate_sha256 "$dmg_path")

    if [[ -z "$sha256" ]]; then
        print_error "Failed to calculate SHA256"
        exit 1
    fi

    print_info "SHA256: $sha256"

    # Show current formula version
    local current_version
    current_version=$(grep -o 'version "[^"]*"' "$FORMULA_FILE" | head -1 | cut -d'"' -f2)
    print_info "Current formula version: $current_version"
    print_info "New formula version: $version"

    # Dry run mode
    if [[ "${DRY_RUN:-}" == "1" ]]; then
        echo ""
        print_warning "DRY RUN MODE - No changes will be made"
        echo "Would update:"
        echo "  Version: $current_version -> $version"
        echo "  SHA256:  -> $sha256"
        exit 0
    fi

    # Update formula
    update_formula "$version" "$sha256" "$FORMULA_FILE"

    # Show diff
    show_diff "$FORMULA_FILE"

    # Verify formula (unless skipped)
    if [[ "${SKIP_TESTS:-}" != "1" ]]; then
        verify_formula "$FORMULA_FILE"
    fi

    echo ""
    print_info "Formula update complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Test the formula locally:"
    echo "     brew install --cask $FORMULA_FILE"
    echo ""
    echo "  2. If using a custom tap, commit and push:"
    echo "     git add homebrew/"
    echo "     git commit -m \"chore(homebrew): update formula to v$version\""
    echo "     git push origin main"
    echo ""
    echo "  3. To submit to Homebrew/homebrew-cask, see:"
    echo "     homebrew/SUBMISSION.md"
}

main "$@"
