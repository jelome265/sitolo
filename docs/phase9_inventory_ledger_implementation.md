# Sitolo — Phase 9: Inventory Ledger Implementation Specification

**Document:** `phase9_inventory_ledger_implementation.md`  
**Phase:** 9 — Inventory Ledger  
**Status:** Binding implementation specification  
**Backend:** Rust + Axum + Tokio  
**Authoritative store:** PostgreSQL 18  
**Client continuity:** SQLite through the existing offline protocol  
**Architecture:** Modular monolith first; workers/adapters; service extraction only when independently justified  
**Prerequisites:** Phases 0–8 complete  

---

# 0. Executive Position

Sitolo inventory is not a mutable `stock_quantity` field with a history table attached to it. Inventory is an authoritative, auditable, concurrency-sensitive ledger domain.

The implementation MUST treat stock as the consequence of accepted inventory events and MUST preserve enough durable evidence to reconstruct why a balance exists.

The core model is:

```text
RECEIPT / OPENING BALANCE / TRANSFER / SALE / RETURN / ADJUSTMENT / COUNT / WRITE-OFF
                                      |
                                      v
                           INVENTORY TRANSACTION
                                      |
                                      v
                              LEDGER POSTING(S)
                                      |
                         +------------+------------+
                         |                         |
                         v                         v
                 CURRENT BALANCE             COST STATE
                         |                         |
                         v                         v
                operational reads          valuation/reporting
```

PostgreSQL remains authoritative. The Rust application determines whether a requested business command is valid; PostgreSQL provides durable transactional state, constraints, locking and RLS defense in depth. This matches the established architecture in which PostgreSQL is authoritative and inventory is ledger-backed rather than a client-controlled balance. fileciteturn10file2L302-L342

The inventory implementation MUST support:

- multi-tenant, multi-branch and multi-location operation;
- warehouses and selling locations as distinct concepts;
- SKU-level stock identity;
- units and controlled unit conversion;
- optional lot/batch and serialized inventory;
- expiry-aware inventory for regulated extensions;
- reservations where business workflows require them;
- receipts, transfers, sales consumption, returns, adjustments and write-offs;
- stock counts with controlled variance posting;
- positive and negative movements;
- deterministic costing policies;
- immutable finalized history;
- offline command reconciliation;
- idempotency and replay protection;
- concurrency-safe stock deduction;
- bounded repair/rebuild operations;
- auditable corrections rather than destructive edits;
- real PostgreSQL verification;
- machine-enforced security and release gates.

The implementation MUST NOT let a cached client balance, UI state, external webhook, or arbitrary API quantity become authoritative stock.

---

# 1. Relationship to Existing Sitolo Contracts

This document is subordinate to the previously frozen project contracts.

```text
LAW / REGULATION / PROVIDER CONTRACT
                 |
                 v
BUSINESS MODEL
                 |
                 v
PRODUCT + DOMAIN MODEL
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
PHASE 8 PRODUCT / CATALOGUE
                 |
                 v
THIS INVENTORY IMPLEMENTATION
                 |
                 v
RUST COMMANDS + SQL + MIGRATIONS + TESTS
```

The domain model already defines Product → SKU identity, tenant ownership, units/conversions, branch/location/warehouse concepts, and the invariant that inventory and business history must retain historical meaning even when catalogue state changes. fileciteturn10file0L11-L30 fileciteturn10file1L211-L270

The API contract requires business commands rather than generic CRUD, explicit server-side authorization, idempotency for retryable mutations, bounded operations, and fail-closed behavior for stale or ambiguous requests. fileciteturn11file2L287-L337

The PostgreSQL phase requires real PostgreSQL for constraints, RLS, transaction behavior, locks and concurrency rather than mocked database assumptions. fileciteturn11file3L468-L474

The existing security-test and CI contracts require negative authorization tests, tenant isolation, concurrency evidence and release-blocking failures. fileciteturn9file1L215-L239

Nothing in this phase may introduce a second tenant model, an alternate source of truth, generic mutable stock CRUD, or a bypass around the authorization/RLS layers.

---

# 2. Inventory Domain Objectives

The implementation must satisfy six simultaneous objectives:

1. **Correctness:** available quantity is derived from accepted postings and invariant-preserving transactions.
2. **Auditability:** every material movement has attributable cause, actor/device context, timestamps, references and durable state.
3. **Concurrency safety:** concurrent sales, receipts, transfers and adjustments cannot create impossible stock states.
4. **Valuation integrity:** cost information is deterministic and policy-controlled.
5. **Offline compatibility:** legitimate offline commands can be accepted within an explicitly bounded authority model without becoming a second stock authority.
6. **Operational performance:** routine POS reads/writes remain fast on modest infrastructure without sacrificing correctness.

---

# 3. Inventory Vocabulary

## 3.1 SKU

The SKU is the core commercially identifiable inventory item established by Phase 8.

Inventory MUST reference a stable SKU identifier, not a mutable product name or barcode string.

## 3.2 Stock item

A stock item is a quantity-bearing inventory identity at a location under a SKU, potentially further segmented by lot/batch, serial number, condition, owner, or other controlled dimensions.

Conceptually:

```text
SKU
 + ORGANIZATION
 + LOCATION
 + [LOT/BATCH]
 + [SERIAL]
 + [CONDITION]
 + [OTHER CONTROLLED DIMENSIONS]
 = INVENTORY STOCK IDENTITY
```

The exact dimensions must remain intentionally minimal. Adding arbitrary dimensions makes every query, uniqueness rule and balance calculation more expensive.

## 3.3 Location

A physical or logical stock-holding/selling point. The domain model explicitly distinguishes warehouse and selling location even when a micro-shop initially uses one room for both. fileciteturn11file0L75-L89

## 3.4 Warehouse

A controlled stock location capable of receiving, storing and dispatching inventory.

## 3.5 Inventory lot/batch

A traceability grouping that identifies a quantity sharing a source batch/lot identity. GS1's traceability model recognizes class-level, batch/lot-level and serialized identification; batch/lot identification is specifically valuable where recall and quality events are tied to a lot. citeturn926956search61

## 3.6 Serial number

An instance-level identifier for uniquely serialized stock. Serial control is materially more expensive than batch control and MUST only be enabled where the product/business model requires it.

## 3.7 Inventory ledger

An append-oriented sequence of accepted inventory postings. It is the authoritative explanation of inventory movement.

## 3.8 Balance projection

A derived representation of current quantity, reserved quantity, available quantity and optionally cost aggregates. It is a performance structure, not the historical source of truth.

## 3.9 Inventory transaction

A domain-level business event grouping one or more postings that must succeed or fail atomically.

Examples:

- receive goods;
- finalize sale consumption;
- return goods;
- transfer stock;
- perform stock adjustment;
- post stock count variance;
- write off damaged/expired goods;
- create opening balance.

---

# 4. Non-Negotiable Inventory Invariants

The following are implementation invariants, not suggestions.

## 4.1 Tenant invariant

A posting, balance, lot, serial, reservation and movement MUST belong to exactly one organization.

A user from tenant A MUST NEVER be able to create, read, mutate or infer tenant B inventory through any direct or indirect route.

## 4.2 Location invariant

Every inventory movement MUST have valid source and/or destination location semantics appropriate to its movement type.

Examples:

```text
RECEIPT
source = external supplier / nullable outside system
 destination = warehouse

SALE
source = selling inventory location
 destination = none

TRANSFER
source = warehouse A
 destination = warehouse B

RETURN
source = customer return intake
 destination = approved stock location
```

## 4.3 Non-negative availability invariant

For inventory types that cannot go negative, the final committed available quantity MUST NOT become negative.

Negative stock may only exist where a domain rule explicitly enables it. Such enablement MUST be configured deliberately, audited and exposed to reporting; it MUST NOT occur because a race condition won.

