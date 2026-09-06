# Sitolo — Implementation Plan

**Document:** `implementation_plan.md`  
**Phase:** 0 — Architecture / Contracts / ADR Freeze  
**File:** 13 of 16  
**Status:** Implementation baseline  
**Date:** 2026-09-05  
**Primary posture:** correctness, security, recoverability and operational evidence before feature breadth.

---

## 1. Purpose

This document converts the frozen Sitolo architecture and contract suite into an executable engineering program. It is not a feature wishlist and it is not a calendar. It is the dependency-aware implementation order that prevents later modules from inventing incompatible rules for authority, authorization, financial state, synchronization, observability or recovery.

The implementation program must preserve the core chain:

```text
IDENTITY
  ↓
ORGANIZATION / BRANCH / RESOURCE SCOPE
  ↓
CATALOGUE / PRICING
  ↓
PROCUREMENT / RECEIVING
  ↓
INVENTORY
  ↓
SALE
  ↓
PAYMENT / CASH
  ↓
RECONCILIATION
  ↓
TAX / MRA EIS
  ↓
REPORTING
```

Across that chain, the persistent controls are:

```text
AUTHORIZATION
TENANT ISOLATION
IDEMPOTENCY
AUDITABILITY
OFFLINE CONTINUITY
CONCURRENCY CONTROL
OBSERVABILITY
BACKUP / RECOVERY
```

The implementation plan therefore follows a strict rule:

> A feature is not complete because its happy path works. It is complete when its authority, invariants, failure modes, security controls, recovery path, tests and operational evidence are also implemented.

---

# 2. Source-of-Truth Hierarchy

When two artifacts disagree, resolve them deliberately rather than coding whichever statement is easier to implement.

Preferred hierarchy:

```text
1. Applicable law / regulator requirement
2. Current external provider contract
3. Approved product/business decision
4. Security architecture and security implementation contract
5. Domain model
6. Database / API / integration specifications
7. Testing / observability / deployment specifications
8. ADRs
9. Implementation convenience
```

Security invariants cannot be weakened merely because another document is older or implementation would be easier. Provider-specific behavior may override an earlier provider assumption, but it does not make client-side authority or tenant bypass acceptable.

---

# 3. Architecture Baseline

## 3.1 Backend

Primary backend:

```text
Rust
Axum
Tokio
SQLx
PostgreSQL
```

The backend begins as a modular monolith with independent worker and adapter boundaries.

The architecture is intentionally not microservice-first. A module may later become a separate service if extraction produces a real operational or scaling benefit and the domain boundary is already proven.

## 3.2 Clients

```text
Flutter → Android-first mobile
Tauri + TypeScript → desktop
TypeScript / Next.js → optional admin web
```

Clients are execution surfaces and continuity mechanisms. They are never the final authorization or financial authority.

## 3.3 Persistence

```text
PostgreSQL → authoritative server state
SQLite → durable device-local operational state
Object storage → controlled files/exports/backups where applicable
Redis → optional acceleration/non-authoritative state only
```

## 3.4 External systems

```text
Payment providers
MRA EIS
Notification providers
Identity provider, if selected
Object storage
```

Every external system is an explicit trust boundary with an adapter, timeout, failure taxonomy and reconciliation model.

---

# 4. Current Platform Anchors

## 4.1 Rust

Rust 1.98.1 is the approved current stable baseline as of 2026-09-05. The Rust project released 1.98.1 on 2026-09-03 to correct a 1.98.0 miscompilation involving trait-object vtable generation. The repository must therefore pin the toolchain instead of using an unpinned moving stable channel. [Official reference: Rust Release Team, 2026-09-03.]

Required consistency:

```text
rust-toolchain.toml
Cargo.lock
CI toolchain
builder image
local development environment
```

must resolve to the same approved baseline.

## 4.2 PostgreSQL

PostgreSQL 18.6 is the current 18.x minor release as of 2026-09-05. The PostgreSQL project released 18.6 on 2026-08-13 and recommends current supported minors. Sitolo therefore targets PostgreSQL 18.x with the currently approved minor in CI and production. [Official reference: PostgreSQL 18.6 release notes.]

Do not treat PostgreSQL 19 beta as a production baseline.

## 4.3 Supply chain

SLSA 1.2 is the current approved SLSA specification. Release provenance must identify the source revision, build environment and artifact identity.

## 4.4 Observability

OpenTelemetry Semantic Conventions 1.44.0 are the current published semantic-convention set used as the baseline for instrumentation. The implementation must pin and govern the version instead of assuming that convention meaning never changes.

---

# 5. Non-Negotiable Engineering Rules

