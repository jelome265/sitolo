$ErrorActionPreference = "Continue"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

# Crates that must not depend on infrastructure or provider machinery.
$PureCrates = @("sitolo-domain")

# Forbidden infrastructure/provider crate-name tokens in the pure layers
# (Phase 1 specification, section 6). Extend with care.
$Forbidden = '\b(axum|sqlx|redis|reqwest|hyper|tokio-postgres|ureq)\b'

$fail = $false
foreach ($crate in $PureCrates) {
    $tree = @(& cargo tree --locked -p $crate 2>&1)
    if ($LASTEXITCODE -ne 0) { Write-Output "cargo tree failed for $crate"; exit $LASTEXITCODE }
    $matches = $tree | Select-String -Pattern $Forbidden -AllMatches
    if ($matches) {
        Write-Output "ARCHITECTURE VIOLATION: $crate depends on a forbidden infrastructure crate:"
        $matches | ForEach-Object { Write-Output $_.Line }
        $fail = $true
    }
}

if ($fail) {
    Write-Output "Architecture check FAILED"
    exit 1
}

Write-Output "Architecture check PASSED"