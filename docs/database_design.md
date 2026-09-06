# SITOLO — Database Design

**Document:** `database_design.md`  
**Phase:** Phase 0 — Architecture / Contracts / ADR Freeze  
**Sequence:** File 03 of 16  
**Status:** Implementation-governing database specification  
**Product:** Sitolo — Business Operating System for African SMEs  
**Prepared:** 2026-09-04  
**Primary database:** PostgreSQL 18 target line  
**Database access:** Rust + SQLx  
**Authoritative state:** PostgreSQL  
**Offline operational store:** SQLite on Flutter/Tauri clients  
**Architecture:** Modular monolith first, durable outbox/workers, adapters at external boundaries

> This document defines the authoritative server-side persistence model for Sitolo. It translates the domain model into relational structures, constraints, indexes, transactions, security boundaries, migration rules, partitioning policy, retention behavior, audit/evidence structures, and operational database practices. It is not an API specification and it must not become a mechanical table-per-endpoint design.

---

# 0. Executive Database Decision

Sitolo will use **PostgreSQL as the authoritative source of server-side business truth**. The database is not merely a persistence layer behind Rust. It is part of the business correctness and security boundary.

PostgreSQL is well suited to Sitolo because the product is transaction-heavy and relationship-heavy: organizations, memberships, branches, catalogues, inventory, sales, payments, approvals, audit, procurement, reconciliation, and subscriptions all have integrity relationships that benefit from relational constraints and ACID transactions. PostgreSQL 18's current documentation covers the relational DDL, native types, row-level security, indexing, partitioning, and related primitives used by this design. citeturn760439search11turn760439search1turn760439search6turn760439search18

The database architecture follows this rule:

```text
CLIENTS
  ↓
RUST APPLICATION / DOMAIN
  ↓
AUTHORIZED TRANSACTION
  ↓
POSTGRESQL
  ├── authoritative transactional state
  ├── immutable financial facts
  ├── inventory ledger
  ├── audit/evidence
  ├── idempotency records
  └── transactional outbox
        ↓
   WORKERS / READ MODELS / INTEGRATIONS
```

The client is never allowed to define authoritative business truth merely because it has a local SQLite copy. Rust is responsible for domain decisions and orchestration. PostgreSQL is responsible for durable transactional state, relational integrity, concurrency controls, and selected defense-in-depth authorization constraints. This preserves the existing Sitolo system contract: clients are operational surfaces, Rust is the decision engine, PostgreSQL is authoritative, and SQLite preserves local continuity. fileciteturn9file3L273-L279

---

# 1. Relationship to Existing Sitolo Contracts

This document derives directly from:

1. `business_model_design.md` — commercial and operational model.
2. `sitolo.md` — product/domain specification.
3. `system_architecture_design.md` — runtime and architectural authority.
4. `security_architecture_design.md` — security architecture and release controls.
5. `security_implementation_spec.md` — implementation-level security enforcement.
6. `domain_model.md` — domain concepts, aggregates, value objects, state machines, and invariants.

The domain model already establishes that Sitolo is not a simple CRUD application. It defines bounded contexts including identity, tenant, catalogue, pricing, procurement, inventory, sales, cash, payments, reconciliation, tax/EIS, reporting, billing, audit, platform administration, and vertical extensions. fileciteturn12file0L144-L220

The implementation sequence also explicitly places PostgreSQL and the trust foundation before large business domains. fileciteturn10file4L585-L647

## 1.1 Non-negotiable inheritance rules

The database MUST preserve these existing architectural rules:

- PostgreSQL is the server-side source of truth.
- SQLite is an offline client store, not a second authority.
- Financial facts are append-oriented and corrected through explicit compensating events.
- Inventory uses a ledger-backed model with a current balance projection.
- Tenant isolation is a security boundary.
- Authorization is server-enforced.
- Database constraints are part of the security model.
- External provider state does not overwrite authoritative Sitolo state without validation.
- Redis, if introduced, is never authoritative financial state.
- Reporting/read models are derived data, not another financial source of truth.
- Transactional outbox is preferred before introducing a heavyweight broker.
- Schema migrations are controlled, reviewable, reproducible, and tested.
- Runtime database credentials have less privilege than migration/schema-owner credentials.

---

# 2. Database Responsibilities

PostgreSQL owns six classes of truth.

## 2.1 Identity and access persistence

PostgreSQL stores Sitolo's application-level identity references, memberships, devices, sessions where the architecture chooses server-side sessions, roles, permissions, policy assignments, and authorization-related state.

The database does not need to store every secret emitted by an external identity provider. Password hashing and primary authentication should preferably be delegated to mature identity infrastructure, while Sitolo persists the identity subject and the application relationship it needs.

## 2.2 Organizational truth

PostgreSQL owns the organization/business hierarchy:

```text
Organization
 ├── Business Entity
 ├── Memberships
 ├── Branches
 │    ├── Locations
 │    ├── Warehouses
 │    ├── Registers
 │    └── Devices
 └── Configuration
```

## 2.3 Operational truth

The database owns products, price versions, purchase orders, receipts, stock movements, sales, cash events, payment intents, reconciliation cases, tax submission state, and related operational facts.

## 2.4 Audit and evidence

Security and business actions that require reconstruction have durable audit records. Operational logs are separate and are not a substitute for business audit.

## 2.5 Derived state

The database may contain read models, cached projections, search-support structures, materialized views, counters, and reporting tables. These remain rebuildable from authoritative state unless explicitly documented otherwise.

## 2.6 Integration durability

The database owns outbox records, idempotency records, provider-event identities, synchronization checkpoints, retry state, and reconciliation evidence required to make external work safe.

---

# 3. PostgreSQL Version Baseline

The target database line is **PostgreSQL 18.x** for production infrastructure unless a hosting constraint requires another currently supported version. PostgreSQL 18 was released in 2025 and its current documentation is the active reference for this design. PostgreSQL 18 includes improvements such as asynchronous I/O, multicolumn B-tree skip-scan support, UUIDv7 generation, and other database improvements. citeturn760439search15

The exact patch version is pinned by the infrastructure environment and upgraded through controlled maintenance.

## 3.1 Why not use an older arbitrary PostgreSQL version

Sitolo's architecture needs current security fixes, supported tooling, predictable extensions, and current planner/runtime improvements. The application must not silently depend on an old vendor image because a developer laptop happens to have it installed.

## 3.2 Version policy

```text
MAJOR
  chosen deliberately and documented

MINOR/PATCH
  managed by infrastructure policy

APPLICATION COMPATIBILITY
  verified in CI

PRODUCTION UPGRADE
  backup + staging test + rollback/recovery plan
```

---

# 4. Logical Database Schemas

The default PostgreSQL database should use logical schemas to separate ownership and reduce accidental coupling.

Recommended baseline:

```text
postgres database
│
├── app
│   ├── tenant
│   ├── identity
│   ├── catalogue
│   ├── pricing
│   ├── procurement
│   ├── inventory
│   ├── sales
│   ├── cash
│   ├── payments
│   ├── reconciliation
│   ├── tax
│   ├── reporting
│   ├── billing
│   └── platform
│
├── audit
│
├── integration
│
├── outbox
│
├── staging
│
└── extensions / infrastructure-owned objects
```

The exact choice between many PostgreSQL schemas and one schema with strong naming may be revisited after migration/tooling experience. The architectural requirement is stronger than the physical choice: modules must have ownership boundaries, and tables must not become uncontrolled shared state.

## 4.1 Schema ownership rule

A domain module may own its tables but not arbitrarily mutate another module's tables.

Bad:

```text
sales handler
  -> direct UPDATE inventory.inventory_balance
```

Preferred:

```text
sales application service
  -> inventory domain/application contract
  -> controlled transaction coordination
  -> repository operations
```

Within the modular monolith, a transaction can still span modules. Logical ownership does not require a network hop.

---

# 5. Naming Conventions

All persistent identifiers and names follow consistent conventions.

## 5.1 Table names

Use lowercase `snake_case`, plural where a collection is represented:

```text
organizations
memberships
branches
products
skus
sales
sale_items
inventory_movements
payment_intents
```

## 5.2 Column names

Use lowercase `snake_case`.

Examples:

```text
organization_id
created_at
updated_at
occurred_at
valid_from
valid_to
provider_event_id
```

## 5.3 Primary keys

Default:

```text
<entity>_id
```

with UUID or UUIDv7-compatible representation depending on the repository identifier decision.

PostgreSQL 18 includes built-in UUIDv7 generation capabilities, making time-ordered UUIDs a viable option where the application wants globally unique sortable identifiers. citeturn760439search15

The final choice must be standardized repository-wide. Mixed identifier strategies are permitted only when there is a documented domain reason.

## 5.4 Timestamp columns

Use explicit timezone-aware timestamps for persisted events:

```sql
timestamp with time zone
```

Avoid storing business-critical event timestamps as ambiguous local time.

Retain client-provided timestamps separately when they are useful evidence for offline operations.

## 5.5 Boolean names

Prefer semantic names:

```text
is_active
is_default
requires_approval
```

Avoid ambiguous fields such as `status_flag`.

---

# 6. Identifier Strategy

Sitolo needs several distinct identifier classes.

```text
Database Identity
  UUID / UUIDv7

Human Business Number
  sale_number
  purchase_order_number
  receipt_number

Security / Command Identity
  command_id
  idempotency_key
  event_id
  correlation_id

External Identity
  provider_transaction_id
  provider_event_id
  external_reference
```

These MUST NOT be treated as interchangeable.

A human sale number can be presented on a receipt. It is not proof of authorization, and it is not necessarily the primary key.

## 6.1 Public IDs

External APIs should expose stable opaque identifiers. Sequential integer database IDs should not be exposed as the authorization boundary.

## 6.2 Human-readable numbers

Sale and receipt numbers may use business-specific sequences, but their generation must remain tenant/branch scoped where the business process requires it and must be concurrency-safe.

A number generator must not assume a single application process.

## 6.3 UUID collision and uniqueness

The database remains the final uniqueness authority.

Application-generated UUIDs are not accepted as an alternative to a database unique constraint.

---

# 7. Core Data Types

PostgreSQL has a rich native type system; use native types when they materially improve correctness. citeturn760439search1

## 7.1 UUID

Use UUID for distributed IDs and security-sensitive object identities.

## 7.2 Numeric / exact decimal

Avoid PostgreSQL floating point for monetary truth.

Money can be represented using integer minor units when currency rules support that cleanly or exact `numeric` values where fractional precision is needed.

The domain model's `Money` value object remains the semantic authority.

## 7.3 Text

Use `text` unless a bounded `varchar(n)` materially communicates a domain invariant. Do not use arbitrary small `varchar` limits merely because a UI currently shows a short field.

## 7.4 JSONB

Use `jsonb` only for bounded extensibility where relational structure would add disproportionate coupling. PostgreSQL provides native JSON/JSONB operations and indexing support, but a JSON column must not become a dumping ground for core relational state. citeturn760439search4turn760439search26

Good candidates:

```text
vertical extension metadata
provider payload snapshots
configuration fragments
versioned policy documents
```

Bad candidates:

```text
organization_id
sku_id
quantity
sale_total
payment_state
```

Core predicates and security boundaries must remain relational and indexable.

## 7.5 Arrays

Use arrays sparingly. They are suitable for genuinely multi-valued attributes with well-defined semantics but should not replace join tables for relational entities.

## 7.6 Enumerations

For closed, stable domain vocabularies, use PostgreSQL enums or constrained text according to migration requirements. Because domain vocabularies evolve, many long-lived business state values should use application/domain enums backed by constrained text or lookup tables when adding states must be backward compatible.

---

# 8. Standard Audit Columns

Most mutable operational tables should have a consistent baseline where relevant:

```text
created_at
updated_at
created_by
updated_by
```

However, immutable facts use a different model.

For example, `inventory_movements` and finalized `sale_items` should not pretend that `updated_at` means the business fact changed. If immutable facts need metadata correction, use a separate correction/evidence model rather than mutating historical meaning.

## 8.1 Soft deletion

Do not default to `deleted_at` on every table.

Use explicit lifecycle state when the business record remains historically meaningful.

Examples:

```text
Product -> DISCONTINUED / ARCHIVED
User -> DISABLED
Branch -> CLOSED
Device -> REVOKED
```

Hard deletion is reserved for records where deletion is legally and operationally appropriate and where no immutable business history depends on the record.

---

# 9. Tenant Key Strategy

Tenant-owned data must have an explicit tenant key wherever practical.

Default pattern:

```sql
organization_id uuid NOT NULL
```

For deeply tenant-owned rows, this column should normally be present even if a parent relation already implies organization membership.

Why duplicate a tenant key at multiple levels?

Because it enables:

- tenant-scoped indexes;
- efficient authorization predicates;
- PostgreSQL RLS policies;
- cross-tenant negative tests;
- defensive composite foreign-key patterns;
- easier operational inspection;
- safer bulk export filtering.

The denormalization is intentional and must be kept consistent by constraints and application logic.

---

# 10. Cross-Tenant Referential Integrity

A simple foreign key such as:

```text
child.organization_id -> parent.organization_id
```

does not by itself guarantee that the child's parent belongs to the same tenant unless the schema is designed to enforce that relationship.

For critical tenant-owned relationships, use one of these patterns.

## Pattern A — Composite parent key

```sql
UNIQUE (organization_id, id)
```

and:

```sql
FOREIGN KEY (organization_id, parent_id)
REFERENCES parent (organization_id, id)
```

This is strongly preferred for relationships where cross-tenant linkage would be catastrophic.

## Pattern B — Parent-derived tenant identity

Use a table relationship where the parent owns the scope and the child never independently receives client-controlled scope.

## Pattern C — Application invariant plus RLS

