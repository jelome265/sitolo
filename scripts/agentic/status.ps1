$ErrorActionPreference = "Stop"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

$WORKSPACE = "workspaces\sitolo-engineering"
$STAGES = @("01-select","02-research","03-investigate","04-plan","05-implement","06-audit","07-remediate","08-verify","09-deliver")

Write-Output "Pipeline Status: Sitolo Engineering"
Write-Output ""

foreach ($stage in $STAGES) {
  $output = "$WORKSPACE\stages\$stage\output"
  $files = @()
  if (Test-Path $output -PathType Container) {
    $files = @(Get-ChildItem $output -File | Where-Object { $_.Name -ne ".gitkeep" } | Sort-Object Name)
  }

  if ($files.Count -gt 0) {
    Write-Output ("[{0}] COMPLETE {1}" -f $stage, $files[0].Name)
  } else {
    Write-Output ("[{0}] PENDING -" -f $stage)
  }
}

Write-Output ""
Write-Output "Status is derived from output-file presence. It does not prove approval or quality."
