$ErrorActionPreference = "Stop"
$bash = Get-Command bash -ErrorAction Stop
& $bash.Source -lc "./scripts/ci/check-icm-workspace"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Output "ICM PowerShell entrypoint PASSED: delegated to canonical POSIX verifier"