Suitable when the relational structure is more complex, but still requires automated cross-tenant tests.

The database design should prefer Pattern A for high-risk ownership relationships.

---

# 11. Organization and Business Entity Tables

## 11.1 `organizations`

Purpose: top-level merchant tenant.

Core fields:

```text
organization_id
name
slug / external_key
status
country_code
default_currency
default_timezone
created_at
updated_at
```

Constraints:

- organization identity unique;
- slug unique according to global/tenant policy;
- country code valid against supported configuration;
- currency required;
- timezone required;
- closed/suspended organization cannot execute prohibited operations.

## 11.2 `business_entities`

Purpose: legal/operating identity within an organization.

Fields:

```text
business_entity_id
organization_id
legal_name
trading_name
registration_number?
tax_identifier?
entity_type
status
created_at
updated_at
```

Tax identifiers are sensitive and must follow the data-classification policy.

An organization may have one primary legal entity initially while preserving the model for more complex enterprises.

## 11.3 `organization_settings`

Configuration that is legitimately tenant scoped:

```text
organization_id
locale
currency_policy
timezone
stock_policy
pricing_policy
tax_policy_version
receipt_policy
security_policy_version
```

Do not place arbitrary configuration into one unrestricted JSON object merely for convenience.

---

# 12. Branch, Location, Warehouse, Register and Device Tables

## 12.1 `branches`

Fields:

```text
branch_id
organization_id
business_entity_id?
code
name
status
address_reference?
timezone?
created_at
updated_at
```

Constraints:

```text
UNIQUE(organization_id, code)
```

## 12.2 `locations`

A location is a physical/logical operational point.

```text
location_id
organization_id
branch_id
code
name
location_type
status
```

`location_type` may distinguish:

```text
SELLING
WAREHOUSE
STORAGE
QUARANTINE
RETURN
OTHER
```

## 12.3 `warehouses`

A warehouse may be a specialized location reference or its own domain entity depending on implementation. The database must not duplicate warehouse and location state unnecessarily.

Preferred initial relation:

```text
warehouse_id
organization_id
branch_id
location_id
code
name
status
```

## 12.4 `registers`

```text
register_id
organization_id
branch_id
location_id
code
name
status
```

Register ownership is organization scoped and branch constrained.

## 12.5 `devices`

```text
device_id
organization_id
branch_id?
user_binding_policy
installation_identifier
status
registered_at
last_seen_at
app_version
os_version
revoked_at
revoke_reason
```

Do not store authentication credentials directly in the device table unless a specific credential-hash lifecycle requires it.

Device IDs are identifiers, not credentials.

---

# 13. Identity and Membership Tables

## 13.1 `users`

The `users` table stores application-level human identity information that Sitolo needs.

Typical fields:

```text
user_id
identity_subject
display_name
status
created_at
updated_at
```

Authentication-provider secrets and password material do not belong here when delegated to an identity service.

## 13.2 `memberships`

```text
membership_id
organization_id
user_id
status
joined_at
left_at
created_at
updated_at
```

Constraints:

```text
UNIQUE(organization_id, user_id)
```

A disabled membership must not authorize new business actions.

## 13.3 `membership_roles`

Associates a membership with one or more role definitions.

## 13.4 `role_definitions`

Roles are configurable bundles of permissions but should reference stable permission identifiers.

## 13.5 `permissions`

Permission records represent capabilities such as:

```text
SALE_CREATE
SALE_VIEW
SALE_VOID
REFUND_CREATE
REFUND_APPROVE
STOCK_ADJUST
STOCK_ADJUST_APPROVE
PRICE_OVERRIDE
PURCHASE_ORDER_CREATE
PURCHASE_ORDER_APPROVE
USER_MANAGE
EXPORT_EXECUTE
```

## 13.6 `membership_scopes`

A normalized scope table can represent:

```text
organization
branch
warehouse
register
module/action restrictions
```

Avoid putting an authorization policy's complete semantics into arbitrary JSON unless a policy engine contract explicitly requires it.

---

# 14. Session and Security Persistence

If Sitolo uses server-managed application sessions, use a table such as:

```text
sessions
 ├── session_id
 ├── user_id
 ├── organization context?
 ├── device_id?
 ├── token_family_hash?
 ├── created_at
 ├── expires_at
 ├── revoked_at
 └── last_seen_at
```

Refresh-token family tracking should store hashes or derived identifiers rather than raw refresh tokens when server persistence is required.

Security-sensitive session rows must be protected by least-privilege access. Reporting roles must not gain session-token material.

---

# 15. Catalogue Schema

The catalogue defines what can be stocked and sold.

## 15.1 `products`

```text
product_id
organization_id
name
description
category_id?
brand_id?
lifecycle_state
domain_classification
created_at
updated_at
```

## 15.2 `skus`

```text
sku_id
organization_id
product_id
sku_code
name
base_unit_id
tax_category_id?
status
attribute_data?
created_at
updated_at
```

Constraints:

```text
UNIQUE(organization_id, sku_code)
```

## 15.3 `units_of_measure`

Depending on final domain complexity, units can be global definitions plus tenant-specific labels.

Core fields:

```text
unit_id
organization_id?
code
name
unit_dimension
allows_fractional
```

## 15.4 `sku_units`

Defines permitted commercial units and conversion factors.

```text
sku_id
unit_id
conversion_to_base
is_purchase_unit
is_sale_unit
valid_from
valid_to
```

Conversion factor constraints:

```text
conversion_to_base > 0
```

Changing conversions requires historical semantics. Existing stock movements preserve the unit and quantity as posted.

## 15.5 `barcodes`

```text
barcode_id
organization_id
sku_id
barcode
barcode_type
status
```

Constraint:

```text
UNIQUE(organization_id, barcode)
```

---

# 16. Catalogue History and Classification

Historical sales must not depend on mutable product names or classifications.

Therefore finalized sale lines store snapshots or immutable references sufficient to reconstruct the commercial fact.

Avoid the dangerous pattern:

```text
sale_item
  -> sku_id
  -> JOIN current sku name/price
```

for historical display where current values can change.

Prefer:

```text
sale_item
  ├── sku_id
  ├── sku_code_snapshot
  ├── product_name_snapshot
  ├── unit_snapshot
  ├── unit_price_snapshot
  └── tax_snapshot
```

The SKU relationship remains useful for analytics; snapshots preserve historical meaning.

---

# 17. Category and Brand Data

Use relational entities where categories/brands are queried, authorized, or reported independently.

Avoid treating category names as free text scattered across transactional tables.

Potential tables:

```text
categories
brands
category_memberships
```

All tenant-owned catalogue classification must be organization-scoped unless explicitly global.

---

# 18. Pricing Schema

## 18.1 `price_lists`

```text
price_list_id
organization_id
name
scope_type
scope_reference?
status
currency
priority
created_at
updated_at
```

## 18.2 `price_versions`

```text
price_version_id
organization_id
price_list_id
sku_id
amount
currency
valid_from
valid_to
created_by
created_at
```

Temporal overlap must be constrained by application/domain rules and, where appropriate, PostgreSQL exclusion constraints using range types. PostgreSQL 18 provides native range/multirange support and current documentation covers range indexing and constraints. citeturn760439search1

Do not implement overlapping prices by trusting a `SELECT` followed by `INSERT`; concurrent writers require database-backed uniqueness/exclusion or a properly locked transaction.

## 18.3 Promotions

Recommended tables:

```text
promotions
promotion_rules
promotion_targets
promotion_prices / promotion_discounts
```

Promotion rules must be bounded and typed. Avoid arbitrary executable expressions in database rows.

## 18.4 Discount approvals

A manual discount record should capture:

```text
discount_id
sale_id?
sale_line_id?
actor_id
reason
requested_amount
approved_amount
approval_id?
created_at
```

---

# 19. Procurement Schema

## 19.1 `suppliers`

```text
supplier_id
organization_id
name
supplier_code
contact_reference?
tax_identifier?
status
created_at
updated_at
```

## 19.2 `purchase_orders`

```text
purchase_order_id
organization_id
branch_id
supplier_id
purchase_order_number
status
currency
requested_by
approved_by?
created_at
submitted_at?
approved_at?
closed_at?
```

Constraint:

```text
UNIQUE(organization_id, purchase_order_number)
```

## 19.3 `purchase_order_items`

```text
purchase_order_item_id
organization_id
purchase_order_id
sku_id
ordered_quantity
unit
unit_cost
expected_total
```

## 19.4 `goods_receipts`

```text
goods_receipt_id
organization_id
branch_id
purchase_order_id?
receipt_number
status
received_by
received_at
created_at
```

## 19.5 `goods_receipt_items`

```text
goods_receipt_item_id
goods_receipt_id
organization_id
sku_id
quantity
unit
unit_cost
lot_id?
expiry_date?
discrepancy_code?
```

Receipt posting must create inventory ledger movements in the same authoritative transaction.

---

# 20. Inventory Schema

Inventory is one of the highest-integrity database domains.

The model consists of:

```text
inventory_lots
inventory_movements
inventory_balances
inventory_transfers
stock_counts
stock_count_items
inventory_adjustments
```

## 20.1 `inventory_lots`

```text
lot_id
organization_id
sku_id
lot_number
expiry_date?
manufacture_date?
status
supplier_id?
created_at
```

Lot status may include:

```text
AVAILABLE
QUARANTINED
EXPIRED
RECALLED
DAMAGED
RETURNED
PENDING_INSPECTION
```

## 20.2 `inventory_movements`

This is an immutable ledger fact.

```text
movement_id
organization_id
branch_id
location_id
sku_id
lot_id?
movement_type
quantity
unit
source_type
source_id
occurred_at
posted_at
actor_id?
device_id?
command_id?
reason_code?
metadata?
```

The quantity should have unambiguous sign semantics. One viable strategy is to store signed quantity for a movement line while preserving movement type. Another is to store positive quantity and derive direction from movement type. The repository MUST choose one and never mix conventions.

This design recommends:

```text
quantity = signed domain quantity
```

with movement type providing semantic explanation.

## 20.3 `inventory_balances`

A projection for fast reads:

```text
organization_id
location_id
sku_id
lot_id?
inventory_state
quantity_available
quantity_reserved
version
updated_at
```

Suggested uniqueness:

```text
UNIQUE(
  organization_id,
  location_id,
  sku_id,
  lot_id,
  inventory_state
)
```

NULL semantics for `lot_id` must be handled deliberately because PostgreSQL's unique constraints normally permit multiple NULL values. Use a design that guarantees the intended cardinality, such as `NULLS NOT DISTINCT` where appropriate in supported PostgreSQL versions, or a normalized representation of lot state.

## 20.4 Atomic stock decrement

Critical inventory consumption should be implemented as a single transactionally protected operation.

Conceptually:

```sql
UPDATE inventory_balances
SET quantity_available = quantity_available - $quantity,
    version = version + 1,
    updated_at = now()
WHERE organization_id = $organization_id
  AND location_id = $location_id
  AND sku_id = $sku_id
  AND inventory_state = 'AVAILABLE'
  AND quantity_available >= $quantity;
```

Then require exactly one affected row before posting the corresponding movement.

The operation MUST occur inside one transaction with authorization, invariant checks, movement insertion, and relevant outbox/audit records.

---

# 21. Inventory Ledger Invariants

The database and application must preserve:

```text
available = ledger-derived quantity under defined policy
```

and:

```text
A posted movement cannot be silently changed into another movement.
```

## Mandatory constraints

- organization ID required;
- SKU required;
- location required;
- quantity non-zero unless zero-valued evidence is explicitly useful;
- movement type valid;
- source identity present for movement classes that require one;
- lot required for domains/policies that mandate lot traceability;
- expiry cannot precede manufacture when both are supplied;
- impossible state transitions are rejected by application/domain logic;
- balance rows cannot become negative under the default stock policy.

Database constraints protect basic invariants; domain services protect cross-aggregate rules.

---

# 22. Inventory Transfer Schema

Internal transfers require one business identity and two linked movement facts.

Tables:

```text
inventory_transfers
inventory_transfer_items
```

`inventory_transfers`:

```text
transfer_id
organization_id
source_location_id
destination_location_id
status
requested_by
approved_by?
created_at
posted_at?
```

A finalized transfer produces:

```text
TRANSFER_OUT
TRANSFER_IN
```

under a single transfer identity.

The transaction must guarantee that a completed transfer cannot post only one side.

---

# 23. Stock Count Schema

Tables:

```text
stock_counts
stock_count_items
```

`stock_counts`:

```text
stock_count_id
organization_id
location_id
status
opened_at
submitted_at?
reviewed_at?
posted_at?
created_by
reviewed_by?
approved_by?
```

Items record expected and observed state:

```text
stock_count_item_id
stock_count_id
sku_id
lot_id?
expected_quantity
counted_quantity
variance_quantity
unit
```

Posting creates explicit adjustment/reconciliation movements. The stock count itself does not rewrite historical movement history.

---

# 24. Sales Schema

## 24.1 `sales`

```text
sale_id
organization_id
branch_id
location_id
register_id?
device_id?
seller_user_id
customer_id?
sale_number
status
payment_status
currency
subtotal
discount_total
tax_total
total
occurred_at
finalized_at?
command_id?
created_at
```

A finalized sale requires immutable commercial facts and an auditable origin.

## 24.2 `sale_items`

```text
sale_item_id
organization_id
sale_id
line_number
sku_id
product_name_snapshot
sku_code_snapshot
quantity
unit
unit_price
line_subtotal
discount_amount
tax_amount
line_total
pricing_snapshot_id?
```

Line number should be unique per sale:

```text
UNIQUE(sale_id, line_number)
```

## 24.3 Tax snapshots

Tax calculation data belongs to historical sale evidence.

Possible fields:

```text
tax_category_code_snapshot
tax_rate_snapshot
tax_amount
```

Tax configuration may evolve; finalized transactions must remain interpretable.

---

# 25. Sale State Machine at Database Level

Database CHECK constraints can enforce that state values are members of known vocabularies, but transitions should primarily be controlled in domain/application transactions.

Conceptual lifecycle:

```text
DRAFT
  ↓
PENDING_CONFIRMATION
  ↓
FINALIZED
  ↓
SETTLED / PARTIALLY_SETTLED / UNSETTLED
```

A finalized sale cannot be changed to arbitrary totals.

Corrections create:

```text
RETURN
REVERSAL
REFUND
CORRECTED TRANSACTION
```

rather than modifying original economic facts.

---

# 26. Sale Idempotency

Every state-changing sale command must have a stable idempotency identity.

Recommended table:

```text
idempotency_records
```

Fields:

```text
idempotency_record_id
organization_id
actor_id
scope_key
idempotency_key
request_hash
status
response_reference?
created_at
expires_at
```

Constraint:

```text
UNIQUE(organization_id, actor_id, scope_key, idempotency_key)
```

The exact scope key is an application decision, but must prevent one tenant's idempotency namespace from colliding with another tenant's.

The idempotency table records the result boundary; financial truth still resides in the sale/payment/inventory tables.

---

# 27. Returns, Reversals and Refunds

Do not create a generic `sale_edits` table.

## 27.1 Returns

```text
returns
return_items
```

A return references the original sale and identifies which quantity/value is being returned.

## 27.2 Reversals

A reversal is an explicit compensating event:

```text
sale_reversals
```

It references the original finalized transaction.

## 27.3 Refunds

```text
refunds
refund_items / refund_allocations
```

Constraints and application rules must ensure:

```text
refunded_amount <= refundable_amount
```

The database should maintain sufficient immutable evidence to prove the calculation.

---

# 28. Cash Management Schema

Cash is not synonymous with sales.

## Tables

```text
register_sessions
cash_events
cash_counts
cash_variances
```

## 28.1 `register_sessions`

```text
register_session_id
organization_id
branch_id
register_id
device_id?
opened_by
status
opening_float
opened_at
closed_by?
closed_at?
expected_cash
counted_cash?
variance_amount?
```

At most one active session should exist per register under the intended business policy.

This requires a concurrency-safe uniqueness strategy, potentially a partial unique index over active states.

## 28.2 `cash_events`

Immutable cash movement facts:

```text
cash_event_id
organization_id
register_session_id
event_type
amount
currency
source_type
source_id
actor_id
occurred_at
```

Examples:

```text
OPENING_FLOAT
CASH_SALE
CASH_REFUND
CASH_IN
CASH_OUT
CASH_ADJUSTMENT
```

## 28.3 Closing reconciliation

Expected cash is derived from immutable events.

The counted amount and variance are additional facts.

---

# 29. Payment Schema

Payments are external-trust-sensitive.

## 29.1 `payment_intents`

```text
payment_intent_id
organization_id
sale_id
provider
expected_amount
currency
status
client_reference
created_at
expires_at
```

The `expected_amount` is authoritative application state. The frontend cannot alter it merely by sending a different total.

## 29.2 `payment_attempts`

Track each provider interaction separately from the logical payment intent.

```text
payment_attempt_id
payment_intent_id
provider
request_reference
provider_attempt_reference
status
started_at
completed_at?
error_code?
```

## 29.3 `provider_payment_events`

```text
provider_event_id
organization_id?
provider
provider_event_external_id
payload_hash
received_at
verified_at?
processing_status
raw_payload_reference?
```

Critical constraint:

```text
UNIQUE(provider, provider_event_external_id)
```

If provider event IDs are absent, a deterministic provider-specific fingerprint must be designed and documented; payload hash alone is not always safe as identity.

---

# 30. Webhook Replay Protection

Webhook processing must obey:

```text
receive
 ↓
authenticate/signature verify
 ↓
validate schema
 ↓
persist/deduplicate event identity
 ↓
apply idempotent state transition
 ↓
commit
```

The unique database constraint is the final race-resistant deduplication mechanism.

Never implement:

```text
SELECT event WHERE provider_id = X
if absent:
    INSERT event
```

as the only protection. Two requests can pass the `SELECT` concurrently.

Use a unique constraint and handle the conflict transactionally.

---

# 31. Reconciliation Schema

Reconciliation is its own bounded context.

Tables:

```text
reconciliation_cases
reconciliation_matches
reconciliation_exceptions
provider_statements
provider_statement_items
```

A reconciliation case captures ambiguity rather than silently choosing a result.

Potential states:

```text
UNMATCHED
CANDIDATE
MATCHED
MISMATCH
MANUAL_REVIEW
RESOLVED
REJECTED
```

The database should preserve evidence for the decision.

---

# 32. Tax / MRA EIS Persistence

Tax integration state is stored separately from core sale truth.

Recommended tables:

```text
tax_configurations
tax_submission_records
tax_submission_attempts
eis_terminals
eis_configuration_snapshots
```

A tax submission failure must not roll back an already-valid internal sale merely because the external tax API was temporarily unavailable, unless the applicable legal and provider contract explicitly requires synchronous acceptance.

The existing architecture requires EIS failure handling to preserve the sale's financial facts and place tax submission into an explicit pending/retry/rejected state. fileciteturn9file8L566-L576

---

# 33. Customer Schema

Customer data is operationally useful but must remain minimized.

Possible fields:

```text
customer_id
organization_id
customer_reference
name?
phone?
email?
status
created_at
updated_at
```

Customer PII must not be replicated to every offline device unless necessary for the device's authorized workflow.

Search indexes and unique constraints must reflect the actual business requirement; phone number uniqueness cannot be assumed globally because real-world records can be shared, reused, mistyped, or absent.

---

# 34. Audit Schema

Audit records are durable evidence, not generic application logs.

Recommended `audit.events` structure:

```text
event_id
organization_id?
actor_id?
actor_type
service_identity?
action
resource_type
resource_id
result
reason_code
request_id
trace_id
command_id?
device_id?
approval_id?
provider_reference?
occurred_at
metadata_redacted
```

The audit event should contain enough context to reconstruct important operations without storing unnecessary secrets or raw sensitive payloads.

## 34.1 Audit immutability

Application roles should not have general-purpose UPDATE/DELETE permissions on historical audit data.

Correction of an audit interpretation should be represented through a new event, not destruction of the old event.

---

# 35. Approval Schema

Approvals must preserve separation of duties.

Tables:

```text
approval_requests
approval_decisions
```

Fields:

```text
approval_request_id
organization_id
request_type
resource_type
resource_id
requested_by
required_role
required_threshold
status
created_at
expires_at
```

Decision:

```text
approval_decision_id
approval_request_id
approver_id
decision
reason
created_at
```

Domain rules must prevent the requesting actor from approving their own restricted operation when segregation of duties applies.

---

# 36. Device and Offline Sync Persistence

Server-side tables should include:

```text
device_sync_states
sync_commands
sync_batches
sync_checkpoints
sync_rejections
```

## 36.1 `device_sync_states`

```text
device_id
organization_id
last_received_sequence?
last_acknowledged_command?
sync_schema_version
status
updated_at
```

## 36.2 `sync_commands`

```text
command_id
organization_id
device_id
actor_id?
schema_version
command_type
payload_hash
client_created_at
server_received_at
processing_status
processed_at?
rejection_code?
result_reference?
```

Constraint:

```text
UNIQUE(organization_id, command_id)
```

A device cannot use a reused command ID to create a new business effect.

## 36.3 Sync state and business truth

The sync tables remember delivery and command processing. They do not become a shadow inventory or financial ledger.

---

# 37. Offline Command Result Semantics

A command may result in:

```text
ACCEPTED
DUPLICATE_REPLAY
REJECTED_AUTHORIZATION
REJECTED_BUSINESS_RULE
REJECTED_SCHEMA
REJECTED_DEVICE_REVOKED
REJECTED_EXPIRED
CONFLICT
TEMPORARY_FAILURE
```

The server must preserve enough result identity for a retry after a timeout to distinguish:

```text
not processed
```

from:

```text
processed but acknowledgement lost
```

This is central to exactly-once-like user semantics even when the transport itself is at-least-once.

---

# 38. Outbox Schema

The transactional outbox is a core reliability primitive.

Suggested table:

```text
outbox.events
```

Fields:

```text
event_id
organization_id?
aggregate_type
aggregate_id
event_type
event_version
payload
created_at
available_at
attempt_count
processed_at
last_error_code?
lease_owner?
lease_until?
```

The event row is created in the same database transaction as the business state change.

Example:

```text
BEGIN
  sale inserted
  inventory movement inserted
  payment intent inserted
  audit inserted
  outbox event inserted
COMMIT
```

If the commit succeeds, the worker can later publish/send external effects.

If the commit fails, none of the authoritative state or outbox event becomes visible as committed business truth.

---

# 39. Inbox / Consumer Idempotency

For asynchronous internal or external events, consumers may need an inbox table.

```text
inbox.events
```

Fields:

```text
consumer_name
source_system
source_event_id
received_at
processed_at
status
payload_hash
```

Constraint:

```text
UNIQUE(consumer_name, source_system, source_event_id)
```

This is especially useful where multiple workers can receive the same event.

---

# 40. Reporting and Read Models

Reporting tables are explicitly derived.

Examples:

```text
reporting.daily_sales
reporting.stock_snapshots
reporting.payment_summary
reporting.branch_performance
```

Rules:

- cannot become authoritative financial state;
- must identify source versions/watermarks where necessary;
- must be rebuildable;
- must be tenant scoped;
- must be protected against cross-tenant leakage;
- large reporting queries must not compete directly with critical POS transactions unnecessarily.

The system architecture specifically prohibits treating analytics/reporting tables as a second financial source of truth. fileciteturn10file1L256-L280

---

# 41. Subscription and Billing Schema

Platform billing is separate from merchant operational finance.

Tables:

```text
plans
plan_features
subscriptions
subscription_events
entitlements
usage_counters
billing_accounts
billing_invoices
```

The domain must distinguish:

```text
PLAN
ENTITLEMENT
USAGE
BILLING STATE
```

A merchant's subscription state can restrict new feature use without corrupting historical sales/inventory data.

---

# 42. Integration Registry

External integrations should have durable configuration metadata separate from secrets.

Table:

```text
integrations
```

Potential fields:

```text
integration_id
organization_id
integration_type
provider
status
configuration_version
enabled_at
disabled_at
last_success_at
last_failure_at
```

Provider credentials belong in managed secret infrastructure, not ordinary business tables.

---

# 43. File/Object References

Sitolo should not store arbitrary filesystem paths in business records.

Use object references:

```text
object_id
storage_provider
bucket_namespace
object_key
content_type
size_bytes
checksum
status
created_at
expires_at?
```

The object key must not be interpreted as an authorization grant.

Every download operation re-evaluates tenant/object authorization.

---

# 44. Database Security Roles

At least these logical PostgreSQL identities should exist:

```text
migration_owner
app_runtime
reporting_readonly
backup_operator
break_glass
```

## `migration_owner`

Can perform schema migrations and DDL.

Must not be used by API request handlers.

## `app_runtime`

Can perform required DML only.

Should not be able to:

- create roles;
- create arbitrary extensions;
- alter schema;
- modify migration history outside controlled paths;
- bypass RLS unnecessarily.

## `reporting_readonly`

Read-only access to approved views/read models.

## `backup_operator`

Dedicated access for backup/restore operations.

## `break_glass`

Exceptional, MFA-protected, separately audited access with explicit operational procedure.

PostgreSQL RLS must not be considered effective against a runtime role that can bypass it. Current PostgreSQL documentation explicitly notes special roles such as superusers and `BYPASSRLS` roles, and the design therefore prohibits making the ordinary application runtime role such a role. citeturn760439search6

---

# 45. Row-Level Security Strategy

RLS is defense in depth, not a replacement for Rust authorization.

Use RLS selectively for high-risk tenant data where the operational model can support it safely.

Potential RLS candidates:

```text
sales
sale_items
inventory_movements
inventory_balances
payment_intents
payment events
customers
suppliers
audit events where merchant-scoped
```

## 45.1 Tenant context

RLS policies should derive tenant context from trusted transaction/session context established by the server.

Never treat a caller-provided HTTP `organization_id` as security proof.

## 45.2 Testing RLS

Tests must verify:

```text
tenant A -> tenant A rows = allowed
 tenant A -> tenant B rows = denied
 missing context -> denied
 revoked membership -> denied
```

Tests MUST execute against real PostgreSQL, not mocks, because application unit tests cannot prove database policy behavior.

---

# 46. RLS and Connection Pooling

RLS session context requires careful interaction with connection pooling.

If tenant context is stored in session parameters, it must be transaction-scoped or reliably reset before a pooled connection is reused.

Dangerous pattern:

```text
SET tenant_id = A
return connection to pool
next request sees tenant A
```

Safer patterns include:

- transaction-scoped settings;
- explicit tenant predicates in queries;
- robust connection reset hooks;
- server-side functions with strongly controlled parameters.

The implementation must have an automated test that proves connection reuse cannot leak tenant context.

---

# 47. Constraints as Security Controls

Constraints should be used aggressively for cheap, local invariants.

Examples:

```sql
CHECK (quantity <> 0)
CHECK (amount >= 0)
CHECK (valid_to IS NULL OR valid_to > valid_from)
UNIQUE (organization_id, sku_code)
FOREIGN KEY (...) REFERENCES ...
```

But constraints should not be abused for rules requiring arbitrary cross-row business logic where PostgreSQL `CHECK` semantics are not appropriate. PostgreSQL's documentation explains that `CHECK` constraints are designed around row-level expressions and should not be used as general cross-row enforcement mechanisms. citeturn760439search11

