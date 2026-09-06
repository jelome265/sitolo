# SITOLO — AGENT.md
## Engineering Governance Contract for AI Coding Agents

**Project:** Sitolo
**Product:** Business Operating System for African SMEs
**Primary market:** Malawi first; controlled African regional expansion
**Contract type:** Repository-wide AI engineering governance and implementation contract
**Status:** Governing implementation policy
**Prepared:** 2026-09-04

---

# 0. Purpose

This file defines how an AI coding agent MUST operate inside the Sitolo repository.

The agent is not an autocomplete engine, generic pair programmer, or feature-generation bot. It is an engineering actor operating under explicit architectural, domain, security, database, reliability, operational, and product constraints.

The agent MUST optimize for:

1. correctness;
2. security;
3. preservation of domain invariants;
4. tenant isolation;
5. financial and inventory integrity;
6. deterministic and recoverable behavior;
7. maintainability;
8. observability;
9. reproducibility;
10. controlled complexity;
11. evidence-backed implementation;
12. explicit failure handling.

Speed is secondary to correctness. A fast implementation that violates a security boundary, creates financial inconsistency, breaks offline convergence, or destroys architectural ownership is a failed implementation.

The agent MUST NOT infer permission to weaken these rules merely because a requested feature appears urgent, small, convenient, or commercially useful.

---

# 1. Governing Source Hierarchy

The agent MUST resolve ambiguity using this hierarchy:

```text
1. Applicable law and regulatory requirements
2. Signed external-provider contracts and current provider behavior
3. Security architecture invariants
4. System architecture invariants
5. Domain model decisions
6. Database design decisions
7. Product requirements
8. UI assumptions
9. Implementation convenience
```

Commercial assumptions are hypotheses. Technical convenience never overrides a security or correctness invariant.

A request such as “make checkout one tap” MUST NOT result in trusting client-calculated totals, skipping authorization, skipping state validation, bypassing inventory controls, or accepting unverified payment state.

When two internal documents appear inconsistent, the agent MUST NOT silently select the easier option. It MUST identify the conflict, apply the source hierarchy, and preserve the higher-authority constraint.

When a required decision is explicitly marked open in the design documents, the agent MUST NOT fabricate certainty. It should implement a boundary that keeps the decision replaceable and record the unresolved choice where appropriate.

---

# 2. Governing Sitolo Architecture

Sitolo is a Rust-first modular-monolith business operating system.

The target runtime model is:

```text
Flutter Mobile / Tauri Desktop / limited Web
                 |
              HTTPS/API
                 |
                 v
       Rust Application Core
        Axum + Tokio
                 |
      +----------+----------+
      |          |          |
      v          v          v
 PostgreSQL   Workers    Integrations
 authority     /outbox    /adapters
      |
      +---- audit / idempotency / read models
```

The architecture intentionally keeps critical business functions together instead of prematurely splitting them into network services.

The agent MUST preserve the following baseline:

- Rust owns business decisions and orchestration.
- Axum is the HTTP/API boundary.
- Tokio is the async runtime.
- SQLx is the PostgreSQL access layer.
- PostgreSQL is authoritative server-side business state.
- SQLite is the local operational store for offline continuity.
- Flutter is the primary mobile client.
- Tauri is the desktop shell/client.
- TypeScript is constrained to web/admin, tooling, generated clients/schema tooling, and Tauri UI where appropriate.
- External systems are trust boundaries.
- Durable outbox/workers are preferred before introducing a heavyweight broker.
- Redis is optional and MUST NOT become authoritative financial or inventory state.
- Object storage is for blobs/evidence/imports/exports, not canonical transactional truth.
- Reporting is derived data.
- Financial history is append-oriented and corrected through explicit compensating actions.
- Tenant scope is a security boundary.
- Service extraction requires explicit justification.

The agent MUST NOT turn the repository into a microservice mesh merely because the system is described as enterprise-grade.

---

# 3. Agent Operating Mode

Before changing code, the agent MUST determine:

```text
What business capability is changing?
Who owns it?
What invariants must remain true?
Which trust boundary is crossed?
What is authoritative?
What is derived?
What transaction boundary is required?
What happens under retry?
What happens concurrently?
What happens offline?
What happens when a dependency fails?
What evidence is required?
What negative tests prove the control?
How is the feature disabled or rolled back?
```

The agent MUST reason in systems terms rather than file terms.

A request to “add an endpoint” is insufficiently scoped until the agent determines whether it introduces:

- new domain behavior;
- new authorization rules;
- tenant access;
- persistent state;
- a financial mutation;
- inventory mutation;
- an external side effect;
- a new asynchronous job;
- an offline command;
- a new audit requirement;
- new PII or sensitive data;
- resource-exhaustion risk;
- a migration;
- a public API contract;
- a new operational failure mode.

The agent MUST inspect the existing implementation before introducing a parallel implementation.

The agent MUST prefer extending an existing correct abstraction over creating a duplicate abstraction.

The agent MUST NOT rewrite unrelated code solely because it prefers a different style.

---

# 4. Mission: Preserve Business Truth

Sitolo exists to turn fragmented merchant operations into a coherent business record.

The core business loop is:

```text
PROCURE
  -> RECEIVE
  -> STOCK
  -> PRICE
  -> SELL
  -> COLLECT
  -> RECONCILE
  -> REPORT
  -> DECIDE
```

The agent MUST treat this loop as an integrity chain.

A mutation in one part of the loop MUST NOT silently invalidate another part.

Examples:

- A purchase order does not increase inventory. A goods receipt does.
- A finalized sale cannot be edited into a different historical total.
- A current price change MUST NOT reinterpret a historical sale.
- A client-calculated payment status MUST NOT become authoritative.
- A provider callback MUST NOT automatically rewrite financial truth without validation.
- A report MUST NOT quietly mix finalized, pending, reversed, or refunded populations.
- An inventory movement MUST remain traceable.
- A cash variance MUST be preserved rather than normalized away.

The agent MUST prefer an explicit correction state over destructive mutation when business history matters.

---

# 5. “Clients Request; Server Decides” Rule

Clients are operational surfaces, not authorities.

Flutter, Tauri, browsers, imported files, provider callbacks, and offline local state MUST be treated as untrusted inputs.

The authoritative processing model is:

```text
client intent
    |
local validation
    |
durable local command where offline is supported
    |
sync/API
    |
authentication
    |
tenant resolution
    |
authorization
    |
domain validation
    |
transaction + invariants
    |
PostgreSQL authoritative state
    |
outbox / audit / derived work
```

The agent MUST NOT trust:

- client-provided tenant IDs;
- client-provided user roles;
- client-provided branch authority;
- client-calculated final totals;
- client-provided payment success status;
- client-provided stock balances;
- client-provided tax submission success;
- hidden UI controls as authorization;
- local SQLite as final authority;
- random IDs as an authorization boundary.

---

# 6. Domain Ownership Rules

The domain layer owns:

- entities;
- value objects;
- state transitions;
- invariants;
- pure calculations;
- domain errors;
- domain events where appropriate.

The domain layer MUST NOT depend on:

- Axum;
- SQLx;
- Redis;
- HTTP client implementations;
- provider SDK implementations;
- Flutter concepts;
- Tauri concepts;
- UI state;
- persistence-specific behavior.

The application layer owns:

- command orchestration;
- authorization invocation;
- transaction ownership;
- unit-of-work behavior;
- outbox registration;
- external-port coordination;
- cross-domain workflow orchestration.

Infrastructure owns:

- SQLx repositories;
- PostgreSQL queries;
- external HTTP clients;
- object storage;
- workers;
- telemetry integrations;
- provider adapters.

The API layer owns:

- HTTP routing;
- DTO parsing/serialization;
- request context extraction;
- API versioning;
- transport concerns;
- mapping application/domain errors to stable API responses.

The agent MUST NOT place business rules inside Axum handlers merely because the handler is convenient.

The agent MUST NOT expose repository functions that bypass domain transitions.

Bad:

```text
update_sale_total(sale_id, total)
```

Preferred:

```text
approve_adjustment(...)
reverse_sale(...)
issue_refund(...)
apply_compensating_action(...)
```

The API DTO, domain object, and database row MUST remain separate conceptual boundaries.

---

# 7. Dependency Direction

The intended dependency direction is:

```text
API -> Application -> Domain
Jobs -> Application -> Domain
Persistence -> Domain ports/types
Integrations -> Domain ports/types
```

The agent MUST reject or refactor dependencies shaped like:

```text
Domain -> Axum
Domain -> SQLx
Domain -> Redis
Domain -> Provider SDK
Domain -> Flutter
Domain -> Tauri UI
```

A dependency graph that violates the direction is an architecture defect even if the code compiles.

The agent MUST use dependency inversion where infrastructure is required by domain/application workflows.

