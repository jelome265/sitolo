# SITOLO — API CONTRACT

**Document:** `api_contract.md`  
**Phase:** Phase 0 — Architecture / Contracts / ADR Freeze  
**Sequence:** File 04 of 16  
**Status:** Implementation-governing API specification  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African expansion  
**Backend:** Rust + Axum + Tokio  
**Persistence:** PostgreSQL authoritative server state; SQLite client continuity  
**Clients:** Flutter Android-first, Tauri desktop, limited TypeScript web/admin  
**Architecture:** Modular monolith first; workers and adapters; selective service extraction only when independently justified

---

# 0. Executive Contract

The Sitolo API is the authoritative machine-facing boundary between untrusted or partially trusted clients/integrations and the Rust application/domain core.

The API exists to expose **business operations**, not to expose the database as a CRUD interface. A route may read or mutate several domain aggregates inside one transaction. Clients express intent; Rust validates, authorizes, executes domain rules, and persists authoritative state in PostgreSQL.

The API contract must preserve the existing Sitolo system contract:

```text
CLIENTS
  |
  | HTTPS / Commands / Queries
  v
EDGE
  |
  v
RUST API
  |
  +--> authentication
  +--> session/device status
  +--> tenant context
  +--> authorization
  +--> validation
  +--> domain/business rules
  +--> transaction orchestration
  +--> PostgreSQL
  +--> audit
  +--> outbox
  |
  v
SAFE RESPONSE
```

The following are non-negotiable:

1. Clients never become business authority merely because they are offline-capable.
2. `tenant_id`, `organization_id`, `branch_id`, permissions, totals, payment status, inventory state and other security-sensitive assertions supplied by clients are never treated as proof.
3. Every protected resource and mutation is authorized server-side.
4. API input is bounded, parsed into typed Rust request models, and validated semantically before domain execution.
5. Financial and inventory mutations execute within explicit transaction boundaries.
6. Finalized financial history is corrected through explicit compensating operations rather than destructive edits.
7. External side effects are never performed by holding an open database transaction across an external network call.
8. Retryable mutations require explicit idempotency semantics even when the underlying HTTP method is `POST`.
9. Expensive operations are resource-bounded and normally asynchronous.
10. API errors are machine-readable, stable, low-information, and correlation-enabled.
11. API documentation and runtime route inventory must agree.
12. Security tests, including negative authorization tests, are release-blocking.
13. Unknown, malformed, stale, replayed or ambiguously authorized requests fail closed.
14. The API is versioned intentionally; database schema details are not a public API contract.

Sitolo's product architecture specifically calls for stable business resources and explicit commands rather than a one-table/one-endpoint CRUD mapping. The system architecture also requires server-side authority, explicit transaction boundaries, offline-safe commands, external adapter boundaries, and reusable security enforcement.

---

# 1. Relationship to Existing Sitolo Documents

This document does not replace the existing product, domain, system, database, or security designs. It operationalizes them at the HTTP/API contract layer.

## 1.1 Source hierarchy

When API behavior is disputed, use:

```text
1. Applicable law/regulation
2. Signed external provider contract/current provider behavior
3. security_architecture_design.md
4. system_architecture_design.md
5. domain_model.md
6. database_design.md
7. this API contract
8. implementation convenience
```

A lower-level implementation cannot override a higher-level invariant.

## 1.2 Sitolo-specific API drivers

The API is shaped by:

- intermittent connectivity;
- Android-first low-end device constraints;
- multi-tenant and multi-branch operation;
- inventory concurrency;
- immutable financial facts;
- reconciliation and payment-provider ambiguity;
- offline commands and later synchronization;
- tax/EIS external integration;
- pharmacy and agro-dealer extension points;
- strict security testing;
- cost-sensitive operations for small merchants;
- future multi-country regionalization.

The existing product specification explicitly treats connectivity as an environmental dependency, keeps finalized financial records append-only, makes tenant isolation a security boundary, and places authorization on the server.

---

# 2. API Design Principles

## 2.1 Business concepts over tables

Do not expose endpoints merely because a table exists.

Bad:

```text
POST /sale_items
PATCH /inventory_balances
PUT /payment_status
DELETE /cash_events/123
```

Preferred:

```text
POST /v1/sales
POST /v1/sales/{sale_id}/finalize
POST /v1/sales/{sale_id}/void
POST /v1/sales/{sale_id}/return
POST /v1/refunds
POST /v1/inventory/adjustments
POST /v1/inventory/transfers
POST /v1/register-sessions/{session_id}/close
```

The preferred API communicates intent and protects state-machine boundaries.

## 2.2 Explicit commands

Commands are used for operations where business semantics matter more than generic partial updates:

```text
CreateSale
FinalizeSale
VoidSale
CreateReturn
ApproveRefund
ReceiveGoods
AdjustStock
CreateTransfer
PostStockCount
OpenRegister
CloseRegister
CreatePaymentIntent
ReconcilePayment
SyncCommands
InviteUser
ChangeMembershipScope
RotateDevice
RequestExport
```

## 2.3 Query endpoints

Queries are allowed to expose read projections designed for the client and use case. They do not imply write capability.

## 2.4 No accidental authority

The following client fields are never inherently authoritative:

```text
tenant_id
organization_id
branch_id
warehouse_id
user_id
role
permissions
is_admin
is_paid
paid
payment_status
payment_amount
inventory_quantity
server_time
approval_status
mra_submitted
```

The server derives or verifies authoritative values.

---

# 3. Transport and Protocol Baseline

## 3.1 HTTPS only

Production API traffic uses TLS. HTTP plaintext is not an accepted application transport in production.

The edge layer terminates TLS according to the deployment architecture and forwards traffic to the application only over an explicitly trusted private channel where applicable.

## 3.2 HTTP version

The application must function correctly over the platform-supported HTTP versions exposed by the selected ingress. API semantics must not depend on connection persistence or a particular HTTP version.

## 3.3 Content types

Primary request and response media type:

```text
application/json
```

Machine-readable API errors should use:

```text
application/problem+json
```

RFC 9457 defines the standardized problem-details structure for HTTP APIs. It exists specifically to provide machine-readable error details without inventing an unrelated error format for every API.

## 3.4 Character encoding

JSON payloads are UTF-8.

The API must reject malformed encodings rather than attempting ambiguous repair for security-sensitive inputs.

## 3.5 HTTP semantics

The API follows standard HTTP meaning for safe and idempotent methods. HTTP `PUT` and `DELETE` are idempotent by specification, while `POST` is not inherently idempotent and should only be retried automatically when the application establishes safe idempotency semantics.

For financial/business commands, Sitolo therefore uses application-level idempotency keys regardless of whether a route happens to use `POST`.

---

# 4. Base URL and Versioning

## 4.1 Production base

Logical form:

```text
https://api.<controlled-sitolo-domain>/v1
```

The actual production hostname is deployment configuration, not hard-coded domain truth inside domain modules.

## 4.2 Version strategy

The external contract is versioned at the API boundary:

```text
/v1/...
```

Versioning is intentionally coarse-grained enough to protect mobile clients from breaking changes while avoiding separate service versions for every internal module.

## 4.3 Breaking changes

The following are breaking changes unless an explicit compatibility mechanism exists:

- removing a route;
- changing authentication requirements;
- changing authorization semantics from allow to deny or vice versa unexpectedly;
- removing a response field consumed by a supported client;
- changing field type incompatibly;
- changing enum meanings;
- altering state-machine semantics;
- changing idempotency behavior;
- changing monetary meaning or rounding semantics;
- changing pagination semantics in a way that can duplicate/omit records;
- changing synchronization semantics;
- changing error codes relied upon by supported clients.

## 4.4 Non-breaking evolution

Prefer:

- additive response fields;
- new optional request fields with safe defaults;
- new endpoint versions for incompatible semantics;
- explicit deprecation periods;
- capability negotiation where necessary.

Never silently reinterpret an existing field in a materially different way.

---

# 5. API Host and Surface Classification

Every route must belong to an explicit exposure class.

```text
PUBLIC
AUTHENTICATED
AUTHENTICATED-SENSITIVE
PRIVILEGED
ADMIN
WEBHOOK
INTERNAL
HEALTH / OPERATIONS
```

## 5.1 Public

Examples:

```text
GET /v1/health/public
GET /v1/metadata/public
```

Public routes must contain no tenant-sensitive or operationally dangerous information.

## 5.2 Authenticated

Requires a valid authenticated principal and applicable scope.

## 5.3 Authenticated-sensitive

Handles financial, PII, security, inventory or similarly sensitive data. Requires additional controls such as stricter rate limits, minimal projections, enhanced audit, and potentially step-up authentication.

## 5.4 Privileged

Examples:

- user management;
- role assignment;
- high-value refunds;
- payment credential changes;
- security-policy changes;
- bulk exports;
- ownership transfer.

These require explicit permissions and may require approvals or fresh authentication.

## 5.5 Admin

Sitolo platform operations are kept separate from merchant operations. Administrative access is never granted merely because a route name contains `/admin`.

## 5.6 Webhook

Provider-originated API callbacks are separately authenticated, validated, replay-protected and resource-bounded.

## 5.7 Internal

Internal routes are not made safe merely by private networking. Workload identity and authorization remain required.

## 5.8 Health and operations

Health and readiness endpoints return the minimum useful operational information. They never disclose connection strings, secrets, internal stack traces, unrestricted dependency state or tenant information.

---

# 6. Canonical Request Lifecycle

Every authenticated business request follows an ordered control pipeline.

```text
REQUEST
  |
  v
TLS / EDGE
  |
  v
HTTP parse
  |
  v
size + header limits
  |
  v
rate / concurrency quota
  |
  v
authentication
  |
  v
session / device state
  |
  v
tenant context
  |
  v
function authorization
  |
  v
request schema validation
  |
  v
object/resource authorization
  |
  v
property authorization
  |
  v
business state validation
  |
  v
idempotency evaluation
  |
  v
database transaction
  |
  +--> audit
  +--> outbox
  |
  v
response projection
  |
  v
redaction / security headers
  |
  v
RESPONSE
```

The order is intentional. For example, input validation does not replace authorization, and authorization does not replace business-state validation.

---

# 7. Authentication Contract

## 7.1 Authentication source

Authentication is delegated to the selected standards-compliant identity mechanism. The API consumes normalized identity claims or a validated session representation.

Sitolo does not invent a proprietary authentication protocol.

## 7.2 Principal

The Rust application receives a typed principal such as:

```text
Principal
  user_id
  session_id
  authentication_time
  authentication_strength
  device_id?
  identity_assurance
```

Claims such as roles and tenant scope must be independently constrained by server-side membership/policy where their freshness or authority matters.

## 7.3 Token validation

Where bearer JWT access tokens are used, the API validates at minimum:

```text
signature
issuer
audience
expiration
not-before where applicable
intended token use / type
key validity
```

No API handler accepts an unsigned or improperly validated token.

## 7.4 Session state

Authentication and current session validity remain separately enforceable. A token can be structurally valid while the session/device/account is revoked.

## 7.5 Authentication failures

Authentication errors should avoid unnecessary account enumeration.

Example response:

```json
{
  "type": "https://api.sitolo.example/problems/authentication-failed",
  "title": "Authentication failed",
  "status": 401,
  "code": "AUTHENTICATION_FAILED",
  "detail": "The supplied authentication credentials are not valid.",
  "request_id": "req_01..."
}
```

The response must not reveal whether a particular phone/email belongs to a Sitolo account when enumeration would materially increase risk.

---

# 8. Tenant Context Contract

## 8.1 Trusted tenant derivation

The server resolves tenant context using:

```text
authenticated principal
      +
organization membership
      +
role/scope
      +
requested resource
      +
policy
```

A caller-provided tenant identifier can select a requested context but cannot grant authority.

## 8.2 Request behavior

A route may allow:

```http
GET /v1/organizations/{organization_id}/products
```

but the backend must prove the authenticated principal may access that organization.

## 8.3 Cross-tenant access

Any attempt to access a valid object in another tenant must fail with a safe authorization/not-found behavior according to the endpoint's enumeration policy.

The API must not leak the object's existence merely because the ID is valid.

## 8.4 Branch scope

A user may be authorized for one branch, several branches, or organization-wide scope. The API must enforce branch scope for every operation where the domain requires it.

## 8.5 Device scope

A device used by a cashier is not automatically equivalent to a user or organization administrator. Device registration narrows risk but never substitutes for user/session authorization.

---

# 9. Authorization Contract

Authorization is evaluated across multiple dimensions.

```text
WHO?
  |
WHICH ORGANIZATION?
  |
WHICH BRANCH / RESOURCE SCOPE?
  |
WHICH ACTION?
  |
WHICH OBJECT?
  |
WHICH FIELDS?
  |
WHICH CURRENT STATE?
  |
WHICH VALUE / THRESHOLD?
  |
NEED APPROVAL OR STEP-UP?
```

OWASP's API Security Top 10 explicitly treats broken object-level authorization, broken object-property authorization and broken function-level authorization as major API risks. Sitolo therefore treats them as separate controls rather than one generic `is_authorized` boolean.

## 9.1 Object-level authorization

Every endpoint accepting a caller-controlled object identifier must verify object scope.

