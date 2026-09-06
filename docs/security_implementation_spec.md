# SITOLO
# Security Implementation Specification

**Document:** `security_implementation_spec.md`  
**Product:** Sitolo — Business Operating System for African SMEs  
**Document class:** Phase 0 implementation contract  
**Status:** Pre-implementation engineering baseline  
**Version:** 1.0  
**Date:** 4 September 2026  
**Primary market:** Malawi first  
**Regional target:** Africa  
**Backend:** Rust  
**Mobile:** Flutter / Android-first  
**Desktop:** Tauri  
**Web/Admin:** TypeScript where materially useful  
**Authoritative server database:** PostgreSQL  
**Offline operational store:** SQLite  
**Architecture:** Modular monolith first, durable workers, explicit adapters, selective extraction only when justified  
**Security posture:** Zero trust, deny by default, least privilege, server authoritative, tenant isolated, fail closed, evidence driven  
**Verification posture:** Mandatory and release blocking

---

## 0. Document Purpose

This document translates Sitolo's existing product, system architecture and security architecture into an implementation-level contract.

It is intentionally not a generic secure-coding checklist. Every control in this specification exists because it protects a concrete Sitolo capability, trust boundary, business invariant, operating condition, or deployment path.

The governing documents are:

1. `business_model_design.md`
2. `sitolo.md`
3. `system_architecture_design.md`
4. `security_architecture_design.md`

This file is subordinate to those documents. When implementation detail conflicts with a higher-level documented invariant, the invariant wins and the conflict becomes an architecture decision record rather than an informal developer choice.

The specification establishes:

- implementation boundaries;
- repository structure;
- Rust workspace expectations;
- security middleware order;
- authentication and session integration boundaries;
- authorization enforcement points;
- tenant-context propagation;
- PostgreSQL security roles and database enforcement;
- request/input validation contracts;
- financial and inventory mutation security;
- offline command security;
- payment and webhook security;
- file, export and SSRF controls;
- observability and audit requirements;
- secrets and configuration handling;
- CI/CD security;
- dependency and artifact controls;
- mandatory security tests;
- release gates;
- failure semantics;
- implementation sequence;
- evidence requirements;
- definition of ready and definition of done.

The central implementation principle is:

> **Do not implement a sensitive feature as an isolated endpoint. Implement its identity, scope, authorization, validation, invariant, transaction, audit, idempotency, observability and test contract as one unit.**

That principle follows directly from Sitolo's existing security architecture, which treats security and business correctness as a single architecture problem at different layers, and from its system contract that clients are operational surfaces while Rust and PostgreSQL own business truth.

---

# 1. Source-of-Truth Hierarchy

When multiple sources contain overlapping detail, implementation follows this precedence order:

```text
BUSINESS MODEL
     |
     v
PRODUCT / DOMAIN SPECIFICATION
     |
     v
SYSTEM ARCHITECTURE
     |
     v
SECURITY ARCHITECTURE
     |
     v
THIS IMPLEMENTATION SPECIFICATION
     |
     v
SOURCE CODE / TESTS / INFRASTRUCTURE
```

The direction is important.

Code does not redefine the business model.  
A route does not redefine authorization.  
A UI does not redefine authority.  
A migration does not silently redefine the domain model.  
A dependency does not redefine security requirements.  
A CI convenience does not override a release gate.

If implementation evidence reveals that a higher-level decision is wrong, the process is:

```text
DISCOVER CONFLICT
      |
      v
DOCUMENT EVIDENCE
      |
      v
OPEN ADR / DESIGN CHANGE
      |
      v
SECURITY REVIEW WHEN REQUIRED
      |
      v
UPDATE AUTHORITATIVE DOCUMENT
      |
      v
IMPLEMENT
```

Never:

```text
NOTICE DESIGN PROBLEM -> PATCH LOCALLY -> MOVE ON
```

That creates architectural drift and eventually makes the documentation false.

---

# 2. Implementation Objectives

The implementation must satisfy the following objectives.

## 2.1 Confidentiality

Prevent unauthorized disclosure of:

- merchant and tenant data;
- employee data;
- customer data;
- supplier information;
- financial records;
- payment evidence;
- tax/EIS evidence;
- audit evidence;
- credentials;
- signing keys;
- device secrets;
- deployment identities;
- operational telemetry.

## 2.2 Integrity

Prevent unauthorized creation, modification, deletion, reversal, duplication, replay or reordering of high-impact business state.

Integrity is particularly important for:

- inventory;
- sales;
- refunds;
- returns;
- cash;
- payment status;
- taxes and receipts;
- user roles;
- device registrations;
- subscription entitlements;
- audit history.

## 2.3 Availability

Protect the service against:

- unbounded requests;
- expensive queries;
- retry storms;
- dependency hangs;
- malformed parser input;
- oversized payloads;
- excessive report/export jobs;
- compromised devices generating sync floods;
- webhook floods;
- queue starvation;
- resource exhaustion.

Availability controls must never silently fabricate financial truth.

## 2.4 Tenant Isolation

A tenant boundary is a product-level security boundary.

A caller must never gain authority over another tenant through:

- direct object IDs;
- crafted tenant IDs;
- branch IDs;
- cache keys;
- report filters;
- exports;
- search;
- background jobs;
- webhook association;
- offline replay;
- support tooling;
- administrative endpoints;
- database access;
- logs or telemetry.

## 2.5 Auditability

Security-sensitive and economically meaningful operations must be reconstructable.

A meaningful investigation must be able to answer:

```text
WHO
  |
  +-- authenticated principal
  |
  +-- session/device
  |
  +-- tenant and scope
  |
  +-- action
  |
  +-- resource
  |
  +-- request / command
  |
  +-- business state transition
  |
  +-- database transaction
  |
  +-- external event if any
  |
  +-- final result
```

## 2.6 Controlled Offline Operation

Offline operation is a core business feature, not a cache optimization.

The device must retain operational continuity while the network is unavailable without becoming an unrestricted second authority.

## 2.7 Supply-Chain Integrity

The build system is part of production security.

A release must be attributable to:

- an authorized source revision;
- a locked dependency graph;
- an approved toolchain;
- a known build workflow;
- validated security checks;
- an identifiable artifact.

---

# 3. Non-Negotiable Implementation Principles

## 3.1 Server authority

Clients express intent.

Rust validates intent, applies authorization and domain rules, and decides the authoritative result.

PostgreSQL records authoritative transactional state.

A client result is never authoritative merely because the client calculated it.

## 3.2 Deny by default

The following must default to denial:

- protected routes;
- unknown permissions;
- missing tenant context;
- unknown object scope;
- unknown fields on sensitive mutations;
- unrecognized webhook signatures;
- revoked devices;
- invalid sessions;
- unverified provider state;
- unsupported external destinations;
- failed security checks;
- missing configuration;
- absent policy decisions.

## 3.3 Explicit trust boundaries

Sitolo has several distinct subjects:

```text
Internet
  -> Edge
  -> Client
  -> API
  -> Identity / Session
  -> Tenant / Authorization
  -> Domain services
  -> PostgreSQL
  -> Workers
  -> External providers
  -> Local device store
  -> CI/CD
  -> Operations / Support
```

No subject is trusted merely because it runs inside infrastructure owned by Sitolo.

## 3.4 Typed security context

Security-sensitive state should not be represented as arbitrary strings when the type system can enforce distinction.

Examples:

```text
TenantId
OrganizationId
BranchId
WarehouseId
RegisterId
DeviceId
UserId
SessionId
Permission
Role
Scope
CommandId
EventId
PaymentIntentId
ProviderTransactionId
```

A raw `String` should not be interchangeable with a tenant identifier simply because both serialize to text.

## 3.5 Business invariants are security controls

Examples:

- refund <= refundable amount;
- stock cannot be decremented below the defined policy limit;
- one provider transaction cannot settle twice;
- a completed sale is not silently edited;
- a cashier cannot approve their own restricted adjustment;
- a revoked device cannot continue normal synchronization;
- a user cannot act outside authorized branch scope.

These are security controls even when no classical “attacker” is involved.

## 3.6 Fail closed

A failed security mechanism is not equivalent to success.

Examples:

```text
AUTHZ engine unavailable  -> DENY sensitive operation
Signature verification failed -> REJECT webhook
Scanner unavailable      -> CI FAIL
Dependency policy unavailable -> CI FAIL
Required secret missing  -> STARTUP FAIL or feature unavailable
Tenant context missing   -> DENY
Policy cache corrupted    -> SAFE FALLBACK / DENY
```

A system may degrade availability before it degrades trust.

---

# 4. Product-Specific Security Context

Sitolo is not a generic SaaS CRUD product.

Its core operating loop is:

```text
PROCURE -> RECEIVE -> STOCK -> PRICE -> SELL -> COLLECT -> RECONCILE -> REPORT
                         ^                                  |
                         |                                  v
                    ADJUST / RETURN <------ AUDIT <------ SYNC
```

This creates several security classes.

### Class A — Identity

Who is performing the operation?

### Class B — Organizational scope

Which organization, branch, warehouse, register or device is involved?

### Class C — Functional authority

Is this actor allowed to perform the operation?

### Class D — Economic validity

Does the operation make business sense?

### Class E — State validity

Is the current object state compatible with the requested transition?

### Class F — Evidence

Can the operation be reconstructed later?

### Class G — External trust

Did an external provider actually authorize/confirm the claimed event?

### Class H — Resource safety

Can this request consume unbounded CPU, memory, connections, storage or queue capacity?

Every important command must pass the relevant classes.

---

# 5. Implementation Boundary: What Sitolo Builds vs. Reuses

Sitolo should not write bespoke implementations for mature commodity security primitives.

## 5.1 Build internally

Sitolo owns:

- tenant-aware authorization semantics;
- organization/branch scope semantics;
- business-state authorization;
- inventory integrity;
- financial invariants;
- offline command semantics;
- synchronization and conflict policy;
- reconciliation rules;
- provider adapter contracts;
- audit event semantics;
- domain-specific security tests;
- support-access semantics.

## 5.2 Reuse mature implementations

Prefer established libraries/services for:

- password hashing;
- OAuth/OIDC protocol mechanics;
- cryptographic primitives;
- TLS;
- secret storage/KMS;
- WAF/DDoS;
- object-storage durability;
- vulnerability databases;
- static analysis;
- software composition analysis;
- artifact signing/attestation tooling;
- operating-system secure storage.

The existing security architecture explicitly adopts this build-vs-buy posture so that Sitolo's custom engineering effort remains concentrated on its actual security moat: tenant isolation, offline continuity, financial correctness, reconciliation, payment/tax boundaries and auditability.

---

# 6. Recommended Technology Baseline

## 6.1 Rust

The repository must pin an explicit stable Rust toolchain.

As of 3 September 2026, the official Rust release feed lists Rust **1.98.1** as the latest stable release. The implementation baseline therefore uses Rust 1.98.1 unless an ADR approves another supported stable version. The important rule is not the number itself; it is that the compiler version is explicit and reproducible across local development and CI.

Recommended baseline:

```text
Rust:       1.98.1
Edition:    2024
Runtime:    Tokio
HTTP:       Axum
DB:         SQLx + PostgreSQL
Serialize:  Serde
Tracing:    tracing + tracing-subscriber
Time:       time
IDs:        UUID and/or ULID according to domain need
Errors:     thiserror + constrained anyhow at edges where useful
```

Rust's Cargo documentation describes workspaces as collections of packages that share dependency resolution, a workspace lockfile and build configuration. Cargo also documents `--locked` as enforcing the existing lockfile and failing when the dependency graph would change. Sitolo therefore treats the workspace and lockfile as part of its reproducible security boundary.

## 6.2 Mobile

Flutter is the primary merchant mobile application.

The mobile client is not the security authority.

It provides:

- operational UI;
- offline data access;
- local command generation;
- local validation for user experience;
- secure credential storage through platform facilities;
- synchronization client.

It does not decide:

- user authority;
- tenant authority;
- final sale truth;
- payment settlement;
- authorization to modify another user or branch;
- final tax validity.

## 6.3 Desktop

Tauri is the desktop shell.

TypeScript is narrowly scoped to UI concerns and selected tooling.

Rust commands exposed to the webview must be:

- narrow;
- allowlisted;
- input validated;
- capability constrained;
- authorization aware where they touch protected business state.

## 6.4 TypeScript

TypeScript is permitted where the web ecosystem provides disproportionate leverage:

- admin web UI;
- desktop webview UI;
- generated API clients;
- validation shared with UI where useful;
- developer tooling;
- build/configuration support;
- documentation tooling.

Business authority should not be duplicated independently in TypeScript.

---

# 7. Repository Target Structure

The repository must support strong ownership and dependency direction.

Recommended shape:

```text
sitolo/
|
+-- Cargo.toml
+-- Cargo.lock
+-- rust-toolchain.toml
+-- deny.toml
+-- .gitignore
+-- .cargo/
|
+-- crates/
|   |
|   +-- sitolo-domain/
|   +-- sitolo-application/
|   +-- sitolo-auth/
|   +-- sitolo-authz/
|   +-- sitolo-tenant/
|   +-- sitolo-api/
|   +-- sitolo-db/
|   +-- sitolo-events/
|   +-- sitolo-outbox/
|   +-- sitolo-security/
|   +-- sitolo-observability/
|   +-- sitolo-integrations/
|   +-- sitolo-workers/
|   +-- sitolo-testkit/
|   +-- sitolo-cli/
|
+-- apps/
|   |
|   +-- admin-web/
|   +-- desktop/
|   +-- mobile/
|
+-- migrations/
|
+-- tests/
|   |
|   +-- security/
|   +-- integration/
|   +-- contract/
|   +-- concurrency/
|   +-- fuzz/
|   +-- fixtures/
|
+-- infra/
|   |
|   +-- docker/
|   +-- terraform/
|   +-- deployment/
|   +-- policies/
|
+-- scripts/
|
+-- docs/
|   |
|   +-- architecture/
|   +-- security/
|   +-- api/
|   +-- database/
|   +-- operations/
|   +-- adr/
|   +-- runbooks/
|
+-- .github/
    |
    +-- workflows/
    +-- CODEOWNERS
    +-- pull_request_template.md
```

