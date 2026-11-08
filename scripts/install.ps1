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

$ASSET = "px-$TARGET.exe"
$MCP_ASSET = "px-mcp-server-$TARGET.exe"

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
$InstallStarted = $false
$InstallCommitted = $false

try {
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

# Verify the release workflow's signed checksums before executing or copying
# any release asset. cosign is deliberately not downloaded by this script.
if (-not (Get-Command cosign -ErrorAction SilentlyContinue)) {
    throw "cosign is required to verify the PX release signature"
}
$ReleaseBase = $URL.Substring(0, $URL.LastIndexOf('/'))
$Checksums = Join-Path $TMP_DIR "SHA256SUMS"
$Signature = Join-Path $TMP_DIR "SHA256SUMS.sigstore.json"
Invoke-WebRequest -Uri "$ReleaseBase/SHA256SUMS" -OutFile $Checksums -UseBasicParsing
Invoke-WebRequest -Uri "$ReleaseBase/SHA256SUMS.sigstore.json" -OutFile $Signature -UseBasicParsing
Invoke-WebRequest -Uri $SKILLS_URL -OutFile $SKILLS_TMP -UseBasicParsing
$OptimizerArchive = "px-optimizer-$TARGET.tar.gz"
$Verifier = "px-verify-optimizer-release-$TARGET.exe"
foreach ($file in @($OptimizerArchive, $Verifier, 'optimizer-release.json', 'manifest.json', 'manifest.sig')) {
    Invoke-WebRequest -Uri "$ReleaseBase/$file" -OutFile (Join-Path $TMP_DIR $file) -UseBasicParsing
}
$identity = '^https://github\.com/portalshq/px/\.github/workflows/cli-release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$'
if ($Version -ne 'latest') {
    if ($Version -cnotmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') { throw 'Version must be a release tag such as v0.10.0' }
    $identity = '^https://github\.com/portalshq/px/\.github/workflows/cli-release\.yml@refs/tags/' + [regex]::Escape($Version) + '$'
}
& cosign verify-blob --bundle $Signature `
    --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' `
    --certificate-identity-regexp $identity $Checksums
if ($LASTEXITCODE -ne 0) { throw "PX release signature verification failed" }
$ChecksumLines = @(Get-Content -LiteralPath $Checksums)
foreach ($file in @($ASSET, $MCP_ASSET, $SKILLS_ARCHIVE, $OptimizerArchive, $Verifier, 'optimizer-release.json', 'manifest.json', 'manifest.sig')) {
    $escaped = [regex]::Escape("./$file")
    $checksumMatches = @($ChecksumLines | Where-Object { $_ -cmatch "^[0-9a-f]{64}  $escaped$" })
    if ($checksumMatches.Count -ne 1) { throw "Missing or duplicate checksum for $file" }
    $expected = $checksumMatches[0].Substring(0, 64)
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $TMP_DIR $file)).Hash.ToLowerInvariant()
    if ($actual -cne $expected) { throw "Checksum mismatch for $file" }
}
& (Join-Path $TMP_DIR $Verifier) (Join-Path $TMP_DIR 'optimizer-release.json') (Join-Path $TMP_DIR 'manifest.json') (Join-Path $TMP_DIR 'manifest.sig') $TARGET (Join-Path $TMP_DIR $OptimizerArchive)
if ($LASTEXITCODE -ne 0) { throw 'Optimizer release verification failed' }
$OptimizerDir = Join-Path $TMP_DIR 'optimizer'
New-Item -ItemType Directory -Path $OptimizerDir | Out-Null
& tar -xzf (Join-Path $TMP_DIR $OptimizerArchive) -C $OptimizerDir
if ($LASTEXITCODE -ne 0) { throw 'Optimizer extraction failed' }

