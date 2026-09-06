# SITOLO
## System Architecture Design — Enterprise Target

**Positioning:** Business Operating System for African SMEs
**Primary market:** Malawi first; architecture designed for eventual African regionalization
**Primary clients:** Flutter mobile application; Tauri desktop application; limited TypeScript for web/admin and tooling
**Backend:** Rust-first
**Primary persistence:** PostgreSQL server; SQLite local operational store
**Architecture stance:** Modular monolith first, durable workers and adapters, selective service extraction only when justified
**Document status:** Architecture target and implementation blueprint
**Prepared:** 2026-09-03

> This document turns the existing Sitolo product/domain/business-model specifications into a concrete technical architecture. It is intentionally detailed enough to guide implementation, code review, security review, testing, operations, migration, and future scaling. It does not assume that every target capability must ship on day one.

# 0. Architecture Executive Decision

Sitolo should be engineered as a Rust-first business operating system with a deliberately small number of runtime boundaries. The default deployment is a single Rust application exposing a versioned API, backed by PostgreSQL, plus asynchronous workers in the same repository and deployment family. Flutter is the principal mobile client and Tauri is the desktop client. TypeScript is deliberately constrained to the places where a web ecosystem provides disproportionate leverage: web/admin UI, API client generation, developer tooling, documentation, and selected build/configuration tasks.

The central architectural decision is not 'Rust versus another language'. The central decision is that Sitolo owns business truth in a strongly typed, transactional domain core and treats clients, payment providers, tax systems, notification providers, and future financial partners as untrusted or failure-prone edges. This creates a stable center and flexible boundaries.

The platform must survive real merchant conditions: unreliable networks, low-end devices, power loss, duplicate callbacks, user mistakes, staff fraud, device replacement, asynchronous tax submission, provider outages, partial imports, and branch growth. These conditions shape the architecture more than theoretical internet-scale throughput.

## The target topology

```text
                             SITOLO PLATFORM

   ┌──────────────────────┐        ┌───────────────────────┐
   │ Flutter Mobile       │        │ Tauri Desktop         │
   │ Android-first        │        │ Back-office / POS     │
   │ SQLite local store   │        │ Local cache + device  │
   │ Offline operation    │        │ integration           │
   └──────────┬───────────┘        └──────────┬────────────┘
              │ HTTPS / sync commands                     │
              └──────────────────┬────────────────────────┘
                                 ▼
                  ┌──────────────────────────────┐
                  │ Rust API / Application Core  │
                  │ Axum + Tokio                 │
                  │ Auth / Tenant / Domain       │
                  │ Commands / Queries / Sync     │
                  │ Audit / Policy / Idempotency  │
                  └──────────────┬───────────────┘
                                 │
             ┌───────────────────┼───────────────────┐
             ▼                   ▼                   ▼
     ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐
     │ PostgreSQL     │  │ Worker Runtime │  │ Integration      │
     │ system of     │  │ jobs/outbox    │  │ adapters         │
     │ record        │  │ reconciliation │  │ MRA / payments   │
     └────────────────┘  └───────┬────────┘  └──────────────────┘
                                  │
                          ┌───────┴────────┐
                          ▼                ▼
                       Redis          Object Storage
                  (optional/cache)    (receipts/imports)
```

## The most important rule

The clients never become the authority merely because they are offline. A client may temporarily become the authority for its own local queue of user commands so that work can continue, but server reconciliation must validate every command against authoritative tenant state and invariants. Offline operation means 'continue safely', not 'bypass the server forever'.

# 1. Relationship to Existing Sitolo Documents

This architecture derives from the existing `sitolo.md` enterprise product specification and `business_model_design.md`. The existing specification already establishes: mobile-first operation, Rust/Axum/Tokio, PostgreSQL, modular-monolith discipline, tenant isolation, append-only financial events, durable synchronization, payment-provider boundaries, MRA EIS integration, pharmacy and agro-dealer extensions, auditability, observability, backups, and disaster recovery. This document does not replace those decisions; it makes them implementable.

| Existing decision | Architecture consequence |

| --- | --- |

| Dedicated mobile app | Flutter application with a durable SQLite operational database and offline command queue |

| Desktop back office | Tauri application sharing domain API contracts but optimized for larger workflows |

| Rust backend | Rust owns authorization, domain logic, transaction orchestration, integrations, jobs and audit |

| Little TypeScript | TS is limited to web/admin and tooling, not core transaction processing |

| PostgreSQL | Server-side source of truth with constraints, transactions, RLS where appropriate and append-only history |

| Offline-first | Client command/event model, deterministic sync, idempotency, checkpoints, conflict states and recovery |

| Financial append-only | Immutable business events and compensating actions rather than destructive edits |

| Provider adapters | Payment/EIS/SMS/email/storage integrations behind ports/adapters |

| Modular monolith | Code boundaries first; network boundaries later |

| Enterprise controls | Audit, policy, entitlements, data governance, SLOs, DR and operational tooling from the architectural baseline |

## What this document deliberately does not do

- It does not require twenty microservices because the product sounds enterprise-grade.

- It does not mandate a custom database engine, custom message broker, custom identity provider, or custom object store.

- It does not make TypeScript a second backend language.

- It does not turn Sitolo into a wallet, lender, insurer, or general ledger merely because the architecture can integrate with one later.

- It does not claim MRA, pharmacy, payments, privacy, or financial-services compliance without certification and legal/operational evidence.

# 2. Architecture Drivers

| Driver | Why it matters | Architecture response |

| --- | --- | --- |

| Intermittent connectivity | Normal merchant work cannot depend on always-on internet | Local-first writes, durable queues, eventual synchronization |

| Low-end Android devices | Memory, CPU, storage and battery are constrained | Small local database, paginated queries, bounded queues, efficient payloads |

| Financial correctness | Wrong stock/cash/payment records directly damage trust | Strong domain invariants, ACID transactions, idempotency, append-only corrections |

| Multi-tenancy | One tenant must never access another tenant | Tenant context, authorization policy, DB constraints/RLS, negative tests |

| Rapid product expansion | New verticals must reuse the retail core | Modular domain architecture and extension modules |

| External dependency failure | Payments and tax systems can fail independently | Adapters, outbox/inbox, retries, timeouts, circuit breaking, reconciliation |

| Merchant simplicity | Users are not buying a distributed-systems course | Complexity hidden behind simple commands and operational UI |

| Scale economics | Infrastructure cost must not erase SaaS margin | Modular monolith, measured capacity, async processing, indexed SQL |

| Regulatory change | Tax and privacy obligations can change faster than app releases | Versioned integration adapters, configuration snapshots and audit evidence |

# 3. Technology Strategy

## 3.1 Rust baseline

As of 20 August 2026, Rust 1.98.0 is the latest stable release listed by the official Rust release announcements. Sitolo should pin an explicit stable toolchain in repository configuration and CI rather than building against whatever compiler happens to be installed on a developer machine. citeturn244519search0turn244519search3

The repository should treat the compiler, Cargo lockfiles and crate versions as part of the reproducible build boundary. Upgrade cadence should be deliberate: security fixes and critical compiler/toolchain fixes can be expedited; routine stable adoption can occur through an upgrade PR with full CI and performance regression checks.

## 3.2 Rust ecosystem roles

| Technology | Role | Decision |

| --- | --- | --- |

| Tokio | Async runtime | Use as the primary runtime for network I/O, timers, jobs and bounded concurrency |

| Axum | HTTP/API framework | Use for versioned API routes, middleware and request extraction |

| SQLx | Database access | Use with PostgreSQL for explicit SQL, compile-time query checking where practical and migration control |

| Serde | Serialization | Use for API and internal wire formats |

| tracing + tracing-subscriber | Observability | Structured logs, spans, request correlation and job correlation |

| thiserror / anyhow | Error handling | Typed domain/application errors; `anyhow` only at application/edge orchestration where context is more important than public error taxonomy |

| uuid / ulid | Identifiers | Prefer sortable IDs where useful for event streams; UUIDs remain acceptable where semantics favor them |

| time | Date/time | Use explicit timezone-aware timestamps; avoid local-time ambiguity in persisted events |

| argon2 | Password hashing where passwords are used | Use memory-hard password hashing and delegated identity where possible |

| jsonwebtoken or JOSE-capable equivalent | Token verification only if required | Prefer standards-based verification and short-lived access tokens; do not build a bespoke token scheme |

SQLx is a deliberate choice for the core system because SQL remains a transparent part of the architecture. The project can use migrations and compile-time checked queries rather than hiding critical financial semantics behind a high-level ORM. SQLx's documentation also describes an explicit MSRV policy and compile-time verification model, which fits a locked CI toolchain. citeturn244519search1turn244519search2

## 3.3 Flutter client

Flutter is the primary mobile client because it supports a cross-platform architecture while compiling release applications to native machine code. Flutter's current architecture guidance also supports persistent SQL storage for complex offline data and an explicit offline-first application pattern. citeturn734608search5turn734608search12turn734608search15

The Flutter application should be organized around feature modules rather than a page-centric folder tree. Local persistence is SQL-backed; UI state is derived from repositories and use cases; synchronization is a separate subsystem. The UI should never directly issue arbitrary SQL or raw HTTP calls.

## 3.4 Tauri desktop

Tauri is appropriate for the desktop application because the desktop shell can combine Rust capabilities with a webview-based interface and does not ship a separate JavaScript runtime as an embedded VM. Tauri's documented architecture explicitly supports Rust and JavaScript/TypeScript APIs bridged through message passing. citeturn734608search4

The desktop application should not duplicate the Rust business engine. It should remain a client of the same authoritative API, while using Tauri plugins/native integrations only where desktop hardware or operating-system features are needed: printing, filesystem import/export, scanners, serial devices, local backup utilities, and secure OS credential storage.

## 3.5 TypeScript boundary

TypeScript is allowed, but it is a controlled secondary language. It belongs in:

1. A web/admin surface where React/Next.js or another mature web stack provides better ecosystem leverage.
2. Generated API clients and shared schema tooling when this reduces contract drift.
3. Developer tooling, code generation, OpenAPI linting, documentation, or release scripts.
4. Tauri UI code where TypeScript is the natural webview language.

TypeScript should not become the second implementation of pricing, stock, financial reconciliation, tenant authorization, payment state machines or tax logic. Duplicating domain logic across Rust and TS is an architectural defect because the two implementations will diverge.

# 4. Reuse Versus Build-From-Scratch Strategy

Sitolo's moat is not a custom implementation of commodity infrastructure. The moat is the correctly modeled business system and its ability to operate reliably in the target environment. Therefore the architecture follows a strict reuse rule: buy, adopt, or integrate for commodity primitives; build only where the implementation itself is part of Sitolo's differentiating capability.

| Capability | Reuse/adopt | Build custom | Reason |

| --- | --- | --- | --- |

| HTTP server/runtime | Axum/Tokio | No | Commodity; mature ecosystem |

| Relational database | PostgreSQL | No | Core transactional truth; do not invent a database |

| Client SQL engine | SQLite | No | Mature embedded database; ideal local persistence |

| Desktop shell | Tauri | No | OS/webview integration is not the product moat |

| Authentication | Standards/provider + Rust authorization layer | Partly | Identity authentication can be delegated; authorization and tenant policy are Sitolo-specific |

| Password hashing | Argon2 or managed identity provider | No custom algorithm | Cryptography is not a product differentiator |

| TLS | Platform/library implementation | No custom crypto | Never invent transport cryptography |

| Push notifications | FCM/APNs or provider abstraction | Adapter only | Commodity delivery |

| Email/SMS | Provider API adapters | Adapter only | External transport boundary |

| Object storage | S3-compatible/cloud storage | Adapter only | Commodity durable blob storage |

| Metrics/tracing | OpenTelemetry-compatible ecosystem | Instrumentation custom | Use standards; define Sitolo semantic dimensions |

| POS domain | No external reuse assumed | Yes | This is core product value |

| Inventory ledger | No external reuse assumed | Yes | Core correctness and workflow semantics |

| Reconciliation engine | No external reuse assumed | Yes | Strategic moat and business control loop |

| Offline sync protocol | No generic sync product assumed | Yes, but standards-aware | This must match Sitolo commands, invariants and business semantics |

| Tax/EIS integration | Official MRA API | Adapter custom | External contract; never reimplement authority |

| Payment integrations | Provider contracts/APIs | Adapter custom | Provider variability is an edge, not the core domain |

## Reuse rule in practice

A custom component must answer one of three questions before it is approved: (a) does it encode a Sitolo business invariant that third-party software cannot know, (b) does it solve the Malawi/Africa operating environment in a differentiated way, or (c) does it provide a control that is too security-sensitive to delegate safely? Otherwise prefer an established component.

# 5. System Context

```text
                     External actors / systems

      Merchant ───────┐
      Cashier ────────┤
      Manager ────────┤
      Owner ──────────┤
      Sitolo Support ─┤
                      ▼
                ┌───────────┐
                │  SITOLO   │
                │  PLATFORM │
                └─────┬─────┘
                      │
       ┌──────────────┼────────────────────┐
       ▼              ▼                    ▼
 Payment providers   MRA EIS         Notifications
 Airtel / TNM /      Tax authority   SMS / email / push
 banks / future APIs
       │              │                    │
       └──────────────┼────────────────────┘
                      ▼
                Storage / analytics
                / enterprise APIs
```

## Actors and trust boundaries

| Actor | Trust level | Allowed authority |

| --- | --- | --- |

| Merchant owner | Authenticated human | Tenant-scoped actions allowed by role/policy |

| Cashier | Authenticated human | Operational sales and permitted register actions |

| Manager | Authenticated human | Approval/report/inventory controls according to role |

| Sitolo support agent | Privileged operator | Only through explicit support mode, scoped tenant access and audit |

| Mobile client | Compromisable client | Never trusted for authorization or final server invariants |

| Desktop client | Compromisable client | Same trust model as mobile |

| Payment provider | External dependency | Untrusted external facts requiring signature/verification/reconciliation |

| MRA EIS | External authority | Authoritative for EIS acceptance/state; Sitolo records integration evidence |

| Object storage | Infrastructure dependency | Blob storage only; application authorization remains in Rust |

# 6. Domain Architecture

The domain is split into modules with explicit ownership of invariants. The codebase is a modular monolith, but modules must behave as if they could later become services. That means domain modules expose commands, queries and domain events instead of reaching into each other's tables indiscriminately.

```text
┌──────────────────────────────────────────────────────────────────────┐
│                         PLATFORM / CONTROL PLANE                     │
│ Identity · Tenancy · Policy · Entitlements · Billing · Audit        │
└─────────────────────────────────┬────────────────────────────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────────┐
        ▼                         ▼                             ▼
┌──────────────┐         ┌────────────────┐            ┌────────────────┐
│ Catalogue    │◄────────│ Inventory      │───────────►│ Procurement    │
└──────┬───────┘         └───────┬────────┘            └───────┬────────┘
       │                         │                              │
       ▼                         ▼                              ▼
┌──────────────┐         ┌────────────────┐            ┌────────────────┐
│ Pricing      │         │ POS / Sales    │            │ Suppliers      │
└──────┬───────┘         └───────┬────────┘            └────────────────┘
       │                         │
       └──────────────┬──────────┘
                      ▼
              ┌────────────────┐
              │ Payments /     │
              │ Cash / Ledger  │
              └───────┬────────┘
                      ▼
              ┌────────────────┐
              │ Reconciliation │
              └───────┬────────┘
                      │
       ┌──────────────┼─────────────────┐
       ▼              ▼                 ▼
    Tax/EIS        Reporting        Notifications
       │              │                 │
       └──────────────┼─────────────────┘
                      ▼
                 Audit / History

Vertical modules (Pharmacy, Agro-dealer, Wholesale, etc.) extend the shared core
rather than fork it.
```

## 6.1 Module list

| Module | Core responsibility | Must own |

| --- | --- | --- |

| identity | Authentication/session/device identity | User identity references, session/device state |

| tenant | Organizations, legal entities, branches, locations, terminals | Tenant hierarchy and tenancy invariants |

| authorization | Roles, permissions, scopes, policy decisions | Server-side access decisions |

| catalogue | Products, SKUs, units, tax metadata, categories | Product master semantics |

| pricing | Price lists, discounts, promotions, pricing rules | Effective price selection |

| procurement | Suppliers, POs, receipts, supplier invoices | Purchasing state transitions |

| inventory | Stock ledger, lots, locations, counts, transfers, adjustments | Quantity and stock movement invariants |

| sales | Carts, sales, returns, voids, refunds | Sale lifecycle and financial linkage |

| cash | Registers, shifts, cash movements, close | Cash control and end-of-day state |

| payments | Payment intents, tender records, provider references | Payment facts and states |

| reconciliation | Expected vs observed money and exceptions | Matching, exception state, evidence |

| tax | Tax calculations and EIS integration state | Tax treatment and external submission state |

| customers | Optional customer profiles and receivables metadata | Minimal customer data |

| reporting | Read models, aggregates, report jobs | Derived views only; never business truth |

| notifications | Email/SMS/push/in-app notifications | Delivery orchestration and retry state |

| audit | Immutable audit trail and access evidence | Who/what/when/why records |

| billing | Subscriptions, plans, entitlements, usage | Commercial entitlement state |

| platform_admin | Sitolo operator controls | Support, incidents, tenant ops, feature flags |

| vertical_pharmacy | Batch/expiry/controlled workflows | Regulated product constraints |

| vertical_agro | Agro-dealer metadata and workflows | Vertical-specific rules |

# 7. Repository and Code Organization

```text
sitolo/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── sitolo-domain/            # pure domain types/invariants
│   ├── sitolo-application/       # commands, queries, orchestration
│   ├── sitolo-api/               # Axum routes, DTOs, middleware
│   ├── sitolo-persistence/       # SQLx repositories and migrations
│   ├── sitolo-sync/              # sync protocol and reconciliation
│   ├── sitolo-integrations/      # payment/EIS/provider adapters
│   ├── sitolo-jobs/              # worker execution
│   ├── sitolo-observability/     # tracing/metrics conventions
│   ├── sitolo-auth/              # auth/session/device integration
│   └── sitolo-cli/               # admin/dev operational CLI if needed
├── migrations/
├── apps/
│   ├── mobile/                   # Flutter
│   ├── desktop/                  # Tauri + TS UI
│   └── admin-web/                # optional Next.js/React + TS
├── packages/
│   ├── api-types/                # generated, not hand-authored truth
│   └── tooling/
├── infra/
├── docs/
└── tests/
```

The exact crate split can start smaller. Do not create a crate for every noun. The critical rule is dependency direction. Domain code must not depend on Axum, SQLx, Redis, HTTP client implementations, or Flutter. Application code may depend on domain and abstract ports. Infrastructure implements those ports. API adapts requests into application commands. This prevents framework details from leaking into business logic and makes future extraction possible without a rewrite.

## Dependency direction

```text
          Framework / infrastructure
                    │
                    ▼
             ┌─────────────┐
             │     API     │
             └──────┬──────┘
                    ▼
             ┌─────────────┐
             │ Application │
             └──────┬──────┘
                    ▼
             ┌─────────────┐
             │   Domain    │
             └─────────────┘
                    ▲
                    │
       ┌────────────┴────────────┐
       │                         │
 Persistence adapters      Integration adapters
```

# 8. Tenant and Business Hierarchy

```text
Platform
  └── Organization / Account
        ├── Legal Business Entity
        │     └── Tax profile / registration / billing identity
        ├── Branches
        │     ├── Locations / warehouses
        │     ├── Registers / terminals / devices
        │     └── Staff assignments
        ├── Suppliers
        ├── Customers
        └── Business configuration / vertical modules
```

The architecture separates the platform tenant from a legal business entity because the commercial account and the legal/tax identity are not always the same concept. A growing group may operate multiple branches under one organization, while different legal entities may exist under a broader customer relationship in later enterprise tiers. The first implementation may map these one-to-one operationally, but the data model should not bake the assumption into primary keys.

## Tenant-scoped identifiers

Every protected business record should carry either an explicit `tenant_id`/`organization_id` or be reachable through a foreign-key path that makes its tenant unambiguous and enforceable. High-risk tables should prefer direct tenant ownership even when logically derivable because it enables database constraints, faster authorization checks, partitioning options later, and simpler forensic queries.

## Support access

Sitolo support must use a separate privileged access path. A support user does not simply receive a merchant role. Support access should be time-bounded, purpose-bound, audited, and visibly disclosed to the customer/operator where commercially appropriate. Read access and mutation access should be separate permissions. Emergency access must produce a high-severity audit record and should be reviewable after the incident.

# 9. Identity, Sessions, Devices and Authorization

Identity is authentication. Authorization is business policy. The architecture must not collapse them into one concept. A valid user token says 'this credential represents this principal'; it does not say 'this principal can refund a sale for branch B' or 'this principal can export tenant data'.

## 9.1 Authentication

Prefer standards-based authentication and a managed identity provider where the commercial and data-residency constraints make that acceptable. The Rust backend validates the resulting identity assertion and maps it to a Sitolo principal. A custom username/password store should only be implemented if there is a compelling product requirement. Never invent password hashing, MFA, recovery tokens, or cryptographic protocols.

## 9.2 Session model

```text
Access token / session
        │
        ├── principal_id
        ├── tenant context (selected, not trusted)
        ├── device_id
        ├── issued_at / expires_at
        ├── session_id
        └── auth strength / assurance

Backend loads effective permissions from authoritative state.
A stale or revoked session is rejected even if a token is otherwise well-formed.
```

Selected tenant and branch identifiers supplied by a client are always treated as selectors, never proof of membership. The server resolves the principal's effective memberships before allowing any tenant-scoped operation.

## 9.3 Device identity

Offline operation requires a durable device identity. Device identity is not equivalent to user identity. A shop may have several staff users on one device, one user may operate multiple devices, and a device may be lost or transferred. The sync layer therefore uses a stable installation/device identifier plus a key pair or secure credential where practical. Device registration, revocation and replacement are explicit workflows.

## 9.4 Authorization model

Use scoped RBAC as the baseline, with policy predicates for higher-risk workflows. Example scopes: `sales:create`, `sales:void`, `sales:refund`, `inventory:adjust`, `inventory:transfer`, `purchase:approve`, `cash:close`, `payment:reconcile`, `user:manage`, `report:export`, `tax:configure`, `support:impersonate_read`, and `support:impersonate_write`.

Policy evaluation should consider principal, tenant, branch, role, resource, action, resource state and sometimes approval requirements. The server should return an explicit policy decision and a reason code that is safe for logs and UI. Do not expose internal authorization policies as a mutable client-side rules file.

## Approval as a first-class concept

High-risk operations can enter `PENDING_APPROVAL` rather than executing immediately. Approval records must identify requester, approver, action, target, original values, reason, timestamp and resulting state. Requester and approver separation is preferred for fraud-sensitive actions where staffing permits.

# 10. Database Architecture

PostgreSQL is the server's system of record. SQL is part of the design, not an implementation detail hidden behind an ORM. Transactions are used to preserve invariants across sale, inventory, cash and payment records. The schema uses constraints aggressively, but constraints do not replace authorization logic.

## 10.1 Schema domains

```text
identity_*
tenant_*
auth_*
catalog_*
pricing_*
procurement_*
inventory_*
sales_*
cash_*
payment_*
reconciliation_*
tax_*
customer_*
reporting_*
audit_*
billing_*
notification_*
platform_*
vertical_pharmacy_*
vertical_agro_*
```

Whether these become PostgreSQL schemas or simply table prefixes is an implementation choice. Separate PostgreSQL schemas can help organization, but a single logical database with strong naming and ownership rules is often sufficient early. Do not use separate databases merely to create the illusion of modularity unless the operational benefit is proven.

## 10.2 Primary key strategy

