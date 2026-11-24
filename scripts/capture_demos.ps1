param([switch]$Check)

# Regenerate the terminal-style gallery SVGs from real bytecode-VM output.
# Headless: runs trc.exe as a redirected child process, writes files, exits.

# PowerShell leaves "--check" in $args; accept it alongside "-Check".
foreach ($a in $args) { if ($a -ieq '--check') { $Check = $true } }

$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
$Trc = Join-Path $Root 'target/debug/trc.exe'
$Assets = Join-Path $Root 'docs/assets'
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$MaxLines = 40

if (-not (Test-Path $Trc)) {
    throw "trc.exe not found at $Trc. Build with scripts/build.ps1 first."
}

function Convert-Lines {
    param([string]$Text)
    $t = [regex]::Replace($Text, "`e\[[0-9;?]*[ -/]*[@-~]", '')
    $t = $t.Replace("`r`n", "`n").Replace("`r", "`n")
    @($t.TrimEnd("`n") -split "`n")
}

function Invoke-Demo {
    param([string]$Source, [string[]]$ExtraArgs = @())
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Trc
    $psi.WorkingDirectory = $Root
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.ArgumentList.Add($Source)
    foreach ($a in $ExtraArgs) { $psi.ArgumentList.Add($a) }
    $p = [System.Diagnostics.Process]::Start($psi)
    $stdout = $p.StandardOutput.ReadToEnd()
    $stderr = $p.StandardError.ReadToEnd()
    $p.WaitForExit()
    if ($p.ExitCode -ne 0) {
        throw "trc failed for $Source (exit $($p.ExitCode)): $stderr"
    }
    Convert-Lines $stdout
}

function Limit-Lines {
    param([string[]]$Lines)
    if ($Lines.Count -le $MaxLines) { return $Lines }
    $result = [System.Collections.Generic.List[string]]::new()
    for ($i = 0; $i -lt $MaxLines; $i++) { $result.Add($Lines[$i]) }
    $result.Add([char]0x2026 + ' (truncated)')
    $result.ToArray()
}

function Escape-Xml {
    param([string]$Text)
    [System.Security.SecurityElement]::Escape($Text)
}

function New-TerminalSvg {
    param([string]$Title, [string[]]$Lines)

    $width = 960
    $barH = 40
    $lineH = 22
    $padX = 24
    $padY = 20
    $height = $barH + $padY * 2 + $Lines.Count * $lineH
    $safeTitle = Escape-Xml $Title

    $o = [System.Collections.Generic.List[string]]::new()
    $o.Add("<svg xmlns='http://www.w3.org/2000/svg' width='$width' height='$height' viewBox='0 0 $width $height' role='img' aria-label='$safeTitle terminal output'>")
    $o.Add("  <title>$safeTitle terminal output</title>")
    $o.Add("  <clipPath id='card'><rect x='1' y='1' width='958' height='$($height - 2)' rx='12'/></clipPath>")
    $o.Add("  <rect x='1' y='1' width='958' height='$($height - 2)' rx='12' fill='#0d1117' stroke='#30363d'/>")
    $o.Add("  <g clip-path='url(#card)'>")
    $o.Add("    <rect x='1' y='1' width='958' height='39' fill='#161b22'/>")
    $o.Add("    <line x1='1' y1='40' x2='959' y2='40' stroke='#30363d'/>")
    $o.Add("    <circle cx='24' cy='20' r='6' fill='#ff5f56'/>")
    $o.Add("    <circle cx='44' cy='20' r='6' fill='#ffbd2e'/>")
    $o.Add("    <circle cx='64' cy='20' r='6' fill='#27c93f'/>")
    $o.Add("    <text x='88' y='25' fill='#c9d1d9' font-family='Consolas, Menlo, monospace' font-size='14'>$safeTitle</text>")
    $o.Add("    <text x='936' y='25' text-anchor='end' fill='#6e7681' font-family='Consolas, Menlo, monospace' font-size='12'>trc / bytecode VM</text>")
    $o.Add("    <text x='$padX' y='$($barH + $padY + 12)' xml:space='preserve' fill='#e6edf3' font-family='Consolas, Menlo, monospace' font-size='14'>")
    $dy = 0
    foreach ($line in $Lines) {
        $o.Add("      <tspan x='$padX' dy='$dy'>$(Escape-Xml $line)</tspan>")
        $dy = $lineH
    }
    $o.Add('    </text>')
    $o.Add('  </g>')
    $o.Add('</svg>')
    ($o -join "`n") + "`n"
}

function Add-StatTile {
    param([System.Collections.Generic.List[string]]$List, [int]$X, [int]$Y, [int]$Value, [string]$Label)
    $List.Add("    <rect x='$X' y='$Y' width='288' height='96' rx='10' fill='#161b22' stroke='#30363d'/>")
    $List.Add("    <text x='$($X + 20)' y='$($Y + 46)' fill='#58a6ff' font-family='Consolas, Menlo, monospace' font-size='32'>$Value</text>")
    $List.Add("    <text x='$($X + 20)' y='$($Y + 72)' fill='#8b949e' font-family='Consolas, Menlo, monospace' font-size='13'>$(Escape-Xml $Label)</text>")
}