The exact structure can change, but three boundaries should remain:

```text
DOMAIN
  cannot depend on HTTP

DOMAIN
  cannot depend on Flutter/Tauri

AUTHORIZATION
  cannot be implemented only in HTTP handlers
```

## 7.1 Dependency direction

Preferred direction:

```text
API / adapters
      |
      v
APPLICATION
      |
      +------> AUTHZ
      |
      +------> DOMAIN
      |
      +------> PORTS
                  |
                  +--> DB
                  +--> OUTBOX
                  +--> EXTERNAL ADAPTERS
```

Avoid:

```text
DOMAIN -> AXUM
DOMAIN -> SQLx-specific web extraction
DOMAIN -> Redis
DOMAIN -> Flutter
DOMAIN -> external provider SDK
```

This preserves testability and makes security boundaries obvious.

---

# 8. Workspace Rules

The Cargo workspace is part of the security and reproducibility model.

## 8.1 Rust toolchain

`rust-toolchain.toml` must pin the intended stable toolchain.

Example:

```toml
[toolchain]
channel = "1.98.1"
profile = "minimal"
components = ["rustfmt", "clippy"]
```

The exact final configuration is subject to toolchain validation in CI.

## 8.2 Lockfile

`Cargo.lock` must be version-controlled for Sitolo's application/workspace builds.

CI uses:

```text
cargo check --workspace --locked
cargo test --workspace --locked
cargo build --workspace --locked
```

A lockfile change is intentional source-controlled change, not incidental CI noise.

Cargo documents `--locked` as failing when the lockfile is missing or when dependency resolution would need to change. This behavior is desirable for deterministic CI.

## 8.3 Dependency changes

Every dependency change should answer:

- why it is needed;
- which crate owns it;
- whether it expands the attack surface;
- whether a transitive dependency introduces risk;
- whether the dependency is maintained;
- whether its license is acceptable;
- whether the dependency can be removed by using an existing capability.

Do not add dependencies for trivial functionality that can be implemented safely with existing standard/library facilities.

## 8.4 Unsafe Rust

Default policy:

```text
unsafe = denied where practical
```

Any required `unsafe` code must have:

- explicit justification;
- local scope;
- safety invariants documented;
- focused tests;
- reviewer approval;
- security impact assessment.

---

# 9. Application Layering

Sensitive operations must follow a predictable path.

```text
HTTP / SYNC / WEBHOOK / CLI / JOB
            |
            v
       TRANSPORT BOUNDS
            |
            v
       AUTHENTICATION
            |
            v
       SESSION / DEVICE
            |
            v
        TENANT SCOPE
            |
            v
       FUNCTION AUTHZ
            |
            v
     INPUT / SCHEMA VALIDATION
            |
            v
      OBJECT AUTHORIZATION
            |
            v
      PROPERTY AUTHORIZATION
            |
            v
     DOMAIN STATE VALIDATION
            |
            v
     TRANSACTION / LOCKING
            |
            v
      AUDIT + OUTBOX
            |
            v
       SAFE RESPONSE
```

This order is a default control sequence.

Individual endpoints may combine or optimize steps internally, but they must not create a path that skips an applicable control.

---

# 10. Security Context Model

A request context should represent the trusted security state that has been proven so far.

Conceptually:

```rust
pub struct SecurityContext {
    pub principal: Principal,
    pub session: SessionContext,
    pub device: Option<DeviceContext>,
    pub tenant: TenantScope,
    pub permissions: PermissionSet,
    pub authn_strength: AuthnStrength,
    pub correlation: CorrelationContext,
}
```

This is conceptual; exact implementation may differ.

The critical rule is that the context is built from verified evidence, not copied blindly from request JSON.

## 10.1 Tenant context

Example:

```rust
pub struct TenantScope {
    pub organization_id: OrganizationId,
    pub branch_ids: BranchScope,
    pub warehouse_ids: WarehouseScope,
    pub register_ids: RegisterScope,
}
```

A caller-supplied `organization_id` is not trusted merely because it parses correctly.

---

# 11. Authentication Implementation Contract

Sitolo should integrate standards-based identity rather than inventing an authentication protocol.

## 11.1 Authentication boundary

The identity integration must produce a normalized internal principal containing at least:

```text
principal_id
identity_provider
authentication_time
session_id
authentication_strength
mfa_state
credential_version / session generation where applicable
```

The application must not store raw provider-specific tokens throughout business modules.

## 11.2 Passwords

If passwords are enabled:

- use a maintained Argon2id implementation;
- never store plaintext;
- never log passwords;
- never expose password hashes to clients;
- rate-limit authentication attempts;
- apply recovery protections;
- define password change behavior;
- revoke or rotate relevant sessions after high-risk account changes.

## 11.3 Public/mobile authentication

For public/native authentication, use a standards-based flow with PKCE and exact redirect registration where applicable.

Do not construct a custom OAuth-like flow.

## 11.4 MFA

Platform administrators must use MFA.

Merchant administrators and high-risk roles should be enforceable by policy.

High-risk actions may require step-up authentication even when the existing session is valid.

Examples:

- changing payout settings;
- changing security policy;
- resetting MFA for another user;
- ownership transfer;
- high-value refund approval;
- bulk export;
- administrator role assignment.

## 11.5 Recovery

Recovery is a privileged authentication path.

Implementation requirements:

- one-time reset artifact;
- short lifetime;
- server-side invalidation after redemption;
- rate limiting;
- anti-enumeration responses;
- relevant session invalidation;
- audit event;
- no elevation of privileges by virtue of recovery.

## 11.6 Session lifecycle

Session states must be explicit:

```text
CREATED
ACTIVE
REAUTH_REQUIRED
REVOKED
EXPIRED
```

Refresh mechanisms must support replay detection or equivalent protection.

---

# 12. Authorization Implementation Contract

Authorization is multi-dimensional.

```text
IDENTITY
   +
TENANT
   +
SCOPE
   +
PERMISSION
   +
RESOURCE
   +
PROPERTY
   +
STATE
   +
RISK / APPROVAL
   =
ALLOW / DENY
```

## 12.1 Permission model

Permission names should be stable machine identifiers.

Examples:

```text
SALES_CREATE
SALES_VIEW
SALES_VOID
SALES_REFUND_REQUEST
SALES_REFUND_APPROVE
INVENTORY_VIEW
INVENTORY_RECEIVE
INVENTORY_ADJUST_REQUEST
INVENTORY_ADJUST_APPROVE
PROCUREMENT_CREATE
PROCUREMENT_APPROVE
CASH_OPEN
CASH_CLOSE
REPORT_VIEW
REPORT_EXPORT
USER_MANAGE
ROLE_ASSIGN
SECURITY_POLICY_CHANGE
DEVICE_REVOKE
PAYMENT_RECONCILE
```

The final permission registry should be domain-owned rather than duplicated across clients.

## 12.2 Scope

A permission may be constrained by:

- organization;
- branch;
- warehouse;
- register;
- module;
- resource ownership;
- monetary threshold;
- approval state.

## 12.3 Function authorization

A route must not merely check “logged in”.

Example:

```text
POST /sales/{id}/refund
```

must evaluate:

1. authenticated principal;
2. current tenant membership;
3. branch scope;
4. refund permission;
5. object ownership/scope;
6. sale state;
7. refundable amount;
8. approval threshold;
9. step-up requirement if configured.

## 12.4 Property authorization

Request DTOs must not expose internal writable fields simply because they exist in the database.

Bad:

```json
{
  "role": "OWNER",
  "organization_id": "victim",
  "is_platform_admin": true
}
```

Good:

```json
{
  "display_name": "Example Business",
  "phone": "..."
}
```

with privileged changes handled by separate commands.

---

# 13. Tenant Isolation Implementation Contract

This is one of the highest-priority implementation boundaries.

## 13.1 Trusted tenant derivation

Tenant scope is resolved from:

```text
Authenticated Principal
       |
       v
Memberships
       |
       v
Role / Permission
       |
       v
Branch / Resource Scope
```

not:

```text
Request JSON -> organization_id -> trust
```

## 13.2 Repository API design

Prefer:

```text
get_product(scope, product_id)
```

instead of:

```text
get_product(product_id)
```

Prefer:

```text
list_sales(scope, filter)
```

instead of:

```text
list_sales(filter)
```

This makes omission of security scope harder.

## 13.3 Composite tenancy

Tenant-owned uniqueness should normally incorporate the tenant boundary where required.

Example conceptual constraint:

```text
UNIQUE (organization_id, sku)
```

not global uniqueness unless global uniqueness is actually a business requirement.

## 13.4 Background jobs

Every background job must carry tenant scope explicitly where business data is tenant-specific.

A worker must not infer tenant from:

- object-storage path alone;
- queue topic alone;
- mutable global configuration;
- process-global state.

## 13.5 Cache isolation

Cache keys must contain all dimensions that influence authorization.

Example:

```text
sitolo:v1:tenant:{tenant}:branch:{branch}:product:{id}
```

rather than:

```text
sitolo:product:{id}
```

when the same object identifier can exist in multiple tenant scopes.

## 13.6 Search/report isolation

Every read model used for:

- reports;
- exports;
- search;
- dashboards;
- autocomplete;
- analytics;

must preserve tenant and scope filtering.

---

# 14. PostgreSQL Security Implementation

PostgreSQL is authoritative for server-side business truth.

## 14.1 Role separation

At minimum:

```text
sitolo_migrator
sitolo_app
sitolo_report
sitolo_backup
sitolo_breakglass
```

### `sitolo_migrator`

Can perform controlled schema changes.

Must not be the runtime application credential.

### `sitolo_app`

Can perform only required DML.

Must not:

- create roles;
- alter schema;
- bypass security policy without explicit architectural justification;
- access unrelated administrative databases.

### `sitolo_report`

Read-only access to approved reporting views/read models.

### `sitolo_backup`

Separate operational identity.

### `sitolo_breakglass`

Exceptional access only.

Must require:

- MFA;
- explicit approval;
- audit;
- time limitation;
- post-use review.

## 14.2 RLS

Row-Level Security is defense in depth, especially for critical tenant-owned data.

Application scope enforcement remains required.

The application must not assume RLS alone is sufficient because configuration mistakes, privileged roles and migration paths can bypass it.

The runtime account must not be an unnecessary superuser or bypass role.

## 14.3 Constraints

Use database constraints for business/security invariants wherever practical.

Examples:

```text
NOT NULL
FOREIGN KEY
UNIQUE
CHECK
EXCLUSION WHERE JUSTIFIED
```

Application validation improves user experience; database constraints enforce the final invariant.

## 14.4 Transactions

Critical mutations must use explicit transaction boundaries.

Example:

```text
BEGIN
  resolve scope
  lock required state
  re-read authoritative data
  validate business invariant
  write primary mutation
  write ledger/movement
  write audit event
  write outbox event
COMMIT
```

If commit succeeds and a response is lost, idempotency determines whether the retry returns the original result or a safe equivalent outcome.

---

# 15. Financial Integrity Contract

Sitolo's financial history is append-oriented.

A finalized transaction is not silently edited to change economic meaning.

## 15.1 Sale

A finalized sale should include immutable commercial facts such as:

- organization;
- branch/location;
- register/device;
- seller;
- line items;
- quantities;
- unit prices;
- discount snapshot;
- tax snapshot;
- currency;
- payment allocation;
- source command/event identifiers;
- timestamps.

## 15.2 Correction model

```text
ORIGINAL SALE
     |
     +---- RETURN
     |
     +---- REFUND
     |
     +---- REVERSAL
     |
     +---- CORRECTED TRANSACTION
```

No “edit completed sale total” endpoint should exist.

## 15.3 Money representation

Never represent monetary values as floating-point business truth.

Use a deterministic representation appropriate to the supported currencies and tax model, typically an integer minor-unit representation or safe decimal type with explicit scale.

The final implementation must define:

- currency code;
- precision;
- rounding mode;
- tax calculation order;
- discount calculation order;
- allocation rules;
- display formatting versus stored value.

## 15.4 Refund invariant

```text
requested_refund <= refundable_balance
```

This must be enforced inside the authoritative transaction.

---

# 16. Inventory Security and Integrity

Inventory is a ledger-backed state projection.

Conceptually:

```text
Opening
+ Purchase Receipt
+ Positive Adjustment
+ Transfer In
+ Customer Return
- Sale
- Supplier Return
- Damage
- Expiry
- Transfer Out
= Inventory State
```

## 16.1 Atomic stock decrement

A stock mutation must be atomic.

Conceptual SQL pattern:

```sql
UPDATE inventory_balance
SET quantity_available = quantity_available - $1,
    version = version + 1
WHERE organization_id = $2
  AND location_id = $3
  AND product_id = $4
  AND quantity_available >= $1;
```

