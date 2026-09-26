# 01 Select

One job: turn the user request into a bounded engineering run.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | User request | current task | Full request | Defines desired change |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Sections 0-3 and relevant definitions | Engineering constraints |
| Reference | Context policy | ../../shared/context-loading.md | Full file | Loading discipline |
| Reference | Request guide | references/request-brief.md | Full file | Output shape |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Classify the request as feature, bug, security, migration, refactor, documentation, or review.
2. Identify the affected capability and likely bounded context.
3. Record requirements, explicit exclusions, open decisions, and success conditions.
4. Always route through Stage 02. That stage records whether external research is required.
5. Write the run brief to output/.

## Human Check

Review the brief for scope and exclusions. Edit it in place. Do not let later stages invent missing scope.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Run brief | output/[run-slug]-brief.md | Markdown |