## 4.4 Reservation invariant

```text
available = on_hand - reserved
```

where the exact definition of reservable quantity is domain-specific.

The system MUST NOT allow:

```text
reserved > on_hand
```

unless an explicit backorder policy exists and models the distinction between reservation and physical ownership.

## 4.5 Unit invariant

All ledger quantities MUST resolve to a canonical base unit for the SKU.

A conversion factor MUST be immutable for a posted historical movement. Changing a product's current conversion cannot retroactively change historical ledger meaning.

## 4.6 Double-entry-like movement invariant

For an internal transfer:

```text
source decrement = destination increment
```

within the same atomic transaction.

The transfer MUST NOT become partially visible as “sent but not received” merely because a database transaction or network request failed.

If the business later requires in-transit inventory, that must be represented as an explicit location/state rather than as an accidental half-commit.

## 4.7 Ledger immutability invariant

Posted inventory ledger entries MUST NOT be silently edited or deleted.

Corrections are represented by compensating postings.

## 4.8 Historical reference invariant

A historical posting must remain interpretable even if:

- a SKU is discontinued;
- a barcode changes;
- a current product name changes;
- current pricing changes;
- current unit conversion changes;
- a branch closes.

Historical postings therefore MUST contain or reference the immutable identifiers and snapshots needed for interpretation.

## 4.9 Idempotency invariant

The same client command, provider observation or integration event MUST NOT create duplicate inventory effects.

## 4.10 Authorization invariant

An inventory mutation requires:

```text
authenticated principal
+
active session/device
+
active organization membership
+
required permission
+
valid organizational/branch/location scope
+
valid target resource
+
valid target state
+
valid business conditions
=
ALLOW
```

Phase 6 authorization remains the decision layer; inventory code MUST not reimplement a second contradictory authorization system.

---

# 5. Inventory State Model

## 5.1 SKU inventory lifecycle

```text
NOT_TRACKED
    |
    v
TRACKED
    |
    +----> DISCONTINUED
    |
    v
ARCHIVED
```

A discontinued SKU can retain stock for legitimate sell-through, returns or historical reporting.

## 5.2 Lot lifecycle

```text
RECEIVED
   |
   v
AVAILABLE
   |
   +--> QUARANTINED
   |
   +--> EXPIRED
   |
   +--> RECALLED
   |
   +--> DEPLETED
```

A lot may not become available solely because quantity exists. Regulatory and quality workflows can require explicit release.

## 5.3 Stock reservation lifecycle

```text
REQUESTED
   |
   v
RESERVED
   |
   +--> RELEASED
   |
   +--> CONSUMED
   |
   +--> EXPIRED
   |
   +--> CANCELLED
```

Reservations MUST expire deterministically and MUST NOT silently survive indefinitely.

---

# 6. Ledger Architecture

The recommended model is:

```text
inventory_transactions
        |
        +--> inventory_postings
        |
        +--> inventory_transaction_lines
        |
        +--> audit evidence
        |
        +--> outbox event
        |
        v
inventory_balance_projection
        |
        +--> on_hand
        +--> reserved
        +--> available
        +--> cost aggregates
```

## 6.1 Why transaction + posting rather than one row per movement

A transfer naturally has two sides. A return may have a stock posting and cost correction. A stock count may produce many SKU-level postings under one operator action.

A transaction envelope provides a durable causal unit; postings provide the granular movement.

## 6.2 Recommended conceptual entities

```text
inventory_transactions
inventory_postings
inventory_balances
inventory_lots
inventory_serials
inventory_reservations
inventory_counts
inventory_count_lines
inventory_adjustment_reasons
inventory_cost_layers / inventory_cost_state
inventory_idempotency_keys
```

Exact physical table names MUST reconcile with Phase 5 rather than duplicating already-frozen tables.

---

# 7. Ledger Posting Model

Each posting SHOULD contain:

```text
posting_id
transaction_id
organization_id
branch_id
location_id
sku_id
lot_id                nullable
serial_id             nullable
posting_type
quantity_base_units
signed_quantity
unit_cost              nullable/required by policy
currency               where cost is monetary
source_reference
created_at
created_by_user_id
created_by_device_id
correlation_id
causation_id
metadata_snapshot
```

The exact monetary representation MUST follow the existing financial model. Inventory quantity MUST NOT use floating-point arithmetic.

## 7.1 Signed quantities

A useful internal convention is:

```text
RECEIPT      +Q
RETURN       +Q
TRANSFER IN  +Q
ADJUST UP    +Q
SALE         -Q
TRANSFER OUT -Q
WRITE OFF    -Q
ADJUST DOWN  -Q
```

The sign convention MUST be centralized. Individual services MUST NOT invent their own interpretation.

---

# 8. Inventory Transaction Types

Minimum transaction types:

```text
OPENING_BALANCE
RECEIPT
SALE_CONSUMPTION
CUSTOMER_RETURN
TRANSFER
ADJUSTMENT
STOCK_COUNT
WRITE_OFF
RECALL
QUARANTINE
RELEASE
RESERVATION_CREATE
RESERVATION_RELEASE
RESERVATION_CONSUME
```

Not every business needs every type in the first deployment, but the command model must preserve room for controlled expansion.

---

# 9. Receiving Goods

Receiving converts an external procurement commitment into internally authoritative stock.

The command should follow:

```text
CreateReceipt
    |
    v
authorize
    |
    v
validate supplier/reference
    |
    v
validate SKUs + units + quantities
    |
    v
validate lot/expiry/serial requirements
    |
    v
begin transaction
    |
    +--> lock affected inventory identities
    +--> validate receiving state
    +--> create receipt record
    +--> create ledger postings
    +--> update balance projection
    +--> persist audit
    +--> enqueue outbox event
    |
    v
commit
```

External supplier systems are evidence sources, not authorities over Sitolo stock state.

## 9.1 Partial receipts

Partial receipt is an explicit domain operation.

Never infer partial receipt by mutating a purchase order's original line quantity without preserving received history.

## 9.2 Duplicate receipt protection

Use a merchant/reference idempotency boundary such as:

```text
organization_id + supplier_id + external_reference
```

only where the business contract guarantees uniqueness.

If external references are not globally reliable, use explicit Sitolo idempotency keys in addition to provider/reference correlation.

---

# 10. Sales Consumption

Sales are one of the highest-risk inventory mutation paths because concurrent POS terminals can target the same stock.

The transaction should be conceptually:

```text
authorize sale
    |
    v
reread authoritative sale state
    |
    v
validate sale state transition
    |
    v
resolve SKU / location / lot policy
    |
    v
lock affected stock rows in deterministic order
    |
    v
verify sufficient available quantity
    |
    v
post consumption
    |
    v
update balance projection
    |
    v
persist financial linkage + audit + outbox
    |
    v
COMMIT
```

The existing database design requires one business invariant per transaction and targeted locking for inventory-sensitive operations; the implementation must follow that rule rather than using a read-then-write race. fileciteturn10file2L340-L386

PostgreSQL documents `SELECT FOR UPDATE` and related locking primitives as appropriate mechanisms for protecting rows against conflicting concurrent updates. citeturn291630search2

## 10.1 Deterministic lock order

When a sale contains multiple SKUs, lock inventory identities in stable order:

```text
organization_id
branch/location_id
sku_id
lot_id
serial_id
```

This reduces deadlock probability.

## 10.2 Do not lock by arbitrary client order

A malicious client must not be able to influence an inconsistent lock ordering that increases deadlock opportunities.

The repository layer MUST normalize and sort target identities before locking.

---

# 11. Stock Transfers

Transfers MUST be atomic unless the explicit business process includes an in-transit state.

Preferred simple model:

```text
WAREHOUSE A
   -10
    |
    | same DB transaction
    v
WAREHOUSE B
   +10
```

