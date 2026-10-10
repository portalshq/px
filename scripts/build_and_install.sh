#!/usr/bin/env bash
set -euo pipefail

LOCAL_INSTALL_DIR="$HOME/.local/bin"

# Build and install the PX CLI.
echo "Building portalshq-px-cli..."
cargo build --release -p portalshq-px-cli

echo "Copying to ~/.local/bin..."
mkdir -p "$LOCAL_INSTALL_DIR"
cp target/release/px "$LOCAL_INSTALL_DIR/px"
chmod +x "$LOCAL_INSTALL_DIR/px"

echo "Done."
