# Sitolo — Phase 10 POS / Sales Implementation Specification

**Document:** `phase10_pos_sales_implementation.md`  
**Phase:** 10 — POS / Sales  
**Status:** Implementation-governing specification  
**Revision:** 1.0  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African expansion  
**Backend:** Rust + Axum + Tokio  
**Persistence:** PostgreSQL authoritative server state; SQLite client operational continuity  
**Clients:** Flutter Android-first, Tauri desktop, limited TypeScript web/admin  
**Architecture:** Modular monolith first; workers and integration adapters; selective service extraction only when justified

---

## 0. Executive Position

Phase 10 turns the previously frozen product, identity, tenant, authorization, database, catalogue and inventory contracts into the authoritative **sales/POS execution subsystem**.

The POS subsystem is not a shopping-cart CRUD feature. It is a financially significant command system that coordinates catalogue resolution, price calculation, taxes, discounts, customer context, inventory availability, tender collection, cash/register state, idempotency, audit evidence and downstream integration intent.

The architectural proposition is:

> **A sale is a stateful business transaction whose commercial facts become authoritative only through a server-validated command and an atomic PostgreSQL transaction.**

The client may construct a draft, scan products, present prices, calculate provisional totals and continue operating locally. It must not establish final financial truth merely because it has a locally calculated total or because a UI state says `completed`.

The authoritative path is:

```text
CLIENT
  |
  | authenticated command / sync command
  v
EDGE / API
  |
  +--> authenticate session/device
  +--> derive tenant context
  +--> authorize command
  +--> validate request
  v
SALES APPLICATION SERVICE
  |
  +--> resolve SKU / barcode
  +--> resolve applicable price policy
  +--> resolve tax policy
  +--> validate sale state
  +--> validate customer/vertical restrictions
  +--> validate register/session
  +--> validate tender constraints
  +--> calculate authoritative totals
  +--> reserve/consume inventory according to command
  +--> persist sale + immutable financial snapshot
  +--> persist payment linkage
  +--> persist audit event
  +--> persist outbox intent
  +--> persist idempotency result
  v
POSTGRESQL COMMIT
  |
  +--> sale becomes authoritative
  +--> inventory effects become authoritative
  +--> payment intent/result state becomes authoritative
  +--> downstream work becomes durable
  |
  v
WORKERS / INTEGRATIONS / REPORTING
```

This phase deliberately does **not** collapse payment-provider confirmation, tax/EIS submission, inventory projection maintenance or reporting into one giant request/transaction. Those boundaries remain explicit. The sale transaction establishes internal truth and durable intent; external systems are reconciled through adapters and workers.

---

# 1. Relationship to Existing Sitolo Contracts

This document is subordinate to the established Sitolo contract hierarchy.

```text
LAW / REGULATION / EXTERNAL CONTRACT
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
AUTHENTICATION / AUTHORIZATION
                |
                v
DATABASE DESIGN
                |
                v
API / SYNC / PAYMENT / EIS CONTRACTS
                |
                v
PHASE 8 CATALOGUE
                |
                v
PHASE 9 INVENTORY LEDGER
                |
                v
THIS PHASE — POS / SALES
                |
                v
RUST CODE / SQL / TESTS
```

The existing domain model already defines the organization and branch hierarchy, Product → SKU semantics, units, pricing boundaries, authorization tuple and high-risk operations. The inventory subsystem is ledger-backed and authoritative in PostgreSQL. The API contract requires server-side authorization, typed inputs, explicit commands, idempotency and release-blocking negative security tests. These are prerequisites rather than optional implementation details.

The POS implementation MUST NOT introduce:

- a second concept of product or SKU;
- a mutable `inventory_balance` as the authoritative stock source;
- a client-controlled final total;
- a second tenant/organization model;
- payment-provider-specific fields directly into core sale semantics unless explicitly abstracted;
- direct external API calls inside the core sale transaction;
- mutable historical facts that can silently rewrite finalized sales;
- generic `PATCH /sales/{id}` mutation of finalized financial state;
- a separate client-side authority model for offline sales;
- an authorization bypass because POS routes are considered “internal” or “merchant-only”.

---

# 2. Why POS / Sales Is a Security-Critical Domain

A POS operation combines several high-impact assets in one workflow:

```text
money
stock
customer data
cash
pricing authority
tax evidence
payment references
staff permissions
business history
```

A compromise can therefore produce more than data disclosure. It can directly create financial loss, inventory disappearance, fraudulent refunds, unauthorized discounts, cash variance, tax inconsistencies, payment mismatches and audit gaps.

The POS subsystem must defend against both malicious and accidental misuse.

Examples:

```text
cashier → unauthorized discount
cashier → cross-branch sale lookup
cashier → forged sale total
cashier → replay FinalizeSale
cashier → refund an already refunded sale
stale device → replay offline command
attacker → change SKU identifier to another tenant
attacker → submit unauthorized price override
attacker → manipulate quantity / unit conversion
attacker → race two sales against one final stock unit
attacker → reuse payment reference
attacker → enumerate customer records through search
operator → mutate historical financial fact directly
```

The implementation must therefore combine application authorization, domain state validation, PostgreSQL transactional guarantees, inventory ledger rules, immutable financial evidence, idempotency and security tests.

OWASP identifies broken object-level authorization and broken function-level authorization as major API risks. Every endpoint that acts on a sale ID, payment ID, customer ID, register ID or inventory object must validate authorization for the specific operation rather than relying only on authentication or a generic role. citeturn229775search1turn229775search8

---

# 3. POS Domain Scope

## 3.1 Core responsibilities

Phase 10 owns:

1. sale draft semantics;
2. sale line composition;
3. price resolution and snapshotting;
4. discount and override policy invocation;
5. tax calculation orchestration;
6. customer association where applicable;
7. register and cashier context;
8. sale lifecycle state transitions;
9. finalization;
10. cancellation/void semantics where legally and operationally permitted;
11. return/refund initiation boundary where the separate Phase 14 rules apply;
12. tender composition and payment linkage;
13. transaction-level idempotency;
14. sale audit evidence;
15. outbox events;
16. offline command acceptance boundaries;
17. read models required for POS performance.

## 3.2 Explicitly owned elsewhere

```text
IDENTITY / SESSION        → Phase 3
TENANT / BRANCH / IAM     → Phase 4
AUTHORIZATION ENGINE      → Phase 6
SECURITY TEST FRAMEWORK   → Phase 7
PRODUCT / SKU              → Phase 8
INVENTORY LEDGER           → Phase 9
PAYMENT PROVIDERS          → Phase 11
SYNC PROTOCOL              → Phase 12
RETURNS / REFUNDS / CASH  → Phase 14
MRA EIS                    → Phase 15
REPORTING                  → Phase 16
BILLING                    → Phase 17
```

The boundaries must be explicit. POS orchestration invokes these modules through typed application interfaces; it must not reach into their persistence tables as an architectural shortcut.

---

# 4. Canonical Sale Lifecycle

The sale lifecycle is explicit and state-machine driven.

A baseline lifecycle is:

```text
DRAFT
  |
  +--> OPEN / EDITABLE
  |
  v
READY_FOR_FINALIZATION
  |
  +--> FINALIZING
  |       |
  |       +--> COMPLETED
  |       +--> REJECTED
  |
  +--> CANCELLED
```

Where the product requires payment-pending semantics, the lifecycle may include:

```text
DRAFT
  ↓
CHECKOUT_PENDING
  ↓
PAYMENT_PENDING
  ↓
PAID / PARTIALLY_PAID / PAYMENT_EXCEPTION
  ↓
FINALIZING
  ↓
COMPLETED
```

The exact payment states are owned by the payment contract, but the sale state machine must never infer payment success merely from an HTTP request returning successfully.

## 4.1 State rules

### DRAFT

A draft may be edited within the authorized scope.

A draft MUST NOT be treated as final financial truth.

### READY_FOR_FINALIZATION

All locally required commercial inputs are present and structurally valid. This is still not final authority.

### FINALIZING

The application is executing authoritative validation and transaction logic.

