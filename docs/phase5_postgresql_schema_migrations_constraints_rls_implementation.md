# SITOLO — Phase 5 PostgreSQL Schema, Migrations, Constraints and Row-Level Security Implementation Specification

**Document:** `phase5_postgresql_schema_migrations_constraints_rls_implementation.md`  
**Phase:** Phase 5 — PostgreSQL schema + migrations + constraints + RLS  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African regional expansion  
**Backend:** Rust / Axum / Tokio  
**Database:** PostgreSQL 18 target line  
**Database access:** Rust + SQLx  
**Clients:** Flutter Android-first, Tauri desktop, TypeScript where materially useful  
**Architecture:** Modular monolith first; durable workers; explicit integration adapters  
**Authority:** PostgreSQL is authoritative for server-side business state  
**Security posture:** zero trust, deny by default, least privilege, defense in depth, fail closed  
**Document status:** Implementation-governing contract  
**Date:** 6 September 2026

---

## 0. Executive Contract

This document defines how Sitolo turns the already-frozen business/domain/security architecture into an executable PostgreSQL foundation.

This is **not** a generic PostgreSQL tutorial, a table inventory, or a SQL style guide. It is the implementation contract for the database authority underneath a mobile-first, offline-capable, multi-tenant Business Operating System serving African SMEs.

The database exists to preserve business truth under conditions that ordinary CRUD systems routinely mishandle:

```text
multiple staff
+ multiple branches
+ concurrent POS transactions
+ unstable mobile connectivity
+ retries
+ duplicate webhooks
+ offline commands
+ provider ambiguity
+ authorization changes
+ inventory races
+ financial corrections
+ regulatory evidence
+ service restarts
+ schema evolution
+ disaster recovery
= database correctness problem
```

The database therefore has five simultaneous responsibilities:

1. preserve relational integrity;
2. enforce cheap and fundamental invariants;
3. provide transactional atomicity for business operations;
4. add defense-in-depth tenant isolation through RLS and privileges;
5. retain durable evidence necessary for synchronization, reconciliation, auditing and recovery.

The controlling authority model is:

```text
CLIENT
  |
  | HTTPS / Sync / Commands
  v
RUST APPLICATION
  |
  | authenticate / authorize / validate / domain rules
  v
POSTGRESQL TRANSACTION
  |
  +--> business state
  +--> financial facts
  +--> inventory ledger
  +--> audit evidence
  +--> idempotency state
  +--> sync command state
  +--> outbox intent
  |
  v
COMMIT
  |
  v
WORKERS / INTEGRATIONS / READ MODELS
```

PostgreSQL 18 is the target database line for the implementation baseline. PostgreSQL 18 documentation currently covers the constraint, row-security and transactional primitives this specification relies on. citeturn794774search8turn794774search1turn794774search0

The central rule is:

> **The application decides business intent; PostgreSQL guarantees durable transactional truth and relational integrity; RLS and least-privilege roles provide a database-side safety boundary; neither the API nor the client may become a competing source of truth.**

---

# 1. Relationship to Existing Sitolo Contracts

This document is subordinate to the existing product, domain, security and architecture decisions.

The source-of-truth hierarchy remains:

```text
LAW / REGULATION / CONTRACT
        |
        v
BUSINESS MODEL
        |
        v
PRODUCT / DOMAIN MODEL
        |
        v
SYSTEM ARCHITECTURE
        |
        v
SECURITY ARCHITECTURE
        |
        v
AUTH / AUTHZ CONTRACT
        |
        v
DATABASE DESIGN
        |
        v
THIS IMPLEMENTATION SPECIFICATION
        |
        v
MIGRATIONS / SQL / RUST CODE
```

The existing database contract already establishes PostgreSQL as authoritative, SQLx as the access layer, SQLite as client-side operational continuity, a modular monolith, transactional outbox semantics, append-oriented financial truth, ledger-backed inventory, explicit tenant scope, least-privileged runtime roles and RLS as defense in depth. fileciteturn7file0L45-L113

The domain model separately establishes that domain behavior belongs in Rust and that repositories/infrastructure own SQLx and PostgreSQL concerns. fileciteturn7file2L380-L417

The API contract establishes that clients are untrusted for authority, that explicit commands are preferred over CRUD, that idempotency is required for retryable business mutations, and that database schema is not itself a public API. fileciteturn4file0L26-L76

The authorization contract establishes the hierarchy of principal, session/device state, membership, organizational scope, permissions, object/property authorization, domain state and approvals. Database RLS is therefore one layer of enforcement, not a replacement for application authorization.

### 1.1 Explicit non-contradiction rules

This database implementation MUST NOT introduce:

- a second tenant model;
- database authority over business decisions that belong in the domain layer;
- generic CRUD mutation paths that bypass domain commands;
- destructive edits to finalized financial facts;
- a database role that can silently bypass required production security controls;
- schema conventions that leak provider-specific concepts into the retail domain;
- a migration process that requires downtime without an architecture decision where zero/low-downtime rollout is required;
- a false “RLS means secure” assumption.

### 1.2 Why database implementation happens here

The previous phases define identity, tenant hierarchy, authorization, observability and implementation boundaries. Phase 5 makes those contracts executable.

Without this phase, the project can have excellent Rust code and still fail because:

```text
foreign key missing
       -> orphaned business record

unique constraint missing
       -> duplicate idempotency key

RLS missing
       -> cross-tenant disclosure

WITH CHECK missing
       -> cross-tenant insert/update

wrong index
       -> POS latency under growth

long transaction
       -> lock contention

unsafe migration
       -> production outage

wrong role privilege
       -> application can bypass policy
```

---

# 2. Design Objectives

The database implementation must satisfy all of the following simultaneously.

## 2.1 Correctness

The database MUST preserve:

- entity identity;
- referential integrity;
- uniqueness rules;
- monetary representation;
- currency consistency;
- state relationships;
- tenant ownership;
- branch scope where applicable;
- inventory movement integrity;
- financial history integrity;
- idempotency uniqueness;
- outbox durability;
- sync command identity;
- audit evidence integrity.

## 2.2 Security

The database MUST:

- separate migration/DDL authority from application runtime authority;
- deny access by default where RLS is enabled;
- avoid accidental `BYPASSRLS` privileges for application roles;
- prevent direct modification of protected historical facts where possible;
- constrain dangerous operations through privileges and schema ownership;
- support tenant-scoped queries safely;
- prevent unsafe dynamic SQL in application paths;
- make authorization failures testable.

PostgreSQL's RLS model requires the table to have row security enabled before policies apply, and `USING` and `WITH CHECK` have different purposes: existing rows are filtered by `USING`, while proposed inserted/updated rows are validated by `WITH CHECK`. This distinction is critical for Sitolo because read isolation without write isolation is incomplete. citeturn794774search0

## 2.3 Performance

The database MUST remain fast enough for merchant-critical workloads, particularly:

```text
POS sale
inventory decrement
cash operation
payment lookup
sync command ingestion
```

Reporting, exports, reconciliation and integration workers MUST NOT be allowed to starve transactional POS workloads.

## 2.4 Evolvability

Schema changes MUST support rolling deployment where operational architecture requires it.

A migration is considered safe only when both schema and application versions have been analyzed.

## 2.5 Recoverability

Database changes MUST be reproducible from source.

The migration history, schema state, RLS policies, roles and critical extensions MUST be recoverable from version-controlled definitions plus documented deployment infrastructure.

---

# 3. Database Authority Model

## 3.1 What PostgreSQL owns

PostgreSQL owns durable server-side state for:

- organizations;
- businesses/entities;
- branches;
- locations;
- warehouses;
- registers;
- users and identity references;
- memberships;
- roles and permissions;
- devices;
- products and SKUs;
- prices and price history;
- suppliers and procurement;
- stock ledger and inventory projections;
- sales;
- sale items;
- returns/refunds/reversals;
- cash sessions and cash movements;
- payment intents and payment evidence;
- reconciliation cases;
- EIS/fiscal submission state;
- audit records;
- synchronization command state;
- idempotency records;
- transactional outbox;
- background job state;
- reporting/export job state;
- billing and entitlement state.

## 3.2 What PostgreSQL does not own alone

PostgreSQL MUST NOT become the sole enforcement mechanism for:

- user-facing authorization policy semantics;
- cryptographic token issuance;
- external provider protocol behavior;
- mobile UI state;
- offline conflict policy;
- business workflow orchestration;
- complex multi-resource policies that are more safely expressed and tested in Rust.

## 3.3 Defense-in-depth model

The security model is intentionally layered:

```text
EDGE
  |
API REQUEST BOUNDS
  |
AUTHENTICATION
  |
SESSION / DEVICE
  |
TENANT / MEMBERSHIP
  |
AUTHORIZATION
  |
DOMAIN INVARIANTS
  |
TRANSACTION
  |
DATABASE PRIVILEGES
  |
RLS
  |
CONSTRAINTS
  |
AUDIT / OUTBOX
```

A single failure should not automatically become a tenant compromise.

---

# 4. Logical Schema Strategy

## 4.1 Schema naming

The recommended PostgreSQL logical separation is:

```text
sitolo_identity
sitolo_tenant
sitolo_catalogue
sitolo_procurement
sitolo_inventory
sitolo_sales
sitolo_cash
sitolo_payments
sitolo_reconciliation
sitolo_tax
sitolo_audit
sitolo_sync
sitolo_platform
sitolo_billing
sitolo_reporting
```

A simpler physical deployment MAY use a single database schema with domain prefixes in the early implementation if operational evidence shows that multiple PostgreSQL schemas materially increase complexity without improving security. The domain boundaries must remain in Rust even when the database is physically consolidated.

The selected strategy MUST be recorded in the repository architecture document before migrations become numerous enough to make renaming expensive.

## 4.2 Why logical schemas exist

Schemas are useful for:

- ownership boundaries;
- permissions;
- migration discoverability;
- operational inspection;
- accidental cross-domain write detection.

They are not microservices.

Creating fourteen schemas does not create fourteen services.

## 4.3 Shared platform tables

Some cross-cutting tables legitimately live in platform/security areas:

```text
organizations
users
memberships
devices
roles
permissions
role_permissions
idempotency_keys
outbox_messages
security_events
jobs
```

Cross-domain references are allowed when they represent genuine ownership relationships, but another domain MUST NOT casually mutate an owned table.

---

# 5. Identifier Strategy

## 5.1 Requirements

Public IDs must be:

- globally unique within their entity domain;
- safe for offline generation where required;
- resistant to enumeration;
- stable across retries;
- compact enough for APIs and logs;
- indexable;
- suitable for correlation.

## 5.2 Default strategy

Use typed Rust newtypes over UUID/ULID-like storage IDs rather than passing plain strings throughout domain code.

Conceptually:

```rust
struct OrganizationId(Uuid);
struct BranchId(Uuid);
struct ProductId(Uuid);
struct SaleId(Uuid);
struct PaymentIntentId(Uuid);
struct DeviceId(Uuid);
struct CommandId(Uuid);
```

The exact identifier format MUST remain aligned with the existing repository strategy. Changing it later requires migration and compatibility analysis.

## 5.3 Human-readable business numbers

Human-facing values such as:

```text
SALE-2026-000123
INV-2026-000812
PO-2026-0012
```

are not primary keys.

They are business references and require their own uniqueness rules at the correct organizational scope.

## 5.4 Offline identity

Offline clients may generate command IDs locally. They MUST be globally collision-resistant and retained in the durable command record.

The database MUST enforce uniqueness on the authoritative identity.

---

