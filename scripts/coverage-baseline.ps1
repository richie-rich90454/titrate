<#
.SYNOPSIS
    Generate a coverage baseline for the Titrate Rust toolchain.

.DESCRIPTION
    Runs `pipette coverage` against the workspace and captures the resulting
    per-file summary. On the first run the summary is copied to
    coverage-baseline.txt so subsequent runs can be diffed against it.

    This script is the entry point for Phase 6 Task 6.1's baseline capture.
    It requires an external coverage tool (cargo-tarpaulin or grcov) to be
    installed. See docs/guide/coverage.md for setup instructions.

.PARAMETER Native
    Also instrument the native (LLVM) test binaries. Equivalent to passing
    --native to `pipette coverage`.

.PARAMETER ForceBaseline
    Overwrite coverage-baseline.txt even if it already exists.

.EXAMPLE
    .\scripts\coverage-baseline.ps1
    .\scripts\coverage-baseline.ps1 -Native
    .\scripts\coverage-baseline.ps1 -ForceBaseline
#>
[CmdletBinding()]
param(
    [switch]$Native,
    [switch]$ForceBaseline
)

$ErrorActionPreference = "Stop"

# Resolve the workspace root (the directory containing this script's parent).
$workspaceRoot = Split-Path -Parent $PSScriptRoot
Set-Location $workspaceRoot

Write-Host "Titrate coverage baseline" -ForegroundColor Cyan
Write-Host "  workspace: $workspaceRoot"
Write-Host "  native:    $(if ($Native) { 'yes' } else { 'no' })"
Write-Host ""

# Ensure pipette is built. We need the binary on PATH or invoke via cargo.
$pipetteArgs = @("run", "--quiet", "-p", "pipette", "--", "coverage")
if ($Native) {
    $pipetteArgs += "--native"
}

Write-Host "Running: cargo $($pipetteArgs -join ' ')" -ForegroundColor DarkGray
& cargo @pipetteArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "pipette coverage failed (exit code $LASTEXITCODE)."
    exit $LASTEXITCODE
}

$summaryPath = Join-Path $workspaceRoot "coverage-summary.txt"
$baselinePath = Join-Path $workspaceRoot "coverage-baseline.txt"

if (-not (Test-Path $summaryPath)) {
    Write-Warning "coverage-summary.txt was not produced; nothing to baseline."
    exit 0
}

if ((-not (Test-Path $baselinePath)) -or $ForceBaseline) {
    Copy-Item $summaryPath $baselinePath -Force
    Write-Host ""
    Write-Host "Baseline written to $baselinePath" -ForegroundColor Green
}
else {
    Write-Host ""
    Write-Host "Baseline already exists at $baselinePath" -ForegroundColor Yellow
    Write-Host "Diff against the latest run:" -ForegroundColor Yellow
    Write-Host "  git diff --no-index $baselinePath $summaryPath"
}

Write-Host ""
Write-Host "Current coverage state:" -ForegroundColor Cyan
Get-Content $summaryPath | ForEach-Object { Write-Host "  $_" }