Require exactly one affected row for a successful decrement under a non-negative stock policy.

## 16.2 No in-memory locks as authority

Do not rely on a process-local mutex as the only concurrency control.

Multiple API instances do not share application memory.

Use PostgreSQL transaction/locking/constraint semantics as the authoritative coordination mechanism.

## 16.3 Stock adjustments

High-risk adjustment paths should include:

```text
REQUESTED
   |
   v
REVIEWED
   |
   v
APPROVED
   |
   v
POSTED
```

The business can configure thresholds where a second actor is required.

## 16.4 Pharmacy and agro-dealer extension

Core inventory types must be extensible for:

- batch/lot;
- expiry;
- quarantine;
- recall;
- units/conversions;
- traceability.

Do not contaminate the generic retail core with hard-coded pharmacy-only assumptions.

---

# 17. API Implementation Contract

Every route must have a security classification.

```text
PUBLIC
AUTHENTICATED_LOW_RISK
AUTHENTICATED_SENSITIVE
PRIVILEGED_ADMIN
INTERNAL
WEBHOOK
```

Classification determines:

- authentication;
- authorization;
- rate limits;
- timeout budget;
- logging policy;
- response projection;
- audit requirements;
- test suite;
- exposure policy.

## 17.1 Route inventory

The implementation must maintain a machine-readable route inventory.

For each route:

```text
route
method
security class
authn requirement
authz permission
scope
request schema
response schema
mutation type
idempotency requirement
timeout
rate limit
audit event
owner
```

## 17.2 No hidden privileged endpoints

Routes such as:

```text
/debug
/admin
/internal
/metrics
/health
```

are not secure merely because their names suggest restricted use.

Explicit exposure policy is required.

## 17.3 Versioning

External API contracts must be versioned deliberately.

Avoid breaking changes that force old mobile clients to execute unknown semantics without a compatibility strategy.

---

# 18. Input Validation Contract

Validation is layered.

```text
TRANSPORT LIMITS
      |
      v
TYPE / SYNTAX
      |
      v
SEMANTIC VALIDATION
      |
      v
AUTHORIZATION
      |
      v
BUSINESS STATE
```

## 18.1 Request limits

Define explicit maximums for:

- body bytes;
- header bytes;
- list elements;
- nested object depth;
- string lengths;
- integer ranges;
- file size;
- sync batch size;
- export range;
- report range.

The exact values should be based on measured workload and documented per route class.

## 18.2 Unknown-field policy

Sensitive mutation DTOs should reject or explicitly ignore unknown fields according to the contract.

Silent mass assignment is prohibited.

## 18.3 Dynamic filters

Search and report filters must be represented as typed values.

Do not accept arbitrary SQL fragments, expression languages, provider-specific query operators or executable formulas from clients.

---

# 19. SQL Injection Contract

SQLx bindings are the default.

Bad:

```text
"SELECT ... WHERE sku = '" + sku + "'"
```

Good:

```rust
sqlx::query!(
    "SELECT ... WHERE organization_id = $1 AND sku = $2",
    organization_id,
    sku,
)
```

Dynamic identifiers cannot be safely parameterized as values.

Therefore dynamic sort/filter behavior must map from closed input values to static SQL fragments.

Example:

```text
name       -> p.name
created_at -> p.created_at
price      -> p.price
anything else -> reject
```

Required tests include:

- quote manipulation;
- SQL comment syntax;
- boolean injection;
- encoded variants;
- Unicode edge cases;
- malformed UTF-8 handling at transport boundaries where applicable;
- dynamic sort injection attempts.

---

# 20. NoSQL / Search Injection Contract

PostgreSQL is authoritative, so classic NoSQL injection is not the primary current risk.

However, any future:

- Redis query-like representation;
- search engine filter;
- document store;
- scripting layer;

must treat client JSON as data, not as query operators.

Do not accept:

```text
{"$where": ...}
{"$regex": attacker_expression}
```

or equivalent expressions unless a strongly typed internal representation deliberately generates the query.

---

# 21. Web Security Contract

The TypeScript/web layer is considered hostile-client-facing code.

## 21.1 XSS

Merchant-created fields must be rendered as text unless rich HTML is explicitly required.

Examples requiring XSS scrutiny:

- product names;
- notes;
- customer names;
- supplier descriptions;
- report labels;
- admin messages;
- imported data.

Use contextual encoding and a strict Content Security Policy.

Sanitization should be narrowly applied to genuine rich-text use cases rather than used as a universal excuse for unsafe rendering.

## 21.2 CSRF

Browser endpoints using cookie authentication require CSRF protection.

SameSite cookies are a defense layer, not the entire control.

High-impact browser mutations should additionally validate request origin signals where practical.

## 21.3 CORS

Production CORS must use explicit allowlists.

Prohibited:

```text
Access-Control-Allow-Origin: *
```

combined with credentials.

Do not blindly mirror arbitrary `Origin` values.

CORS is not authorization.

## 21.4 Browser storage

Do not store long-lived authentication secrets in JavaScript-readable browser storage.

Prefer secure, HttpOnly session cookies or an architecture designed around a backend-for-frontend boundary where suitable.

## 21.5 Source maps

Production source maps containing sensitive application internals must not be publicly exposed by default.

Private maps may be uploaded to controlled error-tracking systems.

CI should inspect emitted bundles and source maps for:

- secrets;
- private endpoints;
- development credentials;
- internal hostnames where disclosure would be harmful.

---

# 22. Flutter Security Contract

The mobile application must assume:

```text
NETWORK = HOSTILE
DEVICE STORAGE = COMPROMISEABLE
CLIENT PROCESS = COMPROMISEABLE
CLOCK = MANIPULABLE
USER INPUT = HOSTILE
```

## 22.1 Secure storage

Use OS secure storage for:

- refresh-token material;
- device secrets;
- local encryption keys.

Do not keep provider credentials in the mobile package.

## 22.2 Local database

SQLite is a local operational store, not the platform authority.

Store only data required for the device's assigned scope and workflow.

## 22.3 Deep links

Deep links must be parsed and validated.

Do not allow an external deep link to invoke privileged operations without server-side validation.

## 22.4 Logout

Logout must remove or invalidate relevant local session material.

Server-side session revocation remains authoritative.

## 22.5 Rooted/jailbroken devices

Device compromise cannot be perfectly prevented at the application layer.

The architecture therefore limits blast radius:

- short-lived credentials;
- revocable device identity;
- limited local replication;
- bounded offline authority;
- minimized secrets.

---

# 23. Tauri Security Contract

Tauri's native side must not become a generic local privilege bridge.

## 23.1 Commands

Every command should:

- expose one narrow capability;
- validate inputs;
- avoid arbitrary shell execution;
- avoid arbitrary filesystem access;
- enforce path boundaries;
- reject unexpected origins where applicable.

Bad:

```text
execute_shell(command_from_webview)
```

Good:

```text
export_report(report_id)
```

with server-authorized report semantics.

## 23.2 External navigation

Allowlist external destinations.

## 23.3 Auto-update

Updates must be authenticated and integrity-verified before installation.

## 23.4 Secrets

No production provider credential belongs in the desktop package.

---

# 24. File Upload Security Contract

Every uploaded file is hostile.

Pipeline:

```text
REQUEST
  |
  v
AUTHORIZATION
  |
  v
TENANT / QUOTA
  |
  v
STREAM SIZE LIMIT
  |
  v
TYPE / SIGNATURE CHECK
  |
  v
GENERATED OBJECT ID
  |
  v
QUARANTINE
  |
  v
SCAN / SANDBOX WHERE REQUIRED
  |
  v
PRIVATE OBJECT STORE
  |
  v
AUTHORIZED EXPIRING DOWNLOAD
```

## 24.1 Never

Never do:

```text
user filename
    -> filesystem path
    -> executable/web-root location
```

## 24.2 Imports

Imports are bulk mutation requests.

Required process:

```text
UPLOAD
  -> SCAN
  -> PARSE
  -> VALIDATE
  -> PREVIEW
  -> APPROVE
  -> COMMIT
```

Imported organization/tenant identifiers must never override the authenticated scope.

---

# 25. Path Traversal Contract

Prefer object IDs to filesystem names.

Where local paths are unavoidable:

1. establish an approved root;
2. normalize/canonicalize safely;
3. reject absolute paths;
4. reject traversal;
5. reject encoded traversal variants;
6. verify final path remains inside root.

Never trust uploaded filenames.

---

# 26. SSRF Contract

Preferred approach:

> **Do not provide arbitrary outbound URL fetching unless the business case is strong enough to justify the security cost.**

For permitted outbound fetching:

- allowlist protocols;
- allowlist destinations;
- resolve DNS carefully;
- block loopback/private/link-local/metadata targets;
- control redirects;
- enforce connection and read timeouts;
- cap response size;
- restrict egress at the network layer;
- isolate risky fetches in workers;
- prevent access to the database and cloud metadata from the fetching process.

---

# 27. Secrets and Configuration Contract

Environment variables are configuration delivery, not a secret-management strategy by themselves.

## 27.1 Secret classes

```text
Identity secrets
Database secrets
Payment secrets
EIS secrets
Webhook signing secrets
Encryption keys
Signing keys
Cloud credentials
CI federation configuration
Device secrets
```

## 27.2 Secret locations prohibited

Do not place secrets in:

- Git history;
- source code;
- comments;
- fixtures containing real data;
- public `.env` files;
- static assets;
- container images/layers;
- JavaScript bundles;
- source maps;
- logs;
- traces;
- issue comments;
- PR descriptions;
- CI artifacts.

## 27.3 Secret lifecycle

```text
PROVISION
   |
   v
STORE
   |
   v
USE
   |
   v
AUDIT
   |
   v
ROTATE
   |
   v
REVOKE
   |
   v
DESTROY
```

## 27.4 Compromise response

```text
DETECT
  -> CLASSIFY
  -> REVOKE
  -> ROTATE
  -> INSPECT BLAST RADIUS
  -> VERIFY OLD CREDENTIAL FAILS
  -> FIX LEAK PATH
  -> ADD REGRESSION CONTROL
```

---

# 28. Configuration Contract

Configuration must be explicit and environment-aware.

Recommended configuration categories:

```text
server
security
identity
database
cache
storage
payments
eis
notifications
observability
limits
feature_flags
```

Each setting must have:

- type;
- default where safe;
- required/optional status;
- allowed range;
- secret/non-secret classification;
- environment applicability;
- startup failure behavior.

A missing production security configuration should normally fail startup rather than silently choose an insecure default.

---

# 29. Logging, Audit and Telemetry Implementation

Separate:

```text
OPERATIONAL LOGS
SECURITY EVENTS
BUSINESS AUDIT
METRICS
TRACES
```

## 29.1 Operational logs

Purpose:

- debugging;
- reliability;
- performance;
- dependency diagnosis.

Never log:

- bearer tokens;
- passwords;
- reset artifacts;
- private keys;
- webhook secrets;
- raw payment credentials;
- unnecessary full PII.

## 29.2 Security events

Examples:

```text
AUTH_FAILURE
MFA_FAILURE
SESSION_REVOKED
DEVICE_REVOKED
BOLA_DENIED
PRIVILEGE_ESCALATION_ATTEMPT
ROLE_CHANGED
EXPORT_REQUESTED
WEBHOOK_SIGNATURE_FAILED
WEBHOOK_REPLAY_REJECTED
SECRET_DETECTED
CI_SECURITY_GATE_FAILED
```

## 29.3 Business audit

At minimum capture:

```text
event_id
occurred_at
tenant_id
actor_id
actor_type
action
resource_type
resource_id
result
reason_code
request_id
trace_id
device_id
approval_id
provider_ref
```

## 29.4 Sensitive metric labels

Do not use arbitrary user-controlled values as high-cardinality metric labels.

Avoid:

```text
metric{username="..."}
metric{raw_error="..."}
metric{product_name="..."}
```

Use bounded identifiers/categories instead.

---

# 30. Error Contract

Clients receive stable low-information error responses.

Example:

```json
{
  "error": {
    "code": "AUTHORIZATION_DENIED",
    "message": "You do not have permission to perform this operation.",
    "request_id": "..."
  }
}
```

Never return production client-visible:

- stack traces;
- SQL statements;
- server file paths;
- environment variables;
- internal network details;
- raw provider responses containing secrets or sensitive diagnostic information.

Internally, the system may retain richer structured diagnostics under restricted access.

---

# 31. Timeouts and Resource Budgets

Every meaningful resource consumer must have a bound.

## 31.1 Required budgets

At minimum define budgets for:

- HTTP request;
- connection establishment;
- DB statement;
- DB transaction;
- provider call;
- webhook processing;
- file upload;
- import parsing;
- report generation;
- export generation;
- sync batch;
- worker execution;
- retry count;
- queue depth;
- response size;
- body size.

## 31.2 Why this is security

A hung external provider can exhaust all worker tasks.

An unlimited report can exhaust database connections.

An unbounded sync batch can exhaust memory.

A parser with no limit can become a denial-of-service primitive.

Therefore missing timeouts and resource limits are security defects.

---

# 32. Retry Contract

Never build a global helper called:

```text
retry_everything()
```

Retryability is operation-specific.

## 32.1 Retry safe candidates

Examples may include:

- transient network failures;
- selected database serialization failures;
- known retryable provider errors.

## 32.2 Retry unsafe candidates

Do not blindly retry:

- non-idempotent money movement;
- unknown commit outcome without idempotency;
- permanent authorization denial;
- validation failure;
- invalid signature.

Retries require:

- bounded count;
- backoff;
- jitter;
- deadline awareness;
- idempotency analysis.

---

# 33. Idempotency Contract

Every command that can safely arrive again should have a stable command or idempotency identity.

Examples:

```text
Create sale
Create refund
Create payment intent
Process webhook
Submit offline command
Create stock adjustment
```

The system should be able to distinguish:

```text
same command repeated
```

from:

```text
different command with similar business reference
```

The database must enforce uniqueness where financial duplication would matter.

---

# 34. Webhook Security Contract

Webhook processing sequence:

```text
RECEIVE
  |
  v
SIZE LIMIT
  |
  v
AUTHENTICITY / SIGNATURE
  |
  v
SCHEMA VALIDATION
  |
  v
REPLAY / EVENT-ID CHECK
  |
  v
PERSIST / DEDUPLICATE
  |
  v
STATE VALIDATION
  |
  v
ASYNC BUSINESS EFFECT
  |
  v
AUDIT
```

## 34.1 Signature

Verify the exact signed representation required by the provider.

Do not verify a reconstructed or normalized payload when the provider's signature semantics require the raw payload.

## 34.2 Event uniqueness

Use database uniqueness such as:

```text
UNIQUE(provider, provider_event_id)
```

and where relevant:

```text
UNIQUE(provider, provider_transaction_id)
```

## 34.3 Unknown webhook behavior

Unknown or contradictory external events become reconciliation exceptions.

They must not silently rewrite payment truth.

---

# 35. Payment Security Contract

Sitolo is a transaction/orchestration/reconciliation layer, not a bank or wallet by default.

## 35.1 Payment intent

A payment intent should bind:

```text
organization
sale
expected amount
currency
provider
client reference
status
created_at
```

## 35.2 Frontend is non-authoritative

Client fields such as:

```text
paid=true
amount=...
provider_reference=...
currency=...
```

are claims.

The backend validates them against the payment intent and trusted provider evidence.

## 35.3 Reconciliation priority

Preferred evidence order:

1. provider transaction ID;
2. provider-defined reference;
3. Sitolo payment intent reference;
4. tightly controlled exact-match fallback;
5. manual review.

No ambiguous matching may silently produce a successful financial state.

---

# 36. Offline Security Implementation Contract

Offline operation is a controlled authority delegation problem.

The device can continue selected operations but only within bounded permissions.

## 36.1 Offline command envelope

Conceptual fields:

```text
command_id
device_id
schema_version
operation
created_at_client
sequence / causal metadata where required
payload
integrity metadata where required
```

The server verifies:

- device identity;
- device status;
- tenant membership;
- allowed operation;
- command uniqueness;
- command age;
- semantic validity;
- current business state.

## 36.2 Device revocation

Device lifecycle:

```text
REGISTERED
   |
   v
ACTIVE
   |
   +--> SUSPENDED
   |
   +--> REVOKED
   |
   +--> RETIRED
```

Device IDs are not secrets.

## 36.3 Bounded offline authority

A cashier device may be allowed to create sales offline.

It should not be allowed offline to:

- change organization security policy;
- grant owner access;
- change payout credentials;
- disable MFA enforcement;
- access platform-wide administration;
- arbitrarily modify another branch.

## 36.4 Time manipulation

Client time is evidence, not security authority.

Use server timestamps and bounded acceptance windows where security-sensitive expiration matters.

## 36.5 Tampered local database

Assume SQLite data can be modified on a compromised device.

The server must not treat local state as proof of authority.

---

# 37. Sync Protocol Implementation Requirements

The final sync protocol document will specify wire details, but implementation must preserve these invariants.

## 37.1 Required properties

- durable command identity;
- schema versioning;
- duplicate rejection/idempotency;
- checkpoint semantics;
- bounded batch size;
- bounded command age;
- explicit acknowledgment;
- partial failure handling;
- conflict classification;
- rejection reasons;
- replay resistance;
- revocation checks;
- audit evidence.

## 37.2 Do not use generic last-write-wins

Financial and stock semantics cannot safely be inferred from generic document timestamps.

Example:

```text
DEVICE A sells last unit
DEVICE B sells last unit
```

The server must apply business concurrency rules rather than allow whichever update arrived last to win.

---

# 38. Business Logic Abuse Controls

Attackers do not always need malformed input.

A legitimate user may abuse legitimate commands.

Examples:

- repeated refunds;
- excessive discounts;
- stock adjustments just below approval threshold;
- export then privilege revocation;
- repeated failed OTP requests;
- creating many devices;
- creating many branches;
- generating expensive reports;
- repeated webhook-triggering actions.

## 38.1 Controls

Use:

- approval thresholds;
- rate limits;
- quotas;
- transaction limits;
- audit;
- anomaly detection;
- separation of duties;
- step-up authentication;
- cool-down periods where economically justified.

---

# 39. Race Condition Implementation Contract

The following are explicitly race-sensitive:

- inventory decrement;
- stock adjustment approval/posting;
- refund amount eligibility;
- payment settlement;
- cash close;
- user role changes;
- device revocation;
- idempotency insertion;
- webhook deduplication.

The general critical mutation pattern is:

```text
BEGIN
  |
  v
RESOLVE AUTHORIZED SCOPE
  |
  v
LOCK / SELECT AUTHORITATIVE STATE
  |
  v
REVALIDATE INVARIANTS
  |
  v
APPLY TRANSITION
  |
  v
WRITE AUDIT + OUTBOX
  |
  v
COMMIT
```

Deadlock/serialization retry must be bounded and conditional on safe retry semantics.

---

# 40. Outbox and Worker Contract

The preferred early architecture uses PostgreSQL transactional outbox plus Rust workers before adopting a larger external broker.

## 40.1 Transactional outbox rule

A database transaction should record:

```text
business state
+
outbox event
```

atomically.

The worker then delivers the side effect.

This prevents:

```text
DB COMMITTED
but event LOST
```

## 40.2 Worker safety

Workers must have:

- bounded concurrency;
- timeouts;
- retry policy;
- idempotent consumers;
- dead-letter handling where required;
- structured correlation;
- tenant context;
- cancellation behavior;
- backpressure.

## 40.3 Worker trust

A worker is not automatically trusted to do anything.

Each job type receives only the permissions required for its function.

---

# 41. External Integration Contract

Each integration is a trust boundary and must expose an adapter interface.

```text
Sitolo Domain
      |
      v
Integration Port
      |
      +--> Payment Adapter
      +--> MRA EIS Adapter
      +--> Notification Adapter
      +--> Storage Adapter
      +--> Future Provider
```

Provider SDKs must not leak into the domain layer unless an ADR explicitly approves it.

Each adapter defines:

- authentication;
- request schema;
- response schema;
- timeout;
- retry policy;
- idempotency;
- reconciliation;
- error mapping;
- audit mapping;
- observability.

---

# 42. MRA EIS Implementation Boundary

The tax integration must preserve Sitolo business truth even when MRA/EIS is unavailable.

Failure path:

```text
SALE COMMITTED LOCALLY
        |
        v
TAX STATUS = PENDING
        |
        v
EIS SUBMISSION
        |
   +----+----+
   |         |
SUCCESS   RETRYABLE FAILURE
   |         |
CONFIRMED   RETRY
              |
              v
        PERMANENT FAILURE
              |
              v
        TAX EXCEPTION
```

A failed external submission must not rewrite the underlying sale economic facts merely to make tax submission succeed.

The exact production certification, credentials, protocol and regulatory behavior remain evidence-driven and must be updated from current authoritative MRA material before production enablement.

---

# 43. Admin and Support Security

Administrative capabilities are high-value attack surfaces.

## 43.1 Roles

At minimum distinguish:

- platform operations;
- security administration;
- billing administration;
- support;
- audit/read-only;
- break-glass.

## 43.2 Just-in-time support

Support access to merchant data should be:

```text
REQUEST
  -> APPROVE
  -> SCOPE
  -> ACTIVATE
  -> EXPIRE
  -> AUDIT
```

## 43.3 Impersonation

Any customer impersonation capability must:

- be explicit;
- display that impersonation is active;
- be time-limited;
- preserve original support actor identity;
- preserve target tenant scope;
- prohibit silent privilege escalation;
- generate audit evidence.

---

# 44. Export Security Contract

Exports are bulk exfiltration primitives.

Controls:

- explicit permission;
- tenant scope;
- branch scope;
- bounded date range;
- quota;
- audit;
- asynchronous execution for expensive jobs;
- private object storage;
- expiring signed downloads;
- retention/expiry;
- optional approval for sensitive exports.

A user may be allowed to view records but not necessarily allowed to export them.

---

# 45. Reporting Security

Reports must not become a tenant bypass because they use a separate read model.

All reporting pipelines must preserve:

- organization scope;
- branch scope;
- role restrictions;
- field restrictions;
- sensitive-data policy.

Large reports execute asynchronously.

Report requests themselves are permission-controlled.

---

# 46. Cache Security

Redis is optional and non-authoritative.

Never store authoritative financial truth only in cache.

Cache invalidation is security-sensitive whenever permissions or tenant membership affect the result.

Therefore:

```text
ROLE CHANGE
  |
  v
INVALIDATE AUTHZ-SENSITIVE CACHE
```

and:

```text
TENANT-SCOPE CHANGE
  |
  v
INVALIDATE / VERSION AFFECTED CACHE
```

A stale cache must never upgrade authority.

---

# 47. Cloud and Network Implementation Contract

Baseline topology:

```text
                 PUBLIC INTERNET
                       |
                       v
                DNS / CDN / WAF
                       |
                       v
                 TLS TERMINATION
                       |
                       v
                 PUBLIC INGRESS
                       |
                       v
              +------------------+
              | RUST API         |
              | RUST WORKERS     |
              +--------+---------+
                       |
                PRIVATE DATA NET
                       |
          +------------+-----------+
          |            |           |
          v            v           v
     PostgreSQL      Redis      Object Store
```

Administrative control paths remain separately protected.

## 47.1 Prohibited public exposure

By default:

- PostgreSQL public port: prohibited
- Redis public port: prohibited
- unrestricted storage bucket: prohibited
- production debug interface: prohibited
- unrestricted management endpoint: prohibited

## 47.2 Egress

Outbound destinations should be restricted according to actual integration requirements.

A compromised process should not automatically have unrestricted internet access.

---

# 48. CI/CD Security Implementation Contract

CI is a production trust boundary.

Two planes:

```text
UNTRUSTED VALIDATION PLANE
--------------------------
Pull requests / attacker-controlled code
No production secrets
No production signing keys
No unrestricted deployment identity

TRUSTED RELEASE PLANE
---------------------
Protected branch/tag
Security gates
Locked dependencies
SBOM
Provenance
Artifact integrity
Protected environment
Short-lived cloud identity
Deployment
```

## 48.1 Workflow security

Workflows must:

- declare minimal token permissions;
- avoid executing untrusted input as shell code;
- avoid exposing secrets to untrusted pull requests;
- pin third-party actions according to repository policy;
- separate trusted deployment workflows from untrusted build workflows;
- use federated short-lived cloud credentials where supported.

## 48.2 Fail closed

If a required security tool:

- fails to start;
- times out unexpectedly;
- returns an empty result when evidence is required;
- is skipped due to conditional logic;
- cannot be installed;
- cannot obtain its required data;

the security gate should fail unless the policy explicitly defines an independently verified equivalent control.

---

# 49. Dependency and Supply-Chain Contract

## 49.1 Locking

Rust:

```text
Cargo.toml
Cargo.lock
rust-toolchain.toml
```

Web:

```text
package.json
lockfile
```

Infrastructure:

```text
provider/module/image versions pinned by policy
```

## 49.2 Scanning

Required classes:

- dependency vulnerabilities;
- source/static analysis;
- secrets;
- licenses/policy;
- container images;
- IaC;
- artifact integrity.

## 49.3 SBOM

Release artifacts should produce a machine-readable SBOM sufficient to identify included components.

## 49.4 Provenance

Release provenance should identify:

- commit;
- builder/workflow;
- toolchain;
- dependency graph;
- artifact identity.

The maturity target should support stronger artifact attestations as deployment infrastructure evolves.

---

# 50. Security Test Architecture

Security tests are first-class engineering assets.

Repository target:

```text
/tests/security/
    auth/
    authz/
    tenant/
    api/
    injection/
    uploads/
    payment/
    webhook/
    offline/
    concurrency/
    ci/
    cloud/
```

## 50.1 Test layers

### Layer 1 — Unit

Test:

- domain invariants;
- permission resolution;
- validators;
- state machines;
- money calculations;
- replay logic;
- error mapping.

### Layer 2 — Integration

Test:

- API + DB;
- RLS;
- transaction boundaries;
- auth/authz;
- outbox;
- payment adapter behavior.

### Layer 3 — Security negative tests

Explicitly attempt forbidden actions.

### Layer 4 — Property tests

Explore broad state spaces for invariants.

### Layer 5 — Fuzz

Target parsers and complex untrusted inputs.

### Layer 6 — Concurrency

Simulate competing operations.

### Layer 7 — DAST

Attack a disposable staging deployment.

### Layer 8 — Manual adversarial review

Threat-led penetration and business-abuse testing.

---

# 51. Mandatory Negative Security Test Fixtures

The test harness must create at least:

```text
TENANT_A
TENANT_B

OWNER_A
MANAGER_A
CASHIER_A
INVENTORY_A
AUDITOR_A

OWNER_B
MANAGER_B
CASHIER_B

BRANCH_A1
BRANCH_A2
BRANCH_B1

DEVICE_A
DEVICE_B
REVOKED_DEVICE

PRODUCT_A
PRODUCT_B
SALE_A
SALE_B
PAYMENT_A
PAYMENT_B
```

Every major resource-level test should prove cross-boundary denial.

Example:

```text
CASHIER_A -> SALE_A -> ALLOW
CASHIER_A -> SALE_B -> DENY
CASHIER_A -> BRANCH_A1 -> ALLOW where scoped
CASHIER_A -> BRANCH_A2 -> DENY where not scoped
CASHIER_A -> OWNER action -> DENY
REVOKED_DEVICE -> SYNC -> DENY
```

---

# 52. Mandatory 48-Control Regression Mapping

Sitolo's security architecture specifies the following controls.

| # | Control | Primary implementation surface | Minimum verification |
|---|---|---|---|
| 1 | Exposed DB credentials | Secret manager / network / CI | Scan + deployment probe |
| 2 | Public `.env` files | Build / web server | HTTP probe + artifact scan |
| 3 | Hardcoded secrets | Source | Secret scanner + review |
| 4 | Weak authentication | Identity layer | Negative auth suite |
| 5 | Missing authz | AuthZ | Route matrix + negative tests |
| 6 | Cross-user access | AuthZ / repository | BOLA tests |
| 7 | Open DB permissions | PostgreSQL | Role inspection tests |
| 8 | Cloud misconfiguration | IaC | IaC scanner + policy |
| 9 | Unprotected admin routes | Admin plane | Authn/Authz tests |
| 10 | Exposed debug | API/infra | Deployment probes |
| 11 | Secret-leaking logs | Telemetry | Redaction tests |
| 12 | Verbose errors | Error boundary | Response tests |
| 13 | Secrets in Git | Repository | Secret scanning/history policy |
| 14 | Secrets in code | Repository/build | Static secret scan |
| 15 | Client-only security | Client/API boundary | Direct API negative tests |
| 16 | Input validation | API/domain | Fuzz + negative tests |
| 17 | SQL injection | SQLx repositories | Injection tests |
| 18 | NoSQL injection | Future query adapters | Policy tests |
| 19 | XSS | Web/Tauri | Payload tests |
| 20 | CSRF | Browser sessions | CSRF suite |
| 21 | Insecure uploads | Storage/upload | Malicious-file suite |
| 22 | Path traversal | File handling | Traversal suite |
| 23 | SSRF | Outbound requests | URL bypass suite |
| 24 | Broken password reset | Identity | Reset/replay tests |
| 25 | Weak sessions | Session service | Rotation/revocation tests |
| 26 | JWT secrets | Identity | Key handling tests |
| 27 | Permissive CORS | Edge/API | Origin matrix |
| 28 | Rate limits | Edge/API | Flood tests |
| 29 | Exposed environments | Deployment | Environment probes |
| 30 | Default credentials | Deployment | Credential-negative tests |
| 31 | Unsigned webhooks | Payments | Signature rejection |
| 32 | Frontend payment checks | Payment service | Tampering tests |
| 33 | IDOR/BOLA | AuthZ | Object swap tests |
| 34 | API + user input | API | Schema/fuzz suite |
| 35 | Exposed logs | Observability | Access-control tests |
| 36 | Exposed source maps | Web build | Asset inspection |
| 37 | No MFA | Identity/admin | Policy test |
| 38 | Enumeration | Identity | Response equivalence tests |
| 39 | Business logic abuse | Domain | Workflow abuse tests |
| 40 | Race conditions | DB/domain | Concurrency suite |
| 41 | Webhook replay | Integration | Replay suite |
| 42 | Insecure CI identity | GitHub/IAM | Workflow policy tests |
| 43 | Untrusted build actions | CI | Review/pinning policy |
| 44 | Unpinned build deps | Dependencies | Lock/pinning checks |
| 45 | Checks fail open | CI | Failure simulation |
| 46 | Missing timeouts | API/DB/integrations | Timeout tests |
| 47 | Sensitive browser storage | Web | Bundle/storage checks |
| 48 | Insecure endpoints | API inventory | Route inventory + DAST |

A release must retain evidence of these verifications.

---

# 53. Security Test Case Catalogue

## 53.1 Authentication

Required tests:

- invalid password;
- unknown account;
- brute-force throttling;
- MFA required where policy says so;
- invalid MFA code;
- expired code;
- reuse of recovery artifact;
- reset artifact replay;
- refresh-token replay;
- revoked session;
- revoked device;
- password change invalidates relevant session state.

## 53.2 Authorization

Required tests:

- same user/scope allowed;
- different user same tenant allowed only when permission permits;
- different tenant denied;
- different branch denied;
- unauthorized field denied;
- unauthorized action denied;
- state-invalid action denied;
- approval-required action denied without approval;
- step-up-required action denied without recent authentication.

## 53.3 Tenant isolation

Every major resource type must have at least one cross-tenant negative test.

This includes:

- users;
- products;
- suppliers;
- inventory;
- sales;
- payments;
- reports;
- exports;
- files;
- devices;
- audit data.

## 53.4 Payment

Tests must include deliberate manipulation of:

- amount;
- currency;
- sale ID;
- provider reference;
- status;
- payment completion flag.

Frontend tampering must not create a false successful payment.

## 53.5 Webhook

Tests:

- wrong signature;
- missing signature;
- expired timestamp where applicable;
- duplicate event ID;
- duplicate transaction ID;
- out-of-order event;
- impossible state transition;
- malformed payload;
- oversized payload.

## 53.6 Offline

Tests:

- duplicated command;
- old command;
- command from revoked device;
- wrong tenant command;
- modified payload;
- invalid sequence/checkpoint;
- huge batch;
- malicious ordering;
- restart after partial sync;
- retry after timeout;
- client clock manipulation.

## 53.7 Concurrency

Tests:

- two final-stock sales;
- concurrent refund attempts;
- duplicate webhook processing;
- concurrent stock adjustment;
- role revocation during request;
- device revocation during sync.

---

# 54. Fuzzing Strategy

High-value fuzz targets:

```text
JSON request DTOs
sync envelopes
CSV import parser
file metadata
webhook payloads
provider response parsers
ID parsers
filter/sort parsers
```

Fuzz properties:

- parser termination;
- no panic;
- no unsafe memory behavior;
- bounded resource use;
- invalid input is rejected;
- valid input retains domain invariants.

A fuzz regression is release-blocking when it reintroduces a known security or correctness defect.

---

# 55. Concurrency Test Strategy

The harness should expose deterministic race scenarios.

Example:

```text
STOCK = 1

Worker A -> SELL 1
Worker B -> SELL 1

Expected:
exactly one succeeds
exactly one fails safely
stock never becomes negative
no partial sale is committed
```

Payment example:

```text
WEBHOOK A -> SUCCESS
WEBHOOK A -> SUCCESS
WEBHOOK B -> REFUND
```

The test must prove that duplicate and out-of-order events do not corrupt the internal state machine.

---

# 56. Security Definition of Done — Implementation Form

A feature is **not done** because the code compiles.

A security-sensitive feature is done only when:

```text
[ ] threat identified
[ ] trust boundary identified
[ ] principal identified
[ ] tenant scope defined
[ ] permission defined
[ ] object scope defined
[ ] property scope defined
[ ] state transitions defined
[ ] input schema bounded
[ ] sensitive fields classified
[ ] secret/logging policy defined
[ ] timeout defined
[ ] resource budget defined
[ ] idempotency defined where required
[ ] transaction boundary defined
[ ] audit event defined
[ ] telemetry defined
[ ] negative tests implemented
[ ] positive tests implemented
[ ] concurrency behavior tested where relevant
[ ] CI gate exists
[ ] rollback behavior exists
[ ] runbook exists for material failure
[ ] owner identified
```

A pull request that adds a privileged endpoint without the corresponding negative authorization tests is incomplete.

---

# 57. API Security Review Template

Every endpoint PR should answer the following directly in code review.

```text
ROUTE:
METHOD:
SECURITY CLASS:

AUTHENTICATION:
- Required?
- Authentication strength?

AUTHORIZATION:
- Permission?
- Scope?
- Object authorization?
- Property authorization?
- Approval?
- Step-up?

TENANT:
- How is tenant derived?
- What prevents caller-controlled tenant substitution?

INPUT:
- Schema?
- Size limits?
- Unknown-field behavior?

STATE:
- Allowed current states?
- Allowed transitions?

TRANSACTION:
- DB transaction boundary?
- Locks?
- Isolation?

IDEMPOTENCY:
- Required?
- Key?
- Replay behavior?

SIDE EFFECTS:
- Audit?
- Outbox?
- External provider?

RESILIENCE:
- Timeout?
- Retry?
- Resource limits?

SECURITY TESTS:
- Positive tests?
- Negative tests?
- Cross-tenant test?
- Abuse/race test?
```

This template should eventually become part of the pull-request workflow for high-risk domain changes.

---

# 58. Database Change Review Template

Every migration must identify:

```text
TABLES AFFECTED
CONSTRAINTS ADDED/REMOVED
INDEXES ADDED/REMOVED
RLS IMPACT
DATA CLASSIFICATION IMPACT
TENANT ISOLATION IMPACT
MIGRATION LOCK DURATION
ROLLBACK / FORWARD FIX PLAN
BACKWARD COMPATIBILITY
PERFORMANCE IMPACT
SECURITY TESTS
```

A schema change touching security-sensitive tables requires security-aware review.

---

# 59. Dependency Review Template

Every new dependency should record:

```text
NAME
VERSION
DIRECT / TRANSITIVE
PURPOSE
OWNER CRATE
LICENSE
MAINTENANCE STATUS
SECURITY HISTORY
ATTACK-SURFACE IMPACT
ALTERNATIVES CONSIDERED
REMOVAL PLAN IF TEMPORARY
```

Avoid adding dependencies merely because an implementation can be shortened.

---

# 60. Repository Security Baseline

At repository initialization, include:

```text
.gitignore
.env.example
CODEOWNERS
SECURITY.md
CONTRIBUTING.md
```

No `.env` with real credentials.

No secrets in test fixtures.

No production data in the development repository.

No public source maps by accident.

No debug credentials.

No hardcoded default administrator password.

---

# 61. Local Development Security Contract

Local development must remain easy without becoming unsafe.

## 61.1 Required local services

Minimum likely stack:

```text
Rust API
PostgreSQL
optional Redis
local object store or test adapter
mock/external provider stubs
observability collector where practical
```

## 61.2 Local data

Use synthetic seed data.

Never copy production customer data into local development merely for convenience.

## 61.3 Local identity

Developer identities must be distinct from production operator identities.

---

# 62. Test Environment Security

CI/test environment:

- synthetic data only;
- isolated credentials;
- no production signing keys;
- no production database;
- no unrestricted cloud access;
- dedicated test infrastructure for destructive security testing.

Security tests must be safe to run repeatedly.

---

# 63. Staging Environment Security

Staging should resemble production security posture closely enough that security tests have real value.

Use:

- production-like authentication configuration;
- production-like network restrictions;
- real TLS;
- realistic rate limits;
- protected admin routes;
- non-production secrets;
- synthetic/sanitized data.

Staging is where DAST and post-deployment security smoke tests run.

---

# 64. Production Deployment Contract

Production deployment requires:

```text
protected source
   |
   v
locked dependencies
   |
   v
build
   |
   v
security tests
   |
   v
SBOM / provenance
   |
   v
artifact verification
   |
   v
protected environment
   |
   v
deployment
   |
   v
post-deploy security smoke
```

No direct developer laptop deployment path should bypass this for normal releases.

Emergency break-glass deployment is separately controlled and audited.

---

# 65. Health and Readiness Endpoints

Health endpoints must not expose:

- credentials;
- environment variables;
- raw dependency errors;
- stack traces;
- internal topology;
- sensitive configuration.

Differentiate:

```text
LIVENESS
READINESS
DEEP DIAGNOSTIC
```

Deep diagnostics should not be public.

---

# 66. API Inventory Enforcement

The build/release process should compare:

```text
Declared API contract
      vs
Runtime route inventory
```

Purpose:

- detect undocumented endpoints;
- detect accidental exposure;
- detect deprecated route drift;
- detect unexpected admin/debug endpoints.

Unknown production routes are deployment defects.

---

# 67. Security Event Taxonomy

Security events should use stable machine-readable names.

Suggested categories:

```text
AUTH.*
SESSION.*
MFA.*
DEVICE.*
AUTHZ.*
TENANT.*
DATA.*
PAYMENT.*
WEBHOOK.*
SYNC.*
ADMIN.*
SUPPORT.*
CI.*
SECRET.*
CLOUD.*
```

This improves alerting, testing and incident response.

---

# 68. Monitoring Baseline

Monitor for:

```text
BOLA denial spikes
login failure spikes
MFA failures
recovery failures
device churn
role changes followed by exports
large export volume
payment mismatch spikes
webhook signature failures
webhook duplicate spikes
revoked-device sync attempts
queue growth
retry storms
DB connection exhaustion
slow queries
public exposure drift
secret-scan findings
CI permission changes
```

Alerts must be bounded and actionable.

---

# 69. Incident-Ready Implementation Hooks

The application must expose enough controlled capabilities to support:

- session revocation;
- device revocation;
- secret rotation;
- key rotation;
- payment integration disablement;
- queue quarantine;
- maintenance mode;
- safe read-only mode where applicable;
- deployment rollback;
- evidence preservation.

