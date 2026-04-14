<#
.SYNOPSIS
    OptiDock AI — Windows Installer
.DESCRIPTION
    Builds the OptiDock CLI from source and installs it to your PATH.
    After installation, run `optidock doctor` to verify everything is ready.
#>

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "  ╭──────────────────────────────────────────╮" -ForegroundColor Cyan
Write-Host "  │           OptiDock AI Installer           │" -ForegroundColor Cyan
Write-Host "  │    Rust-first Docker optimization agent   │" -ForegroundColor Cyan
Write-Host "  ╰──────────────────────────────────────────╯" -ForegroundColor Cyan
Write-Host ""

# ── Check prerequisites ──────────────────────────────────────────────

function Test-CommandExists($command) {
    $null = Get-Command $command -ErrorAction SilentlyContinue
    return $?
}

Write-Host "  [1/5] Checking prerequisites..." -ForegroundColor Yellow

if (-not (Test-CommandExists "rustc")) {
    Write-Host "  ✗ Rust is not installed." -ForegroundColor Red
    Write-Host "    Install from https://rustup.rs" -ForegroundColor Gray
    Write-Host "    Then re-run this installer." -ForegroundColor Gray
    exit 1
}
$rustVersion = (rustc --version 2>&1) -join " "
Write-Host "  ✓ $rustVersion" -ForegroundColor Green

if (-not (Test-CommandExists "cargo")) {
    Write-Host "  ✗ Cargo is not installed." -ForegroundColor Red
    exit 1
}
$cargoVersion = (cargo --version 2>&1) -join " "
Write-Host "  ✓ $cargoVersion" -ForegroundColor Green

if (Test-CommandExists "docker") {
    $dockerVersion = (docker --version 2>&1) -join " "
    Write-Host "  ✓ $dockerVersion" -ForegroundColor Green
} else {
    Write-Host "  ⚠ Docker not found. Some features will be limited." -ForegroundColor Yellow
}

# ── Find project root ────────────────────────────────────────────────

Write-Host ""
Write-Host "  [2/5] Locating project..." -ForegroundColor Yellow

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

if (-not (Test-Path (Join-Path $projectRoot "Cargo.toml"))) {
    Write-Host "  ✗ Cargo.toml not found at $projectRoot" -ForegroundColor Red
    exit 1
}
Write-Host "  ✓ Project root: $projectRoot" -ForegroundColor Green

# ── Build release binary ─────────────────────────────────────────────

Write-Host ""
Write-Host "  [3/5] Building release binary (this may take a few minutes)..." -ForegroundColor Yellow

Push-Location $projectRoot
try {
    cargo build --release --bin optidock 2>&1 | ForEach-Object {
        if ($_ -match "Compiling|Finished") {
            Write-Host "    $_" -ForegroundColor Gray
        }
    }
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  ✗ Build failed." -ForegroundColor Red
        exit 1
    }
} finally {
    Pop-Location
}

$binarySource = Join-Path $projectRoot "target\release\optidock.exe"
if (-not (Test-Path $binarySource)) {
    Write-Host "  ✗ Binary not found at $binarySource" -ForegroundColor Red
    exit 1
}

$size = [math]::Round((Get-Item $binarySource).Length / 1MB, 1)
Write-Host "  ✓ Built optidock.exe ($size MB)" -ForegroundColor Green

# ── Install to PATH ──────────────────────────────────────────────────

Write-Host ""
Write-Host "  [4/5] Installing to PATH..." -ForegroundColor Yellow

$installDir = Join-Path $env:USERPROFILE ".optidock\bin"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$installPath = Join-Path $installDir "optidock.exe"
Copy-Item $binarySource $installPath -Force
Write-Host "  ✓ Copied to $installPath" -ForegroundColor Green

# Add to PATH if not already there
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$installDir", "User")
    $env:PATH = "$env:PATH;$installDir"
    Write-Host "  ✓ Added $installDir to user PATH" -ForegroundColor Green
    Write-Host "    (restart your terminal for PATH to take effect)" -ForegroundColor Gray
} else {
    Write-Host "  ✓ PATH already configured" -ForegroundColor Green
}

# ── Copy default config ──────────────────────────────────────────────

$configDir = Join-Path $env:USERPROFILE ".optidock"
$envFile = Join-Path $configDir ".optidock.env"
if (-not (Test-Path $envFile)) {
    $envSource = Join-Path $projectRoot ".env.example"
    if (Test-Path $envSource) {
        Copy-Item $envSource $envFile
        Write-Host "  ✓ Default config created at $envFile" -ForegroundColor Green
    }
}

# ── Done ──────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "  [5/5] Verifying installation..." -ForegroundColor Yellow

& $installPath --version 2>&1 | ForEach-Object {
    Write-Host "  ✓ $_" -ForegroundColor Green
}

Write-Host ""
Write-Host "  ╭──────────────────────────────────────────╮" -ForegroundColor Green
Write-Host "  │         Installation complete!            │" -ForegroundColor Green
Write-Host "  ╰──────────────────────────────────────────╯" -ForegroundColor Green
Write-Host ""
Write-Host "  Available commands:" -ForegroundColor Cyan
Write-Host "    optidock init          Bootstrap a Docker project" -ForegroundColor White
Write-Host "    optidock analyze .     Analyze Dockerfile for issues" -ForegroundColor White
Write-Host "    optidock security .    Run security audit" -ForegroundColor White
Write-Host "    optidock optimize .    Generate optimized Dockerfile" -ForegroundColor White
Write-Host "    optidock benchmark .   Build and compare images" -ForegroundColor White
Write-Host "    optidock deploy IMG    Deploy a container locally" -ForegroundColor White
Write-Host "    optidock monitor       Show container status" -ForegroundColor White
Write-Host "    optidock rollback NAME Stop and remove container" -ForegroundColor White
Write-Host "    optidock pipeline .    Run deployment moderation" -ForegroundColor White
Write-Host "    optidock providers     Show AI provider config" -ForegroundColor White
Write-Host "    optidock doctor        Check local environment" -ForegroundColor White
Write-Host "    optidock live .        Interactive agent terminal" -ForegroundColor White
Write-Host "    optidock serve         Start HTTP API server" -ForegroundColor White
Write-Host ""
Write-Host "  Quick start:" -ForegroundColor Yellow
Write-Host "    optidock doctor" -ForegroundColor White
Write-Host "    optidock analyze ." -ForegroundColor White
Write-Host "    optidock security ." -ForegroundColor White
Write-Host ""