Unsafe:

```text
GET /v1/sales/{id}
SELECT * FROM sales WHERE id = $1
```

Safe model:

```text
authenticate
→ resolve tenant
→ resolve branch scope
→ authorize READ_SALE
→ query WHERE tenant_id = trusted_tenant AND id = $1
→ project safe fields
```

## 9.2 Function-level authorization

Examples:

```text
SALE_CREATE
SALE_VOID
REFUND_CREATE
REFUND_APPROVE
STOCK_ADJUST
STOCK_ADJUST_APPROVE
USER_INVITE
ROLE_ASSIGN
EXPORT_CREATE
PAYMENT_CREDENTIAL_UPDATE
```

The route itself does not imply permission.

## 9.3 Property-level authorization

Requests must use allowlisted write models.

A cashier's sale request must not accept arbitrary fields such as:

```text
organization_id
approved_by
settled_at
internal_margin
payment_verified
risk_score
```

Unknown or forbidden properties are rejected where mass-assignment risk exists.

---

# 10. Request Schema Rules

Every endpoint has a named request DTO.

Do not deserialize directly into a persistence model.

Recommended layers:

```text
HTTP JSON
  ↓
Request DTO
  ↓
validated command
  ↓
domain value objects
  ↓
application service
```

## 10.1 Unknown fields

For security-sensitive mutation requests, reject unknown fields unless there is a documented compatibility reason not to.

## 10.2 Maximum body size

Every route class has an explicit body limit.

Example policy:

```text
simple query/body       small bounded limit
POS command             bounded moderate limit
bulk import             larger but dedicated endpoint
file upload             streaming quota path
```

Exact limits are deployment/configuration policy and must be load-tested.

## 10.3 Arrays

Every client-controlled array has a maximum element count.

Example:

```text
sale.items <= configured maximum
batch.commands <= configured maximum
export.columns <= configured maximum
```

## 10.4 String limits

Every free-form string has a length limit appropriate to its business meaning.

Examples:

```text
SKU
barcode
product name
sale note
customer name
reason
address
external reference
```

The API must not permit arbitrary megabyte-scale strings simply because JSON can technically carry them.

## 10.5 Numeric bounds

Quantities, discounts, percentages and amounts must have domain-valid ranges.

## 10.6 Date/time

API timestamps use an unambiguous machine-readable representation. Persisted business timestamps should retain timezone/offset semantics appropriate to the domain, while the server remains authoritative for server-side security decisions.

---

# 11. Identifier Contract

Public resource identifiers should use a stable opaque identifier strategy. UUID/ULID selection follows the domain and database contract.

Never assume that opacity alone provides authorization.

The API must reject malformed identifiers before database work where possible.

Examples:

```text
organization_id
branch_id
product_id
sku_id
sale_id
payment_intent_id
device_id
command_id
```

Human-facing numbers such as `SALE-2026-000123` are distinct from internal object IDs.

---

# 12. Pagination Contract

Large collections must never be returned without bounds.

## 12.1 Default

Every collection endpoint has a bounded default page size.

## 12.2 Maximum

Clients cannot exceed a server-defined maximum using `limit` or equivalent parameters.

## 12.3 Cursor pagination

For high-volume or mutable collections, cursor/keyset pagination is preferred.

Canonical pattern:

```http
GET /v1/products?limit=50&after=<opaque-cursor>
```

The cursor is opaque. It does not expose SQL expressions or trusted authorization state.

## 12.4 Stable ordering

Pagination requires deterministic ordering. A unique tie-breaker should be included so records are not repeatedly skipped or duplicated when timestamps collide.

## 12.5 Offset pagination

Offset may be used for small administrative datasets, but it must not become the default for high-volume sales, audit, events or inventory collections where deep offsets become expensive.

---

# 13. Filtering and Sorting

Filtering is represented as structured parameters, not arbitrary SQL.

Allowed example:

```http
GET /v1/products?status=ACTIVE&category_id=cat_123
```

Sorting must map from a closed allowlist:

```text
name       -> static SQL expression
created_at -> static SQL expression
sku        -> static SQL expression
```

An arbitrary client string must never become a SQL identifier, expression or fragment.

---

# 14. Field Projection

Response DTOs are allowlists.

The API should never serialize full database rows merely because serialization is convenient.

Sensitive examples:

```text
internal_cost
provider_credentials
MFA secrets
security metadata
internal notes
support annotations
risk controls
raw provider payloads
```

Bulk exports receive their own authorization and projection contracts.

---

# 15. HTTP Method Policy

## GET

Use for retrieval and safe reads.

## POST

Use for creation and explicit commands where the operation is not naturally represented by `PUT`.

POST mutations should generally require an idempotency key when a retry could duplicate a business effect.

## PUT

Use only where full replacement or a genuinely idempotent command is intended.

## PATCH

Use narrowly for explicit partial updates whose fields are safely allowlisted. Do not use generic JSON merge semantics for sensitive aggregates unless carefully constrained.

## DELETE

Use only for truly deletable resources. Financial facts, audit history, inventory movements and other immutable evidence are not deleted merely because an API consumer wants a delete endpoint.

A business cancellation is frequently a state transition or compensating command, not an HTTP `DELETE`.

---

# 16. Idempotency Contract

## 16.1 Why

A merchant on an unstable mobile network can submit a sale, lose the response, then retry. The server may already have committed the transaction.

Sitolo must make this safe.

## 16.2 Idempotency header

For eligible commands:

```http
Idempotency-Key: <client-generated unique value>
```

The key must be scoped to the authenticated principal/tenant and endpoint semantics as defined by the command contract.

## 16.3 Idempotency record

The server records sufficient information to determine that a request with the same key is:

- already completed;
- currently being processed;
- invalid because the key is being reused with different request semantics;
- expired according to the retention policy.

## 16.4 Payload mismatch

Same key + materially different request body must not silently reuse the previous result.

The API returns a conflict-style error such as:

```text
IDEMPOTENCY_KEY_REUSED_WITH_DIFFERENT_REQUEST
```

## 16.5 Exactly-once language

The API must not claim mathematical “exactly once” delivery over unreliable networks. The implementation provides **duplicate-safe application semantics** for recognized commands.

---

# 17. Conditional Requests and Concurrency

Where resource editing is allowed, the API may use version tokens or ETags to prevent lost updates.

Example:

```http
If-Match: "v42"
```

The server rejects stale mutations rather than silently overwriting newer state.

This is separate from database transaction locking. A conditional HTTP update prevents certain stale-client errors; PostgreSQL transactions protect the actual authoritative mutation.

For high-contention financial/inventory operations, domain commands remain preferred over generic entity patching.

---

# 18. Error Contract

Sitolo uses RFC 9457-style problem details with Sitolo-specific extensions.

Canonical shape:

```json
{
  "type": "https://api.sitolo.example/problems/inventory-insufficient-stock",
  "title": "Insufficient stock",
  "status": 409,
  "code": "INVENTORY_INSUFFICIENT_STOCK",
  "detail": "The requested quantity is not currently available.",
  "request_id": "req_01J...",
  "retryable": false,
  "field_errors": []
}
```

## 18.1 Required fields

At minimum:

```text
type
title
status
code
request_id
```

`detail` is safe explanatory information, not a debugging dump.

## 18.2 Field validation errors

Example:

```json
{
  "type": "https://api.sitolo.example/problems/validation-error",
  "title": "Request validation failed",
  "status": 422,
  "code": "VALIDATION_ERROR",
  "request_id": "req_01...",
  "field_errors": [
    {
      "field": "items[0].quantity",
      "code": "INVALID_QUANTITY"
    }
  ]
}
```

## 18.3 No stack traces

Production responses never include:

- stack traces;
- SQL text;
- filesystem paths;
- environment variables;
- secret values;
- raw provider responses where sensitive;
- Rust panic output;
- internal network addresses.

---

# 19. HTTP Status Policy

The following is the baseline policy; individual routes may have narrower semantics.

```text
200 OK
201 Created
202 Accepted
204 No Content
400 Bad Request
401 Unauthorized
403 Forbidden
404 Not Found
405 Method Not Allowed
409 Conflict
412 Precondition Failed
415 Unsupported Media Type
422 Unprocessable Content
429 Too Many Requests
500 Internal Server Error
502 Bad Gateway
503 Service Unavailable
504 Gateway Timeout
```

## 19.1 401

No usable authentication for an authenticated route.

## 19.2 403

Authenticated principal exists but lacks required authority, where revealing that distinction is safe.

## 19.3 404

May be used for authorization-sensitive object lookup to avoid exposing existence.

## 19.4 409

Business conflict or concurrency/idempotency conflict.

Examples:

```text
INVENTORY_INSUFFICIENT_STOCK
SALE_ALREADY_FINALIZED
REFUND_EXCEEDS_ELIGIBLE_AMOUNT
STALE_RESOURCE_VERSION
IDEMPOTENCY_KEY_REUSED
PAYMENT_STATE_CONFLICT
```

## 19.5 422

Syntactically valid request with domain-validation failure, where that distinction is useful.

## 19.6 429

Explicit rate/quota exhaustion.

The response may include `Retry-After` where a retry time can be communicated safely.

---

# 20. Request Correlation

Every request receives a server-controlled correlation identifier.

Accepted client correlation IDs may be used as trace context only if validated and safely bounded; server-generated request IDs remain authoritative.

Response:

```http
X-Request-ID: req_01J...
```

Internal observability links:

```text
request_id
trace_id
span_id
command_id
sale_id/payment_id/etc.
```

These identifiers must not become unbounded user-controlled log labels.

---

# 21. API Resource Families

The initial resource families are derived from the Sitolo system and domain model.

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
  /warehouses
  /registers
  /memberships
  /roles
  /permissions

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
  /discount-policies

PROCUREMENT
  /suppliers
  /purchase-orders
  /goods-receipts

INVENTORY
  /stock
  /inventory-movements
  /lots
  /transfers
  /counts
  /adjustments

POS / SALES
  /sales
  /sale-lines (only where justified; not necessarily public)
  /returns
  /refunds

CASH
  /register-sessions
  /cash-events
  /cash-reconciliations

PAYMENTS
  /payment-intents
  /payments
  /provider-events
  /reconciliation-cases

TAX / EIS
  /tax-configurations
  /tax-submissions
  /tax-evidence

REPORTING
  /reports
  /exports
  /export-jobs

BILLING
  /subscriptions
  /plans
  /entitlements
  /usage

AUDIT
  /audit-events (strictly privileged/read-only projections)

PLATFORM OPS
  /support-cases
  /support-access-grants
  /feature-flags
  /integrations
```

This list is an initial public contract inventory, not a guarantee that every path is exposed in the first release.

---

# 22. Identity Endpoints

## GET `/v1/me`

Returns the authenticated user's non-sensitive profile and current platform identity context.

Must not expose secrets or provider credentials.

## GET `/v1/sessions`

Returns the current user's sessions subject to privacy and security policy.

## POST `/v1/sessions/revoke`

Revokes a session or controlled set of sessions subject to authorization.

## GET `/v1/devices`

Returns devices the principal is authorized to inspect.

## POST `/v1/devices`

Registers a device through the approved device enrollment flow.

## POST `/v1/devices/{device_id}/revoke`

Revokes a device.

Device revocation is security-sensitive and must invalidate or restrict subsequent sync according to the offline contract.

---

# 23. Organization and Branch Endpoints

Examples:

```text
GET  /v1/organizations
GET  /v1/organizations/{organization_id}
POST /v1/organizations
PATCH /v1/organizations/{organization_id}

