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
INSTALL_DIR="$(cd "$INSTALL_DIR" && pwd -P)"

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

PX_HOME="$HOME/.px"
mkdir -p "$PX_HOME"
export PX_DIR="$PX_HOME"
TXN_DIR="$PX_HOME/installer-transaction"
COMPLETED_DIR="$TXN_DIR.completed"
LOCK_DIR="$PX_HOME/installer.lock"
OPTIMIZER_ARCHIVE="px-optimizer-$TARGET.tar.gz"
VERIFIER="px-verify-optimizer-release-$TARGET"

acquire_installer_lock() {
    if mkdir "$LOCK_DIR" 2>/dev/null; then printf '%s\n' "$$" > "$LOCK_DIR/pid"; return; fi
    local owner command_line
    owner="$(cat "$LOCK_DIR/pid" 2>/dev/null || true)"
    if [[ "$owner" =~ ^[0-9]+$ ]] && kill -0 "$owner" 2>/dev/null; then
        command_line="$(ps -p "$owner" -o command= 2>/dev/null || true)"
        if [[ "$command_line" == *"scripts/install.sh"* ]]; then
            echo 'Another PX installation is already running.' >&2
            return 1
        fi
    fi
    rm -rf "$LOCK_DIR"
    mkdir "$LOCK_DIR" || { echo 'Could not acquire the PX installer lock.' >&2; return 1; }
    printf '%s\n' "$$" > "$LOCK_DIR/pid"
}

release_installer_lock() {
    if [[ "$(cat "$LOCK_DIR/pid" 2>/dev/null || true)" == "$$" ]]; then rm -rf "$LOCK_DIR"; fi
}

acquire_installer_lock

