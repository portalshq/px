#!/usr/bin/env pwsh

param(
    [string]$Version = "latest",
    [string]$InstallDir,
    [string]$BaseUrl
)

$ErrorActionPreference = "Stop"
$REPO = "portalshq/px"
$BINARY_NAME = "px.exe"
$MCP_BINARY_NAME = "px-mcp-server.exe"
$SKILLS_ARCHIVE = "px-skills.zip"
$TARGET = "x86_64-pc-windows-msvc"
$PX_HOME = Join-Path $env:USERPROFILE ".px"
$env:PX_DIR = $PX_HOME
$TXN_DIR = Join-Path $PX_HOME "installer-transaction"
$COMPLETED_DIR = "$TXN_DIR.completed"
New-Item -ItemType Directory -Path $PX_HOME -Force | Out-Null
$lockPath = Join-Path $PX_HOME "installer.lock"
try {
    $script:InstallerLock = [System.IO.File]::Open($lockPath, [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
} catch {
    throw "Another PX installation is running. If it was interrupted, close old installer processes and rerun."
}

if ([string]::IsNullOrEmpty($InstallDir)) { $InstallDir = Join-Path $env:USERPROFILE "bin" }
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
New-Item -ItemType Directory -Path $InstallDir, $PX_HOME -Force | Out-Null
$ASSET = "px-$TARGET.exe"
$MCP_ASSET = "px-mcp-server-$TARGET.exe"
$OPTIMIZER_ARCHIVE = "px-optimizer-$TARGET.tar.gz"
$VERIFIER = "px-verify-optimizer-release-$TARGET.exe"
$RELEASE_FILES = @($ASSET, $MCP_ASSET, $SKILLS_ARCHIVE, $OPTIMIZER_ARCHIVE, $VERIFIER, "optimizer-release.json", "manifest.json", "manifest.sig")

Write-Host "Installing PX"
Write-Host "Platform : $TARGET"
Write-Host "Version  : $Version"
if ($env:PX_TELEMETRY -ne "0") { Write-Host "Anonymous install counting is enabled. Set PX_TELEMETRY=0 to disable it." }

function Get-ReleaseIdentity([string]$ReleaseVersion) {
    if ($ReleaseVersion -eq "latest") { return '^https://github\.com/portalshq/px/\.github/workflows/cli-release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$' }
    if ($ReleaseVersion -cnotmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') { throw "Invalid release version in installer transaction" }
    return '^https://github\.com/portalshq/px/\.github/workflows/cli-release\.yml@refs/tags/' + [regex]::Escape($ReleaseVersion) + '$'
}

function Test-VerifiedPayload([string]$Directory, [string]$ReleaseVersion) {
    $checksums = Join-Path $Directory "SHA256SUMS"
    $signature = Join-Path $Directory "SHA256SUMS.sigstore.json"
    if (-not (Get-Command cosign -ErrorAction SilentlyContinue)) { throw "cosign is required to verify the PX release signature" }
    & cosign verify-blob --bundle $signature --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' --certificate-identity-regexp (Get-ReleaseIdentity $ReleaseVersion) $checksums
    if ($LASTEXITCODE -ne 0) { throw "PX release signature verification failed" }
    $lines = @(Get-Content -LiteralPath $checksums)
    foreach ($file in $RELEASE_FILES) {
        $escaped = [regex]::Escape("./$file")
        $matches = @($lines | Where-Object { $_ -cmatch "^[0-9a-f]{64}  $escaped$" })
        if ($matches.Count -ne 1) { throw "Missing or duplicate checksum for $file" }
        $expected = $matches[0].Substring(0, 64)
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $Directory $file)).Hash.ToLowerInvariant()
        if ($actual -cne $expected) { throw "Checksum mismatch for $file" }
    }
    & (Join-Path $Directory $VERIFIER) (Join-Path $Directory "optimizer-release.json") (Join-Path $Directory "manifest.json") (Join-Path $Directory "manifest.sig") $TARGET (Join-Path $Directory $OPTIMIZER_ARCHIVE)
    if ($LASTEXITCODE -ne 0) { throw "Optimizer release verification failed" }
}

function Set-AtomicFile([string]$Source, [string]$Destination) {
    $parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $temporary = Join-Path $parent (".px-new-" + [Guid]::NewGuid().ToString("N"))
    Copy-Item -LiteralPath $Source -Destination $temporary
    if (Test-Path -LiteralPath $Destination) {
        [System.IO.File]::Move($temporary, $Destination, $true)
    } else {
        [System.IO.File]::Move($temporary, $Destination)
    }
}

function Invoke-InstallTransaction([string]$Directory, [string]$TargetDirectory, [string]$EnableFresh) {
    $env:PX_INSTALL_TRANSACTION = "1"
    $env:PX_DIR = $PX_HOME
    Set-AtomicFile (Join-Path $Directory $ASSET) (Join-Path $TargetDirectory $BINARY_NAME)
    Set-AtomicFile (Join-Path $Directory $MCP_ASSET) (Join-Path $TargetDirectory $MCP_BINARY_NAME)

    $skillRoot = Join-Path $env:USERPROFILE ".agents/skills"
    New-Item -ItemType Directory -Path $skillRoot -Force | Out-Null
    foreach ($skill in @("ask-px", "character-reference-sheet", "px-repo", "px-resolve", "px-update")) {
        $path = Join-Path $skillRoot $skill
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force }
    }
    Expand-Archive -LiteralPath (Join-Path $Directory $SKILLS_ARCHIVE) -DestinationPath $skillRoot -Force

    $px = Join-Path $TargetDirectory $BINARY_NAME
    & $px --version
    if ($LASTEXITCODE -ne 0) { throw "PX version check failed" }
    & (Join-Path $TargetDirectory $MCP_BINARY_NAME) --version
    if ($LASTEXITCODE -ne 0) { throw "PX MCP server version check failed" }

    $pluginFile = Join-Path $Directory "optimizer/plugin.yaml"
    $versionLines = @(Get-Content -LiteralPath $pluginFile | Where-Object { $_ -match '^version:\s*["'']?([0-9]+\.[0-9]+\.[0-9]+)["'']?\s*$' })
    if ($versionLines.Count -ne 1 -or $versionLines[0] -notmatch '^version:\s*["'']?([0-9]+\.[0-9]+\.[0-9]+)["'']?\s*$') { throw "Invalid optimizer package version" }
    $candidateVersion = $Matches[1]
    $info = & $px plugin info px.optimizer 2>$null
    if ($LASTEXITCODE -eq 0) {
        $installedVersion = ($info | ConvertFrom-Json).version
        if (-not $installedVersion) { throw "Cannot determine installed optimizer version" }
        if ($installedVersion -ne $candidateVersion) {
            & $px plugin upgrade (Join-Path $Directory "optimizer")
            if ($LASTEXITCODE -ne 0) { throw "Optimizer upgrade failed" }
        }
    } else {
        & $px plugin install (Join-Path $Directory "optimizer")
        if ($LASTEXITCODE -ne 0) { throw "Optimizer installation failed" }
    }
    if ($EnableFresh -eq "true") {
        & $px plugin enable px.optimizer
        if ($LASTEXITCODE -ne 0) { throw "Optimizer enablement failed" }
    }
    & $px plugin info px.optimizer | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Optimizer verification failed" }
}

