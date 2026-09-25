# Sitolo — Phase 8 Product Catalogue Implementation Contract

**Status:** Normative Phase 8 target-state contract; not evidence that the catalogue runtime is currently implemented  
**Evidence rule:** Implementation state must be established from current source, tests and CI, never from this contract alone  
**Product:** Sitolo — Business Operating System for African SMEs

---

## 1. Purpose

Phase 8 defines the product catalogue capability required to provide stable Product/SKU identity for pricing, inventory, purchasing and sales.

This document specifies the target behavior. It must not be read as a statement that the catalogue engine, `/products` API, database tables or client surfaces already exist.

---

## 2. Target domain model

```text
Product
  -> SKU
     -> identifiers / barcode
     -> units / conversions
     -> price versions
     -> inventory references
     -> sales references
```

A Product describes the business item. A SKU represents the stock/sellable identity used by downstream operational domains.

Attributes that affect authorization, valuation, stock, tax or reporting must have explicit typed semantics where required by the domain and persistence contracts.

---

## 3. Required invariants

1. Product/SKU records are tenant-owned and cannot cross tenant boundaries.
2. SKU identity remains stable enough to preserve inventory and sales history.
3. Identifier uniqueness follows the approved database contract.
4. Units and conversions are deterministic and validated before quantity-affecting operations.
5. Historical records retain the product/SKU/price meaning that was effective when the transaction was committed.
6. Disabling or retiring catalogue records does not erase or reinterpret historical records.
7. Effective price periods obey the temporal constraints defined by the pricing/database contracts.
8. Catalogue mutations require applicable authentication, authorization, tenant scope, branch scope and entitlement checks.

---

## 4. API target state

When Phase 8 catalogue API work is implemented, the API shall expose the approved product/SKU/pricing commands defined by `docs/api_contract.md`.

The required request path is:

```text
transport validation
    ↓
authentication/session
    ↓
trusted tenant / organization / branch scope
    ↓
authorization
    ↓
domain validation
    ↓
transaction
    ↓
authoritative persistence
    ↓
audit / outbox where required
```

The exact route and schema are owned by `api_contract.md`. This document does not create a competing API definition.

**Current-state note:** the current repository baseline does not yet expose a complete catalogue runtime. A future implementation must not cite this contract as proof that `/products` is already live.

---

## 5. Persistence target state

PostgreSQL is the authoritative server-side store.

Phase 8 implementation must use the Phase 5 schema, constraints and RLS model rather than inventing an independent catalogue persistence model.

Required persistence properties include:

- tenant isolation;
- foreign-key integrity;
- deterministic identifiers;
- temporal price validity;
- historical-reference preservation;
- explicit retire/disable semantics;
- transactional coordination with inventory, purchasing and sales where a command crosses those boundaries.

---

## 6. Security target state

Catalogue operations are subject to:

- authentication;
- authorization;
- tenant and organization scope;
- branch/resource scope where applicable;
- entitlement/module controls;
- input bounds and validation;
- replay/idempotency protection where the command is retryable;
- audit evidence for material privileged changes.

Client-provided tenant or authorization fields are never final authority.

---

## 7. Relationship to inventory and sales

Catalogue defines identity.

Inventory defines quantity and stock state.

Sales define committed commercial transactions.

A catalogue change must not silently mutate the meaning of historical inventory or sales records.

Any SKU/unit migration that could change historical quantity meaning requires an explicit migration or compensating business operation.

---

## 8. Testing target state

Phase 8 implementation requires, at minimum:

- cross-tenant read/write denial;
- identifier collision/uniqueness tests;
- invalid unit/conversion rejection;
- deterministic effective-price tests;
- historical-reference preservation;
- branch/resource authorization tests;
- retry/idempotency tests where applicable;
- concurrency tests for competing catalogue mutations;
- real PostgreSQL tests for PostgreSQL-specific guarantees;
- API contract tests once the HTTP surface exists.

A passing unit test against an in-memory reference implementation does not prove PostgreSQL/RLS or production HTTP isolation.

---

## 9. Implementation-status vocabulary

Use these terms consistently:

| Term | Meaning |
|---|---|
| Target | Required by this Phase 8 contract |
| Implemented | Present in the current source tree |
| Verified | Implemented and supported by executable evidence |
| Historical | True for an earlier repository baseline |
| Deferred | Intentionally owned by a later phase |

This contract is **Target** until current source evidence promotes individual requirements to **Implemented** and then **Verified**.

---

## 10. Canonical dependencies

- `docs/domain_model.md`
- `docs/api_contract.md`
- `docs/database_design.md`
- `docs/phase5_postgresql_schema_migrations_constraints_rls_implementation.md`
- `docs/phase6_authorization_engine_policy_enforcement_implementation.md`
- `docs/phase9_inventory_ledger_implementation.md`
- `docs/phase10_pos_sales_implementation.md`
- `docs/testing_strategy.md`

This file is a Phase 8 contract, not a current-state implementation report.