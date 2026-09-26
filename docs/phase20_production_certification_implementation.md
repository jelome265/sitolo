# Sitolo — Phase 20: Production Certification Implementation Specification

**Document:** `phase20_production_certification_implementation.md`  
**Phase:** Phase 20 — Production Certification  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 20 is the final evidence-and-approval gate for a production release candidate. It is not a document-signing exercise. Certification is the structured conclusion of architecture, security, domain correctness, integrations, offline, operations, supply-chain and regulatory evidence, with explicit ownership of residual risk. External certifications remain external decisions.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 20: Production Certification
     ↓
Next phase(s)
```

The phase may not silently bypass unresolved upstream authority, security, data-integrity or migration decisions.

## 2. Mandatory sequential implementation model

Implementation is **sequential and gated**. The phase MUST NOT be implemented as one-shot work.

```text
Part 01 — Select / Reconcile
    ↓ gate
Part 02 — Domain / State
    ↓ gate
Part 03 — Persistence / Transactions
    ↓ gate
Part 04 — Security / Authority
    ↓ gate
Part 05 — Core Implementation
    ↓ gate
Part 06 — Failure / Concurrency / Adversarial Tests
    ↓ gate
Part 07 — Observability / Operations / Recovery
    ↓ gate
Part 08 — Integration / Evidence
    ↓ gate
Part 09 — Semantic Audit
    ↓ gate
Part 10 — Remediation
    ↓ gate