GET  /v1/organizations/{organization_id}/branches
POST /v1/organizations/{organization_id}/branches
GET  /v1/branches/{branch_id}
PATCH /v1/branches/{branch_id}
```

Authorization varies by operation.

Branch mutation is not granted merely because a user can view a branch.

Organization ownership changes require elevated controls.

---

# 24. Membership and IAM Endpoints

Examples:

```text
GET  /v1/organizations/{organization_id}/memberships
POST /v1/organizations/{organization_id}/memberships
PATCH /v1/memberships/{membership_id}
POST /v1/memberships/{membership_id}/suspend
POST /v1/memberships/{membership_id}/restore
```

Role assignment is a privileged operation.

A membership response must not accidentally expose hidden security metadata or authentication secrets.

Scope updates should produce audit evidence.

---

# 25. Product and Catalogue API

## GET `/v1/products`

Supports bounded filters and cursor pagination.

Example:

```http
GET /v1/products?status=ACTIVE&limit=50&after=cursor
```

## POST `/v1/products`

Creates a product inside a trusted organization context.

The request must not trust client-supplied `organization_id` as authorization evidence.

## GET `/v1/products/{product_id}`

Requires object-level authorization.

## PATCH `/v1/products/{product_id}`

Only allowlisted mutable properties.

Historical sale snapshots cannot be modified through this endpoint.

---

# 26. Pricing API

Examples:

```text
GET  /v1/price-lists
POST /v1/price-lists
POST /v1/price-lists/{price_list_id}/prices
POST /v1/promotions
POST /v1/pricing/resolve
```

A pricing-resolution endpoint returns a deterministic decision for a current transaction context, but the finalized sale stores its own historical price snapshot.

The API must not recalculate historical transactions using today's price configuration.

---

# 27. Procurement API

Examples:

```text
GET  /v1/suppliers
POST /v1/suppliers
GET  /v1/purchase-orders
POST /v1/purchase-orders
POST /v1/purchase-orders/{id}/submit
POST /v1/purchase-orders/{id}/approve
POST /v1/purchase-orders/{id}/cancel
POST /v1/purchase-orders/{id}/receipts
```

Approval and receiving are separate state transitions.

A purchase order cancellation does not erase the historical procurement record.

---

# 28. Inventory API

## Query

```text
GET /v1/stock
GET /v1/inventory-movements
GET /v1/lots
```

## Mutation

```text
POST /v1/inventory/adjustments
POST /v1/inventory/transfers
POST /v1/stock-counts
POST /v1/stock-counts/{id}/submit
POST /v1/stock-counts/{id}/approve
POST /v1/stock-counts/{id}/post
```

## Critical rule

There is no generic endpoint equivalent to:

```text
PATCH /v1/inventory/{id}
{ "quantity": 999999 }
```

Inventory mutation occurs through controlled domain commands and ledger postings.

---

# 29. Sales API

The sales API distinguishes draft/cart behavior from finalized business facts.

## POST `/v1/sales`

Creates a sale/draft or submits a command according to the release contract.

Request example:

```json
{
  "location_id": "loc_123",
  "register_id": "reg_123",
  "items": [
    {
      "sku_id": "sku_123",
      "quantity": "2",
      "requested_unit": "piece"
    }
  ],
  "customer_id": null,
  "client_reference": "device-local-123"
}
```

The server resolves:

- organization;
- branch;
- SKU validity;
- available pricing;
- tax policy;
- authorization;
- inventory availability;
- totals.

The client-provided calculated total is not authoritative.

## POST `/v1/sales/{sale_id}/finalize`

Finalizes a sale subject to current state and authorization.

The implementation must protect the operation against duplicate requests.

## POST `/v1/sales/{sale_id}/void`

Uses business semantics, not a generic delete.

## GET `/v1/sales/{sale_id}`

Returns a safe sale projection after object-level authorization.

---

# 30. Sale Finalization Contract

A successful finalized-sale command must establish all required authoritative state atomically.

Conceptually:

```text
BEGIN
  authorize
  resolve current sale state
  validate pricing/tax snapshots
  validate inventory
  lock required stock rows / apply atomic checks
  create finalized sale facts
  create inventory movements
  create payment intent linkage if applicable
  create audit evidence
  create outbox events
COMMIT
```

The API must not report `201/200 success` before the authoritative transaction reaches the required commit point.

External payment/tax calls happen after or outside the transaction through controlled asynchronous flows where appropriate.

---

# 31. Returns, Voids and Refund APIs

These are distinct operations.

```text
POST /v1/sales/{sale_id}/void
POST /v1/returns
POST /v1/refunds
POST /v1/refunds/{refund_id}/approve
```

## Void

Applies only while the sale is in a state where void is legal.

## Return

Creates a goods-return business event, linked to the originating sale where applicable.

## Refund

Creates monetary return intent/state. Refund amount must not exceed eligible value.

## Approval

High-value refunds require configured approval policy.

The API must never expose a generic “edit sale” endpoint capable of silently changing finalized economics.

---

# 32. Cash API

Examples:

```text
GET  /v1/registers
POST /v1/register-sessions
POST /v1/register-sessions/{id}/open
POST /v1/register-sessions/{id}/cash-events
POST /v1/register-sessions/{id}/close
```

Closing a register is a controlled state transition.

A close operation records expected and counted values and preserves variance rather than hiding discrepancies.

---

# 33. Payment API

Payment APIs are deliberately separate from sales APIs.

Examples:

```text
POST /v1/payment-intents
GET  /v1/payment-intents/{id}
POST /v1/payment-intents/{id}/cancel
GET  /v1/payments/{id}
GET  /v1/reconciliation-cases
```

## Frontend authority rule

A request such as:

```json
{
  "paid": true,
  "amount": 100000
}
```

does not settle a payment.

The server compares payment evidence against the authoritative payment intent and provider evidence.

---

# 34. Webhook API

Example pattern:

```text
POST /v1/webhooks/{provider}
```

Webhook handling pipeline:

```text
HTTP receive
  ↓
body size limit
  ↓
provider authentication/signature
  ↓
timestamp/replay validation where supported
  ↓
schema validation
  ↓
provider event identity extraction
  ↓
transactional deduplication
  ↓
persist evidence
  ↓
acknowledge according to provider contract
  ↓
asynchronous business processing
```

The handler must not trust arbitrary provider payload fields as internal financial truth.

If a provider supplies no cryptographic authentication, the event remains an untrusted signal until reconciled through stronger evidence.

---

# 35. Reconciliation API

Examples:

```text
GET  /v1/reconciliation/cases
GET  /v1/reconciliation/cases/{id}
POST /v1/reconciliation/cases/{id}/resolve
```

Ambiguous payment matches must become explicit reconciliation cases rather than silent guesses.

Automatic matching order follows the domain contract:

```text
provider transaction ID
→ provider reference
→ Sitolo payment intent reference
→ tightly bounded fallback
→ manual review
```

---

# 36. MRA EIS API Boundary

Sitolo's public merchant API should expose Sitolo tax submission state rather than raw EIS protocol complexity.

Examples:

```text
GET /v1/tax/submissions/{sale_id}
POST /v1/tax/submissions/{sale_id}/retry
GET /v1/tax/evidence/{sale_id}
```

The API must preserve the distinction:

```text
SALE COMMITTED
      |
      +--> TAX PENDING
      |
      +--> TAX ACCEPTED
      |
      +--> TAX REJECTED
```

A rejected tax submission does not authorize mutation of the sale's financial facts merely to make the external response happy.

The exact EIS protocol is external contract data and must remain subject to current provider documentation/certification.

---

# 37. Offline Sync API

The sync API is a specialized command-ingestion boundary, not a generic replication endpoint.

Conceptual:

```http
POST /v1/sync/commands
```

Request:

```json
{
  "device_id": "dev_123",
  "protocol_version": 1,
  "commands": [
    {
      "command_id": "cmd_123",
      "command_type": "SALE_CREATE",
      "created_at_client": "2026-09-04T08:00:00Z",
      "payload": { }
    }
  ]
}
```

The server verifies:

- authenticated user/session or approved device capability;
- device registration;
- revocation state;
- organization scope;
- branch scope;
- command uniqueness;
- protocol version;
- payload limits;
- command age rules;
- authorization;
- business-state validity;
- idempotency.

The client does not receive authority to bypass these checks because the request came from a registered device.

---

# 38. Sync Response Contract

Responses distinguish:

```text
ACCEPTED
DUPLICATE_ALREADY_APPLIED
REJECTED_INVALID
REJECTED_UNAUTHORIZED
REJECTED_CONFLICT
REQUIRES_REVIEW
RETRYABLE_FAILURE
```

Each command response should include its stable command ID.

Example:

```json
{
  "commands": [
    {
      "command_id": "cmd_123",
      "status": "DUPLICATE_ALREADY_APPLIED",
      "result_reference": "sale_987"
    }
  ],
  "next_checkpoint": "cp_456"
}
```

The server checkpoint is authoritative. Clients cannot claim synchronization progress simply by incrementing a local counter.

---

# 39. Reporting API

Reports are read models and derived views, not alternative financial authorities.

Examples:

```text
GET  /v1/reports/sales-summary
GET  /v1/reports/inventory-summary
POST /v1/report-jobs
GET  /v1/report-jobs/{id}
```

Expensive reports should become asynchronous jobs rather than blocking POS paths.

Report access is tenant/branch/role scoped.

Large result sets are paginated or exported asynchronously.

---

# 40. Export API

Exports are high-risk bulk-data operations.

Examples:

```text
POST /v1/exports
GET  /v1/exports/{id}
POST /v1/exports/{id}/cancel
```

Controls:

- explicit permission;
- tenant scope;
- branch scope;
- date-range limit;
- row/size limit;
- audit event;
- rate/quota limit;
- expiring object URL;
- storage isolation;
- optional approval for high-risk exports.

CSV generation must account for spreadsheet formula injection and other downstream interpretation risks.

---

# 41. Billing and Entitlement API

Billing APIs should distinguish plan state from feature enforcement.

Examples:

```text
GET /v1/subscription
GET /v1/entitlements
GET /v1/usage
```

A UI may show an entitlement, but server-side API routes enforce it.

A client cannot unlock a premium operation by setting:

```json
{ "plan": "enterprise" }
```

---

# 42. Admin and Support API

Support tooling is a separate security boundary.

Examples:

```text
POST /v1/support/access-grants
GET  /v1/support/cases
POST /v1/support/cases/{id}/resolve
```

Merchant-data access for support must be:

```text
scoped
approved where required
time-limited
purpose-bound
audited
revocable
```

There is no silent universal support backdoor.

---

# 43. File Upload API

Example:

```text
POST /v1/files/uploads
```

Uploads must be handled as a dedicated pipeline:

```text
authenticate
→ authorize tenant purpose
→ quota
→ streaming size limit
→ content signature/type detection
→ generated object identity
→ quarantine
→ optional malware/CDR processing
→ private object storage
→ metadata record
```

User filenames are metadata, not storage paths.

---

# 44. SSRF / URL Handling

The API should avoid generic “fetch this URL” features.

If a URL must be accepted:

- validate protocol;
- validate hostname;
- resolve and check destination IPs;
- block loopback/private/link-local/metadata ranges as applicable;
- limit redirects;
- apply strict timeout;
- enforce response size limits;
- isolate network access;
- never expose database/network credentials to the fetching worker.

Merchant-provided URLs must not become arbitrary server-side network authority.

---

# 45. CORS Contract

CORS is an explicit allowlist policy for browser clients.

Production policy:

```text
allowed origins = configured exact origins
credentials = only where required
wildcard origin + credentials = prohibited
origin reflection = prohibited
```

CORS is not an authorization mechanism. Non-browser clients can ignore CORS.

---

# 46. CSRF Contract

Browser sessions authenticated with cookies require CSRF protection for state-changing requests.

Controls may include:

- CSRF token mechanism;
- SameSite cookies;
- Origin validation;
- Referer validation where appropriate;
- narrow cookie scope;
- no state-changing GET endpoints.

Native mobile clients using authorization headers are not a substitute justification for weakening browser protections.

---

# 47. Security Headers

Web-facing API/admin responses should apply an appropriate baseline such as:

```text
Strict-Transport-Security
Content-Security-Policy (for HTML/web surfaces)
X-Content-Type-Options
Referrer-Policy
frame-ancestors / framing restrictions
Cache-Control for sensitive responses
```

Exact deployment policy is documented separately in the security/deployment specifications.

---

# 48. Rate Limiting

Rate limits are multidimensional.

```text
IP
account/user
organization/tenant
device
endpoint class
concurrent request count
request bytes
operation cost
```

Important API classes:

| Class | Required treatment |
|---|---|
| Login | strict abuse controls |
| MFA/OTP | issuance + verification quotas |
| Password reset | anti-enumeration + strict limits |
| POS | high throughput but bounded |
| Sync | device + bytes + commands + age limits |
| Reports | asynchronous for expensive work |
| Exports | strict + audited |
| Webhooks | provider/event/concurrency aware |
| Admin | very strict + alerting |
| Upload | size/storage quotas |

Redis may be used as a shared rate-limiting/coordination mechanism, but it never becomes financial authority.

---

# 49. Timeouts

Every request class has a maximum useful execution time.

The API distinguishes:

```text
edge timeout
request timeout
DB statement timeout
transaction timeout
external provider timeout
worker/job timeout
```

A timeout is not permission to retry blindly. The caller must use the endpoint's documented retry/idempotency semantics.

---

# 50. Retry Contract

Clients must know whether a failure is safely retryable.

Response field:

```json
{
  "retryable": true
}
```

This field is advisory; clients must still honor endpoint-specific semantics.

The server never retries a non-idempotent financial effect automatically unless it can prove duplicate safety.

External provider calls use adapter-specific bounded retry policies.

---

# 51. `Retry-After`

When a rate limit or service-unavailability condition has a meaningful retry interval, the server may return:

```http
Retry-After: 30
```

Clients must not convert this into an unbounded retry loop.

---

# 52. Caching Contract

Safe-cacheable read endpoints must define:

- scope;
- TTL;
- invalidation requirements;
- authorization interaction;
- whether stale data is acceptable.

Security-sensitive resources must not leak through shared caches across tenants.

A cache miss may cause slower execution. Cache availability must not be required to establish financial truth or authorization.

---

# 53. ETags and Resource Versions

ETags or explicit versions can be used for mutable configuration/resources where stale writes are a concern.

Example:

```http
ETag: "resource-v42"
```

Mutation:

```http
If-Match: "resource-v42"
```

If the resource changed:

```text
412 PRECONDITION FAILED
```

or a domain-specific `409 CONFLICT` according to the command's semantics.

Do not use ETag matching as a replacement for proper database transactions.

---

# 54. Security-Sensitive Headers

The application should recognize and safely handle:

```text
Authorization
Content-Type
Content-Length
Accept
Origin
If-Match
Idempotency-Key
Traceparent / tracing headers where enabled
```

Headers are bounded in count, length and total size.

Never log bearer tokens or sensitive cookies.

---

# 55. API Authentication vs Device Authorization

The API distinguishes:

```text
USER AUTHENTICATION
DEVICE REGISTRATION
DEVICE REVOCATION
OFFLINE AUTHORITY
```

A registered device is not automatically an administrator.

A device identifier is not a bearer credential.

Offline capabilities are bounded to the user's/device's approved operational scope.

---

# 56. API Command Envelope

For command-style endpoints, a standardized envelope may carry:

```text
command_id
idempotency_key
client_timestamp
protocol_version
device_id
correlation_id
payload
```

Security-sensitive server facts must never be accepted merely because they are present in this envelope.

The envelope supports transport semantics; domain validation remains authoritative.

---

# 57. Idempotency and Command IDs Are Different

Do not collapse these concepts.

`command_id` identifies a business/client command.

`Idempotency-Key` identifies duplicate submission semantics at the API request boundary.

A single command may produce multiple internal domain events.

Example:

```text
command_id = cmd_123
        |
        +--> SaleCreated
        +--> InventoryDecreased
        +--> AuditRecorded
        +--> OutboxEventCreated