function New-StatsSvg {
    param([int]$Modules, [int]$Tests, [int]$Demos, [int]$Examples, [int]$Natives)

    $o = [System.Collections.Generic.List[string]]::new()
    $o.Add("<svg xmlns='http://www.w3.org/2000/svg' width='960' height='420' viewBox='0 0 960 420' role='img' aria-label='Titrate repository statistics'>")
    $o.Add("  <title>Titrate repository statistics</title>")
    $o.Add("  <rect x='1' y='1' width='958' height='418' rx='12' fill='#0d1117' stroke='#30363d'/>")
    $o.Add("  <text x='32' y='52' fill='#e6edf3' font-family='Consolas, Menlo, monospace' font-size='24'>Titrate at a glance</text>")
    $o.Add("  <text x='32' y='76' fill='#6e7681' font-family='Consolas, Menlo, monospace' font-size='13'>Counts computed from the source tree</text>")

    Add-StatTile $o 32  100 $Modules  'standard library modules'
    Add-StatTile $o 336 100 $Tests    'stdlib test programs'
    Add-StatTile $o 640 100 $Demos    'demo programs'
    Add-StatTile $o 32  212 $Examples 'example programs'
    Add-StatTile $o 336 212 $Natives  'registered VM natives'

    $o.Add("    <rect x='32' y='328' width='896' height='68' rx='10' fill='#161b22' stroke='#30363d'/>")
    $o.Add("    <text x='52' y='350' fill='#6e7681' font-family='Consolas, Menlo, monospace' font-size='11'>VERIFIED IN PRIOR RUNS</text>")
    $o.Add("    <text x='52'  y='378' fill='#7ee787' font-family='Consolas, Menlo, monospace' font-size='15'>123/123 native sample parity</text>")
    $o.Add("    <text x='352' y='378' fill='#7ee787' font-family='Consolas, Menlo, monospace' font-size='15'>88 WASM tests</text>")
    $o.Add("    <text x='652' y='378' fill='#7ee787' font-family='Consolas, Menlo, monospace' font-size='15'>3 backends</text>")
    $o.Add('</svg>')
    ($o -join "`n") + "`n"
}

$defs = @(
    @{ Name = 'demo-hello';         Title = 'hello.tr';         Source = 'examples/hello.tr';       Extra = @() },
    @{ Name = 'demo-fibonacci';     Title = 'fibonacci.tr';     Source = 'examples/fibonacci.tr';   Extra = @() },
    @{ Name = 'demo-collatz';       Title = 'collatz.tr';       Source = 'demos/collatz.tr';        Extra = @() },
    @{ Name = 'demo-caesar-cipher'; Title = 'caesar_cipher.tr'; Source = 'demos/caesar_cipher.tr';  Extra = @() },
    @{ Name = 'demo-csv-to-json';   Title = 'csv_to_json.tr';   Source = 'demos/csv_to_json.tr';    Extra = @('archive/old_test_files/data.csv') }
)

$outputs = [ordered]@{}
foreach ($d in $defs) {
    $lines = Limit-Lines (Invoke-Demo $d.Source $d.Extra)
    $outputs[$d.Name] = New-TerminalSvg $d.Title $lines
}

$modules = @(Get-ChildItem -Path (Join-Path $Root 'lib/tt') -Recurse -File -Filter '*.tr').Count
$tests = @(Get-ChildItem -Path (Join-Path $Root 'tests/stdlib/src/tests') -Recurse -File -Filter '*.tr').Count
$demos = @(Get-ChildItem -Path (Join-Path $Root 'demos') -File -Filter '*.tr').Count
$examples = @(Get-ChildItem -Path (Join-Path $Root 'examples') -File -Filter '*.tr').Count
$lookup = Get-Content (Join-Path $Root 'trc/src/bytecode/vm/natives/lookup.rs') -Raw
$natives = [regex]::Matches($lookup, '"[^"]+"\s*=>\s*Some\(').Count
$outputs['site-stats'] = New-StatsSvg $modules $tests $demos $examples $natives

if (-not $Check) {
    if (-not (Test-Path $Assets)) { New-Item -ItemType Directory -Path $Assets -Force | Out-Null }
}

$stale = @()
foreach ($name in $outputs.Keys) {
    $content = $outputs[$name]
    $null = [xml]$content
    $path = Join-Path $Assets "$name.svg"
    if ($Check) {
        if (-not (Test-Path $path)) { $stale += "$name.svg (missing)"; continue }
        $existing = [System.IO.File]::ReadAllText($path).Replace("`r`n", "`n")
        if ($existing -cne $content) { $stale += "$name.svg (stale)" }
    } else {
        [System.IO.File]::WriteAllText($path, $content, $Utf8NoBom)
        Write-Host "wrote docs/assets/$name.svg"
    }
}

if ($Check) {
    if ($stale.Count -gt 0) {
        Write-Error "Stale gallery assets: $($stale -join ', ')"
        exit 1
    }
    Write-Host 'Gallery assets are up to date.'
    exit 0
}
