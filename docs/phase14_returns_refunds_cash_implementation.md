# Sitolo — Phase 14: Returns / Refunds / Cash Implementation Specification

**Document:** `phase14_returns_refunds_cash_implementation.md`  
**Phase:** Phase 14 — Returns / Refunds / Cash  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 14 implements financial correction workflows without destroying the original sale, inventory or cash history. Void, reversal, return, refund and cash variance are different facts with different authority, timing and evidence requirements.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 14: Returns / Refunds / Cash
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

Primary internal sources: `docs/domain_model.md`, `docs/api_contract.md`, `docs/database_design.md`, `docs/phase10_pos_sales_implementation.md`, `docs/phase9_inventory_ledger_implementation.md`, `docs/payment_integration_spec.md`, `agent.md`, `docs/implementation_plan.md`. Security references include the control register, incident response and exception register. Standards basis includes ISO/IEC 27001:2022, NIST CSF 2.0, NIST SSDF 1.1 and OWASP ASVS 5.0.0.

---

## Part 01 — Select / Reconcile

### Objective

Reconcile correction semantics across sales, inventory, payments and cash without allowing destructive edits.

### Required inputs

- domain_model.md
- phase10 POS contract
- phase9 inventory contract
- payment integration
- database/API contracts
- security control register
- current cash/register model

### Work

For each correction type define eligibility, timing, authority, amount/quantity limits, payment interaction, inventory interaction, tax interaction, audit evidence and retry semantics. Explicitly identify whether the operation can occur offline.

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

Define separate state machines for returns, refunds, reversals and register sessions.

### Canonical state model

Return: REQUESTED / APPROVED / RECEIVED / POSTED / REJECTED. Refund: REQUESTED / AUTHORIZED / INITIATED / UNKNOWN / CONFIRMED / FAILED / MANUAL_REVIEW. Register: OPENING / ACTIVE / COUNT_PENDING / CLOSED. Cash variance: OPEN / REVIEW / RESOLVED.

### Invariants

1. Original sale remains historical truth.
2. Correction entitlement is bounded by original economic facts and prior corrections.
3. A return cannot create stock without a valid received/accepted return fact.
4. A refund cannot exceed refundable entitlement.
5. Refunds remain reconciliable to payment state.
6. Closed registers cannot silently reopen history.
7. Cash variances are explicit events, not overwritten balances.
8. Duplicate correction commands are safe.
9. Tenant/branch/register scope is enforced.
10. Tax correction is coordinated with Phase 15 rather than fabricated locally.

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

Materialize append-oriented correction records and cash-session persistence.

### Persistence authority

Persist return/refund/cash-event identities and links to original sale/payment/register facts. Use unique constraints for command/correction identifiers. Inventory correction remains ledger-backed. Cash sessions retain opening, movement, count and variance evidence.

### Transaction rules

Return posting coordinates eligibility, accepted quantities, inventory effect and audit atomically. Refund initiation records intent before external provider interaction. Cash close atomically records count, variance and closure evidence. Never call external payment systems inside the cash-close or refund business transaction.

### Migration and compatibility

Preserve original sale references and correction chains. Any historical correction migration must retain the derivation path from original fact to corrected state.

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

Prevent fraud, over-refund, double-return, cash tampering and unauthorized financial correction.

### Trust model

High-risk correction operations require explicit permission, object/property/state checks, assurance and approval according to policy. Support overrides use separate support authority. Register actions are branch/register-scoped.

### Required controls

SC-002, SC-003, SC-006, SC-009, SC-010, SC-012.

### Security tests