No public API should expose a mutable `FINALIZING` object as if it were a normal user-editable state.

### COMPLETED

A completed sale has immutable commercial facts except through explicitly defined compensating workflows.

The final sale snapshot must include enough evidence to reconstruct why the total exists.

### CANCELLED

Cancellation is permitted only according to state and role policy. A completed sale must not be “cancelled” through a destructive update; any post-completion reversal must use explicit domain workflows.

### REJECTED

Finalization failures must not leave a partially applied sale, inventory deduction or payment linkage unless the domain explicitly defines a durable intermediate state.

---

# 5. Sale Aggregate and Data Model

The conceptual aggregate is:

```text
Sale
├── identity
│   ├── sale_id
│   ├── organization_id
│   ├── branch_id
│   ├── register_id
│   ├── cashier_membership_id
│   ├── device_id
│   └── session_id
│
├── lifecycle
│   ├── state
│   ├── created_at
│   ├── finalized_at
│   └── cancellation metadata
│
├── commercial snapshot
│   ├── currency
│   ├── subtotal
│   ├── discount_total
│   ├── tax_total
│   ├── grand_total
│   ├── rounding
│   └── calculation_version
│
├── lines[]
│   ├── sku_id
│   ├── quantity
│   ├── unit
│   ├── unit_price
│   ├── discount allocation
│   ├── tax allocation
│   ├── net amount
│   └── catalogue snapshot references
│
├── customer context
│
├── tender/payment linkage
│
├── inventory effect references
│
├── authorization evidence
├── audit metadata
└── idempotency metadata
```

The implementation must distinguish:

```text
current catalogue configuration
        ≠
historical sale snapshot
```

Changing a product name or price after a sale MUST NOT change the meaning of the historical sale.

---

# 6. Sale Identity and Idempotency

Every sale command has a stable client-visible command identity.

Recommended identifiers:

```text
SaleId
CommandId
IdempotencyKey
ClientOperationId
DeviceId
SessionId
```

They serve different purposes and must not be collapsed merely because all of them are UUID-shaped.

## 6.1 Command idempotency invariant

For every retryable mutation:

```text
same authenticated principal
+ same tenant scope
+ same idempotency key
+ same command semantic payload
→ same authoritative result
```

A reused idempotency key with materially different payload is an error.

The application must not interpret:

```text
idempotency_key = "abc"
first payload = create sale for 10,000 MWK
second payload = create sale for 50,000 MWK
```

as a new valid request.

## 6.2 Idempotency persistence

The idempotency record must be persisted atomically with the operation it protects.

```text
BEGIN
  validate authorization
  validate command fingerprint
  acquire required locks
  create/update sale
  apply inventory effect
  persist payment linkage
  persist audit event
  persist outbox
  persist idempotency completion
COMMIT
```

A successful response without a durable idempotency result is not sufficient for a retriable financial command.

---

# 7. Draft Sales vs Finalized Sales

The implementation must explicitly separate edit-friendly draft state from immutable finalized facts.

## 7.1 Draft characteristics

Drafts may be mutable because they have not yet become final commercial truth.

Examples:

```text
change quantity
remove line
change customer
apply eligible discount
switch payment method
```

However, even draft mutation is authorized and validated. A cashier cannot use the draft state as an unrestricted mutation surface for privileged properties.

## 7.2 Finalized characteristics

Once finalized:

```text
quantity snapshot
price snapshot
discount snapshot
tax snapshot
currency
rounding
payment linkage
inventory effect linkage
actor
register
branch
```

become historical facts.

The current Product or Pricing record must never be consulted later to silently recompute the historical sale.

---

# 8. Sale Line Semantics

Every sale line requires:

```text
SKU
sell unit
quantity
resolved unit price
currency
line discount
line tax
net line amount
```

The quantity is a typed domain value consistent with Phase 8 unit rules and Phase 9 inventory conversions.

A client cannot submit:

```text
sku_id = A
unit = KG
conversion = 1.0
```

and force the server to trust its conversion. The authoritative conversion comes from the catalogue/unit model.

## 8.1 Quantity conversion

The flow is:

```text
client sell quantity
      ↓
validate requested unit
      ↓
resolve authoritative conversion
      ↓
convert to inventory base unit
      ↓
validate precision / bounds
      ↓
apply inventory effect
```

The conversion must be deterministic and must reject:

- zero conversion factors;
- negative quantities where not permitted;
- impossible precision;
- unit mismatch;
- unsupported sale units;
- overflow/underflow;
- NaN/infinite floating-point inputs.

For monetary and inventory quantities, exact integer minor units or exact decimal representations must be used instead of binary floating point where exactness matters.

---

# 9. Barcode and SKU Resolution

POS must support low-friction product lookup without turning search into an authorization bypass or enumeration channel.

Typical flows:

```text
barcode scan
   ↓
lookup barcode within tenant scope
   ↓
resolve SKU
   ↓
resolve current availability / sellability
   ↓
resolve price
   ↓
add line
```

The server must scope barcode lookup by authenticated organization context.

A global barcode collision or lookup must not accidentally cross tenant boundaries.

The same security model applies to:

```text
GET /products/search
GET /products/{id}
POST /sales/{sale_id}/lines
```

OWASP warns that object IDs supplied by clients must always receive object-level authorization; UUIDs do not eliminate the need for authorization. citeturn229775search1

---

# 10. Price Resolution

POS does not trust client-submitted prices as authoritative.

The client may send an advisory price or an offline snapshot, but server finalization must resolve or verify the effective price policy.

Canonical flow:

```text
SKU
 ↓
price context
 ├── organization
 ├── branch
 ├── channel
 ├── customer context
 ├── quantity / unit
 ├── effective date/time
 └── promotional conditions
 ↓
authorized price resolver
 ↓
final authoritative unit price
```

The result must be snapshotted into the sale.

## 10.1 Price override

Price override is not equivalent to ordinary sale creation.

A policy may require:

```text
normal sale
→ cashier allowed

small override
→ cashier + bounded threshold

large override
→ manager approval

high-risk override
→ step-up + approval
```

The authorization engine must decide capability; the domain must independently validate price-policy invariants.

A cashier obtaining `SALE_CREATE` must not implicitly obtain unlimited `PRICE_OVERRIDE` authority.

---

# 11. Discount Semantics

Discounts must be modeled explicitly rather than represented as arbitrary negative price edits.

A discount should capture:

```text
discount_type
rule/source
amount or rate
scope
applied_at
actor
approval reference if required
```

Examples:

```text
line percentage discount
line fixed discount
basket promotion
authorized manager override
campaign/promotion rule
```

The domain must enforce:

```text
discount >= 0
net >= 0 unless explicitly supported
currency matches
rate within configured bounds
fixed amount <= eligible amount
```

A client must not be allowed to submit hidden discount fields through generic JSON binding. OWASP specifically highlights mass-assignment/property-level authorization risk. citeturn229775search2

---

# 12. Tax Calculation

Tax calculation is a domain/policy responsibility, not a frontend arithmetic exercise.

The POS service receives:

```text
item classification
price
quantity
currency
customer/tax context
branch/legal entity context
applicable tax policy version
```

It produces:

```text
taxable base
tax components
rounding
final tax total
calculation evidence
```

The exact rates and legal treatment remain governed by the relevant configuration and MRA EIS phase.

Once a sale is finalized, the applied tax calculation facts must be preserved as part of the historical record.

The implementation must not silently recalculate old sales using a new tax policy version.

---

# 13. Monetary Arithmetic

Money must use a representation that is exact for the currency and calculation policy.

For MWK-style whole-unit operations, integer minor-unit semantics may be appropriate; if fractional currencies or tax computations require decimal precision, use an exact decimal strategy.

The following are mandatory properties:

```text
addition is deterministic
subtraction is deterministic
rounding is deterministic
currency cannot be implicitly mixed
sign rules are explicit
overflow is detected
```

A sale must be explainable:

```text
line gross
- line discounts
= taxable base
+ tax
± explicit rounding
= final total
```

The client-provided total is advisory only.

---

