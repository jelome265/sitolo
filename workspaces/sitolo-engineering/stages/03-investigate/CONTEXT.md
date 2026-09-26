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
| Reference | Security context | ../../shared/security-context/CONTEXT.md | Conditional when security_relevant=yes | Threats, controls, tests and evidence |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Read the run brief security applicability. If `security_relevant: yes`, load the security context and applicable control IDs before assessing the implementation.
2. Inspect the affected modules, types, repositories, migrations, tests, and workflows.
3. Identify the current source of truth, ownership, trust boundaries, transaction boundaries, and failure modes.
4. Read only the applicable project documents routed by docs/README.md.
5. Search for existing abstractions before proposing new ones.
6. For security-relevant work, record control → implementation → test/evidence gaps and residual risk.
7. Record contradictions, gaps, and reusable implementation patterns.
8. Write findings to output/.

## Human Check

Review the findings and evidence. Edit or correct them before planning.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Investigation findings | output/[run-slug]-investigation.md | Markdown |
