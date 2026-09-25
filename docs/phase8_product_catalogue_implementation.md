# Sitolo — Phase 8 Product Catalogue Implementation Contract

**Status:** reconstructed implementation contract from the domain model, API contract, database design, Phase 5 schema/RLS work, Phase 9 inventory ledger contract, Phase 10 sales contract and current source tree.

## Objective

Provide a tenant-safe, auditable product catalogue that defines stable product identity and SKU identity for pricing, inventory, purchasing and sales without rewriting historical business meaning.

## Domain model

```text
Product
  -> SKU
     -> barcode / identifiers
     -> units / conversions
     -> price versions
     -> inventory references
     -> sales references
```

A Product describes the business item. A SKU represents the stock/sellable identity used by inventory and sales. Business-specific attributes must remain explicit rather than hidden in untyped blobs when they affect authorization, valuation, stock or reporting.

## Invariants

1. Product/SKU records are tenant-owned and cannot cross tenant boundaries.
2. SKU identity is stable enough to support inventory and sales history.
3. Barcode/identifier uniqueness is scoped according to the database contract and must not create cross-tenant collisions where global uniqueness is not required.
4. Units and conversions are deterministic and validated before affecting quantities.
5. Historical transactions retain their recorded product/SKU/price meaning even after current catalogue state changes.
6. Disabling a catalogue item must not erase or reinterpret historical sales/inventory records.
7. Price versions are time-bounded and deterministic; overlapping effective periods are rejected where the pricing contract requires exclusivity.
8. Catalogue mutations require authorization and tenant/branch scope where applicable.

## API boundary

The API exposes catalogue operations under the existing `/products` and related SKU/pricing surfaces. Request validation, authorization and tenant resolution precede mutation. Exact route schemas remain governed by `api_contract.md`.

## Persistence

PostgreSQL is authoritative. Catalogue tables participate in the Phase 5 migration, constraints and RLS model. Foreign keys must preserve references required by inventory, sales and purchasing. Deletion semantics must prefer disable/retire behavior where historical references exist.

## Security

Catalogue operations are subject to authentication, authorization, tenant isolation and relevant entitlement/module controls. A client cannot select another tenant's product merely by changing an ID.

## Inventory relationship

Catalogue defines identity; inventory defines quantity/state. Inventory mutations must not mutate catalogue identity implicitly. SKU/unit changes that would alter historical quantity meaning require explicit migration/business rules.

## Testing

Required classes include:

- cross-tenant read/write denial;
- duplicate identifier rejection;
- invalid conversion rejection;
- deterministic price-effective-date behavior;
- historical-reference preservation;
- authorization and branch-scope enforcement;
- concurrency/idempotency behavior for competing catalogue mutations where applicable;
- API/database contract tests.

## Canonical references

- `domain_model.md`
- `api_contract.md`
- `database_design.md`
- `phase5_postgresql_schema_migrations_constraints_rls_implementation.md`
- `phase6_authorization_engine_policy_enforcement_implementation.md`
- `phase9_inventory_ledger_implementation.md`
- `phase10_pos_sales_implementation.md`
- `testing_strategy.md`

This document restores the missing Phase 8 filename referenced by later implementation contracts.