---

# 8. Modular Monolith Rules

Logical modules are first-class even when deployed in one process.

Baseline modules include:

```text
identity
tenant
authorization
catalogue
pricing
procurement
inventory
sales
cash
payments
reconciliation
tax
customers
reporting
notifications
audit
billing
platform_admin
integrations
vertical_pharmacy
vertical_agro
vertical_wholesale
sync
```

A module MUST own its concepts and persistence boundaries.

A module MUST NOT arbitrarily mutate another module’s tables simply because SQL makes it possible.

Cross-module coordination should use application/domain contracts, repositories, and a shared transaction context where atomicity is required.

The agent MUST NOT introduce HTTP calls between modules inside the same process.

The agent MUST NOT create a crate/module for every noun. Boundary quality matters more than artificial fragmentation.

New service extraction requires evidence such as:

- materially different scaling characteristics;
- independent failure domain requirement;
- independent operational ownership;
- strong protocol boundary;
- independent deployment value;
- measured contention or capacity reason.

The migration path for justified extraction is:

```text
module boundary
 -> explicit contract
 -> shadow implementation
 -> compare outputs
 -> controlled traffic switch
 -> remove in-process implementation
```

---

# 9. Tenant Isolation — Critical Security Boundary

Tenant isolation is non-negotiable.

Every tenant-owned read, write, job, export, report, integration action, cache lookup, support action, and synchronization operation MUST establish trusted tenant scope.

Caller-controlled `tenant_id` is never proof of access.

The trusted sequence is:

```text
authenticated principal
 -> membership
 -> organization
 -> allowed scope
 -> branch/resource authorization
 -> tenant-scoped operation
```

Tenant-owned repository methods SHOULD make scope explicit:

```rust
get_sale(tenant_id, sale_id)
```

is preferred over:

```rust
get_sale(sale_id)
```

The agent MUST implement defense in depth using appropriate combinations of:

- application authorization;
- scoped repository queries;
- foreign keys;
- tenant-aware constraints;
- PostgreSQL RLS where appropriate;
- negative integration tests.

The agent MUST test the negative case.

A feature is incomplete if it only proves that authorized access works. It MUST also prove that:

- tenant A cannot read tenant B;
- tenant A cannot modify tenant B;
- branch A cannot access branch B when branch isolation applies;
- exports cannot cross tenants;
- background jobs cannot cross tenants;
- reporting cannot cross tenants;
- support tooling cannot bypass scope silently.

Cross-tenant vulnerability is Critical and MUST block release.

---

# 10. Identity and Authorization

Authentication and authorization are distinct.

Authentication answers:

```text
Who are you?
```

Authorization answers:

```text
What may this principal do, to which resource, within which scope, under what policy?
```

The agent MUST enforce:

- function-level authorization;
- object-level authorization;
- property-level authorization;
- tenant scope;
- branch scope;
- role constraints;
- approval requirements;
- separation-of-duties controls where required;
- device restrictions where applicable.

UI visibility is never authorization.

The agent MUST NOT rely on:

```text
if (!buttonVisible) user cannot act
```

The agent MUST assume an attacker can call the API directly.

Privileged administrative paths MUST require stronger controls than ordinary cashier operations according to the security model.

When adding privileged actions, the agent MUST consider:

- step-up authentication;
- MFA requirements;
- approval policy;
- audit evidence;
- scope limitation;
- replay resistance;
- rate limits;
- rollback/revocation.

---

# 11. Authorization Decision Pattern

For protected operations, use a conceptual order similar to:

```text
authenticate
  -> resolve principal
  -> resolve tenant context
  -> resolve resource scope
  -> authorize action
  -> validate request schema
  -> validate domain state
  -> execute transaction
```

Authorization MUST occur before loading broad datasets whenever practical.

Do not fetch an unrestricted object and “check later”.

Avoid repository interfaces that make it easy to forget scope.

Do not return sensitive fields solely because the caller can access the parent resource. Field-level rules matter.

---

# 12. Financial Integrity Rules

Financial facts are high-integrity data.

The agent MUST treat the following as high-risk operations:

- sale finalization;
- refunds;
- voids/reversals;
- discounts above threshold;
- cash close;
- payment confirmation;
- payment capture;
- settlement posting;
- reconciliation resolution;
- financial exports;
- billing mutations.

Finalized financial facts MUST NOT be silently overwritten.

Corrections MUST use explicit compensating operations, such as:

```text
reverse
return
refund
adjust
correct
```

Do not create “god” update methods that can mutate every sale field.

The backend MUST calculate or independently verify authoritative totals.

Client-provided totals, prices, discounts, taxes, payment outcomes, or stock balances are evidence at most, never unquestioned authority.

---

# 13. Inventory Integrity Rules

Inventory is ledger-backed.

An inventory movement MUST have:

- clear movement semantics;
- tenant and location scope;
- product/SKU identity;
- unambiguous quantity semantics;
- traceability to its business cause;
- authorization where required;
- concurrency protection where contested.

There MUST NOT be “magic adjustment” writes that simply overwrite a stock balance without a corresponding business reason and evidence.

Current inventory balance is derived/operational state backed by inventory movements.

A sale MUST NOT consume inventory twice due to retries.

A transfer MUST not post only one side.

Negative stock MUST be controlled by explicit domain policy.

Batch/lot/expiry restrictions MUST be enforced for vertical workflows where applicable.

Stock decrement involving concurrent requests MUST use an explicit concurrency strategy.

---

# 14. POS / Sale Transaction Contract

Finalizing a sale is a critical atomic workflow.

The conceptual transaction is:

```text
BEGIN
  authenticate context
  resolve tenant / branch / register
  authorize FINALIZE_SALE
  load sale draft
  validate sale state
  validate products/SKUs
  calculate or verify prices/tax
  lock/check contested inventory
  post inventory movement
  finalize sale
  create/link payment state as required
  write audit event
  write outbox event
COMMIT
```

The transaction MUST NOT hold a DB transaction open while waiting for an external HTTP provider.

Payment provider interaction belongs outside the core transaction through durable intent/state and reconciliation.

The agent MUST consider race conditions between two devices attempting to sell the same scarce inventory.

---

# 15. Payment Rules

External payment systems are untrusted boundaries.

A payment provider can:

- fail;
- timeout;
- retry;
- duplicate callbacks;
- delay callbacks;
- return impossible states;
- change API behavior;
- produce mismatched references;
- be temporarily unavailable.

The agent MUST NOT make payment correctness depend on one successful network call.

Payment state MUST be modeled explicitly.

The provider event identity MUST be idempotent.

Conceptually:

```text
(provider, provider_event_id) UNIQUE
```

Duplicate provider events MUST NOT produce duplicate financial effects.

Invalid signatures MUST be rejected.

A provider response that conflicts with known business state MUST enter a controlled exception/reconciliation path rather than being blindly applied.

The frontend MUST never be the authority for payment success.

---

# 16. Reconciliation Rules

Reconciliation is a core Sitolo capability, not a reporting cosmetic layer.

The system compares:

```text
expected business money
        vs
observed provider/bank/cash evidence
```

Matching MUST be deterministic where evidence permits.

Unmatched or contradictory evidence becomes an explicit reconciliation case.

The agent MUST NOT “resolve” mismatches by silently overwriting either side.

Reconciliation records MUST preserve sufficient evidence to investigate disputes and fraud.

When adding a reconciliation rule, consider:

- duplicate observations;
- delayed settlement;
- provider fees;
- reference mismatches;
- partial settlements;
- reversals;
- currency/amount mismatch;
- timezone/date cutoffs;
- branch/merchant scoping.

---

# 17. Cash Rules

Cash is represented as explicit events around register sessions.

The agent MUST preserve an audit-capable relationship between:

```text
opening cash
+ cash inflows
- cash outflows
+/- corrections permitted by policy
= expected cash
```

Closing a register is a state transition, not a boolean convenience field.

A cash variance MUST remain visible.

The agent MUST NOT normalize a variance to zero merely to make reports look clean.

Cash close and related approvals must be idempotent and concurrency-safe.

---

# 18. Procurement Rules

Procurement and inventory are related but distinct.

A purchase order expresses intention.

A goods receipt creates the inventory-relevant receipt fact.

The agent MUST preserve this distinction.

The typical lifecycle is:

```text
DRAFT
 -> APPROVED
 -> SENT
 -> PARTIALLY_RECEIVED
 -> FULLY_RECEIVED
```

or:

```text
DRAFT/APPROVED/SENT -> CANCELLED
```

Receiving MUST validate:

- quantity;
- unit conversion;
- batch/lot;
- expiry where relevant;
- supplier scope;
- duplicate receiving attempts.

A supplier invoice is not automatically the same thing as inventory receipt or payment settlement.

---

# 19. Catalogue and Pricing Rules

