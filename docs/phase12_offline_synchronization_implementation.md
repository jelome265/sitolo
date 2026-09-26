# Sitolo — Phase 12: Offline Synchronization Implementation Specification

**Document:** `phase12_offline_synchronization_implementation.md`  
**Phase:** Phase 12 — Offline Synchronization  
**Product:** Sitolo — Business Operating System for African SMEs  
**Baseline:** 2026-09-26  
**Status:** Implementation-governing contract  
**Program authority:** `docs/implementation_plan.md`  
**Coverage authority:** `docs/phase_contract_coverage_register.md`

> This document is a phase-specific engineering contract. It does not replace higher-authority product, domain, security, database, API, provider, regulatory or commercial sources. Where this document conflicts with a higher-authority source, the higher-authority source wins and the conflict must be recorded and reconciled.

## 0. Executive contract

Phase 12 converts Sitolo's offline-first design from a distributed collection of client behaviors into a deterministic synchronization subsystem. The objective is bounded continuity, not client authority. Offline commands must be durable, attributable, replayable and safely ingested by the server without allowing replay, stale authority or conflict resolution to corrupt business truth.

## 1. Dependency position

```text
Previous phase(s)
     ↓
Phase 12: Offline Synchronization
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

Primary internal sources: `docs/sync_protocol.md`, `docs/domain_model.md`, `docs/database_design.md`, `docs/api_contract.md`, `docs/security_implementation_spec.md`, `docs/implementation_plan.md`, `docs/phase9_inventory_ledger_implementation.md`, `docs/phase10_pos_sales_implementation.md`. Standards basis includes ISO/IEC 27001:2022, NIST CSF 2.0, NIST SSDF 1.1, OWASP ASVS 5.0.0 and secure mobile/application engineering guidance. The client is never final authority.

---

## Part 01 — Select / Reconcile

### Objective

Reconcile the synchronization protocol with identity/device trust, inventory/sales authority and the exact offline commands permitted by product policy.

### Required inputs

- sync_protocol.md
- phase3 identity/device contract
- phase6 authorization contract
- phase9 inventory contract
- phase10 POS contract
- database_design.md
- mobile/desktop client architecture

### Work

Inventory every command capable of being created offline. For each command identify authority level, required device state, maximum offline age, replay key, conflict class, server-side validation, acknowledgement semantics and recovery behavior. Explicitly identify operations prohibited offline.

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

Define the durable client command lifecycle, server-ingestion lifecycle and conflict taxonomy.

### Canonical state model

```text
LOCAL COMMAND
DRAFT → COMMITTED → QUEUED → UPLOADING → ACKED
                         ↘ FAILED / BLOCKED / CONFLICT

SERVER INGESTION
RECEIVED → AUTHENTICATED → VALIDATED → IDEMPOTENCY_CHECK
          → APPLIED → ACKED
          ↘ REJECTED / CONFLICT / REFRESH_REQUIRED