Use globally unique identifiers for entities that cross device/server boundaries. IDs must be generated so offline clients can create records without asking the server for a sequence number. UUIDv7 or another sortable UUID family is a strong default. For human-facing document numbers—receipts, invoice numbers, register numbers—use a separate business-number sequence because human numbering semantics differ from database identity semantics.

## 10.3 Monetary representation

Never represent business money as floating point. Persist amounts as integer minor units plus an explicit currency code, or as exact numeric values where a fixed precision is required for a specific external contract. All arithmetic must be deterministic and auditable. Tax rounding policy must be explicit and tested with edge cases.

## 10.4 Time

Persist event timestamps in UTC with an explicit type. Business-local time is resolved from the tenant/branch timezone for display and reporting. The date of a sale, business day and tax period must not be inferred by converting UTC timestamps ad hoc across different code paths.

## 10.5 Constraints

- Foreign keys protect ownership and referential integrity.

- Unique constraints prevent duplicate business identifiers and idempotency keys.

- Check constraints enforce basic domain legality where SQL can do so safely.

- Partial indexes support active/current records without scanning historical data.

- Exclusion constraints are considered for temporal overlaps where appropriate.

- Triggers are used sparingly; business state transitions generally belong in application/domain code where they can be tested and explained.

# 11. Core Data Model

The following is a conceptual model. Exact columns should evolve through migrations after domain review, but the relationships should remain recognizable.

```text
organization
  │ 1..*
  ├──── business_entity
  │            │
  │            └──── tax_profile
  │
  ├──── branch ───── location/warehouse
  │        │             │
  │        └──── terminal/device
  │
  ├──── user_membership ─── role assignment
  │
  ├──── product ─── sku ─── unit/price/tax
  │                  │
  │                  └── inventory_item / lot
  │
  ├──── supplier ─── purchase_order ─── goods_receipt
  │
  ├──── customer
  │
  └──── sale ─── sale_line ─── payment_intent/payment
              │
              ├── inventory_posting
              ├── cash_posting
              ├── tax_submission
              └── reconciliation references
```

# 12. Inventory Architecture

Inventory is not a mutable `product.quantity` field pretending to be a ledger. The authoritative history is a sequence of stock movements. Current balances may be materialized for speed, but every balance can be explained by movements.

## 12.1 Inventory ledger

```text
RECEIPT        +Q
SALE           -Q
RETURN         +Q
TRANSFER_OUT   -Q source
TRANSFER_IN    +Q destination
ADJUSTMENT     ±Q
COUNT_RECON    ±Q
WRITE_OFF      -Q
RECALL         move to quarantine state
```

An inventory posting should identify tenant, location, SKU, quantity, unit of measure, lot/batch where applicable, source document, posting time and actor/device. A sale transaction that decrements stock should be tied to its sale ID. The posting is immutable after commit; correction means a compensating movement.

## 12.2 Optimistic and pessimistic locking

For high-frequency stock decrements, the system should use a transaction that validates the available quantity and updates a materialized balance or inserts a ledger movement atomically. Row-level locks or PostgreSQL atomic updates are preferred over application-side read-modify-write patterns. The exact choice depends on the final schema, but race-prone sequences such as SELECT quantity → compute → UPDATE quantity must not be used without a concurrency control mechanism.

## 12.3 Batch/lot and expiry

The pharmacy and agro-dealer extensions require inventory dimensions beyond SKU quantity. A stock layer may therefore be keyed on SKU + location + lot/batch + quality state. FEFO selection is a policy, not a generic SQL order-by hidden in the UI. Expired, quarantined, recalled or otherwise restricted lots must be excluded from normal sale allocation. Rules may differ by jurisdiction and vertical configuration.

# 13. Point of Sale Architecture

The POS must optimize for three things simultaneously: speed at the till, correctness of inventory/payment recording, and survivability through connectivity failure. The UI should therefore be command-oriented rather than API-form-oriented. A cashier performs 'complete sale', not 'POST sale header', 'POST line', 'POST payment', 'POST inventory'. The server owns the transaction boundary across those components.

## 13.1 Complete sale command

```text
CompleteSaleCommand
  sale_id / client_command_id
  branch_id
  register_id
  lines[]
  pricing_snapshot
  discounts[]
  taxes[]
  tenders[]
  customer_ref?
  device_id
  created_at_client

Server transaction:
  1. authenticate principal
  2. authorize sales:create
  3. validate tenant/branch/register
  4. validate catalogue/pricing state
  5. validate stock allocation
  6. validate payment/tender semantics
  7. create immutable sale
  8. post inventory movements
  9. post cash/payment records
 10. create tax/EIS pending work if required
 11. emit domain/outbox events
 12. commit
```

## 13.2 Why pricing snapshots matter

A sale must be reproducible from historical facts. If a product's price changes tomorrow, yesterday's receipt cannot silently recalculate from today's price table. The sale stores the effective price, discount and tax inputs used for the transaction. Product definitions and current price lists are configuration; a finalized sale is historical evidence.

# 14. Financial Event Model

Sitolo's financial core should be event-oriented without pretending to be a generic accounting ledger. Sales, returns, refunds, reversals, cash movements and reconciliation decisions are recorded as immutable events with explicit relationships.

```text
SALE_FINALIZED
   │
   ├── SALE_PAYMENT_RECORDED
   ├── INVENTORY_POSTED
   ├── CASH_POSTED (when applicable)
   └── TAX_SUBMISSION_PENDING

Correction:
SALE_FINALIZED
   ↓
RETURN / REFUND / VOID / REVERSAL
   ↓
CORRECTIVE POSTINGS

Never:
SALE_FINALIZED
   ↓
UPDATE sale.total = different value
```

This is the technical expression of the existing product rule that finalized financial records are not silently edited. The event history also creates the basis for dispute investigation, staff accountability and later analytics.

# 15. Payments and Reconciliation

Payments are intentionally divided into three concepts: expected tender, observed provider/bank evidence, and reconciliation decision. Conflating these causes silent financial errors. A cashier may say a customer paid via mobile money; that is an operational assertion. A provider callback or statement is an external observation. Reconciliation is the process that decides whether the two correspond.

## 15.1 Payment lifecycle

```text
CREATED
  ↓
PENDING_PROVIDER
  ├──── FAILED
  └──── PROVIDER_CONFIRMED
             ↓
          SETTLED/OBSERVED
             ↓
          RECONCILED
          ├──── MATCHED
          ├──── PARTIAL
          └──── EXCEPTION
```

## 15.2 Deterministic matching

The preferred reconciliation key is the provider transaction reference or an equivalent provider-issued identifier tied to a unique payment intent. Matching should be deterministic on strong identifiers first, then amount/currency/merchant account/time-window constraints. Heuristics such as fuzzy amount/time matching must never silently close an exception; they can propose a candidate match for a human or explicitly configured low-risk workflow.

## 15.3 Idempotency

Every provider webhook and every client-originated financial command must be idempotent. Store the provider event ID or a cryptographic digest plus provider namespace. For client commands, use a stable client command ID generated on the device. Retrying the same command must either return the original result or produce a semantically identical no-op, never a second sale or second payment.

# 16. Offline-First Architecture

Offline is a first-class execution mode. Flutter's architecture guidance describes offline-first systems as relying on local persistent state and explicit synchronization strategies. Sitolo extends that concept with business-specific command semantics because generic cache synchronization is not sufficient for financial transactions. citeturn734608search15

## 16.1 Local database

```text
SQLite on device
├── local_product_snapshot
├── local_price_snapshot
├── local_stock_view
├── draft_cart
├── committed_local_sale
├── outbox_command
├── inbox_ack
├── sync_checkpoint
├── provider_callback_cache (where applicable)
├── device_state
└── local_audit_evidence
```

The local database is not merely an HTTP cache. It is a durable operational store that lets the merchant continue working after process termination or network loss. However, only the minimum required dataset should be cached to minimize exposure if the device is lost or compromised.

## 16.2 Command log

Every offline mutation becomes a durable command with a deterministic identifier and schema version. Example states: `QUEUED`, `SENDING`, `ACKED`, `REJECTED_PERMANENT`, `RETRYABLE`, `CONFLICT`, `STALE_REQUIRES_REFRESH`. The command payload is immutable; retry metadata is mutable.

## 16.3 Sync protocol

```text
Client                              Server
  │                                    │
  │ POST /sync/push(batch commands)    │
  ├───────────────────────────────────►│
  │                                    │ validate/auth/idempotency
  │                                    │ apply in transactions
  │                                    │ generate acknowledgements
  │◄───────────────────────────────────┤
  │ acks + conflicts + new checkpoint │
  │                                    │
  │ GET /sync/pull?checkpoint=...      │
  ├───────────────────────────────────►│
  │◄───────────────────────────────────┤
  │ authoritative changes / snapshots  │
  │                                    │
```

## 16.4 Conflict policy

Not every conflict should be 'last writer wins'. Product names can often converge with a server preference; stock decrements cannot. Pricing edits may be versioned; financial corrections must become separate events. The sync engine should classify mutations into conflict classes: commutative, mergeable, reject-and-refresh, approval-required, and server-authoritative. This classification belongs in the domain model rather than the transport layer.

| Mutation | Offline allowed? | Conflict strategy |

| --- | --- | --- |

| Sale | Yes, within policy/threshold | Validate stock/rules on ingestion; preserve original command; reject or hold if invariant cannot be satisfied |

| Product name edit | Usually | Version check or server-preference merge |

| Price change | Maybe | Effective timestamp/version; manager approval if required |

| Inventory adjustment | Yes for authorized roles | Versioned adjustment; approval for high-risk values |

| User role change | No or restricted | Server authoritative |

| Tenant deletion | No | Server authoritative |

| Cash close | Restricted | Requires authoritative sequence/state and explicit close workflow |

| Tax configuration | No | Server authoritative snapshot |

## 16.5 Offline thresholds

Some workflows need external authority and therefore may require operational limits while disconnected. MRA EIS itself documents offline transaction support and configurable thresholds such as maximum transaction age and cumulative amount. Sitolo should keep its own platform-level offline risk policy separate from the tax integration's rules. A local sale can be valid in Sitolo but later fail external tax submission; that is represented explicitly rather than retroactively deleting the sale. citeturn734608search6turn734608search7turn734608search16

# 17. MRA EIS Architecture

MRA's current EIS developer resources provide API documentation for POS integration. The official documentation describes terminals as POS systems that may run on Windows, Linux, Android, macOS or other platforms, and documents onboarding, configuration, sales and utility endpoints. It also explicitly documents offline transaction support. citeturn734608search0turn734608search1turn734608search3turn734608search6

## 17.1 Adapter boundary

```text
Sitolo Tax Domain
       │
       ▼
EisGateway trait
       │
       ▼
MraEisV1Adapter
       │
       ├── terminal acquisition/activation
       ├── configuration refresh
       ├── product/status interactions
       ├── sale submission
       ├── offline submission
       ├── response validation
       └── external error mapping
```

The tax module must not know the HTTP details of the MRA API. It knows the lifecycle of an external tax submission. The adapter knows endpoint shapes and authentication details. This allows the external contract to change without contaminating sales/inventory code.

## 17.2 Tax state machine

```text
LOCAL_SALE_COMMITTED
        ↓
TAX_SUBMISSION_PENDING
        ↓
SUBMITTING
   ├──── RETRYABLE_FAILURE ───► RETRY_SCHEDULED
   ├──── REJECTED ─────────────► TAX_EXCEPTION
   └──── ACCEPTED ─────────────► TAX_ACCEPTED
```

The MRA documentation states that online sales are submitted to MRA and that offline transactions are stored locally and uploaded when connectivity returns. It also documents a terminal-specific configuration model and offline receipt signing flow. Sitolo should implement those mechanics behind the adapter and retain enough evidence to prove what was sent, when, with which terminal/configuration snapshot, and what MRA returned. citeturn734608search7turn734608search9turn734608search10

## 17.3 Certification gate

MRA EIS integration is not considered production-compliant merely because a request succeeds in development. Terminal acquisition, product identification, certification and operational procedures must be validated against current MRA requirements before Sitolo markets the capability as compliant. The official documentation exposes product identifiers and terminal activation concepts that should be treated as release gates. citeturn734608search11turn734608search0

# 18. Payment Provider Adapter Architecture

```text
                 Payment Domain
                      │
             PaymentProvider trait
         ┌────────────┼────────────┐
         ▼            ▼            ▼
   Airtel adapter  TNM adapter  Bank adapter
         │            │            │
         └────────────┴────────────┘
                      │
                 provider events
                      ▼
               inbox/idempotency
                      ▼
                reconciliation
```

The adapter contract must normalize provider-specific concepts into a canonical internal model without pretending the providers are identical. Provider capabilities should be explicit: payment initiation, status lookup, webhook delivery, settlement statement, refund, reversal, merchant notification, etc. A provider that lacks a capability should fail predictably rather than being emulated with unsafe polling hacks.

## Provider secrets

Provider credentials must never live in mobile or desktop application bundles. Server-side integrations use secret storage or a managed secret system. Per-tenant credentials, when required, are encrypted at rest and access-controlled. Logs must redact tokens, signatures, account numbers and sensitive payloads. Webhook endpoints validate signatures before parsing or persisting business effects where the provider contract supports signatures.

# 19. Cash and Register Architecture

Cash is a physical control process, not simply another payment type. The cash module represents registers, shifts, opening float, cash sales, cash in/out, cash drops, counted cash and closing variances.

```text
REGISTER_OPEN
   ↓
SHIFT_ACTIVE
   ├── cash sale
   ├── cash in/out
   ├── cash drop
   └── corrections with permissions
   ↓
CLOSING_COUNT
   ↓
VARIANCE_REVIEW
   ├── CLOSED
   └── PENDING_APPROVAL / EXCEPTION
```

The end-of-day workflow should be derived from the business model: today's sales plus cash count plus mobile-money observations plus bank movements plus exceptions produces a close state. The architecture should make each component traceable rather than generating a single opaque 'day closed' record.

# 20. Procurement and Supplier Architecture

Procurement flows from intention to receipt to stock. A purchase order does not itself increase inventory. A goods receipt does. This distinction is critical for accurate stock and supplier performance.

```text
DRAFT PO → APPROVED PO → SENT → PARTIALLY RECEIVED → FULLY RECEIVED
                                  │
                                  └──── CANCELLED

GOODS RECEIPT
   ↓
inventory movements
   +
supplier document / cost evidence
   ↓
optional payment obligation
```

A receipt can partially fulfill a purchase order. Quantity, unit conversion, batch/lot and expiry data are validated at receipt. Supplier invoices and cash/bank payments are separate financial concepts, allowing later payable or reconciliation capabilities without corrupting the stock model.

# 21. Catalogue and Pricing

The catalogue is the reference layer for what can be sold, while the sale is the historical layer for what was actually sold. Products should support SKU/barcode identity, units of measure, categories, tax classification, status, supplier references, costing metadata, images where useful, and vertical-specific attributes without forcing every merchant to fill every field.

## 21.1 Unit of measure

Support explicit conversion graphs only where the business model justifies them. Example: one carton = 24 bottles. Conversions should be deterministic and versioned. If a merchant changes a pack relationship after inventory exists, do not reinterpret historic quantities; create a new effective conversion or explicit packaging SKU model.

## 21.2 Pricing

Pricing should resolve through a deterministic policy engine: branch/customer segment/channel/validity period/product rules → effective unit price. The policy engine returns the applied price and rule IDs so the sale can record the exact inputs. Promotion engines should be bounded in complexity; avoid a generic rules language until actual merchant requirements justify it.

# 22. Vertical Extensions

## 22.1 Pharmacy

Pharmacy support is a controlled extension over the common catalogue, inventory and sale domains. It should add batch/lot, expiry, controlled-stock indicators, authorization and potentially prescription metadata as required. It must not implement clinical decision support or professional pharmaceutical judgment. Existing product specification materials already identify FEFO/FIFO, lot/expiry and quarantine as relevant controls.

Architecture pattern:
- Core inventory remains authoritative for quantity.
- Pharmacy policy adds constraints on which stock can be sold.
- Pharmacy audit records link to the core sale/inventory IDs.
- Feature entitlements and regulatory configuration control activation.

## 22.2 Agro-dealer

Agro-dealer workflows reuse the same inventory engine but add metadata and seasonal/restricted-product policies. Lot/batch traceability is likely common with pharmacy but should not be duplicated; the shared stock model should represent lots and quality states generically, while vertical modules decide which metadata and sale constraints apply.

## 22.3 Wholesale

Wholesale adds larger order quantities, customer accounts, price lists, receivables, dispatch and branch/warehouse complexity. The architecture should not fork into a separate wholesale engine. The retail sale model should be able to represent an order-to-sale flow, while the wholesale module adds approval and fulfilment stages.

# 23. API Architecture

The public API is versioned and command-oriented. Internal module APIs are Rust function/trait boundaries, not HTTP calls. HTTP is reserved for actual network boundaries. This avoids turning the modular monolith into a distributed system inside one process.

## 23.1 API layers

```text
HTTP request
  ↓
Router / middleware
  ↓
Authentication context
  ↓
Authorization policy
  ↓
DTO validation
  ↓
Application command/query
  ↓
Domain services / entities
  ↓
Repository / transaction
  ↓
Outbox / response mapping
```

## 23.2 API versioning

Use explicit versioning for externally consumed APIs, e.g. `/api/v1/...`. Internal DTOs can change more rapidly. Public payloads should contain stable semantic identifiers, not database internals. Breaking changes require a version transition rather than silent mutation of existing contracts.

## 23.3 Idempotency headers

For high-impact POST operations, accept an idempotency key. The server stores tenant + principal + operation namespace + key + request fingerprint + resulting status/reference. Replaying the same key with a different body must fail rather than return the old result, because that pattern indicates client misuse or a potential attack.

## 23.4 Pagination

Use cursor/keyset pagination for large datasets. Offset pagination is acceptable for small administrative tables but becomes unstable and expensive for continuously changing sales/inventory history. Cursor tokens should not reveal database implementation details and should expire or be signed where necessary.

# 24. Eventing, Outbox and Jobs

Sitolo needs asynchronous work but does not need a Kafka cluster on day one. The recommended first implementation is a transactional outbox in PostgreSQL plus workers that claim and execute jobs. This guarantees that a domain transaction and the intent to publish/process a side effect can commit atomically.

```text
Business transaction
      │
      ├── domain rows
      └── outbox row
              │
           COMMIT
              │
      worker claims row
              │
      external side effect
              │
      mark complete / retry
```

## 24.1 Job categories

- Tax/EIS submission and retry.

- Payment status polling where contractually necessary.

- Provider statement import.

- Reconciliation candidate generation.

- Notification delivery.

- Report generation/export.

- Search/index refresh where used.

- Data export and retention jobs.

- Device/sync cleanup.

- Billing/metering aggregation.

## 24.2 Job safety

Every job must be retry-safe. Use a job ID, attempt count, next-run timestamp, lease/visibility timeout, error classification and dead-letter state. A job that cannot succeed because the input is permanently invalid must stop retrying and surface an operational exception. Never create infinite retries that hide business failures.

# 25. Redis: Use Sparingly

Redis can be useful for ephemeral caching, distributed rate limiting, short-lived locks, job coordination or session acceleration, but the product must remain correct if Redis is unavailable. PostgreSQL remains authoritative for business state. Never store the only copy of a sale, payment, authorization decision or audit record in Redis.

# 26. Object Storage

Use S3-compatible object storage for receipts, exported reports, migration files, images and other large blobs. Store only object metadata and authorization references in PostgreSQL. Object keys should be non-guessable and tenant-scoped. Signed URLs should be short-lived. Deletion/retention rules must be explicit.

# 27. Search and Analytics

Do not introduce Elasticsearch/OpenSearch or a warehouse before the workload justifies it. PostgreSQL indexes, materialized views, full-text search and reporting read models can serve the initial product. When report queries threaten transactional performance, create dedicated read models or replicas. When multi-tenant analytical volume genuinely outgrows PostgreSQL, move reporting data asynchronously into a warehouse while preserving PostgreSQL as operational truth.

# 28. Reporting Architecture

Reports are derived views, not alternate sources of truth. Every report should declare its source semantics and time zone. Financial reports use immutable event/posting data. Operational reports can use materialized balances. If a report uses eventually consistent data, the UI should state the data freshness where it matters.

## Report classes

| Class | Examples | Consistency target |

| --- | --- | --- |

| Operational | Current stock, low-stock alerts, today sales | Near-real-time |

| Financial control | Cash variance, payment reconciliation | Strong consistency / explicit as-of time |

| Management | Gross margin, branch performance | Near-real-time or scheduled |

| Historical | Monthly/quarterly trends | Stable derived dataset |

| Regulatory | Tax/EIS evidence/export | Exact source lineage |

| Audit | Who changed/approved/exported | Strong consistency and immutable history |

# 29. Audit and Forensics

Audit is not the same as application logs. Logs are operational telemetry; audit records are business evidence. The audit domain should capture actor, tenant, action, target, outcome, source device, request correlation ID, reason where required, before/after summary where legally and operationally appropriate, and timestamp.

## Audit immutability

Application roles should not be able to rewrite audit rows. The storage account or separate database role used for audit inserts should have restricted update/delete privileges. Retention and legal hold rules must be defined independently from ordinary product deletion workflows.

## Forensic correlation

```text
user
 ↓
session_id
 ↓
request_id / trace_id
 ↓
command_id
 ↓
transaction_id
 ↓
sale/payment/inventory IDs
 ↓
outbox event/job ID
 ↓
external provider reference
```

This chain is critical when a merchant disputes a payment or stock result. An engineer should be able to trace a user action through the API, database transaction, external side effect and final reconciliation state without guessing.

# 30. Security Architecture

Security is layered. Rust memory safety reduces an important class of implementation bugs, but it does not prevent authorization failures, insecure business logic, tenant leakage, replay attacks, data exposure, SSRF, injection, weak recovery flows, or fraud. The security model therefore combines language-level safety, application policy, database constraints, network controls, secrets management and operational detection.

## 30.1 Threat model

| Threat | Impact | Primary controls |

| --- | --- | --- |

| Cross-tenant read | Critical | Tenant context, policy checks, DB ownership/RLS, negative tests |

| Cross-tenant write | Critical | Server authorization, FK ownership, transaction checks |

| Duplicate offline sale | Critical | Client command IDs, idempotency store, unique constraints |

| Forged payment callback | Critical | Signature verification, provider event ID, replay checks |

| Stolen device | High | Device revocation, local data minimization, encryption at rest where supported, session controls |

| Privilege escalation | Critical | Central policy engine, explicit permission graph, no client-side authority |

| Malicious import | High | Schema validation, size limits, sandboxed parsers, staged import |

| Report data exfiltration | High | Scoped export permission, row filtering, rate limits, audit |

| Tax integration spoofing | Critical | Credential isolation, TLS, response validation, evidence logging |

| Supply-chain compromise | High | Dependency pinning, lockfiles, SBOM, `cargo audit`, CI review |

| Availability attack | High | Rate limiting, bounded concurrency, timeouts, quotas, backpressure |

## 30.2 OWASP-aligned controls

The Rust stack eliminates some memory-safety classes but not the OWASP application risks that matter most for Sitolo. Security review must concentrate on broken access control, cryptographic mistakes, injection, insecure design, authentication/recovery, software supply chain, logging/monitoring failures, SSRF where network fetches exist, and business-logic abuse.

## 30.3 Input validation

Validate payload size, structure, string length, numeric bounds, enum membership, identifier ownership, and business-state legality. Use structured DTOs and explicit domain constructors. Never pass user-supplied strings directly into dynamic SQL, shell commands, filesystem paths or URLs without a narrowly designed abstraction.

## 30.4 SSRF policy

The server should avoid arbitrary URL fetching entirely. Integration endpoints are configuration, not user-selected destinations. If a future feature requires merchant-supplied URLs, enforce an allowlist, DNS/IP validation and private-network blocking, and execute fetches in an isolated component.

