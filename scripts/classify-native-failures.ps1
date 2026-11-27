<#
.SYNOPSIS
    Classify native harness failures in a .local log into root-cause categories.

.DESCRIPTION
    Reads a `stdlib_runtest_native` harness log (for example
    .local/native_dump_run.log), groups every failure block by its dominant
    root cause, and writes a Markdown table of counts and percentages to
    stdout. A failure block starts at a `[<path>.tr]` header and ends at the
    next header; cargo output and the test summary before the first header
    are ignored.

    Categories are ordered rules and the first matching rule wins:
    LNK1120 linker, unsupported call target, unsupported expression,
    unsupported assignment target, cast failures, crashes (exit codes),
    FAIL lines, other. The rules match the strings emitted by
    trc/tests/stdlib_runtest.rs, so a log produced with STDLIB_NATIVE_DUMP
    classifies cleanly.

.PARAMETER LogPath
    Path to a harness log. Defaults to the newest native*.log under .local/.

.EXAMPLE
    pwsh scripts/classify-native-failures.ps1
    pwsh scripts/classify-native-failures.ps1 .local/native_dump_run.log
    pwsh scripts/classify-native-failures.ps1 .local/native_dump_run.log > native-failures.md
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$LogPath
)

$ErrorActionPreference = "Stop"

$workspaceRoot = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($LogPath)) {
    $localRoot = Join-Path $workspaceRoot ".local"
    if (-not (Test-Path -LiteralPath $localRoot -PathType Container)) {
        throw "No .local directory found; pass -LogPath explicitly."
    }
    $newest = Get-ChildItem -LiteralPath $localRoot -Filter "native*.log" -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $newest) {
        throw "No native*.log found under $localRoot; pass -LogPath explicitly."
    }
    $LogPath = $newest.FullName
}

if (-not (Test-Path -LiteralPath $LogPath -PathType Leaf)) {
    throw "Harness log not found: $LogPath"
}

$headerRe = [regex]'^\[(?<path>.+\.tr)\]\s*$'

# Rule order is also the display order of the emitted table.
$rules = @(
    @{ Name = "LNK1120 linker";              Re = [regex]'LNK1120|LNK2019|unresolved external symbol' }
    @{ Name = "unsupported call target";     Re = [regex]'unsupported call target|unsupported static call' }
    @{ Name = "unsupported expression";      Re = [regex]'unsupported expression' }
    @{ Name = "unsupported assignment target"; Re = [regex]'unsupported assignment target' }
    @{ Name = "cast failures";               Re = [regex]'cast failed|cannot convert value of type' }
    @{ Name = "crashes (exit codes)";        Re = [regex]'native binary exited|stack overflow|access violation|LLVM ERROR' }
    @{ Name = "FAIL lines";                  Re = [regex]'^FAIL lines:' }
    @{ Name = "other";                       Re = $null }
)

$counts = @{}
foreach ($rule in $rules) {
    $counts[$rule.Name] = 0
}

function Add-Failure([string]$detail) {
    $category = "other"
    foreach ($rule in $rules) {
        if ($null -ne $rule.Re -and $rule.Re.IsMatch($detail)) {
            $category = $rule.Name
            break
        }
    }
    $counts[$category]++
}

$current = $null
foreach ($line in [System.IO.File]::ReadLines($LogPath)) {
    $match = $headerRe.Match($line)
    if ($match.Success) {
        if ($null -ne $current -and $current.Lines.Count -gt 0) {
            Add-Failure ([string]::Join("`n", $current.Lines))
        }
        $current = @{
            Path  = $match.Groups["path"].Value
            Lines = [System.Collections.Generic.List[string]]::new()
        }
        continue
    }
    if ($null -ne $current) {
        $current.Lines.Add($line)
    }
}
if ($null -ne $current -and $current.Lines.Count -gt 0) {
    Add-Failure ([string]::Join("`n", $current.Lines))
}

$total = 0
foreach ($rule in $rules) {
    $total += $counts[$rule.Name]
}

$invariant = [System.Globalization.CultureInfo]::InvariantCulture

Write-Host "# Native harness failure classification"
Write-Host ""
Write-Host "Source: $LogPath"
Write-Host ""
Write-Host "| Category | Count | Percent |"
Write-Host "|---|---:|---:|"
foreach ($rule in $rules) {
    $count = $counts[$rule.Name]
    $percent = if ($total -gt 0) { 100.0 * $count / $total } else { 0.0 }
    Write-Host ("| {0} | {1} | {2}% |" -f $rule.Name, $count, $percent.ToString("F1", $invariant))
}
$totalPercent = if ($total -gt 0) { 100.0 } else { 0.0 }
Write-Host ("| **Total** | **{0}** | **{1}%** |" -f $total, $totalPercent.ToString("F1", $invariant))

if ($total -eq 0) {
    Write-Host ""
    Write-Host "No failure blocks found; check the log format or pass -LogPath."
}