Where cross-row integrity is required, consider:

- unique constraints/indexes;
- exclusion constraints;
- foreign keys;
- transactional locking;
- application service orchestration;
- carefully designed triggers only when justified.

---

# 48. Unique Constraints Strategy

Unique constraints are not merely convenience indexes.

They protect:

```text
provider transaction identity
provider event identity
sale number
SKU code within tenant
barcode within tenant
idempotency key
membership per user/organization
active register session
business identifiers
```

The application must expect and correctly interpret uniqueness conflicts.

A unique violation is often evidence of a race or replay, not necessarily an internal server error.

---

# 49. Foreign Key Strategy

Foreign keys should exist for business relationships where deletion/update semantics are clear.

Default preference:

```text
ON DELETE RESTRICT
```

for historical business records.

Use:

```text
ON DELETE CASCADE
```

only where child data is intentionally lifecycle-bound and destroying the parent is guaranteed to be safe.

Do not cascade-delete financial history.

---

# 50. Deletion Rules

## Never hard-delete by default

Historical:

```text
sales
sale_items
inventory_movements
cash_events
payment_events
reconciliation evidence
audit events
```

must be retained according to applicable policy and not destroyed as an ordinary edit.

## Hard delete candidates

Potential candidates include:

```text
a failed draft with no external side effects
unclaimed temporary upload metadata
expired transient jobs
```

subject to retention and operational requirements.

The domain model must decide deletion semantics before implementation.

---

# 51. Temporal Data

Sitolo contains multiple kinds of time:

```text
occurred_at
client_created_at
server_received_at
posted_at
valid_from
valid_to
expires_at
created_at
updated_at
```

Do not collapse all of these into one timestamp.

For offline workflows especially:

```text
client_created_at != server_received_at
```

Both may be necessary evidence.

Server time governs authoritative security decisions; client time is evidence.

---

# 52. Time Zone Policy

Persist authoritative timestamps in UTC/timestamptz form.

Store organization/branch timezone as configuration.

Business-day reporting should calculate local business dates from the configured timezone rather than storing arbitrary local wall-clock strings in transactions.

A branch crossing midnight in UTC must not incorrectly shift sales to the wrong business day.

---

# 53. Currency and Money Schema Rules

For all financial tables:

```text
amount
currency
```

must be semantically tied.

Do not permit:

```text
amount without currency context
```

within domain transactions.

Potentially use:

```text
amount_minor BIGINT
currency_code CHAR(3)
```

when the supported currencies and minor-unit rules make this appropriate.

Alternatively:

```text
amount NUMERIC(p,s)
```

for exact decimal requirements.

The final implementation should benchmark and verify both correctness and ergonomics rather than using PostgreSQL `money` blindly.

---

# 54. Sale and Payment Consistency

A sale may exist before payment is settled.

Therefore:

```text
sale.status
payment.status
```

must be distinct.

Do not make the sale row mean both commercial completion and payment settlement.

For example:

```text
SALE = FINALIZED
PAYMENT = PENDING
```

can be valid.

This is especially important when a provider is asynchronous or temporarily unavailable.

---

# 55. Transaction Boundaries

The default principle is:

> **One business invariant that must be atomic belongs in one PostgreSQL transaction.**

Examples:

### Finalizing a sale

```text
BEGIN
  authenticate/authorize
  re-read authoritative product/price data
  lock/check inventory
  insert sale
  insert sale items
  insert inventory movements
  update inventory balance
  insert payment intent if required
  insert audit event
  insert outbox event
COMMIT
```

Do not hold the database transaction open while waiting on external payment or tax APIs.

External calls happen after the authoritative transaction or through durable workers.

---

# 56. Transaction Isolation Policy

Default isolation should normally remain PostgreSQL's standard `READ COMMITTED` where correct, because stronger isolation carries performance and serialization costs.

Use:

```text
row locking
SELECT ... FOR UPDATE
atomic conditional UPDATE
unique constraints
SERIALIZABLE where justified
```

for specific invariants.

Do not globally raise isolation simply because stronger sounds safer.

Document why a transaction requires stronger isolation.

---

# 57. Lock Ordering

Whenever multiple rows/entities must be locked, establish deterministic ordering.

Example:

```text
organization
 ↓
source location
 ↓
destination location
 ↓
sku
 ↓
lot
```

or another explicitly documented ordering appropriate to the aggregate.

The purpose is to reduce deadlock probability.

Deadlock handling must be explicit; never blindly retry every SQL error.

---

# 58. Concurrency Patterns

## Pattern A — Atomic conditional update

Best for stock decrement and simple counters.

## Pattern B — Row lock + validation + write

Best when several related values must be re-read consistently.

## Pattern C — Unique constraint as mutex

Best for idempotency/replay and singular active records.

## Pattern D — Serialization retry

Only for operations with safe retry semantics.

The database design must document the chosen strategy per high-risk aggregate rather than relying on one universal concurrency helper.

---

# 59. Deadlock Policy

When PostgreSQL detects deadlock:

1. transaction fails;
2. the application maps it to a known retryable database condition;
3. retry is bounded;
4. retry is only permitted if the command has explicit idempotency semantics and is otherwise safe.

Never:

```text
catch any database error
retry forever
```

---

# 60. Query Design Rules

Every SQL statement must answer:

```text
What tenant scope does it operate under?
What indexes support it?
What locks does it take?
What rows can it touch?
Can it become an unbounded query?
Can it leak data through counts or joins?
Can it be retried safely?
```

API route handlers do not contain ad-hoc SQL. Repositories/application services own SQL access.

The existing architecture explicitly prohibits direct SQL from API route handlers. fileciteturn10file1L256-L276

---

# 61. SQLx Integration

Rust uses SQLx for database access.

SQLx's compile-time query-checking model is a strong fit for Sitolo because SQL remains explicit while type mismatches can be caught during build/verification. The project documentation describes compile-time verification and a deliberate compatibility/MSRV model. fileciteturn9file6L431-L465

Rules:

- use parameter binding;
- never interpolate user values into SQL;
- prefer checked query macros where practical;
- keep migrations versioned;
- run compile-time verification in CI;
- keep `Cargo.lock` committed for the deployable application;
- do not bypass query checking to make CI green without documented reason.

---

# 62. Migration Architecture

Migrations are append-only change history.

Example:

```text
migrations/
  000001_initial.sql
  000002_add_branch_status.sql
  000003_add_inventory_lot.sql
```

Rules:

- migration names are descriptive;
- every migration is reviewed;
- migrations run deterministically;
- production applies them exactly once;
- destructive migrations require explicit approval;
- migration compatibility is tested against deployed application versions;
- schema changes that affect synchronization are versioned with the sync protocol.

---

# 63. Expand-and-Contract Migrations

For breaking schema changes:

```text
EXPAND
  ↓
write both / read new with fallback
  ↓
backfill
  ↓
verify
  ↓
switch reads/writes
  ↓
CONTRACT
```

Example:

```text
rename column
```

must not simply drop the old column while an older application binary may still run.

This is particularly important for mobile clients that cannot all upgrade simultaneously.

---

# 64. Mobile Compatibility and Schema Versioning

Server schema and client SQLite schemas evolve independently.

Database design must therefore preserve:

```text
API version
sync schema version
client app version
server DB schema version
```

A mobile client may remain offline for a long time, so the server must retain compatibility with supported command versions or provide a safe rejection/migration path.

---

# 65. Partitioning Strategy

Partitioning is **not enabled by default on every large table**.

It is introduced only when evidence shows that table size, retention, maintenance, vacuum, index growth, or query locality materially benefits.

PostgreSQL supports declarative partitioning, and current documentation describes partitioned tables and partition keys. citeturn760439search18

Likely candidates in the future:

```text
inventory_movements
sales
sale_items
payment events
audit events
outbox events
telemetry-like database records
```

Potential partition keys:

```text
occurred_at / posted_at by time
```

but tenant-aware indexing remains necessary.

Do not partition purely because “enterprise databases partition.”

---

# 66. Indexing Strategy

Indexes serve actual access patterns.

Every major table needs identified:

```text
primary lookup
tenant lookup
foreign-key lookup
common filter
time-range scan
uniqueness constraint
```

## Example: sales

Likely indexes:

```text
PRIMARY KEY(sale_id)
INDEX(organization_id, occurred_at DESC)
INDEX(organization_id, branch_id, occurred_at DESC)
INDEX(organization_id, sale_number)
INDEX(organization_id, seller_user_id, occurred_at DESC)
```

But indexes must be validated against actual query plans rather than added blindly.

---

# 67. Tenant-First Indexing

For shared-table multi-tenancy, many indexes should begin with `organization_id` when tenant filtering is present in normal queries.

Example:

```text
(organization_id, sku_code)
(organization_id, branch_id, occurred_at)
(organization_id, location_id, sku_id)
```

This helps authorization-filtered queries and reduces the amount of unrelated tenant data the planner must consider.

It also makes tenant-local uniqueness explicit.

---

# 68. Composite Index Design

Index ordering must follow common predicates.

Bad generic index:

```text
(sale_status)
```

for a table containing millions of rows where most states are the same.

Potentially better:

```text
(organization_id, sale_status, occurred_at DESC)
```

when the workload actually filters by tenant/status and sorts by time.

Use `EXPLAIN (ANALYZE, BUFFERS)` during performance verification.

---

# 69. Partial Indexes

Partial indexes are valuable for active-state lookups.

Example concept:

```sql
CREATE UNIQUE INDEX one_active_register_session
ON register_sessions (register_id)
WHERE status = 'OPEN';
```

The exact lifecycle states must be finalized before applying this pattern.

Partial indexes can encode a business invariant and reduce index size.

---

# 70. Covering / INCLUDE Indexes

Use `INCLUDE` columns only when a proven read pattern benefits from index-only scans and the additional index size is justified.

Do not optimize imaginary queries.

---

# 71. JSONB Indexing Policy

JSONB fields are not automatically indexed.

If an extension attribute becomes a high-frequency query predicate, it should either:

1. become a relational field;
2. receive a targeted JSONB index; or
3. move into a specialized projection.

GIN indexes can accelerate JSONB containment/search patterns, but they carry write and storage costs. PostgreSQL's current GIN documentation describes JSONB operator classes and indexing behavior. citeturn760439search26

---

# 72. Full-Text / Search Policy

Do not introduce Elasticsearch/OpenSearch before the product needs it.

Start with PostgreSQL search capabilities where sufficient.

For product lookup, prioritize:

```text
barcode exact match
SKU exact match
bounded text search
prefix search where supported
```

The POS path should not issue an expensive fuzzy search across millions of rows.

---

# 73. Pagination

Use bounded pagination.

Offset pagination is acceptable for small administrative pages but dangerous for large transactional histories.

Prefer keyset pagination for large datasets:

```text
occurred_at DESC, sale_id DESC
```

with a stable cursor.

The domain/API contract should define cursor semantics later.

---

# 74. Query Cost and Resource Limits

The database is an availability boundary.

Set or enforce:

```text
statement_timeout
lock_timeout
idle_in_transaction_session_timeout
connection pool maximum
transaction duration budget
```

Application request deadlines must be coordinated with DB timeouts.

The goal is:

```text
request deadline
  > DB operation deadline
  > lock acquisition deadline
```

not the reverse.

---

# 75. Connection Pooling

Rust application pools must be bounded.

A pool is a finite resource, not an infinite concurrency mechanism.

Define:

```text
max connections
min idle
acquire timeout
idle timeout
lifetime
health check policy
```

Avoid allowing report jobs to consume every database connection needed by POS traffic.

This supports the broader security architecture's bulkhead principle.

---

# 76. Workload Isolation

Logical workloads should not starve each other.

At minimum distinguish:

```text
transactional API
worker/outbox
reporting/export
maintenance
migration
```

Possible techniques:

- separate pools;
- separate DB roles;
- read replica later if earned;
- queue concurrency limits;
- expensive-report scheduling.

Do not create a read replica solely because it sounds scalable; benchmark first.

---

# 77. Backup Architecture

PostgreSQL backups are a production security requirement.

Required:

```text
automated backup
PITR where supported
encrypted backups
retention policy
separate backup identity
restore environment
restore verification
```

Backup credentials must not be the same as the application runtime identity.

The existing security architecture requires automated encrypted backups, separate backup access, restore testing, and explicit RPO/RTO. fileciteturn7file7L676-L685

---

# 78. Restore Verification

A backup is not proven useful because the provider says “backup successful.”

A restore exercise must verify:

```text
schema
migrations
constraints
indexes
RLS
roles
extensions
business counts
critical watermark continuity
outbox state
security controls
```

After restoration, external provider states must be reconciled because external systems may have advanced while the database was unavailable.

---

# 79. Database Disaster Recovery

Recovery sequence:

```text
DECLARE
 ↓
STOP unsafe writes if necessary
 ↓
IDENTIFY restore point
 ↓
RESTORE isolated copy
 ↓
VERIFY schema/integrity
 ↓
VERIFY critical business records
 ↓
RECONNECT / PROMOTE
 ↓
RECONCILE external systems
 ↓
RUN SECURITY TESTS
 ↓
RESUME NORMAL OPERATIONS
```

The DR process is part of implementation readiness, not a post-launch document.

---

# 80. Database Encryption

Encryption is layered:

```text
client -> TLS -> database
storage -> provider/database encryption
backups -> encrypted
secrets -> managed secret/KMS infrastructure
```

Database encryption at rest does not make an application role trustworthy.

An authorized runtime query can still expose data, which is why authorization and RLS remain required.

---

# 81. Sensitive Data Handling

Sensitive columns include potentially:

```text
identity identifiers
phone/email
payment references
provider metadata
tax identifiers
support evidence
security metadata
```

