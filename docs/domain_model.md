# SITOLO — Domain Model Design

**Document status:** Phase 0 implementation contract
**Prepared:** 2026-09-04
**Authority:** Derived from the existing Sitolo product, business-model, system-architecture and security architecture documents

---


# SITOLO — Domain Model Design

**Document:** `domain_model.md`  
**Phase:** 0 — Architecture / Contracts / ADR Freeze  
**Sequence:** File 02 of 16  
**Status:** Implementation-governing domain specification  
**Product:** Sitolo — Business Operating System for African SMEs  
**Primary market:** Malawi first; controlled African regional expansion  
**Backend authority:** Rust + Axum + Tokio  
**Primary persistence:** PostgreSQL  
**Offline store:** SQLite  
**Primary mobile surface:** Flutter / Android-first  
**Desktop:** Tauri  
**Web/admin:** TypeScript where materially useful  
**Architecture:** Modular monolith first, with workers and adapters; service extraction only when an independently justified boundary exists

> This document defines the **business domain model** that implementation code must preserve. It is not a database dump, not an API list, and not a collection of CRUD entities. The goal is to identify the concepts, ownership boundaries, invariants, state transitions, aggregate boundaries, commands, events, value objects, and cross-domain contracts that make Sitolo economically correct and operationally trustworthy.

The domain model is derived from the existing Sitolo product specification, business model, system architecture, and security architecture. The existing product defines Sitolo as a mobile-first, offline-capable, multi-tenant operating system for shops, small retailers, pharmacies, agro-dealers and growing multi-branch businesses. The core economic loop is `PROCURE → RECEIVE → STOCK → PRICE → SELL → COLLECT → RECONCILE → REPORT`, with audit and synchronization operating across the loop.

The implementation principle is:

```text
BUSINESS LANGUAGE
      ↓
DOMAIN MODEL
      ↓
DOMAIN INVARIANTS
      ↓
APPLICATION COMMANDS
      ↓
TRANSACTIONS / REPOSITORIES
      ↓
POSTGRESQL AUTHORITY
      ↓
EVENTS / OUTBOX / READ MODELS
      ↓
CLIENTS + INTEGRATIONS
```


# 1. Design Objectives

The domain model must accomplish seven things simultaneously:

1. Represent real merchant operations without turning every screen into a domain concept.
2. Preserve financial and inventory correctness under concurrency, retries, offline work, and provider failure.
3. Make tenant and organizational scope explicit.
4. Keep business authority in Rust rather than duplicating business rules in Flutter, Tauri or TypeScript.
5. Provide stable seams for later extraction without prematurely becoming microservices.
6. Support regulated vertical extensions without contaminating the retail core with pharmacy- or agro-specific assumptions.
7. Make security controls expressible as domain invariants rather than relying only on middleware.

Sitolo is intentionally not modeled as a collection of mutable tables. Domain objects own behavior. A finalized sale, for example, is not just a row with editable fields; it is a business fact whose correction occurs through reversal, return, refund or another explicit compensating action.