The transaction must verify that both locations belong to the same organization and that the actor has authority over both endpoints.

A user with Branch A scope MUST NOT be able to transfer stock into or out of Branch B simply by submitting its identifier.

The existing authorization model explicitly treats branch/resource scope as part of the authorization tuple. fileciteturn10file1L160-L208

## 11.1 Cross-branch transfer

Cross-branch movement requires explicit permission and appropriate scope.

The UI does not grant authority.

The API MUST resolve both locations from authoritative state before performing the transfer.

---

# 12. Stock Adjustments

An adjustment is not a generic “set quantity” command.

Preferred:

```text
AdjustStock
  reason
  quantity_delta
  location
  SKU
  optional lot/serial
  evidence / note
```

Avoid:

```text
SetStock(quantity = 431)
```

The delta model preserves causality and prevents clients from silently overwriting concurrent changes.

## 12.1 Adjustment reasons

At minimum:

```text
FOUND_STOCK
COUNT_CORRECTION
DAMAGE
EXPIRY
THEFT_SHRINKAGE
DATA_CORRECTION
RECEIVING_ERROR
TRANSFER_ERROR
OTHER_CONTROLLED_REASON
```

Every reason SHOULD map to policy, required permissions, approval threshold and reporting classification.

## 12.2 High-risk adjustments

Large-value or high-quantity adjustments should require:

```text
permission
+
threshold evaluation
+
optional second-party approval
+
audit evidence
```

The exact thresholds belong in policy/configuration, not hard-coded route logic.

---

# 13. Stock Counts

Stock counts are observations of physical reality, not direct edits to the ledger.

Correct workflow:

```text
COUNT SESSION
     |
     v
capture observed quantities
     |
     v
freeze/define counting scope
     |
     v
compare against authoritative on-hand
     |
     v
calculate variance
     |
     v
authorize variance posting
     |
     v
create compensating ledger postings
```

A count must record:

- who counted;
- where;
- when;
- which SKUs were included;
- observed quantities;
- unit used;
- optional lot/serial data;
- system quantity used as comparison baseline;
- resulting variance;
- approval if required.

## 13.1 Count concurrency

If sales continue during counting, the implementation MUST define whether:

1. the count captures a timestamped snapshot and calculates later; or
2. the affected location is operationally frozen; or
3. count windows use a controlled movement boundary.

Do not pretend a count is exact without defining how concurrent movements are handled.

---

# 14. Returns

Returns can increase stock, but not every return should automatically return to saleable stock.

Possible dispositions:

```text
SALEABLE
QUARANTINE
DAMAGED
EXPIRED
RECALL
SUPPLIER_RETURN
SCRAP
```

For pharmacies and other regulated domains, return disposition may have regulatory constraints. The product specification explicitly treats pharmacy as a controlled extension with batch/lot, expiry, recall, storage state and authorized-staff controls. fileciteturn11file1L240-L258

---

# 15. Lots, Expiry and FEFO

Batch/lot control SHOULD be enabled per SKU or business policy rather than globally forcing every cheap retail item into lot accounting.

For lot-controlled goods, each receipt creates or updates a lot identity with:

```text
SKU
lot/batch number
supplier/source
received date
manufacture date where known
expiry date where applicable
quantity received
quantity remaining
status
```

For expiry-aware sale policies:

```text
eligible lots
   -> exclude expired
   -> exclude quarantined
   -> exclude recalled
   -> choose FEFO ordering where policy requires
   -> reserve/consume atomically
```

The selection algorithm MUST run against authoritative current state, not a client-provided “recommended lot”.

GS1's traceability framework specifically identifies batch/lot identification as useful for recall and tracing quantities through the supply chain. citeturn926956search61

---

# 16. Serialized Inventory

Serialized inventory is only appropriate when individual units matter.

Examples may include:

- electronics;
- high-value equipment;
- specific medical devices;
- regulated items with unique serial controls.

A serial identity MUST be unique within its applicable organization/domain.

A serialized unit cannot be sold twice.

The invariant should be enforced at both application and database layers.

---

# 17. Unit Conversion

Phase 8 defines SKU unit semantics. Inventory MUST store ledger quantities in a canonical base unit.

Example:

```text
1 carton = 24 pieces

receipt: 10 cartons
ledger: 240 pieces
```

If a sale consumes 3 pieces:

```text
on_hand = 240 - 3 = 237 pieces
```

The historical receipt retains the conversion used at posting time.

Changing today's conversion from 24 to 20 MUST NOT reinterpret historical 240-piece receipts.

## 17.1 Conversion safety

Reject:

- zero conversion factors;
- negative conversion factors;
- overflow/underflow;
- incompatible units;
- ambiguous unit dimensions;
- conversion cycles that cannot resolve to one canonical base unit.

---

# 18. Costing Model

Inventory valuation is separate from selling price.

The implementation MUST explicitly choose and version cost policy.

Potential formulas include:

```text
FIFO
WEIGHTED_AVERAGE
SPECIFIC_IDENTIFICATION
```

IAS 2 permits specific identification for items that are not ordinarily interchangeable and FIFO or weighted average for ordinarily interchangeable inventories. Sitolo must treat this as an accounting policy decision rather than silently choosing an algorithm in code. citeturn291630search1turn291630search56

## 18.1 Recommended initial strategy

For general small retail:

```text
Operational stock ledger: authoritative quantity events
Costing: weighted-average by SKU + valuation scope
```

A FIFO layer model may be implemented where the business/regulatory/reporting requirement justifies it.

Do not implement three costing systems simultaneously without a concrete reporting requirement.

## 18.2 Cost state

A weighted-average model can maintain:

```text
total_quantity_base
 total_cost
 average_unit_cost
```

A receipt changes the cost state:

```text
new_qty = old_qty + received_qty
new_total_cost = old_total_cost + received_cost
new_average = new_total_cost / new_qty
```

The arithmetic MUST use exact monetary representation; floating-point is prohibited for financial calculations.

## 18.3 Sale COGS snapshot

When a sale consumes stock, the resulting cost-of-goods amount MUST be captured according to the selected costing policy at the time of authoritative posting.

Historical COGS must not be recomputed dynamically from today's average cost.

---

# 19. Negative Stock Policy

There must be an explicit organization-level or SKU-policy decision:

```text
ALLOW_NEGATIVE_STOCK = false (recommended default)
```

If temporarily allowed:

- the negative condition must be explicit;
- reports must identify negative inventory;
- later receipts must not silently erase the underlying event history;
- permissions and audit MUST apply;
- valuation behavior MUST be defined.

The recommended default for regulated and financial integrity is to deny negative stock.

---

# 20. Reservation Architecture

Reservations are useful for:

- pending orders;
- customer holds;
- wholesale allocations;
- synchronized multi-device workflows.

Reservations MUST NOT double-count physical stock.

Recommended model:

```text
on_hand
reserved
available = on_hand - reserved
```

Reservation changes are themselves state transitions and need concurrency protection.

A reservation create operation must atomically verify available quantity before increasing reserved quantity.

---

# 21. PostgreSQL Transaction Semantics

Routine transactions should use the project's default `READ COMMITTED` semantics unless a stronger isolation requirement is documented. PostgreSQL documents `READ COMMITTED` as the default and `SERIALIZABLE` as providing serializable behavior with possible serialization failures that application code must handle. citeturn291630search0turn291630search8

## 21.1 When to use row locks

Use row-level locking when the correctness condition is:

```text
read current authoritative state
+
change that same state
```

Examples:

- stock decrement;
- reservation creation;
- stock count finalization;
- lot consumption;
- serialized-unit consumption.

PostgreSQL specifically documents explicit locking as necessary where non-serializable writes must preserve current row validity. citeturn291630search2

## 21.2 When not to use serializable everywhere

`SERIALIZABLE` is not a substitute for understanding domain locking. Universal serializable transactions can increase retry rates and operational complexity.

