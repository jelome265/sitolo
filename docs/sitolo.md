# Sitolo — Product Specification

**Status:** reconstructed canonical baseline; implementation contracts remain authoritative for mechanics.

## Purpose

Sitolo is a Business Operating System for African SMEs, Malawi first. It combines operational execution and business control around one authoritative business record.

## Product scope

- POS and sales
- product catalogue and SKU identity
- pricing
- purchasing and receiving
- inventory and stock control
- customers and suppliers
- cash and payment recording
- payment reconciliation
- reporting and business intelligence
- organization, branch, membership and IAM controls
- auditability and compliance workflows
- offline continuity and synchronization
- enterprise administration and integrations

## Business hierarchy

```text
Principal
  -> Organization
     -> Branch
        -> Business resources
           -> Product / SKU / Price
           -> Inventory
           -> Sales
           -> Payments
           -> Purchasing
```

Organization/tenant ownership is a security boundary. Branch scope is an authorization boundary where applicable.

## Client model

Mobile is the primary operational surface and must support constrained connectivity. Desktop/Tauri is the management/control surface. Web/admin is a supporting administration and acquisition surface.

## Authority rules

- PostgreSQL is authoritative server-side state.
- Client SQLite is continuity state, not permanent business authority.
- Authorization is server-enforced and fail-closed.
- Financial and inventory history must remain auditable and historically meaningful.
- External providers are integration boundaries, not implicit sources of Sitolo truth.
- Business model and commercial assumptions are governed by the commercial corpus.

## Product invariants

1. Cross-tenant access is denied.
2. A principal must have valid identity/session state and applicable scope before mutation.
3. Historical transactions cannot be reinterpreted merely because current catalogue/pricing data changed.
4. Offline commands require identity, tenant scope, command identity, replay protection and server reconciliation.
5. Side effects must be observable and retry-safe where the contract requires them.
6. Product capabilities must respect plan/entitlement boundaries where enabled.

## Canonical implementation references

- `business_model_design.md` — compatibility entry for the commercial source.
- `system_architecture_design.md`
- `domain_model.md`
- `database_design.md`
- `api_contract.md`
- `auth_authorization_spec.md`
- `sync_protocol.md`
- `security_architecture_design.md`
- `security_implementation_spec.md`

This document restores the previously referenced product-specification home; it does not supersede newer implementation contracts.