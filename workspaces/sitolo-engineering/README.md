# Sitolo Engineering Workspace

This is the active ICM workspace for repeatable Sitolo software engineering.

## Start

Read:

1. CLAUDE.md
2. CONTEXT.md
3. the current stage CONTEXT.md

For a new engineering request, begin at stages/01-select/CONTEXT.md.

## Stage progression

01-select → 02-research → 03-investigate → 04-plan → 05-implement → 06-audit → 07-remediate → 08-verify → 09-deliver

Do not skip a stage. Research may conclude that external evidence is unnecessary. Remediation may conclude that no fix is required.

## Status

Use:

scripts/agentic/status

or on Windows:

scripts/agentic/status.ps1

Status is derived from stage output files. It does not prove approval or quality.

## Validation

Use:

scripts/ci/check-icm-workspace

or the Windows verifier through:

scripts/ci/check-icm-workspace.ps1

The canonical repository verifier includes this workspace check.

## Handoff rule

Every stage owns its output directory. Human edits to stage outputs are valid handoffs to the next stage. Long-lived engineering authority does not belong in stage outputs; it belongs in agent.md or the routed project documentation.