# 6. Monetary and Numeric Representation

## 6.1 Money

The database MUST NOT use floating-point types for financial authority.

Preferred representation:

```text
amount_minor BIGINT
currency_code CHAR(3) / constrained text
```

or a carefully specified `NUMERIC` strategy where fractional precision or jurisdiction-specific requirements demand it.

The domain type remains something like:

```rust
struct Money {
    amount_minor: i64,
    currency: CurrencyCode,
}
```

The exact representation MUST be one authoritative decision across domain, database and API layers.

## 6.2 Why floating point is prohibited

Binary floating point creates values whose decimal business meaning can be surprising:

```text
0.1 + 0.2 != exact decimal 0.3
```

That is unacceptable for:

- sale totals;
- discounts;
- tax calculations;
- payment allocations;
- reconciliation;
- cash totals.

## 6.3 Quantity

Quantity representation depends on product semantics.

A generic inventory quantity may require decimal precision because merchants can sell by:

- kilogram;
- litre;
- metre;
- packet;
- bottle;
- piece.

The schema MUST avoid silently converting decimal quantities to integer values merely because some retail products are counted as integers.

## 6.4 Percentage

Percentages must have explicit scale semantics.

Bad:

```text
10
```

Does it mean:

```text
10%
```

or:

```text
1000%
```

The domain representation should eliminate that ambiguity.

---

# 7. Timestamp Strategy

## 7.1 Required timestamp classes

Sitolo frequently needs to distinguish:

```text
created_at
updated_at
server_observed_at
committed_at
client_created_at
client_observed_at
occurred_at
expires_at
revoked_at
processed_at
```

These are not interchangeable.

## 7.2 Server authority

Server-side timestamps remain authoritative for:

- security policy;
- session expiry;
- authorization windows;
- audit commit ordering;
- server transaction ordering.

Client timestamps are retained where offline forensic reconstruction requires them, but cannot override server authority.

This is explicitly consistent with ADR-016. fileciteturn7file5L904-L938

## 7.3 Database type

Use PostgreSQL timestamp types with timezone semantics appropriate to the application contract, normally `timestamptz` for instant-in-time business events.

Do not store timezone-dependent wall-clock assumptions in fields whose meaning is an instant.

---

# 8. Core Tenant Hierarchy Schema

The canonical organizational relationship is:

```text
Organization
   |
   +--> Business Entity
   |
   +--> Branch
          |
          +--> Location
          +--> Warehouse
          +--> Register
          +--> Device
   |
   +--> Memberships
```

## 8.1 Organizations

An organization represents the primary tenant boundary.

Minimum conceptual fields:

```text
id
status
legal_name
trading_name
default_currency
country_code
timezone
created_at
updated_at
security_version
```

Critical constraints:

- `id` primary key;
- valid status vocabulary;
- valid country code;
- valid default currency;
- required timestamps;
- unique business identifiers where applicable.

## 8.2 Branches

Every branch MUST reference an organization.

```text
branch.organization_id -> organizations.id
```

Where a branch identifier is used as an authorization boundary, indexes MUST support:

```text
(organization_id, id)
```

and common scoped queries.

## 8.3 Branch cross-scope integrity

Do not allow:

```text
organization A branch -> parent organization B
```

through inconsistent foreign keys.

A child table that references both organization and branch should use a composite relationship where appropriate to make mismatched combinations impossible or difficult to create.

Example conceptual pattern:

```sql
FOREIGN KEY (organization_id, branch_id)
REFERENCES branches (organization_id, id)
```

This is stronger than independently validating two IDs in Rust.

---

# 9. Membership, Role and Permission Tables

The identity/authz contract establishes:

```text
User
  -> Membership
  -> Organization
  -> Role
  -> Scope
```

The database must model those relationships explicitly.

## 9.1 Memberships

Conceptual columns:

```text
id
user_id
organization_id
status
role_assignment_reference
scope_reference
effective_at
expires_at
security_version
created_at
updated_at
```

Constraints:

- user must exist;
- organization must exist;
- valid state enum;
- expiration cannot precede effectiveness;
- active membership uniqueness must be explicit where required.

## 9.2 Role assignments

Do not encode authorization as a single mutable role string if the product requires:

- multiple roles;
- scoped roles;
- temporal role changes;
- delegated permissions.

The relational model must be capable of representing those concepts without forcing all authority into one column.

## 9.3 Permission vocabulary

The application maintains the semantic permission registry.

Examples already present in the architecture include:

```text
SALE_CREATE
SALE_VIEW
SALE_VOID
REFUND_CREATE
REFUND_APPROVE
INVENTORY_ADJUST
INVENTORY_TRANSFER
USER_INVITE
ROLE_ASSIGN
EXPORT_CREATE
PAYMENT_RECONCILE
EIS_SUBMIT
```

The database may store stable permission identifiers, but permission evaluation remains a Rust/application concern.

---

# 10. Device Persistence

Devices are independent security subjects.

Conceptual fields:

```text
device_id
organization_id
branch_id?
installation_id
platform
app_version
os_version
status
security_version
registered_at
last_seen_at
revoked_at?
```

Constraints MUST prevent a revoked device from being silently recreated through an update of the same device record.

Replacement means a new device identity.

This follows the accepted device-identity ADR: device compromise must be independently containable from user identity. fileciteturn6file2L190-L243

---

# 11. Catalogue Schema

## 11.1 Product

A product is the merchant-facing business concept.

## 11.2 SKU

A SKU represents a sellable stock identity.

The database should distinguish:

```text
Product
  |
  +--> SKU
        |
        +--> barcode(s)
        +--> pricing
        +--> inventory identity
```

Do not collapse product and SKU into a single table if that creates ambiguity around:

- pack size;
- unit of measure;
- stock identity;
- variant attributes;
- barcode identity.

## 11.3 Catalogue constraints

Fundamental constraints include:

- SKU identity uniqueness within organization policy;
- barcode uniqueness at the correct scope;
- positive/zero rules for sellability;
- valid product status transitions;
- valid unit of measure.

PostgreSQL constraints are appropriate for these local integrity rules. PostgreSQL's constraint system includes `NOT NULL`, `UNIQUE`, `PRIMARY KEY`, `FOREIGN KEY`, `CHECK` and exclusion constraints. citeturn794774search1

---

# 12. Pricing Schema

Price changes must preserve historical meaning.

A finalized sale should reference the effective price information needed to reconstruct the transaction rather than dynamically rereading the current catalogue price.

Possible structures:

```text
price_list
price_list_item
price_version
```

The database must enforce non-overlapping effective periods where temporal price validity is used.

PostgreSQL 18's expanded constraint capabilities can support stronger temporal uniqueness/non-overlap patterns, but Sitolo must not adopt a sophisticated temporal constraint merely because the feature exists; it must be justified by the actual pricing model. citeturn794774search6

---

# 13. Procurement Schema

Core concepts:

```text
Supplier
PurchaseOrder
PurchaseOrderLine
GoodsReceipt
GoodsReceiptLine
SupplierDocument
SupplierCredit
```

Key invariants:

- a receipt belongs to a valid purchase workflow where required;
- received quantity cannot exceed allowed quantity unless explicit over-receipt policy exists;
- supplier credits must be linked to economic evidence;
- final receipt history is not silently rewritten.

The database should enforce local arithmetic constraints but leave cross-document economic decisions to Rust transactions.

---

# 14. Inventory Ledger Schema

Inventory is ledger-backed, not a mutable “magic quantity” field.

The conceptual model is:

```text
Inventory Account / Stock Position
          |
          +--> opening balance
          +--> receipt
          +--> sale
          +--> return
          +--> transfer out
          +--> transfer in
          +--> adjustment
          +--> spoilage/write-off
          +--> correction
```

## 14.1 Ledger row

A movement should contain enough information to reconstruct:

```text
what
where
why
when
how much
unit
source operation
actor
command/idempotency reference
```

## 14.2 Append orientation

Inventory history MUST NOT depend solely on mutable current balances.

A derived balance table can exist for performance, but it must be rebuildable from authoritative movements where the architecture requires reconstruction.

## 14.3 Oversell prevention

The database transaction must prevent this race:

```text
stock = 1

transaction A reads 1
transaction B reads 1
A sells 1
B sells 1

WRONG RESULT:
stock = -1 or two accepted sales
```

The chosen database pattern may be:

```sql
UPDATE inventory_balance
SET quantity = quantity - :qty
WHERE organization_id = :org
  AND location_id = :location
  AND sku_id = :sku
  AND quantity >= :qty;
```

followed by checking affected rows.

Or it may use row locks plus validation.

The choice is per aggregate and must be benchmarked.

The database phase MUST NOT globally force `SERIALIZABLE` simply because concurrency is important. The existing design prefers `READ COMMITTED` with targeted locking/conditional updates and stronger isolation only where justified. fileciteturn7file4L656-L731

---

# 15. Sales Schema

Core structures:

```text
sale
sale_item
sale_payment_allocation
sale_tax_snapshot
sale_state_transition
```

A sale must preserve the data required to reconstruct the economic fact at commitment time.

## 15.1 Sale identifiers

A sale must have:

- immutable internal ID;
- business-facing number where needed;
- organization/branch scope;
- origin device where applicable;
- command/idempotency reference;
- commit timestamp;
- state.

## 15.2 Monetary snapshots

Sale items should preserve transaction-time values for:

- quantity;
- unit price;
- discount;
- tax basis/rate where applicable;
- line total.

Do not recalculate historical sale totals from today's product catalogue.

## 15.3 Finalization transaction

The authoritative transaction resembles:

```text
BEGIN
  resolve authorized tenant/branch
  re-read authoritative product/price state
  validate sale state
  validate inventory
  insert sale
  insert sale items
  insert inventory movements
  update inventory projection
  create payment intent/linkage if required
  append audit evidence
  append outbox event
  record idempotency result
COMMIT
```

The existing database contract explicitly defines this atomicity model. fileciteturn7file4L656-L683

External payment/tax calls do not occur inside this transaction.

---

# 16. Cash Schema

Cash is financially sensitive.

Structures include:

```text
register
register_session
cash_movement
cash_count
cash_close
cash_variance
```

## 16.1 Register session

A register should have a lifecycle such as:

```text
CLOSED
  -> OPENING
  -> OPEN
  -> CLOSING
  -> CLOSED
```

The database should prevent multiple active sessions on the same register where business policy permits only one.

A partial unique index or exclusion constraint MAY be used if it clearly expresses the invariant.

## 16.2 Cash close race

Concurrent close requests must not both finalize the same register session.

Use one of:

- row locking;
- compare-and-set state transition;
- unique transition record.

The transaction must include the final authoritative cash state and associated audit evidence atomically.

---

# 17. Payments and Reconciliation Schema

The payment model deliberately separates:

```text
merchant sale fact
payment intent
provider attempt
provider evidence
settlement state
reconciliation case
```

A payment provider is external and potentially inconsistent.

## 17.1 Payment intent

The payment intent represents Sitolo's internal economic requirement.

It should not inherit provider-specific fields as its core meaning.

## 17.2 Provider attempt

Provider-specific attempts should include:

```text
provider
provider_reference
request correlation
attempt number
status
submitted_at
completed_at
provider payload hash/reference
```

Raw provider payload storage MUST be governed by data classification.

## 17.3 Webhook deduplication

Webhook identity must have a database uniqueness constraint where the provider exposes a stable event ID.

If no stable event ID exists, derive a deterministic fingerprint from the provider contract and preserve enough evidence to prevent unsafe replay handling.

---

# 18. Tax / EIS Persistence

