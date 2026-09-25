$ErrorActionPreference = "Stop"

$ROOT = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ROOT

$WORKSPACE = "workspaces\sitolo-engineering"
$STAGES = @("01-select","02-research","03-investigate","04-plan","05-implement","06-audit","07-remediate","08-verify","09-deliver")
$TEMPLATES = @("run-brief","research-memo","investigation","implementation-plan","implementation-report","audit-report","remediation-report","verification-report","delivery-handoff")
$ARTIFACTS = @("brief","research","investigation","plan","implementation","audit","remediation","verification","delivery")
$fail = $false

$required = @(
  "CLAUDE.md","CONTEXT.md","AGENTS.md",
  "$WORKSPACE\CLAUDE.md","$WORKSPACE\CONTEXT.md",
  "$WORKSPACE\setup\questionnaire.md",
  "$WORKSPACE\_config\workflow-policy.md",
  "$WORKSPACE\shared\context-loading.md",
  "$WORKSPACE\skills\engineering-review\SKILL.md",
  "$WORKSPACE\shared\CONTEXT.md",
  "$WORKSPACE\shared\business-context\CONTEXT.md",
  "$WORKSPACE\shared\technical-context\CONTEXT.md",
  "$WORKSPACE\shared\security-context\CONTEXT.md",
  "$WORKSPACE\shared\commercial-context\CONTEXT.md",
  "$WORKSPACE\shared\integration-context\CONTEXT.md",
  "$WORKSPACE\shared\phase-context\CONTEXT.md",
  "$WORKSPACE\_templates\CONTEXT.md",
  "docs\icm_reference_integrity.md","docs\documentation_semantic_remediation_plan.md",
  "docs\current_implementation_status.md",
  "map\CLAUDE.md","map\AGENTS.md","map\routing.md","map\CONTEXT.md",
  "map\_meta\schema.md","map\_templates\object.md","map\_templates\process.md",
  "map\objects\CONTEXT.md","map\objects\_index.md","map\processes\CONTEXT.md","map\effects\CONTEXT.md",
  "scripts\ci\generate-system-map-routing","scripts\ci\generate-system-map-index"
)

foreach ($path in $required) {
  if (-not (Test-Path $path -PathType Leaf)) {
    Write-Error "ICM WORKSPACE VIOLATION: missing required file: $path"
    $fail = $true
  }
}

foreach ($entry in @("CLAUDE.md","$WORKSPACE\CLAUDE.md","$WORKSPACE\CONTEXT.md","map\CLAUDE.md","map\CONTEXT.md")) {
  if ((Test-Path $entry -PathType Leaf) -and ((Get-Content $entry).Count -gt 80)) {
    Write-Error "ICM WORKSPACE VIOLATION: routing/context exceeds 80 lines: $entry"
    $fail = $true
  }
}

  for ($i = 0; $i -lt $STAGES.Count; $i++) {
    $stage = $STAGES[$i]
    $context = Join-Path $coldRoot "workspace\sitolo-engineering\stages\$stage\CONTEXT.md"
    $output = Join-Path $coldRoot "workspace\sitolo-engineering\stages\$stage\output"
    $body = Get-Content $context -Raw

    if ($i -gt 0) {
      $matches = [regex]::Matches($body, "\.\./[0-9]{2}-[^ |\r\n]+/output/\[run-slug\]-[A-Za-z0-9_-]+\.md")
      foreach ($match in $matches) {
        $relative = $match.Value -replace "\[run-slug\]", $coldSlug
        $resolved = Join-Path (Split-Path $context -Parent) $relative
        if (-not (Test-Path $resolved -PathType Leaf)) {
          $fail = $true
          Write-Error "ICM COLD WALK FAILED: missing handoff before ${stage}: $relative"
        }
      }
    }

    $target = Join-Path $output ($coldSlug + "-" + $ARTIFACTS[$i] + ".md")
    $templatePath = Join-Path $coldRoot "workspace\sitolo-engineering\_templates\$($TEMPLATES[$i]).md"
    Copy-Item $templatePath $target -Force
    $artifact = Get-Content $target -Raw
    $artifact = $artifact -replace 'run_slug: "\[run-slug\]"', 'run_slug: "cold-walk"'
    $artifact = $artifact -replace '(?m)^status: draft$', 'status: human-approved'
    Set-Content $target $artifact -NoNewline

    if (-not (Test-Path $target -PathType Leaf)) {
      $fail = $true
      Write-Error "ICM COLD WALK FAILED: artifact not created: $target"
    }
    if ($artifact -notmatch '(?m)^run_slug: "cold-walk"$' -or $artifact -notmatch '(?m)^status: human-approved$') {
      $fail = $true
      Write-Error "ICM COLD WALK FAILED: synthetic human gate metadata missing: $target"
    }
  }
  $count = (Get-ChildItem "$coldRoot\workspace\sitolo-engineering\stages" -Recurse -File | Where-Object { $_.FullName -match "\\output\\" -and $_.Name -ne ".gitkeep" }).Count
  if ($count -ne 9) { $fail = $true; Write-Error "ICM COLD WALK FAILED: expected 9 artifacts, found $count" }

  $mapCard = Get-Content (Join-Path $coldRoot "map\objects\runtime\api-boundary.md") -Raw
  $source = ([regex]::Match($mapCard,"(?m)^source:\s*(.+)$")).Groups[1].Value.Trim()
  if (-not (Test-Path $source -PathType Leaf)) { $fail = $true; Write-Error "ICM COLD WALK FAILED: System Map source hop failed: $source" }
} finally {
  Remove-Item $coldRoot -Recurse -Force -ErrorAction SilentlyContinue
}

if ($fail) { Write-Error "ICM workspace check FAILED"; exit 1 }
Write-Output "ICM workspace check PASSED"
Write-Output "ICM COLD WALK PASSED: 9 stages, exact handoffs, synthetic human gates, System Map source hop"