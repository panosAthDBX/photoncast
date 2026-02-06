#!/bin/bash
#
# generate-appcast.sh - Generate Sparkle appcast XML for PhotonCast auto-updates
#
# Usage: ./scripts/generate-appcast.sh <version> <build_number> <dmg_url> [dmg_path]
#
# Arguments:
#   version      - Release version (e.g., 1.0.0)
#   build_number - Build number for sparkle:version (e.g., 100 or 1.0.0)
#   dmg_url      - Public URL where the DMG will be hosted
#   dmg_path     - Local path to DMG file (for signature generation)
#
# Environment Variables:
#   SPARKLE_SIGNING_KEY - Path to EdDSA private key for signing (default: ./certs/sparkle_signing.key)
#   RELEASE_NOTES       - Release notes text (default: "Bug fixes and improvements")
#
# Example:
#   ./scripts/generate-appcast.sh 1.1.0 110 https://api.photoncast.app/releases/1.1.0/PhotonCast.dmg ./dist/PhotonCast.dmg
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Configuration
APPCAST_TEMPLATE="${PROJECT_ROOT}/resources/appcast-template.xml"
OUTPUT_DIR="${PROJECT_ROOT}/dist"
SIGNING_KEY="${SPARKLE_SIGNING_KEY:-${PROJECT_ROOT}/certs/sparkle_signing.key}"

# Help message
usage() {
    echo "Usage: $0 <version> <build_number> <dmg_url> [dmg_path]"
    echo ""
    echo "Arguments:"
    echo "  version      - Release version (e.g., 1.0.0)"
    echo "  build_number - Build number for sparkle:version (e.g., 100)"
    echo "  dmg_url      - Public URL where the DMG will be hosted"
    echo "  dmg_path     - Local path to DMG file (for signature generation)"
    echo ""
    echo "Environment Variables:"
    echo "  SPARKLE_SIGNING_KEY - Path to EdDSA private key (default: ./certs/sparkle_signing.key)"
    echo "  RELEASE_NOTES       - Release notes text"
    exit 1
}

# Check dependencies
check_dependencies() {
    local missing_deps=()

    if ! command -v openssl &> /dev/null; then
        missing_deps+=("openssl")
    fi

    if ! command -v date &> /dev/null; then
        missing_deps+=("date")
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo -e "${RED}Error: Missing required dependencies: ${missing_deps[*]}${NC}" >&2
        exit 1
    fi
}

