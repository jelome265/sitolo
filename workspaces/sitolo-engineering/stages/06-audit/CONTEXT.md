# 06 Audit

One job: independently compare the implementation with its governing contract and prove or expose gaps.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Scope |
| Working | Plan | ../04-plan/output/[run-slug]-plan.md | Full file | Intended behavior |
| Working | Implementation | ../05-implement/output/[run-slug]-implementation.md | Full file | Claimed changes |
| Working | Research | ../02-research/output/[run-slug]-research.md | Full file | External constraints |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Relevant sections | Engineering authority |
| Reference | Project docs | ../../../../docs/README.md | Applicable contracts | Requirement evidence |
| Reference | Audit skill | ../../skills/engineering-review/SKILL.md | Full file | Review method |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

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