Catalogue describes what can be sold.

Sales preserve what was actually sold.

The agent MUST preserve historical snapshots needed to reconstruct the sale.

A current price change MUST NOT mutate historic sale totals.

Price resolution should be deterministic, for example using:

```text
branch
customer segment
channel
validity period
product rules
promotion rules
```

The applied price/rule context should be preserved where required for audit and reconstruction.

Do not build an unrestricted generic promotion/rules language merely because it looks flexible. Complexity must be justified by actual business requirements.

---

# 20. Offline-First Rules

Offline operation is a first-class subsystem.

Offline means:

```text
continue safely
```

It does NOT mean:

```text
become permanently authoritative
```

The local flow is:

```text
user intent
 -> local validation
 -> durable command
 -> local provisional result
 -> sync
 -> server authorization + validation
 -> authoritative acceptance/rejection
```

The agent MUST ensure that queued work survives process restarts where the feature promises offline continuity.

The agent MUST ensure that syncing the same command twice is harmless.

A revoked device MUST NOT continue to exercise unrestricted authority simply because it was offline.

The agent MUST explicitly model conflict/rejection outcomes.

The system MUST NOT claim impossible guarantees such as zero loss under arbitrary physical destruction of a device before synchronization.

Device data should be minimized and protected using secure storage/platform capabilities.

---

# 21. Synchronization Rules

Synchronization is a distributed-systems boundary.

The agent MUST assume:

- duplicate messages;
- reordered messages where applicable;
- retries;
- stale commands;
- device revocation;
- version mismatch;
- partial batches;
- process termination mid-sync;
- network loss between acknowledgement steps;
- conflicting business state.

Synchronization commands require stable identity.

Conceptually:

```text
same command identity + same semantic payload
    -> previous result / equivalent no-op
```

A reused identity with materially different semantics MUST be rejected.

The agent MUST distinguish:

- accepted authoritative result;
- rejected business rule;
- authorization denial;
- conflict;
- transient infrastructure failure;
- retryable external failure.

A client must not convert a server rejection into apparent success.

---

# 22. Idempotency Is a Domain Requirement

Idempotency MUST be considered for every retryable high-impact operation.

At minimum examine:

- sale finalization;
- payment commands;
- refunds;
- webhook events;
- offline commands;
- import processing;
- exports/jobs;
- outbox delivery;
- approval consumption;
- reconciliation actions.

Never implement idempotency as a client-only convention.

Database uniqueness is required where one logical fact must exist once.

Application “check then insert” without a database uniqueness boundary is unsafe under concurrency.

---

# 23. Database Authority

PostgreSQL is authoritative.

The database is part of the security and business-correctness boundary.

The agent MUST use relational constraints wherever they express durable invariants well:

- NOT NULL;
- UNIQUE;
- PRIMARY KEY;
- FOREIGN KEY;
- CHECK where row-local;
- EXCLUSION where appropriate.

Cross-row or cross-table invariants MUST NOT be faked using unsafe row-local CHECK assumptions.

Application transactions, locking, uniqueness, or other relational mechanisms MUST be used where required.

Runtime DB credentials MUST have substantially less privilege than migration/schema-owner credentials.

The application MUST NOT connect as a PostgreSQL superuser.

---

# 24. PostgreSQL Role Separation

Baseline logical roles:

```text
migration_owner
app_runtime
reporting_readonly
backup_operator
break_glass
```

The runtime account MUST NOT casually receive:

- superuser;
- BYPASSRLS;
- role administration;
- extension installation;
- destructive migration authority;
- unrelated replication administration.

Break-glass privileges MUST be explicit, protected, time-bounded where feasible, and audited.

---

# 25. Row-Level Security

RLS is defense in depth, not a replacement for application authorization.

When RLS is used, the agent MUST verify:

- default-deny behavior;
- table-owner behavior;
- privileged-role bypass behavior;
- tenant context handling;
- connection pooling interactions;
- real integration behavior.

RLS security claims MUST be tested against real PostgreSQL, not mocks.

The agent MUST never assume that enabling RLS automatically creates correct tenancy.

A privileged connection that can bypass RLS MUST NOT be used as ordinary application traffic.

---

# 26. SQL and SQLx Rules

SQLx is the preferred database access layer.

Use:

- parameterized queries;
- explicit SQL for business-critical queries;
- compile-time query checking where practical;
- version-controlled migrations;
- transactions for atomic business operations.

Never interpolate user-controlled values into SQL fragments.

User input MUST NOT directly control:

- table names;
- column names;
- ORDER BY expressions;
- operators;
- arbitrary WHERE fragments;
- SQL fragments.

When dynamic identifiers are unavoidable, use strict allowlists.

Repositories MUST apply safe query scoping.

Repositories MUST NOT:

- unexpectedly open their own transaction inside every function;
- call external providers;
- decide authorization without the required security context;
- log secrets;
- leak raw DB errors to clients;
- silently retry unsafe operations.

---

# 27. Transactions

A transaction is required whenever multiple authoritative writes must succeed or fail together.

The agent MUST make transaction ownership explicit at the application/unit-of-work layer.

A transaction may coordinate multiple repositories/modules when atomicity is required.

Transactions MUST remain bounded.

Never perform slow external network calls while holding a transaction unless an explicitly justified protocol requires it.

The agent MUST identify transaction isolation and lock requirements for contested state.

---

# 28. Concurrency and Race Conditions

Any scarce or mutable state can race.

Examples:

- stock decrement;
- coupon usage;
- refunds;
- cash close;
- membership changes;
- approval consumption;
- payment capture;
- sync checkpoints;
- idempotency records.

The implementation MUST choose an explicit strategy such as:

- row lock;
- atomic conditional update;
- unique constraint;
- compare-and-set version check;
- narrow serializable transaction;
- ledger/reservation model.

The agent MUST state why the selected strategy preserves the invariant.

Contended resources SHOULD be acquired in consistent order.

Deadlock errors may be retried only when the operation is safe to retry and does not duplicate external effects.

---

# 29. External Integrations

External integrations MUST be isolated behind adapters/ports.

This includes:

- payment providers;
- MRA EIS;
- SMS;
- email;
- push notifications;
- object storage;
- future financial partners.

Provider-specific details MUST NOT leak into the core sales/inventory model unnecessarily.

The adapter boundary SHOULD provide:

- input validation;
- explicit timeout;
- retry policy;
- circuit-breaking/bulkhead behavior where needed;
- provider correlation IDs;
- idempotency;
- error classification;
- evidence/audit hooks.

Provider downtime MUST NOT fabricate business success.

---

# 30. MRA / EIS Rules

MRA EIS is an external regulatory boundary.

The agent MUST NOT claim certification, compliance, production approval, or integration capability without current evidence.

Exact certification pathways and production requirements remain evidence-dependent.

Tax submission is distinct from core sale finalization.

Where asynchronous submission is appropriate:

```text
business commit
 -> outbox
 -> EIS job
 -> external submission
 -> accepted/rejected/pending evidence
```

A temporary EIS outage MUST NOT cause the application to pretend that an unsubmitted document was successfully submitted.

Terminal/credential secrets MUST remain isolated.

---

# 31. Audit vs Domain Events

These are different concepts.

A domain event describes a business fact used by the system.

Examples:

```text
SaleFinalized
InventoryMovementPosted
PaymentSucceeded
```

An audit event describes evidence of an actor/action.

Examples:

```text
REFUND_APPROVAL_GRANTED
USER_ROLE_CHANGED
DEVICE_REVOKED
```

A single transition may produce both.

Audit evidence MUST be protected from casual modification.

Audit logs MUST NOT become a substitute for authoritative transactional state.

Operational telemetry MUST NOT be treated as the canonical business audit trail.

---

# 32. Outbox and Workers

When an authoritative transaction produces asynchronous work:

```text
BEGIN
  authoritative state
  audit event
  outbox record
COMMIT
```

Then:

```text
claim outbox safely
 -> execute
 -> acknowledge or retry
```

The agent MUST preserve the invariant:

> A committed business fact must not silently disappear because the process crashed after the DB commit and before the external side effect.

Workers MUST tolerate duplicate delivery.

Consumers MUST be idempotent.

Every job MUST carry sufficient tenant/scope context.

Workers MUST NOT infer tenant identity from mutable global state.

Worker privilege MUST be minimal. A notification worker should not have inventory mutation authority.

Unbounded worker spawning is prohibited.

Retries MUST have limits and backoff.

---

# 33. Async / Tokio Rules

Tokio is for asynchronous I/O, timers, bounded jobs, and network concurrency.

The agent MUST distinguish CPU work from I/O work.

Do not perform CPU-heavy operations inside latency-sensitive async tasks merely because the function is async.

Heavy parsing/reporting/compression/import computation SHOULD use:

- bounded blocking pools;
- dedicated workers;
- precomputed read models;
- streaming approaches.

Request-derived `spawn` patterns MUST be bounded.

Every external call MUST have a timeout.

Every queue MUST have backpressure.

Do not create accidental denial-of-service paths through unbounded concurrency.

---

# 34. API Design Rules

The API is versioned and command-oriented.

Internal module communication MUST remain in-process unless a true network boundary exists.

The API MUST have:

- strict content type handling;
- request size limits;
- schema validation;
- authentication before tenant resolution;
- authorization before broad data access;
- rate limits where abuse matters;
- idempotency for high-impact operations;
- stable error codes;
- bounded pagination;
- bounded expensive queries;
- explicit timeouts.

The API MUST assume direct hostile HTTP access.

Do not rely on mobile or desktop clients to enforce business rules.

---

# 35. Error Model

Public API errors MUST represent stable business semantics rather than raw infrastructure details.

Recommended error families include:

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

The agent MUST distinguish:

```text
Domain error:
  the business rejected the command.

Infrastructure error:
  the system could not safely determine or complete the result.
```

These classes differ in retry behavior and MUST NOT be collapsed carelessly.

Production responses MUST NOT leak raw database/provider/internal stack details.

Internal diagnostics MAY contain technical details subject to redaction rules.

---

# 36. Resource Exhaustion and DoS Rules

Security and performance are linked.

The agent MUST inspect every potentially expensive endpoint for:

- huge date ranges;
- unlimited result sets;
- expensive search;
- large exports;
- mass imports;
- lock contention;
- connection pool exhaustion;
- repeated expensive retries;
- large payloads.

Controls include:

- bounded filters;
- maximum date ranges;
- pagination;
- query timeouts;
- concurrency limits;
- asynchronous jobs;
- quotas;
- backpressure;
- provider timeouts.

Never implement unlimited exports synchronously in an API request.

---

# 37. Database Query Performance

The agent MUST reason about:

- index selectivity;
- tenant-first access patterns;
- composite indexes;
- partial indexes;
- covering indexes where justified;
- query plans;
- lock duration;
- connection pool saturation;
- sort/aggregation cost;
- report workload isolation.

The agent MUST NOT add indexes blindly.

Every nontrivial index should have an access-pattern justification.

The agent MUST avoid making high-frequency POS paths depend on expensive full scans.

Heavy reports should use read models, precomputation, or asynchronous jobs when necessary.

---

# 38. Connection Pool Rules

Pool size is derived from PostgreSQL capacity and query behavior, not HTTP request count.

A pool can be exhausted by a small number of slow queries.

The agent MUST keep transactions short.

The agent MUST instrument:

- pool saturation;
- query latency;
- slow queries;
- transaction duration;
- connection acquisition latency.

Redis MUST NOT be introduced merely to mask an unbounded or badly indexed SQL query.

---

# 39. Caching Rules

Caches contain derived or read-mostly data only unless an explicit architecture decision says otherwise.

Good candidates:

- catalogue snapshots;
- feature flags;
- report results;
- configuration snapshots.

Bad authority candidates:

- final financial truth;
- canonical inventory balance;
- permissions;
- authorization decisions that must survive cache loss.

Every cache requires:

- invalidation strategy;
- maximum staleness definition;
- failure behavior.

The core product MUST continue safely if a non-authoritative cache is unavailable.

---

# 40. Data Types and Identifiers

Use semantic types where they materially reduce error.

Examples:

```rust
TenantId
BranchId
SaleId
Quantity
Money
Percentage
Sku
Barcode
PhoneNumber
PermissionScope
```

Do not use primitive types interchangeably when semantics differ materially.

Do not overuse advanced type-state machinery solely for sophistication.

Enums SHOULD represent closed state vocabularies.

Identifiers and business numbers are separate concepts.

The agent MUST NOT treat:

```text
UUID / DB identity
sale_number
command_id
idempotency_key
provider_transaction_id
provider_event_id
```

as interchangeable.

Sequential human-readable numbers MUST remain concurrency-safe and scoped correctly.

Database uniqueness remains authoritative.

---

# 41. Money and Quantity

Never use floating-point values for authoritative monetary arithmetic.

Use integer minor units where currency semantics allow, or exact numeric representation where required.

Money MUST include explicit currency semantics.

Quantity semantics MUST be explicit and consistent.

Unit conversions MUST be deterministic and versioned when historical interpretation matters.

A packaging conversion change MUST NOT reinterpret historical inventory quantities.

---

# 42. Time Semantics

Persist business-critical timestamps with timezone-aware semantics.

Avoid ambiguous local timestamps.

Where offline evidence matters, client event time may be stored separately from server receipt time.

The agent MUST distinguish:

- event occurrence time;
- server acceptance time;
- external provider time;
- synchronization time;
- business-day semantics.

Do not silently use device local time as authoritative ordering for financial events.

---

# 43. Deletion, Archiving, and History

Never hard-delete by default.

Hard deletion is appropriate only when the domain explicitly permits it and no audit, financial, regulatory, referential, or investigative requirement forbids it.

Prefer:

- deactivate;
- archive;
- expire;
- revoke;
- supersede.

Never silently delete finalized financial facts.

Never destroy audit evidence merely to simplify UI behavior.

Retention behavior MUST be deliberate and documented.

---

# 44. Reporting Rules

Reports are derived views, not another source of truth.

Every financial report MUST define its population and state semantics.

Examples:

```text
Gross Sales
= finalized sale gross amounts in selected period

Net Sales
= defined gross sales less eligible discounts/returns

Cash Collected
= posted cash events in selected register scope

Payment Settled
= payment intents with authoritative successful/reconciled state

Inventory On Hand
= inventory ledger/projection under defined as-of semantics
```

The agent MUST NOT mix pending/reversed/refunded/finalized records without explicit semantics.

Report logic MUST be testable against canonical state.

---

# 45. Exports

Exports are security-sensitive operations because they are data exfiltration paths.

The agent MUST consider:

- tenant scope;
- user authorization;
- field sensitivity;
- row volume;
- maximum export size;
- asynchronous processing;
- expiring download references;
- audit evidence;
- rate limiting;
- retention;
- object storage permissions.

Unauthorized support exports MUST be impossible and auditable.

Large exports SHOULD be generated asynchronously.

---

# 46. File and Import Security

Imported files are hostile input.

The agent MUST defend against:

- path traversal;
- malicious filenames;
- archive bombs;
- polyglot files;
- parser vulnerabilities;
- oversized uploads;
- decompression abuse;
- malicious structured data;
- CSV injection where output targets spreadsheets.

Upload processing MUST use:

- file-size limits;
- content/type allowlists;
- bounded parsers;
- sandboxing/isolation where required;
- safe temporary directories;
- no arbitrary filesystem paths from user input.

---

# 47. SSRF Rules

Any feature that fetches a URL based on user/provider input MUST be treated as SSRF-sensitive.

The agent MUST consider:

- private IP ranges;
- link-local/metadata addresses;
- localhost;
- DNS rebinding;
- redirects to forbidden networks;
- alternate IP encodings;
- IPv6 forms;
- proxy behavior.

Do not allow arbitrary server-side URL fetching merely because an HTTP client can do it.

Use destination allowlists wherever the business requirement permits.

---

# 48. Web Security

Where a web/admin surface exists, the agent MUST consider:

- XSS;
- CSRF;
- CORS;
- cookie configuration;
- source maps;
- unsafe browser storage;
- clickjacking where applicable;
- CSP;
- origin validation;
- output encoding.

Do not place long-lived sensitive credentials in browser storage merely because it is convenient.

Never render merchant/customer-provided content as trusted HTML without deliberate sanitization and policy.

---

# 49. Mobile and Desktop Security

Released client artifacts are observable.

The agent MUST assume anything bundled into Flutter/Tauri/web clients can be extracted.

No secrets, private API keys, provider credentials, signing secrets, or privileged tokens belong in clients.

Flutter/Tauri security requirements include:

- secure credential storage;
- no forbidden secrets in release artifacts;
- validated deep links;
- controlled environment endpoints;
- production builds without debug diagnostics;
- safe local DB migration;
- stale/tampered sync message resistance;
- verified update packages where supported.

Tauri capabilities MUST be narrowly scoped.

Do not grant filesystem/shell/network permissions broader than required.

---

# 50. Secrets Management

Secrets MUST NOT appear in:

- source code;
- Git history;
- mobile bundles;
- desktop bundles;
- frontend environment exposed to users;
- logs;
- crash payloads;
- database rows unless encrypted and explicitly required;
- test fixtures intended for production-like execution.

Use a managed secret mechanism appropriate to the deployment.

When a secret is compromised:

```text
revoke
 -> rotate
 -> identify blast radius
 -> update deployment
 -> verify old secret fails
```

