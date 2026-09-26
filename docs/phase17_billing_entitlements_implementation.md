# Sitolo — Phase 17: Billing / Entitlements Implementation Specification

**Document:** `phase17_billing_entitlements_implementation.md`  
**Phase:** Phase 17 — Billing / Entitlements  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 17 implements Sitolo's own commercial billing and entitlement authority as a separate bounded system. Subscription state may control access to features according to approved commercial policy, but it must never rewrite merchant sales, inventory, payment history or tax facts.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 17: Billing / Entitlements
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

Primary internal sources: `docs/commercial/06_payments_finance/payment_collections_and_subscription_billing.md`, `docs/commercial/01_strategy/business_model_design.md`, `docs/domain_model.md`, `docs/api_contract.md`, `docs/database_design.md`, `docs/security_implementation_spec.md`, `docs/implementation_plan.md`. External payment providers are governed by their current contracts. Standards basis: ISO/IEC 27001:2022, ISO 37301:2021, NIST CSF 2.0, NIST SSDF 1.1 and OWASP ASVS 5.0.0.

---

## Part 01 — Select / Reconcile

### Objective

Reconcile commercial subscription rules with engineering authority boundaries and distinguish Sitolo subscription billing from merchant operational payments.

### Required inputs

- canonical commercial billing model
- domain model
- API/database contracts
- Phase 11 payment/reconciliation contract
- security/control register
- implementation plan

### Work

Identify plan catalog, subscription lifecycle, invoices, billing periods, payment attempts, credits/adjustments, reconciliation, entitlement states, grace periods, suspension rules and manual support actions. Record which decisions are product/commercial authority and which are engineering invariants.

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

Define separate state machines for subscriptions, invoices, billing attempts and entitlements.

### Canonical state model

Subscription: TRIAL / ACTIVE / PAST_DUE / GRACE / SUSPENDED / CANCELLED / EXPIRED. Invoice: DRAFT / ISSUED / DUE / PAID / PARTIAL / VOID / UNCOLLECTIBLE. Entitlement: ACTIVE / DEGRADED / SUSPENDED / EXPIRED. Billing attempt: CREATED / SENT / UNKNOWN / CONFIRMED / FAILED / REQUIRES_RECONCILIATION.

### Invariants

1. Merchant economic history is immutable regardless of subscription state.
2. Entitlements derive from authoritative subscription policy, not client claims.
3. A billing retry cannot silently duplicate provider-side payment.
4. Invoice identity is durable and idempotent.
5. Grace/suspension policy is deterministic and versioned.
6. Manual entitlement override is explicit, scoped, time-bounded and audited.
7. Plan changes preserve historical billing meaning.
8. Cross-tenant subscription state is impossible.

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

Persist subscription, invoice, usage and entitlement authority with durable billing idempotency.

### Persistence authority

Use separate billing/subscription records with explicit tenant/customer scope, plan/version, period, invoice identifiers, provider references, payment state, entitlement projection and adjustment history. Usage is derived from authoritative events, not client-reported counters.

### Transaction rules

Subscription changes, invoice issuance and entitlement recomputation use local transactions. External payment calls happen outside those transactions, with durable attempt records before/after provider interaction as required.

### Migration and compatibility

Plan/version changes use immutable catalog entries. Existing subscriptions retain their applied plan version. Entitlement policy changes require compatibility analysis and explicit rollout/rollback behavior.

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

Prevent unauthorized entitlement grants, billing manipulation, cross-tenant access and provider-side duplicate charges.

### Trust model

Billing is a financial and authorization control plane. Tenant scope, admin support access, adjustment authority and entitlement mutations are separate security domains. Highly privileged plan/price changes require approval and audit.

### Required controls

SC-002, SC-003, SC-005, SC-006, SC-009, SC-010, SC-012.

### Security tests