CHECKPOINT
PROPOSED → DURABLY_ACCEPTED → ADVANCED
```
Conflicts are classified as commutative, mergeable, reject-and-refresh, approval-required or server-authoritative.

### Invariants

1. Checkpoints advance only after durable acceptance.
2. Duplicate command delivery is safe.
3. A revoked device cannot gain authority from stale local state.
4. Offline state never becomes a new server source of truth.
5. Money, inventory and approval conflicts are not resolved by generic last-write-wins.
6. Command identity is stable across retries and process death.
7. Tenant and branch scope are validated at ingestion.
8. Schema/version incompatibility fails explicitly and safely.
9. Local deletion does not imply server deletion of authoritative facts.
10. Acknowledgement represents server disposition, not merely receipt.

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

Define the local SQLite continuity model and server ingestion persistence without coupling client storage directly to server authority.

### Persistence authority

Client-local state includes command journal, local identifiers, device metadata, snapshots, acknowledgements and checkpoints. Server persistence stores accepted command identity, ingestion disposition, sync cursor/checkpoint evidence, conflict outcomes and any domain state changes. PostgreSQL remains authoritative. Client SQLite is durable continuity state only.

### Transaction rules

A server ingestion transaction authenticates the device/session, establishes trusted scope, validates command version and idempotency, executes the domain operation, records the durable disposition and produces an acknowledgement. Checkpoint advancement is atomic with durable acceptance. Client-side command commit must survive application/process death before upload.

### Migration and compatibility

Define command schema versions and compatibility windows. Expand server ingestion before requiring a new version. Support explicit rejection with refresh/upgrade instructions. Do not infer semantics from unknown fields when they affect money, stock or authority.

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

Prevent replay, stale-authority execution, tenant confusion, command tampering, checkpoint corruption and device-revocation bypass.

### Trust model

Offline security is based on authenticated device identity, server-side authorization and bounded capability. Local secrets use the approved device security model. Commands should carry integrity/authentication evidence where defined by the sync protocol, but server-side trust checks remain mandatory. A locally stored permission snapshot is advisory context, not final authority.

### Required controls

- SC-001 identity/session/device
- SC-002 tenant isolation
- SC-003 authorization
- SC-004 boundary/input security
- SC-005 key/secret lifecycle
- SC-006 financial/inventory integrity
- SC-007 offline/sync security
- SC-009 durable audit evidence
- SC-010 resource limits

### Security tests

```text
duplicate command
replayed command after acknowledgement
checkpoint advance before acceptance
modified command payload
changed tenant/branch selector
revoked device replay
downgraded session
expired capability
schema-version mismatch
oversized batch
cross-device command reuse
out-of-order delivery
partial batch failure
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

Implement a deterministic sync engine shared by mobile and desktop where appropriate, with clear client/server responsibilities.

### Mandatory sequence

1. typed command envelope;
2. local journal durability;
3. command identity/idempotency mapping;
4. upload batching with bounded sizes;
5. server ingestion pipeline;
6. acknowledgement model;
7. checkpoint persistence;
8. conflict classification/resolution paths;
9. revocation and security-version invalidation;
10. retry/backoff and observability.

### Implementation rules

No network retry may recreate a semantically new command. Upload batches are bounded by item count, bytes and execution budget. Backpressure is explicit. The client must expose synchronization states rather than fabricate success. Server responses are authoritative for disposition.

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

process death, local database failure, network interruption, upload timeout, response loss, duplicate batch, partial batch acceptance, server restart, schema mismatch, device revocation, authentication expiration, corrupted local journal, exhausted retry budget, conflict storm.

### Concurrency model

Test concurrent uploads from the same device, the same command through multiple processes, and commands that touch the same resource from different devices. Server deduplication and transaction isolation must be authoritative; do not rely on mobile process serialization.

### Adversarial tests

tamper with local timestamps, command IDs, tenant IDs, branch IDs and capability metadata; replay old commands; flood synchronization endpoints; attempt cross-device replay; submit deeply nested/oversized payloads; exploit stale authorization snapshots.

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

Track queue depth, age of oldest command, acceptance/rejection/conflict counts, checkpoint lag, replay detection, device revocation blocks, batch size/latency and schema mismatch rates. Do not log raw commands when they contain sensitive business/customer data.

### Operational controls

Provide bounded queues, retry budgets, dead-letter/manual-review handling where needed, device revocation propagation and client recovery diagnostics. Sync health must be distinguishable from API health.

### Recovery

Recover from durable command journal and server idempotency records. Re-drive unresolved commands according to disposition, never from an assumed client state. Rebuild client snapshots from authoritative server state after severe divergence.

### Runbooks

- checkpoint corruption;
- sync backlog growth;
- device revocation incident;
- schema mismatch rollout;
- repeated conflict storm;
- local database corruption;
- duplicate-command incident.

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

Phase 10 produces offline-capable sale commands subject to the offline authority rules. Phase 9 defines inventory consequences. Phase 11 handles payment reconciliation after server acceptance. Client surfaces must keep those boundaries visible.

### Evidence package

- process-death durability tests;
- replay/idempotency tests;
- revocation tests;
- schema compatibility tests;
- batch-size/resource tests;
- real PostgreSQL ingestion tests;
- conflict fixtures;
- offline/online transition evidence.

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

Evidence must demonstrate offline command durability across process death, duplicate transport, server retry, revocation, conflict and checkpoint recovery.

### Human approval boundary

Human approval is required for changes to offline authority, capability duration, conflict semantics for financial/inventory commands and schema compatibility policy.