Use it when the invariant genuinely depends on a serializable read/write pattern and targeted locking is inadequate.

## 21.3 `SKIP LOCKED`

`SKIP LOCKED` is appropriate for queue-like worker consumption, not for authoritative stock deduction. PostgreSQL documents that skipped locked rows provide an inconsistent view and are suitable for queue-style multi-consumer workloads. citeturn926956search60turn291630search5

Never use `SKIP LOCKED` to make stock availability checks “fast.” That could cause incorrect inventory decisions.

## 21.4 Advisory locks

Advisory locks may be appropriate for coarse-grained operational serialization, but they MUST NOT become a hidden second transaction model.

PostgreSQL supports transaction-level advisory locks specifically for application-defined resources. citeturn926956search1

Preferred hierarchy:

```text
row-level domain locks
        >
short transaction-scoped advisory lock when justified
        >
application mutex
```

An application mutex cannot provide correctness across multiple backend instances.

---

# 22. Exact Stock-Deduction SQL Pattern

Conceptual implementation:

```sql
BEGIN;

SELECT id, on_hand, reserved
FROM inventory_balances
WHERE organization_id = $1
  AND location_id = $2
  AND sku_id = $3
FOR UPDATE;

-- validate available = on_hand - reserved
-- validate sufficient quantity

INSERT INTO inventory_postings (..., signed_quantity, ...)
VALUES (..., -$4, ...);

UPDATE inventory_balances
SET on_hand = on_hand - $4,
    available = on_hand - $4 - reserved,
    version = version + 1
WHERE id = $5;

INSERT INTO outbox_events (...);
INSERT INTO audit_events (...);

COMMIT;
```

The actual implementation SHOULD avoid duplicate mutable formulas by centralizing balance calculation and by ensuring the projection and ledger update remain in the same transaction.

---

# 23. Ledger + Projection Consistency

The strongest model is:

```text
ledger = authority
projection = derived acceleration
```

The transaction that posts a movement SHOULD update the projection in the same PostgreSQL transaction.

This produces:

```text
COMMIT
 |
 +--> ledger posting exists
 +--> projection reflects posting
```

If the projection update fails, the transaction rolls back.

Do not accept:

```text
ledger committed
projection updated later
```

for the critical POS path unless the design explicitly tolerates temporary read-model lag and the API exposes that semantic.

---

# 24. Projection Rebuild

The projection MUST be rebuildable from the ledger.

Rebuild process:

```text
freeze/restrict affected writes if required
       |
       v
create isolated rebuild target
       |
       v
scan ledger in deterministic order
       |
       v
recalculate balances
       |
       v
reconcile totals/checksums
       |
       v
promote projection
```

A rebuild MUST NOT mutate historical ledger records.

For large tables, rebuilds should be batched and bounded. PostgreSQL warns that large updates can increase table bloat, replica lag and lock contention; batch design is therefore an operational requirement rather than a cosmetic optimization. citeturn291630search5

---

# 25. Reconciliation Invariants

For each inventory identity:

```text
computed_on_hand
    = opening_balance
    + Σ(receipts)
    + Σ(returns)
    + Σ(transfers_in)
    + Σ(adjustments_up)
    - Σ(sales)
    - Σ(transfers_out)
    - Σ(write_offs)
    + other approved signed postings
```

The exact equation depends on supported posting types, but there must be one canonical interpretation.

A reconciliation job MUST identify:

```text
ledger_total != projection_total
```

without silently repairing it.

Repair requires:

1. incident classification;
2. evidence capture;
3. root-cause determination;
4. controlled rebuild or compensating correction;
5. post-repair verification.

---

# 26. Inventory API Surface

Preferred command-oriented routes:

```text
POST /v1/inventory/receipts
POST /v1/inventory/transfers
POST /v1/inventory/adjustments
POST /v1/inventory/counts
POST /v1/inventory/counts/{count_id}/finalize
POST /v1/inventory/reservations
POST /v1/inventory/reservations/{id}/release
POST /v1/inventory/write-offs
POST /v1/inventory/returns
GET  /v1/inventory/balances
GET  /v1/inventory/ledger
GET  /v1/inventory/lots
GET  /v1/inventory/alerts
```

No route should exist merely because an inventory table exists.

The API contract already explicitly rejects mutation paths such as `PATCH /inventory_balances` in favor of business commands. fileciteturn11file2L383-L655

---

# 27. Authorization Enforcement Matrix

| Operation | Minimum policy domain |
|---|---|
| View stock | inventory.read + scope |
| Receive goods | inventory.receive + procurement scope |
| Transfer stock | inventory.transfer + source + destination scope |
| Adjust stock | inventory.adjust + reason policy |
| Large adjustment | inventory.adjust + threshold approval |
| Count stock | inventory.count + scope |
| Finalize variance | inventory.count.finalize + approval where required |
| Write off | inventory.writeoff + approval policy |
| View cost | inventory.cost.read + financial scope |
| Export inventory | inventory.export + export policy |
| Manage lot quarantine | inventory.quarantine + regulated scope |
| Manage serialized stock | inventory.serial.manage |

The exact permission identifiers must be drawn from the Phase 6 permission registry, not duplicated here.

---

# 28. Property-Level Authorization

Different roles may legitimately see different inventory fields.

Example:

```text
Cashier:
  SKU
  sellable quantity
  retail-facing availability

Manager:
  stock quantity
  low-stock alerts
  adjustment history

Finance:
  quantity
  valuation
  cost
  COGS

Auditor:
  quantity
  valuation
  actor
  audit history
```

A route returning an inventory object MUST NOT simply serialize every database field because the caller is authorized to read the object.

The API contract already requires property-level authorization and restricted projections for sensitive data. fileciteturn10file9L979-L1012

---

# 29. Tenant and RLS Enforcement

The inventory repository MUST require a trusted organization context.

Conceptually:

```text
request
  |
  v
identity/session/device
  |
  v
membership + scope
  |
  v
authorization decision
  |
  v
trusted DB transaction context
  |
  v
repository query
  |
  v
PostgreSQL RLS
```

RLS is defense in depth, not the application authorization engine.

A query returning zero rows MUST NOT automatically be interpreted as either “not found” or “not authorized” without considering the API error contract.

The system should avoid distinct error timing or response behavior that becomes a cross-tenant existence oracle.

---

# 30. Offline Inventory Model

Offline operation is a major Sitolo requirement, but offline capability cannot turn SQLite into authoritative inventory.

The client may store:

```text
last-known inventory snapshot
pending commands
local optimistic state
server checkpoints
```

The server remains authoritative for accepted stock state.

## 30.1 Offline sale authority

An offline sale can only consume stock under the explicit offline policy defined by the sync protocol.

Possible model:

```text
server-authorized device
      |
      v
bounded offline capability
      |
      v
signed/identified local command
      |
      v
later server validation
      |
      +--> accepted
      +--> conflict
      +--> rejected
```

Do not allow unlimited offline stock deduction merely because the device has an old stock snapshot.

## 30.2 Offline conflict

A conflict is not “last write wins.” Inventory conflicts need domain semantics:

```text
device thinks stock = 8
server stock = 2
command consumes 4
```

The server must evaluate whether 4 units are actually available under its authoritative state.

---

# 31. Idempotency

Every retryable inventory mutation MUST have a stable idempotency boundary.

For example:

```text
organization_id
+
device_id
+
client_command_id
```

The server stores:

```text
command_id
request_hash
result_reference
status
created_at
```

Rules:

- same key + same semantic request → return/replay the prior result safely;
- same key + materially different request → reject;
- unknown outcome → do not invent a second stock effect;
- idempotency record survives long enough to cover expected retries.

---

# 32. Inventory and Payments

A successful payment observation does not automatically mean inventory consumption is valid.

The domain transaction should establish:

