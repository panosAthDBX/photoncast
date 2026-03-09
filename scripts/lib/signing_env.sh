#!/bin/bash

load_photoncast_signing_env() {
    local default_env_file="${HOME}/.config/photoncast/dev-signing.env"
    local env_file="${PHOTONCAST_SIGNING_ENV_FILE:-$default_env_file}"

    export PHOTONCAST_SIGNING_ENV_FILE="$env_file"

    if [[ -f "$env_file" ]]; then
        # shellcheck disable=SC1090
        source "$env_file"
    fi
}

unlock_photoncast_signing_keychain() {
    if [[ -z "${PHOTONCAST_SIGNING_KEYCHAIN:-}" ]]; then
        return 0
    fi

    if [[ ! -f "${PHOTONCAST_SIGNING_KEYCHAIN}" ]]; then
        return 0
    fi

    if [[ -z "${PHOTONCAST_SIGNING_KEYCHAIN_PASSWORD:-}" ]]; then
        return 0
    fi

    security unlock-keychain \
        -p "${PHOTONCAST_SIGNING_KEYCHAIN_PASSWORD}" \
        "${PHOTONCAST_SIGNING_KEYCHAIN}" >/dev/null 2>&1 || true
}