Part 11 — Verification / Delivery
```

**Mandatory rule:** an agent may not implement Part N+1 until the Part N exit gate is explicitly evidenced in the filesystem/PR record. A green build does not waive semantic audit.

Each part MUST produce a durable artifact in the applicable ICM stage output location.

## 3. Authority and standards

Primary internal sources: `docs/implementation_plan.md`, `docs/ci_enforcement.md`, `docs/deployment_spec.md`, `docs/security_architecture_design.md`, `docs/security_control_register.md`, `docs/security_exception_register.md`, `docs/release-governance-evidence.md`, `docs/external_standards_verification_register.md`. External basis includes ISO/IEC 27001:2022, ISO/IEC 38500:2024, ISO 37301:2021, ISO 22301:2019, NIST CSF 2.0, NIST SSDF 1.1, OWASP ASVS 5.0.0 and SLSA 1.2. MRA certification is governed by MRA requirements. These sources provide governance/reference structures; their existence does not certify Sitolo.

---

## Part 01 — Select / Reconcile

### Objective

Select the exact release candidate and reconcile every mandatory production gate against current evidence.

### Required inputs

- exact source commit
- build/artifact digests
- test/security reports
- phase closure records
- migration state
- configuration version
- SBOM/provenance
- external certification records
- residual-risk/exception records

### Work

Freeze the candidate. Map required Phase 20 evidence to the actual release artifact, not another commit. Verify all upstream phase statuses and confirm no stale historical evidence is being reused as current.

### Output

A bounded implementation brief identifying authoritative sources, unresolved decisions, dependencies, acceptance evidence and human approvals.

### Exit gate

```text
[ ] phase scope is explicit
[ ] canonical sources are identified
[ ] no authority conflict is hidden
[ ] dependencies are confirmed
[ ] security controls are mapped
[ ] required evidence is named
[ ] unresolved decisions are recorded
[ ] human approval boundaries are explicit
```

---

## Part 02 — Domain / State

### Objective

Define certification state and finding severity.

### Canonical state model

Candidate: PREPARING / EVIDENCE_COMPLETE / UNDER_REVIEW / BLOCKED / APPROVED / RELEASED / REVOKED. Finding: OPEN / ACCEPTED_RISK / REMEDIATION_REQUIRED / VERIFIED_CLOSED.

### Invariants

1. Release approval applies to an exact artifact identity.
2. Missing evidence is not equivalent to pass.
3. A skipped security test is not a pass.
4. Critical security/tenant/financial/data-loss findings block release absent explicit emergency governance.
5. External certification claims require external evidence.
6. Post-approval artifact changes invalidate the approval and require re-certification.
7. Exceptions have owner, approval, scope and expiry.

### Output

State transition and invariant matrix.

### Exit gate

```text
[ ] state machine is explicit
[ ] illegal transitions are explicit
[ ] invariants have owners
[ ] historical truth rules are explicit
[ ] idempotency boundary is explicit
[ ] cross-phase interactions are explicit
```

---

## Part 03 — Persistence / Transactions

### Objective

Assemble the final persistence/migration/release state and verify deployment reproducibility.

### Persistence authority

Certification records reference migration version, schema compatibility evidence, configuration version, artifact digest, provenance, rollback reference and restore evidence. They are immutable evidence records subject to governance retention.

### Transaction rules

Certification is not itself a product transaction but must be generated from durable evidence. Approval state transitions are atomic and auditable in the release-governance system.

### Migration and compatibility

Verify every schema migration included in the candidate, compatibility windows, rollback posture, restore evidence and external-side-effect handling. No candidate is approved if a migration depends on undocumented manual steps unless formally governed.

### Output

Persistence contract, transaction matrix and migration-risk record.

### Exit gate

```text
[ ] authoritative tables/models are identified
[ ] constraints are defined
[ ] tenant/scope protection is defined
[ ] transaction boundaries are explicit
[ ] idempotency keys/records are durable where required
[ ] migration strategy is compatible
[ ] rollback/recovery behavior is known
```

---

## Part 04 — Security / Authority

### Objective

Prove the release candidate satisfies mandatory security controls and does not rely on unverifiable assurances.

### Trust model

Reconcile security control IDs to implementation evidence, test results and release gates. Check secret scanning, SCA, SAST, API security, tenant isolation, DAST where applicable, fuzzing, IaC/cloud controls, provenance and artifact verification. Maintain a current exception register.

### Required controls

All applicable controls in security_control_register.md, with explicit status and residual risk. Phase 20 is also the final point to ensure the security-control-to-evidence chain is complete.

### Security tests

```text
exact-artifact security scan
BOLA/cross-tenant suite
authorization negative suite
secret scan
SCA/advisories
SAST
DAST where applicable
fuzz smoke/full suite as required
migration security tests
artifact/provenance verification
restore/security regression
```

### Exit gate

```text
[ ] authority source is server-side
[ ] tenant/scope checks are explicit
[ ] privileged operations have policy
[ ] replay/duplicate behavior is defined
[ ] secrets are bounded
[ ] fail-closed behavior is tested
[ ] audit evidence is defined
[ ] security control IDs are mapped
```

---

## Part 05 — Core Implementation

### Objective

Run the final certification package over the exact candidate and freeze the evidence set.

### Mandatory sequence

1. candidate freeze;
2. reproducible build;
3. software/security evidence collection;
4. domain correctness evidence;
5. integration evidence;
6. offline evidence;
7. operations/DR evidence;
8. supply-chain evidence;
9. regulatory/external certification reconciliation;
10. approval/release record.

### Implementation rules

No evidence from a different commit is silently reused. Every artifact has identity. Any candidate mutation after evidence collection restarts the affected gates. Certification package content is versioned and attributable.

### Exit gate

```text
[ ] core use cases implemented
[ ] production code follows approved dependency direction
[ ] no competing authority introduced
[ ] no hidden side effects
[ ] error taxonomy is preserved
[ ] operational limits are enforced
```

---

## Part 06 — Failure / Concurrency / Adversarial Verification

### Required failure classes

missing evidence, failed scan, non-reproducible artifact, stale certification evidence, unclosed high-risk defect, migration incompatibility, restore failure, broken rollback, provider contract drift, MRA certification mismatch, expired security exception.

### Concurrency model

Certification must handle concurrent evidence producers without overwriting status. Approval and candidate mutation are serialized. Re-certification is triggered by any material change.

### Adversarial tests

attempt to mark skipped jobs as passed, substitute artifact digests, reuse stale reports, forge certification records, alter exception expiry, bypass protected branch gates, publish unverified compliance language.

### Exit gate

```text
[ ] positive and negative tests exist
[ ] retry and replay are exercised
[ ] concurrency is tested where applicable
[ ] denial tests prove no unauthorized side effect
[ ] external timeout/unknown outcomes are tested where applicable
[ ] test evidence is attributable to a revision
```

---

## Part 07 — Observability / Operations / Recovery

### Telemetry

Track evidence freshness, gate status, finding counts, exception age, build provenance, release approvals, failed gate classes and post-approval drift signals. Certification telemetry is itself protected evidence.

### Operational controls

Release authority, engineering, security/trust and compliance ownership must be explicit. Production release uses the approved deployment runbook and rollback reference. Post-release monitoring and acceptance are part of delivery.

### Recovery

If a release is found defective, invoke rollback/forward-fix policy based on database compatibility and external side effects. Revoke or suspend the release approval where necessary. Preserve the certification evidence and incident linkage.

### Runbooks

- certification blocker;
- evidence drift;
- post-approval candidate mutation;
- rollback after release;
- security exception expiry;
- external certification mismatch;
- provenance verification failure.

### Exit gate

```text
[ ] operational metrics are bounded
[ ] critical transitions are attributable
[ ] alerts have owners
[ ] failure modes have response paths
[ ] recovery procedure is executable
[ ] recovery evidence is retained
```

---

## Part 08 — Integration / Evidence

### Integration boundaries

Phase 20 consumes all prior phase evidence. It is the terminal program gate; post-release operational learning feeds future changes and does not retroactively falsify the certified artifact.

### Evidence package

- architecture review;
- security control matrix;
- test/security reports;
- domain invariant suite;
- integration/provider evidence;
- offline replay/recovery evidence;
- deployment rehearsal;
- rollback rehearsal;
- restore drill;
- SBOM/provenance/signature verification;
- regulatory/certification records;
- approval record.

### Exit gate

```text
[ ] upstream dependencies are verified
[ ] downstream contracts remain compatible
[ ] integration fixtures are current
[ ] evidence identifies source revision
[ ] no target-state statement is used as proof of implementation
```

---

## Part 09 — Semantic Audit

A separate audit MUST reconstruct this contract and challenge the actual implementation.

### Required audit procedure

```text
1. reconstruct requirements from canonical sources
2. inspect actual code paths and persistence
3. trace authority and tenant scope
4. test state transitions
5. inspect retry/idempotency behavior
6. inspect failure and recovery paths
7. inspect observability and evidence
8. challenge concurrency/race behavior
9. compare implementation with contract
10. produce requirement → implementation → evidence matrix
```

CI and unit tests are evidence only; they are not a substitute for semantic audit.

### Audit outputs

- requirement-to-implementation-to-evidence matrix;
- negative/security control matrix;
- residual-risk list;
- explicit PASS / PARTIAL / FAIL / N/A classification;
- remediation handoff.

### Gate

No phase delivery while a Critical/High semantic defect remains unresolved unless an explicit approved exception exists.

---

## Part 10 — Remediation / Re-audit

Every material finding follows:

```text
finding
 ↓