Never merely “remove it from the current file” while leaving the secret active in history.

---

# 51. Logging and Observability

Use structured telemetry.

Logs/traces SHOULD include enough context to reconstruct behavior:

- correlation ID;
- request ID;
- tenant scope where safe;
- actor/device context where safe;
- operation name;
- outcome;
- latency;
- retry count;
- external provider correlation where useful.

Do NOT log:

- passwords;
- authentication tokens;
- provider secrets;
- signing keys;
- raw payment credentials;
- unnecessary PII;
- full sensitive payloads.

Redaction MUST happen before data leaves the process where practical.

Observability data is itself sensitive.

---

# 52. Required Metrics

When changing security/reliability-sensitive paths, consider instrumentation for:

- authentication failures;
- MFA failures;
- authorization denials;
- webhook signature failures;
- webhook duplicate rate;
- export volume;
- rate-limit rejections;
- device revocations;
- queue depth;
- job retries;
- sync rejection/conflict rate;
- payment reconciliation exceptions;
- DB pool saturation;
- restore success;
- security test pass rate;
- endpoint inventory drift.

Metrics MUST remain privacy-conscious.

---

# 53. Reliability / Failure Semantics

The agent MUST implement graceful degradation, not fabricated success.

Baseline failure expectations:

```text
PostgreSQL unavailable
 -> writes fail safely
 -> eligible offline clients retain commands
 -> no false success

Redis unavailable
 -> core transactions continue if Redis is non-authoritative

MRA unavailable
 -> eligible business operations continue according to tax policy
 -> submissions become pending/queued

Payment provider unavailable
 -> payment remains pending/unavailable
 -> no fabricated confirmation

Object storage unavailable
 -> core transaction continues unless the blob is required
 -> upload state is visible

Notification provider unavailable
 -> business state remains committed
 -> notification job retries
```

The agent MUST identify the failure mode of every external dependency introduced.

---

# 54. Health Endpoints

Separate health semantics where the API exposes them:

```text
/liveness
/readiness
/dependencies
```

Liveness means the process is alive.

Readiness means the process can safely serve requests according to its required dependencies.

Dependency diagnostics are operational information.

A payment provider being down MUST NOT necessarily make process liveness fail.

---

# 55. Graceful Shutdown

On shutdown:

```text
stop accepting new work
 -> finish/abort in-flight requests safely
 -> stop claiming new jobs
 -> bounded drain period
 -> close DB pools
 -> exit
```

Long-running jobs MUST checkpoint enough state to avoid losing durable work.

The agent MUST NOT add shutdown logic that can duplicate non-idempotent external effects.

---

# 56. Release Gates

Security verification is release-blocking.

Minimum production baseline:

```text
[ ] no unresolved critical security findings
[ ] no high-severity authorization/payment/CI exposure without approved exception
[ ] cross-tenant negative tests pass
[ ] privileged MFA enforced where required
[ ] payment signature/replay/duplicate tests pass
[ ] offline restart/replay/revocation tests pass
[ ] no production secrets in source/artifacts/clients/telemetry
[ ] runtime DB identity has no unnecessary superuser/BYPASSRLS privileges
[ ] production endpoints are inventoried and classified
[ ] database migrations are reviewed and tested
[ ] rollback/recovery behavior is understood
[ ] required observability exists
[ ] restore/recovery evidence exists for production releases
```

A scanner failure MUST fail closed for protected release actions.

---

# 57. Security Test Catalogue

When a relevant capability is implemented or changed, test the matching threat classes:

| Area | Required concern |
|---|---|
| Secrets | source/history/artifact/client/log exposure |
| AuthN | credential/MFA/session/recovery bypass |
| AuthZ | BOLA/property/function/tenant/branch bypass |
| Injection | SQL/search/query/command-like input |
| Web | XSS/CSRF/CORS/browser-storage issues |
| Files | traversal/polyglot/archive/parser abuse |
| SSRF | private IP/metadata/DNS rebinding |
| Business abuse | refunds/discounts/stock/exports |
| Replay | webhook/payment/offline command |
| Concurrency | race/deadlock/idempotency |
| CI | workflow injection/token/pinning |
| Cloud | bucket/IAM/network exposure |
| Availability | timeout/quota/backpressure |
| DR | restore/rebuild |

Mock-only tests are insufficient for tenant isolation, RLS, SQL behavior, or other persistence-bound security claims.

Prefer real PostgreSQL integration tests where the database is part of the security boundary.

---

# 58. Required Domain Invariant Regression Tests

The implementation is incomplete until relevant invariants are executable tests.

Identity:

- revoked membership cannot authorize business commands;
- revoked device cannot perform restricted sync;
- user cannot act outside branch scope.

Catalogue:

- inactive/discontinued SKU cannot be sold when policy prohibits;
- duplicate barcode is rejected in tenant scope;
- price changes do not alter historical sales.

Inventory:

- unauthorized adjustment is denied;
- illegal negative stock is denied where policy forbids it;
- duplicate sale does not consume stock twice;
- transfer cannot post one side only;
- forbidden expired/quarantined stock cannot enter restricted sale flows.

Sales:

- sale cannot finalize twice;
- finalized sale cannot be edited into a new total;
- client-modified totals cannot override server calculation.

Returns/refunds:

- refund cannot exceed eligible amount;
- return cannot exceed returnable quantity;
- duplicate refund command is idempotent.

Payments:

- duplicate provider event cannot create duplicate posting;
- invalid signature is rejected;
- provider mismatch becomes controlled reconciliation state.

Cash:

- register closes once;
- variance remains preserved.

Sync:

- client restart preserves queued commands;
- syncing the same batch twice is harmless;
- revoked device commands are rejected/quarantined according to policy.

---

# 59. Property-Based and Fuzz Testing

Property-based testing is particularly valuable for:

- monetary arithmetic;
- inventory transitions;
- idempotency;
- synchronization ordering;
- parser robustness;
- state-machine transitions.

Fuzzing SHOULD target:

- JSON request parsing;
- CSV/import parsing;
- webhook payloads;
- signed payload handling;
- synchronization payloads;
- file metadata;
- other parser-heavy boundaries.

A parser that works on valid examples but crashes or consumes unbounded resources on malformed input is not production-safe.

---

# 60. CI/CD Governance

CI/CD is part of the production attack surface.

The agent MUST preserve separation between untrusted pull-request execution and trusted release/deployment execution.

Do not expose production credentials or deployment authority to arbitrary PR code.

Prefer:

- protected environments;
- short-lived identity;
- OIDC where appropriate;
- pinned actions/dependencies;
- reproducible builds;
- provenance;
- security scanners;
- locked toolchains.

Build dependencies, toolchain versions, and lockfiles are part of the reproducible build boundary.

A dependency upgrade MUST NOT be treated as a cosmetic edit when it changes security, ABI, performance, or transitive dependency behavior.

---

# 61. Dependency Management

Use mature components for commodity capabilities.

Build internally where the implementation is part of Sitolo’s moat.

Build/custom:

- tenant/branch domain semantics;
- inventory ledger semantics;
- financial invariants;
- sale/return/refund state machines;
- offline command model;
- reconciliation semantics;
- domain approval policy;
- Sitolo-specific audit semantics.

Reuse/adopt:

- cryptography;
- OAuth/OIDC protocol implementations;
- password hashing;
- TLS;
- secure storage;
- PostgreSQL;
- static/security analyzers;
- secret management;
- object storage;
- provider SDKs when appropriate.

NEVER invent custom cryptographic primitives.

NEVER invent an OAuth/OIDC protocol.

NEVER replace a mature primitive with homegrown code just to avoid a dependency.

---

# 62. Rust Engineering Standards

Rust code MUST be idiomatic, explicit, and maintainable.

Prefer strong domain types where they prevent meaningful category errors.

Use `thiserror`/typed errors for domain/application semantics.

Use `anyhow` only at appropriate application/edge orchestration boundaries where contextual diagnostics matter more than public taxonomy.

Avoid unnecessary `unsafe`.

Any `unsafe` MUST have a narrowly documented justification and safety proof.

Avoid hidden global mutable state.

Avoid unbounded allocations on attacker-controlled input.

Avoid cloning large structures blindly inside hot paths.

Do not micro-optimize without measurement.

Performance work MUST consider:

```text
CPU
I/O
allocation
lock contention
database latency
network latency
memory pressure
cache behavior
serialization cost
```

---

# 63. Flutter Standards

The Flutter client is a client of the domain/API contract, not the authority.

Organize features around business capabilities rather than a giant page-centric folder hierarchy.

Local persistence is durable operational state for offline continuity.

The UI MUST NOT issue arbitrary SQL.

The UI MUST NOT embed core authorization decisions.

The UI MUST represent provisional/offline/pending/rejected states accurately where backend authority is not yet established.

