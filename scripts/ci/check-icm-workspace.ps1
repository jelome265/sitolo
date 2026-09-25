$ErrorActionPreference = "Stop"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

$WORKSPACE = "workspaces\sitolo-engineering"
$STAGES = @("01-select","02-research","03-investigate","04-plan","05-implement","06-audit","07-remediate","08-verify","09-deliver")
$fail = $false

$required = @(
  "CLAUDE.md","CONTEXT.md","AGENTS.md",
  "$WORKSPACE\CLAUDE.md","$WORKSPACE\CONTEXT.md",
  "$WORKSPACE\setup\questionnaire.md",
  "$WORKSPACE\_config\workflow-policy.md",
  "$WORKSPACE\shared\context-loading.md",
  "$WORKSPACE\skills\engineering-review\SKILL.md"
)

foreach ($path in $required) {
  if (-not (Test-Path $path -PathType Leaf)) {
    Write-Error "ICM WORKSPACE VIOLATION: missing required file: $path"
    $fail = $true
  }
}

foreach ($stage in $STAGES) {
  $context = "$WORKSPACE\stages\$stage\CONTEXT.md"
  $references = "$WORKSPACE\stages\$stage\references"
  $output = "$WORKSPACE\stages\$stage\output"

  if (-not (Test-Path $context -PathType Leaf)) {
    Write-Error "ICM WORKSPACE VIOLATION: missing stage contract: $context"
    $fail = $true
  } else {
    if ((Get-Content $context).Count -gt 80) {
      Write-Error "ICM WORKSPACE VIOLATION: stage contract exceeds 80 lines: $context"
      $fail = $true
    }
    $body = Get-Content $context -Raw
    foreach ($section in @("## Inputs","## Process","## Outputs")) {
      if ($body -notmatch [regex]::Escape($section)) {
        Write-Error ("ICM WORKSPACE VIOLATION: stage contract missing section " + $section + ": " + $context)
        $fail = $true
      }
    }
  }

  if (-not (Test-Path $references -PathType Container)) {
    Write-Error "ICM WORKSPACE VIOLATION: missing references directory: $references"
    $fail = $true
  } else {
    Get-ChildItem $references -Recurse -File -Filter *.md | ForEach-Object {
      if ((Get-Content $_.FullName).Count -gt 200) {
        Write-Error "ICM WORKSPACE VIOLATION: reference exceeds 200 lines: $($_.FullName)"
        $fail = $true
      }
    }
  }

  if (-not (Test-Path $output -PathType Container)) {
    Write-Error "ICM WORKSPACE VIOLATION: missing output directory: $output"
    $fail = $true
  } else {
    if (-not (Test-Path "$output\.gitkeep" -PathType Leaf)) {
      Write-Error "ICM WORKSPACE VIOLATION: output directory must contain .gitkeep: $output"
      $fail = $true
    }
    Get-ChildItem $output -Recurse -File | Where-Object { $_.Name -ne ".gitkeep" } | ForEach-Object {
      if ($_.Extension -ne ".md") {
        Write-Error "ICM WORKSPACE VIOLATION: stage outputs must be Markdown: $($_.FullName)"
        $fail = $true
      }
    }
  }
}

if (Test-Path ".codex" -PathType Container) {
  Write-Error "ICM WORKSPACE VIOLATION: .codex is not the workflow engine; use filesystem-routed workspaces."
  $fail = $true
}
if ((Test-Path ".agents\skills" -PathType Container) -or (Test-Path ".agents\evals" -PathType Container)) {
  Write-Error "ICM WORKSPACE VIOLATION: repository-global agent skills/evals must not replace workspace-local ICM structure."
  $fail = $true
}

$workflowText = Get-ChildItem ".github\workflows" -File -ErrorAction SilentlyContinue | Get-Content -Raw
$workflowText = $workflowText -join [Environment]::NewLine
if ($workflowText -match "openai/codex-action|\[agents\.|codex-review\.yml") {
  Write-Error "ICM WORKSPACE VIOLATION: CI must not introduce a second Codex orchestration/reviewer runtime."
  $fail = $true
}

if ($fail) {
  Write-Error "ICM workspace check FAILED"
  exit 1
}

Write-Output "ICM workspace check PASSED"
