# resumake installer for Windows
# https://github.com/arvinduh/resumake

$ErrorActionPreference = 'Stop'

function Write-Info {
    param([string]$Message)
    Write-Host "info: " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "success: " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "warning: " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Err {
    param([string]$Message)
    Write-Host "error: " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

$target = "x86_64-pc-windows-msvc"
$assetName = "resumake-$target.zip"

$version = if ($env:RESUMAKE_VERSION) { $env:RESUMAKE_VERSION } else { "latest" }
if ($version -eq "latest") {
    $downloadUrl = "https://github.com/arvinduh/resumake/releases/latest/download/$assetName"
} else {
    $downloadUrl = "https://github.com/arvinduh/resumake/releases/download/v$($version -replace '^v', '')/$assetName"
}

$installDir = if ($env:RESUMAKE_INSTALL_DIR) {
    $env:RESUMAKE_INSTALL_DIR
} else {
    Join-Path $HOME "bin"
}

Write-Info "Detected platform: $target"
if ($version -ne "latest") {
    Write-Info "Requested version: v$($version -replace '^v', '')"
}
Write-Info "Installing rsmk into $installDir..."

if (-not (Test-Path -Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
    $zipPath = Join-Path $tempDir $assetName
    Write-Info "Downloading $downloadUrl..."
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

    Write-Info "Verifying checksum..."
    $checksumPath = "$zipPath.sha256"
    Invoke-WebRequest -Uri "$downloadUrl.sha256" -OutFile $checksumPath -UseBasicParsing
    # Checksum file format is "<hash>  <filename>"
    $expectedHash = ((Get-Content -Path $checksumPath -Raw).Trim() -split '\s+')[0]
    $actualHash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash
    if ($actualHash -ne $expectedHash) {
        Write-Err "Checksum verification failed for $assetName (expected $expectedHash, got $actualHash). Refusing to install."
    }
    Write-Success "Checksum verified."

    Write-Info "Extracting binary..."
    Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

    $binarySource = Join-Path $tempDir "rsmk.exe"
    $binaryDest = Join-Path $installDir "rsmk.exe"

    Copy-Item -Path $binarySource -Destination $binaryDest -Force
    Write-Success "rsmk installed successfully to $binaryDest"

    # Check PATH
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$installDir*") {
        Write-Warn "$installDir is not currently in your User PATH."
        Write-Warn "To add it automatically, run:"
        Write-Warn "  [Environment]::SetEnvironmentVariable('Path', `"`$currentPath;$installDir`", 'User')"
    }

    & $binaryDest --version
}
catch {
    Write-Err "Installation failed: $_"
}
finally {
    Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