```text
refund beyond entitlement
refund replay
return replay
cross-branch return
wrong register
unauthorized void
double correction
self-approval
manual variance closure without evidence
support override outside scope
closed-register mutation
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

Implement correction workflows as additive, auditable business operations.

### Mandatory sequence

1. correction entitlement calculator;
2. return aggregate;
3. inventory return posting;
4. refund intent and provider handoff;
5. refund reconciliation integration;
6. register/cash event model;
7. close/count workflow;
8. variance resolution;
9. audit/outbox evidence;
10. tax-correction linkage.

### Implementation rules

Do not edit original sale amounts or payment results to model corrections. Every correction references what it corrects. Eligibility is re-evaluated from authoritative state. Manual override requires reason, actor, scope and evidence.

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

duplicate return/refund, provider timeout, cash close crash, wrong count, partial return, inventory rejection, payment already refunded, tax correction failure, support action race.

### Concurrency model

Test two refund attempts against the same sale, two return attempts against the same line, refund versus chargeback/settlement observation where supported, and two cash-close attempts.

### Adversarial tests

alter original sale ID, claim unrelated return entitlement, forge refund amount, bypass branch/register, replay manual override, use stale approval, submit huge item counts.

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

returns/refunds by outcome, pending refund age, manual-review backlog, register variance distribution, correction failures and unauthorized attempts. Keep metrics bounded and avoid exposing customer/payment secrets.

### Operational controls

Correction queues, refund unknown-state queues and cash-variance queues must have owners and alert policies. Register closure must be an operational checkpoint.

### Recovery

Recover from append-only correction and cash-event history. Reconcile unknown refunds using Phase 11. Rebuild cash session summaries from events rather than editing closed totals.

### Runbooks

- refund unknown outcome;
- duplicate refund;
- cash variance investigation;
- closed-register exception;
- inventory return mismatch;
- tax correction dependency failure.

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

Phase 10 supplies original sales; Phase 9 owns inventory effects; Phase 11 owns provider payment reconciliation; Phase 15 owns tax/EIS correction consequences; Phase 16 consumes reconciled data.

### Evidence package

- entitlement property tests;
- concurrent correction tests;
- real PostgreSQL cash-close tests;
- payment reconciliation fixtures;
- audit evidence;
- support-access negative tests.

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

Prove a full correction chain: original sale → return/void/reversal → refund/cash effect → reconciliation → reporting/tax consequence, with original history preserved.

### Human approval boundary

Human approval is required for thresholded refunds, manual variance resolution, exceptional correction, support override and policy changes affecting financial entitlement.

### Definition of done

Phase 14 is complete only when corrections are additive, entitlement-bounded, authorization-safe, payment/inventory reconciled and historical facts preserved.

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

**End of Phase 14 contract.**

# 12. Correction Entitlement Model

For each correction, calculate:

```text
original economic fact
− prior consumed correction entitlement
− disqualifying current state
= currently refundable/returnable entitlement
```

The calculation must be server-authoritative and recomputed at execution time.

# 13. Correction Chain

Every correction references what it corrects:

```text
sale
 ├─ void/reversal
 ├─ return
 │   └─ refund
 └─ cash consequence
```

The system never replaces the sale row with a corrected version.

# 14. Register/Cash State Matrix

| State | Allowed actions | Forbidden actions |
|---|---|---|
| OPENING | establish float/open evidence | finalize prior history |
| ACTIVE | sales/cash events | arbitrary balance rewrite |
| COUNT_PENDING | count and variance | new normal transactions where policy forbids |
| CLOSED | reporting/review | mutate prior event history |

A correction after close is a new governed event, not reopening history.

# 15. Refund/Provider Boundary

Refund initiation follows:

```text
local eligibility
→ authorization
→ durable refund intent
→ external provider call
→ provider evidence
→ reconciliation
```

Unknown provider outcome remains UNKNOWN/RECONCILIATION_REQUIRED. The cash and sale history must not be rewritten based on an uncertain provider response.

# 16. Tax Boundary

Phase 14 does not invent tax correction semantics.

Where a return/refund affects fiscalization, create the explicit Phase 15 work item with the local tax fact and correction reference. Do not alter prior tax evidence to make the correction appear as though it happened originally.

# 17. Fraud and Abuse Signals

The phase should emit bounded signals for:

```text
high refund frequency
repeated same-item returns
manual override frequency
cash variance outliers
refund attempts after prior refund
corrections outside normal time windows
operator concentration
support-mediated correction activity
```

Signals are evidence for investigation, not automatic proof of misconduct.

# 18. Performance and Resource Limits

Bound:

```text
eligible-line scan
return-line count
refund-attempt rate
cash-event query window
manual-review queue
correction search volume
```

Large historical correction searches must use indexed/read-model paths and cannot starve POS operations.

# 19. Failure Catalogue

- refund request duplicated;
- provider timeout;
- return accepted but inventory posting failed;
- cash close crash;
- count changed during close;
- refund entitlement changed between approval and execution;
- support override races with normal operator action;
- tax correction unavailable;
- already-corrected sale presented again.

# 20. Evidence Ledger

For each correction:

```text
original fact
eligibility calculation
authorization
approval if required
correction command
inventory/payment/cash effect
provider evidence where relevant
audit
outbox
final state
```

# 21. No-Go Conditions

Do not proceed if:

- the original sale can be destructively edited;
- correction entitlement is computed from client state;
- refund and inventory effects can diverge silently;
- register close can rewrite prior history;
- unknown refunds are forced into success/failure without evidence.

# 22. Downstream Handoff

Phase 14 provides Phase 15 the tax-relevant correction reference, Phase 16 the corrected-but-reconstructable reporting facts, and Phase 20 the correction-integrity evidence.
