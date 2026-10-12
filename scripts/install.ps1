#!/usr/bin/env pwsh

param(
    [string]$Version = "latest",
    [string]$InstallDir,
    [string]$BaseUrl
)

$ErrorActionPreference = "Stop"

$REPO = "portalshq/px"
$BINARY_NAME = "px"
$MCP_BINARY_NAME = "px-mcp-server"
$SKILLS_ARCHIVE = "px-skills.tar.gz"

###############################################################################
# Detect platform
###############################################################################

$OS = $PSVersionTable.Platform
$ARCH = if ($env:PROCESSOR_ARCHITECTURE -eq "AMD64") { "x86_64" } elseif ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "aarch64" } else { $env:PROCESSOR_ARCHITECTURE }

if ($OS -eq "Win32NT") {
    $TARGET = "x86_64-pc-windows-msvc"
    $BINARY_NAME = "px.exe"
    $MCP_BINARY_NAME = "px-mcp-server.exe"
} else {
    Write-Error "Unsupported operating system: $OS"
    exit 1
}

###############################################################################
# Install location
###############################################################################

if ([string]::IsNullOrEmpty($InstallDir)) {
    $InstallDir = Join-Path $env:USERPROFILE "bin"
}

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

###############################################################################
# Download
###############################################################################

$ASSET = "$BINARY_NAME-$TARGET"
$MCP_ASSET = "$MCP_BINARY_NAME-$TARGET"

if ([string]::IsNullOrEmpty($BaseUrl)) {
    if ($Version -eq "latest") {
        $URL = "https://github.com/${REPO}/releases/latest/download/${ASSET}"
        $MCP_URL = "https://github.com/${REPO}/releases/latest/download/${MCP_ASSET}"
        $SKILLS_URL = "https://github.com/${REPO}/releases/latest/download/${SKILLS_ARCHIVE}"
    } else {
        $URL = "https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
        $MCP_URL = "https://github.com/${REPO}/releases/download/${VERSION}/${MCP_ASSET}"
        $SKILLS_URL = "https://github.com/${REPO}/releases/download/${VERSION}/${SKILLS_ARCHIVE}"
    }
} else {
    $BaseUrl = $BaseUrl.TrimEnd('/')
    $URL = "${BaseUrl}/${ASSET}"
    $MCP_URL = "${BaseUrl}/${MCP_ASSET}"
    $SKILLS_URL = "${BaseUrl}/${SKILLS_ARCHIVE}"
}

Write-Host "Installing ${BINARY_NAME}..."
Write-Host "Platform : ${TARGET}"
Write-Host "Version  : ${VERSION}"

$TMP_DIR = Join-Path $env:TEMP "px-install-$([Guid]::NewGuid())"
New-Item -ItemType Directory -Path $TMP_DIR -Force | Out-Null

$TMP_FILE = Join-Path $TMP_DIR $ASSET
$MCP_FILE = Join-Path $TMP_DIR $MCP_ASSET
$SKILLS_TMP = Join-Path $TMP_DIR $SKILLS_ARCHIVE

try {
    # Download px binary
    Invoke-WebRequest -Uri $URL -OutFile $TMP_FILE -UseBasicParsing

    # Download px-mcp-server binary
    Write-Host
    Write-Host "Installing ${MCP_BINARY_NAME}..."
    Invoke-WebRequest -Uri $MCP_URL -OutFile $MCP_FILE -UseBasicParsing

    # Move binaries to install directory
    Copy-Item -Path $TMP_FILE -Destination (Join-Path $InstallDir $BINARY_NAME) -Force
    Copy-Item -Path $MCP_FILE -Destination (Join-Path $InstallDir $MCP_BINARY_NAME) -Force

   ###############################################################################
    # Install skills
    ###############################################################################

    Write-Host
    Write-Host "Installing agent skills..."

    $SKILLS_DIR = Join-Path $env:USERPROFILE ".agents\skills"
    if (-not (Test-Path $SKILLS_DIR)) {
        New-Item -ItemType Directory -Path $SKILLS_DIR -Force | Out-Null
    }

    # Download skills archive
    try {
        Invoke-WebRequest -Uri $SKILLS_URL -OutFile $SKILLS_TMP -UseBasicParsing

        # Extract skills
        if (Get-Command tar -ErrorAction SilentlyContinue) {
            tar -xzf $SKILLS_TMP -C $SKILLS_DIR
            Write-Host "Skills installed to $SKILLS_DIR"
        } else {
            Write-Error "Error: tar is required to extract skills. Please install tar or use a Unix-like shell."
            exit 1
        }
    } catch {
        Write-Warning "Failed to download skills archive from $SKILLS_URL"
        Write-Warning "Skills will need to be installed manually."
        Write-Warning "Visit https://github.com/portalshq/px for instructions."
    }

   ###############################################################################
    # PATH hint
    ###############################################################################

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Host
        Write-Host "$InstallDir is not on your PATH."
        Write-Host
        Write-Host "Add it manually:"
        Write-Host "  $env:USERPROFILE\bin"
    }

    ###############################################################################
    # Verify
    ###############################################################################

    Write-Host
    Write-Host "Installed successfully."

    & (Join-Path $InstallDir $BINARY_NAME) --version
    & (Join-Path $InstallDir $MCP_BINARY_NAME) --version
} finally {
    # Cleanup
    Remove-Item -Path $TMP_DIR -Recurse -Force -ErrorAction SilentlyContinue
}