MRA state is external regulatory state, not Sitolo's local financial authority.

The schema should therefore separate:

```text
sale
sale_tax_state
EIS terminal state
EIS configuration snapshot
EIS submission
EIS response evidence
EIS correction/exception
```

A successful local sale can legitimately coexist with:

```text
EIS_PENDING
EIS_RETRY
EIS_REJECTED
```

The local sale MUST NOT be overwritten simply to make external tax state look consistent.

The existing EIS contract explicitly establishes this distinction. fileciteturn2file2L1041-L1059

---

# 19. Audit Schema

Audit evidence is not ordinary application logging.

Conceptual fields:

```text
audit_id
organization_id?
actor_subject_id
actor_type
effective_subject_id?
session_id?
device_id?
action
resource_type
resource_id
scope
policy_version
result
reason_code
occurred_at
committed_at
correlation identifiers
```

Audit records should be append-oriented.

Deletion or rewriting of audit history must be exceptional and tightly controlled.

The observability specification explicitly distinguishes audit evidence from runtime telemetry and requires audit to live in an authoritative/durable evidence path rather than solely in logs. fileciteturn4file2L2293-L2305

---

# 20. Idempotency Schema

An idempotency record is a correctness primitive.

Conceptual fields:

```text
id
scope_type
scope_id
principal_id
endpoint_or_operation
idempotency_key
request_hash
state
response_status
response_body_hash / safe response reference
resource_reference
created_at
expires_at
```

## 20.1 Uniqueness

A unique constraint must encode the semantic scope.

Bad:

```text
UNIQUE(idempotency_key)
```

if different tenants or operations can legitimately reuse the same client-generated key.

Better conceptual key:

```text
(organization_id, principal/device scope, operation_type, idempotency_key)
```

The exact dimensions must follow the API contract.

## 20.2 Mismatch

If the same key appears with a different request hash:

```text
IDEMPOTENCY_KEY_REUSED_WITH_DIFFERENT_REQUEST
```

The database must preserve the first authoritative result and prevent silent substitution.

---

# 21. Synchronization Persistence

The offline protocol depends on durable server-side state.

Core records include:

```text
sync_device
sync_command
sync_batch
sync_ack
sync_checkpoint
sync_conflict
```

## 21.1 Command identity

A command must be uniquely identified by a stable client-generated command ID.

This is enforced by a database unique constraint.

## 21.2 Device binding

The command must belong to a known device identity.

A user ID alone is insufficient because device revocation is a separate security boundary.

## 21.3 Checkpoints

Checkpoint advancement must be monotonic.

The database and application logic must reject:

```text
checkpoint 100
then
checkpoint 90
```

unless the protocol defines a valid rollback/reconciliation operation.

## 21.4 Offline command evidence

Rejected commands must remain explainable and auditable according to the sync contract.

Deleting failed commands just because they cannot be applied would destroy forensic and support evidence.

---

# 22. Outbox Schema

The transactional outbox is required so business state and side-effect intent commit atomically.

Conceptual fields:

```text
outbox_id
organization_id?
event_type
aggregate_type
aggregate_id
payload
payload_version
created_at
available_at
attempt_count
locked_at
locked_by
processed_at
failed_at
last_error_class
```

Transaction rule:

```text
BUSINESS STATE
+
OUTBOX EVENT
=
ONE COMMIT
```

The worker then executes external side effects asynchronously.

This prevents:

```text
DB COMMIT
but side-effect intent disappears
```

The current security implementation and database contracts both explicitly require this pattern. fileciteturn0file13L1202-L1245

---

# 23. Background Jobs

Jobs should be durable where loss of the job would create a business correctness or operational problem.

Minimum conceptual state:

```text
QUEUED
RUNNING
SUCCEEDED
FAILED_RETRYABLE
FAILED_PERMANENT
DEAD_LETTERED
CANCELLED
```

The job record must support:

- lease ownership;
- attempt count;
- next-available timestamp;
- bounded retry;
- error classification;
- correlation ID.

Workers must never infer tenant identity from process-global mutable state. Existing governance explicitly requires trusted tenant context in asynchronous work. fileciteturn7file3L499-L511

---

# 24. Constraint Taxonomy

Not every invariant belongs in a `CHECK` constraint.

Use the following decision matrix:

| Invariant class | Preferred enforcement |
|---|---|
| Row must not be null | `NOT NULL` |
| Local arithmetic/range | `CHECK` |
| Entity identity | `PRIMARY KEY` |
| Uniqueness | `UNIQUE` / unique index |
| Parent existence | `FOREIGN KEY` |
| Scoped parent relationship | composite FK |
| Non-overlap | exclusion/temporal strategy where justified |
| Cross-row race | transaction + lock/atomic update |
| Authorization scope | application AuthZ + RLS |
| External consistency | reconciliation state |
| Complex domain rule | Rust domain + transaction |
| Immutable history | privilege design + append-only model + application commands |

PostgreSQL documentation specifically cautions that `CHECK` constraints are intended for row-local conditions and should not be treated as a general cross-row integrity engine. citeturn794774search1

---

# 25. NOT NULL Policy

The default should be `NOT NULL` unless `NULL` has a real semantic meaning.

Bad database design:

```text
status NULL
currency NULL
organization_id NULL
```

when those values are mandatory for the entity to have meaning.

Null must not become the database equivalent of “developer did not know.”

Where absence is meaningful, document it explicitly:

```text
revoked_at NULL = not revoked
expires_at NULL = no expiration
```

The domain layer must not reinterpret null semantics inconsistently.

---

# 26. CHECK Constraint Policy

Use `CHECK` for cheap immutable row-local rules.

Examples:

```sql
CHECK (quantity >= 0)
CHECK (amount_minor >= 0)
CHECK (expires_at IS NULL OR expires_at > created_at)
CHECK (status IN (...))
```

Do not write a `CHECK` that queries another table or depends on mutable external state.

PostgreSQL assumes `CHECK` conditions are effectively immutable and validates them at row write time; changing dependent function behavior can create historical rows that no longer satisfy the semantic assumption. citeturn794774search1

---

# 27. UNIQUE Constraints and Replay Resistance

Unique constraints are one of Sitolo's most important concurrency primitives.

They are used for:

- idempotency keys;
- webhook event identity;
- command IDs;
- business numbers;
- one active register session where applicable;
- one active invitation token hash;
- one organization-scoped SKU identity;
- provider reference uniqueness where contractually safe.

A uniqueness constraint turns:

```text
check then insert
```

into an atomic database property.

Without the constraint, two concurrent writers can both pass the application check.

---

# 28. Foreign Key Policy

Use foreign keys to preserve fundamental parent-child integrity.

Actions such as `CASCADE` must be chosen carefully.

For financial/history records, cascading deletion is usually dangerous.

Prefer:

```text
RESTRICT / NO ACTION
```

for historical entities unless there is a clearly safe ownership relationship.

A merchant deleting a supplier must not automatically delete:

- purchase history;
- receipts;
- inventory movements;
- audit evidence.

Use lifecycle status rather than destructive deletion when history matters.

---

# 29. Composite Foreign Keys for Tenant Safety

Where a child belongs to a tenant-owned parent, independent foreign keys are often insufficient.

Example of an unsafe conceptual model:

```text
child.organization_id -> organizations.id
child.branch_id -> branches.id
```

This can still permit application-level construction of a mismatched pair if no composite relationship exists.

Prefer:

```text
branches:
PRIMARY KEY (id)
UNIQUE (organization_id, id)

child:
FOREIGN KEY (organization_id, branch_id)
REFERENCES branches (organization_id, id)
```

This provides database-level evidence that the branch belongs to the same tenant claimed by the child.

---

# 30. Indexing Strategy

Indexes are not decorations. Every index has:

```text
read benefit
+
write cost
+
storage cost
+
cache pressure
+
vacuum/maintenance cost
```

## 30.1 Mandatory index reasoning

For every high-volume table, document:

- primary access paths;
- tenant filtering;
- branch filtering;
- time-range queries;
- state queries;
- join columns;
- uniqueness requirements;
- sort order;
- expected selectivity.

## 30.2 Tenant-leading indexes

For tenant-owned tables, common indexes should usually begin with tenant scope where that reflects actual query patterns:

```text
(organization_id, created_at)
(organization_id, status)
(organization_id, branch_id, created_at)
```

Do not automatically prefix every index with `organization_id`. The index must correspond to real workload access patterns.

## 30.3 POS critical path

At minimum benchmark:

```text
sale lookup
SKU/barcode lookup
inventory availability
register session lookup
payment intent lookup
sync command insertion
```

## 30.4 Reporting isolation

Large reporting queries need:

- bounded time ranges;
- indexes appropriate to reporting dimensions;
- read models where needed;
- dedicated connections/pools where justified.

Never let an expensive export consume all database connections needed by POS.

---

# 31. Row-Level Security Architecture

RLS is required where it materially strengthens tenant isolation.

It is **not** a substitute for application authorization.

The intended structure is:

```text
application authorization
       |
       v
trusted tenant context
       |
       v
PostgreSQL session context
       |
       v
RLS USING / WITH CHECK
       |
       v
tenant-scoped rows
```

## 31.1 Enable RLS deliberately

For every protected table:

```sql
ALTER TABLE ... ENABLE ROW LEVEL SECURITY;
```

For especially sensitive tables, force row security semantics where architecture requires it and where role behavior is fully understood.

PostgreSQL requires RLS to be enabled for policies to apply and documents default-deny behavior when no applicable policy permits access. citeturn794774search0turn794774search9

## 31.2 `USING` vs `WITH CHECK`

This distinction is mandatory knowledge for the implementation team.

`USING` controls which existing rows can be read/updated/deleted.

`WITH CHECK` controls which new row values may be introduced by insert/update.

A policy like:

```text
USING organization_id = current_org()
```

without a compatible `WITH CHECK` is incomplete for a write-sensitive tenant table.

The goal is:

```text
READ A → only A
UPDATE A → only A
INSERT A → only A
MOVE A → cannot silently become B
```

## 31.3 Default deny

If the runtime role has RLS enabled and no applicable policy permits access, failure must be denial rather than silent unrestricted access.

That is desirable.

## 31.4 Role model

At minimum distinguish:

```text
sitolo_migrator
sitolo_runtime
sitolo_readonly_reporting
sitolo_worker
sitolo_support
sitolo_breakglass
```

Exact names can differ; the privilege separation cannot.

---

# 32. Database Role and Privilege Model

## 32.1 Migration role

The migration role may perform DDL required by controlled migrations.

It MUST NOT be the default application runtime role.

## 32.2 Runtime role

The normal application role receives only the privileges required to execute business operations.

It should not casually have:

```text
SUPERUSER
BYPASSRLS
CREATEROLE
CREATEDB
CREATE privileges across arbitrary schemas
```

The existing CI contract explicitly requires tests to detect accidental `SUPERUSER`, `BYPASSRLS`, unrestricted schema ownership, DDL access and role administration. fileciteturn7file1L265-L314

## 32.3 Worker roles

Workers should have separate credentials where the blast radius differs.

An EIS worker should not automatically gain user-administration authority.

An export worker should not automatically gain mutation privileges over inventory.

## 32.4 Reporting role

Reporting access should be read-only and preferably directed at read models/views rather than direct write-capable operational tables.

---

# 33. RLS Tenant Context

A trusted application transaction must establish the tenant context explicitly.

Conceptual design:

```text
BEGIN
  establish trusted tenant context
  execute operation
COMMIT
```

A session setting must NOT accept arbitrary tenant IDs directly from an untrusted client.

