# 03 Investigate

One job: understand the existing implementation and authoritative project contracts before design.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Scope |
| Working | Research | ../02-research/output/[run-slug]-research.md | Full file when produced | External constraints |
| Reference | Project docs map | ../../../../docs/README.md | Relevant sections | Locate authoritative documents |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Relevant sections | Engineering policy |
| Reference | Context policy | ../../shared/context-loading.md | Full file | Loading discipline |
| Reference | Investigation guide | references/investigation.md | Full file | Evidence checklist |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

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
