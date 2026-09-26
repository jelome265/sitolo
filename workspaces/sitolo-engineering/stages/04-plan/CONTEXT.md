# 04 Plan

One job: turn selected requirements and investigation evidence into an implementation contract.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Approved brief |
| Working | Research | ../02-research/output/[run-slug]-research.md | Full file | Research memo |
| Working | Investigation | ../03-investigate/output/[run-slug]-investigation.md | Full file | Existing-state evidence |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Relevant definitions of done and rules | Non-negotiable constraints |
| Reference | Project docs | ../../../../docs/README.md | Documents identified by investigation | Design authority |
| Reference | Plan guide | references/plan-contract.md | Full file | Plan structure |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Define affected modules, contracts, ownership, invariants, and state transitions.
2. Define API, database, migration, transaction, concurrency, retry, audit, observability, and rollback impact where applicable.
3. Define tests, including negative/security and failure-mode tests.
4. Identify exact files or bounded areas to change.
5. Record unresolved decisions without inventing answers.
6. Audit the plan for scope creep, missing proof, and authority conflicts.
7. Write the implementation plan to output/.

## Human Check

Review the plan before implementation. This is the primary design gate. Edit the plan in place.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Implementation plan | output/[run-slug]-plan.md | Markdown |
