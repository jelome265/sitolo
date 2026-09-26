# Sitolo — Phase 16: Reporting / Exports Implementation Specification

**Document:** `phase16_reporting_exports_implementation.md`  
**Phase:** Phase 16 — Reporting / Exports  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 16 creates read-oriented reporting and controlled data export without turning analytics into an alternate transaction authority or tenant-data exfiltration path. Reports must derive from authoritative data or governed read models, preserve tenant/branch scope and place hard limits on expensive or sensitive exports.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 16: Reporting / Exports
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

Primary internal sources: `docs/api_contract.md`, `docs/database_design.md`, `docs/observability_spec.md`, `docs/security_implementation_spec.md`, `docs/implementation_plan.md`, `docs/domain_model.md`. Security governance uses the control register and data-governance material. Standards basis: ISO/IEC 27001:2022, NIST CSF 2.0, NIST SSDF 1.1, OWASP ASVS 5.0.0 and OWASP API Security guidance.

---

## Part 01 — Select / Reconcile

### Objective

Reconcile all reporting/export use cases with authoritative business data, tenant scope and the approved commercial/product surfaces.

### Required inputs

- API contract
- database design
- observability spec
- data governance/security contracts
- commercial reporting requirements
- implementation plan

### Work

Inventory each report/export, its source facts, time range, columns, sensitivity, scope, expected workload, delivery mechanism and audit class. Classify exports as synchronous bounded read, asynchronous job or prohibited.

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

Define report generation states and export lifecycle.

### Canonical state model

Report query: REQUESTED / RUNNING / COMPLETED / FAILED / CANCELLED. Export job: QUEUED / RUNNING / READY / EXPIRED / FAILED. Download authorization: ISSUED / USED / EXPIRED / REVOKED.

### Invariants

1. Reports never mutate authoritative transactional state.
2. Tenant/branch scope comes from trusted authorization context.
3. Export fields are allowlisted.
4. Date/time windows are bounded.
5. Large work is asynchronous and resource-limited.
6. Download authorization is short-lived and scoped.
7. Repeated export requests do not bypass quotas.
8. Sensitive fields are excluded unless explicitly authorized.
9. Historical numbers remain reproducible from authoritative facts/read-model versioning.

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

Define governed read models, query limits and export storage.

### Persistence authority

Use authoritative transaction tables or explicitly governed read models. Record report definition/version, requested scope, actor, filter bounds, output digest/identity, expiration and audit reference. Large outputs reside in controlled object storage with short-lived access rather than database blobs unless required.

### Transaction rules

Report requests create job state atomically. Query execution occurs outside transaction locks that would block POS. Export completion records artifact identity and authorization metadata.

### Migration and compatibility

Read models may evolve independently but must declare source schema/version and rebuild semantics. Historical reports requiring reproducibility need documented versioning or snapshot strategy.

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

Prevent tenant data leakage, unrestricted database workload, sensitive-field exposure and report tampering.

### Trust model

Every report resolves actor, tenant, branch and field authority before query construction. Parameterized queries only. Export downloads use scoped, expiring authorization. Support reports require explicit support scope and heightened audit.

### Required controls

SC-002, SC-003, SC-004, SC-009, SC-010, SC-012.

### Security tests

