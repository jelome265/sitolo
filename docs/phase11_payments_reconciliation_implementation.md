# Sitolo — Phase 11: Payments / Reconciliation Implementation Specification

**Document:** `phase11_payments_reconciliation_implementation.md`  
**Phase:** Phase 11 — Payments / Reconciliation  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 11 establishes the authoritative boundary between Sitolo's payment intent, attempts to external providers, observed provider evidence and Sitolo's reconciliation decision. The core correctness problem is not “did the HTTP request return 200?” but whether Sitolo can safely explain what was requested, what was observed externally, what economic state was accepted locally, and what remains unknown.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 11: Payments / Reconciliation
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

Primary internal sources: `docs/payment_integration_spec.md`, `docs/domain_model.md`, `docs/api_contract.md`, `docs/database_design.md`, `docs/security_implementation_spec.md`, `docs/implementation_plan.md`, `docs/phase10_pos_sales_implementation.md`. External provider contracts are authoritative at the provider boundary. Security controls follow the existing Sitolo security hierarchy. Use ISO/IEC 27001:2022 as an ISMS reference, NIST CSF 2.0 for risk outcomes, NIST SSDF 1.1 for secure development practices, and OWASP ASVS 5.0.0/API guidance for application controls; these are reference frameworks, not certification claims.

---

## Part 01 — Select / Reconcile

### Objective

Reconcile the payment-provider contract, existing payment domain model and Phase 10 POS handoff. Establish which payment facts are created locally, which are provider observations, and which transitions require reconciliation authority.

### Required inputs

- payment_integration_spec.md
- domain_model.md
- api_contract.md
- database_design.md
- phase10_pos_sales_implementation.md
- current provider contract/credentials/test environment
- security_control_register.md

### Work

Identify payment methods, merchant accounts, currencies, provider references, command IDs and reconciliation responsibilities. Explicitly classify each operation as intent creation, provider side effect, provider observation, local reconciliation or correction. Record all provider-specific assumptions with source and verification date. Do not invent provider behavior.

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

Define the payment state machine and the distinction between local intent, provider attempt, external observation and reconciled business fact.

### Canonical state model

```text
PaymentIntent
  DRAFT
  READY
  SUBMITTED
  UNKNOWN
  RECONCILIATION_REQUIRED
  CONFIRMED
  FAILED
  CANCELLED

PaymentAttempt
  CREATED
  SENT
  RESPONSE_RECEIVED
  UNKNOWN
  TERMINAL

ProviderObservation
  RECEIVED
  VERIFIED
  CONFLICTING
  SUPERSEDED

Reconciliation
  OPEN
  MATCHED
  MISMATCHED
  MANUAL_REVIEW
  RESOLVED
```
A provider observation is evidence, not automatically a local financial fact. Terminal states must have explicit transition predicates.

### Invariants

1. A timeout never implies provider failure.
2. A provider “success” is not a Sitolo reconciliation decision until identity, amount, currency, merchant account and expected operation are correlated.
3. Reusing an idempotency key with different semantics is rejected.
4. A finalized local payment fact is not destructively rewritten.
5. Duplicate callbacks are harmless.
6. Provider state regressions do not silently rewrite reconciled history.
7. Unknown outcomes remain explicit until positively reconciled.
8. Refunds/corrections cannot exceed original authorized economic entitlement.
9. Cross-tenant payment records are inaccessible.
10. External HTTP is never held inside a long-lived business transaction.

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

Materialize durable payment intent, attempt, provider observation, reconciliation and idempotency state using PostgreSQL authority.

### Persistence authority

Authoritative persistence must distinguish at least:
- payment intent identity and expected economics;
- provider attempt identity and idempotency linkage;
- provider event/callback evidence;
- reconciliation decision and reason;
- correction/refund references;
- audit/evidence linkage.
Use uniqueness constraints for retry boundaries and provider event deduplication. Provider payloads must be minimized, classified and protected; raw payload retention is governed by legal/security requirements rather than convenience.

### Transaction rules

Local transaction boundaries establish intent and durable work before provider calls. Provider calls occur outside the core database transaction. Reconciliation consumes verified provider evidence and commits a local decision atomically with its audit/outbox side effects. Retry workers use short leases and deterministic claims.

### Migration and compatibility

Use expand → compatible application → backfill → validate → contract. Existing sales/payment links must remain reconstructable. Any change to amount/currency/merchant-account semantics requires explicit migration analysis and regression fixtures.

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

Prevent double charging, forged callbacks, cross-tenant payment access, provider confusion and unsafe retries.

### Trust model

