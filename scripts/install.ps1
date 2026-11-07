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
        $URL = "https://github.com/${REPO}/releases/download/${Version}/${ASSET}"
        $MCP_URL = "https://github.com/${REPO}/releases/download/${Version}/${MCP_ASSET}"
        $SKILLS_URL = "https://github.com/${REPO}/releases/download/${Version}/${SKILLS_ARCHIVE}"
    }
} else {
    $BaseUrl = $BaseUrl.TrimEnd('/')
    $URL = "${BaseUrl}/${ASSET}"
    $MCP_URL = "${BaseUrl}/${MCP_ASSET}"
    $SKILLS_URL = "${BaseUrl}/${SKILLS_ARCHIVE}"
}

Write-Host "Installing ${BINARY_NAME}..."
Write-Host "Platform : ${TARGET}"
Write-Host "Version  : ${Version}"
if ($env:PX_TELEMETRY -ne "0") {
    Write-Host "Anonymous install counting is enabled. Set PX_TELEMETRY=0 to disable it."
}

$TMP_DIR = Join-Path $env:TEMP "px-install-$([Guid]::NewGuid())"
New-Item -ItemType Directory -Path $TMP_DIR -Force | Out-Null

$TMP_FILE = Join-Path $TMP_DIR $ASSET
$MCP_FILE = Join-Path $TMP_DIR $MCP_ASSET
$SKILLS_TMP = Join-Path $TMP_DIR $SKILLS_ARCHIVE

try {
    # Download px binary
    Invoke-WebRequest -Uri $URL -OutFile $TMP_FILE -UseBasicParsing
} catch {
    Write-Error "Error: Failed to download px binary"
    exit 1
}

try {
    # Download px-mcp-server binary
    Write-Host
    Write-Host "Installing ${MCP_BINARY_NAME}..."
    Invoke-WebRequest -Uri $MCP_URL -OutFile $MCP_FILE -UseBasicParsing
} catch {
    Write-Error "Error: Failed to download px-mcp-server binary"
    exit 1
}

try {
    # Move binaries to install directory
    Copy-Item -Path $TMP_FILE -Destination (Join-Path $InstallDir $BINARY_NAME) -Force
    Copy-Item -Path $MCP_FILE -Destination (Join-Path $InstallDir $MCP_BINARY_NAME) -Force
} catch {
    Write-Error "Error: Failed to install binaries"
    exit 1
}

   ###############################################################################
    # Install skills
    ###############################################################################

    Write-Host
    Write-Host "Installing agent skills..."

    $SKILLS_DIR = Join-Path $env:USERPROFILE ".agents\skills"

    # Define px-specific skills
    $PX_SKILLS = @(
        "ask-px"
        "character-reference-sheet"
        "px-repo"
        "px-resolve"
        "px-update"
    )

    # Clean only px-specific skills to avoid stale/previous versions
    # Preserve any custom skills the user may have installed
    if (Test-Path $SKILLS_DIR) {
        Write-Host "Cleaning existing px skills..."
        foreach ($skill in $PX_SKILLS) {
            $skillPath = Join-Path $SKILLS_DIR $skill
            if (Test-Path $skillPath) {
                Remove-Item -Path $skillPath -Recurse -Force
            }
        }
    }

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
        Write-Host "Falling back to GitHub raw content..."

        # Fallback: download skills directly from GitHub raw content
        # Use the appropriate git ref: version tag if specified, otherwise resolve latest tag
        if ($Version -eq "latest") {
            # Resolve the actual latest release tag to avoid version mismatch
            try {
                $latestResponse = Invoke-WebRequest -Uri "https://api.github.com/repos/${REPO}/releases/latest" -UseBasicParsing
                $latestTag = ($latestResponse.Content | ConvertFrom-Json).tag_name
                if ($latestTag) {
                    $GIT_REF = $latestTag
                    Write-Host "Resolved latest tag: $GIT_REF"
                } else {
                    Write-Warning "Could not resolve latest tag, falling back to main"
                    $GIT_REF = "main"
                }
            } catch {
                Write-Warning "Could not resolve latest tag, falling back to main"
                $GIT_REF = "main"
            }
        } else {
            # Add 'v' prefix if not present for git tags
            if ($Version -match '^\d+\.\d+\.\d+$') {
                $GIT_REF = "v${Version}"
            } else {
                $GIT_REF = $Version
            }
        }

        $GITHUB_RAW_BASE = "https://raw.githubusercontent.com/${REPO}/${GIT_REF}/skills"

        # Build skill file paths from PX_SKILLS array
        $SKILLS_LIST = foreach ($skill in $PX_SKILLS) {
            "${skill}/SKILL.md"
        }

        $ALL_SKILLS_DOWNLOADED = $true

        foreach ($skillPath in $SKILLS_LIST) {
            $skillDir = Join-Path $SKILLS_DIR (Split-Path $skillPath)
            $skillFile = Join-Path $SKILLS_DIR $skillPath

            if (-not (Test-Path $skillDir)) {
                New-Item -ItemType Directory -Path $skillDir -Force | Out-Null
            }

            try {
                Invoke-WebRequest -Uri "${GITHUB_RAW_BASE}/${skillPath}" -OutFile $skillFile -UseBasicParsing
                Write-Host "Downloaded: $skillPath"
            } catch {
                Write-Host "Failed to download: $skillPath"
                $ALL_SKILLS_DOWNLOADED = $false
            }
        }

        if ($ALL_SKILLS_DOWNLOADED) {
            Write-Host "Skills installed from GitHub raw content to $SKILLS_DIR"
            $SKILLS_INSTALLED = $true
        } else {
            Write-Error "Error: Failed to download some skills from GitHub"
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
    if ($LASTEXITCODE -ne 0) { throw "PX version check failed with exit code $LASTEXITCODE" }
    & (Join-Path $InstallDir $MCP_BINARY_NAME) --version
    if ($LASTEXITCODE -ne 0) { throw "PX MCP server version check failed with exit code $LASTEXITCODE" }

    if ($env:PX_TELEMETRY -ne "0") {
        try {
            $TelemetryDir = Join-Path $env:LOCALAPPDATA "px"
            $InstallIdPath = Join-Path $TelemetryDir "install-id"
            if (-not (Test-Path $TelemetryDir)) {
                New-Item -ItemType Directory -Path $TelemetryDir -Force | Out-Null
            }
            $InstallId = if (Test-Path $InstallIdPath) { (Get-Content -Path $InstallIdPath -Raw).Trim() } else { "" }
            if ($InstallId -notmatch '^[0-9a-fA-F]{32}$') {
                [Guid]::NewGuid().ToString("N") | Set-Content -Path $InstallIdPath -NoNewline
            }
            $InstallId = (Get-Content -Path $InstallIdPath -Raw).Trim()
            if ($InstallId -match '^[0-9a-fA-F]{32}$') {
                $Telemetry = @{
                    install_id = $InstallId.ToLowerInvariant()
                    download_id = $env:PX_DOWNLOAD_ID
                    version = $Version
                    platform = $TARGET
                } | ConvertTo-Json -Compress
                Invoke-RestMethod -Uri "https://portals.works/api/px/install" -Method Post -ContentType "application/json" -Body $Telemetry -TimeoutSec 5 | Out-Null
            }
        } catch {
            # Telemetry must never make a successful install fail.
        }
    }
} finally {
    # Cleanup
    Remove-Item -Path $TMP_DIR -Recurse -Force -ErrorAction SilentlyContinue
}