The Rust application must first derive the effective tenant from:

```text
principal
+
membership
+
scope
+
requested resource
```

and only then set the transaction-local database context.

## 33.1 Transaction-local context

Prefer transaction-local semantics where possible so a pooled PostgreSQL connection cannot accidentally retain one tenant's context for the next request.

This matters because SQL connection pools reuse physical connections across unrelated requests.

A tenant context that leaks across pooled requests is a catastrophic security defect.

## 33.2 Context cleanup

Even with transaction-local settings, the application MUST test that:

```text
request A tenant A
request B tenant B
```

cannot observe A's context through connection reuse.

---

# 34. RLS Policy Design Pattern

A standard policy pattern may conceptually look like:

```sql
CREATE POLICY sale_tenant_select
ON sitolo_sales.sale
FOR SELECT
TO sitolo_runtime
USING (
    organization_id = current_setting('sitolo.organization_id', true)::uuid
);
```

For writes, the policy should include a corresponding `WITH CHECK` expression.

This is only a conceptual pattern. The exact implementation must account for:

- absent context;
- type parsing;
- support access;
- service identities;
- branch scope;
- nested ownership;
- performance.

A missing context should evaluate to denial, not to “all rows.”

## 34.1 Fail-closed helper functions

If helper functions are used inside RLS policies, they must be designed and secured carefully.

PostgreSQL documents that security-sensitive functions and policies themselves can become attack surfaces, and object ownership/search-path control matters. citeturn794774search3

Functions used by RLS must:

- have trusted ownership;
- use safe `search_path` behavior;
- avoid dynamic SQL from untrusted inputs;
- avoid leaking protected data via side channels;
- be covered by integration tests.

---

# 35. RLS and Support Access

Support access is a different trust context from merchant access.

The database must not implement support as:

```text
support = bypass everything
```

Instead, support access should be represented as a controlled capability:

```text
ticket
+
approved tenant
+
approved action
+
expiry
+
actor identity
```

The application creates the trusted support context.

RLS should still prevent accidental access outside the approved tenant.

Break-glass access may be separately privileged, but it remains:

- explicit;
- time-bounded;
- audited;
- exceptional.

---

# 36. RLS and Async Workers

A worker MUST NOT depend on a request-local HTTP context that no longer exists.

Each job must explicitly carry sufficient trusted scope metadata.

Conceptual:

```text
outbox/job
  |
  +--> organization_id
  +--> operation
  +--> object reference
  v
worker identity
  |
  +--> validate authorized job class
  +--> establish scoped DB context
  +--> execute
```

Worker scope must be verified from authoritative job data.

Never use:

```text
GLOBAL_CURRENT_TENANT
```

in process memory.

---

# 37. Views, Functions and Stored Procedures

Sitolo is not a stored-procedure-first architecture.

Use PostgreSQL functions/triggers only where they create a clear correctness or integrity benefit that is difficult to achieve safely in the application.

Good candidates:

- narrow RLS helper functions;
- integrity-supporting triggers where unavoidable;
- generated data mechanisms;
- database-side immutable utility functions.

Bad candidates:

- entire POS workflow hidden inside opaque stored procedures;
- dynamic permission engines embedded in SQL;
- arbitrary JSON command dispatch in the database;
- external API calls from database code.

Business domain behavior remains in Rust.

---

# 38. Trigger Policy

Triggers must be rare and documented.

A trigger creates implicit behavior:

```text
INSERT
  -> hidden trigger
      -> writes another table
          -> trigger
              -> side effect
```

That can make transaction behavior difficult to reason about.

Triggers may be appropriate for:

- immutable audit capture of extremely low-level changes where the application cannot safely control all writes;
- technical timestamp maintenance;
- strict database invariants.

They are not a license to create a hidden second application.

---

# 39. Migration System

## 39.1 Migration source

All schema changes MUST be represented as version-controlled migration files.

A clean database must be buildable from empty state by applying migrations in order.

## 39.2 Determinism

The same migration sequence against the same starting database should produce equivalent schema state.

Do not rely on a developer manually “fixing” production schema drift.

## 39.3 Migration names

Use deterministic names such as:

```text
202609060001_create_organizations.sql
202609060002_create_branches.sql
202609060003_create_memberships.sql
```

or the established repository convention.

Timestamps are identifiers, not proof of execution order; the migration framework remains authoritative.

## 39.4 One logical change per migration

Avoid 4,000-line migrations mixing:

- unrelated domains;
- security changes;
- data correction;
- index rebuilds;
- destructive operations.

Prefer small reviewable units unless atomicity requires a combined migration.

---

# 40. Migration Categories

Every migration should be classified internally as one of:

```text
EXPANSIVE
```

Adds structures without requiring old application removal.

```text
COMPATIBLE
```

Supports old and new application versions during rollout.

```text
CONTRACT-CHANGING
```

Changes semantics or constraints in ways that require coordinated application deployment.

```text
DESTRUCTIVE
```

Drops/renames/removes data or capabilities.

```text
DATA-CORRECTIVE
```

Changes existing data to repair or transform it.

The classification affects deployment strategy.

---

# 41. Expand-and-Contract Migration Pattern

For rolling deployments, prefer:

```text
1. EXPAND
2. DEPLOY COMPATIBLE APPLICATION
3. DUAL READ/WRITE IF REQUIRED
4. BACKFILL
5. VERIFY
6. SWITCH
7. REMOVE OLD STRUCTURE
```

Example column rename:

Do not immediately:

```text
ALTER TABLE ... RENAME COLUMN old TO new;
```

if old application versions still write `old`.

Prefer:

```text
add new column
-> application writes both
-> backfill
-> verify
-> application reads new
-> stop old writes
-> remove old later
```

The exact strategy depends on deployment architecture and load.

---

# 42. Large Table Migration Safety

Large production tables require special analysis.

Risk areas include:

- long locks;
- table rewrites;
- transaction log growth;
- replication lag;
- vacuum pressure;
- IO saturation;
- connection pool starvation.

Before migration:

```text
estimate rows
estimate index size
estimate lock duration
estimate write amplification
estimate replication impact
estimate rollback/recovery path
```

Never assume an operation that is fast on a 10,000-row development database will be fast on a 100-million-row production table.

---

# 43. Concurrent Index Operations

For large production structures, `CREATE INDEX CONCURRENTLY` or equivalent safe patterns may be appropriate.

But concurrent index operations have their own operational semantics and failure/retry behavior.

Do not blindly wrap every migration in one transaction if the PostgreSQL command explicitly has transaction restrictions.

The migration runner must support a migration being intentionally non-transactional where the database operation requires it, with a clear recovery procedure.

---

# 44. Constraint Rollout Strategy

Adding a constraint to existing dirty data can fail deployment.

Before adding:

```sql
ALTER TABLE ... ADD CONSTRAINT ...
```

perform a diagnostic query for violating rows.

Safe process:

```text
find violations
-> classify
-> remediate data
-> verify zero violations
-> add/enforce constraint
```

For very large datasets, PostgreSQL's ability to add constraints without immediately enforcing them can be part of a staged process where justified. PostgreSQL 18 supports `NOT ENFORCED` for certain `CHECK` and foreign-key constraints, but this capability must never be mistaken for a security control because an unenforced constraint is not a correctness boundary. citeturn794774search6

---

# 45. RLS Migration Strategy

Changing an RLS policy is a security-sensitive migration.

Every RLS migration must specify:

```text
before policy
after policy
read impact
write impact
support/admin impact
worker impact
performance impact
rollback path
test evidence
```

Never deploy a permissive temporary policy to make a migration easier unless the exception is explicitly isolated and controlled.

A policy accidentally changed from:

```text
organization A only
```

to:

```text
all organizations
```

is a release-blocking security defect.

---

# 46. Schema Drift Detection

CI and deployment tooling must compare expected schema state with actual schema state where practical.

Drift includes:

- manually-created tables;
- missing indexes;
- missing policies;
- unexpected grants;
- unexpected extensions;
- modified function definitions;
- schema objects not represented by migrations.

Production drift must be treated as an incident or controlled exception, not as something engineers silently “remember.”

---

# 47. SQLx Integration Contract

SQLx remains the PostgreSQL access layer.

The repository must use:

- explicit SQL;
- bound parameters;
- compile-time checking where supported by repository workflow;
- versioned migrations;
- explicit transaction objects;
- bounded query results.

SQLx compile-time checking is valuable because schema/type mismatches can be caught before runtime under the supported build workflow. The existing Sitolo architecture already adopts SQLx for this reason. fileciteturn7file2L472-L486

## 47.1 Query ownership

Do not put raw SQL in HTTP handlers.

Use:

```text
handler
  -> application service
     -> repository port
        -> SQLx repository
```

This preserves separation between transport, business orchestration and persistence.

---

# 48. Transaction API in Rust

Repositories should expose operations that correspond to business transaction boundaries rather than arbitrary table mutations.

Preferred conceptual interface:

```text
SaleRepository::finalize(tx, command)
```

rather than:

```text
SaleRepository::update_total(...)
SaleRepository::set_status(...)
InventoryRepository::decrement(...)
```

when those methods allow callers to bypass aggregate invariants.

The transaction object should be explicitly owned by the application operation.

---

# 49. Transaction Isolation Policy

Default:

```text
READ COMMITTED
```

where adequate.

Use stronger isolation selectively.

Potential tools:

```text
SELECT ... FOR UPDATE
atomic conditional update
unique constraint
advisory lock only where strongly justified
SERIALIZABLE for specific correctness problems
```

The project must document the choice per high-risk workflow.

Do not globally set `SERIALIZABLE` to compensate for bad domain logic.

That approach can increase serialization failures, retries, latency and throughput loss without fixing incorrectly designed transactions.

---

# 50. Lock Ordering

Whenever a transaction acquires multiple locks, the order must be deterministic.

Example inventory transfer order:

```text
organization
  -> source location
  -> destination location
  -> SKU
  -> lot/batch
```

The real order must match the schema/query access path and be documented.

Deadlock handling must classify the error before retry.

Never:

```text
catch all SQL errors
-> retry forever
```

The existing database design requires bounded retry only when the operation is safe to retry. fileciteturn7file4L757-L771

---

# 51. Constraint Naming Standard

Constraints need stable names because:

- tests inspect them;
- error mappings may refer to them;
- migrations need to alter/drop them safely;
- operations need readable diagnostics.

Recommended naming:

```text
pk_<table>
fk_<table>_<parent>
uk_<table>_<columns>
ck_<table>_<rule>
ex_<table>_<rule>
```

Example:

```text
uk_sale_organization_business_number
ck_inventory_movement_quantity_nonzero
fk_sale_branch
```

Constraint names are part of the database engineering surface, not arbitrary decoration.

---

# 52. Deletion Policy

Default stance:

```text
DELETE is rare
ARCHIVE/DEACTIVATE is common
COMPENSATE is required for financial correction
```

Do not delete:

- finalized sales;
- payment history;
- inventory movements;
- audit evidence;
- tax submission evidence;
- sync commands required for reconciliation.

For merchant master data, deletion may be allowed where there is no historical dependency or where records can be anonymized/deactivated safely.

The exact retention requirements remain domain/regulatory decisions.

---

# 53. Partitioning Policy

Partitioning must be evidence-driven.

Candidate high-volume tables:

```text
sales
sale_items
inventory_movements
audit_events
outbox_messages
sync_commands
provider_events
```

Partition by time only when:

- volume justifies it;
- access patterns are naturally time-bounded;
- retention benefits materially;
- operational tooling supports it.