```text
sale finalized
+
financial state valid
+
stock consumption valid
```

External provider calls MUST remain outside the inventory database transaction.

Provider failure should leave the internal inventory and sale state in a well-defined pending/exception state rather than corrupting stock to match an external system.

This follows the existing architecture's separation between authoritative internal state and external integration evidence. fileciteturn10file3L435-L457

---

# 33. Inventory and MRA EIS

Tax/EIS integration must not become a hidden inventory authority.

A tax submission may contain sale/item quantities as evidence, but MRA EIS responses do not rewrite inventory quantities.

The inventory ledger is driven by the local authoritative sales/inventory transaction.

EIS rejection creates tax/integration state and an operational workflow, not a retroactive stock mutation.

---

# 34. Pharmacy Inventory Extension

Pharmacy inventory needs additional controls:

```text
SKU
 + batch/lot
 + expiry
 + registration metadata
 + storage status
 + recall status
 + authorized dispensing policy
```

Operations may include:

- quarantine;
- release;
- recall;
- expiry write-off;
- restricted medicine transfer;
- pharmacist approval;
- prescription-linked dispensing.

These are controlled extensions and MUST NOT weaken generic inventory invariants.

The existing product spec explicitly describes pharmacy as a first-class regulated extension rather than a simple category flag. fileciteturn11file1L240-L258

---

# 35. Agro-Dealer Inventory Extension

Agro-dealer stock may require:

- formulation metadata;
- lot/batch;
- expiry/recommended use date;
- controlled storage;
- traceability;
- specialized movement permissions.

The generic ledger remains the same; vertical modules contribute additional invariants and metadata.

---

# 36. Inventory Search and Read Models

Search must use bounded queries.

Examples:

```text
stock by SKU
stock by branch
stock by location
low-stock items
expiring lots
recent movements
inventory valuation
```

Avoid unbounded ledger scans for interactive POS requests.

Recommended architecture:

```text
POSTGRESQL LEDGER
       |
       +--> balance projection
       +--> lot projection
       +--> low-stock projection
       +--> reporting read model
```

Read models are rebuildable and MUST carry source checkpoints/watermarks where asynchronous.

---

# 37. Inventory Alerts

Alerts SHOULD be derived from authoritative state, such as:

```text
low stock
negative stock
expiring lot
expired stock
unexpected adjustment rate
high shrinkage
repeated stock-count variance
unusual transfer pattern
rapid refund/return pattern
```

Alerts are signals, not accusations.

Security/fraud alerts belong in the existing observability/security event model.

---

# 38. Performance Model

The normal POS inventory path should be optimized around:

```text
one command
small bounded lock set
short DB transaction
few indexed reads
few indexed writes
no external network calls
```

## 38.1 Avoid

- N+1 inventory queries;
- full-ledger scans for current balance;
- locks held across network calls;
- large serialized payloads;
- unbounded count finalization;
- synchronous report generation during POS sale finalization.

## 38.2 Index design

Critical indexes SHOULD cover:

```text
organization_id + location_id + sku_id
organization_id + sku_id + lot_id
organization_id + transaction_id
organization_id + created_at
organization_id + source_reference
```

Exact index set must come from actual query plans and workload evidence.

---

# 39. Worker Model

Workers handle non-critical asynchronous tasks such as:

- low-stock notifications;
- expiry notifications;
- valuation snapshots;
- reconciliation jobs;
- projection rebuilds;
- audit exports;
- inventory analytics.

Worker commands MUST execute with explicit service identity and tenant context.

A worker MUST NOT obtain global authority simply because it is internal.

Long-running jobs should use bounded batches and checkpointing.

For queue-style worker consumption, `SKIP LOCKED` can be used where appropriate, consistent with PostgreSQL's documented queue semantics. citeturn926956search60

---

# 40. Failure Handling

Inventory errors should classify into:

```text
AUTHORIZATION_DENIED
RESOURCE_NOT_FOUND_OR_NOT_VISIBLE
INVALID_STATE
INSUFFICIENT_STOCK
UNIT_CONVERSION_INVALID
LOT_INVALID
SERIAL_ALREADY_CONSUMED
DUPLICATE_COMMAND
CONCURRENT_MODIFICATION
CONSTRAINT_VIOLATION
DEPENDENCY_UNAVAILABLE
UNKNOWN_OUTCOME
```

Do not collapse an `UNKNOWN_OUTCOME` into “stock not available.”

If the server cannot determine whether a mutation committed, the client must retry using the same idempotency key rather than generating a new command.

---

# 41. Concurrency Test Matrix

Minimum tests:

```text
2 concurrent sales consume same final unit
2 concurrent reservations consume same stock
sale + transfer on same stock
sale + stock adjustment
receipt + sale
count finalization + sale
return + sale
lot consumption + expiry update
serial sale + duplicate serial sale
cross-branch transfer concurrency
membership revocation + inventory mutation
```

Expected outcomes must be deterministic according to the documented transaction semantics.

PostgreSQL provides MVCC, row locking, isolation and deadlock detection primitives for these conditions; the application must actually test them against real PostgreSQL. citeturn291630search8turn291630search3

---

# 42. Property-Based Test Properties

Examples:

```text
ledger_replay_determinism
projection_equals_ledger
transfer_conservation
no_duplicate_idempotency_effect
unit_conversion_round_trip
non_negative_when_disallowed
reservation_never_exceeds_policy
serial_uniqueness
historical_snapshot_stability
```

## 42.1 Transfer conservation

For every valid internal transfer:

```text
sum(source change) + sum(destination change) = 0
```

for the same SKU/unit identity, excluding explicit in-transit models.

## 42.2 Projection determinism

Replay the same ordered ledger into two fresh projections.

Expected:

```text
projection_A == projection_B
```

## 42.3 Permission monotonicity

Removing inventory permission must never increase the actor's effective inventory authority.

---

# 43. Fuzzing Targets

Fuzz:

- quantity parser;
- decimal/exact-unit conversion;
- lot/serial identifiers;
- command payloads;
- idempotency request hashes;
- stock-count import records;
- CSV import parsing;
- pagination parameters;
- inventory search queries;
- state-machine transitions.

Targets must include malformed UTF-8 handling where parser boundaries permit it, oversized strings, huge quantities, tiny fractions where supported, integer overflow boundaries, and nested payload abuse.

---

# 44. Mutation Testing

The security-test framework should mutate inventory policy code to verify that tests actually catch:

```text
quantity sign inversion
lock omission
authorization bypass
tenant predicate removal
RLS policy removal
idempotency lookup removal
lot expiry predicate removal
serial uniqueness check removal
cost snapshot recalculation
projection update omission
```

A mutation surviving in a release-blocking invariant area indicates inadequate tests, not merely an interesting test-coverage metric.

---

# 45. Database Constraint Tests

Real PostgreSQL integration tests MUST verify:

- tenant-scoped foreign keys;
- non-null identity columns;
- valid quantity constraints;
- unique serial numbers where required;
- lot identity uniqueness;
- idempotency uniqueness;
- valid status transitions where enforced in SQL;
- no negative balance if DB-enforced;
- cross-tenant references rejected;
- RLS isolation;
- least-privilege runtime role cannot bypass security policies.

The existing database contract explicitly requires real PostgreSQL tests for RLS and related behavior. fileciteturn10file2L302-L342

---

# 46. Audit Requirements

Every material inventory mutation MUST create durable audit evidence sufficient to answer:

```text
WHO
WHAT
WHEN
WHERE
WHICH ORGANIZATION
WHICH DEVICE/SESSION
WHICH SKU
WHICH LOCATION
WHICH QUANTITY
WHICH REASON
WHICH PRIOR STATE
WHICH RESULT
WHICH CORRELATION/COMMAND ID
```

Authorization denials for high-risk inventory actions SHOULD be logged according to the existing security logging policy.