1. PostgreSQL is authoritative for server-side business truth.
2. Tenant scope is resolved from trusted membership/security context.
3. Client-supplied tenant, role, amount or permission data is never authority.
4. Finalized financial facts are immutable.
5. Corrections are explicit compensating operations.
6. Inventory history is ledger-backed and explainable.
7. Financial side effects are idempotent.
8. External calls do not remain inside open business transactions.
9. Unknown outcomes become explicit states, not guessed successes.
10. Offline operation is bounded capability, not permanent authority.
11. Sync checkpoints advance only after durable acceptance.
12. Provider events are evidence and must be reconciled.
13. Security checks fail closed.
14. A scanner that did not execute is not a passing security gate.
15. Resource limits are mandatory on externally triggerable expensive work.
16. Support access is separate, explicit, time-bounded and audited.
17. Production artifacts must be reproducible and attributable to source.
18. Every material failure mode requires an operational response path.
19. Architectural changes to invariants require ADR review.
20. Production readiness is demonstrated by evidence, not document size or developer confidence.

---

# 6. Phase Dependency Model

The implementation order is:

```text
PHASE 0  Architecture / contracts / ADR freeze
   ↓
PHASE 1  Repository / Rust workspace / CI
   ↓
PHASE 2  Config / secrets / logging / errors / telemetry
   ↓
PHASE 3  Identity / sessions / MFA / devices
   ↓
PHASE 4  Tenant / org / branch / IAM
   ↓
PHASE 5  PostgreSQL / migrations / constraints / RLS
   ↓
PHASE 6  Authorization engine / policy enforcement
   ↓
PHASE 7  Security test framework
   ↓
PHASE 8  Product / catalogue / pricing
   ↓
PHASE 9  Inventory ledger
   ↓
PHASE 10 POS / sales
   ↓
PHASE 11 Payments / reconciliation
   ↓
PHASE 12 Offline synchronization
   ↓
PHASE 13 Procurement / suppliers
   ↓
PHASE 14 Returns / refunds / cash
   ↓
PHASE 15 MRA EIS
   ↓
PHASE 16 Reporting / exports
   ↓
PHASE 17 Billing / entitlements
   ↓
PHASE 18 Admin / support
   ↓
PHASE 19 Hardening / performance / DR
   ↓
PHASE 20 Production certification
```

Some foundation work can overlap when dependency-safe. No phase may silently bypass an unresolved upstream security or authority decision.

---

# 7. Phase 0 — Architecture / Contracts / ADR Freeze

## Objective

Create an internally consistent architectural contract before feature implementation expands.

## Required artifacts

The Phase 0 package consists of:

```text
01 security_implementation_spec.md
02 domain_model.md
03 database_design.md
04 api_contract.md
05 auth_authorization_spec.md
06 sync_protocol.md
07 payment_integration_spec.md
08 mra_eis_integration_spec.md
09 testing_strategy.md
10 threat_model.md
11 observability_spec.md
12 deployment_spec.md
13 implementation_plan.md
14 ADR-001 ... ADR-025
15 initial security-test harness
16 initial CI enforcement
```

## Work

- verify cross-document vocabulary;
- map bounded contexts;
- map authority boundaries;
- map transactions;
- map identifiers;
- map state machines;
- map external side effects;
- identify unresolved decisions;
- establish ADR ownership;
- establish release gates.

## Phase exit

```text
[ ] no hidden source of truth
[ ] no undocumented security boundary
[ ] business invariants identified
[ ] state machines identified
[ ] transaction boundaries identified
[ ] provider boundaries identified
[ ] open decisions recorded
[ ] release gates machine-enforceable
```

---

# 8. Phase 1 — Repository / Rust Workspace / CI

## Objective

Create a reproducible repository and build environment.

## Target structure

```text
sitolo/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── sitolo-domain/
│   ├── sitolo-application/
│   ├── sitolo-api/
│   ├── sitolo-persistence/
│   ├── sitolo-sync/
│   ├── sitolo-integrations/
│   ├── sitolo-jobs/
│   ├── sitolo-observability/
│   ├── sitolo-auth/
│   └── sitolo-cli/
├── migrations/
├── apps/
│   ├── mobile/
│   ├── desktop/
│   └── admin-web/
├── packages/
├── infra/
├── tests/
└── docs/
```

The exact crate count may be smaller initially. Do not create a crate for every noun.

## Dependency rule

```text
API
 ↓
Application
 ↓
Domain
```

Infrastructure implements ports exposed to application/domain layers.

Domain code must not import Axum, SQLx, Redis, provider HTTP implementations, Flutter or Tauri.

## CI foundation

Pull request minimum:

```text
format
lint
compile
unit tests
integration tests
dependency scan
secret scan
policy/license scan
```

Release candidate:

```text
property tests
security suite
contract tests
migration tests
fuzz smoke tests
container/image checks
SBOM
provenance
DAST staging
artifact verification
```

## Exit

A clean checkout produces the expected build graph with the pinned compiler and lockfile.

---

# 9. Phase 2 — Configuration / Secrets / Logging / Errors / Telemetry

## Objective

Establish the runtime primitives that every later feature consumes.

## Configuration

Separate:

```text
static build configuration
runtime configuration
secrets
feature flags
tenant/business configuration
provider configuration
```

Secrets never enter source, mobile bundles, desktop bundles, logs or test fixtures.

## Error taxonomy

Start with typed families:

```text
ValidationError
AuthorizationDenied
ResourceNotFound
StateConflict
InvariantViolation
ConcurrencyConflict
IdempotencyConflict
InsufficientStock
PaymentReconciliationRequired
ExternalDependencyUnavailable
ExternalDependencyRejected
TaxSubmissionRejected
ApprovalRequired
FeatureNotEntitled
DeviceRevoked
```

