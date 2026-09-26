# Sitolo — Phase 13: Procurement / Suppliers Implementation Specification

**Document:** `phase13_procurement_suppliers_implementation.md`  
**Phase:** Phase 13 — Procurement / Suppliers  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 13 implements inbound commercial intent and receipt evidence. A purchase order represents expected supply; a goods receipt represents what was actually received. Inventory increases only from authorized receipt evidence, not from merely approved expectation.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 13: Procurement / Suppliers
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

Primary internal sources: `docs/domain_model.md`, `docs/api_contract.md`, `docs/database_design.md`, `agent.md`, `docs/implementation_plan.md`, `docs/phase9_inventory_ledger_implementation.md`. Commercial supplier requirements, where present, remain product/commercial authority. Security controls remain governed by the security register. Standards basis: ISO/IEC 27001:2022, NIST CSF 2.0, NIST SSDF 1.1 and OWASP ASVS 5.0.0 as applicable.

---

## Part 01 — Select / Reconcile

### Objective

Reconcile supplier, purchasing and receiving concepts already present in the domain/data model with Phase 9 inventory authority.

### Required inputs

- domain_model.md
- api_contract.md
- database_design.md
- Phase 9 inventory contract
- implementation_plan.md
- commercial supplier requirements

### Work

Identify supplier identity, purchase order, approval, expected lines, receipt, discrepancy, lot/expiry capture, supplier return and correction semantics. Determine which roles may create, approve, receive and correct purchasing facts and where separation of duties applies.

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

Define lifecycle states for supplier records, purchase orders and receipts.

### Canonical state model

Supplier: ACTIVE / SUSPENDED / RETIRED. Purchase order: DRAFT / SUBMITTED / APPROVED / PARTIALLY_RECEIVED / RECEIVED / CANCELLED / CLOSED. Receipt: IN_PROGRESS / POSTED / DISPUTED / CORRECTED. Discrepancy: OPEN / INVESTIGATING / RESOLVED.

### Invariants

1. A receipt is evidence of actual received quantity.
2. A PO cannot silently become a receipt.
3. Inventory posting references the receipt and its accepted quantities.
4. Over-receipt requires explicit policy or approval.
5. Receipt duplication is prevented.
6. Supplier and purchase facts are tenant-scoped.
7. Historical receipt evidence is immutable except via explicit correction.
8. Lot/expiry data, when required for the SKU, cannot be silently omitted.
9. Approval cannot be self-approved where SoD applies.
10. Supplier returns reverse receipt/inventory effects explicitly.

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

Design authoritative procurement persistence and the transaction boundary connecting receipts to inventory.

### Persistence authority

Core data includes supplier identity, supplier terms/reference metadata, purchase order header/lines, approval evidence, receipt header/lines, discrepancy records and inventory posting references. Use scoped uniqueness and foreign keys. Inventory remains the ledger authority; procurement records explain why inbound stock was accepted.

### Transaction rules

PO approval is separate from receiving. Receipt posting validates scope, SKU/unit/quantity, approval state and duplicate receipt identity, then coordinates the inventory posting atomically. External supplier communication is not held inside the transaction.

### Migration and compatibility

Preserve supplier references and historical receipt identities. Any unit conversion change requires compatibility analysis with existing POs and receipt history. Avoid rewriting historical expected quantities merely to match later receipt reality.

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

Prevent unauthorized procurement, false receipt posting, supplier cross-tenant access and inventory inflation.

### Trust model

Receiving is a security-critical mutation because it changes stock. Supplier administration, PO approval and receipt posting must have distinct authorization decisions where policy requires. Client-supplied supplier/tenant/branch fields are not authority.

### Required controls

SC-002, SC-003, SC-004, SC-006, SC-009, SC-010, SC-012.

### Security tests