```

---

# 58. API and Domain Event Separation

A successful HTTP response does not necessarily mean every asynchronous side effect has completed.

For example:

```text
POST /v1/sales/{id}/finalize
        |
        v
transaction commits
        |
        +--> audit committed
        +--> outbox committed
        |
        +--> async tax submission
        +--> async notification
```

The response must clearly state authoritative business state rather than pretending downstream integrations are already complete.

---

# 59. Async Job Contract

Long-running operations return a job resource or `202 Accepted`.

Example:

```json
{
  "job_id": "job_123",
  "status": "QUEUED",
  "status_url": "/v1/export-jobs/job_123"
}
```

States:

```text
QUEUED
RUNNING
SUCCEEDED
FAILED
CANCELLED
EXPIRED
```

Jobs require tenant scope and cannot be fetched merely because the caller guesses an ID.

---

# 60. Long-Running Report and Export Isolation

Report/export workers should be isolated from latency-sensitive POS operations.

At minimum the platform must bound:

- query duration;
- row count;
- serialized output size;
- concurrent jobs per tenant;
- global concurrent jobs;
- storage consumption;
- job retention.

A merchant running a giant historical export must not starve other merchants' checkout requests.

---

# 61. Bulk Mutation APIs

Bulk operations require stronger validation than ordinary mutation.

A CSV import or bulk product update must not become a method for bypassing per-object authorization.

Pipeline:

```text
upload
→ scan
→ parse
→ schema validate
→ semantic validate
→ preview
→ authorize
→ approve if necessary
→ commit
```

The imported file cannot choose its own tenant scope.

---

# 62. API Security Against Mass Assignment

All writable properties are explicit.

Bad:

```rust
#[derive(Deserialize)]
struct GenericUpdate {
    #[serde(flatten)]
    fields: HashMap<String, Value>,
}
```

unless the route has an extremely controlled purpose and schema.

Preferred:

```rust
struct UpdateProductRequest {
    name: Option<ProductNameInput>,
    description: Option<String>,
    active: Option<bool>,
}
```

Protected fields have no request representation unless the operation explicitly needs them.

---

# 63. SQL Injection Contract

The API layer must not construct SQL from request strings.

Use SQLx parameter binding and allowlisted query construction.

Dynamic sorting/filtering is resolved from typed enumerations.

No API request can supply:

```text
raw SQL
SQL identifiers
arbitrary WHERE expressions
unbounded regex evaluated by a vulnerable engine
```

Security regression tests must actively exercise quote, comment, encoded and boolean-manipulation payloads.

---

# 64. NoSQL / Search Injection

PostgreSQL is authoritative today, but Redis/search adapters may exist.

User input must not become an arbitrary search DSL or query operator tree without typed validation.

A future document/search subsystem must maintain a separate query AST rather than deserialize user JSON directly into backend query objects.

---

# 65. XSS API Boundary

The API may store merchant-authored text, but storage is not permission to execute that text as browser code.

Text fields must be returned as data and rendered safely by clients.

The API should not accept or normalize HTML unless the business explicitly requires rich content.

Any rich-text feature requires an explicit sanitizer and output-context contract.

---

# 66. CSRF API Boundary

The API must not expose state-changing operations through GET.

For browser-cookie authentication, state-changing requests use CSRF protection.

Webhook endpoints are not CSRF-protected by browser tokens; they have their own provider authentication model.

---

# 67. File Upload Security Contract

Upload endpoints require:

```text
authenticated principal
+ tenant scope
+ upload purpose
+ content length limit
+ type validation
+ storage quota
+ malware/sandbox path where required
```

Returned file references should use opaque IDs rather than direct arbitrary filesystem paths.

---

# 68. Path Traversal Contract

Any API field that might be used to identify a file is treated as hostile.

Never concatenate:

```text
storage_root + user_filename
```

The preferred storage API is:

```text
ObjectId
 → storage adapter
 → generated key
```

not user-provided paths.

---

# 69. SSRF Contract

API handlers should not perform arbitrary outbound fetching.

If unavoidable, use a dedicated outbound-fetch service/worker with network policy and destination validation.

The HTTP response must never include secret internal response bodies merely because an outbound call succeeded.

---

# 70. Secrets Contract

No API response includes secrets except carefully designed one-time enrollment flows where a credential is intentionally generated for immediate setup and the exposure is explicitly documented.

Never return:

```text
DB_PASSWORD
JWT_SIGNING_KEY
PAYMENT_SECRET
MRA_SECRET
WEBHOOK_SECRET
PRIVATE_KEY
```

Environment/configuration endpoints are privileged operational interfaces and are not exposed through merchant APIs.

---

# 71. API Logging Contract

Log structured metadata, not raw request bodies by default.

Never log:

- `Authorization` header;
- session/refresh/access tokens;
- password/reset tokens;
- MFA secrets;
- provider secrets;
- private keys;
- full payment credentials;
- unnecessary PII;
- complete uploaded documents;
- arbitrary SQL values.

Sensitive fields are explicitly redacted at the middleware/serializer boundary.

---

# 72. Audit Contract

Security-significant API operations emit audit evidence.

Minimum candidate fields:

```text
event_id
occurred_at
tenant_id
actor_id
actor_type
operation
resource_type
resource_id
result
reason_code
request_id
trace_id
device_id
approval_id
provider_reference where applicable
```

Audit records are not simply ordinary debug logs.

---

# 73. API Inventory Contract

The API inventory is a security artifact.

For every route record:

```text
method
path
version
exposure class
authentication requirement
authorization permission
tenant scope
branch scope
request DTO
response DTO
rate limit class
body size limit
timeout
idempotency requirement
audit requirement
PII class
security tests
owner
status
```

Runtime route discovery and documented OpenAPI routes should be compared in CI.

An undocumented production endpoint is a release defect unless explicitly exempted.

---

# 74. OpenAPI Contract

OpenAPI is the external contract representation for the HTTP surface.

The specification should be generated or maintained so that it can serve:

- documentation;
- client generation;
- contract validation;
- security inventory;
- test generation.

OpenAPI should not be treated as proof that an endpoint is secure. Security enforcement remains in Rust middleware/domain code and test suites.

OpenAPI schemas should be compatible with the project's JSON Schema/request validation strategy where appropriate. JSON Schema's current specification is Draft 2020-12.

---

# 75. Schema Ownership

For each request/response schema:

```text
business owner
security owner
version
compatibility policy
sensitive fields
validation rules
```

Generated TypeScript/Dart clients must be derived from the contract rather than hand-maintaining incompatible copies.

---

# 76. API Contract Tests

Every route requires at least:

```text
happy path
validation failure
authentication failure
authorization failure
cross-tenant attempt
wrong-branch attempt where relevant
object-not-found
state conflict
rate limit behavior
retry behavior
idempotency behavior where required
```

High-risk routes also require:

```text
property-level authorization
race-condition tests
replay tests
fuzz tests
resource exhaustion tests
```

---

# 77. Tenant Negative Test Matrix

For every tenant-scoped endpoint:

| Principal | Target | Expected |
|---|---|---|
| authorized tenant user | own object | allow |
| same tenant wrong role | protected action | deny |
| same tenant wrong branch | branch resource | deny |
| other tenant user | valid foreign object | deny |
| removed member | old tenant object | deny |
| revoked device | sync target | deny/restrict |
| stale session | sensitive mutation | deny |
| unauthenticated | protected route | deny |

The test must use real PostgreSQL for repository/RLS behavior where applicable; application mocks cannot prove database isolation.

---

# 78. BOLA / IDOR Regression Contract

Every route containing an object identifier must have a dedicated negative test.

Examples:

```text
GET /sales/{victim_sale}
PATCH /products/{victim_product}
POST /refunds/{victim_sale}
GET /inventory/{victim_location}
GET /exports/{victim_export}
```

Tests must attempt:

- sequential IDs;
- guessed IDs where applicable;
- valid IDs obtained from another tenant fixture;
- cross-branch IDs;
- stale IDs;
- revoked-user access.

The API must not accidentally leak fields through error responses either.

---

# 79. Function Authorization Regression Contract

Every privileged operation has explicit permission coverage.

Example:

```text
SALE_VOID
REFUND_APPROVE
STOCK_ADJUST
STOCK_ADJUST_APPROVE
ROLE_ASSIGN
EXPORT_CREATE
MFA_RESET
PAYOUT_CHANGE
TENANT_OWNER_TRANSFER
```

For each permission:

```text
authorized actor → success
unauthorized actor → deny
same actor wrong scope → deny
client UI bypass/direct HTTP → still deny
```

---

# 80. Property Authorization Regression Contract

Test that a caller cannot modify protected fields merely by adding them to JSON.

Example attack:

```json
{
  "name": "Normal Product",
  "organization_id": "victim",
  "cost_price": 1,
  "approved": true,
  "role": "OWNER"
}
```

Expected result: protected properties are rejected or ignored only where the contract deliberately defines an allow/ignore compatibility behavior. They must never silently gain authority.

---

# 81. Business Logic Security Contract

API authorization alone is insufficient.

The application service must enforce business invariants.

Examples:

```text
refund <= eligible refund amount
stock decrement <= allowed available quantity
sale cannot finalize twice
closed register cannot receive normal sales
expired/quarantined stock cannot be sold when policy forbids it
cashier cannot approve own restricted adjustment
payment cannot settle twice
provider event cannot be applied twice
```

OWASP explicitly identifies unrestricted access to sensitive business flows as an API risk; Sitolo therefore tests the composition of valid endpoints, not just isolated endpoint permission checks.

---

# 82. Concurrency Contract

A route that changes authoritative state must specify its concurrency model.

Possible mechanisms:

```text
atomic SQL update
row-level lock
optimistic version check
serializable transaction
unique constraint
state-machine recheck
```

Application-memory mutexes are not the primary protection in a horizontally scaled API.

---

# 83. Financial Mutation Contract

Financial mutations require:

```text
authentication
+ authorization
+ tenant scope
+ business-state check
+ idempotency
+ transaction
+ immutable history / compensating event
+ audit
+ outbox where needed
```

A request cannot return success if the authoritative state was not committed.

---

# 84. Payment Mutation Contract

Payment-related API routes add:

```text
provider evidence validation
provider/event deduplication
amount/currency validation
state transition validation
reconciliation state
```

The frontend is never the settlement authority.

---

# 85. Webhook Replay Contract

Webhook handling must be safe under:

```text
same payload twice
same event ID twice
event arrives late
events arrive out of order
signature is invalid
timestamp is outside allowed window
provider retries after timeout
```

A second accepted delivery must not create a second financial effect.

---

# 86. API Resource Exhaustion Contract

Every endpoint gets a resource profile.

Example:

```text
CPU class
memory class
DB query budget
external-call budget
max payload
max result
max concurrency
rate-limit class
timeout class
```

High-cost APIs are not allowed to share unlimited resources with checkout.

This directly addresses OWASP's API4 resource-consumption risk.

---

# 87. Report / Query Cost Controls

Queries must be bounded by:

- page size;
- date range;
- indexed filters where necessary;
- server-side statement timeout;
- maximum joins/operations appropriate to the implementation;
- asynchronous execution for heavy workloads.

Do not expose a general arbitrary query API to merchant clients.

---

# 88. API Security and Caching

Cached responses must include all security-relevant dimensions in their cache key where needed.

Example conceptual key:

```text
resource
+ tenant
+ branch scope
+ user/role context where required
+ policy version where required
```

A cached denial from one principal must not poison a subsequent authorized response.

---

# 89. Authentication Rate-Limit Contract

Login, OTP and recovery APIs are especially abuse-sensitive.

Controls should combine:

```text
source IP
account identifier
attempt count
device signals
cooldown
MFA state
```

Responses remain enumeration-resistant.

The system should distinguish rate limiting from account lockout so that an attacker cannot trivially deny service to a victim by repeatedly attempting the victim's identifier.

---

# 90. Password Reset API Contract

Password reset initiation:

```text
POST /v1/auth/password-reset/request
```

Requirements:

- anti-enumeration response;
- abuse rate limiting;
- one-time reset artifact;
- expiration;
- atomic redemption;
- session invalidation policy;
- audit evidence.

The raw reset token is never returned in ordinary API error messages or logs.

---

# 91. MFA API Contract

MFA setup and recovery are privileged security operations.

Routes must require:

- authenticated session;
- step-up where appropriate;
- explicit factor verification;
- recovery policy;
- audit.

Admin MFA reset is a high-risk privileged workflow and should require strong authorization and possibly approval.

---

# 92. Role Assignment API Contract

Role changes are themselves sensitive.

A user with authority to invite a cashier must not automatically receive authority to assign owner privileges.

The server enforces:

```text
caller permission
+ target role
+ scope
+ separation-of-duties rule
+ approval requirement
```

Role changes invalidate or refresh relevant authorization/cache state.

---

# 93. API Enumeration Resistance

Where object existence itself is sensitive, the API should use:

```text
404-like generic responses
consistent timing where practical
bounded error detail
```

Do not produce a distinguishable:

```text
"Tenant B exists but you are forbidden"
```

when the security model requires non-disclosure.

---

# 94. Error Timing

Avoid unnecessary high-signal timing differences in authentication and sensitive object lookup flows.

Timing equality does not need to be mathematically perfect, but obvious existence oracle behavior should be avoided where practical.

---

# 95. API Retry After Unknown Commit

This is a critical mobile failure case.

```text
CLIENT
  POST finalize sale
       |
       v