# 14. Register and Cashier Context

The register is an independent business/control entity from the employee.

Canonical execution context:

```text
Authenticated User
      +
Membership
      +
Branch Scope
      +
Device
      +
Register
      +
Open Register Session
      =
Permitted POS Context
```

The system must validate:

- the device belongs to the organization;
- the device is active;
- the session is active;
- the register belongs to the branch;
- the register is associated with the device according to policy;
- the user has permission for the register/branch;
- the cashier session has not been revoked or closed;
- the command's branch is consistent with authoritative context.

A client-supplied `register_id` cannot override an authenticated device/register binding.

---

# 15. Sale Finalization Transaction

Finalization is the highest-risk core POS operation.

The baseline transaction is:

```text
BEGIN

1. Revalidate authenticated session/device state.
2. Resolve current membership and authorization.
3. Load sale under tenant scope.
4. Verify sale state is finalizable.
5. Resolve authoritative SKU/unit information.
6. Resolve authoritative prices and policy versions.
7. Resolve authoritative tax policy.
8. Validate discount/override authority.
9. Validate customer/vertical restrictions where applicable.
10. Validate register session.
11. Validate idempotency.
12. Acquire deterministic inventory locks.
13. Validate inventory availability / policy.
14. Calculate authoritative totals.
15. Persist finalized sale snapshot.
16. Persist immutable sale lines.
17. Apply inventory ledger postings.
18. Persist payment/tender linkage.
19. Persist audit evidence.
20. Persist outbox intent.
21. Persist idempotency completion.

COMMIT

Then:

workers / integrations / read models
```

No HTTP call to a payment provider or MRA endpoint should be made while the transaction is holding business locks.

This preserves transaction locality and prevents provider latency from turning a POS sale into a long-lived database lock holder.

---

# 16. Concurrency Model

POS is inherently concurrent.

Possible concurrent actors include:

```text
cashier A
cashier B
mobile offline device
manager adjustment
stock receiving worker
inventory transfer worker
return/refund workflow
payment reconciliation worker
```

The primary inventory race is:

```text
stock = 1

Sale A reads 1
Sale B reads 1

A sells 1
B sells 1

incorrect result = -1 or duplicate sale
```

The correct design is based on authoritative inventory transaction semantics and targeted locking/atomic updates.

## 16.1 Deterministic lock ordering

When finalization touches multiple inventory rows, locks must be acquired in deterministic order, such as by normalized `(location_id, sku_id, lot_id)` order.

This reduces deadlock risk.

The application must not acquire locks in an arbitrary order based on user-supplied line ordering.

## 16.2 Deadlock behavior

Deadlock detection is a transaction failure, not a signal to blindly retry forever.

A bounded retry policy may be used for known transient transaction errors:

```text
attempt 1
  ↓ deadlock/serialization failure
attempt 2
  ↓ deadlock/serialization failure
attempt 3
  ↓ fail safely
```

The request must remain idempotent across retries.

---

# 17. Inventory Interaction

Phase 10 does not maintain stock independently.

The sale finalization produces inventory ledger effects through the Phase 9 inventory service.

Conceptually:

```text
Sale Finalization
      |
      v
Inventory Command
      |
      v
Inventory Ledger
      |
      +--> durable posting
      +--> projection update / rebuildable balance
```

The sale must preserve linkage to the inventory postings so operators can answer:

> Which inventory movement consumed the stock for this sale?

Conversely:

> Which sale or authorized adjustment caused this inventory movement?

Both directions should be reconstructable through durable identifiers.

---

# 18. Negative Stock Policy

The default safety position is:

```text
available quantity < required quantity
→ reject finalization
```

Exceptions require an explicit business policy, not a hidden feature flag.

If negative stock is supported for a particular domain, the sale must record the policy/version that permitted it and the accounting/inventory impact must remain explainable.

A frontend toggle must never be able to turn negative stock into an authority decision.

---

# 19. Payment and Tender Boundary

Phase 10 must support tender intent without pretending payment infrastructure is a POS table.

Examples:

```text
CASH
MOBILE_MONEY
BANK_TRANSFER
CARD
MIXED_TENDER
CREDIT / ACCOUNT
```

A tender line should represent the user's intended allocation and authoritative internal state, while the payment subsystem owns provider interaction and external reconciliation.

For mobile money:

```text
Sale
  ↓
PaymentIntent
  ↓
provider adapter
  ↓
provider evidence
  ↓
reconciliation
  ↓
final payment state
```

A successful request to a provider adapter does not automatically mean the external payment is settled unless the payment contract defines the response as authoritative proof.

Unknown provider outcomes must remain explicit.

---

# 20. Mixed Tender

Mixed tender is supported only through an explicit domain model.

Example:

```text
Sale total = 20,000

Cash        8,000
Mobile      7,000
Bank        5,000
------------------
Total      20,000
```

The invariant is:

```text
sum(tenders) + approved outstanding balance = sale total
```

The implementation must define rounding and residual rules explicitly.

The system must reject:

```text
sum(tenders) > permitted amount
invalid currency
duplicate terminal/provider references
negative tender
reused payment reference
unauthorized tender correction
```

---

# 21. Customer Context

Customer association is optional for ordinary cash sales but becomes significant for:

- credit/account sales;
- loyalty programs;
- tax invoice requirements where applicable;
- regulated transactions;
- returns and refunds;
- customer-specific pricing;
- business reporting.

The cashier should receive only the customer fields necessary for their role.

Cashier-facing search should use restricted projections and bounded result counts. The API contract explicitly identifies search endpoints as potential enumeration channels and requires scope enforcement, bounded results, safe projections and rate controls. fileciteturn10file9L957-L975

---

# 22. Pharmacy Extension

Pharmacy is not just another product category.

The POS domain may invoke a pharmacy policy extension for:

```text
regulated medicine sale
prescription requirement
authorized pharmacist role
lot/expiry validation
quarantine status
recall restrictions
restricted product approval
```

The generic cashier permission:

```text
SALE_CREATE
```

must not automatically imply:

```text
SELL_RESTRICTED_MEDICINE
VIEW_DISPENSING_RECORD
APPROVE_RESTRICTED_ADJUSTMENT
```

The system architecture already treats pharmacy as a controlled regulated-domain extension. fileciteturn11file1L240-L258

Exact legal rules must be implemented only from current applicable requirements and approved product policy.

---

# 23. Agro-Dealer Extension

Agro-dealer POS may require:

```text
lot/batch
expiry/recommended-use
formulation
packaging conversion
traceability metadata
restricted products
```

The extension must integrate with the same inventory and authorization boundaries.

It MUST NOT bypass generic stock, tenant or pricing controls.

---

# 24. Offline POS

Offline POS is a core product capability, but offline mode is not permission to become a second financial authority.

The offline architecture is:

```text
AUTHORIZED DEVICE
       |
       v
LOCAL SQLITE
       |
       +--> draft sale
       +--> offline command
       +--> client event log
       +--> bounded local snapshot
       |
       v
SYNC PROTOCOL
       |
       v
SERVER AUTHORIZATION + VALIDATION
       |
       v
POSTGRESQL AUTHORITATIVE SALE
```

## 24.1 What may be accepted offline

Subject to device policy and risk controls:

- create sale draft;
- scan locally cached SKU;
- compute provisional total;
- queue sale command;
- associate allowed customer context;
- capture local tender intent.

## 24.2 What must not become offline authority

Without an explicit bounded policy, offline clients MUST NOT permanently decide:

- organization membership;
- role assignment;
- unrestricted price overrides;
- privileged refunds;
- owner transfer;
- payment settlement truth;
- final tax/EIS acceptance;
- cross-branch authority;
- security-policy mutation.

## 24.3 Offline stale-price handling

The server may compare:

```text
client_snapshot_version
server_price_version
sale_created_at
```

and decide whether to:

```text
accept
accept with recorded stale version
recalculate
require manager review
reject
```

The product must not silently convert a stale offline command into a materially different financial result without evidence.

---

# 25. Offline Sale Conflict Model

Offline conflicts must be domain-specific.