Incident response is a runtime capability, not merely a PDF or Markdown document.

---

# 70. Backup and Disaster Recovery Implementation Contract

Production backup requires:

- encryption;
- separate backup identity;
- monitoring;
- retention;
- point-in-time recovery where supported;
- immutable/deletion-protected copy where practical;
- regular restore test;
- infrastructure rebuild path;
- secret recovery procedure;
- explicit RPO/RTO.

## 70.1 Restore security test

After restoration, verify:

- authorization still works;
- tenant isolation remains intact;
- audit data is present;
- configuration is correct;
- secrets are restored through intended mechanisms;
- external integration state is reconciled;
- revoked devices remain revoked according to intended continuity semantics.

---

# 71. Mandatory Release Gate Model

A release candidate must pass all applicable gates.

```text
SOURCE
 |
 +--> format
 +--> lint
 +--> compile
 +--> tests
 |
 v
SECURITY
 |
 +--> secret scan
 +--> dependency scan
 +--> SAST
 +--> auth tests
 +--> authz tests
 +--> tenant tests
 +--> injection tests
 +--> API negative tests
 +--> concurrency tests
 |
 v
BUILD / SUPPLY CHAIN
 |
 +--> lock verification
 +--> SBOM
 +--> artifact integrity
 +--> provenance
 |
 v
STAGING
 |
 +--> DAST
 +--> security smoke
 +--> operational smoke
 |
 v
RELEASE
```

Failure at any blocking stage means:

```text
NO RELEASE
```

---

# 72. Fail-Open Detection Tests

The security harness must deliberately break security controls and confirm that the system denies access.

Examples:

### AuthZ service unavailable

Expected:

```text
protected mutation -> DENY
```

### Secret scanner unavailable

Expected:

```text
CI -> FAIL
```

### Dependency advisory service unavailable

Policy-dependent, but the workflow must not falsely report “clean” without evidence.

### Invalid signing key

Expected:

```text
verification -> FAIL
```

### Tenant context absent

Expected:

```text
tenant-owned operation -> DENY
```

---

# 73. Threat Model Coverage Matrix

Implementation must maintain coverage across:

| Threat | Primary control | Test |
|---|---|---|
| Credential stuffing | Rate limit + MFA | Auth flood |
| Account takeover | MFA + recovery controls | Recovery/auth tests |
| BOLA | Scope authorization | Cross-tenant/object swap |
| Privilege escalation | RBAC/scoped permissions | Function/role tests |
| Device theft | Device revocation + secure storage | Revocation tests |
| Webhook forgery | Signature verification | Invalid-signature tests |
| Webhook replay | Event uniqueness | Replay tests |
| Payment tampering | Server authority | Client-field mutation |
| SQL injection | Parameterization | Injection suite |
| XSS | Encoding + CSP | Payload suite |
| CSRF | Tokens/origin checks | Browser suite |
| SSRF | Allowlist + egress control | URL bypass suite |
| Path traversal | Object IDs + normalization | Traversal suite |
| Malicious file | Quarantine + scanning | File suite |
| Expensive request | Limits + timeout | Resource tests |
| Race conditions | DB atomicity | Concurrency suite |
| Supply-chain compromise | Pinning + provenance | CI policy |
| Cloud exposure | Private networking + policy | IaC scans |
| Insider abuse | SoD + JIT + audit | Admin tests |
| Offline replay | Command identity + age | Sync suite |

---

# 74. Architecture Invariants That Code Must Never Violate

The following are hard invariants.

## I1 — Client is not authority

No frontend boolean or local DB row can settle server-side financial truth.

## I2 — Tenant ID is not proof

Client-supplied tenant IDs are selectors, not authority.

## I3 — Final financial records are not silently mutated

Use compensating records.

## I4 — Cache is not truth

A cache outage cannot invent business state.

## I5 — External systems are untrusted

Provider callbacks require verification.

## I6 — Offline storage is not server authority

Local records must be reconciled.

## I7 — Authorization is server-side

Hidden UI is never sufficient.

## I8 — CI is production infrastructure

Build pipelines receive the same security scrutiny as application code.

## I9 — Security failures fail closed

Missing evidence is not a pass.

## I10 — Derived data is rebuildable

Reports/read models/cache can be rebuilt from authoritative data.

---

# 75. Explicitly Rejected Implementation Patterns

The following are prohibited unless a formally approved ADR changes the policy.

## 75.1 Giant handler business logic

Do not put authorization, accounting, inventory, provider integration and database details into one Axum handler.

## 75.2 Generic CRUD for money

Financial workflows require explicit commands and state transitions.

## 75.3 Global mutable tenant state

Never use process-global “current tenant”.

## 75.4 Client-calculated totals as authority

Client totals are presentation/input, not financial truth.

## 75.5 Generic last-write-wins synchronization

Unsafe for inventory and finance.

## 75.6 Blind database retries

Potentially duplicate money-moving operations.

## 75.7 Wildcard CORS with credentials

Prohibited.

## 75.8 `.env` committed to repository

Prohibited.

## 75.9 Secrets in frontend bundles

Prohibited.

## 75.10 Unbounded reports

Prohibited.

## 75.11 Arbitrary outbound HTTP from the API

Prohibited unless explicitly designed and sandboxed.

## 75.12 Generic shell execution from Tauri

Prohibited.

## 75.13 Superuser application DB role

Prohibited.

## 75.14 Security scanners marked “non-blocking” for convenience

Not acceptable for controls defined as blocking.

---

# 76. Open Decisions That Must Not Be Faked

Some decisions remain intentionally evidence-driven.

These include:

- exact identity provider versus self-hosted standards-based identity;
- exact cloud/region;
- exact Redis requirement after load measurement;
- exact reporting architecture after actual workload evidence;
- final payment provider contract capabilities;
- exact MRA production certification pathway;
- exact pharmacy regulatory data requirements;
- dedicated database isolation for specific enterprise contracts.

Until resolved:

```text
DEFAULT = SAFE PLACEHOLDER
```

not:

```text
DEFAULT = INSECURE CONVENIENCE
```

An unresolved decision must have an owner, evidence required, decision date/trigger, and safe interim behavior.

---

# 77. ADR Requirements for Phase 0

At minimum create:

```text
ADR-001  Rust-first backend
ADR-002  Flutter mobile
ADR-003  Tauri desktop
ADR-004  PostgreSQL authority
ADR-005  Modular monolith
ADR-006  Custom offline command protocol
ADR-007  Transactional outbox
ADR-008  Tenant isolation strategy
ADR-009  Authorization model
ADR-010  Authentication/session strategy
ADR-011  Financial append-only model
ADR-012  Inventory ledger model
ADR-013  Payment adapter boundary
ADR-014  MRA EIS adapter boundary
ADR-015  Secret-management strategy
ADR-016  Observability architecture
ADR-017  CI/CD security model
ADR-018  Dependency/lock policy
ADR-019  Backup/DR
ADR-020  API versioning
ADR-021  File/object storage
ADR-022  Support/break-glass access
ADR-023  Feature flags
ADR-024  Deployment and rollback
ADR-025  Multi-country extensibility
```

Each ADR must contain:

```text
Context
Decision
Alternatives
Security implications
Operational implications
Tradeoffs
Reversibility
Evidence
Consequences
```

---

# 78. Implementation Phase Gates

Implementation is organized into gates rather than a continuous stream of feature work.

## Gate 0 — Architecture baseline

Must pass:

```text
[ ] authoritative docs identified
[ ] contradictions resolved
[ ] open decisions catalogued
[ ] ADR structure established
[ ] implementation spec approved
```

## Gate 1 — Repository/build baseline

```text
[ ] workspace builds
[ ] pinned toolchain
[ ] lockfile tracked
[ ] reproducible CI
[ ] security scans run
[ ] no production secrets
```

## Gate 2 — Trust foundation

```text
[ ] authentication
[ ] session lifecycle
[ ] device identity
[ ] tenant context
[ ] authorization engine
[ ] audit
```

## Gate 3 — Data foundation

```text
[ ] PostgreSQL roles
[ ] migrations
[ ] constraints
[ ] RLS where required
[ ] transaction conventions
```

## Gate 4 — Security test foundation

```text
[ ] security fixtures
[ ] auth suite
[ ] tenant suite
[ ] authz suite
[ ] injection suite
[ ] concurrency harness
[ ] CI gates
```

## Gate 5 — Domain implementation

Only after the trust foundation is working should large business domains begin to rely on it.

---

# 79. First Implementation PR Sequence

Before normal feature development, the repository should receive focused PRs in this order:

```text
PR-001  Repository baseline
PR-002  Rust workspace + toolchain
PR-003  Dependency/lock policy
PR-004  Configuration + secrets boundary
PR-005  Error + tracing infrastructure
PR-006  Database connectivity + roles
PR-007  Auth abstraction
PR-008  Session/device model
PR-009  Tenant context
PR-010  Permission model
PR-011  Authorization engine
PR-012  Audit/event infrastructure
PR-013  Security test harness
PR-014  Mandatory CI gates
PR-015  Baseline API skeleton
```

This is intentionally different from building the POS first.

---

# 80. Initial Rust Module Contracts

## `sitolo-domain`

Owns:

- domain objects;
- value objects;
- state machines;
- domain invariants.

Must not know about HTTP.

## `sitolo-auth`

Owns:

- normalized principal representation;
- session abstraction;
- authentication integration boundaries.

## `sitolo-authz`

Owns:

- permission model;
- scope model;
- authorization decisions;
- policy evaluation interfaces.

## `sitolo-tenant`

Owns:

- organization membership;
- branch scope;
- device-to-tenant association.

## `sitolo-db`

Owns:

- repositories;
- transaction helpers;
- migrations interface;
- connection pools;
- SQLx integration.

## `sitolo-security`

Owns:

- security context;
- validation/security middleware helpers;
- redaction;
- security event semantics;
- common security errors.

## `sitolo-api`

Owns:

- Axum routes;
- extraction;
- transport DTOs;
- response mapping.

## `sitolo-events`

Owns:

- event envelopes;
- versioning;
- correlation metadata.

## `sitolo-outbox`

Owns:

- outbox persistence interface;
- dispatch lifecycle;
- retry policy integration.

## `sitolo-workers`

Owns:

- asynchronous execution;
- job-specific adapters;
- bounded concurrency.

---

# 81. Security Context Construction

Security context construction should be centralized enough that endpoints cannot casually bypass it.

Conceptual pipeline:

```text
HTTP Request
    |
    v
Extract Credentials
    |
    v
Verify Identity
    |
    v
Resolve Session
    |
    v
Resolve Device
    |
    v
Resolve Tenant Membership
    |
    v
Resolve Permissions
    |
    v
Build SecurityContext
```

Downstream application services accept the trusted context rather than reparsing credentials.

---

# 82. Authorization Decision Object

Authorization decisions should be explainable internally.

Conceptual:

```rust
pub enum AuthorizationDecision {
    Allow,
    Deny {
        reason: DenialReason,
    },
}
```

Possible reasons:

```text
NOT_AUTHENTICATED
SESSION_REVOKED
TENANT_MISMATCH
BRANCH_OUT_OF_SCOPE
PERMISSION_MISSING
RESOURCE_OUT_OF_SCOPE
PROPERTY_FORBIDDEN
STATE_FORBIDDEN
APPROVAL_REQUIRED
STEP_UP_REQUIRED
DEVICE_REVOKED
POLICY_UNAVAILABLE
```

Client responses should not necessarily expose all internal policy reasoning, but internal audit/debug telemetry should preserve structured reason codes where appropriate.

---

# 83. Property-Level Security Strategy

Instead of accepting database entities directly from JSON, use command-specific request types.

Example:

```text
UpdateBusinessProfileCommand
CreateProductCommand
ChangePriceCommand
RequestRefundCommand
ApproveRefundCommand
AdjustInventoryCommand
ApproveInventoryAdjustmentCommand
```

This naturally limits writable fields.

Avoid:

```text
PATCH /business/{id}
with arbitrary database-column map
```

for privileged data.

---

# 84. State Machine Enforcement

State transitions should be explicit domain operations.

Example:

```rust
sale.finalize(command_context)?;
sale.request_refund(refund_context)?;
sale.reverse(reversal_context)?;
```

Avoid a generic:

```text
UPDATE sale SET status = $status
```

from public application code.

This makes illegal transitions difficult to express.

---

# 85. Approval Model

High-impact workflows can use a general approval abstraction.

Conceptual:

```text
REQUEST
  |
  +--> actor
  +--> resource
  +--> action
  +--> threshold
  +--> reason
  |
  v
PENDING APPROVAL
  |
  v
APPROVER
  |
  v
APPROVED / REJECTED
```

Self-approval must be prevented where separation of duties applies.

Approval objects must be auditable and immutable once used for a high-impact transition.

---

# 86. Audit Integrity

Audit records should be append-oriented.

A security audit trail must not rely on ordinary user-editable domain records remaining untouched.

Sensitive audit access itself should be controlled.

Audit queries must be tenant-scoped unless executed through explicitly authorized platform operations.

---

# 87. Privacy-Aware Logging

Do not solve security logging by logging everything.

The logging contract should explicitly define:

```text
WHAT IS REQUIRED
WHAT IS SENSITIVE
WHAT IS HASHED
WHAT IS REDACTED
WHAT IS RETAINED
WHO CAN READ IT
```

Use identifiers and reason codes instead of copying entire user payloads into logs.

---

# 88. Security Headers Baseline

For web/admin, apply appropriate:

- HSTS;
- Content Security Policy;
- X-Content-Type-Options;
- restrictive framing policy;
- Referrer-Policy;
- controlled cache directives.

Exact directives should be validated against actual frontend requirements rather than blindly copied from templates.

---

# 89. HTTP Security Defaults

Axum service defaults should establish:

- request body limit;
- header limit;
- request deadline;
- graceful cancellation;
- structured tracing;
- secure error mapping;
- explicit CORS policy;
- authentication extraction;
- correlation identifiers.

Sensitive routes may add stricter limits.

---

# 90. Database Connection Pool Security

The PostgreSQL pool must have:

- bounded maximum connections;
- bounded acquisition timeout;
- connection validation where appropriate;
- TLS according to environment/security requirements;
- credentials loaded through approved secret/configuration path.

An overloaded pool should produce controlled degradation, not unbounded task accumulation.

---

# 91. Query Performance Security

Any query that can become expensive due to user-controlled inputs must have:

- bounded page size;
- bounded date range;
- indexed access path;
- maximum filter complexity;
- statement timeout;
- asynchronous path for large work where needed.

Report generation should not share the same unrestricted path as latency-sensitive POS operations.

---

# 92. POS Security Performance Rule

POS operations should remain narrow and predictable.

A typical sale path should not synchronously perform:

- multi-million-row report aggregation;
- arbitrary external API fan-out;
- expensive full-text search;
- file processing;
- analytics reconstruction.

The POS transaction path should protect merchant continuity.

---

# 93. Offline Device Data Minimization

Do not replicate platform-wide data onto every device.

A cashier tablet for Branch A does not require:

- all branches;
- all employees;
- all audit history;
- all organization configuration;
- all customers;

unless the workflow explicitly requires it.

Minimization reduces:

- local compromise impact;
- sync traffic;
- storage;
- privacy risk;
- attack surface.

---

# 94. Data Classification Implementation Matrix

At implementation time, classify sensitive data as follows.

| Data | Classification | Client allowed? | Log allowed? |
|---|---|---:|---:|
| Product name | Internal/business | Yes | Usually yes |
| Product cost | Confidential | Scoped | Carefully |
| Customer PII | Sensitive | Scoped | Minimize |
| Password hash | Highly sensitive | No | No |
| Access token | Secret | No | No |
| Refresh token | Secret | Secure local storage only | No |
| Payment secret | Secret | No | No |
| MRA credentials | Secret | No | No |
| Signing key | Secret | No | No |
| Audit event ID | Internal | Scoped | Yes |
| Provider transaction reference | Confidential | Scoped | Minimize |
| Device ID | Internal | Scoped | Yes where needed |

Exact legal classification can be refined through the privacy/security process.

---

# 95. API Response Minimization

Do not serialize whole domain/database structures to clients merely because they exist.

Responses should use purpose-built projections.

Benefits:

- reduced data leakage;
- smaller payloads;
- stronger property control;
- easier versioning;
- lower client coupling.

---

# 96. Data Export Isolation

Export worker credentials should be able to read only the data required for the export process.

A report export job should not automatically acquire database administrator privilege.

Generated files must inherit tenant scope and expiry metadata.

---

# 97. Object Storage Security

Object keys should be generated server-side.

Prefer:

```text
objects/{tenant_id}/{object_id}
```

with authorization enforced separately.

Do not treat possession of a predictable object path as authorization.

Signed URLs should be:

- short-lived;
- scoped;
- purpose-specific;
- revocable indirectly through object permission where feasible.

---

# 98. File/Export Metadata

Each stored artifact should be associated with:

```text
object_id
tenant_id
owner/resource_id
content_type
size
created_at
retention_policy
classification
status
```

This makes object storage part of the business security model rather than an anonymous blob bucket.

---

# 99. CI Secret Boundary

A pull request workflow must not receive:

- production DB passwords;
- production signing keys;
- unrestricted cloud credentials;
- payment provider production secrets;
- EIS production credentials.

Test secrets should be disposable and environment-specific.

---

# 100. Git Security Controls

Repository policy should include:

- protected default branch;
- required reviews;
- CODEOWNERS on sensitive paths;
- secret scanning;
- dependency updates through controlled PRs;
- no direct production credential commits;
- release tags under protected workflow;
- review of workflow-file changes.

Workflow changes are particularly sensitive because they can change the deployment trust plane.

---

# 101. CODEOWNERS Strategy

Sensitive files should have explicit ownership.

Examples:

```text
/.github/                  Platform/security owner
/infra/                    Platform/SRE
/migrations/               DB owner
/crates/sitolo-authz/      Security/domain owner
/crates/sitolo-payment/    Integrations owner
/crates/sitolo-domain/     Domain owner
/tests/security/           Security owner
```

The final names should reflect the actual team structure.

---

# 102. Security Review Triggers

Automatic security review should be triggered by changes touching:

- authentication;
- authorization;
- tenant isolation;
- roles/permissions;
- payment providers;
- MRA integration;
- offline sync;
- financial invariants;
- inventory concurrency;
- secrets;
- CI workflows;
- infrastructure/IAM;
- public endpoints;
- export logic;
- browser storage;
- cryptographic functionality.

---

# 103. Architecture Review Triggers

Architecture must be reconsidered when:

- adding a new major domain;
- entering a new country;
- adding a new payment rail;
- integrating a new tax protocol;
- materially changing scale assumptions;
- adding a service boundary;
- introducing external messaging infrastructure;
- changing the offline protocol.

Do not add Kafka, service mesh, or microservices simply because they are fashionable.

---

# 104. Build-vs-Buy Decision Rule

For any proposed security component, ask:

```text
Is this a commodity solved by mature infrastructure?
      |
     YES -> reuse/adopt
      |
     NO
      |
      v
Is this part of Sitolo's differentiated business/security semantics?
      |
     YES -> build carefully
      |
     NO
      |
      v
Seek managed/mature alternative
```

This prevents Sitolo from spending engineering time building cryptography, identity protocols or distributed infrastructure that does not form its competitive moat.

---

# 105. Documentation Synchronization Rule

When implementation changes a security invariant:

```text
CODE CHANGE
   -> TEST CHANGE
   -> IMPLEMENTATION SPEC CHANGE
   -> SECURITY ARCHITECTURE REVIEW
   -> ADR WHEN REQUIRED
```

Documentation is versioned with the code.

Stale security documentation is a defect because it causes future developers to implement against false assumptions.

---

# 106. Evidence Standard

For each blocking security control, retain evidence such as:

- CI run ID;
- test artifact;
- scanner report;
- route inventory;
- database permission inspection;
- IaC scan;
- staging DAST report;
- restore exercise record;
- penetration-test result;
- signed artifact/provenance metadata.

A green dashboard without underlying evidence is insufficient for critical controls.

---

# 107. Security Release Evidence Package

Each production release should produce:

```text
release metadata
source commit
Rust toolchain version
lockfile digest
frontend lockfile digest
SBOM
artifact digest
provenance/attestation
security scan results
test results
DAST result where applicable
migration result
deployment result
post-deploy smoke result
```

---

# 108. Incident Regression Rule

Every confirmed security incident must produce at least:

```text
root cause
containment
remediation
regression test
monitoring improvement
runbook update
architecture/documentation update when needed
```

A vulnerability that is fixed only in code but has no regression test is likely to reappear.

---

# 109. Security Metrics

Useful engineering metrics include:

- percentage of endpoints with explicit authorization metadata;
- percentage of privileged endpoints with negative tests;
- tenant-isolation test coverage;
- secret-scanning failures;
- dependency vulnerabilities by severity;
- mean time to remediate critical findings;
- failed deployment gate count;
- webhook replay rejection rate;
- revoked-device sync attempts;
- export anomalies;
- audit event ingestion health;
- restore drill success rate.

Avoid vanity metrics such as “number of security tests” without connecting them to meaningful coverage.

---

# 110. Definition of Ready for Domain Implementation

A domain module is ready to code when:

```text
[ ] business purpose defined
[ ] entities defined
[ ] value objects defined
[ ] state machine defined
[ ] invariants defined
[ ] tenant scope defined
[ ] role/permission matrix defined
[ ] API commands defined
[ ] DB schema defined
[ ] transaction boundaries defined
[ ] concurrency behavior defined
[ ] offline behavior defined where relevant
[ ] external dependencies defined
[ ] audit events defined
[ ] failure states defined
[ ] timeouts/limits defined
[ ] security tests specified
```

---

# 111. Definition of Ready for API Endpoints

An endpoint is ready when:

```text
[ ] route classified
[ ] authentication defined
[ ] authorization defined
[ ] scope defined
[ ] schema defined
[ ] limits defined
[ ] idempotency defined
[ ] transaction defined
[ ] audit defined
[ ] error contract defined
[ ] timeout defined
[ ] rate limit defined
[ ] positive tests defined
[ ] negative tests defined
```

---

# 112. Definition of Ready for Database Tables

A table is ready when:

```text
[ ] owner defined
[ ] tenant scope defined
[ ] classification defined
[ ] primary key defined
[ ] FK strategy defined
[ ] unique constraints defined
[ ] check constraints defined
[ ] indexes defined
[ ] RLS decision defined
[ ] runtime role access defined
[ ] migration path defined
[ ] deletion/retention defined
[ ] audit implications defined
```

---

# 113. Definition of Ready for External Integrations

An integration is ready when:

```text
[ ] provider contract verified
[ ] credentials strategy defined
[ ] authentication defined
[ ] signature/integrity defined
[ ] timeout defined
[ ] retry defined
[ ] idempotency defined
[ ] reconciliation defined
[ ] failure states defined
[ ] audit defined
[ ] sandbox/test strategy defined
[ ] incident/disable procedure defined
```

---

# 114. Definition of Done for Phase 0

Phase 0 completes only when:

```text
[ ] Product scope frozen enough for implementation
[ ] Business invariants documented
[ ] Domain boundaries documented
[ ] Security invariants documented
[ ] Repository structure approved
[ ] Rust toolchain pinned
[ ] Dependency policy defined
[ ] Database role strategy defined
[ ] Auth contract defined
[ ] AuthZ contract defined
[ ] Tenant isolation contract defined
[ ] Offline security constraints defined
[ ] Payment boundary defined
[ ] EIS boundary defined
[ ] API security contract defined
[ ] Error contract defined
[ ] Observability contract defined
[ ] CI security model defined
[ ] Mandatory security test strategy defined
[ ] Release gates defined
[ ] ADR backlog created
[ ] Open decisions catalogued
```

---

# 115. Implementation Sequence After This Document

Once this document is accepted, implementation follows:

```text
PHASE 0
Architecture / Contracts / ADR Freeze
          |
          v
PHASE 1
Repository + Rust Workspace + CI
          |
          v
PHASE 2
Config + Secrets + Logging + Errors + Telemetry
          |
          v
PHASE 3
Identity + Sessions + MFA + Device Identity
          |
          v
PHASE 4
Tenant + Organization + Branch + IAM
          |
          v
PHASE 5
PostgreSQL Schema + Migrations + Constraints + RLS
          |
          v
PHASE 6
Authorization Engine / Policy Enforcement
          |
          v
PHASE 7
Security Test Framework
          |
          v
PHASE 8
Product / Catalogue
          |
          v
PHASE 9
Inventory Ledger
          |
          v
PHASE 10
POS / Sales
          |
          v
PHASE 11
Payments + Reconciliation
          |
          v
PHASE 12
Offline Sync
          |
          v
PHASE 13
Procurement / Suppliers
          |
          v
PHASE 14
Returns / Refunds / Cash
          |
          v
PHASE 15
MRA EIS
          |
          v
PHASE 16
Reporting / Exports
          |
          v
PHASE 17
Billing / Entitlements
          |
          v
PHASE 18
Admin / Support
          |
          v
PHASE 19
Hardening / Performance / DR
          |
          v
PHASE 20
Production Certification
```

The sequence exists to prevent building high-value domain features on top of unstable trust boundaries.

---

# 116. Phase 1 Entry Criteria

Do not begin Phase 1 merely because “the repository exists”.

Required:

- reproducible Rust build;
- pinned toolchain;
- lockfile policy;
- CI executing against clean checkout;
- basic security scanning;
- branch protection;
- no secret leakage;
- repository ownership;
- deterministic local setup documentation.

---

# 117. Phase 2 Entry Criteria

Before identity code depends on the platform foundation:

- typed configuration;
- startup validation;
- structured tracing;
- secret redaction;
- error boundary;
- request correlation;
- timeout infrastructure;
- environment separation.

---

# 118. Phase 3 Entry Criteria

Before merchant identity:

- auth integration decision recorded;
- token/session lifecycle defined;
- device model defined;
- MFA policy defined;
- recovery policy defined;
- auth negative tests present;
- session revocation tested.

---

# 119. Phase 4 Entry Criteria

Before business resources:

- organization hierarchy implemented;
- membership model implemented;
- role/permission registry implemented;
- branch scope implemented;
- device association implemented;
- cross-tenant negative tests passing.

---

# 120. Phase 5 Entry Criteria

Before major data domains:

- migrations controlled;
- runtime DB role least privilege;
- constraints implemented;
- RLS strategy tested;
- transaction conventions enforced;
- DB exposure verified private.

---

# 121. Phase 6 Entry Criteria

Before broad API exposure:

- every protected command has explicit permission;
- object authorization is integrated;
- property authorization strategy exists;
- state transitions are server-enforced;
- missing policy evidence denies;
- authorization negative tests pass.

---