Where column-level encryption is used, key management remains outside PostgreSQL table data where practical.

Do not encrypt every field automatically; encryption can complicate indexing, querying, and operational recovery. Classify first.

---

# 82. Secrets Must Never Be Database Configuration

The database may contain configuration metadata such as:

```text
provider = AIRTEL
integration_status = ENABLED
```

It should not contain raw production API secrets in ordinary tables.

When a provider requires credential material, store it in the managed secret infrastructure and persist only an identifier/version reference in PostgreSQL.

---

# 83. Audit vs Operational Logs

Database audit records:

```text
who
what
when
where
result
why
reference
```

Application logs should contain:

```text
request_id
trace_id
service/module
operation
latency
error category
```

Do not use the database audit table as a high-volume debug log sink.

Do not use application logs as the canonical financial record.

---

# 84. Outbox Retention

Outbox events require a lifecycle.

Example:

```text
PENDING
PROCESSING
PROCESSED
FAILED_RETRYABLE
DEAD_LETTER
```

Processed events can eventually be archived/purged according to operational requirements once downstream durability and audit requirements are satisfied.

Do not delete unprocessed outbox records merely because the table is large.

---

# 85. Idempotency Record Retention

Idempotency records need a retention policy based on:

- command replay window;
- offline maximum command age;
- provider retry behavior;
- operational recovery windows.

The retention period must be longer than any window in which a legitimate duplicate could arrive.

---

# 86. Audit Retention

Audit retention is policy-driven and potentially regulatory.

Never choose an arbitrary short retention because audit tables are expensive.

Before deletion:

```text
legal retention
fraud investigation needs
customer contract
security operations
storage cost
```

must be evaluated.

---

# 87. Database Observability

Monitor at minimum:

```text
connections
connection saturation
query latency
lock waits
deadlocks
table growth
index growth
cache hit behavior
vacuum/analyze health
replication lag if used
transaction age
long-running queries
failed transactions
constraint failures
RLS denial patterns where observable
```

PostgreSQL exposes cumulative statistics for tables and indexes and other server activity, which should feed operational diagnostics. citeturn760439search32

---

# 88. Long-Running Transactions

Long transactions are dangerous because they can:

- hold locks;
- prevent vacuum progress;
- increase bloat;
- consume connections;
- amplify outage impact.

Do not perform external HTTP calls inside a database transaction.

Do not stream large exports while holding a transaction open unnecessarily.

---

# 89. Vacuum and Autovacuum

The database operations specification must tune autovacuum based on actual table write patterns.

High-churn tables such as:

```text
outbox
sync_commands
session state
provider events
```

may need table-specific tuning.

Do not globally disable autovacuum.

---

# 90. Sequence and Number Generation

Where human-readable numbers require sequences, use database-backed generation.

Do not implement:

```text
SELECT max(number) + 1
```

under concurrency.

That pattern races.

Accept that sequence gaps can occur unless the business explicitly requires gapless numbering; if gapless legal numbering is required, it needs a dedicated and carefully reviewed design rather than abusing ordinary sequences.

---

# 91. Trigger Policy

Triggers are permitted but restricted.

Good uses:

- simple invariant enforcement;
- metadata maintenance;
- audit evidence where failure behavior is well understood;
- narrow database-local rules that cannot safely be bypassed.

Bad uses:

- hiding major business workflows in PL/pgSQL;
- making external HTTP calls;
- silently mutating financial meaning;
- creating opaque side effects developers cannot see from Rust.

The business domain remains primarily implemented in Rust.

---

# 92. Stored Procedures / Functions

Stored functions may be used where PostgreSQL's transactional capabilities materially improve correctness, but the default is application-owned orchestration with explicit SQL.

Potential future use:

```text
high-contention atomic stock operations
specialized reporting functions
security-sensitive database utilities
```

Any privileged function must have carefully restricted `SECURITY DEFINER` semantics and safe `search_path` handling.

---

# 93. RLS Security Function Policy

Security-sensitive SQL helper functions must not accidentally execute under a more privileged identity than intended.

If `SECURITY DEFINER` is used:

- owner must be controlled;
- `search_path` must be fixed safely;
- arguments must be validated;
- privilege escalation through object replacement must be prevented;
- execute privileges must be explicit.

Prefer simpler patterns when possible.

---

# 94. Public Database Exposure

Production PostgreSQL must not be directly public on the Internet.

Required topology:

```text
Internet
  ↓
Edge / WAF
  ↓
Rust API
  ↓
Private network
  ↓
PostgreSQL
```

Firewall/security-group policy must permit database access only from approved application/worker networks.

This is an infrastructure requirement, but it is included here because database security is incomplete without network exposure controls.

---

# 95. Database Role Privilege Verification

CI/infrastructure checks must periodically verify:

```text
app_runtime cannot CREATE ROLE
app_runtime cannot ALTER TABLE
app_runtime cannot DROP TABLE
app_runtime cannot bypass RLS
reporting cannot INSERT financial state
backup identity cannot act as merchant application
```

The goal is not to trust an IaC file. The deployed PostgreSQL instance must be checked.

---

# 96. Tenant Isolation Negative Test Matrix

For every tenant-owned table family, test:

```text
Tenant A -> own row -> PASS
Tenant A -> Tenant B row ID -> DENY
Tenant A -> Tenant B row via relationship -> DENY
Tenant A -> forged organization_id -> DENY
Tenant A -> valid object + wrong branch -> DENY
Tenant A -> revoked membership -> DENY
Tenant A -> support role without scope -> DENY
Tenant A -> bulk export of B -> DENY
```

Tests must execute through the actual Rust repository layer and through real PostgreSQL.

---

# 97. Database-Level IDOR Resistance

The database should help make BOLA errors harder to express.

Example repository contract:

```text
find_sale(tenant_scope, sale_id)
```

not:

```text
find_sale(sale_id)
```

The query itself should contain tenant/scope predicates where appropriate.

RLS adds a second barrier for critical tables.

---

# 98. Mass Assignment Protection at DB Layer

Do not expose generic ORM/model update functions that allow every column to be updated.

Instead use explicit UPDATE statements:

```sql
UPDATE products
SET name = $1,
    description = $2
WHERE organization_id = $3
  AND product_id = $4;
```

Never allow fields such as:

```text
organization_id
created_by
approval_status
ledger_total
```

to be updated simply because they appear in a client DTO.

---

# 99. Financial Immutability Enforcement

Finalized sale and movement tables should have permission structures and application contracts that prevent normal UPDATE/DELETE operations.

Possible database strategy:

```text
runtime role
  INSERT allowed
  SELECT allowed
  UPDATE restricted
  DELETE restricted
```

For truly immutable tables, use append-only design plus narrowly controlled correction pathways.

Do not assume an `immutable = true` application flag is sufficient.

---

# 100. Financial Correction Relationships

Every correction should point back to the original fact.

Examples:

```text
sale_reversal.original_sale_id
return.original_sale_id
refund.original_sale_id
inventory_reversal.original_movement_id
```

This allows forensic reconstruction.

A correction must never orphan the original record merely because a UI action was deleted.

---

# 101. Inventory Ledger Rebuildability

The database must be capable of rebuilding `inventory_balances` from the movement ledger.

Operational procedure:

```text
snapshot/balance
      ↓
compare against ledger sum
      ↓
detect divergence
      ↓
rebuild projection
      ↓
verify
```

If the projection cannot be rebuilt, it should not be treated as a disposable cache; the architecture would need another source of truth. Therefore the ledger remains authoritative.

---

# 102. Sale Reconciliation Invariants

A finalized sale should satisfy:

```text
sum(line totals)
    = subtotal +/- documented discounts
    + tax
    = total
```

subject to the exact tax/rounding rules.

The application must calculate these values deterministically, while the database can enforce basic non-negativity and referential integrity.

---

# 103. Payment Reconciliation Invariants

A provider event must not directly mutate arbitrary sale totals.

Instead:

```text
provider event
  ↓
provider identity verified
  ↓
payment intent found
  ↓
amount/currency/reference validated
  ↓
state transition legal?
  ↓
payment state updated
  ↓
reconciliation/audit recorded
```

Unknown events become exceptions, not magical successes.

---

# 104. Cash Reconciliation Invariants

Register close must preserve:

```text
expected_cash
counted_cash
variance
actor
approval where required
```

The variance cannot simply overwrite expected cash.

---

# 105. Product Lifecycle and Historical References

An active product can become:

```text
DISCONTINUED
ARCHIVED
```

without deleting historical references.

Any foreign key to an historical SKU should normally use `RESTRICT` semantics so accidental deletion is impossible.

---

# 106. Pharmacy Data Extensions

Pharmacy-specific relational extensions should attach to core SKUs instead of polluting all products.

Potential tables:

```text
pharmacy_product_attributes
pharmacy_lot_controls
prescription_references
medicine_dispensing_records
recall_records
```

Fields may include:

```text
registration_reference
controlled_classification
prescription_required
storage_requirement
```

The exact legal requirements must be separately verified before production use. The database design must not claim regulatory compliance solely because these tables exist.

---

# 107. Agro-Dealer Extensions

Potential tables:

```text
agro_product_attributes
agro_formulations
agro_batch_controls
```

Core inventory still owns quantity/movement semantics; agro extensions own formulation and regulatory metadata.

---

# 108. Wholesale Extensions

Potential tables may support:

```text
customer_price_tiers
minimum_order_quantities
case_pack_rules
credit_terms_metadata
```

Credit terms metadata do not automatically make Sitolo a lender or credit provider.

---

# 109. Multi-Branch Data Model

Branch scoping must appear where operational ownership matters.

Examples:

```text
inventory location -> branch
register -> branch
cash session -> branch
sale -> branch
purchase order -> branch/receiving scope
```

Organization-wide objects remain organization scoped.

A branch-local sale must not become visible to another branch simply because both belong to the same organization unless the user's permission scope allows it.

---

# 110. Branch-Scoped Authorization and Database Predicates

A query such as:

```sql
SELECT *
FROM sales
WHERE organization_id = $tenant
```

is insufficient for a branch-restricted cashier.

It must also enforce:

```text
branch_id IN authorized branch scope
```

The database can support this with explicit predicates, RLS, or views/functions, but Rust authorization remains the primary decision mechanism.

---

# 111. Support and Break-Glass Data Access

Support users must not receive unrestricted access to merchant tables by default.

Preferred model:

```text
support request
  ↓
approved scope
  ↓
expiring access
  ↓
restricted query
  ↓
audit
  ↓
access expires
```

A support role that can directly modify sales or inventory would effectively be a hidden superuser and violate the security architecture.

---

# 112. Reporting Export Tables

Large exports should be asynchronous.

Use:

```text
export_jobs
export_files
```

`export_jobs`:

```text
export_job_id
organization_id
requested_by
report_type
scope_snapshot
status
created_at
started_at?
completed_at?
expires_at?
```

The scope must be captured so that a role change halfway through an asynchronous job does not silently expand privileges, while the job should also revalidate current permission according to policy.

---

# 113. Database Views

Views can simplify safe read access.

Good use:

```text
authorized_reporting_view
sale_summary_view
inventory_availability_view
```

For security-sensitive views, evaluate whether `security_invoker` semantics are appropriate where RLS/policy interaction matters. PostgreSQL's current view documentation explicitly distinguishes invoker and owner policy behavior in the presence of RLS. citeturn760439search17

Do not assume a view automatically preserves tenant isolation.

---

# 114. Materialized Views

Materialized views may be introduced for expensive reporting but are derived state.

Requirements:

- refresh strategy;
- tenant isolation;
- staleness semantics;
- rebuild process;
- failure recovery;
- no use for authoritative financial decisions unless the decision explicitly accepts staleness and the policy is documented.

---

# 115. Cache Boundary

If Redis is later used:

```text
PostgreSQL = authority
Redis = accelerator
```

Redis must never replace:

```text
sale truth
inventory truth
payment truth
authorization truth
```

Database writes succeed independently of Redis availability where business semantics require continuity.

---

# 116. Referential Integrity Around External Providers

Do not store provider state by replacing Sitolo state with provider text.

Example:

```text
payment_intent.internal status
payment_attempt.provider_status
provider_event.external status
```

Keep the internal domain state separate from raw provider state.

This preserves the ability to support multiple providers and provider behavior changes.

---

# 117. Raw Provider Payload Storage

Raw external payloads may be useful for forensic reconstruction, but they are sensitive.

Prefer:

```text
payload hash
selected normalized fields
secure object reference to raw payload
```

rather than storing unlimited raw provider documents directly in frequently queried tables.

Retention must be explicit.

---

# 118. Database Error Classification

Rust data-access code must classify database errors into stable categories:

```text
NOT_FOUND
UNIQUE_CONFLICT
FOREIGN_KEY_CONFLICT
CHECK_VIOLATION
SERIALIZATION_FAILURE
DEADLOCK
LOCK_TIMEOUT
STATEMENT_TIMEOUT
CONNECTION_FAILURE
PERMISSION_FAILURE
UNKNOWN
```

This enables safe retries and consistent API error behavior.

Never return raw PostgreSQL messages to clients.

---

# 119. Security Around Dynamic SQL

SQL must remain parameterized.

Dynamic identifiers are not parameter values, so sort fields/table names must come from an allowlist.

Example mapping:

```text
"name"       -> p.name
"created_at" -> p.created_at
"sku"        -> s.sku_code
```

Anything else is rejected.

This directly supports Sitolo's SQL injection security contract.

---

# 120. Search Injection

If a future search layer uses a query language:

```text
user search
  -> typed search AST
  -> validated internal representation
  -> controlled SQL/search query
```

Never accept arbitrary JSON as an operator tree and pass it directly into a search engine.