Example:

```text
Device A sells SKU X offline
Device B sells SKU X offline
server stock = 1
```

When synchronized:

```text
first accepted command → inventory consumed
second command → insufficient stock / exception
```

The second device must not invent stock.

The sync subsystem may classify the command as:

```text
ACCEPTED
REJECTED
CONFLICT
REQUIRES_REVIEW
DUPLICATE
UNKNOWN
```

The financial result must be explicit.

---

# 26. Void vs Refund vs Return

These concepts must not be conflated.

### Void

A controlled reversal/cancellation of an operation that remains within the permissible pre/post-finalization lifecycle.

### Return

A business event where goods are returned to the merchant according to return eligibility rules.

### Refund

A monetary reversal associated with a valid business event.

The separate Phase 14 contract governs the detailed implementation. POS must maintain state boundaries so that one cannot be substituted for another through a generic `status=REFUNDED` mutation.

---

# 27. API Contract

Preferred command-oriented endpoints:

```text
POST /v1/sales
POST /v1/sales/{sale_id}/lines
POST /v1/sales/{sale_id}/remove-line
POST /v1/sales/{sale_id}/price-override
POST /v1/sales/{sale_id}/discount
POST /v1/sales/{sale_id}/finalize
POST /v1/sales/{sale_id}/void
POST /v1/sales/{sale_id}/cancel
GET  /v1/sales/{sale_id}
GET  /v1/sales
POST /v1/sales/{sale_id}/tenders
```

The exact public routes must match the canonical route inventory.

Generic endpoints such as:

```text
PATCH /sales/{id}
PATCH /sales/{id}/status
PUT /sales/{id}/total
```

are prohibited for final financial transitions.

This follows the existing Sitolo API rule that business commands are preferred over table-shaped CRUD. fileciteturn11file2L383-L395

---

# 28. API Request Processing Order

Every protected POS mutation follows:

```text
1. parse request
2. reject oversized payload
3. authenticate
4. validate session/device
5. establish trusted tenant context
6. authorize function
7. validate object scope
8. validate properties
9. validate state
10. execute domain command
11. persist transaction
12. emit safe response
```

Authorization before expensive business computation is preferred, but resource ownership and authoritative state must still be rechecked inside the transaction where races matter.

---

# 29. Object-Level Authorization

Every operation on:

```text
sale_id
customer_id
register_id
branch_id
sku_id
payment_id
```

must be authorized against authenticated context.

A UUID is not an authorization control.

For example:

```text
cashier A
  ↓
GET /v1/sales/{sale-from-branch-B}
  ↓
DENY / scoped absence
```

The application should avoid leaking whether a foreign-tenant sale exists when the endpoint semantics permit a uniform not-found response.

OWASP recommends that every function receiving an object identifier and using it to access data perform object-level authorization. citeturn229775search1

---

# 30. Property-Level Authorization

A sale DTO must not expose or accept every internal property merely because the language can deserialize it.

Potentially restricted properties include:

```text
internal calculation metadata
approval references
payment provider details
security metadata
cost price
margin
manager-only override reason
fraud/risk signals
internal tax configuration identifiers
```

The code should use dedicated request/response DTOs rather than generic serialization of domain entities. OWASP explicitly recommends selecting only fields appropriate to the endpoint and controlling client-modifiable properties. citeturn229775search2

---

# 31. Authorization Matrix

The minimum implementation should maintain a machine-readable command registry.

Example:

| Command | Cashier | Manager | Owner | Auditor |
|---|---:|---:|---:|---:|
| CreateSale | yes | yes | yes | no |
| AddSaleLine | yes | yes | yes | no |
| ApplyNormalDiscount | policy | yes | yes | no |
| PriceOverride | bounded | yes | yes | no |
| FinalizeSale | yes | yes | yes | no |
| VoidSale | policy | yes | yes | no |
| ReadSale | scoped | scoped | scoped | yes |
| ExportSales | no | policy | yes | policy |
| ConfigurePricing | no | policy | yes | no |

These are illustrative. The canonical permission registry must be defined centrally and tied to Phase 6 authorization policies.

---

# 32. High-Risk POS Operations

The following require explicit policy and potentially step-up/approval:

```text
large refund
large discount
large price override
sale void after threshold
cash variance correction
credit sale above threshold
manual payment correction
manual payment reference override
restricted medicine sale
bulk sales export
historical correction request
```

A user authenticated as a manager does not automatically receive unlimited authority to all high-risk operations.

Risk thresholds must be evaluated against authoritative values, not client-provided summaries.

---

# 33. Approval Boundaries

Where approval is required:

```text
initiating actor
   ↓
request privileged operation
   ↓
approval object
   ↓
authorized approver
   ↓
step-up if required
   ↓
operation revalidated against current state
   ↓
commit
```

Approval must not be a reusable bearer token detached from its target.

If a sale changes materially after approval:

```text
approved sale
↓
quantity changed
↓
approval invalidated
```

The system must never allow an approval for one semantic operation to authorize another.

---

# 34. Audit Evidence

Every state-changing sale command must have enough audit evidence to establish:

```text
who
what
when
which tenant
which branch
which register
which device
which command
which target sale
which authorization path
which approval
which result
```

The audit layer is durable evidence, not a best-effort application log.

The log/telemetry layer must not duplicate sensitive payment credentials or secrets.

---

# 35. Outbox Events

A successful sale may emit internal events such as:

```text
SaleCreated
SaleFinalized
SaleVoided
SaleCancelled
SalePaymentPending
SalePaymentLinked
SaleInventoryApplied
```

The exact event taxonomy must be stable and versioned.

The outbox record is committed in the same database transaction as the sale state.

Then:

```text
COMMIT
 ↓
outbox worker
 ↓
integration / notification / projection
```

The worker must not create financial truth that the sale transaction never established.

---

# 36. Reporting Read Models

POS reads should use dedicated projections when necessary for performance.

Examples:

```text
current-day sales summary
cashier shift sales
branch sales totals
sales search index
product sales history
```

These are rebuildable projections.

The source of truth remains the finalized sale plus authoritative inventory/payment relationships.

---

# 37. Performance Requirements

POS is latency-sensitive, especially for low-end Android devices and intermittent networks.

The target should be:

```text
fast-path barcode lookup
fast draft mutation
bounded payloads
few database round trips
short transactions
no external provider call in core transaction
bounded authorization evaluation
```

A finalization request should avoid:

```text
N+1 SKU queries
N+1 inventory reads
N+1 permission queries
unbounded customer searches
full-table sale history scans
```

Batch-load where the domain permits.

Use SQL indexes aligned with actual access patterns:

```text
(organization_id, sale_id)
(organization_id, branch_id, finalized_at)
(organization_id, register_id, created_at)
(organization_id, client_operation_id)
```

Exact indexes must follow Phase 5 schema and measured query plans.

---

# 38. Resource Exhaustion Controls

A malicious or malfunctioning client must not be able to create a sale with:

```text
10 million lines
huge strings
unbounded metadata JSON
millions of tenders
pathological discount rules
extremely long customer search
```

Command schemas must have bounded limits.

Example baseline constraints:

```text
max sale lines
max line description length
max tender count
max customer reference length
max metadata size
max batch-sync sales per request
```

The values are configuration and workload dependent, but there must be finite limits.

---

# 39. Database Transaction Boundaries

A completed sale should be one coherent transaction for its internal authoritative effects.

Conceptually:

```text
transaction:
  sale
  sale_lines
  inventory postings
  tender/payment linkage
  audit evidence
  outbox
  idempotency
```

External side effects remain outside.

This preserves the existing Sitolo rule that a business invariant should be committed atomically and that external network calls must not be held inside the database transaction.

---

# 40. Failure Semantics

POS must distinguish at least:

```text
validation failure
authorization denied
stale state
insufficient inventory
pricing conflict
payment pending
payment rejected
provider unknown
transaction conflict
serialization/deadlock retry
internal failure
```

The API error contract must remain low-information for security-sensitive cases while providing enough machine-readable detail for the client to recover.

Example:

```text
SALE_FINALIZATION_CONFLICT
```

rather than exposing internal SQL details.

---

# 41. Crash Consistency

Consider:

```text
inventory applied
sale persisted
server crashes
response lost
```

The retry must not create a duplicate sale.

The authoritative idempotency record solves this class of problem when committed atomically with the sale.

Another case:

```text
sale persisted
outbox missing
```

is prohibited by transactional outbox semantics.

---

# 42. Duplicate Submission

The implementation must handle:

```text
double tap
network retry
HTTP retry
mobile reconnect replay
queue replay
worker replay
client process restart
```

The expected result is either:

```text
same committed sale result
```

or an explicit safe duplicate outcome.

Never:

```text
second financial effect
```

from the same logical command.

---

# 43. Sales Search and Enumeration

Search endpoints must remain scope-aware.

Potential attacks:

```text
GET /sales?customer_id=...
GET /sales?receipt_number=...
GET /sales?date_range=...
GET /sales/{guessable-id}
```

The server must enforce:

- organization/branch scope;
- role permissions;
- bounded page size;
- safe projections;
- stable pagination;
- appropriate rate limits;
- no cross-tenant counts;
- no leakage through sort/filter differences.

The API must avoid turning a privileged reporting query into an unrestricted data export.

---

# 44. Receipt Generation

Receipt generation is a presentation/read concern over authoritative sale facts.

The printed/displayed receipt must derive from:

```text
finalized sale snapshot
```

and not recalculate prices from current catalogue settings.

A receipt may include:

```text
merchant information
branch
receipt number
sale date/time
line descriptions
quantity
unit price
discounts
tax information
payment/tender summary
```

Sensitive internal fields must not leak.

Receipt generation must be deterministic enough to reproduce a sale's historical presentation where required.

---

# 45. Receipt Numbering

Receipt numbers and internal sale IDs serve different purposes.

Do not assume:

```text
sale_id = customer-facing receipt number
```

A receipt number may need a branch/register sequence and may have legal/reporting implications.

The numbering strategy must define:

```text
scope
uniqueness
concurrency
reset policy
recovery behavior
offline sequence handling
collision handling
```

Offline receipt numbers require especially careful conflict design. A client-generated local sequence cannot be assumed globally authoritative.

---

# 46. Offline Receipt Strategy

The safest baseline is to distinguish:

```text
local receipt reference
server authoritative receipt number
```

The client may show:

```text
PENDING-LOCAL-1234
```

until synchronization establishes the authoritative server record, unless the business model explicitly defines an offline legal receipt sequence.

A country-specific or regulated receipt requirement must be closed with legal/product evidence before implementation.

---

# 47. Fraud Signals

The POS module should expose auditable control signals without pretending every anomaly is fraud.

Examples:

```text
unusually large discount
rapid void frequency
unusually high refund rate
many failed payment attempts
cashier sells outside normal branch pattern
repeated override requests
high-value manual price changes
multiple devices for one cashier
```

These signals may feed the risk/reporting subsystem.

They should not silently block legitimate activity unless the product explicitly defines a rule.

---

# 48. Temporal Consistency

Time semantics are important for:

```text
price validity
tax policy
promotions
register sessions
sale timestamp
offline event creation
server acceptance time
```

The server remains authoritative for server-side transaction time.

An offline client timestamp is evidence about when the device claims the event happened, not unquestionable server truth.

Persist both where useful:

```text
client_occurred_at
server_received_at
server_committed_at
```

The exact set should follow the sync and audit contracts.

---

# 49. Currency and Multi-Currency Boundaries

Sitolo's initial market may operate primarily in MWK, but architecture should not assume currency is an untyped string.

Every monetary value must have an explicit currency.

The system must reject:

```text
sale currency = MWK
payment currency = USD
implicit conversion
```

unless a defined FX policy is implemented.

Currency conversion must never be inferred from a client total.

---

# 50. Credit / Account Sales

Credit sales create additional financial semantics.

The POS command must verify:

```text
customer account eligibility
credit limit / policy
current exposure
approval threshold
branch authority
```

A cashier's generic sale capability does not imply unrestricted credit authority.

Credit balance is not represented as an arbitrary editable field on the sale.

Detailed receivables behavior should have a dedicated domain contract before production financial use.

---

# 51. Tax Invoice / Fiscalization Boundary

POS must preserve enough stable identifiers for Phase 15 MRA EIS integration.

Potential references include:

```text
sale_id
receipt number
tax calculation version
legal entity
branch
terminal/register
```

MRA EIS submission must remain an external integration workflow.

A provider rejection must not rewrite the completed sale's internal commercial facts.

The existing system contract explicitly treats EIS as an external trust boundary and preserves local sale truth independently of provider outcomes.

---

# 52. API Authentication and Session Freshness

A sale finalization request must validate the current security context, not merely trust a token's original claims.

The auth stack already separates:

```text
identity
session
membership
role/scope
```

and supports revocation.

Therefore:

```text
valid old token
+ revoked membership
→ DENY
```

A recently revoked device must likewise fail privileged operations.

---

# 53. Authorization Freshness

A cached authorization decision may be used only within the rules defined by Phase 6.

High-risk finalization must have conservative freshness semantics.

The system must never do:

```text
Redis unavailable
→ assume cashier is authorized
```

It must fail closed where authority cannot be established.

---

# 54. Worker and Background Processing

Some POS work may be asynchronous:

```text
receipt rendering
notifications
report projection
EIS submission
payment polling
fraud analytics
```

Workers must authenticate as service identities and operate only on explicitly authorized tenant-scoped jobs.

A worker receiving:

```text
job_id
sale_id
```

must not assume that the job itself grants universal authority.

The worker must validate:

```text
job tenant
sale tenant
job purpose
current job state
idempotency
```

---

# 55. Security Invariants

The following are non-negotiable:

```text
1. Client total is never final authority.
2. Client price is never final authority.
3. Client tenant ID is never authority.
4. Client branch ID is never authority.
5. Client inventory quantity is never final authority.
6. Every finalized sale is authorized.
7. Every sale object is tenant-scoped.
8. Every sale mutation is state-validated.
9. Every retryable mutation is idempotent.
10. Finalized financial facts are not destructively edited.
11. Inventory effects are atomically linked to finalization.
12. Payment evidence is explicit and reconciliable.
13. External provider calls are outside the core DB transaction.
14. Historical sale snapshots remain stable.
15. Offline commands do not become a second authority.
16. Authorization failures fail closed.
17. Unknown payment outcomes remain explicit.
18. High-risk operations require explicit policy.
19. Audit evidence is durable.
20. Outbox intent is atomic with the sale transaction.
```

---

# 56. Threat Model for POS

## Threat: Forged total

**Attack:** client submits a smaller total than the authoritative line calculation.

**Mitigation:** server-side recalculation; client total advisory only.

## Threat: Forged price

**Attack:** client substitutes a lower unit price.

**Mitigation:** authoritative price resolution; override authorization.

## Threat: Cross-tenant sale access

**Attack:** attacker swaps sale UUID.

**Mitigation:** object-level authorization + tenant-scoped repository + RLS.

## Threat: Cross-branch sale mutation

**Attack:** branch-scoped manager mutates another branch.

**Mitigation:** scope-aware authorization + repository scoping + RLS.

## Threat: Double finalization

**Attack:** retry same command.

**Mitigation:** idempotency and state transition constraint.

## Threat: Inventory race

**Attack:** two sales consume one item.

**Mitigation:** authoritative transaction/locking strategy and concurrency tests.

## Threat: Payment replay

**Attack:** same external reference used twice.

**Mitigation:** unique provider reference/idempotency boundaries in payment subsystem.

## Threat: Privileged property injection

**Attack:** client adds manager-only fields to JSON.

**Mitigation:** explicit DTOs and property authorization.

## Threat: Offline replay after revocation

**Attack:** revoked device resubmits queued privileged commands.

**Mitigation:** server validation of device/session/membership plus command lineage.

## Threat: Search enumeration

**Attack:** attacker iterates receipt/customer IDs.