SERVER
  transaction commits
       |
       X response lost
       |
CLIENT
  timeout
       |
       v
RETRY SAME IDEMPOTENCY KEY
       |
       v
SERVER
  finds prior result
       |
       v
RETURN SAME BUSINESS OUTCOME
```

This is a central Sitolo reliability invariant.

---

# 96. API Behavior During Provider Failure

### Payment provider unavailable

The API does not invent success. It returns a safe state such as:

```text
PAYMENT_PENDING
PAYMENT_PROVIDER_UNAVAILABLE
```

subject to the domain contract.

### Tax provider unavailable

The sale may remain committed while tax submission is pending if current applicable policy permits it.

### Notification provider unavailable

Core merchant transaction does not fail merely because a notification provider is unavailable.

The distinction between business-critical and auxiliary dependencies is explicit.

---

# 97. Database Failure Contract

If PostgreSQL is unavailable, API mutations fail rather than pretending the server committed authoritative state.

Offline mobile operation is handled by the client command/sync design, not by a fake server response.

The API never fabricates a success receipt from an uncommitted database operation.

---

# 98. Transaction and External Call Rule

Never hold a PostgreSQL transaction open across an external network call.

Bad:

```text
BEGIN
  update payment
  call provider API  ← external network
  update payment
COMMIT
```

Preferred:

```text
BEGIN
  persist intent / work item
  commit

worker
  call provider
  receive result

BEGIN
  apply verified result
  commit
```

When a provider requires a tightly coupled two-step operation, the integration design must explicitly model compensation and ambiguity.

---

# 99. API and Outbox

When an API mutation changes authoritative state and needs asynchronous side effects:

```text
same DB transaction
  |
  +--> domain state
  +--> audit
  +--> outbox event
```

The worker later processes the outbox.

This prevents a successful DB commit from losing its asynchronous trigger because the process died after commit.

---

# 100. API and Inbox / Deduplication

Provider events and externally originated commands should use a deduplication/inbox concept where necessary.

Example:

```text
provider + event_id UNIQUE
```

A duplicate delivery becomes:

```text
already_processed
```

rather than a second side effect.

---

# 101. API Security Test Harness Requirements

The API test harness must support fixtures for:

```text
Tenant A
Tenant B
Branch A1
Branch B1
Owner A
Manager A
Cashier A
Inventory A
Auditor A
Owner B
Cashier B
Device A
Device B
Revoked Device
Expired Session
```

The harness must be able to submit raw HTTP requests independent of UI state.

That is essential because client-side controls do not count as API security.

---

# 102. Contract Test Fixtures

Every resource family gets deterministic fixture IDs in isolated test data.

Tests should be able to say:

```text
GET /v1/sales/sale-A
Authorization = cashier-A
```

and separately:

```text
GET /v1/sales/sale-B
Authorization = cashier-A
```

with the second request guaranteed to test cross-tenant access.

---

# 103. Fuzzing Contract

Fuzz or property-test at minimum:

```text
request JSON
identifier parsing
pagination cursors
search/filter parameters
sync envelopes
webhook payloads
file metadata
monetary values
quantities
```

A parser crash, unbounded allocation, panic or authorization bypass is a security defect.

---

# 104. HTTP-Level Security Tests

Staging/Dynamic tests should attempt:

```text
missing authentication
expired token
invalid token
wrong audience
wrong issuer
missing tenant context
cross-tenant ID
cross-branch ID
forbidden method
oversized body
oversized headers
malformed JSON
unknown fields
SQL injection
XSS payloads
path traversal values
SSRF payloads
CSRF requests
permissive CORS attempts
replay idempotency key
duplicate webhook
invalid webhook signature
rate limit exhaustion
```

---

# 105. API Panic Safety

Production HTTP handlers must not leak panics.

Unexpected Rust panics are converted into generic server errors, logged with restricted diagnostics and correlated to the request.

The application must decide whether a panic should terminate a process in accordance with Rust/runtime safety policy; the HTTP contract never returns internal stack details.

---

# 106. Rust API Module Boundary

Recommended structure:

```text
crates/
  domain/
  auth/
  authz/
  api/
  application/
  persistence/
  audit/
  outbox/
  integrations/
  observability/
```

Within the API crate:

```text
api/
  mod.rs
  error.rs
  extractors/
  middleware/
  routes/
    identity.rs
    organizations.rs
    catalogue.rs
    pricing.rs
    procurement.rs
    inventory.rs
    sales.rs
    cash.rs
    payments.rs
    tax.rs
    reporting.rs
    billing.rs
    support.rs
    sync.rs
```

The exact workspace/package decomposition may vary, but HTTP concerns must not infect the domain layer.

---

# 107. Axum Handler Contract

Handlers should be thin.

Conceptually:

```rust
async fn finalize_sale(
    State(state): State<AppState>,
    Principal(principal): Principal,
    TenantContext(ctx): TenantContext,
    Path(sale_id): Path<SaleId>,
    Json(request): Json<FinalizeSaleRequest>,
) -> Result<Json<SaleResponse>, ApiError> {
    let command = request.into_command()?;
    let result = state
        .sales
        .finalize(ctx, principal, sale_id, command)
        .await?;
    Ok(Json(result.into_response()))
}
```

The handler should not:

- write SQL directly;
- implement inventory accounting;
- determine authorization from UI data;
- perform arbitrary provider calls;
- generate financial totals itself.

---

# 108. Extractor Security Contract

Custom Axum extractors should enforce reusable boundaries:

```text
AuthenticatedPrincipal
TenantContext
RequestId
ValidatedJson<T>
Pagination
IdempotencyKey
```

Extractors must fail closed on malformed state.

A missing tenant context cannot silently become “global access.”

---

# 109. Middleware Order

The actual Tower/Axum order must be documented and tested because middleware ordering can create security bugs.

Conceptual order:

```text
transport / proxy trust
→ request ID
→ body limits
→ timeout
→ authentication
→ session/device state
→ tenant context
→ authorization hook / route handler
→ response security headers
```

Rate limits may be applied at multiple layers depending on whether the limiter needs the source IP, identity, tenant or endpoint class.

---

# 110. Proxy Trust Contract

If the deployment is behind a load balancer/proxy, the application must have an explicit trusted-proxy configuration for headers such as:

```text
X-Forwarded-For
Forwarded
X-Forwarded-Proto
```

The API must not trust arbitrary client-provided forwarding headers simply because they use a conventional name.

---

# 111. Request Body Streaming

Large inputs should be streamed where practical rather than eagerly allocating unbounded buffers.

File uploads must use a dedicated streaming path.

JSON bodies remain strictly bounded.

---

# 112. Compression

If HTTP compression is enabled:

- use bounded decompression;
- enforce uncompressed size limits;
- consider compression-bomb risk;
- do not compress highly sensitive responses in ways that create cross-origin side-channel concerns without review.

---

# 113. Content Negotiation

The API should support a deliberately narrow set of response media types.

Do not accept arbitrary content types for endpoints that only need JSON.

File upload endpoints should explicitly enumerate accepted formats.

---

# 114. HTTP Redirect Policy

API endpoints should generally avoid redirects.

Authentication/callback flows can define specific, exact redirect destinations.

A client-controlled redirect target must not be blindly echoed or fetched by the server.

---

# 115. API Deprecation

Deprecated endpoints require:

```text
owner
reason
replacement
telemetry
communication period
retirement date
```

Clients are not suddenly broken without a migration path unless an emergency security retirement requires immediate disablement.

---

# 116. Mobile Compatibility Contract

The API must tolerate supported mobile client versions during a declared compatibility window.

Because offline mobile clients may remain disconnected for long periods, version compatibility must be designed rather than assumed.

The sync protocol has its own version negotiation and rejection semantics.

---

# 117. API Capability Discovery

Where client behavior depends on deployment-specific capabilities, the API may expose a non-sensitive capability document.

Example:

```text
GET /v1/capabilities
```

It may advertise:

```text
payment providers available
sync protocol versions
feature availability
country configuration
supported client minimum versions
```

It must not expose secrets or internal architecture.

---

# 118. Feature Flags and API Enforcement

Feature flags are server-side enforcement boundaries.

A disabled feature must be rejected by the server even if an outdated mobile client still displays the feature.

The API response should use a stable domain code such as:

```text
FEATURE_NOT_ENABLED
ENTITLEMENT_REQUIRED
```

---

# 119. Entitlement Enforcement

The API checks current entitlement where an operation is restricted by subscription plan.

Entitlement evaluation must not depend solely on stale client cache.

Critical operations use current authoritative state.

---

# 120. Country / Jurisdiction Headers

Country or jurisdiction context should come from trusted tenant configuration and server-side policy.

A client cannot switch tax jurisdiction merely by setting:

```http
X-Country: XX
```

Client locale is presentation context; regulatory authority is server configuration.

---

# 121. Currency Contract

Every monetary API value is accompanied by a currency or occurs inside a context where the currency is unambiguous.

Use exact monetary representations.

Do not use floating-point JSON values for amounts where precision could affect financial correctness.

Preferred representations should be standardized across the API, for example:

```json
{
  "amount_minor": 150000,
  "currency": "MWK"
}
```

or an exact decimal contract where the domain explicitly requires it.

The chosen representation must be globally consistent and documented in the database/domain contracts.

---

# 122. Quantity Contract

Quantities are domain-aware values.

Avoid generic floating-point quantities.

Example:

```json
{
  "quantity": "2",
  "unit": "piece"
}
```

where the exact wire representation is standardized by the implementation team.

A quantity without a meaningful unit must not enter inventory-sensitive operations where the unit matters.

---

# 123. Tax Contract

Tax data included in sale responses is a snapshot of applied tax policy, not a promise that future policy will remain identical.

The API may expose:

```text
tax_category
tax_rate
calculation basis
tax_amount
configuration_version
```

subject to business/client needs.

It must not allow clients to overwrite authoritative tax outcomes merely by submitting a preferred tax rate.

---

# 124. Timestamp Contract

Returned timestamps should include clear timezone semantics and use a single documented serialization format.

The API distinguishes:

```text
occurred_at
created_at
updated_at
server_received_at
client_created_at
provider_event_time
```

These fields are not interchangeable.

---

# 125. API Sorting and Business Time

Sort order must be based on a named field with deterministic tie-breakers.

For financial chronology, the API should expose authoritative server sequence/timestamp metadata where needed rather than trusting client clocks.

---

# 126. API Privacy Minimization

Responses should expose only the minimum fields needed by the caller.

Examples:

A cashier may need:

```text
product name
SKU
sale price
availability
```

but not necessarily:

```text
supplier cost
margin
owner notes
security metadata
```

The API contract expresses this through role-aware projection, not only UI hiding.

---

# 127. API Object Ownership

The response model should make object scope visible where that improves client correctness but never leak hidden tenancy metadata unnecessarily.

Resource IDs should remain stable even if a client changes active branch context.

---

# 128. API Links

Hypermedia is optional. When used, links must be permission-safe.

Do not emit a link to an operation simply because the resource exists if the current principal cannot execute it.

---

# 129. API Documentation Security

OpenAPI documentation for public/authenticated surfaces must not expose:

- production secrets;
- internal credentials;
- private network names;
- private service endpoints;
- undocumented admin operations.

Internal API documentation can contain more operational detail but must remain access-controlled.

---

# 130. Webhook Documentation

Provider-specific webhook contracts must document:

```text
provider
path
signature scheme
signed bytes
required headers
timestamp semantics
replay window
idempotency key/event ID
payload schema
retry behavior
expected acknowledgment
failure behavior
```

The signed payload must be verified exactly as required by the provider contract.

---

# 131. API Contract for Provider Responses

External provider responses are untrusted input.

Adapters convert them into typed internal results:

```text
ProviderResponse
   ↓
