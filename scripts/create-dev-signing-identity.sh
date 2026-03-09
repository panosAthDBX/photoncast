#!/bin/bash
#
# Creates a stable local code-signing identity for PhotonCast development.
#
# The generated identity is self-signed and trusted for code signing in a
# dedicated keychain so local rebuilds keep the same designated requirement.
#

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# shellcheck source=./lib/signing_env.sh
source "${PROJECT_ROOT}/scripts/lib/signing_env.sh"
load_photoncast_signing_env

IDENTITY_NAME="${PHOTONCAST_SIGNING_IDENTITY:-PhotonCast Local Dev}"
CONFIG_DIR="${HOME}/.config/photoncast"
SIGNING_DIR="${CONFIG_DIR}/dev-signing"
ENV_FILE="${PHOTONCAST_SIGNING_ENV_FILE}"
KEYCHAIN_PATH="${PHOTONCAST_SIGNING_KEYCHAIN:-${HOME}/Library/Keychains/photoncast-dev-signing.keychain-db}"
CERT_PEM="${SIGNING_DIR}/photoncast-local-dev-cert.pem"
KEY_PEM="${SIGNING_DIR}/photoncast-local-dev-key.pem"
P12_PATH="${SIGNING_DIR}/photoncast-local-dev.p12"
OPENSSL_CONFIG="${SIGNING_DIR}/openssl-dev-signing.cnf"
FORCE=false

ensure_keychain_in_search_list() {
    local target_keychain="$1"
    local existing_keychains=()

    while IFS= read -r keychain; do
        keychain="${keychain//\"/}"
        [[ -n "$keychain" ]] && existing_keychains+=("$keychain")
    done < <(security list-keychains -d user)

    for keychain in "${existing_keychains[@]}"; do
        if [[ "$keychain" == "$target_keychain" ]]; then
            return
        fi
    done

    security list-keychains -d user -s "$target_keychain" "${existing_keychains[@]}"
}

usage() {
    cat <<EOF
Usage: ./scripts/create-dev-signing-identity.sh [options]

Options:
  --force                  Replace any existing PhotonCast dev signing identity
  --identity NAME          Certificate common name (default: PhotonCast Local Dev)
  --keychain PATH          Keychain path (default: ~/Library/Keychains/photoncast-dev-signing.keychain-db)
  --env-file PATH          Output env file (default: ~/.config/photoncast/dev-signing.env)
  --help                   Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --force)
            FORCE=true
            shift
            ;;
        --identity)
            IDENTITY_NAME="$2"
            shift 2
            ;;
        --keychain)
            KEYCHAIN_PATH="$2"
            shift 2
            ;;
        --env-file)
            ENV_FILE="$2"
            shift 2
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

if ! command -v openssl >/dev/null 2>&1; then
    echo -e "${RED}Error: OpenSSL is required but not installed.${NC}" >&2
    exit 1
fi

mkdir -p "$SIGNING_DIR" "$(dirname "$ENV_FILE")" "$(dirname "$KEYCHAIN_PATH")"

if [[ "$FORCE" == true ]]; then
    rm -f "$CERT_PEM" "$KEY_PEM" "$P12_PATH" "$OPENSSL_CONFIG" "$ENV_FILE"
    security delete-keychain "$KEYCHAIN_PATH" >/dev/null 2>&1 || true
fi

if security find-identity -v -p codesigning "$KEYCHAIN_PATH" 2>/dev/null | grep -q "\"${IDENTITY_NAME}\""; then
    if [[ ! -f "$ENV_FILE" ]]; then
        echo -e "${RED}Existing signing identity found, but ${ENV_FILE} is missing.${NC}" >&2
        echo "Re-run with --force to recreate the keychain and env file." >&2
        exit 1
    fi
    ensure_keychain_in_search_list "$KEYCHAIN_PATH"
    echo -e "${GREEN}PhotonCast dev signing identity already exists.${NC}"
else
    KEYCHAIN_PASSWORD="$(openssl rand -hex 24)"

    cat > "$OPENSSL_CONFIG" <<EOF
[ req ]
default_bits = 2048
prompt = no
default_md = sha256
distinguished_name = dn
x509_extensions = v3_req

[ dn ]
CN = ${IDENTITY_NAME}
O = PhotonCast
OU = Development

[ v3_req ]
keyUsage = critical, digitalSignature
extendedKeyUsage = codeSigning
basicConstraints = critical, CA:FALSE
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer
EOF

    echo -e "${BLUE}Generating local code-signing certificate...${NC}"
    openssl req \
        -new \
        -newkey rsa:2048 \
        -x509 \
        -days 3650 \
        -nodes \
        -config "$OPENSSL_CONFIG" \
        -keyout "$KEY_PEM" \
        -out "$CERT_PEM" >/dev/null 2>&1
    chmod 600 "$KEY_PEM"

    openssl pkcs12 \
        -export \
        -legacy \
        -inkey "$KEY_PEM" \
        -in "$CERT_PEM" \
        -name "$IDENTITY_NAME" \
        -out "$P12_PATH" \
        -passout pass:"$KEYCHAIN_PASSWORD" >/dev/null 2>&1
    chmod 600 "$P12_PATH"

    security delete-keychain "$KEYCHAIN_PATH" >/dev/null 2>&1 || true
    security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
    security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
    ensure_keychain_in_search_list "$KEYCHAIN_PATH"
    security import "$P12_PATH" \
        -k "$KEYCHAIN_PATH" \
        -P "$KEYCHAIN_PASSWORD" \
        -T /usr/bin/codesign \
        -T /usr/bin/security >/dev/null
    security set-key-partition-list \
        -S apple-tool:,apple:,codesign: \
        -s \
        -k "$KEYCHAIN_PASSWORD" \
        "$KEYCHAIN_PATH" >/dev/null
    security add-trusted-cert \
        -r trustRoot \
        -p codeSign \
        -k "$KEYCHAIN_PATH" \
        "$CERT_PEM" >/dev/null

    cat > "$ENV_FILE" <<EOF
export PHOTONCAST_SIGNING_IDENTITY='${IDENTITY_NAME}'
export PHOTONCAST_SIGNING_KEYCHAIN='${KEYCHAIN_PATH}'
export PHOTONCAST_SIGNING_KEYCHAIN_PASSWORD='${KEYCHAIN_PASSWORD}'
EOF
    chmod 600 "$ENV_FILE"
fi

load_photoncast_signing_env
unlock_photoncast_signing_keychain

echo -e "${BLUE}Signing env file:${NC} ${ENV_FILE}"
echo -e "${BLUE}Signing keychain:${NC} ${PHOTONCAST_SIGNING_KEYCHAIN}"
echo -e "${BLUE}Signing identity:${NC} ${PHOTONCAST_SIGNING_IDENTITY}"
echo -e "${BLUE}Available identities:${NC}"
security find-identity -v -p codesigning "${PHOTONCAST_SIGNING_KEYCHAIN}"

echo ""
echo -e "${GREEN}Local dev signing is ready.${NC}"
echo "Next steps:"
echo "  1. ./scripts/release-build.sh"
echo "  2. ./scripts/install-app.sh"
echo ""
echo "The build scripts will automatically load ${ENV_FILE}."