Transport responses remain stable even if internal provider/database errors change.

## Observability

Every request should have safe correlation data:

```text
trace_id
request_id
operation
outcome
latency
```

Do not log credentials, signatures, full financial payloads or unnecessary PII.

## Exit

The platform has one configuration model, one error model and one telemetry policy.

---

# 10. Phase 3 — Identity / Sessions / MFA / Device Identity

## Objective

Create trusted principal and device state.

## Implementation sequence

1. User identity integration.
2. Session creation/validation.
3. Session revocation.
4. MFA enrollment/verification.
5. Recovery flows.
6. Device registration.
7. Device credential/key management.
8. Device revocation.
9. Session-device binding.
10. Authentication assurance state.

## Security tests

```text
expired session
revoked session
revoked device
invalid MFA
MFA downgrade
replayed recovery
cross-tenant selection
stale membership
```

## Exit

An authenticated request can produce a trusted security context:

```text
principal
session
active device
membership state
assurance level
```

---

# 11. Phase 4 — Tenant / Organization / Branch / IAM

## Objective

Establish scoped business authority.

## Hierarchy

```text
Organization
 ├── business entity
 ├── memberships
 ├── roles
 ├── branches
 │    ├── locations
 │    ├── warehouses
 │    ├── registers
 │    └── devices
 └── configuration
```

## Baseline permissions

```text
sales:create
sales:void
sales:refund
inventory:adjust
inventory:transfer
purchase:approve
cash:close
payment:reconcile
user:manage
report:export
tax:configure
```

## Support

Support access is a separate authorization domain:

```text
purpose
scope
start
expiry
MFA
read/write capability
audit trail
```

## Exit

Cross-tenant and cross-branch access tests fail closed.

---

# 12. Phase 5 — PostgreSQL / Migrations / Constraints / RLS

## Objective

Materialize authoritative server state.

## Domains

```text
identity
tenant
auth
catalog
pricing
procurement
inventory
sales
cash
payment
reconciliation
tax
audit
reporting
billing
notification
platform
```

Vertical schemas are added only when their domain rules are sufficiently defined.

## Roles

```text
migration owner
runtime DML role
reporting read-only role
backup/recovery identity
break-glass identity
```

Runtime access must not casually include schema administration or RLS bypass authority.

## Constraints

Use SQL constraints for enforceable invariants:

```text
PRIMARY KEY
FOREIGN KEY
UNIQUE
CHECK
EXCLUSION where justified
```

## RLS validation

Test with the actual runtime role:

```text
cross-tenant select
cross-tenant insert
cross-tenant update
cross-tenant delete
missing tenant context
incorrect tenant context
support access exceptions
```

## Exit

Schema, migrations, roles, RLS and core transaction semantics are covered by executable evidence.

---

# 13. Phase 6 — Authorization Engine / Policy Enforcement

## Objective

Create one consistent policy evaluation path.

## Decision

```text
principal
+ tenant scope
+ branch/resource scope
+ permission
+ target resource
+ target state
+ amount/context
= authorization decision
```

## Enforcement layers

```text
authentication
membership
permission
scope
object authorization
property authorization
state authorization
approval / separation of duties
DB defense in depth
```

## High-risk actions

Require explicit policy evaluation:

```text
large refund
high-value discount
stock adjustment
role change
MFA reset
payment credential change
bulk export
tenant ownership change
support mutation
```

## Exit

Sensitive operations cannot be enabled by UI state alone and cannot rely on duplicated ad-hoc role logic.

---

# 14. Phase 7 — Security Test Framework

## Objective

Make security regression testing a platform capability.

## Fixtures

```text
Tenant A
Tenant B
Branch A1
Branch A2
Owner
Manager
Cashier
Inventory user
Support user
Active device
Revoked device
Active session
Expired session
```

## Negative suite

```text
BOLA / IDOR
cross-tenant
cross-branch
function bypass
property bypass
forged role
forged approval
replay
modified command
revoked device
expired session
oversized request
rate exhaustion
secret leakage
```

## Mutation targets

```text
remove tenant predicate
remove idempotency check
skip authorization
advance checkpoint early
ignore device revoke
disable signature verification
accept stale capability
```

## Exit

Security suites are locally reproducible and blocking in CI.

---

# 15. Phase 8 — Product / Catalogue / Pricing

## Objective

Implement current commercial definitions.

## Sequence

```text
Product
 ↓
SKU
 ↓
Unit model
 ↓
Barcode
 ↓
Price list/version
 ↓
Promotion
 ↓
Discount authorization
```

## Invariants

- tenant-scoped SKU ownership;
- valid units/conversion factors;
- barcode collision protection;
- historical product snapshots;
- deterministic price resolution;
- bounded promotion stacking;
- no accidental negative pricing.

## Tests

```text
duplicate barcode
invalid conversion
expired price
overlapping validity
promotion conflict
unauthorized override
historical price preservation
```

## Exit

A pricing decision can be reconstructed from the applied policy and historical inputs.