---

# 121. Import Staging Tables

CSV/spreadsheet imports must not directly write production tables.

Use:

```text
import_jobs
import_files
import_rows
import_errors
```

Flow:

```text
upload
 ↓
scan
 ↓
stage
 ↓
parse
 ↓
validate
 ↓
preview
 ↓
approve
 ↓
transactional commit
```

Imported organization IDs must never override trusted tenant scope.

---

# 122. Data Import Idempotency

An import should have:

```text
import_job_id
request hash
file checksum
organization scope
schema version
```

If the user accidentally retries the same import, the system should be able to detect or safely isolate duplicate execution.

---

# 123. Data Export Isolation

Exports use scoped SQL and should never run unrestricted `SELECT *` over tenant tables.

Every export query must define:

```text
tenant
branch
date bounds
maximum row count
projection
```

Sensitive columns are excluded unless the permission explicitly allows them.

---

# 124. CSV Injection Consideration

The database can store strings beginning with formula characters safely, but export code must escape spreadsheet-dangerous values according to the export format policy.

The database design should preserve the raw merchant value; output sanitization belongs at the export boundary.

---

# 125. Data Integrity Checks

Scheduled integrity jobs should test:

```text
inventory balance vs ledger
payment settlement vs intents/events
sale totals vs line totals
cash expected vs event sum
foreign key integrity
orphan records
outbox consistency
sync command uniqueness
audit reference integrity
```

These are detection controls, not replacements for transactional correctness.

---

# 126. Reconciliation Repair

When a derived balance is wrong:

```text
detect
 ↓
freeze affected mutation if required
 ↓
reconstruct from authoritative facts
 ↓
create correction record
 ↓
rebuild projection
 ↓
verify
 ↓
audit
```

Do not directly edit a balance and call the incident resolved without evidence.

---

# 127. Database Migration Test Matrix

Every significant migration must run against:

```text
empty database
current production-like database
representative large database
database with active tenant data
database with historical financial records
```

Test:

- forward migration;
- application compatibility;
- rollback/recovery strategy;
- indexes;
- constraints;
- RLS;
- performance impact;
- lock duration.

---

# 128. Migration Lock Safety

DDL can block critical traffic.

Before production:

```text
estimate lock requirements
estimate migration duration
test on representative data
```

For large changes:

```text
online/low-lock strategy
expand-and-contract
batched backfill
```

must be used where necessary.

---

# 129. Seed Data Policy

Seed data is environment-specific.

Development may have:

```text
demo organization
roles
permissions
sample catalogue
```

Production must never receive development credentials or fake privileged accounts accidentally.

Default credentials are prohibited.

---

# 130. Production Database Initialization

Initialization should be reproducible from:

```text
migration history
controlled seed/configuration
secret manager references
IaC
```

A production system should not depend on an engineer manually clicking through a database console to become functional.

---

# 131. Extensions Policy

Only approved PostgreSQL extensions may be installed.

Examples may include:

```text
uuid-related features
pgcrypto where justified
```

but every extension introduces:

```text
upgrade dependency
security surface
backup/restore implications
provider compatibility requirements
```

No arbitrary extension is allowed merely because it simplifies development.

---

# 132. Search Path Hardening

Do not depend on a mutable default `search_path` for security-sensitive code.

For privileged functions and migrations, define explicit schemas.

This reduces the risk of object shadowing and accidental execution against the wrong object.

---

# 133. Database Ownership Separation

Schema ownership and runtime execution are intentionally different.

```text
owner ≠ application runtime
migration owner ≠ API credential
backup identity ≠ application credential
```

This is a critical blast-radius control.

---

# 134. Connection TLS

Production application-to-PostgreSQL traffic must use TLS according to the hosting model.

Certificate verification must not be disabled merely to fix a local deployment issue.

Local development can use a deliberately different trust configuration, but it must never leak into production.

---

# 135. Database Authentication

Use modern password/identity authentication supported by the managed environment. Credentials are delivered through secret infrastructure.

Do not embed:

```text
postgres://user:password@host
```

in source code, documentation, images, or client bundles.

---

# 136. Database Connection String Policy

Connection strings should be assembled from secure runtime configuration or injected as a secret reference.

Logs must redact:

```text
password
credentials
TLS secrets
connection strings
```

A connection error log may include:

```text
host identifier
port class
error category
request/trace ID
```

but not the credential material.

---

# 137. Tenant Isolation Through Query Shape

The canonical repository query pattern is:

```text
repository method
  -> receives TrustedScope
  -> creates bounded SQL
  -> tenant predicate
  -> branch/object predicate
  -> operation-specific projection
```

The repository must make the secure path the easy path.

A generic unscoped `get_by_id()` function for tenant-owned data is prohibited unless it is private to an already-scoped repository implementation and cannot be called without trusted scope.

---

# 138. Database API Anti-Patterns

Prohibited:

```text
SELECT * from API repositories
client-supplied tenant authority
raw SQL string interpolation
max()+1 business numbers
unbounded list queries
transaction held across HTTP
catch-all retry loops
mutable finalized financial facts
cross-module table writes without contract
uncontrolled JSON business state
```

---

# 139. Query Projection Policy

Do not retrieve every column when the caller needs three.

Example:

```text
POS product lookup
  -> sku_id
  -> barcode
  -> name
  -> current price
  -> sale unit
```

not:

```text
SELECT *
```

Projection minimization reduces memory, latency and accidental sensitive-field exposure.

---

# 140. Database Security Test Harness

A database test harness must provision real PostgreSQL and run:

```text
tenant isolation tests
RLS tests
privilege tests
constraint tests
transaction tests
race tests
migration tests
restore tests
SQL injection regression tests
```

Recommended test fixture:

```text
TENANT_A
TENANT_B
OWNER_A
MANAGER_A
CASHIER_A
OWNER_B
CASHIER_B
BRANCH_A
BRANCH_B
DEVICE_A
DEVICE_B
SKU_X
SALE_A
PAYMENT_A
```

The tests must prove forbidden behavior, not merely successful behavior.

---

# 141. Concurrency Test Scenarios

Mandatory scenarios include:

### Last stock unit

```text
stock = 1
request A sells 1
request B sells 1
```

Expected:

```text
exactly one successful inventory consumption
exactly one failed/declined business operation
```

### Duplicate sale command

```text
same command ID twice concurrently
```

Expected:

```text
one financial effect
same result identity
```

### Duplicate provider event

```text
same provider event concurrently
```

Expected:

```text
one accepted event
one financial effect
```

### Concurrent register close

Expected:

```text
one valid close
```

---

# 142. Property-Based Database Testing

Generate sequences of domain operations and verify invariants.

Examples:

```text
purchase
purchase
sale
return
sale
transfer
adjustment
```

Then assert:

```text
ledger projection = expected balance
```

Generate duplicate commands and verify idempotency.

Generate transaction reorderings only where the domain declares operations independent/commutative.

---

# 143. Database Fuzzing Targets

Database-facing parsers and validators should be fuzzed for:

```text
malformed IDs
large quantities
extreme monetary values
invalid dates
invalid JSON extension fields
malformed provider payloads
large import rows
weird Unicode
oversized strings
```

The goal is to prevent parser/resource failures from becoming database integrity failures.

---

# 144. Security Regression Matrix

Every release affecting database behavior should execute at least:

```text
[ ] cross-tenant reads denied
[ ] cross-tenant writes denied
[ ] cross-branch access denied
[ ] revoked membership denied
[ ] revoked device denied
[ ] RLS bypass resistance
[ ] runtime role least privilege
[ ] SQL injection regression
[ ] mass assignment regression
[ ] duplicate provider event harmless
[ ] duplicate command harmless
[ ] sale atomicity
[ ] inventory atomicity
[ ] refund upper bound
[ ] financial immutability
[ ] migration safety
```

This complements the broader 48-control security matrix already established in the security architecture. fileciteturn7file7L711-L728

---

# 145. Performance Baselines

Before production release, benchmark representative workloads:

```text
POS sale finalization
inventory lookup
barcode lookup
stock decrement
sale history
payment reconciliation
webhook processing
branch report
export staging
sync batch processing
```

Measurements:

```text
p50
p95
p99
throughput
DB CPU
IO
lock wait
connection use
```

Tests should represent many small tenants rather than one artificial mega-tenant because tenant distribution affects index behavior and authorization query patterns. The system architecture already calls for realistic multi-tenant concurrency testing. fileciteturn10file9L1238-L1240

---

# 146. Large-Tenant Safety

The architecture must not assume every tenant remains tiny.

Define growth thresholds for:

```text
sales count
inventory movements
audit events
customers
products
branches
devices
```

At threshold:

```text
index review
partition evaluation
report isolation
archival strategy
query optimization
```

Do not prematurely create separate database infrastructure for every tenant.

---

# 147. Tenant Lifecycle

Organizations need explicit states:

```text
TRIAL
ACTIVE
SUSPENDED
CLOSING
CLOSED
```

Database rules must prevent dangerous writes according to state.

Example:

```text
SUSPENDED
 -> reporting allowed
 -> data export maybe allowed
 -> new sales denied according to billing policy
```

The exact commercial policy belongs to the billing/domain contract.

---

# 148. Data Residency and Regionalization

The schema should avoid encoding Malawi-only assumptions into generic column semantics.

Use:

```text
country_code
currency_code
timezone
localization identifiers
regulatory configuration version
```

Country-specific tables or configuration are used only where the domain actually differs.

This is essential for controlled African regional expansion.

---

# 149. Regulatory Data Separation

Tax, pharmacy and future regulated data may have stricter access/retention rules.

Do not expose these as generic `metadata` simply because the first implementation is small.

Keep regulated-domain tables clearly identifiable and independently reviewable.

---

# 150. Privacy by Design in the Database

For every new table containing personal data, document:

```text
purpose
minimum fields
lawful/business basis as determined by legal review
retention
access scope
export behavior
delete/anonymize behavior
audit requirements
```

Collection should be minimized from the schema design stage.

---

# 151. Anonymization / Deletion Strategy

Where legally required, deletion may conflict with financial audit needs.

Therefore separate:

```text
business fact identity
personally identifying attributes
```

This can allow selected PII to be removed or anonymized while preserving an immutable financial record.

The final implementation must be reviewed against applicable law and contractual requirements.

---

# 152. Backup Data Classification

Backups inherit the sensitivity of the database.

Therefore:

```text
backup access = privileged
backup copies = encrypted
backup URLs = never public
backup exports = audited
```

A backup cannot be used as an informal development dump.

---

# 153. Development Data Policy

Never copy production data into local development casually.

Prefer:

```text
synthetic fixtures
sanitized datasets
masked samples
```

Production snapshots used for debugging require explicit authorization, access controls, retention limits, and secure destruction.

---

# 154. Database Environment Separation

Recommended environments:

```text
LOCAL
CI
DEVELOPMENT
STAGING
SECURITY TEST
PRODUCTION
```

Each has separate credentials and databases.

CI must not receive production DB credentials merely to run integration tests.

---

# 155. Staging Realism

Staging should approximate production in:

```text
PostgreSQL major version
extensions
schema
RLS
roles
timeouts
index structure
migration behavior
```

Synthetic data is preferred.

---

# 156. Production Readiness Gate

Database production readiness requires:

```text
[ ] migration path tested
[ ] backup verified
[ ] restore verified
[ ] runtime role least privilege
[ ] RLS tests pass
[ ] tenant isolation tests pass
[ ] transaction tests pass
[ ] concurrency tests pass
[ ] statement/lock timeouts configured
[ ] monitoring configured
[ ] alerts configured
[ ] schema ownership separated
[ ] public DB exposure denied
[ ] secret rotation tested
[ ] rollback/recovery procedure tested
```

---

# 157. Database Failure Modes

The database design must define behavior for:

```text
connection refused
connection pool exhaustion
statement timeout
lock timeout
deadlock
serialization failure
read replica lag
storage exhaustion
corrupt migration
backup failure
restore failure
RLS misconfiguration
credential expiry
```

Business semantics must distinguish:

```text
retry
reject
queue
fail closed
enter maintenance
```

---

# 158. Storage Exhaustion

Storage exhaustion is a correctness and availability threat.

Monitor:

```text
disk utilization
WAL growth
index/table bloat
log volume
large exports
outbox backlog
```

Set alerts before the database reaches emergency capacity.

---

# 159. WAL and Recovery Considerations

Infrastructure should size WAL/archival behavior for:

```text
POS bursts
large imports
bulk correction jobs
outbox traffic
maintenance
```

A poorly designed batch job must not consume the WAL/storage budget required for normal merchant operations.

---

# 160. Batch Operation Rules

Large operations must be chunked.

Examples:

```text
backfill 10,000,000 rows
```

should not be executed as one unbounded transaction.

Use:

```text
bounded batches
progress tracking
restartability
rate limiting
lock awareness
```

Financial state changes require additional care because batching may interact with business atomicity.

---

# 161. Reconciliation Jobs

Large reconciliation work must avoid blocking critical transactions.

Pattern:

```text
schedule
 ↓
read stable bounded range
 ↓
calculate
 ↓
produce exception/correction proposal
 ↓
approve if required
 ↓
apply bounded mutation
```

---

# 162. Reporting Isolation

If reports become materially expensive:

```text
transactional PostgreSQL
       │
       ├── read replica (earned later)
       └── warehouse (only if earned later)
```

But report correctness must always reference authoritative transactional state and documented freshness.

---

# 163. Materialized Inventory Balance Strategy

`inventory_balances` is a projection optimized for POS.

Possible recovery strategies:

```text
full ledger rebuild
incremental rebuild from checkpoint
snapshot + replay
```

The system must maintain enough information to execute at least one reliable rebuild strategy.

---

# 164. Consistency Checks Between Balance and Ledger

