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
    # Only cleanup temp directory, not installed binaries
    rm -rf "$TMP_DIR"
}

trap cleanup EXIT

curl \
    --fail \
    --location \
    --progress-bar \
    "$URL" \
    --output "$TMP_FILE" || {
    echo "Error: Failed to download px binary"
    exit 1
}

chmod +x "$TMP_FILE"

echo
echo "Installing ${MCP_BINARY_NAME}..."

curl \
    --fail \
    --location \
    --progress-bar \
    "$MCP_URL" \
    --output "$MCP_FILE" || {
    echo "Error: Failed to download px-mcp-server binary"
    exit 1
}

chmod +x "$MCP_FILE"

mv -f "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME" || {
    echo "Error: Failed to install px binary"
    exit 1
}

mv -f "$MCP_FILE" "$INSTALL_DIR/$MCP_BINARY_NAME" || {
    echo "Error: Failed to install px-mcp-server binary"
    exit 1
}

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

# Define px-specific skills
PX_SKILLS=(
    "ask-px"
    "character-reference-sheet"
    "px-repo"
    "px-resolve"
    "px-update"
)

# Clean only px-specific skills to avoid stale/previous versions
# Preserve any custom skills the user may have installed
if [[ -d "$SKILLS_DIR" ]]; then
    echo "Cleaning existing px skills..."
    for skill in "${PX_SKILLS[@]}"; do
        if [[ -d "$SKILLS_DIR/$skill" ]]; then
            rm -rf "$SKILLS_DIR/$skill"
        fi
    done
fi

# Create skills directory
if ! mkdir -p "$SKILLS_DIR"; then
    echo "Error: Failed to create skills directory"
    exit 1
fi

# Download skills archive
if [[ -n "$BASE_URL" ]]; then
    SKILLS_URL="${BASE_URL%/}/${SKILLS_ARCHIVE}"
elif [[ "$VERSION" == "latest" ]]; then
    SKILLS_URL="https://github.com/${REPO}/releases/latest/download/${SKILLS_ARCHIVE}"
else
    SKILLS_URL="https://github.com/${REPO}/releases/download/${VERSION}/${SKILLS_ARCHIVE}"
fi

SKILLS_TMP="$TMP_DIR/$SKILLS_ARCHIVE"

SKILLS_INSTALLED=false

# Try release archive first
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
        SKILLS_INSTALLED=true
    else
        echo "Error: unzip is required to extract skills"
        exit 1
    fi
else
    echo "Warning: Failed to download skills archive from $SKILLS_URL"
    echo "Falling back to GitHub raw content..."

    # Fallback: download skills directly from GitHub raw content
    # Use the appropriate git ref: version tag if specified, otherwise resolve latest tag
    if [[ "$VERSION" == "latest" ]]; then
        # Resolve the actual latest release tag to avoid version mismatch
        LATEST_TAG=$(curl --silent --location "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1')
        if [[ -n "$LATEST_TAG" ]]; then
            GIT_REF="$LATEST_TAG"
            echo "Resolved latest tag: $GIT_REF"
        else
            echo "Warning: Could not resolve latest tag, falling back to main"
            GIT_REF="main"
        fi
    else
        # Add 'v' prefix if not present for git tags
        if [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            GIT_REF="v${VERSION}"
        else
            GIT_REF="$VERSION"
        fi
    fi

    GITHUB_RAW_BASE="https://raw.githubusercontent.com/${REPO}/${GIT_REF}/skills"

    # Build skill file paths from PX_SKILLS array
    SKILLS_LIST=()
    for skill in "${PX_SKILLS[@]}"; do
        SKILLS_LIST+=("${skill}/SKILL.md")
    done

    ALL_SKILLS_DOWNLOADED=true

    for skill_path in "${SKILLS_LIST[@]}"; do
        skill_dir="$SKILLS_DIR/$(dirname "$skill_path")"
        skill_file="$SKILLS_DIR/$skill_path"

        mkdir -p "$skill_dir"

        if curl \
            --fail \
            --location \
            --silent \
            --show-error \
            "${GITHUB_RAW_BASE}/${skill_path}" \
            --output "$skill_file"; then
            echo "Downloaded: $skill_path"
        else
            echo "Failed to download: $skill_path"
            ALL_SKILLS_DOWNLOADED=false
        fi
    done

    if [[ "$ALL_SKILLS_DOWNLOADED" == "true" ]]; then
        echo "Skills installed from GitHub raw content to $SKILLS_DIR"
        SKILLS_INSTALLED=true
    else
        echo "Error: Failed to download some skills from GitHub"
    fi
fi

if [[ "$SKILLS_INSTALLED" == "false" ]]; then
    echo "Error: Failed to install skills. PX will not work correctly for agents."
    echo "Please report this issue at https://github.com/portalshq/px/issues"
    exit 1
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