```text
cross-tenant invoice access
client-forged entitlement
plan-version tampering
billing-period overlap
invoice duplication
provider retry duplication
manual entitlement without expiry
support scope bypass
usage counter forgery
expired grace period bypass
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

Implement billing and entitlement processing as durable commercial workflows.

### Mandatory sequence

1. plan/version model;
2. customer subscription model;
3. invoice generation;
4. provider billing attempt adapter;
5. reconciliation;
6. entitlement projection;
7. usage metering from authoritative events;
8. grace/suspension scheduler;
9. support override workflow;
10. audit/reporting integration.

### Implementation rules

Never derive entitlement from a client toggle. Never hard-code commercial policy in UI or scattered handlers. Plan pricing, limits and grace rules are versioned data/config under approved governance.

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

payment timeout, duplicate invoice job, provider outage, invoice mismatch, subscription race, entitlement lag, usage pipeline delay, manual override expiration failure.

### Concurrency model

Test concurrent invoice issuance, payment retry and reconciliation, plan change at billing-period boundary, subscription cancellation versus renewal, and entitlement recalculation races.

### Adversarial tests

forge plan ID, modify invoice total, bypass suspension, reuse expired entitlement override, cross-tenant invoice enumeration, inflate usage.

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

invoice backlog, payment/reconciliation state, entitlement propagation latency, failed billing jobs, override count/age, usage ingestion lag and customer-impacting suspension errors.

### Operational controls

Billing incidents require explicit customer-impact classification. Entitlement outages must not mutate merchant transaction truth. Manual support operations require JIT access where policy dictates.

### Recovery

Rebuild entitlements from subscription/invoice policy history. Reconcile billing against provider evidence. Reverse incorrect entitlement changes explicitly rather than editing historical invoices.

### Runbooks

- billing provider outage;
- duplicate invoice;
- entitlement drift;
- past-due suspension incident;
- manual override expiration;
- billing reconciliation mismatch.

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

Phase 11 may supply provider/reconciliation mechanics, but Phase 17 owns Sitolo subscription authority. Phase 16 reports billing data. Phase 18 administers support workflows.

### Evidence package

- billing state-machine tests;
- idempotency tests;
- plan-version migration tests;
- entitlement security tests;
- provider reconciliation tests;
- support override audit evidence.

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

Prove plan → subscription → invoice → payment/reconciliation → entitlement lifecycle and show that merchant transaction history remains independent.

### Human approval boundary

Human approval is required for pricing/plan changes, production entitlement overrides, invoice corrections outside normal policy and provider-account changes.

### Definition of done

Phase 17 is complete only when subscription/billing state is independently reconcilable, entitlement decisions are authoritative, provider retries are safe and merchant truth is isolated.

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

**End of Phase 17 contract.**

# 12. Commercial Authority Matrix

| Decision | Owner | Engineering responsibility |
|---|---|---|
| plan definition | product/commercial governance | versioned data/config |
| price | approved commercial authority | deterministic application |
| subscription status | billing system | state transition enforcement |
| invoice | billing authority | durable identity |
| payment observation | provider/reconciliation | evidence ingestion |
| entitlement | billing policy | authoritative projection |
| exception | governance/support | scoped audited override |

Engineering must not invent commercial policy where the commercial corpus is authoritative.

# 13. Entitlement Evaluation

At request time:

~~~text
authenticated subject
→ tenant
→ subscription state/version
→ entitlement policy
→ feature/resource
→ effective decision
~~~

Cached entitlements are optimization only. High-risk operations must re-check current authority according to policy.

# 14. Billing Idempotency

All retryable billing operations need stable identity:

~~~text
business operation ID
+ billing period
+ tenant
+ attempt context
~~~

A recurring job may execute twice; it must not create two economic invoices or two provider payments for one logical operation.

# 15. Plan Migration

Plan changes use:

~~~text
new plan/version
→ eligibility calculation
→ effective date
→ subscription transition
→ entitlement recalculation
→ audit
~~~

Existing invoices retain the plan/version that produced them. Retroactive plan changes require an explicit correction policy.

# 16. Usage Metering

Usage events must originate from authoritative domain events. Client-side counters are hints at most.

Metering must define:

~~~text
event identity
deduplication
event timestamp semantics
attribution
tenant scope
aggregation window
late-event policy
correction policy
~~~

# 17. Customer-Impact States

Entitlement suspension must distinguish:

~~~text
billing delinquency
provider outage
internal processing failure
manual administrative action
security hold
~~~

A technical outage must not masquerade as commercial delinquency.

# 18. Capacity and Resource Controls

Bound invoice-generation batch size, usage-event ingestion rate, entitlement recomputation fan-out, reconciliation polling and notification volume. Apply backpressure rather than unbounded accumulation.

# 19. Evidence Ledger

Required evidence:

~~~text
commercial decision/version
→ billing state test
→ provider evidence
→ reconciliation result
→ entitlement result
→ audit record
~~~

# 20. No-Go Conditions

Stop if:

- a client can self-grant entitlement;
- provider retry can duplicate payment;
- invoice identity is non-durable;
- plan changes erase historical meaning;
- outage states are indistinguishable from delinquency;
- usage can be fabricated by the client.