For a chosen scope:

```text
SUM(all applicable movement quantities)
= expected materialized balance
```

where reservation and state semantics are included correctly.

The checker must handle:

```text
lot states
returns
reversals
quarantine
transfers
opening balances
```

This is a domain-aware integrity check, not just `SUM(quantity)` blindly.

---

# 165. Database Events vs Domain Events

Not every database trigger event is a domain event.

A domain event should mean:

```text
something important happened in the business model
```

For example:

```text
SaleFinalized
PaymentConfirmed
InventoryAdjusted
```

A database `UPDATE` on `updated_at` is not a business event.

---

# 166. Event Versioning

Outbox and provider event schemas must contain versions.

Example:

```text
event_type = SaleFinalized
event_version = 1
```

Consumers must be able to reject unsupported versions safely rather than guessing.

---

# 167. Event Ordering

PostgreSQL commit order does not automatically provide global event ordering.

If a consumer needs ordering:

```text
aggregate sequence
```

or another explicit ordering mechanism must be stored.

Never infer business event ordering from timestamps alone.

---

# 168. Audit Correlation

Important business facts should link:

```text
command_id
request_id
trace_id
actor_id
device_id
sale/payment/inventory IDs
```

This supports the forensic chain defined by the security architecture. fileciteturn8file1L1704-L1744

---

# 169. Database Health Checks

Database readiness must verify what the application actually requires.

Separate:

```text
liveness
readiness
migration compatibility
```

A database connection that technically works does not mean the application schema is compatible.

Health endpoints must not expose:

```text
connection strings
credentials
schema dumps
internal hostnames
raw SQL errors
```

---

# 170. Startup Behavior

On startup:

```text
load safe config
 ↓
validate required secret references
 ↓
connect DB
 ↓
verify schema compatibility
 ↓
verify required extensions/features
 ↓
start workers
```

If critical schema/security prerequisites are missing, startup should fail rather than silently running in a degraded unsafe mode.

---

# 171. Schema Compatibility Guard

The application should verify an expected migration level or compatibility marker.

Example:

```text
application supports schema versions N through N+2
```

A schema newer than the application understands should trigger a controlled refusal or compatibility mode, not undefined behavior.

---

# 172. Transactional Outbox and External Calls

Never:

```text
BEGIN
  insert sale
  call payment provider
  call MRA
  insert success marker
COMMIT
```

The database transaction must not span unpredictable external networks.

Instead:

```text
COMMIT authoritative state + outbox
 ↓
worker performs external side effect
 ↓
record outcome
```

---

# 173. Payment Worker Database Pattern

Worker transaction:

```text
claim work
 ↓
read current authoritative state
 ↓
call provider outside DB transaction
 ↓
start transaction
 ↓
record provider result
 ↓
apply legal state transition
 ↓
write audit/outbox
COMMIT
```

Provider call retry semantics are governed by adapter/idempotency policy.

---

# 174. Tax Worker Database Pattern

Same principle:

```text
sale already committed
 ↓
tax submission job
 ↓
external call
 ↓
record attempt
 ↓
apply tax submission state
```

A rejected EIS submission never edits the sale to make it appear compliant.

---

# 175. Database Support for Incident Containment

Operational controls may require:

```text
freeze tenant mutations
freeze payment workflow
freeze a specific integration
disable exports
reject device sync
```

These controls should be represented by explicit state/feature flags with strong authorization rather than ad-hoc SQL changes.

---

# 176. Security Event Queryability

The database should make it possible to identify:

```text
cross-tenant access attempts
repeated authorization failures
payment mismatch cases
replay attempts
revoked-device synchronization
large exports
privileged changes
```

Do not put arbitrary high-cardinality text into indexes merely for detection.

Use structured fields and downstream telemetry where possible.

---

# 177. Database Logging Policy

Database query logging should be tuned to operational requirements.

Never enable broad logging of sensitive parameters in production merely to debug a feature.

Preferred:

```text
query timing
error category
slow query samples
statement fingerprints
```

rather than full sensitive SQL values.

---

# 178. Slow Query Review

Any query exceeding defined latency thresholds on critical paths should be reviewed for:

```text
missing tenant predicate
missing index
bad join cardinality
unbounded sort
unbounded report range
lock contention
connection contention
```

Security and performance are linked when an expensive query can be remotely triggered.

---

# 179. Denial-of-Service Through the Database

Threats include:

```text
huge report ranges
expensive search
large exports
mass import
lock contention
connection exhaustion
```

Controls:

```text
bounded filters
maximum date ranges
pagination
query timeouts
concurrency limits
async jobs
quotas
```

---

# 180. Data Access Layer Contract

Rust modules should generally expose repository interfaces such as:

```rust
trait SaleRepository { ... }
trait InventoryRepository { ... }
trait PaymentRepository { ... }
```

The exact trait strategy is implementation detail, but the conceptual contract is:

```text
Application Service
     ↓
Repository / DB Gateway
     ↓
SQLx
     ↓
PostgreSQL
```

Repositories should not contain business policy that belongs in domain/application services, but they must enforce safe query scoping.

---

# 181. Transaction Context Object

The application layer should make transaction ownership explicit.

Conceptually:

```text
UnitOfWork / TransactionContext
```

allows multiple repositories to participate in one transaction without opening independent transactions that can partially commit.

Example:

```text
SaleService
  -> transaction
  -> SaleRepository
  -> InventoryRepository
  -> AuditRepository
  -> OutboxRepository
  -> commit
```

---

# 182. Repository Restrictions

Repositories should not:

- open a transaction unexpectedly inside every function;
- call external providers;
- decide user permissions without security context;
- log secrets;
- expose raw database errors to HTTP;
- silently retry unsafe operations.

---

# 183. Database DTO Boundaries

Separate:

```text
API DTO
DB row
Domain entity
```

A database row must not automatically become a domain command object.

This reduces mass assignment and schema coupling.

---

# 184. Migration Naming and Review

Every migration PR should state:

```text
why schema changes
what tables/indexes change
expected lock behavior
backfill volume
backward compatibility
rollback/recovery strategy
security impact
performance impact
```

No migration gets merged merely because it runs locally.

---

# 185. Database Definition of Done

A database feature is complete when:

```text
[ ] domain owner identified
[ ] tenant scope identified
[ ] authoritative vs derived state identified
[ ] invariants documented
[ ] PK/FK/unique/check constraints defined
[ ] indexes justified
[ ] transaction boundary defined
[ ] concurrency behavior tested
[ ] RLS decision made
[ ] role permissions updated
[ ] migration written
[ ] rollback/recovery understood
[ ] SQLx queries verified
[ ] negative security tests exist
[ ] performance tested
[ ] observability added
[ ] retention defined
[ ] backup/restore impact assessed
```

---

# 186. Mandatory Database Acceptance Suite

Before Phase 0 can be considered complete, the database implementation plan must support:

### Integrity

```text
[ ] foreign keys
[ ] unique constraints
[ ] check constraints
[ ] exclusion constraints where required
[ ] financial immutability
[ ] inventory ledger consistency
```

### Security

```text
[ ] runtime least privilege
[ ] RLS verification
[ ] tenant isolation
[ ] branch isolation
[ ] secret separation
```

### Reliability

```text
[ ] transactions
[ ] idempotency
[ ] outbox
[ ] restart recovery
[ ] deadlock handling
```

### Performance

```text
[ ] indexed POS paths
[ ] bounded reports
[ ] connection pool
[ ] lock monitoring
[ ] query plans
```

### Recovery

```text
[ ] backup
[ ] restore
[ ] schema rebuild
[ ] projection rebuild
[ ] reconciliation after recovery
```

---

# 187. Database Readiness Matrix by Domain

| Domain | Authoritative tables | Critical invariant | Security boundary | Primary test |
|---|---|---|---|---|
| Identity | users, sessions, memberships | inactive identity cannot authorize | user/org | auth negative tests |
| Tenant | organizations, branches | hierarchy consistent | tenant | cross-tenant tests |
| Catalogue | products, skus | unique tenant identifiers | tenant | uniqueness tests |
| Pricing | price_versions | deterministic validity | tenant/role | overlap/concurrency tests |
| Procurement | POs, receipts | receipt only posts valid scope | tenant/branch | workflow tests |
| Inventory | movements, balances | no impossible stock transition | tenant/location | race tests |
| Sales | sales, items | finalized facts immutable | tenant/branch/register | atomic sale tests |
| Cash | sessions, cash_events | one valid close/session | branch/register | concurrent close tests |
| Payments | intents/events | duplicate events harmless | tenant/provider | webhook replay tests |
| Reconciliation | cases/matches | ambiguity preserved | tenant/finance | mismatch tests |
| Tax | submissions/attempts | external failure does not corrupt sale | tenant/EIS | failure tests |
| Audit | events | append-oriented evidence | restricted | immutability tests |
| Sync | commands/checkpoints | duplicate commands harmless | device/tenant | replay tests |
| Billing | subscriptions/entitlements | entitlement state deterministic | organization | lifecycle tests |
| Reporting | read models | rebuildable | tenant | leakage/performance tests |

---

# 188. Recommended Initial Migration Ordering

The first schema migration sequence should follow dependency order.

```text
001 extensions / foundational types
002 organizations
003 business entities
004 users
005 memberships
006 roles / permissions
007 branches / locations
008 warehouses / registers / devices
009 catalogue / units / SKUs / barcodes
010 pricing
011 suppliers / procurement
012 inventory lots
013 inventory movements / balances
014 sales / sale items
015 cash
016 payment intents / attempts / provider events
017 returns / refunds / reversals
018 reconciliation
019 tax/EIS
020 audit
021 idempotency
022 sync
023 outbox / inbox
024 reporting projections
025 billing / entitlements
026 support / platform administration
```

The exact migration numbering can change, but the dependency direction should not.

---

# 189. Database Documentation Required Alongside Migrations

The repository should contain:

```text
docs/database/
  data-dictionary.md
  relationship-model.md
  index-catalog.md
  privilege-model.md
  migration-policy.md
  backup-restore.md
  rls-policy.md
  query-performance.md
```

The schema itself is the implementation. Documentation explains decisions and operational boundaries.

---

# 190. Database Diagram — Logical Architecture

```text
                    ┌───────────────────────┐
                    │      ORGANIZATION     │
                    └───────────┬───────────┘
                                │
             ┌──────────────────┼──────────────────┐
             │                  │                  │
             ▼                  ▼                  ▼
        MEMBERSHIPS          BRANCHES          SETTINGS
             │                  │
             │          ┌───────┼───────────────┐
             │          │       │               │
             │          ▼       ▼               ▼
             │      LOCATIONS WAREHOUSES      REGISTERS
             │                                      │
             ▼                                      ▼
          USERS                                  DEVICES

   ┌────────────────────────────────────────────────────────┐
   │                   CORE RETAIL                          │
   │                                                        │
   │ PRODUCT → SKU → PRICING                               │
   │             │                                          │
   │             ▼                                          │
   │      PROCUREMENT → RECEIVING                           │
   │             │                                          │
   │             ▼                                          │
   │       INVENTORY LEDGER                                 │
   │             │                                          │
   │             ▼                                          │
   │          SALES                                         │
   │        /      \                                        │
   │       ▼        ▼                                       │
   │     CASH     PAYMENTS → RECONCILIATION                 │
   │                  │                                     │
   │                  ▼                                     │
   │                TAX/EIS                                 │
   └────────────────────────────────────────────────────────┘

             CROSS-CUTTING
       AUDIT / OUTBOX / SYNC / BILLING
```

---

# 191. Database Diagram — Sale Transaction

```text
CLIENT COMMAND
     │
     ▼
IDEMPOTENCY RECORD
     │
     ▼
┌──────────────────────────────────────────────┐
│             ONE DB TRANSACTION               │
│                                              │
│ SALE                                         │
│  ├── SALE ITEMS                              │
│  ├── PAYMENT INTENT                          │
│  ├── INVENTORY MOVEMENTS                     │
│  ├── INVENTORY BALANCE                       │
│  ├── AUDIT EVENT                             │
│  └── OUTBOX EVENT                             │
│                                              │
└──────────────────────────────────────────────┘
     │
     ▼
COMMIT
     │
     ├──────────────► SYNC ACK
     │
     └──────────────► WORKER
                         ├── PAYMENT
                         ├── EIS
                         └── NOTIFICATION
```

---

# 192. Database Diagram — Tenant Enforcement

```text
                 REQUEST
                    │
                    ▼
              AUTHENTICATED USER
                    │
                    ▼
                MEMBERSHIP
                    │
                    ▼
              TRUSTED TENANT SCOPE
                    │
          ┌─────────┴─────────┐
          ▼                   ▼
   RUST AUTHORIZATION     SQL TENANT PREDICATE
          │                   │
          └─────────┬─────────┘
                    ▼
               POSTGRESQL
                    │
                    ▼
                RLS DEFENSE
                    │
                    ▼
              AUTHORIZED ROWS
```

The desired property is defense in depth: a missed application predicate should still have a second barrier where RLS is deployed, while RLS itself is not treated as permission management.

---

# 193. Database Diagram — Offline Synchronization

```text
FLUTTER / TAURI SQLITE
       │
       │ command_id
       │ schema_version
       │ payload
       ▼
SYNC API
       │
       ▼
SYNC_COMMANDS
       │
       ├── duplicate? ────────► existing result
       │
       └── new
             │
             ▼
       DOMAIN TRANSACTION
             │
       ┌─────┴──────┐
       ▼            ▼
 authoritative   OUTBOX
 state           EVENT
       │            │
       └─────┬──────┘
             ▼
          COMMIT
             │
             ▼
       ACK + CHECKPOINT
```

This design ensures synchronization state does not become a second source of business truth.

---

