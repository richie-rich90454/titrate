# Titrate Build Script — Windows (PowerShell)
# Compiles the trc compiler, pipette build tool, and titrate_native runtime.
# Usage: .\scripts\build.ps1 [-Release] [-Clean] [-Target <target>]
param(
    [switch]$Release,
    [switch]$Clean,
    [string]$Target = "all"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir\.."
$TrcDir = "$ProjectRoot\trc"

Write-Host "=== Titrate Build Script ===" -ForegroundColor Cyan
Write-Host "Project root: $ProjectRoot" -ForegroundColor Gray

# Check prerequisites
Write-Host "`n[1/4] Checking prerequisites..." -ForegroundColor Yellow

# Check Rust
try {
    $rustVersion = rustc --version 2>&1
    Write-Host "  Rust: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "  ERROR: Rust is not installed. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}

# Check LLVM
$llvmConfig = Get-Command llvm-config -ErrorAction SilentlyContinue
if ($llvmConfig) {
    $llvmVersion = & llvm-config --version 2>&1
    Write-Host "  LLVM: $llvmVersion" -ForegroundColor Green
} else {
    Write-Host "  WARNING: llvm-config not found. LLVM native backend may not compile." -ForegroundColor Yellow
    Write-Host "  Install LLVM 22.1 from https://github.com/llvm/llvm-project/releases" -ForegroundColor Yellow
}

# Check for LLVM-C.dll
$llvmDll = "$env:ProgramFiles\LLVM\bin\LLVM-C.dll"
if (Test-Path $llvmDll) {
    Write-Host "  LLVM-C.dll: found" -ForegroundColor Green
} else {
    Write-Host "  WARNING: LLVM-C.dll not found at $llvmDll" -ForegroundColor Yellow
}

# Clean if requested
if ($Clean) {
    Write-Host "`n[2/4] Cleaning previous build artifacts..." -ForegroundColor Yellow
    Push-Location $TrcDir
    cargo clean
    Pop-Location
    Write-Host "  Clean complete." -ForegroundColor Green
} else {
    Write-Host "`n[2/4] Skipping clean (use -Clean to force)." -ForegroundColor Gray
}

# Build
$buildFlag = if ($Release) { "--release" } else { "" }
Write-Host "`n[3/4] Building Titrate workspace..." -ForegroundColor Yellow
if ($Release) {
    Write-Host "  Mode: Release (optimized)" -ForegroundColor Gray
} else {
    Write-Host "  Mode: Debug (fast compile, no optimizations)" -ForegroundColor Gray
}

Push-Location $TrcDir
$buildArgs = @("build")
if ($Release) { $buildArgs += "--release" }
if ($Target -ne "all") {
    $buildArgs += "-p"
    $buildArgs += $Target
}
$buildArgs += "2>&1"

$buildOutput = & cargo @buildArgs
if ($LASTEXITCODE -ne 0) {
    Write-Host "  BUILD FAILED" -ForegroundColor Red
    Write-Host $buildOutput
    Pop-Location
    exit 1
}
Pop-Location
Write-Host "  Build complete." -ForegroundColor Green

# Verify binaries
Write-Host "`n[4/4] Verifying binaries..." -ForegroundColor Yellow
$profile = if ($Release) { "release" } else { "debug" }
$binDir = "$TrcDir\target\$profile"

$bins = @(
    @{Name="trc"; Path="$binDir\trc.exe"},
    @{Name="pipette"; Path="$binDir\pipette.exe"}
)

foreach ($bin in $bins) {
    if (Test-Path $bin.Path) {
        $size = (Get-Item $bin.Path).Length
        $sizeMB = [math]::Round($size / 1MB, 2)
        Write-Host "  $($bin.Name).exe: $sizeMB MB" -ForegroundColor Green
    } else {
        Write-Host "  WARNING: $($bin.Name).exe not found" -ForegroundColor Yellow
    }
}

# Check titrate_native DLL
$nativeDll = "$binDir\titrate_native.dll"
if (Test-Path $nativeDll) {
    $size = (Get-Item $nativeDll).Length
    $sizeMB = [math]::Round($size / 1MB, 2)
    Write-Host "  titrate_native.dll: $sizeMB MB" -ForegroundColor Green
} else {
    Write-Host "  NOTE: titrate_native.dll not found (only needed for LLVM native backend)" -ForegroundColor Gray
}

Write-Host "`n=== Build Complete ===" -ForegroundColor Cyan
Write-Host "Binaries: $binDir" -ForegroundColor Gray
Write-Host "`nNext steps:" -ForegroundColor White
Write-Host "  Run tests:   cd trc && cargo test --lib" -ForegroundColor Gray
Write-Host "  Run a file:  .\trc\target\$profile\trc.exe hello.tr" -ForegroundColor Gray
Write-Host "  Full test:   cd trc && cargo test --all" -ForegroundColor Gray