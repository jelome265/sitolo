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
  if (Test-Path $entry -PathType Leaf -and (Get-Content $entry).Count -gt 80) {
    Write-Error "ICM WORKSPACE VIOLATION: routing/context exceeds 80 lines: $entry"
    $fail = $true
  }
}

for ($i = 0; $i -lt $STAGES.Count; $i++) {
  $stage = $STAGES[$i]
  $context = "$WORKSPACE\stages\$stage\CONTEXT.md"
  $references = "$WORKSPACE\stages\$stage\references"
  $output = "$WORKSPACE\stages\$stage\output"
  if (-not (Test-Path $context -PathType Leaf)) {
    Write-Error "ICM WORKSPACE VIOLATION: missing stage contract: $context"
    $fail = $true
    continue
  }
  if ((Get-Content $context).Count -gt 80) {
    Write-Error "ICM WORKSPACE VIOLATION: stage contract exceeds 80 lines: $context"
    $fail = $true
  }
  $body = Get-Content $context -Raw
  foreach ($section in @("## Inputs","## Process","## Outputs","## Human Check")) {
    if ($body -notmatch [regex]::Escape($section)) {
      Write-Error "ICM WORKSPACE VIOLATION: missing ${section}: $context"
      $fail = $true
    }
  }
  if ($body -match "|s+../[^|]+/output/s+|") {
    Write-Error "ICM WORKSPACE VIOLATION: stage handoff must name exact artifact path: $context"
    $fail = $true
  }
  if (-not (Test-Path $references -PathType Container) -or -not (Test-Path $output -PathType Container) -or -not (Test-Path "$output\.gitkeep" -PathType Leaf)) {
    Write-Error "ICM WORKSPACE VIOLATION: references/output scaffold incomplete: $stage"
    $fail = $true
  }
  if (Test-Path $references -PathType Container) {
    Get-ChildItem $references -Recurse -File -Filter *.md | ForEach-Object {
      if ((Get-Content $_.FullName).Count -gt 200) {
        Write-Error "ICM WORKSPACE VIOLATION: reference exceeds 200 lines: $($_.FullName)"
        $fail = $true
      }
    }
  }
  if (Test-Path $output -PathType Container) {
    Get-ChildItem $output -Recurse -File | Where-Object { $_.Name -ne ".gitkeep" } | ForEach-Object {
      if ($_.Extension -ne ".md") {
        Write-Error "ICM WORKSPACE VIOLATION: stage outputs must be Markdown: $($_.FullName)"
        $fail = $true
      }
    }
  }
}

