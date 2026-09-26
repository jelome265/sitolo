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
| Reference | Security context | ../../shared/security-context/CONTEXT.md | Conditional when security_relevant=yes | Applicable controls and security proof |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Read the run brief security applicability. If `security_relevant: yes`, carry the selected control IDs into the plan and load the applicable security context.
2. Define affected modules, contracts, ownership, invariants, and state transitions.
3. Define API, database, migration, transaction, concurrency, retry, audit, observability, and rollback impact where applicable.
4. Define tests, including negative/security and failure-mode tests.
5. Identify exact files or bounded areas to change.
6. For security-relevant work, include control ID → requirement/threat → implementation change → negative test/evidence → CI/release gate → residual risk/exception.
7. Record unresolved decisions without inventing answers.
8. Audit the plan for scope creep, missing proof, and authority conflicts.
9. Write the implementation plan to output/.

## Human Check

Review the plan before implementation. This is the primary design gate. Edit the plan in place.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Implementation plan | output/[run-slug]-plan.md | Markdown |