$BackupBin = Join-Path $TMP_DIR 'backup/bin'
$BackupSkills = Join-Path $TMP_DIR 'backup/skills'
New-Item -ItemType Directory -Path $BackupBin, $BackupSkills -Force | Out-Null
foreach ($name in @($BINARY_NAME, $MCP_BINARY_NAME)) {
    $existing = Join-Path $InstallDir $name
    if (Test-Path -LiteralPath $existing) { Copy-Item -LiteralPath $existing -Destination (Join-Path $BackupBin $name) }
}
$SkillRoot = Join-Path $env:USERPROFILE '.agents/skills'
foreach ($skill in @('ask-px', 'character-reference-sheet', 'px-repo', 'px-resolve', 'px-update')) {
    $existing = Join-Path $SkillRoot $skill
    if (Test-Path -LiteralPath $existing) { Copy-Item -LiteralPath $existing -Destination (Join-Path $BackupSkills $skill) -Recurse }
}
$InstallStarted = $true

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

    # The skills archive was verified together with the binaries above.
    Expand-Archive -Path $SKILLS_TMP -DestinationPath $SKILLS_DIR -Force
    Write-Host "Skills installed to $SKILLS_DIR"

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
    & (Join-Path $InstallDir $BINARY_NAME) --version
    if ($LASTEXITCODE -ne 0) { throw "PX version check failed with exit code $LASTEXITCODE" }
    & (Join-Path $InstallDir $MCP_BINARY_NAME) --version
    if ($LASTEXITCODE -ne 0) { throw "PX MCP server version check failed with exit code $LASTEXITCODE" }

    $candidateLine = @(Get-Content -LiteralPath (Join-Path $OptimizerDir 'plugin.yaml') | Where-Object { $_ -match '^version:\s*["'']?([0-9]+\.[0-9]+\.[0-9]+)["'']?\s*$' })
    if ($candidateLine.Count -ne 1 -or $candidateLine[0] -notmatch '^version:\s*["'']?([0-9]+\.[0-9]+\.[0-9]+)["'']?\s*$') { throw 'Invalid optimizer package version' }
    $candidateVersion = $Matches[1]
    $px = Join-Path $InstallDir $BINARY_NAME
    $installedInfo = & $px plugin info px.optimizer 2>$null
    if ($LASTEXITCODE -eq 0) {
        $installedVersion = ($installedInfo | ConvertFrom-Json).version
        if (-not $installedVersion) { throw 'Cannot determine installed optimizer version' }
        if ($installedVersion -ne $candidateVersion) {
            & $px plugin upgrade $OptimizerDir
            if ($LASTEXITCODE -ne 0) { throw 'Optimizer upgrade failed' }
        }
    } else {
        & $px plugin install $OptimizerDir
        if ($LASTEXITCODE -ne 0) { throw 'Optimizer installation failed' }
        & $px plugin enable px.optimizer
        if ($LASTEXITCODE -ne 0) {
            & $px plugin uninstall px.optimizer
            throw 'Optimizer enablement failed'
        }
    }
    $InstallCommitted = $true
    Write-Host "Installed successfully."

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
    if ($InstallStarted -and -not $InstallCommitted) {
        Write-Warning 'Installation failed; restoring previous PX files.'
        foreach ($name in @($BINARY_NAME, $MCP_BINARY_NAME)) {
            $destination = Join-Path $InstallDir $name
            $backup = Join-Path $BackupBin $name
            if (Test-Path -LiteralPath $backup) { Copy-Item -LiteralPath $backup -Destination $destination -Force }
            elseif (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Force }
        }
        foreach ($skill in @('ask-px', 'character-reference-sheet', 'px-repo', 'px-resolve', 'px-update')) {
            $destination = Join-Path $SkillRoot $skill
            $backup = Join-Path $BackupSkills $skill
            if (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Recurse -Force }
            if (Test-Path -LiteralPath $backup) { New-Item -ItemType Directory -Path $SkillRoot -Force | Out-Null; Copy-Item -LiteralPath $backup -Destination $destination -Recurse }
        }
    }
    # Cleanup
    Remove-Item -Path $TMP_DIR -Recurse -Force -ErrorAction SilentlyContinue
}
