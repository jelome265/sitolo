# 05 Implement

One job: implement the approved plan without broadening scope.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Implementation plan | ../04-plan/output/[run-slug]-plan.md | Full file | Exact change contract |
| Working | Investigation | ../03-investigate/output/[run-slug]-investigation.md | Full file | Existing-state constraints |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Relevant sections | Engineering rules |
| Reference | Project docs | ../../../../docs/README.md | Documents named by plan | Detailed contracts |
| Reference | Context policy | ../../shared/context-loading.md | Full file | Loading discipline |
| Reference | Implementation guide | references/implementation.md | Full file | Quality constraints |
| Reference | Security context | ../../shared/security-context/CONTEXT.md | Conditional when security_relevant=yes | Control implementation and required proof |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Confirm the plan is present and human-reviewed, including the security control mapping when the run is security-relevant.
2. Inspect the planned write set again before editing.
3. Implement the smallest coherent change.
4. Add or update tests proving behavior and relevant negative/security controls named by the plan.
5. Update migrations, API contracts, observability, or documentation when required by the plan.
6. Run targeted checks for the changed surface.
7. Audit the diff against the plan, ownership, and security/integrity controls, including each selected security control ID.
8. Write an implementation report to output/.

## Human Check

Review the code diff and implementation report. Do not move to audit while material scope deviations remain unexplained.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Implementation report | output/[run-slug]-implementation.md | Markdown |
