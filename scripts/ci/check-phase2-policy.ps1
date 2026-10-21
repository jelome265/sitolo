$ErrorActionPreference = "Stop"
$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

# Registries and runbooks are release evidence, not optional documentation.
$required = @(
  "docs/telemetry/events.yaml", "docs/telemetry/metrics.yaml", "docs/telemetry/redaction.yaml",
  "docs/runbooks/secret-compromise.md", "docs/runbooks/telemetry-blackout.md",
  "docs/runbooks/configuration-poisoning.md", "docs/runbooks/error-storm.md"
)
foreach ($path in $required) { if (-not (Test-Path $path)) { throw "PHASE 2 POLICY VIOLATION: missing $path" } }

# Process configuration is loaded once through sitolo-config. Security's
# development-only secret adapter is the reviewed exception.
$envReads = @(rg -n "std::env::(var|vars)" crates apps -g "*.rs" | Where-Object { $_ -notmatch "crates[/\\]sitolo-config[/\\]" -and $_ -notmatch "crates[/\\]sitolo-security[/\\]" })
if ($envReads.Count -gt 0) { $envReads | ForEach-Object { Write-Output $_ }; throw "PHASE 2 POLICY VIOLATION: direct environment read outside approved boundary" }

# No production code may use ad-hoc console output.
$console = @(rg -n "\b(println!|dbg!)" crates apps -g "*.rs" | Where-Object { $_ -notmatch "#\[cfg\(test\)\]" })
if ($console.Count -gt 0) { $console | ForEach-Object { Write-Output $_ }; throw "PHASE 2 POLICY VIOLATION: unstructured console output" }

# Runtime database identity is assembled only from typed, canonical fields.
# URL and generic connection-string aliases obscure secret boundaries.
$aliases = @(rg -n "\b(DATABASE_URL|DB_URL|POSTGRES_URL|PGURL|CONNECTION_STRING)\b" crates apps -g "*.rs")
if ($aliases.Count -gt 0) { $aliases | ForEach-Object { Write-Output $_ }; throw "PHASE 2 POLICY VIOLATION: database URL or connection-string alias in runtime source" }

Write-Output "Phase 2 policy check PASSED"
exit 0
