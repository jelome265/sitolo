# 06 Audit

One job: independently compare the implementation with its governing contract and prove or expose gaps.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Scope |
| Plan | ../04-plan/output/[run-slug]-plan.md | Full file | Intended behavior |
| Implementation | ../05-implement/output/[run-slug]-implementation.md | Full file | Claimed changes |
| Research | ../02-research/output/ | Research memo | External constraints |
| Governance | ../../../../agent.md | Relevant sections | Engineering authority |
| Project docs | ../../../../docs/README.md | Applicable contracts | Requirement evidence |
| Audit skill | ../../skills/engineering-review/SKILL.md | Full file | Review method |

## Process

1. Inspect the final diff and changed implementation.
2. Build a requirement → implementation → test or evidence matrix.
3. Adversarially check security, tenancy, integrity, concurrency, retry, offline, external failure, migration, observability, and resource controls where relevant.
4. Classify each material requirement PASS, PARTIAL, FAIL, or N/A.
5. Write the audit report to output/.
6. Hand off to 07-remediate even when no finding exists, so that stage records the remediation decision.

## Human Check

Review the audit findings. Confirm that severity, evidence, and unresolved questions are represented accurately.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Audit report | output/[run-slug]-audit.md | Markdown |