# 31. Secrets and Key Management

Secrets have a lifecycle: provision, store, access, rotate, revoke, audit. Environment variables can be acceptable for early deployments but should be backed by a proper secret manager as the platform matures. Application source code, container images, mobile bundles and logs must never contain provider secrets.

## Key separation

- Authentication signing/verification keys are distinct from payment-provider credentials.

- MRA terminal credentials are isolated from generic application credentials.

- Database credentials are not reused as object-store or provider credentials.

- Operational/admin credentials use a separate privilege boundary from runtime service accounts.

# 32. Data Protection and Privacy Architecture

Malawi's Data Protection Act 2024 is part of the legal environment in which Sitolo operates. The architecture should therefore support data minimization, purpose limitation, controlled access, retention schedules, data export and deletion workflows, breach evidence, and processor/third-party inventory. Legal interpretation and compliance claims must be validated by qualified counsel; the architecture does not itself certify compliance.

## Data classification

| Class | Examples | Controls |

| --- | --- | --- |

| Public | Marketing content | Normal CDN/web controls |

| Business confidential | Sales totals, product costs, supplier terms | Tenant authorization, encryption in transit/at rest |

| Personal | Names, phone numbers, staff identity data | Minimize collection, scoped access, retention |

| Highly sensitive operational | Auth secrets, provider tokens, recovery data | Secret manager, strict roles, no normal logging |

| Regulated/evidence | Tax/EIS artifacts, controlled-stock records | Immutable evidence, retention policy, restricted export |

# 33. Performance Architecture

Performance targets should reflect merchant workflows rather than benchmark vanity. A cashier wants a sale to commit quickly even on modest hardware; an owner expects a report to load without blocking sales. The architecture separates interactive transactions from heavy computation.

| Workload | Path | Performance strategy |

| --- | --- | --- |

| POS sale | Synchronous API transaction | Short DB transaction, indexed lookup, bounded payload |

| Offline sale | Local SQLite transaction | Immediate local commit, background sync |

| Report generation | Async job or read model | Pre-aggregate, materialize, cache |

| EIS submission | Async after business commit where possible | Outbox + retry |

| Provider statement import | Async job | Staged parsing and reconciliation |

| Bulk export | Async object-storage job | Streaming generation, quota controls |

| Search | Indexed query/read model | Avoid table scans |

## Concurrency model

Tokio provides the async runtime. The application should use bounded concurrency for outbound calls, job workers and bulk processing. Unbounded `spawn` patterns are prohibited for request-derived work. Every external call needs a timeout. Every queue needs backpressure. CPU-heavy parsing/reporting should not run inside latency-sensitive async tasks; isolate it via blocking pools or dedicated workers.

## Connection pooling

Database pool sizing must be derived from PostgreSQL capacity, not from the number of HTTP requests. A small number of slow queries can exhaust a pool and cascade into timeouts. Use query timing telemetry and pool saturation metrics. Keep transactions short and never hold a database transaction open while calling an external API.

# 34. Caching

Cache only derived or read-mostly data. Product/catalogue snapshots, feature flags, report results and configuration are candidates. Financial truth, stock authority and permissions should not depend solely on caches. Every cache has an explicit invalidation strategy and a maximum staleness bound.

# 35. Reliability Engineering

The architecture assumes component failure. Database outage, Redis outage, object-store outage, payment provider outage and MRA outage have different blast radii and should degrade differently.

| Failure | Expected behavior |

| --- | --- |

| PostgreSQL unavailable | Writes fail safely; clients retain offline-eligible commands; no false success |

| Redis unavailable | Core transactions continue if Redis is not authoritative; affected cache/coordination features degrade |

| MRA unavailable | Local sales continue within allowed business/tax rules; EIS submissions queue and surface pending state |

| Payment provider unavailable | Payment initiation shows pending/unavailable; cash/manual recording follows configured workflow; no fabricated confirmation |

| Object storage unavailable | Core transactions continue unless the specific operation requires the blob; queued upload state visible |

| Notification provider unavailable | Business state remains committed; notification job retries |

# 36. Transactions and External Calls

A fundamental rule: do not hold a database transaction while waiting on an external network. Instead, commit the local intent and use an outbox/job to perform the external side effect. For cases where an external call must occur before local commit, use a carefully modeled pending state and compensating transition rather than pretending the two systems share one transaction.

# 37. Consistency Model

Sitolo uses multiple consistency levels intentionally. Local device state is eventually consistent with the server. The server database is strongly consistent for transactions within a primary database transaction. External integrations are asynchronously consistent. Reports may be eventually consistent. The UI should express these distinctions when users could otherwise misunderstand the state.

# 38. API Security Controls

- TLS for all production traffic.

- Strict content-type validation and request-size limits.

- Authentication middleware before tenant resolution.

- Authorization before data access, not after loading broad datasets.

- Per-principal and per-tenant rate limits where abuse matters.

- Idempotency for high-impact commands.

- Replay protection for webhooks.

- Consistent error envelopes without leaking SQL, secrets or internal topology.

- Security headers for any web surface.

- CSRF protection where browser cookies are used for authenticated sessions.

# 39. API Contract Example

```json
POST /api/v1/sales
Idempotency-Key: 01J...
Authorization: Bearer ...

{
  ""branchId"": ""..."",
  ""registerId"": ""..."",
  ""clientCommandId"": ""..."",
  ""lines"": [
    {""skuId"": ""..."", ""quantity"": 2, ""unitPriceMinor"": 1500}
  ],
  ""tenders"": [
    {""type"": ""CASH"", ""amountMinor"": 3000}
  ]
}

Response 201
{
  ""saleId"": ""..."",
  ""status"": ""COMMITTED"",
  ""receiptNumber"": ""..."",
  ""taxStatus"": ""PENDING""
}
```

The exact wire format is illustrative. The important architectural property is that one logical command crosses the boundary, is idempotent, is authorized, and results in an atomic domain transaction.

# 40. Database Transaction Patterns

## Sale transaction

```text
BEGIN
  lock/validate register
  validate catalogue state
  validate prices / tax inputs
  validate inventory availability
  INSERT sale
  INSERT sale_lines
  INSERT inventory_ledger rows
  UPDATE inventory balance(s) if using materialized balance
  INSERT payment/tender rows
  INSERT cash posting if cash
  INSERT tax submission request if required
  INSERT audit row
  INSERT outbox rows
COMMIT
```

No network call occurs inside the transaction. If an external integration is required, the outbox record becomes the durable handoff.

# 41. Inventory Concurrency

For a sale against a materialized stock balance, an atomic update pattern is preferred. Conceptually: decrement only if available quantity is sufficient, check the affected row count, then insert the immutable ledger movement in the same transaction. For batch/lot allocation, the transaction may allocate one or more lot rows using deterministic FEFO ordering. This avoids two concurrent cashiers both observing the same stock and selling it twice.

# 42. Offline Sale Reconciliation Example

```text
Device A offline
  Sale C1: SKU X qty 2
  Sale C2: SKU X qty 2

Server last known stock: 3

Sync C1 -> accepted; stock becomes 1
Sync C2 -> rejected/held because authoritative stock is 1

The system does NOT:
- create negative stock silently;
- delete C2;
- change C2's quantity without evidence;
- pretend both sales were accepted.

Instead C2 enters an explicit exception state requiring policy-specific handling.
```

Whether Sitolo allows negative stock for a particular segment is a product decision, but it must be explicit and permissioned. An enterprise architecture must never accidentally implement negative inventory simply because concurrency was not designed.

# 43. Sync Security

Sync endpoints are high-risk because they accept durable batches of mutations. The server must validate every command independently. A compromised device cannot obtain authority merely by possessing a valid device identifier. Payloads should be bounded by batch size, byte size, command count and age. The server should reject unknown schema versions and require migration or re-download for incompatible clients.

# 44. Schema Versioning

Every persistent sync command and externally durable event should have a schema version. Client and server can support a compatibility window. Migrations need explicit direction: additive changes are preferred; destructive changes require version transitions. Never infer schema compatibility from application version strings alone.

# 45. Device Replacement and Recovery

Device recovery is a business continuity workflow. A merchant may lose a phone with unsent sales. The server must support device revocation and account recovery without automatically assuming that all local data was synced. The device should display unsynced command counts. Recovery procedures should identify whether the lost device had pending commands and whether those commands later arrived from another device.

# 46. Multi-Branch Architecture

The branch is a security and operational boundary. Products may be shared organization-wide, while stock, registers, cash and staff assignments can be branch-scoped. Cross-branch transfers are explicit inventory movements with source and destination. Reports must support branch selection without bypassing authorization.

# 47. Entitlements and Feature Flags

Business type determines recommended workflows; subscription plan determines entitlement. This is an important architectural distinction. The backend should calculate effective capabilities from organization plan, purchased add-ons, vertical configuration, feature flags, regulatory region and account state. The client receives a capability snapshot for UI adaptation, but server authorization remains authoritative.

# 48. Billing Architecture

Sitolo's own commercial billing follows the same expected-versus-observed-versus-reconciled principle as merchant finance. A subscription invoice creates an expected payment; a provider/bank observation is an external fact; reconciliation confirms payment; entitlement activation follows the resulting billing state. Billing must be isolated from merchant payment records even though the same payment infrastructure adapters may be reused.

# 49. Configuration Management

Configuration should be classified as static, tenant-configurable, provider-derived, or runtime-operational. Static settings live in code/config. Tenant settings live in PostgreSQL. Provider-derived settings are snapshots from the external authority. Runtime settings such as rate limits and feature flags may use a configuration service or database table with cache invalidation. Never place secret values inside ordinary tenant configuration rows without encryption and strict access controls.

# 50. Migrations

Schema migrations are code. Each migration is immutable after release. Production applies migrations in a controlled deployment step with rollback strategy appropriate to the migration. Prefer expand-and-contract migrations for high-volume tables: add nullable/new structures, backfill asynchronously, deploy code that understands both versions, then remove obsolete structures later.

# 51. Observability

Observability has three pillars plus domain audit: logs, metrics and traces, with audit evidence providing business history. Use structured JSON logs. Each request and job carries correlation IDs. Sensitive fields are redacted before logging.

| Signal | Example metric |

| --- | --- |

| API latency | http_request_duration_ms |

| API errors | http_requests_total{status_class} |

| DB pressure | db_pool_in_use, db_pool_wait_ms |

| Sync | sync_commands_queued, sync_conflicts, sync_rejections |

| Payments | payment_pending_age, webhook_rejections, reconciliation_exceptions |

| Tax | eis_pending_count, eis_rejection_count, eis_submission_latency |

| Inventory | negative_stock_attempts, adjustment_rate, stock_sync_lag |

| Jobs | job_queue_depth, retry_count, dead_letter_count |

| Business | sales_count, gross_value, active_registers |

# 52. Distributed Tracing

Use tracing spans around API requests, database transactions, external calls and worker jobs. A trace should reveal when a slow request is caused by SQL, a provider, lock contention or application processing. Do not put full payment/tax payloads in trace attributes. Use references and safe summaries.

# 53. SLOs and Reliability Targets

Initial SLOs should be realistic and measurable rather than aspirational. Recommended targets can be refined after production evidence:

| Service property | Initial target |

| --- | --- |

| API availability excluding scheduled maintenance | 99.9% monthly for core authenticated API |

| Successful interactive sale latency while online | p95 under 500 ms excluding external provider wait |

| Local offline sale commit | p95 under 100 ms on supported devices |

| Sync acknowledgement | p95 under 2 seconds under normal network conditions |

| Critical financial mutation durability | No acknowledged transaction lost |

| Backup recovery point | Define based on tier; target <= 15 minutes for mature tier |

| Restore exercise | At least quarterly for production |

| Critical alert acknowledgement | Defined on-call policy; not an engineering guess |

SLOs must distinguish platform behavior from provider latency. A payment provider being slow should not make the core sale endpoint appear unhealthy if payment initiation is correctly modeled as asynchronous or pending.

# 54. Backup and Disaster Recovery

PostgreSQL backups should include automated snapshots, point-in-time recovery where available, and periodic restore testing. Backup success is not equivalent to recoverability. A restore drill must produce a usable database and verify representative tenant data, migrations and operational credentials.

## RPO/RTO tiers

| Tier | Typical customer | Example RPO | Example RTO |

| --- | --- | --- | --- |

| Standard | Small single-branch SME | 15–60 min | 1–4 h |

| Growth | Multi-branch SME | 15 min | 1–2 h |

| Enterprise | High-dependency customer | <15 min subject to infrastructure | <1 h target |

These are architecture targets, not guarantees. Contractual enterprise SLOs should be priced and provisioned separately. The mobile client's offline mode is itself a form of business continuity, but it is not a substitute for server recovery.

# 55. Deployment Architecture

```text
                         Internet
                            │
                   ┌───────────────┐
                   │ CDN / WAF / LB │
                   └───────┬───────┘
                           │
                ┌──────────┴──────────┐
                │ Rust application     │
                │ stateless replicas   │
                └──────────┬──────────┘
                           │
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
       PostgreSQL      Worker pool   Object storage
             │             │
             └──────┬──────┘
                    ▼
                optional Redis
```

The API process should be stateless with respect to business truth. Any local filesystem use is temporary and non-authoritative. This permits multiple application replicas later. Workers may scale separately when workloads justify it, even while sharing the same codebase and PostgreSQL database.

# 56. Infrastructure Phasing

| Phase | Deployment |

| --- | --- |

| Development | Single developer environment; PostgreSQL + optional Redis + local object storage emulator |

| Pilot | Managed PostgreSQL, one Rust API deployment, one worker deployment, object storage, monitoring |

| Growth | Multiple API replicas, separate worker pools, read replica/reporting path if needed, WAF/rate limiting |

| Scale | Dedicated reporting infrastructure, partitioning strategy if measured necessary, regional deployments only when justified |

# 57. Container and Build Design

Build the Rust backend as a reproducible artifact with a minimal runtime image. Use multi-stage builds. Run as a non-root user. Include only runtime libraries required. Lock Cargo dependencies. Generate SBOMs for releases when operational maturity warrants it. Flutter/Tauri release pipelines should pin SDK/toolchain versions and produce signed artifacts.

# 58. CI/CD

```text
PULL REQUEST
  │
  ├── formatting/lints
  ├── unit tests
  ├── domain property tests
  ├── integration tests
  ├── migration checks
  ├── dependency/security scan
  ├── OpenAPI/schema compatibility
  ├── build all targets
  └── targeted performance checks
         │
         ▼
      MAIN
         │
   release candidate
         │
   staging verification
         │
   production deploy
         │
   post-deploy health
```

The Rust toolchain should be pinned in `rust-toolchain.toml` and lockfiles committed. The current official stable line is Rust 1.98.0 as of August 20, 2026; the exact repository version should be chosen and kept reproducible rather than floating to future stable releases automatically. citeturn244519search0turn244519search3

# 59. Supply Chain Security

- Commit `Cargo.lock` for deployable Rust applications.

- Run `cargo audit` and dependency policy checks.

- Review transitive dependencies that handle parsing, networking or cryptography.

- Pin major versions deliberately and document exceptions.

- Use Dependabot/Renovate or equivalent only with CI verification; automatic merge is not assumed.

- Track SBOM/component provenance for enterprise releases.

- Do not accept an unreviewed dependency merely to remove a few lines of code from the repository.

# 60. Testing Architecture

Testing is layered because no single test class catches the failure modes of a business OS.

| Layer | Purpose |

| --- | --- |

| Pure unit | Domain rules, calculations, state transitions |

| Property-based | Inventory arithmetic, idempotency, serialization, invariants |

| Repository integration | SQL constraints, transactions, row-level authorization behavior |

| Application integration | Commands across multiple modules |

| API contract | Request/response compatibility |

| Sync simulation | Offline queues, duplicates, retries, conflicts, schema versions |

| External contract | Provider adapter behavior with recorded/test fixtures where lawful |

| Security regression | Tenant leakage, privilege escalation, replay, injection |

| Load/performance | Concurrency, pool saturation, report workloads |

| Chaos/failure | Provider outage, job duplication, process termination, delayed network |

| Restore/DR | Backups and actual restoration |

## 60.1 Must-have invariants

- A sale cannot finalize twice for the same command ID.

- A refund cannot exceed eligible value.

- An unauthorized tenant cannot read another tenant's sale even when given a valid object ID.

- A quarantined/expired lot cannot enter a normal sale allocation path where the policy forbids it.

- A duplicate provider event cannot produce duplicate financial postings.

- Restarting the app with queued offline commands does not lose them.

- Syncing the same batch twice is harmless.

- Every finalized financial correction is represented by an explicit compensating event.

# 61. Property-Based Testing Ideas

Property tests are particularly valuable for the inventory and financial core. Generate sequences of receipts, sales, returns, transfers and adjustments and assert that materialized balances equal ledger sums. Generate duplicate commands and assert idempotent result counts. Generate randomized decimal/minor-unit tax combinations and assert rounding invariants. Generate reorderings of independent sync commands and verify commutativity only where the domain has declared it valid.

# 62. Security Testing

Every tenant-scoped endpoint should have a negative test matrix. For each action, test same-tenant authorized, same-tenant unauthorized, cross-tenant authorized-looking, missing membership, revoked membership, wrong branch, stale session, malformed object ID and replayed idempotency key. Security tests should run against a real PostgreSQL instance because application-only mocks cannot prove database ownership constraints.

# 63. Load Testing Model

Load tests should simulate realistic merchant concurrency: many small tenants with bursts at opening/closing hours, not a single tenant generating all traffic. This matters because tenant distribution changes cache efficiency, index behavior and authorization query patterns. Separate transactional latency from reporting throughput. Include slow mobile networks and intermittent disconnect/reconnect in client-side performance tests.

# 64. Migration and Data Import Architecture

Migration is an acquisition tool and a safety boundary. The import pipeline should be staged:

`UPLOAD → PARSE → VALIDATE → PREVIEW → APPROVE → COMMIT → RECONCILE`

Imports should never write directly into core tables from raw CSV/Excel parsing. Use staging tables, validate references and duplicate keys, show row-level errors, and commit only approved batches. Every import gets an import ID and audit history. Large imports run asynchronously.

# 65. Data Export

Exports are a high-risk data movement path. Apply tenant and role authorization before generating the export. Prefer asynchronous generation into short-lived object storage links. Record who exported what, when, filters, format and result count. Large exports should be rate-limited and subject to plan entitlements.

# 66. Support and Internal Operations Tooling

Build a small Rust/TypeScript internal operations surface rather than giving engineers direct production database access for routine support. Required capabilities eventually include tenant lookup, health summary, sync inspection, reconciliation exception inspection, integration state, feature flag inspection and safe account/device recovery. Mutations require explicit permission and produce audit evidence.

# 67. Feature Flags

Feature flags are operational controls, not a replacement for entitlements or authorization. Flags should be evaluated server-side for security-relevant capabilities and client-side only for presentation. Flags should have owners, expiry dates and rollback semantics. Avoid permanent flag branches that create two permanently supported business paths.

# 68. Data Retention

Retention must distinguish legal/business evidence from convenience data. Transaction history may require long retention; device telemetry may not. The retention engine should define data classes, retention periods, legal holds and deletion/anonymization behavior. Deletion of personal data must not be designed in a way that destroys legally required transaction evidence; where necessary, pseudonymization is preferable, subject to legal review.

# 69. Regionalization for Africa

The architecture should avoid Malawi-specific hardcoding where the concept is actually regional: currency is a code, tax rules are configurable by country, phone numbers use normalized international representation, time zones are configuration, language is externalized, and payment providers are adapters. However, do not build an imaginary continent-wide abstraction before Malawi product-market fit. Regionalization must be additive and evidence-driven.

# 70. Localization

Localization is not only translation. It includes currency formatting, number formats, date formats, phone formatting, payment method labels, receipt/legal requirements, tax terminology and offline behavior. The domain stores canonical values; presentation layers localize them.

# 71. Mobile Device Constraints

The mobile client must assume low storage, memory pressure, background execution limits and battery optimization. Sync must be resumable and chunked. SQLite transactions should remain short. Image caching must be bounded. Historical data should be paginated and optionally compacted into summaries. Sensitive local data should be minimized and encrypted where the platform/database solution supports it without destroying performance.

# 72. Mobile UX Architectural Consequences

Fast workflows require local read models. The product should not perform a network round trip for every product lookup. Catalogue, pricing and relevant stock state are synchronized snapshots. The UI reads from local SQLite and renders immediately. Network synchronization updates the local read model later. This architecture supports the merchant experience even when the backend is temporarily unreachable.

# 73. Desktop UX Architectural Consequences

The desktop surface optimizes for keyboard/mouse, larger tables, bulk imports, printing, reconciliation review, reporting and branch management. It may share some domain client code with mobile, but it should not be a desktop mirror of mobile screens. Tauri-specific native integrations are isolated behind a small bridge layer so the main UI remains portable.

# 74. Web/Admin Surface

A limited web/admin application can use Next.js/React + TypeScript because the web ecosystem is valuable for management dashboards, account administration, documentation and enterprise workflows. It is not the transactional core and does not own domain logic. Generated API types should be consumed from the Rust API contract, reducing manual duplication.

# 75. Developer Experience

A strong developer workflow is part of architecture. The repository should provide one command or documented sequence for local infrastructure, migrations, tests, linting, running the API, running workers and generating contracts. Engineers should not need to understand every production component to run the core suite locally.

# 76. Configuration per Environment

| Environment | Purpose |

| --- | --- |

| local | Fast iteration; fake/sandbox external integrations |

| test | Deterministic integration and security suites |

| staging | Production-like infrastructure and external sandboxes |

| production | Real tenant data and provider credentials |

External providers should default to sandbox or mock implementations outside production. Accidentally charging real money or transmitting test data to a real tax endpoint is a severe release failure.

# 77. Error Taxonomy

Errors should be classified into authentication, authorization, validation, conflict, not-found, business-rule rejection, transient dependency, permanent dependency, infrastructure and internal categories. The API translates these into stable error codes. Logs preserve deeper causes. Clients use codes rather than parsing human-readable error messages.

# 78. Backpressure and Rate Limiting

The platform should protect itself and downstream providers through per-IP, per-principal, per-tenant and per-integration limits where appropriate. Batch sync endpoints need separate quotas from normal interactive APIs. Rate limiting should degrade user experience predictably rather than cause partial financial writes.

# 79. Circuit Breakers and Timeouts

Every outbound provider call has a connection timeout, request deadline and bounded retry policy. Circuit breaking is appropriate for integrations whose repeated failure could consume worker capacity. Retries must use exponential backoff with jitter. Do not retry non-idempotent actions unless the external contract makes them safely idempotent.

# 80. Webhook Ingestion

```text
POST /webhooks/provider
  ↓
verify source/signature
  ↓
normalize event envelope
  ↓
check provider event ID
  ↓
store inbox event
  ↓
ACK quickly
  ↓
async process event
  ↓
apply idempotent business effect
```

A webhook handler should do enough validation to safely acknowledge or reject the request, then move complex business processing to a durable job. This avoids provider retries caused by slow application work and reduces the chance of duplicate processing.

# 81. Reconciliation Engine Detailed Design

The reconciliation engine is one of Sitolo's most strategically important custom components. It receives expected records from sales/payment intents and observed records from provider callbacks, statements, bank imports or manual evidence. It produces a match decision and an exception when evidence is insufficient.

```text
EXPECTED PAYMENT                    OBSERVED PAYMENT
----------------                    ------------------
intent_id                           provider_tx_id
sale_id                             account
amount                              amount
currency                            currency
expected_at                         observed_at
provider                            provider
                                      │
                                      ▼
                             MATCHING PIPELINE
                                      │
                    ┌─────────────────┼─────────────────┐
                    ▼                 ▼                 ▼
                 exact             constrained       unmatched
                 match              candidate         exception
                    │                 │
                    └──────────┬──────┘
                               ▼
                         RECONCILIATION
                            DECISION
```