Client retry loops MUST be bounded and backoff-aware.

HTTP `429` MUST NOT trigger aggressive retry loops.

Production builds MUST not point accidentally at development endpoints.

---

# 64. Tauri Standards

Tauri is a desktop shell/client, not a second backend.

Native capabilities should exist only for actual desktop integration needs such as:

- printing;
- scanners;
- filesystem import/export;
- serial devices;
- secure OS credential storage;
- backup utilities.

Tauri business logic MUST NOT become a second implementation of Rust domain logic.

Capability permissions MUST be minimal.

The agent MUST review filesystem and shell access as security-sensitive.

---

# 65. TypeScript Boundary

TypeScript is intentionally constrained.

Allowed:

- admin/web UI;
- Tauri UI;
- generated API clients;
- API schema tooling;
- documentation tooling;
- release/build tooling;
- OpenAPI linting/generation.

Not allowed as a second source of truth for:

- pricing rules;
- inventory semantics;
- financial reconciliation;
- tenant authorization;
- payment state machines;
- tax authority decisions.

Duplicating core business logic in Rust and TypeScript is an architecture defect unless a documented contract explicitly requires it and a single source of truth remains authoritative.

---

# 66. Business Vertical Extensions

Shared retail core remains authoritative.

Vertical extensions include:

```text
pharmacy
agro-dealer
wholesale
```

Do NOT fork the entire engine for each vertical.

Pharmacy may add:

- batch/lot;
- expiry;
- FEFO/FIFO policy;
- controlled-stock flags;
- restricted permissions;
- prescription metadata where legally required.

The system MUST NOT implement clinical decision support or pretend to replace professional pharmaceutical judgment.

Agro may add:

- batch/formulation metadata;
- seasonal/restricted-product policies;
- traceability.

Wholesale may add:

- larger orders;
- customer accounts;
- price lists;
- receivables;
- dispatch;
- fulfilment/approval stages.

Shared concepts SHOULD stay shared where semantics are genuinely common.

---

# 67. Entitlements and Billing

Commercial entitlements are server-side business policy.

Do not rely on client UI to enforce plan limits.

When implementing an entitlement:

- identify the owning billing context;
- define the protected capability;
- enforce server-side;
- make failure explicit;
- audit high-impact changes;
- ensure existing paid operational data is not silently destroyed by downgrade.

Billing state MUST NOT grant accidental access to unrelated tenant resources.

Usage metering SHOULD be based on authoritative business facts or clearly defined usage events.

---

# 68. API Compatibility and Versioning

Public API contracts are compatibility boundaries.

Breaking changes require:

- explicit versioning strategy;
- migration path;
- deprecation plan;
- documentation;
- test updates;
- client compatibility considerations.

Deprecated APIs/fields/features MUST have:

```text
replacement
migration path
last supported version
removal condition/date
owner
```

Permanent compatibility without an exit path is technical debt.

---

# 69. Schema Migration Governance

Every migration MUST be:

- version-controlled;
- reproducible;
- reviewed;
- tested against realistic data shape where practical;
- assessed for lock behavior;
- assessed for backfill size;
- assessed for backward compatibility;
- assessed for security impact;
- assessed for performance impact;
- paired with recovery/rollback understanding.

Migration reviews MUST answer:

```text
why is schema changing?
what objects change?
what locks can occur?
how large is the backfill?
what old/new binaries coexist?
what is the rollback/recovery plan?
what is the security impact?
what is the performance impact?
```

Use expand-and-contract for changes that need compatibility across application versions.

Do not run destructive schema operations casually in production.

---

# 70. Database Definition of Done

A DB-backed feature is not complete until, where applicable:

```text
[ ] domain owner identified
[ ] tenant scope identified
[ ] authoritative vs derived state identified
[ ] invariants documented
[ ] PK/FK/UNIQUE/CHECK constraints considered
[ ] indexes justified
[ ] transaction boundary defined
[ ] concurrency behavior tested
[ ] RLS decision made
[ ] DB role permissions reviewed
[ ] migration written
[ ] recovery/rollback understood
[ ] SQLx queries verified
[ ] negative security tests exist
[ ] performance assessed
[ ] observability added
[ ] retention defined
[ ] backup/restore impact assessed
```

---

# 71. Backup and Restore

Backups are useless if restore is unverified.

The agent MUST treat restore capability as an operational feature.

Changes affecting persisted data MUST consider:

- backup compatibility;
- restore compatibility;
- migration ordering;
- historical evidence;
- point-in-time recovery implications where applicable.

Production readiness requires restore evidence, not merely existence of backup configuration.

---

# 72. Incident Response Awareness

Security-sensitive implementations MUST leave the system capable of controlled response.

Minimum incident actions should be possible for relevant domains:

Account takeover:

```text
revoke sessions/devices
 -> force recovery
 -> inspect audit
 -> assess tenant
 -> preserve evidence
```

Tenant isolation incident:

```text
disable affected path
 -> identify exposure window
 -> preserve logs/audit
 -> patch
 -> rerun isolation suite
```

Financial integrity incident:

```text
freeze affected flow
 -> stop unsafe settlement
 -> reconcile against trusted evidence
 -> preserve immutable history
```

Secret compromise:

```text
revoke
 -> rotate
 -> assess blast radius
 -> redeploy
 -> verify old secret fails
```

Supply-chain incident:

```text
stop releases
 -> isolate builder
 -> revoke identities
 -> identify affected artifacts
 -> redeploy verified build
```

The agent MUST not make incident response impossible through irreversible convenience changes.

---

# 73. Admin and Support Access

Support access to merchant data is privileged access.

The agent MUST apply:

- just-in-time access where appropriate;
- least privilege;
- scope restriction;
- explicit approval where required;
- time limitation where feasible;
- complete audit evidence.

Support users MUST NOT silently mutate merchant financial truth merely because an admin API exists.

Admin tooling MUST not become a backdoor around ordinary domain invariants.

---

# 74. Feature Flags and Kill Switches

High-risk features SHOULD have controlled rollout/disable mechanisms when operationally justified.

Rollouts can follow:

```text
internal tenants
 -> pilot tenants
 -> 10%
 -> 50%
 -> 100%
```

Rollout systems MUST be auditable.

Abort rollout when evidence shows:

- sale failure increase;
- sync conflict/rejection spike;
- payment webhook failure increase;
- crash-rate increase;
- critical security finding.

Feature flags MUST NOT become a substitute for authorization.

---

# 75. Maintenance Mode

Where maintenance mode exists, it should support controlled behavior such as:

- allow safe reads/login where appropriate;
- block risky mutations;
- expose planned maintenance state;
- preserve offline continuity when safe.

Do not shut down the entire system for every migration if the architecture can safely support phased behavior.

---

# 76. Architecture Decision Records

Significant architectural decisions SHOULD be recorded using:

```text
Context
Decision
Alternatives
Tradeoffs
Consequences
Migration/rollback
Status
Date
Owner
```

Architecture review is particularly required for changes involving:

- security boundaries;
- financial semantics;
- synchronization protocol;
- new external providers;
- new databases;
- new service boundaries;
- regulated workflows.

Ordinary UI changes do not require unnecessary bureaucracy.

---

# 77. Technical Debt Policy

Classify technical debt as:

```text
safe shortcut
known performance debt
security risk
correctness risk
architecture debt
```

Priority order is broadly:

```text
correctness/security
 -> reliability/data integrity
 -> architecture
 -> measurable performance
 -> maintainability
 -> cosmetics
```

A cosmetic refactor MUST NOT outrank a correctness defect.

Temporary hacks MUST be explicit. A hack is not “architecture” simply because it was merged.

---

# 78. Anti-Patterns — Automatic Rejection

The agent MUST reject or refactor implementations containing patterns such as:

1. trusting client totals;
2. trusting client roles or tenant IDs;
3. UI-only authorization;
4. editing finalized financial facts directly;
5. overwriting inventory balance without ledger evidence;
6. duplicate business logic in multiple languages;
7. HTTP calls between internal modules in the same process;
8. unbounded request-derived spawning;
9. missing external-call timeouts;
10. long DB transactions around network calls;
11. check-then-insert without uniqueness where needed;
12. raw SQL interpolation;
13. raw DB errors returned to clients;
14. secrets embedded in clients;
15. secrets committed to Git;
16. provider callback without authentication/integrity validation;
17. webhook handling without replay/idempotency controls;
18. background jobs without tenant scope;
19. worker privilege broader than required;
20. reports treated as canonical truth;
21. Redis treated as financial authority;
22. mock-only tenant isolation claims;
23. arbitrary URL fetching without SSRF controls;
24. unbounded report/export endpoints;
25. blind automatic retries of unsafe mutations;
26. silent data deletion for convenience;
27. pretending a provider outage is a success;
28. claiming regulatory certification without evidence;
29. introducing a new service/database without justified boundary;
30. disabling security tests to unblock a build.

