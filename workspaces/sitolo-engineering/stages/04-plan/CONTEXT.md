# 04 Plan

One job: turn selected requirements and investigation evidence into an implementation contract.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Run brief | ../01-select/output/ | Approved brief | Scope |
| Research | ../02-research/output/ | Research memo | External constraints |
| Investigation | ../03-investigate/output/ | Findings | Existing-state evidence |
| Governance | ../../../../agent.md | Relevant definitions of done and rules | Non-negotiable constraints |
| Project docs | ../../../../docs/README.md | Documents identified by investigation | Design authority |
| Plan guide | references/plan-contract.md | Full file | Plan structure |

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