verify_payload() {
    local dir="$1" release_version="$2" identity escaped_version file expected actual
    identity='^https://github\.com/portalshq/px/\.github/workflows/cli-release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$'
    if [[ "$release_version" != latest ]]; then
        [[ "$release_version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Error: invalid release version in installer state' >&2; return 1; }
        escaped_version="$(printf '%s' "$release_version" | sed 's/\./\\./g')"
        identity="^https://github\\.com/portalshq/px/\\.github/workflows/cli-release\\.yml@refs/tags/$escaped_version$"
    fi
    cosign verify-blob --bundle "$dir/SHA256SUMS.sigstore.json" \
        --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
        --certificate-identity-regexp "$identity" "$dir/SHA256SUMS" >/dev/null
    for file in "$ASSET" "$MCP_ASSET" "$SKILLS_ARCHIVE" "$OPTIMIZER_ARCHIVE" "$VERIFIER" optimizer-release.json manifest.json manifest.sig; do
        expected="$(awk -v name="./$file" '$2 == name {print $1; n++} END {if (n != 1) exit 1}' "$dir/SHA256SUMS")" || {
            echo "Error: missing or duplicate checksum for $file" >&2; return 1;
        }
        [[ "$expected" =~ ^[a-f0-9]{64}$ ]] || { echo "Error: invalid checksum for $file" >&2; return 1; }
        actual="$(shasum -a 256 "$dir/$file" | awk '{print $1}')"
        [[ "$actual" == "$expected" ]] || { echo "Error: checksum mismatch for $file" >&2; return 1; }
    done
    chmod +x "$dir/$VERIFIER"
    "$dir/$VERIFIER" "$dir/optimizer-release.json" "$dir/manifest.json" "$dir/manifest.sig" "$TARGET" "$dir/$OPTIMIZER_ARCHIVE"
}

atomic_install() {
    local source="$1" destination="$2" temporary
    mkdir -p "$(dirname "$destination")"
    temporary="$(mktemp "$destination.px-new.XXXXXX")"
    cp -p "$source" "$temporary"
    chmod +x "$temporary"
    mv -f "$temporary" "$destination"
}

apply_transaction() {
    local dir="$1" target_install_dir="$2" enable_fresh="$3" candidate_version installed_version
    local skills_dir="$HOME/.agents/skills"
    export PX_INSTALL_TRANSACTION=1
    export PX_DIR="$PX_HOME"
    atomic_install "$dir/$ASSET" "$target_install_dir/$BINARY_NAME"
    atomic_install "$dir/$MCP_ASSET" "$target_install_dir/$MCP_BINARY_NAME"
    echo 'Installing agent skills...'
    mkdir -p "$skills_dir"
    for skill in ask-px character-reference-sheet px-repo px-resolve px-update; do rm -rf "$skills_dir/$skill"; done
    unzip -q -o "$dir/$SKILLS_ARCHIVE" -d "$skills_dir"
    "$target_install_dir/$BINARY_NAME" --version
    "$target_install_dir/$MCP_BINARY_NAME" --version
    candidate_version="$(awk '$1 == "version:" {print $2; exit}' "$dir/optimizer/plugin.yaml" | tr -d '"\047')"
    [[ "$candidate_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid optimizer package version' >&2; return 1; }
    if installed_info="$("$target_install_dir/$BINARY_NAME" plugin info px.optimizer 2>/dev/null)"; then
        installed_version="$(printf '%s\n' "$installed_info" | sed -n 's/^[[:space:]]*"version": "\([^"]*\)".*/\1/p')"
        [[ -n "$installed_version" ]] || { echo 'Cannot determine installed optimizer version' >&2; return 1; }
        if [[ "$installed_version" != "$candidate_version" ]]; then
            "$target_install_dir/$BINARY_NAME" plugin upgrade "$dir/optimizer"
        fi
    else
        "$target_install_dir/$BINARY_NAME" plugin install "$dir/optimizer"
    fi
    # New installs become enabled, including a retry after interruption.
    # Plugin upgrades keep the previous enabled/disabled setting.
    if [[ "$enable_fresh" == true ]]; then "$target_install_dir/$BINARY_NAME" plugin enable px.optimizer; fi
    "$target_install_dir/$BINARY_NAME" plugin info px.optimizer >/dev/null
}

finish_transaction() {
    local dir="$1" completed="$1.completed"
    [[ ! -e "$completed" ]] || rm -rf "$completed"
    sync
    mv "$dir" "$completed"
    sync
    rm -rf "$completed"
}

resume_transaction() {
    # A completed directory is only garbage left by a force-kill during
    # cleanup. Its atomic rename is the commit point.
    [[ ! -e "$COMPLETED_DIR" ]] || rm -rf "$COMPLETED_DIR"
    [[ -d "$TXN_DIR" ]] || return 0
    [[ -f "$TXN_DIR/STATE" && -f "$TXN_DIR/install-dir" && -f "$TXN_DIR/enable-fresh" && -f "$TXN_DIR/release-version" ]] || {
        echo "Error: incomplete installer transaction at $TXN_DIR; keep it for recovery and rerun the installer." >&2; return 1;
    }
    local target_install_dir release_version enable_fresh
    IFS= read -r target_install_dir < "$TXN_DIR/install-dir"
    IFS= read -r release_version < "$TXN_DIR/release-version"
    IFS= read -r enable_fresh < "$TXN_DIR/enable-fresh"
    [[ -n "$target_install_dir" && "$target_install_dir" == "$INSTALL_DIR" && ( "$enable_fresh" == true || "$enable_fresh" == false ) ]] || {
        echo "Error: resume with the same install directory ($INSTALL_DIR) used by the interrupted installation." >&2; return 1;
    }
    echo "Resuming verified PX installation from $TXN_DIR"
    verify_payload "$TXN_DIR" "$release_version"
    rm -rf "$TXN_DIR/optimizer"
    mkdir "$TXN_DIR/optimizer"
    tar -xzf "$TXN_DIR/$OPTIMIZER_ARCHIVE" -C "$TXN_DIR/optimizer"
    apply_transaction "$TXN_DIR" "$target_install_dir" "$enable_fresh"
    finish_transaction "$TXN_DIR"
}

resume_transaction
TMP_DIR="$(mktemp -d "$PX_HOME/.installer-staging.XXXXXX")"
cleanup() {
    if [[ -n "$TMP_DIR" && -d "$TMP_DIR" && "$TMP_DIR" != "$TXN_DIR" ]]; then rm -rf "$TMP_DIR"; fi
    release_installer_lock
}
trap cleanup EXIT
TMP_FILE="$TMP_DIR/$ASSET"
MCP_FILE="$TMP_DIR/$MCP_ASSET"
curl --fail --location --progress-bar "$URL" --output "$TMP_FILE" || { echo 'Error: Failed to download px binary'; exit 1; }
chmod +x "$TMP_FILE"
echo "Downloading $MCP_BINARY_NAME..."
curl --fail --location --progress-bar "$MCP_URL" --output "$MCP_FILE" || { echo 'Error: Failed to download px-mcp-server binary'; exit 1; }
chmod +x "$MCP_FILE"
RELEASE_BASE="$(dirname "$URL")"
for file in SHA256SUMS SHA256SUMS.sigstore.json "$SKILLS_ARCHIVE" "$OPTIMIZER_ARCHIVE" "$VERIFIER" optimizer-release.json manifest.json manifest.sig; do
    curl --fail --location --silent --show-error "$RELEASE_BASE/$file" --output "$TMP_DIR/$file"
done
verify_payload "$TMP_DIR" "$VERSION"
mkdir "$TMP_DIR/optimizer"
tar -xzf "$TMP_DIR/$OPTIMIZER_ARCHIVE" -C "$TMP_DIR/optimizer"

enable_fresh=true
if "$INSTALL_DIR/$BINARY_NAME" plugin info px.optimizer >/dev/null 2>&1; then enable_fresh=false; fi
printf '%s\n' "$INSTALL_DIR" > "$TMP_DIR/install-dir"
printf '%s\n' "$enable_fresh" > "$TMP_DIR/enable-fresh"
printf '%s\n' "$VERSION" > "$TMP_DIR/release-version"
printf 'px-installer-transaction-v1\n' > "$TMP_DIR/STATE"
sync
mv "$TMP_DIR" "$TXN_DIR"
sync
TMP_DIR="$TXN_DIR"
apply_transaction "$TXN_DIR" "$INSTALL_DIR" "$enable_fresh"
finish_transaction "$TXN_DIR"
TMP_DIR=""

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
