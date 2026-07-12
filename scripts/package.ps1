# Titrate Packaging Script — Windows
# Creates a distributable package of the Titrate compiler and standard library.
# Usage: .\scripts\package.ps1 [-Release] [-Output <dir>]
param(
    [switch]$Release,
    [string]$Output = ""
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir\.."
$TrcDir = "$ProjectRoot\trc"
$Profile = if ($Release) { "release" } else { "debug" }
$BinDir = "$ProjectRoot\target\$Profile"

# Determine output directory
if ($Output -eq "") {
    $Version = "0.2.0"
    $Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }
    $Output = "$ProjectRoot\dist\titrate-$Version-windows-$Arch"
}

Write-Host "=== Titrate Packaging Script ===" -ForegroundColor Cyan
Write-Host "Output: $Output" -ForegroundColor Gray

# Step 1: Build if needed
Write-Host "`n[1/5] Ensuring build exists..." -ForegroundColor Yellow
if (-not (Test-Path "$BinDir\trc.exe")) {
    Write-Host "  Binary not found. Building..." -ForegroundColor Gray
    Push-Location $TrcDir
    $buildArgs = @("build")
    if ($Release) { $buildArgs += "--release" }
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  BUILD FAILED" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Pop-Location
} else {
    Write-Host "  Binary found at $BinDir" -ForegroundColor Green
}

# Step 2: Create output directory structure
Write-Host "`n[2/5] Creating package directory structure..." -ForegroundColor Yellow
if (Test-Path $Output) {
    Remove-Item -Recurse -Force $Output
}
New-Item -ItemType Directory -Force -Path "$Output\bin" | Out-Null
New-Item -ItemType Directory -Force -Path "$Output\lib\tt" | Out-Null
New-Item -ItemType Directory -Force -Path "$Output\examples" | Out-Null
Write-Host "  Directory structure created." -ForegroundColor Green

# Step 3: Copy binaries
Write-Host "`n[3/5] Copying binaries..." -ForegroundColor Yellow
Copy-Item "$BinDir\trc.exe" "$Output\bin\" -Force
Write-Host "  trc.exe copied." -ForegroundColor Green

if (Test-Path "$BinDir\pipette.exe") {
    Copy-Item "$BinDir\pipette.exe" "$Output\bin\" -Force
    Write-Host "  pipette.exe copied." -ForegroundColor Green
}

if (Test-Path "$BinDir\titrate_native.dll") {
    Copy-Item "$BinDir\titrate_native.dll" "$Output\bin\" -Force
    Write-Host "  titrate_native.dll copied." -ForegroundColor Green
}

# Copy LLVM DLL if present
$llvmDll = "$env:ProgramFiles\LLVM\bin\LLVM-C.dll"
if (Test-Path $llvmDll) {
    Copy-Item $llvmDll "$Output\bin\" -Force
    Write-Host "  LLVM-C.dll copied." -ForegroundColor Green
} else {
    Write-Host "  WARNING: LLVM-C.dll not found. Native compilation will not work." -ForegroundColor Yellow
    Write-Host "  Download from https://github.com/llvm/llvm-project/releases" -ForegroundColor Yellow
}

# Step 4: Copy standard library
Write-Host "`n[4/5] Copying standard library..." -ForegroundColor Yellow
Copy-Item "$ProjectRoot\lib\tt\*" "$Output\lib\tt\" -Recurse -Force
$trCount = (Get-ChildItem -Path "$Output\lib\tt" -Filter "*.tr" -Recurse).Count
Write-Host "  $trCount standard library files copied." -ForegroundColor Green

# Step 5: Copy examples and docs
Write-Host "`n[5/5] Copying examples and documentation..." -ForegroundColor Yellow
if (Test-Path "$ProjectRoot\examples") {
    Copy-Item "$ProjectRoot\examples\*.tr" "$Output\examples\" -Force -ErrorAction SilentlyContinue
    $exCount = (Get-ChildItem -Path "$Output\examples" -Filter "*.tr").Count
    Write-Host "  $exCount example files copied." -ForegroundColor Green
}

# Copy README
if (Test-Path "$ProjectRoot\README.md") {
    Copy-Item "$ProjectRoot\README.md" "$Output\" -Force
    Write-Host "  README.md copied." -ForegroundColor Green
}

# Copy AGENTS.md (syntax reference)
if (Test-Path "$ProjectRoot\AGENTS.md") {
    Copy-Item "$ProjectRoot\AGENTS.md" "$Output\" -Force
    Write-Host "  AGENTS.md (syntax reference) copied." -ForegroundColor Green
}

# Create a run script
$runScript = @"
@echo off
setlocal
set "TITRATE_HOME=%~dp0"
set "PATH=%TITRATE_HOME%\bin;%PATH%"
trc %*
endlocal
"@
Set-Content -Path "$Output\trc.bat" -Value $runScript
Write-Host "  trc.bat launcher created." -ForegroundColor Green

# Print summary
Write-Host "`n=== Package Complete ===" -ForegroundColor Cyan
Write-Host "Location: $Output" -ForegroundColor Gray
Write-Host "`nContents:" -ForegroundColor White
Get-ChildItem -Path $Output -Recurse -File | ForEach-Object {
    $relPath = $_.FullName.Replace($Output, "").TrimStart("\")
    $size = if ($_.Length -gt 1MB) { "$([math]::Round($_.Length / 1MB, 2)) MB" } elseif ($_.Length -gt 1KB) { "$([math]::Round($_.Length / 1KB, 2)) KB" } else { "$($_.Length) B" }
    Write-Host "  $relPath ($size)" -ForegroundColor Gray
}

Write-Host "`nUsage:" -ForegroundColor White
Write-Host "  $Output\trc.bat hello.tr" -ForegroundColor Gray
Write-Host "  $Output\bin\trc.exe --native --release hello.tr" -ForegroundColor Gray