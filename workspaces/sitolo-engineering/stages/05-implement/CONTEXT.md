# 05 Implement

One job: implement the approved plan without broadening scope.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Implementation plan | ../04-plan/output/ | Approved plan | Exact change contract |
| Investigation | ../03-investigate/output/ | Findings | Existing-state constraints |
| Governance | ../../../../agent.md | Relevant sections | Engineering rules |
| Project docs | ../../../../docs/README.md | Documents named by plan | Detailed contracts |
| Context policy | ../../shared/context-loading.md | Full file | Loading discipline |
| Implementation guide | references/implementation.md | Full file | Quality constraints |

## Process

1. Confirm the plan is present and human-reviewed.
2. Inspect the planned write set again before editing.
3. Implement the smallest coherent change.
4. Add or update tests proving behavior and relevant negative controls.
5. Update migrations, API contracts, observability, or documentation when required by the plan.
6. Run targeted checks for the changed surface.
7. Audit the diff against the plan, ownership, and security/integrity controls.
8. Write an implementation report to output/.

## Human Check

Review the code diff and implementation report. Do not move to audit while material scope deviations remain unexplained.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Implementation report | output/[run-slug]-implementation.md | Markdown |
