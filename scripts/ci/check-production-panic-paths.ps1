$ErrorActionPreference = "Stop"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

$patterns = @(
    '\.(unwrap|expect)\s*\(',
    '\bpanic!\s*\(',
    '\bunreachable!\s*\(',
    '\btodo!\s*\(',
    '\bunimplemented!\s*\('
)

$offenders = @()
$files = Get-ChildItem -Path "$ROOT\crates","$ROOT\apps" -Recurse -Filter *.rs -File

foreach ($file in $files) {
    $relative = $file.FullName.Substring($ROOT.Length + 1).Replace('\','/')
    if ($relative -match '(^|/)tests?(/|$)' -or $relative -match '(^|/)benches(/|$)') {
        continue
    }

    $text = Get-Content -Raw -LiteralPath $file.FullName
    $production = $text
    $cfgTestIndex = $production.IndexOf('#[cfg(test)]', [System.StringComparison]::Ordinal)
    if ($cfgTestIndex -ge 0) {
        $production = $production.Substring(0, $cfgTestIndex)
    }

    $lineNumber = 0
    foreach ($line in ($production -split "`r?`n")) {
        $lineNumber++
        foreach ($pattern in $patterns) {
            if ($line -match $pattern) {
                $offenders += "${relative}:${lineNumber}:$($line.Trim())"
                break
            }
        }
    }
}

if ($offenders.Count -gt 0) {
    Write-Output "Production panic-capable Rust paths detected:"
    $offenders | ForEach-Object { Write-Output "  $_" }
    Write-Output "Test-only assertions are excluded. Replace production unwrap/expect/panic paths with typed failure or explicit fail-closed handling."
    exit 1
}

Write-Output "Production panic-path policy passed: no unwrap/expect/panic/unreachable/todo/unimplemented macros detected outside tests."
