# Zyra Programming Language Installer
# Windows Installation Script
# 
# This script installs Zyra to your system by:
# 1. Using prebuilt binary from ./bin/windows/zyra.exe if available
# 2. Falling back to building from source if binary not found
# 3. Creating installation directory
# 4. Adding Zyra to PATH
#
# Run as Administrator for system-wide installation,
# or run normally for user-level installation.

param(
    [switch]$Uninstall = $false,
    [switch]$Build = $false,
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"

# Colors
function Write-Header { param($msg) Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-Step { param($msg) Write-Host "  → $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "  ⚠ $msg" -ForegroundColor Yellow }
function Write-Err { param($msg) Write-Host "  ✗ $msg" -ForegroundColor Red }

# Determine install location
$IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if ($InstallDir -eq "") {
    if ($IsAdmin) {
        $InstallDir = "C:\Program Files\Zyra"
    } else {
        $InstallDir = "$env:LOCALAPPDATA\Zyra"
    }
}

$BinDir = "$InstallDir\bin"
$ExePath = "$BinDir\zyra.exe"

# Uninstall
if ($Uninstall) {
    Write-Header "Uninstalling Zyra"
    
    if (Test-Path $InstallDir) {
        Write-Step "Removing $InstallDir..."
        Remove-Item -Recurse -Force $InstallDir
    }
    
    Write-Step "Removing from PATH..."
    $PathScope = if ($IsAdmin) { "Machine" } else { "User" }
    $CurrentPath = [Environment]::GetEnvironmentVariable("PATH", $PathScope)
    $NewPath = ($CurrentPath -split ";" | Where-Object { $_ -ne $BinDir }) -join ";"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, $PathScope)
    
    Write-Host "`n✓ Zyra has been uninstalled." -ForegroundColor Green
    exit 0
}

# Install
Write-Header "Zyra Programming Language Installer"
Write-Host "  Version: 1.0.2"
Write-Host "  Install Dir: $InstallDir"
Write-Host "  Mode: $(if ($IsAdmin) { 'System-wide' } else { 'User-level' })"

# Find project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

# Check for prebuilt binary
$PrebuiltBinary = "$ScriptDir\bin\windows\zyra.exe"
$BuildBinary = "$ProjectRoot\target\release\zyra.exe"
$UsePrebuilt = $false

Write-Header "Checking Binary"
if (-not $Build -and (Test-Path $PrebuiltBinary)) {
    Write-Step "Found prebuilt binary at $PrebuiltBinary"
    $UsePrebuilt = $true
} else {
    if ($Build) {
        Write-Step "Force build requested, will compile from source"
    } else {
        Write-Warn "Prebuilt binary not found at $PrebuiltBinary"
    }
}

# Build from source if needed
if (-not $UsePrebuilt) {
    Write-Header "Building from Source"
    
    # Check for Rust/Cargo
    try {
        $cargoVersion = cargo --version
        Write-Step "Cargo found: $cargoVersion"
    } catch {
        Write-Err "Cargo not found. Please install Rust from https://rustup.rs/"
        Write-Err "Or provide a prebuilt binary at $PrebuiltBinary"
        exit 1
    }
    
    Push-Location $ProjectRoot
    try {
        Write-Step "Building release binary..."
        cargo build --release
        if ($LASTEXITCODE -ne 0) {
            throw "Build failed"
        }
        Write-Step "Build successful!"
    } finally {
        Pop-Location
    }
}

# Create install directory
Write-Header "Installing"
if (-not (Test-Path $BinDir)) {
    Write-Step "Creating $BinDir..."
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
}

# Copy binary
Write-Step "Copying zyra.exe..."
if ($UsePrebuilt) {
    Copy-Item $PrebuiltBinary -Destination $ExePath -Force
} else {
    Copy-Item $BuildBinary -Destination $ExePath -Force
}

# Add to PATH
Write-Header "Configuring PATH"
$PathScope = if ($IsAdmin) { "Machine" } else { "User" }
$CurrentPath = [Environment]::GetEnvironmentVariable("PATH", $PathScope)

if ($CurrentPath -notlike "*$BinDir*") {
    Write-Step "Adding $BinDir to PATH..."
    [Environment]::SetEnvironmentVariable("PATH", "$CurrentPath;$BinDir", $PathScope)
} else {
    Write-Step "Already in PATH"
}

# Verify
Write-Header "Verification"
$env:PATH = "$env:PATH;$BinDir"
try {
    $version = & $ExePath --version
    Write-Step "Installed: $version"
} catch {
    Write-Warn "Could not verify installation"
}

Write-Host "`n" -NoNewline
Write-Host "╔════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  ✓ Zyra has been installed!            ║" -ForegroundColor Cyan
Write-Host "╠════════════════════════════════════════╣" -ForegroundColor Cyan
Write-Host "║  Restart your terminal, then run:      ║" -ForegroundColor Cyan
Write-Host "║    zyra --version                      ║" -ForegroundColor White
Write-Host "╚════════════════════════════════════════╝" -ForegroundColor Cyan