function Complete-InstallTransaction([string]$Directory) {
    $completed = "$Directory.completed"
    if (Test-Path -LiteralPath $completed) { Remove-Item -LiteralPath $completed -Recurse -Force }
    Move-Item -LiteralPath $Directory -Destination $completed
    if (Test-Path -LiteralPath $completed) { Remove-Item -LiteralPath $completed -Recurse -Force }
}

function Resume-InstallTransaction {
    # A completed directory is harmless cleanup left after a force-kill.
    if (Test-Path -LiteralPath $COMPLETED_DIR) { Remove-Item -LiteralPath $COMPLETED_DIR -Recurse -Force }
    if (-not (Test-Path -LiteralPath $TXN_DIR)) { return }
    $required = @("STATE", "install-dir", "enable-fresh", "release-version")
    foreach ($name in $required) {
        if (-not (Test-Path -LiteralPath (Join-Path $TXN_DIR $name))) { throw "Incomplete PX installer transaction at $TXN_DIR; keep it and rerun the installer." }
    }
    $state = (Get-Content -LiteralPath (Join-Path $TXN_DIR "STATE") -Raw).Trim()
    if ($state -cne "px-installer-transaction-v1") { throw "Unknown PX installer transaction format" }
    $targetDirectory = (Get-Content -LiteralPath (Join-Path $TXN_DIR "install-dir") -Raw).TrimEnd([char]13, [char]10)
    $enableFresh = (Get-Content -LiteralPath (Join-Path $TXN_DIR "enable-fresh") -Raw).Trim()
    $releaseVersion = (Get-Content -LiteralPath (Join-Path $TXN_DIR "release-version") -Raw).Trim()
    if (-not $targetDirectory -or $targetDirectory -ne $InstallDir -or $enableFresh -notin @("true", "false")) {
        throw "Resume with the same install directory used by the interrupted installation: $targetDirectory"
    }
    Write-Host "Resuming verified PX installation from $TXN_DIR"
    Test-VerifiedPayload $TXN_DIR $releaseVersion
    $optimizer = Join-Path $TXN_DIR "optimizer"
    if (Test-Path -LiteralPath $optimizer) { Remove-Item -LiteralPath $optimizer -Recurse -Force }
    New-Item -ItemType Directory -Path $optimizer | Out-Null
    & tar -xzf (Join-Path $TXN_DIR $OPTIMIZER_ARCHIVE) -C $optimizer
    if ($LASTEXITCODE -ne 0) { throw "Optimizer extraction failed" }
    Invoke-InstallTransaction $TXN_DIR $targetDirectory $enableFresh
    Complete-InstallTransaction $TXN_DIR
}