```text
cross-tenant report
cross-branch report
field-allowlist bypass
SQL injection
unbounded date range
unbounded row count
expired download
reused download token
support scope bypass
export quota bypass
sensitive field leakage
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

Implement reporting as a bounded read workload with asynchronous export machinery.

### Mandatory sequence

1. report catalog/definitions;
2. authorization metadata;
3. query/read-model layer;
4. bounded filters/pagination;
5. export job queue;
6. controlled artifact storage;
7. download authorization;
8. quotas/rate limits;
9. audit/evidence;
10. operational dashboards.

### Implementation rules

No generic unrestricted SQL/report endpoint. Queries must declare expected indexes and limits. Export jobs are resumable only where semantics permit. Large report generation must not share unbounded resources with POS transactions.

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

query timeout, read-model lag, export storage failure, worker crash, artifact corruption, expired download, quota exhaustion, database overload, schema mismatch.

### Concurrency model

Test simultaneous exports, report generation versus POS, repeated identical export requests, cancellation versus completion and artifact cleanup races.

### Adversarial tests

alter tenant selector, add forbidden columns, inject filters, inflate date ranges, request nested expansions, abuse concurrent exports, guess download URLs.

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

report latency, queue depth, export size distribution, failure classes, read-model lag, quota denials and storage failures. Keep user/tenant dimensions bounded and policy-approved.

### Operational controls

Set report-specific budgets, queue priority and retention. Large exports require operational ownership and cleanup. Alerts distinguish data correctness issues from workload exhaustion.

### Recovery

Rebuild read models from authoritative state. Re-run export jobs idempotently when possible. Expire/revoke compromised download authorizations and retain audit evidence.

### Runbooks

- reporting overload;
- export storage failure;
- read-model divergence;
- sensitive export incident;
- stale report rebuild;
- expired-download support case.

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

Phase 16 consumes outputs from Phases 8–15 but does not become source of those facts. Phase 20 uses report/readiness evidence during certification.

### Evidence package

- authorization matrix;
- tenant/branch negative tests;
- SQL/query safety tests;
- large-export resource tests;
- artifact integrity tests;
- secure-download expiry tests;
- read-model rebuild evidence.

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

Evidence must prove report scope, field controls, workload bounds, artifact authorization and rebuildability from authoritative state.

### Human approval boundary

Human approval is required for introducing sensitive export fields, support/global reporting scope, retention changes and materially new data products.

### Definition of done

Phase 16 is complete only when reporting is read-only, tenant-safe, bounded, auditable, recoverable and resistant to data-exfiltration and workload-abuse paths.

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

**End of Phase 16 contract.**

# 12. Report Definition Contract

A report definition must identify:

~~~text
report ID/version
business purpose
authoritative source/read model
allowed dimensions
allowed measures
maximum time window
required scope
sensitivity classification
freshness expectation
query budget
owner
~~~

A report definition is not permission. Authorization is evaluated at execution.

# 13. Query Safety Model

Every generated query passes:

~~~text
typed filter parsing
→ authorization scope injection
→ field allowlist
→ bounded predicate validation
→ index/plan expectations
→ row/byte/time limits
→ execution
~~~

User-provided filters may constrain a permitted query but cannot broaden tenant, branch or field authority.

# 14. Export Artifact Contract

Large exports must produce an artifact with:

~~~text
export ID
report definition/version
requesting actor
tenant/branch scope
creation timestamp
source data/read-model version
row/byte count
content type
digest
expiration
download authorization reference
audit reference
~~~

The download channel must not expose a durable public URL.

# 15. Data Freshness and Reproducibility

Reports must declare whether they are:

~~~text
authoritative real-time
near-real-time read model
periodically materialized
historical snapshot
~~~

A stale read model must be visible in metadata where correctness could be affected. Financial, inventory and tax reports must define reconciliation expectations with authoritative state.

# 16. Abuse Cases

| Abuse | Control |
|---|---|
| tenant selector tampering | server-derived scope |
| forbidden column injection | field allowlist |
| unlimited date range | hard maximum |
| concurrent export flood | quotas/concurrency cap |
| guessed artifact URL | scoped authorization |
| report query timeout | database/query budget |
| stale report interpreted as final | freshness metadata |
| support global export | separate privileged policy |

# 17. Capacity Gates

Before production use, measure:

~~~text
small report latency
large report latency
export throughput
DB CPU/IO impact
concurrent jobs
storage growth
cleanup latency
read-model lag
~~~

The report workload must be demonstrated not to starve POS, inventory or synchronization workloads.

# 18. Evidence Ledger

The evidence record must connect:

~~~text
report definition/version
→ authorization policy
→ query/read model revision
→ test dataset
→ expected scope/fields
→ output artifact digest
→ reviewer
~~~

# 19. No-Go Conditions

Do not release a report surface if:

- scope is client-authoritative;
- unrestricted field selection exists;
- large exports have no workload budget;
- download artifacts are publicly addressable;
- report freshness is unknown for a materially time-sensitive report;
- read-model rebuildability is unproven.