```text
cross-tenant supplier lookup
cross-tenant PO access
unauthorized approval
self-approval
receipt without approved PO where prohibited
over-receipt bypass
duplicate receipt
wrong SKU/unit
wrong warehouse/branch
forged supplier ID
receipt correction replay
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

Implement the procurement vertical without duplicating catalogue or inventory authority.

### Mandatory sequence

1. supplier aggregate/reference model;
2. PO creation/update rules;
3. approval decision;
4. receipt creation;
5. receipt validation;
6. inventory posting integration;
7. discrepancy workflow;
8. supplier return linkage;
9. audit/outbox evidence;
10. operational reporting hooks.

### Implementation rules

POs describe expected supply; receipts describe actual supply. Do not let a generic PATCH endpoint mutate financial/inventory facts. Corrections are additive and attributable. Receiving must reuse Phase 9 quantity/unit semantics.

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

duplicate PO submit, approval race, partial receipt, over-receipt, wrong supplier, inactive SKU, unit mismatch, database failure during posting, worker/event duplication, receipt correction conflict.

### Concurrency model

Exercise concurrent receipt attempts against the same PO line and stock item, concurrent approval/change, and concurrent supplier-return actions. Deterministic locks and database constraints must protect quantities.

### Adversarial tests

forge approval state, modify supplier IDs, bypass branch scope, submit negative/huge quantities, reuse receipt IDs, alter lot/expiry metadata after posting, replay supplier-return commands.

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

PO approval latency, receipt volume, discrepancy backlog, over-receipt attempts, receiving failures, supplier lookup errors, inventory posting failures; bounded by tenant/branch dimensions allowed by observability policy.

### Operational controls

Receiving failures must leave the PO/receipt and inventory state understandable. Discrepancy queues require ownership and SLA classes. Large imports or bulk receiving must be bounded and asynchronous where appropriate.

### Recovery

A failed receipt transaction must not partially increase inventory. If receipt acceptance succeeded but downstream notification failed, recover from durable outbox/evidence. Reconcile receipt-to-ledger links periodically.

### Runbooks

- duplicate receipt investigation;
- inventory/receipt mismatch;
- approval race;
- supplier return correction;
- bulk receiving resource exhaustion.

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

Phase 8 supplies SKU/unit identity; Phase 9 owns inventory posting; Phase 10 may consume available stock; Phase 14 owns supplier returns where they are modeled as correction/cash workflows.

### Evidence package

- PO state-transition tests;
- SoD tests;
- duplicate/over-receipt tests;
- real PostgreSQL constraint tests;
- inventory posting atomicity tests;
- lot/expiry tests;
- discrepancy workflow evidence.

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

Prove that every authoritative inventory increase caused by procurement has traceable supplier, PO, receipt and inventory evidence.

### Human approval boundary

Human approval is required for supplier master-data authority, approval thresholds, exceptional over-receipts, discrepancy resolution outside policy and destructive supplier changes.

### Definition of done

Phase 13 is complete only when inbound commercial intent and actual receipt evidence are distinct, authorized, tenant-safe, inventory-linked, auditable and recoverable.

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

**End of Phase 13 contract.**

# 12. Procurement Transaction Matrix

| Operation | Authority | Inventory effect | Required evidence |
|---|---|---|---|
| supplier create/update | tenant administration | none | audit |
| PO create | authorized purchaser | none | command |
| PO approve | approver policy | none | approval |
| receipt post | receiving authority | ledger increase | receipt + ledger |
| discrepancy open | receiver/manager | none | discrepancy |
| receipt correction | explicit correction authority | compensating ledger effect | correction chain |
| supplier return | authorized correction flow | ledger decrease | return + ledger |

# 13. Quantity and Unit Controls

Every receiving command must resolve:

```text
SKU
→ purchase unit
→ canonical stock unit
→ validated conversion
→ accepted quantity
```

Conversion factors must come from authoritative catalogue/unit state. The receiving path must not trust a client-supplied conversion factor.

Negative, zero or extreme quantities are handled according to domain policy and input bounds; “unexpected quantity” must be a typed failure rather than a silent clamp.

# 14. Approval and Separation of Duties

Where policy requires approval:

```text
requester ≠ approver
approver scope includes target
approval is bound to target/version
approval cannot be replayed
approval expiry is explicit
```

If the PO changes materially after approval, the prior approval must be invalidated or re-evaluated.

# 15. Receipt Posting Algorithm

```text
authenticate
→ derive tenant/branch scope
→ authorize receiving action
→ validate PO status
→ validate SKU/unit
→ validate quantity
→ validate duplicate receipt identity
→ record receipt
→ post inventory atomically
→ write audit/outbox
→ commit
```

If any step fails, neither the receipt nor inventory effect may be partially committed.

# 16. Discrepancy Control

A discrepancy is a first-class operational object, not an exception string.

It should preserve:

```text
expected quantity
received quantity
difference
reason
actor
timestamp
supplier
PO/receipt references
status
resolution
approval where needed
```

Discrepancy resolution must not rewrite the original receipt.

# 17. Supplier-Data Security

Supplier master data may contain bank/payment/contact information. Classification determines whether fields are visible to purchasers, receivers, support users or platform operators.

Bulk supplier export, when supported, must use the Phase 16 controlled export model rather than a custom privileged endpoint.

# 18. Performance Controls

Bound:

```
PO line count
receipt line count
search result size
supplier lookup rate
bulk-receive payload
discrepancy query window
inventory-posting transaction duration
```

Large receiving operations must use controlled batching and should not hold long transactions across external supplier calls.

# 19. Failure Catalogue

- approval revoked after receipt preparation;
- PO modified while receipt is being posted;
- same receipt submitted twice;
- partial receipt then process death;
- over-receipt;
- unit conversion changed between PO and receipt;
- inventory posting deadlock;
- outbox write failure;
- supplier record suspended during receipt;
- receipt correction racing with supplier return.

# 20. Evidence Ledger

Evidence should prove:

```text
PO state
→ approval state
→ receipt evidence
→ inventory ledger posting
→ discrepancy/reconciliation state
```

No inventory increase caused by procurement is accepted as complete without this chain.

# 21. No-Go Conditions

Stop at the relevant Part if:

- receipt and inventory posting are not atomic;
- approvals are not target/version bound;
- unit conversion is client-authoritative;
- duplicate receipt identity is not durable;
- historical receipt evidence can be destructively edited;
- cross-tenant supplier access is possible.

# 22. Downstream Handoff

Phase 13 exports receipt-to-inventory evidence to Phase 9-derived inventory truth and supplies correction inputs to Phase 14. Phase 16 consumes procurement reporting facts.
