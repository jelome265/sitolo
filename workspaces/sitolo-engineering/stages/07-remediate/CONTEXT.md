# 07 Remediate

One job: correct confirmed findings, or explicitly record that no remediation is required, then route back to audit when a fix was made.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Audit | ../06-audit/output/[run-slug]-audit.md | Full file | Findings |
| Working | Implementation | ../05-implement/output/[run-slug]-implementation.md | Full file | Current change |
| Working | Plan | ../04-plan/output/[run-slug]-plan.md | Full file | Intended state |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Relevant rules | Required controls |
| Reference | Project docs | ../../../../docs/README.md | Documents named by audit | Authority |
| Reference | Remediation order | references/remediation-order.md | Full file | Fix priority |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Determine whether the latest audit contains PARTIAL or FAIL findings.
2. If none exist, record a no-remediation result and continue to 08-verify.
3. If findings exist, map each to its requirement, root cause, owner, dependency, fix, and proof.
4. Apply fixes in dependency order.
5. Add or strengthen proof without weakening the requirement.
6. Run targeted tests for each material correction.
7. Write a remediation report to output/.
8. Mark re-audit required when implementation changed. The human then re-enters 06-audit with the updated implementation; do not proceed to verification until the re-audit is clean.

## Human Check

Review the remediation report and diff. Confirm that fixes address root causes rather than only symptoms.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Remediation report | output/[run-slug]-remediation.md | Markdown |