Threat boundaries include the client, payment adapter, provider callback surface, credential store, reconciliation worker and support tooling. Provider signatures/authentication are mandatory where the provider contract requires them. Payment credentials are secret material and must remain outside ordinary application telemetry. Callback identity must be verified before state mutation.

### Required controls

- SC-001 authentication/session/device state
- SC-002 tenant/org/branch isolation
- SC-003 fail-closed authorization
- SC-004 API/input security
- SC-005 secrets/crypto lifecycle
- SC-006 financial integrity
- SC-008 external callbacks/webhooks
- SC-009 audit evidence
- SC-010 resource limits
- SC-012 privileged support/admin access

### Security tests

```text
duplicate initiation
same idempotency key / changed amount
invalid callback signature
wrong merchant account
wrong currency
wrong payment intent
cross-tenant callback
late success after local cancellation
provider timeout
timeout-after-side-effect
duplicate callback
callback reordering
refund replay
expired credential
credential rotation mismatch
provider status regression
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

Implement payment orchestration and reconciliation as durable application workflows without coupling provider availability to unrelated product operations.

### Mandatory sequence

1. typed payment value objects and identifiers;
2. durable intent creation;
3. adapter interface with explicit timeout/error taxonomy;
4. attempt persistence and provider idempotency mapping;
5. callback ingestion and verification;
6. reconciliation decision engine;
7. worker/lease/retry processing;
8. refund/correction linkage;
9. audit/outbox integration;
10. operational reconciliation views and evidence.

### Implementation rules

No generic “retry payment” helper may create a second provider transaction without consulting the provider's idempotency/status semantics. Every external call has bounded timeout, retry budget and failure classification. Unknown outcomes are first-class application states. Reconciliation decisions are attributable to a policy/version and operator where manual intervention occurs.

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

Provider timeout, DNS/network failure, TLS/authentication failure, provider 4xx/5xx, malformed provider response, callback delay, callback duplication, callback signature failure, provider partial outage, credential rotation mismatch, database failure after local acceptance, worker crash, response loss, reconciliation disagreement, amount/currency mismatch.

### Concurrency model

Race families include duplicate initiation, concurrent reconciliation of the same intent, callback versus status poll, refund versus settlement, support action versus worker action, and credential rotation during provider execution. Use stable lock/claim ordering and durable unique keys rather than process-local mutexes.

### Adversarial tests

Manipulate amount/currency/merchant account; forge provider references; replay callbacks; submit oversized callback bodies; attempt cross-tenant payment lookup; alter client-supplied payment state; exploit support access; force retry storms; supply provider payloads with unexpected fields or deep nesting.

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

Bounded metrics: payment attempts by outcome class, unknown outcome age, reconciliation backlog, callback verification failures, provider latency/error class, refund backlog, credential failures. Trace correlation must never contain raw payment instruments, credentials or signatures.

### Operational controls

Provider outage must degrade the payment capability without corrupting sale history. Reconciliation backlog requires explicit thresholds, alert owner, queue capacity and manual-review path. Operational dashboards distinguish requested, observed and reconciled states.

### Recovery

Recover provider references, pending intents and reconciliation backlog from durable storage. Re-run reconciliation idempotently. Do not recreate external transactions unless the provider contract proves the prior attempt did not execute.

### Runbooks

- provider timeout/unknown outcome;
- callback verification incident;
- reconciliation mismatch;
- duplicate-payment investigation;
- credential rotation failure;
- refund discrepancy;
- provider-wide outage.

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

Phase 10 sale finalization creates the appropriate local payment facts; Phase 15 tax flows consume validated payment/sale state where required; Phase 16 reports consume reconciled facts; Phase 17 billing uses its own commercial billing authority and must not reuse merchant payment semantics without an explicit boundary.

### Evidence package

- provider contract fixture versions;
- idempotency/replay tests;
- callback negative tests;
- reconciliation mismatch fixtures;
- PostgreSQL integration evidence;
- worker retry/lease evidence;
- credential rotation evidence;
- audit/outbox evidence;
- failure-injection results;
- semantic audit matrix.

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

The evidence package MUST prove a complete lifecycle: intent → attempt → provider observation → reconciliation, including unknown outcome, duplicate callback and recovery cases.

### Human approval boundary

Human review is required for provider contract changes, financial-account configuration, manual reconciliation overrides, refund exceptions and any production credential or merchant-account change.

### Definition of done

A payment capability is complete only when it is durably modeled, idempotent, provider-correlated, reconciliation-safe, tenant-isolated, auditable, observable, recoverable and verified under adverse outcomes.

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

**End of Phase 11 contract.**
