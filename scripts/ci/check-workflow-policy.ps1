$ErrorActionPreference = "Continue"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

$workflows = Join-Path $ROOT ".github\workflows"
if (-not (Test-Path $workflows)) {
    Write-Output "Workflow policy check PASSED (no workflow files present)"
    exit 0
}

$fail = $false

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

if ($fail) {
    Write-Output "Workflow policy check FAILED"
    exit 1
}

Write-Output "Workflow policy check PASSED"