Partitioning adds:

- migration complexity;
- routing considerations;
- index duplication/management;
- operational complexity.

It is not automatically “more enterprise.”

The existing architecture explicitly rejects infrastructure complexity without measured need.

---

# 54. Connection Pooling

The Rust service must maintain bounded PostgreSQL connection pools.

Pool parameters must consider:

```text
available DB connections
x
application replicas
x
worker pools
x
reporting consumers
```

A common production failure is each application process independently requesting a “reasonable” pool that collectively exceeds PostgreSQL capacity.

The aggregate connection budget must be calculated.

Example reasoning:

```text
PostgreSQL max safe connections = 200
API replicas = 4
API pool per replica = 25
worker replicas = 2
worker pool = 15
reporting pool = 10

aggregate requested = 4*25 + 2*15 + 10 = 140
```

This is only an arithmetic example. The actual capacity must be measured.

---

# 55. Timeout Policy at Database Layer

Every database operation must have a bounded execution context.

Relevant controls include:

- connection acquisition timeout;
- statement timeout;
- lock timeout;
- transaction timeout at application layer.

Long-running reporting should not silently block critical operational transactions.

The architecture already treats missing timeouts as a security and reliability defect. fileciteturn3file1L203-L224

---

# 56. Database Resource Classes

Not all queries deserve equal priority.

Sitolo should conceptually distinguish:

```text
CLASS A — POS / transactional
CLASS B — inventory / synchronization
CLASS C — integrations / reconciliation
CLASS D — reporting / exports
CLASS E — maintenance
```

The database architecture should preserve enough resource isolation that a Class D workload cannot starve Class A.

This can be achieved through:

- bounded pools;
- separate roles/connections;
- query limits;
- async jobs;
- read models;
- workload scheduling.

Do not solve every workload conflict by adding hardware before understanding query behavior.

---

# 57. Backup and Restore Implications

Schema correctness is meaningless if the database cannot be recovered.

Backup verification must include:

```text
backup exists
backup is recent
backup is restorable
schema is compatible
roles can be recreated
RLS policies survive
extensions survive
critical indexes can be rebuilt
application can reconnect
```

A successful backup job is not equivalent to a successful restore.

The existing testing/deployment contracts explicitly require restore drills. fileciteturn7file0L31-L41

---

# 58. RLS Testing Requirements

RLS tests MUST use real PostgreSQL.

Mocks cannot prove:

- policy evaluation;
- owner/bypass behavior;
- grants;
- `USING` semantics;
- `WITH CHECK` semantics;
- interaction with views/functions;
- actual query plans.

The existing CI contract explicitly states that mocks cannot prove RLS, privileges, locking, isolation, constraints or actual query plans. fileciteturn7file1L265-L304

## 58.1 Minimum matrix

```text
Tenant A -> Tenant A resource -> ALLOW
Tenant A -> Tenant B resource -> DENY
Branch A1 -> Branch A1 resource -> ALLOW
Branch A1 -> Branch A2 resource -> DENY when not scoped
Support A -> approved support scope -> ALLOW
Support A -> unrelated tenant -> DENY
Revoked device -> sync mutation -> DENY
Missing tenant context -> protected table -> DENY
```

## 58.2 Write-side tests

For every protected table:

```text
INSERT correct tenant -> allow
INSERT wrong tenant -> deny
UPDATE correct tenant -> allow if authorized
UPDATE into wrong tenant -> deny
DELETE historical protected record -> deny
```

## 58.3 Pool-reuse test

A critical regression test must perform:

```text
connection A
request tenant A
commit

same pooled connection
request tenant B
```

and verify B cannot see A.

---

# 59. Privilege Regression Tests

CI MUST assert that the runtime role cannot acquire accidental high privilege.

At minimum verify:

```text
rolsuper = false
rolbypassrls = false
rolcreaterole = false
rolcreatedb = false
```

and validate schema/table privileges explicitly.

This is more reliable than reading a role definition file and assuming deployment applied it correctly.

---

# 60. Migration Test Matrix

Every migration should be exercised against:

### Case A — Empty database

```text
create database
apply all migrations
verify schema
```

### Case B — Existing realistic database

```text
seed representative data
apply migration
verify business invariants
```

### Case C — Old app/new schema

Required where rolling deployment demands compatibility.

### Case D — New app/old-compatible schema

Required where application rollout can overlap schema changes.

### Case E — RLS and privilege regression

Every security-sensitive migration must rerun the isolation suite.

The testing strategy already mandates these patterns for schema evolution. fileciteturn7file6L1037-L1086

---

# 61. Migration Data Backfill Rules

Backfills must be:

- bounded;
- restartable;
- observable;
- idempotent where possible;
- throttled;
- resumable;
- safe for concurrent application traffic.

Bad:

```sql
UPDATE giant_table
SET new_column = expensive_function(old_column);
```

with no batch strategy, timeout planning or progress tracking.

Better conceptually:

```text
find batch
-> process batch
-> commit
-> record progress
-> repeat
```

The exact implementation may be a worker rather than a migration transaction.

---

# 62. Data Repair Policy

Production data correction must never become ad-hoc SQL run from a developer laptop.

A repair must have:

```text
incident/change reference
scope
precondition query
backup/recovery consideration
mutation script
postcondition query
business-owner approval
rollback/recovery plan
audit trail
```

For financial data, the preferred correction remains compensating business events rather than direct row edits.

---

# 63. Error Mapping from PostgreSQL

Rust must map database failures into stable semantic errors.

Examples:

```text
unique violation
  -> IdempotencyConflict / DuplicateResource

foreign key violation
  -> InvalidReference / StateConflict

check violation
  -> InvariantViolation

serialization failure
  -> ConcurrencyConflict

deadlock detected
  -> RetryableConcurrencyFailure

statement timeout
  -> DependencyUnavailable / Timeout

connection failure
  -> PersistenceUnavailable
```

Do not expose raw SQLSTATE/error strings to clients.

The existing domain error ADR requires stable semantic error types rather than raw database/provider errors. fileciteturn6file4L406-L469

---

# 64. Security Logging for Database Failures

Database errors must be observable without leaking:

- SQL text with secrets;
- connection strings;
- credentials;
- sensitive row contents;
- internal network topology.

Log structured metadata such as:

```text
operation
error_family
sqlstate class
retryability
request_id
trace_id
transaction classification
```

Do not put raw SQL into high-cardinality metric labels.

The existing observability contract explicitly prohibits this pattern. fileciteturn4file2L2662-L2694

---

# 65. Query Security Rules

Every query must be evaluated for:

```text
tenant scope
branch scope
resource bounds
index support
lock behavior
retry safety
information disclosure
```

Forbidden patterns include:

```text
SELECT * FROM table WHERE id = $1
```

for a tenant-owned resource when the tenant scope is not independently enforced.

Also forbidden:

```text
ORDER BY ${user_input}
```

or dynamic SQL fragments built from client strings.

Use allowlists and parameter binding.

---

# 66. Search Security

Search endpoints can leak data even when direct object reads are protected.

Example:

```text
GET /products?query=paracetamol
```

must not search globally across tenants.

Likewise, counts such as:

```text
GET /sales/count
```

must not become a side channel for cross-tenant existence.

RLS and tenant predicates must apply to search/read models and aggregation sources.

---

# 67. Export Security

Exports are database-intensive and high-risk.

Database queries behind exports require:

- explicit authorization;
- tenant scope;
- row limits;
- time-range limits;
- async job execution for expensive requests;
- read-only data access;
- audit;
- bounded temporary storage.

Do not allow a user to request:

```text
all historical sales for all tenants
```

through an accidental admin query path.

---

# 68. Reporting Read Models

When operational queries and reports compete, create derived read models where justified.

The read model is:

```text
rebuildable
non-authoritative
optimized for reads
```

It must never become a hidden second ledger.

Example:

```text
PostgreSQL transactional sale facts
             |
             v
       reporting projection
             |
             v
        dashboard/export
```

The source of truth remains the transactional model.

---

# 69. Materialized Views

Materialized views may be used for expensive aggregates where freshness requirements permit.

Rules:

- define refresh strategy;
- measure refresh cost;
- never present stale values as real-time financial authority;
- scope tenant visibility correctly;
- prevent refresh workload from starving POS.

A materialized view is a cache, not business truth.

---

# 70. Database Security for Extensions

Every PostgreSQL extension requires security review.

Before enabling an extension, document:

```text
purpose
attack surface
privileges
backup/restore support
version lifecycle
managed-host support
migration behavior
failure mode
```

Do not install random extensions because a package recommends them.

---

# 71. Search Path Security

Database functions and migrations must control `search_path` carefully.

Do not let security-sensitive functions resolve objects from schemas writable by untrusted actors.

PostgreSQL's function-security guidance specifically warns about unsafe object resolution and emphasizes tight control of who can define backend objects. citeturn794774search3

This becomes important for:

- RLS helper functions;
- triggers;
- security-definer functions;
- migration utilities.

---

# 72. Security-Definer Functions

`SECURITY DEFINER` functions are high risk.

When used, they must have:

- trusted owner;
- fixed safe `search_path`;
- minimal privileges;
- explicit argument validation;
- controlled execution surface;
- test coverage against privilege escalation.

Prefer direct privileges/policies where possible.

Do not expose a generic privileged function such as:

```text
execute_as_admin(sql_text)
```

That is equivalent to deleting the database security boundary.

---

# 73. Audit of Database Security Objects

CI must inspect:

```text
roles
role memberships
table privileges
schema privileges
RLS enabled state
RLS policies
functions
function owners
security-definer functions
extensions
```

PostgreSQL exposes policy metadata through `pg_policies`, which should be incorporated into automated policy tests/inspection where useful. citeturn794774search2

Constraint metadata is similarly inspectable through PostgreSQL catalogs such as `pg_constraint`. citeturn794774search5

---

# 74. Schema Introspection Gate

The database test harness should produce an evidence report containing at least:

```text
schema list
migration version
table list
primary keys
foreign keys
unique constraints
check constraints
indexes
RLS-enabled tables
RLS policy list
runtime role grants
function security attributes
extension versions
```

This turns “schema looks correct” into machine-verifiable evidence.

---

# 75. Performance Verification

Phase 5 performance tests must focus on correctness-critical workloads first.

Minimum benchmark categories:

```text
single sale finalize
concurrent sale finalize
inventory decrement race
sync command insertion
idempotency contention
webhook duplicate contention
membership lookup
branch-scoped catalogue lookup
payment lookup
```

Capture:

```text
p50
p95
p99
throughput
lock wait
connection wait
CPU
I/O
rows scanned
rows returned
```

The objective is not to maximize synthetic queries per second. The objective is to preserve merchant operations under realistic contention.

---

# 76. Explain-Plan Discipline

Critical queries should have reviewed query plans.

Review at least:

```text
POS SKU lookup
inventory availability
sale retrieval
branch-scoped product list
sync command pull/push
payment reconciliation query
audit lookup
reporting extraction
```

Look for:

- sequential scans where indexes should apply;
- row estimate errors;
- unexpected joins;
- explosive nested loops;
- huge sort operations;
- missing tenant predicates;
- query paths that become worse at scale.

Never optimize a query only from intuition when the real plan can be inspected.

---

# 77. Vacuum / Autovacuum Considerations

High-churn tables such as:

- sync queues;
- outbox;
- job leases;
- idempotency records;
- webhook inbox;

may create substantial tuple churn.

Autovacuum behavior must be measured and tuned when needed.

Do not globally disable autovacuum because one table has unusual workload behavior.