---

# 16. Phase 9 — Inventory Ledger

## Objective

Implement authoritative stock accounting.

## Core structures

```text
inventory movement
inventory balance projection
location
lot/batch
stock state
stock count
transfer
```

## Ledger rule

```text
receipt +Q
sale -Q
return +Q
supplier return -Q
transfer ±Q
adjustment ±Q
expiry -Q
count reconciliation ±Q
```

The projection accelerates reads. The ledger explains history.

## Concurrency test

Given:

```text
available = 1
```

run concurrent sale attempts.

Required result:

```text
one successful consumption
or one successful + one explicit policy-allowed alternative
never two successful consumption postings
never negative balance
```

The test runs against real PostgreSQL.

## Exit

Inventory remains explainable and consistent after concurrency, replay and recovery.

---

# 17. Phase 10 — POS / Sales

## Objective

Implement the atomic merchant sale workflow.

## Command model

```text
command_id
sale_id
branch_id
register_id
device_id
lines[]
pricing/tax inputs
tenders[]
customer reference
client timestamp
```

## Transaction

```text
BEGIN
 authenticate
 resolve tenant
 authorize sale
 validate branch/register/device
 validate catalogue
 validate price
 validate stock
 validate tender semantics
 create immutable sale
 post inventory
 post cash/payment facts
 create tax pending work if required
 write audit
 write outbox
COMMIT
```

External HTTP must not occur inside this transaction.

## Critical tests

```text
duplicate command
semantic mismatch on reused command ID
insufficient stock
inactive SKU
wrong register
invalid total
crash-after-commit
response lost
concurrent final unit
```

## Exit

A completed sale has one authoritative local economic identity and can safely survive transport failure.

---

# 18. Phase 11 — Payments / Reconciliation

## Objective

Separate money intent from observed provider evidence and reconciliation decision.

## Model

```text
PaymentIntent
 ↓
PaymentAttempt
 ↓
Provider observation
 ↓
Reconciliation
```

## Unknown outcome

The critical state is:

```text
request sent
↓
transport timeout
↓
provider outcome unknown
```

Required handling:

```text
PENDING / UNKNOWN
↓
status query / reconciliation
↓
confirmed outcome
```

Never blindly create a second provider-side transaction merely because the first HTTP response timed out.

## Tests

```text
duplicate webhook
invalid signature
wrong amount
wrong currency
wrong merchant account
late success
provider timeout
timeout-after-commit
refund replay
provider state regression
```

## Exit

The platform can distinguish “we asked for payment,” “provider says it happened,” and “Sitolo has reconciled it.”

---

# 19. Phase 12 — Offline Synchronization

## Objective

Implement durable client commands and deterministic server ingestion.

## Local state

```text
product snapshot
price snapshot
stock view
draft cart
committed local sale
outbox command
inbox acknowledgement
sync checkpoint
device state
local audit evidence
```

## Ingestion pipeline

```text
authenticate
 ↓
device validation
 ↓
tenant/branch validation
 ↓
authorization
 ↓
schema validation
 ↓
idempotency check
 ↓
domain validation
 ↓
PostgreSQL transaction
 ↓
command result
 ↓
acknowledgement
 ↓
checkpoint progression
```

## Conflict classes

```text
commutative
mergeable
reject-and-refresh
approval-required
server-authoritative
```

Do not use generic last-write-wins for money, inventory or approvals.

## Failure tests

```text
local crash
server crash
upload timeout
response loss
duplicate batch
modified command
checkpoint rollback
device revoked
schema mismatch
stale data
```

## Exit

Sync converges deterministically under replay, interruption and duplication.

---

# 20. Phase 13 — Procurement / Suppliers

## Objective

Implement inbound commercial intent and physical receipt evidence.

## Sequence

```text
supplier
 ↓
purchase order
 ↓
approval
 ↓
goods receipt
 ↓
receiving discrepancy
 ↓
inventory posting
```

A PO is expected quantity. A goods receipt is actual received evidence.

## Tests

```text
partial receipt
over-receipt
wrong supplier
duplicate receipt
cross-tenant receipt
lot/expiry capture
discrepancy
receipt correction
```

## Exit

Every procurement-driven inventory increase has receipt evidence.

---

# 21. Phase 14 — Returns / Refunds / Cash

## Objective

Implement correction and physical cash-control workflows.

## Concepts

```text
void
reversal
return
refund
cash event
cash variance
```

They remain distinct facts.

## Refund rule

```text
eligible refund
  = original entitlement
  - already consumed correction entitlement
  - disqualifying state
```

## Register lifecycle

```text
OPENING
 ↓
ACTIVE
 ↓
COUNT_PENDING
 ↓
CLOSED
```

A closed session cannot silently reopen and rewrite prior cash history.

## Exit

No correction operation can exceed its entitlement or destroy the original financial history.

---

# 22. Phase 15 — MRA EIS

## Objective

Implement tax integration only against verified current MRA requirements and certification evidence.

## Adapter

```text
Tax Domain
    ↓
EisGateway
    ↓
MraEisAdapter
    ├── terminal
    ├── activation
    ├── configuration
    ├── sale submission
    ├── offline submission
    ├── status
    └── correction/utility operations
```