validate transport
   ↓
validate authentication/integrity
   ↓
validate schema
   ↓
map known states
   ↓
reject impossible states
   ↓
internal domain result
```

Raw provider payloads should not be blindly propagated to clients.

---

# 132. Unknown Provider States

If an external provider sends a new or impossible state:

```text
do not silently map to SUCCESS
```

Prefer:

```text
UNKNOWN_PROVIDER_STATE
RECONCILIATION_REQUIRED
```

This is especially important for payments and tax integration.

---

# 133. API and Audit Ordering

For critical mutations, audit evidence should be committed atomically with authoritative business state where feasible.

The API must not claim an action is fully auditable if its evidence can routinely be lost after the business transaction commits.

---

# 134. API and Security Event Ordering

Detection events may be emitted asynchronously, but critical security state changes must not depend exclusively on best-effort telemetry.

Example:

```text
revoke device
```

must persist revocation authority independently of:

```text
send alert
```

---

# 135. Audit vs Request Log

An API request log says:

```text
POST /v1/refunds -> 403
```

An audit event says:

```text
actor A attempted REFUND_APPROVE on refund X
result DENIED
reason INSUFFICIENT_PERMISSION
```

These are different artifacts and should remain separately governed.

---

# 136. Security Classification of Routes

Every API endpoint receives a classification:

```text
PUBLIC
INTERNAL
PII
FINANCIAL
SECURITY
ADMIN
INTEGRATION
BULK
```

This classification influences:

- logging;
- retention;
- authorization;
- rate limits;
- audit;
- threat-model coverage;
- DAST coverage.

---

# 137. API Threat Model Mapping

The API must explicitly address:

```text
OWASP API1  BOLA
OWASP API2  Broken Authentication
OWASP API3  Property Authorization
OWASP API4  Resource Consumption
OWASP API5  Function Authorization
OWASP API6  Sensitive Business Flows
OWASP API7  SSRF
OWASP API8  Misconfiguration
OWASP API9  Inventory Management
OWASP API10 Unsafe API Consumption
```

In addition, Sitolo-specific API threats include:

```text
cross-tenant financial access
payment settlement forgery
offline replay
inventory race abuse
cash manipulation
support impersonation
report/export exfiltration
MRA evidence manipulation
branch hopping
approval self-dealing
```

---

# 138. Endpoint Inventory Completeness Test

CI should compare:

```text
implemented runtime routes
       vs
OpenAPI documented routes
       vs
approved endpoint inventory
```

Any unexpected difference should fail the security gate unless explicitly approved.

---

# 139. Contract Drift Test

For every supported generated client:

```text
OpenAPI
   ↓
generated models/client
   ↓
compile
   ↓
contract tests
```

If a backend change breaks generated client compatibility unexpectedly, CI fails.

---

# 140. API Schema Drift and Database Drift

API contracts must not be generated directly from arbitrary SQL schema changes.

Database schema evolves internally through migrations.

API schema evolves through explicit contract review.

A database migration is not automatically an API change, and an API change is not automatically a schema change.

---

# 141. Security Boundary: API vs Repository

Repositories enforce trusted scoping parameters supplied by application services.

An API handler should not have direct authority to bypass tenant-aware repository contracts.

Preferred:

```text
repository.get_sale(AuthorizedSaleScope, SaleId)
```

instead of:

```text
repository.get_sale(SaleId)
```

The latter makes omission of scope easier.

---

# 142. Security Boundary: API vs Domain

API DTOs are not domain entities.

The conversion layer validates and constructs domain values.

This prevents JSON serialization details from becoming the domain's security model.

---

# 143. API Error Taxonomy

Suggested stable codes:

```text
AUTHENTICATION_FAILED
SESSION_REVOKED
DEVICE_REVOKED
TENANT_ACCESS_DENIED
INSUFFICIENT_PERMISSION
RESOURCE_NOT_FOUND
VALIDATION_ERROR
INVALID_IDENTIFIER
STATE_CONFLICT
STALE_RESOURCE
IDEMPOTENCY_KEY_REUSED
RATE_LIMITED
REQUEST_TOO_LARGE
INVENTORY_INSUFFICIENT_STOCK
REFUND_EXCEEDS_ELIGIBLE_AMOUNT
PAYMENT_STATE_CONFLICT
PAYMENT_PROVIDER_UNAVAILABLE
WEBHOOK_SIGNATURE_INVALID
WEBHOOK_REPLAY
RECONCILIATION_REQUIRED
TAX_SUBMISSION_REJECTED
FEATURE_NOT_ENABLED
ENTITLEMENT_REQUIRED
DEPENDENCY_UNAVAILABLE
INTERNAL_ERROR
```

The list is extensible, but semantics must remain stable once client code depends on them.

---

# 144. Retryability Taxonomy

Each error code has an operational classification:

```text
NEVER_RETRY
RETRY_SAME_IDEMPOTENCY_KEY
RETRY_AFTER_DELAY
RETRY_AFTER_STATE_REFRESH
USER_ACTION_REQUIRED
ASYNC_RECONCILIATION
```

This classification should exist in implementation policy even if not all values are serialized directly.

---

# 145. Authentication Refresh Contract

Clients use the identity/session contract to obtain fresh access context.

The business API should not invent parallel refresh token endpoints if the identity platform already provides them.

The application invalidates/restricts sessions according to revocation policy.

---

# 146. API Logout Contract

Logout should revoke or invalidate the relevant session according to session architecture.

The server cannot securely assume that deleting a mobile UI token is sufficient revocation.

---

# 147. Admin Step-Up Contract

For high-risk operations, the API may require recent authentication evidence.

Example operations:

```text
change payout account
reset another user's MFA
transfer ownership
bulk export sensitive information
disable security controls
```

A stale session must not automatically retain unlimited authority for such operations.

---

# 148. Separation of Duties Through API

Where an operation requires approval:

```text
requester != approver
```

unless policy explicitly permits an exception.

The API must enforce this server-side.

Changing the request JSON from:

```json
{ "approved_by": "other-user" }
```

does not create valid approval evidence.

---

# 149. API and Approval State

Approval is a business state, not a boolean field controlled by the client.

Preferred:

```text
POST /v1/stock-adjustments/{id}/approve
```

rather than:

```text
PATCH /v1/stock-adjustments/{id}
{ "approved": true }
```

---

# 150. API and Immutable History

For immutable business records, API actions create new events:

```text
Original Sale
   ↓
Return / Reversal / Refund
   ↓
Corrected state / derived projection
```

The client cannot obtain a `200` response from an API call that silently rewrites the historical sale when policy forbids it.

---

# 151. API Response Consistency

A successful mutation response should describe the committed authoritative state or a known asynchronous state.

Do not return the client-side requested values when the server has intentionally normalized/rejected them.

Example:

```text
requested quantity = 3
actual accepted quantity = 2
```

The response reflects the actual committed result or a domain rejection; it does not mirror the request for convenience.

---

# 152. Partial Success Contract

Batch APIs must explicitly define partial-success behavior.

Do not leave clients guessing whether:

```text
10 commands
```

means:

```text
all succeeded
none succeeded
some succeeded
```

Each command receives a result or the batch contract explicitly guarantees atomic all-or-nothing semantics.

For financial batches, all-or-nothing is often preferable when domain semantics permit it.

---

# 153. Batch Atomicity

A batch endpoint must document whether operations are:

```text
ATOMIC_ALL
ATOMIC_PER_COMMAND
BEST_EFFORT
```

Financial business mutations should generally avoid ambiguous best-effort semantics.

---

# 154. API and Offline Transactions

The client may commit local operational state before server acknowledgement, but when it calls the server it must treat the server response as the authoritative reconciliation result.

The API supports deterministic command outcomes and checkpoints.

---

# 155. Stale Mobile Client Contract

A client may reconnect after days/weeks offline.

The server may respond:

```text
PROTOCOL_TOO_OLD
CLIENT_UPGRADE_REQUIRED
COMMAND_SCHEMA_UNSUPPORTED
DEVICE_REVOKED
CAPABILITY_EXPIRED
```

The server must not silently interpret a structurally old command as a new semantic operation.

---

# 156. API Security Around Device Clock

Client timestamps are evidence.

Server security decisions use server time or trusted protocol metadata.

For offline commands, both client timestamp and server receipt timestamp may be retained.

A modified phone clock must not extend a security capability indefinitely.

---

# 157. API and Device Revocation

After revocation, requests from the device must be rejected/restricted according to the offline security policy.

The API cannot rely solely on client-side logout because the device may be stolen.

---

# 158. API and Network Outages

When merchant internet connectivity is unavailable, the API is naturally unavailable to the device; the client operates within its local offline policy.

When connectivity returns:

```text
local command
→ API sync
→ authoritative validation
→ commit/dedup/conflict
→ acknowledgement
```

The API must be prepared for many delayed commands arriving in bursts.

---

# 159. Sync Burst Protection

Reconnect storms can create concentrated load.

Use:

- device concurrency limits;
- batch size limits;
- bytes-per-batch limits;
- backoff guidance;
- queue processing controls;
- fair scheduling.

Do not let one merchant device monopolize the API after a network outage.

---

# 160. API Fairness Across Tenants

Because the product serves many small merchants, resource controls must avoid a single tenant consuming disproportionate shared capacity.

Examples:

```text
per-tenant query concurrency
per-device sync concurrency
per-tenant report jobs
per-tenant export quota
per-tenant upload quota
```

These are service-protection controls, not substitutes for authorization.

---

# 161. Noisy Neighbor Contract

A tenant generating millions of historical report rows must not degrade checkout for every other tenant.

Possible strategies:

```text
read-model isolation
async reporting
concurrency pools
rate/quota limits
connection pool partitioning
```

The exact implementation is operational architecture, but the API contract must classify these workloads distinctly.

---

# 162. API Availability Tiers

Suggested workload tiers:

```text
TIER 0 — checkout / critical business mutations
TIER 1 — normal operational reads/writes
TIER 2 — integrations
TIER 3 — reports/exports
TIER 4 — admin/bulk/background
```

Failure or saturation in Tier 3/4 must not automatically consume all resources needed by Tier 0.

---

# 163. Health Endpoint Contract

Example:

```text
GET /health/live
GET /health/ready
```

Liveness answers whether the process is alive.

Readiness answers whether the instance should receive traffic.

Do not make liveness depend on every downstream dependency or a transient provider outage.

---

# 164. Dependency Health Disclosure

Internal health details are restricted.

A public readiness response should not reveal:

```text
postgres hostname
redis hostname
provider credential state
internal service names
stack traces
```

---

# 165. Metrics Contract

API metrics should use bounded labels.

Good:

```text
route_template
status_class
method
operation_class
```

Bad:

```text
raw URL
user_id
sale_id
product name
request body
```

Unbounded labels can themselves become a resource-exhaustion issue.

---

# 166. Distributed Tracing Contract

Trace context may propagate through trusted infrastructure according to the observability specification.

The trace ID is an operational correlation value, not an authorization token.

Do not put secrets or sensitive business payloads into span attributes.

---

# 167. API Security Telemetry

Emit metrics/events for:

```text
auth failures
BOLA denials
privilege denials
rate limiting
webhook signature failures
replay detections
device revoke attempts
large export creation
unusual report load
validation abuse
unexpected route access
```

Security telemetry feeds detection but must not become a substitute for enforcement.

---

# 168. Rate Limit Response Contract

Example:

```json
{
  "type": "https://api.sitolo.example/problems/rate-limited",
  "title": "Too many requests",
  "status": 429,
  "code": "RATE_LIMITED",
  "request_id": "req_123",
  "retryable": true
}
```

Potential headers:

```text
Retry-After
```

Do not expose internal limiter implementation details.

---

# 169. API Security for Object Exports

Even if `/v1/sales/{id}` is authorized, a route such as:

```text
GET /v1/sales/export
```

must not be considered safe automatically.

Bulk operations have materially higher exfiltration capability and receive separate permissions, quotas and audits.

---

# 170. API Security for Search

Autocomplete endpoints can become enumeration channels.

Examples:

```text
GET /v1/customers/search?q=...
GET /v1/products/search?q=...
```

Need:

- scope enforcement;
- bounded result count;
- minimum query length where appropriate;
- rate limiting;
- safe projections;
- no cross-tenant leakage through result counts.

---

# 171. API Security for Customers

Customer PII is tenant-scoped and minimized.

Cashiers may need a restricted projection while managers may need more details.

Bulk customer export is separately privileged.

---

# 172. API Security for Suppliers

Supplier financial/contact records require scope controls.

Supplier credential secrets are never general API fields.

---

# 173. API Security for Pharmacy

Pharmacy-sensitive operations may require specialized permissions.

Examples:

```text
view restricted medicine metadata
sell regulated medicine
approve restricted adjustment
view dispensing records
```

The generic retail API must not accidentally make regulated operations available to ordinary cashiers.

Exact regulatory permissions remain subject to current applicable requirements.

---

# 174. API Security for Agro-dealer

Agro-specific lot/formulation/traceability fields are exposed only to applicable business configurations and authorized staff.

The core API must not weaken generic inventory controls merely because an extension module exists.

---

# 175. API Security for Notifications

Notification requests are treated as commands, not generic message-sending authority.

Clients should not be allowed to forge arbitrary platform notification identities or send unrestricted messages through Sitolo infrastructure.

---

# 176. API Security for Integrations

Integration management endpoints are privileged.

Clients must not be able to activate arbitrary adapters or supply unrestricted webhook destinations without authorization and SSRF-safe validation.

---

# 177. API Security for Feature Flags

Changing feature flags can alter security/business behavior.

Therefore feature-flag mutation requires:

```text
permission
+ scope
+ audit
+ change history
```

High-risk security flags may require approval.

---

# 178. API Security for Configuration

Configuration endpoints must distinguish:

```text
merchant-operational configuration
security configuration
platform configuration
integration secrets
```

Never place all configuration under one generic mutable JSON object.

---

# 179. API and Secrets Rotation

Secret rotation APIs are not ordinary merchant features.

They belong to administrative/security operations and require strong authentication and audit.

The API should report rotation state without disclosing the secret itself.

---

# 180. API and Key Material

Where signing keys are managed by KMS/HSM infrastructure, the application should receive key handles/capabilities rather than exporting raw private keys into ordinary API memory or responses.

Clients never receive server signing keys.

---

# 181. API Content Security vs Browser Security

JSON APIs do not need HTML rendering, which helps reduce XSS exposure.

HTML/web-admin endpoints require a stricter browser security contract.

API responses containing merchant-authored strings must still be treated as potentially hostile by clients.

---

# 182. API and Source Maps

API contracts and source maps are separate concerns, but production deployments should not expose client source maps that reveal internal endpoints or configuration unless explicitly justified.

CI should inspect built web assets.

---

# 183. API Authentication Documentation

The API documentation must clearly state for every protected endpoint:

```text
authentication scheme
required scopes/permissions
required tenant scope
required device state
step-up requirement
```

A documentation page that merely says “JWT required” is insufficient for high-risk operations.

---

# 184. API Security Review Checklist

Before adding an endpoint:

```text
[ ] Business purpose defined
[ ] Resource/command identified
[ ] Owner identified
[ ] Tenant scope defined
[ ] Branch scope defined
[ ] Authentication requirement defined
[ ] Function permission defined
[ ] Object authorization defined
[ ] Property authorization defined
[ ] State transition defined
[ ] Request DTO defined
[ ] Response DTO defined
[ ] Data classification defined
[ ] Rate-limit class defined
[ ] Timeout class defined
[ ] Resource budget defined
[ ] Idempotency requirement defined
[ ] Audit requirement defined
[ ] Failure modes defined
[ ] Retry semantics defined
[ ] API inventory entry created
[ ] OpenAPI entry created
[ ] Positive tests created
[ ] Negative security tests created
[ ] Concurrency tests created if applicable
```

---

# 185. API Definition of Ready

An endpoint is **READY FOR IMPLEMENTATION** only when all of the following are known:

```text
business purpose
resource/command semantics
authorization model
tenant/branch scope
request schema
response schema
error codes
state-machine interaction
transaction boundary
idempotency behavior
failure/retry behavior
resource limits
rate limit
timeout
audit requirements
observability
OpenAPI contract
security tests
```

---

# 186. API Definition of Done

An endpoint is not complete when it returns `200` in a manual test.

It is complete when:

```text
implemented
+ authenticated correctly
+ authorized correctly
+ tenant-safe
+ object-safe
+ property-safe
+ state-safe
+ resource-bounded
+ idempotent where required
+ transactionally correct
+ audited where required
+ observable
+ documented
+ negative-tested
+ CI-enforced
+ failure-tested
```

---

# 187. Mandatory API Release Gate

For every release, CI must verify at minimum:

```text
[ ] OpenAPI parses/validates
[ ] runtime routes match inventory
[ ] authentication suite passes
[ ] tenant isolation suite passes
[ ] BOLA/IDOR suite passes
[ ] function authorization suite passes
[ ] property authorization suite passes
[ ] input validation suite passes
[ ] injection suite passes
[ ] rate-limit tests pass
[ ] timeout tests pass
[ ] idempotency tests pass
[ ] webhook tests pass
[ ] concurrency tests pass
[ ] DAST critical/high checks pass
[ ] no secrets leaked in API responses/log fixtures
[ ] error contract tests pass
```

A missing or failed security test is a failed gate, not a warning.

---

# 188. API Testing Pyramid

```text
                 DAST / adversarial
                       /\
                      /  \
                 contract tests
                    /      \
              application integration
                /            \
          repository / DB integration
             /                \
       property / domain tests
          /                    \
              unit tests
