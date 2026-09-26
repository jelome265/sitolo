# 06 Audit

One job: independently determine whether the implemented change is logically correct against its governing contract and expose gaps that tests or CI may not exercise.

## Inputs

| Kind | Source | File/Location | Section/Scope | Why |
|---|---|---|---|---|
| Working | Run brief | ../01-select/output/[run-slug]-brief.md | Full file | Scope |
| Working | Plan | ../04-plan/output/[run-slug]-plan.md | Full file | Intended behavior |
| Working | Implementation | ../05-implement/output/[run-slug]-implementation.md | Full file | Claimed changes |
| Working | Research | ../02-research/output/[run-slug]-research.md | Full file | External constraints |
| Reference | Governance | ../../shared/governance-context/CONTEXT.md | Relevant sections | Engineering authority |
| Reference | Project docs | ../../../../docs/README.md | Applicable contracts | Requirement evidence |
| Reference | Audit skill | ../../skills/engineering-review/SKILL.md | Full file | Review method |
| Reference | Audit contract | references/audit-contract.md | Full file | Independent semantic audit contract |
| Reference | Security checks | references/security-checks.md | Full file | Security-specific adversarial checks |
| Reference | Correctness checks | references/correctness-checks.md | Full file | Logic/invariant/failure checks |
| Reference | Security control register | ../../../../docs/security_control_register.md | Applicable control IDs | Security traceability authority |
| Reference | Security context | ../../shared/security-context/CONTEXT.md | Conditional when security_relevant=yes | Security control and threat evidence |

**Do NOT load:** unrelated stage contracts, unrelated reference families, the full documentation corpus, or the full `agent.md`; use the routed governance context and exact inputs only.

## Process

1. Read the run brief/plan security applicability. If `security_relevant: yes`, load the security context and control register for the named control IDs.
2. Read the audit contract and apply both the correctness and security checklists relevant to the changed surface.
3. Reconstruct the required behavior from the approved plan and authoritative project contracts.
4. Inspect the actual final code path, data boundary, state transitions, failure handling, and tests. Do not infer correctness from names, comments, coverage, or CI status.
5. Challenge the implementation for logic flaws, missing edge cases, invalid assumptions, race conditions, retry/idempotency errors, authorization gaps, boundary escapes, and unsafe failure behavior.
6. Treat CI and passing tests as evidence about exercised behavior only; a green CI result does not satisfy semantic audit by itself.
7. Build a requirement → implementation → evidence matrix, including control ID → implementation → negative test/evidence for each applicable security control.
8. Classify each material requirement PASS, PARTIAL, FAIL, or N/A. Missing proof or an unresolved material logic flaw is not PASS.
9. Write the audit report to output/.
10. Hand off to 07-remediate even when no finding exists, so that stage records the remediation decision.

## Human Check

Review the audit findings. Confirm that severity, evidence, and unresolved questions are represented accurately before verification.

## Outputs

| Artifact | Location | Format |
|---|---|---|
| Audit report | output/[run-slug]-audit.md | Markdown |