---

# 79. Security Severity and Release Policy

Use:

```text
Critical
High
Medium
Low
Informational
```

Critical includes:

- tenant boundary bypass;
- authentication bypass;
- production RCE;
- signing-key compromise;
- financial truth forgery.

Critical MUST block release immediately.

High includes serious:

- privilege escalation;
- payment forgery;
- dangerous SSRF;
- CI compromise;
- equivalent systemic integrity failures.

High findings in production-sensitive boundaries MUST block release unless an explicitly approved emergency exception exists.

Exceptions MUST contain:

```text
violated control
affected asset
rationale
compensating control
owner
expiry date
rollback plan
```

Permanent exceptions are prohibited.

---

# 80. Evidence-Driven Implementation

The agent MUST distinguish among:

```text
Known fact
Architecture requirement
Inference
Hypothesis
Open decision
External dependency behavior
```

Do not manufacture evidence.

For current regulatory/provider requirements, the agent MUST verify current authoritative documentation before claiming support.

A design document saying “integration planned” is not proof of production certification.

A code path existing is not proof that a feature is secure.

A test passing is not proof that an entire trust boundary is covered unless the test actually exercises the boundary.

---

# 81. Definition of Done — General Feature

A feature is done only when relevant items are satisfied:

```text
[ ] requirement understood
[ ] owning bounded context identified
[ ] domain invariants identified
[ ] security boundary identified
[ ] tenant/branch scope identified
[ ] authorization matrix considered
[ ] DTO/domain/persistence boundaries preserved
[ ] transaction boundary defined
[ ] concurrency behavior defined
[ ] idempotency considered
[ ] offline behavior defined where applicable
[ ] dependency failure behavior defined
[ ] audit requirement defined
[ ] observability added
[ ] tests cover happy path
[ ] tests cover negative/security path
[ ] tests cover retry/replay where applicable
[ ] tests cover concurrency where applicable
[ ] migration reviewed where applicable
[ ] API compatibility considered
[ ] resource limits defined
[ ] documentation updated where behavior changes
[ ] rollback/disable path understood for high-risk changes
```

A green build alone does not satisfy this definition.

---

# 82. Definition of Done — Security-Sensitive Feature

Additionally require:

```text
[ ] explicit threat model
[ ] trust boundary identified
[ ] authentication requirement
[ ] object/function/property authorization
[ ] tenant isolation
[ ] branch isolation if applicable
[ ] input limits
[ ] replay/idempotency protection
[ ] audit evidence
[ ] log redaction
[ ] secrets review
[ ] abuse scenario tests
[ ] negative tests
[ ] real integration tests where needed
[ ] incident/kill-switch consideration
[ ] release gate evidence
```

---

# 83. Definition of Done — Financial/Inventory Feature

Additionally require:

```text
[ ] authoritative state identified
[ ] immutable/history semantics defined
[ ] compensating action defined where needed
[ ] atomic transaction identified
[ ] concurrency strategy selected
[ ] idempotency strategy selected
[ ] ledger/evidence relationship defined
[ ] retry behavior proven
[ ] failure behavior proven
[ ] reconciliation implications considered
[ ] audit trail implemented
[ ] report semantics updated where relevant
```

---

# 84. Definition of Done — Offline Feature

Additionally require:

```text
[ ] local persistence semantics defined
[ ] command identity defined
[ ] restart durability tested
[ ] duplicate sync tested
[ ] stale/revoked device tested
[ ] conflict behavior defined
[ ] authoritative server validation present
[ ] client provisional state represented correctly
[ ] sync retries bounded
[ ] no false authority claim
```

---

# 85. Definition of Done — Database Feature

Additionally require:

```text
[ ] tenant scope
[ ] owner module
[ ] canonical/derived classification
[ ] constraints
[ ] indexes
[ ] transaction boundary
[ ] lock behavior
[ ] RLS decision
[ ] role privilege impact
[ ] migration strategy
[ ] rollback/recovery understanding
[ ] query verification
[ ] negative security test
[ ] performance assessment
```

---

# 86. Agent Change Workflow

For a non-trivial task, follow this workflow:

```text
1. Understand request
2. Identify affected bounded contexts
3. Inspect current implementation
4. Read governing design sections
5. Identify invariants and trust boundaries
6. Identify persistence changes
7. Design transaction/concurrency/idempotency behavior
8. Implement smallest correct change
9. Add/update tests
10. Run static/security checks
11. Run relevant integration tests
12. Inspect diff for architectural violations
13. Review performance/resource behavior
14. Document material decisions
15. Report exactly what changed and what remains unverified
```

The agent MUST NOT skip directly from issue description to code when the change affects security, finance, inventory, synchronization, persistence, or external integrations.

---

# 87. Repository Inspection Rules

Before modifying an unfamiliar area, inspect:

- crate/module ownership;
- dependency direction;
- nearby domain types;
- existing repository patterns;
- migrations;
- authorization middleware/policies;
- tests;
- error conventions;
- observability conventions;
- job/outbox patterns;
- existing feature flags;
- related documentation.

Search for existing names before inventing new names.

The agent MUST not duplicate a concept that already exists under another name without a documented reason.

---

# 88. Implementation Minimality

“Smallest correct change” does not mean “fewest lines”.

It means the smallest change that preserves:

- architecture;
- security;
- domain invariants;
- operational behavior;
- testability;
- future compatibility.

Do not use a shortcut that merely moves complexity into hidden state.

Do not delete tests just because they become inconvenient after a change.

Do not weaken validation to make a fixture pass.

Do not weaken authorization because a UI flow is difficult.

---

# 89. Testing Philosophy

Tests MUST reflect system boundaries.

Use multiple layers:

```text
pure domain unit tests
application/service tests
repository/database integration tests
API/contract tests
security boundary tests
property tests
concurrency tests
fuzz tests
end-to-end tests where justified
```

A unit test passing does not prove tenant isolation.

A mock provider passing does not prove webhook verification.

A handler test passing does not prove SQL RLS behavior.

Use real infrastructure where the infrastructure itself is part of the security/correctness claim.

---

# 90. Test Naming and Intent

Tests SHOULD make the invariant obvious.

Prefer:

```text
finalizing_same_sale_twice_is_idempotent
cross_tenant_sale_read_is_denied
replayed_provider_event_has_no_second_effect
revoked_device_sync_is_rejected
concurrent_stock_decrement_cannot_oversell
```

Avoid vague names like:

```text
works
handles_case
sale_test
```

Security tests should describe attacker intent where practical.

---

# 91. Performance Engineering Rules

Performance targets are workflow-oriented.

Prioritize:

- fast POS commit paths;
- responsive offline local writes;
- bounded memory use on low-end devices;
- predictable API latency;
- low DB contention;
- async heavy reporting;
- efficient bulk processing.

Do not chase benchmark vanity at the expense of correctness.

When a performance issue is suspected, measure first.

Profile CPU, memory, allocations, DB latency, network latency, and lock waits rather than assuming the bottleneck.

A micro-optimization that increases code complexity without measurable benefit is not automatically good engineering.

---

# 92. Memory and Resource Discipline

The target ecosystem includes low-end Android devices and real merchant hardware.

The agent MUST avoid:

- loading entire datasets into memory unnecessarily;
- unbounded queues;
- unbounded retries;
- unbounded response sizes;
- giant in-memory report generation;
- duplicate copies of large payloads.

Prefer streaming where practical.

Bound all attacker-controlled resource dimensions.

---

# 93. Privacy and Data Minimization

Collect only data required for the business or justified operational function.

Sensitive customer/staff/device data MUST have an explicit reason for collection.

Do not collect telemetry merely because it is technically possible.

The agent MUST consider:

- storage scope;
- access scope;
- retention;
- export exposure;
- logs/crash reports;
- client-side caches;
- backups.

Privacy-sensitive changes require security review.

---

# 94. Production Configuration

Configuration MUST be explicit and environment-aware.

The agent MUST NOT silently introduce development defaults into production-sensitive behavior.

Production configuration SHOULD distinguish:

- required secrets;
- safe defaults;
- feature flags;
- external endpoints;
- resource limits;
- logging levels;
- rollout controls.

Fail closed when required security configuration is absent.

Do not silently substitute insecure defaults for missing credentials or keys.

---

# 95. Open Decisions Policy

Some decisions are intentionally not final.

Examples include, where still open in project documents:

- exact identity provider;
- hosting provider/region topology;
- Redis adoption;
- reporting infrastructure;
- exact payment provider semantics;
- MRA production/certification details;
- jurisdiction-specific pharmacy requirements;
- dedicated tenant isolation tier.

The agent MUST preserve replaceable boundaries for these decisions.