# Finish a force-killed update from its durable signed payload before applying a new one.
Resume-InstallTransaction
Get-ChildItem -LiteralPath $PX_HOME -Directory -Filter ".installer-staging-*" -ErrorAction SilentlyContinue |
    Remove-Item -Recurse -Force

$stage = Join-Path $PX_HOME (".installer-staging-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
    if ([string]::IsNullOrEmpty($BaseUrl)) {
        if ($Version -eq "latest") { $BaseUrl = "https://github.com/$REPO/releases/latest/download" }
        else { $BaseUrl = "https://github.com/$REPO/releases/download/$Version" }
    }
    $BaseUrl = $BaseUrl.TrimEnd("/")
    foreach ($file in @($ASSET, $MCP_ASSET, "SHA256SUMS", "SHA256SUMS.sigstore.json", $SKILLS_ARCHIVE, $OPTIMIZER_ARCHIVE, $VERIFIER, "optimizer-release.json", "manifest.json", "manifest.sig")) {
        Invoke-WebRequest -Uri "$BaseUrl/$file" -OutFile (Join-Path $stage $file) -UseBasicParsing
    }
    Test-VerifiedPayload $stage $Version
    $optimizer = Join-Path $stage "optimizer"
    New-Item -ItemType Directory -Path $optimizer | Out-Null
    & tar -xzf (Join-Path $stage $OPTIMIZER_ARCHIVE) -C $optimizer
    if ($LASTEXITCODE -ne 0) { throw "Optimizer extraction failed" }

    $enableFresh = "true"
    $currentPx = Join-Path $InstallDir $BINARY_NAME
    if (Test-Path -LiteralPath $currentPx) {
        & $currentPx plugin info px.optimizer *> $null
        if ($LASTEXITCODE -eq 0) { $enableFresh = "false" }
    }
    Set-Content -LiteralPath (Join-Path $stage "install-dir") -Value $InstallDir -NoNewline
    Set-Content -LiteralPath (Join-Path $stage "enable-fresh") -Value $enableFresh -NoNewline
    Set-Content -LiteralPath (Join-Path $stage "release-version") -Value $Version -NoNewline
    Set-Content -LiteralPath (Join-Path $stage "STATE") -Value "px-installer-transaction-v1" -NoNewline
    Move-Item -LiteralPath $stage -Destination $TXN_DIR
    $stage = $TXN_DIR
    Invoke-InstallTransaction $TXN_DIR $InstallDir $enableFresh
    Complete-InstallTransaction $TXN_DIR
    $stage = $null
} finally {
    if ($stage -and $stage -ne $TXN_DIR -and (Test-Path -LiteralPath $stage)) {
        Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    }
    Remove-Item Env:PX_INSTALL_TRANSACTION -ErrorAction SilentlyContinue
    if ($script:InstallerLock) { $script:InstallerLock.Dispose() }
}

$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$InstallDir*") { Write-Host "$InstallDir is not on your PATH. Add it to your user PATH." }
Write-Host "Installed successfully."

if ($env:PX_TELEMETRY -ne "0") {
    try {
        $TelemetryDir = Join-Path $env:LOCALAPPDATA "px"
        $InstallIdPath = Join-Path $TelemetryDir "install-id"
        New-Item -ItemType Directory -Path $TelemetryDir -Force | Out-Null
        $InstallId = if (Test-Path $InstallIdPath) { (Get-Content -Path $InstallIdPath -Raw).Trim() } else { "" }
        if ($InstallId -notmatch '^[0-9a-fA-F]{32}$') {
            [Guid]::NewGuid().ToString("N") | Set-Content -Path $InstallIdPath -NoNewline
            $InstallId = (Get-Content -Path $InstallIdPath -Raw).Trim()
        }
        if ($InstallId -match '^[0-9a-fA-F]{32}$') {
            $Telemetry = @{ install_id = $InstallId.ToLowerInvariant(); download_id = $env:PX_DOWNLOAD_ID; version = $Version; platform = $TARGET } | ConvertTo-Json -Compress
            Invoke-RestMethod -Uri "https://portals.works/api/px/install" -Method Post -ContentType "application/json" -Body $Telemetry -TimeoutSec 5 | Out-Null
        }
    } catch { }
}
