# Sitolo Security Exception & Risk Acceptance Register

**Status:** Current security governance contract  
**Primary owner:** Trust & Compliance  
**Technical remediation owner:** Engineering

## Purpose

Record explicit, time-bounded deviations from mandatory security controls. An exception is controlled residual-risk acceptance; it does not rewrite the canonical requirement.

## Required record

| Field | Requirement |
|---|---|
| Exception ID | Stable unique identifier |
| Control ID | One or more security-control IDs |
| Scope | Exact capability, environment or repository area |
| Reason | Evidence-backed reason for deviation |
| Risk | Confidentiality/integrity/availability impact |
| Compensating control | Existing mitigating control |
| Owner | Accountable owner |
| Approval | Required reviewer/approver |
| Created / expires | Explicit dates |
| Review trigger | Event/date requiring reassessment |
| Evidence | Current implementation/test evidence |
| Status | proposed / active / expired / closed |

## Rules

1. No security exception may be implicit.
2. Every exception names the affected control ID.
3. Production exceptions require explicit approval under security governance.
4. Every exception has an expiry or review date.
5. A compensating control is not equivalent to the missing control unless evidence proves equivalence.
6. Expired exceptions block release until renewed or closed.
7. Repeated exceptions trigger architecture/process review.
8. Exceptions never downgrade or replace the canonical security requirement.

## Current register

**No active security exceptions are recorded here.**

The separate test-scope exception `SCOPE-EXC-001` remains documented in `docs/security_test_harness.md`; it is not production risk acceptance and does not authorize database security claims.

## Closure

An exception closes only when the underlying control is implemented and evidenced, the evidence is current, and the record points to the proof.