This aligns with domain-driven design's emphasis on explicit models and bounded contexts rather than a single undifferentiated model. Fowler's DDD references describe bounded contexts as a way to divide large models and make relationships explicit, and his discussion of aggregates emphasizes keeping related objects consistent as a unit. [Research: https://martinfowler.com/bliki/BoundedContext.html ; https://martinfowler.com/articles/pushing-ai-autonomy.html]


# 2. Source-of-Truth Hierarchy

When two documents or implementation decisions conflict, use the following priority:

```text
1. Legal / regulatory requirements actually applicable to deployment
2. Signed external provider contracts / current provider API behavior
3. Security architecture invariants
4. System architecture invariants
5. Domain model decisions in this document
6. Product requirements
7. UI assumptions / implementation convenience
```

The business model remains the commercial authority, but commercial hypotheses do not override technical invariants. For example, “checkout must be one tap” cannot justify making the backend trust a client-calculated payment amount.

Existing Sitolo architecture decisions that this model MUST preserve include:

- dedicated Flutter mobile application as the primary merchant surface;
- Tauri for desktop back office;
- limited TypeScript for web/admin/tooling;
- Rust as the business decision engine;
- PostgreSQL as authoritative server state;
- SQLite as local operational continuity storage;
- modular monolith first;
- custom offline command/synchronization semantics;
- transactional outbox before an external broker;
- append-only treatment of finalized financial facts;
- provider adapters for payments, EIS and other external systems;
- authorization based on tenant and organizational scope.

The architecture explicitly states that clients are operational surfaces, Rust is the decision engine, PostgreSQL is authoritative business storage, synchronization must converge safely, and financial history is corrected through compensating events rather than destructive edits. Those rules are foundational to this domain model.


# 3. Domain Modeling Method

### 3.1 Concepts

A **domain concept** exists because merchants, staff, operators, regulators or integrations need to reason about it. If an object exists only because a database requires a join table, it is not automatically a first-class domain concept.

### 3.2 Entities

An **entity** has a stable identity across changes. Examples include `Organization`, `Product`, `Sale`, `PurchaseOrder`, `InventoryLot`, `PaymentIntent`, `Device`, and `User`.

### 3.3 Value objects

A **value object** is defined by its value rather than identity. Sitolo should use value objects aggressively for high-risk primitives such as `Money`, `Quantity`, `Sku`, `Barcode`, `PhoneNumber`, `TaxRate`, `DateRange`, and `PermissionScope`.

### 3.4 Aggregates

An **aggregate** is a consistency boundary. Code should mutate an aggregate through its root or through an application service that owns the transaction. Aggregates are not merely “large objects”; they are the smallest boundaries within which business invariants must be guaranteed atomically.

### 3.5 Bounded contexts

A **bounded context** owns a model for a particular business capability and defines what it publishes to other contexts. Two contexts may use the word “customer” but mean different things. The sales context cares about customer identity and transaction association; billing may care about payer/account status; support may care about contactability. Avoid forcing a giant universal customer class.

### 3.6 Domain events

A domain event records a fact that has already occurred. Events do not become a second source of truth merely because they are useful for asynchronous work. PostgreSQL remains authoritative for transactional state; outbox messages publish committed facts to downstream consumers.

### 3.7 Commands

A command expresses intent: `CreateSale`, `FinalizeSale`, `ApproveRefund`, `ReceiveGoods`, `AdjustStock`, `CloseRegister`. A command is not a database operation and should not expose persistence details.


# 4. Bounded Context Map

Sitolo should use the following logical bounded contexts inside the modular monolith:

```text
                           ┌─────────────────────┐
                           │ Identity & Access    │
                           └──────────┬──────────┘
                                      │
                                      ▼
                           ┌─────────────────────┐
                           │ Tenant / Org        │
                           └───────┬─────┬───────┘
                                   │     │
                     ┌─────────────┘     └─────────────┐
                     ▼                                 ▼
              ┌─────────────┐                  ┌─────────────┐
              │ Catalogue   │◀────────────────▶│ Pricing     │
              └──────┬──────┘                  └──────┬──────┘
                     │                                │
                     ▼                                ▼
              ┌─────────────┐                  ┌─────────────┐
              │ Procurement │─────────────────▶│ Inventory   │
              └─────────────┘                  └──────┬──────┘
                                                      │
                                                      ▼
                                               ┌─────────────┐
                                               │ Sales / POS │
                                               └──────┬──────┘
                                                      │
                             ┌────────────────────────┼────────────────┐
                             ▼                        ▼                ▼
                       ┌───────────┐            ┌───────────┐   ┌──────────────┐
                       │ Cash      │            │ Payments  │   │ Tax / EIS    │
                       └─────┬─────┘            └─────┬─────┘   └──────┬───────┘
                             │                        │                │
                             └────────────┬───────────┴────────────────┘
                                          ▼
                                   ┌─────────────┐
                                   │ Reconcile   │
                                   └──────┬──────┘
                                          │
                                          ▼
                              ┌───────────────────────┐
                              │ Reporting / Analytics │
                              └───────────────────────┘

Cross-cutting contexts:
  Audit / Compliance / Notifications / Billing / Platform Admin / Integrations

Vertical extensions:
  Pharmacy / Agro-dealer / Wholesale
```

### Context responsibilities

| Context | Owns | Does not own |
|---|---|---|
| Identity | principals, authentication references, sessions, MFA, device identity | merchant inventory or sales authority |
| Tenant | organizations, business entities, branches, locations, registers | product economics |
| Authorization | permissions, roles, scopes, approval policy | domain transaction state |
| Catalogue | sellable products, SKUs, units, classifications | current stock balance |
| Pricing | price policies, price versions, promotions, discount rules | finalized sale totals |
| Procurement | suppliers, POs, receipts, procurement lifecycle | final sale records |
| Inventory | stock ledger, lots, locations, movements, counts | payment settlement |
| Sales | carts/drafts, sales, lines, tax/price snapshots, returns eligibility | external payment confirmation |
| Cash | registers, sessions, cash events, closes, variance | mobile-money provider ledger |
| Payments | intents, provider references, callbacks, internal payment state | product pricing truth |
| Reconciliation | matching, exceptions, settlement evidence | originating transaction semantics |
| Tax/EIS | tax configuration snapshot, tax submission state, EIS evidence | core sale economics |
| Audit | immutable audit evidence | authorization decisions themselves |
| Reporting | read models and derived metrics | canonical financial truth |
| Billing | plans, entitlements, metering, subscriptions | merchant operational transactions |
| Platform Admin | Sitolo control-plane operations | silent merchant-data mutation |
| Integrations | adapter contracts and boundary state | provider-specific internals outside adapter |
| Pharmacy | regulated inventory/dispensing extensions | generic retail rules |
| Agro | batch/formulation/unit extensions | generic retail stock semantics |


# 5. Aggregate Strategy

The aggregate strategy is deliberately conservative. Overly large aggregates create contention; overly small aggregates make invariants impossible to protect atomically.

### Core aggregates

| Aggregate root | Primary consistency responsibilities | Typical mutation path |
|---|---|---|
| `Organization` | lifecycle, core configuration, organizational identity | Tenant application service |
| `Membership` | user ↔ organization relationship, role/scope | IAM service |
| `Branch` | branch lifecycle and operational configuration | Tenant service |
| `RegisterSession` | register opening/closing and local cash session state | Cash service |
| `Product` / `Sku` | product identity and sellable classification | Catalogue service |
| `PriceList` | coherent price-version changes | Pricing service |
| `PurchaseOrder` | PO lifecycle and approval state | Procurement service |
| `GoodsReceipt` | receiving facts and posting status | Procurement service |
| `InventoryPosition` | current balance for a SKU/location/lot policy boundary | Inventory service + SQL transaction |
| `InventoryTransfer` | paired transfer semantics | Inventory service |
| `StockCount` | count lifecycle and approval/posting | Inventory service |
| `Sale` | sale lifecycle, line snapshots, totals, eligibility for corrections | Sales service |
| `Return` | returned goods and financial relationship to original sale | Sales service |
| `Refund` | money-return intent and state | Payments/Sales coordination |
| `PaymentIntent` | expected payment, lifecycle, provider linkage | Payments service |
| `CashEvent` | immutable cash movement fact | Cash service |
| `ReconciliationCase` | ambiguous payment/settlement resolution | Reconciliation service |
| `Device` | device lifecycle and revocation status | Identity/device service |
| `Subscription` | subscription lifecycle and entitlements | Billing service |
| `ExportJob` | export authorization, execution, expiration | Reporting service |

### What should NOT be one aggregate

`Sale`, `InventoryPosition`, `PaymentIntent`, and `RegisterSession` must not become one aggregate merely because checkout touches all four. They have distinct lifecycles and contention patterns. Checkout orchestration coordinates them in a database transaction without pretending their identities and histories are the same thing.

Similarly, `Organization` must not contain every branch, product and user as an in-memory aggregate that must be loaded to make a change. Organization is a consistency boundary for its own lifecycle and selected configuration, not a giant object graph.


# 6. Universal Value Objects

The following value objects should exist in Rust domain code where practical. The exact newtype representation can evolve, but semantic distinctions must remain.

### Identity and scope

```text
TenantId
OrganizationId
BusinessEntityId
BranchId
LocationId
WarehouseId
RegisterId
UserId
MembershipId
DeviceId
SessionId
RoleId
PermissionId
```

### Product and stock

```text
ProductId
Sku
Barcode
CategoryId
UnitOfMeasure
Quantity
ConversionFactor
LotId
SerialNumber
```

### Money

```text
Money
CurrencyCode
TaxRate
DiscountRate
Percentage
AmountMinor
```

Money must not use IEEE floating-point arithmetic for persisted financial truth. A value object should make illegal currencies, negative amounts and incompatible arithmetic difficult to represent.

### Time

```text
Timestamp
Date
DateRange
BusinessDay
ExpirationDate
ValidityWindow
```

Server time is authoritative for server-side security-sensitive decisions. Client time remains evidence and can be retained separately for offline operations.

### Workflow / integration

```text
SaleId
SaleNumber
PurchaseOrderId
ReceiptId
InventoryMovementId
PaymentIntentId
ProviderTransactionId
ProviderEventId
CommandId
EventId
IdempotencyKey
CorrelationId
TraceId
```

### Domain-specific values

```text
TaxCategory
PaymentMethod
DiscountReason
AdjustmentReason
ReturnReason
RefundReason
InventoryState
ApprovalThreshold
ExternalReference
```

These should be enums or constrained value types where the domain has a closed vocabulary.


# 7. Money and Monetary Semantics

Money is a first-class domain concern because the product controls sales, refunds, cash, payment intents and reconciliation.

### Rules

1. Every monetary value has a currency.
2. Arithmetic between different currencies is prohibited unless an explicit FX operation exists.
3. Currency conversion is not implicit.
4. Rounding policy is explicit and versioned where required.
5. A finalized sale stores monetary snapshots rather than dynamically recalculating historical values from current price/tax configuration.
6. Refund eligibility is derived from authoritative finalized transaction state.
7. Client-supplied totals are advisory input, never final authority.
8. Money must be representable exactly using integer minor units or an exact decimal strategy appropriate to the currency.
9. Tax, discount and net calculations must use deterministic rounding rules.
10. The domain must preserve enough calculation evidence to explain why a total exists.

Example:

```text
Unit price:     1,250 MWK
Quantity:       3
Gross:          3,750 MWK
Discount:         100 MWK
Taxable base:   3,650 MWK
Tax:              547 MWK
Final total:    4,197 MWK
```

The exact tax treatment is configuration/domain policy, but once a sale is finalized, the applied inputs and resulting values become historical facts.

PostgreSQL constraints should enforce basic monetary validity where possible. PostgreSQL documents `CHECK`, `NOT NULL`, `UNIQUE`, primary-key and foreign-key constraints as mechanisms for preventing invalid stored states; use them as defense in depth rather than relying solely on application code. [Research: https://www.postgresql.org/docs/current/ddl-constraints.html]


# 8. Business Hierarchy

Sitolo's canonical hierarchy is:

```text
Platform
└── Organization
    ├── Business Entity / Legal Profile
    ├── Memberships
    ├── Roles / Policies
    ├── Branches
    │   ├── Locations
    │   ├── Warehouses
    │   ├── Registers
    │   └── Devices
    ├── Catalogue
    ├── Pricing
    ├── Procurement
    ├── Inventory
    ├── Sales
    ├── Cash
    ├── Payments
    ├── Tax configuration
    ├── Customers / Suppliers
    ├── Integrations
    └── Reports / Audit
```

### Tenant semantics

The organization is the default tenant boundary. A user may belong to multiple organizations. Current organization scope must be derived from authenticated membership and policy, not trusted from arbitrary client values.

### Business entity semantics

An organization can represent an operating business while the legal profile carries registration/tax identifiers and other jurisdiction-specific information. Do not assume every micro-retailer has complex corporate structures, but do not force the architecture to equate “one account” with “one branch.”

### Branch semantics

A branch is an operational unit. It can own locations, warehouses, registers, staff scope and branch-specific pricing/configuration.

### Location semantics

A location represents a physical or logical stock-holding/selling point. Warehouse and selling location are different concepts even if a micro-shop initially uses the same physical room.

### Register semantics

A register is a selling/cash-control endpoint. It may have a device association and a cash session history. It is not the same as the employee operating it.

### Device semantics

Device identity is security/control-plane data. The device is not a user and not a credential. Device compromise must be independently revocable.


# 9. Identity Domain

### User

A `User` represents a human identity known to Sitolo. It is intentionally separate from organization membership because the same person may legitimately have relationships with more than one organization.

Relevant concepts:

```text
User
 ├── authentication identity reference
 ├── profile
 ├── verification state
 ├── security state
 └── organization memberships
```

### Membership

`Membership` connects a user to an organization and defines baseline role/scope. Membership is the bridge between global identity and tenant authority.

### Device

```text
Device
 ├── organization
 ├── device identity
 ├── registration state
 ├── last-seen
 ├── app version
 ├── scope/profile
 └── revocation state
```

### Session

Sessions represent active authenticated access. A session belongs to a user and may be bound to a device. Session revocation is separate from membership revocation.

### Domain invariant

```text
A user with no active membership in organization X cannot exercise organization X business authority,
even if they possess a previously valid object ID, cached UI state, or stale client-side role.
```

Identity context supplies facts to authorization; it does not itself decide whether a business operation is legal.


# 10. Tenant and Authorization Domain Model

Authorization is a first-class domain concern because Sitolo has owners, managers, cashiers, inventory staff, procurement staff, finance users, pharmacists, auditors, support personnel and platform operators.

### Authorization tuple

Conceptually:

```text
Principal
  + Organization Scope
  + Branch / Resource Scope
  + Permission
  + Target Resource
  + Target State
  + Context / Threshold
  = Authorization Decision
```

### Role is not permission

A role is a named bundle of permissions. A permission represents a capability. Scope narrows where the capability applies.

Example:

```text
Role: Cashier
Permission: SALE_CREATE
Scope: Branch: AREA_25 / Registers: 1,2
```

### High-risk operations

Examples requiring explicit policy:

- large refund;
- price override above threshold;
- stock adjustment above threshold;
- role assignment;
- MFA reset for another user;
- payout/payment credential change;
- bulk export;
- tenant ownership change.

### Domain authorization invariant

No domain command may be accepted merely because the UI exposed it. The backend must receive an authenticated principal and a trusted scope context, then evaluate the permission and the domain state.

The security architecture explicitly treats tenant isolation, function-level authorization, property-level authorization, object authorization and state-machine validation as separate enforcement layers.


# 11. Catalogue Domain

The catalogue defines **what can be sold or stocked**, not how much exists.

### Product

A product is a conceptual commercial item.

Typical properties:

```text
ProductId
OrganizationId
Name
Description
Category
Brand
LifecycleState
DomainClassification
```

### SKU

A SKU is the concrete sellable/inventoriable item. This is the important identity for stock and pricing.

```text
Product
└── SKU
    ├── barcode(s)
    ├── unit
    ├── base unit
    ├── sale unit
    ├── purchase unit
    ├── conversion
    └── attributes
```

### Barcode

A SKU may have several barcodes. Barcode uniqueness is tenant-scoped unless the business explicitly allows shared global identifiers.

### Product lifecycle

```text
DRAFT → ACTIVE → DISCONTINUED → ARCHIVED
```

Discontinued does not mean historical references disappear. A historical sale must continue to resolve its commercial snapshot even when the SKU is no longer actively sold.

### Deletion rule

Do not hard-delete a product/SKU that participates in historical business records. Prefer lifecycle state + archival semantics.

### Catalogue invariants

- SKU belongs to exactly one organization.
- SKU must have a valid unit model before it can enter active inventory.
- An active sellable SKU must have the minimum information required by the business type.
- Barcode collisions inside a tenant are rejected.
- Historical records never depend on current mutable product names/prices to preserve their meaning.


# 12. Units and Quantity Model

Quantity is a value object, not a naked decimal.

### Why

A Malawian retailer may purchase a carton, receive individual pieces, sell pieces, or sell kilograms from bagged stock. Pharmacies need packages/tablets/units. Agro-dealers may deal with bags, litres, kilograms and formulation-specific units.

### Model

```text
Base Unit
   ↑
Conversion Factor
   ↑
Commercial Unit
```

Example:

```text
1 carton = 24 pieces
1 box    = 10 blister packs
1 bag    = 50 kg
```

### Invariants

- Conversion factors are positive.
- A unit conversion is explicit and versioned where changing it could alter existing inventory semantics.
- Historical stock movements store the quantity/unit semantics that applied at posting time.
- Rounding rules for fractional units are defined per product/unit, never silently applied.

Never let a generic “quantity = number” field decide whether fractional selling is legal.


# 13. Pricing Domain

Pricing answers: **what should this customer/location pay now?** It does not own historical sale totals.

### Price list

A `PriceList` is a named pricing policy/scope.

Possible scope:

```text
Organization
Branch
Customer segment
Product/SKU
Time window
```

### Price version

Prices are versioned effective records:

```text
PriceVersion
 ├── amount
 ├── currency
 ├── valid_from
 ├── valid_to
 ├── price_list
 ├── product/SKU
 └── provenance
```

### Resolution precedence

Default strategy from existing Sitolo design:

```text
Explicit approved override
        ↓
Customer-specific active price
        ↓
Promotion
        ↓
Branch price
        ↓
Organization default
```

### Discount model

Discounts require:

- source;
- actor;
- reason where required;
- threshold;
- authorization;
- resulting amount;
- audit evidence.

### Critical invariant

Changing a price today must never rewrite the economics of a finalized sale yesterday.


# 14. Promotion Domain

Promotions are controlled pricing rules rather than ad-hoc UI math.

A promotion can define:

```text
eligibility conditions
start/end window
SKU/product/category scope
quantity thresholds
fixed/percentage discount
stacking policy
customer segment
branch scope
approval requirement
```

### Promotion evaluation

The pricing engine should produce a deterministic `PricingDecision` that explains:

```text
base price
promotion(s) considered
promotion(s) applied
manual override
authorization evidence
final line price
```

### Safety

- No promotion can yield a negative sale price unless explicitly supported by business policy.
- Promotion stacking is closed by policy, not accidental ordering.
- Expired promotions do not apply merely because cached client data says they are active.
- Historical sales keep applied pricing snapshots.


# 15. Procurement Domain

Procurement represents inbound commercial intent and receiving evidence.

### Supplier

Supplier is an organizational business counterparty, separate from inventory and purchasing transactions.

### Purchase Order aggregate

```text
PurchaseOrder
 ├── supplier
 ├── branch / receiving location
 ├── lines
 ├── expected quantities
 ├── expected costs
 ├── approval metadata
 ├── lifecycle state
 └── references
```

### Lifecycle

```text
DRAFT
 ↓
SUBMITTED
 ↓
APPROVED
 ↓
SENT
 ↓
PARTIALLY_RECEIVED
 ↓
RECEIVED
 ↓
CLOSED
```

Cancellation is a state transition, not destructive deletion.

### Goods Receipt

Goods receiving is a separate aggregate because physical receipt can differ from ordered quantity, arrive in several deliveries, contain discrepancies, or require lot/expiry data.

```text
PurchaseOrder
     │
     ├── expected
     ▼
GoodsReceipt
     ├── actual quantity
     ├── lot/batch
     ├── expiry
     ├── cost
     ├── discrepancies
     └── evidence
```

Posting a receipt creates inventory movements; it does not directly manipulate a mutable “stock count” without ledger evidence.


# 16. Inventory Domain — Core Model

Inventory is a **ledger-backed state projection**.

The conceptual equation is:

```text
Opening Balance
+ Purchase Receipts
+ Positive Adjustments
+ Transfers In
+ Customer Returns
- Sales
- Damage
- Expiry
- Supplier Returns
- Transfers Out
= Derived Available Quantity
```

### Inventory dimensions

At minimum the model can distinguish:

```text
Organization
Branch
Location / Warehouse
SKU
Lot / Batch (when applicable)
Inventory State
Unit
```

### Inventory states

```text
AVAILABLE
RESERVED
DAMAGED
QUARANTINED
EXPIRED
RECALLED
RETURNED
PENDING_INSPECTION
```

Not every SKU uses every state. Product/vertical policy controls applicability.

### Stock movement

A stock movement is an immutable operational fact:

```text
InventoryMovement
 ├── movement_id
 ├── organization_id
 ├── location_id
 ├── sku_id
 ├── lot_id?
 ├── movement_type
 ├── quantity
 ├── unit
 ├── occurred_at
 ├── source_type
 ├── source_id
 ├── actor/device
 └── reason/evidence
```

### Balance projection

A current `inventory_balance` projection can exist for fast availability checks, but the ledger remains the evidence needed to reconstruct why the balance exists.


# 17. Inventory Movement Types

Canonical movement vocabulary:

```text
OPENING_BALANCE
PURCHASE_RECEIPT
SALE
SALE_REVERSAL
CUSTOMER_RETURN
SUPPLIER_RETURN
STOCK_ADJUSTMENT_IN
STOCK_ADJUSTMENT_OUT
DAMAGE
EXPIRY
TRANSFER_IN
TRANSFER_OUT
STOCK_COUNT_RECONCILIATION
```

Extensions may be introduced only when they represent a materially different domain fact.

### No “magic adjustment”

Every adjustment needs a reason and actor. High-risk adjustments can require approval.

### Transfer semantics

An internal transfer is one business transaction with two linked inventory movements:

```text
Source Location: -Q
Destination:     +Q
Transfer:         one aggregate / one business identity
```

The two postings must not create an intermediate state visible as completed if only one side succeeded.

### Reversal semantics

A reversal does not mutate the original movement. It posts a compensating movement linked to the original.


# 18. Inventory Concurrency Model

The last unit problem is domain logic, not merely SQL tuning.

Example competing requests:

```text
Device A: sell 1 × SKU-X
Device B: sell 1 × SKU-X
Available: 1
```

Exactly one successful committed sale may consume the last available unit under the default non-negative-stock policy.

The critical section is:

```text
BEGIN
  resolve tenant/object authorization
  acquire deterministic row locks / atomic update
  re-read authoritative state
  validate availability
  post sale movement
  update balance projection
  write audit/outbox
COMMIT
```

The existing architecture specifically rejects application-process memory locks as the primary concurrency mechanism because multiple API instances do not share memory.

PostgreSQL provides row/table locking and constraints that are appropriate for such invariants. The database model must complement application logic with constraints and atomic statements where possible.


# 19. Stock Count Domain

Stock counting is a controlled reconciliation process, not a direct edit to quantity.

### Lifecycle

```text
PLANNED
 ↓
OPEN
 ↓
COUNTING
 ↓
SUBMITTED
 ↓
REVIEW
 ↓
APPROVED
 ↓
POSTED
```

### Aggregate responsibilities

`StockCount` owns:

- counting scope;
- counted quantities;
- count version;
- who counted;
- who reviewed;
- approval evidence;
- resulting reconciliation movements.

### Invariant

Posting a stock count does not erase prior ledger history. It creates explicit reconciliation movements.


# 20. Sales / POS Domain

Sales are the central revenue-producing business aggregate.

### Sale composition

A finalized sale contains immutable commercial facts:

```text
Sale
 ├── sale_id
 ├── sale_number
 ├── organization
 ├── branch/location
 ├── register/device
 ├── seller
 ├── customer reference?
 ├── lines[]
 │   ├── SKU
 │   ├── quantity
 │   ├── unit price snapshot
 │   ├── discount snapshot
 │   ├── tax snapshot
 │   └── line total
 ├── totals
 ├── currency
 ├── payment allocation
 ├── occurrence timestamp
 ├── source command/device
 └── lifecycle state
```

### Sale lifecycle

```text
DRAFT
 ↓
PENDING_CONFIRMATION
 ↓
FINALIZED
 ↓
[SETTLED | PARTIALLY_SETTLED | UNSETTLED]
```

Sale state and payment state are intentionally separate. A completed sale can exist while payment remains pending or partially settled depending on the business workflow.

### Sale finalization invariant

A sale cannot finalize unless:

- actor is authorized;
- branch/register/device scope is valid;
- line items are valid;
- quantities are valid;
- product/SKU lifecycle permits the sale;
- pricing decision is valid;
- tax policy is valid where required;
- inventory policy permits the movement;
- totals reconcile;
- idempotency/command identity is valid;
- transaction can atomically commit the necessary authoritative effects.


# 21. Sale Line Semantics

A sale line is a historical commercial snapshot, not a live pointer to current pricing.

The line should preserve enough evidence to answer:

> What exactly did the merchant sell, at what quantity and price, under which discount and tax assumptions, and why was the line total what it was?

Suggested fields:

```text
SaleLineId
SaleId
SkuId
DescriptionSnapshot
UnitCode
Quantity
UnitPrice
DiscountAmount
TaxCategorySnapshot
TaxRateSnapshot
TaxAmount
NetAmount
GrossAmount
```

The `DescriptionSnapshot` is not redundant: product names can change after the sale.

Historical correctness should not depend on joining current product/pricing rows and hoping nothing changed.


# 22. Returns, Voids, Refunds and Reversals

These are deliberately distinct concepts.

### Void

Cancels a transaction while it is still in a pre-final state according to the sale state machine.

### Reversal

Creates a compensating financial/operational fact for an already finalized transaction.

### Return

Represents goods moving back through the inventory/commercial flow.

### Refund

Represents money being returned to the customer.

### Relationship

A single customer interaction may create multiple domain facts:

```text
Original Sale
   │
   ├── Return ─────────→ Inventory movement
   │
   └── Refund ─────────→ Payment movement
```

Not every return is automatically a full refund and not every refund necessarily means physical inventory returned. Policies define allowed combinations.

### Core invariants

```text
refund_amount <= eligible_refund_amount
returned_quantity <= returnable_quantity
reversal cannot duplicate an already consumed correction entitlement
completed sale cannot be rewritten into a different economic meaning
```


# 23. Cash Management Domain

Cash is its own bounded context because expected sale receipts and physically counted cash are different facts.

### RegisterSession lifecycle

```text
CLOSED
 ↓
OPENING
 ↓
OPEN
 ↓
COUNT_PENDING
 ↓
CLOSED
```

### Cash events

```text
OPENING_FLOAT
CASH_SALE
CASH_REFUND
CASH_IN
CASH_OUT
CASH_ADJUSTMENT
COUNT
CLOSE
```

### Expected cash equation

```text
Expected Cash
= Opening Float
+ Cash Sales
+ Cash In
- Cash Refunds
- Cash Out
+/- approved adjustments according to policy
```

The physical counted value is separate. Variance is preserved as an explicit record and may require approval above threshold.

### Invariant

A register session may not have two successful closes. A closed session cannot silently reopen and rewrite historical cash events.


# 24. Payment Domain

Payments represent money movement intent and evidence across internal and external rails.

Supported method taxonomy:

```text
CASH
MOBILE_MONEY
BANK_TRANSFER
CARD
CREDIT_ACCOUNT
OTHER
```

Provider-specific details remain behind adapter boundaries.

### PaymentIntent

```text
PaymentIntent
 ├── payment_intent_id
 ├── organization_id
 ├── sale_id
 ├── expected_amount
 ├── currency
 ├── method
 ├── provider
 ├── status
 ├── client_reference
 └── provider references
```

### Lifecycle

```text
INITIATED
 ↓
PENDING
 ├── SUCCEEDED
 ├── FAILED
 ├── EXPIRED
 └── CANCELLED
```

### Authority

The frontend can say “I started payment.” It cannot say “payment succeeded.” Provider evidence and backend reconciliation determine the authoritative state.

### Uniqueness

Provider transaction references and event identifiers require database-level uniqueness where the provider contract permits.


# 25. Reconciliation Domain

Reconciliation exists because external money systems and internal business records are not guaranteed to arrive in the same order, with the same identifiers, or with the same availability.

### ReconciliationCase

A reconciliation case captures uncertainty rather than hiding it.

```text
OPEN
 ↓
MATCHING
 ├── MATCHED
 ├── AMBIGUOUS
 ├── MISMATCH
 └── EXCEPTION
 ↓
RESOLVED
```

### Matching priority

Existing Sitolo design establishes:

```text
Provider transaction ID
    ↓
Provider contractual reference
    ↓
Sitolo payment intent reference
    ↓
Controlled exact-match fallback
    ↓
Manual review
```

Never silently settle an ambiguous payment.

### Domain principle

`UNKNOWN` is a valid state. The system must not convert uncertainty into a false success merely to make dashboards look clean.


# 26. Tax / EIS Domain

Tax integration is a regulated external boundary. The tax domain inside Sitolo should represent Sitolo's own tax configuration and evidence/state around submissions, not impersonate the authority.

### Internal concepts

```text
TaxProfile
TaxCategory
TaxRuleVersion
TaxConfigurationSnapshot
TaxSubmission
TaxReceiptEvidence
TaxException
```

### TaxSubmission lifecycle

```text
NOT_REQUIRED
 ↓
PENDING
 ↓
SUBMITTING
 ├── SUBMITTED
 ├── RETRYABLE_FAILURE
 └── PERMANENT_REJECTION
```

### Important boundary

A tax submission failure must not mutate the commercial sale to make it appear that the external authority accepted the transaction.

The existing architecture's MRA runbook explicitly says to confirm the local sale, preserve its financial facts, inspect tax state/configuration, retry only retryable failures, and preserve evidence on permanent rejection.

The exact MRA data fields and production certification path remain evidence-driven and must be validated against current MRA contracts before production implementation.


# 27. Customer Domain

Customer records exist to support operational relationships, not to force every anonymous retail buyer into a CRM profile.

### Customer modes

```text
ANONYMOUS
IDENTIFIED
ACCOUNT_CUSTOMER
```

### Minimalism principle

For a simple cash sale, identifying the customer may be unnecessary. For credit sales, loyalty, regulated workflows or receipts requiring identification, a customer record may be required.

### Sensitive data

Customer PII should be minimized. The domain must distinguish:

```text
Customer identity
Customer contact data
Customer account/credit state
Sale relationship
Consent/preferences
```

Do not use customer profile data as a substitute for transactional history; sales remain authoritative business facts.


# 28. Supplier Domain

Supplier is a reusable business-counterparty concept used by procurement, receiving and reporting.

A supplier record may include:

- name;
- contact channels;
- supplier code;
- tax identifier where appropriate;
- payment terms metadata;
- active/inactive state;
- supplier-product relationships.

Supplier deletion should generally be archival rather than physical deletion when historical procurement records reference the supplier.


# 29. Pharmacy Extension

Pharmacy is a controlled extension over the retail core, not a boolean `business_type = pharmacy` that magically enables unsafe behavior.

### Pharmacy-specific domain concepts

```text
MedicineProductExtension
Batch/Lot
Expiry
Recall
Quarantine
PrescriptionReference
AuthorizedDispenser
DispensingEvent
ControlledStorageState
```

### Reuse from retail core

- Product/SKU identity
- Inventory locations
- Inventory movements
- Sales
- Returns
- Suppliers
- Purchase receipts
- Users and permissions
- Audit

### Add pharmacy-specific invariants

- Expired inventory cannot enter normal sale allocation when policy prohibits it.
- Quarantined/recalled inventory cannot be sold.
- Authorized staff permissions are required for controlled dispensing workflows.
- Batch/lot traceability must survive sale and return operations where required.
- Recall operations can identify affected lots and transactions.
- Pharmacy records receive appropriate retention and audit treatment.

The exact regulated model must be validated against current Malawi pharmacy requirements before claiming legal compliance.


# 30. Agro-dealer Extension

Agro-dealer support similarly extends the core inventory model.

Concepts may include:

```text
Formulation
Concentration
Batch/Lot
RecommendedUseDate
PackagingUnit
SeasonalAvailability
SupplierTraceability
```

The core model already supports the general inventory semantics; the extension adds domain attributes and controlled workflows rather than inventing a second inventory engine.

### Example

```text
Product family: Fertilizer
SKU: 50kg bag
Lot: L2026-07-A
Supplier: Supplier-01
Recommended use / expiry: domain-specific
```

The extension must reuse generic quantity/unit conversion and batch movement infrastructure.


# 31. Wholesale Extension

Wholesale primarily extends pricing and order behavior:

- customer-specific pricing;
- quantity tiers;
- credit/account sales;
- larger order quantities;
- minimum order constraints;
- delivery/order status where required.

The same SKU, inventory and sale models can remain authoritative. Wholesale should not create a parallel product or inventory domain.


# 32. Subscription and Entitlement Domain

Billing is part of the platform control plane, not merchant financial truth.

### Concepts

```text
Plan
Entitlement
Subscription
UsageMeter
BillingPeriod
Invoice
PaymentState
SuspensionState
FeatureFlag
```

### Important distinction

```text
Business type = operational configuration / extension
Subscription plan = entitlement policy
```

A pharmacy does not become entitled to a feature because it is a pharmacy, and a plan does not redefine accounting truth.

### Lifecycle

```text
TRIAL
 ↓
ACTIVE
 ├── PAST_DUE
 ├── GRACE_PERIOD
 └── SUSPENDED
 ↓
CANCELLED
```

Entitlement checks should fail predictably and must not bypass security boundaries.


# 33. Audit Domain

Audit is evidence, not a debug log.

### AuditEvent

Suggested structure:

```text
AuditEvent
 ├── event_id
 ├── timestamp
 ├── organization_id?
 ├── actor_id?
 ├── actor_type
 ├── action
 ├── resource_type
 ├── resource_id
 ├── result
 ├── reason_code
 ├── request_id
 ├── correlation_id
 ├── device_id?
 ├── approval_id?
 └── external_reference?
```

### Audit facts

Audit should explain meaningful state transitions:

- user added/removed;
- role changed;
- refund approved/rejected;
- stock adjusted;
- register opened/closed;
- payment reconciled;
- device revoked;
- export created;
- support access granted;
- configuration changed.

Do not log secrets or raw credentials. Audit evidence is durable and restricted, while operational logs are optimized for service operation.


# 34. Notification Domain

Notifications are derived side effects.

```text
Domain event
   ↓
Outbox
   ↓
Notification job
   ↓
Email / SMS / push / in-app
```

Notification delivery cannot be part of the primary transaction if it requires a remote provider. The business transaction must commit first; asynchronous notification can retry separately.

A notification failure must not roll back an already-completed sale unless the business operation explicitly requires synchronous external confirmation.


# 35. Integration Domain

Integrations are adapters around explicit provider contracts.

### Adapter interface concept

```text
Internal domain
      │
      ▼
Port / capability interface
      │
      ├── Provider Adapter A
      ├── Provider Adapter B
      └── Provider Adapter C
```

The domain should ask for a capability such as:

```text
create_payment_intent(...)
query_payment_status(...)
submit_tax_document(...)
send_notification(...)
```

rather than embedding provider-specific HTTP details inside sales or inventory code.

Provider responses become validated external evidence before they can transition internal state.


# 36. Reporting Domain

Reporting consumes authoritative facts and produces derived representations.

### Principle

```text
PostgreSQL transactional truth
          ↓
Read models / aggregates
          ↓
Reports / dashboards / exports
```

Reports are not allowed to become a second write authority.

### Metrics examples

- daily sales;
- gross/net/tax totals;
- stock-on-hand;
- low-stock indicators;
- inventory aging;
- cash variance;
- payment settlement status;
- supplier purchasing activity;
- refunds/returns;
- branch comparisons;
- staff activity.

### Historical correctness

A report must state its source semantics. For example, a “sales total” may mean finalized sales only, while “cash collected” may mean cash events in a register period. Avoid a single ambiguous `revenue` number that mixes states.


# 37. Export Domain

Exports are high-power read operations.

### ExportJob

```text
REQUESTED
 ↓
AUTHORIZED
 ↓
QUEUED
 ↓
RUNNING
 ├── COMPLETED
 ├── FAILED
 └── CANCELLED
```

Exports carry:

- requested scope;
- actor;
- authorization evidence;
- filter specification;
- output type;
- object-storage reference;
- expiration time;
- audit identity.

An export must never expand tenant scope simply because a report query joins data through an unauthorized path.


# 38. Offline Domain Model

Offline operation introduces a special distinction:

```text
LOCAL CONTINUITY
        ≠
AUTHORITATIVE SERVER TRUTH
```

### Local entities

The client may maintain local representations of:

- products/SKUs within permitted scope;
- prices needed for permitted operation;
- inventory availability projection;
- open register session;
- unsent commands;
- command acknowledgements/checkpoints;
- limited customer/supplier references;
- local audit/UI state.

### Offline Command

```text
OfflineCommand
 ├── command_id
 ├── device_id
 ├── organization scope
 ├── schema_version
 ├── command_type
 ├── payload
 ├── client_created_at
 ├── sequence / causal metadata
 ├── local_status
 └── integrity metadata where required
```

### Server treatment

The server must validate:

```text
Device state
+ membership
+ tenant scope
+ command uniqueness
+ command age/window
+ schema version
+ business invariants
+ current authoritative state
```

### Forbidden assumption

A local database row marked `paid=true` is not authoritative proof of provider settlement.


# 39. Command and State-Transition Model

Sitolo should implement explicit commands rather than arbitrary field mutation.

Examples:

```text
CreateSale
AddSaleLine
RemoveSaleLine
ApplyDiscount
FinalizeSale
VoidSale
CreateReturn
ApproveReturn
CreateRefund
ApproveRefund
OpenRegister
RecordCashIn
RecordCashOut
CloseRegister
CreatePurchaseOrder
ApprovePurchaseOrder
ReceiveGoods
CreateStockAdjustment
ApproveStockAdjustment
CreateTransfer
ApproveTransfer
PostStockCount
CreatePaymentIntent
CancelPaymentIntent
ReconcilePayment
RegisterDevice
RevokeDevice
InviteUser
AssignRole
SuspendMembership
```

Every command should define:

```text
Actor
Scope
Target
Preconditions
State transition
Invariant checks
Side effects
Audit
Idempotency
Events
Failure modes
```

This turns the domain into a system of explicit transitions rather than generic CRUD.


# 40. Domain Event Catalogue

Initial canonical events should include:

### Tenant / identity

```text
OrganizationCreated
OrganizationSuspended
MembershipCreated
MembershipRoleChanged
MembershipRevoked
DeviceRegistered
DeviceRevoked
SessionRevoked
```

### Catalogue / pricing

```text
ProductCreated
ProductActivated
ProductDiscontinued
SkuCreated
BarcodeAssigned
PriceVersionCreated
PromotionActivated
PromotionExpired
```

### Procurement

```text
PurchaseOrderSubmitted
PurchaseOrderApproved
PurchaseOrderCancelled
GoodsReceiptPosted
```

### Inventory

```text
InventoryMovementPosted
StockTransferCompleted
StockCountApproved
StockAdjustmentApproved
```

### Sales

```text
SaleCreated
SaleFinalized
SaleVoided
SaleReversed
ReturnCreated
ReturnApproved
RefundRequested
RefundApproved
```

### Payments

```text
PaymentIntentCreated
PaymentProviderEventReceived
PaymentSucceeded
PaymentFailed
PaymentExpired
PaymentReconciliationExceptionRaised
PaymentReconciled
```

### Tax

```text
TaxSubmissionRequested
TaxSubmissionAccepted
TaxSubmissionRetryableFailure
TaxSubmissionRejected
```

Events should describe facts, not commands disguised as nouns. Consumers must be idempotent.


# 41. Event Versioning

Domain and integration events need explicit versioning because the platform is intended to survive long-lived merchant data and gradual client upgrades.

### Rules

- Never silently change event meaning.
- Add fields compatibly where possible.
- Version when semantics change.
- Persist enough identifiers to trace the originating transaction.
- Consumers must tolerate duplicate delivery.
- Consumer state cannot be updated merely because an event ID is new; semantic validation still applies.

Example:

```text
SaleFinalized.v1
SaleFinalized.v2
```

A new event version should define migration/compatibility behavior before release.


# 42. Cross-Domain Invariants

The most important Sitolo rules span multiple contexts.

### Sale + inventory

```text
A finalized sale that consumes stock must produce the corresponding authoritative inventory effect atomically.
```

### Sale + pricing

```text
Historical sale price is a snapshot; current pricing cannot retroactively rewrite it.
```

### Sale + payment

```text
Sale state and payment state are distinct; payment confirmation derives from authoritative payment evidence.
```

### Payment + reconciliation

```text
A provider event cannot create duplicate financial effects even when delivered multiple times or out of order.
```

### Purchase + inventory

```text
Only finalized/posted receiving facts create stock receipts; editing an old purchase order does not silently alter historical inventory.
```

### Refund + sale

```text
Refund eligibility derives from authoritative correction history; it cannot exceed remaining eligible value.
```

### Tenant + every aggregate

```text
Every tenant-owned aggregate is reachable only through authorized organization scope.
```

### Device + offline command

```text
A revoked device cannot continue normal synchronization merely because it possesses queued commands.
```

### Audit + mutation

```text
Security-sensitive state transitions produce durable evidence sufficient to reconstruct the action.
```


# 43. Aggregate Invariant Matrix

| Aggregate | Critical invariants |
|---|---|
| Organization | stable identity; valid lifecycle; unique tenant identity |
| Membership | one user/org relationship; valid scope; revocation blocks authority |
| Device | unique device identity; revoked means rejected for restricted operations |
| Product/SKU | valid lifecycle; unique scoped SKU/barcode; no historical deletion |
| PriceList | non-overlapping effective semantics where required; deterministic resolution |
| PurchaseOrder | legal state transitions; approval rules; cancellation does not erase history |
| GoodsReceipt | quantities/costs/lot data valid; posting idempotent |
| InventoryPosition | no illegal negative stock; atomic updates; tenant/location/SKU scope |
| InventoryTransfer | source and destination paired; no partial committed transfer |
| StockCount | approval before posting; posted count creates explicit movements |
| Sale | cannot finalize twice; totals reconcile; authorized actor; valid inventory/payment relationships |
| Return | return quantity/value bounded by eligibility; explicit link to originating sale |
| Refund | refund amount bounded; duplicate request safe; approval policy enforced |
| RegisterSession | one active session per register policy; close once; variance preserved |
| PaymentIntent | state transitions legal; amount immutable once committed to provider flow where contract requires |
| ReconciliationCase | ambiguous match remains unresolved until evidence is sufficient |
| TaxSubmission | external failure does not rewrite local commercial truth |
| ExportJob | tenant scope fixed; authorized request; expiring output |
| Subscription | state machine valid; entitlement derived from current subscription policy |


# 44. State Machine Design Rules

State machines must satisfy:

1. Every state has a defined meaning.
2. Every transition has preconditions.
3. Every transition identifies authorized actors.
4. Illegal transitions fail closed.
5. Final states are truly final unless a compensating transition exists.
6. Retries do not accidentally create duplicate transitions.
7. Concurrent transitions are serialized through the aggregate's consistency mechanism.
8. Offline clients do not get to invent new privileged states.
9. State changes generate required audit evidence.
10. External-provider states are kept separate from internal business states.

A state enum without transition rules is insufficient.


# 45. Business Day and Time Semantics

Merchant operations often think in business days while systems operate in UTC or provider-specific timestamps.

The domain should distinguish:

```text
OccurredAt       = when the business event is claimed to have occurred
RecordedAt       = when Sitolo received/persisted it
ProviderAt       = external provider timestamp where available
BusinessDate     = reporting/business-day projection
```

The model must not silently replace one with another.

Examples:

- Offline sale occurred at 23:58 local time but reached the server at 00:04.
- Payment provider event is timestamped differently from the merchant sale.
- Tax submission response arrives hours later.

Reports must choose explicitly which timestamp semantics they use.


# 46. Identifier and Numbering Model

Separate machine identifiers from human-readable numbers.

```text
SaleId      = stable technical identity
SaleNumber  = merchant-facing business number
```

Same pattern may apply to:

```text
PurchaseOrderId / PurchaseOrderNumber
GoodsReceiptId / GoodsReceiptNumber
RegisterId / RegisterNumber
```

### Requirements

- machine IDs must be safe to expose where necessary and not treated as secrets;
- human numbers may have sequential/business rules;
- numbering must tolerate offline operation when required;
- collisions must be impossible within the applicable business scope;
- provider references are stored separately from Sitolo IDs.

Do not use auto-increment database IDs as the only offline command identity mechanism.


# 47. Delete, Archive and Retention Semantics

Deletion must be domain-specific.

### Hard delete may be appropriate for

- transient draft data with no business consequence;
- expired ephemeral job metadata;
- temporary UI state.

### Archive/deactivate is preferred for

- products with historical sales;
- suppliers referenced by procurement;
- users with audit history;
- branches with historical activity;
- price policies used historically;
- business configuration with evidentiary value.

### Never silently delete

- finalized sales;
- inventory movements;
- finalized payment evidence;
- tax submission evidence;
- audit events.

Deletion requests must consider legal retention requirements and business evidence before removing data.


# 48. Read Models and Projections

Read models can be denormalized for performance but remain derived.

Examples:

```text
ProductSearchProjection
InventoryAvailabilityProjection
DailySalesProjection
BranchPerformanceProjection
PaymentSettlementProjection
LowStockProjection
```

A projection may be rebuilt from canonical data. Therefore:

```text
Projection corrupted
   ↓
rebuild
   ↓
canonical truth preserved
```

This is important for disaster recovery and schema evolution.

A report table must not become the only location from which financial truth can be reconstructed unless explicitly designed and proven as canonical—which conflicts with the current Sitolo authority model.


# 49. Repository and Domain Service Boundaries

Recommended Rust layering:

```text
apps / bin
   ↓
interfaces / HTTP / jobs
   ↓
application services
   ↓
domain
   ↓
ports
   ↓
infrastructure adapters
```

### Domain layer

Owns:

- domain entities;
- value objects;
- state transitions;
- invariants;
- pure calculations;
- domain errors.

Must not know about Axum or SQLx.

### Application layer

Owns:

- command orchestration;
- authorization invocation;
- transaction boundaries;
- unit-of-work behavior;
- outbox registration;
- external port coordination.

### Infrastructure

Owns:

- SQLx repositories;
- PostgreSQL queries;
- external HTTP clients;
- object storage;
- queue workers;
- telemetry integrations.

### Anti-pattern

Do not put domain rules inside Axum handlers. Do not allow a repository method like `update_sale_total()` that lets callers mutate a finalized financial fact without passing through a domain operation.


# 50. Rust Representation Strategy

The implementation should use Rust's type system to encode domain distinctions where the complexity is justified.

Example conceptual types:

```rust
struct TenantId(Uuid);
struct BranchId(Uuid);
struct SaleId(Uuid);
struct Quantity(Decimal);
struct Money { amount_minor: i64, currency: CurrencyCode }
struct Percentage(Decimal);
```

Enums should represent closed state vocabularies:

```rust
enum SaleStatus {
    Draft,
    PendingConfirmation,
    Finalized,
    Voided,
}
```

Do not overuse advanced type-state generics for every workflow. Runtime state from persistent data is often clearer as enums plus explicit transition methods. The objective is preventing accidental interchangeability, not winning a type-system contest.

Rust's newtype/advanced-type mechanisms are appropriate for making semantically distinct values distinct. The Rust language documentation provides the underlying type-system tools; the architectural requirement is to apply them selectively to business-critical identifiers and values.


# 51. PostgreSQL Mapping Rules

The domain model must map cleanly to PostgreSQL without becoming a table-per-noun reflex.

### Rules

- Every persistent entity has a stable primary identity.
- Tenant-owned rows carry direct or enforceable tenant scope.
- Foreign keys preserve referential integrity.
- Unique constraints express uniqueness invariants.
- Check constraints express local row invariants.
- Cross-row/cross-table constraints use appropriate relational mechanisms or application transaction logic rather than unsafe `CHECK` assumptions.
- Database roles separate migrations from runtime.
- Critical tables can use RLS as defense in depth.
- Historical facts are not destructively edited.

PostgreSQL's current documentation explicitly describes `CHECK`, `NOT NULL`, `UNIQUE`, primary-key, foreign-key and exclusion constraints and notes that row-local `CHECK` constraints are not intended to safely enforce arbitrary cross-row relationships. This matters when deciding which invariants belong in SQL and which require transactions/application logic. citeturn370984view0

RLS must be designed with runtime roles in mind. PostgreSQL documents that RLS can provide default-deny behavior but also notes bypass considerations for privileged roles and table owners. The Sitolo runtime role therefore must not accidentally have bypass authority. citeturn370984view1


# 52. SQLx Mapping Rules

SQLx remains the preferred PostgreSQL access layer from the existing architecture.

Use:

- explicit SQL for business-critical queries;
- bind parameters for caller values;
- compile-time query checking where practical;
- migrations as version-controlled artifacts;
- transactions around atomic business operations.

The SQLx `query!` family performs compile-time checks against the database schema/metadata under its supported workflow, which fits Sitolo's desire for reproducible, strongly checked database access. citeturn370984view3

Do not let SQLx query convenience collapse domain boundaries. A repository query should express an authorized business operation, not expose arbitrary SQL power to application callers.


# 53. Aggregate Transaction Boundaries

A transaction is required whenever multiple authoritative writes must succeed or fail together.

### Example: finalize sale

```text
BEGIN
  authenticate principal
  resolve tenant + branch + register scope
  authorize FINALIZE_SALE

  load sale draft
  validate sale state
  validate product/SKU state
  calculate/verify final prices and tax
  lock/check inventory
  post inventory movements
  finalize sale
  create payment intent linkage if required
  write audit event
  write outbox event
COMMIT
```

### What NOT to do

```text
BEGIN
  write sale
  HTTP call payment provider
  write inventory
COMMIT
```

The architecture explicitly prohibits keeping database transactions open across external HTTP calls. Instead, provider interaction should be orchestrated outside the transaction using durable intent/state and subsequent reconciliation.


# 54. Idempotency Model

Idempotency is part of the domain, not merely an API header.

Every retryable business command should identify:

```text
Who/what sent it?
Which command?
Which business intent?
What was the previous outcome?
```

### Command idempotency

```text
Same CommandId + Same semantic payload
    → existing result / equivalent no-op
```

A reused command ID with materially different semantics should be rejected, not treated as a free new operation.

### Provider event idempotency

```text
(provider, provider_event_id)
    UNIQUE
```

### Business-reference uniqueness

Use database uniqueness where the business rule truly requires one logical fact to exist once. Application “check then insert” logic is not enough under concurrency.


# 55. Domain Error Taxonomy

Errors should reflect domain semantics, not leak storage exceptions.

Suggested families:

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

The public API maps these into stable response codes. Internal database/provider details remain diagnostic.

A domain error means “the business cannot accept this command.” An infrastructure error means “the system could not safely determine/complete the result.” They should not be conflated because retry behavior differs.


# 56. Security as Domain Invariants

Some of Sitolo's highest-severity security controls are ordinary business rules expressed as invariants.

Examples:

```text
refund <= eligible amount
stock decrement <= available quantity under policy
provider transaction settles at most once
completed sale is not silently edited
cashier cannot self-approve restricted adjustment
revoked device cannot continue restricted sync
tenant-scoped actor cannot access another tenant's object
expired/quarantined medicine cannot enter prohibited sale path
```

This is why the security architecture treats business correctness and application security as related layers. Attackers can exploit valid permissions through invalid combinations of valid actions. Domain invariants are therefore security boundaries.


# 57. Actor and Evidence Model

Every sensitive command should retain enough provenance to answer:

```text
WHO
WHAT
WHERE
WHEN
FROM WHICH DEVICE
UNDER WHICH ROLE/SCOPE
AGAINST WHICH OBJECT
WITH WHICH COMMAND ID
WITH WHICH APPROVAL
WITH WHICH EXTERNAL REFERENCE
```

The model does not require every domain object to embed every provenance field. Instead, sensitive mutation workflows should connect command/audit records to the resulting business fact.

This produces a reconstructable chain:

```text
Principal
  ↓
Session
  ↓
Request / Command
  ↓
Domain transition
  ↓
DB transaction
  ↓
Audit event
  ↓
Outbox event
  ↓
External provider reference
```


# 58. Approval Domain

Approval is a first-class business control for selected high-risk operations.

### ApprovalRequest

```text
REQUESTED
 ↓
PENDING_APPROVAL
 ├── APPROVED
 └── REJECTED
 ↓
APPLIED / EXPIRED
```

The approval record should capture:

- requested operation;
- target resource;
- amount/threshold;
- requester;
- approver;
- timestamp;
- reason;
- resulting action;
- expiration.

### Separation of duties

Where policy requires it, the requester cannot also become the approver.

Approvals are not a universal mandatory workflow. They are a configurable control for operations where the risk/financial exposure justifies the friction.


# 59. Business Configuration Domain

Configuration should be classified into:

```text
Platform configuration
Organization configuration
Branch configuration
Feature/entitlement configuration
Operational policy
Regulated-domain configuration
```

Examples:

- discount limits;
- refund thresholds;
- tax configuration;
- stock adjustment approval threshold;
- supported payment methods;
- register policy;
- offline capabilities;
- retention policy;
- report permissions.

### Versioning

Security- or financially-sensitive configuration should be versioned/snapshotted when the current value alone cannot explain a historical decision.

Example:

```text
Sale 2026-09-04
  uses PriceConfigVersion 42
  uses TaxConfigVersion 17
```

This avoids trying to reconstruct historical truth from today's configuration.


# 60. Feature Flags vs Domain State

Feature flags control deployment/release behavior; domain state represents actual business facts.

Bad:

```text
if feature_flag_enabled:
    pretend_refund_exists
```

Better:

```text
Feature enabled
    ↓
Command available
    ↓
Domain rules validate
    ↓
Actual RefundCreated fact exists
```

A feature flag must never weaken a hard security invariant. Emergency disablement should remove functionality or place it into an explicit safe state rather than bypassing authorization.


# 61. Multi-Branch Semantics

Multi-branch support must be explicit in the model.

A user may have:

```text
Organization-wide role
OR
Branch-specific role
OR
Branch-specific permission override
```

Inventory belongs to a location/warehouse. Sales happen at a selling location/register. Staff scope may cover one or several branches.

### Branch transfer

Moving stock between branches is not merely changing `branch_id`. It is an auditable transfer with source and destination, linked movement records and authorization.

### Branch closure

Closing a branch must not delete its historical transactions. New operations are blocked while reports/audit remain readable to authorized users.


# 62. Register and Device Relationship

A register is a business control point; a device is a technical endpoint.

```text
Register
  ↕
Device
  ↕
User Session
```

The relationship is contextual, not identity-equivalent.

One device may be re-registered or replaced. One register may have different devices over time. Historical sale facts should preserve register and device references available at the time of the operation.

Device replacement must not erase register history.


# 63. Offline Conflict Classification

Do not use generic last-write-wins across business records.

Classify conflicts:

```text
NO_CONFLICT
SAFE_DUPLICATE
ORDERING_ONLY
STATE_CONFLICT
AUTHORIZATION_CONFLICT
STOCK_CONFLICT
PAYMENT_CONFLICT
POLICY_CONFLICT
SCHEMA_CONFLICT
SECURITY_REJECTION
```

Examples:

- Duplicate `CreateSale` with same command ID → safe duplicate.
- Two independent notes changing → potentially ordering-only.
- Two devices sell the last unit → stock conflict.
- Device revoked before sync → security rejection.
- Client sends command for stale role → authorization conflict.

Each conflict type needs a defined resolution path. “Take the newest timestamp” is not an acceptable universal resolution strategy for money or stock.


# 64. Offline Financial Semantics

Offline operation needs special treatment because the server cannot immediately validate global state.

Permitted offline operations should be bounded by policy. Typical cashier operations may continue, while highly privileged actions remain online-only or require an expiring pre-issued capability.

### Offline sale

```text
Local user intent
  ↓
Local validation
  ↓
Local durable command
  ↓
Local operational result
  ↓
Sync
  ↓
Server authorization + invariant validation
  ↓
Authoritative posting
```

The local result should be understood as provisional until server reconciliation establishes authoritative acceptance.

### No false zero-loss claim

A device that is destroyed before synchronization may contain unsent operational evidence. The architecture must reduce this risk with durable local queues, retry, backup/sync checkpoints and device recovery patterns, but should not claim impossible guarantees such as “zero loss under arbitrary physical destruction before sync.”


# 65. Reporting Semantics for Financial Truth

Reports must name the population and state semantics they aggregate.

Examples:

```text
Gross Sales
= finalized sale gross amounts in selected period

Net Sales
= gross sales minus eligible discounts/returns according to report definition

Cash Collected
= posted cash events in selected register scope

Payment Settled
= payment intents with authoritative successful/reconciled state

Inventory On Hand
= derived quantity from inventory ledger/projection under selected as-of semantics
```

The goal is to prevent executive dashboards from quietly mixing pending, reversed, refunded and finalized records.


# 66. Domain Events vs Audit Events

These are not interchangeable.

### Domain event

Represents a business fact needed by another component.

```text
SaleFinalized
InventoryMovementPosted
PaymentSucceeded
```

### Audit event

Represents evidence that a significant action occurred and who/what performed it.

```text
REFUND_APPROVAL_GRANTED
USER_ROLE_CHANGED
DEVICE_REVOKED
```

One business transition may emit both. A domain event is optimized for system behavior; an audit event is optimized for accountability and investigation.


# 67. Outbox Semantics

Outbox rows are part of the same PostgreSQL transaction as the domain fact they publish.

```text
BEGIN
  write Sale
  write InventoryMovement
  write AuditEvent
  write OutboxMessage
COMMIT
```

Worker:

```text
read outbox
 ↓
claim safely
 ↓
deliver / process
 ↓
ack or retry
```

A worker must tolerate duplicate delivery. Downstream consumers require idempotency.

This preserves the important invariant:

> A committed domain fact is not silently lost because a process crashed between the database commit and an external notification attempt.


# 68. Domain Invariants That Must Become Tests

The domain model is not complete until these are executable.

### Identity

- revoked membership cannot authorize business commands;
- revoked device cannot perform restricted sync;
- user cannot act outside branch scope.

### Catalogue

- inactive/discontinued SKU cannot be sold when policy prohibits;
- duplicate barcode rejected in tenant scope;
- current price cannot change historical sales.

### Inventory

- no unauthorized stock adjustment;
- no illegal negative stock;
- duplicate sale command does not consume stock twice;
- transfer cannot post only one side;
- expired/quarantined lot cannot enter forbidden sale path.

### Sales

- sale cannot finalize twice;
- finalized sale cannot be edited into a different total;
- client-modified total cannot override authoritative calculation.

### Returns/refunds

- refund cannot exceed eligible amount;
- return cannot exceed returnable quantity;
- duplicate refund command is idempotent.

### Payments

- duplicate provider event cannot produce duplicate posting;
- invalid signature rejected;
- provider mismatch becomes reconciliation exception.

### Cash

- register closes once per session;
- cash variance is preserved.

### Sync

- restarting client preserves queued commands;
- syncing same batch twice is harmless;
- revoked device commands are rejected according to policy.

These correspond directly to the existing architecture and security testing requirements.


# 69. Test Fixture Topology

A standardized test world should exist for domain testing.

```text
Organization A
├── Branch A1
│   ├── Warehouse A1
│   ├── Register A1
│   └── Device A1
└── Branch A2
    └── Warehouse A2

Organization B
└── Branch B1

Users
├── Owner A
├── Manager A
├── Cashier A1
├── Inventory A1
├── Finance A
├── Auditor A
├── Owner B
└── Cashier B
```

Seed:

- product with normal stock;
- product with one remaining unit;
- expired lot;
- quarantined lot;
- active promotion;
- finalized sale;
- partially refunded sale;
- payment pending;
- payment reconciled;
- open and closed register sessions;
- pending offline commands;
- revoked device.

The same fixtures should support unit, integration, API, security and concurrency tests.


# 70. Property-Based Domain Testing

Property testing is valuable for domains whose correctness depends on long event sequences.

### Inventory property

Generate arbitrary sequences of:

```text
receipts
sales
returns
transfers
adjustments
counts
```

Then assert that derived balances agree with the ledger according to declared policies.

### Financial property

Generate sequences of:

```text
sale
partial refund
return
reversal
additional refund
```

Assert that no eligible amount becomes negative and that total correction value remains bounded.

### Idempotency property

Generate a command, duplicate it N times, shuffle delivery, and assert one logical effect.

### Sync property

Generate valid offline commands, duplicate/reorder them within declared commutativity limits, and assert the final state matches the protocol's expected convergence semantics.


# 71. Domain Security Threat Mapping

| Domain weakness | Attack | Impact |
|---|---|---|
| Missing tenant scope | BOLA/IDOR | cross-tenant disclosure/mutation |
| Editable finalized sale | financial manipulation | revenue/fraud evidence corruption |
| Weak inventory invariant | race/concurrency | stock creation/oversell |
| Weak refund eligibility | business logic abuse | direct financial loss |
| Payment state trusted from UI | client tampering | false settlement |
| Duplicate webhook accepted | replay | duplicate payment effect |
| Unbounded report | resource abuse | availability/cost attack |
| Unsafe import | mass mutation | data corruption |
| Offline command accepted blindly | replay/tampering | fraudulent operations |
| Weak approval semantics | insider abuse | unauthorized high-value mutation |
| Historical dependency on mutable product data | configuration tampering | inaccurate reports/audit |

This mapping demonstrates why the domain model is part of Sitolo's security architecture, not an isolated modeling exercise.


# 72. Domain API Contract Implications

The domain model should shape API endpoints around business resources and commands rather than one endpoint per database table.

Good examples:

```text
POST /sales
POST /sales/{id}/finalize
POST /sales/{id}/void
POST /sales/{id}/returns
POST /refunds
POST /refunds/{id}/approve
POST /inventory/adjustments
POST /inventory/transfers
POST /purchase-orders/{id}/approve
POST /goods-receipts
POST /payments/{id}/reconcile
POST /register-sessions/{id}/close
```

Avoid:

```text
PATCH /sales/{id}
PATCH /inventory/{id}
PATCH /payments/{id}
```

when those PATCH operations could bypass state-machine and financial invariants.

The existing architecture similarly recommends stable business resources plus explicit commands rather than a database-table-shaped CRUD surface.


# 73. Domain Repository Contracts

Repositories should encode scope.

Preferred shape:

```text
SaleRepository.get_for_scope(scope, sale_id)
InventoryRepository.get_position(scope, sku_id, location_id)
PaymentRepository.get_intent(scope, payment_intent_id)
```

Risky shape:

```text
SaleRepository.get(sale_id)
```

The former makes tenant scope part of the call contract. That reduces accidental cross-tenant access.

Repositories should not perform authorization decisions that belong to the application/security layer, but they should make unauthorized data retrieval structurally harder.


# 74. Domain Immutability Rules

### Immutable after finalization

The following facts should generally be append-only or correction-driven:

- finalized sale economics;
- inventory movement history;
- posted cash events;
- successful payment evidence;
- tax submission evidence;
- audit events;
- completed transfer facts.

### Mutable before finalization

Drafts and pending workflows may be editable according to authorization and state.

This distinction should appear in Rust APIs. A finalized `Sale` should expose operations such as `reverse`, `create_return`, or `request_refund` rather than a generic `set_total` method.


# 75. Data Ownership Matrix

| Data | Canonical owner | Consumers |
|---|---|---|
| User identity | Identity | all authenticated contexts |
| Membership | Tenant/IAM | authorization |
| Product/SKU | Catalogue | pricing, inventory, sales |
| Price version | Pricing | sales, reporting |
| Purchase order | Procurement | receiving, reporting |
| Goods receipt | Procurement | inventory |
| Inventory movement | Inventory | sales, reporting, reconciliation |
| Sale | Sales | payments, tax, reporting |
| Cash event | Cash | reporting, audit |
| Payment intent | Payments | sales, reconciliation |
| Provider event | Payments/Integrations | reconciliation, audit |
| Tax submission state | Tax/EIS | reporting, audit |
| Audit event | Audit | security/operations/compliance |
| Subscription | Billing | entitlement checks |
| Report projection | Reporting | clients |

No data product should have multiple competing canonical owners.


# 76. Open Decisions — Deliberately Not Faked

The current architecture identifies several decisions that still require real evidence. They must remain open rather than being invented in this model.

### 76.1 Identity provider

Exact provider versus self-hosted standards-based authentication remains open.

### 76.2 Cloud / region

Hosting provider, region, network topology and managed-service choices remain open.

### 76.3 Redis

Redis is optional and should only become a runtime dependency when load measurements justify it.

### 76.4 Reporting infrastructure

A read replica/warehouse is not required until real workload data demonstrates the need.

### 76.5 Payment provider semantics

Exact webhook/statement APIs determine whether deterministic reconciliation can be achieved for each provider.

### 76.6 MRA EIS

Exact certification pathway, production credentials and final integration requirements require current official verification.

### 76.7 Pharmacy model

The precise regulatory data model and permission requirements require jurisdiction-specific validation before launch.

### 76.8 Dedicated tenancy

Some enterprise customers may require stronger physical database isolation; the default remains shared PostgreSQL with tenant-scoped records unless evidence/contract demands otherwise.

These open decisions do not block defining the domain concepts; they block pretending unsupported implementation details are final.


# 77. Build vs Reuse for the Domain

Sitolo should build what differentiates it and reuse what is commodity.

### Build internally

- tenant/branch domain semantics;
- inventory ledger semantics;
- financial invariants;
- sale/return/refund state machines;
- offline command model;
- reconciliation rules;
- domain-specific approval policy;
- business-specific audit semantics.

### Reuse/adopt

- cryptography;
- OAuth/OIDC protocol implementations;
- password hashing;
- TLS;
- secure storage;
- database engine;
- static analysis/security scanning;
- cloud secret management;
- object storage;
- provider SDKs where trustworthy and useful.

This protects engineering focus and matches the existing security architecture's build-vs-buy stance.


# 78. Implementation Mapping to Rust Modules

A concrete initial package structure could be:

```text
crates/
├── sitolo-domain/
│   ├── identity/
│   ├── tenant/
│   ├── catalogue/
│   ├── pricing/
│   ├── procurement/
│   ├── inventory/
│   ├── sales/
│   ├── cash/
│   ├── payments/
│   ├── reconciliation/
│   ├── tax/
│   ├── billing/
│   └── shared/
├── sitolo-application/
├── sitolo-auth/
├── sitolo-authz/
├── sitolo-persistence/
├── sitolo-integrations/
├── sitolo-jobs/
├── sitolo-observability/
└── sitolo-api/
```

This is an example logical boundary, not a mandate to create one crate for every domain immediately. Start with clear Rust modules if the workspace would otherwise become unnecessarily fragmented. The architectural principle is ownership and dependency direction, not crate count.


# 79. Dependency Direction Rules

Allowed:

```text
API → Application → Domain
Jobs → Application → Domain
Persistence → Domain ports/types
Integrations → Domain ports/types
```

Avoid:

```text
Domain → Axum
Domain → SQLx
Domain → Redis
Domain → Provider SDK
Domain → Flutter concepts
```

The domain should remain testable without a live database or HTTP server whenever the rule under test is pure business logic.


# 80. Domain Invariant Enforcement Matrix

| Invariant class | Rust domain | Application service | PostgreSQL | Test |
|---|---:|---:|---:|---:|
| valid state transition | yes | yes | optional | mandatory |
| tenant scope | policy input | yes | RLS/constraints where justified | mandatory |
| uniqueness | no | no as sole control | yes | mandatory |
| foreign-key integrity | no | no as sole control | yes | mandatory |
| inventory concurrency | yes | yes | yes | mandatory |
| finalized-sale immutability | yes | yes | schema/permissions | mandatory |
| refund bound | yes | yes | supporting constraints | mandatory |
| payment dedupe | yes | yes | unique constraint | mandatory |
| offline command replay | yes | yes | uniqueness/checkpoint state | mandatory |
| audit requirement | yes | yes | persistence | mandatory |

The important rule is defense in depth: no single layer should be the only thing preventing catastrophic corruption where multiple layers can reasonably enforce the same invariant.


# 81. Domain Lifecycle of a Typical Retail Day

A normal merchant day should traverse the model coherently.

```text
OPEN REGISTER
    ↓
CHECK/REVIEW OPENING STOCK
    ↓
RECEIVE GOODS
    ↓
STOCK AVAILABLE
    ↓
PRICE ACTIVE
    ↓
SELL
    ↓
COLLECT CASH / PAYMENT INTENT
    ↓
RECONCILE
    ↓
COUNT REGISTER
    ↓
CLOSE REGISTER
    ↓
REPORT
```

The business system must preserve relationships among these events.

Example failure:

```text
Sale completed
BUT
payment pending
```

This is not necessarily corruption. The model permits distinct states.

Another failure:

```text
Sale completed
BUT
inventory movement missing
```

Under normal transactional sale rules, this is a violation and should not be allowed to commit.


# 82. Example End-to-End Sale

### Input

Cashier scans SKU-X × 2.

### Domain

1. Resolve cashier membership and register scope.
2. Resolve SKU from authorized organization catalogue.
3. Resolve current price.
4. Validate discount policy.
5. Compute tax according to configuration snapshot.
6. Validate inventory availability.
7. Create/finalize sale command with stable command identity.
8. Commit sale and inventory movement atomically.
9. Create payment intent or cash allocation according to selected payment method.
10. Write audit/outbox evidence.

### Outcome

```text
SaleFinalized
InventoryMovementPosted
PaymentIntentCreated / CashEventPosted
AuditEvent
OutboxMessage
```

A client retry with the same command ID returns the original outcome rather than creating a second sale.


# 83. Example Offline Sale

```text
NETWORK AVAILABLE
      ↓
client receives scoped product/price/inventory projection
      ↓
NETWORK LOST
      ↓
cashier sells SKU-X
      ↓
local durable command recorded
      ↓
local operational receipt produced according to offline policy
      ↓
NETWORK RETURNS
      ↓
sync command submitted
      ↓
server authenticates device/session context
      ↓
server validates command identity + scope + current state
      ↓
server applies authoritative transaction
      ↓
ack/checkpoint returned
```

Possible server response:

```text
ACCEPTED
```

or:

```text
SAFE_DUPLICATE
```

or:

```text
STOCK_CONFLICT
```

or:

```text
DEVICE_REVOKED
```

or another explicit conflict class.

The client must never interpret every HTTP 200 or sync acknowledgement as proof that an externally settled payment exists.


# 84. Example Refund

```text
Existing finalized sale
   ↓
Determine refundable balance
   ↓
Cashier requests refund
   ↓
Authorization / threshold
   ↓
Approval if required
   ↓
Create Refund aggregate
   ↓
Provider/internal refund operation
   ↓
Payment state updated
   ↓
Inventory return movement if physical goods returned
   ↓
Audit + outbox
```

A retry of the same refund command must not create two refunds.

The domain should preserve whether the refund was:

- customer-requested;
- automatically generated by a correction policy;
- manually approved;
- provider-confirmed;
- pending reconciliation.


# 85. Example Stock Adjustment

```text
Inventory variance detected
   ↓
Create adjustment request
   ↓
Reason required
   ↓
Authorization threshold check
   ↓
Approval if required
   ↓
Post adjustment movement
   ↓
Update balance projection
   ↓
Audit
```

The original count and adjustment evidence remain visible. “Set quantity to 100” without reason/evidence is deliberately not a valid domain command.


# 86. Example Provider Webhook

```text
Provider webhook arrives
   ↓
verify signature/authentication
   ↓
validate schema
   ↓
validate timestamp/replay controls
   ↓
persist/dedupe provider event
   ↓
load payment intent
   ↓
verify provider reference + amount + currency + expected state
   ↓
transition payment state
   ↓
write audit/outbox
   ↓
respond
```

A duplicate event must produce no second financial effect.

An event referring to an impossible internal payment state becomes a reconciliation exception rather than silently forcing the payment into `SUCCEEDED`.


# 87. Domain Observability Requirements

Every high-impact command should be traceable without logging sensitive payloads.

Minimum correlation chain:

```text
request_id
  ↓
command_id
  ↓
aggregate_id
  ↓
transaction
  ↓
audit_event_id
  ↓
outbox_message_id
```

For integrations:

```text
payment_intent_id
provider_transaction_id
provider_event_id
```

Metrics should count domain outcomes such as:

- authorization_denied;
- stock_conflict;
- duplicate_command;
- payment_mismatch;
- refund_rejected;
- reconciliation_exception;
- device_revoked_sync;
- tax_submission_failure.

Avoid high-cardinality raw user-controlled labels.


# 88. Domain Performance Constraints

Performance engineering should protect merchant workflows without weakening correctness.

### Hot paths

- product lookup;
- barcode lookup;
- price resolution;
- inventory availability;
- sale finalization;
- payment status retrieval;
- register operations.

### Avoid

- loading whole organization graphs;
- unbounded report queries in POS requests;
- serializing giant object graphs;
- synchronous external calls inside critical DB transactions;
- application-level global locks.

### Optimize safely

- indexed tenant+scope queries;
- compact DTOs;
- projections/read models;
- bounded pagination;
- prepared/checked SQL;
- local client caching;
- asynchronous reporting;
- measured concurrency control.

The architecture explicitly prioritizes realistic merchant concurrency and low-end device/network constraints over theoretical internet-scale benchmarks.


# 89. Domain Evolution Strategy

The domain model is expected to evolve. Evolution must preserve historical truth.

### Additive evolution

Prefer:

```text
new optional capability
new event version
new extension attribute
new state only when justified
```

### Breaking evolution

Requires ADR and migration plan when it changes:

- meaning of money;
- inventory semantics;
- sale lifecycle;
- tenant boundary;
- payment state transitions;
- sync semantics;
- authorization meaning.

### Historical compatibility

When a rule changes, preserve the version or evidence needed to explain historical records created under the prior policy.


# 90. Domain Anti-Patterns — Prohibited

### 1. Giant Universal Entity

Do not create one `BusinessTransaction` class with dozens of nullable fields covering sales, purchases, refunds and payments.

### 2. Table-as-Domain

Do not assume every PostgreSQL table is a domain aggregate.

### 3. Generic PATCH for Everything

It makes state transitions and financial invariants difficult to enforce.

### 4. UI-Owned Rules

Flutter/TypeScript may provide UX validation but cannot own business authority.

### 5. Current-State-Only History

Do not rely on current product price/name/tax configuration to interpret historical sales.

### 6. Last-Write-Wins Everywhere

Unsafe for stock, money, approvals and correction workflows.

### 7. Provider Truth as Internal Truth

A provider is evidence, not a license to bypass Sitolo's own state/invariants.

### 8. Soft Delete Everything Forever

Retention and deletion must be deliberately modeled; not every row needs infinite tombstones.

### 9. Microservice-First Domain Design

Do not split contexts across network boundaries before the business/data ownership is clear.

### 10. Boolean State Machines

`is_paid`, `is_refunded`, `is_active` combinations can create impossible states. Use explicit state machines where lifecycle complexity exists.


# 91. Definition of Domain Ready

A domain is ready for implementation only when:

```text
[ ] ubiquitous language defined
[ ] bounded context identified
[ ] aggregate root chosen
[ ] entity/value object distinctions defined
[ ] lifecycle/state machine defined
[ ] invariants written
[ ] authorization requirements defined
[ ] tenant scope defined
[ ] persistence owner defined
[ ] transaction boundary defined
[ ] idempotency semantics defined
[ ] concurrency semantics defined
[ ] offline behavior defined where relevant
[ ] events defined
[ ] audit requirements defined
[ ] error taxonomy defined
[ ] read models defined where needed
[ ] migration strategy defined
[ ] security tests identified
[ ] failure states identified
[ ] operational metrics identified
```

If several of these are unknown, the implementation should be treated as a prototype rather than production-bound code.


# 92. Definition of Domain Done

A domain implementation is done when:

1. Its invariants exist in executable tests.
2. Its state transitions reject illegal transitions.
3. Its authorization is enforced server-side.
4. Its tenant boundaries are covered by negative tests.
5. Its database constraints match the documented invariants.
6. Its transaction boundaries have been tested under concurrency.
7. Its retries/idempotency are tested.
8. Its audit evidence is persisted.
9. Its events/outbox behavior is tested.
10. Its offline behavior is tested if applicable.
11. Its public API is covered by contract tests.
12. Its failure semantics are observable.
13. Its migrations are tested.
14. Its rollback/recovery behavior is documented.
15. Its implementation does not introduce a competing source of truth.


# 93. Phase 0 Dependency Graph

`domain_model.md` depends conceptually on the already completed security and system architecture documents.

```text
business_model_design.md
        │
        ├──────────────┐
        ▼              ▼
    sitolo.md   system_architecture_design.md
        │              │
        └──────┬───────┘
               ▼
    security_architecture_design.md
               │
               ▼
    security_implementation_spec.md
               │
               ▼
        domain_model.md  ← THIS FILE
               │
               ├── database_design.md
               ├── api_contract.md
               ├── auth_authorization_spec.md
               └── sync_protocol.md
```

This ordering prevents database tables, endpoints and client state from becoming accidental definitions of the business model.


# 94. Traceability to Existing Sitolo Architecture

| Existing architectural decision | Domain-model consequence |
|---|---|
| Rust decision engine | domain behavior lives in Rust; clients are not authoritative |
| PostgreSQL authority | transactional business state belongs to relational persistence |
| SQLite offline store | local entities are continuity projections + command journal, not second permanent authority |
| Modular monolith | contexts are code boundaries first; no premature network decomposition |
| Custom sync protocol | offline commands and conflict classes are domain concepts |
| Transactional outbox | domain facts and durable side-effect intent commit together |
| Append-only financial corrections | Sale/Refund/Reversal/Return have explicit relationships |
| Payment adapters | provider details remain outside core financial semantics |
| MRA EIS adapter | tax submission state is distinct from local sale facts |
| Tenant isolation | every business aggregate carries/derives organizational scope |
| Auditability | sensitive transitions emit durable evidence |
| Pharmacy extension | regulated inventory behavior overlays generic stock model |
| Agro extension | batch/unit/formulation semantics reuse inventory core |


# 95. Research Basis

This document was researched against the current Sitolo project documents and current technical references.

### Project sources

- `sitolo.md`: product scope, customer types, business hierarchy, domain scope, sales/inventory/payment/EIS principles.
- `business_model_design.md`: commercial flywheel and the operating-system positioning.
- `system_architecture_design.md`: Rust-first stack, PostgreSQL authority, Flutter/Tauri clients, modular monolith, sync, outbox, testing and implementation rules.
- `security_architecture_design.md`: tenant isolation, authorization, business invariants as security controls, mandatory security gates.
- `security_implementation_spec.md`: implementation governance and enforcement expectations.

The existing Sitolo source states that the business loop is `PROCURE → RECEIVE → STOCK → PRICE → SELL → COLLECT → RECONCILE → REPORT`, that tenant isolation is a security boundary, that financial events are append-only, and that authorization is server-enforced. fileciteturn10file2L304-L324

The system architecture defines identity/tenant/authz, catalogue, pricing, procurement, inventory, sales, cash, payments, reconciliation, tax/EIS, audit and reporting as core modules, with pharmacy, agro-dealer and wholesale as vertical extensions. fileciteturn10file1L176-L221

The same architecture explicitly recommends implementing identity/tenant foundations, then catalogue/pricing, inventory, POS/cash, offline synchronization, procurement, payments/reconciliation, EIS, reporting, enterprise operations and vertical extensions. fileciteturn10file1L224-L254

The security implementation baseline identifies typed tenant/resource IDs, business invariants, fail-closed behavior and clear build-versus-reuse boundaries. fileciteturn10file3L378-L436

### External technical references

- Martin Fowler, Bounded Context — strategic DDD separation and explicit relationships: https://martinfowler.com/bliki/BoundedContext.html
- Martin Fowler, Domain-Driven Design: https://martinfowler.com/bliki/DomainDrivenDesign.html
- Martin Fowler, discussion of AI-assisted domain modeling and aggregate consistency: https://martinfowler.com/articles/pushing-ai-autonomy.html
- PostgreSQL 18 documentation — constraints: https://www.postgresql.org/docs/current/ddl-constraints.html
- PostgreSQL 18 documentation — row security policies: https://www.postgresql.org/docs/current/ddl-rowsecurity.html
- SQLx `query!` macro documentation: https://docs.rs/sqlx/latest/sqlx/macro.query.html

PostgreSQL 18 is the current supported major version referenced in the documentation at the time of preparation, with the 18.6 release dated August 13, 2026. The documented constraint model is directly relevant to Sitolo's use of primary keys, foreign keys, uniqueness and check constraints. citeturn370984view0

The domain model deliberately does not turn these external sources into rigid dogma. DDD patterns are applied where they help preserve Sitolo's business invariants; SQL constraints are applied where they provide strong protection; and infrastructure choices remain evidence-driven.


# 96. Final Domain Contract

The Sitolo domain contract is:

> **The domain model owns business meaning. Rust owns domain decisions. PostgreSQL owns authoritative transactional state. Clients provide intent and local continuity. External systems provide evidence through explicit adapters. Financial and inventory facts are corrected through explicit compensating events. Tenant and organizational scope are part of every protected operation. State transitions are explicit. Business invariants are executable. Derived data is rebuildable.**

A future implementation must be judged against that contract, not merely against whether its endpoints return 200 responses.

The domain is successful when the system can answer, for any important transaction:

```text
What happened?
Who initiated it?
Which organization/branch/register/device was involved?
Was the actor authorized?
What business rules were applied?
What inventory changed?
What money changed?
What external evidence exists?
What audit evidence exists?
What happened if the network failed?
What happened if the request was retried?
Can the history be reconstructed?
Can the result be reproduced from authoritative records?
```

That is the level of correctness required before the next implementation contracts—database, API, authorization and synchronization—are allowed to crystallize around the model.

# 97. Canonical Entity Catalog

This section is the implementation vocabulary. Names here should be treated as canonical unless an ADR explicitly changes them. Database table names, API resources and Rust type names may use different casing conventions, but they should map to these concepts rather than silently inventing competing meanings.

## 97.1 Platform and tenancy entities

| Entity | Identity | Scope | Mutable lifecycle | Historical importance |
|---|---|---|---|---|
| Platform | `PlatformId` | global | active/degraded | low for merchant history, high for control-plane audit |
| Organization | `OrganizationId` | global | trial/active/suspended/closed | very high |
| BusinessEntity | `BusinessEntityId` | organization | active/inactive | high |
| Branch | `BranchId` | organization | setup/active/suspended/closed | high |
| Location | `LocationId` | branch | active/inactive | high |
| Warehouse | `WarehouseId` | branch | active/inactive | high |
| Register | `RegisterId` | branch/location | active/disabled | high |
| Membership | `MembershipId` | organization | invited/active/suspended/revoked | high |
| Role | `RoleId` | organization/platform | active/retired | high |
| Permission | `PermissionId` | platform | fixed/versioned | medium |
| Device | `DeviceId` | organization/branch | pending/active/revoked/retired | high |
| Session | `SessionId` | user/device | active/expired/revoked | security evidence |

## 97.2 Catalogue entities

| Entity | Purpose |
|---|---|
| Product | conceptual product family |
| SKU | concrete sellable/inventoriable item |
| Category | classification hierarchy |
| Brand | optional product classification |
| UnitDefinition | unit semantics |
| UnitConversion | conversion between units |
| Barcode | scanner identifier |
| ProductSupplier | supplier-product relationship |
| ProductAttributeDefinition | controlled extension attributes |
| ProductAttributeValue | product-specific extension data |

## 97.3 Pricing entities

| Entity | Purpose |
|---|---|
| PriceList | named pricing policy/scope |
| PriceVersion | effective-dated price fact |
| Promotion | promotional eligibility/rule |
| PromotionCondition | condition within promotion |
| PromotionBenefit | price/discount effect |
| DiscountPolicy | authorization thresholds |
| PriceOverride | explicit approved transaction override |

## 97.4 Procurement entities

| Entity | Purpose |
|---|---|
| Supplier | supplier master |
| PurchaseOrder | inbound purchasing intent |
| PurchaseOrderLine | requested SKU/quantity/cost |
| GoodsReceipt | physically received stock evidence |
| GoodsReceiptLine | received SKU/quantity/lot data |
| ProcurementApproval | approval evidence |
| SupplierInvoiceReference | external invoice/reference metadata |

## 97.5 Inventory entities

| Entity | Purpose |
|---|---|
| InventoryPosition | current balance projection for SKU/location scope |
| InventoryLot | lot/batch identity and state |
| InventoryMovement | immutable stock event |
| InventoryTransfer | transfer intent and completed relationship |
| StockCount | count workflow |
| StockCountLine | counted SKU quantity |
| StockAdjustment | authorized correction request |
| InventoryReservation | reserved quantity when used |
| InventoryPolicy | stock/lot/negative-stock rules |

## 97.6 Sales entities

| Entity | Purpose |
|---|---|
| Sale | commercial transaction aggregate |
| SaleLine | historical commercial line snapshot |
| SalePaymentAllocation | how a sale is allocated to payment instruments |
| Return | returned goods relationship |
| ReturnLine | returned SKU/quantity |
| Void | pre-finalization cancellation evidence where modeled separately |
| Reversal | compensating transaction relationship |
| Refund | money-return operation |
| RefundAllocation | refund amount allocation |

## 97.7 Cash entities

| Entity | Purpose |
|---|---|
| RegisterSession | open/close control boundary |
| CashEvent | immutable cash movement |
| CashCount | physical cash count |
| CashVariance | expected vs counted difference |
| CashPolicy | thresholds and controls |

## 97.8 Payment and reconciliation entities

| Entity | Purpose |
|---|---|
| PaymentIntent | internal expected payment |
| PaymentAttempt | provider interaction attempt |
| ProviderEvent | external callback/event evidence |
| ProviderReference | external identifier linkage |
| PaymentAllocation | payment applied to business transaction |
| SettlementRecord | provider/bank settlement evidence |
| ReconciliationCase | uncertainty/mismatch workflow |
| ReconciliationMatch | accepted evidence match |

## 97.9 Tax entities

| Entity | Purpose |
|---|---|
| TaxProfile | organization tax configuration |
| TaxCategory | tax classification |
| TaxRuleVersion | effective tax calculation rule |
| TaxConfigurationSnapshot | historical calculation inputs |
| TaxSubmission | submission lifecycle |
| TaxReceiptEvidence | accepted tax authority evidence |
| TaxException | unresolved tax integration issue |

## 97.10 Customer and supplier relationship entities

The `Customer` and `Supplier` concepts intentionally do not require a universal CRM. They exist as business counterparties used where the operating flow needs them.

Customer-related entities may include `Customer`, `CustomerAccount`, `CustomerCreditProfile`, `CustomerConsent`, and `CustomerSaleReference`.

Supplier-related entities may include `Supplier`, `SupplierProduct`, `SupplierTerms`, and `SupplierContact`.

## 97.11 Platform control-plane entities

| Entity | Purpose |
|---|---|
| Plan | commercial subscription offering |
| Entitlement | capability or limit derived from plan |
| Subscription | organization subscription lifecycle |
| BillingPeriod | usage/billing period |
| UsageMeter | measured usage |
| FeatureFlag | release/control-plane behavior |
| ExportJob | bulk data export workflow |
| SupportAccessGrant | controlled support access |
| AuditEvent | durable action evidence |
| Integration | configured external integration |
| IntegrationCredentialRef | secret-store reference rather than secret material |

# 98. Organization Aggregate Contract

The `Organization` is the principal business tenant aggregate boundary for tenant lifecycle and selected high-level configuration.

## 98.1 Responsibilities

- Establish organization identity.
- Maintain organization lifecycle.
- Maintain business profile references.
- Establish default operating configuration.
- Establish permitted branches.
- Establish default policy references.
- Establish subscription relationship.

## 98.2 Non-responsibilities

The organization aggregate should not load and mutate all products, users, sales and inventory as one giant object graph.

## 98.3 Allowed transitions

```text
PROVISIONING → TRIAL
TRIAL → ACTIVE
ACTIVE → SUSPENDED
SUSPENDED → ACTIVE
ACTIVE → CLOSED
SUSPENDED → CLOSED
```

Any transition that affects merchant operational access requires explicit policy and audit evidence.

# 99. Branch Aggregate Contract

A branch is an operational boundary under an organization.

## Responsibilities

- branch identity;
- address/location metadata;
- operating status;
- branch-level configuration;
- allowed staff scope;
- location and register references.

## Invariants

- branch belongs to exactly one organization;
- branch cannot be reassigned between organizations through an ordinary update;
- branch closure does not destroy historical transactions;
- new operational commands are blocked once closed except explicitly permitted administrative actions.

# 100. Membership Aggregate Contract

A membership is the authorization relationship between a user and organization.

## Core fields

```text
membership_id
user_id
organization_id
status
role_assignments
scope_assignments
created_at
updated_at
revoked_at
```

## Invariants

- one effective membership relationship per user/organization unless a formally justified historical model exists;
- revoked membership cannot authorize new commands;
- changing a high-privilege role is auditable;
- scope cannot exceed the organization's actual resource hierarchy;
- client-supplied role names never establish authority.

# 101. Device Aggregate Contract

A device is a security subject and an offline continuity endpoint.

## Lifecycle

```text
PENDING_REGISTRATION
      ↓
ACTIVE
      ↓
SUSPENDED
      ↓
REVOKED
      ↓
RETIRED
```

## Device invariants

1. Device ID is globally unique.
2. Device identity is not itself a credential.
3. Revocation takes effect for restricted operations.
4. Offline commands from revoked devices are evaluated against explicit recovery policy.
5. Device replacement does not erase historical terminal/register evidence.
6. Device registration is organization-scoped.
7. A device cannot silently change organization ownership.

# 102. Product and SKU Aggregate Contract

The catalogue distinguishes conceptual identity from sellable identity.

```text
Product
  ├── SKU A
  ├── SKU B
  └── SKU C
```

A product family may be discontinued while an individual SKU remains active, or vice versa according to policy. The implementation must define exactly which lifecycle controls saleability and which controls catalogue visibility.

## SKU invariants

- SKU belongs to one organization.
- SKU must have a valid unit model before activation when the SKU is stockable.
- A barcode can only resolve to an unambiguous SKU within the relevant scope.
- An SKU that has historical activity cannot be hard-deleted into nonexistence.
- Product classification changes do not mutate prior sale snapshots.

# 103. Unit Model Contract

The unit system must be explicit enough to support:

```text
piece
pack
box
carton
kilogram
gram
litre
millilitre
bag
bottle
metre
```

The actual supported list is configurable by country/domain policy.

## Unit conversion invariant

For an explicitly configured conversion:

```text
1 higher_unit = N lower_units
N > 0
```

Conversions must not be inferred from arbitrary text such as `"1 box maybe 12"`.

Changing a conversion factor after transactions exist requires careful versioning because it can otherwise rewrite the meaning of existing inventory quantities.

# 104. Price Version Contract

A `PriceVersion` is effective-dated commercial configuration.

```text
PriceVersion
  ├── sku_id
  ├── scope
  ├── amount
  ├── currency
  ├── valid_from
  ├── valid_to
  └── provenance
```

The pricing resolver returns a deterministic decision rather than only a number:

```text
PricingDecision
  ├── base_price
  ├── applied_price_list
  ├── promotion_ids
  ├── manual_override?
  ├── discount_total
  ├── authorization_reference?
  └── final_unit_price
```

This is useful for auditability and later dispute resolution.

# 105. Supplier and Procurement Contracts

## Supplier invariants

- supplier is organization-scoped;
- historical purchase records preserve supplier reference;
- supplier deactivation does not erase historical procurement.

## Purchase order invariants

- line quantities are positive;
- referenced SKUs belong to the organization;
- approval transitions are explicit;
- received quantity cannot silently exceed policy limits without being represented as an over-receipt/discrepancy;
- cancellation is a state transition.

## Goods receipt invariants

The receipt is evidence of physical receipt, not merely an update to `purchase_order.received_quantity`.

Posting a receipt must create the corresponding inventory consequences exactly once.

# 106. Inventory Lot Contract

Lots are required where the vertical/business policy needs traceability.

Possible fields:

```text
lot_id
sku_id
supplier_id?
lot_number
manufacture_date?
expiry_date?
received_at
status
quantity metadata
```

## Lot states

```text
AVAILABLE
QUARANTINED
EXPIRED
RECALLED
DAMAGED
RETURNED
DISPOSED
```

A lot state must be evaluated during sale allocation. A UI displaying a green “in stock” indicator is not enough to authorize a sale against an expired or recalled lot.

# 107. Inventory Position Contract

`InventoryPosition` is a projection optimized for availability and operational queries.

Dimensions may include:

```text
organization
location
SKU
lot (optional)
state
```

A position does not erase the movement ledger.

## Position invariants

- current balance cannot be manufactured by arbitrary client mutation;
- balance updates happen in the same transaction as the authoritative movement when the operation requires atomic projection consistency;
- recovery can rebuild a position from ledger history if required;
- a stale client projection cannot force an authoritative negative inventory result.

# 108. Inventory Transfer Contract

A transfer is one business operation even though it creates multiple movements.

```text
Transfer REQUESTED
     ↓
APPROVED (if required)
     ↓
IN_TRANSIT (optional workflow)
     ↓
COMPLETED
```

Where stock physically remains in transit, the model may represent it as an explicit state rather than pretending it is simultaneously available at both locations.

This is important for multi-branch businesses and prevents stock inflation through duplicated transfer events.

# 109. Sale Aggregate — Detailed Contract

The sale is one of Sitolo's highest-integrity aggregates.

## 109.1 Sale identity

```text
sale_id
sale_number
organization_id
branch_id
location_id
register_id
device_id
actor_id
command_id
```

## 109.2 Commercial snapshot

```text
currency
subtotal
discount_total
tax_total
gross_total
rounding_adjustment
final_total
```

## 109.3 Lifecycle

```text
DRAFT
 → PENDING_CONFIRMATION
 → FINALIZED
```

Correction relationships are separate:

```text
FINALIZED
   ├── VOID (only where legally/state-valid)
   ├── RETURN
   ├── REFUND
   └── REVERSAL
```

## 109.4 Finalization invariant

Once finalization commits, the sale's economic meaning cannot be rewritten by ordinary mutation.

Fields that may still change after finalization, if any, must be purely non-economic metadata and should be explicitly identified rather than assuming all columns remain editable.

# 110. Sale Calculation Contract

The calculation pipeline should be deterministic:

```text
catalogue selection
     ↓
base price
     ↓
promotion evaluation
     ↓
manual discount evaluation
     ↓
net line amount
     ↓
tax evaluation
     ↓
rounding
     ↓
sale total
```

The calculation should produce both the result and enough structured evidence to reproduce it.

For example:

```text
Line 1
  quantity = 2
  unit_price = 1000
  promotion_discount = 100
  tax_rate = 16%
```

The exact tax rules are jurisdiction/configuration dependent. The domain model only establishes the requirement that applied rules are deterministic and attributable.

# 111. Return Eligibility Contract

Refunds and returns must not query “the sale total” and assume that it is all still eligible.

The domain should compute:

```text
original eligible quantity/value
- previously returned quantity/value
- previously reversed/refunded quantity/value
= remaining eligible quantity/value
```

The result is an `EligibilityDecision` containing:

```text
eligible_quantity
eligible_amount
already_corrected_quantity
already_corrected_amount
rejection_reason?
```

This makes over-refund prevention testable.

# 112. Refund Aggregate Contract

A refund is a separate economic operation.

## Lifecycle

```text
REQUESTED
 ↓
PENDING_APPROVAL? 
 ↓
APPROVED
 ↓
SUBMITTING
 ├── COMPLETED
 ├── FAILED
 └── RECONCILIATION_REQUIRED
```

A provider-confirmed refund is not inferred from a frontend success screen.

## Invariants

- amount <= eligible amount;
- currency matches eligible transaction unless explicit conversion is supported;
- duplicate command cannot create another refund;
- requester's own approval is blocked where separation of duties applies.

# 113. RegisterSession Contract

The register session is the control boundary for a physical cash drawer.

Fields may include:

```text
register_session_id
register_id
branch_id
device_id
opened_by
opened_at
opening_float
closed_by?
closed_at?
status
```

A session has one opening event and at most one successful close.

Cash events belong to the session or are explicitly outside-session adjustments with separate policy.

# 114. Payment Allocation Contract

A sale can have one or multiple payment allocations where split tender is supported.

Example:

```text
Sale total: MK 10,000
Cash:       MK 4,000
Airtel:     MK 6,000
```

The aggregate invariant is:

```text
sum(payment_allocations) == amount being settled
```

unless the sale is explicitly partially settled.

Payment allocation does not itself prove provider settlement; it expresses internal accounting intent/state and must be reconciled with external evidence where necessary.

# 115. Payment Provider Event Contract

A provider event needs:

```text
provider
provider_event_id
provider_transaction_id?
received_at
provider_occurred_at?
payload_digest / evidence reference where appropriate
verification_status
processing_status
```

The event identity must be deduplicated before downstream financial effects can occur.

Unknown provider events are evidence requiring investigation, not an excuse to invent internal state.

# 116. Reconciliation Match Contract

A successful reconciliation match should record:

```text
reconciliation_case_id
match_method
internal_reference
provider_reference
amount
currency
time evidence
operator/system decision
```

A match method such as “exact provider transaction ID” is materially stronger than “same amount and same day.” The model should preserve the method to support audit and later quality analysis.

# 117. Tax Configuration Snapshot Contract

Tax calculation requires historical reproducibility.

A sale should reference the effective tax configuration/version used at finalization or preserve the required tax snapshot values directly.

```text
TaxConfigurationSnapshot
  ├── jurisdiction
  ├── configuration_version
  ├── tax categories applied
  ├── rates
  ├── exemptions
  └── effective period
```

This prevents later tax configuration changes from silently changing historical reports.

# 118. ExportJob Domain Contract

Export is a bulk read workflow and therefore a distinct domain operation.

## Lifecycle

```text
REQUESTED
 → AUTHORIZED
 → QUEUED
 → RUNNING
 → COMPLETED
```

Failure states:

```text
FAILED
CANCELLED
EXPIRED
```

The export definition must capture an immutable authorized scope. Changing a user's permissions after an export begins must not accidentally broaden the job; scope is established at authorization time and governed by explicit consistency policy.

# 119. Subscription / Entitlement Contract

Entitlement answers:

> Is this organization currently allowed to use this capability under the platform's commercial policy?

It is not an authorization replacement.

A user may have permission to perform an operation but lack a subscribed entitlement. Conversely, a subscribed organization does not grant every user administrative authority.

```text
Identity authorization
        AND
Organization entitlement
        AND
Domain state
        =
operation may proceed
```

# 120. Support Access Contract

Support access is a privileged domain operation.

```text
REQUESTED
 ↓
APPROVED
 ↓
ACTIVE
 ↓
EXPIRED / REVOKED
```

Support access should identify:

- support actor;
- customer/organization;
- purpose;
- permitted operation/data scope;
- start/end time;
- approval evidence.

Support should not receive an unrestricted “become tenant admin” primitive.

# 121. Cross-Context Relationship Rules

### Identity → Tenant

Membership establishes organizational authority.

### Tenant → Catalogue

Catalogue objects belong to exactly one organization by default.

### Catalogue → Pricing

Pricing references sellable SKUs; price history is separate from product master history.

### Catalogue → Inventory

Inventory references SKU identity but owns stock state.

### Procurement → Inventory

Posted goods receipts create stock movements.

### Inventory → Sales

Finalized stock-consuming sales create inventory movements.

### Sales → Payments

Sales establish expected payment obligations; payment context establishes settlement evidence.

### Sales → Tax

Tax domain evaluates/submits configured tax facts but does not redefine the sale's local economics.

### Payments → Reconciliation

Provider evidence is matched against internal intents/allocations.

### All domains → Audit

High-impact state changes emit audit evidence.

# 122. Domain Boundary Rules for Shared Types

Some concepts legitimately cross contexts. The solution is not to create a universal mutable object.

Examples:

```text
SkuId
Money
Currency
OrganizationId
BranchId
UserId
DeviceId
Timestamp
```

Shared types should be immutable/value-oriented wherever possible.

Do not share large mutable domain entities between contexts.

Bad:

```text
shared::Customer
```

used directly as the persistence model, sales aggregate, billing object, CRM projection and support record.

Better:

```text
customer::CustomerReference
sales::CustomerSnapshot
billing::AccountReference
```

Each context owns what it needs.

# 123. Domain Boundary Rules for External Providers

External references are values, not domain entities owned by the provider.

For example:

```text
ProviderTransactionId("abc")
```

is a value attached to a Sitolo payment attempt/evidence record.

Do not allow provider SDK structs to leak into the core domain model.

The adapter converts:

```text
Provider SDK response
      ↓
validated integration DTO
      ↓
domain evidence/value
```

# 124. State Transition Authorization Matrix

| Operation | Cashier | Manager | Owner | Finance | Inventory | Pharmacist |
|---|---:|---:|---:|---:|---:|---:|
| Create sale | yes | yes | yes | policy | policy | yes |
| Finalize sale | yes | yes | yes | policy | policy | yes |
| Standard discount | limited | yes | yes | policy | no | policy |
| High-value refund | request | approve | approve | approve | no | policy |
| Stock adjustment | request | approve | approve | no | yes | policy |
| Purchase approval | no/limited | yes | yes | policy | policy | no |
| Register close | yes | yes | yes | yes | no | yes |
| User role change | no | limited | yes | no | no | no |
| Tenant security policy | no | limited | yes | no | no | no |

This table is intentionally policy-oriented rather than an absolute authorization implementation. The final permission model should use named permissions and scopes rather than role-name conditionals.

# 125. Domain Invariants by Severity

## Tier 0 — Catastrophic

Violation can compromise tenants or falsify financial truth.

```text
tenant isolation
unauthorized role escalation
financial double-posting
payment duplicate settlement
sale mutation after finalization
```

Any Tier 0 invariant failure blocks release.

## Tier 1 — Critical operational

```text
negative stock where policy forbids
refund overrun
transfer duplication
register double-close
revoked device restricted-operation acceptance
```

Block release if reproducible or not formally mitigated.

## Tier 2 — Material correctness

```text
incorrect report projection
stale price display
non-critical audit metadata omission
```

Fix according to release policy but do not treat all Tier 2 defects identically.

# 126. Domain Test Naming Convention

Tests should name the business rule, not only the method.

Good:

```text
finalizing_same_sale_command_twice_is_idempotent
cashier_cannot_refund_beyond_remaining_eligible_amount
cross_tenant_sale_lookup_is_denied
expired_lot_cannot_be_allocated_to_sale
transfer_posts_source_and_destination_atomically
provider_event_replay_does_not_double_settle_payment
```

Avoid vague names such as:

```text
works
sale_test
payment_test
inventory_test
```

# 127. Contract Test Matrix

Every aggregate should have at least:

```text
happy path
invalid input
unauthorized actor
wrong tenant
wrong branch
invalid state transition
duplicate command
concurrent command
retry after failure
historical immutability
serialization/deserialization
migration compatibility where applicable
```

Vertical extensions additionally need:

```text
pharmacy policy tests
agro policy tests
wholesale policy tests
```

# 128. Persistence Independence Test

The domain unit tests should not require PostgreSQL for pure business behavior such as:

- state transitions;
- money arithmetic;
- discount evaluation;
- refund eligibility;
- quantity conversion;
- tax calculation given a supplied rule;
- permission interpretation where pure.

Integration tests must use PostgreSQL for behavior that depends on:

- transaction semantics;
- uniqueness;
- foreign keys;
- RLS;
- locking;
- query correctness;
- migration correctness.

This keeps feedback fast while still testing the authoritative persistence boundary realistically.

# 129. Domain-to-Database Mapping Principles

The next document, `database_design.md`, must derive from this model rather than independently redefining it.

For every proposed table ask:

```text
Which domain concept does this represent?
Who owns it?
What aggregate does it belong to?
What is its lifecycle?
What is immutable?
What relationships require foreign keys?
What uniqueness rule exists?
What tenant scope exists?
What state is derived?
What can be rebuilt?
```

If a table cannot answer those questions, it is probably premature or poorly named.

# 130. Domain-to-API Mapping Principles

The subsequent API contract must expose commands and resources that preserve these state boundaries.

```text
Domain object
    ↓
Application command
    ↓
API resource/action
```

not:

```text
Database table
    ↓
PATCH every column
```

A domain model that cannot survive direct API invocation is not a finished domain model.

# 131. Domain-to-Sync Mapping Principles

The subsequent sync protocol must use domain commands, not raw row synchronization.

Bad:

```text
UPDATE inventory SET quantity = 17
```

Better:

```text
StockAdjustmentRequested
SaleCreated
GoodsReceiptPosted
TransferRequested
```

The server can then validate the command against current state and business invariants.

# 132. Domain-to-Event Mapping Principles

Events describe accepted facts.

Bad:

```text
UpdateSaleRequested
```

Better:

```text
SaleFinalized
```

The former is an intention; the latter is a fact.

Commands may fail. Facts should represent successful transitions.

# 133. Domain-to-Observability Mapping Principles

Metrics should be tied to domain outcomes.

Useful examples:

```text
sales.finalized.count
sales.finalization.conflict.count
inventory.stock_conflict.count
refund.requested.count
refund.rejected.count
payments.reconciliation_exception.count
sync.command.rejected.count
sync.command.safe_duplicate.count
register.variance.count
```

Telemetry names should not expose tenant names, raw customer data, payment secrets or arbitrary user-supplied content.

# 134. Domain Change-Control Rules

Any change to the following requires domain review and likely an ADR:

- aggregate boundary;
- financial invariant;
- inventory invariant;
- tenant relationship;
- authorization precondition;
- state-machine transition;
- payment lifecycle;
- offline acceptance semantics;
- event meaning;
- historical immutability rule;
- tax evidence semantics.

Routine internal refactors that preserve externally observable domain contracts do not need a new ADR.

# 135. What Is Deliberately Not in the Core Domain

Sitolo should resist becoming everything for everyone.

The core does not automatically include:

- a full general ledger;
- a bank;
- a wallet/custodial payment system;
- a lending engine;
- an insurance engine;
- a full HR/payroll system;
- a generic CRM suite;
- an arbitrary workflow engine;
- an AI decision-maker for financial truth.

Integrations can exist around those capabilities, but their inclusion would change the domain boundary and require new evidence and decisions.

# 136. Domain Readiness Gate

`domain_model.md` is ready to hand to database/API implementation when all of the following hold:

```text
[X] bounded contexts defined
[X] canonical vocabulary defined
[X] aggregate roots identified
[X] entities identified
[X] value objects identified
[X] lifecycle states defined
[X] critical state transitions defined
[X] cross-domain relationships defined
[X] tenant scope defined
[X] financial invariants defined
[X] inventory invariants defined
[X] payment/reconciliation semantics defined
[X] offline command implications defined
[X] audit/evidence semantics defined
[X] extension strategy defined
[X] build-vs-reuse boundary preserved
[X] open decisions explicitly recorded
[X] research references recorded
```

# 137. Final Domain Model Statement

Sitolo's domain is not “products + sales + users.” It is a connected operating system for the merchant's economic activity.

The authoritative chain is:

```text
IDENTITY
   ↓
ORGANIZATION / BRANCH / LOCATION
   ↓
CATALOGUE
   ↓
PRICE
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
TAX / EIS
   ↓
REPORTING
```

Across the entire chain:

```text
AUTHORIZATION
AUDIT
IDEMPOTENCY
OFFLINE CONTINUITY
CONCURRENCY CONTROL
OBSERVABILITY
RECOVERY
```

The central engineering rule remains:

> **Do not let the database schema, UI state, external provider response, or offline client cache accidentally become the business model. The business model must be explicit, typed, stateful, tenant-scoped, transactionally protected, testable, and preserved independently of any one interface.**

The next implementation artifact, `database_design.md`, must map this model into PostgreSQL tables, relationships, constraints, indexes, RLS boundaries, migrations, transaction requirements and derived/read-model structures without inventing a competing domain vocabulary.

---

**Document end — Sitolo Domain Model Design, Phase 0 / File 02 of 16.**
