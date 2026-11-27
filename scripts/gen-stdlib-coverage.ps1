<#
.SYNOPSIS
    Generate docs/stdlib-coverage.json from the Titrate standard library source.

.DESCRIPTION
    Scans lib/tt/**/*.tr for public fn/class/interface/enum declarations and
    writes a deterministic inventory grouped by domain. Counts are always
    derived from the files found on disk; nothing is hardcoded except the
    directory-to-domain mapping and the reference-list labels from ROADMAP.md
    section 21.

    The report is an inventory only. "implemented" means the declaration is
    present in source. It is not evidence of correctness, tests, backend
    parity, or parity with any reference list.

.EXAMPLE
    pwsh scripts/gen-stdlib-coverage.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$workspaceRoot = Split-Path -Parent $PSScriptRoot
$sourceRoot = Join-Path $workspaceRoot "lib/tt"
$outputPath = Join-Path $workspaceRoot "docs/stdlib-coverage.json"

if (-not (Test-Path -LiteralPath $sourceRoot -PathType Container)) {
    throw "Standard library source root not found: $sourceRoot"
}

# Domain order matches ROADMAP.md section 21. Reference strings are copied
# from that table; they are labels, not counts or claims of parity.
$domainConfig = [ordered]@{
    "core-util" = @{
        Name       = "Core/util"
        Reference  = "CPython stdlib modules + Java SE packages"
        Directories = @(
            "algo", "algorithms", "argparse", "assay", "assert", "binary",
            "compiler", "config", "contextlib", "copy", "crypto", "crypto2",
            "csv", "dataclass", "debug", "decimal", "docs", "encoding",
            "fractions", "hft", "i18n", "json", "lang", "math",
            "packaging", "pprint", "profile", "random", "regex", "secrets",
            "serialization", "string", "test", "text", "textwrap", "time",
            "tooling", "uuid", "xml"
        )
    }
    "collections" = @{
        Name       = "Collections"
        Reference  = "Java Collections Framework + Python collections/itertools/functools"
        Directories = @("bisect", "functools", "heapq", "itertools", "operator", "util")
    }
    "data-science" = @{
        Name       = "Data/science"
        Reference  = "NumPy top-level + pandas-like tabular"
        Directories = @(
            "bio", "calculus", "chem", "data", "db", "finance", "geom",
            "linalg", "materials", "nlp", "numeric", "physics", "sigproc",
            "sim", "statistics", "units"
        )
    }
    "network-async" = @{
        Name       = "Network/async"
        Reference  = "Python http/urllib/asyncio/selectors + java.net"
        Directories = @("async", "concurrent", "html", "mail", "net")
    }
    "system-os" = @{
        Name       = "System/OS"
        Reference  = "Python os/sys/subprocess/logging + java.nio"
        Directories = @(
            "compression", "ffi", "file", "io", "logging", "os", "subprocess",
            "sys", "tempfile"
        )
    }
    "graphics" = @{
        Name       = "Graphics"
        Reference  = "raylib/SDL-style surface"
        Directories = @("graphics", "image", "ui")
    }
    "audio" = @{
        Name       = "Audio"
        Reference  = "WebAudio/SDL_mixer-style surface"
        Directories = @("audio")
    }
    "ml" = @{
        Name       = "ML"
        Reference  = "scikit-learn estimator API + tensor/autograd surface"
        Directories = @("ml")
    }
}

$directoryToDomain = @{}
foreach ($id in $domainConfig.Keys) {
    foreach ($directory in $domainConfig[$id].Directories) {
        if ($directoryToDomain.ContainsKey($directory)) {
            throw "Directory '$directory' is mapped to both '$($directoryToDomain[$directory])' and '$id'."
        }
        $directoryToDomain[$directory] = $id
    }
}

$relativePaths = [System.Collections.Generic.List[string]]::new()
$fileByRelative = @{}
foreach ($file in Get-ChildItem -LiteralPath $sourceRoot -Filter "*.tr" -File -Recurse) {
    $relative = $file.FullName.Substring($sourceRoot.Length + 1).Replace("\", "/")
    $relativePaths.Add($relative)
    $fileByRelative[$relative] = $file
}
$relativePaths.Sort([System.StringComparer]::Ordinal)
$files = @($relativePaths | ForEach-Object { $fileByRelative[$_] })

$stats = @{}
foreach ($id in $domainConfig.Keys) {
    $stats[$id] = @{
        ModuleCount = 0
        Apis        = [System.Collections.Generic.List[object]]::new()
    }
}

$declaration = [regex]'^\s*public\s+(?<kind>fn|class|interface|enum)\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)'
$totalApis = 0

foreach ($file in $files) {
    $relative = $file.FullName.Substring($sourceRoot.Length + 1).Replace("\", "/")
    $directory = $relative.Split("/")[0]
    if (-not $directoryToDomain.ContainsKey($directory)) {
        throw "No domain mapping for directory '$directory' (from $relative). Update the mapping in this script."
    }
    $domainId = $directoryToDomain[$directory]
    $stats[$domainId].ModuleCount++

    $moduleName = "tt::" + $relative.Substring(0, $relative.Length - 3).Replace("/", "::")
    $lineNumber = 0
    foreach ($line in [System.IO.File]::ReadLines($file.FullName)) {
        $lineNumber++
        $match = $declaration.Match($line)
        if (-not $match.Success) { continue }

        $api = [ordered]@{
            name         = $match.Groups["name"].Value
            kind         = $match.Groups["kind"].Value
            module       = $moduleName
            source       = "lib/tt/$relative"
            line         = $lineNumber
            status       = "implemented"
            referenceList = $domainConfig[$domainId].Reference
        }
        $stats[$domainId].Apis.Add($api)
        $totalApis++
    }
}

$domainReports = [System.Collections.Generic.List[object]]::new()
foreach ($id in $domainConfig.Keys) {
    $config = $domainConfig[$id]
    $domainReports.Add([ordered]@{
        id           = $id
        name         = $config.Name
        referenceList = $config.Reference
        moduleCount  = $stats[$id].ModuleCount
        apiCount     = $stats[$id].Apis.Count
        apis         = $stats[$id].Apis
    })
}

$report = [ordered]@{
    schemaVersion = 1
    generator     = "scripts/gen-stdlib-coverage.ps1"
    sourceRoot    = "lib/tt"
    disclaimer    = "Source-derived inventory of public declarations under lib/tt. Status 'implemented' means a declaration exists in source; it is not evidence of behavioral correctness, test coverage, cross-backend parity, or parity with any reference list. No Java, Python, or NumPy parity is claimed."
    totals        = [ordered]@{
        modules = $files.Count
        apis    = $totalApis
    }
    domains       = $domainReports
}

$json = $report | ConvertTo-Json -Depth 8
[System.IO.File]::WriteAllText(
    $outputPath,
    $json + "`n",
    [System.Text.UTF8Encoding]::new($false)
)

Write-Host "Wrote $outputPath"
Write-Host "  modules: $($files.Count)"
Write-Host "  apis:    $totalApis"
foreach ($id in $domainConfig.Keys) {
    Write-Host ("  {0,-14} modules={1,-4} apis={2}" -f $id, $stats[$id].ModuleCount, $stats[$id].Apis.Count)
}
