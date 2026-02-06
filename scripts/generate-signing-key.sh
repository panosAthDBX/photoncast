#!/bin/bash
#
# generate-signing-key.sh - Generate EdDSA signing key for Sparkle updates
#
# This script generates an Ed25519 key pair for signing app updates.
# The private key signs updates; the public key is embedded in the app.
#
# Usage: ./scripts/generate-signing-key.sh [output_dir]
#

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${1:-${PROJECT_ROOT}/certs}"

# Check for OpenSSL
if ! command -v openssl &> /dev/null; then
    echo -e "${RED}Error: OpenSSL is required but not installed.${NC}" >&2
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

PRIVATE_KEY="$OUTPUT_DIR/sparkle_signing.key"
PUBLIC_KEY="$OUTPUT_DIR/sparkle_signing.pub"
PUBLIC_KEY_DER="$OUTPUT_DIR/sparkle_signing_pub.der"

echo -e "${GREEN}Generating Ed25519 key pair for Sparkle signing...${NC}"
echo "Output directory: $OUTPUT_DIR"
echo ""

# Check if keys already exist
if [ -f "$PRIVATE_KEY" ]; then
    echo -e "${YELLOW}Warning: Private key already exists at $PRIVATE_KEY${NC}"
    read -p "Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 0
    fi
fi

# Generate private key
echo "Generating private key..."
openssl genpkey -algorithm Ed25519 -out "$PRIVATE_KEY"
chmod 600 "$PRIVATE_KEY"

# Extract public key
echo "Extracting public key..."
openssl pkey -in "$PRIVATE_KEY" -pubout -out "$PUBLIC_KEY"

# Convert public key to DER format for Sparkle
echo "Converting public key to DER format..."
openssl pkey -in "$PUBLIC_KEY" -pubin -outform DER -out "$PUBLIC_KEY_DER"

echo ""
echo -e "${GREEN}Key generation complete!${NC}"
echo ""
echo "Files generated:"
echo "  Private key: $PRIVATE_KEY (keep secret!)"
echo "  Public key:  $PUBLIC_KEY"
echo "  Public DER:  $PUBLIC_KEY_DER (for Sparkle)"
echo ""
echo "Next steps:"
echo "  1. Add the private key to your CI/CD secrets (SPARKLE_SIGNING_KEY)"
echo "  2. Embed the DER public key in your app bundle"
echo "  3. Configure Sparkle to use the embedded public key"
echo ""
echo "Public key (hex):"
xxd -p "$PUBLIC_KEY_DER" | tr -d '\n'
echo
echo ""