## Required external state

```text
terminal ID
activation state
configuration version
taxpayer association
offline parameters
credential state
```

## Local tax state

```text
LOCAL_SALE_COMMITTED
 ↓
TAX_SUBMISSION_PENDING
 ↓
SUBMITTING
 ├── accepted
 ├── retryable
 └── rejected
```

EIS failure must not rewrite the economic facts of the local sale.

## Certification gate

Do not call the feature “MRA compliant” merely because a development request returned success. Production claims require the applicable MRA approval/certification and operational evidence.

## Exit

Provider contract fixtures, negative tests, credential controls, offline behavior and certification evidence are present.

---

# 23. Phase 16 — Reporting / Exports

## Objective

Build read-oriented business insight without weakening transactional boundaries.

## Rule

Reports read authoritative data or controlled read models. They do not mutate transactional state.

## Controls

```text
tenant scope
branch scope
field allowlist
date-range bound
pagination
export quota
async job for large export
audit
short-lived download authorization
```

## Tests

```text
cross-tenant report
cross-branch report
unbounded query
large export
expired download
sensitive field leakage
repeated export abuse
```

## Exit

Reporting cannot become a tenant data-exfiltration path or an uncontrolled database workload generator.

---

# 24. Phase 17 — Billing / Entitlements

## Objective

Implement Sitolo's commercial entitlement state separately from merchant financial truth.

## Model

```text
plan
 ↓
subscription
 ↓
invoice
 ↓
payment
 ↓
reconciliation
 ↓
entitlement
```

Usage comes from authoritative events, not client-reported counters.

Billing failures may reduce access according to policy but cannot rewrite sales, stock or payment history.

## Exit

Billing is independently reconcilable and has no authority over merchant transaction history.

---

# 25. Phase 18 — Admin / Support

## Objective

Implement platform operations without creating a hidden superuser path.

## Functions

- tenant operations;
- support search;
- feature flags;
- device revocation;
- queue inspection;
- payment/tax exception handling;
- audit review;
- incident controls;
- credential rotation workflows.

## Break-glass

Break-glass must have:

```text
strong authentication
reason
scope
short lifetime
heightened audit
post-use review
```

## Exit

Support access is explicit, reviewable and bounded.

---

# 26. Phase 19 — Hardening / Performance / Disaster Recovery

## Objective

Turn a functionally complete system into a production operating system.

## Performance layers

Measure:

```text
POS p50/p95/p99
catalogue search
inventory lookup
authorization
DB query latency
sync throughput
worker backlog
external dependency latency
mobile cold start
mobile memory
```

Test with realistic low/mid-range Android devices and intermittent connectivity.

## Availability controls

Every external or expensive operation must have an appropriate:

```text
timeout
concurrency limit
payload limit
retry budget
backoff + jitter
circuit breaker where appropriate
bulkhead
```

## DR

Perform actual restore exercises covering:

```text
backup discovery
restore
schema verification
integrity verification
critical transaction verification
application reconnect
external-state reconciliation
RPO measurement
RTO measurement
```

## Exit

Recovery evidence exists for database, application, queues, secrets and external integration state.

---

# 27. Phase 20 — Production Certification

## Objective

Prove the release candidate across software correctness, security, operations and external compliance.

## Required evidence

### Architecture

```text
ADRs
architecture review
open decisions
```

### Security

```text
secret scanning
SCA
SAST
API security
BOLA/tenant suite
DAST
fuzzing
CI security
cloud/IaC scanning
```

### Domain

```text
invariant suite
state machine suite
financial integrity
inventory concurrency
idempotency
```

### Integrations

```text
payment contract tests
MRA contract tests
failure injection
reconciliation evidence
```

### Offline

```text
crash
replay
duplicate
checkpoint
revocation
schema compatibility
```

### Operations

```text
deployment rehearsal
rollback rehearsal
restore drill
alert verification
incident runbooks
```

### Supply chain

```text
locked dependencies
SBOM
provenance
artifact signature
artifact verification
```

## Blocking findings

The following cannot be shipped without formal emergency governance:

```text
critical tenant isolation
critical financial integrity
critical data-loss
critical authentication bypass
critical secrets exposure
critical supply-chain compromise
```

---

# 28. Cross-Phase Definition of Done

Every phase must satisfy:

```text
[ ] implementation
[ ] authorization
[ ] tenant scope
[ ] state machine
[ ] invariant
[ ] persistence constraints
[ ] idempotency
[ ] audit evidence
[ ] telemetry
[ ] negative tests
[ ] failure tests
[ ] recovery path
[ ] migration review
[ ] performance baseline
[ ] documentation
[ ] CI gates
```

A “green build” without these does not mean phase completion.

---

# 29. Golden End-to-End Transactions

## 29.1 Online sale

```text
login
 ↓
organization
 ↓
branch/register
 ↓
product
 ↓
price
 ↓
receive stock
 ↓
complete sale
 ↓
record tender
 ↓
inventory posting
 ↓
audit
 ↓
outbox
 ↓
tax pending/accepted
 ↓
report
```