root cause
 ↓
bounded remediation
 ↓
regression test
 ↓
re-audit
 ↓
verification
```

Remediation MUST NOT silently change the contract to make the implementation appear compliant.

### Exit gate

```text
[ ] all Critical findings closed
[ ] High findings closed or explicitly risk-accepted
[ ] regression evidence exists
[ ] re-audit completed
[ ] residual risk recorded
[ ] register status updated
```

---

## Part 11 — Verification / Delivery

### Final evidence

The final package must identify exact artifact digest, source revision, compiler/toolchain, dependency lock state, test/security evidence, migration version, configuration version, external certification status, exceptions and approval identities.

### Human approval boundary

Final release approval is a human governance decision. Security, compliance/regulatory and operational owners must approve the evidence within their authority. The software agent cannot self-authorize production release.

### Definition of done

Phase 20 is complete only when the exact candidate is evidenced across correctness, security, integrations, offline, operations, supply chain and external obligations, with all blocking findings resolved or explicitly governed and the final release approval recorded.

### Delivery handoff

The ICM delivery artifact MUST identify:

```text
phase
contract revision
implementation revision
test/evidence revisions
known residual risk
migration state
operational readiness
next-phase dependency
```

**End of Phase 20 contract.**

# 12. Certification Evidence Chain

The final evidence chain is:

~~~text
requirement
→ control
→ implementation
→ test
→ evidence artifact
→ exact source/artifact identity
→ reviewer
→ approval
~~~

Any broken link is an evidence gap.

# 13. Exact Candidate Identity

The certification record must bind:

~~~text
source commit
repository state
Cargo.lock/toolchain
build environment
artifact digest
container/image identity
SBOM
provenance/attestation
configuration version
migration version
~~~

A later rebuild that produces a different digest is a different candidate.

# 14. Cross-Phase Readiness Matrix

Before approval, verify each phase:

~~~text
contract coverage
implementation state
verification state
open findings
exceptions
migration state
operational readiness
downstream dependencies
~~~

A target contract is not implementation evidence.

# 15. Security Certification Matrix

Reconcile the security-control register to:

~~~text
SC-ID
control requirement
implementation location
test
test result
workflow/run
artifact
residual risk
exception
owner
~~~

Critical tenant isolation, financial integrity, authentication-bypass, data-loss and secret-exposure defects block release absent formal emergency governance.

# 16. External and Regulatory Evidence

For any external obligation:

~~~text
external authority/version/date
→ internal requirement
→ implementation
→ test
→ external approval/certification
~~~

Do not state “MRA compliant”, “ISO compliant” or equivalent merely because internal tests pass. External certification or conformity is a separate evidence class.

# 17. Post-Approval Integrity

A material change to the certified candidate invalidates affected evidence.

Examples:

~~~text
source code change
dependency/lockfile change
build toolchain change
configuration change affecting trust/business behavior
migration change
artifact rebuild with changed digest
~~~

The appropriate gates must run again.

# 18. Release Decision Record

The final human decision record contains:

~~~text
candidate identity
scope
evidence summary
blocking findings
accepted residual risk
exceptions + expiry
approvers
decision
decision timestamp
release destination
rollback reference
~~~

# 19. Post-Release Certification Monitoring

Certification is not the end of governance.

Monitor:

~~~text
security regressions
artifact drift
critical incident triggers
provider/regulatory changes
exception expiry
rollback capability
material production deviations
~~~

Material changes reopen the relevant phase/certification gate.

# 20. No-Go Conditions

Phase 20 cannot be approved when:

- the exact release artifact cannot be identified;
- a required security gate did not execute;
- a critical blocking finding remains;
- evidence belongs to another candidate;
- an exception is expired/unapproved;
- external certification is required but absent;
- rollback/restore evidence is absent where required.