**Mitigation:** authorization, bounded projections, pagination and rate limits.

## Threat: Approval replay

**Attack:** reuse an old approval on another sale state.

**Mitigation:** approval target fingerprint + state/version check.

---

# 57. Testing Strategy

Phase 10 requires positive and negative tests.

## 57.1 Unit tests

Test:

```text
money arithmetic
quantity conversion
sale totals
tax calculation
discount bounds
state transitions
tender balancing
approval conditions
```

## 57.2 Authorization tests

```text
cashier can create sale
cashier cannot export all sales
branch manager cannot mutate branch B
cashier cannot override above threshold
revoked membership cannot finalize sale
revoked device cannot finalize sale
```

## 57.3 PostgreSQL tests

Use real PostgreSQL for:

```text
foreign keys
unique idempotency constraints
RLS
locking
transaction rollback
commit atomicity
query plans
```

The existing CI contract explicitly requires real PostgreSQL for critical transaction, RLS, privilege and concurrency verification. fileciteturn9file3L453-L492

## 57.4 Concurrency tests

At minimum:

```text
sale vs sale
sale vs stock adjustment
sale vs transfer
sale finalization vs cancellation
sale finalization vs device revocation
sale finalization vs membership revocation
payment callback vs manual reconciliation
```

## 57.5 Idempotency tests

```text
same command twice
same command 100 times
same idempotency key different payload
crash after DB commit before response
network retry
queue replay
```

## 57.6 Offline tests

```text
offline sale
network restored
duplicate sync
stale product
stale price
inventory conflict
revoked device
expired session
process death before sync acknowledgement
```

## 57.7 Property-based tests

Important properties:

```text
subtotal >= 0
final total >= 0 under supported rules
same deterministic inputs → same total
idempotent retry → same result
permission removal never grants access
cross-tenant request never succeeds
line order cannot alter semantic total when rules are order-independent
```

---

# 58. Mutation Testing

Mutation testing should target:

```text
remove authorization check
flip discount threshold
allow zero/negative quantity
skip inventory lock
skip idempotency lookup
remove tenant predicate
allow finalized sale edit
skip approval check
ignore device state
```

The test suite must kill these mutations.

If a security mutation survives, the suite is insufficient.

---

# 59. Fuzzing Targets

Fuzz:

```text
sale request JSON
line arrays
quantities
decimal representations
discount fields
currency codes
receipt references
customer identifiers
idempotency keys
sync commands
```

Assertions include:

```text
no panic
no integer overflow
no memory blow-up within defined bounds
no bypass of validation
no authorization escalation
no inconsistent persisted state
```

---

# 60. Database Constraints

The schema should enforce where practical:

```text
sale_id PK
organization FK
branch FK
register FK
cashier membership FK
state validity
currency NOT NULL
non-negative quantities where appropriate
unique idempotency key scope
unique receipt reference scope
sale line ownership
payment linkage integrity
```

Foreign keys should prevent impossible combinations, such as a sale in organization A referencing a register owned by organization B.

Phase 5 established the broader principle of composite tenant consistency and RLS as defense in depth.

---

# 61. RLS Requirements

RLS must prevent:

```text
tenant A reading tenant B sales
tenant A inserting tenant B sale
tenant A updating tenant B sale
tenant A reading tenant B customer
```

The application authorization layer remains mandatory.

A zero-row RLS result must not hide correctness bugs in critical operations. Negative tests must explicitly prove the application denied the action before or alongside RLS behavior.

---

# 62. Repository Safety

Repositories should expose scoped methods.

Preferred:

```rust
get_sale_for_authorized_context(ctx, sale_id)
create_sale(ctx, command)
finalize_sale(ctx, sale_id, command)
```

Avoid:

```rust
get_sale(sale_id)
update_sale(id, json)
```

where the caller must remember to add tenant and authorization predicates manually.

The repository API should make the unsafe path difficult to express.

---

# 63. Rust Module Architecture

Recommended structure:

```text
crates/
  sales-domain/
    src/
      sale.rs
      sale_line.rs
      totals.rs
      money.rs
      state.rs
      discount.rs
      errors.rs
      commands.rs

  sales-application/
    src/
      create_sale.rs
      add_line.rs
      apply_discount.rs
      finalize_sale.rs
      void_sale.rs
      authorization.rs
      ports.rs

  sales-infrastructure/
    src/
      postgres.rs
      repositories.rs
      outbox.rs
      idempotency.rs
      projections.rs

  api/
    src/
      routes/sales.rs
      dto/sales.rs
      handlers/sales.rs
```

The domain layer must not know Axum or SQLx.

The application layer orchestrates authorization, transactions and ports.

Infrastructure implements SQLx/PostgreSQL and external boundaries.

---

# 64. CPU vs I/O Separation

POS should be designed around the actual workload.

## CPU-bound

```text
large basket total calculation
complex promotion evaluation
report aggregation if synchronous
heavy receipt generation
```

These should be bounded and, where necessary, moved to asynchronous workers.

## I/O-bound

```text
PostgreSQL
payment provider
EIS provider
object storage
notifications
```

These must have:

```text
timeouts
bounded concurrency
cancellation
retry policy
circuit/failure handling where justified
```

A provider slowdown must not cause a database transaction to sit open holding inventory locks.

---

# 65. Tokio / Async Runtime Requirements

Axum handlers should perform asynchronous I/O without blocking Tokio worker threads.

Avoid:

```text
std::thread::sleep
blocking filesystem operations
unbounded CPU-heavy loops
synchronous provider SDKs without isolation
```

If expensive CPU work is required, use an explicit blocking/CPU execution boundary and cap concurrency.

The sales path should remain latency-predictable.

---

# 66. Caching

Safe cache candidates include:

```text
product lookup
barcode lookup
current catalogue metadata
read-only pricing projections
```

Dangerous cache candidates include authoritative permission or sale state without proper versioning/freshness controls.

Never cache:

```text
user_id -> unlimited POS authority
sale_id -> finalized forever
inventory_available -> authoritative truth
```

Authorization cache outage must not become a privilege escalation.

---

# 67. Receipt and Search Caches

Receipt caches must key on immutable sale identity/version.

Search caches must include:

```text
tenant
branch scope
permission/profile
query
pagination/version
```

A global cache key such as:

```text
sales:recent
```

is unsafe.

Cross-tenant cache contamination must be explicitly tested.

---

# 68. Observability

Metrics should include:

```text
sales.created
sales.finalized
sales.rejected
sales.authorization_denied
sales.inventory_conflict
sales.idempotency_duplicate
sales.payment_pending
sales.finalization_latency
sales.transaction_retry
sales.deadlock
```

Metrics labels must remain bounded.

Do not use unbounded:

```text
sale_id
customer_name
receipt_number
```

as metric labels.

Logs should include safe correlation identifiers:

```text
request_id
trace_id
sale_id where safe
organization_id where policy permits
```

but must exclude secrets, payment credentials and raw authentication tokens.

---

# 69. Business Audit vs Telemetry

These are separate.

```text
Audit
→ durable evidence of business/security action

Telemetry
→ operational visibility
```

A finalized sale must remain auditable even if a telemetry backend is temporarily unavailable.

Conversely, missing a debug log must never erase business history.

---

# 70. Deployment and Migration

POS schema rollout must follow expand/contract principles when required.

Examples:

```text
add nullable column
→ deploy compatible code
→ backfill
→ enforce constraint
→ remove legacy path
```

Never deploy a new sale finalization path that assumes a database column exists before the migration is safely applied across the fleet.

For large historical sales tables, index creation and backfill strategies must be evaluated against production lock and I/O impact.

---

# 71. Data Retention

Sales may be subject to tax, accounting, audit and business retention requirements.

The implementation should therefore avoid destructive deletion of finalized financial facts.

Retention and archival policy must be jurisdiction-aware.

A “delete sale” API is therefore generally a draft cancellation operation, not destruction of authoritative financial history.

---

# 72. Support and Administrative Access

Support personnel must not use normal cashier permissions to investigate customer sales.

A controlled support path should provide:

```text
explicit support role
justification
target tenant
time-bound authorization
audit
read-only by default
step-up / approval for mutations
```

Break-glass access must be rare, bounded, audited and revocable.

---

# 73. Privacy

Customer information should be minimized in sale records and API responses.

Examples of fields requiring careful handling:

```text
phone
name
address
account identifiers
payment references
```

Cashier views should expose only what operationally necessary.

Exports require separate authorization because they create materially larger exfiltration capability. The existing API contract explicitly treats bulk sales export as a separate authorization surface. fileciteturn10file9L943-L985

---

# 74. Incident Runbook — Duplicate Sale

```text
1. Identify duplicated command/idempotency key.
2. Confirm whether both transactions committed.
3. Confirm inventory postings.
4. Confirm payment effects.
5. Identify authoritative sale.
6. Freeze unsafe compensating path if necessary.
7. Create explicit correction workflow.
8. Never delete evidence.
9. Add regression test.
10. Review why idempotency failed.
```

---

# 75. Incident Runbook — Inventory Discrepancy During Sale

```text
1. Identify sale command.
2. Confirm sale state.
3. Inspect inventory posting linkage.
4. Compare ledger vs projection.
5. Determine whether discrepancy is projection lag or ledger inconsistency.
6. Do not manually edit balance rows as a shortcut.
7. Reconcile using authoritative ledger.
8. Create compensating command where required.
9. Preserve evidence.
10. Add regression test.
```

---

# 76. Incident Runbook — Payment Unknown Outcome

```text
1. Confirm local sale state.
2. Identify payment intent/reference.
3. Determine provider observation state.
4. Query/reconcile provider when safe.
5. Do not create duplicate payment intent merely because the first response was lost.
6. Preserve unknown/pending state where evidence is insufficient.
7. Resolve through payment reconciliation policy.
8. Add regression test for the failure mode.
```

---

# 77. CI Release Gates

A POS release is blocked when any of the following fail:

```text
[ ] authorization negative tests
[ ] cross-tenant tests
[ ] cross-branch tests
[ ] RLS tests
[ ] idempotency tests
[ ] concurrency inventory tests
[ ] state-machine tests
[ ] money arithmetic tests
[ ] property-level authorization tests
[ ] offline replay tests
[ ] revoked-device tests
[ ] revoked-membership tests
[ ] mutation testing target suite
[ ] real PostgreSQL integration suite
[ ] migration compatibility tests
[ ] route inventory validation
[ ] secret scan
[ ] dependency/security scan
```

A test that did not run is not a pass. The existing CI contract explicitly distinguishes PASS, FAIL, NOT RUN, NOT APPLICABLE and EXPLICITLY WAIVED. fileciteturn9file1L149-L185

---

# 78. Contract Tests

Provider-independent POS contract tests should validate:

```text
CreateSale
AddLine
CalculateTotals
ApplyDiscount
FinalizeSale
CancelSale
VoidSale
CreateTender
LinkPayment
```

Then provider/integration phases add specific tests for payment and EIS behavior.

---

# 79. Test Fixtures

The security harness should seed:

```text
TENANT_A
TENANT_B
BRANCH_A1
BRANCH_A2
BRANCH_B1
REGISTER_A1
REGISTER_B1
OWNER_A
MANAGER_A
CASHIER_A
AUDITOR_A
OWNER_B
MANAGER_B
CASHIER_B
DEVICE_A
DEVICE_B
REVOKED_DEVICE
SKU_A1
SKU_A2
SKU_B1
SALE_A1
SALE_B1
```

For inventory concurrency:

```text
SKU_RACE
location = A1
available = 1
```

Then execute two concurrent finalizations.

Expected invariant:

```text
accepted sales <= available authoritative quantity
```

under the chosen negative-stock policy.

---

# 80. Definition of Done for Sale Finalization

A finalization implementation is complete only when:

```text
[ ] authenticated principal validated
[ ] session/device validated
[ ] tenant context trusted
[ ] permission evaluated
[ ] object scope verified
[ ] sale state verified
[ ] all line SKUs resolved authoritatively
[ ] units/conversions verified
[ ] prices resolved authoritatively
[ ] discounts authorized and validated
[ ] tax policy resolved
[ ] register session validated
[ ] inventory availability enforced
[ ] concurrency tested
[ ] totals recalculated server-side
[ ] idempotency persisted atomically
[ ] sale snapshot persisted
[ ] inventory posting persisted
[ ] payment linkage persisted
[ ] audit evidence persisted
[ ] outbox persisted
[ ] external calls excluded from core transaction
[ ] retry behavior documented
[ ] failure semantics documented
[ ] telemetry exists
[ ] release gates pass
```

---

# 81. Implementation Sequence

Recommended pull-request order:

```text
PR-001
sales domain primitives + state machine

PR-002
sale draft persistence

PR-003
sale line + SKU resolution

PR-004
price/tax/discount calculation

PR-005
register/cashier authorization integration

PR-006
finalization application service

PR-007
inventory ledger integration

PR-008
idempotency + crash recovery

PR-009
payment/tender linkage

PR-010
audit + outbox

PR-011
offline command integration

PR-012
receipt/read models

PR-013
concurrency/security tests

PR-014
performance/query optimization

PR-015
runbooks + observability

PR-016
release certification
```

Each PR should remain independently reviewable and must not introduce an insecure intermediate state.

---

# 82. Research Basis

This implementation was researched against the current Sitolo project documents and current technical guidance.

## Project sources