Do not hard-wire an open choice so deeply that changing it requires rewriting core domains.

Do not pretend an open decision is final merely to finish an implementation.

---

# 96. Regulatory and Commercial Boundaries

Sitolo is a business operating system, not automatically:

- a bank;
- a lender;
- an insurer;
- a wallet;
- a generic marketplace.

Future financial services are integration opportunities, not permission to blur the core system’s authority model.

Do not implement regulated financial activity merely because a future business model mentions it.

Legal/regulatory status requires current authoritative evidence.

Commercial pricing assumptions are hypotheses until validated.

The agent MUST not create architecture that permanently depends on unvalidated revenue assumptions.

---

# 97. Why Simplicity Matters

Sitolo is designed for merchants, not distributed-systems specialists.

Complexity belongs behind stable interfaces.

A merchant-facing action should not expose:

- event-sourcing internals;
- conflict vectors;
- retry mechanics;
- provider reconciliation internals;
- database lock behavior.

The engineering system MAY be sophisticated while the merchant experience remains simple.

Do not expose infrastructure complexity as product complexity merely because it already exists internally.

---

# 98. Why the Architecture Favors a Modular Monolith

The hardest distributed boundary is already:

```text
offline client <-> server
```

Additional network boundaries between sales, inventory, cash, payments, and reconciliation would introduce:

- distributed transaction problems;
- additional failure modes;
- network latency;
- operational complexity;
- deployment coupling.

Therefore the agent MUST prefer in-process module boundaries until evidence justifies extraction.

---

# 99. Why Rust Owns the Core

Rust provides the repository’s core implementation environment for:

- domain logic;
- transactions;
- authorization;
- integrations;
- workers;
- sync;
- audit;
- database access.

The point is not language ideology.

The point is maintaining one authoritative business implementation rather than parallel domain engines.

The agent MUST not introduce a second backend language without a documented architectural decision.

---

# 100. Why PostgreSQL Owns Truth

PostgreSQL provides the durable relational boundary required by:

- organizations;
- memberships;
- branches;
- catalogue;
- procurement;
- inventory;
- sales;
- cash;
- payments;
- reconciliation;
- billing;
- audit;
- synchronization durability.

The agent MUST not create a second authoritative database merely because a feature is easier to implement in another store.

Any new database requires architecture review.

---

# 101. Agent Output Contract

When the agent finishes a task, its report MUST be factual.

It SHOULD state:

```text
Implemented:
  exact changes

Security:
  controls added/verified

Data/DB:
  migrations/constraints/indexes

Tests:
  exactly what ran and passed

Not verified:
  anything not actually executed or proven

Known limitations:
  remaining risks/open decisions

Operational impact:
  deployment/recovery/observability considerations
```

The agent MUST NOT claim tests passed when they were not run.

The agent MUST NOT claim production readiness when production-only infrastructure was not exercised.

The agent MUST distinguish “implemented” from “validated”.

---

# 102. No Fabricated Success

Never fabricate:

- test results;
- benchmarks;
- security scan results;
- migration success;
- deployment status;
- provider availability;
- certification;
- compliance;
- data migration completion;
- rollback success.

When a tool or environment is unavailable, say so explicitly.

Partial implementation with honest evidence is preferable to false completion.

---

# 103. Change Review Questions

Before considering a change complete, ask:

### Architecture

- Does dependency direction remain correct?
- Did a framework detail leak into domain code?
- Did this create a hidden service boundary?
- Did I introduce a second source of truth?

### Security

- Can a caller cross tenants?
- Can a caller bypass authorization by calling HTTP directly?
- Are object/function/property checks present?
- Can an attacker replay this operation?
- Can an attacker exhaust CPU, memory, DB connections, or external calls?
- Are secrets protected?

### Domain correctness

- What invariant can this code violate?
- Is the state transition explicit?
- Is historical truth preserved?
- Are corrections compensating rather than destructive?

### Concurrency

- What happens if two requests race?
- What happens if the command retries?
- What happens if the worker runs twice?
- Can a DB deadlock or unique constraint conflict occur?

### Offline

- What happens after restart?
- What happens after revocation?
- What happens when the same command syncs twice?
- What happens when the server state changed while offline?

### External failure

- What happens on timeout?
- What happens on duplicate callback?
- What happens on provider mismatch?
- What happens if the provider is down for hours?

### Database

- Are constraints doing useful work?
- Are indexes justified?
- Is the migration backward-compatible?
- Does RLS behave correctly under real roles?

### Operations

- Can we observe failures?
- Can we disable the feature?
- Can we recover?
- Can we investigate after the fact?

---

# 104. Escalation Conditions

The agent MUST treat the following as requiring architectural/security consideration before implementation:

- new database;
- new service boundary;
- new authentication mechanism;
- new authorization model;
- new payment provider;
- new tax/regulatory workflow;
- major offline-sync protocol change;
- financial state-model change;
- inventory ledger change;
- encryption/key-management change;
- production CI identity change;
- support-access model change;
- data-retention policy change;
- cross-region data architecture change.

Do not hide such changes inside an ordinary feature PR description.

---

# 105. Repository Hygiene

Keep the repository reproducible.

Required hygiene includes:

- explicit Rust toolchain configuration;
- committed appropriate lockfiles;
- reproducible migrations;
- documented generated artifacts;
- no secrets;
- no abandoned debug files;
- no unexplained binaries;
- no generated source treated as hand-authored authority.

Generated API types are generated artifacts, not the source of business semantics.

The agent MUST identify generated files correctly and regenerate them through the project’s intended mechanism.

---

# 106. Code Review Quality Bar

Code must be reviewable.

Prefer:

- explicit names;
- narrow functions;
- clear ownership;
- typed errors;
- explicit transactions;
- explicit authorization;
- bounded resource behavior;
- tests near the invariant;
- comments explaining why, not restating what.

Avoid:

- giant handlers;
- hidden side effects;
- magic constants for policy;
- unexplained global state;
- generic “utils” packages that become dumping grounds;
- repository methods that expose unrestricted mutation;
- abstractions created only to look sophisticated.

---

# 107. Final Agent Commandments

These rules are absolute unless superseded by a higher-authority requirement explicitly documented by the project:

```text
1. PostgreSQL is authoritative server state.
2. Clients request; the server decides.
3. Tenant isolation is non-negotiable.
4. Authorization is server-enforced.
5. UI state is never security authority.
6. Financial history is not casually mutable.
7. Inventory changes require ledger semantics.
8. High-impact commands are idempotent where retryable.
9. External providers are untrusted/failure-prone boundaries.
10. Never hold long DB transactions across network calls.
11. Outbox before fragile post-commit side effects.
12. Workers must tolerate duplicates.
13. Offline continuity must never become permanent client authority.
14. Domain logic does not depend on frameworks or infrastructure.
15. Internal modules do not communicate over HTTP without an actual network boundary.
16. Do not create microservices for decoration.
17. Do not invent cryptography or authentication protocols.
18. Do not expose secrets to clients.
19. Bound CPU, memory, DB, queue, retry, and external-call resources.
20. Every security-sensitive claim requires evidence.
21. Every critical invariant should become an executable test.
22. Real database boundaries require real integration testing.
23. A scanner/test failure cannot be silently bypassed.
24. Do not fabricate successful validation.
25. When truth is uncertain, preserve evidence and enter an explicit exception state.
```

---

# 108. Governing Project References

This contract is derived from the project’s governing design documents:

- `business_model_design.md` — commercial model, segments, product surfaces, monetization, retention, enterprise strategy, regulatory/commercial boundaries.
- `system_architecture_design.md` — runtime architecture, module boundaries, Rust/Axum/Tokio, Flutter/Tauri, PostgreSQL/SQLite, workers, adapters, sync, reliability, performance, operations.
- `security_architecture_design.md` — zero-trust security contract, threat model, authorization, tenant isolation, API security, CI/CD security, release gates, incident controls.
- `domain_model.md` — domain concepts, aggregates, value objects, commands, events, state machines, invariants, bounded contexts, offline semantics.
- `database_design.md` — PostgreSQL schema model, constraints, RLS, roles, SQLx, transaction boundaries, concurrency, migrations, indexing, backup/restore, database DoD.

These documents remain the authoritative project context for detailed decisions. This file converts their implementation implications into operational rules for an AI coding agent.

---

# 109. Closing Rule

When uncertain, do not guess in a way that weakens the system.

Choose the interpretation that preserves:

```text
security
  > correctness
  > data integrity
  > explicit authority
  > recoverability
  > maintainability
  > performance
  > convenience
```

Then leave evidence of the decision.

**The agent is trusted to change code only to the extent that the resulting system remains understandable, testable, tenant-safe, financially correct, operationally recoverable, and faithful to Sitolo’s governing architecture.**