Use per-table configuration only after measurement.

---

# 78. Retention and Cleanup

Cleanup jobs must be semantic.

Example:

```text
idempotency record expired
AND no reconciliation dependency
AND retention rule satisfied
-> eligible for deletion
```

But:

```text
unsynced command
-> NEVER cleanup
```

and:

```text
audit / tax evidence
-> retain according to applicable policy
```

Operational cleanup must never silently destroy business or regulatory evidence.

---

# 79. Soft Delete Policy

Soft deletion is not automatically better.

It can create:

- every-query `deleted_at` predicates;
- index complexity;
- accidental exposure of deleted rows;
- uniqueness problems.

Use explicit lifecycle states when the business meaning is active/inactive/retired rather than pretending every entity has a generic delete flag.

---

# 80. Multi-Branch Inventory Integrity

Branch-aware inventory requires explicit scope.

A SKU can exist globally in the catalogue while stock exists at:

```text
branch A / warehouse 1
branch A / warehouse 2
branch B / warehouse 1
```

Do not store one tenant-wide quantity for branch-managed inventory.

Transfers are two-sided economic events:

```text
source decrement
+
destination increment
```

They require an atomic business transaction unless the domain explicitly models an in-transit state.

---

# 81. In-Transit Inventory

If transfers may be asynchronous, do not fake atomicity by immediately increasing destination stock while source removal is not committed.

Instead model:

```text
REQUESTED
-> APPROVED
-> IN_TRANSIT
-> RECEIVED
```

with corresponding ledger events.

That keeps physical and accounting meaning explicit.

---

# 82. Pharmacy / Agro Extension Compatibility

The core database must remain generic enough for future regulated verticals.

Examples of extension concepts:

```text
lot/batch
expiry
serial number
regulated classification
formulation
unit conversion
```

Do not add pharmacy-specific regulatory fields to every generic product because future pharmacy support is planned.

Use extension tables or bounded domain structures where the concept is genuinely vertical.

The existing domain architecture explicitly calls pharmacy and agro-dealer behavior extensions over the generic inventory core. fileciteturn0file8L847-L854

---

# 83. Data Classification at Database Layer

Columns should be classified conceptually as:

```text
PUBLIC
INTERNAL
CONFIDENTIAL
SENSITIVE
SECRET
REGULATED
```

Secrets should normally not be stored in ordinary tables at all.

Examples:

```text
payment provider secret -> secret manager
MRA terminal secret -> isolated cryptographic/integration boundary
password hash -> identity store with strict access
recovery artifact -> protected representation
```

Database persistence of secrets is exceptional and requires an explicit design.

---

# 84. Encryption Considerations

TLS is required for database transport where architecture requires remote connectivity.

At-rest encryption should normally be delegated to the managed storage/database infrastructure rather than implemented by application developers through bespoke field encryption.

Application-level field encryption may be justified for extremely sensitive values, but it creates:

- key lifecycle;
- rotation;
- queryability constraints;
- migration complexity;
- backup implications.

The build-vs-buy principle applies: use mature cryptographic infrastructure rather than inventing storage encryption.

---

# 85. Connection Security

Production PostgreSQL must not be publicly exposed by default.

Connections should use:

```text
private network path
TLS where required
least-privilege credentials
bounded pool
certificate validation where applicable
```

The application must not log full database URLs containing credentials.

---

# 86. Secret Rotation and Database Credentials

Credential rotation must avoid requiring application source changes.

Safe operational pattern:

```text
create replacement credential
-> deploy/configure new credential
-> verify connectivity
-> revoke old credential
-> verify old credential rejected
```

Rotation must consider pooled connections because an existing connection may continue using a credential until reconnect.

---

# 87. Migration Rollback Philosophy

Not every migration should be reversible with a single `down.sql`.

This is especially true for:

- destructive changes;
- data transformations;
- uniqueness changes;
- irreversible external evidence changes.

A migration is safe when the recovery plan is correct, not when a tooling framework can produce a syntactic inverse.

For destructive changes, preferred recovery may be:

```text
restore backup
replay compatible migrations
re-run validated repair
```

rather than pretending a simple reverse migration exists.

---

# 88. Deployment Ordering Contract

For schema/application compatibility:

```text
EXPAND SCHEMA
     ↓
VERIFY MIGRATION
     ↓
DEPLOY COMPATIBLE APPLICATION
     ↓
VERIFY RUNTIME
     ↓
BACKFILL / SWITCH
     ↓
REMOVE OBSOLETE STRUCTURE LATER
```

Never deploy an application that assumes a column exists before the migration creating that column is safely applied.

Likewise, never drop a column while an older application version may still query it.

---

# 89. Production Migration Approval

Production migration PRs must include:

```text
business reason
schema impact
lock analysis
query/index impact
RLS impact
privilege impact
data backfill plan
compatibility matrix
rollback/recovery plan
backup verification
performance evidence
security tests
```

High-risk migrations should require database/platform and security review.

---

# 90. CI Database Gate

A database-related PR must not be mergeable unless required checks pass.

Minimum gate:

```text
migration syntax
migration ordering
fresh database build
existing-schema migration
SQLx compile/test
schema assertions
RLS assertions
privilege assertions
transaction tests
concurrency tests where relevant
query/plan checks where required
```

The CI policy explicitly requires real PostgreSQL integration and release-blocking security checks. fileciteturn7file1L265-L369

---

# 91. Security Mutation Testing

The database security suite should contain mutation targets such as:

```text
remove tenant predicate
remove composite FK
remove unique constraint
weaken RLS policy
remove WITH CHECK
grant BYPASSRLS
change role privilege
remove branch scope
remove idempotency uniqueness
```

The test suite must fail when the security property is weakened.

The existing testing strategy explicitly treats surviving security mutations as evidence that the tests are too weak. fileciteturn7file6L953-L971

---

# 92. Required Database Security Fixtures

At minimum:

```text
TENANT_A
TENANT_B
BRANCH_A1
BRANCH_A2
BRANCH_B1
OWNER_A
MANAGER_A
CASHIER_A
AUDITOR_A
OWNER_B
DEVICE_A
DEVICE_B
REVOKED_DEVICE
```

Fixtures must include:

- products;
- inventory;
- sales;
- payments;
- suppliers;
- reports;
- exports;
- audit records.

The purpose is to test real cross-boundary behavior, not just isolated SQL snippets.

---

# 93. RLS Negative Test Matrix

For every major protected resource:

```text
same tenant + correct branch -> ALLOW
same tenant + unauthorized branch -> DENY
other tenant -> DENY
malformed tenant context -> DENY
missing tenant context -> DENY
revoked membership -> DENY
revoked device -> DENY where device-bound
support without ticket -> DENY
expired support access -> DENY
```

Test:

```text
SELECT
INSERT
UPDATE
DELETE
```

where each operation is intended.

Do not test only `SELECT`.

---

# 94. Information Disclosure Through Aggregation

RLS can still be undermined by poor aggregation/query design.

Example:

```text
SELECT COUNT(*) FROM sales;
```

might be safe under RLS only if policy semantics actually apply as expected to the querying role and query plan.

Search, joins, counts, existence checks, exports and reporting must all be included in the isolation suite.

---

# 95. RLS Performance

RLS predicates execute as part of query evaluation and can materially affect query plans.

Therefore benchmark:

```text
same query without RLS
same query with RLS
same query with real tenant predicate
```

The objective is not to remove RLS for performance. It is to design indexes and policy expressions so the safety boundary remains operationally viable.

Tenant predicates should be simple, deterministic and index-friendly.

---

# 96. RLS Policy Naming

Use stable names such as:

```text
<resource>_tenant_select
<resource>_tenant_insert
<resource>_tenant_update
<resource>_tenant_delete
```

Where support/admin policies are distinct:

```text
<resource>_support_select
```

Avoid one enormous policy expression containing every role in the entire platform.

Simple policies are easier to audit.

---

# 97. Restrictive vs Permissive Policies

PostgreSQL supports permissive and restrictive policy behavior.

Do not mix them casually.

The review for every protected table must document:

```text
which policies are permissive
which policies are restrictive
how multiple policies combine
whether a default-deny state remains intact
```

PostgreSQL's current `CREATE POLICY` semantics distinguish these combinations and require precise understanding of how policies combine. citeturn794774search0

---

# 98. `BYPASSRLS` is a Security Boundary Exception

Any role with `BYPASSRLS` can defeat the intended row security boundary.

Therefore:

```text
runtime application role -> MUST NOT bypass RLS
ordinary worker role -> MUST NOT bypass RLS
reporting role -> MUST NOT bypass RLS unless explicitly justified and isolated
migration role -> may have elevated authority but is never used for application traffic
break-glass -> exceptional and audited
```

The presence of `BYPASSRLS` on a normal runtime role is a release blocker.

---

# 99. Table Ownership

Database ownership matters because table owners and privileged roles may interact with RLS and privileges differently.

The architecture should therefore separate:

```text
object owner
DDL/migration identity
runtime identity
```

Runtime services should not own every production table merely because they created the database in a development environment.

This is a classic deployment hygiene failure.

---

# 100. Database Bootstrap

A clean environment bootstrap must create in deterministic order:

```text
database
extensions
schemas
owner roles
runtime roles
privileges
tables
constraints
indexes
functions
RLS
seed/reference data
verification fixtures
```

Do not create privileged roles ad hoc from local developer SQL scripts that are not represented in infrastructure source.

---

# 101. Reference Data

Reference data includes controlled vocabularies such as:

```text
currency
country
unit_of_measure
permission codes
status vocabularies
payment method classes
```

Reference data is different from tenant data.

Where reference data is globally authoritative, it should be owned and migrated centrally.

Never let one merchant modify a global permission or currency table through a tenant-scoped API.

---

# 102. Enum Strategy

PostgreSQL enums are useful for truly closed, stable vocabularies.

However, if states are expected to evolve rapidly or require tenant-specific extension, a constrained lookup/reference table may be more flexible.

Do not use database enums everywhere simply because they look type-safe.

Decision criteria:

```text
stability
migration frequency
API compatibility
query needs
localization
extension requirements
```

Rust enums remain the preferred domain representation for state machines where the vocabulary is closed.

---

# 103. JSONB Strategy

`JSONB` is permitted where the data is legitimately semi-structured or externally defined.

It is NOT a substitute for relational modeling of core business facts.

Bad:

```text
sale.metadata = entire sale domain as JSONB
```

Good candidates:

- provider-specific raw metadata under a bounded contract;
- optional extension attributes;
- versioned external evidence payloads where queryability is secondary.

Core invariants should remain relational when they materially affect correctness.

---

# 104. Raw Provider Payload Storage

External provider payloads are untrusted evidence.

Store only as much as needed.

Prefer:

```text
normalized provider fields
+
content hash
+
secure raw payload reference
```

rather than allowing huge JSON payloads to pollute every operational query.

Sensitive provider secrets must never be persisted merely because they appeared in a response.

---

# 105. Database Boundary for File/Object Storage

Object contents should not be placed in PostgreSQL merely because the DB can store binary data.

Use object storage for:

- large exports;
- documents;
- receipts/evidence files;
- large provider payloads where appropriate.

PostgreSQL stores:

```text
object identifier
metadata
ownership
classification
hash
retention state
```

Access remains tenant-scoped and audited.

---

# 106. Data Integrity Checks

Production-safe read-only integrity queries should detect:

```text
orphaned rows
negative stock where forbidden
sale totals inconsistent with line totals
refund greater than refundable amount
duplicate business numbers
duplicate webhook IDs
duplicate command IDs
outbox gaps
stale jobs
cross-tenant foreign-key mismatches
impossible states
```