```

No layer replaces another.

---

# 189. API Test Categories

## Unit

- DTO validation
- domain command mapping
- error mapping
- permission predicates

## Integration

- real PostgreSQL
- RLS
- transaction semantics
- idempotency records
- outbox

## Contract

- OpenAPI request/response compatibility
- generated client compatibility

## Security

- BOLA
- privilege escalation
- mass assignment
- injection
- replay
- SSRF

## Performance

- concurrency
- pool saturation
- pagination
- report isolation

## Chaos/failure

- DB unavailable
- provider timeout
- process restart after commit
- duplicate webhook
- network disconnect

---

# 190. Production Smoke Tests

After deployment:

```text
GET /health/live
GET /health/ready
authenticated /me
read authorized product
attempt cross-tenant product
create test sale in controlled environment
verify audit
verify outbox
verify error contract
verify security headers
verify rate limiter
```

Production smoke tests must use safe tenant/test data and must not alter real merchant financial state without an explicit controlled procedure.

---

# 191. Rollback Contract

A release rollback must not leave API and database semantics incompatible.

Therefore migrations should follow the expand/contract strategy and deployments should preserve backward compatibility long enough to roll application versions safely.

A rollback must be tested, not assumed.

---

# 192. API and Database Migration Contract

An API deployment cannot depend on a schema change that has not yet reached the required compatibility stage.

Preferred sequence:

```text
EXPAND
  ↓
backward-compatible application
  ↓
backfill / migrate
  ↓
SWITCH
  ↓
remove old path later
```

Do not couple an emergency application rollback to an irreversible database mutation without a recovery plan.

---

# 193. API and Read Models

Reporting/search endpoints may read projections optimized for their purpose, but the projection must remain traceable to authoritative records.

A stale report can be labeled stale/pending where applicable; it must not be presented as a replacement financial ledger.

---

# 194. API and Financial Correctness

The API is successful only when the business state remains correct under:

```text
concurrent calls
network retries
offline command replay
provider duplicates
provider outages
client tampering
partial failures
process restarts
```

This is the actual correctness bar.

---

# 195. API and Inventory Correctness

The API must prevent:

```text
negative stock where policy forbids it
stock creation through client mutation
duplicate consumption
double transfer
cross-location unauthorized movement
sale against forbidden lot/state
```

Atomic SQL + domain state checks are preferred over application memory locks.

---

# 196. API and Cash Correctness

Cash API operations preserve expected/counting evidence and do not erase variance.

A cashier cannot fabricate a clean close by posting arbitrary expected values.

The server derives expected cash from authoritative cash events.

---

# 197. API and Payment Reconciliation

API clients can observe payment state, request allowed actions and perform approved reconciliation tasks.

They cannot declare a payment settled without authoritative evidence.

---

# 198. API and Tax Evidence

Tax API outputs distinguish:

```text
commercial sale state
vs
external tax submission state
```

A tax submission failure does not become a financial edit primitive.

---

# 199. API and Support Access

Every support-data read should be attributable to:

```text
support actor
customer tenant
purpose/case
scope
start/end
result
```

High-risk support access should require approval and expiration.

---

# 200. API Anti-Patterns Prohibited

The following are explicitly prohibited:

```text
Generic CRUD over every table
Client-controlled tenant authority
Client-controlled payment settlement
Client-controlled approval flags
Generic PATCH over protected aggregates
Raw SQL in handlers
Unbounded collection endpoints
Unbounded report endpoints
Blind retries of financial POSTs
GET endpoints that mutate state
Admin route protected only by obscurity
Wildcard CORS with credentials
Arbitrary URL fetching
User filenames as filesystem paths
Raw provider payload as internal truth
Returning stack traces
Logging access tokens
Returning source-code/config secrets
Assuming a device ID is a credential
Assuming CORS is authorization
Assuming opaque IDs prevent IDOR
```

---

# 201. API Security Traceability to the 48 Sitolo Controls

The API layer is the primary enforcement surface for many of the previously defined controls.

| Control | API responsibility |
|---|---|
| Exposed DB credentials | never expose connection details |
| Public `.env` | never expose configuration routes/files |
| Hardcoded secrets | no secrets in API code/config artifacts |
| Weak Auth | authenticate all protected endpoints |
| Missing authz | explicit permission contract |
| Cross-user access | object/function/property checks |
| Open DB permissions | API uses least-privilege DB path |
| Cloud misconfiguration | safe deployment/health behavior |
| Unprotected admin | explicit admin authz |
| Debug tools | restricted/removal in prod |
| Logs leak secrets | redaction |
| Verbose errors | RFC 9457 safe errors |
| Secrets in git | CI scanning |
| Client-only security | server authority |
| Input validation | bounded DTOs |
| SQLi | parameterized queries |
| NoSQL injection | typed query layer |
| XSS | safe data/API output |
| CSRF | cookie-authenticated browser endpoints protected |
| File uploads | dedicated validated pipeline |
| Path traversal | generated object identities |
| SSRF | constrained outbound access |
| Password reset | abuse-resistant auth API |
| Sessions | server lifecycle/revocation |
| JWT secrets | validated keys/signatures, no client secret exposure |
| CORS | explicit origin policy |
| Rate limits | multi-dimensional limits |
| Exposed environments | endpoint inventory/exposure classes |
| Default creds | production auth policy |
| Unsigned webhooks | provider auth/signature checks |
| FE payment checks | backend settlement authority |
| IDOR/BOLA | object authorization |
| APIs + user input | schema/security validation |
| Exposed logs | restricted telemetry |
| Source maps | build artifact controls |
| MFA | high-risk endpoint enforcement |
| Enumeration | safe responses |
| Business logic abuse | state machines/invariants |
| Race conditions | transactions/locks/idempotency |
| Webhook replay | event identity + dedup |
| Insecure CI/ID | release route protected by deployment controls |
| Untrusted build actions | CI boundaries |
| Unpinned deps | build policy |
| Fail-open checks | CI/release behavior |
| Missing timeouts | endpoint budgets |
| Sensitive browser storage | auth/session contract |
| Insecure endpoints | route inventory + tests |

---

# 202. Route Contract Template

Every new endpoint should document:

```yaml
operation_id: finalizeSale
method: POST
path: /v1/sales/{sale_id}/finalize
classification: FINANCIAL

authentication:
  required: true
  session_required: true
  device_required_for_offline: optional

authorization:
  permission: SALE_FINALIZE
  tenant_scope: organization
  branch_scope: sale.branch
  object_scope: sale
  property_scope: command-schema
  step_up: false

request:
  body: FinalizeSaleRequest
  max_body_bytes: configured
  idempotency: required

response:
  success: SaleResponse
  errors:
    - AUTHENTICATION_FAILED
    - INSUFFICIENT_PERMISSION
    - RESOURCE_NOT_FOUND
    - STATE_CONFLICT
    - IDEMPOTENCY_KEY_REUSED
    - INVENTORY_INSUFFICIENT_STOCK

limits:
  rate_class: POS_MUTATION
  timeout_class: CRITICAL_MUTATION

side_effects:
  audit: required
  outbox: required
  external_call_in_transaction: prohibited

tests:
  positive: required
  tenant_negative: required
  role_negative: required
  idempotency: required
  concurrency: required