# 122. Phase 7 Entry Criteria

Before deep domain expansion:

- security test fixtures exist;
- test database automation works;
- cross-tenant tests run automatically;
- race tests exist for first concurrency-sensitive workflows;
- CI gates block bypass;
- security scan failures fail release as designed.

---

# 123. Product Domain Implementation Rule

Every new domain should answer:

```text
What value does this create?
What entity owns the data?
Who can perform the action?
At what scope?
What can go wrong?
What economic invariant applies?
What state transitions exist?
What happens offline?
What happens twice?
What happens concurrently?
What happens when a provider fails?
What must be audited?
What must be tested negatively?
```

If those questions cannot be answered, the feature is not implementation-ready.

---

# 124. Business Operating System Security Model

Sitolo's long-term advantage should emerge from the combination:

```text
Correct Domain Model
        +
Offline Continuity
        +
Tenant Isolation
        +
Financial Integrity
        +
Inventory Correctness
        +
Payment Reconciliation
        +
Operational Auditability
        +
Security Automation
        =
TRUSTWORTHY SME OPERATING SYSTEM
```

Security is therefore not a separate wrapper around the product.

It is part of the product's correctness.

---

# 125. Malawi-First, Africa-Ready Implementation Rule

The implementation should support future African expansion without embedding assumptions everywhere.

Country-specific concerns should sit behind explicit abstractions for:

- currency;
- tax;
- payment providers;
- tax authority adapters;
- phone/contact formats;
- numbering;
- localization;
- data residency;
- regulatory workflows.

However, the system should not implement multiple-country infrastructure merely to appear “African-ready”.

The rule is:

```text
ABSTRACT WHERE CHANGE IS EXPECTED
SPECIALIZE WHERE CHANGE IS NOT EXPECTED
MEASURE BEFORE GENERALIZING
```

---

# 126. What Must Not Be Prematurely Added

Do not introduce merely for architectural fashion:

- Kubernetes before operational evidence requires it;
- service mesh before service boundaries justify it;
- Kafka before outbox/workers are insufficient;
- distributed caches as authoritative state;
- generic multi-region writes before business requirements require them;
- per-tenant databases by default;
- arbitrary microservices;
- custom cryptography;
- custom OAuth protocol;
- generalized rules engines for simple invariants.

Security architecture and operational simplicity reinforce each other when the system keeps authoritative state close to the business transaction.

---

# 127. High-Risk Areas Requiring the Most Engineering Attention

The implementation effort should prioritize:

1. tenant isolation;
2. authentication/session security;
3. authorization;
4. financial integrity;
5. inventory concurrency;
6. offline synchronization;
7. payment/webhook trust;
8. CI/CD identity;
9. secret management;
10. database privileges;
11. exports and support access;
12. observability/redaction.

These areas have a much larger consequence than superficial UI hardening.

---

# 128. Security Review Questions Before Merge

Before merging sensitive work, reviewers should ask:

```text
Can another tenant reach this?
Can another branch reach this?
Can a direct HTTP caller bypass the UI?
Can this operation be replayed?
Can it happen twice concurrently?
Can it be partially committed?
Can a retry duplicate a financial effect?
Can a revoked device perform it?
Can an admin misuse it without evidence?
Can it leak through logs?
Can it leak through exports?
Can the client alter its economic result?
Can an external provider forge its input?
Can the request consume unbounded resources?
Does CI actually test the intended security property?
```

If any answer is unclear, the feature is not security-complete.

---

# 129. Security Gate Matrix by Phase

| Phase | Security gate |
|---|---|
| 0 | Architecture/ADR freeze |
| 1 | Reproducible build + CI controls |
| 2 | Secrets/logging/error/timeouts |
| 3 | Auth/session/MFA/device tests |
| 4 | Tenant/authz negative suite |
| 5 | DB role/RLS/transaction suite |
| 6 | Function/object/property auth suite |
| 7 | Mandatory security harness operational |
| 8 | Catalogue scope and input security |
| 9 | Inventory race/invariant suite |
| 10 | Sale atomicity/security suite |
| 11 | Payment/webhook/replay suite |
| 12 | Offline tamper/replay/recovery suite |
| 13 | Procurement authorization suite |
| 14 | Refund/cash business-abuse suite |
| 15 | EIS trust/failure/reconciliation suite |
| 16 | Export/PII/exfiltration suite |
| 17 | Billing/entitlement authorization suite |
| 18 | Admin/support/JIT suite |
| 19 | DR/performance/fuzz/DAST hardening |
| 20 | Full certification evidence package |

---

# 130. Production Certification Minimum

Before production certification, the system must demonstrate:

```text
AUTHENTICATION
  valid + invalid + recovery + MFA

AUTHORIZATION
  function + object + property + state

TENANT
  automated cross-tenant denial

DATABASE
  least privilege + RLS where required + constraints

FINANCE
  immutable correction + atomicity

INVENTORY
  concurrency + reconciliation

PAYMENTS
  signature + idempotency + replay defense

OFFLINE
  restart + tamper + revoke + conflict handling

API
  limits + validation + timeout + CORS/CSRF where applicable

FILES
  malware/traversal/resource controls

CI/CD
  protected identity + pinned deps + fail-closed gates

SUPPLY CHAIN
  SCA + SBOM + provenance + artifact integrity

CLOUD
  no public DB/storage/admin exposure

OBSERVABILITY
  redaction + useful security alerts

DR
  restore tested

INCIDENT RESPONSE
  revocation + rotation + rollback rehearsed
```

---

# 131. Standards and External References

This implementation specification is aligned with the following authoritative or widely adopted security references:

- OWASP Application Security Verification Standard (ASVS) 5.0.0 — used as the web/application verification baseline.
- OWASP API Security Top 10 — used for API-specific risk modeling, especially broken object-level authorization, broken authentication, resource consumption and sensitive business flow abuse.
- NIST SP 800-207 — Zero Trust Architecture.
- NIST SP 800-218 — Secure Software Development Framework (SSDF) 1.1.
- RFC 9700 — OAuth 2.0 Security Best Current Practice.
- OWASP Session Management Cheat Sheet.
- OWASP Secrets Management Cheat Sheet.
- OWASP File Upload Cheat Sheet.
- OWASP Logging Cheat Sheet.
- PostgreSQL documentation on Row-Level Security and roles.
- GitHub Actions security and OIDC guidance.
- Rust/Cargo official documentation for toolchain, workspace and locked dependency behavior.

The external standards are reference baselines. Sitolo adds business-specific security requirements for offline operation, tenant isolation, inventory, finance, payments and reconciliation.

---

# 132. Current Research Notes — 4 September 2026

## Rust toolchain

The official Rust release feed lists:

- Rust 1.98.0 — 20 August 2026
- Rust 1.98.1 — 3 September 2026

Therefore this document uses Rust 1.98.1 as the current stable baseline for the preparation date, subject to normal CI verification and future upgrade PRs.

## Cargo reproducibility

Cargo documents that:

- a workspace can share dependency resolution and a common lockfile;
- `Cargo.lock` captures exact dependency versions;
- `--locked` refuses to change the resolved dependency set.

Those properties directly support Sitolo's reproducible-build requirement.

## Application security verification

OWASP describes ASVS as a basis for testing application security controls and provides the current stable 5.0.0 release as a structured verification standard.

## Secure development process

NIST SP 800-218 defines SSDF practices intended to integrate secure-development controls into the SDLC rather than treating security as an isolated final-stage review.

---

# 133. Traceability to Existing Sitolo Architecture

This implementation specification intentionally preserves existing architecture decisions.

| Existing decision | Implementation consequence |
|---|---|
| Rust-first backend | Domain/application/security core implemented in Rust |
| Axum + Tokio | HTTP and async runtime foundation |
| PostgreSQL authority | Financial/inventory/tenant truth remains relational and transactional |
| SQLite offline store | Mobile/desktop local continuity with bounded authority |
| Modular monolith | Strong internal module boundaries before service extraction |
| Transactional outbox | Reliable asynchronous side effects without premature broker dependency |
| Custom sync protocol | Domain-specific offline convergence and replay handling |
| Append-only financial history | Compensating corrections rather than silent edits |
| Multi-tenant model | Scope derived from authenticated membership |
| External adapters | Providers isolated behind explicit trust boundaries |
| Mandatory security tests | CI-enforced negative and positive verification |
| Observability by design | Logs, metrics, traces and audit integrated during implementation |

---

# 134. Traceability to the 20 Implementation Phases

The document set requested for Sitolo is intentionally sequenced around dependency order.

### Phase 0

This document plus ADRs establish the implementation contract.

### Phase 1

`repository + rust workspace + CI` implements the reproducible and trusted build foundation.

### Phase 2

`config + secrets + logging + errors + telemetry` implements the operational security plane.

### Phase 3

`identity + sessions + MFA + device identity` implements the principal/security session plane.

### Phase 4

`tenant + organization + branch + IAM` implements the authority boundary.

### Phase 5

`PostgreSQL schema + migrations + constraints + RLS` implements durable business truth.

### Phase 6

`authorization engine / policy enforcement` makes security decisions reusable and testable.

### Phase 7

`security test framework` turns security requirements into release blockers.

### Phase 8–18

Business domains are implemented on top of the trust foundations.

### Phase 19

System-wide hardening, performance, resilience and DR.

### Phase 20

Evidence-driven production certification.

---

# 135. Why This Is the Correct First MD File

This file comes first because implementation without a security implementation contract creates predictable problems:

```text
AUTHENTICATION gets built one way
AUTHORIZATION gets built another way
TENANT SCOPE gets copied manually
PAYMENT SECURITY becomes endpoint-specific
OFFLINE SECURITY becomes client-specific
AUDIT gets bolted on later
CI scans become optional
```

The result is not one security system.

It is a collection of exceptions.

Sitolo instead needs:

```text
ONE TRUST MODEL
ONE AUTHORITY MODEL
ONE TENANT MODEL
ONE AUTHORIZATION MODEL
ONE ERROR MODEL
ONE OBSERVABILITY MODEL
ONE SECURITY TEST MODEL
ONE RELEASE GATE MODEL
```

The architecture already establishes the conceptual model; this document converts that model into implementation constraints.

---

# 136. Final Engineering Contract

The following statement is binding on implementation unless explicitly changed through the architecture/ADR process:

> **Every externally reachable operation is treated as hostile until authenticated, scoped, authorized, validated and checked against business state. Every tenant-owned operation is executed inside an explicit tenant context derived from trusted membership. Every sensitive mutation is protected by domain invariants, transaction semantics and idempotency where applicable. PostgreSQL is authoritative for server-side business truth. Clients preserve operational continuity but do not determine truth. Offline capability is bounded authority, not permanent authority. External providers are explicit trust boundaries. Secrets never enter source, clients, logs or public artifacts. Administrative access is stronger than ordinary merchant access. CI/CD is treated as production infrastructure. Security checks fail closed. Security tests are mandatory and release-blocking. Every important security control must have executable evidence.**

The intended result is not “perfectly secure software”. That standard is unattainable.

The intended result is a system in which:

```text
ATTACK
  -> encounters explicit boundary
  -> encounters authentication
  -> encounters authorization
  -> encounters scope
  -> encounters invariant
  -> encounters transaction
  -> encounters idempotency
  -> encounters audit
  -> encounters detection
  -> encounters bounded blast radius
```

And when something still fails:

```text
DETECT
  -> CONTAIN
  -> PRESERVE EVIDENCE
  -> RECOVER
  -> VERIFY
  -> ADD REGRESSION TEST
  -> IMPROVE CONTROL
```

That is the implementation posture required for Sitolo to become a serious business operating system rather than a conventional POS application with security features added after the fact.

---

# 137. Phase 0 Exit Checklist

```text
ARCHITECTURE
[ ] system architecture frozen
[ ] security architecture frozen
[ ] business/domain invariants frozen
[ ] open decisions recorded
[ ] ADR process active

IMPLEMENTATION CONTRACT
[ ] this specification accepted
[ ] repository layout accepted
[ ] module boundaries accepted
[ ] API security contract accepted
[ ] database security contract accepted
[ ] offline security contract accepted
[ ] integration security contract accepted

SECURITY
[ ] authentication contract accepted
[ ] authorization model accepted
[ ] tenant model accepted
[ ] secret strategy accepted
[ ] logging/redaction accepted
[ ] CI security model accepted
[ ] security test model accepted

OPERATIONS
[ ] environment strategy accepted
[ ] deployment baseline accepted
[ ] observability baseline accepted
[ ] backup/restore baseline accepted
[ ] incident hooks accepted

RELEASE
[ ] mandatory gates defined
[ ] fail-closed behavior defined
[ ] evidence retention defined
[ ] ownership defined
```

When this checklist is green, Phase 1 can begin.

When it is not green, implementation may still proceed only for explicitly approved foundation work that does not depend on an unresolved decision.

---

# 138. End State

The desired engineering property is:

```text
                      SITOLO
                        |
         +--------------+---------------+
         |                              |
       TRUST                           BUSINESS
         |                              |
   AuthN / AuthZ                    Sales / Stock
   Tenant Isolation                 Money / Payments
   Secrets                          Procurement
   CI/CD                            Tax / Reports
   Device Security                 Offline Operations
         |                              |
         +--------------+---------------+
                        |
                  POSTGRESQL TRUTH
                        |
               AUDIT + OUTBOX + DR
                        |
                 VERIFIED RELEASE
```

And the ultimate operating contract remains:

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

**END OF `security_implementation_spec.md`**