## 81.1 Matching hierarchy

- Exact provider transaction reference.

- Exact merchant account + provider reference + amount/currency.

- Strongly constrained payment intent identifier supplied by merchant workflow.

- Candidate matching using amount/time/account only when explicitly enabled; never silently close high-risk exceptions.

## 81.2 Exceptions

| Exception | Example action |

| --- | --- |

| Missing observation | Wait, query provider, or flag pending |

| Unexpected observation | Investigate; do not attach to a sale automatically |

| Amount mismatch | Manual review or policy-specific partial match |

| Duplicate provider event | Mark duplicate and retain evidence |

| Duplicate merchant reference | Security/operational exception |

| Reversed/charged-back transaction | Create explicit financial correction workflow |

# 82. Fraud Controls

The product should make fraud difficult without making routine work unbearable. Controls include manager approval thresholds, daily cash variance alerts, high-volume refund alerts, unusual discount tracking, rapid void patterns, role separation and device anomaly signals. These are control signals, not automatic accusations. The architecture records evidence and allows review.

# 83. Audit-Friendly Domain Design

Every state-changing command should result in an audit-relevant record. Do not depend on database row diffs alone. Domain events express why a state changed; audit expresses who initiated it and through which control path. This distinction improves support, compliance and future analytics.

# 84. Read Models

For performance, the reporting module may maintain read-optimized projections. These projections are derived and rebuildable. Projection code must record its source watermark/checkpoint so operators can detect lag. Rebuilding a projection should not mutate core transactional tables.

# 85. Event Sourcing: What We Are and Are Not Doing

Sitolo is not required to become a full event-sourced system. Core financial history uses append-only business postings/events because that is valuable for correctness and audit. Other CRUD configuration such as product descriptions can remain ordinary mutable state with audit logging. Forcing full event sourcing onto catalogue settings, feature flags or user profiles would add operational cost without sufficient business benefit.

# 86. CQRS: Limited and Practical

The architecture uses command/query separation conceptually, but not necessarily separate databases or a heavyweight CQRS framework. Commands change authoritative state; queries read optimized projections. This is enough to achieve most of the practical benefit while keeping operational complexity low.

# 87. Service Extraction Criteria

A module may become a separate service only when at least one of these is true: it needs an independent scaling profile, failure isolation is materially valuable, compliance requires isolation, its ownership boundary is stable, its deployment cadence differs substantially, or its data lifecycle is independent. Candidate future services include notification delivery, report processing, payment integration gateway, tax adapter gateway, search/indexing and analytics ingestion. POS, inventory, sales and reconciliation should stay together until there is evidence otherwise because their transactional coupling is a strength, not a defect.

# 88. Why Not Microservices Now

Microservices would introduce network partitions, distributed transactions, versioned service contracts, more observability, more deployments, more secrets, more retries and more operational burden before Sitolo has demonstrated the need. The product already has a hard distributed-system problem—offline synchronization. Adding internal microservice distribution on top of that would multiply the state space. The modular monolith keeps the difficult business problem hard but the infrastructure problem manageable.

# 89. Future Extraction Map

```text
START
│
└── modular monolith
      │
      ├── extract Notifications when delivery volume/ownership justifies
      ├── extract Reporting when analytical load threatens OLTP
      ├── extract Integration Gateway when provider fleet becomes large
      └── extract Search when search requirements exceed PostgreSQL

Do NOT extract just because a module has many files.
Extract when a runtime boundary creates real value.
```

# 90. Cost and Unit Economics

The architecture must preserve gross margin. Track infrastructure cost per active merchant, storage per merchant, database I/O, sync bandwidth, report CPU time and external integration cost. A feature that causes ten times more infrastructure cost than its subscription value is an architectural/business problem. Asynchronous work should be bounded, and large customers should be priced or isolated when their workload materially differs from the standard tier.

# 91. Capacity Planning

Capacity planning starts from business units: active merchants, branches, daily sales, lines per sale, sync commands per device, payment callbacks, report runs and storage objects. Estimate writes per second, read concurrency, daily data volume and peak burst rather than using a generic 'requests per second' number. The modular monolith can handle more than enough for early growth if queries remain disciplined.

# 92. Data Growth Strategy

Append-only transaction tables grow continuously. At meaningful scale, use archival/partitioning strategy based on observed volume. Time-based partitioning may be useful for audit/events and high-volume transaction history, but it should not be introduced before operational tooling exists. PostgreSQL indexes must be monitored for write amplification and bloat.

# 93. Database Observability

- Slow-query logging and query duration metrics.

- Connection pool wait time.

- Lock wait/deadlock metrics.

- Autovacuum health and table bloat monitoring.

- Index usage and unused-index review.

- Replica lag if replicas exist.

- Transaction age and long-running transaction detection.

# 94. Operational Runbook: Sale Incident

```text
1. Identify sale ID / receipt number / client command ID.
2. Locate request/trace/audit evidence.
3. Determine server transaction state.
4. Determine inventory postings.
5. Determine payment/tender state.
6. Determine tax/EIS state.
7. Determine whether a retry occurred.
8. Check idempotency record.
9. Apply an explicit correction workflow if needed.
10. Never edit historical rows to 'make them look right'.
```

# 95. Operational Runbook: Device Lost

```text
1. Disable/revoke device.
2. Revoke device-scoped credentials.
3. Preserve server-side unsynced/received command evidence.
4. Assess whether the device had pending local commands based on last checkpoint.
5. Register replacement device.
6. Rehydrate authoritative data.
7. Resolve any duplicated/replayed commands through idempotency.
8. Review audit trail for unusual activity around the loss.
```

# 96. Operational Runbook: Payment Exception

```text
1. Identify expected payment intent.
2. Identify provider reference/statement line.
3. Verify provider signature/source where applicable.
4. Check duplicate event table.
5. Compare amount/currency/account.
6. Check sale and refund history.
7. Match or escalate as exception.
8. Record human decision and evidence.
9. Re-run downstream projections only after authoritative correction.
```

# 97. Operational Runbook: MRA EIS Failure

```text
1. Confirm local sale is committed.
2. Confirm tax state is PENDING/RETRY/REJECTED.
3. Inspect MRA response/error and configuration version.
4. Confirm terminal activation/configuration status.
5. Retry only when the failure is retryable.
6. For permanent rejection, surface a tax exception and preserve evidence.
7. Never change sale financial facts to match a failed external submission.
```

# 98. Operational Runbook: Database Recovery

```text
1. Declare incident and freeze unsafe write paths if required.
2. Determine latest valid backup/PITR point.
3. Restore into isolated environment.
4. Validate migrations/schema compatibility.
5. Run integrity checks.
6. Compare critical transaction counts/watermarks.
7. Promote/reconnect according to DR procedure.
8. Reconcile external integration states after recovery.
9. Document incident timeline and lessons.
```

# 99. Security Review Gates

| Gate | Required before |

| --- | --- |

| Tenant isolation suite | Every production release touching authorization/data access |

| Dependency security scan | Every release |

| Secret scan | Every PR |

| Migration review | Every schema change |

| External integration contract test | Provider adapter release |

| Threat-model update | New high-risk capability |

| Restore drill | Quarterly / before major infrastructure change |

| Privacy impact review | New personal-data workflow |

| Regulatory review | New tax/pharmacy/payment capability |

# 100. Architecture Decision Records

## ADR-001 Rust-first backend

**Decision:** Rust is the primary backend language.
**Reason:** memory safety, predictable performance, strong type system, efficient concurrency and alignment with user's long-term backend strategy.
**Tradeoff:** smaller pool of application developers and a steeper learning curve.
**Mitigation:** strong repository conventions, modular code, code review templates and reusable internal libraries.

## ADR-002 Flutter mobile

**Decision:** Flutter is the primary mobile client.
**Reason:** cross-platform code reuse, native release compilation, mature UI ecosystem, and explicit support for SQL persistence/offline-first patterns. citeturn734608search5turn734608search12
**Tradeoff:** larger client binary and Dart/Flutter expertise requirement.

## ADR-003 Tauri desktop

**Decision:** Tauri desktop rather than Electron for the merchant back office.
**Reason:** Rust alignment, OS/webview integration, and smaller runtime footprint characteristics. Tauri documents its Rust + webview architecture and message-passing model. citeturn734608search4
**Tradeoff:** native integrations still require careful platform-specific testing.

## ADR-004 PostgreSQL as source of truth

**Decision:** PostgreSQL is authoritative for server-side transactional state.
**Reason:** ACID transactions, constraints, mature operational ecosystem and suitability for multi-tenant business data.
**Tradeoff:** careful schema/index/transaction engineering is required.

## ADR-005 Modular monolith

**Decision:** start as a modular monolith with independent workers/adapters.
**Reason:** preserves transaction locality and lowers distributed-systems overhead while allowing later extraction.
**Tradeoff:** requires architectural discipline to prevent module coupling.

## ADR-006 Custom offline command/sync protocol

**Decision:** implement a Sitolo-specific sync protocol rather than generic last-write-wins synchronization.
**Reason:** sales, stock, payment and financial corrections have semantics that generic document replication cannot safely infer.
**Tradeoff:** significant engineering complexity.
**Why accepted:** offline continuity is a primary product differentiator, not an optional optimization.

## ADR-007 Transactional outbox

**Decision:** use PostgreSQL transactional outbox before introducing an external message broker.
**Reason:** reliable asynchronous side effects without adding an operational messaging platform.
**Tradeoff:** worker/polling logic and database outbox growth must be managed.

# 101. Open Decisions Requiring Evidence

Some decisions should remain deliberately open until prototypes or customer data settle them:

- Exact Flutter SQLite package and synchronization library choices.

- Exact identity provider versus self-hosted standards-based auth.

- Exact hosting/cloud provider and region strategy.

- Redis necessity after real load measurement.

- Whether report workloads justify a read replica or warehouse.

- Whether certain payment providers expose sufficient webhook/statement APIs for deterministic reconciliation.

- Exact MRA EIS certification pathway and production credentials.

- Exact pharmacy regulatory data model and permissions.

- Whether enterprise customers require dedicated database isolation.

# 102. Reference Architecture for First Production Release

```text
CLIENTS
- Flutter Android app
- Tauri desktop app
- Small TypeScript admin web

SERVER
- Rust 1.98.x pinned stable toolchain
- Axum + Tokio
- SQLx + PostgreSQL
- tracing / metrics / OpenTelemetry-compatible export
- transactional outbox + worker runtime

DATA
- PostgreSQL primary
- SQLite local client store
- S3-compatible object storage
- Redis optional, non-authoritative

CORE MODULES
- identity
- tenant
- authorization
- catalogue
- pricing
- procurement
- inventory
- sales
- cash
- payments
- reconciliation
- tax/EIS
- audit
- reporting

VERTICAL EXTENSIONS
- pharmacy
- agro-dealer
- wholesale

EXTERNALS
- payment provider adapters
- MRA EIS adapter
- notifications
```

# 103. Implementation Sequence

The architecture is target-complete but should be implemented in dependency order. The sequence below deliberately follows the business model flywheel and minimizes rework.

```text
FOUNDATION
  ↓
identity + tenant + roles + audit primitives
  ↓
catalogue + pricing
  ↓
inventory ledger + locations
  ↓
POS + cash
  ↓
offline local store + sync protocol
  ↓
procurement + suppliers
  ↓
payments + reconciliation
  ↓
MRA EIS adapter
  ↓
reporting + exports
  ↓
multi-branch / approvals / billing
  ↓
pharmacy / agro / wholesale extensions
  ↓
enterprise APIs / analytics / regionalization
```

# 104. Implementation Rules

- Do not create a service because a folder is getting large.

- Do not allow UI code to implement business rules that the server also needs.

- Do not write direct SQL from API route handlers; use repositories/application services.

- Do not keep a database transaction open across an external HTTP call.

- Do not use floating point for money.

- Do not mutate finalized financial records when a correction event is required.

- Do not trust tenant IDs, branch IDs or permissions supplied by clients.

- Do not let Redis become authoritative for business truth.

- Do not let analytics/reporting tables become a second source of financial truth.

- Do not claim compliance from a passing happy-path integration test.

# 105. Architecture Quality Bar

A feature is architecturally complete when its normal path, failure path, retry path, authorization model, tenant boundary, data ownership, audit trail, offline behavior where relevant, integration semantics, tests, observability, migration strategy and operational recovery path are defined. 'It works on my phone' is not the definition of complete.

| Question | Required answer |

| --- | --- |

| Who owns the state? | Named module and authoritative table/event |

| Who can mutate it? | Explicit permission/policy |

| What happens offline? | Defined local behavior and sync semantics |

| What happens on retry? | Idempotent result or explicit retry policy |

| What happens if external provider is down? | Safe degraded state |

| How is it audited? | Actor + action + target + outcome |

| How is it tested? | Unit + integration + failure/security cases |

| How is it observed? | Metrics/logs/traces and alerts |

| How is it recovered? | Runbook and data reconstruction path |

# 106. Final Architecture Diagram

```text
                                   SITOLO
                BUSINESS OPERATING SYSTEM FOR AFRICAN SMES

 ┌────────────────────────────── CLIENTS ───────────────────────────────┐
 │                                                                      │
 │  Flutter Mobile                     Tauri Desktop       TS Web/Admin │
 │  ┌──────────────┐                 ┌──────────────┐     ┌───────────┐ │
 │  │ UI           │                 │ UI           │     │ UI        │ │
 │  │ Local SQL    │                 │ Local state  │     │ Browser   │ │
 │  │ Sync engine  │                 │ Native bridge│     │ tools     │ │
 │  └──────┬───────┘                 └──────┬───────┘     └─────┬─────┘ │
 └─────────┼────────────────────────────────┼───────────────────┼───────┘
           │                                │                   │
           └───────────────────────HTTPS/API───────────────────┘
                                      │
                                      ▼
 ┌──────────────────────────── RUST CORE ───────────────────────────────┐
 │                                                                      │
 │ Auth → Tenant → Policy → Commands/Queries → Domain → Persistence   │
 │                                                                      │
 │ Catalogue · Pricing · Inventory · POS · Cash · Procurement          │
 │ Payments · Reconciliation · Tax · Customers · Reporting · Billing   │
 │ Audit · Notifications · Vertical Extensions                        │
 │                                                                      │
 └──────────┬─────────────────────┬───────────────────┬────────────────┘
            │                     │                   │
            ▼                     ▼                   ▼
     ┌──────────────┐      ┌───────────────┐   ┌─────────────────┐
     │ PostgreSQL   │      │ Outbox/Workers│   │ Integration      │
     │ source truth │      │ async effects │   │ adapters         │
     └──────┬───────┘      └──────┬────────┘   └───────┬─────────┘
            │                     │                   │
            │               ┌─────┴─────┐       ┌─────┴──────────────┐
            │               │ Redis     │       │ Payments / MRA EIS │
            │               │ optional  │       │ SMS / Email / Bank │
            │               └───────────┘       └────────────────────┘
            │
            ▼
      Reporting / Audit / DR

CORE PRINCIPLE:
Clients create commands. Rust decides truth. PostgreSQL records truth.
Workers handle external side effects. Integrations remain replaceable.
Offline clients preserve continuity without becoming permanently authoritative.
```

# 107. Research and Source Notes

External sources were checked for current technology and Malawi integration context while preparing this architecture. The most material findings are below.

| Source | Architecture relevance |

| --- | --- |

| Rust official release announcements | Rust 1.98.0 is the current stable release listed for August 20, 2026; use a pinned stable toolchain rather than an implicit environment version. |

| Flutter architectural overview | Supports cross-platform native compilation and an architecture designed for reusable application layers. |

| Flutter offline-first guidance | Supports local persistence plus synchronization as an explicit architecture pattern. |

| Flutter SQL persistence guidance | Supports SQL/SQLite for complex local persistence in Flutter applications. |

| Tauri architecture | Rust + webview desktop architecture with message-passing and optional JS/TS APIs. |

| MRA EIS developer resources | Official API documentation and onboarding material for POS/EIS integration. |

| MRA EIS terminal documentation | POS software on Windows/Linux/Android/macOS can act as a terminal. |

| MRA EIS offline documentation | Offline transactions are stored locally and submitted after connectivity returns; configuration includes offline limits. |

| MRA EIS API documentation | Documents onboarding, configuration, sales and utility groups. |

| SQLx project documentation | Compile-time query checking and explicit MSRV/release practices support a reproducible Rust/PostgreSQL backend. |

# 108. References

1. Rust Release Team, **Announcing Rust 1.98.0**, 20 Aug 2026 — https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/
2. Rust official release announcements — https://blog.rust-lang.org/releases/
3. Flutter, **Architectural overview** — https://docs.flutter.dev/resources/architectural-overview
4. Flutter, **Offline-first support** — https://docs.flutter.dev/app-architecture/design-patterns/offline-first
5. Flutter, **Persistent storage architecture: SQL** — https://docs.flutter.dev/app-architecture/design-patterns/sql
6. Tauri, **Architecture** — https://v2.tauri.app/concept/architecture/
7. Malawi Revenue Authority, **EIS Developer Resource Center** — https://eis-portal.mra.mw/Home/DeveloperResources
8. Malawi Revenue Authority EIS API, **Introduction** — https://eis-api.mra.mw/docs/introduction_1_print.htm
9. Malawi Revenue Authority EIS API, **Overview / lifecycle** — https://eis-api.mra.mw/docs/overview.htm
10. Malawi Revenue Authority EIS API, **Terminal** — https://eis-api.mra.mw/docs/terminal.htm
11. Malawi Revenue Authority EIS API, **EIS API** — https://eis-api.mra.mw/docs/eis_api_2.htm
12. Malawi Revenue Authority EIS API, **Transacting Offline** — https://eis-api.mra.mw/docs/transacting_offline.htm
13. Malawi Revenue Authority EIS API, **Signing Offline Receipts** — https://eis-api.mra.mw/docs/signing_offline_receipts_print.htm
14. Malawi Revenue Authority EIS API, **Submitting Offline Transactions** — https://eis-api.mra.mw/docs/submitting_offline_transactions.htm
15. LaunchBadge SQLx, **CHANGELOG / project repository** — https://github.com/launchbadge/sqlx
16. Existing Sitolo source baseline: `sitolo.md` and `business_model_design.md` (internal project documents).

# 109. Change Control

This architecture is a living engineering document. Any change to one of the following requires an architecture decision record or an explicit amendment: primary data authority, tenant isolation model, offline sync semantics, financial event model, external payment/tax trust boundary, authentication architecture, deployment boundary, or service-extraction decision. Ordinary implementation refactors do not require architecture approval if they preserve the documented invariants.

# 110. Closing Position

Sitolo should not try to win by having the most components. It should win by making the business system trustworthy under the conditions its customers actually operate in. Rust provides the implementation foundation for a memory-safe, high-concurrency backend. Flutter provides the mobile operating surface. Tauri provides a native-feeling desktop control surface. PostgreSQL provides durable transactional truth. SQLite provides local continuity. External provider APIs remain adapters. The differentiated engineering lies in the seams: tenant isolation, offline synchronization, inventory correctness, financial history, payment reconciliation, tax submission state, auditability and operational recovery.

The architecture therefore follows a simple equation:

`MATURE PRIMITIVES + STRONG DOMAIN MODEL + OFFLINE RELIABILITY + FINANCIAL CORRECTNESS + SECURITY + OPERATIONAL DISCIPLINE = SITOLO`.

The technical system should remain boring where the market does not care and extremely deliberate where merchant trust depends on it.


---

**Document metrics:** approximately 14,608 words; 2,199 lines.

# 111. Detailed API Resource Model

Sitolo's API should expose business concepts as stable resources and state-changing actions as explicit commands. The API should not map every database table to an endpoint. That approach creates a CRUD surface that leaks implementation details and makes business invariants difficult to enforce.

## 111.1 Resource families

```text
IDENTITY
  /me
  /sessions
  /devices

TENANCY
  /organizations
  /business-entities
  /branches
  /locations
  /registers
  /memberships

CATALOGUE
  /products
  /skus
  /categories
  /units
  /barcodes

PRICING
  /price-lists
  /prices
  /promotions

PROCUREMENT
  /suppliers
  /purchase-orders
  /goods-receipts

INVENTORY
  /stock
  /lots
  /transfers
  /counts
  /adjustments

SALES
  /sales
  /returns
  /refunds
  /voids

CASH
  /registers/{id}/shifts
  /cash-movements
  /cash-closes

PAYMENTS
  /payment-intents
  /payments
  /provider-events

RECONCILIATION
  /reconciliation/runs
  /reconciliation/exceptions
  /reconciliation/decisions

TAX
  /tax/configuration
  /tax/submissions
  /tax/eis/terminals

REPORTING
  /reports
  /exports

ADMINISTRATION
  /plans
  /entitlements
  /billing
  /audit
```

## 111.2 Command endpoints

For operations where semantics matter, use action-style endpoints or explicit command resources:

```text
POST /api/v1/sales/complete
POST /api/v1/sales/{sale_id}/return
POST /api/v1/sales/{sale_id}/void
POST /api/v1/payments/{id}/confirm
POST /api/v1/reconciliation/exceptions/{id}/resolve
POST /api/v1/registers/{id}/open
POST /api/v1/registers/{id}/close
POST /api/v1/inventory/transfers/{id}/approve
POST /api/v1/sync/push
POST /api/v1/sync/pull
```

The benefit is semantic isolation. `POST /sales/complete` communicates that a domain transaction is occurring, whereas a sequence of generic table inserts allows clients to assemble invalid partial states.

# 112. Canonical Command Envelope

All significant commands should have a common envelope internally, even if transport-specific DTOs differ.

```json
{
  "commandId": "01K...",
  "tenantId": "01K...",
  "principalId": "01K...",
  "deviceId": "01K...",
  "occurredAt": "2026-09-03T16:30:00Z",
  "schemaVersion": 1,
  "correlationId": "01K...",
  "payload": {}
}
```

`tenantId` inside the payload is advisory metadata. The authoritative tenant comes from authenticated membership and server-side context. This distinction is important for defense-in-depth.

The `commandId` must be globally unique enough for offline generation. The server should maintain a uniqueness constraint over the relevant namespace, normally `(tenant_id, device_id, command_id)` or `(tenant_id, command_id)` depending on command-generation guarantees.

# 113. HTTP Error Contract

Every API error should have a stable machine-readable code.

```json
{
  "error": {
    "code": "INVENTORY_INSUFFICIENT_STOCK",
    "message": "The requested quantity is not currently available.",
    "retryable": false,
    "correlationId": "01K...",
    "details": {
      "skuId": "01K..."
    }
  }
}
```

The human message is not the contract. The code is. Error details should be carefully bounded because a financial system can leak business data through validation responses. For example, a cross-tenant request should normally return a generic not-found or authorization-safe response rather than confirming that a foreign object exists.

# 114. API Request Validation Pipeline

```text
TLS termination
      ↓
request size / content-type checks
      ↓
HTTP parsing
      ↓
authentication
      ↓
principal + device resolution
      ↓
tenant membership resolution
      ↓
coarse rate limit
      ↓
DTO/schema validation
      ↓
authorization policy
      ↓
idempotency validation
      ↓
application command
      ↓
domain invariants
      ↓
database transaction
      ↓
outbox
      ↓
response
```

The ordering matters. Authorization must not be postponed until after a broad database query. Similarly, idempotency should be checked before expensive work when the operation supports it, but the final uniqueness constraint must still exist at the database layer.

# 115. Transaction Boundaries by Capability

