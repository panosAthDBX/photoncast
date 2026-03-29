#!/bin/bash
set -e

# Ensure Rust toolchain is available
if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo not found. Install Rust toolchain first."
    exit 1
fi

# Verify workspace compiles
echo "Verifying workspace compilation..."
cargo check --workspace 2>&1 | tail -5
echo "Workspace check complete."
