# Sitolo — Phase 18: Admin / Support Implementation Specification

**Document:** `phase18_admin_support_implementation.md`  
**Phase:** Phase 18 — Admin / Support  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 18 creates the platform-operations control plane without creating a hidden superuser. Support, platform administration, incident response and break-glass operations must be explicit authorization domains with bounded scope, purpose, expiry, heightened audit and post-use review.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 18: Admin / Support
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

Primary internal sources: `docs/security_control_register.md`, `docs/security_incident_response.md`, `docs/security_exception_register.md`, `docs/security_cryptography_and_key_management.md`, `agent.md`, `docs/implementation_plan.md`, `docs/api_contract.md` and operational contracts. Governance references include ISO/IEC 27001:2022, ISO/IEC 38500:2024, ISO 37301:2021, NIST CSF 2.0 and IIA Three Lines concepts. These do not create certification claims.

---

## Part 01 — Select / Reconcile

### Objective

Reconcile support, administrator, incident and break-glass authority with the existing tenant/IAM model.

### Required inputs

- security control register
- incident response contract
- exception register
- crypto/key-management contract
- Phase 4 IAM contract
- implementation plan
- current operational tools

### Work

Inventory every privileged administrative function: tenant operations, support search, JIT access, device revocation, queue inspection, payment/tax exceptions, audit review, credential rotation and incident controls. For each define actor, purpose, scope, duration, approval and evidence.

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

Define support-access and privileged-operation state transitions.

### Canonical state model

Support grant: REQUESTED / APPROVED / ACTIVE / EXPIRED / REVOKED / REVIEW_REQUIRED. Break-glass: DECLARED / ACTIVE / EXPIRED / REVOKED / REVIEWED. Incident privilege: OPEN / CONTAINED / CLOSED / POST_REVIEW.

### Invariants

1. No implicit tenant-global support access.
2. Every privileged session has purpose, scope, actor and expiry.
3. Break-glass is exceptional, not a normal bypass.
4. Support mutations use explicit authorization distinct from merchant IAM.
5. Credential rotation is auditable.
6. Privileged search/export is bounded and logged.
7. Expired/revoked access cannot continue.
8. Post-use review is mandatory where policy requires.

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

Persist privileged access grants, support actions, approvals, incident links and audit evidence.

### Persistence authority

Support grants reference operator, target scope, purpose, policy/approval, start/expiry, capabilities and revocation. Actions are attributable and append-oriented. Sensitive access evidence is separated from ordinary application audit where required.

### Transaction rules

Grant/approval/revocation and privileged mutations commit atomically with audit evidence. Break-glass declaration must persist before use. Credential rotation follows the crypto/key-management lifecycle.

### Migration and compatibility

Administrative records are security evidence. Retention/deletion must respect regulatory/legal requirements. Never migrate away actor/approval/expiry fields needed to reconstruct privileged use.

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

Prevent privilege escalation, support abuse, hidden bypasses and credential misuse.

### Trust model

Use strong authentication, step-up/MFA, least privilege, explicit scope and short lifetimes. Support tools must not accept client-provided tenant scope as authority. Platform-wide actions require separate policy. Sensitive search/export is itself an audited operation.

### Required controls

SC-001, SC-002, SC-003, SC-005, SC-009, SC-010, SC-012.

### Security tests