| Capability | Transaction expectation | External call allowed inside transaction? |
|---|---|---|
| Catalogue edit | Single aggregate update | No |
| Product import row | Per row or bounded batch | No |
| Sale completion | Multi-module atomic transaction | No |
| Return | Multi-module atomic transaction | No |
| Cash close | Atomic close + variance record | No |
| Payment webhook effect | Inbox insert + effect transaction | No |
| Reconciliation decision | Atomic decision + evidence | No |
| EIS submission | Local state change + outbox | No |
| Report generation | Read-only query, then async object write | No external API in SQL transaction |
| Billing entitlement transition | Atomic billing state | No |

External APIs are a different consistency domain. The application must represent pending states instead of stretching a relational transaction across the network.

# 116. Domain State Machines

The platform should define state machines explicitly rather than relying on an unbounded collection of booleans.

## 116.1 Sale

```text
DRAFT
  │
  ▼
READY
  │ complete
  ▼
COMMITTED
  ├──── RETURNED_PARTIAL
  ├──── RETURNED_FULL
  └──── VOIDED (only when legally/operationally permitted)
```

A committed sale is immutable. The state can evolve through permitted compensating workflows, but the original facts stay intact.

## 116.2 Purchase order

```text
DRAFT
 → SUBMITTED
 → APPROVED
 → SENT
 → PARTIALLY_RECEIVED
 → FULLY_RECEIVED

DRAFT/SUBMITTED/APPROVED/SENT
 → CANCELLED
```

## 116.3 Stock adjustment

```text
REQUESTED
 → APPROVED
 → POSTED

REQUESTED
 → REJECTED

POSTED → CORRECTED_BY_COMPENSATING_ENTRY
```

## 116.4 Reconciliation exception

```text
OPEN
 → INVESTIGATING
 → MATCHED
 → PARTIALLY_MATCHED
 → DISMISSED
 → ESCALATED
 → CLOSED
```

The state machine must reject illegal transitions. A `CLOSED` exception cannot silently be changed to `OPEN` because a second operator clicked an old UI button.

# 117. Aggregate Boundaries

An aggregate is not merely a table. It is the smallest consistency boundary within which an invariant must be enforced synchronously.

Candidate aggregates:

```text
RegisterShift
Sale
PurchaseOrder
GoodsReceipt
InventoryBalance / StockPosition
InventoryTransfer
PaymentIntent
ReconciliationException
Subscription
ApprovalRequest
```

A `Sale` aggregate may contain sale lines and tender intent, but its inventory effects can be modeled as domain postings owned by the inventory module. The application service orchestrates the transaction while each module enforces its own invariants.

Do not create a giant `BusinessAggregate` that contains every entity. That would make all changes contend on one conceptual object and destroy modularity.

# 118. Invariant Catalogue

The following invariants should be treated as executable design requirements.

## Tenant invariants

```text
resource.tenant_id == authorized_context.tenant_id
```

No tenant-scoped repository method should accept a bare object ID and infer authorization later. Repository queries should generally include tenant criteria when accessing tenant-owned records.

## Sale invariants

```text
sale.total == sum(line.extended_total) + adjustments + taxes
sale.status == COMMITTED → financial facts immutable
refund_total <= refundable_total
```

## Inventory invariants

```text
materialized_balance == sum(authoritative_postings)
transfer_in(source) ↔ transfer_out(source)
quarantined stock is not sale-eligible
```

## Payment invariants

```text
provider_event_id unique within provider namespace
one logical payment cannot be confirmed twice
payment amount is exact in the canonical currency representation
```

## Cash invariants

```text
closing_count occurs only for active shift
closed shift accepts no ordinary sales postings
variance = expected_cash - counted_cash
```

## Approval invariants

```text
approved_at != null → approver_id != null
approval cannot be performed by unauthorized role
high-risk approval rules are server-evaluated
```

# 119. Repository Interface Design

Repository traits should express domain intent rather than expose generic query methods.

Bad:

```rust
trait Repository<T> {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<T>>;
    async fn update(&self, value: T) -> Result<()>;
}
```

Preferred shape:

```rust
#[async_trait]
pub trait SaleRepository {
    async fn get_for_update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        sale_id: SaleId,
    ) -> DomainResult<Option<Sale>>;

    async fn insert_committed(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        sale: &CommittedSale,
    ) -> DomainResult<()>;
}
```

The repository exposes the operation needed by the application workflow. This makes it harder to accidentally create a generic update path that bypasses domain semantics.

# 120. Domain Objects Versus Persistence Models

Do not require domain entities to be identical to SQL rows. A database row may contain storage metadata, denormalized fields or audit columns that are not part of the domain object. Conversely, a domain value object may be represented by multiple columns.

Examples:

```text
Money = amount_minor + currency
PostalAddress = normalized structured value
Price = money + price_source + effective interval
StockPosition = quantity + quality state + lot context
```

This separation prevents persistence concerns from infecting every business function.

# 121. Money Type

The backend should define a dedicated money type.

Conceptually:

```rust
struct Money {
    amount_minor: i64,
    currency: CurrencyCode,
}
```

Operations should require compatible currencies. Tax calculations should explicitly choose a rounding mode. No generic `f64` should appear in financial domain types.

# 122. Tax Calculation Isolation

Tax calculation must be independently testable from invoice transmission.

```text
Product tax classification
        +
Customer/transaction context
        +
Tax policy version
        +
Line amount
        ↓
Tax calculator
        ↓
Tax result snapshot
        ↓
Sale / invoice
```

The EIS adapter receives a snapshot. It should not recalculate tax during a retry because the external submission could occur under a different configuration version.

# 123. Pricing Engine Architecture

The pricing engine should return an explainable result.

```json
{
  "unitPriceMinor": 12500,
  "currency": "MWK",
  "appliedRules": [
    {"ruleId": "...", "reason": "BRANCH_PRICE"},
    {"ruleId": "...", "reason": "PROMOTION"}
  ],
  "pricingVersion": 42
}
```

A sale stores this result. Reports can later answer why a price was applied. This is materially better than looking up today's pricing rules and trying to reconstruct yesterday's sale.

# 124. Inventory Allocation Algorithm

For ordinary non-lot stock:

```text
begin transaction
  read eligible stock position
  attempt atomic decrement
  if affected_rows != 1:
      reject insufficient stock / conflict
  insert posting
commit
```

For lot-controlled stock:

```text
begin transaction
  select eligible lots
    where quality_state = SELLABLE
      and expiry policy permits
    order by expiry_date asc, received_at asc
    for update
  allocate requested quantity across rows
  insert one posting per allocation
  update balances
commit
```

The precise SQL should be optimized after schema design, but the algorithmic property is non-negotiable: concurrent allocation must operate on authoritative rows under database concurrency control.

# 125. Inventory Reconciliation

Inventory counts must not overwrite stock.

```text
SYSTEM COUNT = authoritative expected quantity
PHYSICAL COUNT = observed quantity
DIFFERENCE = physical - expected
```

The difference becomes an adjustment request or approved adjustment posting. The original expected value and counted value are retained. Repeated counting creates a history of operational accuracy rather than erasing evidence.

# 126. Returns Architecture

Returns need explicit eligibility rules.

```text
Original sale
   │
   ├── already returned quantity
   ├── refundable quantity
   ├── item condition
   ├── time policy
   └── payment/tender implications
          ↓
      return decision
          ↓
     inventory return posting
          ↓
     refund / store credit / exception
```

Returnable quantity should be computed from the sale's immutable lines plus prior return events, not from a mutable `returned_quantity` field alone.

# 127. Refund Architecture

Refunds can involve a different external provider than the original tender. Therefore the refund state must be independent:

```text
REFUND_REQUESTED
 → PROVIDER_PENDING
 → REFUNDED
 → FAILED
 → MANUAL_REVIEW
```

A successful refund does not alter the original payment event; it creates a linked compensating financial event.

# 128. Void Architecture

A void is not the same as delete. Whether a committed sale can be voided should depend on its lifecycle, tax status, payment state and policy. The architecture therefore represents a void as an explicit event with reason, actor and timestamp. Where external tax or payment systems have already accepted the transaction, the void may require a different compensating process.

# 129. Cash Variance Controls

At register close:

```text
expected cash
= opening float
+ cash sales
+ cash-in
- cash-out
- cash drops
- approved cash corrections

variance
= counted cash - expected cash
```

Variance thresholds can trigger approval. The calculation must be deterministic and reconstructible from postings rather than a single mutable total.

# 130. Customer Data Minimization

The customer module must avoid turning every retail buyer into a full identity record. Most cash sales require no customer profile. Customer records should only exist when they provide operational value: loyalty, receivables, invoice requirement, delivery, warranty, or another explicit purpose.

The schema should support deletion/anonymization of unnecessary attributes without destroying required financial evidence.

# 131. Supplier Data Model

Supplier records should separate identity from commercial relationship:

```text
Supplier
  ├── legal/trading identity
  ├── contacts
  ├── branches/accounts
  ├── payment terms
  ├── supplier product mappings
  └── performance metrics (derived)
```

Supplier performance must be derived from purchase and receipt history, not manually entered.

# 132. Multi-Currency Readiness

Malawi-first deployments can default to MWK, but the core money model should carry a currency code. This allows later regional expansion without rewriting every table. Exchange-rate functionality should not be introduced until a real business requirement exists; it is a separate domain with its own source-of-truth and rate-timestamp concerns.

# 133. Numbering and Receipts

Human-facing numbers need deterministic operational rules.

Examples:

```text
Receipt: branch/register/day/sequence
Purchase order: organization/fiscal-year/sequence
Transfer: branch/year/sequence
Invoice: tax/legal configuration-specific numbering
```

Database IDs remain opaque globally unique IDs. Human numbers are display identifiers. Mixing them creates painful migration and offline-generation constraints.

# 134. Offline Number Generation

A device that can complete offline sales cannot depend on a central numeric sequence for every receipt unless the external tax protocol provides a compatible mechanism. Therefore the architecture needs a carefully designed business-number strategy, potentially using terminal-specific sequence ranges or globally unique invoice identity plus a display number generated under explicit constraints.

For MRA EIS, the official documentation includes terminal-specific configuration and invoice/offline-signature mechanics. The final numbering implementation must follow the current certified protocol rather than inventing an incompatible scheme. citeturn734608search7turn734608search9

# 135. Local SQLite Schema Strategy

The client database should distinguish:

```text
AUTHORITATIVE SNAPSHOTS
  products
  prices
  allowed stock view
  permissions/capabilities snapshot

LOCAL OPERATIONAL STATE
  cart
  locally committed sale
  device settings

SYNC STATE
  outbox commands
  received changes
  checkpoints
  conflicts
```

The client should not mirror every server table. Replicate the smallest projection required for the device's authorized workflows. This both improves performance and limits data exposure.

# 136. Client Repository Architecture

Flutter should use a layered design such as:

```text
Presentation
  ↓
Feature controller / state notifier
  ↓
Use case
  ↓
Repository interface
  ├── Local data source (SQLite)
  └── Remote data source (API/sync)
```

The business UI reads from the repository. A synchronization process updates the local database, which then triggers UI refresh through the normal local data flow. This prevents a fragmented architecture where some screens read SQLite and others read HTTP directly.

# 137. Flutter Feature Module Layout

```text
lib/
  core/
    networking/
    persistence/
    sync/
    auth/
    errors/
    telemetry/
  features/
    sales/
      data/
      domain/
      presentation/
    inventory/
    catalogue/
    cash/
    payments/
    reconciliation/
    reports/
    settings/
  app/
```

Each feature owns its UI and use-case logic. Shared code should be truly shared; avoid a gigantic `utils/` folder that becomes an implicit dependency network.

# 138. Flutter State Management Rule

Use one predictable state-management approach consistently. The chosen framework is less important than the boundary: UI state is not domain state, and domain state is not network state.

For example:

```text
UI state: loading/error/display mode
Domain state: register shift / sale / stock
Sync state: queued/retrying/conflict/acked
```

A single object should not represent all three because changes in network state should not mutate business truth.

# 139. Desktop Tauri Architecture

```text
Tauri shell
  ├── frontend UI (TypeScript)
  ├── command bridge
  ├── printer adapter
  ├── filesystem adapter
  ├── scanner/terminal adapters
  └── secure credential access

Business truth remains on Rust server.
```

Tauri-native commands should be small and capability-scoped. Do not expose arbitrary filesystem or process execution commands to the webview.

# 140. Desktop Printing

Receipt printing is a physical-world integration. The server should generate canonical receipt data; the desktop/mobile client renders or prints it through a device-specific adapter. Printer availability must not determine whether the sale transaction commits.

```text
Sale committed
    ↓
receipt data persisted
    ↓
print requested
    ├── success
    └── printer failure → reprint queue
```

A cashier must be able to reprint a committed receipt without creating another sale.

# 141. Barcode and Scanner Architecture

Scanners typically act as keyboard input or device-level streams. The product should normalize scanner input into a barcode lookup command. A barcode lookup is read-only and should work from the local catalogue while offline. Product creation from an unknown barcode can require authorization and network synchronization.

# 142. Local Encryption Strategy

Sensitive client data should use operating-system secure storage for credentials and encryption-at-rest for the local database where the supported stack can provide it reliably. The architecture must distinguish between application secrets and cached business data. A stolen device should not expose reusable server credentials.

The application should assume that any client can eventually be compromised. Therefore server authorization remains mandatory even if local data is encrypted.

# 143. Sync Queue Storage Semantics

Each queued command should contain:

```text
command_id
schema_version
command_type
payload
created_at
attempt_count
next_attempt_at
last_error_code
state
```

Payloads should be immutable once queued. When the client needs to correct a user mistake before sync, it should create a new local correction operation rather than silently rewriting the original command, unless the original command is still a local draft and not yet treated as a business event.

# 144. Sync Push Algorithm

```text
while commands remain:
  select bounded batch in FIFO/priority order
  mark SENDING transactionally
  POST batch
  for each acknowledgement:
      mark ACKED or terminal state
  for each retryable error:
      set RETRYABLE + backoff
  for each conflict:
      set CONFLICT with actionable resolution data
  persist new server checkpoint
```

If the process terminates after the request is sent but before acknowledgements are persisted, the client retries using the same command IDs. The server's idempotency layer makes this safe.

# 145. Sync Pull Algorithm

```text
checkpoint = local_checkpoint
repeat:
  request next bounded change page
  apply changes transactionally to local read model
  advance checkpoint only after successful local commit
until page complete
```

This ordering prevents the client from advancing its checkpoint before it has persisted the data. A crash therefore causes replay, not silent loss.

# 146. Change Feed Design

The server needs a durable change representation for clients that subscribe to authoritative updates. A basic implementation can use an append-only change log associated with domain events or outbox records.

```text
change_id
aggregate_type
aggregate_id
tenant_id
sequence
schema_version
payload
created_at
```

Per-tenant or per-device sequence semantics should be chosen carefully. A single global sequence is simpler but may create unnecessary coupling. Per-tenant sequence plus an opaque cursor can be more scalable. The server must guarantee monotonic checkpoint semantics within the chosen scope.

# 147. Sync Conflicts User Experience

A conflict should not be represented to a cashier as a database exception. It needs a domain explanation.

Example:

```text
"This stock adjustment could not be applied because the stock was changed
by another authorized device. No adjustment was silently applied.
Review the current stock and submit a new count."
```

Operational language matters because merchants need recovery actions, not stack traces.

# 148. Offline Capability Matrix

| Feature | Offline behavior |
|---|---|
| Browse cached catalogue | Full within cached scope |
| Build cart | Full |
| Complete cash sale | Yes, subject to configured policy |
| Record cash movement | Yes, with role restrictions |
| Product creation | Queue if allowed |
| Price policy change | Restricted or queued with validation |
| Stock transfer | Queue if business policy permits |
| User management | Server required |
| Role/permission change | Server required |
| Tax configuration | Server authority required |
| Provider reconciliation | Local queue/manual capture only; external evidence requires provider connectivity |
| Reports | Cached/basic local reports; authoritative heavy reports may require server |
| Enterprise administration | Online only |

# 149. Offline Data Horizon

Do not cache unlimited history. Define a local data horizon such as:

```text
Current operational data: cached
Recent history: cached selectively
Old history: server fetch required
Large exports: server-generated
```

The exact retention horizon should be measured against device storage and merchant workflow rather than guessed.

# 150. Authentication Offline

A merchant may need to reopen the application without immediate connectivity. This creates an important security decision. The device can retain a locally cached session for limited offline operation only when policy permits, but it must not be treated as proof that the server-side account remains active forever.

A safer design is:

```text
online authentication
  ↓
short offline session lease
  ↓
local capability snapshot
  ↓
offline operations only within cached role scope
  ↓
reauthentication / server refresh required after lease
```

High-risk administrative functions should remain unavailable offline.

# 151. Device Clock Manipulation

Offline systems cannot fully trust the local clock. A malicious user may alter device time to evade thresholds. The architecture should retain both client timestamp and server receive/commit timestamp when a command reaches the server. Offline tax policies that depend on elapsed time must use the protocol's approved semantics; do not invent a client-only security clock.

# 152. Replay Protection

Replay threats exist at several layers:

```text
HTTP retry
provider webhook retry
sync replay
stolen command payload
stale mobile request
```

Each layer needs its own replay control. A single global nonce is not enough. Command IDs protect logical mutations; provider event IDs protect external events; auth tokens and session IDs protect credentials; request IDs primarily support tracing rather than security.

# 153. Webhook Signature Validation

If a provider supports signing:

```text
raw request bytes
   ↓
canonical signature computation
   ↓
constant-time comparison
   ↓
timestamp/replay validation
   ↓
store provider event ID
   ↓
process
```

Signature verification must occur against the exact canonical representation required by the provider. Do not parse JSON and then reserialize it before verification unless the provider explicitly specifies canonical JSON semantics.

# 154. Payment Provider Normalization

The canonical model should include:

```text
provider
provider_account_id
provider_transaction_id
merchant_reference
payment_intent_id
amount
currency
status
observed_at
raw_reference_hash
```

The system should preserve enough provider metadata to investigate a dispute while avoiding unnecessary storage of sensitive payloads.

# 155. Payment Initiation Versus Recording

A manual cashier selection of "Mobile Money" does not equal provider confirmation.

```text
Tender intent selected
       ↓
Payment intent created
       ↓
Provider initiation (optional)
       ↓
Provider confirmation
       ↓
Observed payment
       ↓
Reconciliation
```

For merchants who simply record mobile money manually, the system can create a payment record marked as manually observed, subject to reconciliation workflows. It should never manufacture a provider transaction ID.

# 156. Merchant Reconciliation Workflow

A high-value daily workflow is:

```text
TODAY'S SALES
     +
CASH COUNT
     +
MOBILE MONEY OBSERVED
     +
BANK OBSERVED
     +
OPEN EXCEPTIONS
     ↓
DAY CLOSE
```

The UI should show unmatched amounts as exceptions. A dashboard that displays 'balanced' because it ignored unmatched payments is actively dangerous.

# 157. Reconciliation Run Model

A reconciliation run should contain:

```text
run_id
tenant_id
source_type
source_period
started_at
completed_at
source_hash
records_observed
matches_created
exceptions_created
operator_id / job_id
```

This gives the merchant and Sitolo support an auditable boundary around each statement/import run.

# 158. Import Parser Isolation

CSV/Excel parsers can be a security and reliability risk. Parsing should happen in a bounded worker process/task with:

- file size limit;
- row count limit;
- column count limit;
- timeout;
- memory budget;
- encoding detection policy;
- formula handling policy for spreadsheet formats;
- staged tables;
- antivirus/content scanning if the threat model justifies it.

Never let an imported formula execute in a spreadsheet engine. The application should parse values as data.

# 159. Enterprise API Access

Third-party enterprise integrations should use separate client credentials and scopes, not a merchant user's session token.

Conceptual model:

```text
API Client
  ├── client_id
  ├── secret/key
  ├── tenant binding
  ├── scopes
  ├── rate limits
  └── lifecycle state
```

Credentials need issuance, rotation and revocation. Every API client action must still be tenant-scoped.

# 160. API Idempotency Store

For critical requests, persist an idempotency record:

```text
tenant_id
principal_or_client_id
operation
idempotency_key
request_hash
status
response_reference
created_at
expires_at
```

When the same key is seen:

```text
same hash → return original result
same key + different hash → reject conflict
```

Retention can be bounded for operations whose business duplication risk expires, but financial operations should retain enough history to defend against replay and disputes.

# 161. Database Row-Level Security Position

Application-layer tenant checks remain mandatory even if PostgreSQL RLS is enabled. RLS is a second boundary, not an excuse to write unsafe queries.

A mature implementation may use RLS for high-value tenant-owned tables with a request-scoped database setting such as a tenant claim. This requires careful pool handling: the connection must not carry stale tenant context into the next request.

Therefore:

```text
borrow connection
  ↓
set tenant context inside transaction
  ↓
perform all tenant-scoped work
  ↓
commit/rollback
  ↓
return connection
```

Do not set a global session variable on a pooled connection and forget to reset it.

# 162. Database Role Separation

Use database roles with different capabilities where practical:

```text
migration_role
application_role
reporting_read_role
audit_insert_role
admin_emergency_role
```

The normal application runtime should not have arbitrary DDL or unrestricted administrative privileges.

# 163. Defense Against SQL Injection

Use parameterized SQL through SQLx. Dynamic SQL should be rare and constructed from trusted enumerations, not arbitrary user input.

For dynamic sorting/filtering:

```text
client sort key
  ↓
allowlisted enum
  ↓
trusted SQL fragment
```

Never interpolate a raw column name, table name or direction received directly from the client.

# 164. Object Storage Authorization

Object access should be mediated by the application. A signed object URL should be issued only after verifying tenant access and intended purpose.

Recommended key layout:

```text
/{tenant_id}/{object_class}/{opaque_object_id}/{version}
```

Do not use customer phone numbers, email addresses or human names as object keys.

# 165. Malware and Unsafe Uploads

For files such as receipts, logos, imports and documents, perform content-type verification and size checks. Do not trust the filename extension or client MIME type. Executable content should not be served from the same origin as the application UI.

# 166. Web Security Boundary

The TypeScript web/admin surface must follow a separate browser security checklist:

```text
Content-Security-Policy
HSTS
secure cookies
SameSite policy
CSRF protection when cookie auth is used
strict origin validation
frame-ancestors protection
safe download handling
```

The backend remains responsible for authorization. A hidden admin button is not an authorization control.

# 167. Mobile Secure Storage

Use platform secure storage for long-lived secrets/refresh material. Avoid storing credentials in plain SQLite tables. Cached business data may be stored locally, but it should be scoped to the installed device and tenant context.

# 168. Threat Scenario: Cross-Tenant Object Guessing

Scenario:

```text
attacker knows sale_id from another tenant
    ↓
GET /sales/{sale_id}
```

Required defenses:

1. Authorization resolves tenant membership before query execution.
2. Repository query includes tenant ownership.
3. Database constraint/RLS provides a secondary barrier where enabled.
4. Response should not reveal whether the ID exists in another tenant.
5. Regression test asserts no information disclosure.

# 169. Threat Scenario: Offline Command Theft

Scenario: a malicious actor copies an offline command payload from a compromised device and submits it directly.

Controls:

```text
authenticated device credential
+ command ID uniqueness
+ tenant binding
+ schema/version checks
+ server-side business authorization
+ timestamp/age checks
+ device revocation
+ anomaly detection
```

Offline commands must be treated as untrusted assertions, not trusted transactions.

# 170. Threat Scenario: Duplicate Refund

Scenario: the cashier taps refund twice, network retries, provider retries, and two application workers see the request.

Required controls:

```text
user intent ID
payment/refund idempotency key
DB uniqueness
provider idempotency if supported
worker claim/lease
state machine transition guard
```

The invariant is that one logical refund produces one accepted effect.

# 171. Threat Scenario: Privilege Escalation Through Branch Selector

A cashier posts a command using another branch ID where they have no assignment.

Control:

```text
principal
  ↓
effective permissions
  ↓
branch scope evaluation
  ↓
resource lookup constrained by tenant + branch
```

The branch ID in the request is only a requested target, never authorization evidence.

# 172. Threat Scenario: Malicious Manager Export

An authorized manager may still be able to exfiltrate too much data. Therefore data export should be its own permission, potentially requiring higher approval for sensitive datasets. Every export is audited.

# 173. Threat Scenario: Provider Credential Exposure

Credentials must exist only on server-side integration workers. Mobile builds contain no provider secret. CI secrets are scoped to deployment environments. Logs redact authorization headers and sensitive fields.

# 174. Rate Limit Dimensions

A single global rate limit is insufficient. Use multiple dimensions:

```text
IP address
principal
organization/tenant
API client
endpoint class
provider destination
```

Financial operations should use stricter concurrency and idempotency controls than catalogue reads.

# 175. Abuse Detection Signals

Useful operational signals include:

- high failed-login rates;
- multiple device registrations from unusual patterns;
- repeated permission denials;
- refund spikes;
- unusual void frequency;
- rapid export generation;
- repeated reconciliation overrides;
- many sync conflicts from one device;
- sudden stock adjustments.

These are detection inputs, not automatic evidence of fraud.

# 176. Logging Policy

Logs should prefer:

```json
{
  "timestamp": "...",
  "level": "INFO",
  "service": "sitolo-api",
  "trace_id": "...",
  "tenant_id_hash": "...",
  "principal_id": "...",
  "operation": "sale.complete",
  "result": "success",
  "duration_ms": 142
}
```

Do not log full access tokens, card data, provider secrets, passwords, recovery tokens or unbounded customer payloads.

# 177. Logging and Tenant Privacy

Even logs can become a cross-tenant data leak. If support engineers use centralized logs, tenant references must be carefully scoped. Access to tenant-identifying log searches should be audited. Retention should be shorter than the retention of core business evidence where possible.

# 178. Metrics Cardinality Rules

Never use raw customer names, full sale IDs or arbitrary user input as unbounded metric labels. High-cardinality dimensions belong in traces/logs, not time-series metric labels.

Good:

```text
sales_completed_total{region="mw",result="success"}
```

Bad:

```text
sales_completed_total{sale_id="01K..."}
```

# 179. Alert Design

Alerts should correspond to actions. Examples:

```text
Critical:
  database unavailable
  repeated payment webhook verification failures
  high cross-tenant authorization failure rate

High:
  sync rejection spike
  EIS rejection spike
  job dead-letter growth
  backup failure

Warning:
  increased latency
  report queue depth
  storage growth
```

A metric without a response path is not a useful alert.

# 180. Job Scheduling Model

Jobs should support:

```text
job_id
job_type
payload
status
priority
attempts
available_at
locked_until
worker_id
last_error
created_at
completed_at
```

A worker claims jobs using a transaction-safe lease. A crashed worker eventually loses the lease and another worker can retry.

# 181. Queue Fairness

The worker system should avoid a large export consuming all capacity needed for financial jobs.

Conceptual queues:

```text
critical transactional side effects
high-priority integration retries
normal notifications
bulk reports/exports
maintenance
```

Workers can be separately bounded by class even when all use the same PostgreSQL-backed job store.

# 182. Worker Idempotency

A worker should be designed as though it can run twice. The safest pattern is:

```text
read durable state
check whether effect already exists
perform idempotent operation
record completion
```

Where the external provider supports its own idempotency key, propagate the stable operation ID.

# 183. Transactional Outbox Schema

Conceptually:

```text
outbox_id
aggregate_type
aggregate_id
tenant_id
event_type
payload
schema_version
available_at
attempt_count
status
created_at
processed_at
last_error
```

The outbox row must be inserted in the same database transaction as the domain state it describes.

# 184. Outbox Retention

Processed outbox rows should not grow forever. Retain enough for operational replay/forensics, then archive or delete according to the recovery model. If the outbox is also serving as a client change log, that retention policy becomes different and should be modeled separately rather than accidentally coupling two lifecycles.

# 185. Event Schema Evolution

Events should be versioned:

```text
sale.completed.v1
sale.completed.v2
```

Consumers should be able to handle supported versions explicitly. Do not silently change the meaning of an existing event type.

# 186. Integration Contract Testing

For each provider adapter:

```text
canonical request → provider request mapping
provider response → canonical state mapping
failure mapping
retry classification
signature validation
idempotency behavior
```

Sandbox tests and recorded fixtures can be used where contractual and legal constraints allow. Production tests must never send synthetic financial transactions to real merchant accounts without explicit authorization and controls.

# 187. MRA EIS Integration Detail

The official MRA EIS API documentation describes an onboarding flow based on terminal acquisition and activation, periodic configuration retrieval, sales reporting and utility endpoints. It also documents a terminal-specific offline mechanism and submission of locally stored offline transactions when connectivity is restored. citeturn734608search3turn734608search7turn734608search6

The architecture should model:

```text
MRA terminal
  ├── external terminal ID
  ├── activation state
  ├── configuration version
  ├── taxpayer association
  └── offline thresholds/configuration
```

This data is stored as an external integration projection. It is not treated as editable ordinary merchant configuration once provisioned by MRA.

# 188. MRA EIS Configuration Refresh

MRA documentation states that terminals should retrieve current configuration and that the response can indicate configuration changes; terminal configuration includes taxpayer and terminal details and offline limits. Therefore the adapter should retain configuration versions and update local integration state in a controlled transaction before future submissions use the new state. citeturn734608search16

A configuration refresh should not rewrite historic sale/tax snapshots. Historic submissions retain the configuration/effective-version reference under which they were produced.

# 189. MRA Offline Receipt Signing Boundary

The official documentation describes an offline-signing mechanism using terminal-specific secret material and transaction information to construct an HMAC-based offline validation URL. The implementation should encapsulate this in the EIS adapter and never expose the terminal secret to general application code. citeturn734608search9

Security consequence:

```text
MRA terminal secret
        ↓
isolated tax/EIS component
        ↓
offline signature
        ↓
receipt QR representation
```

The general sales module should request a tax signature artifact rather than receiving raw secret material.

# 190. MRA EIS Offline Submission

MRA documentation states that offline transactions are stored locally and later uploaded with the offline signature when connectivity is restored. Sitolo therefore needs a dedicated tax submission queue rather than treating tax reporting as a synchronous side effect of a sale. citeturn734608search10

The queue should preserve:

```text
sale_id
terminal_id
submission_payload_hash
configuration_version
attempts
first_attempt_at
last_attempt_at
state
external_response_reference
```

# 191. Tax Integration Failure Domains

A tax failure should not cause unrelated platform failure.

```text
sale DB transaction: SUCCESS
payment recording: SUCCESS
inventory posting: SUCCESS
EIS submission: PENDING
```

That state is operationally healthy if the pending queue is within the documented policy window. The UI must show the distinction.

# 192. Payment Reconciliation Versus Accounting

Sitolo should provide strong operational reconciliation without automatically claiming to be a full accounting ledger. The reconciliation domain answers:

```text
Did money expected by the business arrive?
Which external observation corresponds to it?
What remains unresolved?
```

A future accounting integration answers:

```text
Which account should this event post to?
What is the journal entry?
What financial statement does it affect?
```

Keeping these boundaries prevents scope creep while preserving a clean integration path.

# 193. Accounting Integration Adapter

Future accounting adapters should consume normalized business events and mappings:

```text
sale → revenue account
sale tax → tax liability
payment → cash/bank/mobile-money account
COGS → cost account
inventory adjustment → adjustment account
```

These mappings should be tenant-configurable and versioned. The accounting adapter should not modify Sitolo's source transaction facts.

# 194. Enterprise Data Warehouse Boundary

When analytics outgrows PostgreSQL, export immutable or derived events asynchronously:

```text
PostgreSQL operational events
     ↓
outbox / CDC / export job
     ↓
data ingestion
     ↓
warehouse
     ↓
BI/analytics
```

No analytics query should require direct access to production transactional tables once the warehouse becomes authoritative for analytical workloads.

# 195. Data Lineage

Reports and exports should be traceable to source records. A report definition should record:

```text
source datasets
filters
as-of timestamp
calculation version
query/report version
```

This becomes important when two reports disagree and the business needs to understand whether the discrepancy is due to timing, filters, business rules or data corruption.

# 196. Performance Budget by Layer

Initial performance budgets can be expressed as a rough target rather than a guarantee:

```text
Flutter local query             < 20 ms typical
POS calculation                < 20 ms typical
Rust request parsing            < 10 ms typical
Database transaction            < 100 ms target under normal load
Total online sale endpoint     < 500 ms p95 excluding external wait
Sync page processing            bounded by batch size
```

The point is to expose where latency originates. Real production measurements should override theoretical budgets.

# 197. Large Tenant Isolation

A single high-volume tenant can become a noisy neighbor. The architecture should support quotas and eventually workload isolation:

```text
standard tenants → shared pool
large tenant      → reserved worker/concurrency quota
enterprise tier   → optional dedicated database/compute
```

This should be introduced from measured workload evidence, not sales-tier mythology.

# 198. Read Replicas

Read replicas become useful only when read workloads materially compete with transactional writes. They are suitable for:

- management dashboards;
- historical reports;
- search-like queries;
- support read operations.

They are not suitable for authoritative reads immediately after a critical write when stale results would mislead the user.

# 199. Read-After-Write Semantics

After a sale commit, the cashier should see the committed sale from the authoritative path. The system must not send the request to the primary and then immediately read a replica that has not caught up. Where replicas exist, use primary reads for critical immediate confirmation or a consistency token/watermark mechanism.

# 200. Database Connection Pool Policy

A pool should be sized from:

```text
DB server max connections
× reserved headroom
× number of application replicas
```

and validated under load. More connections do not necessarily mean more throughput. PostgreSQL contention, CPU and disk I/O can degrade when concurrency becomes excessive.

# 201. Async Runtime Hygiene

Rust async code should follow these rules:

```text
no blocking filesystem/CPU work on async reactor threads
no unbounded task spawning from untrusted input
all external calls have deadlines
all channels are bounded
all loops have cancellation paths
```

Tokio's strengths are realized only when the application does not accidentally turn asynchronous workers into unbounded task factories.

# 202. Cancellation Safety

Every request and job should be cancellable where practical. If an HTTP request is cancelled, a database transaction should roll back. If a long-running export is cancelled, the job should leave an explicit cancellation state and clean temporary resources.

Cancellation is especially important for mobile clients that frequently lose network connections.

# 203. Backpressure Model

```text
incoming requests
      ↓
bounded application concurrency
      ↓
database pool
      ↓
bounded outbox workers
      ↓
bounded external calls
```

When downstream capacity is exhausted, the platform rejects or defers work predictably rather than accumulating unlimited memory queues.

# 204. Memory Budgeting

Rust protects against classes of memory-safety bugs, but allocation volume can still be excessive. Avoid loading giant reports or imports into memory. Use streaming/iterative processing, bounded buffers and pagination. Large payloads should move through object storage or asynchronous jobs.

# 205. Report Execution Budget

A report query should have an execution budget. For synchronous reports:

```text
small result set + indexed path → synchronous
large scan / aggregation       → async job
```

If a synchronous report crosses its budget, the application should fail into an asynchronous workflow rather than allowing one request to consume an arbitrary amount of database CPU.

# 206. Search Strategy Progression

Search can evolve:

```text
PostgreSQL B-tree/indexes
      ↓
PostgreSQL full-text / trigram where useful
      ↓
read model / denormalized search table
      ↓
external search engine only when justified
```

The product should earn the complexity of a search cluster through observed requirements.

# 207. API Compatibility Policy

Public APIs need deprecation windows. A version should be supported until clients have a reasonable migration path. Mobile apps complicate this because not all users upgrade immediately. The server should therefore support a compatibility window across client versions.

# 208. Mobile Client Version Policy

Each client should declare:

```text
app version
schema version
sync protocol version
API compatibility version
```

The server may reject clients that are below the minimum supported protocol. Rejection should provide a safe upgrade path rather than a generic 500 error.

# 209. Migration Compatibility Matrix

Example:

| Client | API | Sync schema | Status |
|---|---|---|---|
| N | v1 | v5 | supported |
| N-1 | v1 | v4 | supported |
| N-2 | v1 | v3 | grace period |
| older | v0 | old | blocked |

This is especially important after changes to offline command payloads.

# 210. Schema Rollout Strategy

For changes affecting offline clients:

```text
1. add server support for old + new
2. release compatible client
3. observe adoption
4. migrate/backfill data
5. stop issuing old schema
6. remove old support later
```

Do not deploy a server that immediately rejects commands generated by versions still widely deployed.

# 211. Blue/Green Versus Rolling Deployment

A rolling deployment is generally sufficient for the stateless Rust API if compatibility rules are followed. Blue/green may be useful for high-risk infrastructure changes. Database migrations remain the constraint: code must tolerate the schema during the transition.

# 212. Zero-Downtime Database Migration Pattern

```text
old code
   ↓
expand schema
   ↓
dual-read if necessary
   ↓
new code
   ↓
backfill
   ↓
verify
   ↓
contract old schema
```

Never combine an irreversible destructive migration with the same deployment that first introduces code depending on it.

# 213. Canarying

High-risk releases can be canaried to a small percentage of tenants or internal tenants. Canary selection must be deterministic and should avoid regulated/high-volume customers until the release proves stable.

# 214. Rollback Policy

Rollback is not always database rollback. A migration may be forward-only. Therefore release design must answer:

```text
Can code be rolled back?
Can schema be rolled back?
Can events already emitted be replayed?
Can external side effects be compensated?
```

For financial systems, forward correction is usually safer than destructive rollback.

# 215. Disaster Recovery Validation

A real restore exercise should verify:

```text
DB restore
migrations
application startup
object-store references
job queue continuity
provider credential availability
tenant login
sample sale read
sample report
sync checkpoint behavior
```

A database that restores but cannot restart the workers is not a successful recovery.

# 216. Business Continuity During Outage

Because the mobile client has offline capability, the product has a layered continuity model:

```text
normal network
  ↓
server outage / network loss
  ↓
local offline operation
  ↓
queued sync
  ↓
recovery
  ↓
authoritative convergence
```

The system should communicate what is locally committed versus externally confirmed.

# 217. Merchant-Facing Health Indicators

The mobile app can surface simple operational health states:

```text
ONLINE — synced
CONNECTED — syncing
OFFLINE — operating locally
ACTION REQUIRED — conflicts/exceptions
TAX PENDING — tax submissions queued
PAYMENT EXCEPTIONS — reconciliation needs attention
```

Avoid exposing low-level infrastructure terms such as 'PostgreSQL replica lag' to a cashier.

# 218. Enterprise Admin Health View

The support/admin view can expose deeper signals:

```text
Tenant
  API health
  last device sync
  pending commands
  reconciliation exceptions
  tax queue
  payment integrations
  backup health
  last active user
```

This enables support to solve operational problems without direct database queries.

# 219. Product Analytics Event Boundary

Product analytics should never become the source of financial truth. It can record:

```text
feature_opened
workflow_started
workflow_completed
sync_conflict_viewed
report_opened
```

but the actual sale/payment/stock records remain domain data. Analytics events should be privacy-minimized.

# 220. Tenant Data Export Architecture

A tenant export should produce a consistent snapshot or documented point-in-time view.

```text
export request
   ↓
authorization
   ↓
record snapshot watermark
   ↓
async export
   ↓
checksum
   ↓
short-lived download link
   ↓
audit record
```

Without a snapshot/watermark, a multi-file export could contain inconsistent cross-table states.

# 221. Tenant Offboarding

Offboarding needs an explicit state machine:

```text
ACTIVE
 → SUSPENDED
 → EXPORT_READY
 → RETENTION
 → PURGED / ANONYMIZED
```

Suspension should stop new transactional access without necessarily deleting data. Export and legal retention requirements must be satisfied before destructive processing.

# 222. Subscription Suspension

Billing suspension must not accidentally delete merchant data. Entitlement logic should distinguish:

```text
ACTIVE
PAST_DUE
GRACE_PERIOD
LIMITED
SUSPENDED
CANCELLED
```

The exact behavior—read-only, sales blocked, reports retained—must be a commercial policy implemented through capabilities, not a database deletion action.

# 223. Entitlement Evaluation

A server-side capability evaluation may look like:

```text
plan
+ add-ons
+ vertical configuration
+ organization status
+ branch count
+ user count
+ feature flag
+ region
+ compliance status
→ effective capabilities
```

The result can be cached but must be invalidated when a relevant input changes.

# 224. Feature Gating and Data Model

Do not create different schemas per subscription plan. The schema should represent the full platform model while entitlement controls access. This keeps upgrades additive and avoids data migration whenever a merchant moves plans.

# 225. Platform Administration Separation

Sitolo's own employees need separate platform identities from merchant user identities. A platform support account should not masquerade as a merchant user. Use a distinct principal type or realm and explicit tenant-access grants.

# 226. Break-Glass Access

Emergency administrative access should require:

```text
reason
approver or incident reference
short expiration
minimal scope
audit event
post-event review
```

Break-glass should be rare and reviewable. It is not the normal support workflow.

# 227. Security Incident Flow

```text
Detection
  ↓
triage
  ↓
containment
  ↓
evidence preservation
  ↓
root-cause analysis
  ↓
remediation
  ↓
verification
  ↓
customer/regulatory communication as required
```

The architecture should preserve the evidence needed for this flow through audit events, logs and durable external-reference records.

# 228. Dependency Upgrade Policy

The Rust stack should follow a controlled upgrade workflow:

```text
new dependency/compiler release
  ↓
security/release-note review
  ↓
upgrade branch
  ↓
full CI
  ↓
integration tests
  ↓
performance checks
  ↓
staging
  ↓
production
```

Rust 1.98.0 is the current stable release listed in the official release feed as of this architecture's preparation date. The exact patch level used by the repository should be pinned and upgraded intentionally. citeturn244519search0turn244519search3

# 229. Dependency License and Provenance

Enterprise customers may require dependency inventories. Track license and provenance metadata for direct and important transitive dependencies. Avoid dependencies with unclear maintenance or licensing risk when an established alternative exists.

# 230. Vulnerability Response

A dependency vulnerability should be classified by exploitability in Sitolo's architecture. Not every CVE has equal risk. However, networking, parsing, authentication, cryptography and serialization dependencies receive elevated priority.

The response workflow:

```text
advisory
 ↓
affected component analysis
 ↓
exploitability assessment
 ↓
patch/mitigation
 ↓
regression test
 ↓
release
```

# 231. Software Bill of Materials

For mature enterprise releases, generate an SBOM for backend and desktop artifacts. The purpose is incident response and customer transparency, not marketing. SBOM generation should occur in CI from the exact build inputs.

# 232. Reproducible Builds

The release pipeline should record:

```text
source commit
Rust toolchain
Cargo lockfile hash
base image digest
frontend dependency lockfile
build configuration
artifact checksum
```

A production binary should be traceable to an immutable source state.

# 233. Artifact Signing

Release artifacts should be signed where the deployment and distribution platform supports it. Desktop installers are especially important because merchants may install software outside a managed store.

# 234. Secrets in CI

CI should use environment-scoped secrets. Build jobs that do not need production secrets should not receive them. Pull requests from untrusted forks must not have access to production credentials.

# 235. Static Analysis

Rust CI should include formatting and linting. Security-oriented static analysis should complement, not replace, tests. The team should enforce a policy for `unsafe` blocks: minimize them, isolate them, document the safety invariant, and review them more strictly.

# 236. Unsafe Rust Policy

Sitolo should have a default posture of `unsafe`-free application code. When an unsafe block is genuinely required by a low-level dependency or optimized implementation, it should be isolated behind a small safe abstraction and accompanied by a safety comment explaining the invariant.

# 237. Panic Policy

Request paths should not rely on panics for expected business conditions. `unwrap`/`expect` usage should be limited to invariants established at initialization or truly impossible states. Production code must convert recoverable failures into structured errors.

# 238. Serialization Stability

Persisted JSON or event payloads should use explicit field names and versioning. Avoid depending on default serializer behavior for long-lived external contracts. Enum evolution should be forward-compatible where reasonable; unknown values should not crash the entire sync process.

# 239. API DTO Stability

Domain structs should not be exposed directly as API response models. DTOs should be intentionally designed so database changes do not accidentally become breaking API changes.

# 240. Test Data Strategy

Use several fixture classes:

```text
minimal valid tenant
multi-branch tenant
pharmacy tenant
agro tenant
high-volume tenant
malformed client/device
provider failure fixtures
```

Security tests should include cross-tenant fixture data specifically so accidental unrestricted queries fail loudly.

# 241. Golden Tests for Receipts

Receipt output should have golden/snapshot tests for critical formats. Test:

```text
normal sale
discount
multiple tenders
tax
offline receipt
return/refund indication
long product names
large totals
multi-line customer data
```

This is especially important if EIS QR/validation information has formatting requirements.

# 242. Property Tests for Money

Properties:

```text
adding zero preserves value
currency mismatch is rejected
rounding is deterministic
sum(parts) respects selected rounding policy
negative values require explicit semantic type
```

Avoid allowing a generic `Money` constructor to accept arbitrary floating-point input.

# 243. Property Tests for Inventory

Generate random valid movement sequences and assert:

```text
balance == Σ postings
transfer source decrement == destination increment
correction preserves previous history
no impossible lot state appears
```

This can uncover errors that example-based tests miss.

# 244. Property Tests for Idempotency

For any command `C`:

```text
apply(C) + apply(C)
```

must produce the same durable state as a single logical application, modulo harmless retry metadata. This should be tested across process restarts where practical.

# 245. Fuzzing Boundaries

Fuzz:

- JSON parsing;
- CSV/import parsing;
- sync command decoders;
- provider response parsers;
- receipt rendering inputs;
- identifier parsing.

Fuzz tests are particularly valuable for binary/structured input boundaries.

# 246. Failure Injection Tests

Simulate failure between every major step:

```text
before DB commit
right after DB commit
before outbox enqueue
before provider call
after provider call before result persistence
before sync acknowledgement
```

The goal is to prove recoverability from ambiguous intermediate states.

# 247. Chaos Scenarios

Useful controlled chaos scenarios:

```text
kill worker mid-job
restart API during sync
drop provider network
slow PostgreSQL
force duplicate webhooks
expire device session
corrupt one local sync page
```

The expected outcome should be explicit for every scenario.

# 248. Staging Environment Design

Staging should resemble production in:

- PostgreSQL version family;
- TLS configuration;
- deployment packaging;
- job architecture;
- object storage semantics;
- observability;
- integration contracts.

It can still use fake/sandbox external credentials.

# 249. Data Seeding

Seed only synthetic data. Never clone production customer data into general development environments. A dedicated sanitized support environment can exist under stronger access controls when genuinely necessary.

# 250. Test Tenant Isolation

CI should create at least two tenants with similar IDs, users, products and sale structures. Tests deliberately attempt object access across boundaries. This catches a common bug where queries accidentally omit tenant filtering because the test dataset only contained one tenant.

# 251. Security Regression Corpus

Maintain a corpus of previously discovered vulnerabilities and failed test cases. Every resolved security issue should produce a regression test so the same class does not return later.

# 252. Performance Regression Corpus

Maintain representative fixtures:

```text
10 products / 100 products / 10k products
1 branch / 10 branches / 100 branches
100 sales / 100k sales / 10m historical rows
small reconciliation / large reconciliation
```

Benchmark the functions and SQL queries most likely to degrade as the business grows.

# 253. Database Index Review

Indexes should be justified by query patterns. Common candidates:

```text
(tenant_id, created_at)
(tenant_id, branch_id, created_at)
(tenant_id, sku_id, location_id)
(tenant_id, provider, provider_transaction_id)
(tenant_id, status, updated_at)
```