OWASP specifically recommends logging authorization failures, higher-risk administrative actions and security-relevant business behavior while avoiding excessive sensitive information. citeturn926956search0turn926956search2

---

# 47. Inventory Telemetry

Metrics should include bounded dimensions such as:

```text
inventory_command_total{operation,outcome}
inventory_command_latency_ms{operation}
inventory_concurrency_conflict_total{operation}
inventory_insufficient_stock_total
inventory_projection_lag
inventory_reconciliation_mismatch_total
inventory_ledger_posting_total{posting_type}
inventory_worker_queue_depth
```

Do not label metrics with raw SKU IDs, tenant IDs or arbitrary user input.

Tracing SHOULD connect:

```text
request_id
correlation_id
command_id
inventory_transaction_id
sale_id / receipt_id / transfer_id
```

without logging sensitive payloads.

---

# 48. Inventory Reconciliation Job

A recurring reconciliation process SHOULD compare:

```text
ledger-derived quantity
vs
balance projection
```

and, where appropriate:

```text
physical count history
vs
system history
```

The process MUST produce a durable reconciliation result.

A mismatch should transition into:

```text
DETECTED
  -> CLASSIFIED
  -> INVESTIGATED
  -> REPAIRED OR ACCEPTED AS KNOWN_EXCEPTION
  -> VERIFIED
```

Never silently overwrite a projection to make a mismatch disappear.

---

# 49. Inventory Repair

Repair tools are security-sensitive administrative capabilities.

There must be no generic endpoint such as:

```text
POST /admin/fix-stock
```

Instead use controlled operations:

```text
RebuildInventoryProjection
CreateCorrectiveInventoryPosting
ReconcileInventoryIdentity
```

Each requires elevated authorization, strong authentication, audit and preferably approval for material financial impact.

---

# 50. Import Architecture

Bulk inventory imports are high-risk because a single file can mutate thousands of rows.

Pipeline:

```text
upload
  |
  v
malware/file validation
  |
  v
parse
  |
  v
schema validation
  |
  v
semantic validation
  |
  v
preview/errors
  |
  v
approval if required
  |
  v
bounded transactional batches
  |
  v
ledger postings
```

Never treat imported quantity as a trusted balance replacement.

Imported changes should become normal domain commands/postings.

---

# 51. Branch and Location Scope

A branch-level user may be authorized for:

```text
Branch A
  -> Warehouse A1
  -> Register A2
```

but not automatically:

```text
Branch B
```

Cross-branch access is evaluated from authoritative membership and scope data.

The existing domain model explicitly requires the organization to be the tenant boundary and branch to be an operational scope. fileciteturn11file0L39-L89

---

# 52. Warehouse vs Selling Location

Do not merge these concepts merely to reduce schema size.

A warehouse may permit:

```text
receive
store
transfer
count
adjust
```

A selling location may permit:

```text
sell
reserve
cash-session linkage
count
```

A branch may have one physical room serving both, but the domain still benefits from distinct capabilities.

---

# 53. Security Threat Model for Inventory

Major threats include:

| Threat | Control |
|---|---|
| Cross-tenant stock access | app authz + repository scope + RLS |
| Cross-branch mutation | scope enforcement |
| BOLA/IDOR | resource ownership resolution |
| Double-sale race | row locking + transaction |
| Duplicate receipt | idempotency |
| Duplicate offline command | command identity + server dedupe |
| Stock overwrite | delta/posting commands |
| Negative stock | atomic availability check |
| Lot bypass | server lot eligibility validation |
| Expired sale | authoritative expiry policy |
| Serial double-use | unique constraint + transaction |
| Cost tampering | authoritative costing state |
| Projection corruption | rebuild/reconciliation |
| Admin repair abuse | elevated policy + audit |
| Import abuse | bounded validation + approval |
| Worker privilege abuse | explicit worker identity |
| Enumeration | scoped queries + bounded projections |
| Timing/response leaks | consistent safe errors |

The project threat model already states that frontend hiding is not a control, JWT tenant IDs are not proof of current membership, and RLS is defense in depth rather than the complete authorization model. fileciteturn9file0L11-L49

---

# 54. API Enumeration Safety

Inventory search must not expose cross-tenant existence through:

- result counts;
- autocomplete suggestions;
- different timing for valid/invalid identifiers;
- error messages revealing hidden SKUs;
- bulk export behavior.

The existing API specification explicitly identifies search and export as separate authorization concerns. fileciteturn10file9L943-L975

---

# 55. Caching Rules

Cache only derived data.

Never cache authority as:

```text
sku_id -> may_modify = true
```

without incorporating authorization versioning and scope.

Inventory balance caches MUST be treated as read acceleration, not write authority.

Sensitive mutation paths should fall back to authoritative PostgreSQL state when cache confidence is insufficient.

---

# 56. Event and Outbox Semantics

A successful inventory transaction SHOULD write its outbox event in the same database transaction.

```text
BEGIN
  inventory transaction
  ledger postings
  projection
  audit
  outbox
COMMIT
```

Workers consume committed outbox events after commit.

This ensures:

```text
if inventory change exists
then event intent exists
```

subject to operational delivery semantics.

---

# 57. Event Payload Discipline

Outbox payloads should contain stable identifiers and sufficient metadata for downstream work but should not reproduce entire private inventory objects unnecessarily.

Example:

```json
{
  "event_type": "inventory.transaction.posted",
  "organization_id": "…",
  "inventory_transaction_id": "…",
  "operation": "TRANSFER",
  "correlation_id": "…",
  "schema_version": 1
}
```

Workers should reread authoritative state when they need additional details.

---

# 58. Transaction Boundary Rules

### Receipt

One receipt posting transaction per bounded command/batch.

### Sale consumption

Inventory consumption MUST be part of the sale finalization transaction when the business contract requires atomic finalization.

### Transfer

Source and destination changes MUST be one transaction unless an explicit in-transit model exists.

### Stock count

Observation capture may be separate from final posting; finalization is transactional.

### Projection rebuild

Not part of ordinary POS write transactions.

### External integration

Never hold inventory DB transactions across HTTP/network calls.

These rules align with the project's established explicit transaction boundary and “no external calls inside business transaction” constraints. fileciteturn10file2L380-L386

---

# 59. Rust Module Architecture

Recommended ownership:

```text
crates/
  inventory-domain/
    entities/
    value_objects/
    commands/
    events/
    policies/
    errors/

  inventory-application/
    commands/
    queries/
    ports/
    authorization/
    transaction_services/

  inventory-infrastructure/
    postgres/
    projections/
    repositories/
    migrations/
    workers/

  api/
    routes/inventory.rs
    dto/inventory.rs
```

## 59.1 Domain layer

Owns:

- posting semantics;
- quantity value objects;
- movement-type rules;
- unit conversion invariants;
- state transitions;
- pure costing calculations where appropriate.

Must not know Axum or SQLx.

## 59.2 Application layer

Owns:

- authorization invocation;
- transaction orchestration;
- repository calls;
- idempotency;
- audit/outbox coordination.

## 59.3 Infrastructure

Owns:

- SQLx queries;
- PostgreSQL transaction code;
- row locks;
- projection persistence;
- worker implementation.

This follows the domain/application/infrastructure separation already established in the project. fileciteturn10file2L380-L386

---

# 60. SQLx Guidelines

SQLx should be used for typed, explicit SQL rather than hiding critical inventory semantics behind an ORM abstraction.

Current SQLx releases support compile-time checked SQL and async PostgreSQL access; the exact version must follow the repository's locked dependency baseline. citeturn291630search4turn291630search11

Critical queries SHOULD be explicit enough for reviewers to see:

```text
tenant predicate
location predicate
lock behavior
ordering
bounds
selected columns
```

Avoid dynamic SQL where a static query is sufficient.

---

# 61. Testing Pyramid for Inventory