### Definition of done

Phase 12 is complete only when offline capability is bounded, durable, deterministic, replay-safe, revocation-aware, resource-bounded and provable under interruption.

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

**End of Phase 12 contract.**

# 12. Command Envelope Contract

The canonical synchronization envelope must carry enough information to establish identity and replay semantics without embedding authority claims:

```text
command_id
command_type
schema_version
device_id
session/security-version context where defined
tenant/org/branch scope identifiers
created_at
sequence/ordering metadata where defined
payload
integrity/authentication evidence where required
```

The server must derive authoritative tenant, membership, permission and resource state from trusted context.

# 13. Conflict Decision Table

| Conflict class | Default behavior | Authority |
|---|---|---|
| commutative | accept once per command ID | server |
| mergeable | deterministic merge under domain rule | server/domain |
| reject-and-refresh | reject and return current state/version | server |
| approval-required | queue for explicit approval | policy |
| server-authoritative | discard conflicting client mutation | server |

Money, inventory, authorization and approvals must never fall through to generic last-write-wins.

# 14. Batch Processing Contract

Every batch has bounded:

```text
maximum commands
maximum encoded bytes
maximum execution time
maximum database work
maximum retry attempts
```

Batch failure semantics must distinguish:

```whole-batch rejected
partial acceptance
per-command conflict
per-command duplicate
transient server failure
schema incompatibility
```

The acknowledgement model must allow the client to know which commands were accepted and which require action.

# 15. Checkpoint Safety

Checkpoint movement is a monotonic protocol:

```text
client proposes checkpoint C
        ↓
server durably proves acceptance through C
        ↓
server returns acknowledgement
        ↓
client persists C
```

The client must never advance a checkpoint solely because upload HTTP completed. If response state is lost, the client retries from the previous durable checkpoint and relies on server idempotency.

# 16. Device Revocation Propagation

Device revocation must invalidate all authority that depends on the device:

```text
device state
→ session/capability validity
→ pending command acceptance
→ queued sync work
→ future uploads
```

The server must evaluate revocation against current trusted state during ingestion. A command created before revocation is not automatically entitled to execute after revocation.

# 17. Protocol-Version Migration

Protocol evolution is a security and correctness boundary.

Required sequence:

```text
publish compatible reader
→ accept old/new versions during window
→ migrate clients
→ observe
→ tighten accepted versions
→ retire old version
```

A new schema version cannot silently redefine an old command's meaning.

# 18. Local-Data Security

Mobile and desktop local storage is a continuity boundary, not a server authority.

Protect:

```text
session material
device credentials
command integrity evidence
customer-sensitive local state
payment-related references
tax credentials
```

Store only what is necessary. Local database backups and diagnostic exports must follow the same data-classification rules.

# 19. Synchronization Pressure Model

Measure:

```text
commands/device
commands/second
batch bytes
queue age
conflict rate
checkpoint lag
retry amplification
database work/command
```

Define thresholds before load testing. A client must receive an explicit backpressure state rather than indefinitely accumulating an unbounded command queue.

# 20. Deep Failure Catalogue

- process death during local commit;
- process death after upload before acknowledgement;
- acknowledgement loss;
- server crash during command transaction;
- command duplicated across processes;
- same command uploaded from another device;
- revoked device with queued commands;
- schema version unsupported;
- partial batch acceptance;
- checkpoint persisted before acknowledgement;
- local database corruption;
- clock skew;
- stale snapshot used to construct a financially invalid command.

# 21. Evidence Ledger

For every synchronization property record:

```text
protocol requirement
fixture
device/client build
server revision
database version
fault injection
expected disposition
observed disposition
artifact
reviewer
```

# 22. No-Go Conditions

Stop implementation if:

- command identity cannot survive process death;
- checkpoint advancement is not tied to durable server acceptance;
- revoked devices can continue authoritative ingestion;
- financial conflicts rely on last-write-wins;
- partial acceptance cannot be represented;
- resource bounds are absent;
- schema incompatibility can be silently accepted.

# 23. Downstream Handoff

Phase 12 exports synchronization evidence and accepted-command semantics to later product phases but does not create new business authority. Phase 20 consumes its replay/recovery evidence for certification.
