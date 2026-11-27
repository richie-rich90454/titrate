<#
.SYNOPSIS
    Run the fixed-workload comparison benchmarks and print every measured
    result, including slowdowns ("losses") against the Rust baseline.

.DESCRIPTION
    Four kernels (sum of squares, water-box energy, container iteration,
    string processing) run in three implementations over identical inputs:
    Titrate bytecode, Titrate native (LLVM), and an equivalent Rust function.
    Outputs are checked for equality before timing. A backend whose output was
    not verified, or whose result file is from an earlier run, is reported as
    "not measured" rather than guessed.

    Numbers are printed to the console only. This script never writes results
    into documentation and never publishes them. Record the printed
    environment alongside any result you choose to share.

.PARAMETER Smoke
    Verify all kernel outputs without running or asserting any timings. Does
    not require LLVM or the native runtime library (CI-safe).

.PARAMETER CompileOnly
    Compile the benchmark target only (`cargo bench --no-run`).

.PARAMETER NoNative
    Skip building and measuring the Titrate native backend.

.EXAMPLE
    .\scripts\bench-comparison.ps1
    .\scripts\bench-comparison.ps1 -Smoke
    .\scripts\bench-comparison.ps1 -CompileOnly
    .\scripts\bench-comparison.ps1 -NoNative
#>
[CmdletBinding()]
param(
    [switch]$Smoke,
    [switch]$CompileOnly,
    [switch]$NoNative
)

$ErrorActionPreference = "Stop"
$workspaceRoot = Split-Path -Parent $PSScriptRoot
Set-Location $workspaceRoot

function Invoke-Cargo([string[]]$CargoArgs) {
    Write-Host "cargo $($CargoArgs -join ' ')" -ForegroundColor DarkGray
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($CargoArgs -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Restore-Env([string]$Name, $Value) {
    if ($null -eq $Value) {
        Remove-Item "Env:$Name" -ErrorAction SilentlyContinue
    }
    else {
        Set-Item "Env:$Name" -Value $Value
    }
}

function Convert-ToNs($Estimate) {
    $value = [double]$Estimate.estimate
    switch ("$($Estimate.unit)") {
        "ns" { return $value }
        "us" { return $value * 1000.0 }
        "ms" { return $value * 1000000.0 }
        "s" { return $value * 1000000000.0 }
        default { return $value }
    }
}

function Read-Estimate([string]$Group, [string]$Backend) {
    $path = Join-Path $workspaceRoot "target/criterion/$Group/$Backend/new/estimates.json"
    if (-not (Test-Path $path)) {
        return $null
    }
    $item = Get-Item $path
    # Reject results left over from a previous invocation.
    if ($item.LastWriteTime -lt $script:runStarted) {
        return $null
    }
    $json = Get-Content $path -Raw | ConvertFrom-Json
    if ($null -eq $json.mean) {
        return $null
    }
    return (Convert-ToNs $json.mean)
}

function Format-Loss($Value, $Rust) {
    if ($null -eq $Value -or $null -eq $Rust -or $Rust -le 0) {
        return "not measured"
    }
    $ratio = $Value / $Rust
    $percent = ($ratio - 1.0) * 100.0
    return ("{0:F2}x ({1:F1}%)" -f $ratio, $percent)
}

$benchArgs = @("bench", "-p", "trc", "--bench", "comparison_kernels")

if ($CompileOnly) {
    Invoke-Cargo ($benchArgs + @("--no-run"))
    Write-Host "Compile-only check passed; no timings were run."
    exit 0
}

$savedSmoke = $env:TITRATE_BENCH_SMOKE
$savedNoNative = $env:TITRATE_BENCH_NO_NATIVE
try {
    if ($Smoke) {
        $env:TITRATE_BENCH_SMOKE = "1"
        $env:TITRATE_BENCH_NO_NATIVE = "1"
    }
    elseif ($NoNative) {
        $env:TITRATE_BENCH_NO_NATIVE = "1"
    }
    else {
        Invoke-Cargo @("build", "-p", "trc", "--release")
        Invoke-Cargo @("build", "-p", "titrate_native", "--release")
    }

    Write-Host ""
    Write-Host "Environment (record this with any result you share):" -ForegroundColor Cyan
    $git = Get-Command git -ErrorAction SilentlyContinue
    $commit = if ($null -ne $git) { (& git rev-parse HEAD 2>$null) } else { "git unavailable" }
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    $toolchain = if ($null -ne $rustc) { (& rustc --version) } else { "rustc unavailable" }
    Write-Host "  commit:  $commit"
    Write-Host "  rustc:   $toolchain"
    Write-Host "  OS:      $([System.Environment]::OSVersion.VersionString)"
    $cpu = $env:PROCESSOR_IDENTIFIER
    if ([string]::IsNullOrEmpty($cpu)) {
        $cpu = "unknown (inspect /proc/cpuinfo)"
    }
    Write-Host "  CPU:     $cpu"

    $script:runStarted = Get-Date
    Invoke-Cargo $benchArgs
}
finally {
    Restore-Env "TITRATE_BENCH_SMOKE" $savedSmoke
    Restore-Env "TITRATE_BENCH_NO_NATIVE" $savedNoNative
}

if ($Smoke) {
    Write-Host ""
    Write-Host "Smoke check passed; no timings were run or asserted." -ForegroundColor Green
    exit 0
}

$groups = @("sum_of_squares", "water_box", "container_iteration", "string_processing")
$backends = @("titrate_bytecode", "titrate_native", "rust")

Write-Host ""
Write-Host "Comparison summary (mean ns/iter from this run only):" -ForegroundColor Cyan
$header = "{0,-22} {1,14} {2,14} {3,14} {4,20} {5,20}"
Write-Host ($header -f "kernel", "rust", "bytecode", "native", "bytecode loss", "native loss")
Write-Host ("-" * 108)

foreach ($group in $groups) {
    $row = @{}
    foreach ($backend in $backends) {
        $row[$backend] = Read-Estimate $group $backend
    }
    $rust = $row["rust"]
    $rustText = if ($null -eq $rust) { "not measured" } else { $rust.ToString("F2") }
    $bytecodeText = if ($null -eq $row["titrate_bytecode"]) {
        "not measured"
    }
    else {
        $row["titrate_bytecode"].ToString("F2")
    }
    $nativeText = if ($null -eq $row["titrate_native"]) {
        "not measured"
    }
    else {
        $row["titrate_native"].ToString("F2")
    }
    Write-Host ($header -f $group, $rustText, $bytecodeText, $nativeText,
        (Format-Loss $row["titrate_bytecode"] $rust),
        (Format-Loss $row["titrate_native"] $rust))
}

Write-Host ""
Write-Host "Loss = (backend mean / Rust mean - 1) x 100. A backend is shown as" -ForegroundColor DarkGray
Write-Host "'not measured' when its output was not verified or no new result was written." -ForegroundColor DarkGray
