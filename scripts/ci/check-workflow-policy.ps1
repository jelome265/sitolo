$ErrorActionPreference = "Continue"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

$workflows = Join-Path $ROOT ".github\workflows"
if (-not (Test-Path $workflows)) {
    Write-Output "Workflow policy check PASSED (no workflow files present)"
    exit 0
}

$fail = $false

# Phase 1 requires the trusted artifact and promotion paths to exist. Database
# harness enforcement is intentionally excluded by the approved scope exception.
foreach ($required in @("artifact.yml", "release.yml", "integration.yml")) {
    if (-not (Test-Path (Join-Path $workflows $required))) {
        Write-Output "POLICY VIOLATION: required Phase 1 workflow missing: $required"
        $fail = $true
    }
}

# Third-party actions must be pinned to an immutable 40-character commit SHA.
$MutableUses = '(uses:\s+[A-Za-z0-9._/-]+@(v[0-9]+|stable|latest|nightly|master|main|beta|alpha|v[0-9]+\.[0-9]+))'
$ElevatedEvents = 'pull_request_target|issue_comment|workflow_run|repository_dispatch'
$SecretLint = 'secret_password|secret_token|access_key|private_key|CREDENTIALS'

foreach ($f in Get-ChildItem -Path $workflows -Filter *.yml) {
    $content = Get-Content -LiteralPath $f.FullName
    $bad = $content | Select-String -Pattern $MutableUses
    if ($bad) {
        Write-Output "POLICY VIOLATION in $($f.Name): third-party actions must be pinned to full immutable commit SHAs."
        $bad | ForEach-Object { Write-Output ("    " + $_.Line.Trim()) }
        $fail = $true
    }
    $events = $content | Select-String -Pattern $ElevatedEvents
    if ($events) {
        Write-Output "POLICY VIOLATION in $($f.Name): elevated-trust workflow events require explicit justification and review."
        $events | ForEach-Object { Write-Output ("    " + $_.Line.Trim()) }
        $fail = $true
    }
    $secrets = $content | Select-String -Pattern $SecretLint
    if ($secrets) {
        Write-Output "POLICY NOTICE in $($f.Name): review secret references in workflow files before merge."
        $secrets | ForEach-Object { Write-Output ("    " + $_.Line.Trim()) }
    }
}

if (Test-Path (Join-Path $workflows "artifact.yml")) {
    $artifact = Get-Content -Raw (Join-Path $workflows "artifact.yml")
    if ($artifact -notmatch 'attestations:\s*write' -or $artifact -notmatch 'id-token:\s*write') {
        Write-Output "POLICY VIOLATION in artifact.yml: provenance permissions are required."
        $fail = $true
    }
}
if (Test-Path (Join-Path $workflows "release.yml")) {
    $release = Get-Content -Raw (Join-Path $workflows "release.yml")
    if ($release -notmatch 'environment:\s*production' -or $release -match 'pull_request:') {
        Write-Output "POLICY VIOLATION in release.yml: promotion must be protected and cannot run for pull requests."
        $fail = $true
    }
}

if ($fail) {
    Write-Output "Workflow policy check FAILED"
    exit 1
}

Write-Output "Workflow policy check PASSED"
