#!/usr/bin/env bash

set -euo pipefail

###############################################################################
# Configuration
###############################################################################

REPO="portalshq/px"
BINARY_NAME="px"
MCP_BINARY_NAME="px-mcp-server"
SKILLS_ARCHIVE="px-skills.zip"
VERSION="${VERSION:-latest}"
BASE_URL="${PX_INSTALL_BASE_URL:-}"

###############################################################################
# Utilities
###############################################################################

require() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Error: '$1' is required but not installed."
        exit 1
    }
}

require curl
require chmod
require uname
require mktemp
require unzip

###############################################################################
# Detect platform
###############################################################################

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64)
                TARGET="x86_64-unknown-linux-gnu"
                ;;
            aarch64|arm64)
                TARGET="aarch64-unknown-linux-gnu"
                ;;
            *)
                echo "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            x86_64)
                TARGET="x86_64-apple-darwin"
                ;;
            arm64)
                TARGET="aarch64-apple-darwin"
                ;;
            *)
                echo "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Unsupported operating system: $OS"
        exit 1
        ;;
esac

###############################################################################
# Install location
###############################################################################

INSTALL_DIR="${PX_INSTALL_DIR:-/usr/local/bin}"

if [[ -n "${PX_INSTALL_DIR:-}" ]]; then
    mkdir -p "$INSTALL_DIR"
elif [[ ! -w "$INSTALL_DIR" ]]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

###############################################################################
# Download
###############################################################################

ASSET="${BINARY_NAME}-${TARGET}"
MCP_ASSET="${MCP_BINARY_NAME}-${TARGET}"

if [[ -n "$BASE_URL" ]]; then
    URL="${BASE_URL%/}/${ASSET}"
    MCP_URL="${BASE_URL%/}/${MCP_ASSET}"
elif [[ "$VERSION" == "latest" ]]; then
    URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
    MCP_URL="https://github.com/${REPO}/releases/latest/download/${MCP_ASSET}"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
    MCP_URL="https://github.com/${REPO}/releases/download/${VERSION}/${MCP_ASSET}"
fi

echo "Installing ${BINARY_NAME}..."
echo "Platform : ${TARGET}"
echo "Version  : ${VERSION}"

TMP_DIR="$(mktemp -d)"
TMP_FILE="$TMP_DIR/$ASSET"
MCP_FILE="$TMP_DIR/$MCP_ASSET"

cleanup() {
    rm -rf "$TMP_DIR"
}

trap cleanup EXIT

curl \
    --fail \
    --location \
    --progress-bar \
    "$URL" \
    --output "$TMP_FILE"

chmod +x "$TMP_FILE"

echo
echo "Installing ${MCP_BINARY_NAME}..."

curl \
    --fail \
    --location \
    --progress-bar \
    "$MCP_URL" \
    --output "$MCP_FILE"

chmod +x "$MCP_FILE"

mv -f "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
mv -f "$MCP_FILE" "$INSTALL_DIR/$MCP_BINARY_NAME"

###############################################################################
# Install skills
###############################################################################

echo
echo "Installing agent skills..."

# Determine skills directory
if [[ "$OS" == "Darwin" || "$OS" == "Linux" ]]; then
    SKILLS_DIR="$HOME/.agents/skills"
else
    # Windows
    SKILLS_DIR="$USERPROFILE/.agents/skills"
fi

# Create skills directory
mkdir -p "$SKILLS_DIR"

# Download skills archive
if [[ -n "$BASE_URL" ]]; then
    SKILLS_URL="${BASE_URL%/}/${SKILLS_ARCHIVE}"
elif [[ "$VERSION" == "latest" ]]; then
    SKILLS_URL="https://github.com/${REPO}/releases/latest/download/${SKILLS_ARCHIVE}"
else
    SKILLS_URL="https://github.com/${REPO}/releases/download/${VERSION}/${SKILLS_ARCHIVE}"
fi

SKILLS_TMP="$TMP_DIR/$SKILLS_ARCHIVE"

if curl \
    --fail \
    --location \
    --progress-bar \
    "$SKILLS_URL" \
    --output "$SKILLS_TMP"; then
    # Extract skills to the skills directory
    if command -v unzip >/dev/null 2>&1; then
        unzip -q "$SKILLS_TMP" -d "$SKILLS_DIR"
        echo "Skills installed to $SKILLS_DIR"
    else
        echo "Error: unzip is required to extract skills"
        exit 1
    fi
else
    echo "Warning: Failed to download skills archive from $SKILLS_URL"
    echo "Skills will need to be installed manually."
    echo "Visit https://github.com/portalshq/px for instructions."
fi

###############################################################################
# PATH hint
###############################################################################

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    cat <<EOF

${INSTALL_DIR} is not on your PATH.

Add this to your shell profile:

export PATH="${INSTALL_DIR}:\$PATH"

EOF
fi

###############################################################################
# Verify
###############################################################################

echo
echo "Installed successfully."

"$INSTALL_DIR/$BINARY_NAME" --version
"$INSTALL_DIR/$MCP_BINARY_NAME" --version
