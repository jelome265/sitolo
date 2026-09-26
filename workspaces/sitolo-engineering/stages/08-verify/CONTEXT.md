# 08 Verify

One job: produce reproducible evidence that the approved implementation and re-audit satisfy the required gates.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|
| Working | Latest audit | ../06-audit/output/[run-slug]-audit.md | Full file | Required findings state |
| Working | Remediation | ../07-remediate/output/[run-slug]-remediation.md | Full file when remediation occurred | Corrections and proof |
| Working | Implementation | ../05-implement/output/[run-slug]-implementation.md | Full file | Claimed change |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Definitions of done | Verification requirements |
| Reference | Project docs | ../../../../docs/README.md | Applicable testing/deployment docs | Required evidence |
| Reference | Verification guide | references/verification.md | Full file | Gate selection |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Determine required targeted checks from the plan and audit.
2. Run the canonical verifier: ./scripts/ci/verify.
3. Run additional integration/security/contract tests required by the change.
4. Record exact commands, results, environment limitations, and unverified claims.
5. Confirm that the latest audit has no unresolved material findings.
6. Write the verification report to output/.
7. If verification fails, record the failure and hand control back to 07-remediate; otherwise continue to 09-deliver.

## Human Check

Review the evidence. A green test is evidence only for what it actually exercised.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Verification report | output/[run-slug]-verification.md | Markdown |
