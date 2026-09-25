# 08 Verify

One job: produce reproducible evidence that the approved implementation and re-audit satisfy the required gates.

## Inputs

| Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Latest audit | ../06-audit/output/ | Latest report | Required findings state |
| Remediation | ../07-remediate/output/ | Latest report when remediation occurred | Corrections and proof |
| Implementation | ../05-implement/output/ | Implementation report | Claimed change |
| Governance | ../../../../agent.md | Definitions of done | Verification requirements |
| Project docs | ../../../../docs/README.md | Applicable testing/deployment docs | Required evidence |
| Verification guide | references/verification.md | Full file | Gate selection |

## Process

1. Determine required targeted checks from the plan and audit.
2. Run the canonical verifier: ./scripts/ci/verify.
3. Run additional integration/security/contract tests required by the change.
4. Record exact commands, results, environment limitations, and unverified claims.
5. Confirm that the latest audit has no unresolved material findings.
6. Write the verification report to output/.
7. If verification fails, return to 07-remediate with the evidence. Otherwise continue to 09-deliver.

## Human Check

Review the evidence. A green test is evidence only for what it actually exercised.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Verification report | output/[run-slug]-verification.md | Markdown |