# Validate arguments
validate_args() {
    if [ $# -lt 3 ]; then
        usage
    fi

    VERSION="$1"
    BUILD_NUMBER="$2"
    DMG_URL="$3"
    DMG_PATH="${4:-}"

    # Validate version format (semver-like)
    if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo -e "${RED}Error: Version must be in format X.Y.Z (e.g., 1.0.0)${NC}" >&2
        exit 1
    fi

    # Validate DMG path if provided
    if [ -n "$DMG_PATH" ] && [ ! -f "$DMG_PATH" ]; then
        echo -e "${RED}Error: DMG file not found: $DMG_PATH${NC}" >&2
        exit 1
    fi

    # Check for template file
    if [ ! -f "$APPCAST_TEMPLATE" ]; then
        echo -e "${RED}Error: Appcast template not found: $APPCAST_TEMPLATE${NC}" >&2
        exit 1
    fi
}

# Generate EdDSA signature for DMG using Sparkle's sign_update or OpenSSL
generate_signature() {
    local dmg_path="$1"
    local signature=""

    echo -e "${YELLOW}Generating EdDSA signature for DMG...${NC}"

    # Method 1: Use Sparkle's sign_update tool if available
    if command -v sign_update &> /dev/null; then
        echo "Using Sparkle sign_update tool..."
        if [ -f "$SIGNING_KEY" ]; then
            signature=$(sign_update "$dmg_path" -s "$SIGNING_KEY" 2>/dev/null || echo "")
        else
            echo -e "${YELLOW}Warning: Signing key not found at $SIGNING_KEY${NC}"
            echo -e "${YELLOW}Run: ./scripts/generate-signing-key.sh to generate a key${NC}"
        fi
    fi

    # Method 2: Use OpenSSL for Ed25519 signature
    if [ -z "$signature" ] && command -v openssl &> /dev/null; then
        if [ -f "$SIGNING_KEY" ]; then
            echo "Using OpenSSL for Ed25519 signature..."
            # Generate signature using Ed25519
            signature=$(openssl dgst -sha256 -sign "$SIGNING_KEY" "$dmg_path" 2>/dev/null | base64 || echo "")
        fi
    fi

    # Method 3: Use generate_appcast from Sparkle if available
    if [ -z "$signature" ] && command -v generate_appcast &> /dev/null; then
        echo "Using Sparkle generate_appcast tool..."
        # This generates the entire appcast, but we'll extract just the signature
        local temp_appcast
        temp_appcast=$(mktemp)
        generate_appcast --key-file "$SIGNING_KEY" "$dmg_path" > "$temp_appcast" 2>/dev/null || true
        signature=$(grep -o 'sparkle:edSignature="[^"]*"' "$temp_appcast" | head -1 | sed 's/sparkle:edSignature="//;s/"$//' || echo "")
        rm -f "$temp_appcast"
    fi

    if [ -z "$signature" ]; then
        echo -e "${YELLOW}Warning: Could not generate signature. Using placeholder.${NC}"
        echo -e "${YELLOW}To enable signing, install Sparkle tools or provide a signing key.${NC}"
        signature="SIGNATURE_PLACEHOLDER"
    else
        echo -e "${GREEN}Signature generated successfully${NC}"
    fi

    echo "$signature"
}

# Get file size
get_file_size() {
    local file_path="$1"
    if [ -f "$file_path" ]; then
        stat -f%z "$file_path" 2>/dev/null || stat -c%s "$file_path" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

# Generate RFC 2822 date
generate_pub_date() {
    date -R 2>/dev/null || date "+%a, %d %b %Y %H:%M:%S %z"
}

# Main generation function
generate_appcast() {
    local version="$1"
    local build_number="$2"
    local dmg_url="$3"
    local dmg_path="$4"

    echo -e "${GREEN}Generating appcast for PhotonCast v${version}${NC}"
    echo "Build Number: $build_number"
    echo "DMG URL: $dmg_url"

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Generate dates
    local pub_date build_date
    pub_date=$(generate_pub_date)
    build_date=$(date "+%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u "+%Y-%m-%dT%H:%M:%SZ")

    # Get file size and signature
    local dmg_size="0"
    local signature="SIGNATURE_PLACEHOLDER"

    if [ -n "$dmg_path" ] && [ -f "$dmg_path" ]; then
        dmg_size=$(get_file_size "$dmg_path")
        signature=$(generate_signature "$dmg_path")
    else
        echo -e "${YELLOW}Warning: DMG path not provided or file not found. Using placeholder values.${NC}"
    fi

    # Get release notes
    local release_notes="${RELEASE_NOTES:-Bug fixes and improvements.}"

    # Read template and substitute values
    local appcast_content
    appcast_content=$(cat "$APPCAST_TEMPLATE")

    # Perform substitutions
    appcast_content="${appcast_content//\{\{VERSION\}\}/$version}"
    appcast_content="${appcast_content//\{\{BUILD_NUMBER\}\}/$build_number}"
    appcast_content="${appcast_content//\{\{DMG_URL\}\}/$dmg_url}"
    appcast_content="${appcast_content//\{\{DMG_SIZE\}\}/$dmg_size}"
    appcast_content="${appcast_content//\{\{ED_SIGNATURE\}\}/$signature}"
    appcast_content="${appcast_content//\{\{PUB_DATE\}\}/$pub_date}"
    appcast_content="${appcast_content//\{\{BUILD_DATE\}\}/$build_date}"
    appcast_content="${appcast_content//\{\{RELEASE_NOTES\}\}/$release_notes}"

    # Write output
    local output_file="${OUTPUT_DIR}/appcast-${version}.xml"
    echo "$appcast_content" > "$output_file"

    echo ""
    echo -e "${GREEN}Appcast generated: $output_file${NC}"
    echo ""
    echo "To use this appcast:"
    echo "  1. Upload the DMG to: $dmg_url"
    echo "  2. Host this appcast XML at your update server"
    echo "  3. Configure Sparkle with feed URL: https://api.photoncast.app/updates/appcast.xml"
    echo ""
    echo "File details:"
    echo "  Size: $dmg_size bytes"
    echo "  Signature: ${signature:0:40}..."
}

# Generate signing key helper info
generate_signing_key_info() {
    cat << 'EOF'

To generate a signing key for Sparkle:

1. Using Sparkle's generate_keys tool:
   $ ./bin/generate_keys
   This creates EdDSA keys in your Keychain.

2. Using OpenSSL for Ed25519:
   $ openssl genpkey -algorithm Ed25519 -out certs/sparkle_signing.key
   $ openssl pkey -in certs/sparkle_signing.key -pubout -out certs/sparkle_signing.pub

3. Extract public key for app bundle:
   The public key should be embedded in your app for signature verification.

See: https://sparkle-project.org/documentation/#3-segue-for-security-concerns
EOF
}

# Main execution
main() {
    check_dependencies
    validate_args "$@"
    generate_appcast "$VERSION" "$BUILD_NUMBER" "$DMG_URL" "$DMG_PATH"

    # Show signing key info if signature is placeholder
    if [ -n "$DMG_PATH" ] && [ ! -f "$SIGNING_KEY" ]; then
        echo ""
        echo -e "${YELLOW}Note: To enable signature generation, set up a signing key:${NC}"
        generate_signing_key_info
    fi
}

main "$@"