# 194. Database Design Principles — Final Set

The database implementation MUST follow these principles:

1. PostgreSQL is authoritative.
2. Rust owns domain decisions.
3. Clients do not determine truth.
4. Tenant scope is explicit.
5. Tenant keys are present where they strengthen integrity and policy enforcement.
6. Constraints enforce cheap local invariants.
7. Transactions enforce atomic business outcomes.
8. Unique constraints enforce race-resistant identity.
9. RLS provides defense in depth.
10. Runtime roles are least privileged.
11. Financial facts are append-oriented.
12. Inventory is ledger-backed.
13. Derived balances are rebuildable.
14. Provider events are untrusted until verified.
15. Webhook identity is deduplicated by database constraint.
16. External calls do not occur inside DB transactions.
17. Outbox state is committed atomically with business changes.
18. Offline commands are idempotent and bounded.
19. Reporting data is derived.
20. Hard deletion is exceptional.
21. Production DB is private.
22. Secrets are outside ordinary tables wherever possible.
23. SQL remains explicit and parameterized.
24. Dynamic SQL is allowlisted.
25. Queries are bounded.
26. Connections are bounded.
27. Timeouts are mandatory.
28. Migrations are versioned and compatibility-aware.
29. Backups are restored, not merely reported successful.
30. Security properties are tested against real PostgreSQL.

---

# 195. Implementation Gate — Database Foundation

The database phase cannot be declared complete until the following are true:

```text
[ ] PostgreSQL target version pinned
[ ] local PostgreSQL reproducible
[ ] migrations reproducible
[ ] SQLx integration established
[ ] foundational schemas created
[ ] tenant hierarchy implemented
[ ] users/memberships implemented
[ ] scoped roles/permissions represented
[ ] catalogue schema implemented
[ ] inventory schema implemented
[ ] sale schema implemented
[ ] payment schema implemented
[ ] audit schema implemented
[ ] outbox schema implemented
[ ] idempotency schema implemented
[ ] sync persistence defined
[ ] runtime role least privilege verified
[ ] RLS policy set identified
[ ] tenant negative tests executable
[ ] transaction tests executable
[ ] concurrency tests executable
[ ] backup configured
[ ] restore tested
[ ] statement/lock timeout policy verified
[ ] production public exposure prohibited
```

---

# 196. What This Document Deliberately Does Not Decide Yet

Some decisions remain intentionally open because the existing architecture says they require evidence or external contracts.

These include:

- exact cloud provider;
- exact managed PostgreSQL service;
- exact identity provider;
- exact Redis necessity;
- exact reporting replica/warehouse threshold;
- exact payment provider data contract;
- exact MRA production certification/credential workflow;
- final pharmacy regulatory field set;
- exact enterprise dedicated-database isolation tier;
- precise production capacity thresholds.

These are not omissions. They are deliberate evidence-driven decisions. The existing system architecture explicitly identifies them as open decisions requiring prototype, provider, regulatory, load, or customer evidence. fileciteturn10file1L154-L175

---

# 197. Build vs Buy — Database Capabilities

Do not build:

```text
custom database engine
custom replication system
custom encryption primitive
custom SQL parser
custom database backup engine
```

Build/configure:

```text
schema
constraints
RLS policy
transaction boundaries
ledger semantics
inventory projections
idempotency semantics
outbox semantics
tenant predicates
domain-specific reconciliation
```

This follows the broader Sitolo build-vs-buy philosophy: mature infrastructure should be reused while Sitolo-owned engineering effort remains concentrated on the business-specific moat. fileciteturn9file7L482-L529

---

# 198. Database Anti-Patterns Explicitly Prohibited

```text
NO public production PostgreSQL.
NO application runtime as schema owner.
NO superuser application role.
NO BYPASSRLS runtime role.
NO production database credentials in Git.
NO production database credentials in clients.
NO financial truth in Redis.
NO report table as financial authority.
NO SELECT max(number)+1 for identifiers.
NO arbitrary DELETE from financial history.
NO direct tenant trust from request body.
NO unbounded report query.
NO unbounded transaction.
NO external HTTP inside DB transaction.
NO catch-all DB retry loop.
NO SELECT-then-INSERT replay protection without unique constraint.
NO unrestricted JSON business model.
NO raw user-controlled SQL fragments.
NO migration executed manually without recorded history.
NO restore procedure that has never been tested.
NO security decision based solely on ORM behavior.
```

---

# 199. Final Database Contract

The Sitolo database contract is:

> **PostgreSQL is the authoritative transactional memory of the business. Rust decides what operations mean; PostgreSQL guarantees durable relationships, constraints, transactions, concurrency boundaries and selected defense-in-depth access rules. Financial facts remain reconstructable. Inventory remains ledger-backed. Tenant scope remains explicit. External integrations remain separate from internal truth. Offline synchronization records delivery and intent without becoming a second authority. Derived data remains rebuildable. Database privileges remain least-privileged. Security properties remain machine-testable.**

The database is therefore part of Sitolo's product moat, not merely infrastructure. A merchant does not care whether a table is normalized to third normal form; they care that when two employees sell the last item, the stock is correct, when a network dies the sale is not lost, when a payment provider retries the webhook the customer is not charged twice, and when a financial record is corrected there is an explainable history.

That is what this schema is designed to preserve.

---

# 200. References and Standards

Primary external references consulted for this database design:

1. PostgreSQL 18 documentation — Data Types. citeturn760439search1
2. PostgreSQL 18 documentation — Data Definition / constraints and schema management. citeturn760439search11
3. PostgreSQL 18 documentation — Row Security Policies. citeturn760439search6
4. PostgreSQL 18 documentation — Partitioning. citeturn760439search18
5. PostgreSQL 18 documentation — JSON functions/operators. citeturn760439search4
6. PostgreSQL 18 documentation — GIN indexes. citeturn760439search26
7. PostgreSQL 18 documentation — Views and security-invoker behavior. citeturn760439search17
8. PostgreSQL 18 release documentation. citeturn760439search15
9. PostgreSQL statistics/monitoring documentation. citeturn760439search32
10. Existing Sitolo `domain_model.md`. fileciteturn12file0L79-L143
11. Existing Sitolo `system_architecture_design.md`. fileciteturn10file1L176-L254
12. Existing Sitolo `security_architecture_design.md`. fileciteturn8file1L1024-L1050

---

# Appendix A — Initial Core Table Inventory

```text
TENANT / ORG
organizations
business_entities
organization_settings
branches
locations
warehouses
registers

IDENTITY
users
memberships
roles
permissions
membership_roles
membership_scopes
sessions
devices

CATALOGUE
products
categories
brands
skus
units_of_measure
sku_units
barcodes

PRICING
price_lists
price_versions
promotions
promotion_rules
promotion_targets
discount_records

PROCUREMENT
suppliers
purchase_orders
purchase_order_items
goods_receipts
goods_receipt_items

INVENTORY
inventory_lots
inventory_movements
inventory_balances
inventory_transfers
inventory_transfer_items
stock_counts
stock_count_items
inventory_adjustments

SALES
sales
sale_items
returns
return_items
sale_reversals
refunds
refund_allocations

CASH
register_sessions
cash_events
cash_counts
cash_variances

PAYMENTS
payment_intents
payment_attempts
provider_payment_events
payment_allocations

RECONCILIATION
reconciliation_cases
reconciliation_matches
reconciliation_exceptions
provider_statements
provider_statement_items

TAX / EIS
tax_configurations
tax_submission_records
tax_submission_attempts
eis_terminals
eis_configuration_snapshots

AUDIT / SECURITY
audit.events
approval_requests
approval_decisions

SYNC
sync_commands
sync_batches
sync_checkpoints
sync_rejections
device_sync_states

INTEGRATION
integrations
inbox.events
outbox.events

REPORTING
reporting read models
export_jobs
export_files

BILLING
plans
plan_features
subscriptions
subscription_events
entitlements
usage_counters
billing_accounts
billing_invoices
```

This is the **logical inventory**, not permission to implement every table on day one. Release scope remains staged.

---

# Appendix B — Initial Critical Constraints

```text
organizations.slug                         UNIQUE
memberships(organization_id,user_id)       UNIQUE
skus(organization_id,sku_code)             UNIQUE
barcodes(organization_id,barcode)          UNIQUE
purchase_orders(organization_id,number)    UNIQUE
sales(organization_id,sale_number)         UNIQUE
sale_items(sale_id,line_number)            UNIQUE
provider_events(provider,event_id)         UNIQUE
idempotency(organization_id,key,scope)     UNIQUE
sync_commands(organization_id,command_id)  UNIQUE
```

Plus:

```text
non-negative amounts where applicable
positive conversion factors
valid temporal windows
valid quantity semantics
required tenant IDs
required foreign keys
no impossible state values
```

---

# Appendix C — Initial Security/Database Crosswalk

| Security requirement | Database control |
|---|---|
| Exposed DB credentials | managed secrets + private DB + rotation |
| Public `.env` | deployment policy outside DB + artifact checks |
| Hardcoded secrets | secret scanning + no secret columns |
| Missing authorization | scoped repository + RLS where applicable |
| Cross-user access | object predicates |
| Cross-tenant access | organization keys + RLS + negative tests |
| Open DB permissions | role separation |
| Cloud misconfiguration | private network/IaC |
| Admin route | DB admin role isolation |
| Logs leak secrets | parameter/log redaction |
| SQL injection | SQLx bind parameters |
| Race conditions | transactions/locks/unique constraints |
| Webhook replay | provider event unique identity |
| Offline replay | command ID uniqueness |
| Business logic abuse | state/invariant enforcement |
| Checks fail open | CI migration/security tests |
| Missing timeouts | DB statement/lock timeout |
| Insecure endpoints | controlled query scope |

---

# Appendix D — Database Release Gate

A candidate release touching persistence cannot be certified until:

```text
SCHEMA
  migrations pass

SECURITY
  tenant isolation passes
  RLS tests pass
  role privilege tests pass

INTEGRITY
  constraints pass
  transaction tests pass

CONCURRENCY
  race tests pass
  duplicate tests pass

RELIABILITY
  outbox tests pass
  sync persistence tests pass

PERFORMANCE
  critical query budgets pass

RECOVERY
  backup verified
  restore verified

OPERATIONS
  monitoring active
  alerts active
  rollback/recovery documented
```

A passing unit-test suite without these controls is not evidence that the database is production-safe.

---

# Appendix E — Implementation Checklist

```text
PHASE 0
[ ] Review against domain_model.md
[ ] Review against security_implementation_spec.md
[ ] Finalize identifier policy
[ ] Finalize money representation
[ ] Finalize timestamp policy
[ ] Finalize schema boundaries
[ ] Finalize RLS strategy
[ ] Finalize runtime DB roles
[ ] Finalize migration policy

FOUNDATION
[ ] PostgreSQL 18.x environment
[ ] SQLx migrations
[ ] baseline schemas
[ ] role provisioning
[ ] connection TLS
[ ] connection pool
[ ] timeout policy

TENANCY
[ ] organizations
[ ] branches
[ ] locations
[ ] scoped foreign keys
[ ] tenant indexes
[ ] RLS

DOMAIN
[ ] catalogue
[ ] pricing
[ ] procurement
[ ] inventory
[ ] sales
[ ] cash
[ ] payments
[ ] reconciliation
[ ] tax
[ ] audit

RELIABILITY
[ ] idempotency
[ ] sync commands
[ ] outbox
[ ] inbox
[ ] retry state

TESTING
[ ] migration tests
[ ] tenant tests
[ ] RLS tests
[ ] role tests
[ ] transaction tests
[ ] race tests
[ ] replay tests
[ ] integrity checks

OPERATIONS
[ ] backups
[ ] PITR
[ ] restore drill
[ ] metrics
[ ] alerts
[ ] storage monitoring
[ ] query monitoring
```

---

# Appendix F — Ownership

| Database area | Owner |
|---|---|
| Tenant tables | Tenant/platform domain |
| Identity tables | Identity/platform |
| AuthZ tables | Security/platform |
| Catalogue | Catalogue domain |
| Pricing | Pricing domain |
| Procurement | Procurement domain |
| Inventory | Inventory domain |
| Sales | Sales/POS domain |
| Cash | Cash domain |
| Payments | Payments/integrations |
| Reconciliation | Finance/integrations |
| Tax/EIS | Tax/integrations |
| Audit | Security/platform |
| Sync | Offline/platform |
| Outbox/inbox | Platform/infrastructure |
| Reporting | Reporting/analytics |
| Billing | Billing/platform |
| Database infrastructure | SRE/platform |
| Backup/restore | SRE/platform |
| Migration process | Platform/DB engineering |

Ownership does not mean isolated decision-making. Cross-tenant, financial, identity, synchronization, and security changes require cross-cutting review.

---

# Appendix G — Final Non-Negotiable Database Rules

```text
1. PostgreSQL is authoritative.
2. Financial truth is append-oriented.
3. Inventory truth is ledger-backed.
4. Tenant scope is never inferred from client input alone.
5. Runtime DB credentials are least privileged.
6. RLS is defense in depth.
7. Every critical replay boundary has a database uniqueness constraint.
8. Every critical business mutation has an explicit transaction boundary.
9. No external API call occurs inside a DB transaction.
10. No financial correction is a destructive edit.
11. No production database is publicly exposed.
12. No raw database credentials enter code, clients or logs.
13. No arbitrary SQL interpolation.
14. No unbounded query on user-controlled input.
15. No uncontrolled report workload can starve POS transactions.
16. No migration without compatibility analysis.
17. No backup without restore testing.
18. No RLS without real PostgreSQL tests.
19. No database authorization claim without negative tests.
20. No “enterprise” database feature is adopted without a measured reason.
```

**End of `database_design.md`.**
