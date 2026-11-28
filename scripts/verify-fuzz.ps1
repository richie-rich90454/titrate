[CmdletBinding()]
param(
    [ValidateRange(1, 1000000)]
    [int]$Iterations = 64,
    [string]$Seed = "5068056299717280529",
    [string]$OutFile = ""
)

# CI gate for the deterministic fuzz harnesses. Every harness prints one
# TRC_FUZZ_JSON line; this script only assembles pass/fail status into a JSON
# file. No counts, timings, or other measured numbers are published.
$ErrorActionPreference = "Stop"

$root = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$manifest = Join-Path $root "trc/fuzz/Cargo.toml"
if (-not (Test-Path -LiteralPath $manifest)) {
    throw "missing fuzz harness manifest: $manifest"
}

if ([string]::IsNullOrWhiteSpace($OutFile)) {
    $OutFile = Join-Path $root "target/verification/fuzz-summary.json"
}
$destination = Split-Path -Parent $OutFile
if (-not (Test-Path -LiteralPath $destination)) {
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
}

# Share the workspace target directory so the gate reuses the already-built
# trc and titrate_native artifacts instead of rebuilding them from scratch.
$env:CARGO_TARGET_DIR = Join-Path $root "target"

$names = @("native_args", "expressions", "module_graphs", "differential")
$harnesses = @{}
$failed = $false

foreach ($name in $names) {
    $lines = & cargo run --quiet --manifest-path $manifest --bin trc-fuzz -- --case $name --iterations $Iterations --seed $Seed 2>&1
    $code = $LASTEXITCODE

    $jsonLine = $null
    foreach ($line in $lines) {
        $text = [string]$line
        if ($text.StartsWith("TRC_FUZZ_JSON:")) {
            $jsonLine = $text
            break
        }
    }
    if ($null -eq $jsonLine) {
        throw "harness '$name' produced no machine-readable result (exit $code): $($lines -join ' ')"
    }

    $payload = $jsonLine.Substring("TRC_FUZZ_JSON:".Length).Trim()
    $result = $payload | ConvertFrom-Json
    if ($code -ne 0 -or $result.status -ne "pass") {
        $failed = $true
        $message = "harness exited with code $code"
        if ($result.error) {
            $message = [string]$result.error
        }
        $harnesses[$name] = [ordered]@{
            status = "fail"
            error  = $message
        }
    } else {
        $harnesses[$name] = [ordered]@{ status = "pass" }
    }
}

$status = if ($failed) { "fail" } else { "pass" }
$summary = [ordered]@{
    schema    = "titrate-fuzz-summary-v1"
    status    = $status
    harnesses = $harnesses
}
$json = $summary | ConvertTo-Json -Depth 8 -Compress
[System.IO.File]::WriteAllText($OutFile, $json + [Environment]::NewLine)

Write-Host "fuzz summary written to $OutFile"
if ($failed) {
    exit 1
}