```

This template is governance, not necessarily the literal final OpenAPI format.

---

# 203. API Change Review Template

Every API PR should answer:

```text
What business operation changed?
What resource/state machine changed?
What authorization changed?
What tenant scopes are affected?
What fields are newly readable/writable?
What is the compatibility impact?
What are retry/idempotency semantics?
What are failure modes?
What are the resource limits?
What security tests were added?
What OpenAPI changed?
What client versions are impacted?
What migration is required?
How is rollback handled?
```

---

# 204. API Architecture Decision Records

API-specific ADRs should include:

```text
ADR-026 API versioning policy
ADR-027 Problem Details error contract
ADR-028 OpenAPI governance
ADR-029 Idempotency contract
ADR-030 Cursor pagination policy
ADR-031 API security middleware order
ADR-032 Route exposure classification
ADR-033 API resource budgeting
ADR-034 Async job/report contract
ADR-035 API compatibility window
```

These complement the existing 25 core architecture/security ADRs.

---

# 205. Open Decisions

The following remain intentionally evidence-driven:

1. Final public API host/domain.
2. Exact identity provider integration.
3. Exact JWT/session architecture.
4. Exact JSON schema/OpenAPI toolchain.
5. Final mobile API compatibility window.
6. Exact rate-limit technology and Redis topology.
7. Exact report job orchestration mechanism.
8. Exact MRA endpoint mapping after current certification/access.
9. Exact payment-provider webhook contracts.
10. Exact object-storage upload mechanism.
11. Exact API gateway/WAF product.
12. Exact HTTP/2/HTTP/3 deployment support.

These must not be invented as facts. They become implementation decisions when the corresponding infrastructure/provider evidence exists.

---

# 206. Research Basis

This contract is based on:

- Sitolo's existing product/domain architecture: mobile-first, offline-capable, multi-tenant operation; Rust as business authority; PostgreSQL as server authority; Flutter as primary mobile surface; Tauri for desktop; and a modular monolith with workers/adapters.
- Sitolo's existing domain model: bounded contexts, explicit aggregates, inventory ledger semantics, append-only financial facts, payment intents, reconciliation cases, sync commands and approval boundaries.
- Sitolo's existing security architecture and implementation specification: zero-trust, deny-by-default, server-side authorization, BOLA/property/function controls, mandatory security tests, bounded resources, secure error handling and fail-closed release gates.
- OWASP API Security Top 10 2023 for object-level authorization, authentication, property/function authorization, resource consumption, business-flow abuse, SSRF, misconfiguration, endpoint inventory and unsafe API consumption.
- RFC 9110 for HTTP semantics and idempotent method behavior.
- RFC 9457 for standardized problem-details error representation.
- JSON Schema Draft 2020-12 as the current JSON Schema baseline.
- Current PostgreSQL/database contract documented in `database_design.md`.

The API contract deliberately does not replace application/domain/database controls. It provides the external contract through which those controls become observable and testable.

---

# 207. Final API Contract

The Sitolo API contract can be reduced to this model:

```text
                         UNTRUSTED INPUT
                                |
                                v
                       HTTP / JSON / WEBHOOK
                                |
                                v
                       SIZE / RATE / TIMEOUT
                                |
                                v
                         AUTHENTICATION
                                |
                                v
                     SESSION / DEVICE STATE
                                |
                                v
                         TENANT CONTEXT
                                |
                                v
                   FUNCTION AUTHORIZATION
                                |
                                v
                     SCHEMA VALIDATION
                                |
                                v
                    OBJECT AUTHORIZATION
                                |
                                v
                   PROPERTY AUTHORIZATION
                                |
                                v
                  BUSINESS STATE / RULES
                                |
                                v
                    IDEMPOTENCY / REPLAY
                                |
                                v
                     POSTGRES TRANSACTION
                         /           \
                        /             \
                     AUDIT           OUTBOX
                        \             /
                         \           /
                          v         v
                         COMMITTED STATE
                                |
                                v
                         SAFE RESPONSE
```

The implementation standard is:

> **Every API request is an untrusted attempt to invoke a business capability. The server establishes identity, derives scope, verifies authority, validates bounded input, verifies object/property/state invariants, applies duplicate/replay protection, commits authoritative state transactionally, records required evidence, and returns only the minimum safe representation of the resulting state.**

For Sitolo, that is the difference between an API that merely works and an API that can safely operate a real business system under unreliable networks, concurrent users, staff mistakes, malicious clients, provider failures and enterprise growth.

---

# Appendix A — Initial API Resource Matrix

| Resource | Read | Create | Update | Delete | Command-oriented actions | Sensitive |
|---|---:|---:|---:|---:|---|---:|
| Organization | ✓ | ✓ | ✓ | restricted | ownership transfer | High |
| Branch | ✓ | ✓ | ✓ | lifecycle | activate/deactivate | High |
| Membership | ✓ | ✓ | ✓ | lifecycle | suspend/restore | High |
| Device | ✓ | ✓ | limited | no | revoke | High |
| Product | ✓ | ✓ | ✓ | archive | activate/discontinue | Medium |
| SKU | ✓ | ✓ | limited | archive | activate/discontinue | Medium |
| Price | ✓ | ✓ | versioned | no destructive edit | supersede | Medium |
| Promotion | ✓ | ✓ | controlled | lifecycle | activate/deactivate | Medium |
| Supplier | ✓ | ✓ | ✓ | archive | lifecycle | Medium |
| Purchase Order | ✓ | ✓ | limited | no | submit/approve/cancel | High |
| Goods Receipt | ✓ | ✓ | limited | no | post/reverse where defined | High |
| Inventory Position | ✓ | no | no direct | no | adjust/transfer/count | High |
| Inventory Movement | ✓ | no | no | no | reversal only where defined | High |
| Stock Count | ✓ | ✓ | lifecycle | no | submit/approve/post | High |
| Sale | ✓ | ✓ | restricted | no | finalize/void/return | Critical |
| Return | ✓ | ✓ | restricted | no | approve/post | Critical |
| Refund | ✓ | ✓ | lifecycle | no | approve/execute | Critical |
| Register Session | ✓ | ✓ | lifecycle | no | open/close | High |
| Cash Event | ✓ | create | no | no | reverse/correct | Critical |
| Payment Intent | ✓ | ✓ | lifecycle | no | cancel/reconcile | Critical |
| Provider Event | privileged | ingestion | no | no | replay/reconcile | Critical |
| Reconciliation Case | ✓ | ✓ | controlled | no | resolve | Critical |
| Tax Submission | ✓ | controlled | lifecycle | no | retry/reconcile | High |
| Report | ✓ | no | no | no | execute | Medium |
| Export | ✓ | ✓ | lifecycle | expiry | cancel/download | High |
| Subscription | ✓ | controlled | controlled | no | upgrade/downgrade | High |
| Audit Event | restricted | system | no | no | none | Critical |
| Support Grant | privileged | ✓ | no | revoke | grant/revoke | Critical |

---

# Appendix B — Initial Error Contract Matrix

| Error | HTTP | Retry | Typical cause |
|---|---:|---|---|
| AUTHENTICATION_FAILED | 401 | after credential refresh | invalid/expired auth |
| SESSION_REVOKED | 401 | no | revoked session |
| DEVICE_REVOKED | 401/403 | no | revoked device |
| TENANT_ACCESS_DENIED | 403/404 | no | wrong scope |
| INSUFFICIENT_PERMISSION | 403 | no | missing capability |
| RESOURCE_NOT_FOUND | 404 | no | absent/inaccessible object |
| VALIDATION_ERROR | 422 | after correction | malformed/invalid business input |
| INVALID_IDENTIFIER | 400 | no | malformed ID |
| STATE_CONFLICT | 409 | after refresh/state change | illegal current-state transition |
| STALE_RESOURCE | 412/409 | after refresh | version mismatch |
| IDEMPOTENCY_KEY_REUSED | 409 | no with same changed payload | key collision |
| RATE_LIMITED | 429 | yes after delay | quota exhausted |
| REQUEST_TOO_LARGE | 413 | no | resource bound exceeded |
| INVENTORY_INSUFFICIENT_STOCK | 409 | after inventory refresh | concurrency/business conflict |
| REFUND_EXCEEDS_ELIGIBLE_AMOUNT | 409 | no | business violation |
| PAYMENT_STATE_CONFLICT | 409 | reconcile | incompatible payment state |
| WEBHOOK_SIGNATURE_INVALID | 401/403/400 | provider-defined | forged/invalid webhook |
| WEBHOOK_REPLAY | 409 | no | duplicate event |
| PAYMENT_PROVIDER_UNAVAILABLE | 503 | yes | provider outage |
| DEPENDENCY_UNAVAILABLE | 503 | endpoint-specific | downstream outage |
| TAX_SUBMISSION_REJECTED | 409/422 | only when retryable | external tax rejection |
| RECONCILIATION_REQUIRED | 409 | manual/async | ambiguous external evidence |
| FEATURE_NOT_ENABLED | 403/404 | no | disabled feature |
| ENTITLEMENT_REQUIRED | 403 | after plan change | subscription constraint |
| INTERNAL_ERROR | 500 | endpoint-specific | unexpected failure |

---

# Appendix C — Initial Security Gate Matrix

```text
ROUTE ADDED
    |
    +--> OpenAPI entry?
    +--> Inventory entry?
    +--> AuthN contract?
    +--> AuthZ contract?
    +--> Tenant contract?
    +--> Object-level negative test?
    +--> Property-level test?
    +--> Input bounds?
    +--> Idempotency?
    +--> Rate/timeout budgets?
    +--> Audit?
    +--> Observability?
    +--> Failure tests?
    +--> Concurrency tests?
    +--> CI enforcement?
              |
              v
           RELEASE
```

Any missing mandatory item blocks the endpoint from being considered implementation-complete.

---

# Appendix D — Example Sale Request/Response

## Request

```http
POST /v1/sales
Authorization: Bearer <access-token>
Idempotency-Key: 01J...
Content-Type: application/json
X-Request-ID: optional-client-correlation
```

```json
{
  "location_id": "loc_01J...",
  "register_id": "reg_01J...",
  "items": [
    {
      "sku_id": "sku_01J...",
      "quantity": "2",
      "unit": "piece"
    }
  ]
}
```

## Response

```json
{
  "sale": {
    "id": "sale_01J...",
    "sale_number": "S-2026-000123",
    "status": "FINALIZED",
    "currency": "MWK",
    "total_amount_minor": 50000,
    "payment_status": "PENDING"
  },
  "request_id": "req_01J..."
}
```

Notice that the response does not blindly echo a client-supplied total or payment state.

---

# Appendix E — Example Cross-Tenant Attack

Attacker possesses:

```text
sale_id = valid victim sale ID
```

Request:

```http
GET /v1/sales/sale_VICTIM
Authorization: Bearer attacker-token
```

Required processing:

```text
authenticate attacker
→ resolve attacker memberships
→ target object lookup constrained to attacker scope
→ no authorized object found
→ safe 404/authorization response
→ no sale fields disclosed
→ security telemetry recorded where policy requires
```

The attacker must not receive:

```text
sale customer
sale total
sale timestamp
branch
product lines
payment method
```

---

# Appendix F — Example Payment Tampering Attack

Attacker sends:

```json
{
  "paid": true,
  "amount": 1,
  "currency": "MWK",
  "provider_transaction_id": "victim-provider-id"
}
```

The API must ignore the client's authority over settlement and instead verify:

```text
authoritative payment intent
expected amount
currency
provider evidence
provider transaction identity
webhook signature / trusted provider API
replay state
internal payment state
```

The frontend cannot manufacture settlement.

---

# Appendix G — Example Idempotent Retry

```text
1. Mobile client submits sale with key K.
2. Server validates and commits sale S.
3. Network drops before response.
4. Client retries same request with K.
5. Server finds completed idempotency record.
6. Server returns outcome for S.
7. No second inventory decrement.
8. No second sale.
9. No second financial posting.
10. Audit may record the duplicate request attempt separately.
```

---

# Appendix H — Example Concurrency Attack

Initial stock:

```text
SKU-X = 1
```

Two clients concurrently submit:

```text
A: sell 1
B: sell 1
```

The API/database contract requires:

```text
one succeeds
one receives INVENTORY_INSUFFICIENT_STOCK or an equivalent concurrency-safe conflict
```

The result must not be:

```text
stock = -1
```

unless the business has explicitly chosen a negative-stock policy.

---

# Appendix I — Example Offline Replay

```text
Device creates cmd_123
Device queues cmd_123
Device restarts
Device retries cmd_123
Device reconnects
Server processes cmd_123
Network drops after commit
Device retries cmd_123 again
```

The final state is one business effect.

---

# Appendix J — Implementation Checklist

```text
[ ] Axum application skeleton
[ ] route registry
[ ] OpenAPI source/spec
[ ] request/response DTO conventions
[ ] RFC 9457-compatible error model
[ ] request ID middleware
[ ] authentication extractor
[ ] session/device checks
[ ] tenant context extractor
[ ] authorization service integration
[ ] validation framework
[ ] body limits
[ ] header limits
[ ] rate limiting
[ ] timeout middleware
[ ] CORS
[ ] CSRF for cookie-authenticated browser paths
[ ] security headers
[ ] idempotency subsystem
[ ] cursor pagination
[ ] error taxonomy
[ ] audit integration
[ ] tracing integration
[ ] route inventory
[ ] endpoint security metadata
[ ] test fixture factory
[ ] cross-tenant test harness
[ ] BOLA test helpers
[ ] webhook test helpers
[ ] concurrency test helpers
[ ] API fuzzing hooks
[ ] DAST staging target
[ ] OpenAPI contract CI
[ ] runtime route vs inventory check
```

---

# Appendix K — Final API Engineering Standard

The standard for Sitolo's API is not:

> “Can a client call the endpoint?”

The standard is:

> **Can an authenticated or unauthenticated actor attempt every plausible abuse path against the endpoint and still have the server preserve tenant boundaries, authorization, business invariants, financial integrity, inventory integrity, resource limits, auditability and recoverability?**

The answer must be demonstrable through code, tests and operational evidence.

---

# Document Completion Status

**Phase:** 0 — Architecture / Contracts / ADR Freeze  
**File:** 04 of 16  
**Primary deliverable:** `api_contract.md`  
**Next file:** `auth_authorization_spec.md`  
**Implementation dependency:** `database_design.md` + `domain_model.md` + `security_implementation_spec.md`  
**Release principle:** API security tests are mandatory and release-blocking  
**Authority:** Sitolo API implementation contract

**END OF DOCUMENT**