The exact index set must come from observed queries. Too few indexes cause latency; too many increase write cost and storage.

# 254. Partial and Covering Indexes

Use partial indexes for active/open records when appropriate. Covering indexes may reduce table access for critical read paths, but should be introduced only after measuring query plans.

# 255. Query Plan Review

Every high-frequency query should have a reviewed execution plan in representative data. A query that is fast on 1,000 rows can become dangerous at 10 million rows.

# 256. N+1 Query Prevention

The application layer should avoid per-record database calls in report/admin screens. Use joins, batched queries or explicit data loaders. The repository abstraction should make expensive patterns visible in code review.

# 257. Transaction Isolation

PostgreSQL's default isolation is often sufficient, but some workflows may require stronger isolation or explicit row locks. Isolation should be chosen per invariant rather than globally upgraded without cause. Overuse of serializable transactions can create unnecessary retry complexity.

# 258. Deadlock Handling

Any transaction that can contend with another transaction must use deterministic lock ordering. Where deadlocks remain possible, classify and retry the transaction safely. Financial commands must be idempotent so transaction retries cannot duplicate effects.

# 259. Long-Running Read Avoidance

Large report scans should not hold open transactions indefinitely on the primary. Use snapshot/report infrastructure appropriate to scale. Long-running transactions can delay vacuum and increase storage pressure.

# 260. Postgres Maintenance

Production operations should monitor autovacuum, table bloat, index bloat, dead tuples, checkpoint behavior and WAL growth. These are not DBA-only concerns in an architecture that relies on PostgreSQL for business truth.

# 261. PostgreSQL High Availability

When the business reaches the point where database failure materially threatens continuity, use managed PostgreSQL HA/failover or an equivalent mature operational setup. Do not build a custom database failover cluster before the product needs it.

# 262. Connection Failover Semantics

After database failover, the application must expect connection invalidation and retry safe operations. In-flight transactions may fail. The application should not assume a network reconnect means a transaction committed.

Idempotency is therefore also part of database-failover correctness.

# 263. Object Storage Lifecycle

Objects should move through explicit lifecycle states:

```text
UPLOADING
 → AVAILABLE
 → RETAINED
 → EXPIRED
 → DELETED
```

Deletion should be coordinated with database metadata and legal retention rules.

# 264. Notification Architecture

Notifications are advisory. They should never be the only evidence that an important business state occurred.

```text
business event
    ↓
notification intent
    ↓
provider delivery
    ↓
delivered / failed / retry
```

A failed SMS cannot undo a sale.

# 265. Push Notification Security

Do not put sensitive financial information in push notification payloads. Use a notification reference and let the authenticated client fetch the actual protected record.

# 266. Email Security

Exports and password-recovery links should be short-lived and signed. Sensitive business data should not be embedded directly in email when a secure authenticated view is more appropriate.

# 267. SMS Constraints

SMS is suitable for short operational alerts, not as a secure authentication channel unless the authentication model explicitly accepts its risk. Country/provider delivery constraints must be handled by the notification adapter.

# 268. Audit Event Taxonomy

Use stable verbs:

```text
CREATE
UPDATE
APPROVE
REJECT
VOID
REFUND
REVERSE
EXPORT
LOGIN
LOGOUT
DEVICE_REGISTER
DEVICE_REVOKE
ROLE_ASSIGN
ROLE_REVOKE
RECONCILE
SYNC_ACCEPT
SYNC_REJECT
TAX_SUBMIT
TAX_ACCEPT
TAX_REJECT
```

The taxonomy should be finite and documented. Arbitrary free-text action strings make audit analysis difficult.

# 269. Audit Before/After Data

For low-sensitivity mutable configuration, storing a normalized before/after representation is useful. For sensitive data, store redacted summaries or field-change metadata rather than the entire payload. The policy should be field-aware.

# 270. Audit Correlation

Every audit event should be correlatable to a request, command or job. For scheduled system actions, `principal_id` may be a system principal with a job ID. The operator should never have to guess which automated process changed a record.

# 271. Legal Hold

A tenant or record under legal hold must be exempt from normal destructive retention. The feature should be highly restricted and heavily audited because it affects the retention boundary.

# 272. Data Residency Strategy

Start with a single region appropriate for the initial market and operational capability. Architecture should permit regional deployment later, but data residency requirements for each expansion market must be analyzed before launch. Do not promise regional residency merely because the application is cloud-hosted.

# 273. Regional Deployment Model

Future:

```text
Global control plane
     │
     ├── Malawi region
     ├── Zambia region
     ├── Tanzania region
     └── future regions
```

This model is only appropriate after legal, operational and commercial requirements justify it. Early Sitolo should avoid distributed multi-region writes because they complicate inventory and financial consistency.

# 274. Single-Region Strong Core

For the initial architecture, a single authoritative regional database is preferred. Clients across the region can be offline; the server does not need multi-master replication. This substantially simplifies financial consistency.

# 275. Regional Provider Abstraction

A provider registry should map capabilities to local implementations:

```text
country
currency
payment provider set
tax authority adapter
notification provider set
```

The domain calls capabilities rather than provider-specific names whenever possible.

# 276. Country Tax Policy Boundary

Tax policy should be data/configuration + versioned rules under the relevant jurisdiction. Code should provide calculation primitives, while jurisdiction configuration defines rates/categories where safe. Complex statutory logic must be reviewed with tax professionals before commercialization.

# 277. Regulatory Configuration Versioning

Tax/pharmacy configuration should include:

```text
jurisdiction
policy_version
effective_from
effective_to
source_reference
approved_at
approved_by
```

A transaction snapshots the effective policy version used at time of processing.

# 278. Pharmacy Security Boundary

Controlled medicine workflows require stronger permissions and traceability. The architecture should make a controlled workflow more restrictive than ordinary retail sales, not simply add a checkbox called `is_controlled`.

# 279. Pharmacy Batch Lifecycle

```text
RECEIVED
 → QUARANTINED
 → RELEASED
 → SELLABLE
 → EXPIRED / RECALLED / DESTROYED
```

A quality-state transition must be authorized and auditable.

# 280. Pharmacy Recall Workflow

```text
recall identified
   ↓
mark affected lots
   ↓
block normal sale allocation
   ↓
identify branches/locations
   ↓
quarantine stock
   ↓
record disposition
   ↓
retain evidence
```

The architecture should make the recall footprint queryable by lot, supplier and branch.

# 281. Agro-Dealer Seasonal Controls

Seasonal product policies can use effective windows, but expiry and regulatory constraints remain authoritative. The platform may warn about aging stock; it must not claim agronomic validity without authoritative domain data.

# 282. Wholesale Order/Dispatch Boundary

Wholesale order processing can extend the sales domain:

```text
QUOTE
 → ORDERED
 → APPROVED
 → ALLOCATED
 → PICKED
 → DISPATCHED
 → INVOICED
 → PAID / OUTSTANDING
```

Retail POS sales may skip most of these stages. Both paths ultimately produce the same authoritative sale/inventory/payment semantics.

# 283. Receivables Boundary

Accounts receivable may be introduced as a separate financial-control capability. It should track obligations and settlements without turning every sale into a loan. Credit approval and lending-like behavior are distinct and should be separately governed.

# 284. Merchant Finance Expansion Guardrail

Sitolo can become a financial-control platform without becoming a regulated lender or wallet. The architecture should therefore expose normalized cash/payment/reconciliation history while leaving custody, lending and underwriting to separately governed products and partners.

# 285. Partner Integration Marketplace

Long-term, partners can integrate through a controlled API registry. Each integration should declare:

```text
permissions
webhook endpoints
data fields
rate limits
tenant scope
commercial agreement
lifecycle
```

No partner receives unrestricted database access.

# 286. Data Sharing Contracts

Partner APIs should follow data minimization. A partner asking for customer data must have a defined purpose and consent/legal basis where required. The integration registry should record what data classes are shared.

# 287. API Audit for Partners

All partner access should be attributable to a client credential and tenant. Log:

```text
client_id
scope
endpoint
tenant
request/result class
timestamp
rate-limit outcome
```

# 288. SDK Strategy

A TypeScript SDK may be generated from OpenAPI for web/enterprise developers. A future Rust SDK can be generated or hand-maintained for internal tooling. SDKs must never become alternate domain implementations; they are transport clients.

# 289. OpenAPI Governance

The API contract should be generated from or validated against the Rust API definitions. CI should detect accidental breaking changes. Public documentation must reflect the deployed contract, not a manually maintained document that can drift.

# 290. API Documentation Layers

Maintain:

```text
reference documentation — exact API behavior
how-to guides — workflows
architecture docs — internal decisions
runbooks — operational recovery
```

Mixing these into one enormous API page increases maintenance cost.

# 291. Local Development Environment

Recommended local topology:

```text
Rust API
Rust worker
PostgreSQL
optional Redis
S3-compatible emulator
mock payment adapters
mock MRA adapter
observability collector optional
```

The developer should be able to boot the core stack with a single documented command or script.

# 292. Seed Data Profiles

Provide seed profiles:

```text
basic_duka
small_retail
pharmacy
agro_dealer
multi_branch
```

These are not production fixtures; they accelerate development and acceptance testing.

# 293. Local Provider Mocks

Provider mocks should model failure, not just success:

```text
success
slow success
timeout
invalid signature
duplicate event
provider rejection
partial settlement
```

A fake provider that only returns 200 responses teaches the wrong failure model.

# 294. Contract Snapshot Testing

For external providers and MRA EIS, retain sanitized request/response snapshots when licensing/contract rules allow. Compare adapter output after dependency upgrades to detect accidental mapping changes.

# 295. Versioned Integration Adapters

Do not modify `MraEisV1Adapter` in ways that silently change behavior for old protocol assumptions. Create a new adapter version when the external protocol meaningfully changes:

```text
EisV1Adapter
EisV2Adapter
```

A registry chooses which adapter applies to the tenant/terminal configuration.

# 296. Integration Timeout Policy

Each provider integration should define:

```text
connect timeout
request timeout
retryable status codes
max retries
backoff
circuit threshold
```

These are configuration in the adapter, not arbitrary values scattered through handlers.

# 297. Provider Clock Skew

Webhook and signed-request validation may use timestamps. Allow only a bounded clock-skew window and ensure the policy matches provider requirements. The server clock must be synchronized through managed infrastructure.

# 298. Time Synchronization

Production hosts need reliable NTP/time synchronization. Time is security-critical for token expiry, webhook replay windows, tax policy effective dates and audit ordering.

# 299. Idempotent External State Polling

If a provider requires polling:

```text
payment_intent pending
  ↓
poll state
  ↓
normalize result
  ↓
compare to current state
  ↓
apply only valid transition
```

Polling must not create duplicate payment records if the same state is observed repeatedly.

# 300. Operational State Reconciliation

Every external integration should support a reconciliation operation that can detect drift:

```text
local state
   vs
provider query/statement
   ↓
reconciliation report
```

This is necessary because webhooks can be missed even when the webhook endpoint is healthy.

# 301. Periodic Integrity Jobs

The platform should run background integrity checks:

```text
ledger balance vs materialized balance
outbox orphan detection
stuck jobs
stuck sync commands
pending EIS age
pending payment age
unclosed register age
```

These jobs should report anomalies rather than silently repair business truth.

# 302. Safe Automated Repair

Automated repair is acceptable for derived data but dangerous for financial truth.

Safe examples:

```text
rebuild a report projection
recompute a search index
re-enqueue a failed notification
```

Unsafe without explicit domain logic:

```text
change sale amount
change payment amount
invent stock correction
mark reconciliation matched
```

# 303. Rebuildable Projections

Every reporting/read projection should have a rebuild path from authoritative data. This is a key reliability criterion. A derived table that cannot be reconstructed is secretly a second source of truth.

# 304. Data Integrity Checksums

For important exported files and integration payload batches, store a cryptographic hash. This helps detect accidental corruption and provides evidence of what was transmitted or exported.

# 305. Receipt Integrity

Receipts should have a stable sale/reference ID and, where appropriate, a validation mechanism/QR derived from authoritative records. Reprints use the same sale identity; they are not new financial events.

# 306. Duplicate Receipt Handling

A reprint should be explicitly marked as a reprint if the display/format requires it, while preserving the original transaction date and identity. Reprinting should not re-post inventory or payment.

# 307. Offline Receipt Reprint

If a sale exists locally but has not reached the server, the client can reprint the locally committed receipt. Once synced, server verification can later associate the same logical sale. This requires the local transaction identity to be durable and stable across process restarts.

# 308. Client Database Corruption

If the local SQLite database becomes corrupt:

```text
detect open/read failure
   ↓
preserve forensic copy if possible
   ↓
verify unsynced command backup mechanism
   ↓
restore/rebuild from server snapshot where available
   ↓
resend only commands whose durable status is unknown
```

A mobile app should not automatically wipe the database without determining whether unsynced transactions could be lost.

# 309. Local Backup Strategy

Where supported, the app may maintain encrypted local backup/export of unsynced command state. However, backups are a security-sensitive feature and must not create an unprotected copy of merchant financial data.

# 310. Offline Storage Limits

When local storage approaches a configured limit, the app should compact removable caches and upload eligible history. It must never delete unsynced financial commands merely to free space.

# 311. Sync Priority

A reasonable priority order:

```text
financial mutations
inventory state required for current operation
critical configuration
normal catalogue changes
historical refresh
analytics/cache
```

This ensures recovery bandwidth first moves work that materially affects business continuity.

# 312. Sync Compression

Batch sync payloads can use compression for larger transfers, but only after measuring CPU/battery tradeoffs on low-end Android devices. Encryption already prevents some compression gains for certain payload patterns; benchmark rather than assume.

# 313. Offline Network Transitions

The sync engine should react to connectivity changes but not assume that 'network available' means the server is reachable. It should use real request success and health checks rather than OS connectivity flags alone.

# 314. Client Network Retry Strategy

Use exponential backoff with jitter, capped by a maximum interval. The client should distinguish:

```text
offline/no route
server unavailable
retryable 5xx
rate limited
permanent 4xx business rejection
authentication expired
```

Only retry cases known to be safe.

# 315. Sync Conflict Repair Workflow

When a conflict cannot be resolved automatically:

```text
CONFLICT
 ↓
show reason + current authority
 ↓
operator chooses supported correction
 ↓
new command created
 ↓
old conflict retained
```

This preserves history rather than editing the rejected command.

# 316. Frontend Security for Tauri

The Tauri webview should use an explicit capability policy. Only the native APIs required by the application should be exposed. File access should be scoped to known directories. Shell/process capabilities should be absent unless a specific feature requires them.

# 317. Desktop Auto-Update

Auto-update must verify signed artifacts. A failed update should leave the previous version usable. Updating while offline should either defer safely or use a previously downloaded verified package.

# 318. Mobile Release Security

Android builds should be signed using protected CI credentials. Debug builds must not share production endpoints or keys. Production package identifiers must be distinct from internal test variants where appropriate.

# 319. Build Flavors

Client flavors should include:

```text
development
staging
production
```

Each has distinct API endpoints, analytics configuration and provider environments. Production credentials never appear in development flavors.

# 320. Feature Entitlement Caching on Client

The mobile/desktop client can cache capabilities for offline UI. However, if a high-risk operation becomes unauthorized server-side, the server rejects it. Client cache improves UX; it does not define authority.

# 321. Authorization Decision Logging

For sensitive actions, log the policy outcome at a safe level:

```text
action=sales.refund
result=DENY
reason=INSUFFICIENT_PERMISSION
scope=branch
```

Do not dump full policy objects or secret context into logs.

# 322. Policy Engine Design

A simple policy engine can be implemented as typed Rust functions before adopting a general policy language. Example:

```rust
fn can_refund(ctx: &AuthzContext, sale: &Sale, amount: Money) -> Decision;
```

If policy complexity later grows across many customers, a dedicated policy subsystem can be introduced. Do not start with a generic policy language solely because enterprise products sound like they need one.

# 323. Approval Thresholds

High-risk thresholds should be configuration:

```text
refund > X MWK → manager approval
stock adjustment > Y units → approval
discount > Z% → approval
cash variance > threshold → review
```

Each policy version should be recorded on the resulting approval decision.

# 324. Segregation of Duties

Where staffing permits:

```text
requester != approver
cashier != reconciliation reviewer
support operator != final incident approver
```

For micro-merchants, the system may allow owner override while clearly recording the exception to standard separation rules.

# 325. Micro-Merchant Simplification

Enterprise architecture must not force every merchant to navigate enterprise workflows. Defaults can be simple:

```text
single user
single branch
cash + mobile money
basic catalogue
```

The underlying model remains capable of growth. Complexity is exposed progressively through configuration and entitlements.

# 326. Progressive Disclosure

A new duka should see:

```text
Sell
Stock
Money
Reports
```

A multi-branch operator may see:

```text
Operations
Inventory
Procurement
Finance Control
People
Branches
Compliance
Reports
Integrations
```

The architecture supports both because the modules are the same; only the presentation and entitlements differ.

# 327. Business Type Configuration

Onboarding may ask:

```text
What type of business do you run?
```

The answer activates recommended modules and sensible defaults. It does not permanently hard-code the tenant into a single vertical.

# 328. Module Activation Metadata

Store:

```text
module_key
enabled
source = default | admin | customer | plan
activated_at
activated_by
```

This makes it possible to explain why a feature is active and to reproduce configuration history.

# 329. Enterprise Plan Scaling

A plan can constrain:

```text
users
branches
registers
automations
exports
API rate
history depth
support SLA
```

But plan limits must be enforced through well-defined server capabilities and quotas, not scattered `if plan == enterprise` checks.

# 330. Quota Enforcement

Quotas need precise counting semantics. For example:

```text
active users
active branches
monthly exports
API requests per minute
storage bytes
```

Counts used for billing must be generated from authoritative events, not client-reported usage.

# 331. Usage Metering

Metering events should be append-only or reproducibly aggregated:

```text
usage_event_id
tenant_id
metric
delta
occurred_at
source_reference
```

Billing aggregates can be rebuilt from the usage stream if necessary.

# 332. Billing Reconciliation

Sitolo's own billing follows:

```text
invoice expected
   ↓
payment observed
   ↓
reconciled
   ↓
entitlement state
```

This is strategically consistent with the merchant finance model and demonstrates that Sitolo uses its own operational philosophy internally.

# 333. Subscription Upgrade Flow

```text
plan selected
   ↓
pricing calculated
   ↓
invoice generated
   ↓
payment pending
   ↓
payment observed
   ↓
reconciled
   ↓
entitlement activated
```

For immediate trial upgrades, the product can use a separate explicit trial state rather than silently marking the subscription paid.

# 334. Billing Failure Recovery

A failed payment must not corrupt merchant operations data. The billing module can reduce entitlements according to policy while keeping transaction history intact.

# 335. Data Portability

Export formats should be documented and stable enough for migration. Include identifiers and timestamps that allow importing systems to reconstruct relationships.

# 336. Migration Import Idempotency

A repeated import should not create duplicate products or sales. Use source-system identifiers and import batch IDs. Where no stable source ID exists, require a deterministic mapping or explicit duplicate-review workflow.

# 337. Historical Data Import

Imported historical sales may not have all fields required by newly created Sitolo transactions. The schema should support `source_system`, `import_batch_id` and historical-record flags without weakening current transaction invariants.

# 338. Support Search Index

Support should search by safe identifiers:

```text
tenant ID
receipt number
sale ID
device ID
provider transaction ID
terminal ID
```

Support search should avoid broad full-text access to personal data unless explicitly required.

# 339. Support Tool Rate Limits

Internal support tools need their own rate limiting and auditing. A support agent who can enumerate many tenants is a high-value account and should have stronger controls than ordinary merchant users.

# 340. Customer-Visible Security Events

For sensitive events such as new device registration or role changes, the product can notify the owner. This improves detection of account compromise without exposing technical details.

# 341. Account Recovery

Recovery flows should use standards-based mechanisms. Recovery must not allow support staff to bypass normal controls simply because a customer is on the phone. Every exceptional recovery action is recorded.

# 342. MFA Strategy

MFA should be configurable by plan/risk or required for selected privileged roles. Prefer authenticator-app or hardware-backed methods where practical. SMS may be offered for lower-assurance recovery workflows where justified, but should not be treated as equivalent to stronger factors.

# 343. Session Revocation

When a user changes password, is disabled, or a device is lost, the system should support revoking active sessions and device credentials. Access tokens should be short-lived enough that revocation is operationally meaningful.

# 344. Token Storage

For browser clients, secure, HttpOnly cookies may be preferable depending on the authentication architecture. For mobile/desktop, use platform secure storage. Never put long-lived secrets in URL query parameters.

# 345. CSRF Boundary

If browser cookies authenticate requests, CSRF protection is required for state-changing operations. If bearer tokens are used in a non-cookie context, the CSRF model differs, but XSS prevention remains critical.

# 346. XSS Boundary

The web/admin client must treat server-returned product descriptions, customer names and notes as untrusted content. Render text by default, sanitize rich content if it exists, and avoid unsafe HTML insertion.

# 347. SSRF Integration Guardrails

All provider destinations should be preconfigured server-side. Merchant-entered webhook URLs or arbitrary fetch destinations should be disabled unless there is a compelling use case and a rigorous network isolation model.

# 348. File Path Security

Any export/import or desktop filesystem path must be normalized and scoped. Reject path traversal. On the desktop, native commands should accept logical document IDs or approved directories rather than arbitrary absolute paths from the webview.

# 349. Command Injection

No normal Sitolo workflow should invoke shell commands with user-controlled strings. Where a native tool is genuinely required, use argument arrays rather than shell concatenation and apply allowlists.

# 350. Deserialization Safety

Use explicit typed parsing. Avoid unsafe deserialization formats in untrusted inputs. Do not deserialize arbitrary types or execute code based on payload metadata.

# 351. Compression Bomb Protection

If compressed payloads are accepted, enforce both compressed and decompressed size limits. Attackers can abuse highly compressed content to exhaust CPU/memory.

# 352. Zip/File Bomb Protection

Archive imports should enforce:

```text
max archive size
max uncompressed size
max file count
max path depth
path normalization
no executable extraction
```

# 353. CSV Injection Protection

Exports intended to be opened in spreadsheets should consider formula injection. Values beginning with spreadsheet formula characters may require safe escaping or a documented export option. This is particularly important for customer/product names that may contain malicious text.

# 354. Operational Security of Exports

Large exports are often more dangerous than the database because they are portable. Use short-lived links, optional encryption, audit trails and explicit permissions.

# 355. Privacy-by-Design Review

For every new personal-data field, document:

```text
purpose
necessity
source
retention
access roles
export behavior
delete/anonymize behavior
third-party sharing
```

If the team cannot explain why a field exists, do not collect it.

# 356. Data Protection Controls

The architecture should support the principles required by applicable data-protection law, including access control, minimization, retention, security and rights workflows. Malawi's Data Protection Act 2024 is part of the operating environment; exact legal obligations and registration/processor requirements should be verified with appropriate counsel and the relevant authority before launch claims are made.

# 357. Privacy Request Workflow

A future subject-access/deletion workflow can be:

```text
request received
 → verify identity
 → determine data scope
 → apply legal exceptions
 → export/delete/anonymize
 → record completion
```

The workflow should never erase financial evidence merely because a generic delete button exists.

# 358. Data Classification in Code

Sensitive fields should be identifiable in code review and logging policy. A useful approach is explicit field annotations/documentation rather than an implicit assumption that 'all customer data is sensitive'.

# 359. Redaction Middleware

Central redaction should exist for structured logs and traces. Individual developers should not have to remember every secret field manually, though sensitive field handling must still be explicit.

# 360. Security Architecture Review Checklist

Before a major feature ships, reviewers should ask:

```text
Who can invoke it?
Which tenant does it affect?
Can the client fake the tenant/branch?
Can the action be replayed?
Can it happen offline?
Can a duplicate webhook trigger it?
What is the correction path?
What data is logged?
What data is exported?
What external system trusts it?
How is it recovered?
```

# 361. Performance Review Checklist

```text
What is the hot path?
Which query runs per sale?
What happens at 100x current volume?
Does this allocate large payloads?
Can the request hold a DB lock for too long?
Can a slow provider consume worker capacity?
Does reporting hit OLTP?
What happens on low-end Android?
```

# 362. Maintainability Review Checklist

```text
Does business logic have one source of truth?
Are module boundaries explicit?
Can the feature be tested without the network?
Are external APIs behind adapters?
Are errors typed?
Are migrations reversible or forward-correctable?
Can support diagnose incidents without database surgery?
```

# 363. Architecture Anti-Patterns

Avoid:

```text
GodService
GenericRepository<T>
ControllerContainsBusinessLogic
ORMEntityAsDomainModel
GlobalMutableState
UnboundedTaskSpawn
LastWriteWinsForMoney
RedisAsSourceOfTruth
WebhookDoesEverythingSynchronously
ClientDecidesAuthorization
```

These patterns create exactly the failure modes this architecture is designed to avoid.

# 364. When a Generic CRUD Layer Is Acceptable

Simple low-risk configuration resources can use generic repository helpers if they still preserve authorization and validation. For example, a small settings table is not the same risk as a sale or payment. The architecture should avoid dogma as well as shortcuts.

# 365. Code Review Severity

Critical review areas:

```text
auth/authz
money
inventory
payments
sync
external integrations
migrations
secrets
```

These deserve domain-expert review. UI changes should not need the same depth unless they change authorization or data exposure.

# 366. Architectural Test Fixtures

Create shared test builders:

```rust
TenantBuilder
UserBuilder
BranchBuilder
ProductBuilder
SaleBuilder
PaymentBuilder
InventoryBuilder
```

Builders should make valid state easy to create and invalid state explicit. This increases test coverage without encouraging production shortcuts.

# 367. Integration Test Database Strategy

Use isolated PostgreSQL databases or schemas per test run as appropriate. Tests should exercise real constraints and transactions. SQLite should not be used as a substitute for PostgreSQL integration tests because SQL semantics differ.

# 368. Transaction Test Strategy

Test commit and rollback explicitly:

```text
start transaction
perform partial work
inject failure
assert no visible partial state
```

This is especially important for sale/inventory/payment coupling.

# 369. External Call Mock Boundary

The application layer should depend on traits/interfaces for external systems:

```rust
trait PaymentGateway { ... }
trait TaxGateway { ... }
trait NotificationGateway { ... }
trait ObjectStore { ... }
```

Tests can inject deterministic fakes without changing domain logic.

# 370. Mock Quality Rule

Fakes should behave according to a contract, including error cases. A fake payment provider that cannot produce a timeout or duplicate callback creates blind spots.

# 371. Contract-First Adapter Development

For each integration:

```text
external documentation
 ↓
canonical capability model
 ↓
adapter contract
 ↓
fixture tests
 ↓
implementation
```

Do not start by copying provider JSON throughout the domain layer.

# 372. Provider Capability Matrix

| Capability | Provider A | Provider B | Bank C |
|---|---:|---:|---:|
| Initiate payment | yes/no | yes/no | yes/no |
| Webhook | yes/no | yes/no | yes/no |
| Status query | yes/no | yes/no | yes/no |
| Statement | yes/no | yes/no | yes/no |
| Refund | yes/no | yes/no | yes/no |
| Idempotency | yes/no | yes/no | yes/no |

The implementation should query this capability model rather than assume feature parity.

# 373. Provider Failure Classification

Map external errors into:

```text
TRANSIENT_NETWORK
TRANSIENT_PROVIDER
AUTHENTICATION_FAILURE
RATE_LIMITED
VALIDATION_REJECTED
BUSINESS_REJECTED
PERMANENT_CONFIGURATION
UNKNOWN
```

Only appropriate classes are retried.

# 374. Reconciliation Confidence

A future matching engine may attach confidence to candidate matches, but the confidence should never become a license to auto-match everything. Use tiers:

```text
DETERMINISTIC
HIGH_CONFIDENCE_CANDIDATE
MANUAL_REVIEW
UNMATCHED
```

# 375. Manual Reconciliation Controls

A manual match should record:

```text
operator
reason
source references
selected expected record
selected observed record
previous state
new state
timestamp
```

This becomes important when disputes appear months later.

# 376. Accounting Export Corrections

If a previously exported transaction was corrected, the integration should send a correction event rather than re-export a modified original record invisibly. This keeps downstream systems aligned with the append-only history model.

# 377. Tax Correction Boundary

Tax correction is external-authority-specific. Sitolo should retain local sale/reversal history and create an external submission workflow according to MRA/provider requirements. Never invent a generic 'edit tax invoice' behavior that may be illegal or incompatible with the tax authority's protocol.

# 378. Receipt QR Validation Boundary

QR data used for tax validation should be generated according to the external authority's documented algorithm or response. Sitolo should not invent a private QR format and label it 'tax valid'.

# 379. Regulatory Claim Policy

Product copy may say:

```text
"Supports MRA EIS integration"
```

only when the integration exists and is tested.

Stronger claims such as:

```text
"MRA-certified"
"legally compliant"
```

require documented external evidence and current verification.

# 380. Architecture Documentation Hierarchy

The project should eventually contain:

```text
README.md
CLAUDE.md / engineering governance
product specification
business_model_design.md
system_architecture_design.md
ADRs/
runbooks/
api/
security/
data/
operations/
```

This architecture document is the system-level map; specialized documents should hold narrower detail as they become large.

# 381. ADR Template

Each significant decision should include:

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

This prevents architectural knowledge from disappearing into chat history.

# 382. Architecture Review Board Lite

Early-stage governance does not require bureaucracy. A small review can happen for changes involving:

```text
security boundary
financial semantics
sync protocol
new external provider
new database
new service boundary
regulated workflow
```

Ordinary UI/features do not need formal board approval.

# 383. Technical Debt Policy

Technical debt should be classified:

```text
safe shortcut
known performance debt
security risk
correctness risk
architecture debt
```

Correctness and security debt get priority over cosmetic refactors.

# 384. Deprecation Policy

Every deprecated API/field/feature should have:

```text
replacement
migration path
last supported version
removal date/condition
owner
```

Permanent compatibility without an exit plan becomes a hidden maintenance tax.

# 385. Observability of Offline Clients

The server can track:

```text
last_seen_at
last_sync_at
pending_commands_count
last_sync_error
client_version
sync_protocol_version
```

Do not collect invasive device telemetry merely because it is technically available.

# 386. Device Fleet Health

Enterprise merchants can have many POS devices. The admin surface should identify devices with stale sync or outdated software. Software version enforcement should allow controlled upgrade windows.

# 387. Software Rollout by Tenant

High-risk client releases can use phased rollout:

```text
internal tenants
 → pilot tenants
 → 10%
 → 50%
 → 100%
```

The rollout system itself should be auditable.

# 388. Rollout Abort Criteria

Abort when:

```text
sale failure rate rises
sync conflict/rejection rate spikes
payment webhook failures increase
crashes exceed threshold
critical security issue found
```

# 389. Mobile Crash Diagnostics

Crash reporting should collect app version, device class and stack traces with privacy controls. Avoid logging raw customer data in crash context.

# 390. Backend Crash Diagnostics

Rust panics in production should be captured with context and restart policy. A panic must not take down all workers or API capacity; process supervision and deployment health checks should replace failed instances.

# 391. Graceful Shutdown

On shutdown:

```text
stop accepting new work
finish/abort in-flight requests safely
stop claiming new jobs
allow active jobs a bounded drain period
close DB pools
exit
```

Long-running jobs need checkpoint/state persistence so shutdown does not lose work.

# 392. Health Endpoints

Expose distinct health semantics:

```text
/liveness  → process is alive
/readiness → can serve requests
/dependencies → diagnostic for operations
```

A liveness check should not fail simply because the payment provider is down.

# 393. Dependency Health

The platform should distinguish mandatory dependencies from optional ones. PostgreSQL is mandatory for the API. Redis may be optional. MRA is external and should not make `/liveness` fail.

# 394. Maintenance Mode

The platform should support controlled maintenance that can:

```text
allow login/read operations
block risky mutations
show planned maintenance
continue offline client operation where appropriate
```

Avoid taking down the entire system for every migration.

# 395. Rate-Limit UX

The UI should turn `429` into a clear state such as 'Too many attempts; try again soon' rather than retrying aggressively. Automatic client retries on rate limits can become a feedback loop.

# 396. Accessibility

The client architecture should support accessible components and large-touch targets. This is not cosmetic: a till operator may work quickly on inexpensive devices under poor conditions.

# 397. Localization Architecture

Strings belong in localized resources, not scattered through business code. Currency and date formatting must use tenant/branch locale configuration. Domain code stores canonical values.

# 398. Language Expansion

Adding a new language should not require changing business logic. Text labels and validation messages should be externalized. Error codes remain language-neutral.

# 399. Accessibility and Offline

Offline error messages should be especially clear because the user cannot immediately consult a server. Messages should tell them whether the action was saved locally, queued, rejected or requires connectivity.

# 400. End-to-End Merchant Journey

A complete merchant journey should work like this:

```text
1. Install app
2. Create/accept organization
3. Authenticate
4. Register device
5. Select business type
6. Configure branch/register
7. Load catalogue
8. Sell offline
9. Record payment
10. Close register
11. Reconnect
12. Sync commands
13. Process EIS/payment side effects
14. Reconcile
15. Review report
```

Every stage crosses the same core architecture without creating alternate business truth.

# 401. End-to-End Multi-Branch Journey

```text
head office creates branch
       ↓
branch receives catalogue/pricing snapshot
       ↓
local POS devices operate offline
       ↓
sales sync
       ↓
branch stock/reporting updates
       ↓
inter-branch transfer requested
       ↓
approval
       ↓
source/destination postings
       ↓
central visibility
```

# 402. End-to-End Pharmacy Journey

```text
supplier receipt
 → lot recorded
 → quality state
 → expiry recorded
 → sellable
 → FEFO allocation
 → sale/dispense
 → audit
 → recall/expiry if required
```

# 403. End-to-End Reconciliation Journey

```text
sale
 → payment intent
 → provider transaction
 → webhook/statement
 → deterministic match
 → matched
```

Failure branch:

```text
provider transaction
 → no matching intent
 → exception
 → investigation
 → manual resolution
 → audit
```

# 404. End-to-End Disaster Journey

```text
DB incident
 → API write degradation
 → mobile offline queues continue
 → infrastructure restore
 → API recovery
 → clients sync
 → projections rebuild
 → external states reconciled
 → incident closed
```

# 405. System Invariants Summary

The architecture can be reduced to ten absolute rules:

```text
1. No cross-tenant access.
2. No unauthorized state mutation.
3. No duplicate financial effect from retry.
4. No silent historical financial edit.
5. No external call held inside a long DB transaction.
6. No offline command without durable identity.
7. No report treated as transactional truth.
8. No provider response trusted without verification.
9. No secret stored in a client app.
10. No derived data that cannot be rebuilt.
```

# 406. Final Technology Matrix

| Area | Decision |
|---|---|
| Backend language | Rust |
| Runtime | Tokio |
| HTTP | Axum |
| DB access | SQLx |
| Primary DB | PostgreSQL |
| Local mobile DB | SQLite |
| Mobile | Flutter |
| Desktop | Tauri |
| Web/admin | TypeScript + React/Next.js where needed |
| Async jobs | Rust worker runtime + PostgreSQL outbox |
| Cache | Redis only where justified |
| Blob storage | S3-compatible object storage |
| Observability | tracing + metrics + OpenTelemetry-compatible pipeline |
| Auth | Standards/provider + Rust authorization layer |
| Payments | Provider adapters |
| Tax | MRA EIS adapter |
| Search | PostgreSQL first; external engine only if earned |
| Analytics | PostgreSQL/read models first; warehouse later |

# 407. Final Build/Buy Matrix

| Component | Build? | Rule |
|---|---:|---|
| POS domain | Yes | Product differentiator |
| Inventory ledger | Yes | Product correctness |
| Sync | Yes | Product differentiator |
| Reconciliation | Yes | Strategic moat |
| Auth protocol | No | Use mature standards/provider |
| Password crypto | No | Use audited libraries |
| Database | No | PostgreSQL |
| Desktop shell | No | Tauri |
| Mobile toolkit | No | Flutter |
| Payment rails | No | Providers |
| Tax authority | No | MRA |
| Message broker | Not initially | Add only when required |
| Search cluster | Not initially | PostgreSQL first |
| Data warehouse | Not initially | Add when reporting requires |

# 408. Architecture Readiness Gates

Before the first production release, the following must be demonstrated rather than merely documented:

```text
TENANCY
  cross-tenant negative tests pass

SALES
  sale transaction atomicity passes
  duplicate command is harmless

INVENTORY
  concurrent stock decrement is safe
  balance equals postings

PAYMENTS
  duplicate webhook is harmless
  invalid signature is rejected

SYNC
  offline sale survives restart
  repeated push is idempotent
  checkpoint advances only after persistence

TAX
  EIS integration contract tests pass
  offline submission path is verified

SECURITY
  recovery and device revocation tested
  secrets absent from clients/logs

OPERATIONS
  backup restore tested
  alerting tested
  deployment rollback procedure exercised
```

# 409. Why This Architecture Fits the Business Model

The business model says Sitolo becomes more valuable as one business record connects selling, stock, money, reconciliation, reporting and governance. This architecture preserves that economic loop technically by keeping the most tightly coupled domains within one transactional core while making external systems replaceable.

The result is:

```text
ONE BUSINESS RECORD
       ↓
ONE OPERATIONAL TRUTH
       ↓
ONE FINANCIAL HISTORY
       ↓
ONE RECONCILIATION LAYER
       ↓
ONE ENTERPRISE CONTROL PLANE
```

This is what enables account expansion. A merchant can start with POS and inventory without the platform later requiring a second data model for reconciliation, branches or compliance.

# 410. Why Rust Is Especially Appropriate Here

Rust is not chosen because it is fashionable. Sitolo has a workload profile where memory safety, predictable resource usage and strong type constraints are valuable: asynchronous APIs, concurrent sync processing, financial calculations, background jobs and long-lived services. The language also makes it practical to push invariants into types and explicit state transitions.

Rust does not solve business correctness automatically. A badly designed Rust system can still have broken authorization or financial logic. The architecture therefore uses Rust as a strong foundation, not as a substitute for engineering discipline.

# 411. Why TypeScript Is Deliberately Small

TypeScript remains useful where the web ecosystem is strongest, but keeping it out of the core backend creates one source of truth for business logic. The most important rules—authorization, inventory, payments, reconciliation, tax state—live in Rust.

This reduces the risk of:

```text
Rust says refund allowed
TS says refund allowed differently
→ production inconsistency
```

# 412. Why Flutter Is the Operational Client

Flutter's current documentation supports cross-platform application architecture, native release compilation and SQL persistence for complex local data, while also documenting offline-first patterns. This aligns directly with Sitolo's need to operate on mobile devices under intermittent connectivity. citeturn734608search5turn734608search12turn734608search15

The key decision is architectural, not merely UI technology: local SQLite state and synchronization are first-class subsystems.

# 413. Why Tauri Is the Back-Office Shell

Tauri's architecture combines a webview UI with a Rust-native shell and message-passing bridge. That fits Sitolo's desire for desktop capability without introducing a second desktop backend or a heavy general-purpose runtime. citeturn734608search4

# 414. Why PostgreSQL Is the Financial Core

The platform needs reliable multi-row transactions, foreign keys, indexes, constraints and mature operational tooling. PostgreSQL provides those primitives without Sitolo having to implement a bespoke storage engine.

The architecture uses the database as a strong enforcement layer while keeping business semantics in Rust.

# 415. Why the System Is Not a Microservice Mesh

The hardest distributed system in Sitolo is already the offline client/server synchronization problem. Making sales, inventory, payments and reconciliation separate network services would add distributed transactions and failure modes without improving the merchant experience.

Therefore:

```text
Distributed at the real boundaries
     │
     ├── mobile ↔ server
     ├── server ↔ payments
     ├── server ↔ MRA
     └── server ↔ external notification systems

Not distributed for decoration
     │
     └── sales ↔ inventory ↔ cash inside one transaction core
```

# 416. Future Service Extraction Candidates

When growth demands it, likely candidates are:

```text
Notification Service
Integration Gateway
Reporting/Analytics Pipeline
Search Service
```

Less likely to extract early:

```text
Sales
Inventory
Cash
Payments core
Reconciliation core
```

because their coupling is a functional requirement.

# 417. Service Extraction Migration Pattern

When extraction is justified:

```text
module boundary
   ↓
explicit internal event contract
   ↓
shadow external service
   ↓
dual processing / compare results
   ↓
switch traffic
   ↓
remove in-process implementation
```

The modular monolith is designed to make that migration possible without pretending it must happen now.

# 418. Enterprise Customer Isolation Options

As enterprise accounts grow, the architecture can offer:

```text
shared DB/shared compute
shared DB/dedicated compute
schema/database isolation
region isolation
fully dedicated deployment
```

The commercial plan and risk model should determine which tier is justified.

# 419. Tenant Isolation Assurance Program

Security should periodically verify:

```text
application query coverage
RLS coverage
repository patterns
API endpoint tests
support tooling
exports
background jobs
reports
webhooks
```

Cross-tenant vulnerabilities often hide in background jobs and reporting, not just ordinary HTTP endpoints.

# 420. Background Job Tenant Isolation

Every job payload must contain enough tenant context to enforce scope. Workers must not infer a tenant from mutable global state.

```text
job payload tenant_id
     ↓
load tenant config
     ↓
execute scoped query
     ↓
record scoped audit
```

# 421. Worker Privilege Model

Workers should have the minimum privileges required. A notification worker should not have permission to update inventory tables. Even inside one binary, logical capabilities should remain separated.

Where database roles cannot express the module boundary conveniently, application-level interfaces and code ownership rules become the primary controls.

# 422. Data Access Layer Rules

Each repository method should state its tenancy expectations. Example:

```rust
get_sale(tenant_id, sale_id)
```

is preferable to:

```rust
get_sale(sale_id)
```

for tenant-owned data.

# 423. Internal Event Bus

Inside the monolith, a lightweight in-process event mechanism may notify modules after transactions commit, but durable cross-boundary side effects should still use the outbox. In-process events must not become the only trigger for payment/tax/notification work because process crashes can lose them.

# 424. Domain Events Versus Integration Events

Domain event:

```text
SaleCompleted
```

Integration event:

```text
EisSaleSubmissionRequested
```

The first describes a business fact. The second asks an external adapter to perform work. Keeping these concepts distinct reduces coupling.

# 425. Event Privacy

Events should carry the minimum data required by consumers. Do not emit full customer profiles into every event if a customer ID is sufficient.

# 426. Audit and Event Retention Alignment

Audit records can be retained according to governance requirements. Outbox/integration event records may have shorter operational retention if source-of-truth history exists elsewhere. Retention choices should be explicit rather than accidental.

# 427. Architecture Testing in Production

Production can use safe synthetic checks:

```text
health endpoint
read-only test tenant
provider sandbox ping where supported
backup verification
sync health
```

Synthetic checks must never mutate real financial state without explicit controls.

# 428. Canary Transaction Strategy

A dedicated internal tenant can perform controlled end-to-end tests in production-like infrastructure. This tenant should be clearly marked and excluded from customer reporting.

# 429. Observability Cost Control

Logs and traces can become expensive. Apply sampling to low-value traces while retaining 100% of high-severity errors and security events as appropriate. Business audit records remain unsampled.

# 430. Metric Naming Standard

Use consistent conventions such as:

```text
sitolo_http_requests_total
sitolo_http_request_duration_seconds
sitolo_sync_commands_total
sitolo_sync_conflicts_total
sitolo_payment_events_total
sitolo_reconciliation_exceptions_total
sitolo_eis_submissions_total
```

Consistency matters when dashboards span environments.

# 431. Dashboard Layers

Create dashboards for:

```text
Platform health
Transaction health
Sync health
Payment/reconciliation
Tax/EIS
Database
Jobs
Security
```

A single mega-dashboard becomes unreadable.

# 432. Incident Severity

Example:

```text
SEV-1 — customer financial integrity or broad platform outage
SEV-2 — major function degraded / significant tenant impact
SEV-3 — localized issue / workaround exists
SEV-4 — cosmetic/low urgency
```

The exact definitions can evolve, but financial correctness incidents should receive higher severity than ordinary UI defects.

# 433. Incident Communication

For customer-facing incidents, communicate:

```text
what is affected
what is safe to do
what is not safe
whether offline mode can continue
current mitigation
next update window
```

Do not tell merchants an issue is fixed until the authoritative state and reconciliation have been verified.

# 434. Post-Incident Review

Every significant incident should produce:

```text
timeline
root cause
contributing factors
customer impact
detection quality
containment quality
remediation
regression test
owner
```

Blame is not an engineering control; learning is.

# 435. Operational Readiness Checklist

Before production:

```text
backups verified
restore tested
alerts configured
on-call ownership defined
secrets rotated/tested
provider sandboxes verified
MRA integration readiness verified
device revocation works
support tooling works
data export works
migration rollback/forward plan documented
```

# 436. Documentation Freshness

Documentation should include an owner and last-reviewed date. Critical runbooks should be exercised regularly. A runbook that has never been executed is a hypothesis, not an operational capability.

# 437. Architecture Review Cadence

Review the architecture when:

```text
major new domain added
new country launched
new payment rail integrated
tax protocol changes
scale assumptions materially change
new regulatory obligations emerge
service extraction proposed
```

Annual review is useful, but event-driven review is more important.

# 438. Decision Log for Technology

The repository should track technology choices and rationale. For example:

```text
2026-09 — Rust 1.98 pinned baseline
2026-09 — modular monolith retained
2026-09 — Tauri desktop
2026-09 — Flutter mobile
2026-09 — SQLite local store
```

This avoids re-litigating settled decisions without new evidence.

# 439. Engineering North Star

The architecture is successful when a merchant can experience this:

```text
NETWORK FAILS
     ↓
BUSINESS CONTINUES
     ↓
NETWORK RETURNS
     ↓
DATA CONVERGES
     ↓
MONEY RECONCILES
     ↓
AUDIT EXPLAINS WHAT HAPPENED
```

That is more important than whether the architecture contains a fashionable service mesh or a high benchmark number.

# 440. Final System Contract

The final system contract is:

> **Clients are operational surfaces. Rust is the decision engine. PostgreSQL is authoritative business storage. SQLite preserves local continuity. The synchronization protocol converges client work safely. External systems are explicit trust boundaries. Financial history is immutable and corrected through compensating events. Authorization is server-enforced and tenant-scoped. Derived data is rebuildable. Observability and auditability are designed into the system rather than attached afterward.**

That contract is the foundation on which Sitolo can grow from a small merchant POS into a genuine African SME business operating system without rebuilding its core every time the product becomes more sophisticated.

---

## Architecture Source Verification Notes

The external research used for this architecture was intentionally narrow and authoritative where possible. MRA EIS official developer materials were used for the terminal, onboarding, configuration, sales, offline transaction and submission model. Flutter's official architectural materials were used for mobile architecture, SQL persistence and offline-first patterns. Tauri's official architecture documentation was used for the desktop shell decision. Rust's official release feed was used to verify the current stable Rust line for the preparation date. SQLx's project documentation was used to validate the intended compile-time verification and release/MSRV posture.

The architecture remains subject to verification against the exact provider contracts, MRA certification status, pharmacy regulatory requirements, privacy law obligations and hosting/security requirements applicable at production launch.

**End of system architecture design.**