```text
                   E2E / DEVICE
                      /   \
                   API / SYNC
                  /         \
             REAL POSTGRES  INTEGRATIONS
                /     \
          CONCURRENCY   RLS
             /             \
        DOMAIN / PROPERTY TESTS
                 |
            PURE UNIT TESTS
```

Each layer answers a different question.

A unit test cannot prove PostgreSQL locking.

A controller test cannot prove a worker cannot bypass authorization.

A happy-path E2E test cannot prove idempotency under duplicate commands.

---

# 62. Required Security Tests

Minimum release-blocking inventory security tests:

```text
cross-tenant read denied
cross-tenant write denied
cross-branch read denied
cross-branch write denied
wrong-location mutation denied
cashier adjustment denied where policy forbids
unauthorized write-off denied
unauthorized cost read denied
expired lot sale denied
quarantined lot sale denied
recalled lot sale denied
serialized unit double-sale denied
duplicate receipt denied/idempotent
replayed transfer denied/idempotent
duplicate offline command denied/idempotent
negative stock race handled
cache poisoning cannot grant authority
projection mismatch detected
worker cross-tenant attempt denied
admin repair audited
```

The existing security harness explicitly requires cross-tenant, authorization, RLS, concurrency, worker and offline tests. fileciteturn9file4L616-L685

---

# 63. Concurrency Harness

Use a real PostgreSQL test environment.

Pattern:

```text
Seed stock = 1

Worker A                  Worker B
   |                         |
   | begin                   | begin
   | lock/read               | lock/read
   |                         | waits
   | consume 1               |
   | commit                  |
   |                         | wakes
   |                         | reread/validate
   |                         | insufficient
```

The expected invariant is:

```text
successful_consumptions <= initial_available + valid_receipts - valid_other_consumptions
```

No test may rely solely on sleeps to “create” a race.

Use synchronization barriers/channels to deliberately align transactions.

---

# 64. Deadlock Testing

Construct two transfers with opposite requested ordering.

The repository must normalize lock ordering so both resolve to the same order.

Where a database deadlock still occurs, application code should classify the error as retryable under the project's database retry policy rather than retrying every database error blindly.

PostgreSQL provides deadlock detection and transaction rollback semantics; applications must make retry behavior explicit. citeturn291630search8

---

# 65. Inventory Incident Runbook — Stock Mismatch

```text
1. Freeze destructive repair operations.
2. Identify affected organization/location/SKU.
3. Capture current projection state.
4. Query authoritative ledger postings.
5. Compare projection-derived and ledger-derived totals.
6. Determine whether the issue is:
   - code defect
   - migration defect
   - manual repair defect
   - concurrency defect
   - import defect
   - data corruption
7. Preserve evidence.
8. Rebuild isolated projection if appropriate.
9. Verify against ledger.
10. Apply approved corrective posting if ledger itself requires correction.
11. Re-enable affected workflows.
12. Add regression test.
13. Record incident and root cause.
```

---

# 66. Inventory Incident Runbook — Suspected Double Sale

```text
1. Identify sale IDs and command IDs.
2. Check idempotency records.
3. Inspect inventory transaction ordering.
4. Inspect PostgreSQL lock/transaction evidence where available.
5. Determine whether both sales actually committed.
6. Do not manually overwrite stock.
7. If a false second sale committed, apply the domain-approved correction workflow.
8. Reconcile financial and inventory state.
9. Add deterministic concurrency regression coverage.
```

The financial record and inventory record must not be “fixed” independently if the business event links them.

---

# 67. Inventory Incident Runbook — Expired/Quarantined Stock Sold

```text
1. Identify affected SKU/lot/serials.
2. Determine applicable policy version.
3. Confirm authoritative lot state at transaction time.
4. Identify authorization path.
5. Determine whether sale bypassed policy, used stale cache, or used incorrect data.
6. Preserve sale and inventory facts.
7. Apply regulated correction workflow where required.
8. Revoke or quarantine affected inventory.
9. Add policy regression tests.
```

Never delete the original sale merely to hide an operational mistake.

---

# 68. Inventory Migration Strategy

Schema changes MUST use the Phase 5 migration discipline.

For high-volume inventory tables:

```text
expand
  |
  v
backfill bounded batches
  |
  v
verify
  |
  v
switch reads/writes
  |
  v
contract
```

Do not add a NOT NULL column requiring a full-table rewrite during peak POS operations without evidence that the migration is safe.

---

# 69. Data Retention

Retain inventory evidence according to business, tax, regulatory and contractual requirements.

Do not delete historical ledger postings merely because the SKU is archived.

Deletion/retention controls must distinguish:

```text
operational projection
historical ledger
audit evidence
personal data
external integration payloads
```

---

# 70. Backup and Restore Requirements

A backup is not sufficient evidence.

Run restore tests that verify:

```text
restore database
  |
  v
apply/check migration state
  |
  v
verify ledger integrity
  |
  v
rebuild or validate projections
  |
  v
verify idempotency state
  |
  v
verify audit chain
```

The project's broader DR model requires actual restore evidence and integrity validation rather than assuming backups are usable. fileciteturn9file8L1167-L1191

---

# 71. CI Gates

Every inventory-affecting PR SHOULD trigger at least:

```text
format/lint
unit tests
domain property tests
API tests
real PostgreSQL tests
RLS tests
tenant isolation tests
concurrency tests
idempotency tests
migration tests
security tests
secret scan
dependency scan
```

Inventory changes affecting financial or tenant boundaries MUST be release-blocking when required controls do not run.

The existing CI contract requires mandatory checks to distinguish PASS, FAIL, NOT RUN, NOT APPLICABLE and EXPLICITLY WAIVED; a security check that did not execute is not a pass. fileciteturn9file1L143-L185

---

# 72. Inventory PR Checklist

Every inventory mutation PR must answer:

```text
[ ] What invariant changes?
[ ] What command/state transition changes?
[ ] What permission is required?
[ ] What tenant/branch/location scope applies?
[ ] What database rows are locked?
[ ] In what order are they locked?
[ ] What is the transaction boundary?
[ ] What happens on retry?
[ ] What happens on unknown outcome?
[ ] What happens offline?
[ ] What happens concurrently?
[ ] What audit evidence is created?
[ ] What outbox event is created?
[ ] What projection is updated?
[ ] Can projection be rebuilt?
[ ] What real PostgreSQL tests exist?
[ ] What negative security tests exist?
[ ] What migration is required?
[ ] What runbook changes?
```

---

# 73. Definition of Done

Phase 9 is NOT complete when inventory endpoints return plausible numbers.

It is complete when:

```text
[ ] ledger model implemented
[ ] balance projection implemented
[ ] SKU/base-unit invariant enforced
[ ] receipt workflow implemented
[ ] sale consumption integrated
[ ] transfer workflow implemented
[ ] adjustment workflow implemented
[ ] stock count workflow implemented
[ ] returns implemented
[ ] write-offs implemented
[ ] lot/expiry controls implemented where enabled
[ ] serial controls implemented where enabled
[ ] costing policy selected
[ ] historical cost snapshots preserved
[ ] idempotency implemented
[ ] offline command semantics implemented
[ ] authorization enforced
[ ] RLS verified
[ ] tenant isolation verified
[ ] branch/location scope verified
[ ] concurrency tests pass
[ ] projection reconciliation implemented
[ ] rebuild path tested
[ ] audit implemented
[ ] telemetry implemented
[ ] worker identity implemented
[ ] migration tested
[ ] restore scenario tested
[ ] release gates enforced
```

---

# 74. Recommended Implementation Sequence