- `sitolo.md`
- `domain_model.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `security_architecture_design.md`
- `security_implementation_spec.md`
- `testing_strategy.md`
- `security_test_harness.md`
- `ci_enforcement.md`
- `phase5_postgresql_schema_migrations_constraints_rls_implementation.md`
- `phase8_product_catalogue_implementation.md`
- `phase9_inventory_ledger_implementation.md`

The existing domain model defines sales as part of the canonical business hierarchy and requires the authorization tuple of principal, organizational scope, permission, target resource, target state and contextual conditions. fileciteturn10file1L160-L208

The existing API specification requires protected server-side authorization, bounded typed input, explicit transaction boundaries, idempotency, safe errors and fail-closed handling of malformed or stale requests. fileciteturn11file2L287-L335

The existing database specification makes PostgreSQL the authoritative transactional store, preserves audit/outbox/idempotency state in the transaction and keeps RLS as defense in depth. fileciteturn11file3L409-L474

## External technical references

- OWASP API Security Top 10 — Broken Object Level Authorization: https://owasp.org/API-Security/editions/2023/en/0xa1-broken-object-level-authorization/
- OWASP API Security Top 10 — Broken Object Property Level Authorization: https://owasp.org/API-Security/editions/2023/en/0xa3-broken-object-property-level-authorization/
- OWASP API Security Top 10 — Broken Function Level Authorization: https://owasp.org/API-Security/editions/2023/en/0xa5-broken-function-level-authorization/
- PostgreSQL documentation: https://www.postgresql.org/docs/current/
- GS1 Global Traceability Standard: https://www.gs1.org/standards/gs1-global-traceability-standard/current-standard
- IFRS IAS 2 Inventories: https://www.ifrs.org/issued-standards/list-of-standards/ias-2-inventories/
- Axum documentation: https://docs.rs/axum/latest/axum/
- SQLx documentation: https://docs.rs/sqlx/latest/sqlx/

GS1's traceability standard distinguishes class-level, batch/lot-level and instance-level identification, which is relevant to POS SKU/lot/serial linkage even though the exact traceability level is domain-dependent. citeturn229775search0

IAS 2 identifies specific identification for non-interchangeable inventory and FIFO or weighted-average cost formulas for ordinarily interchangeable inventory; POS must therefore preserve sales facts without confusing sale price with inventory-cost methodology. citeturn229775search5

---

# 83. Advantages

## Strong transactional correctness

A sale finalization path gives Sitolo one authoritative place to establish commercial truth and connect it to inventory, payment and evidence.

## Offline-compatible without dual authority

The client can remain operational under weak connectivity while PostgreSQL remains the authoritative final state.

## Clear security boundaries

Authentication, authorization, object scope, state rules and database isolation each contribute a distinct defense.

## Financial explainability

Historical snapshots preserve why a sale has the value it has.

## Concurrency safety

Inventory and idempotency are explicitly designed around races rather than assuming single-user execution.

## Extensible vertical model

Pharmacy, agro-dealer and future regulated workflows can plug into policy boundaries without corrupting generic POS logic.

---

# 84. Disadvantages

## Higher implementation complexity

A robust POS is materially more complex than a simple cart table.

## More database discipline

Transactions, locks, RLS, indexes and constraints must be designed together.

## More testing cost

Concurrency, replay, offline and negative authorization tests are expensive but necessary.

## More explicit business modelling

Discounts, price overrides, payment states and reversals require real domain semantics.

## Offline complexity

Reconciling disconnected sales creates operational edge cases that generic CRUD systems do not need to solve.

---

# 85. Why This Over Alternatives

## Alternative A — Mutable `sales.total` CRUD

Rejected.

It allows clients or generic update paths to bypass the calculation, inventory and authorization model.

## Alternative B — Client-authoritative offline sales

Rejected.

It would create a second financial authority and make reconciliation unreliable.

## Alternative C — One giant transaction containing payment/EIS provider calls

Rejected.

It couples database locks to external latency and produces fragile unknown-outcome behavior.

## Alternative D — Event sourcing for every POS record

Not required as a blanket architecture. Immutable financial facts and explicit inventory postings provide the necessary auditability without forcing all mutable configuration into event sourcing.

## Alternative E — Microservice-based POS from day one

Rejected.

POS, inventory and transactional business rules benefit from transaction locality. The existing architecture deliberately starts as a modular monolith and extracts services only when the scaling/failure/compliance boundary is justified.

## Alternative F — Generic policy engine for every POS rule

Rejected for initial implementation.

Authorization policy belongs to Phase 6, but monetary calculation, inventory invariants and sale state transitions remain strongly typed Rust domain behavior. A remote policy engine should not become a mandatory network hop for every checkout operation without measured justification.

---

# 86. Phase 10 Exit Criteria

Phase 10 is complete only when every item below is satisfied:

```text
[ ] Sale aggregate implemented
[ ] Sale state machine implemented
[ ] Sale line semantics implemented
[ ] Server-authoritative price resolution implemented
[ ] Discount policy implemented
[ ] Tax calculation boundary implemented
[ ] Register/cashier context implemented
[ ] Finalization transaction implemented
[ ] Inventory ledger linkage implemented
[ ] Idempotency implemented atomically
[ ] Payment/tender boundary implemented
[ ] Customer scope implemented
[ ] Object/property/function authorization integrated
[ ] Offline command boundary integrated
[ ] Audit evidence implemented
[ ] Outbox integrated
[ ] Receipt/read models implemented
[ ] PostgreSQL constraints implemented
[ ] RLS integration implemented
[ ] Concurrency suite passes
[ ] Tenant-isolation suite passes
[ ] Authorization negative suite passes
[ ] Replay/idempotency suite passes
[ ] Offline adversarial suite passes
[ ] Real PostgreSQL integration suite passes
[ ] Mutation tests meet target
[ ] Fuzz suite passes
[ ] Query plans reviewed
[ ] Performance budget reviewed
[ ] Runbooks exist
[ ] CI release gates are blocking
[ ] Documentation and route inventory agree
```

A successful build is not enough. The existing Sitolo threat model explicitly rejects “a successful build means a secure release” and requires executable evidence for critical threats. fileciteturn9file0L39-L80

---

# 87. Final Architectural Position

The POS architecture is intentionally strict because POS is where Sitolo's abstract business model becomes a real financial and inventory event.

The final model is:

```text
                         AUTHENTICATED ACTOR
                                  |
                                  v
                        AUTHORITATIVE CONTEXT
                                  |
                                  v
                         AUTHORIZATION ENGINE
                                  |
                                  v
                           SALES COMMAND
                                  |
             +--------------------+--------------------+
             |                    |                    |
             v                    v                    v
        CATALOGUE             INVENTORY             PAYMENT
        RESOLUTION            VALIDATION             CONTEXT
             |                    |                    |
             +--------------------+--------------------+
                                  |
                                  v
                         AUTHORITATIVE TOTALS
                                  |
                                  v
                         POSTGRES TRANSACTION
                                  |
        +-------------------------+-------------------------+
        |                         |                         |
        v                         v                         v
      SALE                  INVENTORY POSTING            PAYMENT LINK
        |                         |                         |
        +-------------------------+-------------------------+
                                  |
                                  v
                              AUDIT
                                  |
                                  v
                              OUTBOX
                                  |
                                  v
                    WORKERS / EIS / REPORTING
```

The core principle is simple but non-negotiable:

> **A POS sale is complete only when Sitolo has durably established an authorized, internally consistent, idempotent, auditable financial event and its corresponding inventory effect according to the current business policy.**

Everything else — receipt rendering, notifications, provider reconciliation, reporting and analytics — is downstream of that authoritative event.

---

# 88. Traceability to Existing Security Controls

Phase 10 materially exercises:

```text
4   authentication boundary
5   authorization
6   cross-user access
7   database privilege
9   privileged/admin route protection
12  safe errors
15  client-only security prevention
16  input validation
24  session/revocation relationship
25  session security
28  rate/resource limits
33  IDOR/BOLA
34  API input and command safety
37  MFA/step-up for privileged actions
39  business-logic abuse
40  race conditions
45  fail-closed behavior
46  timeouts/resource controls
48  endpoint inventory
```

The dedicated security-test framework must connect these controls to executable tests rather than leaving the mapping as documentation only.

---

# 89. Implementation Anti-Patterns Explicitly Prohibited

```text
"The app already calculated the total."

"The cashier role is trusted."

"The UUID is hard to guess."

"Inventory is just a balance column."

"The provider returned 200 so payment succeeded."

"The offline database is authoritative until tomorrow."

"RLS handles all authorization."

"The frontend won't expose the override field."

"We'll add idempotency later."

"We can just retry the payment."

"We'll edit the sale row to fix mistakes."

"The test passes, but concurrency isn't realistic."

"The provider call can happen inside the transaction."
```

None of these are acceptable engineering arguments for production POS behavior.

---

# 90. Final Phase 10 Checklist

```text
ARCHITECTURE
[X] POS remains inside modular monolith
[X] PostgreSQL remains authority
[X] Phase 9 owns inventory truth
[X] Phase 11 owns provider reconciliation
[X] Phase 12 owns synchronization semantics
[X] Phase 15 owns EIS integration

DOMAIN
[ ] Sale aggregate
[ ] Line model
[ ] State machine
[ ] Totals
[ ] Discount rules
[ ] Tax boundary
[ ] Tender semantics
[ ] Register context

SECURITY
[ ] Authentication enforcement
[ ] Authorization enforcement
[ ] Object-level authorization
[ ] Property-level authorization
[ ] Branch scope
[ ] Device/session freshness
[ ] Step-up/approval
[ ] Audit

DATABASE
[ ] Sale schema
[ ] Sale-line schema
[ ] Idempotency constraints
[ ] Cross-tenant constraints
[ ] RLS
[ ] Indexes
[ ] Transaction boundaries

CONCURRENCY
[ ] Sale vs sale
[ ] Sale vs adjustment
[ ] Sale vs transfer
[ ] Finalize vs revoke
[ ] Payment race

OFFLINE
[ ] Draft offline
[ ] Command queue
[ ] Replay
[ ] Conflict handling
[ ] Stale pricing
[ ] Revoked device

RELEASE
[ ] Real PostgreSQL tests
[ ] Negative auth tests
[ ] Mutation tests
[ ] Fuzz tests
[ ] Performance tests
[ ] Migration tests
[ ] CI release gate
[ ] Runbooks
```

**Status:** Ready to serve as the implementation-governing Phase 10 POS/Sales specification, subject to explicit closure of the project decisions marked as open in the higher-level contracts.
