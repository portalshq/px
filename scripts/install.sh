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
require cosign
require shasum

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
if [[ "${PX_TELEMETRY:-1}" != "0" ]]; then
    echo "Anonymous install counting is enabled. Set PX_TELEMETRY=0 to disable it."
fi

TMP_DIR="$(mktemp -d)"
TMP_FILE="$TMP_DIR/$ASSET"
MCP_FILE="$TMP_DIR/$MCP_ASSET"
INSTALL_STARTED=false
INSTALL_COMMITTED=false

cleanup() {
    local restore_failed=false
    if [[ "$INSTALL_STARTED" == true && "$INSTALL_COMMITTED" == false ]]; then
        echo 'Installation failed; restoring previous PX files.' >&2
        for name in "$BINARY_NAME" "$MCP_BINARY_NAME"; do
            if [[ -f "$TMP_DIR/backup/bin/$name" ]]; then
                cp -p "$TMP_DIR/backup/bin/$name" "$INSTALL_DIR/$name" || restore_failed=true
            else
                rm -f "$INSTALL_DIR/$name" || restore_failed=true
            fi
        done
        for skill in ask-px character-reference-sheet px-repo px-resolve px-update; do
            rm -rf "$HOME/.agents/skills/$skill" || restore_failed=true
            if [[ -d "$TMP_DIR/backup/skills/$skill" ]]; then
                mkdir -p "$HOME/.agents/skills"
                cp -R "$TMP_DIR/backup/skills/$skill" "$HOME/.agents/skills/$skill" || restore_failed=true
            fi
        done
    fi
    if [[ "$restore_failed" == true ]]; then
        echo "Could not fully restore the previous installation; backups remain in $TMP_DIR/backup" >&2
        return
    fi
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

# A release asset must not be executed or installed until the release
# workflow's signature and the checksum for every downloaded asset pass.
RELEASE_BASE="${URL%/*}"
SKILLS_TMP="$TMP_DIR/$SKILLS_ARCHIVE"
OPTIMIZER_ARCHIVE="px-optimizer-${TARGET}.tar.gz"
VERIFIER="px-verify-optimizer-release-${TARGET}"
for file in SHA256SUMS SHA256SUMS.sigstore.json "$SKILLS_ARCHIVE" "$OPTIMIZER_ARCHIVE" "$VERIFIER" optimizer-release.json manifest.json manifest.sig; do
    curl --fail --location --silent --show-error "$RELEASE_BASE/$file" --output "$TMP_DIR/$file"
done
identity='^https://github\.com/portalshq/px/\.github/workflows/cli-release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$'
if [[ "$VERSION" != latest ]]; then
    [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Error: VERSION must be a release tag such as v0.10.0' >&2; exit 1; }
    escaped_version="${VERSION//./\\.}"
    identity="^https://github\\.com/portalshq/px/\\.github/workflows/cli-release\\.yml@refs/tags/${escaped_version}$"
fi
cosign verify-blob --bundle "$TMP_DIR/SHA256SUMS.sigstore.json" \
    --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
    --certificate-identity-regexp "$identity" \
    "$TMP_DIR/SHA256SUMS" >/dev/null
for file in "$ASSET" "$MCP_ASSET" "$SKILLS_ARCHIVE" "$OPTIMIZER_ARCHIVE" "$VERIFIER" optimizer-release.json manifest.json manifest.sig; do
    expected="$(awk -v name="./$file" '$2 == name {print $1; n++} END {if (n != 1) exit 1}' "$TMP_DIR/SHA256SUMS")" || {
        echo "Error: missing or duplicate checksum for $file" >&2; exit 1;
    }
    [[ "$expected" =~ ^[a-f0-9]{64}$ ]] || { echo "Error: invalid checksum for $file" >&2; exit 1; }
    actual="$(shasum -a 256 "$TMP_DIR/$file" | awk '{print $1}')"
    [[ "$actual" == "$expected" ]] || { echo "Error: checksum mismatch for $file" >&2; exit 1; }
done
chmod +x "$TMP_DIR/$VERIFIER"
"$TMP_DIR/$VERIFIER" "$TMP_DIR/optimizer-release.json" "$TMP_DIR/manifest.json" "$TMP_DIR/manifest.sig" "$TARGET" "$TMP_DIR/$OPTIMIZER_ARCHIVE"
mkdir "$TMP_DIR/optimizer"
tar -xzf "$TMP_DIR/$OPTIMIZER_ARCHIVE" -C "$TMP_DIR/optimizer"

mkdir -p "$TMP_DIR/backup/bin" "$TMP_DIR/backup/skills"
for name in "$BINARY_NAME" "$MCP_BINARY_NAME"; do
    if [[ -f "$INSTALL_DIR/$name" ]]; then cp -p "$INSTALL_DIR/$name" "$TMP_DIR/backup/bin/$name"; fi
done
for skill in ask-px character-reference-sheet px-repo px-resolve px-update; do
    if [[ -d "$HOME/.agents/skills/$skill" ]]; then cp -R "$HOME/.agents/skills/$skill" "$TMP_DIR/backup/skills/$skill"; fi
done
INSTALL_STARTED=true

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

# The archive was verified together with the binaries above.
unzip -q "$SKILLS_TMP" -d "$SKILLS_DIR"
echo "Skills installed to $SKILLS_DIR"

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
"$INSTALL_DIR/$BINARY_NAME" --version
"$INSTALL_DIR/$MCP_BINARY_NAME" --version

candidate_version="$(awk '$1 == "version:" {print $2; exit}' "$TMP_DIR/optimizer/plugin.yaml" | tr -d '\"\047')"
[[ "$candidate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid optimizer package version' >&2; exit 1; }
if installed_info="$("$INSTALL_DIR/$BINARY_NAME" plugin info px.optimizer 2>/dev/null)"; then
    installed_version="$(printf '%s\n' "$installed_info" | sed -n 's/^[[:space:]]*"version": "\([^"]*\)".*/\1/p')"
    [[ -n "$installed_version" ]] || { echo 'Cannot determine installed optimizer version' >&2; exit 1; }
    if [[ "$installed_version" != "$candidate_version" ]]; then
        "$INSTALL_DIR/$BINARY_NAME" plugin upgrade "$TMP_DIR/optimizer"
    fi
else
    "$INSTALL_DIR/$BINARY_NAME" plugin install "$TMP_DIR/optimizer"
    if ! "$INSTALL_DIR/$BINARY_NAME" plugin enable px.optimizer; then
        "$INSTALL_DIR/$BINARY_NAME" plugin uninstall px.optimizer || true
        exit 1
    fi
fi
INSTALL_COMMITTED=true
echo "Installed successfully."

###############################################################################
# Anonymous successful-install count
###############################################################################

if [[ "${PX_TELEMETRY:-1}" != "0" ]]; then
    CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/px"
    INSTALL_ID_FILE="$CONFIG_DIR/install-id"
    if mkdir -p "$CONFIG_DIR" 2>/dev/null; then
        chmod 700 "$CONFIG_DIR" 2>/dev/null || true
        INSTALL_ID="$(cat "$INSTALL_ID_FILE" 2>/dev/null || true)"
        if [[ ! "$INSTALL_ID" =~ ^[0-9a-f]{32}$ ]]; then
            INSTALL_ID="$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
            if [[ "$INSTALL_ID" =~ ^[0-9a-f]{32}$ ]]; then
                (umask 077; printf '%s' "$INSTALL_ID" > "$INSTALL_ID_FILE") 2>/dev/null || true
            fi
        fi
        if [[ "$INSTALL_ID" =~ ^[0-9a-f]{32}$ ]]; then
            INSTALL_VERSION="$("$INSTALL_DIR/$BINARY_NAME" --version 2>/dev/null | head -n 1 | tr -cd '[:alnum:].+-' | cut -c1-80)"
            DOWNLOAD_ID="${PX_DOWNLOAD_ID:-}"
            [[ "$DOWNLOAD_ID" =~ ^[0-9a-f]{32}$ ]] || DOWNLOAD_ID=""
            curl --silent --show-error --fail --max-time 5 \
                --request POST \
                --header 'Content-Type: application/json' \
                --data "{\"install_id\":\"$INSTALL_ID\",\"download_id\":\"$DOWNLOAD_ID\",\"version\":\"$INSTALL_VERSION\",\"platform\":\"$TARGET\"}" \
                https://portals.works/api/px/install >/dev/null 2>&1 || true
        fi
    fi
fi
