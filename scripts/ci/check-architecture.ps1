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

# F-021: exactly one platform implementation. Global logging installation
# lives only in apps/ (crates emit via `tracing`; test-only local
# subscribers built with `finish()` install nothing global and are allowed).
# Configuration fingerprinting lives only in sitolo-config
# (`reference_fingerprint` in sitolo-security is a secret-reference
# correlator, not a config fingerprint, and is allowlisted).
$dupes = @(rg -n "try_init\(\)|set_global_default" crates -g "*.rs")
if ($dupes.Count -gt 0) { $dupes | ForEach-Object { Write-Output $_ }; Write-Output "ARCHITECTURE VIOLATION: logging installation outside apps/"; exit 1 }
# Column-0 match: only free-function constructors trip this. Method
# accessors such as `AppState::config_fingerprint` are indented and are
# safe views over the single canonical value.
$dupes = @(rg -n "^(pub )?fn (config_fingerprint|canonical_non_secret_config)\b" crates apps -g "*.rs" | Where-Object { $_ -notmatch "crates[/\\]sitolo-config[/\\]" })
if ($dupes.Count -gt 0) { $dupes | ForEach-Object { Write-Output $_ }; Write-Output "ARCHITECTURE VIOLATION: config fingerprint outside sitolo-config"; exit 1 }

if ($fail) {
    Write-Output "Architecture check FAILED"
    exit 1
}

Write-Output "Architecture check PASSED"