```text
PR-001 inventory domain primitives
        |
PR-002 inventory ledger schema
        |
PR-003 inventory balance projection
        |
PR-004 receiving
        |
PR-005 sale consumption integration
        |
PR-006 transfers
        |
PR-007 adjustments
        |
PR-008 stock counts
        |
PR-009 returns/write-offs
        |
PR-010 lot/expiry controls
        |
PR-011 serialized inventory
        |
PR-012 costing
        |
PR-013 reservations
        |
PR-014 offline integration
        |
PR-015 reconciliation/rebuild
        |
PR-016 audit/telemetry
        |
PR-017 security/concurrency suite
        |
PR-018 migrations/operational runbooks
        |
PR-019 release gates
```

Each PR must remain independently reviewable and must not merge an unsafe dependency that will only become secure several PRs later.

---

# 75. Advantages

## 75.1 Strong historical correctness

An append-oriented ledger preserves why stock changed rather than only what the current balance happens to be.

## 75.2 Concurrency safety

Explicit transaction and lock semantics protect scarce stock under concurrent POS devices.

## 75.3 Better auditability

Every material movement can be attributed to a command, actor, device, location and business reason.

## 75.4 Offline compatibility

The ledger remains server-authoritative while the client can maintain operational continuity.

## 75.5 Vertical extensibility

Lots, expiry, recalls and serials can be added without replacing the core quantity model.

## 75.6 Rebuildability

A projection can be rebuilt because historical postings remain authoritative.

## 75.7 Better fraud control

Large adjustments, unusual shrinkage, duplicate commands and privileged repairs leave explicit evidence.

---

# 76. Disadvantages

## 76.1 More engineering complexity

A ledger and projection architecture is more complex than a single mutable stock column.

## 76.2 Higher write amplification

One business event can create transaction, postings, balance update, audit and outbox rows.

## 76.3 Concurrency engineering cost

Correct stock deduction requires real transaction/locking tests.

## 76.4 Valuation complexity

Costing rules introduce additional state and accounting considerations.

## 76.5 Operational tooling requirements

Reconciliation, projection rebuilds and repair workflows must be built and secured.

These costs are deliberate because Sitolo's stock is part of its business truth, financial controls and trust model.

---

# 77. Why This Over Alternatives

## Alternative A — Mutable `stock_quantity` only

Rejected.

It cannot explain causality, safely handle concurrent changes, or provide reliable historical reconstruction.

## Alternative B — Event sourcing for the entire application

Rejected for now.

The inventory ledger needs append-oriented facts, but forcing full event sourcing onto every Sitolo domain adds operational complexity without equivalent benefit.

## Alternative C — Full accounting double-entry ledger for all inventory immediately

Deferred.

Inventory movements have ledger-like properties, but Sitolo should not become a general accounting system merely because stock needs strong history. Accounting integration remains a separate boundary.

## Alternative D — Client-authoritative offline stock

Rejected categorically.

The existing product architecture explicitly keeps PostgreSQL authoritative while SQLite provides client operational continuity. fileciteturn11file1L182-L206

## Alternative E — Last-write-wins replication

Rejected.

Inventory is not a document whose conflicting values can be merged by timestamp without domain semantics.

## Alternative F — Redis as inventory authority

Rejected.

A cache or distributed lock service cannot replace PostgreSQL's authoritative transactional state.

## Alternative G — `SKIP LOCKED` for stock deduction

Rejected.

PostgreSQL documents `SKIP LOCKED` as appropriate for queue-like workloads because it provides an inconsistent view; authoritative stock checks cannot accept that semantic. citeturn926956search60

---

# 78. Final Architecture

```text
                  FLUTTER / TAURI / ADMIN
                           |
                           v
                    RUST API / COMMANDS
                           |
        +------------------+------------------+
        |                  |                  |
        v                  v                  v
    AUTHZ              DOMAIN RULES       IDEMPOTENCY
        |                  |                  |
        +------------------+------------------+
                           |
                           v
                 INVENTORY APPLICATION
                           |
                           v
                  POSTGRES TRANSACTION
                           |
       +-------------------+-------------------+
       |                   |                   |
       v                   v                   v
    LEDGER             BALANCE             AUDIT
   POSTINGS           PROJECTION
       |                   |
       +-------------------+-------------------+
                           |
                           v
                        OUTBOX
                           |
             +-------------+-------------+
             |             |             |
             v             v             v
         REPORTS       ALERTS        INTEGRATIONS
```

The authority model is therefore:

```text
CLIENT
  -> intent

RUST
  -> authentication context
  -> authorization
  -> business decision
  -> transaction orchestration

POSTGRESQL
  -> authoritative inventory truth
  -> constraints
  -> transaction atomicity
  -> concurrency controls
  -> RLS defense in depth

WORKERS
  -> asynchronous derived work

EXTERNAL SYSTEMS
  -> evidence / integrations, never inventory authority
```

---

# 79. Phase 9 Exit Gate

Phase 9 may be marked complete only when the following are all true:

```text
ARCHITECTURE
[X] authoritative ledger model defined
[X] projection model defined
[X] transaction boundaries defined
[X] costing boundary defined

SECURITY
[X] tenant isolation defined
[X] location/branch scope defined
[X] authorization hooks defined
[X] admin repair protection defined
[X] offline trust boundary defined

CORRECTNESS
[X] non-negative policy defined
[X] transfer conservation defined
[X] unit invariants defined
[X] lot/serial semantics defined
[X] idempotency defined

DATABASE
[X] constraints defined
[X] indexes designed
[X] RLS integration defined
[X] migration strategy defined
[X] reconciliation model defined

CONCURRENCY
[X] stock deduction locking defined
[X] deterministic lock ordering defined
[X] deadlock strategy defined
[X] concurrency test matrix defined

OPERATIONS
[X] projection rebuild defined
[X] reconciliation defined
[X] incident runbooks defined
[X] backup/restore verification defined

TESTING
[X] unit tests defined
[X] property tests defined
[X] fuzz tests defined
[X] real PostgreSQL tests defined
[X] RLS tests defined
[X] concurrency tests defined
[X] security tests defined
[X] mutation tests defined

DELIVERY
[X] CI gates defined
[X] release-blocking conditions defined
[X] implementation PR sequence defined
```

A missing executable test for a Critical/High inventory threat is a release blocker, consistent with the existing Sitolo threat and CI contracts. fileciteturn9file0L53-L81 fileciteturn9file1L173-L239

---

# 80. Research Basis

Primary project sources consulted:

- `sitolo.md`
- `domain_model.md`
- `api_contract.md`
- `database_design.md`
- `security_architecture_design.md`
- `security_implementation_spec.md`
- `auth_authorization_spec.md`
- `testing_strategy.md`
- `security_test_harness.md`
- `ci_enforcement.md`
- Phase 2 configuration/observability implementation
- Phase 3 identity/session implementation
- Phase 4 tenant/IAM implementation
- Phase 5 PostgreSQL implementation
- Phase 6 authorization implementation
- Phase 7 security test framework
- Phase 8 product catalogue implementation

External technical research included current documentation and standards covering PostgreSQL 18 concurrency/locking/transactions, SQLx transactions, OWASP authorization/logging/security testing guidance, GS1 traceability, and IAS 2 inventory costing. Relevant findings were incorporated into this implementation rather than copied wholesale.

---

# 81. Final Engineering Position

Sitolo inventory should be implemented as a **transactional, append-oriented, PostgreSQL-authoritative inventory ledger with derived operational projections, explicit concurrency control, bounded offline capability, policy-driven costing, and auditable corrective workflows**.

The most important invariant is not:

```text
stock_quantity >= 0
```

It is:

```text
Every authoritative inventory state must be explainable as the result
of valid, authorized, durable inventory commands executed under the
correct tenant/location scope and committed atomically.
```

That principle is what makes the system resilient against:

```text
concurrency
retries
offline replay
staff error
malicious clients
provider ambiguity
cache corruption
partial failures
schema evolution
operational repair
```

and it is the core reason the ledger architecture is appropriate for Sitolo.