## 29.2 Offline sale

```text
authenticated device
 ↓
offline
 ↓
local sale commit
 ↓
process death
 ↓
restart
 ↓
local sale retained
 ↓
network returns
 ↓
sync
 ↓
idempotency
 ↓
authoritative acceptance
 ↓
checkpoint advance
```

## 29.3 Financial correction

```text
sale
 ↓
partial return
 ↓
partial refund
 ↓
reconciliation
 ↓
tax correction when required
 ↓
reports
```

Original sale remains historical truth.

## 29.4 Last-unit concurrency

```text
stock = 1
 ↓
100 concurrent attempts
 ↓
exactly one unit consumed
```

Repeated until confidence is statistically meaningful and race conditions are reproducible.

## 29.5 Tenant attack

Authenticated as Tenant A, attempt direct access to Tenant B through IDs, branch selectors, cached references and forged selectors. Every operation must fail without leaking protected data.

---

# 30. Repository-Level Workstreams

## Workstream A — Domain

Own:

```text
aggregates
invariants
state transitions
event semantics
historical truth
```

## Workstream B — Security

Own:

```text
authentication
authorization
tenant isolation
device trust
secret handling
abuse controls
security testing
```

## Workstream C — Data

Own:

```text
migrations
constraints
indexes
RLS
retention
backup
restore
```

## Workstream D — Integrations

Own:

```text
adapter contracts
request mapping
response mapping
error taxonomy
retry classification
external credentials
provider evidence
```

## Workstream E — Clients

Own:

```text
UX
local persistence
offline continuity
sync presentation
secure local state
```

## Workstream F — Reliability

Own:

```text
timeouts
retries
backoff
bulkheads
circuit breakers
queue leases
DLQ
backpressure
recovery
```

---

# 31. API Implementation Order

```text
health/readiness/version
 ↓
auth/session/device
 ↓
organization/membership/branch/resource
 ↓
product/SKU/barcode/price
 ↓
inventory
 ↓
sales
 ↓
cash/payment
 ↓
reconciliation/refunds/returns
 ↓
sync
 ↓
procurement
 ↓
tax/EIS
 ↓
reporting/exports
 ↓
billing
 ↓
admin/support
```

Every route must specify:

```text
authentication
permission
scope
rate limit
timeout
idempotency
sensitivity
audit
```

---

# 32. Database Implementation Order

```text
identity
 ↓
tenant/membership
 ↓
branch/resource
 ↓
authorization
 ↓
catalogue/pricing
 ↓
inventory
 ↓
sales
 ↓
payment/cash
 ↓
reconciliation
 ↓
tax
 ↓
audit/outbox/jobs
 ↓
sync
 ↓
procurement
 ↓
returns/refunds
 ↓
reporting
 ↓
billing
 ↓
platform admin
```

The exact migration count is implementation detail; dependency order is not.

---

# 33. Worker Implementation Order

First implement generic worker machinery:

```text
claim
lease
heartbeat
retry budget
backoff
cancellation
DLQ
metrics
tracing
```

Then specialized jobs:

```text
outbox dispatch
payment reconciliation
MRA EIS submission
sync maintenance
report/export generation
notifications
maintenance
```

A worker must be safe to run twice where the business operation is retryable.

---

# 34. Mobile Implementation Order

## Stage A

```text
secure storage
authentication
device registration
```

## Stage B

```text
SQLite
local migrations
catalogue snapshot
price snapshot
```

## Stage C

```text
POS draft
local sale commit
local evidence
```

## Stage D

```text
outbox commands
sync
retry
conflict handling
checkpoint
```

## Stage E

```text
payments
tax/EIS
recovery UX
```

The client exposes states; it does not invent authority.

---

# 35. Data Migration Strategy

Prefer:

```text
EXPAND
 ↓
compatible application
 ↓
BACKFILL
 ↓
VALIDATE
 ↓
CONTRACT
```

Avoid one-shot destructive changes.

Historical financial data must remain reconstructable after migrations.

Every migration review records:

```text
schema effect
lock behavior
backfill size
compatibility window
failure behavior
recovery
security impact
performance impact
```

---

# 36. Release Train

```text
feature branch
 ↓
PR checks
 ↓
integration validation
 ↓
security suite
 ↓
release candidate
 ↓
staging
 ↓
canary
 ↓
production
```

The release record contains:

```text
source commit
compiler/toolchain
lockfiles
artifact digests
SBOM
provenance
test reports
security reports
migration version
configuration version
rollback reference
```

---

# 37. Rollback Rules

Rollback is component-specific.

## Application

Previous artifact may be redeployed if database compatibility is preserved.

## Database

Do not assume down-migrations are safe. Prefer forward-compatible schema expansion and application rollback.

## External side effects

A rollback must retain knowledge of prior provider references, payment state, tax submissions, idempotency records and reconciliation decisions.

Never deploy an older application that can repeat or forget an already-completed external operation.

---

# 38. Incident-to-Regression Loop

Every material incident follows:

```text
incident
 ↓
root cause
 ↓
missing control
 ↓
code/control change
 ↓
regression test
 ↓
observability update
 ↓
runbook update
 ↓
threat model update if applicable
 ↓
close
```

The objective is systemic prevention, not merely restoration.

---

# 39. Open Decisions — Must Remain Explicit

The following decisions are intentionally open until supported by evidence and ADR approval:

1. exact Flutter SQLite package;
2. exact synchronization library versus custom implementation split;
3. managed identity provider versus standards-based self-managed identity;
4. production cloud/platform;
5. production region and data residency;
6. whether Redis is materially necessary;
7. read replica or analytical warehouse design;
8. exact production payment-provider capabilities/credentials;
9. exact MRA certification pathway and production credentials;
10. pharmacy regulatory operating model;
11. enterprise dedicated-database isolation requirement;
12. exact object-storage provider;
13. final OpenTelemetry collector topology.

These choices must not become implicit implementation facts simply because a developer used one option locally.

---

# 40. Prohibited Shortcuts

Do not implement any of the following:

```text
client-selected tenant as trust
client-controlled financial totals
client-controlled role authority
provider credentials in clients
finalized financial UPDATE as correction
floating-point money
application-memory locks as distributed authority
database transaction held across provider HTTP
blind payment retry after unknown outcome
webhook receipt treated as unconditional proof
last-write-wins for money/stock
checkpoint advance before durable commit
sync acceptance from revoked device
unbounded report generation
unbounded sync batches
unbounded uploads
hidden support superuser
raw provider/database errors to clients
security scans as optional
unsigned/untraceable release artifacts
microservices without proven boundary
regulatory compliance claim without evidence
```

---

# 41. Feature Change Contract

Every production feature PR identifies:

```text
feature
owner
threats
asset
trust boundaries
tenant scope
permission
state transitions
invariant
transaction boundary
idempotency
audit event
metrics
logs
tests
migration
rollback
failure modes
recovery
```

High-risk changes also require:

```text
threat-model reference
security review
negative tests
concurrency analysis
operational runbook
```

---

# 42. Phase State Machine

Use machine-readable lifecycle state:

```text
PLANNED
  ↓
IN_PROGRESS
  ↓
IMPLEMENTED
  ↓
TESTED
  ↓
SECURITY_VERIFIED
  ↓
OPERATIONALLY_VERIFIED
  ↓
REVIEWED
  ↓
COMPLETE
```

A phase may not jump from `IMPLEMENTED` to `COMPLETE`.

Required evidence:

```text
implementation commit
test results
security results where applicable
migration result
telemetry validation
review approval
release artifact where applicable
```

---

# 43. Milestone Model

## M0 — Foundation

```text
Phase 0–2
```

Deliverable:

```text
secure reproducible backend skeleton
```

## M1 — Trusted Platform Core

```text
Phase 3–7
```

Deliverable:

```text
identity + tenant + authorization + secure DB + security harness
```

## M2 — Merchant Core

```text
Phase 8–10 + Phase 14 foundations
```

Deliverable:

```text
catalogue → inventory → POS
```

## M3 — Money and Continuity

```text
Phase 11–13
```

Deliverable:

```text
payments + reconciliation + offline + procurement
```

## M4 — Compliance / Operations

```text
Phase 15–18
```

Deliverable:

```text
tax + reporting + billing + support
```

## M5 — Production

```text
Phase 19–20
```

Deliverable:

```text
verified release candidate and production evidence
```

---

# 44. Testing Strategy Integration

The implementation plan does not create a separate “testing phase” at the end.

Testing is embedded:

```text
unit
 → as domain is built

integration
 → as transaction boundaries are built

security
 → as trust boundaries are built

contract
 → as adapters are built

property/fuzz
 → as parsers/state machines stabilize

concurrency
 → as shared authoritative state is built

performance
 → as hot paths emerge

DR
 → before production certification
```

This prevents the common failure mode in which large amounts of code are written before the organization discovers that the domain semantics cannot be tested cleanly.

---

# 45. Observability Integration

Instrumentation is implemented with the feature, not after launch.

A new operation should define:

```text
metric(s)
trace/span
structured log events
error labels
business outcome
latency
```

Metrics must not use unbounded identifiers such as user IDs, sale IDs or trace IDs as ordinary labels.

Audit and telemetry are separate:

```text
Telemetry → explains runtime behavior
Audit → records controlled business/security evidence
Database/domain state → remains authoritative truth
```

---

# 46. Security Integration

Every phase is mapped against the threat model and security control matrix.

For each new trust boundary:

```text
asset
 ↓
threat
 ↓
control
 ↓
implementation
 ↓
negative test
 ↓
telemetry
 ↓
runbook
```

A security control without executable evidence is considered incomplete.

---

# 47. Recovery Integration

Every stateful operation answers:

```text
What if the process dies?
What if the DB commits but the response disappears?
What if the provider commits but the client times out?
What if the worker runs twice?
What if the device is revoked?
What if configuration changes between attempts?
What if the database is restored to an earlier point?
```

The implementation must encode the answer rather than leaving it to operational folklore.

---

# 48. Final Implementation Contract

Sitolo implementation is successful when the product path is built in the same order as its trust model:

```text
FREEZE THE MODEL
      ↓
MAKE THE TOOLCHAIN REPRODUCIBLE
      ↓
BUILD SECURITY / OBSERVABILITY PRIMITIVES
      ↓
ESTABLISH TRUSTED IDENTITY
      ↓
ESTABLISH TENANT AUTHORITY
      ↓
MAKE POSTGRESQL AUTHORITATIVE
      ↓
MAKE SECURITY TESTING AUTOMATIC
      ↓
BUILD CATALOGUE / INVENTORY / POS
      ↓
BUILD MONEY / RECONCILIATION
      ↓
BUILD OFFLINE CONTINUITY
      ↓
BUILD PROCUREMENT / CORRECTIONS
      ↓
BUILD TAX / EIS
      ↓
BUILD REPORTING / BILLING / ADMIN
      ↓
HARDEN / TEST RECOVERY
      ↓
CERTIFY
```

The guiding rule is:

> **Do not build the feature surface faster than the trust, integrity and recovery model can support it.**

That constraint is what keeps the system coherent as the implementation grows from a Malawi-first offline POS into a multi-tenant business operating system.

---

# Appendix A — Phase Checklist

```text
PHASE 0
[ ] architecture
[ ] security
[ ] domain
[ ] DB
[ ] API
[ ] auth
[ ] sync
[ ] payments
[ ] MRA
[ ] testing
[ ] threat model
[ ] observability
[ ] deployment
[ ] ADRs

PHASE 1
[ ] workspace
[ ] toolchain pin
[ ] lockfile
[ ] CI
[ ] dependency policy
[ ] secret scanning
[ ] artifact/provenance baseline

PHASE 2
[ ] config
[ ] secrets
[ ] logs
[ ] errors
[ ] telemetry
[ ] health/readiness

PHASE 3
[ ] user
[ ] sessions
[ ] MFA
[ ] recovery
[ ] device identity
[ ] revocation

PHASE 4
[ ] organization
[ ] membership
[ ] branch
[ ] location
[ ] warehouse
[ ] register
[ ] role
[ ] permission
[ ] support access

PHASE 5
[ ] schema
[ ] migrations
[ ] constraints
[ ] indexes
[ ] roles
[ ] RLS
[ ] backups

PHASE 6
[ ] policy engine
[ ] object authz
[ ] property authz
[ ] state authz
[ ] approvals
[ ] SoD

PHASE 7
[ ] fixtures
[ ] negative suite
[ ] mutation tests
[ ] fuzz harness
[ ] CI gates

PHASE 8
[ ] product
[ ] SKU
[ ] unit model
[ ] barcode
[ ] pricing
[ ] promotions

PHASE 9
[ ] ledger
[ ] balance
[ ] concurrency
[ ] transfers
[ ] counts
[ ] lots/batches

PHASE 10
[ ] sale command
[ ] atomic finalize
[ ] inventory
[ ] tender
[ ] audit
[ ] outbox

PHASE 11
[ ] intents
[ ] attempts
[ ] provider adapters
[ ] webhooks
[ ] reconciliation
[ ] unknown outcome
[ ] refunds

PHASE 12
[ ] local DB
[ ] command journal
[ ] push
[ ] pull
[ ] checkpoints
[ ] replay
[ ] conflicts
[ ] revocation

PHASE 13
[ ] suppliers
[ ] PO
[ ] approval
[ ] receipt
[ ] discrepancy
[ ] supplier returns

PHASE 14
[ ] void
[ ] reversal
[ ] return
[ ] refund
[ ] register
[ ] close
[ ] variance

PHASE 15
[ ] terminal
[ ] activation
[ ] configuration
[ ] sales
[ ] offline
[ ] signing
[ ] status
[ ] certification evidence

PHASE 16
[ ] read models
[ ] reports
[ ] exports
[ ] quotas
[ ] secure downloads

PHASE 17
[ ] plans
[ ] subscriptions
[ ] invoices
[ ] payments
[ ] reconciliation
[ ] entitlements
[ ] usage

PHASE 18
[ ] support
[ ] JIT
[ ] break-glass
[ ] queue/admin operations
[ ] incident controls

PHASE 19
[ ] performance
[ ] load
[ ] soak
[ ] chaos
[ ] backup
[ ] restore
[ ] DR
[ ] rollback

PHASE 20
[ ] security evidence
[ ] domain evidence
[ ] integration evidence
[ ] offline evidence
[ ] operations evidence
[ ] supply-chain evidence
[ ] regulatory evidence
[ ] release approval
```

---

# Appendix B — Current External Anchors

- Rust Release Team — Rust 1.98.1, 2026-09-03.
- PostgreSQL — 18.6 release notes, 2026-08-13.
- SLSA specification 1.2.
- OpenTelemetry Semantic Conventions 1.44.0.
- OWASP ASVS 5.0.0.
- NIST SP 800-218 SSDF 1.1.

These anchors are references for implementation governance. They do not replace Sitolo-specific contracts or regulator/provider requirements.

---

**Document end — Sitolo Implementation Plan, Phase 0 / File 13 of 16.**
