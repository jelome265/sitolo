# 03 Investigate

One job: understand the existing implementation and authoritative project contracts before design.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Scope |
| Research | ../02-research/output/[run-slug]-research.md | Full file when produced | External constraints |
| Project docs map | ../../../../docs/README.md | Relevant sections | Locate authoritative documents |
| Governance | ../../../../agent.md | Relevant sections | Engineering policy |
| Context policy | ../../shared/context-loading.md | Full file | Loading discipline |
| Investigation guide | references/investigation.md | Full file | Evidence checklist |

## Process

1. Inspect the affected modules, types, repositories, migrations, tests, and workflows.
2. Identify the current source of truth, ownership, trust boundaries, transaction boundaries, and failure modes.
3. Read only the applicable project documents routed by docs/README.md.
4. Search for existing abstractions before proposing new ones.
5. Record contradictions, gaps, and reusable implementation patterns.
6. Write findings to output/.

## Human Check

Review the findings and evidence. Edit or correct them before planning.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Investigation findings | output/[run-slug]-investigation.md | Markdown |