These checks must be read-only.

An integrity script must not silently “repair” production records.

---

# 107. Invariant Ownership Matrix

| Invariant | Rust | PostgreSQL | Both |
|---|---:|---:|---:|
| valid command shape | ✓ |  |  |
| authorized operation | ✓ | RLS defense | ✓ |
| tenant ownership | ✓ | ✓ | ✓ |
| foreign-key existence |  | ✓ |  |
| uniqueness |  | ✓ |  |
| local numeric range | ✓ | ✓ | ✓ |
| complex refund eligibility | ✓ |  |  |
| inventory race | ✓ | ✓ | ✓ |
| financial append-only semantics | ✓ | privileges/constraints | ✓ |
| API error mapping | ✓ |  |  |
| external provider state | ✓ | persistence | ✓ |

The rule is not “put everything in the database.” The rule is:

> **Put every invariant in the strongest layer that can enforce it safely without creating hidden complexity, and duplicate critical security properties where defense in depth is justified.**

---

# 108. Advantages of This Database Architecture

## 108.1 Strong business correctness

Relational constraints prevent entire classes of malformed state that application-only validation cannot reliably block under races.

## 108.2 Strong tenant isolation

Application authorization plus RLS plus scoped foreign keys creates defense in depth.

## 108.3 Good fit for African SME operations

Sitolo's business model contains relationships that are naturally relational:

```text
merchant
branch
staff
product
stock
sale
payment
reconciliation
```

PostgreSQL is well suited to preserving these relationships transactionally.

## 108.4 Concurrency primitives

Atomic updates, locking, uniqueness constraints and PostgreSQL transaction semantics directly address POS/inventory races.

## 108.5 Auditability

Append-oriented facts and durable evidence make recovery and dispute analysis materially stronger.

## 108.6 Evolvability

A modular PostgreSQL foundation supports later service extraction without prematurely imposing distributed transaction complexity.

---

# 109. Disadvantages and Costs

## 109.1 Database expertise required

Incorrect schema design creates long-lived technical debt.

## 109.2 RLS complexity

RLS introduces policy behavior engineers must understand deeply. Incorrect policy combinations can create either leaks or unexpected denials.

## 109.3 Migration complexity

Growing a live multi-tenant system requires careful expand-and-contract patterns.

## 109.4 Index cost

Every index consumes memory, disk and write bandwidth.

## 109.5 Reporting separation

At scale, analytics may need read models or replicas, creating additional operational complexity.

---

# 110. Why PostgreSQL Over Alternatives

## 110.1 PostgreSQL over document-first persistence

Sitolo requires strong relations between:

- organizations;
- memberships;
- branches;
- inventory;
- sales;
- payments;
- reconciliation;
- audit.

A document-first design would not automatically simplify those relationships and would weaken the fit of mature relational constraints.

## 110.2 PostgreSQL over event store as sole authority

An event store alone would introduce additional operational and query complexity without removing the need for transactional relational views.

Sitolo can retain append-oriented facts and an outbox without making event sourcing the universal architecture.

## 110.3 PostgreSQL over client authority

Offline clients need continuity but cannot be permanent authorities for server truth.

The project explicitly chose PostgreSQL as server authority and SQLite as operational continuity. fileciteturn7file5L838-L887

## 110.4 RLS over application-only tenant filtering

Application-only filtering is fragile because a missed predicate can become a cross-tenant breach.

RLS provides defense in depth.

It must not be used as an excuse to weaken application authorization.

---

# 111. Phase 5 Implementation Sequence

The recommended implementation sequence is:

```text
STEP 1  PostgreSQL version/tooling lock
   ↓
STEP 2  database role/bootstrap strategy
   ↓
STEP 3  schema ownership
   ↓
STEP 4  organization/tenant foundation
   ↓
STEP 5  identity/membership/device tables
   ↓
STEP 6  RLS context mechanism
   ↓
STEP 7  RLS policies + privilege tests
   ↓
STEP 8  catalogue/pricing schema
   ↓
STEP 9  procurement/receiving schema
   ↓
STEP 10 inventory ledger/projections
   ↓
STEP 11 sales/cash schema
   ↓
STEP 12 payment/reconciliation schema
   ↓
STEP 13 tax/EIS schema
   ↓
STEP 14 audit/idempotency/sync/outbox
   ↓
STEP 15 indexes and query tuning
   ↓
STEP 16 integrity suite
   ↓
STEP 17 concurrency suite
   ↓
STEP 18 migration compatibility suite
   ↓
STEP 19 performance verification
   ↓
STEP 20 production database certification
```

---

# 112. Initial Migration Ordering

A first implementation should broadly follow dependency order:

```text
001 extensions / schemas
002 organizations
003 business_entities
004 branches
005 locations
006 warehouses
007 registers
008 users/identity references
009 memberships
010 roles/permissions
011 devices
012 catalogue/products
013 SKUs/barcodes
014 pricing
015 suppliers
016 procurement
017 inventory structure
018 inventory ledger
019 sales
020 cash
021 payments
022 reconciliation
023 tax/EIS
024 audit
025 idempotency
026 sync
027 outbox
028 jobs
029 reporting/billing foundations
030 RLS
031 privilege hardening
032 indexes/optimization
033 verification metadata
```

Actual migration order may differ when dependency analysis demands it. This sequence is conceptual, not a command to blindly create thirty-three files.

---

# 113. Initial Repository Layout

The repository should contain something close to:

```text
migrations/
  202609060001_bootstrap.sql
  202609060002_tenant.sql
  202609060003_identity.sql
  202609060004_authz.sql
  202609060005_catalogue.sql
  ...

db/
  fixtures/
  verification/
  policies/
  queries/
  docs/

crates/
  sitolo-persistence/
  sitolo-domain/
  sitolo-application/
  sitolo-testkit/
```

Do not duplicate migration SQL in application source code.

---

# 114. Database Test Harness Architecture

The security test harness should support:

```text
start PostgreSQL
    ↓
apply migrations
    ↓
create realistic roles
    ↓
seed tenants/branches/users/devices
    ↓
run schema assertions
    ↓
run privilege assertions
    ↓
run RLS suite
    ↓
run transaction suite
    ↓
run concurrency suite
    ↓
collect evidence
    ↓
destroy
```

Real PostgreSQL is mandatory for database security claims.

---

# 115. Failure Injection

The test harness must be capable of forcing:

```text
unique constraint violation
foreign key violation
check constraint violation
serialization failure
deadlock
statement timeout
connection failure
transaction rollback
worker crash after DB commit
worker crash before DB commit
```

The objective is to verify that the application never reports a false business success.

---

# 116. Transaction Atomicity Tests

For a sale finalization, deliberately fail after each critical stage:

```text
sale insert
sale item insert
inventory movement
inventory balance update
payment linkage
audit event
outbox insert
idempotency persistence
```

Expected result:

```text
ALL COMMIT
or
NONE COMMIT
```

except where a deliberate domain workflow explicitly creates a separate state.

---

# 117. Concurrency Tests

Minimum scenarios:

```text
two sellers consume final stock
sale + stock adjustment
sale + refund
refund + refund
payment callback + polling
duplicate payment callback
worker retry + previous success
sync command + manual mutation
cash close + late sale
membership revoke + in-flight request
```

The existing testing contract identifies these exact categories as high-value race scenarios. fileciteturn7file6L975-L1012

---

# 118. Tenant Boundary Tests for Nested Data

Do not stop at top-level tables.

Test nested relationships such as:

```text
Tenant A sale
  -> Tenant A sale_item
  -> Tenant A payment
  -> Tenant A inventory movement
  -> Tenant A audit
  -> Tenant A outbox
```

Attempt cross-tenant substitution at every relationship boundary.

This catches errors such as:

```text
sale tenant validated
but payment lookup not tenant-scoped
```

which is a common partial-authorization failure.

---

# 119. RLS with Joins

RLS tests must include joins.

Example risk:

```sql
SELECT sales.*, customers.phone
FROM sales
JOIN customers ON customers.id = sales.customer_id
WHERE sales.id = $1;
```

A safe sales policy does not automatically prove the joined customer data is correctly isolated.

Every protected table participating in a query must have appropriate access semantics.

---

# 120. RLS with Views

Views must be reviewed because they may:

- expose columns unintentionally;
- alter ownership semantics;
- bypass expected policy reasoning if built incorrectly;
- create reporting leakage.

Tenant-sensitive views require dedicated cross-tenant tests.

---

# 121. RLS with Functions

Functions called through the runtime role must be tested for:

```text
tenant leakage
privilege escalation
search_path attacks
security-definer abuse
unexpected row visibility
```

No function should return unrestricted data solely because it was convenient to expose it through one call.

---

# 122. RLS Policy Inventory

CI should maintain an expected policy inventory:

```text
table
RLS enabled?
forced?
select policy
insert policy
update policy
delete policy
runtime roles
support roles
worker roles
```

A protected table appearing without expected RLS should fail a security gate where RLS is required.

---

# 123. Constraint Inventory

Similarly maintain an expected constraint inventory:

```text
primary key
foreign keys
unique constraints
check constraints
exclusion constraints
```

Critical invariant documentation should link to the exact database object enforcing it.

---

# 124. Business Invariant Traceability

For every critical invariant, document:

```text
invariant ID
business rule
Rust enforcement
DB enforcement
API enforcement
security test
concurrency test
failure semantics
```

Example:

```text
INV-SALE-001
Finalized sale total is immutable.

Rust:
explicit compensation commands only

DB:
restricted update/delete privileges + append-oriented tables

API:
no generic PATCH

Tests:
attempt direct update
attempt refund over-sale
attempt replay
```

This creates a traceable engineering contract.

---

# 125. Performance Failure Modes

The database implementation must explicitly monitor:

```text
connection pool exhaustion
lock contention
hot-row contention
index bloat
vacuum lag
long transactions
replication lag
slow queries
report starvation
RLS plan regression
```

## 125.1 Hot-row risk

A single inventory balance row may become highly contended for very popular products.

Optimization options include:

- better physical partitioning;
- atomic operations;
- ledger plus projections;
- controlled sharding by location where later scale justifies it.

Do not prematurely distribute data across databases.

## 125.2 Idempotency hot rows

A popular client or provider replay storm can create contention on the same uniqueness/index paths.

Monitor this before adding new infrastructure.

---

# 126. Data Modeling Anti-Patterns

Explicitly prohibited:

```text
one giant tenant table
one generic entity table for everything
JSONB-only core domain
financial state as booleans
inventory as one mutable integer with no ledger
unscoped repository methods
arbitrary soft-delete everywhere
application-only uniqueness checks
audit-only-in-logs
RLS-only security model
public production DB
runtime role = migration owner
manual production SQL as normal workflow
```

---

# 127. Schema Review Questions

Every schema PR must answer:

```text
What business fact does this table represent?
Who owns it?
What is its aggregate boundary?
What tenant scope does it have?
What is immutable?
What is mutable?
What is the primary identity?
What uniqueness rules exist?
What foreign keys exist?
What local checks exist?
What transaction creates it?
What transaction changes it?
Can it be deleted?
What indexes support real queries?
What happens under concurrency?
What RLS policy protects it?
Which roles can access it?
What audit evidence is required?
What is the migration rollout strategy?
What is the recovery path?
```

A table that cannot answer these questions is not production-ready.

---

# 128. Database Definition of Ready

Phase 5 database work is ready for implementation when:

```text
[X] PostgreSQL target line documented
[X] authority model documented
[X] logical ownership model documented
[X] identifier strategy inherited from architecture
[X] monetary representation specified
[X] timestamp semantics specified
[X] tenant hierarchy defined
[X] domain ownership defined
[X] transaction boundaries inherited
[X] RLS strategy defined
[X] role model defined
[X] migration policy defined
[X] test model defined
[X] backup/recovery dependencies identified
```

---

# 129. Database Definition of Done

A Phase 5 implementation is done only when:

```text
[ ] clean database builds from migrations
[ ] migration history is deterministic
[ ] runtime roles are least privileged
[ ] no runtime role bypasses RLS unexpectedly
[ ] expected RLS policies exist
[ ] positive tenant tests pass
[ ] negative tenant tests pass
[ ] nested/indirect access tests pass
[ ] foreign keys pass
[ ] unique constraints pass
[ ] check constraints pass
[ ] transaction atomicity passes
[ ] concurrency suite passes
[ ] idempotency race tests pass
[ ] sync duplicate tests pass
[ ] query budgets pass
[ ] migration compatibility tests pass
[ ] backup succeeds
[ ] restore succeeds
[ ] integrity checks pass
[ ] observability is wired
[ ] database runbook exists
[ ] production migration approval process is active
```

---

# 130. Production Release Blockers

The database phase blocks release for:

```text
cross-tenant read
cross-tenant write
RLS disabled unexpectedly
runtime BYPASSRLS
missing critical unique constraint
duplicate command acceptance
inventory oversell
destructive financial mutation
broken foreign key integrity
migration lock risk without mitigation
unbounded report query
missing timeout
failed restore verification
schema drift
unknown production migration state
```

No product feature is important enough to waive a proven tenant-isolation failure.

---

# 131. Incident Runbook — Suspected Tenant Isolation Failure

```text
1. Declare security incident.
2. Identify affected role/table/policy/query.
3. Stop affected mutation path if necessary.
4. Preserve audit and database evidence.
5. Determine blast radius.
6. Identify policy/privilege/schema change.
7. Reproduce in isolated environment.
8. Correct authorization/RLS/privilege defect.
9. Run cross-tenant regression suite.
10. Verify no active exploit path remains.
11. Assess exposed records and regulatory impact.
12. Add permanent regression test.
13. Review ADR/security architecture if invariant changed.
```

---

# 132. Incident Runbook — Failed Migration

```text
1. Stop further deployment.
2. Determine whether migration committed, partially committed or failed before execution.
3. Inspect migration framework state.
4. Inspect schema/catalog state.
5. Determine application/schema compatibility.
6. Preserve logs and migration evidence.
7. Restore isolated copy if uncertainty is high.
8. Choose forward-fix or recovery path.
9. Never guess and rerun destructive SQL.
10. Verify constraints, RLS and privileges after recovery.
11. Resume deployment only after schema certification.
```

---

# 133. Incident Runbook — Connection Exhaustion

```text
1. Confirm pool saturation.
2. Check PostgreSQL connection count.
3. Identify source by role/application.
4. Inspect long-running transactions.
5. Inspect stuck workers/reporting jobs.
6. Protect POS pool.
7. Reduce or stop non-critical workloads.
8. Verify statement/lock timeout behavior.
9. Recover connections.
10. Determine why aggregate connection budget failed.
11. Add capacity or policy correction.
12. Add load/regression evidence.
```

---

# 134. Phase 5 Security Control Mapping

Phase 5 materially contributes to the existing 48-control security matrix:

```text
#1  exposed DB credentials
#3  hardcoded secrets
#5  missing authz defense
#6  cross-user access
#7  open DB permissions
#11 log leakage
#15 client-only security
#16 input/data validation
#17 SQL injection resistance
#24 recovery data protection
#25 session persistence state
#28 rate/DB resource protection
#32 payment server authority
#33 BOLA/IDOR
#34 API/resource constraints
#35 exposed logs
#39 business logic abuse
#40 race conditions
#41 webhook replay
#44 pinned dependencies/tooling
#46 missing timeouts
#48 complete endpoint/data security coverage
```

The existing security architecture maps database privileges, injection, tenant isolation and concurrency explicitly into the 48-control model. fileciteturn3file1L135-L179

---

# 135. Phase 5 ADR Dependencies

Phase 5 depends on:

```text
ADR-001 Rust-first backend
ADR-004 PostgreSQL authority
ADR-005 modular monolith
ADR-007 transactional outbox
ADR-008 authorization
ADR-009 tenant isolation
ADR-010 financial append-only model
ADR-011 inventory ledger
ADR-015 SQLite operational state
ADR-019 device identity
ADR-020 workers
ADR-021 OpenTelemetry + audit separation
ADR-022 release-blocking security gates
ADR-024 reproducible artifacts
ADR-025 failure/recovery evidence
```

The existing ADR set explicitly states that these decisions are cross-cutting architectural invariants. fileciteturn3file0L24-L48

---

# 136. Phase 5 Research Basis

This specification is based on the existing Sitolo contracts plus current PostgreSQL documentation.

### Sitolo sources

- `sitolo.md`
- `business_model_design.md`
- `system_architecture_design.md`
- `security_architecture_design.md`
- `security_implementation_spec.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `testing_strategy.md`
- `observability_spec.md`
- `ADR-001-025.md`
- Phase 1 repository/Rust/CI implementation document
- Phase 2 config/secrets/logging/errors/telemetry implementation
- Phase 3 identity/session/device implementation
- Phase 4 tenant/organization/branch/IAM implementation

### Current external technical references

- PostgreSQL 18.6 current documentation and supported-version information. citeturn794774search8
- PostgreSQL 18 constraint documentation. citeturn794774search1
- PostgreSQL 18 `CREATE POLICY` / RLS documentation. citeturn794774search0
- PostgreSQL security-definer/function security guidance. citeturn794774search3
- PostgreSQL policy and constraint catalog documentation. citeturn794774search2turn794774search5

---

# 137. Final Engineering Position

The correct Sitolo database is not:

```text
A bunch of tables behind a Rust API.
```

It is:

```text
A TRANSACTIONAL BUSINESS AUTHORITY
        |
        +--> RELATIONAL INTEGRITY
        |
        +--> FINANCIAL IMMUTABILITY
        |
        +--> INVENTORY LEDGER
        |
        +--> TENANT ISOLATION
        |
        +--> RLS DEFENSE IN DEPTH
        |
        +--> IDEMPOTENCY
        |
        +--> OUTBOX
        |
        +--> AUDIT EVIDENCE
        |
        +--> RECOVERABILITY
        |
        +--> CONCURRENCY CONTROL
        |
        +--> OBSERVABLE FAILURE
```

The most important database property is not raw throughput.

It is this:

```text
two cashiers race
      ↓
exactly one valid business result
      ↓
no invalid partial state
      ↓
audit explains the result
      ↓
retry does not duplicate it
      ↓
tenant boundary remains intact
```

The database must remain trustworthy when:

- the network disappears;
- mobile clients retry;
- two employees act simultaneously;
- payment providers replay events;
- tax systems reject submissions;
- devices are revoked;
- permissions change;
- a worker crashes;
- a deployment rolls back;
- a schema evolves;
- a report becomes unexpectedly expensive;
- a developer makes a mistake.

That is the standard required for Sitolo to be a real **Business Operating System for African SMEs**, rather than a conventional CRUD POS with a sophisticated diagram.

---

# 138. Phase 5 Exit Checklist

```text
ARCHITECTURE
[ ] PostgreSQL 18 target line verified
[ ] database authority preserved
[ ] schema ownership preserved
[ ] no competing source of truth introduced

SCHEMA
[ ] tenant hierarchy implemented
[ ] identity/membership/device persistence implemented
[ ] catalogue implemented
[ ] pricing implemented
[ ] procurement implemented
[ ] inventory ledger implemented
[ ] sales implemented
[ ] cash implemented
[ ] payments implemented
[ ] reconciliation implemented
[ ] tax/EIS state implemented
[ ] audit implemented
[ ] idempotency implemented
[ ] sync persistence implemented
[ ] outbox implemented
[ ] worker job persistence implemented

INTEGRITY
[ ] PK/FK constraints verified
[ ] unique constraints verified
[ ] check constraints verified
[ ] composite tenant FKs verified
[ ] financial immutability strategy verified
[ ] inventory invariants verified

SECURITY
[ ] runtime role least privilege verified
[ ] BYPASSRLS absent from runtime role
[ ] RLS-enabled tables inventory verified
[ ] USING policies verified
[ ] WITH CHECK policies verified
[ ] cross-tenant reads denied
[ ] cross-tenant writes denied
[ ] branch-scope isolation verified
[ ] support isolation verified
[ ] worker scope verified

MIGRATIONS
[ ] clean bootstrap works
[ ] existing-data migration works
[ ] compatibility path documented
[ ] destructive migrations identified
[ ] large-table migration plan verified
[ ] schema drift detection active

CONCURRENCY
[ ] inventory race tests pass
[ ] idempotency race tests pass
[ ] webhook replay tests pass
[ ] cash-close race tests pass
[ ] deadlock handling verified
[ ] serialization behavior verified

PERFORMANCE
[ ] POS query budgets pass
[ ] tenant-scoped queries indexed
[ ] lock waits measured
[ ] pool limits measured
[ ] report workload isolated
[ ] RLS performance reviewed

RECOVERY
[ ] backup configured
[ ] restore tested
[ ] migration recovery runbook tested
[ ] integrity verification executable

CI
[ ] schema checks blocking
[ ] migration checks blocking
[ ] RLS checks blocking
[ ] role checks blocking
[ ] concurrency checks blocking
[ ] security mutations fail
```

**Phase 5 may advance to Phase 6 only when the remaining unchecked items are either completed or explicitly tracked as approved, non-blocking dependencies. Security-critical unknowns may not be silently converted into assumptions.**

---

# 139. Non-Negotiable Database Rules

```text
1. PostgreSQL is authoritative for server-side business truth.
2. SQLite is continuity state, not a competing authority.
3. Financial history is append-oriented.
4. Inventory is ledger-backed.
5. Tenant scope is explicit.
6. Client-provided tenant IDs are not proof of authority.
7. Runtime roles are least privileged.
8. Runtime application roles do not bypass RLS.
9. RLS is defense in depth, not the only authorization layer.
10. Every protected write has a WITH CHECK strategy.
11. Critical replay boundaries have uniqueness constraints.
12. Cross-tenant parent/child relationships use strong scoped integrity where useful.
13. Every critical mutation has an explicit transaction boundary.
14. External calls never occur inside long-lived business transactions.
15. No financial correction is implemented as destructive editing.
16. No uncontrolled reporting workload can starve POS traffic.
17. No unbounded connection pool exists.
18. No unbounded query exists on user-controlled input.
19. No migration depends on undocumented manual SQL.
20. No destructive migration is called “safe” merely because tooling can generate a down script.
21. No secret belongs in source code or logs.
22. No RLS claim is accepted without real PostgreSQL tests.
23. No database security gate becomes advisory because a scanner is inconvenient.
24. No schema change ships without compatibility analysis.
25. No restore claim is accepted without a restore drill.
26. No production database is publicly exposed without an explicit architecture exception.
27. No generic repository method may bypass domain transaction semantics.
28. No worker receives unrestricted database authority merely because it runs internally.
29. No support operator receives implicit tenant-global access.
30. Enterprise database complexity must be justified by measured requirements.
```

---

**END OF `phase5_postgresql_schema_migrations_constraints_rls_implementation.md`**