if (Test-Path ".codex" -PathType Container) { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: .codex must not be the workflow engine" }
if ((Test-Path ".agents\skills" -PathType Container) -or (Test-Path ".agents\evals" -PathType Container)) { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: repository-global agent workflow detected" }

$workflowText = (Get-ChildItem ".github\workflows" -File -ErrorAction SilentlyContinue | ForEach-Object { Get-Content $_.FullName -Raw }) -join [Environment]::NewLine
if ($workflowText -match "openai/codex-action|\[agents\.|codex-review\.yml") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: second Codex orchestration runtime detected" }

if ((Get-FileHash "map\CLAUDE.md").Hash -ne (Get-FileHash "map\AGENTS.md").Hash -or (Get-FileHash "map\CLAUDE.md").Hash -ne (Get-FileHash "map\routing.md").Hash) {
  $fail = $true
  Write-Error "ICM WORKSPACE VIOLATION: System Map routing twins differ"
}

Get-ChildItem "map\objects" -Recurse -File -Filter *.md | Where-Object { $_.Name -notin @("CONTEXT.md","_index.md") } | ForEach-Object {
  $text = Get-Content $_.FullName -Raw
  foreach ($field in @("type: object","status:","universe:","source_revision:","source:","source_citation:")) {
    if ($text -notmatch "(?m)^$([regex]::Escape($field))") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: object missing ${field}: $($_.FullName)" }
  }
  $source = ([regex]::Match($text,"(?m)^source:\s*(.+)$")).Groups[1].Value.Trim()
  if ([string]::IsNullOrWhiteSpace($source) -or -not (Test-Path $source -PathType Leaf)) { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: object source missing: $($_.FullName) -> $source" }
  if ($text -match "(?m)^status:\s*verified\s*$" -and $text -notmatch "(?m)^universe:\s*live\s*$") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: verified object must be live: $($_.FullName)" }
}

Get-ChildItem "map\processes" -File -Filter *.md | Where-Object { $_.Name -ne "CONTEXT.md" -and $_.Name -ne "_index.md" } | ForEach-Object {
  $text = Get-Content $_.FullName -Raw
  foreach ($field in @("type: process","status:","universe:","source_revision:","source:","source_citation:")) {
    if ($text -notmatch "(?m)^$([regex]::Escape($field))") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: process missing ${field}: $($_.FullName)" }
  }
  foreach ($section in @("## Input","## Movement","## Output","## Consumes","## Produces","## If you change this","### Hits","### Does not hit","## Verification","## See")) {
    if ($text -notmatch [regex]::Escape($section)) { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: process missing ${section}: $($_.FullName)" }
  }
  if ($text -notmatch "(?m)^\s*\d+\. ") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: process movement has no numbered steps: $($_.FullName)" }
  foreach ($section in @("## Consumes","## Produces")) {
    $start = $text.IndexOf($section)
    if ($start -ge 0) {
      $sectionText = $text.Substring($start)
      if ($sectionText -notmatch "\]\(\.\./objects/[^)]+\.md\)") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: ${section} lacks object links: $($_.FullName)" }
    }
  }
  $sources = ([regex]::Match($text,"(?m)^source:\s*(.+)$")).Groups[1].Value -split ";"
  foreach ($source in $sources) {
    $source = $source.Trim()
    if ([string]::IsNullOrWhiteSpace($source) -or -not (Test-Path $source -PathType Leaf)) { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: process source missing: $($_.FullName) -> $source" }
  }
  if ($text -match "(?m)^status:\s*verified\s*$" -and $text -notmatch "(?m)^universe:\s*live\s*$") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: verified process must be live: $($_.FullName)" }
  if ($text -match "(?m)^universe:\s*ghost\s*$" -and $text -notmatch "Ghost/unwired") { $fail = $true; Write-Error "ICM WORKSPACE VIOLATION: ghost process missing unwired evidence: $($_.FullName)" }
}

# Cold walk in an isolated temporary run.
$coldRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("sitolo-icm-cold-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $coldRoot -Force | Out-Null
try {
  Copy-Item $WORKSPACE (Join-Path $coldRoot "workspace") -Recurse -Force
  Copy-Item "map" (Join-Path $coldRoot "map") -Recurse -Force
  $coldSlug = "cold-walk"
  for ($i = 0; $i -lt $STAGES.Count; $i++) {
    $stage = $STAGES[$i]
    $context = Join-Path $coldRoot "$WORKSPACE\stages\$stage\CONTEXT.md"
    $output = Join-Path $coldRoot "$WORKSPACE\stages\$stage\output"
    $body = Get-Content $context -Raw
    $matches = [regex]::Matches($body, "\.\./[0-9]{2}-[^ |\r\n]+/output/\[run-slug\]-[A-Za-z0-9_-]+\.md")
    foreach ($match in $matches) {
      $relative = $match.Value -replace "\[run-slug\]", $coldSlug
      $resolved = Join-Path (Split-Path $context -Parent) $relative
      if (-not (Test-Path $resolved -PathType Leaf)) { $fail = $true; Write-Error "ICM COLD WALK FAILED: missing handoff before ${stage}: $relative" }
    }
    $target = Join-Path $output ($coldSlug + "-" + $ARTIFACTS[$i] + ".md")
    Copy-Item (Join-Path $coldRoot "$WORKSPACE\_templates\$($TEMPLATES[$i]).md") $target -Force
    $artifact = Get-Content $target -Raw
$artifact = $artifact -replace 'run_slug: "\[run-slug\]"', 'run_slug: "cold-walk"'
    $artifact = $artifact -replace "(?m)^status: draft$", "status: human-approved"
    Set-Content $target $artifact -NoNewline
    if (-not (Test-Path $target -PathType Leaf)) { $fail = $true; Write-Error "ICM COLD WALK FAILED: artifact not created: $target" }
    if ($artifact -notmatch '(?m)^run_slug: "cold-walk"$' -or $artifact -notmatch '(?m)^status: human-approved$') { $fail = $true; Write-Error "ICM COLD WALK FAILED: synthetic human gate metadata missing: $target" }
  }
  $count = (Get-ChildItem "$coldRoot\$WORKSPACE\stages" -Recurse -File | Where-Object { $_.FullName -match "\\output\\" -and $_.Name -ne ".gitkeep" }).Count
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