```text
expired support grant
revoked support session
cross-tenant search
support-to-production escalation
break-glass without reason
break-glass after expiry
missing approval
operator self-approval
credential rotation without audit
privileged export abuse
impersonation path
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

Implement an explicit admin/support control plane around the existing authorization engine rather than adding bypass APIs.

### Mandatory sequence

1. privileged operation catalog;
2. support-grant model;
3. approval and JIT workflow;
4. scoped support sessions;
5. break-glass workflow;
6. administrative actions;
7. incident integration;
8. credential rotation workflows;
9. audit/review tooling;
10. operational dashboards.

### Implementation rules

Do not create a `superuser=true` shortcut that bypasses tenant or object authority. Every privileged operation declares scope and policy. Support data access is minimized and should prefer masked/search-limited views.

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

approval service unavailable, grant expiry race, operator session revocation, credential rotation failure, incident closure race, queue inspection overload, audit-write failure.

### Concurrency model

Exercise grant revocation during active session, expiry during mutation, two approvers racing, duplicate break-glass declaration, credential rotation versus admin operation and incident closure versus ongoing response.

### Adversarial tests

forge support scope, alter expiry, reuse revoked grant, enumerate tenants, create hidden admin route, abuse bulk search, bypass approval through internal worker.

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

active support grants, privileged action counts, grant expiries, revocations, break-glass activations, failed privilege checks, credential rotation results and review backlog.

### Operational controls

Security operations owns the privileged-access lifecycle. High-risk operations have clear on-call ownership and escalation. Break-glass must trigger review workflows.

### Recovery

Revoke all affected grants/sessions on suspected compromise. Reconcile credential state. Preserve security evidence before remediation. Re-run privilege regression tests after correction.

### Runbooks

- suspected privilege escalation;
- support credential compromise;
- break-glass incident;
- JIT grant failure;
- emergency credential rotation;
- audit trail integrity issue.

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

Phase 18 consumes Phase 4/6 IAM controls, Phase 11/15 exception workflows and Phase 19/20 operational readiness. It does not replace the domain security model.

### Evidence package

- privileged-action matrix;
- JIT lifecycle tests;
- break-glass tests;
- session revocation evidence;
- support tenant-isolation tests;
- credential rotation evidence;
- post-use review records.

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

Every privileged capability must have attributable actor, explicit authority, scope, duration, action evidence and review status.

### Human approval boundary

Human approval is mandatory for production break-glass activation, irreversible administrative actions, global scope grants and security-credential changes.

### Definition of done

Phase 18 is complete only when platform administration and support are explicit, bounded, auditable, revocable and cannot silently bypass merchant security boundaries.

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

**End of Phase 18 contract.**

# 12. Privileged Operation Catalog

Every admin/support operation must have machine-readable metadata:

~~~text
operation ID
actor class
target resource
required authentication assurance
permission
tenant/global scope
maximum scope
approval requirement
maximum duration
audit class
data sensitivity
reversibility
incident linkage
~~~

Operations without an inventory entry are not production-ready.

# 13. JIT Access Contract

~~~text
request
→ reason
→ target scope
→ capability
→ approval
→ activation
→ expiry/revocation
→ post-use review
~~~

A support session cannot extend itself. Renewal is a new authorization decision.

# 14. Break-Glass Contract

Break-glass is used only when normal authority paths are unavailable or insufficient for incident response.

It requires:

~~~text
strong identity
explicit reason
specific incident/reference
minimal scope
short expiry
heightened audit
automatic review trigger
~~~

Break-glass does not remove the requirement to preserve evidence.

# 15. Data Access Minimization

Support interfaces should prefer:

~~~text
masked fields
narrow search
redacted payloads
bounded result counts
purpose-specific views
~~~

Raw production data is not a default support capability.

# 16. Privileged Mutation Safety

Administrative mutation must use the same business invariants as ordinary product operations unless an explicit, governed repair operation exists.

A support operator cannot bypass a financial invariant simply because the interface is internal.

# 17. Audit Integrity

Privileged evidence must connect:

~~~text
operator
→ grant
→ reason
→ target
→ action
→ result
→ timestamp
→ session
→ review
~~~

Failure to record required privileged evidence must fail the sensitive operation where policy requires it.

# 18. Resource Controls

Bound tenant search size, bulk operations, queue inspection, audit queries, credential rotation jobs and support session concurrency.

# 19. Evidence Ledger

Evidence includes:

~~~text
privileged operation inventory
JIT lifecycle tests
break-glass tests
scope-isolation tests
session revocation tests
audit completeness tests
post-use review evidence
credential rotation evidence
~~~

# 20. No-Go Conditions

Do not proceed if:

- a hidden global admin path exists;
- support scope can be client-selected;
- break-glass has no expiry;
- privileged actions lack durable audit;
- an internal tool bypasses domain invariants;
- expired grants remain usable.
