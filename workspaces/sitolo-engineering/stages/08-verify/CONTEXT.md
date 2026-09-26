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
| Reference | Security control register | ../../../../docs/security_control_register.md | Applicable control IDs | Security proof and release status |
| Reference | Security context | ../../shared/security-context/CONTEXT.md | Conditional when security_relevant=yes | Security verification and current standards |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Read security applicability and control IDs from the latest plan/audit. If `security_relevant: yes`, load the security context and control register before selecting gates.
2. Determine required targeted checks from the plan and audit.
3. Run the canonical verifier: ./scripts/ci/verify.
4. Run additional integration/security/contract tests required by the change.
5. For security-relevant work, record control ID → test/gate → result → evidence reference → limitation.
6. Record exact commands, results, environment limitations, and unverified claims.
7. Confirm that the latest audit has no unresolved material findings and no applicable security control remains unproven.
8. Write the verification report to output/.
9. If verification fails, record the failure and hand control back to 07-remediate; otherwise continue to 09-deliver.

## Human Check

Review the evidence. A green test is evidence only for what it actually exercised.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Verification report | output/[run-slug]-verification.md | Markdown |
