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
$SKILLS_ARCHIVE = "px-skills.zip"

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
    $SKILLS_INSTALLED = $false

    try {
        Invoke-WebRequest -Uri $SKILLS_URL -OutFile $SKILLS_TMP -UseBasicParsing

        # Extract skills using native PowerShell cmdlet
        Expand-Archive -Path $SKILLS_TMP -DestinationPath $SKILLS_DIR -Force
        Write-Host "Skills installed to $SKILLS_DIR"
        $SKILLS_INSTALLED = $true
    } catch {
        Write-Warning "Failed to download skills archive from $SKILLS_URL"
        Write-Host "Falling back to GitHub repo..."

        # Fallback: download skills directly from GitHub repo
        $GITHUB_SKILLS_URL = "https://github.com/${REPO}/archive/refs/heads/main.zip"
        $REPO_TMP = Join-Path $TMP_DIR "repo.zip"

        try {
            Invoke-WebRequest -Uri $GITHUB_SKILLS_URL -OutFile $REPO_TMP -UseBasicParsing

            # Extract repo to temp directory
            $REPO_EXTRACT_DIR = Join-Path $TMP_DIR "repo"
            Expand-Archive -Path $REPO_TMP -DestinationPath $REPO_EXTRACT_DIR -Force

            $REPO_EXTRACTED = Join-Path $REPO_EXTRACT_DIR "px-main"

            if (Test-Path "$REPO_EXTRACTED/skills") {
                # Copy skills to target directory
                Copy-Item -Path "$REPO_EXTRACTED/skills/*" -Destination $SKILLS_DIR -Recurse -Force
                Write-Host "Skills installed from GitHub repo to $SKILLS_DIR"
                $SKILLS_INSTALLED = $true
            } else {
                Write-Error "Error: skills directory not found in downloaded repo"
            }
        } catch {
            Write-Error "Error: Failed to download skills from GitHub repo"
        }
    }

    if (-not $SKILLS_INSTALLED) {
        Write-Error "Error: Failed to install skills. PX will not work correctly for agents."
        Write-Error "Please report this issue at https://github.com/portalshq/px/issues"
        exit 1
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
